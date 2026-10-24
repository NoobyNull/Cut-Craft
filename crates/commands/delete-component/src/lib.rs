//! Delete Component.

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::{Change, Command, CommandError, Meta, existing};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct DeleteComponent;

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

impl Command for DeleteComponent {
    const ID: &'static str = "delete_component";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Delete Component",
            group: "Modeling",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        existing(&doc.components, p.component, "component")?;
        let mut c = Change::builder(doc);
        c.remove_component(p.component);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{domed, rejection, run_on};

    cut_craft_core::contract_tests!(DeleteComponent, domed);

    #[test]
    fn removes_the_component() {
        let mut doc = domed();
        run_on::<DeleteComponent>(&mut doc, &Params::default());
        assert!(doc.components.is_empty());
        assert!(rejection::<DeleteComponent>(&Params::default()).contains("does not exist"));
    }
}
