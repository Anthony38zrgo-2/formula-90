//! CLI tool to validate all PSX/Retro Art Preset JSON files.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::exit;

use psx_art_plugin::parse_preset_json;

fn main() {
    let args: Vec<String> = env::args().collect();
    let target_dir = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        PathBuf::from("game/data/visuals")
    };

    println!("=== Formula-90 PSX Art Preset Validator ===");
    println!("Scanning directory: {}", target_dir.display());

    if !target_dir.exists() {
        eprintln!("Error: Directory does not exist: {}", target_dir.display());
        exit(1);
    }

    let mut total_files = 0;
    let mut failed_files = 0;

    let entries = match fs::read_dir(&target_dir) {
        Ok(e) => e,
        Err(err) => {
            eprintln!("Error reading directory: {}", err);
            exit(1);
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
            total_files += 1;
            validate_file(&path, &mut failed_files);
        }
    }

    println!("--------------------------------------------------");
    println!(
        "Summary: {} preset(s) checked, {} passed, {} failed.",
        total_files,
        total_files - failed_files,
        failed_files
    );

    if failed_files > 0 {
        exit(1);
    }
}

fn validate_file(path: &Path, failed_count: &mut usize) {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(err) => {
            eprintln!("[FAIL] Could not read {}: {}", path.display(), err);
            *failed_count += 1;
            return;
        }
    };

    match parse_preset_json(&content) {
        Ok(preset) => {
            println!(
                "[PASS] {} -> '{}' ({}x{}, BitDepth: {}, Snap: {:.2}, Affine: {:.2})",
                path.file_name().unwrap().to_string_lossy(),
                preset.profile_name,
                preset.display.internal_width,
                preset.display.internal_height,
                preset.color.bit_depth,
                preset.geometry.vertex_snap_distance,
                preset.geometry.affine_texture_strength
            );
        }
        Err(err) => {
            eprintln!("[FAIL] {}: {}", path.display(), err);
            *failed_count += 1;
        }
    }
}
