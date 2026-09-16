//! SUS-GEO-07: build validated pose tables for a geometric profile.
//!
//! Usage (offline, no new dependencies):
//!   cargo run -p vehicle_physics_engine --example build_geo_table -- \
//!     game/data/vehicles/f1_2030/f1_2030_v10_geometric.json out/f1_2030_tables.json
//!
//! Writes `{corner: CornerTable}` for FL/FR/RL/RR with 33x9 (front) and 33x1
//! (rear) nodes. The same schema is consumed by
//! `game/scripts/vehicle/suspension_table.gd`.
use std::collections::BTreeMap;
use vehicle_physics_engine::{
    generate_table, GeometricSuspensionConfig, SuspensionModelKind, VehicleConfig, WheelIndex,
};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: build_geo_table <profile.json> <out_tables.json>");
        std::process::exit(2);
    }
    let text = std::fs::read_to_string(&args[1]).expect("read profile");
    let cfg = VehicleConfig::from_json_str(&text).expect("parse profile");
    assert_eq!(cfg.suspension_model, SuspensionModelKind::Geometric);
    let geo: &GeometricSuspensionConfig =
        cfg.geometric_suspension.as_ref().expect("physical geometry");
    let mut tables = BTreeMap::new();
    for wheel in WheelIndex::ALL {
        let (n_q, n_r) = if wheel.is_front() { (33, 9) } else { (33, 1) };
        let t = generate_table(geo, wheel, n_q, n_r, 0.02).expect("generate");
        let (hub_err, ang_err) = vehicle_physics_engine::parity_error(&t, geo);
        eprintln!("{wheel:?}: q [{:.4}, {:.4}], parity hub {hub_err:.2e} m angle {ang_err:.2e}", t.q_min, t.q_max);
        assert!(
            hub_err < vehicle_physics_engine::TABLE_PARITY_HUB_M,
            "{wheel:?} table parity"
        );
        tables.insert(format!("{wheel:?}"), t.to_json_value());
    }
    let out = serde_json::json!({ "version": 1, "tables": tables });
    std::fs::write(&args[2], serde_json::to_string_pretty(&out).expect("json"))
        .expect("write tables");
    println!("wrote {}", args[2]);
}
