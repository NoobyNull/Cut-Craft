//! Icon themes: add-ons of kind `icons`, installed from Gadgets ▸ Add-ons, each bringing one
//! or more sets of icons laid out as PowerWhittle's own. Edit ▸ Options ▸ Icons picks one; the
//! built-in (flat) icons stay for anything the set has no picture for.

use cut_craft_updater::addon::{self, Manifest};
use std::path::{Path, PathBuf};

/// One set of icons an installed theme brings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IconSet {
    /// What Options keeps to remember it: the add-on's id, `/`, the set's folder.
    pub key: String,
    /// What Options ▸ Icons calls it.
    pub name: String,
    /// Signed with PowerWhittle's release key: official.
    pub signed: bool,
}

/// The icon sets installed in `dir` (the add-ons folder), by add-on id, each add-on's in the
/// order it lists them.
pub fn installed(dir: &Path) -> Vec<IconSet> {
    let mut ids: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().join(addon::RECORD).is_file())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    ids.sort();
    ids.into_iter()
        .flat_map(|id| {
            let manifest = std::fs::read_to_string(dir.join(&id).join(addon::MANIFEST))
                .ok()
                .and_then(|text| Manifest::parse(&text).ok())
                .filter(|m| m.kind == "icons");
            let signed = addon::installed(dir, &id).is_some_and(|r| r.signed);
            manifest
                .map_or_default(|m| m.icon_sets)
                .into_iter()
                .map(move |set| IconSet {
                    key: format!("{id}/{}", set.folder),
                    name: set.name,
                    signed,
                })
        })
        .collect()
}

/// The folder of the icon set `key` (as [`IconSet::key`]) in `dir`, while it is installed.
pub fn folder(dir: &Path, key: &str) -> Option<PathBuf> {
    installed(dir)
        .iter()
        .any(|s| s.key == key)
        .then(|| dir.join(key))
        .filter(|f| f.is_dir())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn addons(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("pw-icon-themes-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn install(dir: &Path, id: &str, manifest: &str, signed: bool) {
        let folder = dir.join(id);
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join(addon::MANIFEST), manifest).unwrap();
        let record = format!(
            "{{\"repo\":{{\"owner\":\"Power-Whittle\",\"name\":\"{id}\"}},\"tag\":\"v1.0.0\",\"version\":\"1.0.0\",\"signed\":{signed}}}"
        );
        fs::write(folder.join(addon::RECORD), record).unwrap();
    }

    #[test]
    fn the_sets_of_every_installed_icon_theme_are_offered_and_nothing_else() {
        let dir = addons("listed");
        install(
            &dir,
            "icons-3d",
            "id = \"icons-3d\"\nname = \"3D Icons\"\nkind = \"icons\"\nversion = \"1.0.0\"\n\
             [[icon_sets]]\nfolder = \"3d\"\nname = \"3D\"\n\
             [[icon_sets]]\nfolder = \"3d-angled\"\nname = \"3D (angled)\"\n",
            true,
        );
        install(
            &dir,
            "language-ru",
            "id = \"language-ru\"\nname = \"Русский\"\nkind = \"language\"\nlanguage = \"ru\"\n\
             version = \"1.0.0\"\n",
            true,
        );
        fs::create_dir_all(dir.join("icons-3d/3d")).unwrap();
        let sets = installed(&dir);
        let keys: Vec<_> = sets
            .iter()
            .map(|s| (s.key.as_str(), s.name.as_str()))
            .collect();
        assert_eq!(
            keys,
            [("icons-3d/3d", "3D"), ("icons-3d/3d-angled", "3D (angled)")]
        );
        assert!(sets.iter().all(|s| s.signed));
        assert_eq!(folder(&dir, "icons-3d/3d"), Some(dir.join("icons-3d/3d")));
        // listed but its folder missing, or not listed at all: no folder
        assert_eq!(folder(&dir, "icons-3d/3d-angled"), None);
        assert_eq!(folder(&dir, "language-ru/3d"), None);
        fs::remove_dir_all(dir).unwrap();
    }
}
