//! What each post-processor can do (its controller, units and moves), so the post lists can
//! be filtered and grouped: read from the post's own lines, and kept in a small file
//! (`capabilities.json` beside the custom posts) when someone corrects what was read.

use crate::library::{Entry, Library};
use crate::machine::Machine;
use cut_craft_bug_report::hint::HoverHint;
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// The kind of controller a post writes G-code for; the post lists are grouped by it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    /// grbl 1.1 and grblHAL
    Grbl,
    FluidNc,
    /// Mach3 and Mach4
    Mach,
    /// None of these, or not known
    #[default]
    Other,
}

impl Family {
    pub const ALL: [Self; 4] = [Self::Grbl, Self::FluidNc, Self::Mach, Self::Other];

    pub fn label(self) -> &'static str {
        match self {
            Self::Grbl => "GRBL / grblHAL",
            Self::FluidNc => "FluidNC",
            Self::Mach => "Mach3 / Mach4",
            Self::Other => "Other",
        }
    }

    /// The family a post's name (or, failing that, its notes) names.
    fn named_in(text: &str) -> Option<Self> {
        let text = text.to_lowercase();
        if text.contains("fluidnc") {
            Some(Self::FluidNc)
        } else if text.contains("mach3") || text.contains("mach4") || text.contains("mach 3") {
            Some(Self::Mach)
        } else if text.contains("grbl") {
            Some(Self::Grbl)
        } else {
            None
        }
    }
}

/// How a post writes the cutting moves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Moves {
    /// Curves as arcs (G2 / G3) where they can be: smaller files, smoother curves
    #[default]
    Arcs,
    /// Lines only (G1), for controllers that do not do arcs
    Straight,
    /// A 4th axis (A): the side that goes round the cylinder turned into degrees
    Rotary,
}

impl Moves {
    pub const ALL: [Self; 3] = [Self::Arcs, Self::Straight, Self::Rotary];

    pub fn label(self) -> &'static str {
        match self {
            Self::Arcs => "Arcs",
            Self::Straight => "Straight",
            Self::Rotary => "Rotary (A)",
        }
    }

    /// What the choice means, for its tooltip.
    pub fn hint(self) -> &'static str {
        match self {
            Self::Arcs => "Curves written as arcs (G2 / G3): smaller files and smoother curves",
            Self::Straight => "Lines only (G1), for controllers that cannot do arcs",
            Self::Rotary => {
                "For a 4th axis: the side that goes round the cylinder is written as A, in degrees"
            }
        }
    }
}

/// What a post can do.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Capabilities {
    pub family: Family,
    /// It writes inches; millimetres when not.
    pub inches: bool,
    pub moves: Moves,
    /// Its rotary feeds are in inverse time (G93).
    pub inverse_time: bool,
    /// What it is for, in a line or two (its opening notes), shown on hover.
    pub about: String,
}

impl Capabilities {
    /// What a post's lines say it can do: its `POST_NAME` (or its opening notes) for the
    /// controller, `UNITS`, the rotary and inverse-time lines, and whether it has arc blocks.
    pub fn read(text: &str) -> Self {
        let mut caps = Self::default();
        let mut name = String::new();
        let mut arcs = false;
        let mut notes: Vec<&str> = vec![];
        let mut in_notes = true;
        for line in text.lines().map(str::trim) {
            if in_notes {
                match line.strip_prefix('+').map(str::trim) {
                    Some("History") => in_notes = false,
                    Some(note)
                        if !note.is_empty()
                            && !note.starts_with("Made by")
                            && !note.starts_with("Written for Cut Craft") =>
                    {
                        notes.push(note);
                    }
                    Some(_) => {}
                    None if line.is_empty() && notes.is_empty() => {}
                    None => in_notes = false,
                }
            }
            if line.starts_with('+') {
                continue;
            }
            let upper = line.to_uppercase();
            let value = || {
                line.split_once('=')
                    .map(|(_, v)| v.trim().trim_matches('"'))
            };
            if upper.starts_with("POST_NAME") {
                value().unwrap_or_default().clone_into(&mut name);
            } else if upper.starts_with("UNITS") {
                caps.inches = value().is_some_and(|v| v.to_uppercase().starts_with("IN"));
            } else if upper.starts_with("ROTARY_WRAP_X") || upper.starts_with("ROTARY_WRAP_Y") {
                caps.moves = Moves::Rotary;
            } else if upper.starts_with("INVERSE_TIME_MODE") {
                caps.inverse_time = value().is_some_and(|v| v.eq_ignore_ascii_case("YES"));
            } else if upper.starts_with("BEGIN CW_ARC_MOVE")
                || upper.starts_with("BEGIN CCW_ARC_MOVE")
            {
                arcs = true;
            }
        }
        if caps.moves != Moves::Rotary {
            caps.moves = if arcs { Moves::Arcs } else { Moves::Straight };
        }
        caps.about = notes.join(" ");
        caps.family = Family::named_in(&name)
            .or_else(|| Family::named_in(&caps.about))
            .unwrap_or_default();
        caps
    }

