//! Delete Items: Aspire's right-click delete family (This, Visible, Invisible, Empty, All) for
//! layers, toolpaths or components, built from each kind's delete command.

use cut_craft_core::document::{Document, EntityId, LayerId};
use cut_craft_core::params::Items;
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct DeleteItems;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Which {
    #[schemars(title = "Delete This")]
    This,
    #[schemars(title = "Delete Visible")]
    Visible,
    #[schemars(title = "Delete Invisible")]
    Invisible,
    /// Layers with no vectors on them
    #[default]
    #[schemars(title = "Delete Empty")]
    Empty,
    #[schemars(title = "Delete All")]
    All,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub items: Items,
    /// The item right-clicked, for Delete This
    pub item: u64,
    pub which: Which,
}

impl Command for DeleteItems {
    const ID: &'static str = "delete_items";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Delete Items",
            group: "Edit",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let items = doc.items(p.items);
        let pick = |keep: bool| -> Vec<u64> {
            items
                .iter()
                .filter(|&&(_, shown)| shown == keep)
                .map(|&(id, _)| id)
                .collect()
        };
        let targets = match p.which {
            Which::This if items.iter().any(|&(id, _)| id == p.item) => vec![p.item],
            Which::This => {
                return Err(CommandError::InvalidParams(format!(
                    "{} {} does not exist",
                    p.items.one(),
                    p.item
                )));
            }
            Which::Visible => pick(true),
            Which::Invisible => pick(false),
            Which::All => items.iter().map(|&(id, _)| id).collect(),
            Which::Empty if p.items == Items::Layers => doc
                .layers
                .iter()
                .filter(|l| doc.vectors_on(l.id).next().is_none())
                .map(|l| u64::from(l.id.0))
                .collect(),
            Which::Empty => {
                return Err(CommandError::InvalidParams(
                    "only layers can be empty".into(),
                ));
            }
        };
        // cascade: each item deleted by its kind's delete command
        Change::chain_over(doc, &targets, |d, &id| match p.items {
            Items::Layers => cmd_delete_layer::DeleteLayer::run(
                d,
                &cmd_delete_layer::Params {
                    layer: LayerId(u32::try_from(id).unwrap_or(u32::MAX)),
                },
            ),
            Items::Toolpaths => cmd_delete_toolpath::DeleteToolpath::run(
                d,
                &cmd_delete_toolpath::Params {
                    toolpath: EntityId(id),
                },
            ),
            Items::Components => cmd_delete_component::DeleteComponent::run(
                d,
                &cmd_delete_component::Params {
                    component: EntityId(id),
                },
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::document::EntityId;
    use cut_craft_core::testing::{after_err, domed, run_on, two_layers, with_toolpath};

    cut_craft_core::contract_tests!(DeleteItems);

    /// `with_toolpath` plus two more toolpaths: ids 1, 2, 3.
    fn three_toolpaths() -> Document {
        let mut doc = with_toolpath();
        let first = doc.toolpaths[&EntityId(1)].clone();
        let mut c = Change::builder(&doc);
        c.add_toolpath(first.clone());
        c.add_toolpath(first);
        let change = c.finish();
        doc.apply(&change).unwrap();
        doc
    }

    /// `domed` plus a second component: ids 1, 2.
    fn two_components() -> Document {
        let mut doc = domed();
        let first = doc.components[&EntityId(1)].clone();
        let mut c = Change::builder(&doc);
        c.add_component(first);
        let change = c.finish();
        doc.apply(&change).unwrap();
        doc
    }

    fn delete(doc: &mut Document, items: Items, item: u64, which: Which) {
        run_on::<DeleteItems>(doc, &Params { items, item, which });
    }

    fn ids(doc: &Document, kind: Items) -> Vec<u64> {
        doc.items(kind).into_iter().map(|(id, _)| id).collect()
    }

    #[test]
    fn deletes_this_the_hidden_the_shown_or_every_toolpath() {
        let mut doc = three_toolpaths();
        doc.toolpaths.get_mut(&EntityId(2)).unwrap().visible = false;
        let start = doc.clone();
        delete(&mut doc, Items::Toolpaths, 0, Which::Invisible);
        assert_eq!(ids(&doc, Items::Toolpaths), [1, 3]);
        doc = start.clone();
        delete(&mut doc, Items::Toolpaths, 0, Which::Visible);
        assert_eq!(ids(&doc, Items::Toolpaths), [2]);
        doc = start.clone();
        delete(&mut doc, Items::Toolpaths, 3, Which::This);
        assert_eq!(ids(&doc, Items::Toolpaths), [1, 2]);
        doc = start;
        delete(&mut doc, Items::Toolpaths, 0, Which::All);
        assert!(doc.toolpaths.is_empty());
    }

    #[test]
    fn deletes_empty_layers_but_never_the_last_one() {
        let mut doc = two_layers();
        delete(&mut doc, Items::Layers, 0, Which::Empty);
        assert_eq!(ids(&doc, Items::Layers), [1], "layer 2 had nothing on it");
        let why = after_err::<DeleteItems>(&Params {
            items: Items::Layers,
            item: 0,
            which: Which::All,
        });
        assert!(why.contains("last layer"), "{why}");
    }

    #[test]
    fn deletes_a_component() {
        let mut doc = two_components();
        delete(&mut doc, Items::Components, 1, Which::This);
        assert_eq!(ids(&doc, Items::Components), [2]);
    }

    #[test]
    fn only_layers_are_ever_empty() {
        let why = after_err::<DeleteItems>(&Params {
            items: Items::Toolpaths,
            item: 0,
            which: Which::Empty,
        });
        assert!(why.contains("only layers"), "{why}");
    }
}
