//! Move Component (the component tree's blue arrows): a component one place up or down
//! among those on its level. The tree combines from the bottom up, so the order changes the
//! model. The two components trade places by trading ids, so the moved one ends up at
//! [`neighbour`].

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::{Change, Command, CommandError, Meta, existing};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct ReorderComponent;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// Toward the top of the tree: combined later
    #[default]
    Up,
    /// Toward the bottom: combined sooner
    Down,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub component: EntityId,
    pub direction: Direction,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            component: EntityId(1),
            direction: Direction::Up,
        }
    }
}

/// The component on the same level next to `id` in `direction`, if there is one.
pub fn neighbour(doc: &Document, id: EntityId, direction: Direction) -> Option<EntityId> {
    let level = doc.components.get(&id)?.level;
    let same = |(k, c): (&EntityId, &cut_craft_core::component::Component)| {
        (c.level == level).then_some(*k)
    };
    match direction {
        Direction::Up => doc.components.range(id..).skip(1).find_map(same),
        Direction::Down => doc.components.range(..id).rev().find_map(same),
    }
}

impl Command for ReorderComponent {
    const ID: &'static str = "reorder_component";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Move Component",
            group: "Modeling",
            icon: "",
            shortcut: "",
        }
    }

    fn initial_params(doc: &Document) -> Params {
        Params {
            component: doc.chosen_component().unwrap_or(EntityId(1)),
            ..Params::default()
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let moving = existing(&doc.components, p.component, "component")?;
        let other = neighbour(doc, p.component, p.direction).ok_or_else(|| {
            CommandError::Failed(match p.direction {
                Direction::Up => "it is already at the top of its level".into(),
                Direction::Down => "it is already at the bottom of its level".into(),
            })
        })?;
        let mut c = Change::builder(doc);
        c.replace_component(other, moving.clone());
        c.replace_component(p.component, doc.components[&other].clone());
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::component::Combine;
    use cut_craft_core::testing::{run_on, two_components};

    /// Two domes, the top one (2) subtracting.
    fn bowl_over_dome() -> Document {
        let mut doc = two_components();
        let mut c = Change::builder(&doc);
        let mut bowl = doc.components[&EntityId(2)].clone();
        bowl.name = "bowl".into();
        bowl.combine = Combine::Subtract;
        c.replace_component(EntityId(2), bowl);
        doc.apply(&c.finish()).unwrap();
        doc
    }

    cut_craft_core::contract_tests!(ReorderComponent, bowl_over_dome);

    #[test]
    fn moving_down_trades_places_with_the_one_below() {
        let mut doc = bowl_over_dome();
        let p = Params {
            component: EntityId(2),
            direction: Direction::Down,
        };
        assert_eq!(
            neighbour(&doc, EntityId(2), Direction::Down),
            Some(EntityId(1))
        );
        run_on::<ReorderComponent>(&mut doc, &p);
        assert_eq!(doc.components[&EntityId(1)].name, "bowl");
        assert_eq!(doc.components[&EntityId(2)].name, "dome");
    }

    #[test]
    fn the_ends_and_other_levels_are_refused() {
        let mut doc = bowl_over_dome();
        let up = Params {
            component: EntityId(2),
            direction: Direction::Up,
        };
        assert!(ReorderComponent::run(&doc, &up).is_err(), "already on top");
        let mut c = Change::builder(&doc);
        let mut other = doc.components[&EntityId(1)].clone();
        other.level = 3;
        c.replace_component(EntityId(1), other);
        doc.apply(&c.finish()).unwrap();
        let down = Params {
            direction: Direction::Down,
            ..up
        };
        assert!(
            ReorderComponent::run(&doc, &down).is_err(),
            "nothing below on its level"
        );
    }
}
