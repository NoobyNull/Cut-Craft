//! Vectric-style `.pp` post-processors: a text format of global settings, `VAR` output formats
//! and `begin BLOCK` templates (see `re/SPEC.md`). `Post::parse` reads one; `Post::render`
//! writes a document's toolpaths through it.

mod arcs;
mod built_in;
mod winding;

pub use built_in::BUILT_IN;
mod render;

/// The name one of Cut Craft's own rotary posts has now, given the name it had before they
/// were renamed (machines keep their posts by name): "… Wrap Y to A G93 (mm) …" is now
/// "… Rotary A, cylinder along X, G93 (mm) …", and GRBL's rotary posts are grblHAL's
/// (standard GRBL 1.1 has no A axis). `None` for any other name.
pub fn renamed(old: &str) -> Option<String> {
    if old == "Cut Craft FluidNC ATC Wrap A G93 (mm) (*.nc)" {
        return Some("Cut Craft FluidNC ATC Rotary A, G93 (mm) (*.nc)".into());
    }
    let rest = old.strip_prefix("Cut Craft ")?;
    let (controller, rest) = ["GRBL", "Mach3", "FluidNC ATC"]
        .iter()
        .find_map(|c| Some((*c, rest.strip_prefix(c)?.strip_prefix(" Wrap ")?)))?;
    let (along, rest) = if let Some(r) = rest.strip_prefix("Y to A") {
        ("X", r)
    } else {
        ("Y", rest.strip_prefix("X to A")?)
    };
    let (g93, rest) = rest
        .strip_prefix(" G93")
        .map_or((false, rest), |r| (true, r));
    // exactly a former built-in name: a custom post named like one keeps its name
    let ext = if controller == "Mach3" { "tap" } else { "nc" };
    if ![" (mm) ", " (inch) "]
        .iter()
        .any(|units| rest == format!("{units}(*.{ext})"))
    {
        return None;
    }
    let controller = if controller == "GRBL" {
        "grblHAL"
    } else {
        controller
    };
    let g93 = if g93 { ", G93" } else { "" };
    Some(format!(
        "Cut Craft {controller} Rotary A, cylinder along {along}{g93}{rest}"
    ))
}

use cut_craft_core::document::{Along, Datum, Document, Job, ZZero};
use cut_craft_core::drill_bank::{BankHit, DrillBank, DrillId};
use cut_craft_core::glam::DVec3;
use cut_craft_core::toolpath::{MoveKind, Toolpath};
use std::collections::HashMap;
use std::path::Path;

const MM_PER_INCH: f64 = 25.4;
/// How far apart, mm, an arc's printed ends may be from its centre before the post nudges
/// the centre a printed step to bring them closer (controllers allow 0.005 mm or so).
const CLOSE_ENOUGH: f64 = 0.0005;
/// Numeric values a post can print; the ones in `LENGTHS` become inches for inch posts
/// (feed rates are lengths per minute).
const NUMBERS: &[&str] = &[
    "ACROSS",
    "X",
    "Y",
    "Z",
    "F",
    "FC",
    "FP",
    "S",
    "T",
    "N",
    "SAFEZ",
    "XH",
    "YH",
    "ZH",
    "PARKX",
    "PARKY",
    "PARKZ",
    "XLENGTH",
    "YLENGTH",
    "ZLENGTH",
    "XMIN",
    "XMAX",
    "YMIN",
    "YMAX",
    "ZMIN",
    "ZMAX",
    "TDIA",
    "WRAP_DIA",
    "FI",
    "P",
    "I",
    "J",
    "IA",
    "JA",
    "ArcStartX",
    "ArcStartY",
    "ArcMidX",
    "ArcMidY",
    "ArcMidXI",
    "ArcMidYI",
    "Radius",
    "Angle",
    "DWELL",
    "SPINUP",
    "DB",
    "DBX",
    "DBY",
    "CD",
];
const LENGTHS: &[&str] = &[
    "ACROSS",
    "X",
    "Y",
    "Z",
    "F",
    "FC",
    "FP",
    "SAFEZ",
    "XH",
    "YH",
    "ZH",
    "PARKX",
    "PARKY",
    "PARKZ",
    "XLENGTH",
    "YLENGTH",
    "ZLENGTH",
    "XMIN",
    "XMAX",
    "YMIN",
    "YMAX",
    "ZMIN",
    "ZMAX",
    "TDIA",
    "WRAP_DIA",
    "I",
    "J",
    "IA",
    "JA",
    "ArcStartX",
    "ArcStartY",
    "ArcMidX",
    "ArcMidY",
    "ArcMidXI",
    "ArcMidYI",
    "Radius",
];
/// Numbers printed to three places when a post gives no `VAR` format; the rest print
/// whole (`re/SPEC.md`, the vocabulary table).
const THREE_PLACES: &[&str] = &[
    "X",
    "Y",
    "Z",
    "SAFEZ",
    "XH",
    "YH",
    "ZH",
    "PARKX",
    "PARKY",
    "PARKZ",
    "XLENGTH",
    "YLENGTH",
    "ZLENGTH",
    "XMIN",
    "XMAX",
    "YMIN",
    "YMAX",
    "ZMIN",
    "ZMAX",
    "TDIA",
    "WRAP_DIA",
    "FI",
    "I",
    "J",
    "IA",
    "JA",
    "ArcStartX",
    "ArcStartY",
    "ArcMidX",
    "ArcMidY",
    "ArcMidXI",
    "ArcMidYI",
    "Radius",
    "Angle",
];
const TEXTS: [&str; 20] = [
    "TOOLNAME",
    "SPINUP_SECONDS",
    "TOOLPATH_NAME",
    "TP_FILENAME",
    "TP_DIR",
    "TP_EXT",
    "PATHNAME",
    "FILE_NAME",
    "FILE_PATHNAME",
    "DATE",
    "TIME",
    "Z_ORIGIN",
    "XY_ORIGIN",
    "UNITS",
    "PRODUCT",
    "POST_NAME",
    "FILE_NOTES",
    "TOOLPATH_NOTES",
    "TOOL_NOTES",
    "DL",
];
/// Texts that are lists: a template line holding one is written once per item.
const LISTS: [&str; 2] = ["TOOLS_USED", "TOOLPATHS_OUTPUT"];
/// The blocks Cut Craft writes; a post may define others (arcs, lasers...) that it never uses.
const WRITTEN: [&str; 28] = [
    "JET_TOOL_ON",
    "JET_TOOL_POWER",
    "JET_TOOL_OFF",
    "HEADER",
    "FOOTER",
    "TOOLCHANGE",
    "SPINDLE_ON",
    "NEW_SEGMENT",
    "INITIAL_RAPID_MOVE",
    "RAPID_MOVE",
    "FIRST_PLUNGE_MOVE",
    "PLUNGE_MOVE",
    "FIRST_FEED_MOVE",
    "FEED_MOVE",
    "FIRST_CW_ARC_MOVE",
    "CW_ARC_MOVE",
    "FIRST_CCW_ARC_MOVE",
    "CCW_ARC_MOVE",
    "FIRST_CW_HELICAL_ARC_MOVE",
    "CW_HELICAL_ARC_MOVE",
    "FIRST_CCW_HELICAL_ARC_MOVE",
    "CCW_HELICAL_ARC_MOVE",
    "DWELL_MOVE",
    "SETUP_DRILL_BANK",
    "DRILL_BANK_PATTERN",
    "DRILL_BANK_EXTEND_DRILL",
    "DRILL_BANK_RETRACT_DRILL",
    "FINAL_DRILL_BANK_ACTION",
];
// ponytail: arcs are fitted to cuts only, so the helical plunge arc and tape splitting blocks
// are never written; posts using unknown variables in written blocks are refused
/// The largest arc written when the post sets no `MAX_ARC_RADIUS`: flatter runs stay lines,
/// so near-straight ones never print huge centres.
// ponytail: the help's example MAX_ARC_RADIUS, in mm
const LARGEST_ARC: f64 = 1000.0;

