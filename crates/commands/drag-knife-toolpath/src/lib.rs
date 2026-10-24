//! Drag Knife Toolpath: a trailing blade cuts along the selected vectors. At sharp corners
//! the knife runs past the corner by its blade offset, lifts to the swivel depth and swings
//! round the corner so the blade faces the new direction before cutting on.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::Point;
use cut_craft_core::glam::DVec3;
use cut_craft_core::toolpath::{Move, MoveKind, Tool, cut_points, made_by};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct DragKnifeToolpath;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
    /// From the knife's pivot to its tip (from the maker)
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub blade_offset: f64,
    /// Corners turning less than this are cut straight through, without a swivel
    #[schemars(extend("x-unit" = "°"), range(min = 0, max = 180))]
    pub tolerance_angle: f64,
    /// Final depth of the cut
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub cut_depth: f64,
    /// The blade lifts to this depth to swivel; less than the cut depth
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub swivel_depth: f64,
    /// Equal passes down to the cut depth
    #[schemars(range(min = 1))]
    pub passes: u32,
    /// Feed and plunge rates (switch the spindle off)
    pub tool: Tool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Drag Knife".into(),
            blade_offset: 0.25,
            tolerance_angle: 10.0,
            cut_depth: 3.0,
            swivel_depth: 0.5,
            passes: 1,
            tool: Tool {
                name: "Drag Knife".into(),
                diameter: 0.5,
                spindle_speed: 0.0,
                ..Tool::default()
            },
        }
    }
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

impl Params {
    /// One pass along `points` at `cut` (negative z), starting with the blade facing
    /// `facing`, coming down from `fresh` (a safe height) or else already down at the start;
    /// returns
    /// the direction the blade faces at the end.
    fn pass(
        &self,
        points: &[Point],
        closed: bool,
        cut: f64,
        facing: Point,
        fresh: Option<f64>,
        moves: &mut Vec<Move>,
    ) -> Point {
        let lift = -self.swivel_depth;
        let off = self.blade_offset;
        let tolerance = self.tolerance_angle.to_radians();
        let mut ring = points.to_vec();
        if closed {
            ring.push(points[0]);
        }
        let dirs: Vec<Point> = ring.windows(2).map(|w| (w[1] - w[0]).normalize()).collect();
        let start = ring[0];
        // swing the blade round to face along the first span before cutting
        let turn = facing.angle_to(dirs[0]).abs();
        if let (Some(safe), true) = (fresh, turn > 1e-9) {
            // from above, about a point the blade offset behind the start
            let centre = start - off * dirs[0];
            let from = centre + off * facing;
            moves.push(Move {
                kind: MoveKind::Rapid,
                to: from.extend(safe),
            });
            moves.push(Move {
                kind: MoveKind::Plunge,
                to: from.extend(lift),
            });
            moves.extend(swivel(centre, from, start, lift));
            moves.push(Move {
                kind: MoveKind::Plunge,
                to: start.extend(cut),
            });
        } else if fresh.is_none() && turn > tolerance {
            // already down at the start: lift and swing about it
            let (from, to) = (start + off * facing, start + off * dirs[0]);
            moves.push(Move {
                kind: MoveKind::Cut,
                to: from.extend(lift),
            });
            moves.extend(swivel(start, from, to, lift));
            moves.push(Move {
                kind: MoveKind::Plunge,
                to: to.extend(cut),
            });
        } else {
            if let Some(safe) = fresh {
                moves.push(Move {
                    kind: MoveKind::Rapid,
                    to: start.extend(safe),
                });
            }
            moves.push(Move {
                kind: MoveKind::Plunge,
                to: start.extend(cut),
            });
        }
        for (i, &dir) in dirs.iter().enumerate() {
            let corner = ring[i + 1];
            let next = dirs.get(i + 1).copied();
            match next {
                Some(next) if dir.angle_to(next).abs() > tolerance => {
                    let (from, to) = (corner + off * dir, corner + off * next);
                    moves.push(Move {
                        kind: MoveKind::Cut,
                        to: from.extend(cut),
                    });
                    moves.push(Move {
                        kind: MoveKind::Cut,
                        to: from.extend(lift),
                    });
                    moves.extend(swivel(corner, from, to, lift));
                    moves.push(Move {
                        kind: MoveKind::Plunge,
                        to: to.extend(cut),
                    });
                }
                _ => moves.push(Move {
                    kind: MoveKind::Cut,
                    to: corner.extend(cut),
                }),
            }
        }
        *dirs.last().expect("a path with a span")
    }
}

