//! Replay/parity telemetry for the audio core from real vehicle CSV sessions.
//!
//! Reads the CSV produced by `telemetry_manager.gd` (25 columns), feeds each
//! row through `VehicleAudioState::mix()` (+ surface/trigger derivation) and
//! produces per-frame `MixFrame` + a `Summary`. No Godot deps, deterministic.

use serde::{Deserialize, Serialize};

use crate::state::{engine_pitch_scale, StateInput, VehicleAudioState};

/// One row of vehicle telemetry relevant to audio.
#[derive(Debug, Clone, PartialEq)]
pub struct TelemetryRow {
    pub time_ms: u64,
    pub speed_kph: f64,
    pub rpm: f64,
    pub gear: i32,
    pub throttle: f32,
    pub slip: f32,
    pub raw_surface: String,
}

/// Result of replaying one row through the audio mix core.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MixFrame {
    pub time_ms: u64,
    pub rpm: f64,
    pub normalized_rpm: f32,
    pub gear: i32,
    pub throttle: f32,
    pub slip: f32,
    pub surface: String,
    pub weights: [f32; 5],
    pub pitch_scales: [f32; 5],
    pub engine_gain: f32,
    pub surface_key: Option<String>,
    pub surface_gain: f32,
    pub trigger: Option<String>,
    pub dominant_band: usize,
    pub crossfade_active: bool,
}

/// Aggregated stats for a session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Summary {
    pub rows: usize,
    pub rpm_min: f64,
    pub rpm_max: f64,
    pub time_shares: [f32; 5],
    pub pitch_min: [f32; 5],
    pub pitch_mean: [f32; 5],
    pub pitch_max: [f32; 5],
    pub clamp_low_hits: [usize; 5],
    pub clamp_high_hits: [usize; 5],
    pub crossfade_share: f32,
    pub shift_up: usize,
    pub shift_down: usize,
}

/// RFC4180-lite line splitter: respects quotes and doubled quotes (`""`).
pub fn split_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '"' {
            if in_quotes {
                if chars.peek() == Some(&'"') {
                    cur.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                in_quotes = true;
            }
        } else if ch == ',' && !in_quotes {
            fields.push(cur);
            cur = String::new();
        } else {
            cur.push(ch);
        }
    }
    fields.push(cur);
    fields
}

fn parse_f64(s: &str) -> f64 {
    s.trim().parse::<f64>().unwrap_or(0.0)
}
fn parse_f32(s: &str) -> f32 {
    s.trim().parse::<f32>().unwrap_or(0.0)
}
fn parse_i32(s: &str) -> i32 {
    // Gear column may be "N", float-like, or integer.
    let t = s.trim();
    if t.eq_ignore_ascii_case("n") || t.is_empty() {
        return 0;
    }
    if let Ok(v) = t.parse::<i32>() {
        return v;
    }
    if let Ok(v) = t.parse::<f64>() {
        return v as i32;
    }
    0
}

// Header column names (from telemetry_manager.gd); kept for reference, not enforced strictly.
#[allow(dead_code)]
const EXPECTED_HEADER: [&str; 25] = [
    "Time_ms",
    "Speed_kmh",
    "RPM",
    "Gear",
    "Throttle",
    "Brake",
    "Steering",
    "Lat_G",
    "Long_G",
    "FL_Comp",
    "FR_Comp",
    "RL_Comp",
    "RR_Comp",
    "Front_Slip",
    "Rear_Slip",
    "Session_Id",
    "Session_Timestamp_UTC",
    "Physics_Hz",
    "Test_Id",
    "Track_Scene",
    "Vehicle_Node_Path",
    "Vehicle_Scene",
    "Vehicle_Script",
    "Setup_Schema_Version",
    "Setup_JSON",
];

