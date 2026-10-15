use std::path::Path;

use formula90_core::{CoreConfig, CoreFacade};

#[test]
fn active_versioned_profile_initializes_and_renders_the_recorded_engine() {
    let game_directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let profile_directory = game_directory.join("data/vehicles/f1_2030/profiles/geometric");
    let audio_document: serde_json::Value = serde_json::from_slice(
        &std::fs::read(profile_directory.join("audio.json")).expect("audio document"),
    )
    .expect("parse audio document");
    assert_eq!(
        audio_document["grand_prix_sampler"]["manifest"],
        "sounds/banks/v10-v2-bank/manifest.json"
    );
    let mut core = CoreFacade::new(CoreConfig {
        bank_dir: Some(game_directory.join("sounds/banks/commons")),
        config_json_path: Some(profile_directory.join("vehicle.json")),
        use_canonical: false,
        enable_audio: true,
        ..CoreConfig::default()
    })
    .expect("active complete profile");
    let entity_identifier = core.ensure_spawned().expect("vehicle");
    assert!(core.audio_healthy());
    assert!(core.audio_grand_prix_sampler_enabled());
    let mut accumulated_energy = 0.0;
    for _ in 0..20 {
        let contact_samples = core.flat_samples(entity_identifier);
        core.step_standalone(
            entity_identifier,
            &Default::default(),
            &contact_samples,
            0.01,
        );
        let mut left_channel = [0.0; 441];
        let mut right_channel = [0.0; 441];
        assert_eq!(
            core.audio_render(&mut left_channel, &mut right_channel, 441),
            441
        );
        for sample in left_channel.iter().chain(right_channel.iter()) {
            assert!(sample.is_finite());
            accumulated_energy += sample * sample;
        }
    }
    assert!(accumulated_energy > 0.01);
}
