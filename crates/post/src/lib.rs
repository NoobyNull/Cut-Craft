//! Vectric-style `.pp` post-processors: a text format of global settings, `VAR` output formats
//! and `begin BLOCK` templates (see `re/SPEC.md`). `Post::parse` reads one; `Post::render`
//! writes a document's toolpaths through it.

use cut_craft_core::document::Document;
use cut_craft_core::toolpath::MoveKind;
use std::collections::HashMap;

const MM_PER_INCH: f64 = 25.4;
/// Numeric values a post can print; the ones in `LENGTHS` become inches for inch posts
/// (feed rates are lengths per minute).
const NUMBERS: [&str; 13] = [
    "X", "Y", "Z", "F", "FC", "FP", "S", "T", "N", "SAFEZ", "XH", "YH", "ZH",
];
const LENGTHS: [&str; 10] = ["X", "Y", "Z", "F", "FC", "FP", "SAFEZ", "XH", "YH", "ZH"];
const TEXTS: [&str; 2] = ["TOOLNAME", "TOOLPATH_NAME"];
// ponytail: arcs, helical moves, DATE/TIME, file names, tape splitting and jet (laser) blocks
// are not supported yet; posts using their variables are refused at parse time

#[derive(Debug)]
pub struct Post {
    pub name: String,
    pub extension: String,
    /// Output in inches instead of millimetres.
    pub inches: bool,
    line_ending: String,
    numbering: Numbering,
    /// (original, replacement) characters applied to text values.
    substitute: Vec<(char, char)>,
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
    /// The documented default, `[X|A||1.0]`.
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
            // other globals (DIRECT_OUTPUT, TAPE_SPLITTING, ...) do not change the text
            _ => {}
        }
        Ok(())
    }

    fn check(&self) -> Result<(), String> {
        for required in ["RAPID_MOVE", "FEED_MOVE"] {
            if !self.blocks.contains_key(required) {
                return Err(format!("the post has no {required} block"));
            }
        }
        for line in self.blocks.values().flatten() {
            for label in labels(line) {
                if !NUMBERS.contains(&label) && !TEXTS.contains(&label) {
                    return Err(format!(
                        "the post uses [{label}], which is not supported yet"
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn render(&self, doc: &Document) -> Result<String, String> {
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
        let safe = doc.job.clearance;
        let mut out = Output {
            post: self,
            text: String::new(),
            last: HashMap::new(),
            line_number: self.numbering.start,
        };
        let mut v = Values::default();
        v.texts.insert("TOOLPATH_NAME", first.name.clone());
        v.texts.insert("TOOLNAME", first.tool.name.clone());
        for (label, value) in [
            ("XH", 0.0),
            ("YH", 0.0),
            ("ZH", safe),
            ("SAFEZ", safe),
            ("X", 0.0),
            ("Y", 0.0),
            ("Z", safe),
        ] {
            v.numbers.insert(label, value);
        }
        v.tool(&first.tool, &tools);
        out.block("HEADER", &v);

        let mut previous_tool: Option<&str> = None;
        let mut first_rapid = true;
        let mut feeding = false;
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
                v.numbers.insert("X", m.to.x);
                v.numbers.insert("Y", m.to.y);
                v.numbers.insert("Z", m.to.z);
                let block = match m.kind {
                    MoveKind::Rapid => {
                        feeding = false;
                        let initial = std::mem::replace(&mut first_rapid, false);
                        if initial && self.blocks.contains_key("INITIAL_RAPID_MOVE") {
                            "INITIAL_RAPID_MOVE"
                        } else {
                            "RAPID_MOVE"
                        }
                    }
                    MoveKind::Plunge if self.blocks.contains_key("PLUNGE_MOVE") => {
                        v.numbers.insert("F", toolpath.tool.plunge_rate);
                        let start = !std::mem::replace(&mut feeding, true);
                        if start && self.blocks.contains_key("FIRST_PLUNGE_MOVE") {
                            "FIRST_PLUNGE_MOVE"
                        } else {
                            "PLUNGE_MOVE"
                        }
                    }
                    kind => {
                        let rate = if kind == MoveKind::Plunge {
                            toolpath.tool.plunge_rate
                        } else {
                            toolpath.tool.feed_rate
                        };
                        v.numbers.insert("F", rate);
                        let start = !std::mem::replace(&mut feeding, true);
                        if start && self.blocks.contains_key("FIRST_FEED_MOVE") {
                            "FIRST_FEED_MOVE"
                        } else {
                            "FEED_MOVE"
                        }
                    }
                };
                out.block(block, &v);
            }
        }
        out.block("FOOTER", &v);
        Ok(out.text)
    }
}

/// The values variables print from at one point in the output.
#[derive(Default)]
struct Values {
    numbers: HashMap<&'static str, f64>,
    texts: HashMap<&'static str, String>,
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
            let mut line = String::new();
            let mut rest = template.as_str();
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
        let value = if self.post.inches && LENGTHS.contains(&label) {
            raw / MM_PER_INCH
        } else {
            raw
        };
        let format = self.post.formats.get(label).cloned().unwrap_or_default();
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
        }
    }

    fn render(post: &str, toolpaths: Vec<Toolpath>) -> Vec<String> {
        let mut doc = Document::default();
        let mut c = Change::builder(&doc);
        for t in toolpaths {
            c.add_toolpath(t);
        }
        doc.apply(&c.finish()).unwrap();
        let text = Post::parse(post).unwrap().render(&doc).unwrap();
        assert!(text.ends_with("\r\n"), "LINE_ENDING is honoured");
        text.split("\r\n")
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect()
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
        let post = BASIC.replace("\"G21 G90\"", "\"G21 G90 ([DATE])\"");
        assert_eq!(
            Post::parse(&post).unwrap_err(),
            "the post uses [DATE], which is not supported yet"
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
