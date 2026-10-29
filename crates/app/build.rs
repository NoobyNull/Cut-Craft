//! Puts the commit Cut Craft is built from into bug reports (`CUT_CRAFT_COMMIT`), and lists
//! every icon under `assets/icons` for `include_bytes!`, so the program carries its icons.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

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

    let icons = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/icons");
    let mut files = vec![];
    let mut folders = vec![icons.clone()];
    while let Some(folder) = folders.pop() {
        // a file added or removed changes its folder, which reruns this; a file's own change
        // reaches rustc through include_bytes!
        println!("cargo:rerun-if-changed={}", folder.display());
        let Ok(entries) = std::fs::read_dir(&folder) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                folders.push(path);
            } else if path
                .extension()
                .is_some_and(|e| matches!(e.to_str(), Some("svg" | "bmp" | "png")))
            {
                files.push(path);
            }
        }
    }
    files.sort();
    let mut table = String::from(
        "/// Every icon under `assets/icons`: its path with `/`, and its bytes.\n\
         pub static BUILT_IN: &[(&str, &[u8])] = &[\n",
    );
    for path in &files {
        let key = key_of(path, &icons);
        let _ = writeln!(
            table,
            "    ({key:?}, include_bytes!({:?})),",
            path.display()
        );
    }
    table.push_str("];\n");
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));
    std::fs::write(out.join("icons.rs"), table).expect("the icon table is written");
}

/// `drawing/align/left.png` for that file under `icons`, whatever the platform's separator.
fn key_of(path: &Path, icons: &Path) -> String {
    path.strip_prefix(icons)
        .expect("under the icon folder")
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}
