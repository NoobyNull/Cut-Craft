//! Rest machining for any toolpath: only what the earlier (larger) tools could not reach is
//! cut. What the toolpath would leave is worked out by cutting its moves into uncut
//! material; each earlier tool's best try at that shape shows what it left; the moves are
//! kept only over areas every earlier tool left (at least the minimum detail across, grown
//! by the overlap), joined by rapids at a safe height.

use crate::document::Job;
use crate::geometry::Point;
use crate::simulation::{Material, rest_areas};
use crate::toolpath::{Move, MoveKind, Tool, Toolpath};
use crate::{Change, CommandError, Edit};
use glam::DVec3;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Rest {
    /// The tools that cut before this one, in order
    pub after: Vec<Tool>,
    /// Leave alone unmachined areas narrower than this
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub minimum_detail: f64,
    /// Grow each unmachined area by this before cutting it
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub overlap: f64,
}

impl Rest {
    /// The rest machining a toolpath's settings ask for (`rest` in them), if any.
    pub fn of(params: &serde_json::Value) -> Option<Self> {
        let rest: Self = serde_json::from_value(params.get("rest")?.clone()).ok()?;
        (!rest.after.is_empty()).then_some(rest)
    }
}

/// `toolpath` kept to rest machining when `params` ask for it, the rest settings kept on it
/// so recalculating does it again. Nothing left to cut is an error.
pub fn apply(
    job: &Job,
    toolpath: &mut Toolpath,
    params: &serde_json::Value,
) -> Result<(), CommandError> {
    let Some(rest) = Rest::of(params) else {
        return Ok(());
    };
    toolpath.moves = rest_moves(job, &toolpath.tool, &toolpath.moves, &rest);
    if toolpath.moves.is_empty() {
        return Err(CommandError::Failed(format!(
            "the earlier tools leave nothing for {} to cut",
            toolpath.name
        )));
    }
    toolpath.params["rest"] = serde_json::to_value(&rest).expect("rest settings serialise");
    Ok(())
}

/// `change` with each toolpath it makes kept to rest machining, as [`apply`].
pub fn with_rest(
    job: &Job,
    params: &serde_json::Value,
    mut change: Change,
) -> Result<Change, CommandError> {
    for edit in &mut change.edits {
        if let Edit::AddToolpath { toolpath, .. }
        | Edit::ReplaceToolpath {
            after: toolpath, ..
        } = edit
        {
            apply(job, toolpath, params)?;
        }
    }
    Ok(change)
}

/// `moves` (cut by `tool`) kept only over what every earlier tool left.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a small positive cell count"
)]
pub fn rest_moves(job: &Job, tool: &Tool, moves: &[Move], rest: &Rest) -> Vec<Move> {
    if rest.after.is_empty() {
        return moves.to_vec();
    }
    // ponytail: a quarter of the tool across, no finer than the job's modeling cells; finer
    // finds smaller rest areas but grows as the square of it
    let cell = (tool.diameter / 4.0).max(job.model_cell());
    let cells = ((job.width.max(job.height) / cell).ceil() as usize).max(1);
    let mut target = Material::new(job, cells);
    for pair in moves.windows(2) {
        target.sweep(pair[0].to, pair[1].to, tool);
    }
    let areas: Vec<_> = rest
        .after
        .iter()
        .map(|earlier| {
            let left = target.left_by(earlier);
            rest_areas(&target, &left, rest.minimum_detail, rest.overlap)
        })
        .collect();
    let safe = moves.iter().map(|m| m.to.z).fold(job.clearance, f64::max);
    keep(
        moves,
        &|p| areas.iter().all(|area| area(p)),
        safe,
        target.cell,
    )
}

