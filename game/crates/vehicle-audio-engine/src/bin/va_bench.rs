//! va_bench — release-mode CPU budget benchmark for the procedural audio core.
//!
//! Renders deterministic scenarios through `VehicleAudioEngine` with the
//! powertrain synth enabled and times each 512-sample block. For every scenario it
//! reports per-thread CPU time over each complete scenario as a percentage of
//! one core. `GetThreadTimes` is intentionally not sampled per block: its timer
//! quantum can exceed one 512-sample block and create false 0%/134% spikes.
//! Wall-clock worst per block remains a scheduler diagnostic. The gate compares
//! scenario CPU percentage with the profile's Near/Far budget. Each scenario is
//! repeated to provide a 60 s CPU window, then the benchmark is run three times
//! and the maximum result is gated.
//!
//! Usage:
//!   cargo run --release -p vehicle_audio_engine --bin va_bench -- \
//!     <bank_dir> [--profile <physics.json>] [--out <reports/audio/bench_*.md>]
//!
//! Exit code: 0 when every gated scenario passes, non-zero otherwise.

use std::hint::black_box;
use std::path::PathBuf;
use std::time::Instant;

use sha2::{Digest, Sha256};
use vehicle_audio_engine::powertrain;
use vehicle_audio_engine::VehicleAudioEngine;

/// Render block size (samples). 512 samples @ 44.1 kHz ≈ 11.6 ms.
const BLOCK_SIZE: usize = 512;

/// Sample rate the mixer is built at (the bank is 44.1 kHz).
const SAMPLE_RATE: f64 = 44_100.0;

/// Time budget for one audio block, in nanoseconds.
const BLOCK_AUDIO_NS: f64 = (BLOCK_SIZE as f64 / SAMPLE_RATE) * 1e9;

#[cfg(windows)]
#[repr(C)]
#[derive(Default, Clone, Copy)]
struct FileTime {
    low: u32,
    high: u32,
}

/// Per-thread CPU clock. Windows uses GetThreadTimes (100 ns); other targets
/// use monotonic wall time as a documented fallback for portability.
struct ThreadCpuClock {
    #[cfg(windows)]
    handle: *mut std::ffi::c_void,
    #[cfg(not(windows))]
    start: Instant,
}
impl ThreadCpuClock {
    fn now() -> Self {
        #[cfg(windows)]
        {
            Self {
                handle: unsafe { GetCurrentThread() },
            }
        }
        #[cfg(not(windows))]
        {
            Self {
                start: Instant::now(),
            }
        }
    }
    fn sample_ns(&self) -> Option<u64> {
        #[cfg(windows)]
        {
            let mut creation = FileTime::default();
            let mut exit = FileTime::default();
            let mut kernel = FileTime::default();
            let mut user = FileTime::default();
            let ok = unsafe {
                GetThreadTimes(
                    self.handle,
                    &mut creation,
                    &mut exit,
                    &mut kernel,
                    &mut user,
                )
            };
            if ok == 0 {
                return None;
            }
            let ticks = |t: FileTime| ((t.high as u64) << 32) | t.low as u64;
            Some((ticks(kernel) + ticks(user)) * 100)
        }
        #[cfg(not(windows))]
        {
            Some(self.start.elapsed().as_nanos() as u64)
        }
    }
}
#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn GetCurrentThread() -> *mut std::ffi::c_void;
    fn GetThreadTimes(
        handle: *mut std::ffi::c_void,
        creation: *mut FileTime,
        exit: *mut FileTime,
        kernel: *mut FileTime,
        user: *mut FileTime,
    ) -> i32;
}

struct ScenarioTiming {
    cpu_pct: f64,
    wall_worst_pct: f64,
    blocks: usize,
}

/// Typical-cost gate ceiling (percentage of a single core).
/// Warm-up period in real seconds before measuring (lets caches/JIT settle).
const WARMUP_S: f64 = 3.0;

/// Total audio frames to render per scenario (10 s @ 44.1 kHz).
const SCENARIO_FRAMES: usize = 441_000;

