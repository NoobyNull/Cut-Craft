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

/// Where the heights come from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DepthFrom {
    /// The depth Estimate Depth worked out for the photo
    #[default]
    Estimated,
    /// The picture is already a depth map (white nearest), made elsewhere
    Picture,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Params {
    pub name: String,
    /// Heights from the estimated depth, or from the picture itself when it is a depth map
    pub depth_from: DepthFrom,
    /// Background by colour: how different from the photo's border a part must be to count
    /// as the subject. Raise it if bits of background are kept; 0 turns it off
    #[schemars(range(min = 0, max = 1))]
    pub background_cut_off: f64,
    /// Background by distance: everything farther back than this share of the way from
    /// the farthest point to the nearest is dropped. Raise it for a busy background; 0
    /// turns it off
    #[schemars(range(min = 0, max = 0.9))]
    pub depth_cut_off: f64,
    /// Near and far swapped, for a depth map drawn with black nearest
    pub invert: bool,
    /// Fine texture (hair, fur, scales) taken from the photo's light and shade. 0 for none
    #[schemars(range(min = 0, max = 2))]
    pub fine_detail: f64,
    /// Makes the relief's own small details stand out more. 0 for none
    #[schemars(range(min = 0, max = 2))]
    pub sharpen: f64,
    /// Evens out speckle while keeping edges crisp. 0 for none
    #[schemars(range(min = 0, max = 50))]
    pub smoothness: u32,
    /// How high the nearest point stands
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub max_height: f64,
    /// Height of the subject's lowest point, so it stands clear of the background
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub lift: f64,
    /// How far the subject's edge slopes down to the background instead of dropping
    /// straight down (easier to carve and to print). 0 for a straight drop
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub soft_edge: f64,
    /// A flat slab under the whole picture, for a plaque or a printed base. 0 for none
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub base_height: f64,
    /// Shows the relief as your bit will really carve it: pits and grooves too narrow for
    /// the bit fill in, and the foot of a steep wall slopes at the bit's taper
    #[schemars(extend("x-group" = "Carving bit"))]
    pub only_what_the_bit_can_carve: bool,
    /// The ball nose bit's tip diameter (1/64 inch is 0.397 mm)
    #[schemars(
        extend(
            "x-unit" = "mm",
            "x-group" = "Carving bit",
            "x-enable-when" = {"only_what_the_bit_can_carve": "true"}
        ),
        range(min = 0)
    )]
    pub bit_tip_diameter: f64,
    /// A tapered bit's angle on each side, from its listing; 0 for a straight ball nose
    #[schemars(
        extend(
            "x-unit" = "deg",
            "x-group" = "Carving bit",
            "x-enable-when" = {"only_what_the_bit_can_carve": "true"}
        ),
        range(min = 0, max = 45)
    )]
    pub bit_taper: f64,
    pub combine: Combine,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Photo relief".into(),
            depth_from: DepthFrom::Estimated,
            background_cut_off: 0.1,
            depth_cut_off: 0.1,
            invert: false,
            fine_detail: 0.5,
            sharpen: 0.0,
            smoothness: 4,
            max_height: 10.0,
            lift: 0.8,
            soft_edge: 0.0,
            base_height: 0.0,
            only_what_the_bit_can_carve: false,
            // a 1/64 inch tapered ball nose
            bit_tip_diameter: 0.397,
            bit_taper: 10.0,
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

    /// A grey picture with no depth estimated for it is taken to be a depth map already,
    /// so the form opens ready to carve it; a colour photo still asks for an estimate.
    fn initial_params(doc: &Document) -> Params {
        let depth_map = doc.chosen_bitmap().is_some_and(|photo| {
            !photo.grey.is_empty() && photo.rgb.is_empty() && photo.depth.is_empty()
        });
        Params {
            depth_from: if depth_map {
                DepthFrom::Picture
            } else {
                DepthFrom::Estimated
            },
            ..Params::default()
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("max height", p.max_height)?;
        if p.lift < 0.0 || p.lift >= p.max_height {
            return Err(CommandError::InvalidParams(
                "set the lift to 0 or more and below the max height".into(),
            ));
        }
        let Some(photo) = doc.chosen_bitmap() else {
            return Err(CommandError::Failed("import the photo first".into()));
        };
        if photo.grey.is_empty() {
            return Err(CommandError::Failed(
                "the picture is empty. Pick another picture".into(),
            ));
        }
        let estimated = photo.depth.len() == photo.grey.len();
        if p.depth_from == DepthFrom::Estimated && !estimated {
            return Err(CommandError::Failed(
                "this photo has no depth yet: use Estimate Depth on it first, or choose \
                 'Picture' under Depth from if the picture is already a depth map"
                    .into(),
            ));
        }
        if p.soft_edge < 0.0 || p.base_height < 0.0 {
            return Err(CommandError::InvalidParams(
                "set the soft edge and base to 0 or more".into(),
            ));
        }
        if p.only_what_the_bit_can_carve
            && !(p.bit_tip_diameter > 0.0 && (0.0..=45.0).contains(&p.bit_taper))
        {
            return Err(CommandError::InvalidParams(
                "set the bit's tip diameter above 0 and its taper from 0 to 45 degrees".into(),
            ));
        }
        let shaped = shaped(photo, p, doc.job.model_cell());
        let relief = Component::from_fn(
            &p.name,
            p.combine,
            shaped.rect,
            doc.job.model_cell(),
            |at| {
                shaped
                    .tone(at)
                    .map(|tone| p.base_height + p.max_height * tone)
            },
        );
        let mut c = Change::builder(doc);
        c.add_component(relief);
        Ok(c.finish())
    }
}

