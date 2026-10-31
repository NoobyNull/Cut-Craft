//! Draw Text: the words as vectors in a TrueType or single-line font, grouped so they pick
//! and move as one, as Aspire's text objects do. Ungroup is Convert Text to Curves.

use cut_craft_core::document::Document;
use cut_craft_core::font::{self, Align};
use cut_craft_core::geometry::{Point, Rect};
use cut_craft_core::glam::DAffine2;
use cut_craft_core::params::Xy;
use cut_craft_core::{Change, Command, CommandError, Drag, Interactive, Meta, ensure_positive};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};

pub struct DrawText;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Text of a set height at the anchor point
    #[default]
    Text,
    /// Text scaled to fit a box whose lower-left corner is the anchor point
    TextInABox,
}

/// The border Text in a Box leaves at the box's left and right.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Margin {
    /// Fill the box's width or height
    None,
    /// 10 % of the width each side
    #[default]
    Normal,
    /// 20 % each side
    Wide,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    #[serde(default)]
    #[schemars(extend("x-widget" = "row"))]
    pub mode: Mode,
    /// Each line of text is a line of the block
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
    /// Height of a capital letter
    #[schemars(title = "Text height", extend("x-unit" = "mm"), range(min = 0))]
    pub height: f64,
    /// Where the last line's baseline starts (left), centres or ends (right); click the
    /// drawing to set it
    #[schemars(title = "Anchor point")]
    pub anchor: Xy,
    /// Text in a Box: the box's width and height
    #[serde(default = "default_box")]
    #[schemars(title = "Bounding box dimensions")]
    pub box_size: Xy,
    /// Text in a Box: the border left and right
    #[serde(default)]
    #[schemars(title = "Margin size", extend("x-widget" = "row"))]
    pub margin: Margin,
    /// Kerning and line spacing changed in the 2D view: a click between two letters or two
    /// lines while the form is open
    #[schemars(skip)]
    #[serde(default)]
    pub spacing: font::Spacing,
    /// The text group being edited: its vectors are replaced. Set when the form opens on
    /// selected text.
    #[schemars(skip)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replaces: Option<u64>,
}

fn default_box() -> Xy {
    Xy::new(100.0, 30.0)
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
            align: Align::Left,
            // ponytail: 10 mm [inf]; Aspire remembers the last height used
            height: 10.0,
            anchor: Xy::new(10.0, 10.0),
            mode: Mode::Text,
            box_size: default_box(),
            margin: Margin::Normal,
            spacing: font::Spacing::default(),
            replaces: None,
        }
    }
}

impl Command for DrawText {
    const ID: &'static str = "draw_text";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Draw Text",
            group: "Create Vectors",
            icon: "drawing/create_vectors/draw_text.png",
            shortcut: "",
        }
    }

    /// On selected text, its own settings, so the form edits it.
    fn initial_params(doc: &Document) -> Params {
        selected_text_group(doc)
            .and_then(|group| {
                let p: Params = serde_json::from_value(doc.texts[&group].clone()).ok()?;
                Some(Params {
                    replaces: Some(group),
                    ..p
                })
            })
            .unwrap_or_default()
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("text height", p.height)?;
        let font = font::load(&p.font, p.bold, p.italic, &p.text).map_err(CommandError::Failed)?;
        let paths = match p.mode {
            Mode::Text => {
                font.layout_spaced(&p.text, p.height, p.align, p.anchor.into(), &p.spacing)
            }
            Mode::TextInABox => boxed(&font, p)?,
        };
        if paths.is_empty() {
            return Err(CommandError::InvalidParams(
                "enter some text the font can draw".into(),
            ));
        }
        let group = p
            .replaces
            .filter(|g| doc.texts.contains_key(g))
            .unwrap_or_else(|| doc.next_group());
        let mut c = Change::builder(doc);
        for id in doc.group_members_of(group) {
            c.remove(id);
        }
        let ids: Vec<_> = paths.into_iter().map(|path| c.add(path)).collect();
        for &id in &ids {
            c.set_group(id, Some(group));
        }
        let spec = Params {
            replaces: None,
            ..p.clone()
        };
        c.set_text(
            group,
            Some(serde_json::to_value(spec).expect("params serialise")),
        );
        c.select(ids);
        Ok(c.finish())
    }
}

