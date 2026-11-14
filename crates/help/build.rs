//! Carries every help page under `pages/` into the program (`include_str!`), by its id: the
//! file's name without `.md`, whatever folder it is in, so help works with no files beside
//! the program and no network.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn main() {
    let pages = Path::new(env!("CARGO_MANIFEST_DIR")).join("pages");
    let mut files = vec![];
    let mut folders = vec![pages];
    while let Some(folder) = folders.pop() {
        // a page added or removed changes its folder; a page's own change reaches rustc
        // through include_str!
        println!("cargo:rerun-if-changed={}", folder.display());
        let Ok(entries) = std::fs::read_dir(&folder) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                folders.push(path);
            } else if path.extension().is_some_and(|e| e == "md") {
                files.push(path);
            }
        }
    }
    files.sort();
    let mut table = String::from(
        "/// Every help page under `pages/`: its id (the file's name) and its text.\n\
         pub static PAGES: &[(&str, &str)] = &[\n",
    );
    for path in &files {
        let id = path
            .file_stem()
            .expect("a page has a name")
            .to_string_lossy();
        let _ = writeln!(table, "    ({id:?}, include_str!({:?})),", path.display());
    }
    table.push_str("];\n");
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));
    std::fs::write(out.join("pages.rs"), table).expect("the page table is written");
}
