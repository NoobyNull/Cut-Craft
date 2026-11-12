//! Create Decorative Relief: the baroque or western-leaf scrolls of Create Vector Texture
//! raised as a 3D component inside the selected closed vectors (or the job): rounded bands
//! rolling into raised eyes, lobed leaves lying over them and curling up towards their tips,
//! domed flowers, ready for 3D roughing and finishing. Nothing in it is finer than the
//! finishing bit can carve, and being a height map it has no undercuts.

pub mod relief;

use cmd_vector_texture::{Ornament, ornament_pieces};
use cut_craft_core::component::{Combine, Component};
use cut_craft_core::document::{Document, Units};
use cut_craft_core::geometry::{Path, Point, Rect};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct DecorativeRelief;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    #[serde(default)]
    #[schemars(extend("x-widget" = "named-icons", "x-icon" = "drawing_forms/fill_{}.png"))]
    pub style: Ornament,
    /// How high the carving stands at its highest point, above the material it sits on
    #[serde(default = "height")]
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub height: f64,
    /// How big a scroll is at its smallest: bigger makes fewer, bolder scrolls
    #[serde(default = "pattern_size")]
    #[schemars(title = "Pattern size", extend("x-unit" = "mm"), range(min = 0))]
    pub size: f64,
    /// The ball-nose bit you will finish with. Nothing in the carving is finer than it, so it
    /// carves what you see: 1/8 inch is 3.175 mm
    #[serde(default = "finishing_bit")]
    #[schemars(title = "Finishing bit", extend("x-unit" = "mm"), range(min = 0))]
    pub bit: f64,
    /// Carved lines inside each leaf and scroll that follow its edge, evenly spaced to its
    /// middle, as carvers echo a leaf's outline; left out where a piece is too narrow
    #[serde(default)]
    #[schemars(title = "Echo lines", range(min = 0, max = 3))]
    pub lines: u32,
    /// Another seed gives another carving with the same settings
    #[schemars(title = "Variation", extend("x-widget" = "seed"))]
    pub seed: u32,
    /// The name the carving goes by in the list of 3D components
    pub name: String,
    pub combine: Combine,
}

fn height() -> f64 {
    6.0
}

fn pattern_size() -> f64 {
    40.0
}

fn finishing_bit() -> f64 {
    3.175
}

impl Default for Params {
    fn default() -> Self {
        Self {
            style: Ornament::default(),
            height: height(),
            size: pattern_size(),
            bit: finishing_bit(),
            lines: 0,
            seed: 1,
            name: "Decorative Relief".into(),
            combine: Combine::Add,
        }
    }
}

/// The most cells a relief may have; a bigger area gets coarser cells.
const MOST_CELLS: f64 = 1_500_000.0;

impl Command for DecorativeRelief {
    const ID: &'static str = "decorative_relief";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Create Decorative Relief",
            group: "Modeling",
            icon: "modeling/decorative_relief.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("height", p.height)?;
        ensure_positive("finishing bit", p.bit)?;
        let (_, boundary) = doc.selected_paths(|path| path.closed);
        let mut rings: Vec<_> = boundary
            .iter()
            .map(|b| b.flatten(cut_craft_core::polygon::TOLERANCE))
            .collect();
        let area = boundary
            .iter()
            .map(Path::bounds)
            .reduce(Rect::union)
            .unwrap_or_else(|| doc.job.area());
        if rings.is_empty() {
            rings.push(vec![
                area.min,
                Point::new(area.max.x, area.min.y),
                area.max,
                Point::new(area.min.x, area.max.y),
            ]);
        }
        let part = carve(p, rings, area, doc.job.model_cell(), doc.job.units)?;
        let mut c = Change::builder(doc);
        c.add_component(part);
        Ok(c.finish())
    }
}

