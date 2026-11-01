//! Move Toolpath (the toolpath list's up and down arrows): a toolpath one place up or down
//! the list, which is the order it is cut and saved in. The two toolpaths trade places by
//! trading ids, and Merged and Array Copy toolpaths naming either by id in their `sources`
//! are told of the trade, so they keep the toolpaths they were made from.

use cut_craft_core::document::{Document, EntityId};
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
    pub toolpath: EntityId,
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
}
