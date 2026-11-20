//! The machines, kept between runs; one is active.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// The Machine Configuration: what the machine is, and the posts it uses.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Machine {
    pub name: String,
    pub manufacturer: String,
    pub model: String,
    pub controller: String,
    /// The firmware its controller runs, as picked in Machine Configuration; `None` when
    /// never picked, read from [`Self::controller`] instead (see [`Self::firmware`]).
    pub firmware: Option<Firmware>,
    /// Bed size, mm (0 when not given).
    pub width: f64,
    pub height: f64,
    /// Z travel, mm (0 when not given).
    pub depth: f64,
    /// The fastest each axis moves, X, Y, Z, mm/min (0 when not given): rapids run at it and
    /// no feed may pass it: the controller's max rate (velocity in Mach3).
    pub max_rate: [f64; 3],
    /// Each axis's acceleration, X, Y, Z, mm/s² (0 when not given).
    pub acceleration: [f64; 3],
    /// How far from a sharp corner the controller lets the bit stray to keep its speed
    /// through the corner (junction deviation, GRBL's $11), mm; 0 for the usual 0.01 mm.
    /// Only the cutting time estimates use it (added with better cutting times).
    pub junction_deviation: f64,
    /// The spindle's slowest and fastest speed, rpm (0 when not given).
    pub spindle: [f64; 2],
    /// Show the bed size in inches.
    pub inches: bool,
    /// The associated posts by name (`POST_NAME`), in the order shown.
    pub posts: Vec<String>,
    /// The one toolpaths are saved through; else the first.
    pub default_post: Option<String>,
    /// An automatic tool changer: the tool loaded in each slot by name, slot 1 first;
    /// empty when the machine has none (tool changes are by hand).
    pub slots: Vec<Option<String>>,
    /// A 4th axis (A) turning the material, for rotary jobs (2.5 axes + A): those are saved
    /// through the machine's post that wraps their side round onto A.
    pub rotary: bool,
    /// Where the 4th axis lies, which way it turns, how fast, and its chuck and tailstock:
    /// used once [`Self::has_rotary`].
    pub rotary_axis: cut_craft_core::rotary_axis::RotaryAxis,
    /// Areas of the bed a tool must never pass over in any job (clamps fixed to the bed),
    /// in the machine's millimetres from the job's XY origin, as its G-code is.
    pub keep_out: Vec<Zone>,
    /// A drill bank: several drills on the head, dropped together to drill rows of holes in
    /// one plunge (Drill Bank Toolpath); `None` when the machine has none.
    pub drill_bank: Option<cut_craft_core::drill_bank::DrillBank>,
    /// Where its G-code programs are saved, and the folder its files are sent to.
    pub send_to: crate::send::SendTo,
    /// What holds the bits: the collet nut and the spindle above it, checked for hitting
    /// the material on deep cuts (added with collision checks).
    pub holder: cut_craft_core::collision::Holder,
    /// Holder per bit: each bit can have a holder of its own in the Tool Database, used
    /// instead of [`Self::holder`] (a tool changer's bits each sit in their own holder).
    pub holder_per_bit: bool,
}

/// The firmware a machine's controller runs: it decides whether the machine can have a 4th
/// axis (A) at all.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Firmware {
    /// Standard GRBL 1.1: X, Y and Z only. Also what an unknown firmware is taken for.
    #[default]
    Grbl,
    /// grblHAL, which can be built with up to 8 axes.
    GrblHal,
    /// FluidNC, whose axes are set in its config file.
    FluidNc,
    /// Mach3 or Mach4, 6 axes.
    Mach,
    /// Another firmware, picked as such (it is trusted to have the axes it is used with).
    Other,
}

/// What a 4th axis says on a machine whose firmware has none.
pub const NO_A_AXIS: &str = "Standard GRBL supports only 3 axes (X, Y and Z). For an A axis, \
                             use custom firmware or your controller's own firmware.";

