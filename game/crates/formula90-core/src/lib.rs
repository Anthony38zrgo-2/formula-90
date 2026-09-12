#![deny(unsafe_op_in_unsafe_fn)]
//! Formula-90 orchestrator facade — `formula90_core`.
//!
//! Single runtime controller that Godot talks to. It owns three layers:
//!
//! 1. **Simulation world** (`game_sim`) — the authoritative physics core.
//! 2. **Vehicle audio** (`vehicle_audio_engine`) — the sample-accurate mixer,
//!    driven from the SAME telemetry the physics step produced (no round-trip).
//! 3. **Module registry** (`SimModule`) — expandable: climatology, AI, race
//!    director, ... each is a crate registered here. The loop never knows modules.
//!
//! Design invariants:
//! - `CoreFacade::step` (force path, in-engine) / `CoreFacade::step_standalone`
//!   (headless/CLI) are the ONLY places the sim advances, modules tick and audio
//!   inputs update, producing one atomic `CoreFrame` (see [`frame::CoreFrame`]).
//! - Audio render (`audio_render`) reads the telemetry the step already delivered,
//!   so audio↔physics desync is bounded and there is no Godot property round-trip.
//! - Pure Rust: no Godot deps. The headless `core_cli` runs the identical
//!   orchestrator, keeping byte-level parity with the in-engine path.

pub mod audio;
mod audio_telemetry;
pub mod ffi;
pub mod frame;
pub mod module;
pub mod modules;
pub mod underfloor;

#[cfg(test)]
mod aud04_integration {
    use super::*;

