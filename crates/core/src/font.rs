//! Fonts for Text: glyph outlines and laying a string out as vector paths.
//!
//! Font units are whatever the font file uses; `cap_height` (the height of a capital letter, in
//! those units) is what Text's height scales to, as Aspire's Text Height does.

use crate::geometry::{Path, Point, Segment};
use glam::DAffine2;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Glyph {
    /// How far the pen moves after this glyph, in font units.
    pub advance: f64,
    /// Outlines (or, in single-line fonts, strokes) with the baseline at y = 0.
    pub paths: Vec<Path>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Font {
    pub name: String,
    pub cap_height: f64,
    /// Baseline to baseline, in font units.
    pub line_pitch: f64,
    pub glyphs: BTreeMap<char, Glyph>,
    /// Extra advance between a pair of glyphs, in font units (negative pulls them together).
    pub kerning: HashMap<(char, char), f64>,
}

/// How the lines of a block line up with each other and with the anchor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
}

impl Font {
    /// The text as paths, `height` tall per capital letter. The anchor is where the last line's
    /// baseline meets the block's left edge, centre or right edge, following `align`.
    // ponytail: anchor on the last baseline [inf]; Aspire's own anchor rule is not in its help
    pub fn layout(&self, text: &str, height: f64, align: Align, anchor: Point) -> Vec<Path> {
        let scale = height / self.cap_height;
        let lines: Vec<&str> = text.lines().collect();
        let mut out = Vec::new();
        for (row, line) in lines.iter().enumerate() {
            let width = self.width(line);
            let x0 = match align {
                Align::Left => 0.0,
                Align::Center => -width / 2.0,
                Align::Right => -width,
            };
            #[expect(clippy::cast_precision_loss, reason = "line counts are small")]
            let y = (lines.len() - 1 - row) as f64 * self.line_pitch;
            let mut x = x0;
            let mut prev = None;
            for ch in line.chars() {
                let Some(glyph) = self.glyph(ch) else {
                    continue;
                };
                x += prev.map_or(0.0, |before| self.kern(before, ch));
                let place = DAffine2::from_translation(anchor)
                    * DAffine2::from_scale(glam::DVec2::splat(scale))
                    * DAffine2::from_translation(glam::DVec2::new(x, y));
                out.extend(glyph.paths.iter().map(|p| p.transformed(&place)));
                x += glyph.advance;
                prev = Some(ch);
            }
        }
        out
    }

    /// Each glyph of one line, `height` tall per capital: where it starts along the
    /// baseline, its advance, and its paths placed at x = 0 on the baseline.
    pub fn glyphs(&self, line: &str, height: f64) -> Vec<(f64, f64, Vec<Path>)> {
        let k = height / self.cap_height;
        let scale = DAffine2::from_scale(glam::DVec2::splat(k));
        let mut x = 0.0;
        let mut prev = None;
        let mut out = Vec::new();
        for ch in line.chars() {
            let Some(glyph) = self.glyph(ch) else {
                continue;
            };
            x += prev.map_or(0.0, |before| self.kern(before, ch));
            out.push((
                x * k,
                glyph.advance * k,
                glyph.paths.iter().map(|p| p.transformed(&scale)).collect(),
            ));
            x += glyph.advance;
            prev = Some(ch);
        }
        out
    }

    /// A line's width in font units: advances and kerning.
    pub fn width(&self, line: &str) -> f64 {
        let mut w = 0.0;
        let mut prev = None;
        for c in line.chars() {
            if let Some(g) = self.glyph(c) {
                w += g.advance + prev.map_or(0.0, |p| self.kern(p, c));
                prev = Some(c);
            }
        }
        w
    }

    fn glyph(&self, c: char) -> Option<&Glyph> {
        self.glyphs.get(&c)
    }

    fn kern(&self, a: char, b: char) -> f64 {
        self.kerning.get(&(a, b)).copied().unwrap_or_default()
    }
}

/// Where Cut Craft looks for Vectric `.vfont` single-line fonts: `fonts` in its settings
/// folder. Point it at a Vectric install's `Fonts` folder (a link will do).
pub fn vfont_dir() -> Option<PathBuf> {
    crate::file::config_dir().map(|d| d.join("fonts"))
}

/// Every font Text can use: single-line `.vfont` fonts by file name, then the system's
/// TrueType / OpenType families.
pub fn installed() -> Vec<String> {
    let mut names = vfonts();
    let mut families: Vec<String> = system()
        .faces()
        .filter_map(|f| f.families.first().map(|(n, _)| n.clone()))
        .collect();
    families.sort();
    families.dedup();
    families.retain(|f| !names.contains(f));
    names.extend(families);
    #[cfg(feature = "testing")]
    names.insert(0, tests::BOXES.into());
    names
}

