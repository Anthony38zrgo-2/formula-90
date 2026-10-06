use std::process::Command;

fn main() {
    let source_output = Command::new("git").args(["rev-parse", "HEAD"]).output().expect("Physical world requires Git source provenance");
    assert!(source_output.status.success(), "Cannot resolve physical world source");
    let source_identifier = String::from_utf8(source_output.stdout).expect("Physical source identifier must be UTF-8");
    let source_identifier = source_identifier.trim();
    assert_eq!(source_identifier.len(), 40, "Physical world source must be a full commit identifier");
    println!("cargo:rustc-env=COUPLED_WORLD_SOURCE_IDENTIFIER={source_identifier}");
    if let Ok(reference_output) = Command::new("git").args(["symbolic-ref", "-q", "HEAD"]).output() {
        if reference_output.status.success() {
            let reference = String::from_utf8(reference_output.stdout).unwrap();
            let reference_path_output = Command::new("git").args(["rev-parse", "--git-path", reference.trim()]).output().unwrap();
            let reference_path = String::from_utf8(reference_path_output.stdout).unwrap();
            println!("cargo:rerun-if-changed={}", reference_path.trim());
        }
    }
    let git_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok().map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
    // Re-run the build script whenever crate sources change so the embedded
    // GIT_HASH reflects the commit the DLL was actually built from.
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src");
    // HEAD changes on every commit (even with unchanged sources); watch it so
    // the stale build-script output is invalidated and GIT_HASH is refreshed
    // (CAL-1300 parity fix, mirrors formula90-core/vehicle-audio-engine).
    println!("cargo:rerun-if-changed=../../../.git/HEAD");
    println!("cargo:rerun-if-env-changed=FORMULA90_FORCE_BUILD_SHA_REFRESH");
}
