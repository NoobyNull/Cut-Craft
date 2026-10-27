//! Edit Picture: a picture's contrast, brightness and gamma, inverted if asked, and a border
//! that fades its edges to white (Aspire's help: Edit Picture). Pictures are kept in grey,
//! so Aspire's Grayscale has nothing to do here.

use cut_craft_core::bitmap::Bitmap;
use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::{Change, Command, CommandError, Meta, existing};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct EditPicture;

/// Aspire's first section: the picture's tones.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Tones {
    /// Above 0 the light and dark parts move apart; below, toward middle grey
    #[schemars(range(min = -100, max = 100))]
    pub contrast: f64,
    /// Above 0 lighter, below darker
    #[schemars(range(min = -100, max = 100))]
    pub brightness: f64,
    /// Above 1 lighter and below darker, keeping the detail
    #[schemars(range(min = 0.1, max = 10))]
    pub gamma: f64,
    /// White becomes black and black white
    pub invert: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Shape {
    #[default]
    #[schemars(title = "Rectangular Border")]
    Rectangular,
    #[schemars(title = "Oval Border")]
    Oval,
}

/// Aspire's second section: a border fading the edges to white.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Border {
    #[schemars(title = "Add Border")]
    pub add: bool,
    #[schemars(extend("x-widget" = "row"))]
    pub shape: Shape,
    /// How far in the fade reaches, as a percentage of the picture's shorter side
    #[schemars(range(min = 0, max = 50), extend("x-unit" = "%"))]
    pub width: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// The picture to change
    pub bitmap: EntityId,
    #[schemars(title = "Picture", extend("x-section" = true))]
    pub tones: Tones,
    #[schemars(title = "Border", extend("x-section" = true))]
    pub border: Border,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            bitmap: EntityId(1),
            tones: Tones {
                contrast: 0.0,
                brightness: 0.0,
                gamma: 1.0,
                invert: false,
            },
            border: Border {
                add: false,
                shape: Shape::Rectangular,
                width: 10.0,
            },
        }
    }
}