fn vfonts() -> Vec<String> {
    let mut names: Vec<String> = vfont_dir()
        .and_then(|d| std::fs::read_dir(d).ok())
        .into_iter()
        .flatten()
        .filter_map(|e| {
            let p = e.ok()?.path();
            (p.extension()? == "vfont").then(|| p.file_stem()?.to_str().map(String::from))?
        })
        .collect();
    names.sort();
    names
}

/// The font called `name`, with the glyphs `text` needs. `bold` and `italic` pick a
/// TrueType family's face; single-line fonts have one face.
pub fn load(name: &str, bold: bool, italic: bool, text: &str) -> Result<Font, String> {
    #[cfg(feature = "testing")]
    if name == tests::BOXES {
        return Ok(tests::boxes());
    }
    if let Some(path) = vfont_dir()
        .map(|d| d.join(format!("{name}.vfont")))
        .filter(|p| p.is_file())
    {
        let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut font = crate::vfont::read(&bytes)?;
        font.name = name.into();
        return Ok(font);
    }
    let db = system();
    let query = fontdb::Query {
        families: &[fontdb::Family::Name(name)],
        weight: if bold {
            fontdb::Weight::BOLD
        } else {
            fontdb::Weight::NORMAL
        },
        style: if italic {
            fontdb::Style::Italic
        } else {
            fontdb::Style::Normal
        },
        ..fontdb::Query::default()
    };
    let id = db
        .query(&query)
        .ok_or_else(|| format!("no font called {name}"))?;
    db.with_face_data(id, |data, index| {
        let face = ttf_parser::Face::parse(data, index).map_err(|e| e.to_string())?;
        Ok(true_type(name, &face, text))
    })
    .ok_or_else(|| format!("could not read the font {name}"))?
}

fn system() -> &'static fontdb::Database {
    static DB: OnceLock<fontdb::Database> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = fontdb::Database::new();
        db.load_system_fonts();
        db
    })
}

/// A TrueType / OpenType face's glyphs for the characters of `text`.
// ponytail: no TrueType kerning (kern / GPOS tables) yet; vfonts carry theirs
fn true_type(name: &str, face: &ttf_parser::Face, text: &str) -> Font {
    let glyph = |c| {
        let id = face.glyph_index(c)?;
        let mut outline = Outline::default();
        face.outline_glyph(id, &mut outline);
        ttf_parser::OutlineBuilder::close(&mut outline);
        Some(Glyph {
            advance: f64::from(face.glyph_hor_advance(id)?),
            paths: outline.paths,
        })
    };
    let glyphs: BTreeMap<char, Glyph> = text
        .chars()
        .chain(['H'])
        .filter_map(|c| Some((c, glyph(c)?)))
        .collect();
    let cap_height = face
        .capital_height()
        .filter(|h| *h > 0)
        .map(f64::from)
        .or_else(|| {
            glyphs
                .get(&'H')
                .map(|g| g.paths.iter().map(|p| p.bounds().max.y).fold(0.0, f64::max))
        })
        .filter(|h| *h > 0.0)
        .unwrap_or_else(|| f64::from(face.units_per_em()) * 0.7);
    Font {
        name: name.into(),
        cap_height,
        line_pitch: f64::from(face.ascender()) - f64::from(face.descender())
            + f64::from(face.line_gap()),
        glyphs,
        kerning: HashMap::new(),
    }
}

/// Collects a glyph outline as closed paths; quadratic curves become cubics.
#[derive(Default)]
struct Outline {
    paths: Vec<Path>,
    current: Option<Path>,
}

impl Outline {
    fn at(&self) -> Point {
        self.current.as_ref().map_or(Point::ZERO, |p| {
            p.segments.last().map_or(p.start, Segment::end)
        })
    }

    fn push(&mut self, s: Segment) {
        if let Some(p) = &mut self.current {
            p.segments.push(s);
        }
    }
}

fn pt(x: f32, y: f32) -> Point {
    Point::new(f64::from(x), f64::from(y))
}

impl ttf_parser::OutlineBuilder for Outline {
    fn move_to(&mut self, x: f32, y: f32) {
        self.close();
        self.current = Some(Path {
            start: pt(x, y),
            segments: vec![],
            closed: false,
        });
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.push(Segment::Line { to: pt(x, y) });
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (from, c, to) = (self.at(), pt(x1, y1), pt(x, y));
        self.push(Segment::Cubic {
            c1: from + (c - from) * (2.0 / 3.0),
            c2: to + (c - to) * (2.0 / 3.0),
            to,
        });
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.push(Segment::Cubic {
            c1: pt(x1, y1),
            c2: pt(x2, y2),
            to: pt(x, y),
        });
    }

    fn close(&mut self) {
        let Some(mut p) = self.current.take() else {
            return;
        };
        if p.segments.is_empty() {
            return;
        }
        let end = p.segments.last().map_or(p.start, Segment::end);
        if end.distance(p.start) > 0.0 {
            p.segments.push(Segment::Line { to: p.start });
        }
        p.closed = true;
        self.paths.push(p);
    }
}