impl Command for DragKnifeToolpath {
    const ID: &'static str = "drag_knife_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Drag Knife Toolpath",
            group: "Toolpath Operations",
            icon: "toolpaths/drag_knife_toolpath.svg",
            shortcut: "",
        }
    }

    fn initial_params(doc: &Document) -> Params {
        Params {
            cut_depth: doc.job.thickness,
            ..Params::default()
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("blade offset", p.blade_offset)?;
        ensure_positive("cut depth", p.cut_depth)?;
        if !(p.swivel_depth >= 0.0 && p.swivel_depth < p.cut_depth) {
            return Err(CommandError::InvalidParams(
                "the swivel depth must be less than the cut depth".into(),
            ));
        }
        if p.passes == 0 {
            return Err(CommandError::InvalidParams(
                "it needs at least 1 pass".into(),
            ));
        }
        let (ids, paths) = doc.selected_paths(|_| true);
        if ids.is_empty() {
            return Err(CommandError::Failed("select the vectors to cut".into()));
        }
        let safe = doc.job.clearance;
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
                let cut = -p.cut_depth * f64::from(pass) / f64::from(p.passes);
                // an open vector starts each pass again from above; a closed one carries on
                let fresh = (pass == 1 || !path.closed).then_some(safe);
                if fresh.is_some() && pass > 1 {
                    let at = contour.last().map_or(DVec3::ZERO, |m: &Move| m.to);
                    contour.push(Move {
                        kind: MoveKind::Rapid,
                        to: at.truncate().extend(safe),
                    });
                }
                facing = p.pass(&points, path.closed, cut, facing, fresh, &mut contour);
            }
            let end = contour.last().map_or(DVec3::ZERO, |m| m.to);
            contour.push(Move {
                kind: MoveKind::Rapid,
                to: end.truncate().extend(safe),
            });
            moves.extend(contour);
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

    fn cut(paths: Vec<Path>, p: &Params) -> Toolpath {
        made_toolpath::<DragKnifeToolpath>(paths, p).unwrap()
    }

    #[test]
    fn a_square_starting_along_x_swivels_at_its_three_inner_corners() {
        let p = Params::default();
        let t = cut(vec![square(0.0, 0.0, 10.0)], &p);
        let lifts = t
            .moves
            .windows(2)
            .filter(|w| w[0].to.z < -2.9 && (w[1].to.z + p.swivel_depth).abs() < 1e-9)
            .count();
        assert_eq!(
            lifts, 3,
            "no swivel to start: the blade already faces along x"
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
        let p = Params::default();
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
                .all(|m| (m.to.z + p.swivel_depth).abs() > 1e-9)
        );
    }

    #[test]
    fn the_blade_is_swung_round_before_a_first_cut_off_the_x_axis() {
        let up = Path::polyline(&[Point::new(0.0, 0.0), Point::new(0.0, 10.0)], false);
        let p = Params::default();
        let t = cut(vec![up], &p);
        // swings about a point the blade offset behind the start
        let swivels = t
            .moves
            .iter()
            .filter(|m| (m.to.z + p.swivel_depth).abs() < 1e-9)
            .count();
        assert!(swivels > 2, "{swivels}");
        let first_cut = t.moves.iter().position(|m| m.to.z < -1.0).unwrap();
        assert!(t.moves[first_cut].to.truncate().distance(Point::ZERO) < 1e-9);
    }

    #[test]
    fn passes_split_the_cut_depth_evenly() {
        let p = Params {
            passes: 3,
            ..Params::default()
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
            ..Params::default()
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
    fn the_swivel_depth_must_be_above_the_cut() {
        let p = Params {
            swivel_depth: 3.0,
            ..Params::default()
        };
        let why = made_toolpath::<DragKnifeToolpath>(vec![square(0.0, 0.0, 10.0)], &p).unwrap_err();
        assert!(why.contains("swivel depth"), "{why}");
    }
}
