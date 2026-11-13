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

/// Clearing bits for a toolpath finished with `finishing`, checked: no two the same size and,
/// when `finishing_smallest` (Pocket: the finishing bit cuts what the clearing bits left),
/// each bigger than the finishing bit. In any order: they cut largest first.
pub fn check_clearing(
    finishing: &Tool,
    clearing: &[Tool],
    finishing_smallest: bool,
) -> Result<(), CommandError> {
    let mut sorted: Vec<&Tool> = clearing.iter().collect();
    sorted.sort_by(|a, b| b.diameter.total_cmp(&a.diameter));
    for pair in sorted.windows(2) {
        if (pair[0].diameter - pair[1].diameter).abs() < 1e-6 {
            return Err(CommandError::InvalidParams(format!(
                "Two clearing bits are the same size ({} and {}, {} across): remove one. Each \
                 clearing bit must be smaller than the one before it.",
                pair[0].name,
                pair[1].name,
                size(pair[0])
            )));
        }
    }
    if finishing_smallest
        && let Some(small) = sorted
            .iter()
            .find(|t| t.diameter <= finishing.diameter + 1e-6)
    {
        return Err(CommandError::InvalidParams(format!(
            "The clearing bit {} ({}) must be bigger than the finishing bit {}: pick a clearing \
             bit bigger than {}, or remove it to cut with {} alone.",
            small.name,
            size(small),
            finishing.name,
            size(finishing),
            finishing.name
        )));
    }
    Ok(())
}

/// How wide `tool` is, in its own units.
fn size(tool: &Tool) -> String {
    let trim = |v: f64| {
        let text = format!("{v:.4}");
        text.trim_end_matches('0').trim_end_matches('.').to_owned()
    };
    match tool.units {
        crate::document::Units::Inch => format!("{}\"", trim(tool.diameter / 25.4)),
        crate::document::Units::Mm => format!("{} mm", trim(tool.diameter)),
    }
}