/// Cut Craft's own posts, used until one is installed (`crates/post/posts/`).
const DEFAULT_FLAT: &str = include_str!("../posts/fluidnc_atc_mm.pp");
const DEFAULT_ROTARY: &str = include_str!("../posts/fluidnc_atc_wrap_a_mm.pp");

/// The file being written, for the variables that describe it.
pub struct File<'a> {
    pub path: &'a Path,
    /// The job it comes from, once saved.
    pub job: Option<&'a Path>,
    /// When it was made, local time.
    pub made: chrono::NaiveDateTime,
    /// The machine's tool changer: the tool loaded in each slot by name, slot 1 first. A
    /// tool prints its slot as `[T]`; every tool used must be loaded. Empty for a machine
    /// without one: tools are numbered in the order they are first used.
    pub slots: &'a [Option<String>],
    /// The job's notes, variables filled in, as lines safe in a comment
    /// (`cut_craft_core::notes::gcode_lines`): `[FILE_NOTES]`, and comment lines under the
    /// header when the job asks for them (Notes ▸ Put these notes at the top of the G-code).
    /// Empty: the notes as typed, variables left unfilled.
    pub notes: &'a [String],
}

/// The notes as comment-safe lines: the ones `file` brings (variables filled in), else the
/// job's own as typed.
fn notes_lines(doc: &Document, file: &File) -> Vec<String> {
    if file.notes.is_empty() {
        cut_craft_core::notes::gcode_lines(&doc.notes, &[])
    } else {
        file.notes.to_vec()
    }
}

/// How long, in seconds, `toolpath` waits at the bottom of each plunge (Drilling's dwell).
fn dwell(toolpath: &Toolpath) -> f64 {
    toolpath
        .params
        .get("dwell")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0)
}