    #[test]
    fn physical_signed_torque_reaches_audio_from_standalone_step() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut core = CoreFacade::new(CoreConfig {
            bank_dir: Some(root.join("sounds/banks/v10_vehicle")),
            config_json_path: Some(root.join("data/vehicles/f1_2026_2008/f1_2026_2008_physics.json")),
            use_canonical: false, enable_audio: true, ..CoreConfig::default()
        }).unwrap();
        let id = core.ensure_spawned().unwrap();
        for _ in 0..20 {
            let samples = core.flat_samples(id);
            core.step_standalone(id, &game_sim::DriverInput { throttle: 0.9, ..Default::default() }, &samples, 1.0 / 120.0);
        }
        let ent = &core.world.entities[0];
        let expected = (ent.sim.state.powertrain.engine_torque / ent.sim.config.max_torque).clamp(-1.0, 1.0) as f32;
        assert!(expected.abs() > 0.01);
        let received = core.audio.mut_engine().unwrap().last_normalized_engine_torque();
        assert!((received - expected).abs() < 1e-6, "physical={expected}, audio={received}");
    }

    #[test]
    fn physical_transport_matches_facade_and_gf509() {
        use game_sim::DriverInput;
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut core = CoreFacade::new(CoreConfig {
            bank_dir: Some(root.join("sounds/banks/v10_vehicle")),
            config_json_path: Some(root.join("data/vehicles/f1_2026_2008/f1_2026_2008_physics.json")),
            use_canonical: false,
            enable_audio: true,
            ..CoreConfig::default()
        }).unwrap();
        let id = core.ensure_spawned().unwrap();
        assert!(core.audio_gf509_enabled(), "GF509 must be active for AUD-04 transport");
        for _ in 0..20 {
            let samples = core.flat_samples(id);
            core.step_standalone(id, &DriverInput { throttle: 0.9, ..Default::default() }, &samples, 1.0 / 120.0);
        }

        // Helper: expected acoustic packet computed manually from the
        // authoritative powertrain fields (no call to the mapping under test).
        let expected_from_physical = |p: &vehicle_physics_engine::PowertrainState, max_torque: f64| {
            let torque = (p.engine_torque / max_torque.max(1.0)).clamp(-1.0, 1.0) as f32;
            let mut load = if p.clutch_engagement > 1e-4 {
                (p.clutch_torque.abs() / max_torque.max(1.0)).clamp(0.0, 1.0)
            } else { 0.0 };
            if p.engine_torque > 0.0 {
                load *= 1.0 - p.tc_cut_ratio_smoothed.clamp(0.0, 1.0);
            }
            (load as f32, torque,
             p.clutch_engagement.clamp(0.0, 1.0) as f32,
             p.tc_cut_ratio_smoothed.clamp(0.0, 1.0) as f32,
             p.is_rev_limited, p.shift_timer > 0.0, p.target_gear < p.current_gear)
        };

        let check_transport = |core: &mut CoreFacade, label: &str| {
            let (powertrain, max_torque, rpm, throttle) = {
                let ent = &core.world.entities[0];
                (ent.sim.state.powertrain.clone(), ent.sim.config.max_torque,
                 ent.last.as_ref().map(|t| t.rpm).unwrap_or(0.0), core.frame.throttle)
            };
            let (exp_load, exp_torque, exp_clutch, exp_tc, exp_limiter, exp_shifting, exp_down) =
                expected_from_physical(&powertrain, max_torque);
            let engine = core.audio.mut_engine().unwrap();
            assert!((engine.last_normalized_engine_load() - exp_load).abs() < 1e-6,
                "{label}: load physical={exp_load} audio={}", engine.last_normalized_engine_load());
            assert!((engine.last_normalized_engine_torque() - exp_torque).abs() < 1e-6,
                "{label}: torque physical={exp_torque} audio={}", engine.last_normalized_engine_torque());
            assert!((engine.last_clutch_engagement() - exp_clutch).abs() < 1e-6,
                "{label}: clutch physical={exp_clutch} audio={}", engine.last_clutch_engagement());
            assert!((engine.last_tc_cut_ratio() - exp_tc).abs() < 1e-6,
                "{label}: tc physical={exp_tc} audio={}", engine.last_tc_cut_ratio());
            assert_eq!(engine.last_rev_limiter_active(), exp_limiter, "{label}: limiter");
            // Shift phase comes from the control-rate adapter: a physical cut
            // must surface as Cut; otherwise None or the named 40 ms Recovery.
            let phase = engine.last_shift_phase();
            if exp_shifting {
                assert!(phase == 1 || phase == 3, "{label}: shifting must be Cut, got {phase}");
                if exp_down { assert_eq!(phase, 3, "{label}: downshift cut"); }
                else { assert_eq!(phase, 1, "{label}: upshift cut"); }
            } else {
                assert!(phase == 0 || phase == 2 || phase == 5,
                    "{label}: settled must be None/Recovery, got {phase}");
            }
            // The packet actually consumed by GF509 (target, not mixer snapshot).
            let gf509 = engine.gf509_telemetry().expect("{label}: GF509 telemetry missing");
            assert!((gf509.normalized_engine_load - exp_load).abs() < 1e-6, "{label}: GF509 load");
            assert!((gf509.normalized_engine_torque - exp_torque).abs() < 1e-6, "{label}: GF509 torque");
            assert!((gf509.clutch_engagement - exp_clutch).abs() < 1e-6, "{label}: GF509 clutch");
            assert!((gf509.tc_cut_ratio - exp_tc).abs() < 1e-6, "{label}: GF509 tc");
            assert_eq!(gf509.rev_limiter_active, exp_limiter, "{label}: GF509 limiter");
            assert!(gf509.dt_seconds > 0.0, "{label}: facade must use timed route (dt>0), got {}", gf509.dt_seconds);
            assert!((gf509.rpm as f64 - rpm).abs() < 1.0, "{label}: GF509 rpm {rpm}");
            assert!((gf509.throttle as f64 - throttle).abs() < 1e-6, "{label}: GF509 throttle");
            (rpm, exp_torque, exp_load)
        };

        // Power: positive drive torque expected after full-throttle warmup.
        for _ in 0..8 {
            let samples = core.flat_samples(id);
            core.step_standalone(id, &DriverInput { throttle: 0.9, ..Default::default() }, &samples, 1.0 / 120.0);
        }
        let (rpm_power, torque_power, _) = check_transport(&mut core, "power");
        assert!(torque_power > 0.01, "power must deliver positive torque, got {torque_power}");
        let mut frame = [0.0f32; 512];
        assert_eq!(core.audio_render(&mut frame, &mut [0.0; 512], 512), 512);
        assert!(frame.iter().all(|s| s.is_finite()));
        let energy_power: f32 = frame.iter().map(|s| s * s).sum();

        // Coast: closed throttle, retention path (throttle/load/torque split).
        for _ in 0..8 {
            let samples = core.flat_samples(id);
            core.step_standalone(id, &DriverInput::default(), &samples, 1.0 / 120.0);
        }
        let (_, torque_coast, _) = check_transport(&mut core, "coast");
        assert!(torque_coast <= torque_power, "coast torque {torque_coast} must not exceed power {torque_power}");
        assert_eq!(core.audio_render(&mut frame, &mut [0.0; 512], 512), 512);
        assert!(frame.iter().all(|s| s.is_finite()));
        let energy_coast: f32 = frame.iter().map(|s| s * s).sum();
        assert!((energy_power - energy_coast).abs() > 0.0, "power/coast must render different energy");

        // Clutch open (coherent mapping: engagement->0 implies load->0).
        for _ in 0..8 {
            let samples = core.flat_samples(id);
            core.step_standalone(id, &DriverInput { clutch: 1.0, ..Default::default() }, &samples, 1.0 / 120.0);
        }
        let (_, _, load_clutch) = check_transport(&mut core, "clutch_open");
        {
            let ent = &core.world.entities[0];
            assert!(ent.sim.state.powertrain.clutch_engagement < 0.5,
                "clutch input must disengage, got {}", ent.sim.state.powertrain.clutch_engagement);
            assert_eq!(load_clutch, 0.0, "clutch open must deliver load=0");
        }
        assert_eq!(core.audio_render(&mut frame, &mut [0.0; 512], 512), 512);
        assert!(frame.iter().all(|s| s.is_finite()));

        // Upshift via authoritative gear_request (DriverInput shift_up is not
        // consumed by physics; gear_request is the real contract). First settle
        // to a stable gear with room to shift both directions.
        let settle = |core: &mut CoreFacade, id: u32| {
            for _ in 0..200 {
                let timer = core.world.entities[0].sim.state.powertrain.shift_timer;
                if timer <= 0.0 { break; }
                let samples = core.flat_samples(id);
                core.step_standalone(id, &DriverInput { throttle: 0.5, ..Default::default() }, &samples, 1.0 / 120.0);
            }
        };
        settle(&mut core, id);
        let max_gear = core.world.entities[0].sim.config.gear_ratios.len() as i8;
        // Move away from the top gear so upshift then downshift are both real.
        if core.world.entities[0].sim.state.powertrain.current_gear >= max_gear {
            let samples = core.flat_samples(id);
            core.step_standalone(id, &DriverInput { throttle: 0.5, gear_request: Some(max_gear - 1), ..Default::default() }, &samples, 1.0 / 120.0);
            settle(&mut core, id);
        }
        let base_gear: i8 = core.world.entities[0].sim.state.powertrain.current_gear;
        assert!(core.world.entities[0].sim.state.powertrain.shift_timer <= 0.0, "must start shifts from settled state");
        let up_target = (base_gear + 1).min(max_gear);
        assert!(up_target > base_gear, "need room for upshift from {base_gear} (max {max_gear})");
        let samples = core.flat_samples(id);
        core.step_standalone(id, &DriverInput { throttle: 0.8, gear_request: Some(up_target), ..Default::default() }, &samples, 1.0 / 120.0);
        {
            let p = &core.world.entities[0].sim.state.powertrain;
            assert!(p.shift_timer > 0.0, "gear_request must open a physical cut, timer={}", p.shift_timer);
            assert_eq!(p.target_gear, up_target);
        }
        let _ = check_transport(&mut core, "upshift_cut");
        assert_eq!(core.audio_render(&mut frame, &mut [0.0; 512], 512), 512);
        assert!(frame.iter().all(|s| s.is_finite()));
        settle(&mut core, id);
        assert_eq!(core.world.entities[0].sim.state.powertrain.current_gear, up_target);
        let _ = check_transport(&mut core, "upshift_settle");

        // Downshift back toward the original gear.
        let samples = core.flat_samples(id);
        core.step_standalone(id, &DriverInput { throttle: 0.2, gear_request: Some(base_gear), ..Default::default() }, &samples, 1.0 / 120.0);
        {
            let p = &core.world.entities[0].sim.state.powertrain;
            assert!(p.shift_timer > 0.0, "downshift request must open a physical cut");
            assert!(p.target_gear < p.current_gear, "must be a downshift");
        }
        let _ = check_transport(&mut core, "downshift_cut");
        assert_eq!(core.audio_render(&mut frame, &mut [0.0; 512], 512), 512);
        assert!(frame.iter().all(|s| s.is_finite()));

        // TC toggle: aids mask flips; audio TC field still tracks physics
        // (flat ground yields cut=0, but the transport must stay exact).
        let tc_before = core.world.entities[0].aids.traction_control;
        let samples = core.flat_samples(id);
        core.step_standalone(id, &DriverInput { throttle: 0.9, toggle_traction_control: true, ..Default::default() }, &samples, 1.0 / 120.0);
        let tc_after = core.world.entities[0].aids.traction_control;
        assert_ne!(tc_before, tc_after, "TC toggle must flip aids state");
        let _ = check_transport(&mut core, "tc_toggle");
        assert_eq!(core.audio_render(&mut frame, &mut [0.0; 512], 512), 512);
        assert!(frame.iter().all(|s| s.is_finite()));

        // Limiter field tracks physics even when not engaged on flat ground;
        // the DSP response to a true limiter is covered at fixed RPM below.
        let _ = check_transport(&mut core, "limiter_track");
        let _ = rpm_power; // documented: sequential physics cannot hold RPM
        // fixed across regimes; fixed-RPM DSP separation is the next test.

        core.audio.reset();
        assert_eq!(core.audio.mut_engine().unwrap().last_normalized_engine_torque(), 0.0);
    }

    #[test]
    fn mechanical_dsp_matrix_differs_at_fixed_rpm() {
        use v10_engine_synth::{Gf509Runtime, Gf509RuntimeConfig, RuntimeTelemetry, ShiftPhase, TorqueSign};
        // DSP separation at identical initial control: only the mechanical
        // state varies, so any difference is the state's acoustic response.
        // No productive physics is modified; this is the second half of R2.
        fn render_fixed(base: RuntimeTelemetry) -> Vec<f32> {
            let mut config = Gf509RuntimeConfig::default();
            config.max_block_frames = 512;
            let mut runtime = Gf509Runtime::new(config).unwrap();
            runtime.update_telemetry(base).unwrap();
            let mut left = [0.0f32; 512];
            let mut right = [0.0f32; 512];
            for _ in 0..10 {
                runtime.render_block(&mut left, &mut right).unwrap();
            }
            runtime.render_block(&mut left, &mut right).unwrap();
            assert!(left.iter().all(|s| s.is_finite()));
            left.to_vec()
        }
        let steady = RuntimeTelemetry {
            rpm: 10_000.0, throttle: 0.9, normalized_engine_load: 0.9,
            normalized_engine_torque: 0.9, torque_sign: TorqueSign::Positive,
            rpm_derivative: 0.0, throttle_derivative: 0.0, gear: 4,
            shift_phase: ShiftPhase::None, clutch_engagement: 1.0,
            tc_cut_ratio: 0.0, rev_limiter_active: false, dt_seconds: 1.0 / 120.0,
        };
        let coast = RuntimeTelemetry {
            throttle: 0.0, normalized_engine_load: 0.1,
            normalized_engine_torque: -0.45, torque_sign: TorqueSign::Negative, ..steady
        };
        let cut = RuntimeTelemetry { shift_phase: ShiftPhase::UpshiftCut, ..steady };
        let limiter = RuntimeTelemetry { rev_limiter_active: true, ..steady };
        let tc = RuntimeTelemetry { tc_cut_ratio: 0.6, normalized_engine_load: 0.36, ..steady };
        let out_steady = render_fixed(steady);
        let out_coast = render_fixed(coast);
        let out_cut = render_fixed(cut);
        let out_limiter = render_fixed(limiter);
        let out_tc = render_fixed(tc);
        let max_diff = |a: &[f32], b: &[f32]| {
            a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).fold(0.0f32, f32::max)
        };
        assert!(max_diff(&out_steady, &out_coast) > 1e-4, "coast must differ from power at fixed RPM");
        assert!(max_diff(&out_steady, &out_cut) > 1e-4, "cut must differ from steady at fixed RPM");
        assert!(max_diff(&out_steady, &out_limiter) > 1e-4, "limiter must differ from steady at fixed RPM");
        assert!(max_diff(&out_steady, &out_tc) > 1e-4, "TC texture must differ from steady at fixed RPM");
    }
}

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use game_sim::snapshot::{EntityTelemetry, Snapshot};
use game_sim::world::{yaw_from_transform, World};
use serde::{Deserialize, Serialize};
use vehicle_physics_engine::{
    AeroEnvironment, AeroProbeMode, AidsMask, BodyKinematics, Mat3, SurfaceType, Transform3D,
    TriRaycastSample, Vec3, VehicleConfig, VehicleInput,
};

