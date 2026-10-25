//! Draw Text: the words as vectors in a TrueType or single-line font, grouped so they pick
//! and move as one, as Aspire's text objects do. Ungroup is Convert Text to Curves.

use cut_craft_core::document::Document;
use cut_craft_core::font::{self, Align};
use cut_craft_core::params::Xy;
use cut_craft_core::{Change, Command, CommandError, Drag, Interactive, Meta, ensure_positive};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};

pub struct DrawText;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
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

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("text height", p.height)?;
        let font = font::load(&p.font, p.bold, p.italic, &p.text).map_err(CommandError::Failed)?;
        let paths = font.layout(&p.text, p.height, p.align, p.anchor.into());
        if paths.is_empty() {
            return Err(CommandError::InvalidParams(
                "enter some text the font can draw".into(),
            ));
        }
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
    use cut_craft_core::geometry::{Point, Rect};
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
