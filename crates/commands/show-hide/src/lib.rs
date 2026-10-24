//! Show / Hide: Aspire's right-click visibility family (Show This, Show Only This, Show All But
//! This, Show All, Hide This, Hide All) for layers, toolpaths or components, built from each
//! kind's properties command.

use cut_craft_core::document::{Document, EntityId, LayerId};
use cut_craft_core::params::Items;
use cut_craft_core::{Change, Command, CommandError, Meta, existing, existing_layer};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct ShowHide;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    #[schemars(title = "Show This")]
    ShowThis,
    #[schemars(title = "Show Only This")]
    ShowOnlyThis,
    #[schemars(title = "Show All But This")]
    ShowAllButThis,
    #[default]
    #[schemars(title = "Show All")]
    ShowAll,
    #[schemars(title = "Hide This")]
    HideThis,
    #[schemars(title = "Hide All")]
    HideAll,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub items: Items,
    /// The item right-clicked (a layer, toolpath or component id)
    pub item: u64,
    pub action: Action,
}

impl Command for ShowHide {
    const ID: &'static str = "show_hide";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Show / Hide",
            group: "Edit",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let items = doc.items(p.items);
        let names_one = !matches!(p.action, Action::ShowAll | Action::HideAll);
        if names_one && !items.iter().any(|&(id, _)| id == p.item) {
            return Err(CommandError::InvalidParams(format!(
                "{} {} does not exist",
                p.items.one(),
                p.item
            )));
        }
        let wanted = |id: u64, shown: bool| match p.action {
            Action::ShowThis => shown || id == p.item,
            Action::ShowOnlyThis => id == p.item,
            Action::ShowAllButThis => id != p.item,
            Action::ShowAll => true,
            Action::HideThis => shown && id != p.item,
            Action::HideAll => false,
        };
        let changing: Vec<(u64, bool)> = items
            .into_iter()
            .filter_map(|(id, shown)| {
                let want = wanted(id, shown);
                (want != shown).then_some((id, want))
            })
            .collect();
        // cascade: each item shown or hidden by its kind's properties command
        Change::chain_over(doc, &changing, |d, &(id, visible)| {
            set_visible(d, p.items, id, visible)
        })
    }
}

fn set_visible(
    doc: &Document,
    kind: Items,
    id: u64,
    visible: bool,
) -> Result<Change, CommandError> {
    match kind {
        Items::Layers => {
            let layer = LayerId(u32::try_from(id).unwrap_or(u32::MAX));
            let l = existing_layer(doc, layer)?;
            cmd_layer_properties::LayerProperties::run(
                doc,
                &cmd_layer_properties::Params {
                    layer,
                    name: l.name.clone(),
                    visible,
                    locked: l.locked,
                },
            )
        }
        Items::Toolpaths => {
            let toolpath = EntityId(id);
            let t = existing(&doc.toolpaths, toolpath, "toolpath")?;
            cmd_toolpath_properties::ToolpathProperties::run(
                doc,
                &cmd_toolpath_properties::Params {
                    toolpath,
                    name: t.name.clone(),
                    visible,
                },
            )
        }
        Items::Components => {
            let component = EntityId(id);
            let c = existing(&doc.components, component, "component")?;
            cmd_component_properties::ComponentProperties::run(
                doc,
                &cmd_component_properties::Params {
                    component,
                    name: c.name.clone(),
                    combine: c.combine,
                    visible,
                },
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{after_err, run_on, three_toolpaths, two_components, two_layers};

    cut_craft_core::contract_tests!(ShowHide);

    fn visible(doc: &Document, kind: Items) -> Vec<bool> {
        doc.items(kind).into_iter().map(|(_, v)| v).collect()
    }

    fn act(doc: &mut Document, items: Items, item: u64, action: Action) {
        run_on::<ShowHide>(
            doc,
            &Params {
                items,
                item,
                action,
            },
        );
    }

    #[test]
    fn show_only_this_hides_every_other_layer_and_deselects_on_them() {
        let mut doc = two_layers();
        assert!(!doc.selection.is_empty());
        act(&mut doc, Items::Layers, 2, Action::ShowOnlyThis);
        assert_eq!(visible(&doc, Items::Layers), [false, true]);
        assert!(
            doc.selection.is_empty(),
            "Layer Properties deselects on hidden layers"
        );
    }

    #[test]
    fn hide_this_then_show_all_toolpaths() {
        let mut doc = three_toolpaths();
        act(&mut doc, Items::Toolpaths, 2, Action::HideThis);
        assert_eq!(visible(&doc, Items::Toolpaths), [true, false, true]);
        act(&mut doc, Items::Toolpaths, 3, Action::ShowThis);
        assert_eq!(
            visible(&doc, Items::Toolpaths),
            [true, false, true],
            "already shown"
        );
        act(&mut doc, Items::Toolpaths, 1, Action::ShowAll);
        assert_eq!(visible(&doc, Items::Toolpaths), [true, true, true]);
    }

    #[test]
    fn show_all_but_this_and_hide_all_components() {
        let mut doc = two_components();
        act(&mut doc, Items::Components, 1, Action::ShowAllButThis);
        assert_eq!(visible(&doc, Items::Components), [false, true]);
        act(&mut doc, Items::Components, 1, Action::HideAll);
        assert_eq!(visible(&doc, Items::Components), [false, false]);
    }

    #[test]
    fn a_missing_item_is_reported() {
        let why = after_err::<ShowHide>(&Params {
            items: Items::Layers,
            item: 9,
            action: Action::ShowThis,
        });
        assert!(why.contains("layer 9 does not exist"), "{why}");
    }
}