#[derive(Clone, Debug)]
pub struct Post {
    pub name: String,
    pub extension: String,
    /// Output in inches instead of millimetres.
    pub inches: bool,
    line_ending: String,
    numbering: Numbering,
    /// (original, replacement) characters applied to text values.
    substitute: Vec<(char, char)>,
    /// A rotary post: the flat axis (`"X"` or `"Y"`) wrapped round the cylinder, and the
    /// letter its angle prints with.
    wrap: Option<(&'static str, String)>,
    /// Feed moves print `[FI]`, one over their time in minutes (G93).
    inverse_time: bool,
    /// The machine's rotary, for a wrapping post: how far A may turn, its backlash and top
    /// speed ([`Post::with_rotary`]). Without it A is written just as the job unrolls.
    rotary: Option<cut_craft_core::rotary_axis::RotaryAxis>,
    /// Laser toolpaths may be written (`LASER_SUPPORT = "YES"`).
    laser: bool,
    /// Arcs smaller (`MIN_ARC_RADIUS`) or larger (`MAX_ARC_RADIUS`) than these stay lines, in
    /// the post's units.
    arc_radius: (Option<f64>, Option<f64>),
    formats: HashMap<String, Format>,
    /// How `[DL]` lists a drill bank's drills: before each number, between numbers, and
    /// between the first two (`VAR DRILL_LIST = [DL|prefix|separator|first separator]`).
    drill_list: (String, String, String),
    blocks: HashMap<String, Vec<String>>,
    /// Where a pause for a bit change takes the head ([`Pause::park`]); printed as `[PARKX]`
    /// `[PARKY]` `[PARKZ]`.
    park: Option<[f64; 3]>,
    /// How long a pause for a bit change waits for the spindle, seconds; printed as
    /// `[SPINUP]` the way the post prints its own dwell (`[DWELL]`), so a post that gives
    /// dwells in milliseconds gets milliseconds.
    spin_up: f64,
}

/// The block written after `SPINDLE_ON` at a bit change: the spin-up wait, when the tool
/// change leaves starting the spindle to `SPINDLE_ON`. Not a name a post file can use.
pub(crate) const AFTER_SPINDLE_ON: &str = "+SPIN_UP";

/// The spindle's time to come up to speed after a pause Cut Craft writes, seconds.
pub const DEFAULT_SPIN_UP: f64 = 3.0;

/// The note before the wait for the spindle after a pause, saying how long it is in
/// seconds whatever units the dwell prints in.
pub(crate) const SPIN_UP_NOTE: &str =
    "(Waits [SPINUP_SECONDS] s for the spindle to come up to speed)";

/// `value` seconds as a note says them: to a tenth, without a trailing `.0`.
pub(crate) fn seconds(value: f64) -> String {
    let text = format!("{:.1}", value.max(0.0));
    text.strip_suffix(".0")
        .map_or_else(|| text.clone(), str::to_owned)
}

/// How [`Post::with_pauses`] stops for a bit to be changed by hand.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pause {
    /// The spindle's time to come up to speed once resumed, seconds; `None` keeps the
    /// post's own wait.
    pub spin_up: Option<f64>,
    /// Where the head waits for the change: X and Y from the job's X0 Y0, and Z above the
    /// top of the material (never lower than the safe Z), mm. `None` stays where the bit
    /// lifted to the safe Z. A post that wraps onto a rotary axis always stays.
    pub park: Option<[f64; 3]>,
}

#[derive(Clone, Copy, Debug)]
struct Numbering {
    start: f64,
    increment: f64,
    maximum: f64,
}

/// How one variable prints: `[LABEL|WHEN|PREFIX|FORMAT|MULTIPLIER]`.
#[derive(Clone, Debug)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the .pp format's independent output flags"
)]
struct Format {
    only_changes: bool,
    prefix: String,
    left: bool,
    plus: bool,
    zero: bool,
    hash: bool,
    width: usize,
    decimals: usize,
    separator: char,
    multiplier: f64,
}

impl Default for Format {
    /// A whole number, `[S|A||1.0]`.
    fn default() -> Self {
        Self {
            only_changes: false,
            prefix: String::new(),
            left: false,
            plus: false,
            zero: false,
            hash: false,
            width: 1,
            decimals: 0,
            separator: '.',
            multiplier: 1.0,
        }
    }
}

impl Format {
    /// How `label` prints when the post gives it no `VAR` line.
    fn default_for(label: &str) -> Self {
        Self {
            decimals: if THREE_PLACES.contains(&label) { 3 } else { 0 },
            ..Self::default()
        }
    }

    fn parse(spec: &str) -> Result<(String, Self), String> {
        let inner = spec.trim().trim_start_matches('[').trim_end_matches(']');
        let parts: Vec<&str> = inner.split('|').collect();
        let label = parts[0].trim().to_owned();
        let mut f = Self {
            only_changes: parts.get(1).is_some_and(|w| w.trim() == "C"),
            ..Self::default()
        };
        parts
            .get(2)
            .copied()
            .unwrap_or_default()
            .clone_into(&mut f.prefix);
        let number = parts.get(3).copied().unwrap_or("1.0").trim();
        let mut chars = number.chars().peekable();
        while let Some(&c) = chars.peek() {
            match c {
                '-' => f.left = true,
                '+' => f.plus = true,
                '#' => f.hash = true,
                _ => break,
            }
            chars.next();
        }
        let rest: String = chars.collect();
        if rest.len() > 1 && rest.starts_with('0') && rest.as_bytes()[1].is_ascii_digit() {
            f.zero = true;
        }
        let (width, decimals) = rest.split_once(['.', ',']).unwrap_or((&rest, "0"));
        f.separator = if rest.contains(',') { ',' } else { '.' };
        f.width = width
            .parse()
            .map_err(|_| format!("bad number format `{number}` for [{label}]"))?;
        f.decimals = decimals
            .parse()
            .map_err(|_| format!("bad number format `{number}` for [{label}]"))?;
        if let Some(m) = parts.get(4) {
            f.multiplier = m
                .trim()
                .parse()
                .map_err(|_| format!("bad multiplier `{m}` for [{label}]"))?;
        }
        Ok((label, f))
    }

    fn number(&self, value: f64) -> String {
        let v = value * self.multiplier;
        let mut digits = format!("{:.*}", self.decimals, v.abs());
        if self.separator != '.' {
            digits = digits.replace('.', &self.separator.to_string());
        }
        if self.decimals == 0 && self.hash {
            digits.push(self.separator);
        }
        let negative = v < 0.0 && digits.chars().any(|c| matches!(c, '1'..='9'));
        let sign = if negative {
            "-"
        } else if self.plus {
            "+"
        } else {
            ""
        };
        let len = sign.len() + digits.len();
        let body = if self.zero && !self.left && len < self.width {
            format!("{sign}{}{digits}", "0".repeat(self.width - len))
        } else {
            format!("{sign}{digits}")
        };
        let width = self.width;
        let padded = if self.left {
            format!("{body:<width$}")
        } else {
            format!("{body:>width$}")
        };
        format!("{}{padded}", self.prefix)
    }
}

