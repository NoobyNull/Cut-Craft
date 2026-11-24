//! 3D Pencil Toolpath: a small ball nose runs along the valleys and inside corners of the
//! model, where the bigger bit that finished it was too round to reach, and cleans out what
//! it left there.

mod valleys;

use cut_craft_core::component::{MachiningBoundary, machining_boundary, placed_model};
use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::geometry::{Point, inside};
use cut_craft_core::glam::DVec3;
use cut_craft_core::progress;
use cut_craft_core::simulation::rest_areas;
use cut_craft_core::toolpath::{
    Drop, Tool, ToolDatabase, ToolShape, Toolpath, cut_points, follow_linked, made_by,
};
use cut_craft_core::{Change, Command, CommandError, Form, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct PencilToolpath3d;

/// Cleans up after a 3D finish: a small ball nose (the pencil bit) runs along each valley and
/// inside corner of the model, where the bigger bit that finished it was too round to get
/// right in, and cuts away what it left there. Use it after a 3D Finishing toolpath, with the
/// finishing bit picked under "Bits that finished before": only what those bits left is cut,
/// so it is quick. Extra passes either side widen the clean-up strip, a stepover apart.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// What the toolpath is called in the toolpath list
    pub name: String,
    /// The pencil bit: a ball nose smaller than the bit that finished before, so it fits into
    /// the corners that bit could not. Its finishing stepover spaces the extra passes
    pub tool: Tool,
    /// The bits that finished the model before this one (usually the 3D Finishing bit): the
    /// pencil bit cuts only where they left material. With none, it runs along every valley
    /// sharper than the angle below
    #[serde(default)]
    #[schemars(title = "Bits that finished before")]
    pub bits_before: Vec<Tool>,
    /// How sharp a valley must be to get a pencil pass: how much the surface bends where its
    /// two sides meet. 90° is a wall meeting the floor; lower picks up gentler valleys too
    #[schemars(title = "Valleys sharper than", extend("x-unit" = "°"), range(min = 5, max = 170))]
    pub sharpest: f64,
    /// Passes beside each valley line, on each side, a stepover apart (the pencil bit's
    /// finishing stepover), to clean a wider strip. 0 cuts the valley line alone. Passes stop
    /// where the bits that finished before already cut
    #[schemars(title = "Extra passes each side", range(min = 0, max = 50))]
    pub extra_passes: u32,
    /// Material the bits before left in strips narrower than this is left alone
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    #[schemars(extend("x-group" = "What the bits before left", "x-hide-when" = {"bits_before.count": 0}))]
    pub minimum_detail: f64,
    /// How far the cut reaches past what the bits before left, so it blends in with no step
    #[schemars(title = "Overlap", extend("x-unit" = "mm"), range(min = 0))]
    #[schemars(extend("x-group" = "What the bits before left", "x-hide-when" = {"bits_before.count": 0}))]
    pub overlap: f64,
    /// Where the tool's tip may go
    #[serde(default)]
    #[schemars(title = "Machining Limit Boundary")]
    #[schemars(extend("x-group" = "Machining Limit Boundary"))]
    pub machining_boundary: MachiningBoundary,
    /// The level whose components make the boundary
    #[serde(default)]
    #[schemars(title = "Level", extend("x-show-when" = {"machining_boundary": "selected_level"}))]
    #[schemars(extend("x-group" = "Machining Limit Boundary"))]
    pub boundary_level: String,
    /// Grows the boundary outward by this much (negative shrinks it)
    #[serde(default)]
    #[schemars(title = "Boundary Offset", extend("x-unit" = "mm"))]
    #[schemars(extend("x-group" = "Machining Limit Boundary"))]
    pub machining_boundary_offset: f64,
}

/// A starter ball nose `diameter` mm across.
fn starter_ball(diameter: f64) -> Tool {
    ToolDatabase::starter()
        .tools
        .into_iter()
        .find(|t| t.shape == ToolShape::BallNose && (t.diameter - diameter).abs() < 1e-9)
        .expect("the starter tools include 3 mm and 6 mm ball noses")
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "3D Pencil".into(),
            // ponytail: our own defaults, the starter 3D Finishing bit before a half-size one
            tool: starter_ball(3.0),
            bits_before: vec![starter_ball(6.0)],
            sharpest: 20.0,
            extra_passes: 3,
            minimum_detail: 0.5,
            overlap: 0.5,
            machining_boundary: MachiningBoundary::Material,
            boundary_level: String::new(),
            machining_boundary_offset: 0.0,
        }
    }
}

