//! Relief from Photo: a photo whose depth has been estimated (the Depth add-on fills the
//! picture's depth) shaped into a relief: the backdrop cut away, the subject lifted clear
//! of it, fine detail from the photo's shading added back and the noise smoothed.

pub mod shape;

use cut_craft_core::bitmap::Bitmap;
use cut_craft_core::component::{Combine, Component};
use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shape::{Grid, Shaping};

pub struct ReliefFromPhoto;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
    /// How different from the background a part of the photo must be to count as the
    /// subject. Raise it if bits of background are kept; 0 keeps the whole photo
    #[schemars(range(min = 0, max = 1))]
    pub background_cut_off: f64,
    /// Fine texture (hair, fur, scales) taken from the photo's light and shade. 0 for none
    #[schemars(range(min = 0, max = 2))]
    pub fine_detail: f64,
    /// Evens out speckle while keeping edges crisp. 0 for none
    #[schemars(range(min = 0, max = 50))]
    pub smoothness: u32,
    /// How high the nearest point stands
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub max_height: f64,
    /// Height of the subject's lowest point, so it stands clear of the background
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub lift: f64,
    pub combine: Combine,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Photo relief".into(),
            background_cut_off: 0.1,
            fine_detail: 0.5,
            smoothness: 4,
            max_height: 10.0,
            lift: 0.8,
            combine: Combine::Add,
        }
    }
}

impl Command for ReliefFromPhoto {
    const ID: &'static str = "relief_from_photo";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Relief from Photo",
            group: "Modeling",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("max height", p.max_height)?;
        if p.lift < 0.0 || p.lift >= p.max_height {
            return Err(CommandError::InvalidParams(
                "the lift must be at least 0 and below the max height".into(),
            ));
        }
        let Some(photo) = doc.chosen_bitmap() else {
            return Err(CommandError::Failed("import the photo first".into()));
        };
        if photo.depth.len() != photo.grey.len() || photo.grey.is_empty() {
            return Err(CommandError::Failed(
                "this photo has no depth yet: use Estimate Depth on it first".into(),
            ));
        }
        let shaped = shaped(photo, p);
        let relief = Component::from_fn(
            &p.name,
            p.combine,
            shaped.rect,
            doc.job.model_cell(),
            |at| shaped.tone(at).map(|tone| p.max_height * tone),
        );
        let mut c = Change::builder(doc);
        c.add_component(relief);
        Ok(c.finish())
    }
}

/// The photo with its depth shaped into heights 0 to 1, as a depth map.
#[expect(clippy::cast_possible_truncation, reason = "settings near 0..1")]
fn shaped(photo: &Bitmap, p: &Params) -> Bitmap {
    let (columns, rows) = (photo.columns, photo.rows);
    let light = Grid::new(
        columns,
        rows,
        photo.grey.iter().map(|&g| f32::from(g) / 255.0).collect(),
    );
    let depth = Grid::new(columns, rows, photo.depth.clone());
    let shaping = Shaping {
        cut_off: p.background_cut_off as f32,
        lift: (p.lift / p.max_height) as f32,
        detail: p.fine_detail as f32,
        radius: radius(columns.max(rows)),
        smoothing: p.smoothness as usize,
    };
    let heights = shape::heights(&light, &depth, &shaping);
    Bitmap {
        depth: heights.values,
        ..photo.clone()
    }
}

/// Detail finer than about 1% of the photo's long side is taken from its shading (12 px
/// on a 1000-px photo, as depth2stl found works), never under 2 px.
#[expect(clippy::cast_precision_loss, reason = "pixel counts")]
fn radius(long_side: usize) -> f32 {
    (long_side as f32 * 0.012).max(2.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::{Point, Rect};

    /// A 12 x 12 photo over 24 x 24 mm: a dark subject on a light background, its depth
    /// rising left to right.
    fn with_photo() -> Document {
        let mut doc = Document::default();
        let lo = 3..9;
        let grey = (0..144)
            .map(|i| {
                if lo.contains(&(i % 12)) && lo.contains(&(i / 12)) {
                    60
                } else {
                    240
                }
            })
            .collect();
        let mut c = Change::builder(&doc);
        c.add_bitmap(Bitmap {
            name: "photo".into(),
            columns: 12,
            rows: 12,
            grey,
            rect: Rect::new(Point::ZERO, Point::new(24.0, 24.0)),
            depth: (0..144_u16).map(|i| f32::from(i % 12)).collect(),
            rgb: Vec::new(),
        });
        doc.apply(&c.finish()).unwrap();
        doc
    }

    cut_craft_core::contract_tests!(ReliefFromPhoto, with_photo);

    fn relief(doc: &mut Document, params: &Params) -> Component {
        doc.apply(&ReliefFromPhoto::run(doc, params).unwrap())
            .unwrap();
        doc.components.values().next().unwrap().clone()
    }

    #[test]
    fn the_background_is_flat_and_the_subject_stands_clear_of_it() {
        let mut doc = with_photo();
        let params = Params {
            fine_detail: 0.0,
            smoothness: 0,
            max_height: 10.0,
            lift: 2.0,
            ..Params::default()
        };
        let r = relief(&mut doc, &params);
        let at = |x: f64, y: f64| r.height_at(Point::new(x, y)).unwrap_or(0.0);
        assert!(at(1.0, 1.0).abs() < 1e-6, "background");
        let (near_left, near_right) = (at(7.0, 12.0), at(17.0, 12.0));
        assert!(near_left >= 2.0 - 1e-6, "lifted: {near_left}");
        assert!(
            near_right > near_left && near_right <= 10.0 + 1e-6,
            "{near_left} {near_right}"
        );
    }

    #[test]
    fn a_photo_without_depth_says_what_to_do() {
        let mut doc = with_photo();
        let id = *doc.bitmaps.keys().next().unwrap();
        let mut plain = doc.bitmaps[&id].clone();
        plain.depth.clear();
        let mut c = Change::builder(&doc);
        c.replace_bitmap(id, plain);
        doc.apply(&c.finish()).unwrap();
        let Err(CommandError::Failed(message)) = ReliefFromPhoto::run(&doc, &Params::default())
        else {
            panic!("should fail");
        };
        assert!(message.contains("Estimate Depth"), "{message}");
    }

    #[test]
    fn the_lift_must_sit_below_the_top() {
        let doc = with_photo();
        let params = Params {
            lift: 10.0,
            max_height: 10.0,
            ..Params::default()
        };
        assert!(matches!(
            ReliefFromPhoto::run(&doc, &params),
            Err(CommandError::InvalidParams(_))
        ));
    }
}
