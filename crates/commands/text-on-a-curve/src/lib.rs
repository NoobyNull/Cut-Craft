//! Text on a Curve: one line of selected text (made with Draw Text) and a selected curve;
//! the letters are laid along the curve, above, on or below it, turned with it or upright.

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::font;
use cut_craft_core::geometry::{Path, Point};
use cut_craft_core::glam::{DAffine2, DVec2};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct TextOnACurve;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Size {
    #[default]
    MaintainTextSize,
    /// The letters grow or shrink so the text runs the curve's whole length
    ScaleTextToFillCurve,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Position {
    #[default]
    AboveCurve,
    OnCurve,
    BelowCurve,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Along {
    #[default]
    Left,
    Middle,
    Right,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Turn {
    /// Each letter leans with the curve
    #[default]
    AlignToCurve,
    KeepVertical,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    #[schemars(title = "Text size")]
    pub size: Size,
    /// Spacing between letters, as a share of their normal spacing
    #[schemars(title = "Text spacing", extend("x-unit" = "%"), range(min = -50, max = 300))]
    pub spacing: f64,
    #[schemars(title = "Text position")]
    pub position: Position,
    /// Put the text on the curve's other side
    pub text_on_other_side: bool,
    /// Gap between the curve and the text
    #[schemars(title = "Offset distance", extend("x-unit" = "mm"), range(min = 0))]
    pub offset: f64,
    #[schemars(title = "Text alignment")]
    pub alignment: Along,
    #[schemars(title = "Letters")]
    pub turn: Turn,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            size: Size::MaintainTextSize,
            spacing: 100.0,
            position: Position::AboveCurve,
            text_on_other_side: false,
            offset: 0.0,
            alignment: Along::Left,
            turn: Turn::AlignToCurve,
        }
    }
}

impl Command for TextOnACurve {
    const ID: &'static str = "text_on_a_curve";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Text on a Curve",
            group: "Create Vectors",
            icon: "drawing/create_vectors/text_on_a_curve.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let refuse = || {
            CommandError::Failed(
                "select a single line of text and the curve to wrap it onto".into(),
            )
        };
        let group = doc
            .selection
            .iter()
            .find_map(|id| {
                doc.vector_groups
                    .get(id)
                    .filter(|g| doc.texts.contains_key(g))
            })
            .copied()
            .ok_or_else(refuse)?;
        let letters: Vec<EntityId> = doc.group_members_of(group);
        let curve = doc
            .selection
            .iter()
            .find(|id| !letters.contains(id))
            .map(|id| &doc.vectors[id])
            .ok_or_else(refuse)?;
        let spec = &doc.texts[&group];
        let text = spec["text"].as_str().unwrap_or_default();
        if text.lines().count() != 1 {
            return Err(refuse());
        }
        let face = font::load(
            spec["font"].as_str().unwrap_or_default(),
            spec["bold"].as_bool().unwrap_or(false),
            spec["italic"].as_bool().unwrap_or(false),
            text,
        )
        .map_err(CommandError::Failed)?;
        let height = spec["height"].as_f64().unwrap_or(10.0);
        let mut curve = curve.clone();
        if p.text_on_other_side {
            curve = curve.reversed();
        }
        let walk = Walk::new(&curve);
        let spread = p.spacing / 100.0;
        let natural = |h: f64| {
            let g = face.glyphs(text, h);
            g.last().map_or(0.0, |last| (last.0 + last.1) * spread)
        };
        let height = match p.size {
            Size::MaintainTextSize => height,
            Size::ScaleTextToFillCurve => {
                let w = natural(height);
                if w > 0.0 {
                    height * walk.total / w
                } else {
                    height
                }
            }
        };
        let width = natural(height);
        let start = match p.alignment {
            Along::Left => 0.0,
            Along::Middle => (walk.total - width) / 2.0,
            Along::Right => walk.total - width,
        };
        let lift = match p.position {
            Position::AboveCurve => p.offset,
            Position::OnCurve => -height / 2.0,
            Position::BelowCurve => -height - p.offset,
        };
        let mut placed = Vec::new();
        for (x, advance, paths) in face.glyphs(text, height) {
            let middle = start + x * spread + advance / 2.0;
            let (at, dir) = walk.at(middle);
            let angle = match p.turn {
                Turn::AlignToCurve => dir.to_angle(),
                Turn::KeepVertical => 0.0,
            };
            let place = DAffine2::from_translation(at)
                * DAffine2::from_angle(angle)
                * DAffine2::from_translation(DVec2::new(-advance / 2.0, lift));
            placed.extend(paths.iter().map(|q| q.transformed(&place)));
        }
        let mut c = Change::builder(doc);
        for &id in &letters {
            c.set_group(id, None);
            c.remove(id);
        }
        // ponytail: the curved text becomes plain curves; Aspire keeps it editable
        c.set_text(group, None);
        let ids: Vec<_> = placed.into_iter().map(|q| c.add(q)).collect();
        for &id in &ids {
            c.set_group(id, Some(group));
        }
        c.select(ids);
        Ok(c.finish())
    }
}

