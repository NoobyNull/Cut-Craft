//! Move Toolpath (the toolpath list's up and down arrows): a toolpath one place up or down
//! the list, which is the order it is cut and saved in. The two toolpaths trade places by
//! trading ids, and Merged and Array Copy toolpaths naming either by id in their `sources`
//! are told of the trade, so they keep the toolpaths they were made from.

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::toolpath::Toolpath;
use cut_craft_core::{Change, Command, CommandError, Meta, existing};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct ReorderToolpath;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// Toward the top of the list: cut sooner
    #[default]
    Up,
    /// Toward the bottom: cut later
    Down,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// The toolpath to move in the list
    pub toolpath: EntityId,
    /// Which way it moves in the toolpath list
    pub direction: Direction,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            toolpath: EntityId(1),
            direction: Direction::Down,
        }
    }
}

/// The toolpath next to `id` in the list, in `direction`.
pub fn neighbour(doc: &Document, id: EntityId, direction: Direction) -> Option<EntityId> {
    match direction {
        Direction::Up => doc.toolpaths.range(..id).next_back().map(|(&k, _)| k),
        Direction::Down => doc.toolpaths.range(id..).nth(1).map(|(&k, _)| k),
    }
}

/// `params` with `a` and `b` traded wherever its `sources` name them.
fn traded(params: &serde_json::Value, a: EntityId, b: EntityId) -> serde_json::Value {
    let mut params = params.clone();
    if let Some(sources) = params
        .get_mut("sources")
        .and_then(serde_json::Value::as_array_mut)
    {
        for source in sources {
            if *source == serde_json::json!(a.0) {
                *source = serde_json::json!(b.0);
            } else if *source == serde_json::json!(b.0) {
                *source = serde_json::json!(a.0);
            }
        }
    }
    params
}

impl Command for ReorderToolpath {
    const ID: &'static str = "reorder_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Move Toolpath",
            group: "Toolpaths",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let moving = existing(&doc.toolpaths, p.toolpath, "toolpath")?;
        let other = neighbour(doc, p.toolpath, p.direction).ok_or_else(|| {
            CommandError::Failed(match p.direction {
                Direction::Up => "it is already at the top of the list".into(),
                Direction::Down => "it is already at the bottom of the list".into(),
            })
        })?;
        let mut c = Change::builder(doc);
        for (&id, toolpath) in &doc.toolpaths {
            let mut toolpath = match id {
                id if id == p.toolpath => doc.toolpaths[&other].clone(),
                id if id == other => moving.clone(),
                _ => toolpath.clone(),
            };
            let params = traded(&toolpath.params, p.toolpath, other);
            let swapped = id == p.toolpath || id == other;
            if swapped || params != toolpath.params {
                toolpath.params = params;
                c.replace_toolpath(id, toolpath);
            }
        }
        Ok(c.finish())
    }
}

/// The change that leaves the toolpaths in cut order `order`: each entry an id the toolpath
/// had and what it holds now (its group may have changed), every toolpath once. The ids keep
/// their places; each takes the toolpath that comes there, and toolpaths made from others
/// (Merged, Array Copy) follow theirs to their new ids.
///
/// # Errors
/// When `order` does not name every toolpath exactly once.
pub fn arranged(doc: &Document, order: &[(EntityId, Toolpath)]) -> Result<Change, CommandError> {
    let mut named: Vec<EntityId> = order.iter().map(|(id, _)| *id).collect();
    named.sort_unstable();
    named.dedup();
    let slots: Vec<EntityId> = doc.toolpaths.keys().copied().collect();
    if named != slots || order.len() != slots.len() {
        return Err(CommandError::Failed(
            "the new order must name every toolpath once".into(),
        ));
    }
    let moved: std::collections::BTreeMap<EntityId, EntityId> = order
        .iter()
        .zip(&slots)
        .map(|((from, _), &to)| (*from, to))
        .collect();
    let mut c = Change::builder(doc);
    for ((_, toolpath), &slot) in order.iter().zip(&slots) {
        let mut toolpath = toolpath.clone();
        toolpath.params = followed(&toolpath.params, &moved);
        if doc.toolpaths.get(&slot) != Some(&toolpath) {
            c.replace_toolpath(slot, toolpath);
        }
    }
    Ok(c.finish())
}

/// `params` with each toolpath its `sources` name at its new id.
fn followed(
    params: &serde_json::Value,
    moved: &std::collections::BTreeMap<EntityId, EntityId>,
) -> serde_json::Value {
    let mut params = params.clone();
    if let Some(sources) = params
        .get_mut("sources")
        .and_then(serde_json::Value::as_array_mut)
    {
        for source in sources {
            let to = source.as_u64().and_then(|id| moved.get(&EntityId(id)));
            if let Some(to) = to {
                *source = serde_json::json!(to.0);
            }
        }
    }
    params
}