use crate::audio::AudioModule;
use crate::frame::{CoreFrame, ModuleOutput};
use crate::module::{ModuleCtx, ModuleRegistry, SimModule};

/// Configuration for a facade instance (equivalent to the legacy per-module create
/// options, consolidated into one handshake).
#[derive(Debug, Clone)]
pub struct CoreConfig {
    /// OS path to the `v10_vehicle` sound bank dir. `None` disables the mixer.
    pub bank_dir: Option<PathBuf>,
    /// OS path to the vehicle JSON config. Ignored when `use_canonical`.
    pub config_json_path: Option<PathBuf>,
    /// Spawn the built-in canonical F1-94 config instead of a JSON file.
    pub use_canonical: bool,
    /// Fixed simulation timestep (seconds).
    pub fixed_dt: f64,
    pub enable_audio: bool,
    pub idle_rpm: f64,
    pub max_rpm: f64,
    /// Opaque packed-scene references the mirror resolves (entity metadata).
    pub vehicle_scene: String,
    pub track_scene: String,
    /// Module names to instantiate at startup. Unknown names are silently ignored
    /// (forward compatible: an older core ignores a newer module request).
    pub modules: Vec<String>,
    pub underfloor_contact: underfloor::UnderfloorContactConfig,
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            bank_dir: None,
            config_json_path: None,
            use_canonical: true,
            fixed_dt: 1.0 / 120.0,
            enable_audio: false,
            idle_rpm: 1000.0,
            max_rpm: 15000.0,
			vehicle_scene: "res://scenes/vehicles/f1_2026_2008/f1_2026_2008_rust.tscn".to_string(),
            track_scene: "res://scenes/tracks/test_field/la_chutana_track.tscn".to_string(),
            modules: Vec::new(),
            underfloor_contact: underfloor::UnderfloorContactConfig::default(),
        }
    }
}

