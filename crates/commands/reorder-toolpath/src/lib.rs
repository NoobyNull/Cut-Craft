//! Move Toolpath (the toolpath list's up and down arrows): a toolpath one place up or down
//! the list, which is the order it is cut and saved in. The two toolpaths trade places by
//! trading ids, and Merged and Array Copy toolpaths naming either by id in their `sources`
//! (or an Array Copy's `only`) are told of the trade, so they keep the toolpaths they were made from.

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::toolpath::Toolpath;
use cut_craft_core::{Change, Command, CommandError, Form, Meta, existing};
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

/// Each id in `params` naming another toolpath: its `sources`, and an Array Copy's `only`
/// (the one source it copies).
fn source_ids(params: &mut serde_json::Value) -> Vec<&mut serde_json::Value> {
    let Some(params) = params.as_object_mut() else {
        return vec![];
    };
    params
        .iter_mut()
        .flat_map(|(key, value)| match key.as_str() {
            "sources" => value
                .as_array_mut()
                .map_or_else(Vec::new, |ids| ids.iter_mut().collect()),
            "only" if value.is_u64() => vec![value],
            _ => vec![],
        })
        .collect()
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
            form: Form::Apply,
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
        // the two trade places; every other toolpath stays where it is
        let order: Vec<(EntityId, Toolpath)> = doc
            .toolpaths
            .iter()
            .map(|(&id, toolpath)| match id {
                id if id == p.toolpath => (other, doc.toolpaths[&other].clone()),
                id if id == other => (p.toolpath, moving.clone()),
                _ => (id, toolpath.clone()),
            })
            .collect();
        arranged(doc, &order)
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
    // what each toolpath cut after before: one that cuts only what those leave is out of
    // date once they are different
    let cuts = |t: &Toolpath| (t.moves.clone(), t.tool.clone(), t.power.clone());
    let above_before: std::collections::BTreeMap<EntityId, Vec<_>> = slots
        .iter()
        .enumerate()
        .map(|(i, id)| {
            (
                *id,
                slots[..i].iter().map(|s| cuts(&doc.toolpaths[s])).collect(),
            )
        })
        .collect();
    let mut c = Change::builder(doc);
    for (i, ((from, toolpath), &slot)) in order.iter().zip(&slots).enumerate() {
        let mut toolpath = toolpath.clone();
        toolpath.params = followed(&toolpath.params, &moved);
        if toolpath.cuts_what_is_left() {
            let above: Vec<_> = order[..i].iter().map(|(_, t)| cuts(t)).collect();
            toolpath.stale |= above_before.get(from) != Some(&above);
        }
        if doc.toolpaths.get(&slot) != Some(&toolpath) {
            c.replace_toolpath(slot, toolpath);
        }
    }
    Ok(c.finish())
}

/// `params` with each toolpath it names at its new id.
fn followed(
    params: &serde_json::Value,
    moved: &std::collections::BTreeMap<EntityId, EntityId>,
) -> serde_json::Value {
    let mut params = params.clone();
    for source in source_ids(&mut params) {
        let to = source.as_u64().and_then(|id| moved.get(&EntityId(id)));
        if let Some(to) = to {
            *source = serde_json::json!(to.0);
        }
    }
    params
}

/// Puts the toolpaths in a new cut order at once, as dragging one in the toolpath list or
/// moving a whole group does, and moves groups into other groups: one step to undo.
pub struct ArrangeToolpaths;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct ArrangeParams {
    /// Every toolpath, in the order they are to cut
    pub order: Vec<EntityId>,
    /// The group each toolpath named here goes in (empty: none); the others keep theirs
    #[serde(default)]
    pub groups: std::collections::BTreeMap<EntityId, String>,
    /// The group each toolpath group named here goes inside (empty: the top of the list);
    /// the others stay where they are
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub parents: std::collections::BTreeMap<String, String>,
}

impl Default for ArrangeParams {
    fn default() -> Self {
        Self {
            order: [2, 1, 3, 4].map(EntityId).to_vec(),
            groups: std::collections::BTreeMap::new(),
            parents: std::collections::BTreeMap::new(),
        }
    }
}

