//! 3D Finishing Toolpath: a ball nose rasters over the model, as low as it can go at each
//! point without cutting into it. The model's top sits at the material surface.

use aspire_core::component::model;
use aspire_core::document::Document;
use aspire_core::geometry::Point;
use aspire_core::glam::DVec3;
use aspire_core::toolpath::{Move, MoveKind, Tool, ToolDatabase, ToolShape, made_by};
use aspire_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct FinishingToolpath3d;

/// How far a straight move may pass under where the tool should be before it is split.
const GOUGE: f64 = 0.002;
/// Most times a move between two samples is halved to keep within `GOUGE`.
const HALVINGS: u32 = 6;

/// Most model cells along the job, however small the tool.
// ponytail: caps the work on big jobs; a job model resolution setting can replace it
const MAX_CELLS: usize = 2000;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
    /// A ball nose follows curves best; its stepover spaces the raster lines
    pub tool: Tool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "3D Finishing".into(),
            tool: ToolDatabase::starter()
                .tools
                .into_iter()
                .find(|t| t.shape == ToolShape::BallNose)
                .expect("the starter tools include a ball nose"),
        }
    }
}

impl Command for FinishingToolpath3d {
    const ID: &'static str = "3d_finishing_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "3D Finishing Toolpath",
            group: "3D Toolpaths",
            icon: "toolpaths/3d_finishing_toolpath.svg",
            shortcut: "",
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "cell and line counts are small and non-negative"
    )]
    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let tool = &p.tool;
        ensure_positive("tool diameter", tool.diameter)?;
        if !(tool.stepover > 0.0 && tool.stepover <= 1.0) {
            return Err(CommandError::InvalidParams(
                "stepover must be more than 0 and at most 1".into(),
            ));
        }
        if doc.components.is_empty() {
            return Err(CommandError::Failed(
                "there is no 3D model to machine: create a shape first".into(),
            ));
        }
        // model cells a twentieth of the tool across, sampled once per cell along each line
        let longest = doc.job.width.max(doc.job.height);
        let cells = ((longest / (tool.diameter / 20.0)).ceil() as usize).clamp(1, MAX_CELLS);
        let surface = model(doc, cells);
        let top = surface
            .heights
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let lowest = surface
            .heights
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
        if top - lowest > doc.job.thickness {
            return Err(CommandError::Failed(format!(
                "the model is {:.1} mm tall but the material is only {:.1} mm thick",
                top - lowest,
                doc.job.thickness
            )));
        }
        let (first, last) = (
            surface.centre(0, 0),
            surface.centre(surface.columns - 1, surface.rows - 1),
        );
        let step = tool.stepover * tool.diameter;
        let lines = ((last.y - first.y) / step).floor() as usize + 1;
        // zigzag along x; each point as low as the tool goes there, the model's top at z = 0
        // ponytail: the step between lines is a straight move; sample it if it ever gouges
        let mut points: Vec<DVec3> = vec![];
        for line in 0..lines {
            let y = first.y + line as f64 * step;
            let drop = |x: f64| {
                let at = Point::new(x, y);
                at.extend(surface.drop_tool(tool, at) - top)
            };
            let mut row: Vec<DVec3> = vec![];
            for column in 0..surface.columns {
                let next = drop(surface.centre(column, 0).x);
                if let Some(&previous) = row.last() {
                    refine(previous, next, &drop, HALVINGS, &mut row);
                }
                row.push(next);
            }
            if line % 2 == 1 {
                row.reverse();
            }
            for point in row {
                // a point in the middle of a level run along the line adds nothing
                let n = points.len();
                if n >= 2 && level_run(points[n - 2], points[n - 1], point) {
                    points.pop();
                }
                points.push(point);
            }
        }
        let safe = doc.job.clearance;
        let above = |p: DVec3| p.truncate().extend(safe);
        let mut moves = vec![
            Move {
                kind: MoveKind::Rapid,
                to: above(points[0]),
            },
            Move {
                kind: MoveKind::Plunge,
                to: points[0],
            },
        ];
        moves.extend(points[1..].iter().map(|&to| Move {
            kind: MoveKind::Cut,
            to,
        }));
        moves.push(Move {
            kind: MoveKind::Rapid,
            to: above(points[points.len() - 1]),
        });
        let mut c = Change::builder(doc);
        c.add_toolpath(made_by::<Self>(
            p,
            p.name.clone(),
            tool.clone(),
            vec![],
            moves,
        ));
        Ok(c.finish())
    }
}