#[derive(Debug)]
pub enum CoreError {
    Config(String),
    Spawn(String),
}

impl std::fmt::Display for CoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoreError::Config(m) => write!(f, "config error: {m}"),
            CoreError::Spawn(m) => write!(f, "spawn error: {m}"),
        }
    }
}

impl std::error::Error for CoreError {}

/// Serialized orchestrated snapshot = authoritative sim snapshot + per-module
/// contributions. Ship these bytes to a mirror (postcard), e.g. headless parity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacadeSnapshot {
    pub core: Snapshot,
    pub modules: Vec<(String, Vec<u8>)>,
}

impl FacadeSnapshot {
    pub fn to_bytes(&self) -> Result<Vec<u8>, postcard::Error> {
        postcard::to_allocvec(self)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, postcard::Error> {
        postcard::from_bytes(bytes)
    }
}

/// The orchestrator. Owns the world, the audio subsystem and the module registry.
pub struct CoreFacade {
    world: World,
    config: CoreConfig,
    audio: AudioModule,
    registry: ModuleRegistry,
    frame: CoreFrame,
    latest: RwLock<Arc<CoreFrame>>,
    spawned: bool,
    underfloor: underfloor::UnderfloorState,
}

impl CoreFacade {
    pub fn new(config: CoreConfig) -> Result<Self, CoreError> {
        let audio = AudioModule::new(config.bank_dir.as_deref(), config.enable_audio);
        let world = World::new(config.fixed_dt);
        let mut registry = ModuleRegistry::new();
        for name in &config.modules {
            match name.as_str() {
                "weather" => {
                    registry.register(Box::new(crate::modules::weather_stub::WeatherStub::new()))
                }
                "ai" => registry.register(Box::new(crate::modules::ai_stub::AiStub::new())),
                _ => eprintln!("[formula90_core] unknown module '{name}' ignored"),
            }
        }
        Ok(Self {
            world,
            config,
            audio,
            registry,
            frame: CoreFrame::default(),
            latest: RwLock::new(Arc::new(CoreFrame::default())),
            spawned: false,
            underfloor: underfloor::UnderfloorState::default(),
        })
    }

    /// Spawn the primary entity if not spawned yet. Returns its id.
    pub fn ensure_spawned(&mut self) -> Result<u32, CoreError> {
        if self.spawned {
            return self
                .world
                .entities
                .first()
                .map(|e| e.id)
                .ok_or_else(|| CoreError::Spawn("world has no entities".to_string()));
        }
        let (id, cfg) = if self.config.use_canonical {
			let cfg = VehicleConfig::f1_2026_2008_canonical();
            (self.spawn(&cfg), cfg)
        } else {
            let json = self.config.config_json_path.as_ref().ok_or_else(|| {
                CoreError::Config(
                    "config_json_path is required when use_canonical=false".to_string(),
                )
            })?;
            let cfg = VehicleConfig::from_json_path(json)
                .map_err(|e| CoreError::Spawn(format!("failed to parse config: {e}")))?;
            (self.spawn(&cfg), cfg)
        };
        // Enable the procedural powertrain synth from the profile `audio` section
        // (best-effort: a missing/disabled section silently keeps the sampled path).
        self.audio.enable_synth_from_profile(cfg.audio.as_ref());
        self.spawned = true;
        Ok(id)
    }