/// Whether the job's 3D model is fine enough to hold what a bit with a `tip`-wide ball can
/// cut. The model's points are `cell` apart, and `finest` apart at the highest Model
/// Resolution; a length is written by `length`, in the job's units. A bit cuts detail down
/// to about its tip's radius, so points farther apart than that lose it.
pub fn resolution_note(
    cell: f64,
    finest: f64,
    tip: f64,
    length: impl Fn(f64) -> String,
) -> Option<String> {
    let radius = tip / 2.0;
    if tip <= 0.0 || cell <= radius {
        return None;
    }
    let lost = format!(
        "This job's model points are {} apart, but this bit can cut detail down to about {}, \
         so the relief is coarser than the bit.",
        length(cell),
        length(radius)
    );
    Some(if finest < cell * 0.99 {
        let enough = if finest <= radius {
            ""
        } else {
            " A smaller job is finer still."
        };
        format!(
            "{lost} Model \u{25b8} Model Resolution \u{25b8} Highest puts them {} apart.{enough}",
            length(finest)
        )
    } else {
        format!("{lost} A smaller job puts them closer together.")
    })
}

/// The photo with its depth shaped into heights 0 to 1, as a depth map.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "settings near 0..1, and a soft edge of a few pixels"
)]
fn shaped(photo: &Bitmap, p: &Params, cell: f64) -> Bitmap {
    let light = Grid::new(
        photo.columns,
        photo.rows,
        photo.grey.iter().map(|&g| f32::from(g) / 255.0).collect(),
    );
    let depth = Grid::new(photo.columns, photo.rows, source(photo, p));
    // shaped at the relief's resolution: finer pixels than its cells only cost time
    let size = photo.rect.size();
    let (columns, rows) = (
        (size.x / cell).ceil().max(1.0) as usize,
        (size.y / cell).ceil().max(1.0) as usize,
    );
    let (light, depth) = (
        shape::shrunk(&light, columns, rows),
        shape::shrunk(&depth, columns, rows),
    );
    let (columns, rows) = (light.columns, light.rows);
    let pixels_per_mm = columns as f64 / size.x.max(f64::EPSILON);
    let shaping = Shaping {
        cut_off: p.background_cut_off as f32,
        far: p.depth_cut_off as f32,
        invert: p.invert,
        lift: (p.lift / p.max_height) as f32,
        detail: p.fine_detail as f32,
        sharpen: p.sharpen as f32,
        radius: radius(columns.max(rows)),
        smoothing: p.smoothness as usize,
        soft_edge: (p.soft_edge * pixels_per_mm).round().max(0.0) as usize,
    };
    let mut heights = shape::heights(&light, &depth, &shaping);
    if p.only_what_the_bit_can_carve {
        let bit = shape::Bit {
            tip: p.bit_tip_diameter,
            taper: p.bit_taper,
        };
        heights = shape::carvable(&heights, 1.0 / pixels_per_mm, p.max_height, &bit);
    }
    Bitmap {
        columns,
        rows,
        grey: light
            .values
            .iter()
            .map(|&l| (l * 255.0).round() as u8)
            .collect(),
        depth: heights.values,
        rgb: Vec::new(),
        ..photo.clone()
    }
}