impl Post {
    /// Cut Craft's default post for `job`: grbl / `FluidNC` with a tool changer, wrapping the
    /// side that goes round onto A (in inverse time) for a rotary job.
    pub fn default_for(job: &Job) -> Self {
        let Some(rotary) = job.rotary else {
            return Self::parse(DEFAULT_FLAT).expect("the default post parses");
        };
        let mut post = Self::parse(DEFAULT_ROTARY).expect("the default rotary post parses");
        // Y goes round a cylinder along X; X goes round one along Y
        if rotary.along == Along::Y {
            post.wrap = Some(("X", "A".into()));
            if let Some(across) = post.formats.get_mut("ACROSS") {
                "X".clone_into(&mut across.prefix);
            }
        }
        post
    }

    /// It can change tools within one file: it has a `TOOLCHANGE` block with something in
    /// it. Toolpaths using different tools need one, or files of their own.
    pub fn changes_tools(&self) -> bool {
        self.blocks.get("TOOLCHANGE").is_some_and(|b| !b.is_empty())
    }

    /// Its bit changes stop the machine for the tool to be changed by hand: a `TOOLCHANGE`
    /// block with a stop (`M0`/`M1`) and no automatic change (`M6`).
    pub fn pauses_for_tools(&self) -> bool {
        let has = |codes: &[&str]| {
            self.blocks.get("TOOLCHANGE").is_some_and(|lines| {
                lines.iter().any(|line| {
                    line.split_whitespace()
                        .any(|word| codes.contains(&word.to_ascii_uppercase().as_str()))
                })
            })
        };
        has(&["M0", "M00", "M1", "M01"]) && !has(&["M6", "M06"])
    }

    /// This post with each bit change a stop to change the tool by hand: its own tool
    /// change when it already pauses, else one that lifts to the safe Z, stops the spindle,
    /// names the next tool, stops (`M0`), starts the spindle again and waits for it, using
    /// only the variables the post has. Either way `pause` adds the trip to the change
    /// position and sets the wait ([`Self::paused_as`]). The header's bit change (`M6`)
    /// becomes a note to fit the first tool before starting.
    #[must_use]
    pub fn with_pauses(&self, pause: Pause) -> Self {
        if self.pauses_for_tools() {
            return self.paused_as(pause);
        }
        let has = |label: &str| self.formats.contains_key(label);
        let mut lines = vec![];
        if has("SAFEZ") {
            lines.push("G0 [SAFEZ]".to_owned());
        }
        lines.push("M5".to_owned());
        let tool = if has("T") {
            "[T]: [TOOLNAME]"
        } else {
            "[TOOLNAME]"
        };
        lines.push(format!("(Change to {tool}, set Z zero, then resume)"));
        lines.push("M0".to_owned());
        lines.push(if has("S") { "[S] M3" } else { "M3" }.to_owned());
        // the spindle's time to come up to speed, in the post's own dwell units
        lines.push(SPIN_UP_NOTE.to_owned());
        lines.push("G4 [SPINUP]".to_owned());
        let mut post = self.clone();
        post.spin_up_as_dwell();
        post.blocks.insert("TOOLCHANGE".to_owned(), lines);
        // the first tool is fitted by hand before starting, not loaded by the changer
        let fitted = format!("(First tool {tool}, fitted and zeroed before starting)");
        if let Some(header) = post.blocks.get_mut("HEADER") {
            for line in header.iter_mut() {
                if line
                    .split_whitespace()
                    .any(|w| matches!(w.to_ascii_uppercase().as_str(), "M6" | "M06"))
                {
                    line.clone_from(&fitted);
                }
            }
        }
        post.paused_as(pause)
    }

    /// This pausing post with `pause` made in its bit change: the head lifted to the
    /// change position's Z and taken to its X Y around the spindle stopping (`M5`, else just
    /// before the stop), and the wait after the spindle starts again (`G4`) set to
    /// `pause`'s spin-up. A rotary (wrapping) post keeps the head where it lifted, as its
    /// position would print as an angle.
    fn paused_as(&self, pause: Pause) -> Self {
        let mut post = self.clone();
        let Some(lines) = self.blocks.get("TOOLCHANGE") else {
            return post;
        };
        let words = |line: &str| -> Vec<String> {
            line.split_whitespace()
                .map(str::to_ascii_uppercase)
                .collect()
        };
        let is =
            |line: &str, codes: &[&str]| words(line).iter().any(|w| codes.contains(&w.as_str()));
        let stop = lines
            .iter()
            .position(|l| is(l, &["M0", "M00", "M1", "M01"]));
        let timed = pause.spin_up.is_some();
        if let Some(seconds) = pause.spin_up {
            post.spin_up = seconds.max(0.0);
        }
        let Some(lines) = post.blocks.get_mut("TOOLCHANGE") else {
            return post;
        };
        let mut after_start = false;
        if timed {
            // the wait follows the spindle starting again after the stop: a wait there is
            // replaced, else one is added after the start, else (the start left to
            // SPINDLE_ON) it is written after that block
            let after = stop.unwrap_or(0);
            let start = lines
                .iter()
                .skip(after)
                .position(|l| is(l, &["M3", "M03", "M4", "M04"]))
                .map(|at| after + at);
            match start {
                Some(start) => {
                    let wait = lines
                        .iter()
                        .skip(start)
                        .position(|l| is(l, &["G4", "G04"]))
                        .map(|at| start + at);
                    let at = if let Some(wait) = wait {
                        lines.remove(wait);
                        wait
                    } else {
                        start + 1
                    };
                    lines.insert(at, "G4 [SPINUP]".to_owned());
                    if lines.get(at - 1).map(String::as_str) != Some(SPIN_UP_NOTE) {
                        lines.insert(at, SPIN_UP_NOTE.to_owned());
                    }
                }
                None => after_start = true,
            }
        }
        let park = pause.park.filter(|_| self.wrap.is_none());
        if let (Some(park), Some(stop)) = (park, stop) {
            let off = lines[..stop].iter().position(|l| is(l, &["M5", "M05"]));
            // lift where the bit is, stop the spindle, then cross to the change position
            let (up, across) = off.map_or((stop, stop), |at| (at, at + 1));
            lines.insert(across, "G0 [PARKX] [PARKY]".to_owned());
            lines.insert(up, "G0 [PARKZ]".to_owned());
            for (label, axis) in [("PARKX", "X"), ("PARKY", "Y"), ("PARKZ", "Z")] {
                let mut format = self
                    .formats
                    .get(axis)
                    .cloned()
                    .unwrap_or_else(|| Format::default_for(axis));
                // a post that writes the letter in its own lines (`X[X]`) has no prefix
                if format.prefix.is_empty() {
                    axis.clone_into(&mut format.prefix);
                }
                format.only_changes = false;
                post.formats.insert(label.to_owned(), format);
            }
            post.park = Some(park);
        }
        if after_start {
            post.blocks.insert(
                AFTER_SPINDLE_ON.to_owned(),
                vec![SPIN_UP_NOTE.to_owned(), "G4 [SPINUP]".to_owned()],
            );
        }
        if timed {
            post.spin_up_as_dwell();
        }
        post
    }

