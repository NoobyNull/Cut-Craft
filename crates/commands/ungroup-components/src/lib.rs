//! Ungroup Components: the group of the chosen component taken apart, its members left as
//! lone components where they are.

use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct UngroupComponents;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// The group to take apart
    #[schemars(skip)]
    #[serde(default)]
    pub group: u64,
}

impl Default for Params {
    fn default() -> Self {
        Self { group: 1 }
    }
}

impl Command for UngroupComponents {
    const ID: &'static str = "ungroup_components";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Ungroup Components",
            group: "Modeling",
            icon: "",
            shortcut: "",
        }
    }

    fn initial_params(doc: &Document) -> Params {
        Params {
            group: doc
                .chosen_component()
                .and_then(|id| doc.components[&id].group.as_ref())
                .map_or(0, |g| g.id),
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let members: Vec<_> = doc
            .components
            .iter()
            .filter(|(_, k)| k.group.as_ref().is_some_and(|g| g.id == p.group))
            .collect();
        if members.is_empty() {
            return Err(CommandError::Failed(format!(
                "group {} does not exist",
                p.group
            )));
        }
        let mut c = Change::builder(doc);
        for (&id, k) in members {
            let mut lone = k.clone();
            lone.group = None;
            c.replace_component(id, lone);
        }
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::component::{Combine, Group};
    use cut_craft_core::document::EntityId;
    use cut_craft_core::testing::{run_on, two_components};

    fn grouped() -> Document {
        let mut doc = two_components();
        let mut c = Change::builder(&doc);
        for id in [EntityId(1), EntityId(2)] {
            let mut k = doc.components[&id].clone();
            k.group = Some(Group {
                id: 1,
                name: "Pair".into(),
                combine: Combine::Merge,
            });
            c.replace_component(id, k);
        }
        doc.apply(&c.finish()).unwrap();
        doc
    }

    cut_craft_core::contract_tests!(UngroupComponents, grouped);

    #[test]
    fn every_member_is_let_go() {
        let mut doc = grouped();
        let p = UngroupComponents::initial_params(&doc);
        assert_eq!(p.group, 1);
        run_on::<UngroupComponents>(&mut doc, &p);
        assert!(doc.components.values().all(|k| k.group.is_none()));
    }

    #[test]
    fn a_missing_group_is_refused() {
        assert!(UngroupComponents::run(&grouped(), &Params { group: 9 }).is_err());
    }
}
