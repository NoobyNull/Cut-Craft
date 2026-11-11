//! Embellish: folk-swirl ornament designed for the selected shape rather than cut out of a
//! pattern. The shape is read as a distance field (how far each point is from its edge):
//! a border band and bead row follow the outline, and inside, a shape that repeats round
//! its middle (a regular polygon, a star) is designed one wedge at a time and mirrored into
//! the rest, while any other shape gets a vine down its middle with scrolls branching off
//! it, leaf fans in its points and flowers where it is roomiest. Every piece keeps the
//! smallest gap from the others, so the finishing bit can carve between them. The pieces
//! come out as vectors, as a 3D component, or both.

#![allow(
    clippy::many_single_char_names,
    clippy::similar_names,
    reason = "geometry written as its formulas: points p, q, a, b, gaps g, sizes s"
)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    reason = "counts and grid cells of shapes no bigger than a sheet of material"
)]
#![allow(
    clippy::too_many_lines,
    clippy::too_many_arguments,
    reason = "each design is one sequence of steps, read top to bottom, as its sketch was"
)]

mod design;
mod field;
mod free;
mod motif;
mod occ;
mod relief;
mod rng;
mod sym;

use cut_craft_core::component::{Combine, Component};
use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Path, Point, Rect};
use cut_craft_core::trace::{Fit, fit};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use design::{Settings, Where};
use field::Field;
use motif::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct Embellish;

/// Where the ornament goes in the shape.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Place {
    /// Inside, with no border
    Fill,
    /// A border band and bead row round the edge, and ornament along it
    Border,
    /// The border and the inside
    #[default]
    Both,
}

/// What the ornament is made as.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Make {
    /// The pieces' outlines, to carve with 2D toolpaths
    Vectors,
    /// A 3D component, each piece rounded over, for 3D roughing and finishing
    #[schemars(title = "3D carving")]
    Carving,
    /// Both
    #[default]
    Both,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Params {
    #[schemars(title = "Where", extend("x-widget" = "row"))]
    pub place: Place,
    /// Low makes bold, plain pieces; high adds bead rows, sunbursts, striped scrolls and
    /// small curls
    #[schemars(range(min = 1, max = 10), extend("x-widget" = "slider"))]
    pub refinement: u32,
    /// How many scrolls and small pieces go in: low leaves open ground
    #[schemars(title = "Number of objects", range(min = 1, max = 10), extend("x-widget" = "slider"))]
    pub objects: u32,
    /// How big the scrolls may grow
    #[schemars(title = "Scroll size", range(min = 50, max = 160), extend("x-unit" = "%", "x-widget" = "slider"))]
    pub scroll_size: u32,
    /// How close the pieces sit: the smallest gap between them is 3 mm at 1 and 0.75 mm at
    /// 10. Finish with a ball-nose bit no wider than the gap
    #[schemars(range(min = 1, max = 10), extend("x-widget" = "slider"))]
    pub tightness: u32,
    /// How far the carving's highest pieces stand above the material
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub height: f64,
    #[schemars(extend("x-widget" = "row"))]
    pub make: Make,
    /// Small dots punched into the open ground of the 3D carving
    #[schemars(title = "Stippled ground")]
    pub stipple: bool,
    /// Another seed gives another design with the same settings
    #[schemars(title = "Design", extend("x-widget" = "seed"))]
    pub seed: u32,
    pub name: String,
    pub combine: Combine,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            place: Place::Both,
            refinement: 6,
            objects: 6,
            scroll_size: 100,
            tightness: 7,
            height: 3.0,
            make: Make::Both,
            stipple: false,
            seed: 4,
            name: "Embellishment".into(),
            combine: Combine::Add,
        }
    }
}

impl Params {
    /// The smallest gap between pieces, in mm.
    pub fn gap(&self) -> f64 {
        3.25 - 0.25 * f64::from(self.tightness.clamp(1, 10))
    }

    fn settings(&self, seed: u32) -> Settings {
        let unit = |v: u32| f64::from(v.clamp(1, 10) - 1) / 9.0;
        Settings {
            gap: self.gap(),
            detail: unit(self.refinement),
            density: unit(self.objects),
            scale: f64::from(self.scroll_size.clamp(50, 160)) / 100.0,
            place: match self.place {
                Place::Fill => Where::Fill,
                Place::Border => Where::Border,
                Place::Both => Where::Both,
            },
            stipple: self.stipple,
            seed,
        }
    }
}

/// Room left round a shape in its field, mm.
const MARGIN: f64 = 6.0;

/// The most cells the carving may have; a bigger area gets coarser cells.
const MOST_CELLS: f64 = 4_000_000.0;

/// One shape to embellish: an outline and the holes in it, and where its field sits.
struct Region {
    field: Field,
    offset: Point,
}

