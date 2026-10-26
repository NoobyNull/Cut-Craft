//! Group Properties: a component group's name and how it combines with what is under it,
//! set on every member.

use cut_craft_core::component::{Combine, Group};
use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_named};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct GroupProperties;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    #[schemars(skip)]
    #[serde(default)]
    pub group: u64,
    pub name: String,
    #[schemars(title = "Combine Mode")]
    pub combine: Combine,
}

impl Params {
    /// The settings group `g` has now, so a caller can change one and keep the rest.
    pub fn of(g: &Group) -> Self {
        Self {
            group: g.id,
            name: g.name.clone(),
            combine: g.combine,
        }
    }
}

impl Default for Params {
    fn default() -> Self {
        Self {
            group: 1,
            name: "Group".into(),
            combine: Combine::Add,
        }
    }
}

impl Command for GroupProperties {
    const ID: &'static str = "group_properties";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Group Properties",
            group: "Modeling",
            icon: "",
            shortcut: "",
        }
    }

    fn initial_params(doc: &Document) -> Params {
        doc.chosen_component()
            .and_then(|id| doc.components[&id].group.as_ref())
            .map_or_else(Params::default, Params::of)
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let group = Group {
            id: p.group,
            name: ensure_named(&p.name)?,
            combine: p.combine,
        };
        let mut c = Change::builder(doc);
        let mut found = false;
        for (&id, k) in &doc.components {
            if k.group.as_ref().is_some_and(|g| g.id == p.group) {
                found = true;
                let mut member = k.clone();
                member.group = Some(group.clone());
                c.replace_component(id, member);
            }
        }
        if !found {
            return Err(CommandError::Failed(format!(
                "group {} does not exist",
                p.group
            )));
        }
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
                combine: Combine::Add,
            });
            c.replace_component(id, k);
        }
        doc.apply(&c.finish()).unwrap();
        doc
    }

    cut_craft_core::contract_tests!(GroupProperties, grouped);

    #[test]
    fn every_member_gets_the_new_combine_mode() {
        let mut doc = grouped();
        let p = Params {
            combine: Combine::Subtract,
            ..GroupProperties::initial_params(&doc)
        };
        run_on::<GroupProperties>(&mut doc, &p);
        assert!(
            doc.components
                .values()
                .all(|k| k.group.as_ref().unwrap().combine == Combine::Subtract)
        );
    }

    #[test]
    fn a_missing_group_is_refused() {
        let p = Params {
            group: 7,
            ..Params::default()
        };
        assert!(GroupProperties::run(&grouped(), &p).is_err());
    }
}