    /// `[SPINUP]` printed as the post prints its own dwell (`[DWELL]`: its letter, places
    /// and multiplier, so seconds or milliseconds as its controller reads them), else as
    /// `P` and seconds to a tenth.
    fn spin_up_as_dwell(&mut self) {
        let mut format = self
            .formats
            .get("DWELL")
            .cloned()
            .unwrap_or_else(|| Format {
                prefix: "P".to_owned(),
                decimals: 1,
                ..Format::default()
            });
        // a post that writes the letter in its own dwell line (`G4 P[DWELL]`) has none
        if format.prefix.is_empty() {
            "P".clone_into(&mut format.prefix);
        }
        format.only_changes = false;
        self.formats.insert("SPINUP".to_owned(), format);
    }

    /// This post writing for a machine whose rotary is `axis`: A wound no further than it may
    /// turn, its backlash taken up, and inverse-time feeds kept to the bit's real speed.
    /// Only a wrapping post uses it.
    #[must_use]
    pub fn with_rotary(mut self, axis: cut_craft_core::rotary_axis::RotaryAxis) -> Self {
        self.rotary = Some(axis);
        self
    }

    /// The axis it wraps onto the rotary one (`X` or `Y`); `None` for a flat post.
    pub fn wraps(&self) -> Option<&str> {
        self.wrap.as_ref().map(|(axis, _)| *axis)
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let mut post = Self {
            name: String::new(),
            extension: "nc".into(),
            inches: false,
            line_ending: "\n".into(),
            numbering: Numbering {
                start: 0.0,
                increment: 1.0,
                maximum: f64::MAX,
            },
            substitute: vec![],
            wrap: None,
            inverse_time: false,
            rotary: None,
            laser: false,
            arc_radius: (None, None),
            formats: HashMap::new(),
            drill_list: (String::new(), ",".into(), ",".into()),
            blocks: HashMap::new(),
            park: None,
            spin_up: DEFAULT_SPIN_UP,
        };
        let mut block: Option<String> = None;
        for line in text.lines().map(str::trim) {
            if line.is_empty() || line.starts_with('+') {
                continue;
            }
            if let Some(name) = line.strip_prefix("begin ") {
                let name = match name.trim() {
                    // the format's documentation names the block both ways
                    "DRILL_BANK_FINAL_ACTION" => "FINAL_DRILL_BANK_ACTION",
                    name => name,
                }
                .to_owned();
                post.blocks.entry(name.clone()).or_default();
                block = Some(name);
            } else if line.starts_with('"') {
                let name = block
                    .as_ref()
                    .ok_or("a template line comes before any `begin` block")?;
                post.blocks
                    .get_mut(name)
                    .expect("block was created at `begin`")
                    .push(unquote(line));
            } else if let Some(rest) = line.strip_prefix("VAR ") {
                let (_, spec) = rest
                    .split_once('=')
                    .ok_or_else(|| format!("bad VAR line: {line}"))?;
                if let Some(list) = drill_list(spec) {
                    post.drill_list = list;
                    continue;
                }
                let (label, format) = Format::parse(spec)?;
                post.formats.insert(label, format);
            } else if let Some((key, value)) = line.split_once('=') {
                post.global(key.trim(), value)?;
            }
        }
        post.check()?;
        Ok(post)
    }

