//! Delete Level: a level and the components on it; a job keeps at least one level.

use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct DeleteLevel;

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    #[schemars(skip)]
    #[serde(default)]
    pub level: u64,
}

impl Command for DeleteLevel {
    const ID: &'static str = "delete_level";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Delete Level",
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
        if doc.levels.len() == 1 {
            return Err(CommandError::Failed(
                "a job keeps at least one level".into(),
            ));
        }
        let mut c = Change::builder(doc);
        for (&id, _) in doc.components.iter().filter(|(_, k)| k.level == p.level) {
            c.remove_component(id);
        }
        c.set_level(p.level, None);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::component::Level;
    use cut_craft_core::document::EntityId;
    use cut_craft_core::testing::{domed, run_on};

    fn two_levels() -> Document {
        let mut doc = domed();
        let mut c = Change::builder(&doc);
        c.set_level(1, Some(Level::default()));
        let mut k = doc.components[&EntityId(1)].clone();
        k.level = 1;
        c.add_component(k);
        doc.apply(&c.finish()).unwrap();
        doc
    }

    cut_craft_core::contract_tests!(DeleteLevel, two_levels);

    #[test]
    fn removes_the_level_and_its_components_only() {
        let mut doc = two_levels();
        run_on::<DeleteLevel>(&mut doc, &Params { level: 1 });
        assert_eq!(doc.levels.len(), 1);
        assert_eq!(doc.components.len(), 1);
        assert_eq!(doc.components[&EntityId(1)].level, 0);
    }

    #[test]
    fn the_last_level_stays() {
        assert!(DeleteLevel::run(&domed(), &Params { level: 0 }).is_err());
    }
}