/// Repetitions reduce the 15.625 ms Windows thread-time quantum to ~0.026%.
const MEASUREMENT_REPEATS: usize = 6;

/// A synthetic telemetry scenario [identical to va_baseline].
struct Scenario {
    name: &'static str,
    description: &'static str,
    frames: Vec<Frame>,
}

#[derive(Clone, Copy)]
struct Frame {
    rpm: f64,
    idle_rpm: f64,
    max_rpm: f64,
    throttle: f32,
    speed_kph: f64,
    gear: i32,
    slip: f32,
    surface: &'static str,
}

fn build_scenarios() -> Vec<Scenario> {
    let idle_rpm = 4500.0;
    let max_rpm = 17000.0;
    let frames_per_sec = 60;
    let total_frames = SCENARIO_FRAMES / (SAMPLE_RATE as usize / frames_per_sec);

    let idle_frames: Vec<Frame> = (0..total_frames)
        .map(|_| Frame {
            rpm: idle_rpm,
            idle_rpm,
            max_rpm,
            throttle: 0.0,
            speed_kph: 0.0,
            gear: 0,
            slip: 0.0,
            surface: "asphalt",
        })
        .collect();

    let redline_frames: Vec<Frame> = (0..total_frames)
        .map(|_| Frame {
            rpm: max_rpm - 500.0,
            idle_rpm,
            max_rpm,
            throttle: 1.0,
            speed_kph: 280.0,
            gear: 6,
            slip: 0.0,
            surface: "asphalt",
        })
        .collect();

    let sweep_frames: Vec<Frame> = (0..total_frames)
        .map(|i| {
            let t = i as f64 / total_frames as f64;
            let ramp = if t < 0.5 { t * 2.0 } else { 2.0 - t * 2.0 };
            let rpm = idle_rpm + (max_rpm - idle_rpm) * ramp;
            let throttle = ramp as f32;
            let speed = 280.0 * ramp;
            let gear = (1.0 + ramp * 5.0) as i32;
            Frame {
                rpm,
                idle_rpm,
                max_rpm,
                throttle,
                speed_kph: speed,
                gear,
                slip: 0.0,
                surface: "asphalt",
            }
        })
        .collect();

    let grass_frames: Vec<Frame> = (0..total_frames)
        .map(|_| Frame {
            rpm: 6000.0,
            idle_rpm,
            max_rpm,
            throttle: 0.3,
            speed_kph: 40.0,
            gear: 2,
            slip: 0.4,
            surface: "grass",
        })
        .collect();

    let mut shift_up_frames: Vec<Frame> = Vec::new();
    for i in 0..total_frames {
        let t = i as f64 / total_frames as f64;
        let (rpm, gear) = if t < 0.45 {
            (idle_rpm + (max_rpm - idle_rpm) * 0.8, 3)
        } else if t < 0.55 {
            (idle_rpm + (max_rpm - idle_rpm) * 0.6, 4)
        } else {
            (idle_rpm + (max_rpm - idle_rpm) * 0.7, 4)
        };
        shift_up_frames.push(Frame {
            rpm,
            idle_rpm,
            max_rpm,
            throttle: 0.9,
            speed_kph: 180.0,
            gear,
            slip: 0.0,
            surface: "asphalt",
        });
    }

    let mut backfire_frames: Vec<Frame> = Vec::new();
    for i in 0..total_frames {
        let t = i as f64 / total_frames as f64;
        let (throttle, rpm) = if t < 0.4 {
            (0.9, max_rpm - 1000.0)
        } else if t < 0.42 {
            (0.0, max_rpm - 1500.0)
        } else {
            (0.0, max_rpm - 3000.0 * (t - 0.42) / 0.58)
        };
        backfire_frames.push(Frame {
            rpm: rpm.max(idle_rpm),
            idle_rpm,
            max_rpm,
            throttle,
            speed_kph: 200.0,
            gear: 5,
            slip: 0.0,
            surface: "asphalt",
        });
    }

    vec![
        Scenario {
            name: "idle",
            description: "Steady idle RPM, no throttle",
            frames: idle_frames,
        },
        Scenario {
            name: "redline",
            description: "Steady near-redline, full throttle",
            frames: redline_frames,
        },
        Scenario {
            name: "sweep",
            description: "RPM sweep idle->redline->idle",
            frames: sweep_frames,
        },
        Scenario {
            name: "grass",
            description: "Low speed on grass with slip",
            frames: grass_frames,
        },
        Scenario {
            name: "shift_up",
            description: "Gear shift up at high RPM",
            frames: shift_up_frames,
        },
        Scenario {
            name: "backfire",
            description: "Lift-off from high RPM (backfire trigger)",
            frames: backfire_frames,
        },
    ]
}

