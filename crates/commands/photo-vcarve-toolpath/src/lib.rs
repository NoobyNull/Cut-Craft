//! Photo V-Carve Toolpath: a V-bit etches a picture as grooves, deeper (so wider) where it
//! is darker, along parallel lines across it or along the selected vectors.

use cut_craft_core::bitmap::Bitmap;
use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Path, Point};
use cut_craft_core::glam::DVec3;
use cut_craft_core::toolpath::{
    Move, MoveKind, Tool, ToolDatabase, ToolShape, cut_points, hatch_lines, made_by,
    project_onto_model,
};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct PhotoVcarveToolpath;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    /// One set of parallel lines
    #[default]
    Raster,
    /// Two sets of lines at right angles: denser, twice as long to cut
    Hatch,
    /// Along the selected vectors
    SelectedVectors,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
    /// Below the material surface where the carving is (the floor of an earlier pocket)
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub start_depth: f64,
    /// Where the picture is black
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub max_carving_depth: f64,
    /// Lift above the start depth to move between lines
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub stepover_retract: f64,
    pub strategy: Strategy,
    /// Between lines, as a percentage of the groove width at the max depth (100: grooves meet)
    #[schemars(range(min = 1, max = 1000))]
    pub line_spacing_percent: f64,
    /// Of the lines, from horizontal
    #[schemars(extend("x-unit" = "°"))]
    pub line_angle: f64,
    /// A V-bit
    pub tool: Tool,
    /// After calculating, drop the toolpath onto the 3D model: its depths are then below the
    /// model's surface, going no lower than the bottom of the material
    #[serde(default)]
    #[schemars(title = "Project toolpath onto 3D Model")]
    pub project_onto_model: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Photo V-Carve".into(),
            start_depth: 0.0,
            max_carving_depth: 1.0,
            stepover_retract: 0.5,
            strategy: Strategy::Raster,
            line_spacing_percent: 100.0,
            line_angle: 0.0,
            tool: ToolDatabase::starter()
                .tools
                .into_iter()
                .find(|t| t.shape == ToolShape::VBit)
                .expect("the starter tools include a V-bit"),
            project_onto_model: false,
        }
    }
}

/// `line` sampled every `step` or closer, each point at the depth the picture sets there,
/// leaving out stretches off the picture: each piece on it, as 3D points.
fn carve_along(
    line: &[Point],
    step: f64,
    bitmap: &Bitmap,
    depth: impl Fn(f64) -> f64,
) -> Vec<Vec<DVec3>> {
    let mut pieces = vec![];
    let mut piece: Vec<DVec3> = vec![];
    let mut visit = |p: Point| match bitmap.darkness(p) {
        Some(dark) => piece.push(p.extend(depth(dark))),
        None if !piece.is_empty() => pieces.push(std::mem::take(&mut piece)),
        None => {}
    };
    for (i, w) in line.windows(2).enumerate() {
        let (a, b) = (w[0], w[1]);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a count"
        )]
        let n = (a.distance(b) / step).ceil().max(1.0) as u32;
        for k in u32::from(i > 0)..=n {
            visit(a.lerp(b, f64::from(k) / f64::from(n)));
        }
    }
    if line.len() == 1 {
        visit(line[0]);
    }
    if !piece.is_empty() {
        pieces.push(piece);
    }
    pieces
}

/// The pieces cut one after another, lifting to `lift` between them (from and back to
/// `safe` at the very start and end).
fn link(pieces: impl Iterator<Item = Vec<DVec3>>, lift: f64, safe: f64) -> Vec<Move> {
    let mut moves: Vec<Move> = vec![];
    for piece in pieces {
        let start = piece[0];
        let first = moves.is_empty();
        moves.push(Move {
            kind: if first {
                MoveKind::Rapid
            } else {
                MoveKind::Cut
            },
            to: start.truncate().extend(if first { safe } else { lift }),
        });
        moves.push(Move {
            kind: MoveKind::Plunge,
            to: start,
        });
        moves.extend(piece[1..].iter().map(|&to| Move {
            kind: MoveKind::Cut,
            to,
        }));
        moves.push(Move {
            kind: MoveKind::Cut,
            to: piece[piece.len() - 1].truncate().extend(lift),
        });
    }
    if let Some(last) = moves.last().map(|m| m.to) {
        moves.push(Move {
            kind: MoveKind::Rapid,
            to: last.truncate().extend(safe),
        });
    }
    moves
}

