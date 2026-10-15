use std::path::PathBuf;
use vehicle_audio_engine::grand_prix_sample_bank::GrandPrixSampleBank;

fn artificial_engine_bank_path() -> PathBuf {
    let crate_directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_directory.join("../../sounds/banks/v10-bank-ai")
}

#[test]
fn artificial_engine_bank_loads_through_production_loader() {
    let bank_path = artificial_engine_bank_path();
    let bank = GrandPrixSampleBank::load(&bank_path).expect("AI engine bank must load");
    assert_eq!(bank.bank_id, "v10_ai_engine_bank");
    assert_eq!(bank.loops.len(), 6);
    assert_eq!(bank.transitions.len(), 5);
    assert_eq!(bank.coast_loops.len(), 4);
    assert_eq!(bank.coast_transitions.len(), 3);
    assert!(bank.transmission_whine.is_none());
    assert!(bank.events.is_empty());
    assert!(bank.groups.is_empty());
    assert_eq!(bank.coverage_minimum_revolutions_per_minute, 4500.0);
    assert_eq!(bank.coverage_maximum_revolutions_per_minute, 18000.0);
    for decoded_loop in bank.loops.iter().chain(bank.coast_loops.iter()) {
        assert!(!decoded_loop.pcm.is_empty());
        assert!(decoded_loop.calibrated_gain > 0.0);
    }
}
