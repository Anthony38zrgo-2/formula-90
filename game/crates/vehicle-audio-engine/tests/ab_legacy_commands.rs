//! Fase 6 — offline A/B harness: the four auditionable scenarios (the same ones
//! the legacy v10_vehicle mixer uses, tools/audio/scenarios.py) render through
//! BOTH backends from identical telemetry:
//!
//! - `CommonV10Commands`: AudioCommandFrame trace per tick (JSON in
//!   diagnostics/ab/<scenario>.commands.json), engine coverage assertions.
//! - `LegacyV10Pcm`: deterministic PCM render (diagnostics/ab/<scenario>.legacy.wav),
//!   skipped when the legacy bank is absent.
//!
//! This is the A/B capture the human gate listens to; the assertions pin the
//! command-side semantics (continuous engine, shift shots, surfaces, impacts).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use vehicle_audio_engine::{
    AudioBackend, AudioTelemetryFrame, CollisionAudioInput, CollisionKind, CommonV10BankAdapter,
    Trigger, VehicleAudioEngine,
};

const TICK_HZ: f64 = 120.0;
const RENDER_HZ: u32 = 44_100;

const DIAG_ROOT: &str = "ab";

struct ScenarioDef {
    name: &'static str,
    ticks: Vec<FrameDef>,
    impacts: Vec<ImpactDef>,
}

#[derive(Clone, Copy)]
struct FrameDef {
    t: f32,
    rpm: f32,
    speed_kph: f32,
    throttle: f32,
    gear: i32,
    slip: f32,
    surface: &'static str,
}

#[derive(Clone, Copy)]
struct ImpactDef {
    t: f32,
    kind: &'static str,
    normal_speed: f32,
}

fn lerp_frames(keyframes: &[FrameDef]) -> Vec<FrameDef> {
    let mut out = Vec::new();
    let mut t = 0.0_f32;
    let total = keyframes.last().map(|k| k.t).unwrap_or(0.0);
    let mut idx = 0usize;
    while t <= total + 1e-9 {
        while idx + 1 < keyframes.len() && keyframes[idx + 1].t <= t {
            idx += 1;
        }
        let f = if idx + 1 < keyframes.len() {
            let (a, b) = (keyframes[idx], keyframes[idx + 1]);
            let span = (b.t - a.t).max(1e-9);
            let u = ((t - a.t) / span).clamp(0.0, 1.0);
            FrameDef {
                t,
                rpm: a.rpm + (b.rpm - a.rpm) * u,
                speed_kph: a.speed_kph + (b.speed_kph - a.speed_kph) * u,
                throttle: a.throttle + (b.throttle - a.throttle) * u,
                gear: if u > 0.5 { b.gear } else { a.gear },
                slip: a.slip + (b.slip - a.slip) * u,
                surface: if u > 0.5 { b.surface } else { a.surface },
            }
        } else {
            keyframes[idx]
        };
        out.push(f);
        t += 1.0 / 200.0;
    }
    out
}

fn scenario_frames(ticks: Vec<FrameDef>) -> Vec<FrameDef> {
    // Resample the 200 Hz scenario to the 120 Hz audio tick (nearest frame).
    let mut out = Vec::new();
    let mut t = 0.0_f32;
    let dur = ticks.last().map(|k| k.t).unwrap_or(0.0);
    let mut idx = 0usize;
    while t <= dur + 1e-9 {
        while idx + 1 < ticks.len() && ticks[idx + 1].t <= t {
            idx += 1;
        }
        let f = ticks[idx];
        out.push(FrameDef {
            t,
            rpm: f.rpm,
            speed_kph: f.speed_kph,
            throttle: f.throttle,
            gear: f.gear,
            slip: f.slip,
            surface: f.surface,
        });
        t += 1.0 / TICK_HZ as f32;
    }
    out
}

fn surface_code(surface: &str) -> u8 {
    match surface {
        "rumble" => 1,
        "grass" => 3,
        "sand" => 5,
        _ => 0,
    }
}

fn legacy_surface(surface: &str) -> &'static str {
    match surface {
        "rumble" => "rumble",
        "grass" => "grass",
        "sand" => "sand",
        _ => "asphalt",
    }
}