/// A path walked by distance along it, extended straight past its ends.
struct Walk {
    pts: Vec<Point>,
    lengths: Vec<f64>,
    total: f64,
}

impl Walk {
    fn new(path: &Path) -> Self {
        let pts = path.flatten(cut_craft_core::polygon::TOLERANCE);
        let mut lengths = vec![0.0];
        for pair in pts.windows(2) {
            lengths.push(lengths[lengths.len() - 1] + pair[0].distance(pair[1]));
        }
        let total = lengths[lengths.len() - 1];
        Self {
            pts,
            lengths,
            total,
        }
    }

    fn at(&self, along: f64) -> (Point, Point) {
        let i = self
            .lengths
            .partition_point(|&l| l < along)
            .clamp(1, self.pts.len() - 1);
        let (from, to) = (self.pts[i - 1], self.pts[i]);
        let span = self.lengths[i] - self.lengths[i - 1];
        let dir = (to - from).normalize_or_zero();
        let part = if span > 0.0 {
            along - self.lengths[i - 1]
        } else {
            0.0
        };
        (from + dir * part, dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Rect;
    use cut_craft_core::testing::run_on;

    /// "HI" in the test font (made by Draw Text's settings) and a 100 mm line up the y axis.
    fn text_and_line() -> Document {
        let mut doc = Document::default();
        let face = font::load("Test Boxes", false, false, "HI").unwrap();
        let mut c = Change::builder(&doc);
        let ids: Vec<_> = face
            .layout("HI", 10.0, font::Align::Left, Point::new(200.0, 0.0))
            .into_iter()
            .map(|q| c.add(q))
            .collect();
        for &id in &ids {
            c.set_group(id, Some(1));
        }
        c.set_text(
            1,
            Some(serde_json::json!({ "text": "HI", "font": "Test Boxes", "height": 10.0 })),
        );
        let line = c.add(Path::polyline(
            &[Point::new(0.0, 0.0), Point::new(0.0, 100.0)],
            false,
        ));
        let mut all = ids;
        all.push(line);
        c.select(all);
        let change = c.finish();
        doc.apply(&change).unwrap();
        doc
    }

    cut_craft_core::contract_tests!(TextOnACurve, text_and_line);

    fn letters(doc: &Document) -> Rect {
        doc.selection
            .iter()
            .map(|id| doc.vectors[id].bounds())
            .reduce(Rect::union)
            .unwrap()
    }

    #[test]
    fn letters_turn_to_run_up_the_line_on_its_left() {
        let mut doc = text_and_line();
        run_on::<TextOnACurve>(&mut doc, &Params::default());
        assert_eq!(doc.vectors.len(), 3, "two letters and the line");
        let b = letters(&doc);
        // running up the line, "above" is to its left: x from -10 to 0, y from 0 to 18
        assert!(
            (b.min.x + 10.0).abs() < 1e-6 && b.max.x.abs() < 1e-6,
            "{b:?}"
        );
        assert!(
            b.min.y.abs() < 1e-6 && (b.max.y - 18.0).abs() < 1e-6,
            "{b:?}"
        );
        assert!(doc.texts.is_empty(), "the curved text is curves now");
    }

    #[test]
    fn filling_the_curve_and_keeping_upright() {
        let mut doc = text_and_line();
        let p = Params {
            size: Size::ScaleTextToFillCurve,
            turn: Turn::KeepVertical,
            position: Position::OnCurve,
            ..Params::default()
        };
        run_on::<TextOnACurve>(&mut doc, &p);
        let b = letters(&doc);
        // two advances fill 100 mm: the letters are 50 mm tall, upright
        assert!(b.size().y > 90.0, "{b:?}");
    }
}