    fn global(&mut self, key: &str, value: &str) -> Result<(), String> {
        let number = || {
            value
                .trim()
                .parse::<f64>()
                .map_err(|_| format!("{key} must be a number"))
        };
        match key {
            "POST_NAME" => self.name = unquote(value),
            "FILE_EXTENSION" => self.extension = unquote(value),
            "UNITS" => self.inches = unquote(value).eq_ignore_ascii_case("INCHES"),
            "LINE_ENDING" => {
                self.line_ending = unquote(value)
                    .split(['[', ']'])
                    .filter(|code| !code.is_empty())
                    .map(|code| {
                        code.parse::<u8>()
                            .map(char::from)
                            .map_err(|_| format!("bad LINE_ENDING code {code}"))
                    })
                    .collect::<Result<_, _>>()?;
            }
            "LINE_NUMBER" => self.numbering.start = number()?,
            "LINE_NUMBER_INCREMENT" => self.numbering.increment = number()?,
            "LINE_NUMBER_MAXIMUM" => self.numbering.maximum = number()?,
            "SUBSTITUTE" => {
                let chars: Vec<char> = unquote(value).chars().collect();
                self.substitute = chars
                    .chunks(2)
                    .filter(|p| p.len() == 2)
                    .map(|p| (p[0], p[1]))
                    .collect();
            }
            "ROTARY_WRAP_X" => self.wrap = Some(("X", unquote(value))),
            "ROTARY_WRAP_Y" => self.wrap = Some(("Y", unquote(value))),
            "INVERSE_TIME_MODE" => self.inverse_time = unquote(value).eq_ignore_ascii_case("YES"),
            "LASER_SUPPORT" => self.laser = unquote(value).eq_ignore_ascii_case("YES"),
            "MIN_ARC_RADIUS" => self.arc_radius.0 = Some(number()?),
            "MAX_ARC_RADIUS" => self.arc_radius.1 = Some(number()?),
            // other globals (DIRECT_OUTPUT, TAPE_SPLITTING, ...) do not change the text
            _ => {}
        }
        Ok(())
    }

    fn check(&self) -> Result<(), String> {
        if self.inverse_time && self.wrap.is_none() {
            return Err(
                "INVERSE_TIME_MODE can only be used when wrapping is on (ROTARY_WRAP_X or _Y)"
                    .into(),
            );
        }
        for required in ["RAPID_MOVE", "FEED_MOVE"] {
            if !self.blocks.contains_key(required) {
                return Err(format!("the post has no {required} block"));
            }
        }
        let written = WRITTEN.iter().filter_map(|name| self.blocks.get(*name));
        for line in written.flatten() {
            for label in labels(line) {
                if !NUMBERS.contains(&label) && !TEXTS.contains(&label) && !LISTS.contains(&label) {
                    return Err(format!(
                        "the post uses [{label}], which is not supported yet"
                    ));
                }
            }
        }
        Ok(())
    }
}

fn z_origin(job: &Job) -> &'static str {
    match (job.z_zero, job.rotary) {
        (ZZero::MaterialSurface, None) => "Material Surface",
        (ZZero::MachineBed, None) => "Machine Bed",
        // as Job Setup names them: the top of the material at A=0, or the axis it turns on
        (ZZero::MaterialSurface, Some(_)) => "Material Top",
        (ZZero::MachineBed, Some(_)) => "Turning Axis",
    }
}

/// The own wording is not known (`re/SPEC.md`); these are ours.
fn xy_origin(datum: Datum) -> &'static str {
    match datum {
        Datum::BottomLeft => "Bottom Left Corner",
        Datum::TopLeft => "Top Left Corner",
        Datum::TopRight => "Top Right Corner",
        Datum::BottomRight => "Bottom Right Corner",
        Datum::Centre => "Center",
    }
}

/// The values variables print from at one point in the output.
#[derive(Clone, Default)]
struct Values {
    numbers: HashMap<&'static str, f64>,
    texts: HashMap<&'static str, String>,
    /// Lists, each item written on its own copy of the line (`LISTS`).
    lists: HashMap<&'static str, Vec<String>>,
}

impl Values {
    fn tool(&mut self, tool: &cut_craft_core::toolpath::Tool, number: u32) {
        self.numbers.insert("T", f64::from(number));
        self.numbers.insert("S", tool.spindle_speed);
        self.numbers.insert("FC", tool.feed_rate);
        self.numbers.insert("FP", tool.plunge_rate);
        self.numbers.insert("TDIA", tool.diameter);
    }

    /// The values only arcs print, for `arc` starting at `from`.
    fn arc(&mut self, from: DVec3, arc: &arcs::Arc) {
        let mid = arc.mid(from);
        for (label, value) in [
            ("I", arc.centre.x - from.x),
            ("J", arc.centre.y - from.y),
            ("IA", arc.centre.x),
            ("JA", arc.centre.y),
            ("ArcStartX", from.x),
            ("ArcStartY", from.y),
            ("ArcMidX", mid.x),
            ("ArcMidY", mid.y),
            ("ArcMidXI", mid.x - from.x),
            ("ArcMidYI", mid.y - from.y),
            ("Radius", from.truncate().distance(arc.centre)),
            // ponytail: radians anticlockwise (re/SPEC.md)
            ("Angle", arc.sweep),
        ] {
            self.numbers.insert(label, value);
        }
    }
}

/// One move as it is written: as the toolpath has it (with the laser's power along it), or
/// an arc replacing a run of its cuts.
#[derive(Clone, Copy)]
enum Step {
    /// A straight move, with a laser's power and, on a rotary, how far off the turning axis
    /// the bit is held (mm).
    Line(MoveKind, DVec3, f64, f64),
    /// An arc cut at the feed rate, or gone down at the plunge rate (a helix).
    Arc(MoveKind, arcs::Arc),
}