/// The selected outlines grouped into shapes: each outer outline with the holes directly
/// inside it.
fn regions(rings: &[Vec<Point>]) -> Vec<Region> {
    let depth: Vec<usize> = rings
        .iter()
        .enumerate()
        .map(|(i, r)| {
            rings
                .iter()
                .enumerate()
                .filter(|&(j, o)| j != i && free::inside_ring(o, r[0]))
                .count()
        })
        .collect();
    let mut out = vec![];
    for (i, outer) in rings.iter().enumerate() {
        if depth[i] % 2 == 1 {
            continue;
        }
        let mut members = vec![outer.clone()];
        for (j, hole) in rings.iter().enumerate() {
            if depth[j] == depth[i] + 1 && free::inside_ring(outer, hole[0]) {
                members.push(hole.clone());
            }
        }
        let bounds = members
            .iter()
            .flatten()
            .fold(Rect::new(outer[0], outer[0]), |b, &p| {
                b.union(Rect::new(p, p))
            });
        let offset = bounds.min - Point::splat(MARGIN);
        let size = bounds.size() + Point::splat(2.0 * MARGIN);
        let local = members
            .into_iter()
            .map(|r| r.into_iter().map(|p| p - offset).collect())
            .collect();
        out.push(Region {
            field: Field::new(local, size.x, size.y),
            offset,
        });
    }
    out
}

/// The ornament for one shape, in its field's frame.
fn design(field: &Field, s: Settings) -> Vec<Item> {
    match sym::detect(field) {
        Some(found) => sym::design(field, found, s),
        None => free::design(field, s),
    }
}

impl Command for Embellish {
    const ID: &'static str = "embellish";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Embellish",
            group: "Create Vectors",
            icon: "drawing/create_vectors/embellish.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if p.make != Make::Vectors {
            ensure_positive("height", p.height)?;
        }
        let (_, shapes) = doc.selected_paths(|path| path.closed);
        if shapes.is_empty() {
            return Err(CommandError::Failed(
                "select a closed vector to embellish: the ornament is designed to fit it".into(),
            ));
        }
        let rings: Vec<Vec<Point>> = shapes
            .iter()
            .map(|s| s.flatten(cut_craft_core::polygon::TOLERANCE))
            .filter(|r| r.len() >= 3)
            .collect();
        let regions = regions(&rings);
        let mut items: Vec<Item> = vec![];
        for (k, region) in regions.iter().enumerate() {
            let seed = p.seed.wrapping_add(k as u32 * 7919);
            let offset = region.offset;
            items.extend(
                design(&region.field, p.settings(seed))
                    .iter()
                    .map(|it| it.map(&|q| q + offset)),
            );
        }
        if !items
            .iter()
            .any(|it| matches!(it, Item::Shape { sink: false, .. }))
        {
            return Err(CommandError::Failed(
                "the shape is too small for the ornament. Draw it bigger, or turn Tightness up"
                    .into(),
            ));
        }
        let mut c = Change::builder(doc);
        if p.make != Make::Carving {
            let group = doc.next_group();
            for it in &items {
                for path in outlines(it) {
                    let id = c.add(path);
                    c.set_group(id, Some(group));
                }
            }
        }
        if p.make != Make::Vectors {
            c.add_component(carving(p, &regions, &items, doc.job.model_cell()));
        }
        Ok(c.finish())
    }
}

/// A piece's vectors: its outline rings, or its line; the ground's stipple has none.
fn outlines(it: &Item) -> Vec<Path> {
    match it {
        Item::Shape { sink: true, .. } => vec![],
        Item::Shape { rings, .. } => rings
            .iter()
            .filter(|r| r.len() >= 3)
            .map(|r| fit(r, true, Fit::Beziers, 0.05, 50_f64.to_radians()))
            .collect(),
        Item::Line { points, .. } => {
            vec![fit(points, false, Fit::Beziers, 0.05, 50_f64.to_radians())]
        }
    }
}

