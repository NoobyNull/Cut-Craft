//! Component Properties: a component's name, how it combines with the model under it, and
//! whether it is part of the model.

use cut_craft_core::component::{Combine, Component};
use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_named, existing};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct ComponentProperties;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub component: EntityId,
    pub name: String,
    #[schemars(title = "Combine Mode")]
    pub combine: Combine,
    /// Hidden components are left out of the model
    pub visible: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            component: EntityId(1),
            name: "Component".into(),
            combine: Combine::Add,
            visible: true,
        }
    }
}

impl Command for ComponentProperties {
    const ID: &'static str = "component_properties";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Component Properties",
            group: "Modeling",
            icon: "",
            shortcut: "",
        }
    }

    fn initial_params(doc: &Document) -> Params {
        doc.chosen_component()
            .map(|id| (id, &doc.components[&id]))
            .map_or_else(Params::default, |(component, c)| Params {
                component,
                name: c.name.clone(),
                combine: c.combine,
                visible: c.visible,
            })
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let component = existing(&doc.components, p.component, "component")?;
        let after = Component {
            name: ensure_named(&p.name)?,
            combine: p.combine,
            visible: p.visible,
            ..component.clone()
        };
        let mut c = Change::builder(doc);
        c.replace_component(p.component, after);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{domed, rejection, run_on};

    cut_craft_core::contract_tests!(ComponentProperties, domed);

    #[test]
    fn renames_recombines_and_hides_a_component() {
        let mut doc = domed();
        let p = Params {
            name: "Bowl".into(),
            combine: Combine::Subtract,
            visible: false,
            ..ComponentProperties::initial_params(&doc)
        };
        run_on::<ComponentProperties>(&mut doc, &p);
        let c = &doc.components[&EntityId(1)];
        assert_eq!(
            (c.name.as_str(), c.combine, c.visible),
            ("Bowl", Combine::Subtract, false)
        );
    }

    #[test]
    fn needs_the_component() {
        assert!(rejection::<ComponentProperties>(&Params::default()).contains("does not exist"));
    }
}
