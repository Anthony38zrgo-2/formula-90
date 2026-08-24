//! CLI validator for Formula-90 background presets.
//!
//! Usage:
//! ```
//! cargo run --bin bg_validate -- path/to/background.json
//! ```
//!
//! Validates the preset JSON against all rules and prints the result.
//! Exit code 0 = valid, 1 = errors found.

use skybox_engine::{parse_preset_json, validate_preset};
use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: bg_validate <preset.json>");
        eprintln!();
        eprintln!("Validates a Formula-90 background preset JSON file.");
        process::exit(2);
    }

    let path = &args[1];

    let json_str = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[ERROR] Could not read '{}': {}", path, e);
            process::exit(2);
        }
    };

    let preset = match parse_preset_json(&json_str) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[FAIL] Parse error in '{}': {}", path, e);
            process::exit(1);
        }
    };

    println!("Preset: {} ({})", preset.id, preset.display_name);
    println!(
        "Skybox: {}",
        preset
            .skybox
            .as_ref()
            .map(|s| format!("{:?}", s.mode))
            .unwrap_or_else(|| "none".to_string())
    );
    println!("Layers: {}", preset.layers.len());

    let result = validate_preset(&preset);

    if result.is_valid {
        println!("[PASS] Preset is valid.");
        if !result.warnings.is_empty() {
            for w in &result.warnings {
                println!("[WARN] {}", w);
            }
        }
        process::exit(0);
    } else {
        eprintln!("[FAIL] Preset has {} error(s):", result.errors.len());
        for err in &result.errors {
            eprintln!("  - {}", err);
        }
        process::exit(1);
    }
}