/// What `toolpath` writes, in parts: all its moves, or for a drill bank toolpath each plunge
/// of the head (its hit) with the moves of its first hole moved to where the head is.
fn parts(toolpath: &Toolpath) -> Vec<(Option<&BankHit>, std::borrow::Cow<'_, Toolpath>)> {
    let Some(bank) = &toolpath.drill_bank else {
        return vec![(None, std::borrow::Cow::Borrowed(toolpath))];
    };
    bank.hits
        .iter()
        .map(|hit| {
            // every hole of a hit is drilled with the same moves: the first hole's, up to its
            // rise back to the height it came in at (ways round keep-out zones may come first)
            let hit_moves = toolpath
                .moves
                .get(hit.start..hit.end.min(toolpath.moves.len()))
                .unwrap_or_default();
            let safe = hit_moves.first().map_or(0.0, |m| m.to.z);
            let mut plunged = false;
            let first_hole = hit_moves
                .iter()
                .position(|m| {
                    plunged |= m.kind == MoveKind::Plunge;
                    plunged && m.kind == MoveKind::Rapid && m.to.z >= safe - 1e-9
                })
                .map_or(hit_moves.len(), |i| i + 1);
            let moves = hit_moves[..first_hole]
                .iter()
                .map(|m| cut_craft_core::toolpath::Move {
                    kind: m.kind,
                    to: DVec3::new(hit.head.x, hit.head.y, m.to.z),
                })
                .collect();
            let part = Toolpath {
                moves,
                power: vec![],
                drill_bank: None,
                fallbacks: vec![],
                across: vec![],
                params: serde_json::Value::Null,
                vectors: vec![],
                name: String::new(),
                strategy: String::new(),
                tool: toolpath.tool.clone(),
                visible: true,
                stale: false,
                group: None,
            };
            (Some(hit), std::borrow::Cow::Owned(part))
        })
        .collect()
}

/// Each tool's name once, in the order the toolpaths use them.
/// Each tool's `[T]`: on a machine with a tool changer its holder, or 0 for a bit not in
/// the magazine (changed by hand); without one, its own bit number, else the order it is
/// first used.
fn tool_numbers<'a>(
    tools: &[(&str, u32)],
    slots: &'a [Option<String>],
) -> impl Fn(&str) -> u32 + 'a {
    let order: Vec<(String, u32)> = tools.iter().map(|&(t, n)| (t.to_owned(), n)).collect();
    move |name: &str| {
        if slots.is_empty() {
            return order
                .iter()
                .position(|(t, _)| t == name)
                .map_or(1, |i| match order[i].1 {
                    0 => u32::try_from(i + 1).unwrap_or(1),
                    own => own,
                });
        }
        slots
            .iter()
            .position(|s| s.as_deref() == Some(name))
            .and_then(|i| u32::try_from(i + 1).ok())
            .unwrap_or(0)
    }
}

/// Each tool `toolpaths` use, in the order first used, with the `[T]` a post writes for it
/// on a machine with these tool changer `slots` (empty for none): its slot, or 0 for a bit
/// changed by hand; without a changer its own number, else the order it is first used.
pub fn tools_and_numbers(toolpaths: &[&Toolpath], slots: &[Option<String>]) -> Vec<(String, u32)> {
    let tools = tools_used(toolpaths);
    let number = tool_numbers(&tools, slots);
    tools
        .iter()
        .map(|&(name, _)| (name.to_owned(), number(name)))
        .collect()
}

fn tools_used<'a>(toolpaths: &[&'a cut_craft_core::toolpath::Toolpath]) -> Vec<(&'a str, u32)> {
    let mut tools: Vec<(&str, u32)> = vec![];
    for t in toolpaths {
        if !tools.iter().any(|(name, _)| *name == t.tool.name) {
            tools.push((&t.tool.name, t.tool.number));
        }
    }
    tools
}

/// Text being written, with the state that modal (`C`) variables and line numbers need.
struct Output<'a> {
    post: &'a Post,
    text: String,
    /// Last text each only-when-changed variable printed.
    last: HashMap<String, String>,
    line_number: f64,
}