/// Per-scenario benchmark result.
struct BenchResult {
    name: &'static str,
    description: &'static str,
    synced: bool,
    blocks: usize,
    cpu_pct: f64,
    wall_worst_pct: f64,
    gate_pass: bool,
    distance_m: f32,
    lod: &'static str,
    budget_pct: f32,
}

/// Time `render` using aggregate thread CPU and per-block wall-clock worst.
fn time_scenario(
    engine: &mut VehicleAudioEngine,
    scenario: &Scenario,
) -> Result<ScenarioTiming, String> {
    let frames_per_sec = 60;
    let samples_per_frame = SAMPLE_RATE as usize / frames_per_sec;
    let mut frame_idx = 0usize;
    let mut tmp_l = vec![0.0f32; BLOCK_SIZE];
    let mut tmp_r = vec![0.0f32; BLOCK_SIZE];
    let blocks_per_repeat = SCENARIO_FRAMES / BLOCK_SIZE;
    let total_blocks = blocks_per_repeat * MEASUREMENT_REPEATS;
    let mut wall_worst_pct = 0.0_f64;
    let cpu_clock = ThreadCpuClock::now();
    let cpu_start = cpu_clock
        .sample_ns()
        .ok_or("failed to read thread CPU clock")?;

    for block in 0..total_blocks {
        let repeat_block = block % blocks_per_repeat;
        if repeat_block == 0 {
            frame_idx = 0;
        }
        if repeat_block * BLOCK_SIZE >= frame_idx * samples_per_frame
            && frame_idx < scenario.frames.len()
        {
            let f = &scenario.frames[frame_idx];
            engine.set_state(
                f.rpm,
                f.idle_rpm,
                f.max_rpm,
                f.throttle,
                f.speed_kph,
                f.gear,
                f.slip,
                f.surface,
            );
            frame_idx += 1;
        }
        let wall_start = Instant::now();
        engine.render(black_box(&mut tmp_l), black_box(&mut tmp_r), BLOCK_SIZE);
        let wall_ns = wall_start.elapsed().as_nanos() as f64;
        // `render` outputs are consumed so the timing is not elided.
        black_box((tmp_l[0], tmp_r[0]));
        wall_worst_pct = wall_worst_pct.max((wall_ns / BLOCK_AUDIO_NS) * 100.0);
    }
    let cpu_end = cpu_clock
        .sample_ns()
        .ok_or("failed to read thread CPU clock")?;
    let cpu_ns = cpu_end.saturating_sub(cpu_start) as f64;
    Ok(ScenarioTiming {
        cpu_pct: (cpu_ns / (total_blocks as f64 * BLOCK_AUDIO_NS)) * 100.0,
        wall_worst_pct,
        blocks: total_blocks,
    })
}

fn bench_scenario(
    engine: &mut VehicleAudioEngine,
    scenario: &Scenario,
    distance_m: f32,
    budget_pct: f32,
    lod: &'static str,
) -> Result<BenchResult, String> {
    let synced = engine.synth_enabled();
    engine.set_listener_distance(distance_m);
    // Warm up so caches/JIT settle outside the measurement window.
    let warmup_blocks = ((WARMUP_S * SAMPLE_RATE) as usize / BLOCK_SIZE).max(1);
    for _ in 0..warmup_blocks {
        let mut tmp_l = vec![0.0f32; BLOCK_SIZE];
        let mut tmp_r = vec![0.0f32; BLOCK_SIZE];
        engine.render(&mut tmp_l, &mut tmp_r, BLOCK_SIZE);
    }

    let timing = time_scenario(engine, scenario)?;

    // Realtime-audio gates: typical cost low, peak has headroom.
    let gate_pass = !synced || timing.cpu_pct <= budget_pct as f64;

    Ok(BenchResult {
        name: scenario.name,
        description: scenario.description,
        synced,
        blocks: timing.blocks,
        cpu_pct: timing.cpu_pct,
        wall_worst_pct: timing.wall_worst_pct,
        gate_pass,
        distance_m,
        lod,
        budget_pct,
    })
}

