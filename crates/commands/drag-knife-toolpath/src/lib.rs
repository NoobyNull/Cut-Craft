//! Drag Knife Toolpath: a trailing blade cuts along the selected vectors. At sharp corners
//! the knife runs past the corner by its blade offset, lifts to the swivel depth and swings
//! round the corner so the blade faces the new direction before cutting on.

use cut_craft_core::document::Document;
use cut_craft_core::drag_knife::{Overrides, surface_top};
use cut_craft_core::geometry::Point;
use cut_craft_core::glam::DVec3;
use cut_craft_core::toolpath::{Move, MoveKind, Tool, cut_points, made_by, project_onto_model};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct DragKnifeToolpath;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// What the toolpath is called in the toolpath list
    pub name: String,
    /// From the knife's pivot to its tip (from the maker)
    #[schemars(extend("x-unit" = "mm", "x-dim" = "O"), range(min = 0))]
    #[schemars(extend("x-group" = "Knife"))]
    pub blade_offset: f64,
    /// Corners turning less than this are cut straight through, without a swivel
    #[schemars(extend("x-unit" = "°", "x-dim" = "A"), range(min = 0, max = 180))]
    #[schemars(extend("x-group" = "Knife"))]
    pub tolerance_angle: f64,
    /// How far the knife holder's spring lets the blade push back into it (from the
    /// holder's maker). Zero Z with the blade tip just touching the material: the knife then
    /// presses down by a share of this
    #[serde(default = "spring_travel")]
    #[schemars(extend("x-unit" = "mm", "x-dim" = "T"), range(min = 0))]
    #[schemars(extend("x-group" = "Knife"))]
    pub spring_travel: f64,
    /// How hard the blade presses while cutting, as a share of the spring travel: more for
    /// thicker or tougher material. The knife goes down this share of the spring travel
    /// below the top
    #[serde(default = "pressure")]
    #[schemars(extend("x-unit" = "%", "x-dim" = "P"), range(min = 0, max = 100))]
    #[schemars(extend("x-group" = "Cutting Depths"))]
    pub pressure: f64,
    /// How hard it presses while it turns at a corner, as a share of the spring travel: less
    /// than the cutting pressure, so the blade can swivel
    #[serde(default = "swivel_pressure")]
    #[schemars(extend("x-unit" = "%", "x-dim" = "P2"), range(min = 0, max = 100))]
    #[schemars(extend("x-group" = "Cutting Depths"))]
    pub swivel_pressure: f64,
    /// Corners turning more than this are sharp: the knife cuts on through the corner by
    /// the overshoot, lifts out and starts again along the next edge. Gentler corners turn
    /// the blade without lifting it out
    #[serde(default = "sharp_angle")]
    #[schemars(title = "Sharp Corner Angle")]
    #[schemars(extend("x-unit" = "°", "x-dim" = "A2"), range(min = 0, max = 180))]
    #[schemars(extend("x-group" = "Knife"))]
    pub sharp_angle: f64,
    /// How far the cut runs on past a sharp corner (and past the start of a closed shape),
    /// so the cuts meet cleanly; never into another vector
    #[serde(default = "overshoot")]
    #[schemars(extend("x-unit" = "mm", "x-dim" = "L"), range(min = 0))]
    #[schemars(extend("x-group" = "Knife"))]
    pub overshoot: f64,
    /// Equal passes down to the cut depth
    #[schemars(range(min = 1))]
    #[schemars(extend("x-group" = "Cutting Depths"))]
    pub passes: u32,
    /// Feed and plunge rates (switch the spindle off)
    pub tool: Tool,
    /// A knife needs a flat surface, so it must be the first toolpath or come straight after
    /// a Surfacing toolpath. Tick this only if you know the surface under it is flat anyway
    /// (for example flattened in another job)
    #[serde(default)]
    #[schemars(title = "Cut on an uneven surface anyway")]
    pub allow_uneven_surface: bool,
    /// Drop the knife's cut onto the 3D model, so it cuts its depth below the model's
    /// surface: for domes and other curved work
    #[serde(default)]
    #[schemars(title = "Project toolpath onto 3D Model")]
    pub project_onto_model: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Drag Knife".into(),
            blade_offset: 0.25,
            tolerance_angle: 10.0,
            sharp_angle: sharp_angle(),
            overshoot: overshoot(),
            spring_travel: spring_travel(),
            pressure: pressure(),
            swivel_pressure: swivel_pressure(),
            passes: 1,
            tool: Tool {
                name: "Drag Knife".into(),
                diameter: 0.5,
                spindle_speed: 0.0,
                ..Tool::default()
            },
            allow_uneven_surface: false,
            project_onto_model: false,
        }
    }
}