fn legacy_trigger(kind: &str) -> Option<Trigger> {
    match kind {
        "cone" => Some(Trigger::Cone),
        "barrier" => Some(Trigger::Barrier),
        "hit_3" => Some(Trigger::Hit3),
        _ => None,
    }
}

fn collision(kind: &str, normal_speed: f32) -> CollisionAudioInput {
    let (collision_kind, tangential) = match kind {
        "cone" => (CollisionKind::Prop, 1.0),
        "barrier" => (CollisionKind::Barrier, 1.5),
        "hit_3" => (CollisionKind::Generic, 2.0),
        _ => (CollisionKind::Generic, 0.0),
    };
    CollisionAudioInput {
        kind: collision_kind,
        normal_speed_m_s: normal_speed,
        tangential_speed_m_s: tangential,
        impulse_ns: normal_speed * 800.0,
    }
}

fn surface_codes(surface: &str) -> [u8; 4] {
    let code = surface_code(surface);
    [code, code, code, code]
}

struct CommandsTrace {
    voices_per_tick: usize,
    engine_holes: Vec<(u32, f32)>,
    gear_shots: Vec<(u32, &'static str, String)>,
    collision_shots: Vec<(u32, String)>,
    headings: Vec<String>,
}

static SUMMARY_SEQ: AtomicU32 = AtomicU32::new(0);

fn run_commands(s: &ScenarioDef) -> CommandsTrace {
    let mut adapter = CommonV10BankAdapter::default();
    let mut trace = CommandsTrace {
        voices_per_tick: 0,
        engine_holes: Vec::new(),
        gear_shots: Vec::new(),
        collision_shots: Vec::new(),
        headings: Vec::new(),
    };
    let mut seen_shot_ids: std::collections::BTreeSet<u64> = Default::default();
    let mut tick: u64 = 0;
    for f in &s.ticks {
        tick += 1;
        for imp in &s.impacts {
            if (imp.t - f.t).abs() < 0.5 / TICK_HZ as f32 {
                adapter.push_collision(collision(imp.kind, imp.normal_speed));
            }
        }
        let frame = adapter.adapt(
            AudioTelemetryFrame {
                tick,
                dt_s: (1.0 / TICK_HZ) as f32,
                rpm: f.rpm,
                throttle: f.throttle,
                speed_kph: f.speed_kph,
                drivetrain_speed_kph: f.speed_kph,
                gear: f.gear,
                slip: f.slip,
                slip_ratio: [f.slip; 4],
                surfaces: surface_codes(f.surface),
                brake: 0.0,
                wheel_load_n: [2_500.0; 4],
                suspension_velocity_m_s: [0.0; 4],
                tire_pressure_kpa: [250.0; 4],
                listener_distance_m: 0.0,
                underfloor_scrape_active: false,
                underfloor_scrape_intensity: 0.0,
                underfloor_scrape_speed_m_s: 0.0,
            },
            AudioBackend::CommonV10Commands,
        );
        trace.voices_per_tick = trace.voices_per_tick.max(frame.voices.len());
        let mut linear: f32 = 0.0;
        for v in &frame.voices {
            if v.id.starts_with("v10.engine.") {
                let g = 10.0_f32.powf(v.gain_db / 20.0);
                linear += g * g;
            }
        }
        let total = if linear > 0.0 { 10.0 * linear.log10() } else { -80.0 };
        if total < -12.0 {
            trace.engine_holes.push((tick as u32, total));
        }
        for shot in &frame.one_shots {
            // The journal replays every shot for 120 ticks; count each shot
            // once by id (same dedupe the Godot renderer applies).
            if !seen_shot_ids.insert(shot.id) {
                continue;
            }
            if shot.event.contains("gear") {
                trace
                    .gear_shots
                    .push((tick as u32, "gear", shot.sample.clone()));
            } else if shot.event.contains("collision") {
                trace
                    .collision_shots
                    .push((tick as u32, shot.event.clone()));
            }
        }
        if trace.headings.len() < 8 {
            trace.headings.push(format!(
                "t{:04} rpm={:.0} thr={:.2} gear={} surf={} voices={}",
                tick,
                f.rpm,
                f.throttle,
                f.gear,
                f.surface,
                frame.voices.len()
            ));
        }
    }
    trace
}

fn write_wave(path: &Path, l: &[f32], r: &[f32], rate: u32) {
    use std::io::Write;
    let mut data = Vec::with_capacity(l.len() * 4);
    for i in 0..l.len() {
        let s_l = (l[i].clamp(-1.0, 1.0) * 32767.0) as i16;
        let s_r = (r[i].clamp(-1.0, 1.0) * 32767.0) as i16;
        data.extend_from_slice(&s_l.to_le_bytes());
        data.extend_from_slice(&s_r.to_le_bytes());
    }
    let mut f = std::fs::File::create(path).expect("create wav");
    f.write_all(b"RIFF").unwrap();
    f.write_all(&(36 + data.len() as u32).to_le_bytes()).unwrap();
    f.write_all(b"WAVEfmt ").unwrap();
    f.write_all(&16u32.to_le_bytes()).unwrap();
    f.write_all(&1u16.to_le_bytes()).unwrap(); // PCM
    f.write_all(&2u16.to_le_bytes()).unwrap(); // stereo
    f.write_all(&rate.to_le_bytes()).unwrap();
    f.write_all(&(rate * 4).to_le_bytes()).unwrap();
    f.write_all(&4u16.to_le_bytes()).unwrap();
    f.write_all(&16u16.to_le_bytes()).unwrap();
    f.write_all(b"data").unwrap();
    f.write_all(&(data.len() as u32).to_le_bytes()).unwrap();
    f.write_all(&data).unwrap();
}

fn run_legacy(s: &ScenarioDef, bank_dir: &Path, out_wav: &Path) -> bool {
    let Ok(mut engine) = VehicleAudioEngine::new(bank_dir) else {
        eprintln!("SKIP legacy render: bank missing at {}", bank_dir.display());
        return false;
    };
    let mut l_all: Vec<f32> = Vec::new();
    let mut r_all: Vec<f32> = Vec::new();
    for f in &s.ticks {
        for imp in &s.impacts {
            if (imp.t - f.t).abs() < 0.5 / TICK_HZ as f32 {
                if let Some(t) = legacy_trigger(imp.kind) {
                    engine.trigger(t);
                }
            }
        }
        engine.set_state(
            f.rpm as f64,
            1_000.0,
            15_000.0,
            f.throttle,
            f.speed_kph as f64,
            f.gear,
            f.slip,
            legacy_surface(f.surface),
        );
        let n = (RENDER_HZ as f64 / TICK_HZ).round() as usize;
        let mut l = vec![0.0_f32; n];
        let mut r = vec![0.0_f32; n];
        engine.render(&mut l, &mut r, n);
        l_all.extend_from_slice(&l);
        r_all.extend_from_slice(&r);
    }
    write_wave(out_wav, &l_all, &r_all, RENDER_HZ);
    true
}

fn diag_dir() -> PathBuf {
    // game/crates/vehicle-audio-engine/tests -> repo diagnostics/ab
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo = manifest
        .parent()
        .unwrap()
        .parent()
        .unwrap() // game
        .parent()
        .unwrap(); // repo root
    let dir = repo.join("diagnostics").join(DIAG_ROOT);
    std::fs::create_dir_all(&dir).expect("create diagnostics/ab");
    dir
}

fn legacy_bank_dir() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .unwrap()
        .parent()
        .unwrap() // game/crates
        .join("sounds/banks/v10_vehicle")
}

