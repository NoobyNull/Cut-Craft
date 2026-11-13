//! Vectric-style `.pp` post-processors: a text format of global settings, `VAR` output formats
//! and `begin BLOCK` templates (see `re/SPEC.md`). `Post::parse` reads one; `Post::render`
//! writes a document's toolpaths through it.

mod arcs;
mod built_in;

pub use built_in::BUILT_IN;

use cut_craft_core::document::{Along, Datum, Document, Job, ZZero};
use cut_craft_core::drill_bank::{BankHit, DrillBank, DrillId};
use cut_craft_core::glam::DVec3;
use cut_craft_core::toolpath::{MoveKind, Toolpath};
use std::collections::HashMap;
use std::path::Path;

const MM_PER_INCH: f64 = 25.4;
/// Numeric values a post can print; the ones in `LENGTHS` become inches for inch posts
/// (feed rates are lengths per minute).
const NUMBERS: &[&str] = &[
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
        let wrapped = |label: &str, flat: f64| self.wrapped(&doc.job, label, flat);
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
        out.block("HEADER", &v);

        let mut previous_tool: Option<&str> = None;
        let mut first_rapid = true;
        // the rate of the feed moves since the last rapid: a new one starts a FIRST_ block,
        // which is where most posts print it
        let mut feeding: Option<f64> = None;
        let home = doc.job.home();
        let mut at = DVec3::new(home.x, home.y, home.z);
        for toolpath in &toolpaths {
            v.texts.insert("TOOLPATH_NAME", toolpath.name.clone());
            v.texts.insert("TOOLNAME", toolpath.tool.name.clone());
            v.tool(&toolpath.tool, number(&toolpath.tool.name));
            match previous_tool {
                Some(t) if t != toolpath.tool.name => {
                    out.block("TOOLCHANGE", &v);
                    out.block("SPINDLE_ON", &v);
                }
                Some(_) => out.block("NEW_SEGMENT", &v),
                None => {}
            }
            previous_tool = Some(&toolpath.tool.name);
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
                for step in self.steps(&part, &doc.job, at) {
                    let (kind, to) = match step {
                        Step::Line(kind, to, _) => (kind, to),
                        Step::Arc(kind, arc) => (kind, arc.to),
                    };
                    let from = std::mem::replace(&mut at, to);
                    let length = to.distance(from);
                    if self.inverse_time && kind != MoveKind::Rapid && length == 0.0 {
                        // a move that goes nowhere has no time to invert
                        continue;
                    }
                    v.numbers.insert("X", wrapped("X", to.x));
                    v.numbers.insert("Y", wrapped("Y", to.y));
                    v.numbers.insert("Z", doc.job.machine_z(to.z));
                    if let (true, Step::Line(_, _, power)) = (laser, step) {
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
                        v.numbers.insert("FI", rate / length);
                        let start = feeding.replace(rate) != Some(rate);
                        if let Step::Arc(_, arc) = step {
                            v.arc(from, &arc);
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
        // (kind, to, laser power)
        let mut lines = Vec::with_capacity(toolpath.moves.len());
        let mut from = at;
        for (i, m) in toolpath.moves.iter().enumerate() {
            if m.kind != MoveKind::Rapid && from.z > gap + 1e-9 && m.to.z < gap {
                let down = from.lerp(m.to, (from.z - gap) / (from.z - m.to.z));
                lines.push((MoveKind::Rapid, down, 0.0));
            }
            let power = toolpath.power.get(i).copied().unwrap_or(0.0);
            lines.push((m.kind, m.to, power));
            from = m.to;
        }
        let line = |&(kind, to, power): &(MoveKind, DVec3, f64)| Step::Line(kind, to, power);
        let Some(limits) = self.arc_limits(!toolpath.power.is_empty()) else {
            return lines.iter().map(line).collect();
        };
        // runs of cuts, and of plunges (a helix goes down in arcs at the plunge rate)
        let mut steps = Vec::with_capacity(lines.len());
        for run in lines.chunk_by(|a, b| a.0 == b.0) {
            let kind = run[0].0;
            if kind != MoveKind::Rapid {
                let cuts: Vec<DVec3> = run.iter().map(|l| l.1).collect();
                steps.extend(arcs::fit(at, &cuts, &limits).into_iter().map(|p| match p {
                    arcs::Piece::Line(to) => Step::Line(kind, to, 0.0),
                    arcs::Piece::Arc(arc) => Step::Arc(kind, arc),
                }));
            } else {
                steps.extend(run.iter().map(line));
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
            ("FILE_NOTES", Some(doc.notes.clone())),
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
        (ZZero::MaterialSurface, _) => "Material Surface",
        (ZZero::MachineBed, None) => "Machine Bed",
        // under an unrolled cylinder is its axis
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
    Line(MoveKind, DVec3, f64),
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
mod tests {
    use super::*;
    use cut_craft_core::Change;
    use cut_craft_core::glam::DVec3;
    use cut_craft_core::toolpath::{Move, MoveKind, Tool, Toolpath};

    /// A small post written for these tests, in the documented format.
    const BASIC: &str = r#"
+ test post
POST_NAME = "Test G-code (mm) (*.nc)"
FILE_EXTENSION = "nc"
UNITS = "MM"
LINE_ENDING = "[13][10]"

VAR SPINDLE_SPEED = [S|A|S|1.0]
VAR FEED_RATE = [F|C|F|1.1]
VAR X_POSITION = [X|A|X|1.3]
VAR Y_POSITION = [Y|A|Y|1.3]
VAR Z_POSITION = [Z|A|Z|1.3]
VAR SAFE_Z_HEIGHT = [SAFEZ|A||1.3]

begin HEADER
"([TOOLPATH_NAME] with [TOOLNAME])"
"G21 G90"
"G0 Z[SAFEZ]"
"[S] M3"

begin RAPID_MOVE
"G0 [X] [Y] [Z]"

begin FIRST_FEED_MOVE
"G1 [X] [Y] [Z] [F]"

begin FEED_MOVE
"G1 [X] [Y] [Z] [F]"

begin TOOLCHANGE
"M5"
"(change to [TOOLNAME])"
"M0"
"[S] M3"

begin FOOTER
"G0 Z[SAFEZ]"
"M5"
"M30"
"#;

    fn mv(kind: MoveKind, x: f64, y: f64, z: f64) -> Move {
        Move {
            kind,
            to: DVec3::new(x, y, z),
        }
    }

    #[test]
    fn the_tools_used_are_listed_once_each_with_their_number() {
        let (first, v) = (
            toolpath("Profile", "6 mm End Mill"),
            toolpath("Carve", "V-Bit"),
        );
        let toolpaths = [&first, &v, &first];
        let names = |numbers: [u32; 2]| {
            vec![
                ("6 mm End Mill".to_owned(), numbers[0]),
                ("V-Bit".to_owned(), numbers[1]),
            ]
        };
        assert_eq!(tools_and_numbers(&toolpaths, &[]), names([1, 2]));
        let slots = [None, None, Some("V-Bit".to_owned())];
        assert_eq!(tools_and_numbers(&toolpaths, &slots), names([0, 3]));
    }

    fn toolpath(name: &str, tool: &str) -> Toolpath {
        Toolpath {
            name: name.into(),
            strategy: "profile_toolpath".into(),
            params: serde_json::json!({}),
            tool: Tool {
                name: tool.into(),
                ..Tool::default()
            },
            power: vec![],
            stale: false,
            group: None,
            vectors: vec![],
            moves: vec![
                mv(MoveKind::Rapid, 0.0, 0.0, 5.0),
                mv(MoveKind::Plunge, 0.0, 0.0, -3.0),
                mv(MoveKind::Cut, 25.4, 0.0, -3.0),
                mv(MoveKind::Cut, 25.4, 25.4, -3.0),
                mv(MoveKind::Rapid, 25.4, 25.4, 5.0),
            ],
            visible: true,
            drill_bank: None,
            fallbacks: vec![],
        }
    }

    fn render(post: &str, toolpaths: Vec<Toolpath>) -> Vec<String> {
        render_for(post, cut_craft_core::document::Job::default(), toolpaths)
    }

    fn render_for(
        post: &str,
        job: cut_craft_core::document::Job,
        toolpaths: Vec<Toolpath>,
    ) -> Vec<String> {
        let post = Post::parse(post).unwrap();
        assert_eq!(post.line_ending, "\r\n", "LINE_ENDING is honoured");
        render_with(&post, job, toolpaths)
    }

    fn render_with(
        post: &Post,
        job: cut_craft_core::document::Job,
        toolpaths: Vec<Toolpath>,
    ) -> Vec<String> {
        let mut doc = Document {
            job,
            ..Document::default()
        };
        let mut c = Change::builder(&doc);
        for t in toolpaths {
            c.add_toolpath(t);
        }
        doc.apply(&c.finish()).unwrap();
        let made = chrono::NaiveDate::from_ymd_opt(2026, 10, 24)
            .and_then(|d| d.and_hms_opt(9, 37, 0))
            .unwrap();
        let file = File {
            path: std::path::Path::new("/jobs/sign/front.nc"),
            job: Some(std::path::Path::new("/jobs/sign.ccjob")),
            made,
            slots: &[],
        };
        let text = post.render(&doc, &file).unwrap();
        text.lines()
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect()
    }

    #[test]
    fn laser_toolpaths_turn_the_jet_on_after_moving_and_off_before_rapids() {
        let post = BASIC.replace(
            "begin HEADER",
            "LASER_SUPPORT = \"YES\"\nVAR POWER = [P|C|S|1.0|10.0]\n\
             begin JET_TOOL_ON\n\"M4 [P]\"\nbegin JET_TOOL_POWER\n\"[P]\"\n\
             begin JET_TOOL_OFF\n\"M5\"\nbegin HEADER",
        );
        let mut laser = toolpath("Burn", "Laser");
        laser.power = vec![0.0, 50.0, 50.0, 80.0, 0.0];
        let lines = render(&post, vec![laser]);
        let at = |text: &str| {
            lines
                .iter()
                .position(|l| l == text)
                .unwrap_or_else(|| panic!("{text} in {lines:?}"))
        };
        let on = at("M4 S500");
        assert!(on > at("G0 X0.000 Y0.000 Z5.000"), "on once it is in place");
        assert!(at("S800") > on, "a new power prints on its own");
        assert!(lines[at("S800") + 1].starts_with("G1 X25.400 Y25.400"));
        let off = lines.iter().rposition(|l| l == "M5").unwrap();
        assert!(
            lines.iter().position(|l| l == "M5").unwrap() < at("G0 X25.400 Y25.400 Z5.000"),
            "off before the rapid"
        );
        assert!(off > on);
    }

    #[test]
    fn a_post_without_laser_support_refuses_laser_toolpaths() {
        let mut laser = toolpath("Burn", "Laser");
        laser.power = vec![0.0, 50.0, 50.0, 50.0, 0.0];
        let mut doc = Document::default();
        let mut c = Change::builder(&doc);
        c.add_toolpath(laser);
        let change = c.finish();
        doc.apply(&change).unwrap();
        let file = File {
            path: Path::new("x.nc"),
            job: None,
            made: chrono::NaiveDateTime::default(),
            slots: &[],
        };
        let why = Post::parse(BASIC).unwrap().render(&doc, &file).unwrap_err();
        assert!(why.contains("LASER_SUPPORT"), "{why}");
    }

    #[test]
    fn z_zero_on_the_machine_bed_raises_every_z_by_the_thickness() {
        use cut_craft_core::document::{Job, ZZero};
        let job = Job {
            z_zero: ZZero::MachineBed,
            thickness: 20.0,
            ..Job::default()
        };
        let surface = render(BASIC, vec![toolpath("P", "T")]);
        let bed = render_for(BASIC, job, vec![toolpath("P", "T")]);
        assert!(surface.contains(&"G0 Z5.000".to_owned()), "{surface:?}");
        assert!(bed.contains(&"G0 Z25.000".to_owned()), "{bed:?}");
    }

    #[test]
    fn reads_the_global_settings() {
        let post = Post::parse(BASIC).unwrap();
        assert_eq!(
            (post.name.as_str(), post.extension.as_str(), post.inches),
            ("Test G-code (mm) (*.nc)", "nc", false)
        );
    }

    #[test]
    fn renders_header_moves_and_footer() {
        let lines = render(BASIC, vec![toolpath("Profile 1", "6 mm End Mill")]);
        assert_eq!(
            lines,
            [
                "(Profile 1 with 6 mm End Mill)",
                "G21 G90",
                "G0 Z5.000",
                "S18000 M3",
                "G0 X0.000 Y0.000 Z5.000",
                "G1 X0.000 Y0.000 Z-3.000 F300.0",
                "G1 X25.400 Y0.000 Z-3.000 F1000.0",
                "G1 X25.400 Y25.400 Z-3.000",
                "G0 X25.400 Y25.400 Z5.000",
                "G0 Z5.000",
                "M5",
                "M30",
            ]
        );
    }

    #[test]
    fn a_new_tool_uses_the_toolchange_block() {
        let lines = render(BASIC, vec![toolpath("A", "6 mm"), toolpath("B", "3 mm")]);
        assert!(lines.contains(&"(change to 3 mm)".to_owned()), "{lines:#?}");
        assert_eq!(lines.iter().filter(|l| *l == "M0").count(), 1);
    }

    #[test]
    fn pausing_for_tools_swaps_a_tool_changer_for_a_stop_to_change_by_hand() {
        let job = cut_craft_core::document::Job::default();
        let two = || vec![toolpath("A", "6 mm"), toolpath("B", "3 mm")];
        let atc = Post::parse(include_str!("../posts/fluidnc_atc_mm.pp")).unwrap();
        assert!(!atc.pauses_for_tools(), "an ATC post changes tools itself");
        let paused = atc.with_pauses();
        assert!(paused.pauses_for_tools());
        let lines = render_with(&paused, job, two());
        assert!(!lines.iter().any(|l| l.contains("M6")), "{lines:#?}");
        assert!(
            lines.iter().any(|l| l.starts_with("(First tool T1: 6 mm")),
            "{lines:#?}"
        );
        let at = lines.iter().position(|l| l == "M0").expect("a stop");
        assert!(lines[at - 1].contains("3 mm"), "names the tool: {lines:#?}");
        // a post that already pauses keeps its own tool change
        let grbl = Post::parse(include_str!("../posts/grbl_straight_mm.pp")).unwrap();
        assert!(grbl.pauses_for_tools());
        assert_eq!(grbl.with_pauses().blocks, grbl.blocks);
        // one without any tool change gets one
        let plain = Post::parse(
            "POST_NAME = \"Plain (*.nc)\"\nFILE_EXTENSION = \"nc\"\nUNITS = \"MM\"\n\
             begin HEADER\n\"G21\"\nbegin RAPID_MOVE\n\"G0 [X] [Y] [Z]\"\n\
             begin FEED_MOVE\n\"G1 [X] [Y] [Z]\"\n",
        )
        .unwrap();
        assert!(!plain.changes_tools());
        let paused = plain.with_pauses();
        assert!(paused.changes_tools() && paused.pauses_for_tools());
        let lines = render_with(&paused, job, two());
        assert_eq!(lines.iter().filter(|l| *l == "M0").count(), 1, "{lines:#?}");
    }

    #[test]
    fn inch_posts_convert_positions_and_feeds() {
        let post = BASIC
            .replace(r#"UNITS = "MM""#, r#"UNITS = "INCHES""#)
            .replace("[X|A|X|1.3]", "[X|A|X|1.4]");
        let lines = render(&post, vec![toolpath("P", "T")]);
        assert!(
            lines.iter().any(|l| l.starts_with("G1 X1.0000 Y0.000")),
            "{lines:#?}"
        );
        assert!(
            lines.iter().any(|l| l.ends_with("F39.4")),
            "1000 mm/min is 39.4 in/min: {lines:#?}"
        );
    }

    #[test]
    fn format_flags_multipliers_and_line_numbers() {
        let post = format!(
            "{}\nLINE_NUMBER = 10\nLINE_NUMBER_INCREMENT = 10\nVAR LINE_NUMBER = [N|A|N|1.0]\nVAR Z_POSITION = [Z|A|Z|+08.2]\nVAR FEED_RATE = [F|A|F|1.0|0.5]\n",
            BASIC.replace("\"G0 [X] [Y] [Z]\"", "\"[N] G0 [X] [Y] [Z]\"")
        );
        let lines = render(&post, vec![toolpath("P", "T")]);
        assert!(
            lines.contains(&"N10 G0 X0.000 Y0.000 Z+0005.00".to_owned()),
            "{lines:#?}"
        );
        assert!(
            lines.contains(&"N20 G0 X25.400 Y25.400 Z+0005.00".to_owned()),
            "{lines:#?}"
        );
        assert!(
            lines.iter().any(|l| l.ends_with("F150")),
            "plunge rate 300 x 0.5, as an integer: {lines:#?}"
        );
    }

    #[test]
    fn substitute_replaces_characters_in_names() {
        let post = format!("{BASIC}\nSUBSTITUTE = \"({{)}}\"\n");
        let lines = render(&post, vec![toolpath("Profile (outside)", "6 mm")]);
        assert_eq!(lines[0], "(Profile {outside} with 6 mm)");
    }

    #[test]
    fn unsupported_variables_are_named_not_ignored() {
        let post = BASIC.replace(
            "\"G1 [X] [Y] [Z] [F]\"",
            "\"G1 [X] [Y] [Z] [REVISION] [F]\"",
        );
        assert_eq!(
            Post::parse(&post).unwrap_err(),
            "the post uses [REVISION], which is not supported yet"
        );
    }

    /// A rotary job 100 mm round (so 1 mm is 3.6 degrees), zeroed on its axis.
    fn hundred_round() -> cut_craft_core::document::Job {
        use cut_craft_core::document::{Along, Job, ZZero};
        Job {
            z_zero: ZZero::MachineBed,
            ..Job::rotary(300.0, 100.0 / std::f64::consts::PI, Along::X)
        }
    }

    #[test]
    fn wrapping_y_prints_it_as_degrees_round_the_cylinder() {
        let post = BASIC.replace(
            "begin HEADER\n",
            "ROTARY_WRAP_Y = \"A\"\nbegin HEADER\n\"([WRAP_DIA] [Z_ORIGIN])\"\n",
        );
        let lines = render_for(&post, hundred_round(), vec![toolpath("P", "T")]);
        assert_eq!(lines[0], "(31.831 Cylinder Axis)");
        assert!(
            lines.contains(&"G1 X25.400 A91.440 Z12.915".to_owned()),
            "25.4 mm round is 91.44 degrees, 3 mm into a 15.915 radius: {lines:#?}"
        );
    }

    #[test]
    fn inverse_time_feeds_are_the_rate_over_the_move_length() {
        let post = BASIC
            .replace(
                "begin HEADER\n",
                "ROTARY_WRAP_Y = \"A\"\nINVERSE_TIME_MODE = YES\nVAR INVERSE_TIME = [FI|A|F|1.3]\nbegin HEADER\n",
            )
            .replace("[Z] [F]\"", "[Z] [FI]\"");
        let lines = render_for(&post, hundred_round(), vec![toolpath("P", "T")]);
        assert!(
            lines.contains(&"G1 X0.000 A0.000 Z12.915 F37.500".to_owned()),
            "an 8 mm plunge at 300 mm/min takes 1/37.5 minutes: {lines:#?}"
        );
        assert!(
            lines.contains(&"G1 X25.400 A0.000 Z12.915 F39.370".to_owned()),
            "{lines:#?}"
        );
    }

    #[test]
    fn a_wrapping_post_needs_a_rotary_job() {
        let post = BASIC.replace("begin HEADER\n", "ROTARY_WRAP_Y = \"A\"\nbegin HEADER\n");
        let mut doc = Document::default();
        let mut c = Change::builder(&doc);
        c.add_toolpath(toolpath("P", "T"));
        let change = c.finish();
        doc.apply(&change).unwrap();
        let file = File {
            path: std::path::Path::new("/out.nc"),
            job: None,
            made: chrono::NaiveDateTime::default(),
            slots: &[],
        };
        let why = Post::parse(&post).unwrap().render(&doc, &file).unwrap_err();
        assert!(why.contains("rotary"), "{why}");
    }

    #[test]
    fn inverse_time_needs_wrapping() {
        let post = BASIC.replace("begin HEADER\n", "INVERSE_TIME_MODE = YES\nbegin HEADER\n");
        let why = Post::parse(&post).unwrap_err();
        assert!(why.contains("INVERSE_TIME_MODE"), "{why}");
    }

    /// Renders through the default post for `job`.
    fn render_default(job: cut_craft_core::document::Job, toolpaths: Vec<Toolpath>) -> String {
        let mut doc = Document {
            job,
            ..Document::default()
        };
        let mut c = Change::builder(&doc);
        for t in toolpaths {
            c.add_toolpath(t);
        }
        let change = c.finish();
        doc.apply(&change).unwrap();
        let file = File {
            path: std::path::Path::new("/out.nc"),
            job: None,
            made: chrono::NaiveDateTime::default(),
            slots: &[],
        };
        Post::default_for(&doc.job).render(&doc, &file).unwrap()
    }

    #[test]
    fn the_default_post_is_fluidnc_with_a_tool_changer() {
        let g = render_default(
            cut_craft_core::document::Job::default(),
            vec![toolpath("A", "6 mm"), toolpath("B", "3 mm")],
        );
        assert!(g.contains("M6 T1") && g.contains("M6 T2"), "{g}");
        assert!(g.contains("F1000.0") && !g.contains("G93"), "{g}");
    }

    #[test]
    fn a_post_changes_tools_only_with_a_toolchange_block() {
        assert!(Post::parse(BASIC).unwrap().changes_tools());
        let without = BASIC.replace("begin TOOLCHANGE", "begin UNUSED_BLOCK");
        assert!(!Post::parse(&without).unwrap().changes_tools());
        let job = cut_craft_core::document::Job::default();
        assert!(Post::default_for(&job).changes_tools());
    }

    #[test]
    fn a_dwelling_drill_waits_after_each_plunge_where_the_post_can() {
        let mut drill = toolpath("Holes", "6 mm");
        drill.params = serde_json::json!({ "dwell": 0.5 });
        let mut doc = Document::default();
        let mut c = Change::builder(&doc);
        c.add_toolpath(drill);
        doc.apply(&c.finish()).unwrap();
        let file = File {
            path: std::path::Path::new("/out.nc"),
            job: None,
            made: chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            slots: &[],
        };
        let plain = Post::parse(BASIC).unwrap();
        let why = plain.render(&doc, &file).unwrap_err();
        assert!(why.contains("DWELL_MOVE"), "{why}");
        let dwelling = Post::parse(&format!(
            "{BASIC}\nVAR DWELL_TIME = [DWELL|A|P|1.1]\nbegin DWELL_MOVE\n\"G4 [DWELL]\"\n"
        ))
        .unwrap();
        let g = dwelling.render(&doc, &file).unwrap();
        assert!(g.contains("G4 P0.5"), "{g}");
        // Cut Craft's own posts dwell
        let own = Post::default_for(&doc.job).render(&doc, &file).unwrap();
        assert!(own.contains("G4 P0.5"), "{own}");
    }

    #[test]
    fn rotary_jobs_get_the_default_post_wrapped_onto_a() {
        use cut_craft_core::document::Along;
        let round = |along| {
            let job = cut_craft_core::document::Job::rotary(300.0, 50.0, along);
            render_default(job, vec![toolpath("P", "T")])
        };
        // 25.4 mm round a 50 mm cylinder is 58.213 degrees
        let along_x = round(Along::X);
        assert!(
            along_x.contains("G93") && along_x.contains("G94"),
            "{along_x}"
        );
        assert!(
            along_x.contains("X25.400") && along_x.contains("A58.213"),
            "Y goes round a cylinder along X: {along_x}"
        );
        let along_y = round(Along::Y);
        assert!(
            along_y.contains("Y25.400")
                && along_y.contains("A58.213")
                && !along_y.contains("X25.400"),
            "X goes round a cylinder along Y: {along_y}"
        );
    }

    #[test]
    fn a_new_feed_rate_starts_with_the_first_feed_move_block() {
        // like most real posts, only FIRST_FEED_MOVE prints the rate
        let post = BASIC.replace(
            "begin FEED_MOVE\n\"G1 [X] [Y] [Z] [F]\"",
            "begin FEED_MOVE\n\"G1 [X] [Y] [Z]\"",
        );
        let lines = render(&post, vec![toolpath("P", "T")]);
        assert!(
            lines.contains(&"G1 X25.400 Y0.000 Z-3.000 F1000.0".to_owned()),
            "after plunging at 300 the cut prints its own 1000: {lines:#?}"
        );
    }

    #[test]
    fn posts_may_define_blocks_cut_craft_never_writes() {
        let post = format!(
            "{BASIC}\nbegin CW_HELICAL_ARC_PLUNGE_MOVE\n\"G2 [X] [Y] [Z] [I] [J] [FP]\"\nbegin REVISION_COMMENT\n\"(revision [REVISION])\"\n"
        );
        let lines = render(&post, vec![toolpath("P", "T")]);
        assert!(
            lines
                .iter()
                .all(|l| !l.starts_with("G2 ") && !l.starts_with("(revision")),
            "{lines:#?}"
        );
    }

    /// `BASIC` with arc blocks, centres relative to the start.
    fn arc_post(helical: bool) -> String {
        let mut blocks = String::from(
            "VAR ARC_CENTRE_I_INC_POSITION = [I|A|I|1.3]\nVAR ARC_CENTRE_J_INC_POSITION = [J|A|J|1.3]\n\
             begin FIRST_CW_ARC_MOVE\n\"G2 [X] [Y] [Z] [I] [J] [F]\"\nbegin CW_ARC_MOVE\n\"G2 [X] [Y] [Z] [I] [J]\"\n\
             begin FIRST_CCW_ARC_MOVE\n\"G3 [X] [Y] [Z] [I] [J] [F]\"\nbegin CCW_ARC_MOVE\n\"G3 [X] [Y] [Z] [I] [J]\"\n",
        );
        if helical {
            blocks.push_str(
                "begin CW_HELICAL_ARC_MOVE\n\"G2 [X] [Y] [Z] [I] [J] (helix)\"\n\
                 begin CCW_HELICAL_ARC_MOVE\n\"G3 [X] [Y] [Z] [I] [J] (helix)\"\n",
            );
        }
        format!("{BASIC}\n{blocks}")
    }

    /// A toolpath cutting through `points` from the first, 3 mm deep.
    fn cutting(points: &[DVec3]) -> Toolpath {
        let start = points[0];
        let mut t = toolpath("Circle", "6 mm");
        t.moves = [
            mv(MoveKind::Rapid, start.x, start.y, 5.0),
            mv(MoveKind::Plunge, start.x, start.y, start.z),
        ]
        .into_iter()
        .chain(points[1..].iter().map(|p| Move {
            kind: MoveKind::Cut,
            to: *p,
        }))
        .chain([mv(MoveKind::Rapid, 0.0, 0.0, 5.0)])
        .collect();
        t
    }

    /// The outside of a 40 mm circle for a 6 mm tool, as a profile would cut it: the offset
    /// outline, round to its start.
    fn circle_profile() -> Vec<DVec3> {
        use cut_craft_core::geometry::{Path, Point};
        use cut_craft_core::polygon::{Corners, offset};
        let outline = offset(
            &[Path::circle(Point::new(100.0, 100.0), 20.0)],
            3.0,
            Corners::Round,
        );
        let mut points: Vec<DVec3> = outline[0]
            .flatten(cut_craft_core::polygon::TOLERANCE)
            .iter()
            .map(|p| p.extend(-3.0))
            .collect();
        if points.first() != points.last() {
            points.push(points[0]);
        }
        points
    }

    /// The XY path a G-code text cuts at Z below 0, arcs expanded into fine steps.
    fn expand(lines: &[String]) -> Vec<cut_craft_core::geometry::Point> {
        use cut_craft_core::geometry::Point;
        let word = |line: &str, letter: char| {
            line.split_whitespace()
                .find_map(|w| w.strip_prefix(letter)?.parse::<f64>().ok())
        };
        let mut at = DVec3::ZERO;
        let mut path = vec![];
        for line in lines {
            let Some(code) = line.split_whitespace().next() else {
                continue;
            };
            if !matches!(code, "G0" | "G1" | "G2" | "G3") {
                continue;
            }
            let to = DVec3::new(
                word(line, 'X').unwrap_or(at.x),
                word(line, 'Y').unwrap_or(at.y),
                word(line, 'Z').unwrap_or(at.z),
            );
            if matches!(code, "G2" | "G3") {
                let centre =
                    at.truncate() + Point::new(word(line, 'I').unwrap(), word(line, 'J').unwrap());
                let (from, end) = (at.truncate() - centre, to.truncate() - centre);
                let mut sweep = from.angle_to(end);
                if code == "G2" && sweep > 0.0 {
                    sweep -= std::f64::consts::TAU;
                } else if code == "G3" && sweep < 0.0 {
                    sweep += std::f64::consts::TAU;
                }
                for i in 0..=100 {
                    let turn = Point::from_angle(sweep * f64::from(i) / 100.0);
                    path.push(centre + turn.rotate(from));
                }
            } else if to.z < 0.0 {
                path.push(to.truncate());
            }
            at = to;
        }
        path
    }

    #[test]
    fn a_circle_posts_as_a_few_arcs_that_retrace_its_lines() {
        use cut_craft_core::geometry::{Point, polyline_distance};
        let circle = circle_profile();
        assert!(circle.len() > 100, "{} lines", circle.len());
        let lines = render(&arc_post(false), vec![cutting(&circle)]);
        let arcs = lines
            .iter()
            .filter(|l| l.starts_with("G2 ") || l.starts_with("G3 "));
        assert!((2..=4).contains(&arcs.count()), "{lines:#?}");
        assert!(
            !lines
                .iter()
                .any(|l| l.starts_with("G1 ") && l.contains("Z-3.000") && !l.contains("F300")),
            "only the plunge is straight: {lines:#?}"
        );
        // within the tolerance both ways, plus the 3 places the post prints to
        let near = cut_craft_core::polygon::TOLERANCE + 0.002;
        let flat: Vec<Point> = circle.iter().map(|p| p.truncate()).collect();
        let traced = expand(&lines);
        for p in &flat {
            assert!(polyline_distance(&traced, *p) < near, "{p} is off the arcs");
        }
        for p in &traced {
            assert!(polyline_distance(&flat, *p) < near, "{p} is off the lines");
        }
    }

    #[test]
    fn lines_that_are_not_arcs_stay_straight() {
        let lines = render(&arc_post(true), vec![toolpath("P", "T")]);
        assert!(
            lines.contains(&"G1 X25.400 Y0.000 Z-3.000 F1000.0".to_owned())
                && lines.contains(&"G1 X25.400 Y25.400 Z-3.000".to_owned()),
            "{lines:#?}"
        );
        assert!(
            !lines
                .iter()
                .any(|l| l.starts_with("G2 ") || l.starts_with("G3 ")),
            "{lines:#?}"
        );
    }

    #[test]
    fn a_post_without_arc_blocks_writes_every_line() {
        let circle = circle_profile();
        let lines = render(BASIC, vec![cutting(&circle)]);
        let cuts = lines.iter().filter(|l| l.starts_with("G1 ")).count();
        assert_eq!(cuts, circle.len(), "the plunge and each line: {lines:#?}");
    }

    #[test]
    fn a_ramp_round_a_circle_is_a_helix_when_the_post_has_helical_arcs() {
        // a quarter turn of radius 10 going down 2 mm, in 30 steps
        let helix: Vec<DVec3> = (0..=30)
            .map(|i| {
                let turn = f64::from(i) / 30.0 * std::f64::consts::FRAC_PI_2;
                DVec3::new(
                    10.0 * turn.cos(),
                    10.0 * turn.sin(),
                    -1.0 - 2.0 * f64::from(i) / 30.0,
                )
            })
            .collect();
        let lines = render(&arc_post(true), vec![cutting(&helix)]);
        assert!(
            lines.contains(&"G3 X0.000 Y10.000 Z-3.000 I-10.000 J0.000 (helix)".to_owned()),
            "{lines:#?}"
        );
        let flat = render(&arc_post(false), vec![cutting(&helix)]);
        assert!(!flat.iter().any(|l| l.starts_with("G3 ")), "{flat:#?}");
    }

    #[test]
    fn a_helix_entry_goes_down_in_helical_arcs_at_the_plunge_rate() {
        // an entry helix: a quarter turn of radius 10 down 2 mm, gone down at the plunge rate
        let mut t = cutting(&[DVec3::new(10.0, 0.0, 0.0), DVec3::new(10.0, 0.0, -3.0)]);
        let helix = (1..=30).map(|i| {
            let turn = f64::from(i) / 30.0 * std::f64::consts::FRAC_PI_2;
            mv(
                MoveKind::Plunge,
                10.0 * turn.cos(),
                10.0 * turn.sin(),
                -1.0 - 2.0 * f64::from(i) / 30.0,
            )
        });
        t.moves = [
            mv(MoveKind::Rapid, 10.0, 0.0, 5.0),
            mv(MoveKind::Plunge, 10.0, 0.0, -1.0),
        ]
        .into_iter()
        .chain(helix)
        .chain([mv(MoveKind::Rapid, 0.0, 10.0, 5.0)])
        .collect();
        let plunge = t.tool.plunge_rate;
        let lines = render(&arc_post(true), vec![t]);
        let arc = lines
            .iter()
            .find(|l| l.starts_with("G3 X0.000 Y10.000 Z-3.000"))
            .unwrap_or_else(|| panic!("{lines:#?}"));
        assert!(arc.ends_with("(helix)"), "{arc}");
        // the feed is set to the plunge rate before it, not the cutting feed
        let at = lines.iter().position(|l| l == arc).unwrap();
        let fed = lines[..at]
            .iter()
            .rev()
            .find_map(|l| l.split_whitespace().find(|w| w.starts_with('F')))
            .unwrap();
        assert_eq!(fed, format!("F{plunge:.1}"), "{lines:#?}");
    }

    #[test]
    fn arcs_turning_clockwise_are_g2_with_the_centre_from_their_start() {
        // a half turn of radius 10 round 50, 50 from its right, clockwise
        let half: Vec<DVec3> = (0..=40)
            .map(|i| {
                let turn = -f64::from(i) / 40.0 * std::f64::consts::PI;
                DVec3::new(50.0 + 10.0 * turn.cos(), 50.0 + 10.0 * turn.sin(), -3.0)
            })
            .collect();
        let lines = render(&arc_post(false), vec![cutting(&half)]);
        assert!(
            lines.contains(&"G2 X40.000 Y50.000 Z-3.000 I-10.000 J0.000 F1000.0".to_owned()),
            "{lines:#?}"
        );
    }

    #[test]
    fn plunges_rapid_down_to_the_plunge_gap_first() {
        use cut_craft_core::document::Job;
        let job = Job {
            plunge: Some(1.0),
            ..Job::default()
        };
        let lines = render_for(BASIC, job, vec![toolpath("P", "T")]);
        let at = lines
            .iter()
            .position(|l| l == "G0 X0.000 Y0.000 Z5.000")
            .unwrap();
        assert_eq!(
            lines[at + 1..at + 3],
            ["G0 X0.000 Y0.000 Z1.000", "G1 X0.000 Y0.000 Z-3.000 F300.0"]
        );
        // with the gap at the clearance (older jobs) nothing changes
        let lines = render(BASIC, vec![toolpath("P", "T")]);
        assert_eq!(lines[5], "G1 X0.000 Y0.000 Z-3.000 F300.0");
    }

    #[test]
    fn the_tool_starts_and_ends_at_home() {
        use cut_craft_core::document::{Home, Job, ZZero};
        let job = Job {
            home: Some(Home {
                x: 10.0,
                y: 20.0,
                z: 30.0,
            }),
            z_zero: ZZero::MachineBed,
            ..Job::default()
        };
        let g = render_default(job, vec![toolpath("P", "T")]);
        let lines: Vec<&str> = g.lines().collect();
        let header = lines.iter().position(|l| *l == "G0 Z50.000").unwrap();
        assert_eq!(lines[header + 1], "G0 X10.000 Y20.000", "{g}");
        assert_eq!(
            lines[lines.len() - 4..],
            ["G0 Z50.000", "M5", "G0 X10.000 Y20.000", "M30"],
            "30 above the 20 mm material, zeroed on the bed: {g}"
        );
    }

    #[test]
    fn the_default_post_writes_arcs() {
        let g = render_default(
            cut_craft_core::document::Job::default(),
            vec![cutting(&circle_profile())],
        );
        assert!(g.contains("\nG2 ") || g.contains("\nG3 "), "{g}");
        assert!(g.lines().count() < 40, "{g}");
    }

    #[test]
    fn variables_without_a_var_line_print_in_default_formats() {
        let post = "POST_NAME = \"Bare\"\nFILE_EXTENSION = \"nc\"\nUNITS = \"MM\"\nLINE_ENDING = \"[13][10]\"\nbegin HEADER\n\"[S] M3 [XLENGTH]\"\nbegin RAPID_MOVE\n\"G0 [X] [Y] [Z]\"\nbegin FEED_MOVE\n\"G1 [X] [Y] [Z] [F]\"\n";
        let lines = render(post, vec![toolpath("P", "T")]);
        assert_eq!(
            lines[0], "18000 M3 600.000",
            "rates whole, lengths to 3 places"
        );
        assert!(
            lines.contains(&"G1 25.400 0.000 -3.000 1000".to_owned()),
            "{lines:#?}"
        );
    }

    #[test]
    fn header_variables_describe_the_job_the_file_and_the_time() {
        let post = BASIC.replace(
            "begin HEADER\n",
            "begin HEADER\n\"([TP_FILENAME]|[TP_DIR]|[TP_EXT]|[PATHNAME]|[FILE_NAME]|[FILE_PATHNAME])\"\n\"([DATE] - [TIME])\"\n\"([XLENGTH] [YLENGTH] [ZLENGTH] [XMIN] [XMAX] [YMIN] [YMAX] [ZMIN] [ZMAX])\"\n\"([Z_ORIGIN] / [XY_ORIGIN] / [UNITS] / [TDIA])\"\n\"([PRODUCT] / [POST_NAME] / [FILE_NOTES][TOOLPATH_NOTES][TOOL_NOTES])\"\n",
        );
        let lines = render(&post, vec![toolpath("P", "T")]);
        assert_eq!(
            lines[..5],
            [
                "(front|/jobs/sign|nc|/jobs/sign/front.nc|sign|/jobs/sign.ccjob)",
                "(Saturday October 24 2026 - 09:37 AM)",
                "(600.000 400.000 20.000 0.000 600.000 0.000 400.000 -20.000 0.000)",
                "(Material Surface / Bottom Left Corner / MM / 6.000)",
                &format!(
                    "(Cut Craft {} / Test G-code (mm) (*.nc) / )",
                    env!("CARGO_PKG_VERSION")
                ),
            ]
        );
    }

    #[test]
    fn a_tool_changer_numbers_tools_by_their_slot_and_a_manual_bit_0() {
        let post = BASIC.replace("begin TOOLCHANGE\n", "begin TOOLCHANGE\n\"M6 T[T]\"\n");
        let mut doc = Document::default();
        let mut c = Change::builder(&doc);
        for t in [toolpath("A", "6 mm"), toolpath("B", "3 mm")] {
            c.add_toolpath(t);
        }
        doc.apply(&c.finish()).unwrap();
        let slots = [None, Some("3 mm".to_owned()), None, Some("6 mm".to_owned())];
        let file = |slots| File {
            path: std::path::Path::new("/out.nc"),
            job: None,
            made: chrono::NaiveDateTime::default(),
            slots,
        };
        let post = Post::parse(&post).unwrap();
        let text = post.render(&doc, &file(&slots)).unwrap();
        assert!(text.contains("M6 T2"), "3 mm is in slot 2: {text}");
        let unloaded = [Some("6 mm".to_owned())];
        let text = post.render(&doc, &file(&unloaded)).unwrap();
        assert!(text.contains("M6 T0"), "3 mm is changed by hand: {text}");
        // no tool changer: numbered in the order used
        assert!(post.render(&doc, &file(&[])).unwrap().contains("M6 T2"));
    }

    #[test]
    fn every_made_variant_writes_its_units_tool_changes_arcs_and_wrapping() {
        use cut_craft_core::document::{Along, Job};
        let mut names: Vec<String> = BUILT_IN
            .iter()
            .map(|(_, text)| Post::parse(text).unwrap().name)
            .collect();
        // the two hand-written FluidNC defaults have tests of their own
        let made = BUILT_IN
            .iter()
            .filter(|(f, _)| !["fluidnc_atc_mm.pp", "fluidnc_atc_wrap_a_mm.pp"].contains(f));
        for (file, text) in made {
            // the test renderer splits on the CR LF line ending
            let post = text.replace(r#"LINE_ENDING = "[10]""#, r#"LINE_ENDING = "[13][10]""#);
            let along = if file.contains("wrap_y") {
                Some(Along::X)
            } else if file.contains("wrap_x") {
                Some(Along::Y)
            } else {
                None
            };
            let two = vec![toolpath("A", "6 mm"), toolpath("B", "3 mm")];
            let job = along.map_or_else(Job::default, |a| Job::rotary(300.0, 50.0, a));
            let lines = render_for(&post, job, two);
            let any = |s: &str| lines.iter().any(|l| l.contains(s));
            let has = |s: &str| lines.iter().any(|l| l == s);
            assert_eq!(any("G20"), file.contains("inch"), "{file}");
            if file.starts_with("grbl") {
                assert!(has("M0") && !any("M6"), "{file}: {lines:?}");
            } else if file.starts_with("mach3") {
                assert!(has("T2 M6") && !any("TT"), "{file}: {lines:?}");
            } else {
                assert!(has("M6 T2"), "{file}: {lines:?}");
            }
            assert_eq!(has("G91.1"), file.starts_with("mach3_arcs"), "{file}");
            if along.is_some() {
                assert!(any(" A"), "{file} turns A: {lines:?}");
            }
            if file.contains("g93") {
                let feeds = lines.iter().filter(|l| l.starts_with("G1"));
                assert!(feeds.clone().all(|l| l.contains(" F")), "{file}: {lines:?}");
                assert!(has("G93") && has("G94"), "{file}");
            }
        }
        names.sort();
        names.dedup();
        assert_eq!(names.len(), BUILT_IN.len(), "each has its own name");
    }

    #[test]
    fn tools_used_and_toolpaths_output_list_one_line_each_in_order_of_use() {
        let post = BASIC.replace(
            "begin HEADER\n",
            "begin HEADER\n\"(tool [TOOLS_USED])\"\n\"(path [TOOLPATHS_OUTPUT])\"\n",
        );
        let toolpaths = vec![
            toolpath("A", "6 mm"),
            toolpath("B", "3 mm"),
            toolpath("C", "6 mm"),
        ];
        let lines = render(&post, toolpaths);
        assert_eq!(
            lines[..5],
            [
                "(tool 6 mm)",
                "(tool 3 mm)",
                "(path A)",
                "(path B)",
                "(path C)"
            ]
        );
    }

    #[test]
    fn posts_without_moves_are_rejected() {
        let post = BASIC.replace("begin FEED_MOVE", "begin SOMETHING_ELSE");
        assert_eq!(
            Post::parse(&post).unwrap_err(),
            "the post has no FEED_MOVE block"
        );
    }

    /// A five-drill bank toolpath of two plunges: holes 0..5 at x = 0, 32... 128, then
    /// holes 5 and 6 at x = 160 and 192, each drilled straight down 10 mm.
    fn bank_toolpath() -> Toolpath {
        use cut_craft_core::drill_bank::{Arm, BankHit, BankToolpath, DrillBank, DrillId};
        let bank = DrillBank::new(5, 0, 32.0, 5.0);
        let mut moves = vec![];
        for i in 0..7_i32 {
            let x = 32.0 * f64::from(i);
            moves.push(mv(MoveKind::Rapid, x, 0.0, 5.0));
            moves.push(mv(MoveKind::Plunge, x, 0.0, -10.0));
            moves.push(mv(MoveKind::Rapid, x, 0.0, 5.0));
        }
        let h = |index| DrillId {
            arm: Arm::Horizontal,
            index,
        };
        let hits = vec![
            BankHit {
                start: 0,
                end: 15,
                head: cut_craft_core::geometry::Point::new(64.0, 0.0),
                drills: (0..5).map(h).collect(),
            },
            BankHit {
                start: 15,
                end: 21,
                // the reference (middle) drill two pitches before the first hole
                head: cut_craft_core::geometry::Point::new(224.0, 0.0),
                drills: vec![h(0), h(1)],
            },
        ];
        Toolpath {
            strategy: "drill_bank_toolpath".into(),
            moves,
            drill_bank: Some(BankToolpath { bank, hits }),
            fallbacks: vec![],
            ..toolpath("Shelf Pins", "5 mm Drill")
        }
    }

    const PATTERN: &str = "VAR DRILL_LIST = [DL|D|,| ]\n\
         begin SETUP_DRILL_BANK\n\"(bank on)\"\n\
         begin DRILL_BANK_PATTERN\n\"M100 P[DB] (X[DBX] Y[DBY]) [DL]\"\n\
         begin FINAL_DRILL_BANK_ACTION\n\"(bank off)\"\n\
         begin HEADER";

    #[test]
    fn a_drill_bank_plunges_once_for_each_pattern_of_drills() {
        let post = BASIC.replace("begin HEADER", PATTERN);
        let lines = render(&post, vec![bank_toolpath()]);
        let at = |text: &str| {
            lines
                .iter()
                .position(|l| l == text)
                .unwrap_or_else(|| panic!("{text} in {lines:?}"))
        };
        let on = at("(bank on)");
        let first = at("M100 P31 (X31 Y0) D1 D2,D4,D8,D16");
        let second = at("M100 P3 (X3 Y0) D1 D2");
        assert!(on < first && first < second && second < at("(bank off)"));
        // the head goes where its reference drill is: one plunge a hit, not a hole
        let plunges: Vec<&String> = lines.iter().filter(|l| l.contains("Z-10.000")).collect();
        assert_eq!(plunges.len(), 2, "{lines:?}");
        assert!(
            lines[first + 1].starts_with("G0 X64.000 Y0.000"),
            "{lines:?}"
        );
        assert!(
            lines[second + 1].starts_with("G0 X224.000 Y0.000"),
            "{lines:?}"
        );
    }

    #[test]
    fn a_way_round_a_keep_out_zone_before_a_plunge_still_plunges_once() {
        let mut t = bank_toolpath();
        // a waypoint leading into the second hit, as keep-out routing puts there
        t.moves.insert(15, mv(MoveKind::Rapid, 150.0, 40.0, 5.0));
        let bank = t.drill_bank.as_mut().unwrap();
        bank.hits[1].end += 1;
        let post = BASIC.replace("begin HEADER", PATTERN);
        let lines = render(&post, vec![t]);
        let plunges = lines.iter().filter(|l| l.contains("Z-10.000")).count();
        assert_eq!(plunges, 2, "{lines:?}");
    }

    #[test]
    fn a_per_drill_post_extends_and_retracts_only_the_drills_that_change() {
        let post = BASIC.replace(
            "begin HEADER",
            "begin SETUP_DRILL_BANK\n\"(bank on)\"\n\
             begin DRILL_BANK_EXTEND_DRILL\n\"M61 P[CD]\"\n\
             begin DRILL_BANK_RETRACT_DRILL\n\"M62 P[CD]\"\n\
             begin FINAL_DRILL_BANK_ACTION\n\"(bank off)\"\n\
             begin HEADER",
        );
        let lines = render(&post, vec![bank_toolpath()]);
        let bank: Vec<&str> = lines
            .iter()
            .map(String::as_str)
            .filter(|l| l.starts_with("M6") || l.starts_with("(bank"))
            .collect();
        assert_eq!(
            bank,
            [
                "(bank on)",
                "M61 P1",
                "M61 P2",
                "M61 P4",
                "M61 P8",
                "M61 P16",
                // the second plunge keeps 1 and 2 down
                "M62 P4",
                "M62 P8",
                "M62 P16",
                // all up again before leaving
                "M62 P1",
                "M62 P2",
                "(bank off)",
            ]
        );
    }

    #[test]
    fn a_post_without_drill_bank_blocks_refuses_drill_bank_toolpaths() {
        let mut doc = Document::default();
        let mut c = Change::builder(&doc);
        c.add_toolpath(bank_toolpath());
        doc.apply(&c.finish()).unwrap();
        let file = File {
            path: Path::new("x.nc"),
            job: None,
            made: chrono::NaiveDateTime::default(),
            slots: &[],
        };
        let why = Post::parse(BASIC).unwrap().render(&doc, &file).unwrap_err();
        assert!(
            why.starts_with("Shelf Pins uses a drill bank") && why.contains("SETUP_DRILL_BANK"),
            "{why}"
        );
        // both modes at once cannot be told apart
        let both = BASIC.replace(
            "begin HEADER",
            &format!(
                "{PATTERN}\nbegin DRILL_BANK_EXTEND_DRILL\n\"M61\"\n\
                 begin DRILL_BANK_RETRACT_DRILL\n\"M62\"\nbegin HEADER"
            ),
        );
        let why = Post::parse(&both).unwrap().render(&doc, &file).unwrap_err();
        assert!(why.contains("not both"), "{why}");
    }
}

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
