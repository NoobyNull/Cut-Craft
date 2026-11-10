//! Draw Text: the words as vectors in a TrueType or single-line font, grouped so they pick
//! and move as one, as Aspire's text objects do. Ungroup is Convert Text to Curves.

use cut_craft_core::document::Document;
use cut_craft_core::font::{self, Align};
use cut_craft_core::geometry::{Path, Point, Rect};
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

/// Text in a Box: how text that fits the box's width fills the room above and below it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VerticalStretch {
    #[default]
    NoVerticalStretch,
    /// The lines move apart to fill the height
    StretchLineSpaceToFit,
    /// The letters grow taller to fill the height
    StretchCharactersToFit,
}

/// Text in a Box: how text that fits the box's height fills the room at its sides.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HorizontalStretch {
    #[default]
    NoHorizontalStretch,
    /// The spaces between words grow
    StretchSpacesBetweenWords,
    /// The room between letters grows
    StretchKerning,
    /// The letters grow wider
    StretchCharacterSize,
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
    #[schemars(
        title = "Bounding box dimensions",
        extend("x-show-when" = {"mode": "text_in_a_box"})
    )]
    pub box_size: Xy,
    /// Text in a Box: the border left and right
    #[serde(default)]
    #[schemars(
        title = "Margin size",
        extend("x-widget" = "row", "x-show-when" = {"mode": "text_in_a_box"})
    )]
    pub margin: Margin,
    #[serde(default)]
    #[schemars(extend("x-show-when" = {"mode": "text_in_a_box"}))]
    pub vertical_stretch: VerticalStretch,
    #[serde(default)]
    #[schemars(extend("x-show-when" = {"mode": "text_in_a_box"}))]
    pub horizontal_stretch: HorizontalStretch,
    /// Text: the line bent into an arc, the angle the arc turns through; above zero it bends
    /// up, below zero down. Drag the green handles above and below the text to set it
    #[serde(default)]
    #[schemars(
        title = "Arc angle",
        extend("x-unit" = "°", "x-show-when" = {"mode": "text"}),
        range(min = -350, max = 350)
    )]
    pub arc: f64,
    /// Text on an arc: the arc turned about its centre, anticlockwise
    #[serde(default)]
    #[schemars(
        title = "Arc rotation",
        extend("x-unit" = "°", "x-show-when" = {"mode": "text"})
    )]
    pub arc_rotation: f64,
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
            vertical_stretch: VerticalStretch::NoVerticalStretch,
            horizontal_stretch: HorizontalStretch::NoHorizontalStretch,
            arc: 0.0,
            arc_rotation: 0.0,
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
            Mode::Text if p.arc != 0.0 => arched(&font, p),
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

/// The text scaled to fit the box (less the margin's share of its width), centred in it,
/// then stretched into the room left over as the stretch settings ask.
fn boxed(
    font: &font::Font,
    p: &Params,
) -> Result<Vec<cut_craft_core::geometry::Path>, CommandError> {
    let (w, h) = (p.box_size.x, p.box_size.y);
    ensure_positive("box width", w)?;
    ensure_positive("box height", h)?;
    let share = match p.margin {
        Margin::None => 1.0,
        Margin::Normal => 0.8,
        Margin::Wide => 0.6,
    };
    // laid out 1 unit tall, then scaled
    let lay =
        |spacing: &font::Spacing| font.layout_spaced(&p.text, 1.0, p.align, Point::ZERO, spacing);
    let bounds = |paths: &[Path]| paths.iter().map(Path::bounds).reduce(Rect::union);
    let mut spacing = font::Spacing::default();
    let Some(b) = bounds(&lay(&spacing)) else {
        return Ok(vec![]);
    };
    let room = Point::new(w * share, h);
    let scale = (room.x / b.size().x).min(room.y / b.size().y);
    // what the box has left over, in the 1-unit layout's units
    let target = room / scale;
    let mut stretch = Point::ONE;
    let wide = room.x / b.size().x <= room.y / b.size().y;
    if wide {
        let lines = p.text.lines().count();
        match p.vertical_stretch {
            VerticalStretch::StretchLineSpaceToFit if lines > 1 => {
                #[expect(clippy::cast_precision_loss, reason = "line counts are small")]
                let gaps = (lines - 1) as f64 * font.line_pitch / font.cap_height;
                spacing.line_pitch = 1.0 + (target.y - b.size().y) / gaps;
            }
            // one line has no line space to stretch
            VerticalStretch::NoVerticalStretch | VerticalStretch::StretchLineSpaceToFit => {}
            VerticalStretch::StretchCharactersToFit => stretch.y = target.y / b.size().y,
        }
    } else {
        match p.horizontal_stretch {
            HorizontalStretch::NoHorizontalStretch => {}
            HorizontalStretch::StretchCharacterSize => stretch.x = target.x / b.size().x,
            HorizontalStretch::StretchSpacesBetweenWords | HorizontalStretch::StretchKerning => {
                let words = p.horizontal_stretch == HorizontalStretch::StretchSpacesBetweenWords;
                spacing.kerning = widened(font, p, target.x, words);
            }
        }
    }
    let natural = lay(&spacing);
    let Some(b) = bounds(&natural) else {
        return Ok(vec![]);
    };
    let anchor: Point = p.anchor.into();
    let centre = anchor + Point::new(w, h) / 2.0;
    let place = DAffine2::from_translation(centre)
        * DAffine2::from_scale(stretch * scale)
        * DAffine2::from_translation(-b.center());
    Ok(natural.iter().map(|q| q.transformed(&place)).collect())
}