fn sharp_angle() -> f64 {
    60.0
}

fn overshoot() -> f64 {
    0.5
}

fn spring_travel() -> f64 {
    2.0
}

fn pressure() -> f64 {
    50.0
}

fn swivel_pressure() -> f64 {
    12.5
}

/// Degrees per straight move along a swivel arc.
const ARC_STEP: f64 = 10.0;

/// Moves round an arc about `centre` from `from` to `to` (both at the same radius), the
/// short way round, at height `z`.
fn swivel(centre: Point, from: Point, to: Point, z: f64) -> impl Iterator<Item = Move> {
    let (a, b) = (from - centre, to - centre);
    let turn = a.angle_to(b);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a few steps"
    )]
    let steps = (turn.abs().to_degrees() / ARC_STEP).ceil().max(1.0) as u32;
    (1..=steps).map(move |i| Move {
        kind: MoveKind::Cut,
        to: (centre + Point::from_angle(turn * f64::from(i) / f64::from(steps)).rotate(a))
            .extend(z),
    })
}

/// The blade, down with its tip at `tip` and facing `from`, rising to `lift` and swung
/// about its tip to face `to`, then pressed back down to `cut`.
fn swing(
    moves: &mut Vec<Move>,
    tip: Point,
    (from, to): (Point, Point),
    offset: f64,
    (lift, cut): (f64, f64),
) {
    let (from, to) = (tip + offset * from, tip + offset * to);
    moves.push(Move {
        kind: MoveKind::Cut,
        to: from.extend(lift),
    });
    moves.extend(swivel(tip, from, to, lift));
    moves.push(Move {
        kind: MoveKind::Plunge,
        to: to.extend(cut),
    });
}

/// How a pass is cut: its heights, and what each overshoot has room for.
struct Pass<'a> {
    /// The top the knife rides on.
    top: f64,
    /// How deep this pass cuts (negative z).
    cut: f64,
    /// The safe height to lift out to.
    safe: f64,
    /// Whether it comes down from the safe height at the start (else it is already down).
    fresh: bool,
    /// Whether it is a closed shape's last pass: it cuts on past the start to close the cut.
    closing: bool,
    /// How far the tip may run on from a point in a direction, up to the overshoot, before
    /// it would cut into another vector.
    room: &'a dyn Fn(Point, Point) -> f64,
}

/// How far short of another vector an overshoot stops, mm.
const OVERSHOOT_GAP: f64 = 0.25;

/// Every span of `paths`, for overshoots to stop short of.
fn spans(paths: &[&cut_craft_core::geometry::Path]) -> Vec<(Point, Point)> {
    paths
        .iter()
        .flat_map(|path| {
            let points = cut_points(path);
            let mut spans: Vec<(Point, Point)> = points.windows(2).map(|w| (w[0], w[1])).collect();
            if path.closed && points.len() > 2 {
                spans.push((points[points.len() - 1], points[0]));
            }
            spans
        })
        .collect()
}