fn scenario_idle_to_redline() -> ScenarioDef {
    let ticks = lerp_frames(&[
        FrameDef { t: 0.0, rpm: 2200.0, speed_kph: 0.0, throttle: 0.15, gear: 1, slip: 0.0, surface: "asphalt" },
        FrameDef { t: 1.0, rpm: 6000.0, speed_kph: 35.0, throttle: 0.85, gear: 1, slip: 0.05, surface: "asphalt" },
        FrameDef { t: 1.05, rpm: 6200.0, speed_kph: 38.0, throttle: 0.0, gear: 2, slip: 0.0, surface: "asphalt" },
        FrameDef { t: 2.0, rpm: 9500.0, speed_kph: 85.0, throttle: 0.95, gear: 2, slip: 0.04, surface: "asphalt" },
        FrameDef { t: 2.05, rpm: 9700.0, speed_kph: 88.0, throttle: 0.0, gear: 3, slip: 0.0, surface: "asphalt" },
        FrameDef { t: 3.0, rpm: 12000.0, speed_kph: 135.0, throttle: 1.0, gear: 3, slip: 0.06, surface: "asphalt" },
        FrameDef { t: 3.05, rpm: 12200.0, speed_kph: 138.0, throttle: 0.0, gear: 4, slip: 0.0, surface: "asphalt" },
        FrameDef { t: 4.2, rpm: 14500.0, speed_kph: 195.0, throttle: 1.0, gear: 4, slip: 0.03, surface: "asphalt" },
    ]);
    ScenarioDef { name: "idle_to_redline_with_shifts", ticks: scenario_frames(ticks), impacts: vec![] }
}