/// The relief inside `rings` (whose box is `area`), its cells no coarser than `model_cell`
/// and fine enough for the bit.
///
/// # Errors
/// When a size is not positive, or the pattern is too small or too fine for the area.
pub fn carve(
    p: &Params,
    rings: Vec<Vec<Point>>,
    area: Rect,
    model_cell: f64,
    units: Units,
) -> Result<Component, CommandError> {
    // scrolls closer than the bit could not be carved apart
    let pieces = ornament_pieces(p.style, rings, p.size, p.bit, p.seed).map_err(|e| match e {
        CommandError::Failed(m) if m.contains("smallest gap") => CommandError::Failed(format!(
            "the pattern size must be at least three times the finishing bit ({}). Make the \
             pattern size bigger",
            units.length(p.bit * 3.0, 1)
        )),
        e => e,
    })?;
    if pieces.is_empty() {
        return Err(CommandError::Failed(
            "the area is too small for the pattern. Make the pattern size smaller, or draw a \
             bigger area"
                .into(),
        ));
    }
    let size = area.size();
    let cell = (p.bit / 5.0)
        .min(model_cell)
        .max((size.x * size.y / MOST_CELLS).sqrt());
    let shaping = relief::Shaping {
        height: p.height,
        bit_radius: p.bit / 2.0,
        fluted: p.style == Ornament::Baroque,
        lines: p.lines,
    };
    let grid = relief::raise(&pieces, area, cell, shaping);
    let mut part = Component::from_fn(&p.name, p.combine, area, cell, |_| None);
    debug_assert_eq!((part.columns, part.rows), (grid.columns, grid.rows));
    part.heights = grid.heights;
    Ok(part)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{document_with, square};

    cut_craft_core::contract_tests!(DecorativeRelief, framed);

    /// A 150 mm square, selected.
    fn framed() -> Document {
        document_with(vec![square(0.0, 0.0, 150.0)])
    }

    fn relief(p: &Params) -> Component {
        let mut doc = framed();
        let change = DecorativeRelief::run(&doc, p).unwrap();
        doc.apply(&change).unwrap();
        doc.components.values().next().unwrap().clone()
    }

    #[test]
    fn makes_one_component_as_high_as_asked_inside_the_shape() {
        for style in [Ornament::Baroque, Ornament::Western] {
            let p = Params {
                style,
                ..Params::default()
            };
            let part = relief(&p);
            let (lo, hi) = part.height_range().unwrap();
            assert!(lo >= 0.0, "{style:?}: {lo}");
            assert!((hi - p.height).abs() < 0.05, "{style:?}: {hi}");
            let e = part.extent();
            let edge = 150.0 + part.cell;
            assert!(e.min.x >= -0.01 && e.max.x <= edge && e.max.y <= edge);
            let raised = part.heights.iter().filter(|h| !h.is_nan()).count();
            assert!(raised > part.heights.len() / 20, "{style:?}: {raised}");
        }
    }

    #[test]
    fn another_variation_carves_another_pattern() {
        let a = relief(&Params::default());
        let b = relief(&Params {
            seed: 2,
            ..Params::default()
        });
        assert!(!cut_craft_core::component::same_heights(
            &a.heights, &b.heights
        ));
        let again = relief(&Params::default());
        assert!(cut_craft_core::component::same_heights(
            &a.heights,
            &again.heights
        ));
    }

    #[test]
    fn the_finishing_bit_carves_it_as_made() {
        // closing it again with the bit's ball changes nothing: no hollow is narrower than
        // the bit and no ridge sharper than it can leave
        let p = Params::default();
        let part = relief(&p);
        let mut grid = relief::Grid {
            origin: part.origin,
            cell: part.cell,
            columns: part.columns,
            rows: part.rows,
            heights: part.heights.clone(),
        };
        relief::close(&mut grid, p.bit / 2.0);
        let worst = part
            .heights
            .iter()
            .zip(&grid.heights)
            .map(|(a, b)| (if a.is_nan() { 0.0 } else { *a } - b).abs())
            .fold(0.0, f64::max);
        assert!(worst < 0.01, "{worst}");
    }

    #[test]
    fn echo_lines_carve_creases_the_bit_can_still_follow() {
        let fine = Params {
            bit: 1.5875,
            ..Params::default()
        };
        let plain = relief(&fine);
        let p = Params { lines: 2, ..fine };
        let lined = relief(&p);
        assert!(!cut_craft_core::component::same_heights(
            &plain.heights,
            &lined.heights
        ));
        let mut grid = relief::Grid {
            origin: lined.origin,
            cell: lined.cell,
            columns: lined.columns,
            rows: lined.rows,
            heights: lined.heights.clone(),
        };
        relief::close(&mut grid, p.bit / 2.0);
        let worst = lined
            .heights
            .iter()
            .zip(&grid.heights)
            .map(|(a, b)| (if a.is_nan() { 0.0 } else { *a } - b).abs())
            .fold(0.0, f64::max);
        assert!(worst < 0.01, "{worst}");
    }

    #[test]
    fn a_pattern_smaller_than_three_bits_says_how_to_fix_it() {
        let doc = framed();
        let p = Params {
            size: 6.0,
            ..Params::default()
        };
        let Err(CommandError::Failed(m)) = DecorativeRelief::run(&doc, &p) else {
            panic!("expected an error");
        };
        assert!(m.contains("finishing bit"), "{m}");
    }

    /// Writes each style as a shaded picture to the folder named by `RELIEF_PREVIEWS`.
    #[test]
    #[ignore = "writes pictures to look at"]
    fn draw_previews() {
        let dir = std::env::var("RELIEF_PREVIEWS").unwrap();
        let seeds: Vec<u32> = std::env::var("RELIEF_SEEDS").map_or_else(
            |_| vec![1],
            |s| s.split(',').map(|v| v.parse().unwrap()).collect(),
        );
        let shapes: [(&str, Vec<Point>); 3] = [
            (
                "oval",
                (0..96)
                    .map(|i| {
                        let a = std::f64::consts::TAU * f64::from(i) / 96.0;
                        Point::new(100.0 + 95.0 * a.cos(), 70.0 + 65.0 * a.sin())
                    })
                    .collect(),
            ),
            (
                "triangle",
                vec![
                    Point::new(2.0, 2.0),
                    Point::new(198.0, 2.0),
                    Point::new(100.0, 148.0),
                ],
            ),
            (
                "square",
                vec![
                    Point::new(0.0, 0.0),
                    Point::new(150.0, 0.0),
                    Point::new(150.0, 150.0),
                    Point::new(0.0, 150.0),
                ],
            ),
        ];
        for style in [Ornament::Baroque, Ornament::Western] {
            for (name, ring) in &shapes {
                for &seed in &seeds {
                    let area = ring.iter().fold(Rect::point(ring[0]), |r, &q| r.include(q));
                    let p = Params {
                        style,
                        seed,
                        lines: std::env::var("RELIEF_LINES").map_or(0, |v| v.parse().unwrap()),
                        bit: std::env::var("RELIEF_BIT").map_or(3.175, |v| v.parse().unwrap()),
                        ..Params::default()
                    };
                    let part = carve(&p, vec![ring.clone()], area, 0.25, Units::Mm).unwrap();
                    shade(&part, ring)
                        .save(format!("{dir}/{style:?}-{name}-{seed}.png"))
                        .unwrap();
                    // the heights too, for a 3D render: columns, rows, cell, then rows of f32
                    let mut raw = Vec::new();
                    for v in [part.columns, part.rows] {
                        raw.extend(u32::try_from(v).unwrap().to_le_bytes());
                    }
                    raw.extend(part.cell.to_le_bytes());
                    for h in &part.heights {
                        #[expect(clippy::cast_possible_truncation, reason = "a picture")]
                        raw.extend((if h.is_nan() { 0.0 } else { *h } as f32).to_le_bytes());
                    }
                    std::fs::write(format!("{dir}/{style:?}-{name}-{seed}.heights"), raw).unwrap();
                }
            }
        }
    }

    /// The component lit from the upper left in a wood tone over a pale background, the
    /// outline drawn in.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::many_single_char_names,
        reason = "pixels, rows and columns"
    )]
    fn shade(part: &Component, ring: &[Point]) -> image::RgbImage {
        let (w, h) = (part.columns, part.rows);
        let z = |c: usize, r: usize| {
            let v = part.heights[r * w + c];
            if v.is_nan() { 0.0 } else { v }
        };
        let light = light();
        let mut img = image::RgbImage::new(w as u32, h as u32);
        for r in 0..h {
            for c in 0..w {
                let dx = (z((c + 1).min(w - 1), r) - z(c.saturating_sub(1), r)) / (2.0 * part.cell);
                let dy = (z(c, (r + 1).min(h - 1)) - z(c, r.saturating_sub(1))) / (2.0 * part.cell);
                let n = [-dx, -dy, 1.0];
                let len = (n[0] * n[0] + n[1] * n[1] + 1.0).sqrt();
                let lit = ((n[0] * light[0] + n[1] * light[1] + n[2] * light[2]) / len).max(0.0);
                let raised = !part.heights[r * w + c].is_nan();
                let base = if raised {
                    [214.0, 160.0, 102.0]
                } else {
                    [226.0, 196.0, 160.0]
                };
                let k = 0.35 + 0.75 * lit;
                let px = base.map(|b: f64| (b * k).clamp(0.0, 255.0) as u8);
                img.put_pixel(c as u32, (h - 1 - r) as u32, image::Rgb(px));
            }
        }
        // the outline
        for (i, &a) in ring.iter().enumerate() {
            let b = ring[(i + 1) % ring.len()];
            for s in 0..400 {
                let q = a.lerp(b, f64::from(s) / 400.0);
                let (c, r) = (
                    (q.x - part.origin.x) / part.cell,
                    (q.y - part.origin.y) / part.cell,
                );
                if c >= 0.0 && r >= 0.0 && (c as usize) < w && (r as usize) < h {
                    img.put_pixel(
                        c as u32,
                        (h - 1 - r as usize) as u32,
                        image::Rgb([60, 40, 25]),
                    );
                }
            }
        }
        img
    }

    fn light() -> [f64; 3] {
        let v: [f64; 3] = [-0.5, 0.6, 0.65];
        let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        v.map(|c| c / len)
    }
}
