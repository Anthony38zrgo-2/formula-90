#![deny(unsafe_op_in_unsafe_fn)]
//! AUD-03 release benchmark for GF509, mixer, and the real facade route.

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use formula90_core::{CoreConfig, CoreFacade};
use game_sim::DriverInput;
use serde::Serialize;
use v10_engine_synth::{
    Gf509Runtime, Gf509RuntimeConfig, RuntimeTelemetry, ShiftPhase, TorqueSign,
};
use vehicle_audio_engine::ffi::{VehicleAudioTelemetryV3, VEHICLE_AUDIO_ABI_VERSION};
use vehicle_audio_engine::{ContinuousSourceKind, VehicleAudioEngine};

const SAMPLE_RATE: u32 = 44_100;
const BLOCK: usize = 256;
const WARMUP_BLOCKS: usize = 256;
const MEASURE_BLOCKS: usize = 4_096;
const DT: f32 = BLOCK as f32 / SAMPLE_RATE as f32;

struct CountingAllocator;
static ALLOCS: AtomicU64 = AtomicU64::new(0);
static BYTES: AtomicU64 = AtomicU64::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(size as u64, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, size) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

#[cfg(windows)]
#[repr(C)]
#[derive(Default)]
struct FileTime {
    low: u32,
    high: u32,
}

#[cfg(windows)]
fn thread_cpu_ns() -> Result<u64, String> {
    unsafe extern "system" {
        fn GetCurrentThread() -> *mut std::ffi::c_void;
        fn GetThreadTimes(
            h: *mut std::ffi::c_void,
            c: *mut FileTime,
            e: *mut FileTime,
            k: *mut FileTime,
            u: *mut FileTime,
        ) -> i32;
    }
    let (mut c, mut e, mut k, mut u) = (
        FileTime::default(),
        FileTime::default(),
        FileTime::default(),
        FileTime::default(),
    );
    if unsafe { GetThreadTimes(GetCurrentThread(), &mut c, &mut e, &mut k, &mut u) } == 0 {
        return Err("GetThreadTimes failed".into());
    }
    let ticks = |v: FileTime| ((v.high as u64) << 32) | v.low as u64;
    Ok((ticks(k) + ticks(u)) * 100)
}

#[cfg(not(windows))]
fn thread_cpu_ns() -> Result<u64, String> {
    Err("thread CPU clock unavailable".into())
}

#[derive(Serialize)]
struct ResultRow {
    route: &'static str,
    includes: &'static str,
    blocks: usize,
    thread_cpu_percent: f64,
    wall_us_p50: f64,
    wall_us_p95: f64,
    wall_us_p99: f64,
    wall_us_max: f64,
    allocations: u64,
    allocated_bytes: u64,
}

fn telemetry(block: usize) -> RuntimeTelemetry {
    let phase = (block % 1024) as f32 / 1023.0;
    let rpm = 4_500.0 + 10_500.0 * phase;
    RuntimeTelemetry {
        rpm,
        throttle: 0.88,
        normalized_engine_load: 0.82,
        normalized_engine_torque: 0.78,
        torque_sign: TorqueSign::Positive,
        rpm_derivative: 10_500.0 / (1024.0 * DT),
        throttle_derivative: 0.0,
        gear: if phase < 0.5 { 3 } else { 4 },
        shift_phase: ShiftPhase::None,
        clutch_engagement: 1.0,
        tc_cut_ratio: 0.0,
        rev_limiter_active: false,
        dt_seconds: DT,
    }
}

fn packet(t: RuntimeTelemetry) -> VehicleAudioTelemetryV3 {
    VehicleAudioTelemetryV3 {
        schema_version: VEHICLE_AUDIO_ABI_VERSION,
        struct_size: std::mem::size_of::<VehicleAudioTelemetryV3>() as u32,
        rpm: t.rpm as f64,
        idle_rpm: 4_500.0,
        max_rpm: 17_000.0,
        throttle: t.throttle,
        normalized_engine_load: t.normalized_engine_load,
        normalized_engine_torque: t.normalized_engine_torque,
        rpm_derivative: t.rpm_derivative,
        throttle_derivative: t.throttle_derivative,
        speed_kph: 180.0,
        slip: 0.0,
        gear: t.gear as i32,
        torque_sign: t.torque_sign as i32,
        shift_phase: t.shift_phase as i32,
        clutch_engagement: 1.0,
        tc_cut_ratio: 0.0,
        rev_limiter_active: 0,
    }
}

fn percentile(sorted: &[u128], p: f64) -> f64 {
    let index = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[index] as f64 / 1_000.0
}

fn measure<F>(
    route: &'static str,
    includes: &'static str,
    mut render: F,
) -> Result<ResultRow, String>
where
    F: FnMut(usize) -> Result<(), String>,
{
    for block in 0..WARMUP_BLOCKS {
        render(block)?;
    }
    let mut wall = Vec::with_capacity(MEASURE_BLOCKS);
    let alloc_start = ALLOCS.load(Ordering::Relaxed);
    let bytes_start = BYTES.load(Ordering::Relaxed);
    let cpu_start = thread_cpu_ns()?;
    for block in 0..MEASURE_BLOCKS {
        let start = Instant::now();
        render(block)?;
        wall.push(start.elapsed().as_nanos());
    }
    let cpu_ns = thread_cpu_ns()?.saturating_sub(cpu_start);
    let allocations = ALLOCS.load(Ordering::Relaxed).saturating_sub(alloc_start);
    let allocated_bytes = BYTES.load(Ordering::Relaxed).saturating_sub(bytes_start);
    wall.sort_unstable();
    let audio_ns = MEASURE_BLOCKS as f64 * BLOCK as f64 / SAMPLE_RATE as f64 * 1e9;
    Ok(ResultRow {
        route,
        includes,
        blocks: MEASURE_BLOCKS,
        thread_cpu_percent: cpu_ns as f64 / audio_ns * 100.0,
        wall_us_p50: percentile(&wall, 0.50),
        wall_us_p95: percentile(&wall, 0.95),
        wall_us_p99: percentile(&wall, 0.99),
        wall_us_max: percentile(&wall, 1.0),
        allocations,
        allocated_bytes,
    })
}

