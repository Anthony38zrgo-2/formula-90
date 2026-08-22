//! Golden trace: deterministic RPM sweep through `CommonV10BankAdapter`.
//!
//! Baseline documentation of what the CURRENT approximation tables emit, used
//! to verify "misma telemetría produce misma secuencia" and to expose coverage
//! holes before the exact FMOD curve tables land (handoff section 7, paso 3).
//!
//! Writes `F90_GOLDEN_TRACE_OUT` (JSON) when that env var is set; the in-memory
//! determinism/budget checks always run.

use std::io::Write;
use vehicle_audio_engine::{AudioBackend, AudioTelemetryFrame, CommonV10BankAdapter};

const RPM_MIN: f32 = 2053.0;
const RPM_MAX: f32 = 17400.0;
const RPM_STEP: f32 = 100.0;
const TICKS_PER_STEP: u64 = 2; // one idle frame + one settled frame per rpm
/// Fase 1 gate (handoff section 7 paso 3): interior sweep without gain holes
/// deeper than 12 dB anywhere in 2053..17400 rpm, on and off throttle.
const HOLE_THRESHOLD_DB: f32 = -12.0;

#[derive(serde::Serialize)]
struct VoiceTrace {
    id: String,
    event: String,
    sample: String,
    gain_db: f32,
    pitch_semitones: f32,
    looped: bool,
    restart_on_finish: bool,
    emitter: String,
}

#[derive(serde::Serialize)]
struct TickTrace {
    tick: u64,
    rpm: f32,
    throttle: f32,
    voices: Vec<VoiceTrace>,
    one_shot_ids: Vec<u64>,
    stops: Vec<String>,
}

#[derive(serde::Serialize)]
struct HoleReport {
    rpm: f32,
    engine_int_total_db: f32,
    layer_gain_db: Vec<(String, f32)>,
}

#[derive(serde::Serialize)]
struct SweepTrace {
    mode: &'static str,
    ticks: Vec<TickTrace>,
    engine_int_min_total_db: f32,
    holes: Vec<HoleReport>,
    ext_bed_min_total_db: f32,
    ext_bed_holes: Vec<HoleReport>,
    max_voices: usize,
    one_shot_count: u64,
}

#[derive(serde::Serialize)]
struct GoldenTrace {
    schema: &'static str,
    backend: &'static str,
    rpm_min: f32,
    rpm_max: f32,
    rpm_step: f32,
    sweeps: Vec<SweepTrace>,
}

fn base_telemetry(rpm: f32, throttle: f32, tick: u64) -> AudioTelemetryFrame {
    base_telemetry_with_listener(rpm, throttle, tick, 0.0)
}

fn base_telemetry_with_listener(
    rpm: f32,
    throttle: f32,
    tick: u64,
    listener_distance_m: f32,
) -> AudioTelemetryFrame {
    AudioTelemetryFrame {
        tick,
        dt_s: 1.0 / 120.0,
        rpm,
        throttle,
        speed_kph: (rpm / 60.0).clamp(0.0, 300.0),
        drivetrain_speed_kph: (rpm / 60.0).clamp(0.0, 300.0),
        gear: 3,
        slip: 0.0,
        slip_ratio: [0.0; 4],
        surfaces: [0; 4],
        brake: 0.0,
        wheel_load_n: [2500.0; 4],
        suspension_velocity_m_s: [0.0; 4],
        tire_pressure_kpa: [250.0; 4],
        listener_distance_m, // cockpit listener: interior mix at full
        underfloor_scrape_active: false,
        underfloor_scrape_intensity: 0.0,
        underfloor_scrape_speed_m_s: 0.0,
    }
}

fn run_sweep(throttle: f32, mode: &'static str) -> SweepTrace {
    run_sweep_with_listener(throttle, mode, 0.0)
}

