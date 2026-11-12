//! Merged Toolpath: the visible toolpaths that use one tool made into one, with their cuts
//! in whichever of the orders asked for leaves the least travelling between them, each
//! part's cuts together when asked; and Ungroup, which takes a merged toolpath apart again.

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::geometry::{Point, inside};
use cut_craft_core::glam::DVec3;
use cut_craft_core::toolpath::{Move, MoveKind, Toolpath, made_by};
use cut_craft_core::{Change, Command, CommandError, Meta, existing};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct MergedToolpath;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "Aspire's Ordering tick boxes"
)]
pub struct Params {
    pub name: String,
    /// The toolpaths to merge (none: the visible ones)
    pub sources: Vec<EntityId>,
    /// Cuts in order across the job; of the orders ticked, the quickest is kept
    #[serde(default = "yes")]
    #[schemars(title = "Left to Right", extend("x-group" = "Ordering"))]
    pub left_to_right: bool,
    /// Cuts in order up the job
    #[serde(default = "yes")]
    #[schemars(title = "Bottom to Top", extend("x-group" = "Ordering"))]
    pub bottom_to_top: bool,
    /// Cuts in rows up the job, across each row
    #[serde(default = "yes")]
    #[schemars(title = "Grid", extend("x-group" = "Ordering"))]
    pub grid: bool,
    /// Each cut the nearest to where the last ended
    #[serde(default = "yes")]
    #[schemars(title = "Shortest Path", extend("x-group" = "Ordering"))]
    pub shortest_path: bool,
    /// Each part's cuts together: a part is an outermost closed cut with the cuts inside
    /// it, cut in the toolpath list's order within the part
    #[serde(default)]
    #[schemars(title = "Merge By Part", extend("x-group" = "Ordering"))]
    pub merge_by_part: bool,
}

fn yes() -> bool {
    true
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Merged".into(),
            sources: vec![],
            left_to_right: true,
            bottom_to_top: true,
            grid: true,
            shortest_path: true,
            merge_by_part: false,
        }
    }
}

/// One plunge-to-retract run of feed moves, with each move's laser power.
#[derive(Clone)]
struct Cut {
    moves: Vec<Move>,
    power: Vec<f64>,
}

impl Cut {
    fn start(&self) -> DVec3 {
        self.moves[0].to
    }

    fn end(&self) -> DVec3 {
        self.moves[self.moves.len() - 1].to
    }
}

/// A toolpath's feed moves split at its rapids.
fn cuts(t: &Toolpath) -> Vec<Cut> {
    let mut out = vec![];
    let mut cut = Cut {
        moves: vec![],
        power: vec![],
    };
    for (i, m) in t.moves.iter().enumerate() {
        if m.kind == MoveKind::Rapid {
            if !cut.moves.is_empty() {
                out.push(std::mem::replace(
                    &mut cut,
                    Cut {
                        moves: vec![],
                        power: vec![],
                    },
                ));
            }
        } else {
            cut.moves.push(*m);
            cut.power.push(t.power.get(i).copied().unwrap_or(0.0));
        }
    }
    if !cut.moves.is_empty() {
        out.push(cut);
    }
    out
}

/// Distance travelled between the cuts in this order, from the first.
fn travel(order: &[Cut]) -> f64 {
    order
        .windows(2)
        .map(|w| w[0].end().truncate().distance(w[1].start().truncate()))
        .sum()
}

/// The cuts sorted by `key` of where each starts.
fn sorted_by(cuts: &[Cut], key: impl Fn(Point) -> (f64, f64)) -> Vec<Cut> {
    let mut order = cuts.to_vec();
    order.sort_by(|a, b| {
        let (ka, kb) = (key(a.start().truncate()), key(b.start().truncate()));
        ka.partial_cmp(&kb).unwrap_or(std::cmp::Ordering::Equal)
    });
    order
}

/// How many rows the Grid order cuts the job into.
const GRID_ROWS: f64 = 8.0;