/// Points between `a` and `b` where a straight move would dip more than `GOUGE` under where
/// the tool should be: the edge passing a point beside the line dips between two safe ends,
/// most often where the move climbs or falls, and not always halfway.
fn refine(a: DVec3, b: DVec3, drop: &impl Fn(f64) -> DVec3, halvings: u32, out: &mut Vec<DVec3>) {
    // a level move cannot dip: any bump in the safe height peaks over a sampled cell
    if halvings == 0 || (b.z - a.z).abs() <= GOUGE {
        return;
    }
    let dips = |t: f64, p: DVec3| p.z - (a.z + (b.z - a.z) * t) > GOUGE;
    let at = |t: f64| drop(a.x + (b.x - a.x) * t);
    let middle = at(0.5);
    if dips(0.5, middle) || dips(0.25, at(0.25)) || dips(0.75, at(0.75)) {
        refine(a, middle, drop, halvings - 1, out);
        out.push(middle);
        refine(middle, b, drop, halvings - 1, out);
    }
}

/// Three points along one raster line at one height.
fn level_run(a: DVec3, b: DVec3, c: DVec3) -> bool {
    let same = |u: f64, v: f64| (u - v).abs() < 1e-9;
    same(a.y, b.y) && same(b.y, c.y) && same(a.z, b.z) && same(b.z, c.z)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aspire_core::component::{Combine, Component, model};
    use aspire_core::document::Job;
    use aspire_core::geometry::{Point, Rect};
    use aspire_core::simulation::Material;
    use aspire_core::toolpath::Toolpath;

    /// A 40 x 40 x 10 mm job with an 8 mm high hemisphere in the middle.
    fn domed() -> Document {
        let mut doc = Document {
            job: Job {
                width: 40.0,
                height: 40.0,
                thickness: 10.0,
                ..Job::default()
            },
            ..Document::default()
        };
        let middle = Point::new(20.0, 20.0);
        let area = Rect::new(Point::new(12.0, 12.0), Point::new(28.0, 28.0));
        let dome = Component::from_fn("dome", Combine::Add, area, 0.2, |p| {
            let r = p.distance(middle);
            (r < 8.0).then(|| (64.0 - r * r).sqrt())
        });
        let mut c = aspire_core::Change::builder(&doc);
        c.add_component(dome);
        let change = c.finish();
        doc.apply(&change).unwrap();
        doc
    }

    aspire_core::contract_tests!(FinishingToolpath3d, domed);

    fn finish(doc: &Document, params: &Params) -> Toolpath {
        let mut doc = doc.clone();
        let change = FinishingToolpath3d::run(&doc, params).unwrap();
        doc.apply(&change).unwrap();
        doc.toolpaths.into_values().next().unwrap()
    }

    #[test]
    fn it_follows_the_model_closely_without_cutting_into_it() {
        let doc = domed();
        let params = Params {
            tool: ToolDatabase::starter()
                .get("6 mm Ball Nose")
                .unwrap()
                .clone(),
            ..Params::default()
        };
        let toolpath = finish(&doc, &params);
        // the grid the toolpath was worked out on: cells a twentieth of the tool
        let cells = 134;
        let mut cut = Material::new(&doc.job, cells);
        cut.cut(&toolpath);
        let shape = model(&doc, cells);
        let top = shape
            .heights
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let gouges = cut
            .heights
            .iter()
            .zip(&shape.heights)
            .filter(|&(&z, &h)| z < h - top - 0.01)
            .count();
        assert_eq!(gouges, 0, "cells cut more than 0.01 mm below the model");
        // the top, halfway down the side, and the flat around it are cut close
        for (x, y) in [(20.0, 20.0), (25.0, 20.0), (3.0, 3.0)] {
            let at = Point::new(x, y);
            let (z, model_z) = (cut.at(at).unwrap(), shape.at(at).unwrap() - top);
            assert!(
                z <= model_z + 0.05,
                "({x}, {y}) left at {z}, model at {model_z}"
            );
        }
    }

    #[test]
    fn the_model_must_fit_in_the_material() {
        let mut doc = domed();
        doc.job.thickness = 5.0;
        let why = FinishingToolpath3d::run(&doc, &Params::default())
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        assert!(
            why.contains("8.0 mm tall") && why.contains("5.0 mm thick"),
            "{why}"
        );
    }

    #[test]
    fn it_needs_a_model() {
        let why = FinishingToolpath3d::run(&Document::default(), &Params::default())
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        assert!(why.contains("3D model"), "{why}");
    }
}