/// How far from `from` along unit `dir` the tip can run, up to `want`, stopping
/// [`OVERSHOOT_GAP`] short of any of `spans` it would cross (those it starts on don't count).
fn room(spans: &[(Point, Point)], from: Point, dir: Point, want: f64) -> f64 {
    let hit = spans
        .iter()
        .filter_map(|&(a, b)| {
            let along = b - a;
            let denominator = dir.perp_dot(along);
            if denominator.abs() < 1e-12 {
                return None;
            }
            let t = (a - from).perp_dot(along) / denominator;
            let u = (a - from).perp_dot(dir) / denominator;
            (t > 1e-6 && (-1e-9..=1.0 + 1e-9).contains(&u)).then_some(t)
        })
        .fold(f64::INFINITY, f64::min);
    want.min((hit - OVERSHOOT_GAP).max(0.0))
}

impl Params {
    /// How far below the top the knife presses down while cutting, mm.
    fn cut_depth(&self) -> f64 {
        self.spring_travel * self.pressure / 100.0
    }

    /// How far below the top it presses while it swivels, mm.
    fn swivel_depth(&self) -> f64 {
        self.spring_travel * self.swivel_pressure / 100.0
    }

    /// One pass along `points` (`how` says at what depth), starting with the blade facing
    /// `facing`; returns the direction the blade faces at the end. Points are where the
    /// knife's pivot goes; its tip trails the blade offset behind.
    ///
    /// Gentle corners (turning less than the tolerance angle) are cut straight through.
    /// Standard corners: the blade stays in the material, rises to the swivel depth and
    /// turns about its tip to the new direction. Sharp corners (turning more than the sharp
    /// corner angle): the tip cuts on through the corner by the overshoot, lifts out, and
    /// starts again the overshoot back along the next edge, so the two cuts cross and the
    /// corner comes out clean. No overshoot runs into another vector.
    fn pass(
        &self,
        points: &[Point],
        closed: bool,
        how: &Pass,
        facing: Point,
    ) -> (Point, Vec<Move>) {
        let (cut, safe) = (how.cut, how.safe);
        let lift = how.top - self.swivel_depth();
        let off = self.blade_offset;
        let tolerance = self.tolerance_angle.to_radians();
        let sharp = self.sharp_angle.to_radians();
        let mut moves = vec![];
        let mut ring = points.to_vec();
        if closed {
            ring.push(points[0]);
        }
        let dirs: Vec<Point> = ring.windows(2).map(|w| (w[1] - w[0]).normalize()).collect();
        let start = ring[0];
        // comes down from above with the tip at `tip`, swung round from `facing` to `dir`
        let enter = |moves: &mut Vec<Move>, tip: Point, facing: Point, dir: Point| {
            let from = tip + off * facing;
            moves.push(Move {
                kind: MoveKind::Rapid,
                to: from.extend(safe),
            });
            if facing.angle_to(dir).abs() < 1e-9 {
                // already facing the way: straight down
                moves.push(Move {
                    kind: MoveKind::Plunge,
                    to: from.extend(cut),
                });
                return;
            }
            moves.push(Move {
                kind: MoveKind::Plunge,
                to: from.extend(lift),
            });
            moves.extend(swivel(tip, from, tip + off * dir, lift));
            moves.push(Move {
                kind: MoveKind::Plunge,
                to: (tip + off * dir).extend(cut),
            });
        };
        let turn = facing.angle_to(dirs[0]).abs();
        if how.fresh {
            // the tip goes down on the start, the blade swung round to face along the first
            // span
            enter(&mut moves, start, facing, dirs[0]);
        } else if turn > tolerance {
            // already down with the tip at the start: lift and swing about it
            swing(&mut moves, start, (facing, dirs[0]), off, (lift, cut));
        }
        for (i, &dir) in dirs.iter().enumerate() {
            let corner = ring[i + 1];
            let next = dirs.get(i + 1).copied();
            let turn = next.map_or(0.0, |next| dir.angle_to(next).abs());
            match next {
                Some(next) if turn > sharp.max(tolerance) => {
                    // through past the corner, out, and in again behind it on the next edge
                    let over = (how.room)(corner, dir);
                    let back = (how.room)(corner, -next);
                    let through = corner + (off + over) * dir;
                    moves.push(Move {
                        kind: MoveKind::Cut,
                        to: through.extend(cut),
                    });
                    moves.push(Move {
                        kind: MoveKind::Rapid,
                        to: through.extend(safe),
                    });
                    enter(&mut moves, corner - back * next, dir, next);
                }
                Some(next) if turn > tolerance => {
                    moves.push(Move {
                        kind: MoveKind::Cut,
                        to: (corner + off * dir).extend(cut),
                    });
                    swing(&mut moves, corner, (dir, next), off, (lift, cut));
                }
                Some(_) => moves.push(Move {
                    kind: MoveKind::Cut,
                    to: corner.extend(cut),
                }),
                // the end: the tip right to it, and on through the start of a closed shape
                // to close the cut
                None => {
                    let over = if how.closing {
                        (how.room)(corner, dir)
                    } else {
                        0.0
                    };
                    moves.push(Move {
                        kind: MoveKind::Cut,
                        to: (corner + (off + over) * dir).extend(cut),
                    });
                }
            }
        }
        (*dirs.last().expect("a path with a span"), moves)
    }
}