impl Command for PencilToolpath3d {
    const ID: &'static str = "3d_pencil_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "3D Pencil Toolpath",
            group: "3D Toolpaths",
            icon: "toolpaths/3d_pencil_toolpath.png",
            shortcut: "",
            form: Form::TOOLPATH,
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let (toolpath, notes) = pencil(doc, p, cut_craft_core::accuracy::get().finishing_3d)?;
        let mut c = Change::builder(doc);
        c.add_toolpath(toolpath);
        for note in notes {
            c.note(note);
        }
        Ok(c.finish())
    }
}

/// The finishing stepover used when the pencil bit's own is not usable.
const FALLBACK_STEPOVER: f64 = 0.1;

/// What to say of the bits picked, and how far apart the extra passes go.
fn bit_notes(doc: &Document, p: &Params) -> (f64, Vec<String>) {
    let tool = &p.tool;
    let units = doc.job.units;
    let mut notes = vec![];
    if tool.shape != ToolShape::BallNose {
        notes.push(format!(
            "{} is not a ball nose: a pencil pass follows curved valleys best with one. \
             Select… a ball nose smaller than the bit that finished before",
            tool.name
        ));
    }
    for before in p
        .bits_before
        .iter()
        .filter(|b| b.diameter <= tool.diameter + 1e-9)
    {
        notes.push(format!(
            "{} ({}) is no bigger than the pencil bit {} ({}), so it leaves little the pencil \
             bit can reach: pick a smaller pencil bit",
            before.name,
            units.length(before.diameter, 2),
            tool.name,
            units.length(tool.diameter, 2)
        ));
    }
    let mut stepover = tool.finishing_stepover();
    if !(stepover > 0.0 && stepover <= 1.0) {
        stepover = FALLBACK_STEPOVER;
        notes.push(format!(
            "{}'s finishing stepover is not between 0% and 100%, so the extra passes are 10% \
             of it apart: set it in the Tool Database",
            tool.name
        ));
    }
    (stepover * tool.diameter, notes)
}

/// The pencil toolpath, every move checked to cut no more than `gouge` mm into the model,
/// and what to warn of. It is made whatever the bits: anything odd is said, not refused.
fn pencil(doc: &Document, p: &Params, gouge: f64) -> Result<(Toolpath, Vec<String>), CommandError> {
    let tool = &p.tool;
    ensure_positive("tool diameter", tool.diameter)?;
    let (step, mut notes) = bit_notes(doc, p);
    progress::stage("Placing the model");
    let surface = placed_model(doc, tool)?;
    // where every bit before left material, each worked out on the pencil bit's grid
    let mut left = vec![];
    if !p.bits_before.is_empty() {
        progress::stage("Finding what the bits before left");
    }
    for before in &p.bits_before {
        progress::stop_if_cancelled()?;
        ensure_positive("diameter of a bit that finished before", before.diameter)?;
        let surface_left = surface.left_by(before);
        left.push(rest_areas(
            &surface,
            &surface_left,
            p.minimum_detail,
            p.overlap + tool.diameter / 2.0,
        ));
    }
    let regions = machining_boundary(
        doc,
        p.machining_boundary,
        p.machining_boundary_offset,
        &p.boundary_level,
    )?;
    let rings: Vec<Vec<Point>> = regions.iter().map(cut_points).collect();
    let material = doc.job.area();
    let wanted =
        |at: Point| material.contains(at) && inside(&rings, at) && left.iter().all(|area| area(at));
    progress::report(0.2);
    progress::stop_if_cancelled()?;
    progress::stage("Finding the valleys");
    let lines = valley_lines(&surface, tool, p.sharpest, &wanted);
    if lines.is_empty() {
        notes.push(format!(
            "Found no valleys sharper than {:.0}° {}: nothing for {} to clean up. Lower \
             \"Valleys sharper than\", or pick a smaller pencil bit",
            p.sharpest,
            if p.bits_before.is_empty() {
                "on the model"
            } else {
                "where the bits before left material"
            },
            tool.name
        ));
    }
    progress::report(0.4);
    progress::stop_if_cancelled()?;
    progress::stage("Following the valleys");
    let passes = in_cutting_order(&lines, p.extra_passes, step);
    let drop = Drop {
        grid: &surface,
        tool,
        tolerance: gouge,
        floor: f64::NEG_INFINITY,
        ceiling: f64::INFINITY,
        over_ceiling: false,
    };
    let runs: Vec<Vec<DVec3>> = drop.along_all(&passes, &wanted);
    progress::report(0.8);
    progress::stage("Linking the passes");
    let peaks = surface.peaks();
    let clear = |a, b| surface.overcut_on(tool, a, b, &peaks) <= gouge;
    let moves = follow_linked(&runs, doc.job.clearance, 2.0 * step, 0.0, &clear);
    let toolpath = made_by::<PencilToolpath3d>(
        p,
        p.name.clone(),
        tool.clone(),
        boundary_vectors(doc, p.machining_boundary),
        moves,
    );
    Ok((toolpath, notes))
}

