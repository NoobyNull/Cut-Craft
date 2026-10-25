//! Sketch Carving Toolpath: carves the lines of a sketch drawn from a picture or the 3D
//! model: wherever it is darker (or lower) than its surroundings.

use cut_craft_core::component::model;
use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Point, Rect};
use cut_craft_core::polygon::{self, Corners};
use cut_craft_core::toolpath::{Move, Tool, contour_moves, made_by};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct SketchCarvingToolpath;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    /// The newest bitmap
    #[default]
    Bitmap,
    /// The 3D model: its hollows are the lines
    Model,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Boundary {
    /// The edges of the picture or the job
    #[default]
    BitmapBoundary,
    /// Inside the selected closed vectors
    SelectedVectors,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
    pub source: Source,
    /// Below the material surface where carving starts
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub start_depth: f64,
    /// Below the start depth
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub cut_depth: f64,
    pub boundary: Boundary,
    /// Moves the boundary out by this much
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub boundary_offset: f64,
    /// Heavier lines but less detail as it goes up
    #[schemars(range(min = 0, max = 100))]
    pub line_thickness: f64,
    /// Its stepover spaces the passes over the lines
    pub tool: Tool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Sketch Carving".into(),
            source: Source::Bitmap,
            start_depth: 0.0,
            cut_depth: 1.0,
            boundary: Boundary::BitmapBoundary,
            boundary_offset: 0.0,
            line_thickness: 50.0,
            tool: Tool {
                name: "Engraver (1 mm)".into(),
                diameter: 1.0,
                stepover: 0.5,
                ..Tool::default()
            },
        }
    }
}

/// How much darker a cell must be than its surroundings to be part of a line.
const CONTRAST: f64 = 0.04;

/// A grid of values over `area`, `cell` apart, by row from the bottom.
struct Grid {
    area: Rect,
    cell: f64,
    columns: usize,
    rows: usize,
    values: Vec<f64>,
}

impl Grid {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "cell counts"
    )]
    fn sample(area: Rect, cell: f64, value: impl Fn(Point) -> f64) -> Self {
        let size = area.max - area.min;
        let columns = ((size.x / cell).ceil() as usize).max(1);
        let rows = ((size.y / cell).ceil() as usize).max(1);
        let mut values = Vec::with_capacity(columns * rows);
        for r in 0..rows {
            for c in 0..columns {
                values.push(value(
                    area.min + Point::new(c as f64 + 0.5, r as f64 + 0.5) * cell,
                ));
            }
        }
        Self {
            area,
            cell,
            columns,
            rows,
            values,
        }
    }

    /// Each value's mean over the square `radius` cells round it (summed-area table).
    fn blurred(&self, radius: usize) -> Vec<f64> {
        let (w, h) = (self.columns, self.rows);
        let mut sums = vec![0.0; (w + 1) * (h + 1)];
        for r in 0..h {
            for c in 0..w {
                sums[(r + 1) * (w + 1) + c + 1] = self.values[r * w + c]
                    + sums[r * (w + 1) + c + 1]
                    + sums[(r + 1) * (w + 1) + c]
                    - sums[r * (w + 1) + c];
            }
        }
        let mut out = Vec::with_capacity(w * h);
        for r in 0..h {
            for c in 0..w {
                let (c0, c1) = (c.saturating_sub(radius), (c + radius + 1).min(w));
                let (r0, r1) = (r.saturating_sub(radius), (r + radius + 1).min(h));
                let total =
                    sums[r1 * (w + 1) + c1] - sums[r0 * (w + 1) + c1] - sums[r1 * (w + 1) + c0]
                        + sums[r0 * (w + 1) + c0];
                #[expect(clippy::cast_precision_loss, reason = "cell counts")]
                out.push(total / ((c1 - c0) * (r1 - r0)) as f64);
            }
        }
        out
    }
}