/// The groups' parents after `moves` (group to the group it goes inside, empty: the top).
/// Refused when a group would go inside itself or a group inside it.
fn moved_parents(
    doc: &Document,
    moves: &std::collections::BTreeMap<String, String>,
) -> Result<std::collections::BTreeMap<String, String>, CommandError> {
    let mut parents = doc.toolpath_group_parents.clone();
    for (group, parent) in moves {
        let parent = parent.trim();
        if parent.is_empty() {
            parents.remove(group);
            continue;
        }
        if doc
            .toolpath_group_and_subgroups(group)
            .iter()
            .any(|g| g == parent)
        {
            return Err(CommandError::Failed(format!(
                "{group} can't go inside {parent}: it is {group} or sits inside it"
            )));
        }
        parents.insert(group.clone(), parent.to_owned());
    }
    Ok(parents)
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
            form: Form::Apply,
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
        let parents = moved_parents(doc, &p.parents)?;
        let mut c = Change::builder(doc);
        // a group named only here (made to hold another) joins the list
        let mut groups = doc.toolpath_groups_in_use();
        for name in p.parents.iter().flat_map(|(g, at)| [g, at]) {
            let name = name.trim();
            if !name.is_empty() && !groups.iter().any(|g| g == name) {
                groups.push(name.to_owned());
            }
        }
        c.set_toolpath_groups(groups);
        c.set_toolpath_group_parents(parents);
        Ok(arranged(doc, &order)?.then(c.finish()))
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
        // an array copy's `only` names the one source it copies, as its `sources` do
        merged.params = serde_json::json!({ "name": "Merged", "sources": [1, 2], "only": 1 });
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
        assert_eq!(doc.toolpaths[&EntityId(4)].params["only"], 2, "A, now at 2");
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
                parents: std::collections::BTreeMap::new(),
            };
            run_on::<ArrangeToolpaths>(&mut doc, &p);
            assert_eq!(names(&doc), ["C", "A", "Merged", "B"]);
            // Merged was made from A and B, now at 2 and 4
            assert_eq!(
                doc.toolpaths[&EntityId(3)].params["sources"],
                serde_json::json!([2, 4])
            );
            assert_eq!(doc.toolpaths[&EntityId(3)].params["only"], 2, "A, now at 2");
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
                    parents: std::collections::BTreeMap::new(),
                };
                assert!(ArrangeToolpaths::run(&doc, &p).is_err());
            }
        }

        #[test]
        fn a_group_moves_inside_another_but_never_inside_itself() {
            let mut doc = listed();
            doc.toolpath_groups = vec!["Outside".into(), "Tabs".into()];
            let into = |group: &str, parent: &str| ArrangeParams {
                order: [1, 2, 3, 4].map(EntityId).to_vec(),
                groups: std::collections::BTreeMap::new(),
                parents: [(group.to_owned(), parent.to_owned())].into(),
            };
            run_on::<ArrangeToolpaths>(&mut doc, &into("Tabs", "Outside"));
            assert_eq!(
                doc.toolpath_group_parent("Tabs").as_deref(),
                Some("Outside")
            );
            assert!(ArrangeToolpaths::run(&doc, &into("Outside", "Tabs")).is_err());
            assert!(ArrangeToolpaths::run(&doc, &into("Outside", "Outside")).is_err());
            run_on::<ArrangeToolpaths>(&mut doc, &into("Tabs", ""));
            assert_eq!(doc.toolpath_group_parent("Tabs"), None, "back to the top");
            // a new group made to hold one joins the list
            run_on::<ArrangeToolpaths>(&mut doc, &into("Tabs", "Edge"));
            assert_eq!(doc.toolpath_groups, ["Outside", "Tabs", "Edge"]);
            assert_eq!(doc.toolpath_group_path("Tabs"), ["Edge", "Tabs"]);
        }
    }

    /// A, B and a Rotary Roughing (ids 1, 2, 3), each cutting differently.
    fn roughed_last() -> Document {
        let mut doc = three_toolpaths();
        let mut c = Change::builder(&doc);
        for (id, name) in [(1u32, "A"), (2, "B"), (3, "Rough")] {
            let mut t = doc.toolpaths[&EntityId(id.into())].clone();
            t.name = name.into();
            t.moves = vec![cut_craft_core::toolpath::Move {
                kind: cut_craft_core::toolpath::MoveKind::Cut,
                to: cut_craft_core::glam::DVec3::new(0.0, 0.0, -f64::from(id)),
            }];
            if name == "Rough" {
                t.strategy = "rotary_roughing_toolpath".into();
            }
            c.replace_toolpath(EntityId(id.into()), t);
        }
        doc.apply(&c.finish()).unwrap();
        doc
    }

    fn stale(doc: &Document) -> Vec<&str> {
        doc.toolpaths
            .values()
            .filter(|t| t.stale)
            .map(|t| t.name.as_str())
            .collect()
    }

    #[test]
    fn a_toolpath_cutting_what_is_left_is_out_of_date_once_what_is_above_it_changes() {
        // what is above the roughing changes order: it would cut differently
        let mut doc = roughed_last();
        run_on::<ReorderToolpath>(
            &mut doc,
            &Params {
                toolpath: EntityId(1),
                direction: Direction::Down,
            },
        );
        assert_eq!(names(&doc), ["B", "A", "Rough"]);
        assert_eq!(stale(&doc), ["Rough"]);
        // moved up above B: it no longer cuts after B
        let mut doc = roughed_last();
        run_on::<ReorderToolpath>(
            &mut doc,
            &Params {
                toolpath: EntityId(3),
                direction: Direction::Up,
            },
        );
        assert_eq!(names(&doc), ["A", "Rough", "B"]);
        assert_eq!(stale(&doc), ["Rough"]);
        // one made for the whole blank (Settings' advanced rotary mode) never goes out of date
        let mut doc = roughed_last();
        let mut c = Change::builder(&doc);
        let mut whole = doc.toolpaths[&EntityId(3)].clone();
        whole.params = serde_json::json!({ "whole_blank": true });
        c.replace_toolpath(EntityId(3), whole);
        doc.apply(&c.finish()).unwrap();
        run_on::<ReorderToolpath>(
            &mut doc,
            &Params {
                toolpath: EntityId(1),
                direction: Direction::Down,
            },
        );
        assert!(stale(&doc).is_empty(), "{:?}", stale(&doc));
    }

    #[test]
    fn a_toolpath_cutting_what_is_left_is_out_of_date_once_one_above_it_is_made_again() {
        let mut doc = roughed_last();
        let mut c = Change::builder(&doc);
        let mut a = doc.toolpaths[&EntityId(1)].clone();
        a.moves[0].to.z -= 1.0;
        c.replace_toolpath(EntityId(1), a);
        let change = c.finish().marking_stale(&doc);
        doc.apply(&change).unwrap();
        assert_eq!(stale(&doc), ["Rough"]);
        // renamed only, it cuts the same: nothing goes out of date
        let mut doc = roughed_last();
        let mut c = Change::builder(&doc);
        let mut a = doc.toolpaths[&EntityId(1)].clone();
        a.name = "A again".into();
        c.replace_toolpath(EntityId(1), a);
        let change = c.finish().marking_stale(&doc);
        doc.apply(&change).unwrap();
        assert_eq!(stale(&doc), Vec::<&str>::new());
    }
}