impl Command for DragKnifeToolpath {
    const ID: &'static str = "drag_knife_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Drag Knife Toolpath",
            group: "Toolpath Operations",
            icon: "toolpaths/drag_knife_toolpath.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("blade offset", p.blade_offset)?;
        ensure_positive("spring travel", p.spring_travel)?;
        if !(p.pressure > 0.0 && p.pressure <= 100.0) {
            return Err(CommandError::InvalidParams(
                "set the pressure between 1 and 100% of the spring travel".into(),
            ));
        }
        if !(p.swivel_pressure >= 0.0 && p.swivel_pressure < p.pressure) {
            return Err(CommandError::InvalidParams(
                "set the swivel pressure less than the cutting pressure".into(),
            ));
        }
        if p.passes == 0 {
            return Err(CommandError::InvalidParams(
                "set passes to 1 or more".into(),
            ));
        }
        let (ids, paths) = doc.selected_paths(|_| true);
        if ids.is_empty() {
            return Err(CommandError::Failed("select the vectors to cut".into()));
        }
        let overrides = Overrides {
            follows_model: p.project_onto_model,
            uneven_allowed: p.allow_uneven_surface,
        };
        let top = surface_top(doc, &p.name, &doc.toolpaths_before_made(), &ids, overrides)
            .map_err(CommandError::Failed)?;
        let safe = doc.job.clearance;
        let all: Vec<&cut_craft_core::geometry::Path> = doc.vectors.values().collect();
        let all = spans(&all);
        let room = |from: Point, dir: Point| room(&all, from, dir, p.overshoot.max(0.0));
        // the blade starts out facing along +X
        let mut facing = Point::X;
        let mut moves = vec![];
        for path in &paths {
            let mut points = cut_points(path);
            points.dedup_by(|a, b| a.distance(*b) < 1e-9);
            if points.len() < 2 {
                continue;
            }
            let mut contour = vec![];
            for pass in 1..=p.passes {
                let cut = top - p.cut_depth() * f64::from(pass) / f64::from(p.passes);
                // an open vector starts each pass again from above; a closed one carries on
                let fresh = pass == 1 || !path.closed;
                if fresh && pass > 1 {
                    let at = contour.last().map_or(DVec3::ZERO, |m: &Move| m.to);
                    contour.push(Move {
                        kind: MoveKind::Rapid,
                        to: at.truncate().extend(safe),
                    });
                }
                let how = Pass {
                    top,
                    cut,
                    safe,
                    fresh,
                    closing: path.closed && pass == p.passes,
                    room: &room,
                };
                let (now, cut) = p.pass(&points, path.closed, &how, facing);
                facing = now;
                contour.extend(cut);
            }
            let end = contour.last().map_or(DVec3::ZERO, |m| m.to);
            contour.push(Move {
                kind: MoveKind::Rapid,
                to: end.truncate().extend(safe),
            });
            moves.extend(contour);
        }
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
    use cut_craft_core::geometry::Path;
    use cut_craft_core::testing::{made_toolpath, square};
    use cut_craft_core::toolpath::Toolpath;