    fn spawn(&mut self, cfg: &VehicleConfig) -> u32 {
        let y = vehicle_physics_engine::default_spawn_height(cfg);
        let spawn = Transform3D::new(Vec3::new(0.0, y, 0.0), Mat3::IDENTITY);
        self.world.spawn_vehicle(
            cfg.clone(),
            spawn,
            self.config.vehicle_scene.clone(),
            self.config.track_scene.clone(),
        )
    }

    pub fn config(&self) -> &CoreConfig {
        &self.config
    }

    pub fn world_time(&self) -> f64 {
        self.world.time
    }

    pub fn entity_count(&self) -> usize {
        self.world.entities.len()
    }

    pub fn register_module(&mut self, m: Box<dyn SimModule>) {
        self.registry.register(m);
    }

    pub fn module_names(&self) -> Vec<String> {
        self.registry
            .names()
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }

    pub fn audio_healthy(&self) -> bool {
        self.audio.healthy()
    }

    /// Whether the procedural powertrain synth is driving the engine path.
    pub fn audio_synth_enabled(&self) -> bool {
        self.audio.synth_enabled()
    }

    /// True only when the packaged GF509 continuous runtime initialized.
    pub fn audio_gf509_enabled(&self) -> bool {
        self.audio.gf509_enabled()
    }

    /// Tri-ray samples for flat ground at the entity's current pose (headless/CLI
    /// and parity tests use these when Godot has no scene to raycast).
    pub fn flat_samples(&self, id: u32) -> [TriRaycastSample; 4] {
        self.world
            .entities
            .iter()
            .find(|e| e.id == id)
            .map(|e| game_sim::world::flat_ground_samples(&e.sim))
            .unwrap_or_default()
    }

    /// Advance everything one fixed step on the FORCE path (in-engine) and return
    /// the orchestrated frame. The physics solver is fed the caller's real raycasts
    /// (tri-ray samples); gravity and rigid-body integration stay in Godot.
    pub fn step(
        &mut self,
        id: u32,
        body: BodyKinematics,
        input: &VehicleInput,
        aids_mask: u32,
        samples: &[TriRaycastSample; 4],
        dt: f64,
    ) -> &CoreFrame {
        self.step_with_underfloor_mode(
            id,
            body,
            input,
            aids_mask,
            samples,
            &underfloor::UnderfloorSample::default(),
            AeroProbeMode::Disabled,
            dt,
        )
    }

    pub fn step_with_underfloor(
        &mut self,
        id: u32,
        body: BodyKinematics,
        input: &VehicleInput,
        aids_mask: u32,
        samples: &[TriRaycastSample; 4],
        underfloor_sample: &underfloor::UnderfloorSample,
        dt: f64,
    ) -> &CoreFrame {
        self.step_with_underfloor_mode(
            id, body, input, aids_mask, samples, underfloor_sample, AeroProbeMode::Measured, dt,
        )
    }

