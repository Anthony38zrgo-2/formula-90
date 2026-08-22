//! Fase 6 — offline A/B replay of a LIVE recorded driving session.
//!
//! Usage: cargo run -p vehicle_audio_engine --example live_replay -- <telemetry.csv>
//!
//! The CSV is produced at runtime by `f90_core_audio_set_record_path`
//! (renderer export `debug_record_path`): one line per 120 Hz audio tick with
//! the exact adapter inputs. The SAME lines render through BOTH backends:
//!   diagnostics/ab/live_<stem>.legacy.wav      (LegacyV10Pcm, PCM render)
//!   diagnostics/ab/live_<stem>.commands.json   (CommonV10Commands trace)
//!
//! This is the A/B pair the human gate listens to ("misma telemetría produce
//! la misma secuencia", handoff section 6/10).

use std::path::{Path, PathBuf};

use vehicle_audio_engine::{
    AudioBackend, AudioTelemetryFrame, CommonV10BankAdapter, VehicleAudioEngine,
};

const TICK_HZ: f64 = 120.0;
const RENDER_HZ: u32 = 44_100;

#[derive(Default, Clone)]
struct Tick {
    tick: u64,
    rpm: f32,
    throttle: f32,
    speed_kph: f32,
    drivetrain_speed_kph: f32,
    gear: i32,
    slip: f32,
    slip_ratio: [f32; 4],
    surfaces: [u8; 4],
    brake: f32,
    wheel_load_n: [f32; 4],
    suspension_velocity_m_s: [f32; 4],
    tire_pressure_kpa: [f32; 4],
    listener_distance_m: f32,
}

fn parse_csv(path: &Path) -> Result<Vec<Tick>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for (lineno, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let p: Vec<&str> = line.split(',').collect();
        if p.len() != 29 {
            return Err(format!("line {}: expected 29 fields, got {}", lineno + 1, p.len()));
        }
        let num = |i: usize| -> f32 {
            p[i].trim().parse::<f32>().unwrap_or(0.0)
        };
        let mut t = Tick {
            tick: p[0].trim().parse().unwrap_or(0),
            rpm: num(1),
            throttle: num(2),
            speed_kph: num(3),
            drivetrain_speed_kph: num(4),
            gear: p[5].trim().parse().unwrap_or(0),
            slip: num(6),
            ..Default::default()
        };
        for (i, slot) in t.slip_ratio.iter_mut().enumerate() {
            *slot = num(7 + i);
        }
        for (i, slot) in t.surfaces.iter_mut().enumerate() {
            *slot = num(11 + i) as u8;
        }
        t.brake = num(15);
        for (i, slot) in t.wheel_load_n.iter_mut().enumerate() {
            *slot = num(16 + i);
        }
        for (i, slot) in t.suspension_velocity_m_s.iter_mut().enumerate() {
            *slot = num(20 + i);
        }
        for (i, slot) in t.tire_pressure_kpa.iter_mut().enumerate() {
            *slot = num(24 + i);
        }
        t.listener_distance_m = num(28);
        out.push(t);
    }
    if out.is_empty() {
        return Err("no ticks parsed".into());
    }
    Ok(out)
}

fn to_frame(t: &Tick) -> AudioTelemetryFrame {
    AudioTelemetryFrame {
        tick: t.tick,
        dt_s: (1.0 / TICK_HZ) as f32,
        rpm: t.rpm,
        throttle: t.throttle,
        speed_kph: t.speed_kph,
        drivetrain_speed_kph: t.drivetrain_speed_kph,
        gear: t.gear,
        slip: t.slip,
        slip_ratio: t.slip_ratio,
        surfaces: t.surfaces,
        brake: t.brake,
        wheel_load_n: t.wheel_load_n,
        suspension_velocity_m_s: t.suspension_velocity_m_s,
        tire_pressure_kpa: t.tire_pressure_kpa,
        listener_distance_m: t.listener_distance_m,
        underfloor_scrape_active: false,
        underfloor_scrape_intensity: 0.0,
        underfloor_scrape_speed_m_s: 0.0,
    }
}