/// Extra room, 1-unit layout, that brings each line of `p`'s text to `width`: shared
/// between the spaces of the line (`words`) or between all its letters.
fn widened(font: &font::Font, p: &Params, width: f64, words: bool) -> Vec<(usize, usize, f64)> {
    let placed = font.placed(
        &p.text,
        1.0,
        Align::Left,
        Point::ZERO,
        &font::Spacing::default(),
    );
    let lines: Vec<&str> = p.text.lines().collect();
    let mut out = vec![];
    for (row, line) in lines.iter().enumerate() {
        let glyphs: Vec<_> = placed.iter().filter(|(g, _)| g.line == row).collect();
        let (Some(first), Some(last)) = (glyphs.first(), glyphs.last()) else {
            continue;
        };
        let chars: Vec<char> = line.chars().collect();
        // the gaps after these letters take the room (never after the last letter)
        let gaps: Vec<usize> = glyphs[..glyphs.len() - 1]
            .iter()
            .map(|(g, _)| g.index)
            .filter(|&i| !words || chars[i] == ' ')
            .collect();
        // the line's ink, not its advances, is what fills the width
        let ink = |(at, glyph): &(font::Placed, &font::Glyph)| {
            glyph
                .paths
                .iter()
                .map(Path::bounds)
                .reduce(Rect::union)
                .map(|b| {
                    (
                        at.left + b.min.x / font.cap_height,
                        at.left + b.max.x / font.cap_height,
                    )
                })
        };
        let (Some((left, _)), Some((_, right))) = (ink(first), ink(last)) else {
            continue;
        };
        let short = width - (right - left);
        if gaps.is_empty() || short <= 0.0 {
            continue;
        }
        #[expect(clippy::cast_precision_loss, reason = "letter counts are small")]
        let each = short / gaps.len() as f64;
        out.extend(gaps.into_iter().map(|i| (row, i, each)));
    }
    out
}

/// Which way an arc bends the text: up (the ends drop) or down (the ends rise).
fn bend_sign(arc: f64) -> f64 {
    if arc < 0.0 { -1.0 } else { 1.0 }
}

/// The middle of the unbent text: the point that stays put as it bends, and the box it fills.
fn unbent(font: &font::Font, p: &Params) -> Option<Rect> {
    font.layout_spaced(&p.text, p.height, p.align, p.anchor.into(), &p.spacing)
        .iter()
        .map(Path::bounds)
        .reduce(Rect::union)
}

/// The arc's centre and radius for the text `p` (arc not zero): the text's middle stays
/// where it was, its width runs along the arc.
fn arc_of(b: Rect, arc: f64) -> (Point, f64) {
    let radius = b.size().x / arc.to_radians().abs();
    let middle = b.center();
    (middle - Point::new(0.0, bend_sign(arc) * radius), radius)
}

