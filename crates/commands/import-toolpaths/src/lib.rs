//! Import Toolpaths: a G-code file (as a post-processor writes it, from Cut Craft or another
//! program) read back as toolpaths, one for each tool it uses, to preview, check and save
//! again through another post.
//!
//! Read: G0 / G1 / G2 / G3 (arcs in the XY plane, by I J centre or R radius), G20 / G21,
//! G90 / G91, F, S and T with M6. Comments in brackets or after `;` and line numbers are
//! passed over. Drilling cycles (G81 to G89) are refused with a message, as reading them
//! wrongly would show holes where none are cut.

use cut_craft_core::document::{Document, Job, ZZero};
use cut_craft_core::glam::DVec3;
use cut_craft_core::toolpath::{Move, MoveKind, Tool, made_by};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct ImportToolpaths;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// The toolpath's name; with more than one tool in the file, each is named after it and
    /// its tool number
    pub name: String,
    /// The G-code file (.nc, .tap, .gcode, .ngc, .cnc, .txt)
    pub file: String,
    /// The tool the file cuts with: its shape and size draw the preview; the file's own
    /// feed rate, spindle speed and tool number are used where it gives them
    pub tool: Tool,
    /// Which tool's part of the file, counting from 1 in the order they come (0: every
    /// part, a toolpath each)
    #[serde(default)]
    #[schemars(skip)]
    pub part: u32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Imported".into(),
            file: String::new(),
            tool: Tool::default(),
            part: 0,
        }
    }
}

impl Command for ImportToolpaths {
    const ID: &'static str = "import_toolpaths";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Import Toolpaths",
            group: "Toolpath Operations",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if p.file.trim().is_empty() {
            return Err(CommandError::Failed(
                "pick the G-code file to import (File ▸ Import ▸ Import Toolpaths…)".into(),
            ));
        }
        let text = std::fs::read_to_string(&p.file)
            .map_err(|e| CommandError::Failed(format!("could not read {}: {e}", p.file)))?;
        let parts = parse(&text, &doc.job).map_err(CommandError::Failed)?;
        let several = parts.len() > 1;
        let mut c = Change::builder(doc);
        let mut made = 0;
        for (n, part) in (1u32..).zip(parts) {
            if p.part != 0 && p.part != n {
                continue;
            }
            let mut tool = p.tool.clone();
            if let Some(number) = part.tool {
                tool.number = number;
            }
            if let Some(feed) = part.feed {
                tool.feed_rate = feed;
            }
            if let Some(speed) = part.spindle {
                tool.spindle_speed = speed;
            }
            let name = match part.tool {
                Some(t) if several => format!("{} T{t}", p.name),
                _ if several => format!("{} {n}", p.name),
                _ => p.name.clone(),
            };
            let own = Params {
                name: p.name.clone(),
                part: n,
                ..p.clone()
            };
            c.add_toolpath(made_by::<Self>(&own, name, tool, vec![], part.moves));
            made += 1;
        }
        if made == 0 {
            return Err(CommandError::Failed(format!(
                "{} has no part {} (it has fewer tools)",
                p.file, p.part
            )));
        }
        Ok(c.finish())
    }
}

/// The moves made with one tool.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Part {
    /// Its `T` number, if the file gives one.
    pub tool: Option<u32>,
    /// The first feed rate given, mm/min.
    pub feed: Option<f64>,
    pub spindle: Option<f64>,
    pub moves: Vec<Move>,
}

/// How far an arc's chords may stray from it, mm.
const ARC_TOLERANCE: f64 = 0.01;
const MM_PER_INCH: f64 = 25.4;

