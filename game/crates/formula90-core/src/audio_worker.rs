//! Dedicated audio worker: moves the whole mixer DSP off the render thread.
//!
//! The host (Godot `F90Core`) spawns one OS thread pinned to a reserved CPU core
//! and calls [`AudioWorker::run`] on it; this module owns the mixer and produces
//! PCM into a lock-free SPSC ring. The main thread only pushes control data
//! (telemetry packets + one-shot/ambient commands) and pulls PCM for
//! `push_buffer` — it never calls the DSP, so a slow render frame can no longer
//! throttle the mixer and a slow mixer can no longer throttle render.
//!
//! Control data crosses threads through bounded queues behind short-lived mutexes
//! (both sides hold them for a memcpy); the sample path is lock-free. Telemetry is
//! queued in order (never latest-value) so gear-change one-shots and timed
//! transitions can't be skipped. The mixer is only ever touched by the worker.

use std::cell::UnsafeCell;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use vehicle_physics_engine::SurfaceType;

use crate::audio::AudioModule;
use crate::audio_telemetry::MechanicalAudioState;
use crate::frame::AudioReadouts;

/// Default ring depth: half a second of stereo frames absorbs long frame hitches.
pub const DEFAULT_RING_CAPACITY_FRAMES: usize = 22_050;
/// Refill target until the host sets a frame-aware one (the host overrides this
/// every `_process`); kept small on purpose so a never-configured worker still
/// adds little latency.
pub const DEFAULT_HIGH_WATER_FRAMES: usize = 1_024;
/// DSP block size; small enough to interleave with control updates.
pub const DEFAULT_CHUNK_FRAMES: usize = 1_024;
/// Per-iteration catch-up ceiling.
pub const DEFAULT_MAX_BATCH_FRAMES: usize = 2_048;
/// Worker wake period. Kept short because each iteration also spends the render
/// time for whatever it tops up: in debug a 1.5-frame top-up is ~5 ms of DSP, so
/// a 5 ms sleep made the cycle (~11 ms) longer than a rendered frame and the
/// host could pull twice between refills (measured starvation at 110 FPS).
pub const DEFAULT_PERIOD: Duration = Duration::from_millis(1);
/// Bounded telemetry queue (1024 packets ~= 8.5 s at 120 Hz).
pub const DEFAULT_QUEUE_CAPACITY: usize = 1_024;
/// Bounded command queue (triggers/ambient/reset).
pub const DEFAULT_COMMAND_CAPACITY: usize = 256;

/// Surface kind carried across threads (mirrors the audible `SurfaceType` split).
pub(crate) const SURFACE_KIND_ROAD: u8 = 0;
pub(crate) const SURFACE_KIND_CURB: u8 = 1;
pub(crate) const SURFACE_KIND_GRASS: u8 = 2;
pub(crate) const SURFACE_KIND_GRAVEL: u8 = 3;
pub(crate) const SURFACE_KIND_DIRT: u8 = 4;
pub(crate) const SURFACE_KIND_SAND: u8 = 5;

/// Collapse a physics surface to the audible kind (Wall/Metal scrub like road).
pub(crate) fn surface_kind(surface: SurfaceType) -> u8 {
    match surface {
        SurfaceType::Curb => SURFACE_KIND_CURB,
        SurfaceType::Grass => SURFACE_KIND_GRASS,
        SurfaceType::Gravel => SURFACE_KIND_GRAVEL,
        SurfaceType::Dirt => SURFACE_KIND_DIRT,
        SurfaceType::Sand => SURFACE_KIND_SAND,
        _ => SURFACE_KIND_ROAD,
    }
}

/// Rebuild the physics surface token from its cross-thread kind.
pub(crate) fn surface_from_kind(kind: u8) -> SurfaceType {
    match kind {
        SURFACE_KIND_CURB => SurfaceType::Curb,
        SURFACE_KIND_GRASS => SurfaceType::Grass,
        SURFACE_KIND_GRAVEL => SurfaceType::Gravel,
        SURFACE_KIND_DIRT => SurfaceType::Dirt,
        SURFACE_KIND_SAND => SurfaceType::Sand,
        _ => SurfaceType::Road,
    }
}

/// Underfloor scrape step. `present` distinguishes "not sampled this path"
/// (standalone ticks) from an explicit "no scrape" observation.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct AudioScrapeStep {
    pub present: bool,
    pub active: bool,
    pub intensity: f32,
    pub speed_m_s: f32,
    pub onset_strength: f32,
}

