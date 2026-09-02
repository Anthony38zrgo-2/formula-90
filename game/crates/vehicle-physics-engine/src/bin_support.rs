// Shared best-effort metadata helpers for the calibration binaries
// (physics_cli, tire_sweep). Everything here is best-effort: failures degrade
// to "unknown" strings instead of aborting a baseline run.
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// FNV-1a 64 hex digest of a file, or "unreadable".
pub fn file_fnv64(path: &Path) -> String {
    match std::fs::read(path) {
        Ok(bytes) => format!("{:016x}", fnv1a64(&bytes)),
        Err(_) => "unreadable".to_string(),
    }
}

/// (branch, short commit) from git, best-effort.
pub fn git_head_info() -> (String, String) {
    let branch = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let commit = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    (branch, commit)
}

/// Godot version, best-effort. First tries `config/version=` in the nearest
/// project.godot, then the Godot binary set in `.tools/godot/` (e.g.
/// `Godot_v4.7.1-stable_win64.exe`), then "unknown".
pub fn godot_project_version() -> String {
    let mut dir = std::env::current_dir().ok();
    while let Some(d) = dir {
        let project = d.join("project.godot");
        if project.is_file() {
            if let Ok(content) = std::fs::read_to_string(&project) {
                for line in content.lines() {
                    if let Some(value) = line.strip_prefix("config/version=") {
                        return value.trim().to_string();
                    }
                }
            }
            break;
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }

    let mut dir = std::env::current_dir().ok();
    while let Some(d) = dir {
        let tools = d.join(".tools").join("godot");
        if tools.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&tools) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if let Some(rest) = name.strip_prefix("Godot_v") {
                        if let Some(ver) = rest.split('-').next() {
                            return ver.to_string();
                        }
                    }
                }
            }
            break;
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }

    "unknown".to_string()
}

/// Writes pretty JSON metadata next to a baseline output file.
pub fn write_meta_json(out_path: &Path, value: serde_json::Value) -> std::io::Result<()> {
    let meta_path = meta_path_for(out_path);
    let mut text = serde_json::to_string_pretty(&value).unwrap_or_default();
    text.push('\n');
    std::fs::write(meta_path, text)
}

pub fn meta_path_for(out_path: &Path) -> PathBuf {
    let mut s = out_path.as_os_str().to_os_string();
    s.push(".meta.json");
    PathBuf::from(s)
}