impl Firmware {
    pub const ALL: [Self; 5] = [
        Self::Grbl,
        Self::GrblHal,
        Self::FluidNc,
        Self::Mach,
        Self::Other,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Grbl => "GRBL 1.1 (standard)",
            Self::GrblHal => "grblHAL",
            Self::FluidNc => "FluidNC",
            Self::Mach => "Mach3 / Mach4",
            Self::Other => "Other",
        }
    }

    /// What it is, for its hover.
    pub fn hint(self) -> &'static str {
        match self {
            Self::Grbl => {
                "The GRBL most hobby routers ship with (often on an Arduino board): X, Y and Z \
                 only. Pick this if you are not sure"
            }
            Self::GrblHal => {
                "GRBL rewritten for faster boards. For an A axis, its firmware must be built \
                 with 4 or more axes and $376 set to make A rotary"
            }
            Self::FluidNc => "For ESP32 boards, often WiFi; an A axis is added in its config file",
            Self::Mach => "Mach3 or Mach4 on a PC, with up to 6 axes",
            Self::Other => "Another firmware; Cut Craft takes it to have the axes you use",
        }
    }

    /// It can drive an A axis together with X, Y and Z.
    pub fn has_a_axis(self) -> bool {
        self != Self::Grbl
    }

    /// The firmware a machine's Controller text names; standard GRBL when it names none
    /// (unknown firmware is taken for standard GRBL, which has no A axis).
    pub fn guess(controller: &str) -> Self {
        let text = controller.to_ascii_lowercase().replace(' ', "");
        // first-time setup's "grbl or grblHAL" does not say which
        if text.contains("grblorgrblhal") {
            Self::Grbl
        } else if text.contains("grblhal") {
            Self::GrblHal
        } else if text.contains("fluidnc") {
            Self::FluidNc
        } else if text.contains("mach3") || text.contains("mach4") {
            Self::Mach
        } else {
            Self::Grbl
        }
    }
}

impl Machine {
    /// The firmware its controller runs: the one picked, else the one its Controller text
    /// names, else standard GRBL.
    pub fn firmware(&self) -> Firmware {
        self.firmware
            .unwrap_or_else(|| Firmware::guess(&self.controller))
    }

    /// It has a 4th axis (A) its firmware can drive: ticked, on firmware other than standard
    /// GRBL. A tick kept from before stays, and counts again if the firmware is changed.
    pub fn has_rotary(&self) -> bool {
        self.rotary && self.firmware().has_a_axis()
    }

    /// How it holds its bits, for the collet and holder checks.
    pub fn holding(&self) -> cut_craft_core::collision::Holding {
        cut_craft_core::collision::Holding {
            holder: self.holder,
            per_bit: self.holder_per_bit,
        }
    }
}

/// A rectangle of the bed kept out of: its lower-left corner and its size, mm, and what its
/// label calls it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Zone {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// Its name ("Front clamp"); empty for "Machine clamp 1" and so on, by its place.
    pub name: String,
    /// How tall the clamp stands above the material's top, mm, for the collet and spindle
    /// checks; 0 for not given (taken as the usual clamp's; added with collision checks).
    pub stands: f64,
}

impl Zone {
    /// What its label calls zone `index` (from 0) of a machine's.
    pub fn name_at(&self, index: usize) -> String {
        if self.name.trim().is_empty() {
            format!("Machine clamp {}", index + 1)
        } else {
            self.name.trim().to_owned()
        }
    }

    /// The zone as a closed path.
    pub fn path(&self) -> cut_craft_core::geometry::Path {
        use cut_craft_core::geometry::{Path, Point};
        let (a, b) = (
            Point::new(self.x, self.y),
            Point::new(self.x + self.width, self.y + self.height),
        );
        Path::polyline(&[a, Point::new(b.x, a.y), b, Point::new(a.x, b.y)], true)
    }
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

    /// Loads `tool` into holder `slot` (from 1), taking it out of any other holder.
    pub fn load_tool(&mut self, tool: &str, slot: usize) {
        for s in &mut self.slots {
            if s.as_deref() == Some(tool) {
                *s = None;
            }
        }
        if let Some(s) = slot.checked_sub(1).and_then(|i| self.slots.get_mut(i)) {
            *s = Some(tool.to_owned());
        }
    }