/// The pieces raised as one component over the shapes.
fn carving(p: &Params, regions: &[Region], items: &[Item], model_cell: f64) -> Component {
    let area = regions
        .iter()
        .map(|r| {
            Rect::new(
                r.offset + Point::splat(MARGIN),
                r.offset + Point::new(r.field.w, r.field.h) - Point::splat(MARGIN),
            )
        })
        .reduce(Rect::union)
        .expect("at least one shape");
    let size = area.size();
    let cell = (p.gap() / 4.0)
        .min(model_cell)
        .max((size.x * size.y / MOST_CELLS).sqrt());
    let mut part = Component::from_fn(&p.name, p.combine, area, cell, |_| None);
    let mut grid = relief::grid(area.min, cell, part.columns, part.rows, |q| {
        regions.iter().any(|r| r.field.sample(q - r.offset) > 0.0)
    });
    relief::raise(&mut grid, items, p.height);
    part.heights = grid.heights;
    part
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{document_with, square};
    use std::f64::consts::TAU;

    fn framed() -> Document {
        document_with(vec![square(0.0, 0.0, 200.0)])
    }

    cut_craft_core::contract_tests!(Embellish, framed);

    fn polygon(n: u32, r: f64) -> Path {
        let pts: Vec<Point> = (0..n)
            .map(|k| {
                Point::new(150.0, 150.0) + Point::from_angle(TAU * f64::from(k) / f64::from(n)) * r
            })
            .collect();
        Path::polyline(&pts, true)
    }

    fn lopsided() -> Path {
        let p = Point::new;
        Path::polyline(
            &[
                p(0.0, 0.0),
                p(240.0, 20.0),
                p(200.0, 120.0),
                p(120.0, 200.0),
                p(60.0, 90.0),
            ],
            true,
        )
    }

    fn made(doc: &Document, p: &Params) -> Document {
        let mut after = doc.clone();
        after.apply(&Embellish::run(doc, p).unwrap()).unwrap();
        after
    }

    #[test]
    fn makes_grouped_vectors_and_one_carving_inside_the_shape() {
        for shape in [polygon(6, 110.0), lopsided()] {
            let doc = document_with(vec![shape.clone()]);
            let after = made(&doc, &Params::default());
            let new: Vec<&Path> = after.vectors.values().filter(|v| **v != shape).collect();
            assert!(new.len() > 40, "{} pieces", new.len());
            let bounds = shape.bounds();
            for v in &new {
                let b = v.bounds();
                assert!(
                    b.min.x >= bounds.min.x - 0.5 && b.max.x <= bounds.max.x + 0.5,
                    "{b:?}"
                );
            }
            assert_eq!(after.components.len(), 1);
            let part = after.components.values().next().unwrap();
            let top = part
                .heights
                .iter()
                .copied()
                .filter(|h| !h.is_nan())
                .fold(0.0, f64::max);
            assert!((top - 3.0).abs() < 0.3, "highest {top}");
        }
    }

    #[test]
    fn a_seed_makes_the_same_design_and_another_seed_another() {
        let doc = document_with(vec![lopsided()]);
        let p = Params {
            make: Make::Vectors,
            ..Params::default()
        };
        let shapes = |d: &Document| {
            let mut v: Vec<String> = d
                .vectors
                .values()
                .map(|v| format!("{:?}", v.bounds()))
                .collect();
            v.sort();
            v
        };
        assert_eq!(shapes(&made(&doc, &p)), shapes(&made(&doc, &p)));
        let other = Params {
            seed: 5,
            ..p.clone()
        };
        assert_ne!(shapes(&made(&doc, &p)), shapes(&made(&doc, &other)));
    }

    #[test]
    fn more_objects_and_refinement_make_more_pieces() {
        let doc = document_with(vec![lopsided()]);
        let count = |objects, refinement| {
            let p = Params {
                objects,
                refinement,
                make: Make::Vectors,
                ..Params::default()
            };
            made(&doc, &p).vectors.len()
        };
        assert!(count(10, 10) > count(1, 1));
    }

    #[test]
    fn a_border_keeps_to_the_edge_and_a_fill_has_no_band() {
        let shape = polygon(4, 110.0);
        let doc = document_with(vec![shape]);
        let centre = Point::new(150.0, 150.0);
        let near_middle = |place| {
            let p = Params {
                place,
                make: Make::Vectors,
                ..Params::default()
            };
            made(&doc, &p)
                .vectors
                .values()
                .filter(|v| {
                    let b = v.bounds();
                    b.size().x < 60.0 && b.center().distance(centre) < 25.0
                })
                .count()
        };
        assert_eq!(near_middle(Place::Border), 0);
        assert!(near_middle(Place::Both) > 0);
    }

    #[test]
    fn tightness_sets_the_gap() {
        let p = |tightness| Params {
            tightness,
            ..Params::default()
        };
        assert_eq!(p(1).gap(), 3.0);
        assert_eq!(p(10).gap(), 0.75);
    }

    #[test]
    fn needs_a_closed_vector() {
        let err = Embellish::run(&Document::default(), &Params::default()).unwrap_err();
        assert!(err.to_string().contains("closed vector"), "{err}");
    }
}

/// Pictures of the carving for a look by eye: `EMBELLISH_OUT=dir cargo nextest run -p
/// cmd-embellish pictures --run-ignored only` (and `EMBELLISH_SHAPE`, a JSON list of
/// `[x, y]`, for a drawn shape).
#[cfg(test)]
mod pictures {
    use super::*;
    use std::f64::consts::{PI, TAU};