/// The valley lines over `surface` sharper than `sharpest` degrees for `tool`'s tip, where
/// `wanted` says yes, each smoothed and with whether it closes on itself.
fn valley_lines(
    surface: &cut_craft_core::simulation::Material,
    tool: &Tool,
    sharpest: f64,
    wanted: &dyn Fn(Point) -> bool,
) -> Vec<(Vec<Point>, bool)> {
    let tips = cut_craft_core::compute::compute().drop_everywhere(surface, tool);
    let grid = valleys::Grid {
        values: &tips,
        columns: surface.columns,
        rows: surface.rows,
        cell: surface.cell,
        origin: surface.origin,
    };
    let creases = valleys::creases(&grid, sharpest.to_radians(), &|i| wanted(grid.centre(i)));
    let thinned = valleys::thin(&creases, grid.columns, grid.rows);
    valleys::chains(&thinned, grid.columns, grid.rows)
        .into_iter()
        .map(|chain| {
            let points: Vec<Point> = chain.cells.iter().map(|&i| grid.centre(i)).collect();
            (valleys::smoothed(&points, chain.closed), chain.closed)
        })
        // a speck is no valley
        .filter(|(points, closed)| valleys::length(points, *closed) >= tool.diameter / 2.0)
        .collect()
}

/// The passes along each valley line, in the order they cut: the extra passes on one side
/// from the outside in, then the other side's, then the valley line itself, each going back
/// the way the one before came so they link without lifting. Closed lines stay closed.
fn in_cutting_order(lines: &[(Vec<Point>, bool)], extra: u32, step: f64) -> Vec<Vec<Point>> {
    let mut out = vec![];
    for (points, closed) in lines {
        let close = |mut line: Vec<Point>| {
            if *closed && let Some(&first) = line.first() {
                line.push(first);
            }
            line
        };
        let mut passes: Vec<Vec<Point>> = vec![];
        for side in [1.0, -1.0] {
            for k in (1..=extra).rev() {
                passes.push(valleys::offset(points, *closed, side * f64::from(k) * step));
            }
        }
        passes.push(points.clone());
        // back and forth, so each starts near where the last ended
        for (i, pass) in passes.into_iter().enumerate() {
            let mut pass = close(pass);
            if i % 2 == 1 && !*closed {
                pass.reverse();
            }
            out.push(pass);
        }
    }
    out
}

