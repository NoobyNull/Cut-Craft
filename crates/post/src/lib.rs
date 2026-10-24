//! Vectric-style `.pp` post-processors: a text format of global settings, `VAR` output formats
//! and `begin BLOCK` templates (see `re/SPEC.md`). `Post::parse` reads one; `Post::render`
//! writes a document's toolpaths through it.

use cut_craft_core::document::{Along, Datum, Document, Job, ZZero};
use cut_craft_core::toolpath::MoveKind;
use std::collections::HashMap;
use std::path::Path;

const MM_PER_INCH: f64 = 25.4;
/// Numeric values a post can print; the ones in `LENGTHS` become inches for inch posts
/// (feed rates are lengths per minute).
const NUMBERS: [&str; 25] = [
    "X", "Y", "Z", "F", "FC", "FP", "S", "T", "N", "SAFEZ", "XH", "YH", "ZH", "XLENGTH", "YLENGTH",
    "ZLENGTH", "XMIN", "XMAX", "YMIN", "YMAX", "ZMIN", "ZMAX", "TDIA", "WRAP_DIA", "FI",
];
const LENGTHS: [&str; 21] = [
    "X", "Y", "Z", "F", "FC", "FP", "SAFEZ", "XH", "YH", "ZH", "XLENGTH", "YLENGTH", "ZLENGTH",
    "XMIN", "XMAX", "YMIN", "YMAX", "ZMIN", "ZMAX", "TDIA", "WRAP_DIA",
];
/// Numbers Aspire prints to three places when a post gives no `VAR` format; the rest print
/// whole (`re/SPEC.md`, the vocabulary table).
const THREE_PLACES: [&str; 19] = [
    "X", "Y", "Z", "SAFEZ", "XH", "YH", "ZH", "XLENGTH", "YLENGTH", "ZLENGTH", "XMIN", "XMAX",
    "YMIN", "YMAX", "ZMIN", "ZMAX", "TDIA", "WRAP_DIA", "FI",
];
const TEXTS: [&str; 18] = [
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
];
/// Texts that are lists: a template line holding one is written once per item.
const LISTS: [&str; 2] = ["TOOLS_USED", "TOOLPATHS_OUTPUT"];
/// The blocks Cut Craft writes; a post may define others (arcs, lasers...) that it never uses.
const WRITTEN: [&str; 11] = [
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
];
// ponytail: toolpaths are straight moves, so arc, helical, tape splitting and jet (laser)
// blocks are never written; posts using unknown variables in written blocks are refused

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
    formats: HashMap<String, Format>,
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
            formats: HashMap::new(),
            blocks: HashMap::new(),
        };
        let mut block: Option<String> = None;
        for line in text.lines().map(str::trim) {
            if line.is_empty() || line.starts_with('+') {
                continue;
            }
            if let Some(name) = line.strip_prefix("begin ") {
                let name = name.trim().to_owned();
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

    pub fn render(&self, doc: &Document, file: &File) -> Result<String, String> {
        let toolpaths: Vec<_> = doc.toolpaths.values().collect();
        let Some(first) = toolpaths.first() else {
            return Err("there are no toolpaths to save".into());
        };
        let mut tools: Vec<&str> = vec![];
        for t in &toolpaths {
            if !tools.contains(&t.tool.name.as_str()) {
                tools.push(&t.tool.name);
            }
        }
        let mut out = Output {
            post: self,
            text: String::new(),
            last: HashMap::new(),
            line_number: self.numbering.start,
        };
        let rotary = doc.job.rotary;
        if let Some((axis, letter)) = &self.wrap
            && rotary.is_none()
        {
            return Err(format!(
                "this post wraps {axis} onto {letter}: set the job up as rotary in Job Setup"
            ));
        }
        // a wrapped axis prints degrees round the cylinder
        let round = rotary.map_or(1.0, |r| std::f64::consts::PI * r.diameter);
        let wrapped = |label: &str, flat: f64| match &self.wrap {
            Some((axis, _)) if *axis == label => flat / round * 360.0,
            _ => flat,
        };
        let mut v = self.job_values(doc, file);
        v.lists
            .insert("TOOLS_USED", tools.iter().map(|&t| t.to_owned()).collect());
        v.lists.insert(
            "TOOLPATHS_OUTPUT",
            toolpaths.iter().map(|t| t.name.clone()).collect(),
        );
        v.texts.insert("TOOLPATH_NAME", first.name.clone());
        v.texts.insert("TOOLNAME", first.tool.name.clone());
        v.tool(&first.tool, &tools);
        out.block("HEADER", &v);

        let mut previous_tool: Option<&str> = None;
        let mut first_rapid = true;
        // the rate of the feed moves since the last rapid: a new one starts a FIRST_ block,
        // which is where most posts print it
        let mut feeding: Option<f64> = None;
        let mut at = cut_craft_core::glam::DVec3::new(0.0, 0.0, doc.job.clearance);
        for toolpath in &toolpaths {
            v.texts.insert("TOOLPATH_NAME", toolpath.name.clone());
            v.texts.insert("TOOLNAME", toolpath.tool.name.clone());
            v.tool(&toolpath.tool, &tools);
            match previous_tool {
                Some(t) if t != toolpath.tool.name => {
                    out.block("TOOLCHANGE", &v);
                    out.block("SPINDLE_ON", &v);
                }
                Some(_) => out.block("NEW_SEGMENT", &v),
                None => {}
            }
            previous_tool = Some(&toolpath.tool.name);
            for m in &toolpath.moves {
                let length = m.to.distance(at);
                at = m.to;
                if self.inverse_time && m.kind != MoveKind::Rapid && length == 0.0 {
                    // a move that goes nowhere has no time to invert
                    continue;
                }
                v.numbers.insert("X", wrapped("X", m.to.x));
                v.numbers.insert("Y", wrapped("Y", m.to.y));
                v.numbers.insert("Z", doc.job.machine_z(m.to.z));
                let block = if m.kind == MoveKind::Rapid {
                    feeding = None;
                    let initial = std::mem::replace(&mut first_rapid, false);
                    if initial && self.blocks.contains_key("INITIAL_RAPID_MOVE") {
                        "INITIAL_RAPID_MOVE"
                    } else {
                        "RAPID_MOVE"
                    }
                } else {
                    let rate = if m.kind == MoveKind::Plunge {
                        toolpath.tool.plunge_rate
                    } else {
                        toolpath.tool.feed_rate
                    };
                    v.numbers.insert("F", rate);
                    v.numbers.insert("FI", rate / length);
                    self.feed_block(m.kind, feeding.replace(rate) != Some(rate))
                };
                out.block(block, &v);
            }
        }
        out.block("FOOTER", &v);
        Ok(out.text)
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

    /// What stays the same through the file: the material, the file, the time and the
    /// starting position.
    fn job_values(&self, doc: &Document, file: &File) -> Values {
        let job = &doc.job;
        let safe = job.machine_z(job.clearance);
        let area = job.area();
        let mut v = Values::default();
        for (label, value) in [
            ("XH", 0.0),
            ("YH", 0.0),
            ("ZH", safe),
            ("SAFEZ", safe),
            ("X", 0.0),
            ("Y", 0.0),
            ("Z", safe),
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
            // Cut Craft has no notes yet
            ("FILE_NOTES", None),
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

/// Aspire's own wording is not known (`re/SPEC.md`); these are ours.
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
    fn tool(&mut self, tool: &cut_craft_core::toolpath::Tool, tools: &[&str]) {
        let number = tools
            .iter()
            .position(|t| *t == tool.name)
            .map_or(1, |i| i + 1);
        self.numbers
            .insert("T", f64::from(u32::try_from(number).unwrap_or(u32::MAX)));
        self.numbers.insert("S", tool.spindle_speed);
        self.numbers.insert("FC", tool.feed_rate);
        self.numbers.insert("FP", tool.plunge_rate);
        self.numbers.insert("TDIA", tool.diameter);
    }
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

    fn toolpath(name: &str, tool: &str) -> Toolpath {
        Toolpath {
            name: name.into(),
            strategy: "profile_toolpath".into(),
            params: serde_json::json!({}),
            tool: Tool {
                name: tool.into(),
                ..Tool::default()
            },
            vectors: vec![],
            moves: vec![
                mv(MoveKind::Rapid, 0.0, 0.0, 5.0),
                mv(MoveKind::Plunge, 0.0, 0.0, -3.0),
                mv(MoveKind::Cut, 25.4, 0.0, -3.0),
                mv(MoveKind::Cut, 25.4, 25.4, -3.0),
                mv(MoveKind::Rapid, 25.4, 25.4, 5.0),
            ],
            visible: true,
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
        };
        let text = Post::parse(post).unwrap().render(&doc, &file).unwrap();
        assert!(text.ends_with("\r\n"), "LINE_ENDING is honoured");
        text.split("\r\n")
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect()
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
        let post = BASIC.replace("\"G1 [X] [Y] [Z] [F]\"", "\"G1 [X] [Y] [Z] [I] [F]\"");
        assert_eq!(
            Post::parse(&post).unwrap_err(),
            "the post uses [I], which is not supported yet"
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
            "{BASIC}\nVAR ARC_CENTRE_I_INC_POSITION = [I|A|I|1.3]\nbegin CW_ARC_MOVE\n\"G2 [X] [Y] [I] [J] [F]\"\nbegin REVISION_COMMENT\n\"(revision [REVISION])\"\n"
        );
        let lines = render(&post, vec![toolpath("P", "T")]);
        assert!(
            lines
                .iter()
                .all(|l| !l.starts_with("G2 ") && !l.starts_with("(revision")),
            "{lines:#?}"
        );
    }

    #[test]
    fn variables_without_a_var_line_print_in_aspires_default_formats() {
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
}
