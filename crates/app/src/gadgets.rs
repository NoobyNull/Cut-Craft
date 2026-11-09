//! Lua gadgets in the Gadgets menu: the ones that come with Cut Craft, and the user's own
//! from the My Gadgets folder. Each one becomes a command with its own settings form.

use cut_craft_core::{DynCommand, Registry};
use cut_craft_scripting::load_gadget;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The gadgets that come with Cut Craft: file name and Lua source.
const BUILT_IN: [(&str, &str); 2] = [
    ("drill-grid.lua", include_str!("../gadgets/drill-grid.lua")),
    (
        "hole-pattern.lua",
        include_str!("../gadgets/hole-pattern.lua"),
    ),
];

/// Where the user's own gadgets go: a `gadgets` folder beside Cut Craft's settings.
pub fn my_gadgets_dir() -> Option<PathBuf> {
    crate::files::config_dir().map(|dir| dir.join("gadgets"))
}

/// A gadget as the menu lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listed {
    pub label: String,
    pub id: String,
}

/// What [`load`] found.
#[derive(Debug, Default)]
pub struct Loaded {
    pub built_in: Vec<Listed>,
    /// From the My Gadgets folder.
    pub mine: Vec<Listed>,
    /// Gadgets that could not be loaded, each with why, worded for the user.
    pub problems: Vec<String>,
}

/// The folders of gadget add-ons installed in `addons` (Gadgets ▸ Add-ons), each with
/// whether it was signed with Cut Craft's release key.
pub fn from_addons(addons: &Path) -> Vec<(PathBuf, bool)> {
    use cut_craft_updater::addon::{MANIFEST, Manifest, installed};
    let mut found: Vec<(PathBuf, bool)> = std::fs::read_dir(addons)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let id = entry.file_name().into_string().ok()?;
            let record = installed(addons, &id)?;
            let manifest = std::fs::read_to_string(entry.path().join(MANIFEST)).ok()?;
            (Manifest::parse(&manifest).ok()?.kind == "gadget")
                .then(|| (entry.path(), record.signed))
        })
        .collect();
    found.sort();
    found
}

/// Loads the built-in gadgets, every `.lua` file in `mine`, and those of the gadget add-ons
/// `addons` (folder, signed) into `registry`: a signed add-on's beside the built-in ones, an
/// unsigned one's with the user's own, warned about the same way. A gadget runs only Cut
/// Craft's own commands, never another gadget.
pub fn load(registry: &mut Registry, mine: Option<&Path>, addons: &[(PathBuf, bool)]) -> Loaded {
    let commands = Arc::new(crate::commands::registry());
    let mut loaded = Loaded::default();
    for (file, source) in BUILT_IN {
        match add(registry, file, source, &commands) {
            Ok(listed) => loaded.built_in.push(listed),
            Err(why) => loaded.problems.push(why),
        }
    }
    let folders = mine
        .map(|dir| (dir.to_path_buf(), false))
        .into_iter()
        .chain(addons.iter().cloned());
    for (dir, signed) in folders {
        for path in lua_files(&dir) {
            let file = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let added = std::fs::read_to_string(&path)
                .map_err(|e| format!("{file}: could not be read ({e})"))
                .and_then(|source| add(registry, &file, &source, &commands));
            match added {
                Ok(listed) if signed => loaded.built_in.push(listed),
                Ok(listed) => loaded.mine.push(listed),
                Err(why) => loaded.problems.push(why),
            }
        }
    }
    loaded
}

fn add(
    registry: &mut Registry,
    file: &str,
    source: &str,
    commands: &Arc<Registry>,
) -> Result<Listed, String> {
    let gadget =
        load_gadget(file, source, Arc::clone(commands)).map_err(|e| format!("{file}: {e}"))?;
    if registry.get(gadget.id()).is_some() {
        return Err(format!(
            "{file}: skipped, another gadget already has the name \"{}\"",
            gadget.meta().label
        ));
    }
    let listed = Listed {
        label: gadget.meta().label.to_owned(),
        id: gadget.id().to_owned(),
    };
    registry.add_dyn(Box::new(gadget));
    Ok(listed)
}