impl Command for EditPicture {
    const ID: &'static str = "edit_picture";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Edit Picture",
            group: "Edit Objects",
            icon: "drawing/edit_objects/edit_picture.svg",
            shortcut: "",
        }
    }

    /// The picture picked first.
    fn initial_params(doc: &Document) -> Params {
        let bitmap = doc
            .selected_bitmaps
            .iter()
            .copied()
            .find(|id| doc.bitmaps.contains_key(id))
            .or_else(|| doc.bitmaps.keys().next().copied())
            .unwrap_or(EntityId(1));
        Params {
            bitmap,
            ..Params::default()
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let before = existing(&doc.bitmaps, p.bitmap, "picture")?;
        if p.tones.gamma <= 0.0 {
            return Err(CommandError::InvalidParams("gamma must be above 0".into()));
        }
        let mut c = Change::builder(doc);
        c.replace_bitmap(p.bitmap, edited(before, p));
        Ok(c.finish())
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "grey levels are clamped to 0..=255 and pixel counts are small"
)]
fn edited(picture: &Bitmap, params: &Params) -> Bitmap {
    let tones = &params.tones;
    let (columns, rows) = (picture.columns as f64, picture.rows as f64);
    let reach = params.border.width / 100.0 * columns.min(rows);
    // one pixel's tone (0 black to 1 white) edited
    let adjust = |i: usize, tone: f64| {
        let mut v = (tone - 0.5) * (1.0 + tones.contrast / 100.0) + 0.5 + tones.brightness / 100.0;
        v = v.clamp(0.0, 1.0).powf(1.0 / tones.gamma);
        if tones.invert {
            v = 1.0 - v;
        }
        if params.border.add && reach > 0.0 {
            // how far in from the border this pixel's centre is, in pixels
            let (x, y) = (
                (i % picture.columns) as f64 + 0.5,
                (i / picture.columns) as f64 + 0.5,
            );
            let inward = match params.border.shape {
                Shape::Rectangular => x.min(y).min(columns - x).min(rows - y),
                Shape::Oval => {
                    let (dx, dy) = (x / columns * 2.0 - 1.0, y / rows * 2.0 - 1.0);
                    (1.0 - dx.hypot(dy)) * columns.min(rows) / 2.0
                }
            };
            let kept = (inward / reach).clamp(0.0, 1.0);
            v = v * kept + (1.0 - kept);
        }
        v
    };
    let grey = picture
        .grey
        .iter()
        .enumerate()
        .map(|(i, &level)| {
            (adjust(i, f64::from(level) / 255.0) * 255.0)
                .round()
                .clamp(0.0, 255.0) as u8
        })
        .collect();
    // a depth map is edited at its own precision
    #[expect(clippy::cast_possible_truncation, reason = "a tone from 0 to 1")]
    let depth = picture
        .depth
        .iter()
        .enumerate()
        .map(|(i, &tone)| adjust(i, f64::from(tone)) as f32)
        .collect();
    Bitmap {
        grey,
        depth,
        ..picture.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::{Point, Rect};

    /// A 20 × 10 picture in middle grey, apart from a black left half.
    fn picture() -> Document {
        let grey = (0..200)
            .map(|i| if i % 20 < 10 { 0 } else { 128 })
            .collect();
        let mut doc = Document::default();
        let mut c = Change::builder(&doc);
        c.add_bitmap(Bitmap {
            name: "p".into(),
            columns: 20,
            rows: 10,
            grey,
            rect: Rect::new(Point::ZERO, Point::new(20.0, 10.0)),
            depth: Vec::new(),
        });
        doc.apply(&c.finish()).unwrap();
        doc
    }

    cut_craft_core::contract_tests!(EditPicture, picture);

    fn after(p: &Params) -> Vec<u8> {
        let mut doc = picture();
        cut_craft_core::testing::run_on::<EditPicture>(&mut doc, p);
        doc.bitmaps[&EntityId(1)].grey.clone()
    }

    fn with(tones: Tones) -> Params {
        Params {
            tones,
            ..Params::default()
        }
    }

    #[test]
    fn the_defaults_change_nothing() {
        assert_eq!(
            after(&Params::default()),
            picture().bitmaps[&EntityId(1)].grey
        );
    }

    #[test]
    fn tones_brighten_darken_and_invert() {
        let base = Params::default().tones;
        let brighter = after(&with(Tones {
            brightness: 50.0,
            ..base.clone()
        }));
        assert!(
            brighter[0] > 100 && brighter[15] == 255,
            "{:?}",
            &brighter[..20]
        );
        let flat = after(&with(Tones {
            contrast: -100.0,
            ..base.clone()
        }));
        assert!(
            flat.iter().all(|&g| g == 128),
            "no contrast is all middle grey"
        );
        let inverted = after(&with(Tones {
            invert: true,
            ..base.clone()
        }));
        assert_eq!((inverted[0], inverted[15]), (255, 127));
        let lighter = after(&with(Tones { gamma: 2.0, ..base }));
        assert!(
            lighter[15] > 128 && lighter[0] == 0,
            "gamma keeps black black"
        );
    }

    #[test]
    fn a_border_fades_the_edges_to_white() {
        for shape in [Shape::Rectangular, Shape::Oval] {
            let g = after(&Params {
                border: Border {
                    add: true,
                    shape,
                    width: 40.0,
                },
                ..Params::default()
            });
            assert!(g[0] > 200, "{shape:?}: the corner is nearly white");
            assert_eq!(g[5 * 20 + 9], 0, "{shape:?}: the middle is untouched");
        }
    }

    #[test]
    fn needs_the_picture() {
        assert!(
            cut_craft_core::testing::rejection::<EditPicture>(&Params::default())
                .contains("picture 1 does not exist")
        );
    }
}