    /// Short tags for the end of a post's row: `mm · Arcs`, `inch · Rotary (A) · G93`.
    pub fn tags(&self) -> String {
        let mut tags = format!(
            "{} · {}",
            if self.inches { "inch" } else { "mm" },
            self.moves.label()
        );
        if self.inverse_time {
            tags.push_str(" · G93");
        }
        tags
    }
}

/// The capabilities file: the corrections people made, by the post's key (its file name),
/// over what each post's lines say.
#[derive(Clone, Debug, Default)]
pub struct CapabilityDb {
    /// `capabilities.json`; `None` keeps corrections only while Cut Craft runs.
    path: Option<PathBuf>,
    edits: BTreeMap<String, Capabilities>,
}

impl CapabilityDb {
    /// The file's name, in the custom posts' folder.
    pub const FILE: &'static str = "capabilities.json";

    /// The corrections kept for `library`'s posts (none when there is no file yet, or it
    /// cannot be read).
    pub fn load(library: &Library) -> Self {
        let path = library.custom_dir.as_ref().map(|d| d.join(Self::FILE));
        let edits = path
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        Self { path, edits }
    }

    /// What `entry` can do: the correction kept for it, else what its lines say.
    pub fn of(&self, entry: &Entry) -> Capabilities {
        self.edits
            .get(&entry.key)
            .cloned()
            .unwrap_or_else(|| Library::text(entry).map_or_default(|t| Capabilities::read(&t)))
    }

    /// Whether `entry`'s capabilities were corrected by hand.
    pub fn corrected(&self, entry: &Entry) -> bool {
        self.edits.contains_key(&entry.key)
    }

    /// Keeps `caps` for `entry` (or, `None`, goes back to what its lines say), and saves.
    pub fn set(&mut self, entry: &Entry, caps: Option<Capabilities>) -> Result<(), String> {
        match caps {
            Some(caps) => {
                self.edits.insert(entry.key.clone(), caps);
            }
            None => {
                self.edits.remove(&entry.key);
            }
        }
        let Some(path) = &self.path else {
            return Ok(());
        };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        let text = serde_json::to_string_pretty(&self.edits).map_err(|e| e.to_string())?;
        cut_craft_core::file::write_atomic(path, text.as_bytes())
            .map_err(|e| format!("{}: {e}", path.display()))
    }
}

/// The filters over a post list. Each opening starts afresh from [`Filter::starting`].
#[derive(Clone, Debug, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "one tick box each, independent of each other"
)]
pub struct Filter {
    /// Words the name (or its notes) must hold.
    pub search: String,
    /// One controller only; `None` for any.
    pub family: Option<Family>,
    pub mm: bool,
    pub inch: bool,
    pub arcs: bool,
    pub straight: bool,
    pub rotary: bool,
}

impl Default for Filter {
    /// [`Filter::starting`] with no machine: millimetres, flat posts.
    fn default() -> Self {
        Self::starting(None)
    }
}

