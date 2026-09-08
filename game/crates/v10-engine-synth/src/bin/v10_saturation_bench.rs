//! Isolated V10-018 cost measurement for AcousticScene.
//!
//! The full and graph-excised passes use identical synthetic corrected-source
//! frames. A second pair measures the complete V10Engine + AcousticScene path
//! so the module saving is not mistaken for a game-wide CPU percentage.

use std::hint::black_box;
use std::time::Instant;

use v10_engine_synth::{
    AcousticScene, AcousticSceneConfig, EngineConfig, EngineFrame, EngineInput, V10Engine,
};

const SAMPLE_RATE: u32 = 48_000;
const WARMUP_SAMPLES: usize = 48_000;
const MEASURE_SAMPLES: usize = 192_000;

#[derive(Clone, Copy)]
struct Scenario {
    name: &'static str,
    rpm: f32,
    throttle: f32,
    load: f32,
}

const SCENARIOS: [Scenario; 3] = [
    Scenario {
        name: "steady_partial_load",
        rpm: 9_000.0,
        throttle: 0.55,
        load: 0.50,
    },
    Scenario {
        name: "steady_full_load",
        rpm: 9_000.0,
        throttle: 1.0,
        load: 1.0,
    },
    Scenario {
        name: "steady_coast",
        rpm: 9_000.0,
        throttle: 0.0,
        load: 0.0,
    },
];

fn source_frame(scenario: Scenario, sample: usize) -> EngineFrame {
    let phase = (sample as f32 * scenario.rpm * 6.0 / SAMPLE_RATE as f32).rem_euclid(720.0);
    let time = sample as f32 / SAMPLE_RATE as f32;
    let pulse = (std::f32::consts::TAU * 75.0 * time).sin();
    let envelope = 0.65 + 0.35 * (std::f32::consts::TAU * 1.7 * time).sin();
    let mut frame = EngineFrame {
        throttle: scenario.throttle,
        load: scenario.load,
        pressure_derivative: pulse * envelope * (0.18 + 0.62 * scenario.load),
        crank_phase_deg: phase,
        master: pulse * (0.12 + 0.38 * scenario.load),
        fired_mask: if sample % 128 == 0 { 1 } else { 0 },
        ..EngineFrame::default()
    };
    for index in 0..10 {
        let cylinder_phase = phase + index as f32 * 72.0;
        let cylinder_pulse = (cylinder_phase.to_radians()).sin();
        frame.cylinder_pressure_derivative[index] = cylinder_pulse * (0.12 + 0.35 * scenario.load);
        frame.cylinder_blowdown[index] = cylinder_pulse.abs() * (0.08 + 0.22 * scenario.load);
        frame.cylinder_headers[index] = cylinder_pulse * (0.06 + 0.12 * scenario.load);
    }
    frame
}

fn run_scene(scenario: Scenario, excised: bool) -> (f64, u64, bool) {
    let mut scene = AcousticScene::new(SAMPLE_RATE as f32, AcousticSceneConfig::default())
        .expect("valid scene config");
    scene.set_load_saturation_excised(excised);
    for sample in 0..WARMUP_SAMPLES {
        black_box(scene.process(&source_frame(scenario, sample)));
    }
    let start = Instant::now();
    let mut checksum = 0u64;
    let mut finite = true;
    for sample in 0..MEASURE_SAMPLES {
        let frame = black_box(scene.process(&source_frame(scenario, WARMUP_SAMPLES + sample)));
        finite &= frame.output.is_finite()
            && frame.low_mid_parallel.is_finite()
            && frame.load_saturation.is_finite();
        checksum = checksum
            .wrapping_add(u64::from(frame.output.to_bits()))
            .rotate_left(7);
    }
    let elapsed = start.elapsed();
    (
        elapsed.as_secs_f64() * 1_000_000_000.0 / MEASURE_SAMPLES as f64,
        checksum,
        finite,
    )
}