/// `toolpath` kept to rest machining when `params` ask for it, the rest settings kept on it
/// so recalculating does it again. Nothing left to cut is an error.
pub fn apply(
    job: &Job,
    toolpath: &mut Toolpath,
    params: &serde_json::Value,
) -> Result<(), CommandError> {
    let Some(mut rest) = Rest::of(params) else {
        return Ok(());
    };
    // a toolpath that already rests after tools of its own (a Pocket's clearance tools)
    // keeps them: the form's list adds to them
    if let Some(own) = Rest::of(&toolpath.params) {
        for tool in own.after {
            if !rest.after.contains(&tool) {
                rest.after.push(tool);
            }
        }
    }
    toolpath.moves = rest_moves(job, &toolpath.tool, &toolpath.moves, &rest);
    if toolpath.moves.is_empty() {
        return Err(CommandError::Failed(format!(
            "the earlier tools leave nothing for {} to cut. Turn off rest machining, or pick another tool",
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
    target.cut_along(moves, tool);
    // what every earlier tool left: the lowest of what each left
    let mut left = target.left_by(&rest.after[0]);
    for earlier in &rest.after[1..] {
        let also = target.left_by(earlier);
        for (h, a) in left.heights.iter_mut().zip(&also.heights) {
            *h = h.min(*a);
        }
    }
    // the tool cuts there while its edge is over it, not only its centre: grown by its radius
    let area = rest_areas(
        &target,
        &left,
        rest.minimum_detail,
        rest.overlap + tool.diameter / 2.0,
    );
    let safe = moves.iter().map(|m| m.to.z).fold(job.clearance, f64::max);
    let kept = keep(moves, &area, safe, target.cell);
    spot_by_spot(kept, tool.diameter + target.cell)
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

/// The spot `i` is in, its path to it shortened on the way.
fn root(spot: &mut [usize], mut i: usize) -> usize {
    while spot[i] != i {
        spot[i] = spot[spot[i]];
        i = spot[i];
    }
    i
}

/// `moves` (pieces, each a rapid down to a plunge and its cuts up to a rapid lift) taken a
/// spot at a time: every piece of a spot, in the order they came (so shallower passes first),
/// then on to the spot whose first piece starts nearest. Pieces within `reach` of each other
/// are one spot: with it the tool across, pieces of different spots never cut the same
/// material, so no pass goes into material a shallower one should have cut first.
fn spot_by_spot(moves: Vec<Move>, reach: f64) -> Vec<Move> {
    // the pieces: each starts at a rapid that a plunge follows
    let mut pieces: Vec<Vec<Move>> = vec![];
    for (i, m) in moves.iter().enumerate() {
        let starts = m.kind == MoveKind::Rapid
            && moves.get(i + 1).is_some_and(|n| n.kind == MoveKind::Plunge);
        if starts || pieces.is_empty() {
            pieces.push(vec![]);
        }
        pieces.last_mut().expect("a piece").push(*m);
    }
    if pieces.len() < 3 {
        return moves;
    }
    // each piece's extent, then pieces near each other joined into spots
    let extent: Vec<(Point, Point)> = pieces
        .iter()
        .map(|piece| {
            let cut = piece.iter().filter(|m| m.kind != MoveKind::Rapid);
            cut.fold(
                (Point::splat(f64::INFINITY), Point::splat(f64::NEG_INFINITY)),
                |(lo, hi), m| (lo.min(m.to.truncate()), hi.max(m.to.truncate())),
            )
        })
        .collect();
    let near = |a: usize, b: usize| {
        let ((alo, ahi), (blo, bhi)) = (extent[a], extent[b]);
        alo.x <= bhi.x + reach
            && blo.x <= ahi.x + reach
            && alo.y <= bhi.y + reach
            && blo.y <= ahi.y + reach
    };
    let mut spot: Vec<usize> = (0..pieces.len()).collect();
    for a in 0..pieces.len() {
        for b in a + 1..pieces.len() {
            if near(a, b) {
                let (ra, rb) = (root(&mut spot, a), root(&mut spot, b));
                if ra != rb {
                    spot[rb.max(ra)] = rb.min(ra);
                }
            }
        }
    }
    let mut spots: Vec<Vec<usize>> = vec![];
    let mut index_of = std::collections::HashMap::new();
    for i in 0..pieces.len() {
        let r = root(&mut spot, i);
        let at = *index_of.entry(r).or_insert_with(|| {
            spots.push(vec![]);
            spots.len() - 1
        });
        spots[at].push(i);
    }
    // the first spot first, then always the nearest one not yet cut
    let start = |s: &[usize]| {
        pieces[s[0]]
            .iter()
            .find(|m| m.kind == MoveKind::Plunge)
            .map_or(Point::ZERO, |m| m.to.truncate())
    };
    let mut out = Vec::with_capacity(moves.len());
    let mut left: Vec<usize> = (0..spots.len()).collect();
    let mut at = start(&spots[0]);
    while !left.is_empty() {
        let (k, _) = left
            .iter()
            .enumerate()
            .map(|(k, &s)| (k, start(&spots[s]).distance(at)))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .expect("a spot left");
        let s = left.remove(k);
        for &i in &spots[s] {
            out.extend_from_slice(&pieces[i]);
        }
        if let Some(last) = out.last() {
            at = last.to.truncate();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toolpath::ToolShape;

    #[test]
    fn a_middle_bit_cuts_the_corners_its_edge_reaches_though_its_centre_does_not() {
        // an 8 mm bit after a 16 mm one in the 20 mm square: the corners the big bit left are
        // under the 4 mm bit's edge, never under its centre, which the 12 mm bit cleared
        let (job, _, _) = pocket();
        let o = job.area().min;
        let at = |x: f64, y: f64, z: f64| DVec3::new(o.x + x, o.y + y, z);
        let mut moves = vec![
            Move {
                kind: MoveKind::Rapid,
                to: at(24.0, 24.0, 5.0),
            },
            Move {
                kind: MoveKind::Plunge,
                to: at(24.0, 24.0, -3.0),
            },
        ];
        for row in 0..7 {
            let y = 24.0 + f64::from(row) * 2.0;
            let (from, to) = if row % 2 == 0 {
                (24.0, 36.0)
            } else {
                (36.0, 24.0)
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
        let rest = Rest {
            after: vec![end_mill(16.0)],
            ..Rest::default()
        };
        let kept = rest_moves(&job, &end_mill(8.0), &moves, &rest);
        assert!(cut_length(&kept) > 0.0, "the 8 mm bit has corners to cut");
        assert!(
            cut_length(&kept) < cut_length(&moves) / 2.0,
            "but only near them"
        );
    }

    #[test]
    fn clearing_bits_bigger_than_the_finishing_bit_pass() {
        let finishing = end_mill(2.0);
        assert!(
            check_clearing(&finishing, &[], true).is_ok(),
            "no clearing: a normal toolpath"
        );
        let chain = [end_mill(12.0), end_mill(6.0)];
        assert!(check_clearing(&finishing, &chain, true).is_ok());
        // the order picked in does not matter: they cut largest first
        let mixed = [end_mill(6.0), end_mill(12.0)];
        assert!(check_clearing(&finishing, &mixed, true).is_ok());
    }

    #[test]
    fn a_clearing_bit_not_bigger_than_the_finishing_bit_is_refused_saying_how_to_fix_it() {
        let finishing = end_mill(6.0);
        for small in [end_mill(6.0), end_mill(3.0)] {
            let name = small.name.clone();
            let Err(CommandError::InvalidParams(why)) =
                check_clearing(&finishing, &[end_mill(12.0), small], true)
            else {
                panic!("a {name} clearing bit after a 6 mm finishing bit");
            };
            assert!(
                why.contains(&name) && why.contains("6 mm End Mill"),
                "{why}"
            );
            assert!(
                why.contains("bigger than 6 mm"),
                "says what size it must be: {why}"
            );
            assert!(why.contains("remove"), "says how to fix it: {why}");
        }
    }

    #[test]
    fn two_clearing_bits_the_same_size_are_refused() {
        let Err(CommandError::InvalidParams(why)) =
            check_clearing(&end_mill(2.0), &[end_mill(6.0), end_mill(6.0)], true)
        else {
            panic!("two 6 mm clearing bits");
        };
        assert!(
            why.contains("same size") && why.contains("remove one"),
            "{why}"
        );
    }

    #[test]
    fn a_v_bit_finishes_after_clearing_bits_of_any_size() {
        // V-Carve: the V-bit carves the slopes; it need not be the smallest
        let v_bit = Tool {
            name: "60° V-Bit".into(),
            shape: ToolShape::VBit,
            diameter: 12.0,
            ..Tool::default()
        };
        assert!(check_clearing(&v_bit, &[end_mill(6.0), end_mill(3.0)], false).is_ok());
    }

    #[test]
    fn inch_bits_are_sized_in_inches() {
        let inch = |name: &str, diameter: f64| Tool {
            name: name.into(),
            diameter: diameter * 25.4,
            units: crate::document::Units::Inch,
            ..Tool::default()
        };
        let Err(CommandError::InvalidParams(why)) = check_clearing(
            &inch("1/4\" End Mill", 0.25),
            &[inch("1/8\" End Mill", 0.125)],
            true,
        ) else {
            panic!("a smaller clearing bit");
        };
        assert!(why.contains("bigger than 0.25\""), "{why}");
    }

    #[test]
    fn a_form_rest_list_adds_to_the_tools_a_toolpath_already_rests_after() {
        // Pocket with clearance tools: its last toolpath already rests after the 10 mm
        // tool; an older job's Rest Machining list adds its tools, not replaces them
        let (job, tool, moves) = pocket();
        let mut t = Toolpath {
            name: "Pocket [2]".into(),
            strategy: "pocket_toolpath".into(),
            params: serde_json::json!({ "depth": 3.0 }),
            tool,
            vectors: vec![],
            moves,
            visible: true,
            power: vec![],
            stale: false,
            group: None,
            drill_bank: None,
            fallbacks: vec![],
        };
        let own = serde_json::json!({ "rest": { "after": [end_mill(10.0)], "minimum_detail": 0.0, "overlap": 0.0 } });
        t.params["rest"] = own["rest"].clone();
        let params = t.params.clone();
        apply(&job, &mut t, &params).unwrap();
        let form = serde_json::json!({ "rest": { "after": [end_mill(4.0)], "minimum_detail": 0.0, "overlap": 0.0 } });
        apply(&job, &mut t, &form).unwrap();
        let after: Vec<f64> = Rest::of(&t.params)
            .unwrap()
            .after
            .iter()
            .map(|t| t.diameter)
            .collect();
        assert_eq!(after, vec![4.0, 10.0], "both kept for recalculating");
    }

    /// How many times `moves` go more than `far` across between cutting.
    fn long_jumps(moves: &[Move], far: f64) -> usize {
        let mut from = None;
        let mut jumps = 0;
        for m in moves {
            match m.kind {
                MoveKind::Plunge => {
                    if let Some(f) = from
                        && m.to.truncate().distance(f) > far
                    {
                        jumps += 1;
                    }
                }
                MoveKind::Cut => from = Some(m.to.truncate()),
                MoveKind::Rapid => {}
            }
        }
        jumps
    }

    #[test]
    fn each_corner_left_is_finished_before_going_to_the_nearest_next() {
        // the raster runs across the pocket: kept in its order, the corners left by a 10 mm
        // tool would be visited left, right, left, right on every row
        let (job, tool, moves) = pocket();
        let rest = Rest {
            after: vec![end_mill(10.0)],
            ..Rest::default()
        };
        let kept = rest_moves(&job, &tool, &moves, &rest);
        let jumps = long_jumps(&kept, 10.0);
        assert!(
            jumps <= 3,
            "{jumps} jumps from corner to corner; four corners need 3"
        );
        // nothing is lost by the reordering
        let left = cut_length(&kept);
        let o = job.area().min;
        for m in kept.iter().filter(|m| m.kind == MoveKind::Cut) {
            let (x, y) = (m.to.x - o.x, m.to.y - o.y);
            let near = |v: f64| (v - 20.0).abs() < 6.0 || (v - 40.0).abs() < 6.0;
            assert!(near(x) && near(y), "a cut at {x}, {y}");
        }
        assert!(left > 0.0);
    }

    #[test]
    fn deeper_passes_in_a_corner_come_after_the_shallower_ones() {
        let (job, tool, moves) = pocket();
        // the same raster again a pass deeper
        let mut two = moves.clone();
        two.extend(moves.iter().map(|m| Move {
            to: if m.kind == MoveKind::Rapid {
                m.to
            } else {
                m.to - DVec3::Z * 3.0
            },
            ..*m
        }));
        let rest = Rest {
            after: vec![end_mill(10.0)],
            ..Rest::default()
        };
        let kept = rest_moves(&job, &tool, &two, &rest);
        // in each corner, every cut at -6 comes after every cut there at -3
        let o = job.area().min;
        let corner = |m: &Move| ((m.to.x - o.x) > 30.0, (m.to.y - o.y) > 30.0);
        for c in [(false, false), (false, true), (true, false), (true, true)] {
            let depths: Vec<f64> = kept
                .iter()
                .filter(|m| m.kind == MoveKind::Cut && corner(m) == c)
                .map(|m| m.to.z)
                // the passes' own levels, not the lift out at the end
                .filter(|z| (z + 3.0).abs() < 1e-9 || (z + 6.0).abs() < 1e-9)
                .collect();
            let first_deep = depths
                .iter()
                .position(|z| *z < -4.0)
                .expect("a deeper pass");
            assert!(
                depths[first_deep..].iter().all(|z| *z < -4.0),
                "{c:?}: {depths:?}"
            );
        }
        assert!(long_jumps(&kept, 10.0) <= 3);
    }

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
            stale: false,
            group: None,
            drill_bank: None,
            fallbacks: vec![],
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
