//! Insert New Level: an empty level on top of the component tree.

use cut_craft_core::component::Level;
use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_named};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct InsertLevel;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Level 2".into(),
        }
    }
}

impl Command for InsertLevel {
    const ID: &'static str = "insert_level";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Insert New Level",
            group: "Modeling",
            icon: "",
            shortcut: "",
        }
    }

    fn initial_params(doc: &Document) -> Params {
        Params {
            name: format!("Level {}", doc.levels.len() + 1),
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let id = doc.levels.keys().next_back().map_or(0, |last| last + 1);
        let mut c = Change::builder(doc);
        c.set_level(
            id,
            Some(Level {
                name: ensure_named(&p.name)?,
                ..Level::default()
            }),
        );
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{domed, run_on};

    cut_craft_core::contract_tests!(InsertLevel, domed);

    #[test]
    fn adds_a_named_level_on_top() {
        let mut doc = domed();
        let p = InsertLevel::initial_params(&doc);
        assert_eq!(p.name, "Level 2");
        run_on::<InsertLevel>(&mut doc, &p);
        let (&top, level) = doc.levels.iter().next_back().unwrap();
        assert_eq!((top, level.name.as_str()), (1, "Level 2"));
    }
}