impl Params {
    /// Darkness (0 to 1) over the source's extent, `cell` apart.
    fn darkness(&self, doc: &Document, cell: f64) -> Result<Grid, CommandError> {
        Ok(match self.source {
            Source::Bitmap => {
                // the bitmap picked last, else the newest
                let Some(bitmap) = doc.chosen_bitmap() else {
                    return Err(CommandError::Failed(
                        "import the picture to sketch first".into(),
                    ));
                };
                Grid::sample(bitmap.rect, cell, |q| bitmap.darkness(q).unwrap_or(0.0))
            }
            Source::Model => {
                if doc.components.is_empty() {
                    return Err(CommandError::Failed(
                        "there is no 3D model to sketch".into(),
                    ));
                }
                let area = doc.job.area();
                let size = area.max - area.min;
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "a cell count"
                )]
                let surface = model(doc, (size.x.max(size.y) / cell).ceil() as usize);
                let (low, high) = surface
                    .heights
                    .iter()
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), &z| {
                        (l.min(z), h.max(z))
                    });
                let range = (high - low).max(1e-9);
                // lower is darker
                Grid::sample(area, cell, |q| {
                    surface.at(q).map_or(0.0, |z| (high - z) / range)
                })
            }
        })
    }
}

/// Cuts each row's runs of line cells (where `keep` allows) flat at `z`.
fn cut_rows(
    grid: &Grid,
    line: &[bool],
    keep: impl Fn(Point) -> bool,
    z: f64,
    safe: f64,
) -> Vec<Move> {
    let mut moves = vec![];
    #[expect(clippy::cast_precision_loss, reason = "cell counts")]
    let centre =
        |c: usize, r: usize| grid.area.min + Point::new(c as f64 + 0.5, r as f64 + 0.5) * grid.cell;
    for r in 0..grid.rows {
        let mut c = 0;
        while c < grid.columns {
            if !(line[r * grid.columns + c] && keep(centre(c, r))) {
                c += 1;
                continue;
            }
            let first = c;
            while c + 1 < grid.columns && line[r * grid.columns + c + 1] && keep(centre(c + 1, r)) {
                c += 1;
            }
            moves.extend(contour_moves(
                &[centre(first, r), centre(c, r)],
                false,
                &[z],
                safe,
            ));
            c += 1;
        }
    }
    moves
}

