//! Bank-specialised FMOD metadata adapter.
//!
//! This module does not load FMOD banks at runtime.  It translates authoritative
//! simulation telemetry into a small, renderer-neutral command frame whose sample
//! paths point at assets exported from `common.bank` and the R25 V10 bank.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, VecDeque};

pub const AUDIO_COMMAND_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioBackend {
    CommonV10Commands,
    LegacyV10Pcm,
    Disabled,
}

impl Default for AudioBackend {
    fn default() -> Self {
        Self::Disabled
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AudioTelemetryFrame {
    pub tick: u64,
    pub dt_s: f32,
    pub rpm: f32,
    pub throttle: f32,
    pub speed_kph: f32,
    /// Drivetrain shaft speed equivalent (km/h), the bank's `drivetrain_speed`
    /// domain (0..350). Filled from the driven wheels' mean surface speed.
    pub drivetrain_speed_kph: f32,
    pub gear: i32,
    pub slip: f32,
    pub slip_ratio: [f32; 4],
    /// Physics `SurfaceType` discriminants, one per wheel when available.
    pub surfaces: [u8; 4],
    pub brake: f32,
    pub wheel_load_n: [f32; 4],
    pub suspension_velocity_m_s: [f32; 4],
    pub tire_pressure_kpa: [f32; 4],
    pub listener_distance_m: f32,
    pub underfloor_scrape_active: bool,
    pub underfloor_scrape_intensity: f32,
    pub underfloor_scrape_speed_m_s: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollisionKind {
    Barrier,
    Prop,
    Vehicle,
    Generic,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CollisionAudioInput {
    pub kind: CollisionKind,
    pub normal_speed_m_s: f32,
    pub tangential_speed_m_s: f32,
    pub impulse_ns: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceCommand {
    pub id: String,
    pub event: String,
    pub sample: String,
    pub looped: bool,
    /// Non-loop instruments whose FMOD timeline retriggered while the event was
    /// active use this policy. False means play once per activation.
    pub restart_on_finish: bool,
    /// Optional validated loop region for WAV assets. Whole-file looping caused
    /// a 2x-RMS discontinuity in the R25 mid-high layer.
    pub loop_begin_s: Option<f32>,
    pub loop_end_s: Option<f32>,
    pub gain_db: f32,
    pub pitch_semitones: f32,
    pub emitter: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OneShotCommand {
    pub id: u64,
    pub event: String,
    pub sample: String,
    pub gain_db: f32,
    pub pitch_semitones: f32,
    pub emitter: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioCommandFrame {
    pub schema_version: u32,
    pub tick: u64,
    pub backend: AudioBackend,
    pub voices: Vec<VoiceCommand>,
    pub one_shots: Vec<OneShotCommand>,
    pub stops: Vec<String>,
}

impl Default for AudioCommandFrame {
    fn default() -> Self {
        Self {
            schema_version: AUDIO_COMMAND_SCHEMA_VERSION,
            tick: 0,
            backend: AudioBackend::Disabled,
            voices: vec![],
            one_shots: vec![],
            stops: vec![],
        }
    }
}

/// Per-family diagnostic muting. Muted families still emit voices at -80 dB so
/// the renderer never restarts phase or changes voice identity; this is an A/B
/// sweep aid (handoff section 7 paso 2), never a removal path.
#[derive(Debug, Clone, Copy, Default)]
pub struct FamilyMutes {
    pub engine_int: bool,
    pub engine_ext: bool,
    pub transmission: bool,
    pub wind: bool,
    pub wheel: bool,
    pub surfaces: bool,
    pub collisions: bool,
    pub ambience: bool,
}

/// Stateful interpreter for the two approved banks.  FMOD-specific curve choices
/// live here, not in Godot and not in the generic legacy PCM mixer.
#[derive(Debug, Default)]
pub struct CommonV10BankAdapter {
    active: BTreeSet<String>,
    pending_collisions: Vec<CollisionAudioInput>,
    previous_gear: i32,
    dirtiness: f32,
    sequence: u64,
    /// Event journal retained long enough for a renderer running slower than the
    /// 120 Hz simulation to observe every discrete event.
    recent_shots: VecDeque<(u64, OneShotCommand)>,
    nominal_pressure_kpa: [f32; 4],
    previous_inflation: f32,
    mutes: FamilyMutes,
}

impl CommonV10BankAdapter {
    pub fn set_mutes(&mut self, mutes: FamilyMutes) {
        self.mutes = mutes;
    }

    pub fn push_collision(&mut self, collision: CollisionAudioInput) {
        self.pending_collisions.push(collision);
    }

    pub fn adapt(&mut self, t: AudioTelemetryFrame, backend: AudioBackend) -> AudioCommandFrame {
        if backend != AudioBackend::CommonV10Commands {
            let stops = self.active.iter().cloned().collect();
            self.active.clear();
            self.pending_collisions.clear();
            self.recent_shots.clear();
            return AudioCommandFrame {
                tick: t.tick,
                backend,
                stops,
                ..Default::default()
            };
        }
        let mut voices = Vec::new();
        let mut shots = Vec::new();
        self.add_v10(&t, &mut voices, &mut shots);
        self.add_common(&t, &mut voices);
        self.add_collisions(&mut shots);
        for shot in shots {
            self.recent_shots.push_back((t.tick, shot));
        }
        while self
            .recent_shots
            .front()
            .is_some_and(|(tick, _)| t.tick.saturating_sub(*tick) > 120)
        {
            self.recent_shots.pop_front();
        }
        let shots = self
            .recent_shots
            .iter()
            .map(|(_, shot)| shot.clone())
            .collect();
        let next: BTreeSet<_> = voices.iter().map(|v| v.id.clone()).collect();
        let stops = self.active.difference(&next).cloned().collect();
        self.active = next;
        self.previous_gear = t.gear;
        AudioCommandFrame {
            schema_version: AUDIO_COMMAND_SCHEMA_VERSION,
            tick: t.tick,
            backend,
            voices,
            one_shots: shots,
            stops,
        }
    }

    fn add_v10(
        &mut self,
        t: &AudioTelemetryFrame,
        out: &mut Vec<VoiceCommand>,
        shots: &mut Vec<OneShotCommand>,
    ) {
        let interior_mix = (1.0 - (t.listener_distance_m / 3.0).clamp(0.0, 1.0)).powi(2);
        let exterior_mix = (t.listener_distance_m / 3.0).clamp(0.0, 1.0).sqrt();
        let load = t.throttle.clamp(0.0, 1.0).sqrt();
        let bed_blend = 1.0_f32;
        let engine_pitch = linear_curve(t.rpm, 5281.519, -11.444349, 15687.891, 0.0);
        // engine_int — bank-exact instrument tables (catalog.v2.json, recovered
        // from vrc_2005_renault_r25.bank via the offline FModBankParser dump).
        // Bed layers always play (region mixer, no throttle gate): idle,
        // off_low, off_mid (two windows), off_midhigh, off_downshift. Load
        // layers scale with throttle: on_mid/on_midhigh/on_high/on_upshift.
        // Each voice keeps its RPM window (enter/exit ramps); the union of bed
        // windows covers 0..17400+ rpm with no gaps (golden trace asserts it).
        for (name, sample, base_db, enter, exit, blend) in [
            (
                "idle",
                "r25 int idle.ogg",
                -2.5,
                None,
                Some((3800.0, 4600.0)),
                bed_blend,
            ),
            (
                "off_low",
                "r25 int off low.ogg",
                -3.0,
                Some((3800.0, 4600.0)),
                Some((6600.0, 8200.0)),
                bed_blend,
            ),
            (
                "off_mid",
                "r25 int off mid.wav",
                -3.0,
                Some((6600.0, 8200.0)),
                Some((9400.0, 11000.0)),
                bed_blend,
            ),
            (
                "off_mid_hi",
                "r25 int off mid.wav",
                -3.0,
                Some((9400.0, 11000.0)),
                Some((11800.0, 14200.0)),
                bed_blend,
            ),
            (
                "off_midhigh",
                "r25 int off midhigh.ogg",
                -4.0,
                Some((11800.0, 14200.0)),
                Some((15200.0, 15600.0)),
                bed_blend,
            ),
            (
                "off_downshift",
                "r25 int off downshift.wav",
                -4.0,
                Some((15200.0, 15600.0)),
                None,
                bed_blend,
            ),
        ] {
            let gain = bank_window(t.rpm, enter, exit) * blend * interior_mix;
            voice(
                out,
                &format!("v10.engine.{name}"),
                "v10/engine_int",
                &format!("res://sounds/runtime/v10_v2/{sample}"),
                gain_to_db(gain) + base_db,
                engine_pitch,
                "cockpit",
                true,
                false,
                self.mutes.engine_int,
            );
        }
        for (name, sample, base_db, enter, exit) in [
            (
                "on_mid",
                "r25 int on mid.wav",
                -3.0,
                Some((6600.0, 8200.0)),
                Some((9200.0, 10600.0)),
            ),
            (
                "on_midhigh",
                "r25 int on midhigh.wav",
                -4.0,
                Some((9200.0, 10600.0)),
                Some((13400.0, 15600.0)),
            ),
            (
                "on_high",
                "r25 int on high.wav",
                -5.0,
                Some((16800.0, 17400.0)),
                None,
            ),
            (
                "on_upshift",
                "r25 int on upshift.wav",
                -5.0,
                Some((13400.0, 15600.0)),
                Some((16800.0, 17400.0)),
            ),
        ] {
            // Load layers follow the bank's throttle automation: their window
            // ramps gate the region, throttle scales the layer in.
            let gain = bank_window(t.rpm, enter, exit) * load * interior_mix;
            voice(
                out,
                &format!("v10.engine.{name}"),
                "v10/engine_int",
                &format!("res://sounds/runtime/v10_v2/{sample}"),
                gain_to_db(gain) + base_db,
                engine_pitch,
                "cockpit",
                true,
                false,
                self.mutes.engine_int,
            );
        }
        // engine_ext — bank-exact instrument tables (catalog.v2.json, 17
        // merged layer voices from the 24 recovered ext instrument instances).
        // Bed layers (idle/off_*) always play; load layers (on_*) scale with
        // throttle. `r25 ext on high rear close 1` has no recoverable
        // automation (controller-owner chain unresolved): it follows its
        // siblings' high entry window (documented approximation).
        for (name, sample, base_db, base_pitch_st, enter, exit, blend, emitter) in [
            (
                "idle",
                "r25 ext idle.ogg",
                -3.0,
                0.0,
                None,
                Some((5400.0, 7000.0)),
                bed_blend,
                "body",
            ),
            (
                "off_mid",
                "r25 ext off mid rear close.wav",
                -5.0,
                0.0,
                Some((5200.0, 6800.0)),
                Some((9600.0, 12800.0)),
                bed_blend,
                "rear",
            ),
            (
                "off_midhigh",
                "r25 ext off midhigh rear close.wav",
                -6.0,
                0.0,
                Some((9600.0, 12800.0)),
                Some((15200.0, 15600.0)),
                bed_blend,
                "rear",
            ),
            (
                "off_midhigh_front",
                "r25 ext off midhigh front close.wav",
                0.0,
                0.0,
                Some((9600.0, 12800.0)),
                Some((15200.0, 15600.0)),
                bed_blend,
                "front",
            ),
            (
                "off_downshift_rear",
                "r25 ext off downshift rear close.wav",
                -6.0,
                0.0,
                Some((15200.0, 15600.0)),
                None,
                bed_blend,
                "rear",
            ),
            (
                "off_downshift_front",
                "r25 ext off downshift front close.wav",
                0.0,
                0.0,
                Some((15200.0, 15600.0)),
                None,
                bed_blend,
                "front",
            ),
        ] {
            let gain = bank_window(t.rpm, enter, exit) * blend * exterior_mix;
            voice(
                out,
                &format!("v10.engine_ext.{name}"),
                "v10/engine_ext",
                &format!("res://sounds/runtime/v10_v2/{sample}"),
                gain_to_db(gain) + base_db,
                engine_pitch + base_pitch_st,
                emitter,
                true,
                false,
                self.mutes.engine_ext,
            );
        }
        for (name, sample, base_db, base_pitch_st, enter, exit, emitter) in [
            (
                "on_mid",
                "r25 ext on mid rear close.wav",
                -4.0,
                0.0,
                Some((5200.0, 6800.0)),
                Some((9000.0, 10600.0)),
                "rear",
            ),
            (
                "on_midhigh",
                "r25 ext on midhihg rear close.wav",
                -3.5,
                0.0,
                Some((9000.0, 10600.0)),
                Some((11400.0, 13800.0)),
                "rear",
            ),
            (
                "on_high",
                "r25 ext on high rear close 1.wav",
                -5.0,
                0.0,
                Some((17000.0, 17400.0)),
                None,
                "rear",
            ),
            (
                "on_high_front",
                "r25 ext on high front close.ogg",
                -4.0,
                -1.0,
                Some((17000.0, 17400.0)),
                None,
                "front",
            ),
            (
                "on_high_front_far",
                "r25 ext on high front far 3.wav",
                -4.5,
                -1.0,
                Some((17000.0, 17400.0)),
                None,
                "front",
            ),
            (
                "on_high_rear_far",
                "r25 ext on high rear far.wav",
                -3.0,
                0.0,
                Some((17200.0, 17400.0)),
                None,
                "rear",
            ),
            (
                "on_upshift_front",
                "r25 ext on upshift front close.wav",
                -4.0,
                -1.0,
                Some((12800.0, 14200.0)),
                Some((17000.0, 17400.0)),
                "front",
            ),
            (
                "on_upshift_front_far",
                "r25 ext on upshift front far 2.wav",
                -4.5,
                -1.0,
                Some((11200.0, 14200.0)),
                Some((17000.0, 17400.0)),
                "front",
            ),
            (
                "on_upshift_rear_far",
                "r25 ext on upshift rear far.wav",
                -3.0,
                0.0,
                Some((11400.0, 13800.0)),
                Some((17200.0, 17400.0)),
                "rear",
            ),
            (
                "f2000_high",
                "f2000 ext on high rear close.wav",
                -5.0,
                0.0,
                Some((17200.0, 17400.0)),
                None,
                "rear",
            ),
            (
                "f2000_upshift",
                "f2000 ext on upshift rear close.wav",
                -5.0,
                0.0,
                Some((11400.0, 13800.0)),
                Some((17200.0, 17400.0)),
                "rear",
            ),
        ] {
            let gain = bank_window(t.rpm, enter, exit) * load * exterior_mix;
            voice(
                out,
                &format!("v10.engine_ext.{name}"),
                "v10/engine_ext",
                &format!("res://sounds/runtime/v10_v2/{sample}"),
                gain_to_db(gain) + base_db,
                engine_pitch + base_pitch_st,
                emitter,
                true,
                false,
                self.mutes.engine_ext,
            );
        }
        let speed = (t.speed_kph / 300.0).clamp(0.0, 1.0);
        let load_ratio = (t.wheel_load_n.iter().sum::<f32>() / 20_000.0).clamp(0.25, 1.4);
        if speed > 0.015 {
            voice(
                out,
                "v10.wind",
                "v10/wind",
                "res://sounds/runtime/v10_v2/single seater highspeed wind.ogg",
                gain_to_db(speed.powf(1.5)) - 8.0,
                0.0,
                "cockpit",
                true,
                false,
                self.mutes.wind,
            );
            voice(
                out,
                "v10.wheel",
                "v10/wheel",
                "res://sounds/runtime/v10_v2/tyre_rolling.wav",
                gain_to_db(speed * load_ratio) - 12.0,
                speed * 3.0,
                "wheels",
                true,
                false,
                self.mutes.wheel,
            );
            // transmission — bank-exact `drivetrain_speed` windows (0..350 km/h
            // domain): gt3r mid 0..152.2, z4gt3 midhigh 143..176.7, z4gt3 high
            // 172.4..350. Driven by the REAL drivetrain shaft speed
            // (drivetrain_speed_kph), not the vehicle-speed proxy.
            let shaft = t.drivetrain_speed_kph;
            for (id, sample, enter, exit, base_db) in [
                (
                    "v10.transmission.mid",
                    "gt3r 19 trans main mid 2.ogg",
                    None,
                    Some((143.0, 152.21606)),
                    6.5,
                ),
                (
                    "v10.transmission.midhigh",
                    "z4gt3 int on tr midhigh.ogg",
                    Some((142.73883, 152.21606)),
                    Some((172.43005, 176.74503)),
                    4.0,
                ),
                (
                    "v10.transmission.high",
                    "z4gt3 int on tr high.ogg",
                    Some((172.43005, 176.74503)),
                    None,
                    2.0,
                ),
            ] {
                let weight = bank_window(shaft, enter, exit) * (0.25 + load * 0.75);
                voice(
                    out,
                    id,
                    "v10/transmission",
                    &format!("res://sounds/runtime/v10_v2/{sample}"),
                    gain_to_db(weight) + base_db,
                    3.4,
                    "cockpit",
                    true,
                    false,
                    self.mutes.transmission,
                );
            }
        }
        let weighted_slip = t
            .slip_ratio
            .iter()
            .zip(t.wheel_load_n.iter())
            .map(|(slip, load)| ((slip.abs() - 0.06).max(0.0)) * load.max(0.0))
            .sum::<f32>()
            / t.wheel_load_n.iter().sum::<f32>().max(1.0);
        let skid = ((weighted_slip.max(t.slip.abs()) - 0.06) / 0.50).clamp(0.0, 1.0);
        if skid > 0.01 {
            voice(
                out,
                "v10.skid",
                "v10/skid_int",
                "res://sounds/runtime/v10_v2/Skid.ogg",
                gain_to_db(skid) - 2.0,
                -3.0,
                "wheels",
                true,
                false,
                self.mutes.wheel,
            );
        }
        if t.brake > 0.04 && speed > 0.03 {
            voice(
                out,
                "v10.brakes",
                "v10/wheel/brakes",
                "res://sounds/runtime/v10_v2/lb brakes.ogg",
                gain_to_db(t.brake * speed) - 8.0,
                0.0,
                "wheels",
                true,
                false,
                self.mutes.wheel,
            );
        }
        for index in 0..4 {
            if self.nominal_pressure_kpa[index] <= 1.0 && t.tire_pressure_kpa[index] > 20.0 {
                self.nominal_pressure_kpa[index] = t.tire_pressure_kpa[index];
            }
        }
        let inflation = t
            .tire_pressure_kpa
            .iter()
            .zip(self.nominal_pressure_kpa.iter())
            .filter(|(current, nominal)| **current > 1.0 && **nominal > 1.0)
            .map(|(current, nominal)| current / nominal)
            .fold(1.0_f32, f32::min)
            .clamp(0.0, 1.0);
        if !self.mutes.wheel && inflation < 0.78 && speed > 0.03 {
            voice(
                out,
                "v10.flat_tyre",
                "v10/wheel/flat",
                "res://sounds/runtime/v10_v2/flat_tyre_mono.wav",
                gain_to_db((1.0 - inflation) * speed) - 4.0,
                0.0,
                "wheels",
                true,
                false,
                self.mutes.wheel,
            );
            voice(
                out,
                "v10.tyre_flutter",
                "v10/wheel/flutter",
                "res://sounds/runtime/v10_v2/flutter_4.wav",
                gain_to_db((1.0 - inflation) * speed) - 8.0,
                0.0,
                "wheels",
                true,
                false,
                self.mutes.wheel,
            );
        }
        if !self.mutes.wheel && self.previous_inflation > 0.45 && inflation <= 0.45 {
            self.sequence += 1;
            shots.push(OneShotCommand {
                id: self.sequence,
                event: "v10/wheel/explosion".into(),
                sample: "res://sounds/runtime/v10_v2/tyre_explosion.wav".into(),
                gain_db: -2.0,
                pitch_semitones: 0.0,
                emitter: "wheels".into(),
            });
        }
        self.previous_inflation = inflation;
        if !self.mutes.engine_int && self.previous_gear != 0 && t.gear != self.previous_gear {
            let up = t.gear > self.previous_gear;
            self.sequence += 1;
            shots.push(OneShotCommand {
                id: self.sequence,
                event: if up {
                    "v10/gear_int/up"
                } else {
                    "v10/gear_int/down"
                }
                .into(),
                sample: format!(
                    "res://sounds/runtime/v10_v2/{}",
                    if up {
                        "r25 int on upshift.wav"
                    } else {
                        "r25 int off downshift.wav"
                    }
                ),
                gain_db: -2.0,
                pitch_semitones: 0.0,
                emitter: "cockpit".into(),
            });
        }
    }

    fn add_common(&mut self, t: &AudioTelemetryFrame, out: &mut Vec<VoiceCommand>) {
        voice(
            out,
            "common.ambience",
            "common/ambience",
            "res://sounds/runtime/common/ambience_mix.wav",
            -28.0,
            0.0,
            "world",
            true,
            false,
            self.mutes.ambience,
        );
        let offroad = t.surfaces.iter().any(|s| matches!(*s, 2 | 3 | 4 | 5));
        let rate = if offroad { 1.8 } else { -0.65 };
        self.dirtiness = (self.dirtiness + rate * t.dt_s).clamp(0.0, 1.0);
        let speed = (t.speed_kph / 140.0).clamp(0.0, 1.0);
        let total_load = t.wheel_load_n.iter().sum::<f32>();
        for (codes, id, event, sample, base_db, base_pitch, looped, restart) in [
            (
                &[1_u8][..],
                "common.kerb",
                "common/surfaces/kerb",
                "kerb_rumble_digital.wav",
                6.0,
                0.0,
                false,
                true,
            ),
            (
                &[1_u8][..],
                "common.kerb_mid",
                "common/surfaces/kerb_mid",
                "kerb3_p6.wav",
                2.0,
                2.2,
                false,
                true,
            ),
            (
                &[1_u8][..],
                "common.kerb_impacts",
                "common/surfaces/kerb_impacts",
                "kerb p.wav",
                0.0,
                19.5,
                false,
                true,
            ),
            (
                &[3_u8][..],
                "common.grass",
                "common/surfaces/grass",
                "grass_4_p.wav",
                -3.5,
                0.7,
                true,
                false,
            ),
            (
                &[4_u8][..],
                "common.gravel",
                "common/debris/gravel",
                "tyres_gravel.wav",
                4.5,
                2.4,
                false,
                true,
            ),
            (
                &[2_u8, 5_u8][..],
                "common.sand",
                "common/surfaces/sand",
                "sand2_p.wav",
                7.5,
                -0.9,
                false,
                true,
            ),
        ] {
            let matching_load = t
                .surfaces
                .iter()
                .zip(t.wheel_load_n.iter())
                .filter(|(surface, _)| codes.contains(surface))
                .map(|(_, load)| load.max(0.0))
                .sum::<f32>();
            let matching_count = t
                .surfaces
                .iter()
                .filter(|surface| codes.contains(surface))
                .count() as f32;
            let contact = if total_load > 1.0 {
                matching_load / total_load
            } else {
                matching_count / 4.0
            };
            if contact <= 0.0 || speed <= 0.01 {
                continue;
            }
            let slip_texture = (1.0 + t.slip.abs().clamp(0.0, 1.0) * 0.6).min(1.6);
            voice(
                out,
                id,
                event,
                &format!("res://sounds/runtime/common/{sample}"),
                gain_to_db(speed * contact.sqrt() * slip_texture) + base_db - 8.0,
                base_pitch,
                "wheels",
                looped,
                restart,
                self.mutes.surfaces,
            );
        }
        if self.dirtiness > 0.02 && t.speed_kph > 15.0 {
            voice(
                out,
                "common.debris",
                "common/debris/dirtiness",
                "res://sounds/runtime/common/stone hits unp.wav",
                gain_to_db(self.dirtiness * speed) + 4.0 - 12.0,
                11.0,
                "body",
                true,
                false,
                self.mutes.surfaces,
            );
        }
        if t.underfloor_scrape_active && t.underfloor_scrape_intensity > 0.01 {
            voice(
                out,
                "common.floor_scrape",
                "common/mechanical/floor_scrape",
                "res://sounds/runtime/common/floorboard_indy.wav",
                gain_to_db(t.underfloor_scrape_intensity) + 10.0,
                0.6,
                "underfloor",
                true,
                false,
                self.mutes.surfaces,
            );
        }
    }

    fn add_collisions(&mut self, out: &mut Vec<OneShotCommand>) {
        for c in self.pending_collisions.drain(..) {
            if self.mutes.collisions {
                continue;
            }
            if c.normal_speed_m_s < 1.5 && c.tangential_speed_m_s < 3.0 {
                continue;
            }
            let (event, sample, base_db, base_pitch) = match c.kind {
                CollisionKind::Barrier if c.normal_speed_m_s < 3.0 => {
                    ("common/collision/scrape", "crash_squeak_unp.wav", 9.0, -0.5)
                }
                CollisionKind::Barrier => {
                    ("common/collision/barrier", "crash_wall_p.wav", 6.0, 0.5)
                }
                CollisionKind::Prop => ("common/collision/prop", "cone_hit.wav", 7.5, 0.7),
                CollisionKind::Vehicle => {
                    ("common/collision/vehicle", "crash_hit_nascar.wav", 5.0, 0.0)
                }
                CollisionKind::Generic => ("common/collision/body", "impact_2.wav", 6.5, 0.3),
            };
            self.sequence += 1;
            let strength = (c.normal_speed_m_s / 18.0 + c.impulse_ns / 9000.0).clamp(0.08, 1.0);
            out.push(OneShotCommand {
                id: self.sequence,
                event: event.into(),
                sample: format!("res://sounds/runtime/common/{sample}"),
                gain_db: (gain_to_db(strength) + base_db - 8.0).clamp(-80.0, 6.0),
                pitch_semitones: base_pitch + ((self.sequence % 7) as f32 - 3.0) * 0.12,
                emitter: "body".into(),
            });
            // In common.bank cockpit rattles are layers of collision events, not
            // a standalone suspension bed. Keep that ownership here so normal
            // suspension movement cannot create an unrelated periodic rattle.
            if matches!(c.kind, CollisionKind::Barrier | CollisionKind::Vehicle) && strength >= 0.35
            {
                self.sequence += 1;
                out.push(OneShotCommand {
                    id: self.sequence,
                    event: "common/collision/cockpit_rattle".into(),
                    sample: "res://sounds/runtime/common/rattle_cockpit_p.wav".into(),
                    gain_db: (gain_to_db(strength) - 10.0).clamp(-80.0, -2.0),
                    pitch_semitones: 1.2,
                    emitter: "cockpit".into(),
                });
            }
        }
    }
}

fn voice(
    out: &mut Vec<VoiceCommand>,
    id: &str,
    event: &str,
    sample: &str,
    gain_db: f32,
    pitch: f32,
    emitter: &str,
    looped: bool,
    restart_on_finish: bool,
    muted: bool,
) {
    let loop_region = if sample.ends_with("r25 int on mid.wav") {
        // Fase 2: measured k*period-aligned regions (loop_regions.json /
        // tools/audio/fix_loop_regions.py). on_mid seam 4.4 -> 0.3 dB;
        // on_high 4.2 -> 3.5 dB; off_mid 4.6 -> 4.1 dB; on_midhigh kept
        // (measured seam 2.1 dB vs 1.8 dB for the original region).
        Some((0.033628, 1.639977))
    } else if sample.ends_with("r25 int on midhigh.wav") {
        Some((0.098141, 2.551587))
    } else if sample.ends_with("r25 int on high.wav") {
        Some((0.035034, 4.927007))
    } else if sample.ends_with("r25 int off mid.wav") {
        Some((0.031837, 2.588571))
    } else {
        None
    };
    out.push(VoiceCommand {
        id: id.into(),
        event: event.into(),
        sample: sample.into(),
        looped,
        restart_on_finish,
        loop_begin_s: loop_region.map(|v| v.0),
        loop_end_s: loop_region.map(|v| v.1),
        gain_db: if muted { -80.0 } else { gain_db.clamp(-80.0, 6.0) },
        pitch_semitones: pitch,
        emitter: emitter.into(),
    });
}

/// Bank window ramp: optional enter (0->1 over `a..b`) and exit (1->0 over
/// `a..b`) ramps per instrument automation (catalog.v2.json). Linear
/// interpolation between the recovered FMOD curve points (ramps are short, so
/// the curve "shape" power term is within ~0.5 dB of linear mid-ramp).
fn bank_window(rpm: f32, enter: Option<(f32, f32)>, exit: Option<(f32, f32)>) -> f32 {
    let mut g = 1.0;
    if let Some((a, b)) = enter {
        g *= ramp01(rpm, a, b, 0.0, 1.0);
    }
    if let Some((a, b)) = exit {
        g *= ramp01(rpm, a, b, 1.0, 0.0);
    }
    g.max(0.0)
}

fn ramp01(x: f32, a: f32, b: f32, y0: f32, y1: f32) -> f32 {
    if x <= a {
        y0
    } else if x >= b {
        y1
    } else {
        y0 + (y1 - y0) * (x - a) / (b - a)
    }
}

fn gain_to_db(gain: f32) -> f32 {
    20.0 * gain.max(0.0001).log10()
}

fn linear_curve(x: f32, x0: f32, y0: f32, x1: f32, y1: f32) -> f32 {
    let t = ((x - x0) / (x1 - x0)).clamp(0.0, 1.0);
    y0 + (y1 - y0) * t
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn commands_are_stable_and_stops_are_emitted() {
        let mut a = CommonV10BankAdapter::default();
        let mut t = AudioTelemetryFrame {
            tick: 1,
            dt_s: 1.0 / 120.0,
            rpm: 8000.0,
            throttle: 1.0,
            speed_kph: 180.0,
            gear: 4,
            slip: 0.2,
            surfaces: [1, 0, 0, 0],
            ..Default::default()
        };
        let first = a.adapt(t, AudioBackend::CommonV10Commands);
        assert!(first.voices.iter().any(|v| v.id == "common.kerb"));
        t.tick = 2;
        t.surfaces = [0; 4];
        t.slip = 0.0;
        let second = a.adapt(t, AudioBackend::CommonV10Commands);
        assert!(second.stops.iter().any(|v| v == "common.kerb"));
    }

    #[test]
    fn r25_high_layer_never_uses_transmission_sample() {
        let mut adapter = CommonV10BankAdapter::default();
        let frame = adapter.adapt(
            AudioTelemetryFrame {
                tick: 1,
                dt_s: 1.0 / 120.0,
                rpm: 16_000.0,
                throttle: 1.0,
                speed_kph: 250.0,
                gear: 6,
                ..Default::default()
            },
            AudioBackend::CommonV10Commands,
        );
        let high = frame
            .voices
            .iter()
            .find(|v| v.id == "v10.engine.on_high")
            .unwrap();
        assert!(high.sample.ends_with("r25 int on high.wav"));
        assert!(!high.sample.contains("z4gt3"));
    }

    #[test]
    fn one_shot_journal_survives_slower_renderer() {
        let mut adapter = CommonV10BankAdapter::default();
        let base = AudioTelemetryFrame {
            dt_s: 1.0 / 120.0,
            rpm: 10_000.0,
            throttle: 1.0,
            speed_kph: 180.0,
            gear: 3,
            ..Default::default()
        };
        adapter.adapt(
            AudioTelemetryFrame { tick: 1, ..base },
            AudioBackend::CommonV10Commands,
        );
        let shifted = adapter.adapt(
            AudioTelemetryFrame {
                tick: 2,
                gear: 4,
                ..base
            },
            AudioBackend::CommonV10Commands,
        );
        assert_eq!(shifted.one_shots.len(), 1);
        let later = adapter.adapt(
            AudioTelemetryFrame {
                tick: 4,
                gear: 4,
                ..base
            },
            AudioBackend::CommonV10Commands,
        );
        assert_eq!(later.one_shots[0].id, shifted.one_shots[0].id);
    }

    #[test]
    fn backend_switch_does_not_block_new_one_shot_ids() {
        let mut adapter = CommonV10BankAdapter::default();
        let base = AudioTelemetryFrame {
            dt_s: 1.0 / 120.0,
            rpm: 9_000.0,
            throttle: 1.0,
            speed_kph: 160.0,
            gear: 3,
            wheel_load_n: [2_500.0; 4],
            ..Default::default()
        };
        // First activation: gear 3 -> 4 emits shot id 1.
        adapter.adapt(
            AudioTelemetryFrame { tick: 1, ..base },
            AudioBackend::CommonV10Commands,
        );
        let shifted = adapter.adapt(
            AudioTelemetryFrame {
                tick: 2,
                gear: 4,
                ..base
            },
            AudioBackend::CommonV10Commands,
        );
        assert_eq!(shifted.one_shots.len(), 1);
        let first_id = shifted.one_shots[0].id;
        // Leave commands (backend switch clears journal; ids keep climbing).
        adapter.adapt(
            AudioTelemetryFrame {
                tick: 3,
                gear: 4,
                ..base
            },
            AudioBackend::LegacyV10Pcm,
        );
        // Return: gear 4 -> 5 emits a NEW id (never reused).
        let back = adapter.adapt(
            AudioTelemetryFrame {
                tick: 4,
                gear: 5,
                ..base
            },
            AudioBackend::CommonV10Commands,
        );
        assert!(
            back.one_shots.iter().any(|s| s.id > first_id),
            "new shot id {} must exceed prior {} after backend switch (renderer dedupes monotonically)",
            back.one_shots.first().map(|s| s.id).unwrap_or(0),
            first_id
        );
    }

    #[test]
    fn collisions_process_multiple_contacts_per_tick_sorted_strongest_first() {
        let mut adapter = CommonV10BankAdapter::default();
        // C++ delivers strongest-first (sorted by normal impact); the adapter
        // preserves arrival order deterministically.
        adapter.push_collision(CollisionAudioInput {
            kind: CollisionKind::Barrier,
            normal_speed_m_s: 14.0,
            tangential_speed_m_s: 1.0,
            impulse_ns: 10_000.0,
        });
        adapter.push_collision(CollisionAudioInput {
            kind: CollisionKind::Prop,
            normal_speed_m_s: 7.0,
            tangential_speed_m_s: 1.0,
            impulse_ns: 6_000.0,
        });
        let frame = adapter.adapt(
            AudioTelemetryFrame {
                tick: 1,
                dt_s: 1.0 / 120.0,
                rpm: 9_000.0,
                ..Default::default()
            },
            AudioBackend::CommonV10Commands,
        );
        let shots = &frame.one_shots;
        assert_eq!(
            shots.len(),
            3,
            "both contacts become shots plus the strong-impact rattle layer"
        );
        assert_eq!(
            shots.iter().map(|s| s.id).collect::<std::collections::BTreeSet<_>>().len(),
            3,
            "distinct monotonic ids"
        );
        let barrier_at = shots
            .iter()
            .position(|s| s.event == "common/collision/barrier")
            .expect("barrier shot");
        let prop_at = shots
            .iter()
            .position(|s| s.event == "common/collision/prop")
            .expect("prop shot");
        assert!(barrier_at < prop_at, "strongest contact first");
        assert!(
            shots
                .iter()
                .any(|s| s.event == "common/collision/cockpit_rattle"),
            "strong impacts carry the cockpit rattle layer"
        );
    }

    #[test]
    fn surface_codes_select_the_matching_common_samples() {
        let mut adapter = CommonV10BankAdapter::default();
        let base = AudioTelemetryFrame {
            speed_kph: 80.0,
            wheel_load_n: [2_500.0; 4],
            ..Default::default()
        };
        let grass = adapter.adapt(
            AudioTelemetryFrame {
                surfaces: [3, 0, 0, 0],
                ..base.clone()
            },
            AudioBackend::CommonV10Commands,
        );
        assert!(grass
            .voices
            .iter()
            .any(|v| v.sample.ends_with("grass_4_p.wav")));
        assert!(!grass
            .voices
            .iter()
            .any(|v| v.sample.ends_with("tyres_gravel.wav")));

        let gravel = adapter.adapt(
            AudioTelemetryFrame {
                tick: 1,
                surfaces: [4, 0, 0, 0],
                ..base
            },
            AudioBackend::CommonV10Commands,
        );
        assert!(gravel
            .voices
            .iter()
            .any(|v| v.sample.ends_with("tyres_gravel.wav")));
        assert!(!gravel
            .voices
            .iter()
            .any(|v| v.sample.ends_with("grass_4_p.wav")));
    }

    #[test]
    fn family_mutes_keep_voices_alive_at_minus_80_db() {
        let mut adapter = CommonV10BankAdapter::default();
        adapter.set_mutes(FamilyMutes {
            engine_int: true,
            engine_ext: true,
            transmission: true,
            wind: true,
            wheel: true,
            surfaces: true,
            collisions: true,
            ambience: true,
            ..Default::default()
        });
        let frame = adapter.adapt(
            AudioTelemetryFrame {
                tick: 1,
                dt_s: 1.0 / 120.0,
                rpm: 8000.0,
                throttle: 1.0,
                speed_kph: 180.0,
                gear: 4,
                slip: 0.2,
                surfaces: [1, 0, 0, 0],
                wheel_load_n: [2500.0; 4],
                ..Default::default()
            },
            AudioBackend::CommonV10Commands,
        );
        assert!(!frame.voices.is_empty(), "voices stay alive when muted");
        assert!(
            frame.voices.iter().all(|v| v.gain_db <= -80.0),
            "all muted voices pinned at -80 dB"
        );
        assert!(frame.one_shots.is_empty());
    }

    #[test]
    fn muting_surfaces_keeps_gear_shots_visible() {
        let mut adapter = CommonV10BankAdapter::default();
        adapter.set_mutes(FamilyMutes {
            surfaces: true,
            ..Default::default()
        });
        let base = AudioTelemetryFrame {
            dt_s: 1.0 / 120.0,
            rpm: 9000.0,
            throttle: 1.0,
            speed_kph: 160.0,
            gear: 3,
            wheel_load_n: [2500.0; 4],
            ..Default::default()
        };
        adapter.adapt(
            AudioTelemetryFrame { tick: 1, ..base },
            AudioBackend::CommonV10Commands,
        );
        let shifted = adapter.adapt(
            AudioTelemetryFrame {
                tick: 2,
                gear: 5,
                ..base
            },
            AudioBackend::CommonV10Commands,
        );
        assert!(shifted.one_shots.iter().any(|s| s.event == "v10/gear_int/up"));
        assert!(!shifted
            .voices
            .iter()
            .any(|v| v.sample.contains("grass_4_p.wav")));
    }
}
