//! Create Component from Bitmap: the newest picture as a relief, its tones as heights.

use cut_craft_core::component::{Combine, Component};
use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct CreateComponentFromBitmap;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
    /// Height added by the lightest (or darkest) parts
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub max_height: f64,
    /// Height under the whole picture
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub base_height: f64,
    /// White is high; otherwise black is
    pub white_high: bool,
    pub combine: Combine,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Bitmap".into(),
            max_height: 5.0,
            base_height: 0.0,
            white_high: true,
            combine: Combine::Add,
        }
    }
}

impl Command for CreateComponentFromBitmap {
    const ID: &'static str = "create_component_from_bitmap";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Create Component from Bitmap",
            group: "Modeling",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("max height", p.max_height)?;
        // ponytail: bitmaps cannot be selected yet, so the newest one is used
        let Some(bitmap) = doc.bitmaps.values().next_back() else {
            return Err(CommandError::Failed("import the picture first".into()));
        };
        let relief = Component::from_fn(
            &p.name,
            p.combine,
            bitmap.rect,
            doc.job.model_cell(),
            |at| {
                bitmap.darkness(at).map(|dark| {
                    let tone = if p.white_high { 1.0 - dark } else { dark };
                    p.base_height + p.max_height * tone
                })
            },
        );
        let mut c = Change::builder(doc);
        c.add_component(relief);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::bitmap::Bitmap;
    use cut_craft_core::geometry::{Point, Rect};

    /// Black on the left half of a 20 x 10 mm picture, white on the right.
    fn with_picture() -> Document {
        let mut doc = Document::default();
        let mut c = Change::builder(&doc);
        c.add_bitmap(Bitmap {
            name: "half".into(),
            columns: 2,
            rows: 1,
            grey: vec![0, 255],
            rect: Rect::new(Point::ZERO, Point::new(20.0, 10.0)),
        });
        let change = c.finish();
        doc.apply(&change).unwrap();
        doc
    }

    cut_craft_core::contract_tests!(CreateComponentFromBitmap, with_picture);

    #[test]
    fn white_is_high_unless_asked_otherwise() {
        let mut doc = with_picture();
        let change = CreateComponentFromBitmap::run(
            &doc,
            &Params {
                base_height: 1.0,
                ..Params::default()
            },
        )
        .unwrap();
        doc.apply(&change).unwrap();
        let relief = doc.components.values().next().unwrap();
        assert!(
            (relief.height_at(Point::new(1.0, 5.0)).unwrap() - 1.0).abs() < 1e-9,
            "black: the base"
        );
        assert!(
            (relief.height_at(Point::new(19.0, 5.0)).unwrap() - 6.0).abs() < 1e-9,
            "white: base plus max"
        );
    }
}