#[cfg(any(test, feature = "testing"))]
pub(crate) mod tests {
    #[allow(
        clippy::wildcard_imports,
        reason = "test builds use more of the module"
    )]
    use super::*;

    /// The test font's name; `installed` lists it and `load` gives it in test builds.
    pub const BOXES: &str = "Test Boxes";

    /// A font whose every printable ASCII glyph is a box 8 wide and 10 tall, advancing 10.
    pub fn boxes() -> Font {
        let mut glyphs = BTreeMap::new();
        for c in '!'..='~' {
            let square = Path::polyline(
                &[
                    Point::new(0.0, 0.0),
                    Point::new(8.0, 0.0),
                    Point::new(8.0, 10.0),
                    Point::new(0.0, 10.0),
                ],
                true,
            );
            glyphs.insert(
                c,
                Glyph {
                    advance: 10.0,
                    paths: vec![square],
                },
            );
        }
        glyphs.insert(
            ' ',
            Glyph {
                advance: 5.0,
                paths: vec![],
            },
        );
        Font {
            name: "Boxes".into(),
            cap_height: 10.0,
            line_pitch: 15.0,
            glyphs,
            kerning: HashMap::from([(('A', 'V'), -2.0)]),
        }
    }

    #[cfg(test)]
    fn bounds(paths: &[Path]) -> crate::geometry::Rect {
        paths
            .iter()
            .map(Path::bounds)
            .reduce(crate::geometry::Rect::union)
            .unwrap()
    }

    #[test]
    fn capitals_are_the_asked_height_and_sit_on_the_anchor() {
        let paths = boxes().layout("HI", 20.0, Align::Left, Point::new(100.0, 50.0));
        assert_eq!(paths.len(), 2);
        let b = bounds(&paths);
        assert!((b.min - Point::new(100.0, 50.0)).length() < 1e-9);
        assert!((b.size().y - 20.0).abs() < 1e-9);
        // two advances of 10 at scale 2, the last glyph 8 wide
        assert!((b.size().x - 36.0).abs() < 1e-9);
    }

    #[test]
    fn glyphs_report_their_place_along_the_line() {
        let g = boxes().glyphs("AV", 20.0);
        assert_eq!(g.len(), 2);
        // scale 2: A at 0, advance 20; V kerned in by 2 × 2
        assert!((g[0].0).abs() < 1e-9 && (g[0].1 - 20.0).abs() < 1e-9);
        assert!((g[1].0 - 16.0).abs() < 1e-9);
        assert!((g[1].2[0].bounds().size().y - 20.0).abs() < 1e-9);
    }

    #[test]
    fn kerning_pulls_pairs_together_and_unknown_characters_are_skipped() {
        let f = boxes();
        assert!((f.width("AV") - 18.0).abs() < 1e-9);
        assert!((f.width("HI") - 20.0).abs() < 1e-9);
        assert!((f.width("H\u{2603}I") - 20.0).abs() < 1e-9);
    }

    #[test]
    fn lines_stack_upwards_from_the_last_baseline_and_align() {
        let f = boxes();
        let right = f.layout("HI\nH", 10.0, Align::Right, Point::ZERO);
        let b = bounds(&right);
        // advances end at the anchor; the last box is 2 short of its advance
        assert!((b.max.x + 2.0).abs() < 1e-9);
        assert!(
            (b.max.y - 25.0).abs() < 1e-9,
            "second line on top: pitch 15 + 10 tall"
        );
        let centred = f.layout("HI", 10.0, Align::Center, Point::ZERO);
        let c = bounds(&centred);
        assert!(
            (c.min.x + 10.0).abs() < 1e-9,
            "a 20-wide line starts 10 left of the anchor"
        );
    }
}

#[cfg(test)]
#[test]
fn a_system_true_type_font_gives_capitals_of_the_asked_height() {
    let name = installed()
        .into_iter()
        .find(|n| {
            n.starts_with("DejaVu Sans")
                || ["Liberation Sans", "Noto Sans", "Arial"].contains(&n.as_str())
        })
        .expect("these tests need DejaVu Sans, Liberation Sans, Noto Sans or Arial installed");
    let font = load(&name, false, false, "HI").unwrap();
    let paths = font.layout("H", 10.0, Align::Left, Point::ZERO);
    assert!(!paths.is_empty() && paths.iter().all(|p| p.closed));
    let b = paths
        .iter()
        .map(Path::bounds)
        .reduce(crate::geometry::Rect::union)
        .unwrap();
    assert!(
        (b.size().y - 10.0).abs() < 0.2,
        "{name}: H is {} tall",
        b.size().y
    );
    assert!(
        load(&name, true, false, "H").is_ok(),
        "bold falls back to the nearest face"
    );
    assert!(load("No Such Font Anywhere", false, false, "H").is_err());
}
