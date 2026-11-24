//! Lua gadgets in the Gadgets menu: the ones that come with Cut Craft, and the user's own
//! from the My Gadgets folder. Each one becomes a command with its own settings form.
//!
//! An unsigned gadget's code does not run, not even to read its name, until the user has
//! said yes to it ([`crate::trust`]): until then it is listed by its file's name.

use crate::trust::{Code, Trust};
use cut_craft_core::{DynCommand, Registry};
use cut_craft_scripting::{Waiting, load_gadget, waiting_gadget};
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

/// An unsigned gadget: asked about before it runs.
pub struct Unsigned {
    /// What the user is asked about: its name, and its code's fingerprint.
    pub code: Code,
    /// Opens it once the user says yes; `None` once it is open (read, named, runnable).
    pub waiting: Option<Waiting>,
}

/// What [`load`] found.
#[derive(Default)]
pub struct Loaded {
    pub built_in: Vec<Listed>,
    /// From the My Gadgets folder, and unsigned gadget add-ons: warned about before they run.
    /// Named after its file until the user has said yes to it.
    pub mine: Vec<Listed>,
    /// The unsigned ones of `mine`, by command id.
    pub unsigned: std::collections::HashMap<String, Unsigned>,
    /// From gadget add-ons signed with Cut Craft's release key.
    pub signed_addons: Vec<Listed>,
    /// Gadgets that could not be loaded, each with why, worded for the user.
    pub problems: Vec<String>,
}

impl Loaded {
    /// The gadgets the user installed (not Cut Craft's own), by name: Gadgets ▸ My Gadgets
    /// and the Gadgets toolbar section list them.
    pub fn installed(&self) -> impl Iterator<Item = &Listed> {
        let mut all: Vec<&Listed> = self.mine.iter().chain(&self.signed_addons).collect();
        all.sort_by_key(|g| g.label.to_lowercase());
        all.into_iter()
    }
}

/// The folders of gadget add-ons installed in `addons` (Gadgets ▸ Add-ons), each with
/// whether it was signed with Cut Craft's release key. Add-ons that list web hosts
/// (`network`) are left out: their gadgets are not job commands.
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
            let manifest = Manifest::parse(&manifest).ok()?;
            // gadgets that reach the web are not job commands; they wait for their own
            // runner (issue #587) rather than joining the Gadgets menu as one
            (manifest.kind == "gadget" && manifest.network.is_empty())
                .then(|| (entry.path(), record.signed))
        })
        .collect();
    found.sort();
    found
}