/// Sweep with an explicit listener distance (0 = cockpit: exterior fully
/// muted by the interior/exterior mix; 10 = chase cam: exterior audible).
fn run_sweep_with_listener(throttle: f32, mode: &'static str, listener: f32) -> SweepTrace {
    let mut adapter = CommonV10BankAdapter::default();
    let mut tick: u64 = 1;
    let mut ticks = Vec::new();
    let mut holes = Vec::new();
    let mut ext_bed_holes = Vec::new();
    let mut engine_min_total_db = f32::MAX;
    let mut ext_bed_min_total_db = f32::MAX;
    let mut max_voices = 0usize;
    let mut one_shot_count = 0u64;
    let mut rpm = RPM_MIN;
    while rpm <= RPM_MAX + f32::EPSILON {
        for _settle in 0..TICKS_PER_STEP {
            let frame = adapter.adapt(
                base_telemetry_with_listener(rpm, throttle, tick, listener),
                AudioBackend::CommonV10Commands,
            );
            max_voices = max_voices.max(frame.voices.len());
            one_shot_count += frame.one_shots.len() as u64;

            // Per-layer engine_int gain (linear power sum -> dB).
            let mut linear: f32 = 0.0;
            let mut ext_bed_linear: f32 = 0.0;
            let mut layers = Vec::new();
            let mut ext_layers = Vec::new();
            for v in &frame.voices {
                if v.id.starts_with("v10.engine.") {
                    let g = 10.0_f32.powf(v.gain_db / 20.0);
                    linear += g * g;
                    layers.push((v.id.clone(), v.gain_db));
                } else if v.id.starts_with("v10.engine_ext.") {
                    // Bed layers only (idle/off_*): the always-on exterior.
                    let is_bed = v.id.contains("idle")
                        || v.id.contains("off_mid")
                        || v.id.contains("off_midhigh")
                        || v.id.contains("off_downshift");
                    if is_bed {
                        let g = 10.0_f32.powf(v.gain_db / 20.0);
                        ext_bed_linear += g * g;
                        ext_layers.push((v.id.clone(), v.gain_db));
                    }
                }
            }
            let total_db = if linear > 0.0 { 10.0 * linear.log10() } else { -80.0 };
            engine_min_total_db = engine_min_total_db.min(total_db);
            if total_db < HOLE_THRESHOLD_DB {
                holes.push(HoleReport {
                    rpm,
                    engine_int_total_db: total_db,
                    layer_gain_db: layers,
                });
            }
            let ext_db = if ext_bed_linear > 0.0 {
                10.0 * ext_bed_linear.log10()
            } else {
                -80.0
            };
            ext_bed_min_total_db = ext_bed_min_total_db.min(ext_db);
            if ext_db < HOLE_THRESHOLD_DB {
                ext_bed_holes.push(HoleReport {
                    rpm,
                    engine_int_total_db: ext_db,
                    layer_gain_db: ext_layers,
                });
            }

            // Surface latency: recording the own tick line for the report.
            if _settle == TICKS_PER_STEP - 1 {
                ticks.push(TickTrace {
                    tick,
                    rpm,
                    throttle,
                    voices: frame
                        .voices
                        .iter()
                        .map(|v| VoiceTrace {
                            id: v.id.clone(),
                            event: v.event.clone(),
                            sample: v.sample.clone(),
                            gain_db: v.gain_db,
                            pitch_semitones: v.pitch_semitones,
                            looped: v.looped,
                            restart_on_finish: v.restart_on_finish,
                            emitter: v.emitter.clone(),
                        })
                        .collect(),
                    one_shot_ids: frame.one_shots.iter().map(|s| s.id).collect(),
                    stops: frame.stops.clone(),
                });
            }
            tick += 1;
        }
        rpm += RPM_STEP;
    }
    SweepTrace {
        mode,
        ticks,
        engine_int_min_total_db: engine_min_total_db,
        holes,
        ext_bed_min_total_db: ext_bed_min_total_db,
        ext_bed_holes,
        max_voices,
        one_shot_count,
    }
}

#[test]
fn golden_sweep_is_deterministic_and_within_voice_budget() {
    let a = run_sweep(1.0, "on");
    let b = run_sweep(1.0, "on");
    let a_json = serde_json::to_vec(&a.ticks).expect("serialize a");
    let b_json = serde_json::to_vec(&b.ticks).expect("serialize b");
    assert_eq!(a_json, b_json, "same telemetry must produce the same command sequence");
    assert!(
        a.max_voices <= 48,
        "continuous voice budget exceeded: {}",
        a.max_voices
    );
    assert_eq!(a.one_shot_count, 0, "fixed gear sweep must not emit one-shots");
    eprintln!(
        "[golden] on-sweep engine_int min total {:.1} dB; {} hole(s); max voices {}",
        a.engine_int_min_total_db,
        a.holes.len(),
        a.max_voices
    );
}

