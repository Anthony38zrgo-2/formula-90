use formula90_core::{CoreConfig, CoreFacade};
use game_sim::DriverInput;
use vehicle_audio_engine::{AudioBackend, AudioCommandFrame};

const DT: f64 = 1.0 / 120.0;

#[test]
fn command_backend_publishes_schema_v2_and_listener_ownership() {
    let mut cfg = CoreConfig::default();
    cfg.enable_audio = true;
    cfg.audio_backend = AudioBackend::CommonV10Commands;
    let mut facade = CoreFacade::new(cfg).expect("facade");
    let id = facade.ensure_spawned().expect("spawn");
    facade.audio_set_listener_distance(8.0);
    let input = DriverInput {
        throttle: 1.0,
        ..Default::default()
    };
    for _ in 0..30 {
        let samples = facade.flat_samples(id);
        facade.step_standalone(id, &input, &samples, DT);
    }
    let frame: AudioCommandFrame =
        serde_json::from_str(&facade.audio_commands_json()).expect("command JSON");
    assert_eq!(frame.schema_version, 2);
    assert!(frame
        .voices
        .iter()
        .any(|voice| voice.id.starts_with("v10.engine_ext")));
    assert!(frame
        .voices
        .iter()
        .all(|voice| !voice.sample.contains("z4gt3 int on tr high")
            || voice.event == "v10/transmission"));
}
