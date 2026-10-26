//! Group Components: the selected components (two or more, on one level) made one group of
//! the tree. Its members combine with each other first; the group then combines, Add to
//! start with, where its top member is.

use cut_craft_core::component::{Combine, Group};
use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_named};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct GroupComponents;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Group".into(),
        }
    }
}

impl Command for GroupComponents {
    const ID: &'static str = "group_components";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Group Components",
            group: "Modeling",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let picked: Vec<EntityId> = doc
            .selected_components
            .iter()
            .copied()
            .filter(|id| doc.components.contains_key(id))
            .collect();
        if picked.len() < 2 {
            return Err(CommandError::Failed(
                "select two or more components to group".into(),
            ));
        }
        let level = doc.components[&picked[0]].level;
        if picked.iter().any(|id| doc.components[id].level != level) {
            return Err(CommandError::Failed(
                "components on different levels cannot be grouped".into(),
            ));
        }
        let id = doc
            .components
            .values()
            .filter_map(|k| k.group.as_ref().map(|g| g.id))
            .max()
            .map_or(1, |last| last + 1);
        let group = Group {
            id,
            name: ensure_named(&p.name)?,
            combine: Combine::Add,
        };
        let mut c = Change::builder(doc);
        for member in picked {
            let mut k = doc.components[&member].clone();
            k.group = Some(group.clone());
            c.replace_component(member, k);
        }
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{domed, run_on, two_components};

    fn both_picked() -> Document {
        let mut doc = two_components();
        doc.selected_components = [EntityId(1), EntityId(2)].into();
        doc
    }

    cut_craft_core::contract_tests!(GroupComponents, both_picked);

    #[test]
    fn the_picked_components_share_a_new_group() {
        let mut doc = both_picked();
        run_on::<GroupComponents>(&mut doc, &Params::default());
        let groups: Vec<_> = doc.components.values().map(|k| k.group.clone()).collect();
        assert!(groups[0].is_some() && groups[0] == groups[1]);
        assert_eq!(groups[0].as_ref().unwrap().combine, Combine::Add);
    }

    #[test]
    fn one_component_or_two_levels_are_refused() {
        let mut one = domed();
        one.selected_components = [EntityId(1)].into();
        assert!(GroupComponents::run(&one, &Params::default()).is_err());
        let mut apart = both_picked();
        let mut k = apart.components[&EntityId(2)].clone();
        k.level = 4;
        let mut c = Change::builder(&apart);
        c.replace_component(EntityId(2), k);
        apart.apply(&c.finish()).unwrap();
        assert!(GroupComponents::run(&apart, &Params::default()).is_err());
    }
}