/// `text`'s moves, a part for each tool, in job millimetres with z from the material
/// surface (the job's Z zero undone).
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "G, M and T numbers"
)]
pub fn parse(text: &str, job: &Job) -> Result<Vec<Part>, String> {
    let bed = if job.z_zero == ZZero::MachineBed {
        job.thickness
    } else {
        0.0
    };
    let home = job.home();
    // the machine's position, in the file's coordinates converted to mm
    let mut at = DVec3::new(home.x, home.y, job.machine_z(home.z));
    let (mut scale, mut absolute, mut motion) = (1.0, true, None::<u32>);
    let mut parts = vec![Part::default()];
    // a T picks the next tool; the change itself comes with M6, on its line or later
    let mut next_tool = None;
    for (line_number, raw) in (1usize..).zip(text.lines()) {
        let words = words(raw).map_err(|e| format!("line {line_number}: {e}"))?;
        if words.is_empty() {
            continue;
        }
        let value = |letter: char| words.iter().find(|(l, _)| *l == letter).map(|&(_, v)| v);
        let mut moved = false;
        for &(letter, v) in &words {
            match letter {
                'G' => {
                    let code = (v * 10.0).round() as i64;
                    match code {
                        0 | 10 | 20 | 30 => motion = Some(u32::try_from(code / 10).unwrap_or(0)),
                        200 => scale = MM_PER_INCH,
                        210 => scale = 1.0,
                        900 => absolute = true,
                        910 => absolute = false,
                        810..=890 => {
                            return Err(format!(
                                "line {line_number} uses a drilling cycle (G{}), which Import \
                                 Toolpaths cannot read yet: save the file from a post that \
                                 writes drilling as plain moves",
                                code / 10
                            ));
                        }
                        // G28 / G53 / G92 and the like: moves this reader cannot place
                        280 | 530 | 920 => moved = true,
                        _ => {}
                    }
                }
                'M' if v.round() as i64 == 6 => {
                    new_part(&mut parts, value('T').or(next_tool));
                }
                'T' => {
                    next_tool = Some(v);
                    // before anything is cut, the first tool is the one in the spindle
                    if let Some(first) = parts.last_mut().filter(|p| p.moves.is_empty()) {
                        first.tool = Some(v.round() as u32);
                    }
                }
                'F' => {
                    let part = parts.last_mut().expect("a part");
                    part.feed.get_or_insert(v * scale);
                }
                'S' => {
                    parts.last_mut().expect("a part").spindle.get_or_insert(v);
                }
                _ => {}
            }
        }
        if moved {
            continue;
        }
        let axis = |letter: char, now: f64| match value(letter) {
            Some(v) if absolute => v * scale,
            Some(v) => now + v * scale,
            None => now,
        };
        let has_axis = ['X', 'Y', 'Z'].iter().any(|&l| value(l).is_some());
        let Some(kind) = motion.filter(|_| has_axis) else {
            continue;
        };
        let to = DVec3::new(axis('X', at.x), axis('Y', at.y), axis('Z', at.z));
        let made = moves_to(at, to, kind, &value, scale)
            .map_err(|e| format!("line {line_number}: {e}"))?;
        let part = parts.last_mut().expect("a part");
        part.moves.extend(made.into_iter().map(|mut m| {
            m.to.z -= bed;
            m
        }));
        at = to;
    }
    parts.retain(|p| !p.moves.is_empty());
    if parts.is_empty() {
        return Err("the file has no moves in it: is it G-code?".into());
    }
    Ok(parts)
}

/// The moves of one G0 to G3 (`kind` 0 to 3) from `at` to `to`, an arc's in chords; the
/// line's words are read by `value` in the file's units, `scale` mm each.
fn moves_to(
    at: DVec3,
    to: DVec3,
    kind: u32,
    value: &impl Fn(char) -> Option<f64>,
    scale: f64,
) -> Result<Vec<Move>, String> {
    let straight = |kind| vec![Move { kind, to }];
    Ok(match kind {
        0 => straight(MoveKind::Rapid),
        1 => {
            let down = (to.x - at.x).abs() < 1e-9 && (to.y - at.y).abs() < 1e-9 && to.z < at.z;
            straight(if down {
                MoveKind::Plunge
            } else {
                MoveKind::Cut
            })
        }
        _ => {
            let clockwise = kind == 2;
            let centre = match (value('I'), value('J'), value('R')) {
                (None, None, Some(radius)) => centre_from_radius(at, to, radius * scale, clockwise)
                    .ok_or("an arc's radius is too small to reach its end")?,
                (i, j, _) => DVec3::new(
                    at.x + i.unwrap_or(0.0) * scale,
                    at.y + j.unwrap_or(0.0) * scale,
                    0.0,
                ),
            };
            arc(at, to, centre, clockwise)
                .into_iter()
                .map(|p| Move {
                    kind: MoveKind::Cut,
                    to: p,
                })
                .collect()
        }
    })
}

/// A tool change: a new part, unless nothing was cut with the last tool yet.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a T number"
)]
fn new_part(parts: &mut Vec<Part>, tool: Option<f64>) {
    let tool = tool.map(|t| t.round() as u32);
    match parts.last_mut() {
        Some(last) if last.moves.is_empty() => {
            if tool.is_some() {
                last.tool = tool;
            }
        }
        _ => parts.push(Part {
            tool,
            ..Part::default()
        }),
    }
}

