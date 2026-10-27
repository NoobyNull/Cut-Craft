//! Auto Layout Text: the words wrapped into as many lines as make them biggest, scaled to fill
//! the selected closed vector inside a border, and grouped so they pick and move as one.

use cut_craft_core::document::Document;
use cut_craft_core::font::{self, Align, Font};
use cut_craft_core::geometry::{Path, Point, Rect};
use cut_craft_core::glam::DAffine2;
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};

pub struct AutoLayoutText;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// The words; they are wrapped onto as many lines as make them largest
    #[schemars(extend("x-widget" = "multiline"))]
    pub text: String,
    /// Single-line fonts (Vectric `.vfont` files in Cut Craft's fonts folder) come first,
    /// then the system's TrueType fonts
    #[schemars(schema_with = "font_choices")]
    pub font: String,
    /// TrueType fonts only
    pub bold: bool,
    /// TrueType fonts only
    pub italic: bool,
    #[schemars(title = "Text alignment", extend("x-widget" = "row"))]
    pub align: Align,
    /// Baseline to baseline, as a percentage of the font's own
    #[schemars(title = "Line spacing", extend("x-unit" = "%"), range(min = 50, max = 300))]
    pub line_spacing: f64,
    /// Left clear inside the vector on every side, as a percentage of its width and height
    #[schemars(title = "Border", extend("x-unit" = "%"), range(min = 0, max = 45))]
    pub border: f64,
}

fn font_choices(_: &mut SchemaGenerator) -> Schema {
    json_schema!({ "type": "string", "enum": font::installed(), "x-widget": "drop-down" })
}

impl Default for Params {
    fn default() -> Self {
        Self {
            text: "Text".into(),
            font: font::installed().into_iter().next().unwrap_or_default(),
            bold: false,
            italic: false,
            align: Align::Center,
            line_spacing: 100.0,
            border: 10.0,
        }
    }
}

impl Command for AutoLayoutText {
    const ID: &'static str = "auto_layout_text";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Auto Layout Text",
            group: "Create Vectors",
            icon: "drawing/create_vectors/auto_layout_text.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if !(0.0..50.0).contains(&p.border) {
            return Err(CommandError::InvalidParams(
                "the border must be from 0 to under 50 %".into(),
            ));
        }
        ensure_positive("line spacing", p.line_spacing)?;
        let words: Vec<&str> = p.text.split_whitespace().collect();
        if words.is_empty() {
            return Err(CommandError::InvalidParams("enter some text".into()));
        }
        let mut font =
            font::load(&p.font, p.bold, p.italic, &p.text).map_err(CommandError::Failed)?;
        font.line_pitch *= p.line_spacing / 100.0;
        let (_, selected) = doc.selected_paths(|path| path.closed);
        let outline = selected.first().ok_or_else(|| {
            CommandError::Failed("select a closed vector to fit the text in".into())
        })?;
        let bounds = outline.bounds();
        let inset = bounds.size() * (p.border / 100.0);
        let fit = Rect::new(bounds.min + inset, bounds.max - inset);
        let paths = largest_layout(&font, &words, p.align, fit).ok_or_else(|| {
            CommandError::InvalidParams("enter some text the font can draw".into())
        })?;
        let group = doc.next_group();
        let mut c = Change::builder(doc);
        let ids: Vec<_> = paths.into_iter().map(|path| c.add(path)).collect();
        for &id in &ids {
            c.set_group(id, Some(group));
        }
        c.select(ids);
        Ok(c.finish())
    }
}

/// The words on 1, 2, ... lines, each count balanced by width, drawn at the capital height
/// that fits the block in `fit`; the count that comes out largest wins, centred in `fit`.
/// `None` when the font draws none of the words.
fn largest_layout(font: &Font, words: &[&str], align: Align, fit: Rect) -> Option<Vec<Path>> {
    let mut best: Option<(f64, Vec<Path>)> = None;
    for count in 1..=words.len() {
        let lines = wrapped(font, words, count);
        if lines.len() != count {
            continue;
        }
        let text = lines.join("\n");
        let natural = font.layout(&text, 1.0, align, Point::ZERO);
        let Some(b) = natural.iter().map(Path::bounds).reduce(Rect::union) else {
            continue;
        };
        let scale = (fit.size().x / b.size().x).min(fit.size().y / b.size().y);
        if best.as_ref().is_none_or(|(s, _)| scale > *s) {
            let place = DAffine2::from_translation(fit.center())
                * DAffine2::from_scale(cut_craft_core::glam::DVec2::splat(scale))
                * DAffine2::from_translation(-b.center());
            best = Some((
                scale,
                natural.iter().map(|q| q.transformed(&place)).collect(),
            ));
        }
    }
    best.map(|(_, paths)| paths)
}