/// The `.lua` files in `dir`, by name.
fn lua_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| {
                    p.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("lua"))
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str, files: &[(&str, &str)]) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cut-craft-gadgets-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for (file, source) in files {
            std::fs::write(dir.join(file), source).unwrap();
        }
        dir
    }

    #[test]
    fn the_built_in_gadgets_load() {
        let mut registry = crate::commands::registry();
        let loaded = load(&mut registry, None, &[]);
        assert!(loaded.problems.is_empty(), "{:?}", loaded.problems);
        let labels: Vec<&str> = loaded.built_in.iter().map(|g| g.label.as_str()).collect();
        assert_eq!(labels, ["Drill Grid", "Hole Pattern"]);
        assert!(registry.get("gadget_hole_pattern").is_some());
    }

    #[test]
    fn the_users_own_gadgets_load_and_broken_ones_say_why() {
        let dir = folder(
            "mine",
            &[
                ("star-burst.lua", "function run() end"),
                ("typo.lua", "function run(\n"),
                ("hole-pattern.lua", "function run() end"),
                ("notes.txt", "not a gadget"),
            ],
        );
        let mut registry = crate::commands::registry();
        let loaded = load(&mut registry, Some(&dir), &[]);
        let mine: Vec<&str> = loaded.mine.iter().map(|g| g.label.as_str()).collect();
        assert_eq!(mine, ["Star Burst"]);
        assert_eq!(loaded.problems.len(), 2, "{:?}", loaded.problems);
        assert!(loaded.problems.iter().any(|p| p.starts_with("typo.lua: ")));
        assert!(
            loaded
                .problems
                .iter()
                .any(|p| p.starts_with("hole-pattern.lua: skipped"))
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn no_my_gadgets_folder_is_no_problem() {
        let mut registry = crate::commands::registry();
        let loaded = load(&mut registry, Some(Path::new("/no/such/folder")), &[]);
        assert!(loaded.mine.is_empty() && loaded.problems.is_empty());
    }

    #[test]
    fn gadget_add_ons_load_signed_ones_beside_the_built_in_ones() {
        let addons = folder("addons", &[]);
        let gadget = |id: &str, file: &str, signed: bool, kind: &str| {
            let dir = addons.join(id);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join("addon.toml"),
                format!("id = \"{id}\"\nname = \"{id}\"\nkind = \"{kind}\"\nversion = \"1.0.0\""),
            )
            .unwrap();
            let record = format!(
                r#"{{"repo":{{"owner":"o","name":"{id}"}},"tag":"v1.0.0","version":"1.0.0","signed":{signed}}}"#
            );
            std::fs::write(dir.join("installed.json"), record).unwrap();
            std::fs::write(dir.join(file), "function run() end").unwrap();
        };
        gadget("trusted", "box-maker.lua", true, "gadget");
        gadget("stranger", "star-burst.lua", false, "gadget");
        // a tool add-on's Lua is not a gadget
        gadget("depth", "helper.lua", true, "tool");
        let found = from_addons(&addons);
        assert_eq!(
            found,
            [
                (addons.join("stranger"), false),
                (addons.join("trusted"), true)
            ]
        );
        let mut registry = crate::commands::registry();
        let loaded = load(&mut registry, None, &found);
        let built_in: Vec<&str> = loaded.built_in.iter().map(|g| g.label.as_str()).collect();
        assert_eq!(built_in, ["Drill Grid", "Hole Pattern", "Box Maker"]);
        let mine: Vec<&str> = loaded.mine.iter().map(|g| g.label.as_str()).collect();
        assert_eq!(mine, ["Star Burst"], "unsigned: warned before it runs");
        let _ = std::fs::remove_dir_all(addons);
    }
}
