//! The toolpath list's groups: which there are, which sits in which (groups inside groups),
//! and the toolpaths each holds.

use super::{Document, EntityId};

impl Document {
    /// The toolpath groups in list order, then any a toolpath names that the list lacks (a
    /// template brings its toolpaths' groups in).
    #[must_use]
    pub fn toolpath_groups_in_use(&self) -> Vec<String> {
        let mut groups = self.toolpath_groups.clone();
        for name in self.toolpaths.values().filter_map(|t| t.group.as_ref()) {
            if !groups.contains(name) {
                groups.push(name.clone());
            }
        }
        groups
    }

    /// The toolpaths in `group` itself (not its sub-groups), in list order.
    #[must_use]
    pub fn toolpaths_in_group(&self, group: &str) -> Vec<EntityId> {
        self.toolpaths
            .iter()
            .filter(|(_, t)| t.group.as_deref() == Some(group))
            .map(|(&id, _)| id)
            .collect()
    }

    /// The toolpaths in `group` and in every group inside it, in list order: what moves,
    /// shows, hides or saves with the group.
    #[must_use]
    pub fn toolpaths_under_group(&self, group: &str) -> Vec<EntityId> {
        let groups = self.toolpath_group_and_subgroups(group);
        self.toolpaths
            .iter()
            .filter(|(_, t)| t.group.as_ref().is_some_and(|g| groups.contains(g)))
            .map(|(&id, _)| id)
            .collect()
    }

    /// The groups `group` sits in, outermost first, then `group` itself. A parent that is
    /// no longer a group, or one that would make a loop, ends the line there.
    #[must_use]
    pub fn toolpath_group_path(&self, group: &str) -> Vec<String> {
        let groups = self.toolpath_groups_in_use();
        let mut path = vec![group.to_owned()];
        let mut at = group;
        while let Some(parent) = self.toolpath_group_parents.get(at) {
            if !groups.contains(parent) || path.contains(parent) {
                break;
            }
            path.push(parent.clone());
            at = parent;
        }
        path.reverse();
        path
    }

    /// The group `group` sits in, if any.
    #[must_use]
    pub fn toolpath_group_parent(&self, group: &str) -> Option<String> {
        let mut path = self.toolpath_group_path(group);
        path.pop();
        path.pop()
    }

    /// `group` and every group inside it, however deep.
    #[must_use]
    pub fn toolpath_group_and_subgroups(&self, group: &str) -> Vec<String> {
        self.toolpath_groups_in_use()
            .into_iter()
            .filter(|g| self.toolpath_group_path(g).iter().any(|p| p == group))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::three_toolpaths;

    /// Toolpath 1 in Outside, 2 in Tabs (inside Outside), 3 in none.
    fn nested() -> Document {
        let mut doc = three_toolpaths();
        doc.toolpath_groups = vec!["Outside".into(), "Tabs".into(), "Spare".into()];
        doc.toolpath_group_parents
            .insert("Tabs".into(), "Outside".into());
        doc.toolpaths.get_mut(&EntityId(1)).unwrap().group = Some("Outside".into());
        doc.toolpaths.get_mut(&EntityId(2)).unwrap().group = Some("Tabs".into());
        doc
    }

    #[test]
    fn a_group_holds_its_sub_groups_toolpaths_too() {
        let doc = nested();
        assert_eq!(doc.toolpaths_in_group("Outside"), [EntityId(1)]);
        assert_eq!(
            doc.toolpaths_under_group("Outside"),
            [EntityId(1), EntityId(2)]
        );
        assert_eq!(doc.toolpath_group_path("Tabs"), ["Outside", "Tabs"]);
        assert_eq!(
            doc.toolpath_group_parent("Tabs").as_deref(),
            Some("Outside")
        );
        assert_eq!(doc.toolpath_group_parent("Outside"), None);
        assert_eq!(
            doc.toolpath_group_and_subgroups("Outside"),
            ["Outside", "Tabs"]
        );
    }

    #[test]
    fn a_missing_parent_or_a_loop_never_hangs_and_ends_the_line() {
        let mut doc = nested();
        doc.toolpath_group_parents
            .insert("Outside".into(), "Tabs".into());
        assert_eq!(doc.toolpath_group_path("Tabs"), ["Outside", "Tabs"]);
        doc.toolpath_group_parents
            .insert("Spare".into(), "Gone".into());
        assert_eq!(doc.toolpath_group_parent("Spare"), None);
    }

    #[test]
    fn a_job_with_no_sub_groups_saves_as_before_and_sub_groups_survive_saving() {
        let flat = serde_json::to_value(three_toolpaths()).unwrap();
        assert!(flat.get("toolpath_group_parents").is_none());
        assert!(flat.get("toolpath_groups_closed").is_none());
        let mut doc = nested();
        doc.toolpath_groups_closed.insert("Tabs".into());
        let back: Document = serde_json::from_value(serde_json::to_value(&doc).unwrap()).unwrap();
        assert_eq!(back.toolpath_group_parents, doc.toolpath_group_parents);
        assert_eq!(back.toolpath_groups_closed, doc.toolpath_groups_closed);
    }
}