/// The orders asked for, each of `cuts` (the toolpath list's order): the quickest is kept.
fn orders(cuts: &[Cut], p: &Params, height: f64) -> Vec<Vec<Cut>> {
    let mut tried = vec![];
    if p.left_to_right {
        tried.push(sorted_by(cuts, |q| (q.x, q.y)));
    }
    if p.bottom_to_top {
        tried.push(sorted_by(cuts, |q| (q.y, q.x)));
    }
    if p.grid {
        let row = (height / GRID_ROWS).max(1e-9);
        tried.push(sorted_by(cuts, |q| ((q.y / row).floor(), q.x)));
    }
    if p.shortest_path {
        tried.push(nearest_first(cuts.to_vec()));
    }
    tried
}

/// `cuts` as parts: each outermost closed cut with every cut starting inside it, in the
/// order given; a cut inside none is a part of its own.
fn parts(cuts: &[Cut]) -> Vec<Vec<Cut>> {
    let rings: Vec<Option<Vec<Point>>> = cuts
        .iter()
        .map(|c| {
            let ring: Vec<Point> = c.moves.iter().map(|m| m.to.truncate()).collect();
            let closed = ring.len() > 2 && ring[0].distance(ring[ring.len() - 1]) < 1e-6;
            closed.then_some(ring)
        })
        .collect();
    // the cut that holds each one: the smallest closed cut it starts inside, if any
    let holder = |i: usize| -> Option<usize> {
        let at = cuts[i].start().truncate();
        (0..cuts.len())
            .filter(|&j| j != i)
            .filter(|&j| {
                rings[j]
                    .as_ref()
                    .is_some_and(|r| inside(std::slice::from_ref(r), at))
            })
            .min_by(|&a, &b| {
                let size = |j: usize| rings[j].as_ref().map_or(usize::MAX, Vec::len);
                size(a).cmp(&size(b))
            })
    };
    let mut outer_of: Vec<usize> = (0..cuts.len()).collect();
    for (i, outer) in outer_of.iter_mut().enumerate() {
        let mut at = i;
        while let Some(h) = holder(at) {
            if h == i {
                break;
            }
            at = h;
        }
        *outer = at;
    }
    let mut out: Vec<(usize, Vec<Cut>)> = vec![];
    for (i, cut) in cuts.iter().enumerate() {
        match out.iter_mut().find(|(outer, _)| *outer == outer_of[i]) {
            Some((_, part)) => part.push(cut.clone()),
            None => out.push((outer_of[i], vec![cut.clone()])),
        }
    }
    out.into_iter().map(|(_, part)| part).collect()
}

/// The cuts, each next one the nearest start to where the last ended.
fn nearest_first(mut left: Vec<Cut>) -> Vec<Cut> {
    let mut order: Vec<Cut> = vec![];
    while !left.is_empty() {
        let at = order.last().map_or(left[0].start(), Cut::end).truncate();
        let next = (0..left.len())
            .min_by(|&a, &b| {
                let d = |i: usize| left[i].start().truncate().distance(at);
                d(a).total_cmp(&d(b))
            })
            .expect("left is not empty");
        order.push(left.swap_remove(next));
    }
    order
}

