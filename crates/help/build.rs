//! Carries every help page under `pages/` into the program (`include_str!`), by its id: the
//! file's name without `.md`, whatever folder it is in, and every picture under `pictures/`
//! (`include_bytes!`), by its path there, so help works with no files beside the program and
//! no network.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let files = under(&root.join("pages"), &["md"]);
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
    table.push_str(
        "/// The pages under `pages/concepts/`: the ideas behind the tools, by id.\n\
         pub static IDEAS: &[&str] = &[\n",
    );
    let concepts = root.join("pages").join("concepts");
    for path in files.iter().filter(|p| p.starts_with(&concepts)) {
        let id = path
            .file_stem()
            .expect("a page has a name")
            .to_string_lossy();
        let _ = writeln!(table, "    {id:?},");
    }
    table.push_str("];\n");
    let pictures_dir = root.join("pictures");
    let pictures = under(&pictures_dir, &["png", "gif", "jpg", "webp", "svg"]);
    table.push_str(
        "/// Every picture under `pictures/`: its path there, with `/`, and its bytes.\n\
         pub static PICTURES: &[(&str, &[u8])] = &[\n",
    );
    for path in &pictures {
        let key: Vec<String> = path
            .strip_prefix(&pictures_dir)
            .expect("under the pictures folder")
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        let _ = writeln!(
            table,
            "    ({:?}, include_bytes!({:?})),",
            key.join("/"),
            path.display()
        );
    }
    table.push_str("];\n");
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));
    std::fs::write(out.join("pages.rs"), table).expect("the page table is written");
}

/// Every file under `dir` with one of `extensions`, sorted.
fn under(dir: &Path, extensions: &[&str]) -> Vec<PathBuf> {
    let mut files = vec![];
    let mut folders = vec![dir.to_path_buf()];
    while let Some(folder) = folders.pop() {
        // a file added or removed changes its folder; a file's own change reaches rustc
        // through include_str! or include_bytes!
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
                .is_some_and(|e| extensions.iter().any(|x| e == *x))
            {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}