impl Output<'_> {
    /// Switches a laser on, to a new power or off for the move about to be written, given
    /// the power it is `burning` at (`None`: off).
    fn jet(&mut self, v: &Values, kind: MoveKind, power: f64, burning: &mut Option<f64>) {
        match (kind, *burning) {
            (MoveKind::Rapid, Some(_)) => {
                self.block("JET_TOOL_OFF", v);
                *burning = None;
            }
            (MoveKind::Rapid, None) => {}
            (_, None) => {
                self.block("JET_TOOL_ON", v);
                *burning = Some(power);
            }
            (_, Some(was)) if (was - power).abs() > 1e-9 => {
                self.block("JET_TOOL_POWER", v);
                *burning = Some(power);
            }
            _ => {}
        }
    }

    /// Puts down `drills` of `bank` (none: all up) given those `down` already: by pattern,
    /// one `DRILL_BANK_PATTERN` when the drills change; else each drill that goes up, then
    /// each that comes down, on its own.
    fn drop_drills(
        &mut self,
        v: &mut Values,
        bank: &DrillBank,
        drills: &[DrillId],
        by_pattern: bool,
        down: &mut Vec<DrillId>,
    ) {
        let same = drills.len() == down.len() && drills.iter().all(|d| down.contains(d));
        if same || (by_pattern && drills.is_empty()) {
            return;
        }
        let chosen = bank.selection(drills);
        v.numbers.insert("DB", f64::from(chosen.all));
        v.numbers.insert("DBX", f64::from(chosen.horizontal));
        v.numbers.insert("DBY", f64::from(chosen.vertical));
        v.texts
            .insert("DL", self.post.drill_list_text(&chosen.list));
        if by_pattern {
            self.block("DRILL_BANK_PATTERN", v);
        } else {
            let number = |id: DrillId| bank.selection(&[id]).list.first().copied();
            for &id in down.iter().filter(|d| !drills.contains(d)) {
                v.numbers.insert("CD", number(id).map_or(0.0, f64::from));
                self.block("DRILL_BANK_RETRACT_DRILL", v);
            }
            for &id in drills.iter().filter(|d| !down.contains(d)) {
                v.numbers.insert("CD", number(id).map_or(0.0, f64::from));
                self.block("DRILL_BANK_EXTEND_DRILL", v);
            }
        }
        *down = drills.to_vec();
    }

    fn block(&mut self, name: &str, v: &Values) {
        let Some(lines) = self.post.blocks.get(name) else {
            return;
        };
        for template in lines {
            let list = v
                .lists
                .iter()
                .find(|(label, _)| template.contains(&format!("[{label}]")));
            if let Some((&label, items)) = list {
                for item in items {
                    let mut one = v.clone();
                    one.texts.insert(label, item.clone());
                    self.line(template, &one);
                }
            } else {
                self.line(template, v);
            }
        }
    }

    fn line(&mut self, template: &str, v: &Values) {
        let mut line = String::new();
        let mut rest = template;
        let mut numbered = false;
        while let Some(open) = rest.find('[') {
            let Some(close) = rest[open..].find(']') else {
                break;
            };
            line.push_str(&rest[..open]);
            let label = &rest[open + 1..open + close];
            numbered |= label == "N";
            line.push_str(&self.value(label, v));
            rest = &rest[open + close + 1..];
        }
        line.push_str(rest);
        if numbered {
            let n = self.post.numbering;
            self.line_number = if self.line_number + n.increment > n.maximum {
                n.start
            } else {
                self.line_number + n.increment
            };
        }
        let line = line.trim_end();
        if !line.is_empty() {
            self.text.push_str(line);
            self.text.push_str(&self.post.line_ending);
        }
    }

    /// The job's notes as comments, in the style the post's header writes its own (`;` or
    /// brackets), under a line saying what they are: after the header's own comments and
    /// before its first command (the header written from `header_at`), so they are read
    /// before the spindle starts. Lines come already safe for a comment.
    fn notes(&mut self, header_at: usize, lines: &[String]) {
        if lines.is_empty() {
            return;
        }
        let semicolon = self.post.blocks.get("HEADER").is_some_and(|header| {
            header.iter().any(|l| l.trim_start().starts_with(';'))
                && !header.iter().any(|l| l.trim_start().starts_with('('))
        });
        let comment = |text: &str| {
            if semicolon {
                format!("; {text}")
            } else {
                format!("({text})")
            }
        };
        let mut notes = String::new();
        for line in std::iter::once("Job notes:").chain(lines.iter().map(String::as_str)) {
            notes.push_str(&comment(line));
            notes.push_str(&self.post.line_ending);
        }
        // the first header line that is not a comment
        let ending = self.post.line_ending.as_str();
        let mut at = header_at;
        for line in self.text[header_at..].split_inclusive(ending) {
            let t = line.trim_start();
            if !(t.starts_with('(') || t.starts_with(';') || t.trim().is_empty()) {
                break;
            }
            at += line.len();
        }
        self.text.insert_str(at, &notes);
    }

    fn value(&mut self, label: &str, v: &Values) -> String {
        if let Some(text) = v.texts.get(label) {
            return text
                .chars()
                .map(|c| {
                    self.post
                        .substitute
                        .iter()
                        .find(|(from, _)| *from == c)
                        .map_or(c, |(_, to)| *to)
                })
                .collect();
        }
        if label == "ACROSS" && !v.numbers.contains_key(label) {
            // no toolpath holds the bit off the axis: leave the across axis where it was set up
            return String::new();
        }
        let raw = if label == "N" {
            self.line_number
        } else {
            v.numbers.get(label).copied().unwrap_or_default()
        };
        // a wrapped axis (and its home) is in degrees, printed with the wrap letter
        let wrap = self
            .post
            .wrap
            .as_ref()
            .filter(|(axis, _)| label == *axis || label.strip_suffix('H') == Some(*axis));
        let value = if self.post.inches && LENGTHS.contains(&label) && wrap.is_none() {
            raw / MM_PER_INCH
        } else {
            raw
        };
        let mut format = self
            .post
            .formats
            .get(label)
            .cloned()
            .unwrap_or_else(|| Format::default_for(label));
        if let Some((axis, letter)) = wrap {
            format.prefix = format.prefix.replace(axis, letter);
        }
        let text = format.number(value);
        if format.only_changes {
            if self.last.get(label) == Some(&text) {
                return String::new();
            }
            self.last.insert(label.to_owned(), text.clone());
        }
        text
    }
}

/// `[DL|prefix|separator|first separator]`: how a drill bank's drill list prints; `None`
/// for any other variable.
fn drill_list(spec: &str) -> Option<(String, String, String)> {
    let inner = spec.trim().strip_prefix('[')?.strip_suffix(']')?;
    let mut parts = inner.split('|');
    if parts.next()?.trim() != "DL" {
        return None;
    }
    let prefix = parts.next().unwrap_or_default().to_owned();
    let separator = parts.next().unwrap_or(",").to_owned();
    let first = parts
        .next()
        .map_or_else(|| separator.clone(), str::to_owned);
    Some((prefix, separator, first))
}

fn unquote(s: &str) -> String {
    s.trim()
        .trim_start_matches('"')
        .trim_end_matches('"')
        .to_owned()
}

/// The `[LABEL]`s in a template line.
fn labels(line: &str) -> impl Iterator<Item = &str> {
    line.split('[')
        .skip(1)
        .filter_map(|part| part.split_once(']').map(|(label, _)| label))
}

#[cfg(test)]
mod tests;

#[cfg(test)]
#[test]
fn without_a_tool_changer_a_tool_writes_its_own_number_else_its_order() {
    let tools = [("6 mm End Mill", 7), ("V-Bit", 0)];
    let number = tool_numbers(&tools, &[]);
    assert_eq!((number("6 mm End Mill"), number("V-Bit")), (7, 2));
    // with a changer the holder wins, and a tool not in it is changed by hand: 0
    let slots = [None, Some("V-Bit".to_owned())];
    let number = tool_numbers(&tools, &slots);
    assert_eq!((number("6 mm End Mill"), number("V-Bit")), (0, 2));
}