impl Filter {
    /// Where the filters start: the machine's units, arcs and straight, and rotary only for a
    /// machine with a 4th axis.
    pub fn starting(machine: Option<&Machine>) -> Self {
        let inches = machine.is_some_and(|m| m.inches);
        Self {
            search: String::new(),
            family: None,
            mm: !inches,
            inch: inches,
            arcs: true,
            straight: true,
            rotary: machine.is_some_and(Machine::has_rotary),
        }
    }

    /// Everything shown.
    pub fn all() -> Self {
        Self {
            search: String::new(),
            family: None,
            mm: true,
            inch: true,
            arcs: true,
            straight: true,
            rotary: true,
        }
    }

    /// Whether the post named `name`, which can do `caps`, is shown.
    pub fn shows(&self, name: &str, caps: &Capabilities) -> bool {
        let units = if caps.inches { self.inch } else { self.mm };
        let moves = match caps.moves {
            Moves::Arcs => self.arcs,
            Moves::Straight => self.straight,
            Moves::Rotary => self.rotary,
        };
        let family = self.family.is_none_or(|f| f == caps.family);
        let words = self.search.to_lowercase();
        let haystack = format!("{name} {}", caps.about).to_lowercase();
        let found = words.split_whitespace().all(|w| haystack.contains(w));
        units && moves && family && found
    }

