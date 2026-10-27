//! Crop Bitmap: the selected picture cleared to white outside the selected closed vectors
//! and trimmed to them (Aspire's help: Crop Bitmap). Several vectors keep what is inside any
//! of them, a vector inside another making a hole.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Point, Rect, inside};
use cut_craft_core::{Change, Command, CommandError, Meta, NoParams};

pub struct CropBitmap;

/// The grey level outside the vectors.
const CLEARED: u8 = 255;

impl Command for CropBitmap {
    const ID: &'static str = "crop_bitmap";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Crop Bitmap",
            group: "Edit Objects",
            icon: "drawing/edit_objects/crop_bitmap.png",
            shortcut: "",
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "pixel counts are small and the range is clamped to the picture"
    )]
    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        let Some((&id, bitmap)) = doc
            .selected_bitmaps
            .iter()
            .find_map(|id| doc.bitmaps.get_key_value(id))
        else {
            return Err(CommandError::Failed(
                "select the picture to crop first".into(),
            ));
        };
        let rings: Vec<Vec<Point>> = doc
            .selection
            .iter()
            .filter_map(|id| doc.vectors.get(id))
            .filter(|p| p.closed)
            .map(|p| p.flatten(cut_craft_core::polygon::TOLERANCE))
            .collect();
        let Some(bounds) = rings
            .iter()
            .flatten()
            .map(|&p| Rect::new(p, p))
            .reduce(Rect::union)
        else {
            return Err(CommandError::Failed(
                "select closed vectors to crop the picture to".into(),
            ));
        };
        let r = bitmap.rect;
        let pixel = r.size() / Point::new(bitmap.columns as f64, bitmap.rows as f64);
        // the pixels the vectors' bounds cover; rows count from the top
        let span = |lo: f64, hi: f64, n: usize| {
            let first = (lo.floor().max(0.0) as usize).min(n);
            let last = (hi.ceil().max(0.0) as usize).min(n);
            first..last
        };
        let columns = span(
            (bounds.min.x - r.min.x) / pixel.x,
            (bounds.max.x - r.min.x) / pixel.x,
            bitmap.columns,
        );
        let rows = span(
            (r.max.y - bounds.max.y) / pixel.y,
            (r.max.y - bounds.min.y) / pixel.y,
            bitmap.rows,
        );
        if columns.is_empty() || rows.is_empty() {
            return Err(CommandError::Failed(
                "the vectors do not cover the picture".into(),
            ));
        }
        let mut grey = Vec::with_capacity(columns.len() * rows.len());
        // a depth map is cut with the picture
        let deep = bitmap.depth.len() == bitmap.grey.len();
        let mut depth = Vec::new();
        for row in rows.clone() {
            for column in columns.clone() {
                let centre = Point::new(
                    r.min.x + (column as f64 + 0.5) * pixel.x,
                    r.max.y - (row as f64 + 0.5) * pixel.y,
                );
                let at = row * bitmap.columns + column;
                let kept = inside(&rings, centre);
                grey.push(if kept { bitmap.grey[at] } else { CLEARED });
                if deep {
                    depth.push(if kept {
                        bitmap.depth[at]
                    } else {
                        f32::from(CLEARED) / 255.0
                    });
                }
            }
        }
        let rect = Rect::new(
            Point::new(
                r.min.x + columns.start as f64 * pixel.x,
                r.max.y - rows.end as f64 * pixel.y,
            ),
            Point::new(
                r.min.x + columns.end as f64 * pixel.x,
                r.max.y - rows.start as f64 * pixel.y,
            ),
        );
        let mut c = Change::builder(doc);
        c.replace_bitmap(
            id,
            cut_craft_core::bitmap::Bitmap {
                columns: columns.len(),
                rows: rows.len(),
                grey,
                rect,
                depth,
                ..bitmap.clone()
            },
        );
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::bitmap::Bitmap;
    use cut_craft_core::document::EntityId;
    use cut_craft_core::testing::{rejection, run_on, square};

    /// A black 10 × 10 picture of 1 mm pixels at the origin, selected, with a selected
    /// 4 mm square over pixels 2 to 6 across and up.
    fn cropping() -> Document {
        let mut doc = Document::default();
        let mut c = Change::builder(&doc);
        let picture = c.add_bitmap(Bitmap {
            name: "black".into(),
            columns: 10,
            rows: 10,
            grey: vec![0; 100],
            rect: Rect::new(Point::ZERO, Point::new(10.0, 10.0)),
            depth: Vec::new(),
        });
        let frame = c.add(square(2.0, 2.0, 4.0));
        c.select([frame]);
        doc.apply(&c.finish()).unwrap();
        doc.selected_bitmaps = [picture].into();
        doc
    }

    cut_craft_core::contract_tests!(CropBitmap, cropping);

    #[test]
    fn keeps_only_what_is_inside_trimmed_to_it() {
        let mut doc = cropping();
        run_on::<CropBitmap>(&mut doc, &NoParams {});
        let b = doc.bitmaps.values().next().unwrap();
        assert_eq!((b.columns, b.rows), (4, 4));
        assert_eq!(
            b.rect,
            Rect::new(Point::new(2.0, 2.0), Point::new(6.0, 6.0))
        );
        assert!(
            b.grey.iter().all(|&g| g == 0),
            "all of it inside the square"
        );
    }

    #[test]
    fn a_round_vector_clears_the_corners() {
        let mut doc = cropping();
        let circle = cut_craft_core::geometry::Path::circle(Point::new(5.0, 5.0), 3.0);
        let mut c = Change::builder(&doc);
        let id = c.add(circle);
        c.select([id]);
        doc.apply(&c.finish()).unwrap();
        run_on::<CropBitmap>(&mut doc, &NoParams {});
        let b = &doc.bitmaps[&EntityId(1)];
        assert_eq!((b.columns, b.rows), (6, 6));
        assert_eq!(b.grey[0], CLEARED, "a corner is outside the circle");
        assert_eq!(b.grey[2 * 6 + 2], 0, "the middle is kept");
    }

    #[test]
    fn needs_a_picture_and_closed_vectors() {
        assert!(rejection::<CropBitmap>(&NoParams {}).contains("picture"));
        let mut doc = cropping();
        doc.selection = cut_craft_core::selection::Selection::new();
        assert!(
            CropBitmap::run(&doc, &NoParams {})
                .unwrap_err()
                .to_string()
                .contains("closed vectors")
        );
    }
}