/// A line's words, a letter and its number each, comments and line numbers left out.
fn words(line: &str) -> Result<Vec<(char, f64)>, String> {
    let mut text = String::with_capacity(line.len());
    let mut depth = 0;
    for ch in line.chars() {
        match ch {
            '(' => depth += 1,
            ')' => depth = (depth - 1).max(0),
            ';' if depth == 0 => break,
            _ if depth == 0 => text.push(ch.to_ascii_uppercase()),
            _ => {}
        }
    }
    let mut out = vec![];
    let mut chars = text.chars().filter(|c| !c.is_whitespace()).peekable();
    while let Some(letter) = chars.next() {
        if letter == '%' {
            continue;
        }
        if !letter.is_ascii_alphabetic() {
            return Err(format!("`{letter}` is not a G-code word"));
        }
        let mut number = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_ascii_digit() || c == '.' || c == '-' || c == '+' {
                number.push(c);
                chars.next();
            } else {
                break;
            }
        }
        if number.is_empty() {
            return Err(format!("`{letter}` has no number after it"));
        }
        let value: f64 = number
            .parse()
            .map_err(|_| format!("`{letter}{number}` is not a number"))?;
        if letter != 'N' {
            out.push((letter, value));
        }
    }
    Ok(out)
}

/// The centre of the arc from `a` to `b` of radius `r` (negative: the longer way round).
fn centre_from_radius(a: DVec3, b: DVec3, r: f64, clockwise: bool) -> Option<DVec3> {
    let chord = (b - a).truncate();
    let half = chord.length() / 2.0;
    if half < 1e-12 || r.abs() < half - 1e-9 {
        return None;
    }
    let off = (r * r - half * half).max(0.0).sqrt();
    let mid = (a + b).truncate() / 2.0;
    let left = chord.perp().normalize();
    // the centre is on the right of the chord for a clockwise arc under 180°
    let side = if clockwise == (r > 0.0) { -1.0 } else { 1.0 };
    let c = mid + left * off * side;
    Some(DVec3::new(c.x, c.y, 0.0))
}