    cut_craft_core::contract_tests!(DragKnifeToolpath);

    /// A 3 mm cut (all of a 3 mm spring travel), swivelling 0.5 mm down.
    fn deep() -> Params {
        Params {
            spring_travel: 3.0,
            pressure: 100.0,
            swivel_pressure: 50.0 / 3.0,
            ..Params::default()
        }
    }

    fn cut(paths: Vec<Path>, p: &Params) -> Toolpath {
        made_toolpath::<DragKnifeToolpath>(paths, p).unwrap()
    }

    #[test]
    fn standard_corners_turn_the_blade_without_lifting_it_out() {
        // no corner of the square counts as sharp
        let p = Params {
            sharp_angle: 120.0,
            ..deep()
        };
        let t = cut(vec![square(0.0, 0.0, 10.0)], &p);
        let lifts = t
            .moves
            .windows(2)
            .filter(|w| w[0].to.z < -2.9 && (w[1].to.z + p.swivel_depth()).abs() < 1e-9)
            .count();
        assert_eq!(
            lifts, 3,
            "no swivel to start: the blade already faces along x"
        );
        assert!(
            t.moves[1..t.moves.len() - 1]
                .iter()
                .all(|m| m.kind != MoveKind::Rapid),
            "never lifted out between start and end"
        );
        // a swivel overshoots the corner by the blade offset first
        assert!(
            t.moves
                .iter()
                .any(|m| m.to.distance(DVec3::new(10.25, 0.0, -3.0)) < 1e-9)
        );
        assert!(
            t.moves
                .iter()
                .any(|m| m.to.distance(DVec3::new(10.0, 0.25, -3.0)) < 1e-9)
        );
    }

    #[test]
    fn gentle_corners_are_cut_straight_through() {
        let p = deep();
        let bend = Path::polyline(
            &[
                Point::new(0.0, 0.0),
                Point::new(10.0, 0.0),
                Point::new(20.0, 1.0),
            ],
            false,
        );
        let t = cut(vec![bend], &p);
        assert!(
            t.moves
                .iter()
                .all(|m| (m.to.z + p.swivel_depth()).abs() > 1e-9)
        );
    }

    #[test]
    fn the_blade_is_swung_round_before_a_first_cut_off_the_x_axis() {
        let up = Path::polyline(&[Point::new(0.0, 0.0), Point::new(0.0, 10.0)], false);
        let p = deep();
        let t = cut(vec![up], &p);
        // swings about the start, where the tip goes down
        let swivels = t
            .moves
            .iter()
            .filter(|m| (m.to.z + p.swivel_depth()).abs() < 1e-9)
            .count();
        assert!(swivels > 2, "{swivels}");
        let first_cut = t.moves.iter().position(|m| m.to.z < -1.0).unwrap();
        // the pivot leads the tip by the blade offset
        let pivot = t.moves[first_cut].to.truncate();
        assert!(pivot.distance(Point::new(0.0, 0.25)) < 1e-9, "{pivot}");
    }