/// The parts of `moves` cutting (below the surface) where `inside` says, looked at every
/// `step`, joined by rapids at `safe` height.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "a small count of pieces"
)]
fn keep(moves: &[Move], inside: &dyn Fn(Point) -> bool, safe: f64, step: f64) -> Vec<Move> {
    let mut out: Vec<Move> = vec![];
    let mut cutting = false;
    let lift = |out: &mut Vec<Move>| {
        if let Some(last) = out.last().map(|m| m.to) {
            out.push(Move {
                kind: MoveKind::Rapid,
                to: DVec3::new(last.x, last.y, safe),
            });
        }
    };
    for pair in moves.windows(2) {
        let (a, b, kind) = (pair[0].to, pair[1].to, pair[1].kind);
        let pieces = (a.distance(b) / step).ceil().max(1.0) as usize;
        for k in 0..pieces {
            let (p, q) = (
                a.lerp(b, k as f64 / pieces as f64),
                a.lerp(b, (k + 1) as f64 / pieces as f64),
            );
            let middle = p.lerp(q, 0.5);
            let wanted = middle.z < 0.0 && inside(middle.truncate());
            if wanted && !cutting {
                out.push(Move {
                    kind: MoveKind::Rapid,
                    to: DVec3::new(p.x, p.y, safe),
                });
                out.push(Move {
                    kind: MoveKind::Plunge,
                    to: p,
                });
            } else if !wanted && cutting {
                lift(&mut out);
            }
            cutting = wanted;
            if wanted {
                let kind = if kind == MoveKind::Rapid {
                    MoveKind::Cut
                } else {
                    kind
                };
                // one move per run along a move, not one per piece
                match out.last_mut() {
                    Some(last) if last.kind == kind && k > 0 && last.to == p => last.to = q,
                    _ => out.push(Move { kind, to: q }),
                }
            }
        }
    }
    if cutting {
        lift(&mut out);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toolpath::ToolShape;

    fn end_mill(diameter: f64) -> Tool {
        Tool {
            name: format!("{diameter} mm End Mill"),
            diameter,
            shape: ToolShape::EndMill,
            ..Tool::default()
        }
    }

    /// A small job, and a 2 mm end mill clearing a 20 × 20 square pocket 3 deep by rows
    /// 1 mm apart.
    fn pocket() -> (Job, Tool, Vec<Move>) {
        let job = Job {
            width: 60.0,
            height: 60.0,
            ..Job::default()
        };
        let o = job.area().min;
        let at = |x: f64, y: f64, z: f64| DVec3::new(o.x + x, o.y + y, z);
        let mut moves = vec![Move {
            kind: MoveKind::Rapid,
            to: at(21.0, 21.0, 5.0),
        }];
        moves.push(Move {
            kind: MoveKind::Plunge,
            to: at(21.0, 21.0, -3.0),
        });
        for row in 0..19 {
            let y = 21.0 + f64::from(row);
            let (from, to) = if row % 2 == 0 {
                (21.0, 39.0)
            } else {
                (39.0, 21.0)
            };
            moves.push(Move {
                kind: MoveKind::Cut,
                to: at(from, y, -3.0),
            });
            moves.push(Move {
                kind: MoveKind::Cut,
                to: at(to, y, -3.0),
            });
        }
        moves.push(Move {
            kind: MoveKind::Rapid,
            to: at(21.0, 39.0, 5.0),
        });
        (job, end_mill(2.0), moves)
    }

    fn cut_length(moves: &[Move]) -> f64 {
        moves
            .windows(2)
            .filter(|w| w[1].kind == MoveKind::Cut)
            .map(|w| w[0].to.distance(w[1].to))
            .sum()
    }

    #[test]
    fn after_a_larger_end_mill_only_the_corners_are_cut() {
        let (job, tool, moves) = pocket();
        let rest = Rest {
            after: vec![end_mill(10.0)],
            ..Rest::default()
        };
        let kept = rest_moves(&job, &tool, &moves, &rest);
        let (all, left) = (cut_length(&moves), cut_length(&kept));
        assert!(left > 0.0, "the corners the 10 mm tool could not reach");
        assert!(left < all / 4.0, "{left} of {all}");
        // every cut is near a corner of the pocket
        let o = job.area().min;
        for m in kept.iter().filter(|m| m.kind == MoveKind::Cut) {
            let (x, y) = (m.to.x - o.x, m.to.y - o.y);
            let near = |v: f64| (v - 20.0).abs() < 6.0 || (v - 40.0).abs() < 6.0;
            assert!(near(x) && near(y), "a cut at {x}, {y}");
        }
    }

    #[test]
    fn a_toolpath_keeps_its_rest_settings_for_recalculating() {
        let (job, tool, moves) = pocket();
        let mut t = Toolpath {
            name: "Pocket".into(),
            strategy: "pocket_toolpath".into(),
            params: serde_json::json!({ "depth": 3.0 }),
            tool,
            vectors: vec![],
            moves,
            visible: true,
            power: vec![],
        };
        let params = serde_json::json!({ "rest": { "after": [end_mill(10.0)], "minimum_detail": 0.0, "overlap": 0.0 } });
        apply(&job, &mut t, &params).unwrap();
        assert!(Rest::of(&t.params).is_some(), "kept with the toolpath");
        let huge = serde_json::json!({ "rest": { "after": [end_mill(0.5)], "minimum_detail": 0.0, "overlap": 0.0 } });
        let mut again = t.clone();
        assert!(
            apply(&job, &mut again, &huge).is_err(),
            "a smaller earlier tool leaves nothing"
        );
    }

    #[test]
    fn no_earlier_tools_keeps_every_move_and_settings_carry_the_rest() {
        let (job, tool, moves) = pocket();
        assert_eq!(rest_moves(&job, &tool, &moves, &Rest::default()), moves);
        let params = serde_json::json!({ "rest": { "after": [end_mill(6.0)], "minimum_detail": 0.0, "overlap": 0.5 } });
        assert_eq!(Rest::of(&params).unwrap().after[0].diameter, 6.0);
        assert!(Rest::of(&serde_json::json!({ "rest": { "after": [] } })).is_none());
        assert!(Rest::of(&serde_json::json!({})).is_none());
    }
}
