use std::process::Command;

fn main() {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("git must be available to stamp vehicle audio BUILD");
    assert!(
        output.status.success(),
        "cannot resolve git HEAD for vehicle audio BUILD"
    );
    let sha = String::from_utf8(output.stdout).expect("git HEAD must be UTF-8");
    let sha = sha.trim();
    assert_eq!(sha.len(), 40, "vehicle audio BUILD must be a full git SHA");
    println!("cargo:rustc-env=FORMULA90_BUILD_SHA={sha}");
    println!("cargo:rerun-if-env-changed=FORMULA90_FORCE_BUILD_SHA_REFRESH");
    println!("cargo:rerun-if-changed=../../../.git/HEAD");
}