/// Puts the toolpaths in a new cut order at once, as dragging one in the toolpath list or
/// moving a whole group does: one step to undo.
pub struct ArrangeToolpaths;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct ArrangeParams {
    /// Every toolpath, in the order they are to cut
    pub order: Vec<EntityId>,
    /// The group each toolpath named here goes in (empty: none); the others keep theirs
    #[serde(default)]
    pub groups: std::collections::BTreeMap<EntityId, String>,
}

impl Default for ArrangeParams {
    fn default() -> Self {
        Self {
            order: [2, 1, 3, 4].map(EntityId).to_vec(),
            groups: std::collections::BTreeMap::new(),
        }
    }
}

impl Command for ArrangeToolpaths {
    const ID: &'static str = "arrange_toolpaths";
    type Params = ArrangeParams;

    fn meta() -> Meta {
        Meta {
            label: "Arrange Toolpaths",
            group: "Toolpaths",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &ArrangeParams) -> Result<Change, CommandError> {
        let order = p
            .order
            .iter()
            .map(|&id| {
                let mut toolpath = existing(&doc.toolpaths, id, "toolpath")?.clone();
                if let Some(group) = p.groups.get(&id) {
                    let group = group.trim();
                    toolpath.group = (!group.is_empty()).then(|| group.to_owned());
                }
                Ok((id, toolpath))
            })
            .collect::<Result<Vec<_>, CommandError>>()?;
        arranged(doc, &order)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{run_on, three_toolpaths};

    /// Toolpaths named A, B and C (ids 1, 2, 3) and a Merged one (4) made from A and B.
    fn listed() -> Document {
        let mut doc = three_toolpaths();
        let mut c = Change::builder(&doc);
        for (id, name) in [(1, "A"), (2, "B"), (3, "C")] {
            let mut t = doc.toolpaths[&EntityId(id)].clone();
            t.name = name.into();
            c.replace_toolpath(EntityId(id), t);
        }
        let mut merged = doc.toolpaths[&EntityId(1)].clone();
        merged.name = "Merged".into();
        merged.params = serde_json::json!({ "name": "Merged", "sources": [1, 2] });
        c.add_toolpath(merged);
        doc.apply(&c.finish()).unwrap();
        doc
    }

    fn names(doc: &Document) -> Vec<&str> {
        doc.toolpaths.values().map(|t| t.name.as_str()).collect()
    }

    cut_craft_core::contract_tests!(ReorderToolpath, listed);

    #[test]
    fn moving_trades_places_with_the_neighbour_and_keeps_merged_sources() {
        let mut doc = listed();
        run_on::<ReorderToolpath>(
            &mut doc,
            &Params {
                toolpath: EntityId(1),
                direction: Direction::Down,
            },
        );
        assert_eq!(names(&doc), ["B", "A", "C", "Merged"]);
        // the merged toolpath still names A and B, now at 2 and 1
        assert_eq!(
            doc.toolpaths[&EntityId(4)].params["sources"],
            serde_json::json!([2, 1])
        );
        run_on::<ReorderToolpath>(
            &mut doc,
            &Params {
                toolpath: EntityId(3),
                direction: Direction::Up,
            },
        );
        assert_eq!(names(&doc), ["B", "C", "A", "Merged"]);
        assert_eq!(
            doc.toolpaths[&EntityId(4)].params["sources"],
            serde_json::json!([3, 1])
        );
    }

    #[test]
    fn the_ends_of_the_list_are_refused() {
        let doc = listed();
        let up = Params {
            toolpath: EntityId(1),
            direction: Direction::Up,
        };
        assert!(ReorderToolpath::run(&doc, &up).is_err(), "already first");
        let down = Params {
            toolpath: EntityId(4),
            direction: Direction::Down,
        };
        assert!(ReorderToolpath::run(&doc, &down).is_err(), "already last");
    }

    mod arrange {
        use super::*;

        cut_craft_core::contract_tests!(ArrangeToolpaths, listed);

        #[test]
        fn a_new_order_moves_every_toolpath_at_once_and_keeps_merged_sources() {
            let mut doc = listed();
            let p = ArrangeParams {
                order: [3, 1, 4, 2].map(EntityId).to_vec(),
                groups: [(EntityId(3), "Outside".to_owned())].into(),
            };
            run_on::<ArrangeToolpaths>(&mut doc, &p);
            assert_eq!(names(&doc), ["C", "A", "Merged", "B"]);
            // Merged was made from A and B, now at 2 and 4
            assert_eq!(
                doc.toolpaths[&EntityId(3)].params["sources"],
                serde_json::json!([2, 4])
            );
            assert_eq!(
                doc.toolpaths[&EntityId(1)].group.as_deref(),
                Some("Outside")
            );
        }

        #[test]
        fn an_order_missing_a_toolpath_or_naming_one_twice_is_refused() {
            let doc = listed();
            for order in [vec![1, 2, 3], vec![1, 2, 3, 3], vec![1, 2, 3, 4, 4]] {
                let p = ArrangeParams {
                    order: order.into_iter().map(EntityId).collect(),
                    groups: std::collections::BTreeMap::new(),
                };
                assert!(ArrangeToolpaths::run(&doc, &p).is_err());
            }
        }
    }
}