    fn step_with_underfloor_mode(
        &mut self,
        id: u32,
        body: BodyKinematics,
        input: &VehicleInput,
        aids_mask: u32,
        samples: &[TriRaycastSample; 4],
        underfloor_sample: &underfloor::UnderfloorSample,
        probe_mode: AeroProbeMode,
        dt: f64,
    ) -> &CoreFrame {
        let dt = dt.clamp(1.0 / 1000.0, 1.0 / 20.0);
        self.world.time += dt;
        let mut frame = CoreFrame {
            time_ms: (self.world.time * 1000.0).round() as i64,
            ..CoreFrame::default()
        };
        let mut mechanical_audio = audio_telemetry::MechanicalAudioState::default();
        self.underfloor.step(
            underfloor_sample,
            Some(&body),
            &self.config.underfloor_contact,
            dt,
        );
        let aero_environment = AeroEnvironment {
            clearance_m: self.underfloor.filtered_clearance_m,
            valid_mask: self.underfloor.valid_mask,
            rake_rad: self.underfloor.rake_rad,
            roll_rad: self.underfloor.roll_rad,
            bottoming_mask: self.underfloor.active_probe_mask,
            contact_confidence: self.underfloor.contact_confidence,
        };

        if let Some(ent) = self.world.entities.iter_mut().find(|e| e.id == id) {
            // The CPU side (Godot / driving_aids) is the authority on the aids mask.
            // Apply the FULL 8-bit mask into the solver each step — no toggle pulse,
            // no frozen defaults (fixes "no ESP/TCS" and the first-frame TC spurious
            // toggle).
            ent.sim.aids = AidsMask::from_bits(aids_mask);
            ent.aids.traction_control = ent.sim.aids.traction_control;
            ent.aids.steering_slip_assist = ent.sim.aids.steering_slip_assist;
            ent.aids.abs = ent.sim.aids.abs;
            ent.aids.stability = ent.sim.aids.stability;
            // A missed live ray can mean airborne/out of range. Preserve that
            // observation; only the explicit no-probe API reports Disabled.
            ent.sim.aero_probe_mode = probe_mode;
            let (forces, telem) =
                ent.sim
                    .solve_external_with_aero(body, input, samples, &aero_environment, dt);
            ent.last = Some(telem);
            Self::fill_frame_from_entity(&mut frame, ent, dt);
            frame.force = [
                forces.force_world.x,
                forces.force_world.y,
                forces.force_world.z,
            ];
            frame.torque = [
                forces.torque_world.x + self.underfloor.torque_world[0],
                forces.torque_world.y + self.underfloor.torque_world[1],
                forces.torque_world.z + self.underfloor.torque_world[2],
            ];
            frame.force[0] += self.underfloor.force_world[0];
            frame.force[1] += self.underfloor.force_world[1];
            frame.force[2] += self.underfloor.force_world[2];
            frame.throttle = input.throttle;
            mechanical_audio = audio_telemetry::MechanicalAudioState::from_physics(&ent.sim.state.powertrain, &ent.sim.config);
        }
        let surface = dominant_surface(samples);
        let slip = frame.front_slip.abs().max(frame.rear_slip.abs()) as f32;
        frame.underfloor_clearance_m = self.underfloor.filtered_clearance_m;
        frame.underfloor_valid_mask = self.underfloor.valid_mask;
        frame.underfloor_scrape_phase = self.underfloor.scrape_phase as i32;
        frame.underfloor_min_clearance_m = self.underfloor.minimum_clearance_m;
        frame.underfloor_rake_rad = self.underfloor.rake_rad;
        frame.underfloor_roll_rad = self.underfloor.roll_rad;
        frame.underfloor_contact_confidence = self.underfloor.contact_confidence;
        frame.underfloor_scrape_intensity = self.underfloor.scrape_intensity;
        frame.underfloor_compression_m = self.underfloor.compression_m;
        frame.underfloor_closing_speed_m_s = self.underfloor.closing_speed_m_s;
        frame.underfloor_normal_force_n = self.underfloor.normal_force_n;
        frame.underfloor_bottoming_phase = self.underfloor.bottoming_phase.map(|v| v as i32);
        frame.underfloor_active_probe_mask = self.underfloor.active_probe_mask;
        frame.underfloor_total_normal_force_n = self.underfloor.total_normal_force_n;
        frame.underfloor_max_probe_force_n = self.underfloor.max_probe_force_n;
        frame.underfloor_force_center_local = self.underfloor.force_center_local;
        frame.underfloor_bottoming_torque = self.underfloor.torque_world;
        frame.underfloor_dissipated_energy_j = self.underfloor.dissipated_energy_j;
        frame.underfloor_rigid_contact_blend = self.underfloor.rigid_contact_blend;
        let scrape_active = matches!(
            self.underfloor.scrape_phase,
            underfloor::ScrapePhase::Impact | underfloor::ScrapePhase::Scraping
        );
        self.audio.set_scrape_state(
            scrape_active,
            self.underfloor.scrape_intensity as f32,
            underfloor_sample.rigid_contact.tangential_speed_m_s as f32,
            self.underfloor.onset_strength as f32,
        );
        self.finish_frame(dt, frame, surface, slip, mechanical_audio)
    }

    /// Advance everything one fixed step on the STANDALONE path (headless, same as
    /// `game_sim`'s `step`): the core integrates the pose internally using the
    /// supplied samples. Use this for CLI / parity / tests where Godot has no body.
    pub fn step_standalone(
        &mut self,
        id: u32,
        input: &game_sim::DriverInput,
        samples: &[TriRaycastSample; 4],
        dt: f64,
    ) -> &CoreFrame {
        let dt = dt.clamp(1.0 / 1000.0, 1.0 / 20.0);
        self.world.step_with_samples(id, input, samples, dt);
        let mut frame = CoreFrame {
            time_ms: (self.world.time * 1000.0).round() as i64,
            ..CoreFrame::default()
        };
        let mut mechanical_audio = audio_telemetry::MechanicalAudioState::default();
        if let Some(ent) = self.world.entities.iter().find(|e| e.id == id) {
            Self::fill_frame_from_entity(&mut frame, ent, dt);
            mechanical_audio = audio_telemetry::MechanicalAudioState::from_physics(&ent.sim.state.powertrain, &ent.sim.config);
        }
        frame.throttle = input.throttle;
        let surface = dominant_surface(samples);
        let slip = frame.front_slip.abs().max(frame.rear_slip.abs()) as f32;
        self.finish_frame(dt, frame, surface, slip, mechanical_audio)
    }

