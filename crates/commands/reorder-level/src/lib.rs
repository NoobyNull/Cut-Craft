//! Move Level: a level one place up or down the component tree. Levels combine from the
//! bottom up, so the order changes the model. The two levels trade ids, their components
//! going with them.

use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct ReorderLevel;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// Toward the top of the tree: combined later
    #[default]
    Up,
    /// Toward the bottom: combined sooner
    Down,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    #[schemars(skip)]
    #[serde(default)]
    pub level: u64,
    pub direction: Direction,
}

impl Command for ReorderLevel {
    const ID: &'static str = "reorder_level";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Move Level",
            group: "Modeling",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let moving = doc
            .levels
            .get(&p.level)
            .ok_or_else(|| CommandError::Failed(format!("level {} does not exist", p.level)))?;
        let other = match p.direction {
            Direction::Up => doc.levels.range(p.level + 1..).next(),
            Direction::Down => doc.levels.range(..p.level).next_back(),
        }
        .map(|(&id, _)| id)
        .ok_or_else(|| CommandError::Failed("there is no level that way".into()))?;
        let mut c = Change::builder(doc);
        c.set_level(other, Some(moving.clone()));
        c.set_level(p.level, Some(doc.levels[&other].clone()));
        for (&id, k) in &doc.components {
            let to = if k.level == p.level {
                other
            } else if k.level == other {
                p.level
            } else {
                continue;
            };
            let mut moved = k.clone();
            moved.level = to;
            c.replace_component(id, moved);
        }
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::component::Level;
    use cut_craft_core::document::EntityId;
    use cut_craft_core::testing::{domed, run_on};

    /// The dome on the first level, and an empty "Top" level above it.
    fn two_levels() -> Document {
        let mut doc = domed();
        let mut c = Change::builder(&doc);
        c.set_level(
            1,
            Some(Level {
                name: "Top".into(),
                ..Level::default()
            }),
        );
        doc.apply(&c.finish()).unwrap();
        doc
    }

    cut_craft_core::contract_tests!(ReorderLevel, two_levels);

    #[test]
    fn a_level_moves_up_with_its_components() {
        let mut doc = two_levels();
        run_on::<ReorderLevel>(&mut doc, &Params::default());
        assert_eq!(doc.levels[&0].name, "Top");
        assert_eq!(doc.levels[&1].name, "Level 1");
        assert_eq!(doc.components[&EntityId(1)].level, 1);
    }

    #[test]
    fn the_ends_are_refused() {
        let doc = two_levels();
        let down = Params {
            level: 0,
            direction: Direction::Down,
        };
        assert!(ReorderLevel::run(&doc, &down).is_err());
    }
}
