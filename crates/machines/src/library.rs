//! The post library: Cut Craft's own posts, provided and not edited, and the user's custom
//! posts, each a `.pp` file in a folder of their own (`My_PostP`). Posts are known
//! by their `POST_NAME`, as machines name them.

use cut_craft_post::{BUILT_IN, Post};
use std::path::{Path, PathBuf};

/// Where a post comes from.
#[derive(Clone, Debug, PartialEq)]
pub enum Source {
    /// One of Cut Craft's own: its text.
    BuiltIn(&'static str),
    /// A custom post's file.
    Custom(PathBuf),
}

/// A post in the library.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// Its `POST_NAME`, or its file name when it does not parse.
    pub name: String,
    /// Its file's name (`grbl_arcs_mm.pp`): what its capabilities are kept by, as its name
    /// may change.
    pub key: String,
    pub source: Source,
    /// Why it does not parse, if it does not.
    pub broken: Option<String>,
}

impl Entry {
    pub fn is_custom(&self) -> bool {
        matches!(self.source, Source::Custom(_))
    }
}

pub struct Library {
    /// The custom posts' folder; `None` has no custom posts (tests, or no config folder).
    pub custom_dir: Option<PathBuf>,
}

impl Library {
    pub fn new(custom_dir: Option<PathBuf>) -> Self {
        Self { custom_dir }
    }

    /// The custom posts by name, then Cut Craft's own.
    pub fn entries(&self) -> Vec<Entry> {
        let mut custom: Vec<Entry> = self
            .custom_dir
            .as_deref()
            .and_then(|dir| std::fs::read_dir(dir).ok())
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("pp")))
            .map(|path| {
                let text = std::fs::read_to_string(&path).map_err(|e| e.to_string());
                let parsed = text.and_then(|t| Post::parse(&t));
                let file = path
                    .file_stem()
                    .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
                Entry {
                    key: file_name(&path),
                    name: parsed.as_ref().map_or(file, |p| p.name.clone()),
                    broken: parsed.err(),
                    source: Source::Custom(path),
                }
            })
            .collect();
        custom.sort_by_key(|e| e.name.to_lowercase());
        let own = BUILT_IN.iter().map(|(file, text)| Entry {
            key: (*file).to_owned(),
            name: Post::parse(text).expect("Cut Craft's posts parse").name,
            source: Source::BuiltIn(text),
            broken: None,
        });
        custom.into_iter().chain(own).collect()
    }

    pub fn find(&self, name: &str) -> Option<Entry> {
        self.entries().into_iter().find(|e| e.name == name)
    }

    /// The post's instructions, for View.
    pub fn text(entry: &Entry) -> Result<String, String> {
        match &entry.source {
            Source::BuiltIn(text) => Ok((*text).to_owned()),
            Source::Custom(path) => {
                std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
            }
        }
    }

    /// The rows of a post's change log: the comment lines after its `History` comment (rule
    /// lines of `=` left out), as Who / When / What. Empty for a post without one.
    pub fn change_log(text: &str) -> Vec<String> {
        text.lines()
            .map(str::trim)
            .skip_while(|l| l.trim_start_matches('+').trim() != "History")
            .skip(1)
            .map_while(|l| l.strip_prefix('+'))
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.chars().all(|c| c == '=' || c == ' '))
            .map(str::to_owned)
            .collect()
    }

    /// A post's version: when its last change was made (the change log's last row).
    pub fn version(text: &str) -> Option<String> {
        let log = Self::change_log(text);
        // the first row names the columns
        log.iter()
            .skip(1)
            .last()?
            .split_whitespace()
            .find(|w| w.contains(|c: char| c.is_ascii_digit()) && w.contains(['/', '-', '.']))
            .map(str::to_owned)
    }

    /// The post named `name`, read.
    pub fn load(&self, name: &str) -> Result<Post, String> {
        let entry = self
            .find(name)
            .ok_or_else(|| format!("no post-processor named {name}"))?;
        Post::parse(&Self::text(&entry)?).map_err(|e| format!("{name}: {e}"))
    }

    fn dir(&self) -> Result<&Path, String> {
        let dir = self
            .custom_dir
            .as_deref()
            .ok_or("there is no folder for custom post-processors")?;
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        Ok(dir)
    }

    /// A file in the custom folder not yet used, named after `stem`.
    fn free_path(dir: &Path, stem: &str) -> PathBuf {
        let clean: String = stem
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || "-_ ()".contains(c) {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let mut n = 1;
        loop {
            let name = if n == 1 {
                format!("{clean}.pp")
            } else {
                format!("{clean} {n}.pp")
            };
            let path = dir.join(name);
            if !path.exists() {
                return path;
            }
            n += 1;
        }
    }

    /// Install Post-Processor: a copy of `file` in the custom folder, once it is seen to
    /// parse; its entry. A post whose name is already listed (one installed before, or one
    /// of Cut Craft's own) is kept beside it as a copy, named with a copy number and when it
    /// was installed: `Mine (mm) (copy 1) (11/15/26 075422) (*.nc)`.
    pub fn install(&self, file: &Path) -> Result<Entry, String> {
        self.install_at(file, chrono::Local::now().naive_local())
    }

    /// [`Library::install`], installed at `now`.
    pub fn install_at(&self, file: &Path, now: chrono::NaiveDateTime) -> Result<Entry, String> {
        let text = std::fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display()))?;
        let post = Post::parse(&text).map_err(|e| format!("{}: {e}", file.display()))?;
        let taken: Vec<String> = self.entries().into_iter().map(|e| e.name).collect();
        let (name, text) = if taken.contains(&post.name) {
            let when = now.format("%m/%d/%y %H%M%S");
            let (base, kind) = match post.name.rfind(" (*.") {
                Some(at) => post.name.split_at(at),
                None => (post.name.as_str(), ""),
            };
            let copy = |n: usize| format!("{base} (copy {n})");
            let mut n = 1;
            while taken.iter().any(|t| t.starts_with(&copy(n))) {
                n += 1;
            }
            let name = format!("{} ({when}){kind}", copy(n));
            let renamed = with_name(&text, &name);
            (name, renamed)
        } else {
            (post.name, text)
        };
        let stem = if name.contains("(copy ") {
            name.replace(" (*.", " ").replace(['(', ')', '/'], "")
        } else {
            file.file_stem()
                .map_or_else(|| "post".into(), |s| s.to_string_lossy().into_owned())
        };
        let path = Self::free_path(self.dir()?, &stem);
        std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(Entry {
            name,
            key: file_name(&path),
            source: Source::Custom(path),
            broken: None,
        })
    }

    /// Customize: an editable copy of `entry` in the custom folder, named apart from it
    /// (`… Custom (*.nc)`); its entry.
    pub fn customize(&self, entry: &Entry) -> Result<Entry, String> {
        let text = Self::text(entry)?;
        let taken: Vec<String> = self.entries().into_iter().map(|e| e.name).collect();
        let named = |n: usize| {
            let mark = if n == 1 {
                " Custom".to_owned()
            } else {
                format!(" Custom {n}")
            };
            // before the file type the names end with, as `(*.nc)`
            match entry.name.rfind(" (*.") {
                Some(at) => format!("{}{mark}{}", &entry.name[..at], &entry.name[at..]),
                None => format!("{}{mark}", entry.name),
            }
        };
        let mut n = 1;
        while taken.contains(&named(n)) {
            n += 1;
        }
        let name = named(n);
        let renamed = with_name(&text, &name);
        let path = Self::free_path(self.dir()?, &name.replace(" (*.", " ").replace(')', ""));
        std::fs::write(&path, renamed).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(Entry {
            name,
            key: file_name(&path),
            source: Source::Custom(path),
            broken: None,
        })
    }

    /// Edit's Save: a custom post's new text, written whole or not at all.
    pub fn save(path: &Path, text: &str) -> Result<(), String> {
        cut_craft_core::file::write_atomic(path, text.as_bytes())
            .map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Delete: takes a custom post's file away (Cut Craft's own cannot be).
    pub fn delete(entry: &Entry) -> Result<(), String> {
        match &entry.source {
            Source::Custom(path) => {
                std::fs::remove_file(path).map_err(|e| format!("{}: {e}", path.display()))
            }
            Source::BuiltIn(_) => Err("Cut Craft's own post-processors cannot be deleted".into()),
        }
    }
}