/// The words in `count` lines, each filled to about an equal share of the whole width.
#[expect(clippy::cast_precision_loss, reason = "word and line counts are small")]
fn wrapped(font: &Font, words: &[&str], count: usize) -> Vec<String> {
    let space = font.width(" ");
    let total: f64 = words.iter().map(|w| font.width(w)).sum::<f64>()
        + space * (words.len().saturating_sub(1)) as f64;
    #[expect(clippy::cast_precision_loss, reason = "line counts are small")]
    let share = total / count as f64;
    let mut lines: Vec<String> = vec![];
    let mut line = String::new();
    let mut width = 0.0;
    for word in words {
        let w = font.width(word);
        let grown = if line.is_empty() {
            w
        } else {
            width + space + w
        };
        // a word moves to the next line when that leaves this one closer to its share,
        // while lines are left to fill
        if !line.is_empty() && lines.len() + 1 < count && (grown - share) > (share - width) {
            lines.push(std::mem::take(&mut line));
            width = 0.0;
        }
        if !line.is_empty() {
            line.push(' ');
            width += space;
        }
        line.push_str(word);
        width += w;
    }
    lines.push(line);
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{after, document_with, rejection, run_on, square};

    cut_craft_core::contract_tests!(AutoLayoutText);

    fn boxes(text: &str) -> Params {
        Params {
            text: text.into(),
            font: "Test Boxes".into(),
            ..Params::default()
        }
    }

    /// The box round everything the command drew.
    fn drawn(doc: &Document) -> Rect {
        doc.selection
            .iter()
            .map(|id| doc.vectors[id].bounds())
            .reduce(Rect::union)
            .expect("text was drawn")
    }

    #[test]
    fn fills_the_selected_vector_to_its_border_and_groups_the_letters() {
        let mut doc = document_with(vec![square(0.0, 0.0, 100.0)]);
        run_on::<AutoLayoutText>(&mut doc, &boxes("HI"));
        let b = drawn(&doc);
        // "HI" is 18 wide by 10 tall: the width fills the 80 inside a 10 % border
        assert!((b.size().x - 80.0).abs() < 1e-6, "{b:?}");
        assert!(
            (b.center() - Point::new(50.0, 50.0)).length() < 1e-6,
            "{b:?}"
        );
        assert!(b.min.y > 10.0 && b.max.y < 90.0);
        let ids: Vec<_> = doc.selection.iter().copied().collect();
        assert_eq!(ids.len(), 2, "one box per letter");
        assert_eq!(doc.group_members(ids[0]).len(), 2, "grouped as one");
    }

    #[test]
    fn wraps_onto_the_lines_that_make_the_text_largest() {
        // a tall narrow box: four words on four lines are far bigger than on one
        let mut doc = document_with(vec![square(0.0, 0.0, 40.0).transformed(
            &DAffine2::from_scale(cut_craft_core::glam::DVec2::new(1.0, 5.0)),
        )]);
        run_on::<AutoLayoutText>(&mut doc, &boxes("HI HI HI HI"));
        let b = drawn(&doc);
        assert!(
            (b.size().x - 32.0).abs() < 1e-6,
            "each line fills the width: {b:?}"
        );
        // four lines of capitals 17.8 tall, 1.5 capitals apart: about 98
        assert!(b.size().y > 90.0, "four lines tall: {b:?}");
        let one_line = after::<AutoLayoutText>(&Params {
            border: 0.0,
            ..boxes("HI")
        });
        assert!(drawn(&one_line).size().y <= drawn(&one_line).size().x);
    }

    #[test]
    fn lines_line_up_as_asked() {
        let mut doc = document_with(vec![square(0.0, 0.0, 100.0)]);
        run_on::<AutoLayoutText>(
            &mut doc,
            &Params {
                align: Align::Left,
                ..boxes("HIHI HI")
            },
        );
        let lefts: Vec<f64> = doc
            .selection
            .iter()
            .map(|id| doc.vectors[id].bounds().min.x)
            .collect();
        let leftmost = lefts.iter().copied().fold(f64::INFINITY, f64::min);
        assert!(
            lefts
                .iter()
                .filter(|&&x| (x - leftmost).abs() < 1e-6)
                .count()
                >= 2,
            "both lines start at the left: {lefts:?}"
        );
    }

    #[test]
    fn needs_a_closed_vector_some_text_and_a_sane_border() {
        assert!(rejection::<AutoLayoutText>(&boxes("  ")).contains("text"));
        let open = document_with(vec![Path::polyline(
            &[Point::ZERO, Point::new(10.0, 0.0), Point::new(10.0, 10.0)],
            false,
        )]);
        let why = AutoLayoutText::run(&open, &boxes("HI")).unwrap_err();
        assert!(why.to_string().contains("closed vector"), "{why}");
        assert!(
            rejection::<AutoLayoutText>(&Params {
                border: 60.0,
                ..boxes("HI")
            })
            .contains("border")
        );
        assert!(
            rejection::<AutoLayoutText>(&Params {
                font: "No Such Font Anywhere".into(),
                ..boxes("HI")
            })
            .contains("No Such Font")
        );
    }
}