/// Fase 4: transmission band coverage over the full drivetrain_speed domain
/// (0..350 km/h, the bank's window domain): gt3r mid 0..~152, z4gt3 midhigh
/// ~143..~177, z4gt3 high ~172..350 — no gaps.
#[test]
fn transmission_windows_cover_full_drivetrain_domain() {
    let mut adapter = CommonV10BankAdapter::default();
    let mut speed = 5.0_f32; // ~standstill (< 0.015 gating) is intentionally silent
    let mut holes = Vec::new();
    while speed <= 350.0 + f32::EPSILON {
        let mut t = base_telemetry(9000.0, 1.0, 1);
        t.drivetrain_speed_kph = speed;
        t.speed_kph = speed;
        let frame = adapter.adapt(t, AudioBackend::CommonV10Commands);
        let linear: f32 = frame
            .voices
            .iter()
            .filter(|v| v.id.starts_with("v10.transmission."))
            .map(|v| {
                let g = 10.0_f32.powf(v.gain_db / 20.0);
                g * g
            })
            .sum();
        let db = if linear > 0.0 { 10.0 * linear.log10() } else { -80.0 };
        if db < -12.0 {
            holes.push(format!("{:.0}km/h={:.1}dB", speed, db));
        }
        speed += 5.0;
    }
    assert!(
        holes.is_empty(),
        "transmission coverage holes: {}",
        holes.join(", ")
    );
    eprintln!("[golden] transmission domain 5..350 km/h covered (no holes)");
}

/// Fase 1 gate: the bank-exact windows (catalog.v2.json) must leave NO
/// interior gain hole deeper than HOLE_THRESHOLD_DB across the full RPM sweep,
/// both on and off throttle (the approximation tables failed this: idle was
/// silent and the 15.6k..17.4k lift-off band was dead).
#[test]
fn engine_int_sweep_has_no_gain_holes_on_or_off_throttle() {
    let on = run_sweep(1.0, "on");
    let off = run_sweep(0.0, "off");
    let report = |h: &[HoleReport]| {
        let parts = h
            .iter()
            .map(|x| format!("{:.0}rpm={:.1}dB", x.rpm, x.engine_int_total_db))
            .collect::<Vec<_>>()
            .join(", ");
        format!("{}", if parts.is_empty() { "<none>".to_string() } else { parts })
    };
    assert!(
        on.holes.is_empty(),
        "engine_int ON sweep has gain holes (>{:.0} dB): {}",
        -HOLE_THRESHOLD_DB,
        report(&on.holes)
    );
    assert!(
        off.holes.is_empty(),
        "engine_int OFF sweep has gain holes (>{:.0} dB): {}",
        -HOLE_THRESHOLD_DB,
        report(&off.holes)
    );
    eprintln!(
        "[golden] int on {:.1} dB / off {:.1} dB; max voices on={} off={}",
        on.engine_int_min_total_db,
        off.engine_int_min_total_db,
        on.max_voices,
        off.max_voices
    );
}

/// Fase 3 gate: exterior bed layers (idle/off_*) at chase-cam distance must
/// cover the full RPM range without holes deeper than the threshold.
#[test]
fn engine_ext_bed_sweep_has_no_gain_holes() {
    let on = run_sweep_with_listener(1.0, "on", 10.0);
    let off = run_sweep_with_listener(0.0, "off", 10.0);
    assert!(
        on.ext_bed_holes.is_empty(),
        "engine_ext BED ON holes: {}",
        on.ext_bed_holes
            .iter()
            .map(|x| format!("{:.0}rpm={:.1}dB", x.rpm, x.engine_int_total_db))
            .collect::<Vec<_>>()
            .join(", ")
    );
    assert!(
        off.ext_bed_holes.is_empty(),
        "engine_ext BED OFF holes: {}",
        off.ext_bed_holes
            .iter()
            .map(|x| format!("{:.0}rpm={:.1}dB", x.rpm, x.engine_int_total_db))
            .collect::<Vec<_>>()
            .join(", ")
    );
    eprintln!(
        "[golden] ext bed (10 m): on min {:.1} dB, off min {:.1} dB",
        on.ext_bed_min_total_db, off.ext_bed_min_total_db
    );
}

#[test]
fn golden_trace_writes_file_when_requested() {
    let Some(path) = std::env::var_os("F90_GOLDEN_TRACE_OUT") else {
        return;
    };
    let trace = GoldenTrace {
        schema: "golden-trace-v1",
        backend: "common_v10_commands",
        rpm_min: RPM_MIN,
        rpm_max: RPM_MAX,
        rpm_step: RPM_STEP,
        sweeps: vec![
            run_sweep(1.0, "on"),
            run_sweep(0.0, "off"),
            run_sweep_with_listener(1.0, "on_ext_10m", 10.0),
            run_sweep_with_listener(0.0, "off_ext_10m", 10.0),
        ],
    };
    let json = serde_json::to_string_pretty(&trace).expect("serialize trace");
    let mut f = std::fs::File::create(&path).expect("create trace file");
    f.write_all(json.as_bytes()).expect("write trace");
    eprintln!("[golden] wrote {}", path.to_string_lossy());
}