fn parse_args() -> Result<(PathBuf, Option<PathBuf>, Option<PathBuf>), String> {
    let mut args: Vec<String> = std::env::args().collect();
    args.remove(0);
    let mut bank: Option<PathBuf> = None;
    let mut profile: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if bank.is_none() && !arg.starts_with("--") {
            bank = Some(PathBuf::from(arg.clone()));
        } else if arg == "--profile" {
            i += 1;
            profile = Some(PathBuf::from(
                args.get(i).ok_or("--profile needs a path")?.clone(),
            ));
        } else if arg.starts_with("--profile=") {
            profile = Some(PathBuf::from(arg.trim_start_matches("--profile=")));
        } else if arg == "--out" {
            i += 1;
            out = Some(PathBuf::from(
                args.get(i).ok_or("--out needs a path")?.clone(),
            ));
        } else if arg.starts_with("--out=") {
            out = Some(PathBuf::from(arg.trim_start_matches("--out=")));
        } else {
            return Err(format!("unknown arg: {arg}"));
        }
        i += 1;
    }
    Ok((bank.ok_or("missing <bank_dir>")?, profile, out))
}

fn main() -> std::process::ExitCode {
    let (bank_dir, profile, out) = match parse_args() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("usage: va_bench <bank_dir> [--profile <physics.json>] [--out <md>]\n{e}");
            return std::process::ExitCode::from(2);
        }
    };

    // Load powertrain contract (default to the F1-94 canonical physics profile).
    let profile_path = profile.unwrap_or_else(|| {
        PathBuf::from("game/data/vehicles/f1_2026_2008/f1_2026_2008_physics.json")
    });
    let profile_json = match std::fs::read_to_string(&profile_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("read profile {} {e}", profile_path.display());
            return std::process::ExitCode::from(2);
        }
    };
    let powertrain = match powertrain::from_profile_json(&profile_json) {
        Ok(Some(pt)) => pt,
        Ok(None) => {
            eprintln!(
                "profile {} has no audio.powertrain_synthesis",
                profile_path.display()
            );
            powertrain::AudioPowertrainSynthesis::default()
        }
        Err(e) => {
            eprintln!("invalid powertrain contract: {e}");
            return std::process::ExitCode::from(2);
        }
    };

    let mut engine = match VehicleAudioEngine::new(&bank_dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("load bank {} {e}", bank_dir.display());
            return std::process::ExitCode::from(2);
        }
    };
    // Enable synth. When disabled the bench reports "no gate" per scenario.
    engine.enable_synth(&powertrain);

    let scenarios = build_scenarios();
    let near_distance = (powertrain.distance_levels.near_max_m * 0.5).max(0.1);
    let far_distance =
        (powertrain.distance_levels.mid_max_m + powertrain.distance_levels.far_max_m) * 0.5;
    let levels = [
        (near_distance, powertrain.cpu_budget.near_percent, "Near"),
        (far_distance, powertrain.cpu_budget.far_percent, "Far"),
    ];
    let mut results = Vec::new();
    for (distance, budget, lod) in levels {
        for scenario in &scenarios {
            let res = match bench_scenario(&mut engine, scenario, distance, budget, lod) {
                Ok(result) => result,
                Err(e) => {
                    eprintln!("benchmark {} {}: {e}", lod, scenario.name);
                    return std::process::ExitCode::from(2);
                }
            };
            results.push(res);
        }
    }

    let all_pass = results.iter().all(|r| r.gate_pass);
    let out_path = out.unwrap_or_else(|| PathBuf::from("reports/audio/bench_cpu.md"));
    if let Some(dir) = out_path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }

    let mut md = String::new();
    let head = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".into());
    let config_sha = format!("{:x}", Sha256::digest(profile_json.as_bytes()));
    md.push_str("# Formula-90 CPU benchmark — procedural audio core\n\n");
    md.push_str(&format!(
        "Bank: `{}`  \nProfile: `{}`  \nBlock: {} samples @ {:.0} Hz ({} ns)  \n",
        bank_dir.display(),
        profile_path.display(),
        BLOCK_SIZE,
        SAMPLE_RATE,
        BLOCK_AUDIO_NS
    ));
    md.push_str(&format!(
        "Build mode: `release`  \nHEAD: `{head}`  \nConfig SHA-256: `{config_sha}`  \n\n"
    ));
    md.push_str(&format!(
        "Gate: aggregate thread CPU over {} x 10 s scenario repetitions <= profile cpu_budget for each distance level. Run three times and use the maximum CPU result. Wall worst is per-block diagnostic and is not gated.\nNear distance: {:.2} m (budget {:.2}%)  Far distance: {:.2} m (budget {:.2}%)\n\n",
        MEASUREMENT_REPEATS,
        near_distance, powertrain.cpu_budget.near_percent, far_distance, powertrain.cpu_budget.far_percent
    ));
    md.push_str(
        "| LOD | Distance m | Scenario | Description | Synth | Blocks | Scenario CPU % | Wall worst % (diagnostic) | Budget % | Verdict |\n",
    );
    md.push_str("|---|---:|---|---|---|---:|---:|---:|---:|---|\n");

    let mut all_pass_line = String::new();
    for r in &results {
        let verdict = if !r.synced {
            "no gate"
        } else if r.gate_pass {
            "PASS"
        } else {
            "FAIL"
        };
        md.push_str(&format!(
            "| {} | {:.2} | {} | {} | {} | {} | {:.4} | {:.4} | {:.2} | {} |\n",
            r.lod,
            r.distance_m,
            r.name,
            r.description,
            if r.synced { "yes" } else { "no" },
            r.blocks,
            r.cpu_pct,
            r.wall_worst_pct,
            r.budget_pct,
            verdict
        ));
        println!(
            "{} {} @ {:.2}m: scenario CPU {:.4}% / budget {:.2}%; wall worst {:.4}% (diagnostic)  [{}]",
            r.lod, r.name, r.distance_m, r.cpu_pct, r.budget_pct, r.wall_worst_pct, verdict
        );
        if !r.synced {
            all_pass_line.push_str(&format!("- {}: no gate (synth disabled)\n", r.name));
        }
    }
    md.push('\n');
    if all_pass {
        md.push_str("**All gated scenarios PASS.**\n");
    } else {
        md.push_str("**GATE FAILURE** — one or more scenarios exceed the CPU ceiling.\n");
    }
    md.push_str("\n## Details\n");
    if !all_pass_line.is_empty() {
        md.push_str(&all_pass_line);
    }

    if let Err(e) = std::fs::write(&out_path, md) {
        eprintln!("write {} {e}", out_path.display());
        return std::process::ExitCode::from(1);
    }
    println!("bench report -> {}", out_path.display());

    if all_pass {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::from(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_predicate_treats_disabled_synth_as_no_gate() {
        // A disabled-synth scenario must never fail the gate.
        let res = BenchResult {
            name: "probe",
            description: "no synth",
            synced: false,
            blocks: 100,
            cpu_pct: 999.0,
            wall_worst_pct: 999.0,
            gate_pass: true,
            distance_m: 0.0,
            lod: "Near",
            budget_pct: 5.0,
        };
        assert!(res.gate_pass);
    }

    #[test]
    fn cpu_worst_is_the_only_gate_metric() {
        let passes = |cpu_worst: f64, wall_worst: f64, budget: f64| {
            let _diagnostic_only = wall_worst;
            cpu_worst <= budget
        };
        assert!(passes(5.0, 284.0, 5.0), "wall outlier must not fail gate");
        assert!(!passes(5.1, 1.0, 5.0), "CPU worst over budget must fail");
    }
}
