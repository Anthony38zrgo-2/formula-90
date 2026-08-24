use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: telemetry_compare <godot_run.csv> <rust_run.csv>");
        std::process::exit(1);
    }

    let path_a = &args[1];
    let path_b = &args[2];

    println!("=== Formula-90 Telemetry Parity Comparator ===");
    println!("File A (Reference): {}", path_a);
    println!("File B (Candidate): {}", path_b);

    let rows_a = read_csv(path_a);
    let rows_b = read_csv(path_b);

    if rows_a.is_empty() || rows_b.is_empty() {
        eprintln!("Error: One or both files are empty or unreadable.");
        std::process::exit(1);
    }

    let min_rows = rows_a.len().min(rows_b.len());
    println!("Comparing {} rows of telemetry data...", min_rows);

    let mut speed_diff_sq = 0.0;
    let mut rpm_diff_sq = 0.0;
    let mut max_speed_diff = 0.0f64;

    for i in 0..min_rows {
        let speed_a: f64 = rows_a[i].get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let speed_b: f64 = rows_b[i].get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let diff_speed = (speed_a - speed_b).abs();
        speed_diff_sq += diff_speed * diff_speed;
        max_speed_diff = max_speed_diff.max(diff_speed);

        let rpm_a: f64 = rows_a[i].get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let rpm_b: f64 = rows_b[i].get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let diff_rpm = (rpm_a - rpm_b).abs();
        rpm_diff_sq += diff_rpm * diff_rpm;
    }

    let rms_speed = (speed_diff_sq / min_rows as f64).sqrt();
    let rms_rpm = (rpm_diff_sq / min_rows as f64).sqrt();

    println!("----------------------------------------------");
    println!("Results:");
    println!(
        "- Speed RMS Error:   {:.2} km/h (Max Diff: {:.2} km/h)",
        rms_speed, max_speed_diff
    );
    println!("- RPM RMS Error:     {:.1} RPM", rms_rpm);
    println!("----------------------------------------------");

    if rms_speed < 5.0 && rms_rpm < 300.0 {
        println!("[PASS] Telemetry correlation meets acceptance thresholds.");
    } else {
        println!("[WARN] Telemetry differences exceed strict parity threshold.");
    }
}

fn read_csv(path: &str) -> Vec<Vec<String>> {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Cannot open '{}': {}", path, e);
            return Vec::new();
        }
    };
    let reader = BufReader::new(file);
    let mut lines = Vec::new();
    for (idx, line) in reader.lines().enumerate() {
        if idx == 0 {
            continue; // skip header
        }
        if let Ok(l) = line {
            if !l.trim().is_empty() {
                let cols: Vec<String> = l.split(',').map(|s| s.trim().to_string()).collect();
                lines.push(cols);
            }
        }
    }
    lines
}
