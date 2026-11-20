//! Carries the English words (`locales/en/*.ftl`) into the program, so a new file needs no
//! code change and English always works, offline, with no pack installed.

use std::fmt::Write as _;
use std::path::Path;

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("locales/en");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .expect("locales/en is there")
        .map(|e| e.expect("a file in locales/en").path())
        .filter(|p| p.extension().is_some_and(|e| e == "ftl"))
        .collect();
    files.sort();
    let mut out = String::from("/// The English words, one `(file name, text)` per `.ftl` file.\n");
    out.push_str("pub const ENGLISH: &[(&str, &str)] = &[\n");
    for file in &files {
        println!("cargo:rerun-if-changed={}", file.display());
        let name = file.file_name().unwrap().to_string_lossy();
        writeln!(
            out,
            "    ({name:?}, include_str!({:?})),",
            file.display().to_string()
        )
        .unwrap();
    }
    out.push_str("];\n");
    let out_dir = std::env::var("OUT_DIR").unwrap();
    std::fs::write(Path::new(&out_dir).join("english.rs"), out).unwrap();
}