impl Command for MergedToolpath {
    const ID: &'static str = "merged_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Merged Toolpath",
            group: "Toolpath Operations",
            icon: "toolpaths/merge_toolpaths.png",
            shortcut: "",
        }
    }

    fn initial_params(doc: &Document) -> Params {
        Params {
            sources: doc
                .toolpaths
                .iter()
                .filter(|(_, t)| t.visible && t.strategy != Self::ID)
                .map(|(&id, _)| id)
                .collect(),
            ..Params::default()
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let named = if p.sources.is_empty() {
            Self::initial_params(doc).sources
        } else {
            p.sources.clone()
        };
        let sources: Vec<&Toolpath> = named
            .iter()
            .filter_map(|id| doc.toolpaths.get(id))
            .collect();
        let Some(first) = sources.first() else {
            return Err(CommandError::Failed(
                "show the toolpaths to merge in the toolpath list".into(),
            ));
        };
        // a drill bank's plunges are its own: merged, they would be drilled by one drill
        if let Some(bank) = sources.iter().find(|t| t.drill_bank.is_some()) {
            return Err(CommandError::Failed(format!(
                "{} is a drill bank toolpath, which cannot be merged: hide it before merging",
                bank.name
            )));
        }
        if let Some(other) = sources.iter().find(|t| t.tool.name != first.tool.name) {
            return Err(CommandError::Failed(format!(
                "only toolpaths with the same tool merge: {} uses {} and {} uses {}. Show only toolpaths that use one tool",
                first.name, first.tool.name, other.name, other.tool.name
            )));
        }
        let in_order: Vec<Cut> = sources.iter().flat_map(|t| cuts(t)).collect();
        let height = doc.job.height;
        let order = if p.merge_by_part {
            // the parts in the quickest of the orders asked for, each part's cuts as listed
            let parts = parts(&in_order);
            let firsts: Vec<Cut> = parts.iter().map(|part| part[0].clone()).collect();
            let quickest = orders(&firsts, p, height)
                .into_iter()
                .min_by(|a, b| travel(a).total_cmp(&travel(b)))
                .unwrap_or(firsts);
            quickest
                .iter()
                .flat_map(|first| {
                    parts
                        .iter()
                        .find(|part| {
                            part[0].start() == first.start() && part[0].end() == first.end()
                        })
                        .cloned()
                        .unwrap_or_default()
                })
                .collect()
        } else {
            orders(&in_order, p, height)
                .into_iter()
                .min_by(|a, b| travel(a).total_cmp(&travel(b)))
                .unwrap_or(in_order)
        };
        let safe = doc.job.clearance;
        let laser = sources.iter().any(|t| !t.power.is_empty());
        let (mut moves, mut power) = (vec![], vec![]);
        for cut in &order {
            moves.push(Move {
                kind: MoveKind::Rapid,
                to: cut.start().truncate().extend(safe),
            });
            moves.extend(&cut.moves);
            moves.push(Move {
                kind: MoveKind::Rapid,
                to: cut.end().truncate().extend(safe),
            });
            power.push(0.0);
            power.extend(&cut.power);
            power.push(0.0);
        }
        // the sources as resolved, so Ungroup and a recalculation know them
        let settings = Params {
            sources: named.clone(),
            ..p.clone()
        };
        let mut toolpath =
            made_by::<Self>(&settings, p.name.clone(), first.tool.clone(), vec![], moves);
        if laser {
            toolpath.power = power;
        }
        let mut c = Change::builder(doc);
        c.add_toolpath(toolpath);
        Ok(c.finish())
    }
}

/// Ungroup: a merged toolpath taken away, its sources shown again.
pub struct UngroupMergedToolpath;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct UngroupParams {
    /// The merged toolpath to take apart
    pub toolpath: EntityId,
}

impl Default for UngroupParams {
    fn default() -> Self {
        Self {
            toolpath: EntityId(1),
        }
    }
}

impl Command for UngroupMergedToolpath {
    const ID: &'static str = "ungroup_merged_toolpath";
    type Params = UngroupParams;