impl Command for PhotoVcarveToolpath {
    const ID: &'static str = "photo_vcarve_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Photo V-Carve Toolpath",
            group: "Toolpath Operations",
            icon: "toolpaths/engraving.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if p.tool.shape != ToolShape::VBit {
            return Err(CommandError::InvalidParams(
                "photo V-carving needs a V-bit".into(),
            ));
        }
        ensure_positive("max carving depth", p.max_carving_depth)?;
        ensure_positive("line spacing", p.line_spacing_percent)?;
        // the bitmap picked last, else the newest
        let Some(bitmap) = doc.chosen_bitmap() else {
            return Err(CommandError::Failed(
                "import the picture to carve first".into(),
            ));
        };
        let groove = 2.0 * p.max_carving_depth * (p.tool.angle.to_radians() / 2.0).tan();
        let spacing = groove * p.line_spacing_percent / 100.0;
        let size = bitmap.rect.max - bitmap.rect.min;
        #[expect(clippy::cast_precision_loss, reason = "pixel counts")]
        let pixel = (size.x / bitmap.columns as f64).min(size.y / bitmap.rows as f64);
        let step = pixel.min(spacing / 2.0);
        let (ids, lines): (Vec<_>, Vec<Vec<Point>>) = match p.strategy {
            Strategy::SelectedVectors => {
                let (ids, paths) = doc.selected_paths(|_| true);
                if ids.is_empty() {
                    return Err(CommandError::Failed(
                        "select the vectors to carve along".into(),
                    ));
                }
                let lines = paths
                    .iter()
                    .map(|path| {
                        let mut points = cut_points(path);
                        if path.closed {
                            points.push(points[0]);
                        }
                        points
                    })
                    .collect();
                (ids, lines)
            }
            Strategy::Raster | Strategy::Hatch => {
                let r = bitmap.rect;
                let outline = Path::polyline(
                    &[
                        r.min,
                        Point::new(r.max.x, r.min.y),
                        r.max,
                        Point::new(r.min.x, r.max.y),
                    ],
                    true,
                );
                let mut angles = vec![p.line_angle];
                if p.strategy == Strategy::Hatch {
                    angles.push(p.line_angle + 90.0);
                }
                let lines = angles
                    .into_iter()
                    .flat_map(|a| hatch_lines(std::slice::from_ref(&outline), spacing, a))
                    .flatten()
                    .enumerate()
                    // back and forth
                    .map(|(i, (a, b))| if i % 2 == 0 { vec![a, b] } else { vec![b, a] })
                    .collect();
                (vec![], lines)
            }
        };
        let depth = |dark: f64| -(p.start_depth + p.max_carving_depth * dark);
        let (lift, safe) = (-p.start_depth + p.stepover_retract, doc.job.clearance);
        let pieces = lines
            .iter()
            .flat_map(|line| carve_along(line, step, bitmap, depth));
        let mut moves = link(pieces, lift, safe);
        if p.project_onto_model {
            moves = project_onto_model(doc, &p.tool, &moves)?;
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
    use cut_craft_core::geometry::Rect;

    /// A 20 x 10 mm picture: black on the left half, white on the right.
    fn with_picture() -> Document {
        let mut doc = cut_craft_core::testing::sample_document();
        let mut c = Change::builder(&doc);
        c.add_bitmap(Bitmap {
            name: "half".into(),
            columns: 2,
            rows: 1,
            grey: vec![0, 255],
            rect: Rect::new(Point::ZERO, Point::new(20.0, 10.0)),
            depth: Vec::new(),
        });
        let change = c.finish();
        doc.apply(&change).unwrap();
        doc
    }

    cut_craft_core::contract_tests!(PhotoVcarveToolpath, with_picture);

    fn carve(p: &Params) -> Vec<Move> {
        let mut doc = with_picture();
        let change = PhotoVcarveToolpath::run(&doc, p).unwrap();
        doc.apply(&change).unwrap();
        doc.toolpaths.into_values().next().unwrap().moves
    }

    #[test]
    fn darker_is_deeper_along_lines_a_groove_width_apart() {
        let moves = carve(&Params::default());
        let cuts: Vec<DVec3> = moves
            .iter()
            .filter(|m| m.kind != MoveKind::Rapid && m.to.z < 1e-9)
            .map(|m| m.to)
            .collect();
        let at = |x: f64| cuts.iter().find(|q| (q.x - x).abs() < 0.05).unwrap().z;
        assert!((at(1.0) + 1.0).abs() < 1e-9, "black: the max depth");
        assert!(at(19.0).abs() < 0.01, "white: at the surface: {}", at(19.0));
        // a 90 degree bit 1 mm deep cuts 2 mm wide: 5 lines across 10 mm
        let mut rows: Vec<f64> = cuts.iter().map(|q| q.y).collect();
        rows.sort_by(f64::total_cmp);
        rows.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        assert_eq!(rows.len(), 5, "{rows:?}");
    }

    #[test]
    fn projected_onto_the_model_it_carves_below_its_surface() {
        // an all-black picture over the domed 40 mm job: 1 mm deep everywhere
        let mut doc = cut_craft_core::testing::domed();
        let mut c = Change::builder(&doc);
        c.add_bitmap(Bitmap {
            name: "black".into(),
            columns: 1,
            rows: 1,
            grey: vec![0],
            depth: Vec::new(),
            rect: Rect::new(Point::ZERO, Point::new(40.0, 40.0)),
        });
        let change = c.finish();
        doc.apply(&change).unwrap();
        let p = Params {
            project_onto_model: true,
            ..Params::default()
        };
        let change = PhotoVcarveToolpath::run(&doc, &p).unwrap();
        doc.apply(&change).unwrap();
        let moves = doc.toolpaths.into_values().next().unwrap().moves;
        let carved = || moves.iter().filter(|m| m.kind != MoveKind::Rapid);
        // over the dome's top: 1 mm below it, or lifted half a millimetre above it
        let top = carved().filter(|m| m.to.truncate().distance(Point::new(20.0, 20.0)) < 1.5);
        assert!(top.clone().count() > 0);
        let odd = top.clone().find(|m| !(-1.3..=0.5 + 1e-9).contains(&m.to.z));
        assert!(odd.is_none(), "{odd:?}");
        assert!(top.clone().any(|m| m.to.z < -0.9));
        // 1 mm below the flat round it, and never through the 10 mm material
        assert!(carved().any(|m| (m.to.z + 9.0).abs() < 0.01));
        assert!(carved().all(|m| m.to.z >= -10.0 - 1e-9));
    }

    #[test]
    fn hatch_adds_lines_at_right_angles() {
        let raster = carve(&Params::default()).len();
        let hatch = carve(&Params {
            strategy: Strategy::Hatch,
            ..Params::default()
        })
        .len();
        assert!(hatch > raster);
    }

    #[test]
    fn between_lines_it_lifts_only_the_stepover_retract() {
        let moves = carve(&Params::default());
        let highest_inside = moves[1..moves.len() - 1]
            .iter()
            .map(|m| m.to.z)
            .fold(f64::NEG_INFINITY, f64::max);
        assert!((highest_inside - 0.5).abs() < 1e-9, "{highest_inside}");
    }

    #[test]
    fn needs_a_picture() {
        let why = PhotoVcarveToolpath::run(&Document::default(), &Params::default())
            .unwrap_err()
            .to_string();
        assert!(why.contains("picture"), "{why}");
    }
}
