//! Merged Toolpath: the visible toolpaths that use one tool made into one, with their cuts
//! in whichever order leaves the least travelling between them.

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::glam::DVec3;
use cut_craft_core::toolpath::{Move, MoveKind, Toolpath, made_by};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct MergedToolpath;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
    /// The toolpaths to merge (none: the visible ones)
    pub sources: Vec<EntityId>,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Merged".into(),
            sources: vec![],
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

/// The cuts, each next one the nearest start to where the last ended.
// ponytail: nearest neighbour; Aspire also tries merging by part
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
            icon: "toolpaths/merge_toolpaths.svg",
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
        if let Some(other) = sources.iter().find(|t| t.tool.name != first.tool.name) {
            return Err(CommandError::Failed(format!(
                "only toolpaths with the same tool merge: {} uses {} and {} uses {}",
                first.name, first.tool.name, other.name, other.tool.name
            )));
        }
        let in_order: Vec<Cut> = sources.iter().flat_map(|t| cuts(t)).collect();
        let nearest = nearest_first(in_order.clone());
        // the quicker of the toolpath list's order and nearest-first
        let order = if travel(&nearest) < travel(&in_order) {
            nearest
        } else {
            in_order
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
        let mut toolpath = made_by::<Self>(p, p.name.clone(), first.tool.clone(), vec![], moves);
        if laser {
            toolpath.power = power;
        }
        let mut c = Change::builder(doc);
        c.add_toolpath(toolpath);
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
}
