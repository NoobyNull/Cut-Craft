//! The machines, kept between runs; one is active.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Aspire's Machine Configuration: what the machine is, and the posts it uses.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Machine {
    pub name: String,
    pub manufacturer: String,
    pub model: String,
    pub controller: String,
    /// Bed size, mm (0 when not given).
    pub width: f64,
    pub height: f64,
    /// Show the bed size in inches.
    pub inches: bool,
    /// The associated posts by name (`POST_NAME`), in the order shown.
    pub posts: Vec<String>,
    /// The one toolpaths are saved through; else the first.
    pub default_post: Option<String>,
    /// An automatic tool changer: the tool loaded in each slot by name, slot 1 first;
    /// empty when the machine has none (tool changes are by hand).
    pub slots: Vec<Option<String>>,
}

impl Machine {
    /// Sets how many slots the tool changer has (0: none), keeping what is loaded in the
    /// slots that remain.
    pub fn set_slot_count(&mut self, count: usize) {
        self.slots.resize(count, None);
    }

    /// The slot `tool` is loaded in, counted from 1.
    pub fn slot_of(&self, tool: &str) -> Option<usize> {
        self.slots
            .iter()
            .position(|s| s.as_deref() == Some(tool))
            .map(|i| i + 1)
    }

    /// The post toolpaths are saved through, by name.
    pub fn post(&self) -> Option<&str> {
        self.default_post
            .as_deref()
            .filter(|d| self.posts.iter().any(|p| p == d))
            .or_else(|| self.posts.first().map(String::as_str))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Machines {
    pub machines: Vec<Machine>,
    /// The active machine, by index.
    pub active: usize,
}

impl Machines {
    /// The machines kept at `path`; none when there is no file or it cannot be read.
    pub fn load(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let json = serde_json::to_vec_pretty(self).expect("machines serialise");
        cut_craft_core::file::write_atomic(path, &json)
            .map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn active(&self) -> Option<&Machine> {
        self.machines.get(self.active)
    }

    pub fn active_mut(&mut self) -> Option<&mut Machine> {
        self.machines.get_mut(self.active)
    }

    /// `name`, else `name 2`, `name 3`... not already a machine's (two cannot share one).
    pub fn unused_name(&self, name: &str) -> String {
        let taken = |n: &str| self.machines.iter().any(|m| m.name == n);
        let mut n = 1;
        loop {
            let candidate = if n == 1 {
                name.to_owned()
            } else {
                format!("{name} {n}")
            };
            if !taken(&candidate) {
                return candidate;
            }
            n += 1;
        }
    }

    /// Add Machine: a new one, made active.
    pub fn add(&mut self) {
        let name = self.unused_name("My Machine");
        self.machines.push(Machine {
            name,
            ..Machine::default()
        });
        self.active = self.machines.len() - 1;
    }

    /// Duplicate Machine: a copy of the active one, made active.
    pub fn duplicate(&mut self) {
        let Some(mut copy) = self.active().cloned() else {
            return;
        };
        copy.name = self.unused_name(&format!("{} (copy)", copy.name));
        self.machines.insert(self.active + 1, copy);
        self.active += 1;
    }

    /// Delete Machine: the active one; the one above it becomes active.
    pub fn delete(&mut self) {
        if self.active < self.machines.len() {
            self.machines.remove(self.active);
            self.active = self.active.saturating_sub(1);
        }
    }

    /// Install Offline Package: `package`'s machines (a machines file of its own) added after
    /// these, each unless one of the same name is here already. How many were added, and
    /// how many were here already.
    pub fn install(&mut self, package: Self) -> (usize, usize) {
        let offered = package.machines.len();
        let before = self.machines.len();
        for machine in package.machines {
            if !self.machines.iter().any(|m| m.name == machine.name) {
                self.machines.push(machine);
            }
        }
        let added = self.machines.len() - before;
        (added, offered - added)
    }

    /// Whether two machines share a name.
    pub fn names_clash(&self) -> bool {
        self.machines
            .iter()
            .enumerate()
            .any(|(i, m)| self.machines[..i].iter().any(|o| o.name == m.name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn machines_are_added_duplicated_and_deleted_with_unique_names() {
        let mut m = Machines::default();
        assert!(m.active().is_none());
        m.add();
        m.add();
        assert_eq!(m.machines[1].name, "My Machine 2");
        assert_eq!(m.active, 1);
        m.active_mut().unwrap().posts = vec!["A".into(), "B".into()];
        m.duplicate();
        assert_eq!(
            (m.active, m.machines[2].name.as_str()),
            (2, "My Machine 2 (copy)")
        );
        assert_eq!(m.machines[2].posts.len(), 2, "with its posts");
        m.delete();
        assert_eq!((m.machines.len(), m.active), (2, 1));
        m.machines[0].name = "My Machine 2".into();
        assert!(m.names_clash());
    }

    #[test]
    fn the_default_post_else_the_first_saves_toolpaths() {
        let mut machine = Machine {
            posts: vec!["A".into(), "B".into()],
            ..Machine::default()
        };
        assert_eq!(machine.post(), Some("A"));
        machine.default_post = Some("B".into());
        assert_eq!(machine.post(), Some("B"));
        machine.posts.pop();
        assert_eq!(
            machine.post(),
            Some("A"),
            "a default no longer associated is passed over"
        );
        assert_eq!(Machine::default().post(), None);
    }

    #[test]
    fn a_tool_changer_keeps_its_loaded_tools_as_it_is_resized() {
        let mut machine = Machine::default();
        machine.set_slot_count(5);
        machine.slots[3] = Some("6 mm End Mill".into());
        assert_eq!(machine.slot_of("6 mm End Mill"), Some(4));
        machine.set_slot_count(4);
        assert_eq!(machine.slot_of("6 mm End Mill"), Some(4));
        machine.set_slot_count(3);
        assert_eq!(
            machine.slot_of("6 mm End Mill"),
            None,
            "a slot taken away unloads it"
        );
        assert_eq!(machine.slots.len(), 3);
    }

    #[test]
    fn machines_are_kept_between_runs() {
        let path =
            std::env::temp_dir().join(format!("cut-craft-machines-{}.json", std::process::id()));
        let mut m = Machines::default();
        m.add();
        m.machines[0].width = 1220.0;
        m.save(&path).unwrap();
        assert_eq!(Machines::load(&path), m);
        let _ = std::fs::remove_file(&path);
        assert_eq!(Machines::load(&path), Machines::default());
    }
}

#[cfg(test)]
mod packages {
    use super::*;

    /// The machine packages in `machines/`, as Install Offline Package reads them.
    const PACKAGES: [(&str, &str); 5] = [
        (
            "carbide3d",
            include_str!("../../../machines/carbide3d.json"),
        ),
        ("foxalien", include_str!("../../../machines/foxalien.json")),
        ("genmitsu", include_str!("../../../machines/genmitsu.json")),
        ("sienci", include_str!("../../../machines/sienci.json")),
        ("twotrees", include_str!("../../../machines/twotrees.json")),
    ];

    fn package(text: &str) -> Machines {
        serde_json::from_str(text).expect("a package reads")
    }

    #[test]
    fn installing_adds_the_new_machines_and_leaves_ones_already_here() {
        let mut mine = Machines::default();
        mine.add();
        mine.machines[0].name = "Genmitsu 4040-PRO".into();
        mine.machines[0].width = 1.0;
        let offered = package(PACKAGES[2].1).machines.len();
        assert_eq!(mine.install(package(PACKAGES[2].1)), (offered - 1, 1));
        assert_eq!(mine.machines[0].width, 1.0, "the user's own is kept");
        assert_eq!(mine.machines.len(), offered);
        assert_eq!(
            mine.install(package(PACKAGES[2].1)),
            (0, offered),
            "again adds none"
        );
    }

    #[test]
    fn every_package_reads_names_cut_crafts_posts_and_gives_a_size() {
        let posts: Vec<String> = cut_craft_post::BUILT_IN
            .iter()
            .map(|(_, text)| cut_craft_post::Post::parse(text).unwrap().name)
            .collect();
        let mut names = vec![];
        for (file, text) in PACKAGES {
            let machines = package(text).machines;
            assert!(!machines.is_empty(), "{file}");
            for m in machines {
                assert!(m.width > 0.0 && m.height > 0.0, "{file}: {}", m.name);
                assert!(
                    m.post().is_some_and(|p| posts.iter().any(|n| n == p)),
                    "{file}: {}",
                    m.name
                );
                // metric and SAE, arcs and straight, each a post Cut Craft has
                assert_eq!(m.posts.len(), 4, "{file}: {}", m.name);
                assert!(
                    m.posts.iter().all(|p| posts.contains(p)),
                    "{file}: {}",
                    m.name
                );
                let units = if m.inches { "(inch)" } else { "(mm)" };
                let arcs = |p: &str| p.contains("Arcs") && p.contains(units);
                assert!(
                    m.post().is_some_and(arcs),
                    "defaults to its own units: {}",
                    m.name
                );
                assert!(!names.contains(&m.name), "{} is in two packages", m.name);
                names.push(m.name);
            }
        }
    }
}