/// One physics-tick audio input sample. Everything the mixer needs to advance its
/// control state for that tick; the worker applies these in push order.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AudioStepPacket {
    pub surface: u8,
    pub rpm: f64,
    pub idle_rpm: f64,
    pub max_rpm: f64,
    pub throttle: f32,
    pub speed_kph: f64,
    pub gear: i32,
    pub slip: f32,
    pub dt: f32,
    pub mechanical: MechanicalAudioState,
    pub scrub_slip_ratio: [f32; 4],
    pub scrub_slip_angle: [f32; 4],
    pub scrub_contact_fraction: [f32; 4],
    pub scrub_normal_force: [f32; 4],
    pub scrub_speed_kph: f32,
    pub scrape: AudioScrapeStep,
}

/// Out-of-band controls pushed by the render/GDScript side.
#[derive(Clone, Copy, Debug)]
pub(crate) enum AudioCommand {
    Trigger(i32),
    Ambient {
        distance_m: f32,
        tc_cut_ratio: f32,
        limiter_active: bool,
    },
    Reset,
}

/// Tunables; `Default` matches the shipped runtime, tests shrink the ring.
#[derive(Clone, Copy, Debug)]
pub struct AudioWorkerConfig {
    pub ring_capacity_frames: usize,
    pub high_water_frames: usize,
    pub chunk_frames: usize,
    pub max_batch_frames: usize,
    pub period: Duration,
    pub queue_capacity: usize,
    pub command_capacity: usize,
}

impl Default for AudioWorkerConfig {
    fn default() -> Self {
        Self {
            ring_capacity_frames: DEFAULT_RING_CAPACITY_FRAMES,
            high_water_frames: DEFAULT_HIGH_WATER_FRAMES,
            chunk_frames: DEFAULT_CHUNK_FRAMES,
            max_batch_frames: DEFAULT_MAX_BATCH_FRAMES,
            period: DEFAULT_PERIOD,
            queue_capacity: DEFAULT_QUEUE_CAPACITY,
            command_capacity: DEFAULT_COMMAND_CAPACITY,
        }
    }
}

/// POD stats snapshot for the host (mirrors `F90AudioWorkerStats`).
#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct AudioWorkerStats {
    pub produced_frames: u64,
    pub consumed_frames: u64,
    pub starved_iterations: u64,
    pub packets_applied: u64,
    pub packets_dropped: u64,
    pub commands_applied: u64,
    pub render_usec_total: u64,
    pub iterations: u64,
    pub ring_frames: u32,
    pub ring_capacity_frames: u32,
    pub high_water_frames: u32,
    pub healthy: u32,
}

/// Single-producer/single-consumer stereo frame ring (interleaved `f32`).
///
/// Producer = worker thread (`write_frames`), consumer = host thread
/// (`read_frames`). Indices are frame counts published with release stores, so
/// the two sides never need a lock on the sample path.
struct SpscRing {
    data: UnsafeCell<Box<[f32]>>,
    head: AtomicUsize,
    tail: AtomicUsize,
    capacity_frames: usize,
}

// SAFETY: access is split by role (one writer, one reader) and synchronized by the
// release/acquire index stores; the buffer is never resized after construction.
unsafe impl Sync for SpscRing {}

impl SpscRing {
    fn new(capacity_frames: usize) -> Self {
        let capacity_frames = capacity_frames.max(2);
        Self {
            data: UnsafeCell::new(vec![0.0; capacity_frames * 2].into_boxed_slice()),
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            capacity_frames,
        }
    }

    fn available_frames(&self) -> usize {
        self.tail.load(Ordering::Acquire) - self.head.load(Ordering::Acquire)
    }

    fn free_frames(&self) -> usize {
        self.capacity_frames - self.available_frames()
    }

    fn write_frames(&self, src: &[f32]) -> usize {
        let frames = src.len() / 2;
        let n = frames.min(self.free_frames());
        if n == 0 {
            return 0;
        }
        let tail = self.tail.load(Ordering::Relaxed);
        let start = tail % self.capacity_frames;
        let first = n.min(self.capacity_frames - start);
        // SAFETY: only the worker (producer) calls this, and `n <= free_frames()`
        // guarantees the written slots are not visible to the consumer yet.
        let data = unsafe { &mut *self.data.get() };
        data[start * 2..(start + first) * 2].copy_from_slice(&src[..first * 2]);
        if n > first {
            let rest = n - first;
            data[..rest * 2].copy_from_slice(&src[first * 2..(first + rest) * 2]);
        }
        self.tail.store(tail + n, Ordering::Release);
        n
    }