    /// Ticking the 4th axis: for each of Cut Craft's flat posts the machine has, its rotary
    /// twins in `library` (`… Rotary A, cylinder along X, G93 (units)` and the same along
    /// Y), so both directions of cylinder can be cut. GRBL's twins are grblHAL's: standard
    /// GRBL has no A axis. Custom posts have no twins to find.
    pub fn add_rotary_posts(&mut self, library: &[String]) {
        let mut twins = vec![];
        for post in &self.posts {
            let base = post.replace(" Arcs (", " (").replace(" Straight (", " (");
            let Some(at) = base.find(" (") else { continue };
            let controller = base[..at].replace("Cut Craft GRBL", "Cut Craft grblHAL");
            for along in ["X", "Y"] {
                let twin = format!(
                    "{controller} Rotary A, cylinder along {along}, G93{}",
                    &base[at..]
                );
                if library.contains(&twin) && !self.posts.contains(&twin) && !twins.contains(&twin)
                {
                    twins.push(twin);
                }
            }
        }
        self.posts.extend(twins);
    }

    /// Its posts named as Cut Craft's own rotary posts were before they were renamed
    /// ("… Wrap Y to A …"), given their names now.
    pub fn rename_old_posts(&mut self) {
        for name in self.posts.iter_mut().chain(&mut self.default_post) {
            if let Some(new) = cut_craft_post::renamed(name) {
                *name = new;
            }
        }
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
    /// The machines kept at `path`; none when there is no file yet. A file that cannot be
    /// read or understood (half written, edited by hand, or from a newer Cut Craft) is an
    /// error naming it, so it is never taken for no machines and saved over.
    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(format!("{}: {e}", path.display())),
        };
        let mut machines: Self =
            serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        for machine in &mut machines.machines {
            machine.rename_old_posts();
        }
        Ok(machines)
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
        for mut machine in package.machines {
            machine.rename_old_posts();
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
        m.machines[0].depth = 80.0;
        m.machines[0].max_rate = [5000.0, 5000.0, 1500.0];
        m.machines[0].acceleration = [500.0, 500.0, 200.0];
        m.machines[0].spindle = [8000.0, 24000.0];
        m.machines[0].drill_bank =
            Some(cut_craft_core::drill_bank::DrillBank::new(5, 3, 32.0, 5.0));
        m.save(&path).unwrap();
        assert_eq!(Machines::load(&path), Ok(m));
        let _ = std::fs::remove_file(&path);
        assert_eq!(Machines::load(&path), Ok(Machines::default()));
    }

