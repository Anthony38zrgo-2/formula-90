use race_intelligence_engine::local_driver::driver_tuning::load_driver_tuning;
use race_intelligence_engine::local_driver::longitudinal_control::LongitudinalController;
use race_intelligence_engine::local_driver::longitudinal_profile::LongitudinalProfile;
use race_intelligence_engine::track::speed_profile::SpeedProfileSample;
use race_intelligence_engine::track::vehicle_performance::PerformanceSample;

fn performance() -> PerformanceSample {
    PerformanceSample {
        speed_metres_per_second: 40.0,
        lateral_acceleration_metres_per_second_squared: 15.0,
        braking_deceleration_metres_per_second_squared: 31.0,
        coasting_deceleration_metres_per_second_squared: 1.0,
    }
}

fn profile() -> LongitudinalProfile {
    LongitudinalProfile::new(
        [40.0, 40.0, 20.0, 20.0]
            .into_iter()
            .enumerate()
            .map(|(index, speed)| SpeedProfileSample {
                arc_length_metres: index as f64 * 100.0,
                target_speed_metres_per_second: speed,
                curvature_per_metre: 0.0,
            })
            .collect(),
        400.0,
    )
}

#[test]
fn ordinary_braking_releases_accelerator_progressively_without_overlapping_pedals() {
    let tuning = load_driver_tuning(include_str!(
        "../../../data/race_intelligence/unrestricted_driver_tuning.json"
    ))
    .unwrap();
    let mut controller = LongitudinalController::default();
    for _ in 0..120 {
        controller.advance(&tuning, 20.0, 0.0, performance(), 1.0 / 60.0);
    }
    assert_eq!(controller.signed_pedal_fraction, 1.0);
    let mut saw_coasting_transition = false;
    let mut saw_braking = false;
    for _ in 0..120 {
        let previous = controller.signed_pedal_fraction;
        let (throttle, brake) = controller.advance(&tuning, 0.0, -15.0, performance(), 1.0 / 60.0);
        assert!(throttle == 0.0 || brake == 0.0);
        assert!((controller.signed_pedal_fraction - previous).abs() <= 4.0 / 60.0 + 1e-12);
        saw_coasting_transition |= throttle > 0.0 && throttle < 1.0;
        saw_braking |= brake > 0.1;
    }
    assert!(saw_coasting_transition && saw_braking);
}

#[test]
fn planned_deceleration_does_not_toggle_at_the_former_speed_error_threshold() {
    let tuning = load_driver_tuning(include_str!(
        "../../../data/race_intelligence/unrestricted_driver_tuning.json"
    ))
    .unwrap();
    let mut controller = LongitudinalController::default();
    for index in 0..240 {
        let error = if index % 2 == 0 { 0.49 } else { 0.51 };
        let (throttle, brake) =
            controller.advance(&tuning, error, -15.0, performance(), 1.0 / 60.0);
        assert!(throttle == 0.0 || brake == 0.0);
        if index > 30 {
            assert_eq!(throttle, 0.0);
            assert!(brake > 0.3);
        }
    }
}

#[test]
fn brake_entry_and_release_have_separate_thresholds() {
    let tuning = load_driver_tuning(include_str!(
        "../../../data/race_intelligence/unrestricted_driver_tuning.json"
    ))
    .unwrap();
    let mut controller = LongitudinalController::default();
    for _ in 0..30 {
        controller.advance(&tuning, 0.0, -1.45, performance(), 1.0 / 60.0);
    }
    assert!(!controller.braking_engaged);
    controller.advance(&tuning, 0.0, -2.0, performance(), 1.0 / 60.0);
    assert!(controller.braking_engaged);
    controller.advance(&tuning, 0.0, -1.45, performance(), 1.0 / 60.0);
    assert!(controller.braking_engaged);
    controller.advance(&tuning, 0.0, -1.1, performance(), 1.0 / 60.0);
    assert!(!controller.braking_engaged);
}

#[test]
fn anticipation_applies_the_brake_before_the_original_braking_boundary() {
    let tuning = load_driver_tuning(include_str!(
        "../../../data/race_intelligence/unrestricted_driver_tuning.json"
    ))
    .unwrap();
    let profile = profile();
    let mut controller = LongitudinalController::default();
    for _ in 0..120 {
        controller.advance(&tuning, 20.0, 0.0, performance(), 1.0 / 60.0);
    }
    let mut first_braking_arc = None;
    for index in 0..100 {
        let arc = 50.0 + index as f64 * 40.0 / 60.0;
        let (target, acceleration) = profile.demand_at(arc, 40.0, &tuning.longitudinal_control);
        let (_, brake) = controller.advance(
            &tuning,
            target - 40.0,
            acceleration,
            performance(),
            1.0 / 60.0,
        );
        if brake > 0.01 && first_braking_arc.is_none() {
            first_braking_arc = Some(arc);
        }
    }
    assert!(first_braking_arc.unwrap() < 100.0);
}

#[test]
fn anticipation_never_raises_the_original_speed_envelope() {
    let tuning = load_driver_tuning(include_str!(
        "../../../data/race_intelligence/unrestricted_driver_tuning.json"
    ))
    .unwrap();
    let profile = profile();
    for index in -400..800 {
        let arc = index as f64;
        let (target, _) = profile.demand_at(arc, 50.0, &tuning.longitudinal_control);
        assert!(target.powi(2) <= profile.speed_squared_at(arc) + 1e-10);
    }
}

#[test]
fn acceleration_regularization_and_lap_wrap_remain_continuous() {
    let tuning = load_driver_tuning(include_str!(
        "../../../data/race_intelligence/unrestricted_driver_tuning.json"
    ))
    .unwrap();
    let profile = profile();
    for arc in [0.0, 100.0, 200.0, 300.0, 400.0] {
        let before = profile.demand_at(arc - 1e-6, 40.0, &tuning.longitudinal_control);
        let after = profile.demand_at(arc + 1e-6, 40.0, &tuning.longitudinal_control);
        assert!((before.0 - after.0).abs() < 1e-5);
        assert!((before.1 - after.1).abs() < 1e-5);
    }
}

#[test]
fn urgent_overspeed_braking_keeps_a_faster_bounded_response() {
    let tuning = load_driver_tuning(include_str!(
        "../../../data/race_intelligence/unrestricted_driver_tuning.json"
    ))
    .unwrap();
    let mut controller = LongitudinalController::default();
    for _ in 0..120 {
        controller.advance(&tuning, 20.0, 0.0, performance(), 1.0 / 60.0);
    }
    let mut first_braking_seconds = None;
    for index in 0..30 {
        let previous = controller.signed_pedal_fraction;
        let (throttle, brake) =
            controller.advance(&tuning, -20.0, -25.0, performance(), 1.0 / 60.0);
        assert!(throttle == 0.0 || brake == 0.0);
        assert!((previous - controller.signed_pedal_fraction).abs() <= 8.0 / 60.0 + 1e-12);
        if brake > 0.01 && first_braking_seconds.is_none() {
            first_braking_seconds = Some((index + 1) as f64 / 60.0);
        }
    }
    assert!(first_braking_seconds.unwrap() < 0.2);
}

#[test]
fn contradictory_hysteresis_configuration_is_rejected() {
    let mut document: serde_json::Value = serde_json::from_str(include_str!(
        "../../../data/race_intelligence/unrestricted_driver_tuning.json"
    ))
    .unwrap();
    document["longitudinal_control"]["brake_release_fraction"] = serde_json::json!(0.5);
    assert!(load_driver_tuning(&document.to_string()).is_err());
}
