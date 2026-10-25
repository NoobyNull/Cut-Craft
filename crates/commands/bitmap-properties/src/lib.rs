//! Bitmap Properties: a picture's name, position and size on the job.

use cut_craft_core::bitmap::Bitmap;
use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::geometry::{Point, Rect};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct BitmapProperties;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// The bitmap to change
    pub bitmap: EntityId,
    pub name: String,
    /// Left edge
    #[schemars(extend("x-unit" = "mm"))]
    pub x: f64,
    /// Bottom edge
    #[schemars(extend("x-unit" = "mm"))]
    pub y: f64,
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub width: f64,
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub height: f64,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            bitmap: EntityId(0),
            name: "Bitmap".into(),
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        }
    }
}

impl Command for BitmapProperties {
    const ID: &'static str = "bitmap_properties";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Bitmap Properties",
            group: "Edit",
            icon: "",
            shortcut: "",
        }
    }

    fn initial_params(doc: &Document) -> Params {
        let chosen = doc
            .selected_bitmaps
            .iter()
            .rev()
            .copied()
            .find(|id| doc.bitmaps.contains_key(id))
            .or_else(|| doc.bitmaps.keys().next_back().copied());
        chosen.map_or_else(Params::default, |id| {
            let b = &doc.bitmaps[&id];
            let size = b.rect.size();
            Params {
                bitmap: id,
                name: b.name.clone(),
                x: b.rect.min.x,
                y: b.rect.min.y,
                width: size.x,
                height: size.y,
            }
        })
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("width", p.width)?;
        ensure_positive("height", p.height)?;
        let id = if doc.bitmaps.contains_key(&p.bitmap) {
            p.bitmap
        } else {
            *doc.bitmaps
                .keys()
                .next_back()
                .ok_or_else(|| CommandError::Failed("there are no bitmaps".into()))?
        };
        let min = Point::new(p.x, p.y);
        let after = Bitmap {
            name: p.name.clone(),
            rect: Rect::new(min, min + Point::new(p.width, p.height)),
            ..doc.bitmaps[&id].clone()
        };
        let mut c = Change::builder(doc);
        c.replace_bitmap(id, after);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_picture() -> Document {
        let mut doc = Document::default();
        let mut c = Change::builder(&doc);
        c.add_bitmap(Bitmap {
            name: "b".into(),
            columns: 2,
            rows: 1,
            grey: vec![0, 255],
            rect: Rect::new(Point::ZERO, Point::new(20.0, 10.0)),
        });
        let change = c.finish();
        doc.apply(&change).unwrap();
        doc
    }

    cut_craft_core::contract_tests!(BitmapProperties, with_picture);

    #[test]
    fn moves_and_resizes_the_picture() {
        let mut doc = with_picture();
        let p = Params {
            x: 5.0,
            y: 6.0,
            width: 40.0,
            ..BitmapProperties::initial_params(&doc)
        };
        assert!(
            (p.height - 10.0).abs() < 1e-12,
            "the form starts from the picture"
        );
        let change = BitmapProperties::run(&doc, &p).unwrap();
        doc.apply(&change).unwrap();
        let b = doc.bitmaps.values().next().unwrap();
        assert_eq!(
            b.rect,
            Rect::new(Point::new(5.0, 6.0), Point::new(45.0, 16.0))
        );
        assert_eq!(b.grey, [0, 255], "the pixels are untouched");
    }
}