/// The text scaled to fit the box (less the margin's share of its width), centred in it.
// ponytail: no vertical or horizontal stretch options yet [obs: dialog 279 has both]
fn boxed(
    font: &font::Font,
    p: &Params,
) -> Result<Vec<cut_craft_core::geometry::Path>, CommandError> {
    let (w, h) = (p.box_size.x, p.box_size.y);
    ensure_positive("box width", w)?;
    ensure_positive("box height", h)?;
    let natural = font.layout(&p.text, 1.0, p.align, cut_craft_core::geometry::Point::ZERO);
    let Some(b) = natural
        .iter()
        .map(cut_craft_core::geometry::Path::bounds)
        .reduce(Rect::union)
    else {
        return Ok(vec![]);
    };
    let share = match p.margin {
        Margin::None => 1.0,
        Margin::Normal => 0.8,
        Margin::Wide => 0.6,
    };
    let scale = (w * share / b.size().x).min(h / b.size().y);
    let anchor: cut_craft_core::geometry::Point = p.anchor.into();
    let centre = anchor + cut_craft_core::geometry::Point::new(w, h) / 2.0;
    let place = DAffine2::from_translation(centre)
        * DAffine2::from_scale(cut_craft_core::glam::DVec2::splat(scale))
        * DAffine2::from_translation(-b.center());
    Ok(natural.iter().map(|q| q.transformed(&place)).collect())
}

/// The first selected group that is text (plain groups may come before it).
#[must_use]
pub fn selected_text_group(doc: &Document) -> Option<u64> {
    doc.selection
        .iter()
        .filter_map(|id| doc.vector_groups.get(id))
        .find(|group| doc.texts.contains_key(group))
        .copied()
}

/// What a click in a laid-out block falls between.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gap {
    /// Two letters of a line: the line, and the index of the letter before the gap.
    Letters { line: usize, index: usize },
    /// Two lines.
    Lines,
}

/// Where the glyphs of the text `p` lays out sit.
fn glyph_places(p: &Params) -> Result<Vec<font::Placed>, String> {
    let face = font::load(&p.font, p.bold, p.italic, &p.text)?;
    let placed = face.placed(&p.text, p.height, p.align, p.anchor.into(), &p.spacing);
    Ok(placed.into_iter().map(|(at, _)| at).collect())
}

/// What a click at `at`, within `reach`, falls between in the text `p` lays out: two letters
/// (nearest the middle of the room between them, within the line's height), or two lines
/// (between the lower line's capitals and the upper line's baseline).
pub fn gap_at(p: &Params, at: Point, reach: f64) -> Result<Option<Gap>, String> {
    let placed = glyph_places(p)?;
    for pair in placed.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if a.line != b.line {
            continue;
        }
        let middle = f64::midpoint(a.right, b.left);
        let half = ((b.left - a.right) / 2.0).abs() + reach;
        let band = a.baseline - reach..=a.baseline + p.height + reach;
        if (at.x - middle).abs() <= half && band.contains(&at.y) {
            return Ok(Some(Gap::Letters {
                line: a.line,
                index: a.index,
            }));
        }
    }
    let (mut left, mut right) = (f64::INFINITY, f64::NEG_INFINITY);
    let mut baselines: Vec<f64> = vec![];
    for g in &placed {
        left = left.min(g.left);
        right = right.max(g.right);
        if !baselines.iter().any(|b| (b - g.baseline).abs() < 1e-9) {
            baselines.push(g.baseline);
        }
    }
    baselines.sort_by(f64::total_cmp);
    for pair in baselines.windows(2) {
        let (lower, upper) = (pair[0] + p.height, pair[1]);
        let (lo, hi) = (lower.min(upper), lower.max(upper));
        if (lo - reach..=hi + reach).contains(&at.y)
            && (left - reach..=right + reach).contains(&at.x)
        {
            return Ok(Some(Gap::Lines));
        }
    }
    Ok(None)
}

