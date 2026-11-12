//! Toolpath groups (the toolpath list's folders): toolpaths put in a group, a group renamed,
//! and a group taken apart, its toolpaths kept or deleted with it.

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::toolpath::Toolpath;
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_named, existing};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// `toolpath` in `group` (`None`: in no group).
fn in_group(toolpath: &Toolpath, group: Option<&str>) -> Toolpath {
    Toolpath {
        group: group.map(str::to_owned),
        ..toolpath.clone()
    }
}

/// `group` must be one of the document's toolpath groups.
fn ensure_group(doc: &Document, group: &str) -> Result<(), CommandError> {
    if doc.toolpath_groups_in_use().iter().any(|g| g == group) {
        Ok(())
    } else {
        Err(CommandError::Failed(format!(
            "no toolpath group is called {group}. Pick a group from the list"
        )))
    }
}

/// Puts toolpaths in a group, made if it is new: no toolpaths makes an empty group, and an
/// empty name takes the toolpaths out of their groups.
pub struct GroupToolpaths;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct GroupParams {
    /// The toolpaths to put in the group; none makes an empty group
    pub toolpaths: Vec<EntityId>,
    /// The group's name; empty for no group
    pub group: String,
}

impl Default for GroupParams {
    fn default() -> Self {
        Self {
            toolpaths: vec![EntityId(1)],
            group: "Group 1".into(),
        }
    }
}

impl Command for GroupToolpaths {
    const ID: &'static str = "group_toolpaths";
    type Params = GroupParams;

    fn meta() -> Meta {
        Meta {
            label: "Group Toolpaths",
            group: "Toolpaths",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &GroupParams) -> Result<Change, CommandError> {
        let group = p.group.trim();
        let group = (!group.is_empty())
            .then(|| ensure_named(group))
            .transpose()?;
        let mut c = Change::builder(doc);
        if let Some(name) = &group {
            let mut groups = doc.toolpath_groups_in_use();
            if !groups.contains(name) {
                groups.push(name.clone());
            }
            c.set_toolpath_groups(groups);
        }
        for &id in &p.toolpaths {
            let toolpath = existing(&doc.toolpaths, id, "toolpath")?;
            if toolpath.group != group {
                c.replace_toolpath(id, in_group(toolpath, group.as_deref()));
            }
        }
        Ok(c.finish())
    }
}

/// Renames a toolpath group; its toolpaths follow.
pub struct RenameToolpathGroup;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct RenameParams {
    /// The group as it is called now
    #[schemars(skip)]
    pub group: String,
    /// Its new name
    pub name: String,
}

impl Default for RenameParams {
    fn default() -> Self {
        Self {
            group: "Group 1".into(),
            name: "Group 2".into(),
        }
    }
}

impl Command for RenameToolpathGroup {
    const ID: &'static str = "rename_toolpath_group";
    type Params = RenameParams;

    fn meta() -> Meta {
        Meta {
            label: "Rename Toolpath Group",
            group: "Toolpaths",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &RenameParams) -> Result<Change, CommandError> {
        ensure_group(doc, &p.group)?;
        let name = ensure_named(&p.name)?;
        let groups = doc.toolpath_groups_in_use();
        if name != p.group && groups.contains(&name) {
            return Err(CommandError::Failed(format!(
                "a toolpath group is already called {name}. Type another name"
            )));
        }
        let mut c = Change::builder(doc);
        c.set_toolpath_groups(
            groups
                .into_iter()
                .map(|g| if g == p.group { name.clone() } else { g })
                .collect(),
        );
        for id in doc.toolpaths_in_group(&p.group) {
            c.replace_toolpath(id, in_group(&doc.toolpaths[&id], Some(&name)));
        }
        Ok(c.finish())
    }
}

/// Takes a toolpath group apart: its toolpaths stay, in no group, or go with it.
pub struct UngroupToolpaths;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct UngroupParams {
    /// The toolpath group to take apart
    pub group: String,
    /// Delete the group's toolpaths with it, not only the group
    pub delete_toolpaths: bool,
}

impl Default for UngroupParams {
    fn default() -> Self {
        Self {
            group: "Group 1".into(),
            delete_toolpaths: false,
        }
    }
}

impl Command for UngroupToolpaths {
    const ID: &'static str = "ungroup_toolpaths";
    type Params = UngroupParams;