/// Loads the built-in gadgets, every `.lua` file in `mine`, and those of the gadget add-ons
/// `addons` (folder, signed) into `registry`: an unsigned add-on's with the user's own,
/// warned about the same way. An unsigned gadget is read now only if `trust` already lets
/// this very file run; otherwise it waits, unread. A gadget runs only Cut Craft's own
/// commands, never another gadget.
pub fn load(
    registry: &mut Registry,
    mine: Option<&Path>,
    addons: &[(PathBuf, bool)],
    trust: &Trust,
) -> Loaded {
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
                .map_or_default(|n| n.to_string_lossy().into_owned());
            let source = match std::fs::read_to_string(&path) {
                Ok(source) => source,
                Err(e) => {
                    loaded
                        .problems
                        .push(format!("{file}: could not be read ({e})"));
                    continue;
                }
            };
            if signed {
                match add(registry, &file, &source, &commands) {
                    Ok(listed) => loaded.signed_addons.push(listed),
                    Err(why) => loaded.problems.push(why),
                }
                continue;
            }
            let (gadget, waiting) = waiting_gadget(&file, &source, Arc::clone(&commands));
            let id = gadget.id().to_owned();
            if registry.get(&id).is_some() {
                loaded.problems.push(format!(
                    "{file}: skipped, another gadget already has the name \"{}\"",
                    gadget.meta().label
                ));
                continue;
            }
            let mut code = Code::gadget(&id, &file, source.as_bytes());
            // said yes to before (Run and Remember): read now, so it shows its own name
            let waiting = if trust.allowed(&code) {
                match waiting.open() {
                    Ok(label) => {
                        label.clone_into(&mut code.name);
                        None
                    }
                    Err(e) => {
                        loaded.problems.push(format!("{file}: {e}"));
                        continue;
                    }
                }
            } else {
                Some(waiting)
            };
            loaded.mine.push(Listed {
                label: gadget.meta().label.to_owned(),
                id: id.clone(),
            });
            registry.add_dyn(Box::new(gadget));
            loaded.unsigned.insert(id, Unsigned { code, waiting });
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
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir).map_or_default(|entries| {
        entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("lua")))
            .collect()
    });
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
        let loaded = load(&mut registry, None, &[], &Trust::new(None));
        assert!(loaded.problems.is_empty(), "{:?}", loaded.problems);
        let labels: Vec<&str> = loaded.built_in.iter().map(|g| g.label.as_str()).collect();
        assert_eq!(labels, ["Drill Grid", "Hole Pattern"]);
        assert!(registry.get("gadget_hole_pattern").is_some());
    }

    #[test]
    fn the_users_own_gadgets_wait_unread_by_their_file_names_until_said_yes_to() {
        let dir = folder(
            "mine",
            &[
                (
                    "star-burst.lua",
                    "gadget = { name = \"Star Burst\" }\nfunction run() end",
                ),
                ("typo.lua", "function run(\n"),
                ("hole-pattern.lua", "function run() end"),
                ("notes.txt", "not a gadget"),
            ],
        );
        let mut registry = crate::commands::registry();
        let loaded = load(&mut registry, Some(&dir), &[], &Trust::new(None));
        // none of their code has run: a broken one is only found once it is opened
        let mine: Vec<&str> = loaded.mine.iter().map(|g| g.label.as_str()).collect();
        assert_eq!(mine, ["star-burst.lua", "typo.lua"]);
        assert_eq!(loaded.problems.len(), 1, "{:?}", loaded.problems);
        assert!(loaded.problems[0].starts_with("hole-pattern.lua: skipped"));
        let star = &loaded.unsigned["gadget_star_burst"];
        assert_eq!(star.code.name, "star-burst.lua");
        assert_eq!(star.waiting.as_ref().unwrap().open().unwrap(), "Star Burst");
        let typo = loaded.unsigned["gadget_typo"]
            .waiting
            .as_ref()
            .unwrap()
            .open();
        assert!(typo.is_err(), "says why once opened");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_gadget_said_yes_to_for_good_is_read_at_once_until_it_changes() {
        let source = "gadget = { name = \"Star Burst\" }\nfunction run() end";
        let dir = folder("remembered", &[("star-burst.lua", source)]);
        let mut trust = Trust::new(None);
        trust.ask = |_| crate::trust::Answer::RunAndRemember;
        assert!(trust.may_run(&Code::gadget("gadget_star_burst", "x", source.as_bytes())));
        let mut registry = crate::commands::registry();
        let loaded = load(&mut registry, Some(&dir), &[], &trust);
        assert_eq!(loaded.mine[0].label, "Star Burst");
        assert!(loaded.unsigned["gadget_star_burst"].waiting.is_none());
        // edited: it waits again
        std::fs::write(dir.join("star-burst.lua"), format!("{source}\n-- changed")).unwrap();
        let mut registry = crate::commands::registry();
        let loaded = load(&mut registry, Some(&dir), &[], &trust);
        assert_eq!(loaded.mine[0].label, "star-burst.lua");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn no_my_gadgets_folder_is_no_problem() {
        let mut registry = crate::commands::registry();
        let loaded = load(
            &mut registry,
            Some(Path::new("/no/such/folder")),
            &[],
            &Trust::new(None),
        );
        assert!(loaded.mine.is_empty() && loaded.problems.is_empty());
    }

    #[test]
    fn gadget_add_ons_load_into_my_gadgets_and_only_unsigned_ones_warn() {
        let addons = folder("addons", &[]);
        let gadget = |id: &str, file: &str, signed: bool, kind: &str| {
            let dir = addons.join(id);
            std::fs::create_dir_all(&dir).unwrap();
            let network = if id == "web" {
                "\nnetwork = [\"relay.example.com\"]"
            } else {
                ""
            };
            std::fs::write(
                dir.join("addon.toml"),
                format!(
                    "id = \"{id}\"\nname = \"{id}\"\nkind = \"{kind}\"\nversion = \"1.0.0\"{network}"
                ),
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
        // one that reaches the web is not a job command
        gadget("web", "fetch.lua", true, "gadget");
        let found = from_addons(&addons);
        assert_eq!(
            found,
            [
                (addons.join("stranger"), false),
                (addons.join("trusted"), true)
            ]
        );
        let mut registry = crate::commands::registry();
        let loaded = load(&mut registry, None, &found, &Trust::new(None));
        let built_in: Vec<&str> = loaded.built_in.iter().map(|g| g.label.as_str()).collect();
        assert_eq!(
            built_in,
            ["Drill Grid", "Hole Pattern"],
            "only Cut Craft's own"
        );
        let mine: Vec<&str> = loaded.mine.iter().map(|g| g.label.as_str()).collect();
        assert_eq!(
            mine,
            ["star-burst.lua"],
            "unsigned: unread until said yes to"
        );
        let signed: Vec<&str> = loaded
            .signed_addons
            .iter()
            .map(|g| g.label.as_str())
            .collect();
        assert_eq!(signed, ["Box Maker"]);
        let installed: Vec<&str> = loaded.installed().map(|g| g.label.as_str()).collect();
        assert_eq!(
            installed,
            ["Box Maker", "star-burst.lua"],
            "by name, for My Gadgets"
        );
        let _ = std::fs::remove_dir_all(addons);
    }
}