    fn meta() -> Meta {
        Meta {
            label: "Ungroup Merged Toolpath",
            group: "Toolpaths",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &UngroupParams) -> Result<Change, CommandError> {
        let merged = existing(&doc.toolpaths, p.toolpath, "toolpath")?;
        if merged.strategy != MergedToolpath::ID {
            return Err(CommandError::Failed(format!(
                "{} is not a merged toolpath. Pick a merged toolpath to ungroup",
                merged.name
            )));
        }
        let settings: Params = serde_json::from_value(merged.params.clone())
            .map_err(|e| CommandError::Failed(e.to_string()))?;
        let mut c = Change::builder(doc);
        c.remove_toolpath(p.toolpath);
        for id in settings.sources {
            if let Some(source) = doc.toolpaths.get(&id).filter(|t| !t.visible) {
                c.replace_toolpath(
                    id,
                    Toolpath {
                        visible: true,
                        ..source.clone()
                    },
                );
            }
        }
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::with_toolpath;
    use cut_craft_core::toolpath::Tool;

    fn mv(kind: MoveKind, x: f64, z: f64) -> Move {
        Move {
            kind,
            to: DVec3::new(x, 0.0, z),
        }
    }

    /// A plunge-and-cut at each x, in that order, as one toolpath.
    fn cutting_at(xs: &[f64]) -> Vec<Move> {
        xs.iter()
            .flat_map(|&x| {
                [
                    mv(MoveKind::Rapid, x, 5.0),
                    mv(MoveKind::Plunge, x, -1.0),
                    mv(MoveKind::Cut, x + 1.0, -1.0),
                    mv(MoveKind::Rapid, x + 1.0, 5.0),
                ]
            })
            .collect()
    }

    /// Two toolpaths, one tool: cuts at 0 and 100, then at 50.
    fn two() -> Document {
        let mut doc = with_toolpath();
        let mut first = doc.toolpaths[&EntityId(1)].clone();
        first.moves = cutting_at(&[0.0, 100.0]);
        let mut second = first.clone();
        second.name = "Second".into();
        second.moves = cutting_at(&[50.0]);
        let mut c = Change::builder(&doc);
        c.replace_toolpath(EntityId(1), first);
        c.add_toolpath(second);
        let change = c.finish();
        doc.apply(&change).unwrap();
        doc
    }

    cut_craft_core::contract_tests!(MergedToolpath, two);

    fn merged(doc: &Document) -> Toolpath {
        let change = MergedToolpath::run(doc, &Params::default()).unwrap();
        let mut after = doc.clone();
        after.apply(&change).unwrap();
        after
            .toolpaths
            .into_values()
            .find(|t| t.strategy == MergedToolpath::ID)
            .unwrap()
    }

    #[test]
    fn merges_every_cut_in_the_order_with_least_travel() {
        let t = merged(&two());
        let starts: Vec<f64> = t
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Plunge)
            .map(|m| m.to.x)
            .collect();
        assert_eq!(starts, [0.0, 50.0, 100.0]);
    }

    #[test]
    fn drill_bank_toolpaths_do_not_merge() {
        let mut doc = two();
        let id = *doc.toolpaths.keys().last().unwrap();
        let mut bank = doc.toolpaths[&id].clone();
        bank.drill_bank = Some(cut_craft_core::drill_bank::BankToolpath::default());
        let mut c = Change::builder(&doc);
        c.replace_toolpath(id, bank);
        doc.apply(&c.finish()).unwrap();
        let err = MergedToolpath::run(&doc, &Params::default()).unwrap_err();
        assert!(err.to_string().contains("drill bank"), "{err}");
    }

    #[test]
    fn toolpaths_with_different_tools_do_not_merge() {
        let mut doc = two();
        let mut t = doc.toolpaths[&EntityId(2)].clone();
        t.tool = Tool {
            name: "Other".into(),
            ..Tool::default()
        };
        let mut c = Change::builder(&doc);
        c.replace_toolpath(EntityId(2), t);
        let change = c.finish();
        doc.apply(&change).unwrap();
        let why = MergedToolpath::run(&doc, &Params::default())
            .unwrap_err()
            .to_string();
        assert!(why.contains("same tool"), "{why}");
    }

    fn starts(t: &Toolpath) -> Vec<(f64, f64)> {
        t.moves
            .iter()
            .filter(|m| m.kind == MoveKind::Plunge)
            .map(|m| (m.to.x, m.to.y))
            .collect()
    }

    /// Plunge-and-cut at each point, in that order.
    fn cutting_at_points(points: &[(f64, f64)]) -> Vec<Move> {
        points
            .iter()
            .flat_map(|&(x, y)| {
                let at = |z| DVec3::new(x, y, z);
                [
                    Move {
                        kind: MoveKind::Rapid,
                        to: at(5.0),
                    },
                    Move {
                        kind: MoveKind::Plunge,
                        to: at(-1.0),
                    },
                    Move {
                        kind: MoveKind::Cut,
                        to: DVec3::new(x + 1.0, y, -1.0),
                    },
                    Move {
                        kind: MoveKind::Rapid,
                        to: DVec3::new(x + 1.0, y, 5.0),
                    },
                ]
            })
            .collect()
    }

    #[test]
    fn only_the_orders_ticked_are_tried() {
        let mut doc = with_toolpath();
        let mut t = doc.toolpaths[&EntityId(1)].clone();
        t.moves = cutting_at_points(&[(50.0, 0.0), (0.0, 50.0), (100.0, 100.0)]);
        let mut c = Change::builder(&doc);
        c.replace_toolpath(EntityId(1), t);
        let change = c.finish();
        doc.apply(&change).unwrap();
        let order = |p: &Params| {
            let change = MergedToolpath::run(&doc, p).unwrap();
            let mut after = doc.clone();
            after.apply(&change).unwrap();
            let t = after
                .toolpaths
                .into_values()
                .find(|t| t.strategy == MergedToolpath::ID)
                .unwrap();
            starts(&t)
        };
        let only = |left, bottom| Params {
            left_to_right: left,
            bottom_to_top: bottom,
            grid: false,
            shortest_path: false,
            ..Params::default()
        };
        assert_eq!(
            order(&only(true, false)),
            [(0.0, 50.0), (50.0, 0.0), (100.0, 100.0)]
        );
        assert_eq!(
            order(&only(false, true)),
            [(50.0, 0.0), (0.0, 50.0), (100.0, 100.0)]
        );
        assert_eq!(
            order(&only(false, false)),
            [(50.0, 0.0), (0.0, 50.0), (100.0, 100.0)],
            "none ticked: the list's order"
        );
    }

    #[test]
    fn merging_by_part_cuts_each_outer_shape_with_what_is_inside_it() {
        // two squares, each with a small cut inside: the insides first in the list
        let square = |x0: f64| {
            let p = |x, y| DVec3::new(x, y, -1.0);
            let corners = [
                p(x0, 0.0),
                p(x0 + 20.0, 0.0),
                p(x0 + 20.0, 20.0),
                p(x0, 20.0),
                p(x0, 0.0),
            ];
            let mut moves = vec![
                Move {
                    kind: MoveKind::Rapid,
                    to: DVec3::new(x0, 0.0, 5.0),
                },
                Move {
                    kind: MoveKind::Plunge,
                    to: corners[0],
                },
            ];
            moves.extend(corners[1..].iter().map(|&to| Move {
                kind: MoveKind::Cut,
                to,
            }));
            moves.push(Move {
                kind: MoveKind::Rapid,
                to: DVec3::new(x0, 0.0, 5.0),
            });
            moves
        };
        let mut doc = with_toolpath();
        let mut insides = doc.toolpaths[&EntityId(1)].clone();
        insides.moves = cutting_at_points(&[(5.0, 10.0), (105.0, 10.0)]);
        let mut squares = insides.clone();
        squares.name = "Squares".into();
        squares.moves = [square(0.0), square(100.0)].concat();
        let mut c = Change::builder(&doc);
        c.replace_toolpath(EntityId(1), insides);
        c.add_toolpath(squares);
        let change = c.finish();
        doc.apply(&change).unwrap();
        let merged = |by_part| {
            let p = Params {
                merge_by_part: by_part,
                ..Params::default()
            };
            let change = MergedToolpath::run(&doc, &p).unwrap();
            let mut after = doc.clone();
            after.apply(&change).unwrap();
            let t = after
                .toolpaths
                .into_values()
                .find(|t| t.strategy == MergedToolpath::ID)
                .unwrap();
            starts(&t).into_iter().map(|(x, _)| x).collect::<Vec<_>>()
        };
        assert_eq!(
            merged(true),
            [5.0, 0.0, 105.0, 100.0],
            "each part: its inside, then its square"
        );
        assert_eq!(
            merged(false),
            [0.0, 5.0, 100.0, 105.0],
            "else the quickest order of all the cuts"
        );
    }

    #[test]
    fn ungroup_takes_the_merged_toolpath_away_and_shows_its_sources() {
        let mut doc = two();
        let change = MergedToolpath::run(&doc, &Params::default()).unwrap();
        doc.apply(&change).unwrap();
        let merged = *doc
            .toolpaths
            .iter()
            .find(|(_, t)| t.strategy == MergedToolpath::ID)
            .map(|(id, _)| id)
            .unwrap();
        doc.toolpaths.get_mut(&EntityId(1)).unwrap().visible = false;
        let p = UngroupParams { toolpath: merged };
        cut_craft_core::testing::run_on::<UngroupMergedToolpath>(&mut doc, &p);
        assert!(!doc.toolpaths.contains_key(&merged));
        assert!(
            doc.toolpaths[&EntityId(1)].visible,
            "a hidden source is shown again"
        );
        assert_eq!(doc.toolpaths.len(), 2);
        let not_merged = UngroupParams {
            toolpath: EntityId(1),
        };
        let why = UngroupMergedToolpath::run(&doc, &not_merged)
            .unwrap_err()
            .to_string();
        assert!(why.contains("not a merged toolpath"), "{why}");
    }
}