    fn meta() -> Meta {
        Meta {
            label: "Ungroup Toolpaths",
            group: "Toolpaths",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &UngroupParams) -> Result<Change, CommandError> {
        ensure_group(doc, &p.group)?;
        let mut c = Change::builder(doc);
        c.set_toolpath_groups(
            doc.toolpath_groups_in_use()
                .into_iter()
                .filter(|g| *g != p.group)
                .collect(),
        );
        for id in doc.toolpaths_in_group(&p.group) {
            if p.delete_toolpaths {
                c.remove_toolpath(id);
            } else {
                c.replace_toolpath(id, in_group(&doc.toolpaths[&id], None));
            }
        }
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{after_err, run_on, three_toolpaths};

    /// Three toolpaths, the first two in Group 1.
    fn grouped() -> Document {
        let mut doc = three_toolpaths();
        let p = GroupParams {
            toolpaths: vec![EntityId(1), EntityId(2)],
            group: "Group 1".into(),
        };
        run_on::<GroupToolpaths>(&mut doc, &p);
        doc
    }

    fn groups_of(doc: &Document) -> Vec<Option<&str>> {
        doc.toolpaths.values().map(|t| t.group.as_deref()).collect()
    }

    mod group {
        use super::*;
        cut_craft_core::contract_tests!(GroupToolpaths, three_toolpaths);
    }

    mod rename {
        use super::*;
        cut_craft_core::contract_tests!(RenameToolpathGroup, grouped);
    }

    mod ungroup {
        use super::*;
        cut_craft_core::contract_tests!(UngroupToolpaths, grouped);
    }

    #[test]
    fn toolpaths_go_into_a_new_group_an_empty_group_is_made_and_a_blank_name_ungroups_one() {
        let mut doc = grouped();
        assert_eq!(doc.toolpath_groups, ["Group 1"]);
        assert_eq!(groups_of(&doc), [Some("Group 1"), Some("Group 1"), None]);
        assert_eq!(doc.next_toolpath_group_name(), "Group 2");
        let empty = GroupParams {
            toolpaths: vec![],
            group: doc.next_toolpath_group_name(),
        };
        run_on::<GroupToolpaths>(&mut doc, &empty);
        assert_eq!(doc.toolpath_groups, ["Group 1", "Group 2"]);
        let out = GroupParams {
            toolpaths: vec![EntityId(2)],
            group: "  ".into(),
        };
        run_on::<GroupToolpaths>(&mut doc, &out);
        assert_eq!(groups_of(&doc), [Some("Group 1"), None, None]);
        assert_eq!(
            doc.toolpath_groups,
            ["Group 1", "Group 2"],
            "the groups stay"
        );
        let missing = GroupParams {
            toolpaths: vec![EntityId(9)],
            group: "Group 3".into(),
        };
        assert!(after_err::<GroupToolpaths>(&missing).contains("toolpath"));
    }

    #[test]
    fn a_group_is_renamed_with_its_toolpaths_but_not_onto_another_groups_name() {
        let mut doc = grouped();
        run_on::<GroupToolpaths>(
            &mut doc,
            &GroupParams {
                toolpaths: vec![EntityId(3)],
                group: "Drills".into(),
            },
        );
        let p = RenameParams {
            group: "Group 1".into(),
            name: " Roughing ".into(),
        };
        run_on::<RenameToolpathGroup>(&mut doc, &p);
        assert_eq!(doc.toolpath_groups, ["Roughing", "Drills"]);
        assert_eq!(
            groups_of(&doc),
            [Some("Roughing"), Some("Roughing"), Some("Drills")]
        );
        let taken = RenameParams {
            group: "Roughing".into(),
            name: "Drills".into(),
        };
        let err = RenameToolpathGroup::run(&doc, &taken)
            .unwrap_err()
            .to_string();
        assert!(err.contains("already called Drills"), "{err}");
        let unknown = RenameParams {
            group: "Group 9".into(),
            name: "Any".into(),
        };
        let err = RenameToolpathGroup::run(&doc, &unknown)
            .unwrap_err()
            .to_string();
        assert!(err.contains("No toolpath group is called Group 9"), "{err}");
    }

    #[test]
    fn ungrouping_keeps_the_toolpaths_or_deletes_them_with_the_group() {
        let mut keep = grouped();
        run_on::<UngroupToolpaths>(&mut keep, &UngroupParams::default());
        assert_eq!(keep.toolpath_groups, Vec::<String>::new());
        assert_eq!(groups_of(&keep), [None, None, None]);
        let mut gone = grouped();
        let p = UngroupParams {
            group: "Group 1".into(),
            delete_toolpaths: true,
        };
        run_on::<UngroupToolpaths>(&mut gone, &p);
        assert_eq!(gone.toolpath_groups, Vec::<String>::new());
        assert_eq!(
            gone.toolpaths.keys().copied().collect::<Vec<_>>(),
            [EntityId(3)]
        );
    }
}