/// Parse a telemetry CSV string (header + rows) into `TelemetryRow`s.
pub fn parse_rows(csv: &str) -> Vec<TelemetryRow> {
    let mut rows = Vec::new();
    for (idx, line) in csv.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let fields = split_csv_line(line);
        if idx == 0 {
            // Optional header sanity (lenient: accept superset/subset).
            continue;
        }
        // Defensive: need at least the 15 numeric columns we consume.
        if fields.len() < 15 {
            continue;
        }
        let rpm = parse_f64(&fields[2]);
        // Slip: max of front/rear.
        let front_slip = parse_f32(&fields[13]);
        let rear_slip = parse_f32(&fields[14]);
        let slip = front_slip.max(rear_slip).clamp(0.0, 1.0);
        rows.push(TelemetryRow {
            time_ms: fields[0].trim().parse::<u64>().unwrap_or(0),
            speed_kph: parse_f64(&fields[1]),
            rpm,
            gear: parse_i32(&fields[3]),
            throttle: parse_f32(&fields[4]),
            slip,
            raw_surface: String::new(),
        });
    }
    rows
}

/// Replay rows through the audio core, deriving per-frame mixes.
/// `idle_rpm`/`max_rpm` come from the session setup snapshot; defaults 4500/17000.
pub fn replay(rows: &[TelemetryRow], idle_rpm: f64, max_rpm: f64) -> Vec<MixFrame> {
    let mut out = Vec::with_capacity(rows.len());
    let mut prev_gear: Option<i32> = None;
    for row in rows {
        let state = VehicleAudioState::from_input(&StateInput {
            rpm: row.rpm,
            idle_rpm,
            max_rpm,
            throttle: row.throttle,
            speed_kph: row.speed_kph,
            gear: row.gear,
            slip: row.slip,
            surface: "asphalt",
        });
        let mix = state.mix();
        // Derive trigger from gear transitions (as the runtime controller does).
        let trigger = match prev_gear {
            Some(pg) if pg != 0 && row.gear != pg => Some(
                if row.gear > pg {
                    "shift_up"
                } else {
                    "shift_down"
                }
                .to_string(),
            ),
            _ => None,
        };
        prev_gear = Some(row.gear);
        let dominant = mix
            .engine_weights
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(0);
        let crossfade = mix.engine_weights.iter().filter(|&&w| w > 0.1).count() >= 2;
        // Per-band pitch for this rpm
        let mut pitches = [1.0f32; 5];
        for (i, p) in pitches.iter_mut().enumerate() {
            *p = engine_pitch_scale(row.rpm, i);
        }
        out.push(MixFrame {
            time_ms: row.time_ms,
            rpm: row.rpm,
            normalized_rpm: state.normalized_rpm,
            gear: row.gear,
            throttle: row.throttle,
            slip: row.slip,
            surface: "asphalt".to_string(),
            weights: mix.engine_weights,
            pitch_scales: pitches,
            engine_gain: mix.engine_gain,
            surface_key: mix.surface_key.map(|s| s.to_string()),
            surface_gain: mix.surface_gain,
            trigger,
            dominant_band: dominant,
            crossfade_active: crossfade,
        });
    }
    out
}