    /// Copy the entity's telemetry + pose/velocity into `frame` (physics-agnostic
    /// tail shared by the force path and the standalone path). Static: it must not
    /// borrow `self` while the caller holds a borrow into `self.world`.
    fn fill_frame_from_entity(
        frame: &mut CoreFrame,
        ent: &game_sim::world::VehicleEntity,
        _dt: f64,
    ) {
        if let Some(et) = ent.last.as_ref().map(EntityTelemetry::from) {
            frame.speed_kmh = et.speed_kmh;
            frame.rpm = et.rpm;
            frame.gear = et.gear as i32;
            frame.steer = et.steering;
            frame.lat_g = et.lat_g;
            frame.long_g = et.long_g;
            frame.vert_g = et.vert_g;
            frame.fl_comp_mm = et.fl_comp_mm;
            frame.fr_comp_mm = et.fr_comp_mm;
            frame.rl_comp_mm = et.rl_comp_mm;
            frame.rr_comp_mm = et.rr_comp_mm;
            frame.front_slip = et.front_slip;
            frame.rear_slip = et.rear_slip;
            frame.tc_active = et.tc_active;
            frame.drive_torque = et.drive_torque;
        }
        frame.tc_active = ent.sim.state.powertrain.tc_active;
        frame.tc_enabled = ent.sim.aids.traction_control;
        frame.tc_eligible = ent.sim.state.powertrain.tc_eligible;
        frame.tc_gear_authority = ent.sim.state.powertrain.tc_gear_authority;
        frame.tc_slip_target = ent.sim.state.powertrain.tc_slip_target;
        frame.tc_raw_cut_ratio = ent.sim.state.powertrain.tc_raw_cut_ratio;
        frame.tc_cut_ratio = ent.sim.state.powertrain.tc_cut_ratio;
        frame.tc_slip_ratio = ent.sim.state.powertrain.tc_slip_ratio;
        frame.wheel_drive_torque_pre_tc_nm = ent.sim.state.powertrain.drive_torques_pre_tc;
        frame.wheel_drive_torque_nm = ent.sim.state.powertrain.drive_torques;
        frame.drive_torque = frame.wheel_drive_torque_nm.iter().sum();
        frame.net_drive_power_w = frame
            .wheel_drive_torque_nm
            .iter()
            .zip(ent.sim.state.tires.wheels.iter())
            .map(|(torque, wheel)| torque * wheel.spin)
            .sum();
        frame.pre_tc_drive_power_w = frame
            .wheel_drive_torque_pre_tc_nm
            .iter()
            .zip(ent.sim.state.tires.wheels.iter())
            .map(|(torque, wheel)| torque * wheel.spin)
            .sum();
        for i in 0..4 {
            frame.wheel_slip_ratio[i] = ent.sim.state.tires.wheels[i].slip_ratio;
            frame.wheel_slip_angle_rad[i] = ent.sim.state.tires.wheels[i].slip_angle_rad;
            frame.wheel_contact_fraction[i] = ent.sim.state.suspension.wheels[i].contact_fraction;
            frame.wheel_normal_force_n[i] = ent.sim.state.suspension.wheels[i].total_normal_force;
        }
        let pose = &ent.sim.state.transform;
        let lv = ent.sim.state.linear_velocity;
        let av = ent.sim.state.angular_velocity;
        frame.px = pose.origin.x;
        frame.py = pose.origin.y;
        frame.pz = pose.origin.z;
        frame.yaw = yaw_from_transform(pose);
        frame.lvx = lv.x;
        frame.lvy = lv.y;
        frame.lvz = lv.z;
        frame.avx = av.x;
        frame.avy = av.y;
        frame.avz = av.z;
        let aero = ent.sim.state.aero;
        frame.aero_total_downforce_n = aero.total_downforce;
        frame.aero_raw_downforce_n = aero.raw_downforce;
        frame.aero_front_downforce_n = aero.front_downforce;
        frame.aero_floor_downforce_n = aero.diffuser_downforce;
        frame.aero_rear_downforce_n = aero.rear_downforce;
        frame.aero_drag_n = aero.drag_force;
        frame.aero_front_wing_angle_deg = aero.front_wing_angle_deg;
        frame.aero_rear_wing_angle_deg = aero.rear_wing_angle_deg;
        frame.aero_front_wing_cl = aero.front_wing_cl;
        frame.aero_rear_wing_cl = aero.rear_wing_cl;
        frame.aero_floor_height_factor = aero.floor_height_factor;
        frame.aero_floor_rake_factor = aero.floor_rake_factor;
        frame.aero_floor_seal_factor = aero.floor_seal_factor;
        frame.aero_diffuser_stall_factor = aero.diffuser_stall_factor;
        frame.aero_global_limit_factor = aero.global_limit_factor;
        frame.aero_load_ratio = aero.load_ratio;
        frame.aero_balance_front = aero.balance_front;

        // Tire pressure + thermal state (WheelIndex order FL/FR/RL/RR).
        for (i, w) in ent.sim.state.tire_thermal.wheels.iter().enumerate() {
            frame.tire_pressure_kpa[i] = w.pressure_kpa_gauge;
            frame.tire_tread_inner_c[i] = w.tread_inner_c;
            frame.tire_tread_center_c[i] = w.tread_center_c;
            frame.tire_tread_outer_c[i] = w.tread_outer_c;
            frame.tire_carcass_c[i] = w.carcass_c;
            frame.tire_gas_c[i] = w.gas_c;
        }
        for (i, w) in ent.sim.state.brake_thermal.wheels.iter().enumerate() {
            frame.brake_disc_c[i] = w.disc_c;
            frame.brake_rim_c[i] = w.rim_c;
            frame.brake_efficiency[i] = w.efficiency;
            frame.duct_mass_flow_kg_s[i] = w.duct.mass_flow_kg_s;
            frame.duct_drag_n[i] = w.duct.drag_force_n;
            frame.total_brake_duct_drag_n += w.duct.drag_force_n;
        }
        frame.brake_optimal_min_c = ent.sim.config.brake_thermal.optimal_min_temperature_c;
        frame.brake_optimal_max_c = ent.sim.config.brake_thermal.optimal_max_temperature_c;
        frame.brake_fade_start_c = ent.sim.config.brake_thermal.fade_start_temperature_c;
        frame.brake_critical_c = ent.sim.config.brake_thermal.critical_temperature_c;
        for (i, w) in ent.sim.state.brake_thermal.wheels.iter().enumerate() {
            frame.brake_torque_nm[i] = w.applied_brake_torque_nm;
            frame.brake_spin_pre_rad_s[i] = w.wheel_spin_pre_rad_s;
            frame.brake_spin_post_rad_s[i] = w.wheel_spin_post_rad_s;
            frame.brake_power_w[i] = w.brake_power_w;
            frame.brake_energy_j[i] = w.brake_energy_j;
            frame.brake_natural_cooling_w_k[i] = w.natural_cooling_w_k;
            frame.brake_speed_cooling_w_k[i] = w.speed_cooling_w_k;
        }
    }