/// The depth to shape: the estimate for Estimated, and for Picture the picture's own tones,
/// at full precision when it was imported with them (its depth then matches its greys to
/// within a level, which an estimate never does), else its greys.
fn source(photo: &Bitmap, p: &Params) -> Vec<f32> {
    let greys = || photo.grey.iter().map(|&g| f32::from(g) / 255.0);
    let has_depth = photo.depth.len() == photo.grey.len();
    let own_tones = has_depth
        && photo
            .depth
            .iter()
            .zip(greys())
            .all(|(d, g)| (d - g).abs() <= 1.0 / 255.0);
    match p.depth_from {
        DepthFrom::Estimated if has_depth => photo.depth.clone(),
        DepthFrom::Picture if own_tones => photo.depth.clone(),
        _ => greys().collect(),
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

    /// `with_photo`'s picture with no estimate: its greys are all there is.
    fn without_depth() -> Document {
        let mut doc = with_photo();
        let id = *doc.bitmaps.keys().next().unwrap();
        let mut plain = doc.bitmaps[&id].clone();
        plain.depth.clear();
        let mut c = Change::builder(&doc);
        c.replace_bitmap(id, plain);
        doc.apply(&c.finish()).unwrap();
        doc
    }

    #[test]
    fn a_depth_map_made_elsewhere_is_used_as_it_is() {
        // the 12 x 12 picture: the light background high, the dark square low
        let mut doc = without_depth();
        let params = Params {
            depth_from: DepthFrom::Picture,
            background_cut_off: 0.0,
            depth_cut_off: 0.0,
            fine_detail: 0.0,
            smoothness: 0,
            lift: 0.0,
            ..Params::default()
        };
        let r = relief(&mut doc, &params);
        let at = |x: f64, y: f64| r.height_at(Point::new(x, y)).unwrap();
        assert!(at(1.0, 1.0) > at(12.0, 12.0) + 5.0, "white is high");
        let inverted = Params {
            invert: true,
            ..params
        };
        let mut doc = without_depth();
        let r = relief(&mut doc, &inverted);
        let at = |x: f64, y: f64| r.height_at(Point::new(x, y)).unwrap();
        assert!(
            at(12.0, 12.0) > at(1.0, 1.0) + 5.0,
            "inverted, black is high"
        );
    }

    #[test]
    fn picture_uses_the_picture_even_with_an_estimate_attached() {
        // with_photo's estimate rises left to right; its picture is a dark square on light
        let params = Params {
            depth_from: DepthFrom::Picture,
            background_cut_off: 0.0,
            depth_cut_off: 0.0,
            fine_detail: 0.0,
            smoothness: 0,
            lift: 0.0,
            ..Params::default()
        };
        let mut doc = with_photo();
        let r = relief(&mut doc, &params);
        let at = |x: f64, y: f64| r.height_at(Point::new(x, y)).unwrap();
        assert!(at(1.0, 12.0) > at(12.0, 12.0) + 5.0, "white is high");
        assert!(
            (at(7.0, 12.0) - at(17.0, 12.0)).abs() < 1e-6,
            "not the estimate"
        );
    }

    #[test]
    fn picture_keeps_a_16_bit_depth_map_s_own_tones() {
        // a depth map imported at full precision: its depth is its own greys, finer
        let mut photo = without_depth().bitmaps.values().next().unwrap().clone();
        photo.depth = photo
            .grey
            .iter()
            .enumerate()
            .map(|(i, &g)| (f32::from(g) + if i % 2 == 0 { 0.3 } else { -0.3 }) / 255.0)
            .collect();
        let p = Params {
            depth_from: DepthFrom::Picture,
            ..Params::default()
        };
        assert_eq!(source(&photo, &p), photo.depth);
    }

    /// A 60 x 60 depth map over 30 x 30 mm in a job whose model cells are 0.5 mm: white
    /// (high) all over but a black groove one pixel wide down the middle.
    fn with_groove() -> Document {
        let mut doc = Document::default();
        doc.job.width = 500.0;
        doc.job.height = 500.0;
        let grey = (0..3600)
            .map(|i| if i % 60 == 30 { 0 } else { 255 })
            .collect();
        let mut c = Change::builder(&doc);
        c.add_bitmap(Bitmap {
            name: "groove".into(),
            columns: 60,
            rows: 60,
            grey,
            rect: Rect::new(Point::ZERO, Point::new(30.0, 30.0)),
            depth: Vec::new(),
            rgb: Vec::new(),
        });
        doc.apply(&c.finish()).unwrap();
        doc
    }

    #[test]
    fn a_grey_picture_with_no_estimate_opens_as_a_depth_map() {
        // estimated: the form asks for the estimate
        let mut doc = with_photo();
        let opens = |doc: &Document| ReliefFromPhoto::initial_params(doc).depth_from;
        assert_eq!(opens(&doc), DepthFrom::Estimated);
        // a plain grey picture is the depth map itself
        let id = *doc.bitmaps.keys().next().unwrap();
        doc.bitmaps.get_mut(&id).unwrap().depth.clear();
        assert_eq!(opens(&doc), DepthFrom::Picture);
        // a colour photo is not
        doc.bitmaps.get_mut(&id).unwrap().rgb = vec![0; 144 * 3];
        assert_eq!(opens(&doc), DepthFrom::Estimated);
        assert_eq!(opens(&Document::default()), DepthFrom::Estimated);
    }

    #[test]
    fn a_groove_too_narrow_for_the_bit_fills_in() {
        let asked = Params {
            depth_from: DepthFrom::Picture,
            background_cut_off: 0.0,
            depth_cut_off: 0.0,
            fine_detail: 0.0,
            smoothness: 0,
            lift: 0.0,
            max_height: 10.0,
            ..Params::default()
        };
        let groove = Point::new(15.25, 15.0);
        let as_drawn = relief(&mut with_groove(), &asked)
            .height_at(groove)
            .unwrap();
        assert!(as_drawn < 0.5, "the groove as drawn: {as_drawn}");
        let carved = Params {
            only_what_the_bit_can_carve: true,
            ..asked.clone()
        };
        let r = relief(&mut with_groove(), &carved);
        // a 0.5 mm groove 10 mm deep: the 10-degree bit gets about 1.8 mm into it
        let left = r.height_at(groove).unwrap();
        assert!(left > 7.0 && left < 9.5, "the groove as carved: {left}");
        assert!((r.height_at(Point::new(5.0, 15.0)).unwrap() - 10.0).abs() < 1e-4);
        let no_bit = Params {
            bit_tip_diameter: 0.0,
            ..carved
        };
        assert!(matches!(
            ReliefFromPhoto::run(&with_groove(), &no_bit),
            Err(CommandError::InvalidParams(_))
        ));
    }

    #[test]
    fn the_note_says_when_the_model_is_coarser_than_the_bit() {
        let mm = |v: f64| format!("{v:.2} mm");
        // points 0.49 mm apart, a 1/64 inch bit (0.2 mm radius), 0.19 mm at Highest
        let note = resolution_note(0.49, 0.185, 0.397, mm).unwrap();
        assert!(note.contains("0.49 mm apart"), "{note}");
        assert!(note.contains("about 0.20 mm"), "{note}");
        assert!(note.contains("Highest puts them 0.18 mm apart."), "{note}");
        assert!(
            !note.contains("smaller job"),
            "fine enough at Highest: {note}"
        );
        // already at Highest and still too coarse
        let note = resolution_note(0.49, 0.49, 0.397, mm).unwrap();
        assert!(
            note.ends_with("A smaller job puts them closer together."),
            "{note}"
        );
        // Highest helps but not enough
        let note = resolution_note(0.9, 0.34, 0.397, mm).unwrap();
        assert!(note.ends_with("A smaller job is finer still."), "{note}");
        assert_eq!(resolution_note(0.15, 0.1, 0.397, mm), None, "fine enough");
        assert_eq!(resolution_note(0.49, 0.185, 0.0, mm), None, "no bit");
    }

    #[test]
    fn the_base_lies_under_the_whole_picture() {
        let mut doc = with_photo();
        let params = Params {
            base_height: 1.5,
            fine_detail: 0.0,
            smoothness: 0,
            ..Params::default()
        };
        let r = relief(&mut doc, &params);
        let background = r.height_at(Point::new(1.0, 1.0)).unwrap();
        assert!((background - 1.5).abs() < 1e-6, "{background}");
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
        assert!(message.contains("depth map"), "{message}");
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