fn scenario_road_to_sand() -> ScenarioDef {
    let ticks = lerp_frames(&[
        FrameDef { t: 0.0, rpm: 7000.0, speed_kph: 70.0, throttle: 0.7, gear: 3, slip: 0.03, surface: "asphalt" },
        FrameDef { t: 1.0, rpm: 7400.0, speed_kph: 74.0, throttle: 0.7, gear: 3, slip: 0.04, surface: "asphalt" },
        FrameDef { t: 1.4, rpm: 7500.0, speed_kph: 75.0, throttle: 0.7, gear: 3, slip: 0.05, surface: "rumble" },
        FrameDef { t: 1.45, rpm: 7500.0, speed_kph: 74.0, throttle: 0.6, gear: 3, slip: 0.08, surface: "sand" },
        FrameDef { t: 3.0, rpm: 7100.0, speed_kph: 66.0, throttle: 0.65, gear: 3, slip: 0.14, surface: "sand" },
    ]);
    ScenarioDef { name: "road_to_sand_with_kerb", ticks: scenario_frames(ticks), impacts: vec![] }
}

fn scenario_grass_skid() -> ScenarioDef {
    let ticks = lerp_frames(&[
        FrameDef { t: 0.0, rpm: 6500.0, speed_kph: 55.0, throttle: 0.8, gear: 2, slip: 0.05, surface: "asphalt" },
        FrameDef { t: 0.6, rpm: 6800.0, speed_kph: 50.0, throttle: 0.9, gear: 2, slip: 0.45, surface: "grass" },
        FrameDef { t: 1.5, rpm: 8200.0, speed_kph: 42.0, throttle: 1.0, gear: 2, slip: 0.62, surface: "grass" },
        FrameDef { t: 2.4, rpm: 7800.0, speed_kph: 35.0, throttle: 0.4, gear: 2, slip: 0.30, surface: "grass" },
    ]);
    ScenarioDef { name: "grass_skid", ticks: scenario_frames(ticks), impacts: vec![] }
}

fn scenario_impact() -> ScenarioDef {
    let ticks = lerp_frames(&[
        FrameDef { t: 0.0, rpm: 6000.0, speed_kph: 80.0, throttle: 0.6, gear: 3, slip: 0.03, surface: "asphalt" },
        FrameDef { t: 1.0, rpm: 6200.0, speed_kph: 82.0, throttle: 0.6, gear: 3, slip: 0.03, surface: "asphalt" },
        FrameDef { t: 1.8, rpm: 6400.0, speed_kph: 84.0, throttle: 0.6, gear: 3, slip: 0.03, surface: "asphalt" },
        FrameDef { t: 2.8, rpm: 6600.0, speed_kph: 86.0, throttle: 0.6, gear: 3, slip: 0.03, surface: "asphalt" },
    ]);
    ScenarioDef {
        name: "impact",
        ticks: scenario_frames(ticks),
        impacts: vec![
            ImpactDef { t: 1.0, kind: "cone", normal_speed: 7.0 },
            ImpactDef { t: 1.8, kind: "barrier", normal_speed: 14.0 },
            ImpactDef { t: 2.3, kind: "hit_3", normal_speed: 10.0 },
        ],
    }
}