    /// Shared tail of every step: drive the audio from the frame telemetry, tick the
    /// modules, publish the frame, return it.
    fn finish_frame(
        &mut self,
        dt: f64,
        mut frame: CoreFrame,
        surface: SurfaceType,
        slip: f32,
        mechanical_audio: audio_telemetry::MechanicalAudioState,
    ) -> &CoreFrame {
        // --- audio driven from the SAME tick (no round-trip) ------------------
        self.audio.set_tire_scrub_state(
            frame.wheel_slip_ratio.map(|v| v as f32),
            frame.wheel_slip_angle_rad.map(|v| v as f32),
            frame.wheel_contact_fraction.map(|v| v as f32),
            frame.wheel_normal_force_n.map(|v| v as f32),
            frame.speed_kmh as f32,
            surface,
        );
        self.audio.set_physical_state(
            frame.rpm,
            self.config.idle_rpm,
            self.config.max_rpm,
            frame.throttle as f32,
            frame.speed_kmh,
            frame.gear,
            mechanical_audio,
            dt as f32,
            slip,
            surface,
        );
        frame.audio = self.audio.readouts();

        // --- modules tick (deterministic, core clock only) --------------------
        {
            let mut sink = |_: crate::module::ModuleSignal| {};
            let mut ctx = ModuleCtx {
                fixed_dt: dt,
                clock_ms: frame.time_ms,
                latest: &frame,
                emit: &mut sink,
            };
            self.registry.tick_all(&mut ctx);
        }
        frame.modules = self
            .registry
            .snapshots()
            .into_iter()
            .map(|(name, payload)| ModuleOutput { name, payload })
            .collect();

        self.frame = frame.clone();
        *self.latest.write().expect("latest RwLock poisoned") = Arc::new(frame);
        &self.frame
    }

    /// Latest published frame (immutable view for any thread).
    pub fn latest_frame(&self) -> Arc<CoreFrame> {
        self.latest.read().expect("latest RwLock poisoned").clone()
    }

    /// Render `n` stereo audio frames into `out_l`/`out_r` (0 on no mixer).
    pub fn audio_render(&mut self, out_l: &mut [f32], out_r: &mut [f32], n: usize) -> usize {
        self.audio.render(out_l, out_r, n)
    }

    /// Fire a named one-shot by legacy code (0..10). Returns true if consumed.
    pub fn audio_trigger(&mut self, code: i32) -> bool {
        self.audio.trigger_code(code)
    }

    /// Apply the listener/ambient downlink. Returns true when the mixer is present.
    pub fn audio_set_ambient(
        &mut self,
        distance_m: f32,
        tc_cut_ratio: f32,
        limiter_active: bool,
    ) -> bool {
        self.audio
            .set_ambient(distance_m, tc_cut_ratio, limiter_active)
    }

    pub fn audio_readouts(&mut self) -> frame::AudioReadouts {
        self.audio.readouts()
    }

    /// Authoritative sim snapshot (game_sim) — used for byte parity with `game_sim`.
    pub fn core_snapshot(&self) -> Snapshot {
        self.world.snapshot()
    }

    /// Orchestrated snapshot incl. module contributions (the serializable contract).
    pub fn facade_snapshot(&self) -> FacadeSnapshot {
        FacadeSnapshot {
            core: self.world.snapshot(),
            modules: self.registry.snapshots(),
        }
    }

    /// Reset every entity to a pose/yaw (recreates its simulator, mirroring the
    /// legacy `f1_94_physics_reset`) and resets every module.
    pub fn reset(&mut self, x: f64, y: f64, z: f64, yaw: f64) {
        for ent in self.world.entities.iter_mut() {
            ent.sim = vehicle_physics_engine::VehicleSimulator::new(
                ent.sim.config.clone(),
                Vec3::new(x, y, z),
                yaw,
            );
        }
        self.registry.reset_all();
        self.underfloor.reset();
        self.audio.set_scrape_state(false, 0.0, 0.0, 0.0);
        self.audio.reset();
    }

    /// Apply a runtime-tunable config (mirror of the legacy `FfiRuntimeConfig`) to
    /// an entity's simulator. The CPU side (GDScript tuning panel / setters) writes
    /// JSON-tunable parameters into the facade even in `bridge_controlled` mode —
    /// previously those setters landed on the legacy sim that is NOT executed.
    pub fn apply_runtime_config(
        &mut self,
        id: u32,
        cfg: &vehicle_physics_engine::FfiRuntimeConfig,
    ) -> bool {
        let Some(ent) = self.world.entities.iter_mut().find(|e| e.id == id) else {
            return false;
        };
        let ok = vehicle_physics_engine::apply_runtime_config_to_sim(&mut ent.sim, cfg);
        ent.aids.traction_control = ent.sim.aids.traction_control;
        ent.aids.steering_slip_assist = ent.sim.aids.steering_slip_assist;
        ent.aids.abs = ent.sim.aids.abs;
        ent.aids.stability = ent.sim.aids.stability;
        ok
    }
}

/// Derive the dominant surface token from the tick's tri-ray samples. Precedence and
/// counting mirror the legacy C++ `detect_surface` (rumble > grass > sand > asphalt).
fn dominant_surface(samples: &[TriRaycastSample; 4]) -> SurfaceType {
    use SurfaceType::*;
    let mut rumble = 0;
    let mut grass = 0;
    let mut sand = 0;
    for s in samples {
        if !s.center.is_colliding {
            continue;
        }
        match s.center.surface {
            Curb => rumble += 1,
            Grass => grass += 1,
            Gravel | Dirt | Sand => sand += 1,
            _ => {}
        }
    }
    if rumble > 0 {
        Curb
    } else if grass > 0 {
        Grass
    } else if sand > 0 {
        Sand
    } else {
        Road
    }
}
