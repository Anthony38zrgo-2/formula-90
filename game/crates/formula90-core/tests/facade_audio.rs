//! Audio integration tests for the facade.

use formula90_core::{CoreConfig, CoreFacade};
use game_sim::DriverInput;

const DT: f64 = 1.0 / 120.0;

fn bank_dir() -> std::path::PathBuf {
    // Crates live at game/crates/<crate>; the bank is at game/sounds/banks/v10_vehicle.
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sounds/banks/v10_vehicle")
}

fn run_steps(facade: &mut CoreFacade, id: u32, throttle: f64, steps: usize) {
    let input = DriverInput {
        throttle,
        ..Default::default()
    };
    for _ in 0..steps {
        let samples = facade.flat_samples(id);
        facade.step_standalone(id, &input, &samples, DT);
    }
}

/// Audio disabled by config -> telemetry-only, no mixer.
#[test]
fn audio_disabled_degrades_to_telemetry_only() {
    let mut cfg = CoreConfig::default();
    cfg.enable_audio = false;
    let mut facade = CoreFacade::new(cfg).expect("facade");
    let id = facade.ensure_spawned().expect("spawn");
    run_steps(&mut facade, id, 1.0, 30);

    assert!(!facade.audio_healthy(), "no bank -> mixer unavailable");

    let mut l = [0.0f32; 32];
    let mut r = [0.0f32; 32];
    let written = facade.audio_render(&mut l, &mut r, 32);
    assert_eq!(written, 0, "render must write 0 frames without a mixer");

    let ro = facade.audio_readouts();
    assert_eq!(ro.surface_code, 0, "default surface = asphalt");
    assert!(ro.last_rpm >= 0.0);
}

/// Externally requested one-shots are consumed and reflected in readouts.
#[test]
fn external_trigger_code_is_recorded() {
    let mut cfg = CoreConfig::default();
    cfg.enable_audio = false; // telemetry-only still records the trigger for the HUD
    let mut facade = CoreFacade::new(cfg).expect("facade");
    let id = facade.ensure_spawned().expect("spawn");
    run_steps(&mut facade, id, 1.0, 10);

    assert!(facade.audio_trigger(3), "impact_hit_1 (code 3) accepted");
    assert_eq!(facade.audio_readouts().trigger_code, 3);
}

/// End-to-end with a real bank: mixer loads, render produces frames, weights sum.
#[test]
fn audio_enabled_mixer_renders_from_facade_telemetry() {
    let mut cfg = CoreConfig::default();
    cfg.enable_audio = true;
    cfg.bank_dir = Some(bank_dir());
    let mut facade = CoreFacade::new(cfg).expect("facade with bank");
    let id = facade.ensure_spawned().expect("spawn");

    assert!(facade.audio_healthy(), "loaded bank -> mixer healthy");

    run_steps(&mut facade, id, 1.0, 60);

    // Render a chunk and confirm the mixer advances state.
    let mut l = vec![0.0f32; 256];
    let mut r = vec![0.0f32; 256];
    let written = facade.audio_render(&mut l, &mut r, 256);
    assert_eq!(written, 256, "mixer must render the requested frames");

    let all_finite = l.iter().chain(r.iter()).all(|v| v.is_finite());
    assert!(all_finite, "rendered audio must be finite");

    let ro = facade.audio_readouts();
    assert!(
        ro.last_rpm.is_finite() && ro.last_rpm >= 0.0,
        "rpm readout sane"
    );
    let weight_sum: f32 = ro.weights.iter().sum();
    assert!(
        weight_sum > 0.5,
        "band weights must sum > 0.5 after rendering, got {weight_sum}"
    );
    assert!(ro.last_engine_gain >= 0.0, "engine gain sane");
}