    #[test]
    fn a_machines_file_that_cannot_be_read_is_an_error_not_no_machines() {
        let path = std::env::temp_dir().join(format!(
            "cut-craft-machines-broken-{}.json",
            std::process::id()
        ));
        std::fs::write(&path, b"{\"machines\": [{\"width\": \"wide\"}]}").unwrap();
        let e = Machines::load(&path).unwrap_err();
        assert!(e.contains(&path.display().to_string()), "{e}");
        let _ = std::fs::remove_file(&path);
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
    fn a_4th_axis_adds_the_rotary_twins_of_cut_crafts_posts() {
        let library: Vec<String> = cut_craft_post::BUILT_IN
            .iter()
            .map(|(_, text)| cut_craft_post::Post::parse(text).unwrap().name)
            .collect();
        let mut genmitsu = package(PACKAGES[2].1).machines.remove(0);
        genmitsu.posts.push("My Own (*.nc)".into());
        genmitsu.add_rotary_posts(&library);
        genmitsu.add_rotary_posts(&library);
        let added: Vec<&str> = genmitsu.posts[5..].iter().map(String::as_str).collect();
        assert_eq!(
            added,
            [
                "Cut Craft grblHAL Rotary A, cylinder along X, G93 (mm) (*.nc)",
                "Cut Craft grblHAL Rotary A, cylinder along Y, G93 (mm) (*.nc)",
                "Cut Craft grblHAL Rotary A, cylinder along X, G93 (inch) (*.nc)",
                "Cut Craft grblHAL Rotary A, cylinder along Y, G93 (inch) (*.nc)",
            ],
            "both directions in both units, once; a custom post has none"
        );
        let mut atc = Machine {
            posts: vec!["Cut Craft FluidNC ATC (mm) (*.nc)".into()],
            ..Machine::default()
        };
        atc.add_rotary_posts(&library);
        assert_eq!(
            atc.posts.len(),
            3,
            "the default post's twins: {:?}",
            atc.posts
        );
    }

    #[test]
    fn the_firmware_is_the_one_picked_else_read_from_the_controller_else_standard_grbl() {
        let on = |controller: &str| Machine {
            controller: controller.into(),
            ..Machine::default()
        };
        for (controller, firmware) in [
            ("", Firmware::Grbl),
            ("GRBL", Firmware::Grbl),
            ("Carbide Motion (GRBL)", Firmware::Grbl),
            ("grbl or grblHAL", Firmware::Grbl),
            ("grblHAL", Firmware::GrblHal),
            ("FluidNC", Firmware::FluidNc),
            ("Mach3", Firmware::Mach),
            ("Mach 4", Firmware::Mach),
            ("LinuxCNC", Firmware::Grbl),
        ] {
            assert_eq!(on(controller).firmware(), firmware, "{controller}");
        }
        let mut picked = on("GRBL");
        picked.firmware = Some(Firmware::FluidNc);
        assert_eq!(picked.firmware(), Firmware::FluidNc);
    }

    #[test]
    fn a_4th_axis_counts_only_on_firmware_with_an_a_axis() {
        let mut machine = Machine {
            rotary: true,
            ..Machine::default()
        };
        assert!(!machine.has_rotary(), "unknown firmware is standard GRBL");
        machine.firmware = Some(Firmware::GrblHal);
        assert!(machine.has_rotary());
        machine.firmware = Some(Firmware::Grbl);
        assert!(!machine.has_rotary());
        assert!(
            machine.rotary,
            "the tick is kept for when the firmware changes"
        );
        assert!(
            Firmware::ALL
                .iter()
                .filter(|f| !f.has_a_axis())
                .eq([&Firmware::Grbl])
        );
    }

    #[test]
    fn old_rotary_post_names_are_renamed_on_loading() {
        let path =
            std::env::temp_dir().join(format!("cut-craft-renamed-{}.json", std::process::id()));
        let old = "Cut Craft GRBL Wrap Y to A G93 (mm) (*.nc)";
        let machines = Machines {
            machines: vec![Machine {
                posts: vec!["Cut Craft GRBL Arcs (mm) (*.nc)".into(), old.into()],
                default_post: Some(old.into()),
                ..Machine::default()
            }],
            active: 0,
        };
        machines.save(&path).unwrap();
        let loaded = Machines::load(&path).unwrap();
        std::fs::remove_file(&path).ok();
        let new = "Cut Craft grblHAL Rotary A, cylinder along X, G93 (mm) (*.nc)";
        assert_eq!(
            loaded.machines[0].posts,
            ["Cut Craft GRBL Arcs (mm) (*.nc)", new]
        );
        assert_eq!(loaded.machines[0].default_post.as_deref(), Some(new));
        let mut mine = Machines::default();
        mine.install(machines);
        assert_eq!(
            mine.machines[0].posts[1], new,
            "a package's are renamed too"
        );
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

#[cfg(test)]
mod holder_tests {
    use super::Machine;

    #[test]
    fn a_machine_saved_before_holders_has_the_usual_one() {
        let machine: Machine = serde_json::from_str(r#"{"name": "Shapeoko"}"#).unwrap();
        let holding = machine.holding();
        assert_eq!(holding.holder, cut_craft_core::collision::Holder::default());
        assert!(!holding.per_bit);
    }
}