    fn read_frames(&self, out_l: &mut [f32], out_r: &mut [f32], frames: usize) -> usize {
        let n = frames
            .min(out_l.len())
            .min(out_r.len())
            .min(self.available_frames());
        if n == 0 {
            return 0;
        }
        let head = self.head.load(Ordering::Relaxed);
        let start = head % self.capacity_frames;
        let first = n.min(self.capacity_frames - start);
        // SAFETY: only the host (consumer) calls this, and `n <= available_frames()`
        // guarantees the read slots were fully published by the producer.
        let data = unsafe { &*self.data.get() };
        for i in 0..first {
            out_l[i] = data[(start + i) * 2];
            out_r[i] = data[(start + i) * 2 + 1];
        }
        if n > first {
            let rest = n - first;
            for i in 0..rest {
                out_l[first + i] = data[i * 2];
                out_r[first + i] = data[i * 2 + 1];
            }
        }
        self.head.store(head + n, Ordering::Release);
        n
    }
}

/// Worker-private mixer state. Guarded by a mutex that only the worker locks; the
/// host never contends for it.
struct WorkerState {
    module: AudioModule,
    scratch_l: Vec<f32>,
    scratch_r: Vec<f32>,
    interleaved: Vec<f32>,
}

#[derive(Default)]
struct WorkerCounters {
    produced_frames: AtomicU64,
    consumed_frames: AtomicU64,
    starved_iterations: AtomicU64,
    packets_applied: AtomicU64,
    packets_dropped: AtomicU64,
    commands_applied: AtomicU64,
    render_usec_total: AtomicU64,
    iterations: AtomicU64,
}

/// Owns the mixer on its own thread; the facade keeps a client handle.
pub struct AudioWorker {
    state: Mutex<WorkerState>,
    packets: Mutex<VecDeque<AudioStepPacket>>,
    commands: Mutex<VecDeque<AudioCommand>>,
    readouts: Mutex<AudioReadouts>,
    ring: SpscRing,
    counters: WorkerCounters,
    stop: AtomicBool,
    /// Steady-state ring occupancy target in frames. The host lowers/raises it
    /// from the measured frame demand so the extra latency over the inline path
    /// stays at roughly one frame instead of a fixed 0.1 s.
    target_frames: AtomicUsize,
    config: AudioWorkerConfig,
    healthy: bool,
    synth_enabled: bool,
    gf509_enabled: bool,
}

impl AudioWorker {
    /// Take ownership of `module` and prepare the worker.
    pub fn new(module: AudioModule, config: AudioWorkerConfig) -> Self {
        let healthy = module.healthy();
        let synth_enabled = module.synth_enabled();
        let gf509_enabled = module.gf509_enabled();
        let readouts = AudioReadouts::default();
        let chunk = config.chunk_frames.max(1);
        Self {
            state: Mutex::new(WorkerState {
                module,
                scratch_l: vec![0.0; chunk],
                scratch_r: vec![0.0; chunk],
                interleaved: vec![0.0; chunk * 2],
            }),
            packets: Mutex::new(VecDeque::with_capacity(config.queue_capacity.min(4_096))),
            commands: Mutex::new(VecDeque::with_capacity(config.command_capacity.min(1_024))),
            readouts: Mutex::new(readouts),
            ring: SpscRing::new(config.ring_capacity_frames),
            counters: WorkerCounters::default(),
            stop: AtomicBool::new(false),
            target_frames: AtomicUsize::new(config.high_water_frames),
            config,
            healthy,
            synth_enabled,
            gf509_enabled,
        }
    }

    pub fn healthy(&self) -> bool {
        self.healthy
    }

    pub fn synth_enabled(&self) -> bool {
        self.synth_enabled
    }

    pub fn gf509_enabled(&self) -> bool {
        self.gf509_enabled
    }

    /// Push one physics-tick packet. Drops the oldest packet on overflow (counted,
    /// never silent) so the queue can never grow without bound.
    pub(crate) fn push_step(&self, packet: AudioStepPacket) {
        let mut queue = lock(&self.packets);
        if queue.len() >= self.config.queue_capacity {
            queue.pop_front();
            self.counters
                .packets_dropped
                .fetch_add(1, Ordering::Relaxed);
        }
        queue.push_back(packet);
    }

