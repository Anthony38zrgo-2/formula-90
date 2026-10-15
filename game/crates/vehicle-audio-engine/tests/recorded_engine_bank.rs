use std::path::Path;

use vehicle_audio_engine::{
    GrandPrixEventKind, GrandPrixSampleBank, GrandPrixSampler, GrandPrixTelemetry,
};

fn sampler(directory_name: &str) -> GrandPrixSampler {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../sounds/banks")
        .join(directory_name);
    let bank = GrandPrixSampleBank::load(&directory).expect("verified bank");
    let mut sampler =
        GrandPrixSampler::new(bank, 44_100, Some(4500.0), Some(18000.0)).expect("supported range");
    sampler.set_group_gains(1.0, 0.561329504, 1.0, 0.3);
    sampler.set_gearbox_whine_gain(0.137826425);
    sampler.set_gearbox_whine_component_gains(0.12, 0.025);
    sampler
}

fn assert_non_engine_component_parity(replacement_directory: &str) {
    let mut previous = sampler("v10-v2-bank");
    let mut replacement = sampler(replacement_directory);
    let mut engine_difference = 0.0;
    let mut effects_energy = [0.0; 4];
    for block in 0..180 {
        let telemetry = GrandPrixTelemetry {
            rpm: 4500.0 + block as f64 * 75.0,
            throttle: if block % 50 < 25 { 0.9 } else { 0.0 },
            gear: 3,
            normalized_transmitted_load: 0.8,
            transmitted_torque_sign: 1,
            rev_limiter_active: (140..150).contains(&block),
            dt_seconds: 256.0 / 44100.0,
            ..GrandPrixTelemetry::default()
        };
        previous.ingest(&telemetry);
        replacement.ingest(&telemetry);
        if block == 20 || block == 60 || block == 100 || block == 140 {
            let kind = [
                GrandPrixEventKind::Upshift,
                GrandPrixEventKind::Downshift,
                GrandPrixEventKind::LiftBackfire,
                GrandPrixEventKind::Limiter,
            ][(block - 20) / 40];
            previous.ingest_direct(kind, telemetry.rpm);
            replacement.ingest_direct(kind, telemetry.rpm);
        }
        for _ in 0..256 {
            let previous_sample = previous.render_sample_components();
            let replacement_sample = replacement.render_sample_components();
            let previous_effects = [
                previous_sample.gearbox,
                previous_sample.gearbox_whine,
                previous_sample.backfire,
                previous_sample.limiter,
            ];
            let replacement_effects = [
                replacement_sample.gearbox,
                replacement_sample.gearbox_whine,
                replacement_sample.backfire,
                replacement_sample.limiter,
            ];
            assert_eq!(previous_effects, replacement_effects);
            for index in 0..4 {
                effects_energy[index] += replacement_effects[index].abs();
            }
            engine_difference += (previous_sample.engine - replacement_sample.engine).abs();
        }
    }
    assert!(engine_difference > 1.0);
    assert!(
        effects_energy.iter().all(|energy| *energy > 0.0),
        "all preserved effects must be exercised: {effects_energy:?}"
    );
}

fn assert_powered_coast_coverage_and_reset(replacement_directory: &str) {
    let mut replacement = sampler(replacement_directory);
    for revolutions_per_minute in [
        4500.0,
        7000.0,
        7800.0,
        8400.0,
        10300.0,
        11990.78809,
        13200.0,
        15162.99805,
        16000.0,
        18000.0,
    ] {
        for throttle in [0.0, 0.5, 1.0] {
            let mut renders = Vec::new();
            for _ in 0..2 {
                replacement.reset();
                replacement.ingest(&GrandPrixTelemetry {
                    rpm: revolutions_per_minute,
                    throttle,
                    gear: 3,
                    dt_seconds: 0.2,
                    ..GrandPrixTelemetry::default()
                });
                let samples: Vec<f32> = (0..8820)
                    .map(|_| replacement.render_sample_components().engine)
                    .collect();
                assert!(samples
                    .iter()
                    .all(|sample| sample.is_finite() && sample.abs() < 1.5));
                let energy: f32 = samples[4410..].iter().map(|sample| sample * sample).sum();
                assert!(
                    energy > 0.001,
                    "silent engine at {revolutions_per_minute} throttle {throttle}"
                );
                renders.push(samples);
            }
            assert_eq!(renders[0], renders[1]);
        }
    }
}

#[test]
fn replacement_preserves_every_non_engine_component_sample() {
    assert_non_engine_component_parity("v8-bank");
}

#[test]
fn recorded_engine_covers_powered_and_coast_range_and_resets() {
    assert_powered_coast_coverage_and_reset("v8-bank");
}

#[test]
fn renault_interior_preserves_every_non_engine_component_sample() {
    assert_non_engine_component_parity("renault-interior-bank");
}

#[test]
fn renault_interior_covers_powered_coast_range_and_resets() {
    assert_powered_coast_coverage_and_reset("renault-interior-bank");
}