/// Every gap between two letters of a line in the text `p` lays out: (line, index of the
/// letter before it).
pub fn letter_gaps(p: &Params) -> Result<Vec<(usize, usize)>, String> {
    let placed = glyph_places(p)?;
    Ok(placed
        .windows(2)
        .filter(|pair| pair[0].line == pair[1].line)
        .map(|pair| (pair[0].line, pair[0].index))
        .collect())
}

/// Text ▸ Break Block into Lines: a block of several lines becomes a text of its own per
/// line, each where its line was, editable on its own.
pub struct BreakTextIntoLines;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct BreakParams {
    /// The text group
    pub group: u64,
}

impl Default for BreakParams {
    /// The first group a document gives out.
    fn default() -> Self {
        Self { group: 1 }
    }
}

impl Command for BreakTextIntoLines {
    const ID: &'static str = "break_text_into_lines";
    type Params = BreakParams;

    fn meta() -> Meta {
        Meta {
            label: "Break Block into Lines",
            group: "Create Vectors",
            icon: "",
            shortcut: "",
        }
    }

    fn initial_params(doc: &Document) -> BreakParams {
        BreakParams {
            group: selected_text_group(doc).unwrap_or(0),
        }
    }

    #[expect(clippy::cast_precision_loss, reason = "line counts are small")]
    fn run(doc: &Document, p: &BreakParams) -> Result<Change, CommandError> {
        let spec = doc.texts.get(&p.group).ok_or_else(|| {
            CommandError::Failed("select a block of text to break into lines".into())
        })?;
        let text: Params = serde_json::from_value(spec.clone())
            .map_err(|e| CommandError::Failed(e.to_string()))?;
        if matches!(text.mode, Mode::TextInABox) {
            return Err(CommandError::Failed(
                "Text in a Box is one block: it has no lines of its own".into(),
            ));
        }
        let lines: Vec<&str> = text.text.lines().collect();
        if lines.len() < 2 {
            return Err(CommandError::Failed("the text is one line already".into()));
        }
        let face = font::load(&text.font, text.bold, text.italic, &text.text)
            .map_err(CommandError::Failed)?;
        let pitch = face.line_pitch * text.spacing.line_pitch * text.height / face.cap_height;
        let mut c = Change::builder(doc);
        for id in doc.group_members_of(p.group) {
            c.remove(id);
        }
        c.set_text(p.group, None);
        let mut selected = vec![];
        for (group, (row, line)) in (doc.next_group()..).zip(lines.iter().enumerate()) {
            let anchor =
                Point::from(text.anchor) + Point::new(0.0, (lines.len() - 1 - row) as f64 * pitch);
            let kerning = text
                .spacing
                .kerning
                .iter()
                .filter(|k| k.0 == row)
                .map(|k| (0, k.1, k.2))
                .collect();
            let one = Params {
                text: (*line).to_owned(),
                anchor: Xy::new(anchor.x, anchor.y),
                spacing: font::Spacing {
                    kerning,
                    line_pitch: text.spacing.line_pitch,
                },
                replaces: None,
                ..text.clone()
            };
            let paths = face.layout_spaced(line, one.height, one.align, anchor, &one.spacing);
            let ids: Vec<_> = paths.into_iter().map(|path| c.add(path)).collect();
            for &id in &ids {
                c.set_group(id, Some(group));
            }
            c.set_text(
                group,
                Some(serde_json::to_value(one).expect("params serialise")),
            );
            selected.extend(ids);
        }
        c.select(selected);
        Ok(c.finish())
    }
}