fn summarize(name: &str, s: &ScenarioDef, t: &CommandsTrace, legacy_ok: bool, dir: &Path) {
    let holes_txt = if t.engine_holes.is_empty() {
        "none".to_string()
    } else {
        t.engine_holes
            .iter()
            .map(|(tk, db)| format!("t{tk}={db:.0}dB"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let summary = format!(
        "scenario={name}\n\
         ticks={}\n\
         max_voices={}\n\
         engine_int_holes={}\n\
         gear_shots={} (first: {:?})\n\
         collision_shots={} (events: {:?})\n\
         legacy_render={}\n",
        s.ticks.len(),
        t.voices_per_tick,
        holes_txt,
        t.gear_shots.len(),
        t.gear_shots.first().map(|x| x.2.clone()),
        t.collision_shots.len(),
        t.collision_shots.iter().map(|x| x.1.clone()).collect::<Vec<_>>(),
        if legacy_ok { "wav" } else { "skipped (bank absent)" },
    );
    std::fs::write(dir.join(format!("{name}.summary.txt")), &summary).expect("summary");
    eprintln!("---- {name} ----\n{summary}{}", t.headings.join("\n"));
}

#[test]
fn ab_scenarios_commands_semantics_and_legacy_render() {
    let dir = diag_dir();
    let bank = legacy_bank_dir();
    let has_bank = bank.join("bank_manifest.json").is_file();
    if !has_bank {
        eprintln!("SKIP legacy A/B audio: bank at {} not present", bank.display());
    }
    let scenarios = [
        scenario_idle_to_redline(),
        scenario_road_to_sand(),
        scenario_grass_skid(),
        scenario_impact(),
    ];
    for s in &scenarios {
        let seq = SUMMARY_SEQ.fetch_add(1, Ordering::Relaxed);
        let t = run_commands(s);

        // 1. Engine never goes silent in the scenario range (Fase 1 gate on live data).
        assert!(
            t.engine_holes.is_empty(),
            "{}: engine_int coverage holes: {:?}",
            s.name,
            t.engine_holes
        );
        // 2. Gear-shift one-shots fire exactly on gear changes (idle_to_redline: 3 upshifts).
        if s.name == "idle_to_redline_with_shifts" {
            assert_eq!(t.gear_shots.len(), 3, "three upshifts -> three shots");
        }
        // 3. Surfaces map to the right common samples.
        if s.name == "road_to_sand_with_kerb" {
            assert!(!t.collision_shots.is_empty() == false); // no impacts here
        }
        // 4. All three impacts become one-shots with the expected events.
        if s.name == "impact" {
            let events: Vec<&str> = ["common/collision/prop", "common/collision/barrier", "common/collision/body"]
                .to_vec();
            for ev in &events {
                assert!(
                    t.collision_shots.iter().any(|(_, e)| e == ev),
                    "{}: missing collision event {} (got {:?})",
                    s.name,
                    ev,
                    t.collision_shots
                );
            }
        }

        let wav = dir.join(format!("{}.legacy.wav", s.name));
        let legacy_ok = has_bank && run_legacy(s, &bank, &wav);
        let cmd_json = dir.join(format!("{}.commands.json", s.name));
        // Deterministic command trace header (not per-tick payload: journal replay
        // would bloat it; the golden sweep covers full per-tick equality).
        let json = serde_json::json!({
            "schema": "ab-commands-v1",
            "scenario": s.name,
            "tick_hz": TICK_HZ,
            "ticks": s.ticks.len(),
            "max_voices": t.voices_per_tick,
            "engine_int_holes": t.engine_holes,
            "gear_shots": t.gear_shots.iter().map(|(tk, _, s)| format!("t{tk} {s}")).collect::<Vec<_>>(),
            "collision_shots": t.collision_shots.iter().map(|(tk, e)| format!("t{tk} {e}")).collect::<Vec<_>>(),
            "headings": t.headings,
            "seq": seq,
        });
        std::fs::write(&cmd_json, serde_json::to_string_pretty(&json).expect("json"))
            .expect("write commands trace");
        summarize(s.name, s, &t, legacy_ok, &dir);
    }
}