    /// Push an out-of-band command (trigger/ambient/reset).
    pub(crate) fn push_command(&self, command: AudioCommand) {
        let mut queue = lock(&self.commands);
        if queue.len() >= self.config.command_capacity {
            queue.pop_front();
            self.counters
                .packets_dropped
                .fetch_add(1, Ordering::Relaxed);
        }
        queue.push_back(command);
    }

    /// Latest published presentation readouts (lock-free enough: a short copy).
    pub fn readouts(&self) -> AudioReadouts {
        *lock(&self.readouts)
    }

    /// Drain up to `frames` stereo frames for the host's `push_buffer`.
    pub fn pull(&self, out_l: &mut [f32], out_r: &mut [f32], frames: usize) -> usize {
        let n = self.ring.read_frames(out_l, out_r, frames);
        self.counters
            .consumed_frames
            .fetch_add(n as u64, Ordering::Relaxed);
        n
    }

    pub fn ring_frames(&self) -> usize {
        self.ring.available_frames()
    }

    pub fn ring_capacity_frames(&self) -> usize {
        self.config.ring_capacity_frames
    }

    /// Set the steady-state ring occupancy target (frames). Called by the host
    /// every rendered frame with the measured frame demand; clamped so it can
    /// never starve (below 256 frames) or exceed half the ring.
    pub fn set_target(&self, frames: usize) {
        let ceiling = (self.config.ring_capacity_frames / 2).max(256);
        let clamped = frames.clamp(256, ceiling);
        self.target_frames.store(clamped, Ordering::Relaxed);
    }

    pub fn target_frames(&self) -> usize {
        self.target_frames.load(Ordering::Relaxed)
    }

    pub fn stats(&self) -> AudioWorkerStats {
        AudioWorkerStats {
            produced_frames: self.counters.produced_frames.load(Ordering::Relaxed),
            consumed_frames: self.counters.consumed_frames.load(Ordering::Relaxed),
            starved_iterations: self.counters.starved_iterations.load(Ordering::Relaxed),
            packets_applied: self.counters.packets_applied.load(Ordering::Relaxed),
            packets_dropped: self.counters.packets_dropped.load(Ordering::Relaxed),
            commands_applied: self.counters.commands_applied.load(Ordering::Relaxed),
            render_usec_total: self.counters.render_usec_total.load(Ordering::Relaxed),
            iterations: self.counters.iterations.load(Ordering::Relaxed),
            ring_frames: self.ring_frames() as u32,
            ring_capacity_frames: self.config.ring_capacity_frames as u32,
            high_water_frames: self.target_frames() as u32,
            healthy: u32::from(self.healthy),
        }
    }

    /// Apply pending control data, publish readouts and render up to
    /// `frames_requested` into the ring. Runs on the worker thread only.
    pub fn pump_once(&self, frames_requested: usize) -> usize {
        let mut guard = lock(&self.state);
        // Split the worker state into independent borrows so the mixer call can take
        // both scratch buffers mutably at once.
        let WorkerState {
            module,
            scratch_l,
            scratch_r,
            interleaved,
        } = &mut *guard;
        let mut applied = 0u64;
        let mut commands = 0u64;
        loop {
            let command = { lock(&self.commands).pop_front() };
            match command {
                Some(AudioCommand::Trigger(code)) => {
                    module.trigger_code(code);
                    commands += 1;
                }
                Some(AudioCommand::Ambient {
                    distance_m,
                    tc_cut_ratio,
                    limiter_active,
                }) => {
                    module.set_ambient(distance_m, tc_cut_ratio, limiter_active);
                    commands += 1;
                }
                Some(AudioCommand::Reset) => {
                    // Mirror `CoreFacade::reset`: clear the scrape first, then the mixer.
                    module.set_scrape_state(false, 0.0, 0.0, 0.0);
                    module.reset();
                    commands += 1;
                }
                None => break,
            }
        }
        loop {
            let packet = { lock(&self.packets).pop_front() };
            match packet {
                Some(packet) => {
                    module.apply_step_packet(&packet);
                    applied += 1;
                }
                None => break,
            }
        }
        if applied > 0 || commands > 0 {
            *lock(&self.readouts) = module.readouts();
        }

        let mut produced = 0usize;
        let mut render_usec = 0u64;
        let mut remaining = frames_requested;
        while remaining > 0 {
            let free = self.ring.free_frames();
            if free == 0 {
                break;
            }
            let n = remaining.min(free).min(self.config.chunk_frames).max(1);
            if scratch_l.len() < n {
                scratch_l.resize(n, 0.0);
                scratch_r.resize(n, 0.0);
                interleaved.resize(n * 2, 0.0);
            }
            let start = Instant::now();
            module.render(&mut scratch_l[..n], &mut scratch_r[..n], n);
            render_usec += start.elapsed().as_micros() as u64;
            for i in 0..n {
                interleaved[i * 2] = scratch_l[i];
                interleaved[i * 2 + 1] = scratch_r[i];
            }
            let wrote = self.ring.write_frames(&interleaved[..n * 2]);
            produced += wrote;
            if wrote < n {
                break;
            }
            remaining -= n;
        }

        self.counters
            .packets_applied
            .fetch_add(applied, Ordering::Relaxed);
        self.counters
            .commands_applied
            .fetch_add(commands, Ordering::Relaxed);
        self.counters
            .produced_frames
            .fetch_add(produced as u64, Ordering::Relaxed);
        self.counters
            .render_usec_total
            .fetch_add(render_usec, Ordering::Relaxed);
        self.counters.iterations.fetch_add(1, Ordering::Relaxed);
        produced
    }

