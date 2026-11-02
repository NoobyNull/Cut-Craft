//! The Library's tags: words the user gives files, to find them by. Kept in the config
//! folder by path, so the files themselves are never written to.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Tags by file, and where they are kept.
#[derive(Default)]
pub struct Tags {
    map: BTreeMap<PathBuf, BTreeSet<String>>,
    /// `None` keeps them only for this run.
    store: Option<PathBuf>,
}

/// The tag that makes a picture a depth map, whatever its colours.
pub const DEPTH_MAP: &str = "depth map";

impl Tags {
    /// The tags kept at `store`; none if there are none yet or they can't be read.
    pub fn load(store: Option<PathBuf>) -> Self {
        let map = store
            .as_deref()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Self { map, store }
    }

    /// `file`'s tags, in order.
    pub fn of(&self, file: &Path) -> impl Iterator<Item = &str> {
        self.map.get(file).into_iter().flatten().map(String::as_str)
    }

    /// The tags any of `files` has, in order, once each.
    pub fn of_all(&self, files: &[PathBuf]) -> BTreeSet<String> {
        files
            .iter()
            .flat_map(|f| self.of(f))
            .map(str::to_owned)
            .collect()
    }

    /// Gives each of `files` `tag` (trimmed), unless it has it already in any letter case.
    pub fn add(&mut self, files: &[PathBuf], tag: &str) -> Result<(), String> {
        let tag = tag.split_whitespace().collect::<Vec<_>>().join(" ");
        if tag.is_empty() {
            return Ok(());
        }
        for file in files {
            let tags = self.map.entry(file.clone()).or_default();
            if !tags.iter().any(|t| t.eq_ignore_ascii_case(&tag)) {
                tags.insert(tag.clone());
            }
        }
        self.save()
    }

    /// Takes `tag` off each of `files`.
    pub fn remove(&mut self, files: &[PathBuf], tag: &str) -> Result<(), String> {
        for file in files {
            if let Some(tags) = self.map.get_mut(file) {
                tags.retain(|t| !t.eq_ignore_ascii_case(tag));
                if tags.is_empty() {
                    self.map.remove(file);
                }
            }
        }
        self.save()
    }

    /// Forgets `files` (deleted from disk).
    pub fn forget(&mut self, files: &[PathBuf]) -> Result<(), String> {
        let before = self.map.len();
        for file in files {
            self.map.remove(file);
        }
        if self.map.len() == before {
            return Ok(());
        }
        self.save()
    }

    /// Whether one of `file`'s tags has `wanted` (lower case) in it.
    pub fn matches(&self, file: &Path, wanted: &str) -> bool {
        self.of(file).any(|t| t.to_lowercase().contains(wanted))
    }

    /// Whether `file` is tagged as a depth map.
    pub fn is_depth_map(&self, file: &Path) -> bool {
        self.of(file).any(|t| {
            let t = t.to_lowercase();
            t == DEPTH_MAP || t == "depthmap" || t == "height map" || t == "heightmap"
        })
    }

    fn save(&self) -> Result<(), String> {
        let Some(store) = &self.store else {
            return Ok(());
        };
        let bytes = serde_json::to_vec_pretty(&self.map).map_err(|e| e.to_string())?;
        cut_craft_core::file::write_atomic(store, &bytes)
            .map_err(|e| format!("Couldn't save the Library's tags: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store(name: &str) -> PathBuf {
        let p =
            std::env::temp_dir().join(format!("cut-craft-tags-{name}-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&p);
        p
    }

    /// Tags are kept between runs, tidied, and given once each whatever their letter case.
    #[test]
    fn kept_between_runs() {
        let at = store("kept");
        let (a, b) = (PathBuf::from("eagle.stl"), PathBuf::from("skull.stl"));
        let mut tags = Tags::load(Some(at.clone()));
        tags.add(&[a.clone(), b.clone()], "  Birds   and skulls ")
            .unwrap();
        tags.add(std::slice::from_ref(&a), "birds AND skulls")
            .unwrap();
        tags.add(std::slice::from_ref(&a), "eagle").unwrap();
        tags.add(std::slice::from_ref(&a), "   ").unwrap();
        let tags = Tags::load(Some(at.clone()));
        assert_eq!(
            tags.of(&a).collect::<Vec<_>>(),
            ["Birds and skulls", "eagle"]
        );
        assert_eq!(tags.of(&b).collect::<Vec<_>>(), ["Birds and skulls"]);
        assert!(tags.matches(&b, "skull"));
        assert!(!tags.matches(&b, "eagle"));
        std::fs::remove_file(at).unwrap();
    }

    #[test]
    fn removed_and_forgotten() {
        let at = store("removed");
        let (a, b) = (PathBuf::from("a.png"), PathBuf::from("b.png"));
        let mut tags = Tags::load(Some(at.clone()));
        tags.add(&[a.clone(), b.clone()], "Depth Map").unwrap();
        assert!(tags.is_depth_map(&a));
        tags.remove(std::slice::from_ref(&a), "depth map").unwrap();
        assert!(!tags.is_depth_map(&a));
        tags.forget(std::slice::from_ref(&b)).unwrap();
        let tags = Tags::load(Some(at.clone()));
        assert!(tags.of_all(&[a, b]).is_empty());
        std::fs::remove_file(at).unwrap();
    }
}