pub fn summarize(frames: &[MixFrame]) -> Summary {
    if frames.is_empty() {
        return Summary {
            rows: 0,
            rpm_min: 0.0,
            rpm_max: 0.0,
            time_shares: [0.0; 5],
            pitch_min: [1.0; 5],
            pitch_mean: [1.0; 5],
            pitch_max: [1.0; 5],
            clamp_low_hits: [0; 5],
            clamp_high_hits: [0; 5],
            crossfade_share: 0.0,
            shift_up: 0,
            shift_down: 0,
        };
    }
    let rpm_min = frames.iter().map(|f| f.rpm).fold(f64::INFINITY, f64::min);
    let rpm_max = frames
        .iter()
        .map(|f| f.rpm)
        .fold(f64::NEG_INFINITY, f64::max);
    let mut counts = [0usize; 5];
    let mut pitch_min = [f32::INFINITY; 5];
    let mut pitch_max = [f32::NEG_INFINITY; 5];
    let mut pitch_sum = [0.0f32; 5];
    let mut clamp_low = [0usize; 5];
    let mut clamp_high = [0usize; 5];
    let mut cross = 0usize;
    let mut up = 0usize;
    let mut down = 0usize;
    // Weight threshold: only bands that are actually audible contribute to
    // pitch/clamp stats (a muted band would otherwise inflate means and count
    // clamp hits for a silent layer).
    const AUDIBLE_WEIGHT: f32 = 0.05;

    fn audible_count(frames: &[MixFrame], band: usize) -> usize {
        frames
            .iter()
            .filter(|f| f.weights[band] > AUDIBLE_WEIGHT)
            .count()
    }
    for f in frames {
        counts[f.dominant_band] += 1;
        for i in 0..5 {
            let p = f.pitch_scales[i];
            if f.weights[i] <= AUDIBLE_WEIGHT {
                continue;
            }
            pitch_min[i] = pitch_min[i].min(p);
            pitch_max[i] = pitch_max[i].max(p);
            pitch_sum[i] += p;
            if p <= crate::state::PITCH_MIN + f32::EPSILON {
                clamp_low[i] += 1;
            }
            if p >= crate::state::PITCH_MAX - f32::EPSILON {
                clamp_high[i] += 1;
            }
        }
        if f.crossfade_active {
            cross += 1;
        }
        if f.trigger.as_deref() == Some("shift_up") {
            up += 1;
        }
        if f.trigger.as_deref() == Some("shift_down") {
            down += 1;
        }
    }
    let n = frames.len() as f32;
    let mut shares = [0.0f32; 5];
    let mut means = [0.0f32; 5];
    for i in 0..5 {
        shares[i] = counts[i] as f32 / n;
        means[i] = if pitch_sum[i] > 0.0 {
            pitch_sum[i] / audible_count(frames, i) as f32
        } else {
            1.0
        };
        if pitch_min[i].is_infinite() {
            pitch_min[i] = 1.0;
        }
        if pitch_max[i].is_infinite() {
            pitch_max[i] = 1.0;
        }
    }
    Summary {
        rows: frames.len(),
        rpm_min,
        rpm_max,
        time_shares: shares,
        pitch_min,
        pitch_mean: means,
        pitch_max,
        clamp_low_hits: clamp_low,
        clamp_high_hits: clamp_high,
        crossfade_share: cross as f32 / n,
        shift_up: up,
        shift_down: down,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_CSV: &str = r#"Time_ms,Speed_kmh,RPM,Gear,Throttle,Brake,Steering,Lat_G,Long_G,FL_Comp,FR_Comp,RL_Comp,RR_Comp,Front_Slip,Rear_Slip,Session_Id,Session_Timestamp_UTC,Physics_Hz,Test_Id,Track_Scene,Vehicle_Node_Path,Vehicle_Scene,Vehicle_Script,Setup_Schema_Version,Setup_JSON
0,0.1,4500,0,0.000,0.000,0.000,0.000,0.000,0.0,0.0,0.0,0.0,0.000,0.000,s1,ts,120,t,sc,p,vs,vscr,1,"{}"
50,10.0,6000,1,1.000,0.000,0.000,0.000,0.000,0.0,0.0,0.0,0.0,0.010,0.010,s1,ts,120,t,sc,p,vs,vscr,1,"{}"
100,20.0,8000,2,0.800,0.000,0.000,0.000,0.000,0.0,0.0,0.0,0.0,0.020,0.020,s1,ts,120,t,sc,p,vs,vscr,1,"{}"
150,30.0,12000,3,1.000,0.000,0.000,0.000,0.000,0.0,0.0,0.0,0.0,0.030,0.030,s1,ts,120,t,sc,p,vs,vscr,1,"{}"
"#;

    #[test]
    fn split_handles_quoted_commas_and_doubled_quotes() {
        let line = r#"a,"b,c","d""e",f"#;
        let fields = split_csv_line(line);
        assert_eq!(fields, vec!["a", "b,c", r#"d"e"#, "f"]);
    }

    #[test]
    fn parse_rows_from_fixture() {
        let rows = parse_rows(FIXTURE_CSV);
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].rpm, 4500.0);
        assert_eq!(rows[1].gear, 1);
        assert_eq!(rows[2].gear, 2);
        assert_eq!(rows[3].rpm, 12000.0);
        assert!((rows[3].slip - 0.03).abs() < 1e-6);
    }

    #[test]
    fn replay_is_deterministic_and_produces_mixes() {
        let rows = parse_rows(FIXTURE_CSV);
        let frames1 = replay(&rows, 4500.0, 17000.0);
        let frames2 = replay(&rows, 4500.0, 17000.0);
        assert_eq!(frames1, frames2);
        assert_eq!(frames1[0].dominant_band, 0);
        // 12000 rpm with idle 4500 / max 17000 => norm ≈ 0.6, mid-high band dominant.
        assert!(frames1[3].dominant_band >= 2);
        // Pitch at each band native ~1.0 for the matching row.
        assert!((frames1[1].pitch_scales[1] - 6000.0 / 7429.0).abs() < 0.05);
    }

    #[test]
    fn trigger_derived_from_gear_change() {
        let rows = parse_rows(FIXTURE_CSV);
        let frames = replay(&rows, 4500.0, 17000.0);
        // Row 1 gear 0->1: no trigger (prev was 0, guard).
        assert_eq!(frames[0].trigger, None);
        // Row 2 gear 1->2: shift_up.
        assert_eq!(frames[2].trigger.as_deref(), Some("shift_up"));
        // Row gap: 2->3 still triggers if distinct (3>2 -> up).
        assert_eq!(frames[3].trigger.as_deref(), Some("shift_up"));
    }

    #[test]
    fn summary_stats_are_sane() {
        let rows = parse_rows(FIXTURE_CSV);
        let frames = replay(&rows, 4500.0, 17000.0);
        let s = summarize(&frames);
        assert_eq!(s.rows, 4);
        assert!(s.rpm_min <= s.rpm_max);
        assert!((s.time_shares.iter().sum::<f32>() - 1.0).abs() < 1e-6);
        assert_eq!(s.shift_up, 2);
    }

    #[test]
    fn summary_ignores_muted_bands_for_pitch_and_clamp() {
        // Fixture: all rows at idle (4500 rpm) -> only the idle band is audible;
        // redline band weight is 0, so its pitch/clamp must not be counted.
        let rows = parse_rows(FIXTURE_CSV);
        // Force all rows to idle rpm by rebuilding a tiny fixture.
        let idle_rows: Vec<TelemetryRow> = rows
            .iter()
            .map(|r| TelemetryRow {
                time_ms: r.time_ms,
                speed_kph: r.speed_kph,
                rpm: 4500.0,
                gear: r.gear,
                throttle: r.throttle,
                slip: r.slip,
                raw_surface: String::new(),
            })
            .collect();
        let frames = replay(&idle_rows, 4500.0, 17000.0);
        let s = summarize(&frames);
        // Redline band never audible at idle -> no clamp hits and pitch stays 1.0.
        assert_eq!(
            s.clamp_high_hits[4], 0,
            "muted redline band must not count clamp hits"
        );
        assert_eq!(
            s.pitch_mean[4], 1.0,
            "muted band mean pitch should not be averaged in"
        );
        // Idle band is audible and its pitch is 4500/3941 = 1.142.
        assert!((s.pitch_mean[0] - 4500.0 / 3941.0).abs() < 0.05);
    }

    #[test]
    fn garbled_rows_are_skipped_not_panicked() {
        let rows = parse_rows(
            "Time_ms,Speed_kmh,RPM,Gear,Throttle,Brake,Steering\nnot-a-row\n0,0,3000,1,0.5,0,0",
        );
        assert!(rows.len() <= 1);
    }
}
