//! Module registry tests: the expandability contract of the facade.

use formula90_core::modules::ai_stub::AiDirective;
use formula90_core::modules::weather_stub::WeatherState;
use formula90_core::{CoreConfig, CoreFacade, FacadeSnapshot};
use game_sim::DriverInput;

const DT: f64 = 1.0 / 120.0;

fn run(facade: &mut CoreFacade, steps: usize) -> FacadeSnapshot {
    let id = facade.ensure_spawned().expect("spawn");
    let input = DriverInput {
        throttle: 1.0,
        ..Default::default()
    };
    for _ in 0..steps {
        let samples = facade.flat_samples(id);
        facade.step_standalone(id, &input, &samples, DT);
    }
    facade.facade_snapshot()
}

#[test]
fn registered_modules_appear_in_snapshot() {
    let mut cfg = CoreConfig::default();
    cfg.modules = vec!["weather".to_string(), "ai".to_string()];
    let mut facade = CoreFacade::new(cfg).expect("facade");
    assert_eq!(
        facade.module_names(),
        vec!["weather".to_string(), "ai".to_string()]
    );

    let snap = run(&mut facade, 30);
    assert_eq!(snap.modules.len(), 2, "two module contributions expected");
    let names: Vec<&str> = snap.modules.iter().map(|(n, _)| n.as_str()).collect();
    assert!(names.contains(&"weather") && names.contains(&"ai"));

    // Payloads decode and carried an advanced core clock.
    for (name, payload) in &snap.modules {
        match name.as_str() {
            "weather" => {
                let st: WeatherState = bincode::deserialize(payload).expect("decode weather");
                assert!(st.clock_ms > 0, "weather module observed the core clock");
            }
            "ai" => {
                let st: AiDirective = bincode::deserialize(payload).expect("decode ai");
                assert!(st.clock_ms > 0, "ai module observed the core clock");
            }
            other => panic!("unexpected module '{other}'"),
        }
    }
}

#[test]
fn unknown_module_name_is_ignored_forward_compatible() {
    let mut cfg = CoreConfig::default();
    cfg.modules = vec!["future_module_not_here".to_string()];
    let facade = CoreFacade::new(cfg).expect("facade");
    assert_eq!(
        facade.module_names().len(),
        0,
        "unknown modules are ignored"
    );
}

#[test]
fn reset_restores_modules_to_pristine() {
    let mut cfg = CoreConfig::default();
    cfg.modules = vec!["weather".to_string()];
    let mut facade = CoreFacade::new(cfg).expect("facade");
    let snap_after_run = run(&mut facade, 20);

    // After reset the module clock must be back at 0 for a fresh facade too.
    let id = facade.ensure_spawned().expect("spawn");
    facade.reset(0.0, 0.5, 0.0, 0.0);
    let _ = id;
    let snap_after_reset = facade.facade_snapshot();
    for (name, payload) in &snap_after_reset.modules {
        if name == "weather" {
            let st: WeatherState = bincode::deserialize(payload).expect("decode weather");
            // No tick since reset, but the module was reset to pristine (clock 0).
            assert_eq!(st.clock_ms, 0, "reset must restore module state");
        }
    }
    // Sanity: the pre-reset run held a higher clock, so the two snapshots differ.
    assert!(
        snap_after_run.to_bytes().unwrap() != snap_after_reset.to_bytes().unwrap(),
        "reset must change one-shot module state"
    );
}