    /// The search box, the controller choice, the units and moves tick boxes, and how many of
    /// `total` posts are `shown` (with Show All).
    pub fn ui(&mut self, ui: &mut egui::Ui, shown: usize, total: usize) {
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.search)
                    .hint_text("Search posts…")
                    .desired_width(200.0),
            )
            .on_hover_hint("Words in the post's name or notes");
            egui::ComboBox::from_id_salt(ui.scope_id().with("post_family"))
                .selected_text(self.family.map_or("Any controller", Family::label))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.family, None, "Any controller");
                    for f in Family::ALL {
                        ui.selectable_value(&mut self.family, Some(f), f.label());
                    }
                })
                .response
                .on_hover_hint("The controller your machine runs");
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Units");
            ui.checkbox(&mut self.mm, "mm")
                .on_hover_hint("Posts that write millimetres");
            ui.checkbox(&mut self.inch, "inch")
                .on_hover_hint("Posts that write inches");
            ui.add_space(12.0);
            ui.label("Moves");
            ui.checkbox(&mut self.arcs, Moves::Arcs.label())
                .on_hover_hint(Moves::Arcs.hint());
            ui.checkbox(&mut self.straight, Moves::Straight.label())
                .on_hover_hint(Moves::Straight.hint());
            ui.checkbox(&mut self.rotary, Moves::Rotary.label())
                .on_hover_hint(Moves::Rotary.hint());
        });
        ui.horizontal(|ui| {
            ui.weak(format!("Showing {shown} of {total}"));
            if shown < total
                && ui
                    .small_button("Show All")
                    .on_hover_hint("Clear the filters")
                    .clicked()
            {
                *self = Self::all();
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_post::BUILT_IN;

    #[test]
    fn every_own_post_reads_as_its_file_name_says() {
        for (file, text) in BUILT_IN {
            let caps = Capabilities::read(text);
            let family = if file.starts_with("grbl") {
                Family::Grbl
            } else if file.starts_with("fluidnc") {
                Family::FluidNc
            } else {
                Family::Mach
            };
            assert_eq!(caps.family, family, "{file}");
            assert_eq!(caps.inches, file.contains("_inch"), "{file}");
            let moves = if file.contains("_wrap_") {
                Moves::Rotary
            } else if file.contains("_straight_") {
                Moves::Straight
            } else {
                Moves::Arcs
            };
            assert_eq!(caps.moves, moves, "{file}");
            let g93 = file.contains("_g93_") || file == "fluidnc_atc_wrap_a_mm.pp";
            assert_eq!(caps.inverse_time, g93, "{file}");
            assert!(!caps.about.is_empty(), "{file}: notes to show on hover");
        }
    }

    #[test]
    fn a_post_from_elsewhere_is_read_from_its_lines() {
        let theirs = "+ For my old router\nPOST_NAME = \"Shop (*.tap)\"\nUNITS = \"Inches\"\n\
                      begin RAPID_MOVE\n\"G0 [X] [Y] [Z]\"\n";
        let caps = Capabilities::read(theirs);
        assert_eq!(caps.family, Family::Other, "nothing names a controller");
        assert!(caps.inches);
        assert_eq!(caps.moves, Moves::Straight, "no arc blocks");
        assert_eq!(caps.about, "For my old router");
        assert_eq!(caps.tags(), "inch · Straight");
        let named = theirs.replace("Shop", "Shop Mach4");
        assert_eq!(Capabilities::read(&named).family, Family::Mach);
    }

    fn machine(inches: bool, rotary: bool) -> Machine {
        Machine {
            inches,
            rotary,
            firmware: Some(crate::Firmware::GrblHal),
            ..Machine::default()
        }
    }

    #[test]
    fn the_filters_start_from_the_machine() {
        let flat = Filter::starting(Some(&machine(false, false)));
        assert!(flat.mm && !flat.inch && flat.arcs && flat.straight && !flat.rotary);
        let rotary = Filter::starting(Some(&machine(true, true)));
        assert!(!rotary.mm && rotary.inch && rotary.rotary);
        assert_eq!(Filter::starting(None), flat, "no machine: mm, flat");
        let standard = Machine {
            firmware: Some(crate::Firmware::Grbl),
            ..machine(true, true)
        };
        assert!(
            !Filter::starting(Some(&standard)).rotary,
            "standard GRBL has no A axis"
        );
    }

    #[test]
    fn rotary_posts_are_hidden_until_ticked_and_every_filter_must_match() {
        let caps = |file: &str| {
            let text = BUILT_IN.iter().find(|(f, _)| *f == file).unwrap().1;
            Capabilities::read(text)
        };
        let mut f = Filter::starting(Some(&machine(false, false)));
        let shown = |f: &Filter| {
            BUILT_IN
                .iter()
                .filter(|(_, t)| f.shows("", &Capabilities::read(t)))
                .count()
        };
        assert!(f.shows("", &caps("grbl_arcs_mm.pp")));
        assert!(!f.shows("", &caps("grbl_wrap_y_a_mm.pp")), "no 4th axis");
        assert!(!f.shows("", &caps("grbl_arcs_inch.pp")), "machine in mm");
        assert_eq!(
            shown(&f),
            6,
            "FluidNC, GRBL and Mach3, arcs and straight, in mm"
        );
        f.rotary = true;
        assert!(f.shows("", &caps("grbl_wrap_y_a_mm.pp")), "ticked: shown");
        f.family = Some(Family::Mach);
        f.arcs = false;
        f.straight = false;
        assert_eq!(shown(&f), 4, "Mach3's mm rotary posts");
        assert_eq!(shown(&Filter::all()), BUILT_IN.len());
        let words = Filter {
            search: "grbl   ARCS".into(),
            ..Filter::all()
        };
        assert!(words.shows("Cut Craft GRBL Arcs (mm) (*.nc)", &Capabilities::default()));
        assert!(!words.shows("Cut Craft Mach3 Arcs (mm) (*.nc)", &Capabilities::default()));
    }

    #[test]
    fn corrections_are_kept_by_file_name_and_can_be_undone() {
        let dir =
            std::env::temp_dir().join(format!("cut-craft-capabilities-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let library = Library::new(Some(dir.clone()));
        let entry = library.entries().remove(0);
        let mut db = CapabilityDb::load(&library);
        let read = db.of(&entry);
        assert!(!db.corrected(&entry));
        let fixed = Capabilities {
            family: Family::Other,
            about: "Mine".into(),
            ..read.clone()
        };
        db.set(&entry, Some(fixed.clone())).unwrap();
        let again = CapabilityDb::load(&library);
        assert_eq!(again.of(&entry), fixed, "kept in the file");
        assert!(again.corrected(&entry));
        db.set(&entry, None).unwrap();
        assert_eq!(
            CapabilityDb::load(&library).of(&entry),
            read,
            "back to its lines"
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