/// The text bent into an arc: each letter kept whole, its middle moved onto a circle round
/// the arc's centre and turned to face out from it (bent down: in towards it), the text's
/// middle staying put; then the whole turned about the centre by the arc's rotation.
fn arched(font: &font::Font, p: &Params) -> Vec<Path> {
    let Some(b) = unbent(font, p) else {
        return vec![];
    };
    let (centre, radius) = arc_of(b, p.arc);
    let sign = bend_sign(p.arc);
    let middle = b.center();
    let scale = p.height / font.cap_height;
    let turn = DAffine2::from_translation(centre)
        * DAffine2::from_angle(p.arc_rotation.to_radians())
        * DAffine2::from_translation(-centre);
    font.placed(&p.text, p.height, p.align, p.anchor.into(), &p.spacing)
        .into_iter()
        .flat_map(|(at, glyph)| {
            let here = Point::new(f64::midpoint(at.left, at.right), at.baseline);
            let angle = (here.x - middle.x) / radius;
            let r = radius + sign * (here.y - middle.y);
            let there = centre + Point::new(angle.sin(), sign * angle.cos()) * r;
            let place = turn
                * DAffine2::from_translation(there)
                * DAffine2::from_angle(-sign * angle)
                * DAffine2::from_translation(-here)
                * DAffine2::from_translation(Point::new(at.left, at.baseline))
                * DAffine2::from_scale(cut_craft_core::glam::DVec2::splat(scale));
            glyph
                .paths
                .iter()
                .map(move |q| q.transformed(&place))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The two green handles that bend the text `p` into an arc: above and below the middle of
/// the text as it is before it bends, a quarter of the text height clear of it. None for
/// Text in a Box or text the font cannot draw.
#[must_use]
pub fn arc_handles(p: &Params) -> Option<[Point; 2]> {
    if p.mode != Mode::Text {
        return None;
    }
    let face = font::load(&p.font, p.bold, p.italic, &p.text).ok()?;
    let b = unbent(&face, p)?;
    let clear = p.height / 4.0;
    let x = b.center().x;
    Some([
        Point::new(x, b.max.y + clear),
        Point::new(x, b.min.y - clear),
    ])
}

/// How far the middle of the text `p` rises above a line through its ends: what
/// [`arc_from_rise`] turns back into its arc.
#[must_use]
pub fn arc_rise(p: &Params) -> f64 {
    let Some(width) = font::load(&p.font, p.bold, p.italic, &p.text)
        .ok()
        .and_then(|face| unbent(&face, p))
        .map(|b| b.size().x)
    else {
        return 0.0;
    };
    let angle = p.arc.to_radians().abs();
    if angle == 0.0 {
        return 0.0;
    }
    width / angle * (1.0 - (angle / 2.0).cos()) * bend_sign(p.arc)
}

/// The arc angle a drag of a green handle by `rise` (up positive) gives the text `p`: the
/// middle of the text then sits `rise` above (or below) a line through its ends, so the
/// handle follows the pointer. At most a full circle less a little.
#[must_use]
pub fn arc_from_rise(p: &Params, rise: f64) -> f64 {
    const MOST: f64 = 350.0;
    let Some(width) = font::load(&p.font, p.bold, p.italic, &p.text)
        .ok()
        .and_then(|face| unbent(&face, p))
        .map(|b| b.size().x)
        .filter(|w| *w > 0.0)
    else {
        return 0.0;
    };
    // the rise of an arc of length `width` through `angle` (radians): r (1 - cos(a/2))
    let rise_of = |angle: f64| width / angle * (1.0 - (angle / 2.0).cos());
    let want = rise.abs();
    let (mut lo, mut hi) = (1e-9, MOST.to_radians());
    if want >= rise_of(hi) {
        return MOST * rise.signum();
    }
    for _ in 0..60 {
        let mid = f64::midpoint(lo, hi);
        if rise_of(mid) < want {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo.to_degrees() * rise.signum() * 100.0).round() / 100.0
}

/// The first selected group that is text (plain groups may come before it).
#[must_use]
pub fn selected_text_group(doc: &Document) -> Option<u64> {
    doc.selection
        .iter()
        .filter_map(|id| doc.vector_groups.get(id))
        .find(|group| {
            doc.texts
                .get(group)
                .is_some_and(|t| t.get("text").is_some())
        })
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
    // ponytail: no kerning clicks on arched text; flatten it, kern, bend it again
    if p.arc != 0.0 {
        return Ok(None);
    }
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
                "Text in a Box is one block: it has no lines of its own. Select text drawn as lines".into(),
            ));
        }
        let lines: Vec<&str> = text.text.lines().collect();
        if lines.len() < 2 {
            return Err(CommandError::Failed(
                "the text is one line already. Select text with more than one line".into(),
            ));
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

    fn bounds_of(doc: &Document) -> Rect {
        doc.selection
            .iter()
            .map(|id| doc.vectors[id].bounds())
            .reduce(Rect::union)
            .unwrap()
    }

    fn in_a_box(text: &str) -> Params {
        Params {
            mode: Mode::TextInABox,
            box_size: Xy::new(100.0, 50.0),
            margin: Margin::None,
            ..boxes(text)
        }
    }

    #[test]
    fn stretching_characters_fills_the_box_height() {
        // "HI" fits the width (18 by 10 into 100 by 80): 100 mm wide, 55.6 tall
        let tall = |v| Params {
            box_size: Xy::new(100.0, 80.0),
            vertical_stretch: v,
            ..in_a_box("HI")
        };
        let plain = bounds_of(&after::<DrawText>(&tall(
            VerticalStretch::NoVerticalStretch,
        )));
        assert!((plain.size().x - 100.0).abs() < 1e-6, "{plain:?}");
        assert!(plain.size().y < 79.0);
        let b = bounds_of(&after::<DrawText>(&tall(
            VerticalStretch::StretchCharactersToFit,
        )));
        assert!(
            (b.size().x - 100.0).abs() < 1e-6 && (b.size().y - 80.0).abs() < 1e-6,
            "{b:?}"
        );
        assert!((b.min.y - 10.0).abs() < 1e-6, "in the box: {b:?}");
    }

    #[test]
    fn stretching_line_space_moves_the_lines_apart_to_the_box_height() {
        let p = Params {
            vertical_stretch: VerticalStretch::StretchLineSpaceToFit,
            ..in_a_box("HI\nHI")
        };
        let doc = after::<DrawText>(&p);
        let b = bounds_of(&doc);
        assert!((b.size().y - 50.0).abs() < 1e-6, "{b:?}");
        // the letters keep their shape: each box as wide as it is tall less 2/10
        let one = doc.vectors[doc.selection.iter().next().unwrap()].bounds();
        assert!((one.size().x / one.size().y - 0.8).abs() < 1e-6, "{one:?}");
    }

    #[test]
    fn horizontal_stretch_fills_the_box_width_three_ways() {
        // "H H" in a flat box fits its height: room at the sides
        let flat = |h| Params {
            box_size: Xy::new(200.0, 10.0),
            horizontal_stretch: h,
            ..in_a_box("H H")
        };
        let plain = bounds_of(&after::<DrawText>(&flat(
            HorizontalStretch::NoHorizontalStretch,
        )));
        assert!(plain.size().x < 100.0, "{plain:?}");
        for h in [
            HorizontalStretch::StretchSpacesBetweenWords,
            HorizontalStretch::StretchKerning,
            HorizontalStretch::StretchCharacterSize,
        ] {
            let doc = after::<DrawText>(&flat(h));
            let b = bounds_of(&doc);
            assert!((b.size().x - 200.0).abs() < 1e-6, "{h:?}: {b:?}");
            assert!((b.size().y - 10.0).abs() < 1e-6, "{h:?}: {b:?}");
            let wide = doc
                .selection
                .iter()
                .map(|id| doc.vectors[id].bounds().size().x)
                .fold(0.0, f64::max);
            // only the character size stretch widens the letters (8 wide at 10 tall)
            assert_eq!(
                wide > 8.5,
                h == HorizontalStretch::StretchCharacterSize,
                "{h:?}"
            );
        }
    }

    #[test]
    fn arched_text_bends_about_its_middle_and_back() {
        let flat = boxes("HIHIH");
        let up = Params {
            arc: 90.0,
            ..flat.clone()
        };
        let b0 = bounds_of(&after::<DrawText>(&flat));
        let doc = after::<DrawText>(&up);
        let b = bounds_of(&doc);
        // the middle letter stays where it was; the ends drop
        assert!((b.max.y - b0.max.y).abs() < 0.5, "{b0:?} {b:?}");
        assert!(b.min.y < b0.min.y - 3.0, "{b:?}");
        let down = bounds_of(&after::<DrawText>(&Params { arc: -90.0, ..flat }));
        assert!(down.max.y > b0.max.y + 3.0 && (down.min.y - b0.min.y).abs() < 0.5);
        // the arc is kept with the text: the form opens on it
        assert!((DrawText::initial_params(&doc).arc - 90.0).abs() < 1e-9);
    }

    #[test]
    fn a_handle_dragged_up_bends_the_text_so_its_middle_rises_that_far() {
        let p = boxes("HIHIH");
        assert_eq!(arc_from_rise(&p, 0.0), 0.0);
        let arc = arc_from_rise(&p, 5.0);
        assert!(arc > 0.0 && (arc_from_rise(&p, -5.0) + arc).abs() < 1e-9);
        // the letters' middles: the middle one 5 above the ends'
        let bent = Params { arc, ..p.clone() };
        let face = font::load("Test Boxes", false, false, "HIHIH").unwrap();
        let mids: Vec<Point> = arched(&face, &bent)
            .iter()
            .map(|q| q.bounds().center())
            .collect();
        let rise = mids[2].y - f64::midpoint(mids[0].y, mids[4].y);
        // the ends' middles sit inside the arc's ends: a little less than the drag
        assert!(rise > 3.0 && rise < 5.0, "{rise}");
        assert!((arc_rise(&bent) - 5.0).abs() < 1e-3);
        let [top, bottom] = arc_handles(&p).unwrap();
        assert!(top.y > 20.0 && bottom.y < 10.0 && (top.x - bottom.x).abs() < 1e-9);
        assert!(arc_handles(&in_a_box("H")).is_none());
    }

    #[test]
    fn arc_rotation_turns_the_arched_text_about_the_arc_centre() {
        let p = Params {
            arc: 60.0,
            arc_rotation: 180.0,
            ..boxes("HIHIH")
        };
        let b = bounds_of(&after::<DrawText>(&p));
        assert!(b.max.y < 10.0, "turned over below the centre: {b:?}");
    }
}