/// The last part of `path`, as text.
fn file_name(path: &Path) -> String {
    path.file_name()
        .map_or_else(String::new, |s| s.to_string_lossy().into_owned())
}

/// `text` with its `POST_NAME` line saying `name` (one added at the top if it has none).
fn with_name(text: &str, name: &str) -> String {
    let line = format!("POST_NAME = \"{name}\"");
    let mut found = false;
    let mut out: Vec<String> = text
        .lines()
        .map(|l| {
            if !found && l.trim_start().starts_with("POST_NAME") {
                found = true;
                line.clone()
            } else {
                l.to_owned()
            }
        })
        .collect();
    if !found {
        out.insert(0, line);
    }
    out.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_own_post_has_a_change_log_and_a_version() {
        for entry in Library::new(None).entries() {
            let text = Library::text(&entry).unwrap();
            let log = Library::change_log(&text);
            assert!(log.len() > 1, "{}: {log:?}", entry.name);
            assert!(log[0].starts_with("Who"), "{log:?}");
            assert_eq!(
                Library::version(&text).as_deref(),
                Some("2026-10-31"),
                "{}",
                entry.name
            );
        }
        // a Vectric-style history, and none
        let theirs = "+ History\n+ Who  When       What\n+ ==== ========== ====\n+ Mark 14/07/2020 Written\n\nPOST_NAME = \"x\"";
        assert_eq!(
            Library::change_log(theirs),
            ["Who  When       What", "Mark 14/07/2020 Written"]
        );
        assert_eq!(Library::version(theirs).as_deref(), Some("14/07/2020"));
        assert_eq!(
            Library::change_log("POST_NAME = \"x\""),
            Vec::<String>::new()
        );
    }

    fn library(name: &str) -> (Library, PathBuf) {
        let dir =
            std::env::temp_dir().join(format!("cut-craft-posts-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        (Library::new(Some(dir.clone())), dir)
    }

    #[test]
    fn cut_crafts_own_posts_are_listed_after_custom_ones() {
        let (lib, dir) = library("list");
        let own = lib.entries();
        assert_eq!(own.len(), BUILT_IN.len());
        assert!(own.iter().all(|e| !e.is_custom() && e.broken.is_none()));
        let custom = lib.customize(&own[0]).unwrap();
        assert_eq!(custom.name, "Cut Craft FluidNC ATC (mm) Custom (*.nc)");
        let all = lib.entries();
        assert_eq!(all[0], custom, "custom posts first");
        assert!(
            lib.load(&custom.name).is_ok(),
            "the copy parses under its new name"
        );
        let again = lib.customize(&own[0]).unwrap();
        assert_eq!(again.name, "Cut Craft FluidNC ATC (mm) Custom 2 (*.nc)");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn installed_posts_are_copied_in_and_deleted_from_the_custom_folder() {
        let (lib, dir) = library("install");
        let src = std::env::temp_dir().join(format!("cut-craft-src-{}.pp", std::process::id()));
        std::fs::write(
            &src,
            BUILT_IN[0]
                .1
                .replace("Cut Craft FluidNC ATC (mm)", "Mine (mm)"),
        )
        .unwrap();
        let entry = lib.install(&src).unwrap();
        assert_eq!(entry.name, "Mine (mm) (*.nc)");
        let Source::Custom(path) = &entry.source else {
            panic!("custom")
        };
        assert!(path.starts_with(&dir));
        std::fs::remove_file(&src).unwrap();
        assert!(
            lib.find("Mine (mm) (*.nc)").is_some(),
            "kept after the original goes"
        );
        Library::delete(&entry).unwrap();
        assert!(lib.find("Mine (mm) (*.nc)").is_none());
        assert!(
            Library::delete(&lib.entries()[0]).is_err(),
            "Cut Craft's own stay"
        );
        std::fs::write(&src, "not a post at all [").unwrap();
        let err = lib.install(&src).unwrap_err();
        assert!(
            err.contains("cut-craft-src-"),
            "a broken post is refused, naming the file: {err}"
        );
        let _ = std::fs::remove_file(&src);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_post_installed_under_a_name_already_listed_is_kept_as_a_dated_copy() {
        let (lib, dir) = library("install-twice");
        let src = std::env::temp_dir().join(format!("cut-craft-twice-{}.pp", std::process::id()));
        std::fs::write(
            &src,
            BUILT_IN[0]
                .1
                .replace("Cut Craft FluidNC ATC (mm)", "Mine (mm)"),
        )
        .unwrap();
        let at = |h, m, s| {
            chrono::NaiveDate::from_ymd_opt(2026, 11, 15)
                .and_then(|d| d.and_hms_opt(h, m, s))
                .unwrap()
        };
        assert_eq!(
            lib.install_at(&src, at(7, 54, 22)).unwrap().name,
            "Mine (mm) (*.nc)"
        );
        let copy = lib.install_at(&src, at(7, 54, 22)).unwrap();
        assert_eq!(copy.name, "Mine (mm) (copy 1) (11/15/26 075422) (*.nc)");
        assert!(
            lib.load(&copy.name).is_ok(),
            "the copy is listed under its name"
        );
        let next = lib.install_at(&src, at(8, 0, 1)).unwrap();
        assert_eq!(next.name, "Mine (mm) (copy 2) (11/15/26 080001) (*.nc)");
        // a post named like one of Cut Craft's own never takes its place
        let own = lib.entries().last().unwrap().name.clone();
        std::fs::write(&src, with_name(BUILT_IN[0].1, &own)).unwrap();
        let named = lib.install_at(&src, at(9, 0, 0)).unwrap().name;
        assert_ne!(named, own);
        assert!(named.contains("(copy 1)"), "{named}");
        let firsts: Vec<String> = lib
            .entries()
            .into_iter()
            .map(|e| e.name)
            .filter(|n| *n == own)
            .collect();
        assert_eq!(firsts.len(), 1, "one post with each name");
        let _ = std::fs::remove_file(&src);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_post_without_a_name_line_gets_one() {
        assert_eq!(with_name("A = 1\n", "N"), "POST_NAME = \"N\"\nA = 1\n");
        assert_eq!(
            with_name("POST_NAME = \"Old\"\nB", "N"),
            "POST_NAME = \"N\"\nB\n"
        );
    }
}