    /// Blocking worker loop; returns after `request_stop()`.
    pub fn run(&self) {
        self.run_until(|| self.stop.load(Ordering::Relaxed));
    }

    /// Blocking worker loop with an external stop predicate. The host polls its own
    /// flag (it owns the OS thread); `request_stop()` stays available for
    /// in-process callers and tests.
    pub fn run_until<F: Fn() -> bool>(&self, stop: F) {
        while !stop() {
            let available = self.ring.available_frames();
            let want = self
                .target_frames()
                .saturating_sub(available)
                .min(self.config.max_batch_frames);
            let produced = self.pump_once(want);
            if want > 0 && produced == 0 {
                self.counters
                    .starved_iterations
                    .fetch_add(1, Ordering::Relaxed);
            }
            // Refill BEFORE sleeping: the host drains on rendered frames, which
            // can be shorter than this loop's work+sleep cycle. Sleeping while
            // below target let two host pulls land between refills and starved
            // the generator (measured 213 skips at 115 FPS in debug).
            if want == 0 || produced == 0 || self.ring.available_frames() >= self.target_frames() {
                std::thread::sleep(self.config.period);
            }
        }
        // Flush any control data still queued so shutdown leaves no pending work.
        self.pump_once(0);
    }

    pub fn request_stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// Lock helper that survives a poisoned mutex: the guarded data is plain audio
/// state, so a panicking peer must not take the mixer down with it.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Resolve the packaged GF509 bank shipped with the game (tests/diagnostics).
#[cfg(test)]
pub(crate) fn packaged_bank_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sounds/banks/v10_vehicle")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn test_module() -> Option<AudioModule> {
        let bank = packaged_bank_dir();
        if !bank.exists() {
            return None;
        }
        let mut module = AudioModule::new(Some(&bank), true);
        let profile = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/vehicles/f1_2030/f1_2030_v10_geometric.json");
        if let Ok(text) = std::fs::read_to_string(&profile) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                let mut audio = json.get("audio").cloned();
                // The Faust/C++ layer is an external DLL with its own state; keep
                // the parity test on the deterministic Rust path.
                if let Some(section) = audio.as_mut().and_then(|a| a.get_mut("cpp_dsp")) {
                    if let Some(object) = section.as_object_mut() {
                        object.insert("enabled".to_string(), serde_json::Value::Bool(false));
                    }
                }
                module.enable_synth_from_profile(audio.as_ref());
            }
        }
        Some(module)
    }

    fn test_packet(step: usize) -> AudioStepPacket {
        let t = step as f32 / 400.0;
        let rpm = 4_000.0 + 11_000.0 * t;
        let throttle = if t < 0.7 { 0.9 } else { 0.1 };
        AudioStepPacket {
            surface: if t > 0.5 { SURFACE_KIND_CURB } else { SURFACE_KIND_ROAD },
            rpm: rpm as f64,
            idle_rpm: 1_000.0,
            max_rpm: 15_000.0,
            throttle,
            speed_kph: (30.0 + 220.0 * t) as f64,
            gear: if t < 0.4 { 3 } else { 4 },
            slip: 0.05 + 0.1 * t,
            dt: 1.0 / 120.0,
            mechanical: MechanicalAudioState {
                load: 0.6 + 0.3 * t,
                torque: 0.5 - t,
                clutch: 1.0,
                tc: 0.0,
                limiter: false,
                shifting: (0.40..0.42).contains(&t),
                downshift: false,
            },
            scrub_slip_ratio: [0.1 + t, 0.12 + t, 0.2 + t, 0.22 + t],
            scrub_slip_angle: [0.01, -0.01, 0.02, -0.02],
            scrub_contact_fraction: [1.0, 1.0, 0.95, 0.95],
            scrub_normal_force: [3_500.0, 3_600.0, 4_100.0, 4_200.0],
            scrub_speed_kph: (30.0 + 220.0 * t) as f32,
            scrape: AudioScrapeStep::default(),
        }
    }

    #[test]
    fn ring_round_trip_wraps_without_loss() {
        let ring = SpscRing::new(8);
        let mut l = [0.0f32; 5];
        let mut r = [0.0f32; 5];
        let mut written_total = 0usize;
        let mut read_total = 0usize;
        // Force several wraps: write 5, read 3, repeatedly.
        for block in 0..50 {
            let src: Vec<f32> = (0..5)
                .flat_map(|i| [block as f32 * 100.0 + i as f32, -(block as f32 * 100.0 + i as f32)])
                .collect();
            let wrote = ring.write_frames(&src);
            written_total += wrote;
            let n = ring.read_frames(&mut l, &mut r, 3);
            for i in 0..n {
                assert_eq!(l[i], r[i] * -1.0, "block {block} channel mismatch");
            }
            read_total += n;
        }
        assert!(written_total >= read_total);
        assert!(ring.available_frames() <= 8);
    }

    #[test]
    fn worker_pcm_matches_inline_pump() {
        let Some(reference) = test_module() else {
            eprintln!("[audio_worker] bank missing; skipping PCM parity test");
            return;
        };
        let Some(module) = test_module() else {
            eprintln!("[audio_worker] bank missing; skipping PCM parity test");
            return;
        };
        let mut reference = reference;
        let config = AudioWorkerConfig {
            ring_capacity_frames: 8_192,
            high_water_frames: 8_192,
            chunk_frames: 256,
            max_batch_frames: 256,
            queue_capacity: 512,
            command_capacity: 64,
            ..AudioWorkerConfig::default()
        };
        let worker = AudioWorker::new(module, config);

        let frames = 256usize;
        let mut ref_l = vec![0.0f32; frames];
        let mut ref_r = vec![0.0f32; frames];
        let mut got_l = vec![0.0f32; frames];
        let mut got_r = vec![0.0f32; frames];
        for step in 0..400 {
            let packet = test_packet(step);
            reference.apply_step_packet(&packet);
            worker.push_step(packet);
            reference.render(&mut ref_l, &mut ref_r, frames);
            let produced = worker.pump_once(frames);
            assert_eq!(produced, frames, "worker must produce a full block at step {step}");
            let pulled = worker.pull(&mut got_l, &mut got_r, frames);
            assert_eq!(pulled, frames);
            for i in 0..frames {
                assert_eq!(
                    ref_l[i].to_bits(),
                    got_l[i].to_bits(),
                    "left PCM diverged at step {step} sample {i}"
                );
                assert_eq!(
                    ref_r[i].to_bits(),
                    got_r[i].to_bits(),
                    "right PCM diverged at step {step} sample {i}"
                );
            }
        }
        let stats = worker.stats();
        assert_eq!(stats.packets_applied, 400);
        assert_eq!(stats.packets_dropped, 0);
        assert_eq!(stats.consumed_frames, 400 * frames as u64);
    }

    #[test]
    fn triggers_and_ambient_survive_the_queue() {
        let Some(module) = test_module() else {
            eprintln!("[audio_worker] bank missing; skipping command test");
            return;
        };
        let worker = AudioWorker::new(module, AudioWorkerConfig::default());
        worker.push_command(AudioCommand::Trigger(0));
        worker.push_command(AudioCommand::Ambient {
            distance_m: 12.0,
            tc_cut_ratio: 0.25,
            limiter_active: false,
        });
        worker.push_command(AudioCommand::Reset);
        worker.pump_once(0);
        let stats = worker.stats();
        assert_eq!(stats.commands_applied, 3);
    }
}
