use formula90_core::{CoreConfig, CoreFacade};
use vehicle_physics_engine::coupled_vehicle_world::{CoupledVehicleWorld, CoupledWorldSnapshot};
use vehicle_physics_engine::physical_collision_world::PhysicalCollisionWorld;
use vehicle_physics_engine::{VehicleConfig, Vec3};

fn physical_world_and_presentation() -> (CoupledVehicleWorld, CoreFacade, u32) {
    physical_world_and_presentation_with_audio(false)
}

fn physical_world_and_presentation_with_audio(audio_enabled: bool) -> (CoupledVehicleWorld, CoreFacade, u32) {
    let profile_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/vehicles/f1_2030/f1_2030_v10_geometric.json");
    let configuration = VehicleConfig::from_json_str(&std::fs::read_to_string(&profile_path).unwrap()).unwrap();
    let terrain = PhysicalCollisionWorld { source_digest: "analytical_empty_world".into(), shapes: Vec::new() };
    let mut world = CoupledVehicleWorld::new(terrain, configuration.coupled_world.clone().unwrap()).unwrap();
    world.register_vehicle(configuration, Vec3::new(0.0, 10.0, 0.0), 0.0).unwrap();
    let mut presentation = CoreFacade::new(CoreConfig {
        bank_dir: Some(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sounds/banks/commons")),
        config_json_path: Some(profile_path), use_canonical: false, enable_audio: audio_enabled, ..CoreConfig::default()
    }).unwrap();
    let entity_identifier = presentation.ensure_spawned().unwrap();
    (world, presentation, entity_identifier)
}

#[test]
fn physical_snapshot_prevents_a_second_clock_and_legacy_services() {
    let (mut world, mut presentation, entity_identifier) = physical_world_and_presentation();
    world.advance_host_interval(0.002).unwrap();
    let snapshot = world.snapshots().unwrap().remove(0);
    presentation.accept_physical_world_snapshot(entity_identifier, &snapshot).unwrap();
    assert_eq!(presentation.world_time(), world.time_seconds);
    let before = presentation.latest_frame();
    let samples = presentation.flat_samples(entity_identifier);
    presentation.step_standalone(entity_identifier, &Default::default(), &samples, 1.0);
    assert_eq!(presentation.world_time(), world.time_seconds);
    assert_eq!(presentation.latest_frame().time_ms, before.time_ms);
    assert!(!presentation.set_fuel_kg(entity_identifier, 1.0));
    assert!(!presentation.replace_tires(entity_identifier));
    presentation.reset(100.0, 100.0, 100.0, 1.0);
    assert_eq!(presentation.world_time(), world.time_seconds);
    assert_eq!(snapshot.solved_suspension_corners.len(), 4);
    assert!(snapshot.solved_suspension_corners.iter().all(|corner| corner.converged));
}

#[test]
fn snapshot_identity_time_and_mass_rejections_preserve_presentation() {
    let (mut world, mut presentation, entity_identifier) = physical_world_and_presentation();
    world.advance_host_interval(0.002).unwrap();
    let snapshot = world.snapshots().unwrap().remove(0);
    presentation.accept_physical_world_snapshot(entity_identifier, &snapshot).unwrap();
    let assert_rejected = |presentation: &mut CoreFacade, invalid: CoupledWorldSnapshot| {
        let time_seconds = presentation.world_time();
        assert!(presentation.accept_physical_world_snapshot(entity_identifier, &invalid).is_err());
        assert_eq!(presentation.world_time(), time_seconds);
    };
    let mut invalid = snapshot.clone();
    invalid.entity_identifier += 1;
    assert_rejected(&mut presentation, invalid);
    let mut invalid = snapshot.clone();
    invalid.state.suspension.simulated_time_seconds = 0.0;
    assert_rejected(&mut presentation, invalid);
    let mut invalid = snapshot.clone();
    invalid.operating_mass_kilograms += 1.0;
    assert_rejected(&mut presentation, invalid);
    let mut invalid = snapshot;
    invalid.operating_mass_kilograms = f64::NAN;
    assert_rejected(&mut presentation, invalid);
}

#[test]
fn physical_snapshot_drives_the_existing_sampler_without_legacy_stepping() {
    let (mut world, mut presentation, entity_identifier) = physical_world_and_presentation_with_audio(true);
    assert!(presentation.audio_grand_prix_sampler_enabled());
    world.advance_host_interval(0.01).unwrap();
    let snapshot = world.snapshots().unwrap().remove(0);
    presentation.accept_physical_world_snapshot(entity_identifier, &snapshot).unwrap();
    let mut left_channel = [0.0; 512];
    let mut right_channel = [0.0; 512];
    assert_eq!(presentation.audio_render(&mut left_channel, &mut right_channel, 512), 512);
    assert!(left_channel.iter().chain(right_channel.iter()).all(|sample| sample.is_finite()));
    assert!(left_channel.iter().chain(right_channel.iter()).any(|sample| sample.abs() > 1e-8));
    assert_eq!(presentation.world_time(), world.time_seconds);
}