    fn shade(part: &Component, path: &std::path::Path) {
        let (w, h) = (part.columns, part.rows);
        let at = |i: usize, j: usize| {
            let v = part.heights[j.min(h - 1) * w + i.min(w - 1)];
            if v.is_nan() { 1.0 } else { v }
        };
        let (el, az) = (24_f64.to_radians(), (-135_f64).to_radians() + PI / 2.0);
        let l = [el.cos() * az.cos(), el.cos() * az.sin(), el.sin()];
        let mut img = image::RgbImage::new(w as u32, h as u32);
        for j in 0..h {
            for i in 0..w {
                let dx = (at(i + 1, j) - at(i.saturating_sub(1), j)) / (2.0 * part.cell);
                let dy = (at(i, j + 1) - at(i, j.saturating_sub(1))) / (2.0 * part.cell);
                let n = [-dx, -dy, 1.0];
                let len = (n[0] * n[0] + n[1] * n[1] + 1.0).sqrt();
                let lam = ((n[0] * l[0] + n[1] * l[1] + n[2] * l[2]) / len).max(0.0);
                let k = 0.34 + 1.1 * lam;
                let px = image::Rgb([
                    (232.0 * k).min(255.0) as u8,
                    (190.0 * k).min(255.0) as u8,
                    (138.0 * k).min(255.0) as u8,
                ]);
                img.put_pixel(i as u32, (h - 1 - j) as u32, px);
            }
        }
        img.save(path).unwrap();
    }

    fn polygon(n: u32, r: f64, catenary: bool) -> Path {
        let c = Point::new(150.0, 150.0);
        let rot = if n.is_multiple_of(2) {
            PI / f64::from(n)
        } else {
            0.0
        } + PI / 2.0;
        let mut pts = vec![];
        for k in 0..n {
            let a0 = rot + TAU * f64::from(k) / f64::from(n);
            let (p0, p1) = (
                c + Point::from_angle(a0) * r,
                c + Point::from_angle(a0 + TAU / f64::from(n)) * r,
            );
            for s in 0..40 {
                let t = f64::from(s) / 40.0;
                let q = p0.lerp(p1, t);
                let sag = if catenary {
                    let half = p0.distance(p1) / 2.0;
                    let x = (t - 0.5) * 2.0 * half;
                    let cc = half * 0.55;
                    let full = (half * 0.3).min(r * (PI / f64::from(n)).cos() * 0.26);
                    full * (1.0
                        - (((x / cc).cosh() - 1.0) / ((half / cc).cosh() - 1.0)).clamp(0.0, 1.0))
                } else {
                    0.0
                };
                pts.push(q + (c - q).normalize() * sag);
            }
        }
        Path::polyline(&pts, true)
    }

    #[test]
    #[ignore = "writes pictures to look at"]
    fn pictures() {
        let out = std::path::PathBuf::from(std::env::var("EMBELLISH_OUT").unwrap());
        let mut shapes: Vec<(String, Path)> = vec![];
        for (n, name) in [
            (3, "triangle"),
            (4, "square"),
            (6, "hexagon"),
            (8, "octagon"),
        ] {
            shapes.push((name.into(), polygon(n, 120.0, false)));
            shapes.push((format!("{name}-catenary"), polygon(n, 120.0, true)));
        }
        if let Ok(f) = std::env::var("EMBELLISH_SHAPE") {
            let pts: Vec<[f64; 2]> =
                serde_json::from_str(&std::fs::read_to_string(f).unwrap()).unwrap();
            let pts: Vec<Point> = pts.iter().map(|p| Point::new(p[0], p[1])).collect();
            shapes.push(("your-shape".into(), Path::polyline(&pts, true)));
        }
        let variants: Vec<(&str, Params)> = vec![
            ("", Params::default()),
            (
                "-low",
                Params {
                    refinement: 1,
                    objects: 1,
                    scroll_size: 60,
                    ..Params::default()
                },
            ),
            (
                "-high",
                Params {
                    refinement: 10,
                    objects: 10,
                    scroll_size: 150,
                    tightness: 10,
                    ..Params::default()
                },
            ),
            (
                "-border",
                Params {
                    place: Place::Border,
                    ..Params::default()
                },
            ),
        ];
        let only = std::env::var("EMBELLISH_ONLY").unwrap_or_default();
        for (name, shape) in &shapes {
            if !only.is_empty() && !name.contains(&only) {
                continue;
            }
            for (suffix, p) in &variants {
                let doc = cut_craft_core::testing::document_with(vec![shape.clone()]);
                let t = std::time::Instant::now();
                let change = Embellish::run(&doc, p).unwrap();
                let ms = t.elapsed().as_millis();
                let mut after = doc.clone();
                after.apply(&change).unwrap();
                eprintln!("{name}{suffix}: {} vectors, {ms} ms", after.vectors.len());
                shade(
                    after.components.values().next().unwrap(),
                    &out.join(format!("{name}{suffix}.png")),
                );
            }
        }
    }
}