/// Points along the arc from `a` to `b` round `centre` (in XY; z going evenly), ending at `b`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "a segment count"
)]
fn arc(from: DVec3, end: DVec3, centre: DVec3, clockwise: bool) -> Vec<DVec3> {
    let middle = centre.truncate();
    let (out, back) = (from.truncate() - middle, end.truncate() - middle);
    let radius = out.length();
    if radius < 1e-9 {
        return vec![end];
    }
    let start = out.y.atan2(out.x);
    let mut sweep = back.y.atan2(back.x) - start;
    let tau = std::f64::consts::TAU;
    if clockwise {
        if sweep >= -1e-9 {
            sweep -= tau;
        }
    } else if sweep <= 1e-9 {
        sweep += tau;
    }
    let step = 2.0 * (1.0 - (ARC_TOLERANCE / radius).min(1.0)).acos();
    let chords = (sweep.abs() / step.max(1e-3)).ceil().clamp(1.0, 10_000.0) as usize;
    (1..=chords)
        .map(|i| {
            if i == chords {
                return end;
            }
            let along = i as f64 / chords as f64;
            let angle = start + sweep * along;
            DVec3::new(
                middle.x + radius * angle.cos(),
                middle.y + radius * angle.sin(),
                from.z + (end.z - from.z) * along,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_is_a_slug_and_meta_is_labelled() {
        cut_craft_core::testing::id_is_a_slug_and_meta_is_labelled::<ImportToolpaths>();
    }

    #[test]
    fn default_params_round_trip_through_json() {
        cut_craft_core::testing::default_params_round_trip_through_json::<ImportToolpaths>();
    }

    #[test]
    fn schema_describes_an_object() {
        cut_craft_core::testing::schema_describes_an_object::<ImportToolpaths>();
    }

    fn job() -> Job {
        Job::default()
    }

    fn kinds(part: &Part) -> Vec<MoveKind> {
        part.moves.iter().map(|m| m.kind).collect()
    }

    #[test]
    fn rapids_plunges_and_cuts_are_read_with_comments_left_out() {
        let text = "%\n(a square)\nN10 G21 G90\nG0 Z5\nX10 Y10 ; start\nG1 Z-2 F300\nX20\nY20\n";
        let parts = parse(text, &job()).unwrap();
        assert_eq!(parts.len(), 1);
        let p = &parts[0];
        assert_eq!(
            kinds(p),
            [
                MoveKind::Rapid,
                MoveKind::Rapid,
                MoveKind::Plunge,
                MoveKind::Cut,
                MoveKind::Cut
            ]
        );
        assert_eq!(p.moves[1].to, DVec3::new(10.0, 10.0, 5.0));
        assert_eq!(p.moves[4].to, DVec3::new(20.0, 20.0, -2.0));
        assert_eq!(p.feed, Some(300.0));
    }

    #[test]
    fn inches_and_incremental_moves_come_out_in_millimetres() {
        let text = "G20 G91\nG0 X1 Y1 Z0.5\nG1 X1\n";
        let parts = parse(text, &job()).unwrap();
        let last = parts[0].moves.last().unwrap().to;
        let home = job().home();
        assert!((last.x - (home.x + 50.8)).abs() < 1e-9, "{last}");
        assert!((last.y - (home.y + 25.4)).abs() < 1e-9, "{last}");
    }

    #[test]
    fn a_bed_zero_file_is_put_back_on_the_material_surface() {
        let job = Job {
            z_zero: ZZero::MachineBed,
            thickness: 18.0,
            ..Job::default()
        };
        let parts = parse("G0 X0 Y0 Z23\nG1 Z16\n", &job).unwrap();
        assert_eq!(parts[0].moves[0].to.z, 5.0);
        assert_eq!(parts[0].moves[1].to.z, -2.0);
    }

    #[test]
    fn arcs_by_centre_and_by_radius_follow_the_circle() {
        for text in [
            "G0 X10 Y0 Z0\nG3 X-10 Y0 I-10 J0\n",
            "G0 X10 Y0 Z0\nG3 X-10 Y0 R10\n",
        ] {
            let moves = &parse(text, &job()).unwrap()[0].moves;
            assert!(moves.len() > 10, "{text}: chords, {}", moves.len());
            for m in &moves[1..] {
                let r = m.to.truncate().length();
                assert!((r - 10.0).abs() < 0.02, "{text}: {r}");
                // anticlockwise from +x to -x goes over the top
                assert!(m.to.y >= -1e-9, "{text}: {}", m.to);
            }
            assert_eq!(moves.last().unwrap().to, DVec3::new(-10.0, 0.0, 0.0));
        }
        let clockwise = &parse("G0 X10 Y0 Z0\nG2 X-10 Y0 I-10 J0\n", &job()).unwrap()[0].moves;
        assert!(clockwise[1..].iter().all(|m| m.to.y <= 1e-9), "under");
        let full = &parse("G0 X10 Y0 Z0\nG2 X10 Y0 I-10 J0\n", &job()).unwrap()[0].moves;
        assert!(full.iter().any(|m| m.to.x < -9.9), "a whole circle");
    }

    #[test]
    fn each_tool_change_starts_a_part() {
        let text = "T1 M6\nS18000 M3\nG0 X0 Y0 Z5\nG1 Z-1 F500\nT2\nM6\nG0 X5 Y5\nG1 Z-1 F800\n";
        let parts = parse(text, &job()).unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!((parts[0].tool, parts[1].tool), (Some(1), Some(2)));
        assert_eq!(parts[0].spindle, Some(18000.0));
        assert_eq!((parts[0].feed, parts[1].feed), (Some(500.0), Some(800.0)));
    }

    #[test]
    fn what_cannot_be_read_says_so() {
        let cycle = parse("G0 X0 Y0 Z5\nG81 X1 Y1 Z-3 R1\n", &job()).unwrap_err();
        assert!(cycle.contains("drilling cycle"), "{cycle}");
        assert!(parse("hello there\n", &job()).is_err());
        assert!(
            parse("(nothing but a comment)\n", &job())
                .unwrap_err()
                .contains("no moves")
        );
    }

    #[test]
    fn running_it_adds_a_toolpath_for_each_tool_that_recalculates_alone() {
        let dir = std::env::temp_dir().join(format!("cut-craft-import-tp-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("sign.nc");
        std::fs::write(
            &file,
            "T1 M6\nG0 X0 Y0 Z5\nG1 Z-1\nT3 M6\nG0 X5 Y5\nG1 Z-2\n",
        )
        .unwrap();
        let doc = Document::default();
        let p = Params {
            name: "sign".into(),
            file: file.display().to_string(),
            ..Params::default()
        };
        let change = ImportToolpaths::run(&doc, &p).unwrap();
        let mut after = doc.clone();
        after.apply(&change).unwrap();
        let mut names: Vec<_> = after.toolpaths.values().map(|t| t.name.clone()).collect();
        names.sort();
        assert_eq!(names, ["sign T1", "sign T3"]);
        let t3 = after
            .toolpaths
            .values()
            .find(|t| t.name == "sign T3")
            .unwrap();
        assert_eq!(t3.tool.number, 3);
        assert_eq!(
            t3.params["part"], 2,
            "recalculating reads its own part again"
        );
        let again: Params = serde_json::from_value(t3.params.clone()).unwrap();
        let alone = ImportToolpaths::run(&doc, &again).unwrap();
        let mut one = doc.clone();
        one.apply(&alone).unwrap();
        assert_eq!(one.toolpaths.len(), 1);
        assert!(
            ImportToolpaths::run(&doc, &Params::default())
                .unwrap_err()
                .to_string()
                .contains("pick the G-code file")
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