impl Interactive for DrawText {
    /// A click places the anchor.
    fn from_drag(drag: Drag, current: &Params) -> Params {
        Params {
            anchor: Xy::new(drag.start.x, drag.start.y),
            ..current.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::document::EntityId;
    use cut_craft_core::geometry::Point;
    use cut_craft_core::testing::{after, rejection};

    cut_craft_core::contract_tests!(DrawText);

    fn boxes(text: &str) -> Params {
        Params {
            text: text.into(),
            font: "Test Boxes".into(),
            ..Params::default()
        }
    }

    #[test]
    fn text_is_one_selected_group_of_the_asked_height_at_the_anchor() {
        let doc = after::<DrawText>(&boxes("HI"));
        let ids: Vec<EntityId> = doc.selection.iter().copied().collect();
        assert_eq!(ids.len(), 2, "one box per letter");
        assert_eq!(doc.group_members(ids[0]).len(), 2);
        let b = ids
            .iter()
            .map(|id| doc.vectors[id].bounds())
            .reduce(Rect::union)
            .unwrap();
        assert!((b.min.x - 10.0).abs() < 1e-9 && (b.min.y - 10.0).abs() < 1e-9);
        assert!((b.size().y - 10.0).abs() < 1e-9);
    }

    #[test]
    fn selected_text_opens_its_settings_and_is_replaced_when_edited() {
        let mut doc = after::<DrawText>(&boxes("HI"));
        let p = DrawText::initial_params(&doc);
        assert_eq!(p.text, "HI");
        let group = p.replaces.expect("editing that text");
        let edited = Params {
            text: "HIH".into(),
            ..p
        };
        cut_craft_core::testing::run_on::<DrawText>(&mut doc, &edited);
        let members: Vec<_> = doc
            .vector_groups
            .values()
            .filter(|g| **g == group)
            .collect();
        assert_eq!(
            members.len(),
            3,
            "the old letters went, the new ones joined the group"
        );
        assert_eq!(
            doc.vectors.len(),
            cut_craft_core::testing::sample_document().vectors.len() + 3
        );
        assert_eq!(doc.texts[&group]["text"], "HIH");
        // deleting the letters takes the text's settings with them, undoably
        let mut c = Change::builder(&doc);
        for id in doc.group_members_of(group) {
            c.remove(id);
        }
        let delete = c.finish();
        let kept = doc.clone();
        doc.apply(&delete).unwrap();
        assert!(
            !doc.texts.contains_key(&group),
            "no settings for vanished text"
        );
        doc.apply(&delete.inverse()).unwrap();
        assert_eq!(doc, kept);
    }

    #[test]
    fn text_in_a_box_fits_the_box_inside_its_margin() {
        let p = Params {
            mode: Mode::TextInABox,
            box_size: Xy::new(100.0, 50.0),
            anchor: Xy::new(0.0, 0.0),
            ..boxes("HI")
        };
        let doc = after::<DrawText>(&p);
        let b = doc
            .selection
            .iter()
            .map(|id| doc.vectors[id].bounds())
            .reduce(Rect::union)
            .unwrap();
        // "HI" is 18 wide by 10 tall: the width fills 80 % of 100, centred in the box
        assert!((b.size().x - 80.0).abs() < 1e-6, "{b:?}");
        assert!((b.center() - cut_craft_core::geometry::Point::new(50.0, 25.0)).length() < 1e-6);
    }

    #[test]
    fn empty_text_unknown_fonts_and_no_height_are_refused() {
        assert!(rejection::<DrawText>(&boxes(" ")).contains("text"));
        let p = Params {
            font: "No Such Font Anywhere".into(),
            ..boxes("H")
        };
        assert!(rejection::<DrawText>(&p).contains("No Such Font"));
        let p = Params {
            height: 0.0,
            ..boxes("H")
        };
        assert!(rejection::<DrawText>(&p).contains("height"));
    }

    #[test]
    fn a_click_moves_only_the_anchor() {
        let at = Point::new(30.0, 40.0);
        let drag = Drag {
            start: at,
            end: at,
            from_center: false,
            constrain: false,
        };
        let p = DrawText::from_drag(drag, &boxes("H"));
        assert_eq!((p.anchor.x, p.anchor.y), (30.0, 40.0));
        assert_eq!(p.text, "H");
    }

    fn two_lines() -> Document {
        after::<DrawText>(&boxes("HI\nHI"))
    }

    mod break_block {
        use super::*;
        cut_craft_core::contract_tests!(BreakTextIntoLines, two_lines);
    }

    #[test]
    fn a_click_falls_between_two_letters_or_two_lines_and_kerning_moves_the_letter_after() {
        let p = boxes("HI\nHI");
        // the boxes font: each letter 8 wide, advanced 9, so the gap sits at x = 19 from
        // the anchor at 10
        let letters = gap_at(&p, Point::new(19.0, 15.0), 1.0).unwrap();
        assert_eq!(letters, Some(Gap::Letters { line: 1, index: 0 }));
        let lines = gap_at(&p, Point::new(15.0, 22.0), 1.0).unwrap();
        assert_eq!(lines, Some(Gap::Lines));
        assert_eq!(gap_at(&p, Point::new(60.0, 60.0), 1.0).unwrap(), None);
        assert_eq!(letter_gaps(&p).unwrap(), [(0, 0), (1, 0)]);
        let mut kerned = p.clone();
        kerned.spacing.kern(1, 0, 4.0);
        let plain = after::<DrawText>(&p);
        let wide = after::<DrawText>(&kerned);
        let width = |doc: &Document| {
            doc.selection
                .iter()
                .map(|id| doc.vectors[id].bounds())
                .reduce(Rect::union)
                .unwrap()
                .size()
                .x
        };
        assert!((width(&wide) - width(&plain) - 4.0).abs() < 1e-9);
    }

    #[test]
    fn breaking_a_block_into_lines_leaves_a_text_per_line_where_each_line_was() {
        let mut doc = two_lines();
        let group = selected_text_group(&doc).unwrap();
        let was: Vec<Rect> = doc
            .selection
            .iter()
            .map(|id| doc.vectors[id].bounds())
            .collect();
        let p = BreakTextIntoLines::initial_params(&doc);
        assert_eq!(p.group, group);
        cut_craft_core::testing::run_on::<BreakTextIntoLines>(&mut doc, &p);
        assert!(!doc.texts.contains_key(&group), "the block is gone");
        assert_eq!(doc.texts.len(), 2, "a text per line");
        let mut now: Vec<Rect> = doc
            .selection
            .iter()
            .map(|id| doc.vectors[id].bounds())
            .collect();
        let mut was = was;
        let key = |r: &Rect| (r.min.y, r.min.x);
        was.sort_by(|a, b| key(a).partial_cmp(&key(b)).unwrap());
        now.sort_by(|a, b| key(a).partial_cmp(&key(b)).unwrap());
        assert_eq!(now.len(), was.len());
        for (a, b) in now.iter().zip(&was) {
            assert!((a.min - b.min).length() < 1e-9, "{a:?} vs {b:?}");
        }
        let texts: Vec<&str> = doc
            .texts
            .values()
            .filter_map(|t| t["text"].as_str())
            .collect();
        assert_eq!(texts, ["HI", "HI"]);
        let one_line = after::<DrawText>(&boxes("HI"));
        let p = BreakTextIntoLines::initial_params(&one_line);
        assert!(
            BreakTextIntoLines::run(&one_line, &p)
                .unwrap_err()
                .to_string()
                .contains("one line")
        );
    }
}
