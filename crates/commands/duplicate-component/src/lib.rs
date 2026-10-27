//! Duplicate Component: a copy of a component, in the same place on the same level, just
//! above it in the tree (the top of the tree, as new components go).

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::{Change, Command, CommandError, Meta, existing};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct DuplicateComponent;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub component: EntityId,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            component: EntityId(1),
        }
    }
}

impl Command for DuplicateComponent {
    const ID: &'static str = "duplicate_component";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Duplicate Component",
            group: "Modeling",
            icon: "",
            shortcut: "",
        }
    }

    fn initial_params(doc: &Document) -> Params {
        Params {
            component: doc.chosen_component().unwrap_or(EntityId(1)),
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let mut copy = existing(&doc.components, p.component, "component")?.clone();
        // ponytail: Aspire's copy name is not documented [inf]
        copy.name = format!("{} copy", copy.name);
        let mut c = Change::builder(doc);
        c.add_component(copy);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{domed, rejection, run_on};

    cut_craft_core::contract_tests!(DuplicateComponent, domed);

    #[test]
    fn copies_the_component_with_its_heights_and_level() {
        let mut doc = domed();
        let p = DuplicateComponent::initial_params(&doc);
        run_on::<DuplicateComponent>(&mut doc, &p);
        let [one, two] = [EntityId(1), EntityId(2)].map(|id| doc.components[&id].clone());
        assert_eq!(two.name, "dome copy");
        assert!(cut_craft_core::component::same_heights(
            &two.heights,
            &one.heights
        ));
        assert_eq!(two.level, one.level);
    }

    #[test]
    fn needs_the_component() {
        assert!(rejection::<DuplicateComponent>(&Params::default()).contains("does not exist"));
    }
}
