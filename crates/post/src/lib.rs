//! Vectric-style `.pp` post-processors: a text format of global settings, `VAR` output formats
//! and `begin BLOCK` templates (see `re/SPEC.md`). `Post::parse` reads one; `Post::render`
//! writes a document's toolpaths through it.

mod arcs;
mod built_in;
mod winding;

pub use built_in::BUILT_IN;

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
const TEXTS: [&str; 19] = [
    "TOOLNAME",
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

    /// Its tool changes stop the machine for the tool to be changed by hand: a `TOOLCHANGE`
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

    /// This post with each tool change a stop to change the tool by hand: its own tool
    /// change when it already pauses, else one that lifts to the safe Z, stops the spindle,
    /// names the next tool, stops (`M0`) and starts the spindle again, using only the
    /// variables the post has. The header's tool change (`M6`) becomes a note to fit the
    /// first tool before starting.
    #[must_use]
    pub fn with_pauses(&self) -> Self {
        if self.pauses_for_tools() {
            return self.clone();
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
        // the spindle's time to come up to speed
        lines.push("G4 P3".to_owned());
        let mut post = self.clone();
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
        post
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

    #[expect(
        clippy::too_many_lines,
        reason = "one pass writing every toolpath in order"
    )]
    pub fn render(&self, doc: &Document, file: &File) -> Result<String, String> {
        let toolpaths: Vec<_> = doc.toolpaths.values().collect();
        let Some(first) = toolpaths.first() else {
            return Err("there are no toolpaths to save".into());
        };
        let tools = tools_used(&toolpaths);
        self.can_write(doc, &toolpaths)?;
        let number = tool_numbers(&tools, file.slots);
        let mut out = Output {
            post: self,
            text: String::new(),
            last: HashMap::new(),
            line_number: self.numbering.start,
        };
        let mut winding = self.winding(&doc.job);
        let wrapped = |label: &str, flat: f64, winding: &mut Option<winding::Winding>| match (
            winding.as_mut(),
            &self.wrap,
        ) {
            (Some(w), Some((axis, _))) if *axis == label => w.degrees(flat),
            _ => self.wrapped(&doc.job, label, flat),
        };
        let mut v = self.job_values(doc, file);
        v.lists.insert(
            "TOOLS_USED",
            tools.iter().map(|&(t, _)| t.to_owned()).collect(),
        );
        v.lists.insert(
            "TOOLPATHS_OUTPUT",
            toolpaths.iter().map(|t| t.name.clone()).collect(),
        );
        v.texts.insert("TOOLPATH_NAME", first.name.clone());
        v.texts.insert("TOOLNAME", first.tool.name.clone());
        v.tool(&first.tool, number(&first.tool.name));
        let header_at = out.text.len();
        out.block("HEADER", &v);
        if doc.notes_in_gcode {
            out.notes(header_at, &notes_lines(doc, file));
        }

        let mut previous_tool: Option<&str> = None;
        let mut previous_speed: Option<f64> = None;
        let mut first_rapid = true;
        // the rate of the feed moves since the last rapid: a new one starts a FIRST_ block,
        // which is where most posts print it
        let mut feeding: Option<f64> = None;
        let home = doc.job.home();
        let mut at = DVec3::new(home.x, home.y, home.z);
        // how far off the turning axis the bit is held, and where that is on the machine's
        // across axis (Y for a rotary along X): from the axis's centre, the way A turns
        let mut held = 0.0;
        // only when a toolpath holds the bit off the axis: otherwise the file leaves the
        // across axis where the machine was set up, as it always has
        let holds_off = toolpaths
            .iter()
            .any(|t| t.across.iter().any(|a| a.abs() > 1e-9));
        let across_at = doc
            .job
            .rotary
            .filter(|_| self.wrap.is_some() && holds_off)
            .map(|r| {
                let axis = self.rotary.unwrap_or_default();
                let sign = axis.across_sign(r.along);
                move |across: f64| axis.centre_across + sign * across
            });
        for toolpath in &toolpaths {
            v.texts.insert("TOOLPATH_NAME", toolpath.name.clone());
            v.texts.insert("TOOLNAME", toolpath.tool.name.clone());
            v.tool(&toolpath.tool, number(&toolpath.tool.name));
            match previous_tool {
                Some(t) if t != toolpath.tool.name => {
                    out.block("TOOLCHANGE", &v);
                    out.block("SPINDLE_ON", &v);
                    // the new bit has just been zeroed by hand, so the machine's height is
                    // unknown: print every value again, Z above all, rather than leave the
                    // first rapid to travel sideways at whatever height the bit was left
                    out.last.clear();
                    // lift straight up to the height the last toolpath left at before any
                    // sideways move: only Z changes, so print X and Y only if the post
                    // always prints them
                    v.numbers.insert("X", wrapped("X", at.x, &mut winding));
                    v.numbers.insert("Y", wrapped("Y", at.y, &mut winding));
                    v.numbers.insert("Z", doc.job.machine_z(at.z));
                    if let Some(across_at) = &across_at {
                        v.numbers.insert("ACROSS", across_at(held));
                    }
                    out.value("X", &v);
                    out.value("Y", &v);
                    out.block("RAPID_MOVE", &v);
                }
                Some(_) => {
                    out.block("NEW_SEGMENT", &v);
                    // the same bit at another speed (another material, or the speed edited
                    // between toolpaths): its feeds print on every move, but the spindle
                    // speed only where the post starts the spindle, so start it again
                    if previous_speed != Some(toolpath.tool.spindle_speed)
                        && toolpath.power.is_empty()
                    {
                        if out.post.blocks.contains_key("SPINDLE_ON") {
                            out.block("SPINDLE_ON", &v);
                        } else {
                            out.line("[S] M3", &v);
                        }
                    }
                }
                None => {}
            }
            previous_tool = Some(&toolpath.tool.name);
            previous_speed = Some(toolpath.tool.spindle_speed);
            let laser = !toolpath.power.is_empty();
            // the laser's power while it is on
            let mut burning: Option<f64> = None;
            // a drill bank: which drills are down, and how the post chooses them
            let bank = toolpath.drill_bank.as_ref();
            let by_pattern = bank.is_some() && self.bank_mode()?;
            let mut down = vec![];
            if bank.is_some() {
                out.block("SETUP_DRILL_BANK", &v);
            }
            for (hit, part) in parts(toolpath) {
                if let (Some(b), Some(hit)) = (bank, hit) {
                    out.drop_drills(&mut v, &b.bank, &hit.drills, by_pattern, &mut down);
                }
                let mut steps = self.steps(&part, &doc.job, at);
                if let Some(w) = winding.as_mut() {
                    steps = w.plan(steps, at);
                }
                for step in steps {
                    let (kind, to, across) = match step {
                        Step::Line(kind, to, _, across) => (kind, to, across),
                        Step::Arc(kind, arc) => (kind, arc.to, 0.0),
                    };
                    let from = std::mem::replace(&mut at, to);
                    let moved_across = across - std::mem::replace(&mut held, across);
                    // the bit moving only across the axis still moves
                    let length = (to.distance_squared(from) + moved_across * moved_across).sqrt();
                    if self.inverse_time && kind != MoveKind::Rapid && length == 0.0 {
                        // a move that goes nowhere has no time to invert
                        continue;
                    }
                    v.numbers.insert("X", wrapped("X", to.x, &mut winding));
                    v.numbers.insert("Y", wrapped("Y", to.y, &mut winding));
                    v.numbers.insert("Z", doc.job.machine_z(to.z));
                    if let Some(across_at) = &across_at {
                        v.numbers.insert("ACROSS", across_at(across));
                    }
                    if let (true, Step::Line(_, _, power, _)) = (laser, step) {
                        v.numbers.insert("P", power);
                        out.jet(&v, kind, power, &mut burning);
                    }
                    let block = if kind == MoveKind::Rapid {
                        feeding = None;
                        let initial = std::mem::replace(&mut first_rapid, false);
                        if initial && self.blocks.contains_key("INITIAL_RAPID_MOVE") {
                            "INITIAL_RAPID_MOVE"
                        } else {
                            "RAPID_MOVE"
                        }
                    } else {
                        let rate = if kind == MoveKind::Plunge {
                            toolpath.tool.plunge_rate
                        } else {
                            toolpath.tool.feed_rate
                        };
                        v.numbers.insert("F", rate);
                        v.numbers.insert(
                            "FI",
                            winding.as_ref().map_or(rate / length, |w| {
                                w.inverse_time(from, to, moved_across, rate)
                            }),
                        );
                        let start = feeding.replace(rate) != Some(rate);
                        if let Step::Arc(_, arc) = step {
                            let (printed_from, arc) = self.as_printed(from, arc);
                            v.arc(printed_from, &arc);
                            self.arc_block(&arc, from, start)
                        } else {
                            self.feed_block(kind, start)
                        }
                    };
                    out.block(block, &v);
                    // a drill waits at the bottom of each peck when its toolpath asks
                    let line = matches!(step, Step::Line(..));
                    if line && kind == MoveKind::Plunge && dwell(toolpath) > 0.0 {
                        v.numbers.insert("DWELL", dwell(toolpath));
                        out.block("DWELL_MOVE", &v);
                    }
                }
            }
            if let Some(b) = bank {
                out.drop_drills(&mut v, &b.bank, &[], by_pattern, &mut down);
                out.block("FINAL_DRILL_BANK_ACTION", &v);
            }
            if burning.is_some() {
                out.block("JET_TOOL_OFF", &v);
            }
        }
        if let (Some(w), Some((axis, _))) = (winding.as_mut(), &self.wrap) {
            // home round the material: unwound only after a spin back
            let (flat, now) = if *axis == "X" {
                (home.x, at.x)
            } else {
                (home.y, at.y)
            };
            let label = if *axis == "X" { "XH" } else { "YH" };
            let w_home = w.home(flat, now);
            v.numbers.insert(label, w.degrees(w_home));
        }
        out.block("FOOTER", &v);
        Ok(out.text)
    }

    /// Refuses what this post cannot write: a flat job for a rotary post, lasers without
    /// laser support.
    fn can_write(
        &self,
        doc: &Document,
        toolpaths: &[&cut_craft_core::toolpath::Toolpath],
    ) -> Result<(), String> {
        if let Some((axis, letter)) = &self.wrap
            && doc.job.rotary.is_none()
        {
            return Err(format!(
                "this post wraps {axis} onto {letter}: set the job up as rotary in Job Setup"
            ));
        }
        // how far the bit is held off the axis goes move by move: a list that has lost step
        // with the moves would write each move after the first gap at another move's place
        if let Some(t) = toolpaths
            .iter()
            .find(|t| !t.across.is_empty() && t.across.len() != t.moves.len())
        {
            return Err(format!(
                "\"{}\" holds the bit off the turning axis, but its moves and its places across \
                 the axis no longer match ({} moves, {} places): calculate it again before saving",
                t.name,
                t.moves.len(),
                t.across.len()
            ));
        }
        if let Some(t) = toolpaths
            .iter()
            .find(|t| t.across.iter().any(|a| a.abs() > 1e-9))
            && (self.wrap.is_none() || !self.formats.contains_key("ACROSS"))
        {
            return Err(format!(
                "\"{}\" holds the bit off the turning axis, and this post can't move the bit \
                 across: pick one of Cut Craft's Rotary A posts",
                t.name
            ));
        }
        if let Some(t) = toolpaths
            .iter()
            .find(|t| dwell(t) > 0.0 && !self.blocks.contains_key("DWELL_MOVE"))
        {
            return Err(format!(
                "{} dwells at the bottom of each peck, but this post has no DWELL_MOVE block",
                t.name
            ));
        }
        if let Some(t) = toolpaths.iter().find(|t| t.drill_bank.is_some()) {
            self.bank_mode()
                .map_err(|why| format!("{} uses a drill bank, but this post {why}", t.name))?;
        }
        if let Some(t) = toolpaths
            .iter()
            .find(|t| !t.power.is_empty() && !self.laser)
        {
            return Err(format!(
                "{} is a laser toolpath, but this post does not support lasers (LASER_SUPPORT)",
                t.name
            ));
        }
        Ok(())
    }

    /// How this post drives a drill bank: by pattern (`true`: one block choosing every drill
    /// at once) or drill by drill (`false`), as the format's documentation says the blocks choose; why
    /// it cannot when it cannot.
    fn bank_mode(&self) -> Result<bool, String> {
        let has = |name: &str| self.blocks.contains_key(name);
        if !has("SETUP_DRILL_BANK") || !has("FINAL_DRILL_BANK_ACTION") {
            return Err("cannot drive one: it needs SETUP_DRILL_BANK and \
                        FINAL_DRILL_BANK_ACTION blocks (ask the machine's maker for a post with \
                        drill bank support)"
                .into());
        }
        let per_drill = (
            has("DRILL_BANK_EXTEND_DRILL"),
            has("DRILL_BANK_RETRACT_DRILL"),
        );
        match (has("DRILL_BANK_PATTERN"), per_drill) {
            (true, (false, false)) => Ok(true),
            (false, (true, true)) => Ok(false),
            (true, _) => Err(
                "has both DRILL_BANK_PATTERN and the extend and retract blocks: \
                              it should have one or the other, not both"
                    .into(),
            ),
            (false, _) => Err("needs a DRILL_BANK_PATTERN block, or both \
                               DRILL_BANK_EXTEND_DRILL and DRILL_BANK_RETRACT_DRILL"
                .into()),
        }
    }

    /// The block for a feed move: plunges have their own if the post does, and a new rate
    /// (`start`) begins with the FIRST_ one.
    fn feed_block(&self, kind: MoveKind, start: bool) -> &'static str {
        let (first, then) = if kind == MoveKind::Plunge && self.blocks.contains_key("PLUNGE_MOVE") {
            ("FIRST_PLUNGE_MOVE", "PLUNGE_MOVE")
        } else {
            ("FIRST_FEED_MOVE", "FEED_MOVE")
        };
        if start && self.blocks.contains_key(first) {
            first
        } else {
            then
        }
    }

    /// How many millimetres `mm` is once printed as `label`: rounded to the places the post
    /// prints it to, in its units.
    fn printed(&self, label: &str, mm: f64) -> f64 {
        let step = self.printed_step(label);
        (mm / step).round() * step
    }

    /// The smallest step, in millimetres, `label` prints in.
    fn printed_step(&self, label: &str) -> f64 {
        let format = self
            .formats
            .get(label)
            .cloned()
            .unwrap_or_else(|| Format::default_for(label));
        let unit = if self.inches && LENGTHS.contains(&label) {
            MM_PER_INCH
        } else {
            1.0
        };
        let places = i32::try_from(format.decimals).unwrap_or(i32::MAX);
        let multiplier = if format.multiplier.abs() > f64::EPSILON {
            format.multiplier.abs()
        } else {
            1.0
        };
        unit / multiplier / 10_f64.powi(places)
    }

    /// `arc` from `from` as the machine reads it: from where the last move was printed to,
    /// with its centre moved onto the line halfway between its printed ends and to the
    /// printed step nearest that keeps both ends the same distance from it. Controllers
    /// check this (GRBL stops with error 33 past 0.005 mm), and rounding to the places an
    /// inch post prints could otherwise put a small arc's ends that far apart.
    fn as_printed(&self, from: DVec3, arc: arcs::Arc) -> (DVec3, arcs::Arc) {
        let at = |p: DVec3| {
            cut_craft_core::glam::DVec2::new(self.printed("X", p.x), self.printed("Y", p.y))
        };
        let (start, end) = (at(from), at(arc.to));
        let chord = end - start;
        let mut centre = arc.centre;
        if chord.length_squared() > f64::EPSILON {
            let middle = (start + end) / 2.0;
            centre -= chord * (centre - middle).dot(chord) / chord.length_squared();
        }
        let (si, sj) = (self.printed_step("I"), self.printed_step("J"));
        let offset = centre - start;
        let (i, j) = ((offset.x / si).round(), (offset.y / sj).round());
        let mismatch = |c: cut_craft_core::glam::DVec2| (c.distance(start) - c.distance(end)).abs();
        let nearest = start + cut_craft_core::glam::DVec2::new(i * si, j * sj);
        // the nearest rounding unless a step either way matches the ends noticeably better
        if mismatch(nearest) <= CLOSE_ENOUGH {
            return (
                start.extend(from.z),
                arcs::Arc {
                    centre: nearest,
                    ..arc
                },
            );
        }
        let best = (-1..=1)
            .flat_map(|a| (-1..=1).map(move |b| (f64::from(a), f64::from(b))))
            .map(|(a, b)| start + cut_craft_core::glam::DVec2::new((i + a) * si, (j + b) * sj))
            .min_by(|a, b| mismatch(*a).total_cmp(&mismatch(*b)))
            .unwrap_or(centre);
        (
            start.extend(from.z),
            arcs::Arc {
                centre: best,
                ..arc
            },
        )
    }

    /// The block for an arc starting at `from`, by its direction and whether it changes
    /// height; a new rate (`start`) begins with the FIRST_ one, as for feed moves.
    fn arc_block(&self, arc: &arcs::Arc, from: DVec3, start: bool) -> &'static str {
        let helical = (arc.to.z - from.z).abs() > 1e-9;
        let [first, then] = match (arc.sweep < 0.0, helical) {
            (true, false) => ["FIRST_CW_ARC_MOVE", "CW_ARC_MOVE"],
            (false, false) => ["FIRST_CCW_ARC_MOVE", "CCW_ARC_MOVE"],
            (true, true) => ["FIRST_CW_HELICAL_ARC_MOVE", "CW_HELICAL_ARC_MOVE"],
            (false, true) => ["FIRST_CCW_HELICAL_ARC_MOVE", "CCW_HELICAL_ARC_MOVE"],
        };
        if start && self.blocks.contains_key(first) {
            first
        } else {
            then
        }
    }

    /// Which arcs may replace runs of cuts, if any: the post needs both arc blocks (and both
    /// helical ones for arcs that change height), and a flat job.
    fn arc_limits(&self, laser: bool) -> Option<arcs::Limits> {
        let has = |names: [&str; 2]| names.iter().all(|n| self.blocks.contains_key(*n));
        // a wrapped axis is round, so arcs in the plane are not arcs on the machine
        // ponytail: lasers keep their lines, whose powers may differ
        if !has(["CW_ARC_MOVE", "CCW_ARC_MOVE"]) || self.wrap.is_some() || laser {
            return None;
        }
        let unit = if self.inches { MM_PER_INCH } else { 1.0 };
        Some(arcs::Limits {
            // the tolerance curves were flattened to, Options ▸ Advanced's curve accuracy
            tolerance: cut_craft_core::accuracy::curve(),
            min_radius: self.arc_radius.0.map_or(0.0, |r| r * unit),
            max_radius: self.arc_radius.1.map_or(LARGEST_ARC, |r| r * unit),
            helical: has(["CW_HELICAL_ARC_MOVE", "CCW_HELICAL_ARC_MOVE"]),
        })
    }

    /// The moves of `toolpath`, starting from `at`, as they are written: a plunge from above
    /// the job's Plunge (Z2) gap goes down to it at the rapid rate first, and runs of cuts
    /// become arcs where the post can write them.
    fn steps(&self, toolpath: &Toolpath, job: &Job, mut at: DVec3) -> Vec<Step> {
        let gap = job.plunge_gap();
        // a bit held off the turning axis: its moves are where its tip cuts, written as where A
        // turns to and how high the bit is
        let held = match (&self.wrap, job.rotary) {
            (Some((axis, _)), Some(r)) if !toolpath.across.is_empty() => {
                Some((usize::from(*axis == "Y"), r.diameter / 2.0))
            }
            _ => None,
        };
        // (kind, to, laser power, across)
        let mut lines = Vec::with_capacity(toolpath.moves.len());
        let mut from = at;
        for (i, m) in toolpath.moves.iter().enumerate() {
            let across = toolpath.across.get(i).copied().unwrap_or(0.0);
            let mut to = m.to;
            if let Some((round, radius)) = held {
                let (w, z) = cut_craft_core::rotary_axis::held_at(radius, to[round], to.z, across);
                to[round] = w;
                to.z = z;
            }
            if m.kind != MoveKind::Rapid && from.z > gap + 1e-9 && to.z < gap {
                let down = from.lerp(to, (from.z - gap) / (from.z - to.z));
                lines.push((MoveKind::Rapid, down, 0.0, across));
            }
            let power = toolpath.power.get(i).copied().unwrap_or(0.0);
            lines.push((m.kind, to, power, across));
            from = to;
        }
        let line = |&(kind, to, power, across): &(MoveKind, DVec3, f64, f64)| {
            Step::Line(kind, to, power, across)
        };
        let Some(limits) = self.arc_limits(!toolpath.power.is_empty()) else {
            return lines.iter().map(line).collect();
        };
        // runs of cuts, and of plunges (a helix goes down in arcs at the plunge rate)
        let mut steps = Vec::with_capacity(lines.len());
        for run in lines.chunk_by(|a, b| a.0 == b.0) {
            let kind = run[0].0;
            if kind == MoveKind::Rapid {
                steps.extend(run.iter().map(line));
            } else {
                let cuts: Vec<DVec3> = run.iter().map(|l| l.1).collect();
                steps.extend(arcs::fit(at, &cuts, &limits).into_iter().map(|p| match p {
                    arcs::Piece::Line(to) => Step::Line(kind, to, 0.0, 0.0),
                    arcs::Piece::Arc(arc) => Step::Arc(kind, arc),
                }));
            }
            at = run[run.len() - 1].1;
        }
        steps
    }

    /// A drill bank's drill numbers as `[DL]` prints them.
    fn drill_list_text(&self, numbers: &[u32]) -> String {
        let (prefix, separator, first) = &self.drill_list;
        let mut text = String::new();
        for (i, n) in numbers.iter().enumerate() {
            match i {
                0 => {}
                1 => text.push_str(first),
                _ => text.push_str(separator),
            }
            text.push_str(prefix);
            text.push_str(&n.to_string());
        }
        text
    }

    /// How A winds through the file, for a wrapping post told the machine's rotary.
    fn winding(&self, job: &Job) -> Option<winding::Winding> {
        let (axis, _) = self.wrap.as_ref()?;
        let rotary = job.rotary?;
        let round = usize::from(*axis == "Y");
        Some(winding::Winding::new(
            self.rotary?,
            round,
            rotary.diameter,
            // clear of a square's corners or a board's edges as they swing round
            job.turning_gap(),
        ))
    }

    /// A flat X or Y as it prints: a wrapped axis in degrees round the cylinder.
    fn wrapped(&self, job: &Job, label: &str, flat: f64) -> f64 {
        match (&self.wrap, job.rotary) {
            (Some((axis, _)), Some(r)) if *axis == label => {
                flat / (std::f64::consts::PI * r.diameter) * 360.0
            }
            _ => flat,
        }
    }

    /// What stays the same through the file: the material, the file, the time and the
    /// starting position.
    fn job_values(&self, doc: &Document, file: &File) -> Values {
        let job = &doc.job;
        let safe = job.machine_z(job.clearance);
        let home = job.home();
        let (x, y, z) = (
            self.wrapped(job, "X", home.x),
            self.wrapped(job, "Y", home.y),
            job.machine_z(home.z),
        );
        let area = job.area();
        let mut v = Values::default();
        for (label, value) in [
            ("XH", x),
            ("YH", y),
            ("ZH", z),
            ("SAFEZ", safe),
            ("X", x),
            ("Y", y),
            ("Z", z),
            ("XLENGTH", job.width),
            ("YLENGTH", job.height),
            ("ZLENGTH", job.thickness),
            ("XMIN", area.min.x),
            ("XMAX", area.max.x),
            ("YMIN", area.min.y),
            ("YMAX", area.max.y),
            ("ZMIN", job.machine_z(-job.thickness)),
            ("ZMAX", job.machine_z(0.0)),
            ("WRAP_DIA", job.rotary.map_or(0.0, |r| r.diameter)),
        ] {
            v.numbers.insert(label, value);
        }
        let text = |s: Option<&std::ffi::OsStr>| s.map(|s| s.to_string_lossy().into_owned());
        let shown = |p: Option<&Path>| p.map(|p| p.display().to_string());
        for (label, value) in [
            ("TP_FILENAME", text(file.path.file_stem())),
            ("TP_DIR", shown(file.path.parent())),
            ("TP_EXT", text(file.path.extension())),
            ("PATHNAME", shown(Some(file.path))),
            ("FILE_NAME", text(file.job.and_then(Path::file_stem))),
            ("FILE_PATHNAME", shown(file.job)),
            ("DATE", Some(file.made.format("%A %B %d %Y").to_string())),
            ("TIME", Some(file.made.format("%I:%M %p").to_string())),
            ("Z_ORIGIN", Some(z_origin(job).into())),
            ("XY_ORIGIN", Some(xy_origin(job.datum).into())),
            (
                "UNITS",
                Some(if self.inches { "INCHES" } else { "MM" }.into()),
            ),
            (
                "PRODUCT",
                Some(concat!("Cut Craft ", env!("CARGO_PKG_VERSION")).into()),
            ),
            ("POST_NAME", Some(self.name.clone())),
            // one line, so a note can never break out of the comment it is printed in
            ("FILE_NOTES", Some(notes_lines(doc, file).join(" / "))),
            ("TOOLPATH_NOTES", None),
            ("TOOL_NOTES", None),
        ] {
            v.texts.insert(label, value.unwrap_or_default());
        }
        v
    }
}

fn z_origin(job: &Job) -> &'static str {
    match (job.z_zero, job.rotary) {
        (ZZero::MaterialSurface, None) => "Material Surface",
        (ZZero::MachineBed, None) => "Machine Bed",
        // an unrolled cylinder's top is its surface, and under it is its axis
        (ZZero::MaterialSurface, Some(_)) => "Cylinder Surface",
        (ZZero::MachineBed, Some(_)) => "Cylinder Axis",
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
/// the magazine (changed by hand); without one, its own tool number, else the order it is
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
