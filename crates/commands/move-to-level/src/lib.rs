//! Move to Level: the selected components onto another level.

use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct MoveToLevel;

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    #[schemars(skip)]
    #[serde(default)]
    pub level: u64,
}

impl Command for MoveToLevel {
    const ID: &'static str = "move_to_level";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Move to Level",
            group: "Modeling",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if !doc.levels.contains_key(&p.level) {
            return Err(CommandError::Failed(format!(
                "level {} does not exist",
                p.level
            )));
        }
        let picked: Vec<_> = doc
            .selected_components
            .iter()
            .filter_map(|id| doc.components.get(id).map(|k| (*id, k)))
            .collect();
        if picked.is_empty() {
            return Err(CommandError::NothingSelected);
        }
        let mut c = Change::builder(doc);
        for (id, k) in picked.into_iter().filter(|(_, k)| k.level != p.level) {
            let mut moved = k.clone();
            moved.level = p.level;
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

    fn dome_picked_two_levels() -> Document {
        let mut doc = domed();
        let mut c = Change::builder(&doc);
        c.set_level(1, Some(Level::default()));
        doc.apply(&c.finish()).unwrap();
        doc.selected_components = [EntityId(1)].into();
        doc
    }

    cut_craft_core::contract_tests!(MoveToLevel, dome_picked_two_levels);

    #[test]
    fn moves_the_selected_component() {
        let mut doc = dome_picked_two_levels();
        run_on::<MoveToLevel>(&mut doc, &Params { level: 1 });
        assert_eq!(doc.components[&EntityId(1)].level, 1);
    }

    #[test]
    fn needs_a_selection_and_the_level() {
        let doc = dome_picked_two_levels();
        assert!(MoveToLevel::run(&doc, &Params { level: 4 }).is_err());
        assert_eq!(
            MoveToLevel::run(&domed(), &Params { level: 0 }),
            Err(CommandError::NothingSelected)
        );
    }
}