impl Command for SketchCarvingToolpath {
    const ID: &'static str = "sketch_carving_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Sketch Carving Toolpath",
            group: "Toolpath Operations",
            icon: "toolpaths/sketch_carving.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("cut depth", p.cut_depth)?;
        ensure_positive("tool diameter", p.tool.diameter)?;
        if !(p.tool.stepover > 0.0 && p.tool.stepover <= 1.0) {
            return Err(CommandError::InvalidParams(
                "stepover must be more than 0 and at most 1".into(),
            ));
        }
        let step = p.tool.stepover * p.tool.diameter;
        let cell = step;
        let grid = p.darkness(doc, cell)?;
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a small radius"
        )]
        let radius = 1 + (p.line_thickness.clamp(0.0, 100.0) / 100.0 * 8.0).round() as usize;
        let around = grid.blurred(radius);
        let line: Vec<bool> = grid
            .values
            .iter()
            .zip(&around)
            .map(|(v, m)| v - m > CONTRAST)
            .collect();
        let (ids, boundary) = match p.boundary {
            Boundary::BitmapBoundary => (vec![], None),
            Boundary::SelectedVectors => {
                let (ids, paths) = doc.selected_paths(|path| path.closed);
                if ids.is_empty() {
                    return Err(CommandError::Failed(
                        "select the closed boundary vectors".into(),
                    ));
                }
                let grown = polygon::offset(&paths, p.boundary_offset.max(0.0), Corners::Round);
                let rings: Vec<Vec<Point>> = grown
                    .iter()
                    .map(|g| g.flatten(polygon::TOLERANCE))
                    .collect();
                (ids, Some(rings))
            }
        };
        let inside = |q: Point| {
            boundary
                .as_ref()
                .is_none_or(|rings| cut_craft_core::geometry::inside(rings, q))
        };
        // ponytail: each row's runs of line cells are cut flat; offsetting round the lines
        // would give smoother edges
        let z = -(p.start_depth + p.cut_depth);
        let moves = cut_rows(&grid, &line, inside, z, doc.job.clearance);
        if moves.is_empty() {
            return Err(CommandError::Failed(
                "the sketch has no lines: try a thicker line".into(),
            ));
        }
        let mut c = Change::builder(doc);
        c.add_toolpath(made_by::<Self>(
            p,
            p.name.clone(),
            p.tool.clone(),
            ids,
            moves,
        ));
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::bitmap::Bitmap;
    use cut_craft_core::toolpath::MoveKind;

    /// 40 x 40 mm, white with a black vertical line 2 mm wide down the middle.
    fn with_picture() -> Document {
        let mut doc = cut_craft_core::testing::sample_document();
        let grey = (0..40 * 40)
            .map(|i| if (19..21).contains(&(i % 40)) { 0 } else { 255 })
            .collect();
        let mut c = Change::builder(&doc);
        c.add_bitmap(Bitmap {
            name: "line".into(),
            columns: 40,
            rows: 40,
            grey,
            rect: Rect::new(Point::ZERO, Point::new(40.0, 40.0)),
        });
        let change = c.finish();
        doc.apply(&change).unwrap();
        doc
    }

    cut_craft_core::contract_tests!(SketchCarvingToolpath, with_picture);

    fn sketch(doc: &Document, p: &Params) -> Vec<Move> {
        let change = SketchCarvingToolpath::run(doc, p).unwrap();
        let mut doc = doc.clone();
        doc.apply(&change).unwrap();
        doc.toolpaths.into_values().next().unwrap().moves
    }

    #[test]
    fn carves_the_dark_line_and_nothing_on_the_white() {
        let moves = sketch(&with_picture(), &Params::default());
        let cuts: Vec<_> = moves
            .iter()
            .filter(|m| m.kind == MoveKind::Cut)
            .map(|m| m.to)
            .collect();
        assert!(cuts.len() > 2, "{cuts:?}");
        assert!(
            cuts.iter().all(|q| (18.0..22.0).contains(&q.x)),
            "only along the line"
        );
        assert!(cuts.iter().all(|q| (q.z + 1.0).abs() < 1e-9));
    }

    #[test]
    fn a_boundary_vector_limits_it() {
        let mut doc = with_picture();
        let mut c = Change::builder(&doc);
        let id = c.add(cut_craft_core::testing::square(0.0, 0.0, 40.0).transformed(
            &cut_craft_core::glam::DAffine2::from_scale(cut_craft_core::glam::DVec2::new(
                1.0, 0.25,
            )),
        ));
        c.select([id]);
        let change = c.finish();
        doc.apply(&change).unwrap();
        doc.selection = [id].into();
        let moves = sketch(
            &doc,
            &Params {
                boundary: Boundary::SelectedVectors,
                ..Params::default()
            },
        );
        assert!(
            moves
                .iter()
                .filter(|m| m.kind == MoveKind::Cut)
                .all(|m| m.to.y <= 10.0 + 1e-9)
        );
    }

    #[test]
    fn needs_a_picture_or_model() {
        let why = SketchCarvingToolpath::run(&Document::default(), &Params::default())
            .unwrap_err()
            .to_string();
        assert!(why.contains("picture"), "{why}");
        let why = SketchCarvingToolpath::run(
            &Document::default(),
            &Params {
                source: Source::Model,
                ..Params::default()
            },
        )
        .unwrap_err()
        .to_string();
        assert!(why.contains("3D model"), "{why}");
    }
}