fn facade(bank: &Path, profile: &Path, enabled: bool) -> Result<(CoreFacade, u32), String> {
    let config = CoreConfig {
        bank_dir: Some(bank.to_owned()),
        config_json_path: Some(profile.to_owned()),
        use_canonical: false,
        enable_audio: enabled,
        idle_rpm: 4_500.0,
        max_rpm: 17_000.0,
        ..CoreConfig::default()
    };
    let mut facade = CoreFacade::new(config).map_err(|e| e.to_string())?;
    let id = facade.ensure_spawned().map_err(|e| e.to_string())?;
    if enabled && (!facade.audio_healthy() || !facade.audio_gf509_enabled()) {
        return Err("CoreFacade audio/GF509 failed to initialize".into());
    }
    Ok((facade, id))
}

fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 5 {
        return Err("usage: aud_path_bench <bank> <gf509-assets> <profile> <out.json>".into());
    }
    let bank = PathBuf::from(&args[1]);
    let assets = PathBuf::from(&args[2]);
    let profile = PathBuf::from(&args[3]);
    let output = PathBuf::from(&args[4]);
    let mut l = [0.0f32; BLOCK];
    let mut r = [0.0f32; BLOCK];

    let mut procedural_cfg = Gf509RuntimeConfig::default();
    procedural_cfg.engine.sample_rate = SAMPLE_RATE;
    procedural_cfg.max_block_frames = BLOCK;
    let mut procedural = Gf509Runtime::new(procedural_cfg)?;
    let direct_procedural = measure(
        "Gf509RuntimeProcedural",
        "GF509 engine + scene; sample layer disabled",
        |block| {
            procedural.update_telemetry(telemetry(block))?;
            procedural.render_block(black_box(&mut l), black_box(&mut r))?;
            black_box(l[0]);
            Ok(())
        },
    )?;

    let mut cfg = Gf509RuntimeConfig::default();
    cfg.engine.sample_rate = SAMPLE_RATE;
    cfg.max_block_frames = BLOCK;
    cfg.sample_layer_directory = Some(assets.clone());
    let mut gf = Gf509Runtime::new(cfg)?;
    let direct = measure(
        "Gf509Runtime",
        "GF509 engine + scene + sample layer",
        |block| {
            gf.update_telemetry(telemetry(block))?;
            gf.render_block(black_box(&mut l), black_box(&mut r))?;
            black_box(l[0]);
            Ok(())
        },
    )?;

    let mut mixer = VehicleAudioEngine::new(&bank).map_err(|e| e.to_string())?;
    mixer.enable_v10_gf509(&assets)?;
    if mixer.continuous_source() != ContinuousSourceKind::V10Gf509 {
        return Err("mixer GF509 fallback".into());
    }
    let mixed = measure(
        "VehicleAudioEngine",
        "mixer + GF509 + event/surface paths",
        |block| {
            mixer.set_telemetry(&packet(telemetry(block)), "asphalt");
            mixer.render(black_box(&mut l), black_box(&mut r), BLOCK);
            black_box(l[0]);
            Ok(())
        },
    )?;
    mixer.set_telemetry(&packet(telemetry(700)), "asphalt");
    let mixer_callback = measure(
        "VehicleAudioEngineRenderOnly",
        "audio callback render after controls are fixed",
        |_block| {
            mixer.render(black_box(&mut l), black_box(&mut r), BLOCK);
            black_box(l[0]);
            Ok(())
        },
    )?;

    let (mut core, id) = facade(&bank, &profile, true)?;
    let facade_audio = measure(
        "CoreFacade",
        "physics step + AudioModule + mixer + GF509 render",
        |block| {
            let input = DriverInput {
                throttle: 0.88,
                ..Default::default()
            };
            let samples = core.flat_samples(id);
            core.step_standalone(id, &input, &samples, DT as f64);
            core.audio_render(black_box(&mut l), black_box(&mut r), BLOCK);
            black_box((block, l[0]));
            Ok(())
        },
    )?;
    let (mut core_no_audio, id_no_audio) = facade(&bank, &profile, false)?;
    let facade_control = measure(
        "CoreFacadeNoAudio",
        "matched physics step; audio disabled control",
        |_block| {
            let input = DriverInput {
                throttle: 0.88,
                ..Default::default()
            };
            let samples = core_no_audio.flat_samples(id_no_audio);
            core_no_audio.step_standalone(id_no_audio, &input, &samples, DT as f64);
            black_box(core_no_audio.world_time());
            Ok(())
        },
    )?;
    let payload = serde_json::json!({"schema":"aud03-path-benchmark-v1", "release":true,
        "sample_rate_hz":SAMPLE_RATE,"block_frames":BLOCK,"warmup_blocks":WARMUP_BLOCKS,
        "measure_blocks":MEASURE_BLOCKS,"gf509_activated":true,
        "facade_note":"CoreFacade row includes physics; compare matched no-audio control. Facade telemetry is physics-produced and differs from synthetic direct/mixer trace.",
        "results":[direct_procedural,direct,mixed,mixer_callback,facade_audio,facade_control]});
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(
        &output,
        serde_json::to_string_pretty(&payload).unwrap() + "\n",
    )
    .map_err(|e| e.to_string())?;
    println!("{}", serde_json::to_string_pretty(&payload).unwrap());
    Ok(())
}