/// The closed vectors a Selected Vector(s) boundary is made from, kept with the toolpath so
/// recalculating uses them again (and changing them marks it out of date).
fn boundary_vectors(doc: &Document, limit: MachiningBoundary) -> Vec<EntityId> {
    match limit {
        MachiningBoundary::SelectedVectors => doc.selected_paths(|path| path.closed).0,
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::component::model;
    use cut_craft_core::simulation::Material;
    use cut_craft_core::testing::domed;
    use cut_craft_core::toolpath::MoveKind;

    cut_craft_core::contract_tests!(PencilToolpath3d, domed);

    const MIDDLE: Point = Point::new(20.0, 20.0);

    /// The toolpath `params` make on `doc`, and the notes said.
    fn made(doc: &Document, params: &Params) -> (Toolpath, Vec<String>) {
        let change = PencilToolpath3d::run(doc, params).unwrap();
        let notes = change.notes.clone();
        let mut doc = doc.clone();
        doc.apply(&change).unwrap();
        (doc.toolpaths.into_values().next().unwrap(), notes)
    }

    fn cuts(toolpath: &Toolpath) -> Vec<DVec3> {
        toolpath
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Cut)
            .map(|m| m.to)
            .collect()
    }

    #[test]
    fn it_runs_round_the_foot_of_the_dome_and_nowhere_else() {
        // the dome is 8 mm round with steep sides meeting the flat: the 6 mm ball leaves a
        // fillet there, which the 3 mm ball's tip reaches 1.5 mm out from the dome
        let params = Params {
            extra_passes: 0,
            ..Params::default()
        };
        let (toolpath, notes) = made(&domed(), &params);
        assert!(notes.is_empty(), "{notes:?}");
        let cuts = cuts(&toolpath);
        assert!(cuts.len() > 20, "a ring of cuts: {}", cuts.len());
        for p in &cuts {
            let r = p.truncate().distance(MIDDLE);
            assert!(
                (r - 9.5).abs() < 0.5,
                "{p} is {r} mm out, not round the foot"
            );
        }
        // all the way round: every quarter has some
        for (sx, sy) in [(1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)] {
            assert!(
                cuts.iter()
                    .any(|p| (p.x - 20.0) * sx > 0.0 && (p.y - 20.0) * sy > 0.0),
                "a quarter has none"
            );
        }
    }

    #[test]
    fn extra_passes_widen_the_strip_but_stay_where_the_bit_before_left_material() {
        let one = made(
            &domed(),
            &Params {
                extra_passes: 0,
                ..Params::default()
            },
        )
        .0;
        let more = made(&domed(), &Params::default()).0;
        let reach = |t: &Toolpath| {
            cuts(t)
                .iter()
                .map(|p| p.truncate().distance(MIDDLE))
                .fold(0.0, f64::max)
        };
        assert!(reach(&more) > reach(&one) + 0.5, "the passes go wider");
        // the 6 mm ball leaves nothing past about 11 mm out (8 + its radius), give or take
        // the overlap and the pencil bit's own radius
        assert!(
            reach(&more) < 8.0 + 3.0 + 0.5 + 1.5 + 0.5,
            "{}",
            reach(&more)
        );
    }

    #[test]
    fn it_never_cuts_into_the_model() {
        let (toolpath, _) = made(&domed(), &Params::default());
        let doc = domed();
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
    }

    #[test]
    fn with_no_bits_before_it_follows_every_valley() {
        let params = Params {
            bits_before: vec![],
            extra_passes: 0,
            ..Params::default()
        };
        let (toolpath, notes) = made(&domed(), &params);
        assert!(notes.is_empty(), "{notes:?}");
        assert_ne!(cuts(&toolpath), []);
    }

    #[test]
    fn odd_bits_are_warned_of_not_refused() {
        let params = Params {
            tool: starter_ball(6.0),
            bits_before: vec![starter_ball(3.0)],
            ..Params::default()
        };
        let (_, notes) = made(&domed(), &params);
        assert!(
            notes
                .iter()
                .any(|n| n.contains("no bigger than the pencil bit")),
            "{notes:?}"
        );
        let end_mill = ToolDatabase::starter()
            .tools
            .into_iter()
            .find(|t| t.shape == ToolShape::EndMill)
            .unwrap();
        let (_, notes) = made(
            &domed(),
            &Params {
                tool: end_mill,
                ..Params::default()
            },
        );
        assert!(
            notes.iter().any(|n| n.contains("not a ball nose")),
            "{notes:?}"
        );
    }

    #[test]
    fn nothing_to_clean_up_is_said_and_still_makes_the_toolpath() {
        let params = Params {
            sharpest: 170.0,
            ..Params::default()
        };
        let (toolpath, notes) = made(&domed(), &params);
        assert_eq!(cuts(&toolpath), []);
        assert!(
            notes.iter().any(|n| n.contains("Found no valleys")),
            "{notes:?}"
        );
    }

    #[test]
    fn passes_go_outside_in_then_the_valley_line_last() {
        let line = vec![Point::new(0.0, 0.0), Point::new(10.0, 0.0)];
        let passes = in_cutting_order(&[(line, false)], 2, 0.5);
        let ys: Vec<f64> = passes.iter().map(|p| p[0].y).collect();
        assert_eq!(ys, [1.0, 0.5, -1.0, -0.5, 0.0]);
        // back and forth
        assert!(passes[0][0].x < passes[1][0].x);
    }

    #[test]
    fn a_selected_vectors_boundary_is_kept_so_the_toolpath_recalculates() {
        let mut doc = domed();
        let mut c = Change::builder(&doc);
        let id = c.add(cut_craft_core::testing::square(0.0, 0.0, 40.0));
        c.select([id]);
        doc.apply(&c.finish()).unwrap();
        let p = Params {
            machining_boundary: MachiningBoundary::SelectedVectors,
            ..Params::default()
        };
        doc.apply(&PencilToolpath3d::run(&doc, &p).unwrap())
            .unwrap();
        let (&tp, made) = doc.toolpaths.iter().next().unwrap();
        assert_eq!(made.vectors, [id]);
        let mut r = cut_craft_core::Registry::default();
        r.add::<PencilToolpath3d>();
        let again = r.recalculate(&doc, tp);
        assert!(again.is_ok(), "{:?}", again.err());
    }

    /// A job with valleys to clean: four domes run together, and a round-topped wall.
    fn valleys_job() -> Document {
        use cut_craft_core::component::{Combine, Component};
        use cut_craft_core::geometry::Rect;
        let mut doc = Document {
            job: cut_craft_core::document::Job {
                width: 60.0,
                height: 40.0,
                thickness: 12.0,
                ..cut_craft_core::document::Job::default()
            },
            ..Document::default()
        };
        let area = Rect::new(Point::new(2.0, 2.0), Point::new(58.0, 38.0));
        let shape = Component::from_fn("valleys", Combine::Add, area, 0.1, |p| {
            let dome = |c: Point| {
                let r = p.distance(c);
                (r < 8.0).then(|| (1.0 - (r / 8.0).powi(2)).sqrt() * 6.0)
            };
            let domes = [(14.0, 14.0), (26.0, 14.0), (14.0, 26.0), (26.0, 26.0)]
                .map(|(x, y)| dome(Point::new(x, y)))
                .into_iter()
                .flatten()
                .fold(None, |a: Option<f64>, b| Some(a.map_or(b, |a| a.max(b))));
            // a wall along the right, its top rounded, standing on the floor
            let wall = (p.x > 44.0 && p.x < 54.0).then(|| {
                let t = (p.x - 49.0) / 5.0;
                (1.0 - t * t).max(0.0).sqrt() * 5.0
            });
            match (domes, wall) {
                (Some(a), Some(b)) => Some(a.max(b)),
                (a, b) => a.or(b).or(Some(0.0)),
            }
        });
        let mut c = Change::builder(&doc);
        c.add_component(shape);
        doc.apply(&c.finish()).unwrap();
        doc
    }

    #[test]
    fn the_valleys_between_domes_and_at_the_foot_of_a_wall_get_passes() {
        let doc = valleys_job();
        let (toolpath, notes) = made(
            &doc,
            &Params {
                extra_passes: 0,
                ..Params::default()
            },
        );
        assert!(notes.is_empty(), "{notes:?}");
        let cuts = cuts(&toolpath);
        // between two domes, along x = 20
        assert!(
            cuts.iter()
                .any(|p| (p.x - 20.0).abs() < 0.5 && (p.y - 14.0).abs() < 2.0)
        );
        // at the foot of the wall's round top (it meets the floor at 44 and 54)
        // (a straight level run keeps only its ends)
        let foot: Vec<&DVec3> = cuts.iter().filter(|p| p.x > 42.0 && p.x < 46.0).collect();
        assert!(foot.iter().any(|p| p.y < 10.0) && foot.iter().any(|p| p.y > 30.0));
        // never along the top of the wall
        assert!(!cuts.iter().any(|p| (p.x - 49.0).abs() < 2.0));
    }
}