fn run_end_to_end(scenario: Scenario, excised: bool) -> (f64, u64, bool) {
    let mut config = EngineConfig::default();
    config.sample_rate = SAMPLE_RATE;
    config.seed = 0xF018_0018;
    let mut engine = V10Engine::new(config).expect("valid engine config");
    let mut scene = AcousticScene::new(SAMPLE_RATE as f32, AcousticSceneConfig::default())
        .expect("valid scene config");
    scene.set_load_saturation_excised(excised);
    for sample in 0..WARMUP_SAMPLES {
        engine
            .set_input(EngineInput {
                rpm: scenario.rpm,
                throttle: scenario.throttle,
                load: scenario.load,
            })
            .expect("valid input");
        black_box(scene.process(&engine.render_sample()));
        let _ = sample;
    }
    let start = Instant::now();
    let mut checksum = 0u64;
    let mut finite = true;
    for _ in 0..MEASURE_SAMPLES {
        engine
            .set_input(EngineInput {
                rpm: scenario.rpm,
                throttle: scenario.throttle,
                load: scenario.load,
            })
            .expect("valid input");
        let frame = black_box(scene.process(&engine.render_sample()));
        finite &= frame.output.is_finite();
        checksum = checksum
            .wrapping_add(u64::from(frame.output.to_bits()))
            .rotate_left(7);
    }
    let elapsed = start.elapsed();
    (
        elapsed.as_secs_f64() * 1_000_000_000.0 / MEASURE_SAMPLES as f64,
        checksum,
        finite,
    )
}

fn saving_percent(full: f64, excised: f64) -> f64 {
    (1.0 - excised / full) * 100.0
}

fn main() {
    let output = std::env::args()
        .skip(1)
        .collect::<Vec<_>>()
        .windows(2)
        .find(|pair| pair[0] == "--output")
        .map(|pair| pair[1].clone());
    let mut rows = Vec::new();
    for scenario in SCENARIOS {
        let (scene_full, scene_full_checksum, scene_full_finite) = run_scene(scenario, false);
        let (scene_excised, scene_excised_checksum, scene_excised_finite) =
            run_scene(scenario, true);
        let (e2e_full, e2e_full_checksum, e2e_full_finite) = run_end_to_end(scenario, false);
        let (e2e_excised, e2e_excised_checksum, e2e_excised_finite) =
            run_end_to_end(scenario, true);
        rows.push(format!(
            "    {{\"name\":\"{}\",\"scene_full_ns_per_sample\":{:.3},\"scene_excised_ns_per_sample\":{:.3},\"scene_saving_percent\":{:.3},\"scene_full_checksum\":{},\"scene_excised_checksum\":{},\"scene_finite\":{},\"end_to_end_full_ns_per_sample\":{:.3},\"end_to_end_excised_ns_per_sample\":{:.3},\"end_to_end_saving_percent\":{:.3},\"end_to_end_full_checksum\":{},\"end_to_end_excised_checksum\":{},\"end_to_end_finite\":{}}}",
            scenario.name,
            scene_full,
            scene_excised,
            saving_percent(scene_full, scene_excised),
            scene_full_checksum,
            scene_excised_checksum,
            scene_full_finite && scene_excised_finite,
            e2e_full,
            e2e_excised,
            saving_percent(e2e_full, e2e_excised),
            e2e_full_checksum,
            e2e_excised_checksum,
            e2e_full_finite && e2e_excised_finite,
        ));
    }
    let payload = format!(
        "{{\n  \"sample_rate\": {},\n  \"warmup_samples\": {},\n  \"measure_samples\": {},\n  \"comparison\": \"full corrected scene versus load-saturation graph excision; parallel compressor unchanged\",\n  \"scenarios\": [\n{}\n  ]\n}}\n",
        SAMPLE_RATE,
        WARMUP_SAMPLES,
        MEASURE_SAMPLES,
        rows.join(",\n"),
    );
    if let Some(path) = output {
        std::fs::write(&path, &payload).expect("write benchmark output");
        println!("{path}");
    } else {
        print!("{payload}");
    }
}
