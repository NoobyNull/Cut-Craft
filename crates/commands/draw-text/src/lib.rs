//! Draw Text: the words as vectors in a TrueType or single-line font, grouped so they pick
//! and move as one, as Aspire's text objects do. Ungroup is Convert Text to Curves.

use cut_craft_core::document::Document;
use cut_craft_core::font::{self, Align};
use cut_craft_core::geometry::Rect;
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
            icon: "drawing/create_vectors/draw_text.svg",
            shortcut: "",
        }
    }

    /// On selected text, its own settings, so the form edits it.
    fn initial_params(doc: &Document) -> Params {
        doc.selection
            .iter()
            .find_map(|id| doc.vector_groups.get(id))
            .and_then(|group| {
                let spec = doc.texts.get(group)?;
                let p: Params = serde_json::from_value(spec.clone()).ok()?;
                Some(Params {
                    replaces: Some(*group),
                    ..p
                })
            })
            .unwrap_or_default()
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("text height", p.height)?;
        let font = font::load(&p.font, p.bold, p.italic, &p.text).map_err(CommandError::Failed)?;
        let paths = match p.mode {
            Mode::Text => font.layout(&p.text, p.height, p.align, p.anchor.into()),
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
            c.set_group(id, None);
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
}