fn write_wave(path: &Path, l: &[f32], r: &[f32]) {
    use std::io::Write;
    let mut data = Vec::with_capacity(l.len() * 4);
    for i in 0..l.len() {
        data.extend_from_slice(&((l[i].clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
        data.extend_from_slice(&((r[i].clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    let mut f = std::fs::File::create(path).expect("create wav");
    f.write_all(b"RIFF").unwrap();
    f.write_all(&(36 + data.len() as u32).to_le_bytes()).unwrap();
    f.write_all(b"WAVEfmt ").unwrap();
    f.write_all(&16u32.to_le_bytes()).unwrap();
    f.write_all(&1u16.to_le_bytes()).unwrap();
    f.write_all(&2u16.to_le_bytes()).unwrap();
    f.write_all(&RENDER_HZ.to_le_bytes()).unwrap();
    f.write_all(&(RENDER_HZ * 4).to_le_bytes()).unwrap();
    f.write_all(&4u16.to_le_bytes()).unwrap();
    f.write_all(&16u16.to_le_bytes()).unwrap();
    f.write_all(b"data").unwrap();
    f.write_all(&(data.len() as u32).to_le_bytes()).unwrap();
    f.write_all(&data).unwrap();
}

fn legacy_bank_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("sounds/banks/v10_vehicle")
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: cargo run -p vehicle_audio_engine --example live_replay -- <telemetry.csv>");
        std::process::exit(2);
    }
    let csv = PathBuf::from(&args[1]);
    let ticks = parse_csv(&csv).unwrap_or_else(|e| {
        eprintln!("parse error: {e}");
        std::process::exit(2)
    });
    let stem = csv
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "live".into());
    let out_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("diagnostics/ab");
    std::fs::create_dir_all(&out_dir).expect("create out dir");

    // --- commands side -------------------------------------------------------
    let mut adapter = CommonV10BankAdapter::default();
    let mut holes: Vec<(u64, f32)> = Vec::new();
    let mut max_voices = 0usize;
    let debug = std::env::var("F90_REPLAY_DEBUG").is_ok();
    for (i, t) in ticks.iter().enumerate() {
        let frame = adapter.adapt(to_frame(t), AudioBackend::CommonV10Commands);
        max_voices = max_voices.max(frame.voices.len());
        // The vehicle is never silent: the interior/exterior crossfade is a
        // camera choice, so a "hole" means BOTH sides are quiet.
        let mut int_linear = 0.0_f32;
        let mut ext_linear = 0.0_f32;
        for v in &frame.voices {
            if v.id.starts_with("v10.engine.") {
                let g = 10.0_f32.powf(v.gain_db / 20.0);
                int_linear += g * g;
            } else if v.id.starts_with("v10.engine_ext.") {
                let g = 10.0_f32.powf(v.gain_db / 20.0);
                ext_linear += g * g;
            }
        }
        let int_db = if int_linear > 0.0 { 10.0 * int_linear.log10() } else { -80.0 };
        let ext_db = if ext_linear > 0.0 { 10.0 * ext_linear.log10() } else { -80.0 };
        let db = int_db.max(ext_db);
        if debug && i < 3 {
            let engine: Vec<(String, f32)> = frame
                .voices
                .iter()
                .filter(|v| v.id.starts_with("v10.engine."))
                .map(|v| (v.id.clone(), v.gain_db))
                .collect();
            eprintln!(
                "DBG tick={} rpm={} thr={} gear={} listener={} surf={:?} -> holes?{} engine={:?}",
                t.tick, t.rpm, t.throttle, t.gear, t.listener_distance_m, t.surfaces, db, engine
            );
        }
        if debug && db < -12.0 && holes.iter().filter(|(tk, _)| *tk > t.tick.saturating_sub(2)).count() == 0 {
            let engine: Vec<(String, f32)> = frame
                .voices
                .iter()
                .filter(|v| v.id.starts_with("v10.engine."))
                .map(|v| (v.id.clone(), v.gain_db))
                .collect();
            eprintln!(
                "DBG-HOLE tick={} rpm={} thr={} gear={} speed={} dt_speed={} listener={} -> {} dB engine={:?}",
                t.tick, t.rpm, t.throttle, t.gear, t.speed_kph, t.drivetrain_speed_kph, t.listener_distance_m, db, engine
            );
        }
        if db < -12.0 {
            holes.push((t.tick, db));
        }
    }
    let trace = serde_json::json!({
        "schema": "live-ab-commands-v1",
        "source": csv.to_string_lossy(),
        "ticks": ticks.len(),
        "max_voices": max_voices,
        "engine_int_holes": holes.iter().map(|(tk, db)| format!("t{tk}={db:.1}dB")).collect::<Vec<_>>(),
    });
    let json_path = out_dir.join(format!("live_{stem}.commands.json"));
    std::fs::write(&json_path, serde_json::to_string_pretty(&trace).unwrap()).expect("json");

    // --- legacy side ----------------------------------------------------------
    let bank = legacy_bank_dir();
    let legacy_ok = match VehicleAudioEngine::new(&bank) {
        Ok(mut engine) => {
            let mut l_all: Vec<f32> = Vec::new();
            let mut r_all: Vec<f32> = Vec::new();
            for t in &ticks {
                engine.set_state(
                    t.rpm as f64,
                    1_000.0,
                    15_000.0,
                    t.throttle,
                    t.speed_kph as f64,
                    t.gear,
                    t.slip,
                    match t.surfaces.iter().copied().max().unwrap_or(0) {
                        1 => "rumble",
                        3 => "grass",
                        5 => "sand",
                        _ => "asphalt",
                    },
                );
                let n = (RENDER_HZ as f64 / TICK_HZ).round() as usize;
                let mut l = vec![0.0_f32; n];
                let mut r = vec![0.0_f32; n];
                engine.render(&mut l, &mut r, n);
                l_all.extend_from_slice(&l);
                r_all.extend_from_slice(&r);
            }
            let wav = out_dir.join(format!("live_{stem}.legacy.wav"));
            write_wave(&wav, &l_all, &r_all);
            true
        }
        Err(e) => {
            eprintln!("legacy bank unavailable ({}): legacy WAV skipped", e);
            false
        }
    };

    println!(
        "live A/B replay: {} ticks from {}\n  commands: {}\n  legacy: {} (wav={})\n  engine_int_holes={}",
        ticks.len(),
        csv.display(),
        json_path.display(),
        legacy_ok,
        if legacy_ok { "yes" } else { "no" },
        holes.len()
    );
    if !holes.is_empty() {
        println!("  first holes: {}", holes.iter().take(5).map(|(tk, db)| format!("t{tk}={db:.1}dB")).collect::<Vec<_>>().join(", "));
    }
}