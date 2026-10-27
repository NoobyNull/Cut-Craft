//! Puts the commit Cut Craft is built from into bug reports (`CUT_CRAFT_COMMIT`).

fn main() {
    let commit = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned());
    if let Some(commit) = commit {
        println!("cargo:rustc-env=CUT_CRAFT_COMMIT={commit}");
    }
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/refs/heads");
}