    #[test]
    fn passes_split_the_cut_depth_evenly() {
        let p = Params {
            passes: 3,
            ..deep()
        };
        let t = cut(vec![square(0.0, 0.0, 10.0)], &p);
        let mut depths: Vec<f64> = t
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Plunge && m.to.z < -0.6)
            .map(|m| m.to.z)
            .collect();
        depths.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        assert!(
            depths.iter().any(|z| (z + 1.0).abs() < 1e-9)
                && depths.iter().any(|z| (z + 3.0).abs() < 1e-9)
        );
    }

    #[test]
    fn rapids_only_happen_above_the_material() {
        let open = Path::polyline(&[Point::new(0.0, 0.0), Point::new(0.0, 10.0)], false);
        let p = Params {
            passes: 2,
            ..deep()
        };
        let t = cut(vec![square(0.0, 0.0, 10.0), open], &p);
        let safe = cut_craft_core::document::Job::default().clearance;
        assert!(
            t.moves
                .iter()
                .filter(|m| m.kind == MoveKind::Rapid)
                .all(|m| (m.to.z - safe).abs() < 1e-9)
        );
    }

    #[test]
    fn the_swivel_pressure_must_be_less_than_the_cutting_pressure() {
        let p = Params {
            swivel_pressure: 60.0,
            ..Params::default()
        };
        let why = made_toolpath::<DragKnifeToolpath>(vec![square(0.0, 0.0, 10.0)], &p).unwrap_err();
        assert!(why.contains("swivel pressure"), "{why}");
    }

    /// A job with a square to cut and `before` already in the toolpath list.
    fn job_with(before: &[(&str, serde_json::Value)]) -> Document {
        let mut doc = cut_craft_core::testing::document_with(vec![square(0.0, 0.0, 10.0)]);
        for (i, (strategy, params)) in before.iter().enumerate() {
            let mut t = cut(vec![square(50.0, 50.0, 5.0)], &deep());
            t.name = format!("{strategy} {i}");
            t.strategy = (*strategy).into();
            t.params = params.clone();
            doc.toolpaths
                .insert(cut_craft_core::document::EntityId(1000 + i as u64), t);
        }
        doc
    }

    fn deepest(t: &Toolpath) -> f64 {
        t.moves.iter().map(|m| m.to.z).fold(f64::INFINITY, f64::min)
    }

    #[test]
    fn the_depth_is_the_pressure_as_a_share_of_the_spring_travel() {
        let p = Params {
            spring_travel: 4.0,
            pressure: 25.0,
            swivel_pressure: 10.0,
            ..Params::default()
        };
        assert!((p.cut_depth() - 1.0).abs() < 1e-9);
        assert!((p.swivel_depth() - 0.4).abs() < 1e-9);
        // a new one presses half the spring travel, never the material's thickness
        let mut doc = Document::default();
        doc.job.thickness = 49.5;
        let p = DragKnifeToolpath::initial_params(&doc);
        assert!((p.cut_depth() - 1.0).abs() < 1e-9, "{}", p.cut_depth());
    }

    #[test]
    fn it_cuts_its_depth_below_the_untouched_surface_when_first() {
        let doc = job_with(&[]);
        let change = DragKnifeToolpath::run(&doc, &Params::default()).unwrap();
        let mut done = doc.clone();
        done.apply(&change).unwrap();
        let t = done.toolpaths.values().next().unwrap();
        assert!((deepest(t) + 1.0).abs() < 1e-9, "{}", deepest(t));
    }

    #[test]
    fn after_any_other_cut_calculate_refuses_and_says_how_to_fix_it() {
        let doc = job_with(&[("pocket_toolpath", serde_json::json!({}))]);
        let why = DragKnifeToolpath::run(&doc, &Params::default())
            .unwrap_err()
            .to_string();
        assert!(why.contains("needs a flat surface"), "{why}");
        assert!(why.contains("Move Drag Knife to the top"), "{why}");
        // ticking the override cuts anyway
        let anyway = Params {
            allow_uneven_surface: true,
            ..Params::default()
        };
        assert!(DragKnifeToolpath::run(&doc, &anyway).is_ok());
    }

    #[test]
    fn after_surfacing_it_rides_on_the_flattened_top() {
        let doc = job_with(&[(
            "surfacing_toolpath",
            serde_json::json!({ "area": "whole_material", "depth": 0.5 }),
        )]);
        let change = DragKnifeToolpath::run(&doc, &Params::default()).unwrap();
        let mut done = doc.clone();
        done.apply(&change).unwrap();
        let t = done.toolpaths.values().last().unwrap();
        assert!((deepest(t) + 1.5).abs() < 1e-9, "{}", deepest(t));
        let swivels = t
            .moves
            .iter()
            .filter(|m| (m.to.z + 0.75).abs() < 1e-9)
            .count();
        assert!(swivels > 0, "swivels at the swivel depth below the new top");
    }

    #[test]
    fn made_again_it_looks_only_at_the_toolpaths_above_it() {
        let mut doc = job_with(&[("pocket_toolpath", serde_json::json!({}))]);
        // the knife at the top, the pocket under it
        doc.remaking = Some(cut_craft_core::document::EntityId(1));
        assert!(DragKnifeToolpath::run(&doc, &Params::default()).is_ok());
        doc.remaking = Some(cut_craft_core::document::EntityId(2000));
        assert!(DragKnifeToolpath::run(&doc, &Params::default()).is_err());
    }

    /// Where the knife's tip is at `m`, the pivot leading it along `dir`.
    fn tip(m: &Move, dir: Point) -> Point {
        m.to.truncate() - 0.25 * dir
    }

    #[test]
    fn sharp_corners_cut_through_by_the_overshoot_then_lift_and_start_again() {
        let p = deep();
        let t = cut(vec![square(0.0, 0.0, 10.0)], &p);
        // along the bottom edge the tip runs 0.5 past the corner at (10, 0)
        let through = t
            .moves
            .iter()
            .position(|m| tip(m, Point::X).distance(Point::new(10.5, 0.0)) < 1e-9)
            .expect("cut through the corner");
        assert!((t.moves[through].to.z + 3.0).abs() < 1e-9);
        assert_eq!(t.moves[through + 1].kind, MoveKind::Rapid, "lifted out");
        // and goes down again 0.5 back along the right edge
        let again = t.moves[through..]
            .iter()
            .find(|m| m.kind == MoveKind::Plunge && (m.to.z + 3.0).abs() < 1e-9)
            .unwrap();
        assert!(tip(again, Point::Y).distance(Point::new(10.0, -0.5)) < 1e-9);
        // a closed shape's cut runs on past its start to close
        let last_cut = t
            .moves
            .iter()
            .rev()
            .find(|m| m.kind == MoveKind::Cut)
            .unwrap();
        assert!(tip(last_cut, -Point::Y).distance(Point::new(0.0, -0.5)) < 1e-9);
    }

    #[test]
    fn an_overshoot_never_runs_into_another_vector() {
        // a line 0.6 past the square's right edge: the overshoot stops 0.25 short of it
        let wall = Path::polyline(&[Point::new(10.6, -5.0), Point::new(10.6, 15.0)], false);
        let mut doc = cut_craft_core::testing::document_with(vec![square(0.0, 0.0, 10.0)]);
        doc.vectors
            .insert(cut_craft_core::document::EntityId(99), wall);
        let change = DragKnifeToolpath::run(&doc, &deep()).unwrap();
        let mut done = doc.clone();
        done.apply(&change).unwrap();
        let t = done.toolpaths.values().next().unwrap();
        assert!(
            t.moves.iter().all(|m| m.to.x < 10.6 - 0.25 + 0.25 + 1e-9),
            "the pivot (0.25 ahead of the tip) stays clear"
        );
        assert!(
            t.moves
                .iter()
                .any(|m| tip(m, Point::X).distance(Point::new(10.35, 0.0)) < 1e-9),
            "the tip stops 0.25 short of the line"
        );
    }

    #[test]
    fn with_no_overshoot_a_sharp_corner_still_lifts_and_restarts_at_it() {
        let p = Params {
            overshoot: 0.0,
            ..deep()
        };
        let t = cut(vec![square(0.0, 0.0, 10.0)], &p);
        assert!(
            t.moves
                .iter()
                .any(|m| tip(m, Point::X).distance(Point::new(10.0, 0.0)) < 1e-9
                    && (m.to.z + 3.0).abs() < 1e-9)
        );
        let rapids = t.moves.iter().filter(|m| m.kind == MoveKind::Rapid).count();
        assert!(rapids > 4, "{rapids}");
    }
}
