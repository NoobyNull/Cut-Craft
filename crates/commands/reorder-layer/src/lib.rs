//! Move Layer (the Layers List's arrows): a layer one place up or down the list, the order
//! the layers are listed and drawn in. The list shows the document's last layer at the top.

use cut_craft_core::document::{Document, LayerId};
use cut_craft_core::{Change, Command, CommandError, Meta, existing_layer};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct ReorderLayer;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// Toward the top of the list
    #[default]
    Up,
    /// Toward the bottom
    Down,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// The layer to move in the list
    pub layer: LayerId,
    /// Which way it moves in the Layers list
    pub direction: Direction,
}

/// Where layer `id` would go moved one place in `direction`, if it can move.
pub fn moved_to(doc: &Document, id: LayerId, direction: Direction) -> Option<usize> {
    let at = doc.layers.iter().position(|l| l.id == id)?;
    match direction {
        Direction::Up => (at + 1 < doc.layers.len()).then_some(at + 1),
        Direction::Down => at.checked_sub(1),
    }
}

impl Command for ReorderLayer {
    const ID: &'static str = "reorder_layer";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Move Layer",
            group: "Layers",
            icon: "",
            shortcut: "",
        }
    }

    fn initial_params(doc: &Document) -> Params {
        Params {
            layer: doc.current_layer,
            direction: Direction::Up,
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        existing_layer(doc, p.layer)?;
        let to = moved_to(doc, p.layer, p.direction).ok_or_else(|| {
            CommandError::Failed(match p.direction {
                Direction::Up => "it is already at the top of the list".into(),
                Direction::Down => "it is already at the bottom of the list".into(),
            })
        })?;
        let mut c = Change::builder(doc);
        c.move_layer(p.layer, to);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{run_on, two_layers};

    cut_craft_core::contract_tests!(ReorderLayer, two_layers);

    #[test]
    fn a_layer_moves_up_and_back_down_and_not_past_the_ends() {
        let mut doc = two_layers();
        let order = |d: &Document| d.layers.iter().map(|l| l.id).collect::<Vec<_>>();
        let before = order(&doc);
        let first = before[0];
        run_on::<ReorderLayer>(
            &mut doc,
            &Params {
                layer: first,
                direction: Direction::Up,
            },
        );
        assert_eq!(order(&doc), [before[1], before[0]]);
        assert!(
            ReorderLayer::run(
                &doc,
                &Params {
                    layer: first,
                    direction: Direction::Up
                }
            )
            .is_err(),
            "already at the top"
        );
        // undo puts it back
        let change = ReorderLayer::run(
            &doc,
            &Params {
                layer: first,
                direction: Direction::Down,
            },
        )
        .unwrap();
        doc.apply(&change).unwrap();
        assert_eq!(order(&doc), before);
        doc.apply(&change.inverse()).unwrap();
        assert_eq!(order(&doc), [before[1], before[0]]);
    }
}
