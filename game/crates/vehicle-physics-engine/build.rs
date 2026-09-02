use std::process::Command;

fn main() {
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
