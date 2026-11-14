//! The toolpath list's order: the toolpaths in the order they cut, each group's together
//! under its heading, and the new orders that moving a toolpath or a group, grouping
//! toolpaths or dropping one somewhere else ask for. Toolpaths cut in id order; the list
//! shows them in that order, so what is seen is what is cut.

use cut_craft_core::document::{Document, EntityId};

/// A toolpath and its group, in cut order.
pub type Listed = (EntityId, Option<String>);

/// The toolpaths of `doc` in cut order with their groups.
pub fn listed(doc: &Document) -> Vec<Listed> {
    doc.toolpaths
        .iter()
        .map(|(&id, t)| (id, t.group.clone()))
        .collect()
}

/// What moves as one in the list: a group's toolpaths, or one in no group.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unit {
    Group(String, Vec<EntityId>),
    Single(EntityId),
}

impl Unit {
    fn ids(&self) -> &[EntityId] {
        match self {
            Self::Group(_, ids) => ids,
            Self::Single(id) => std::slice::from_ref(id),
        }
    }
}

/// `listed` as units in cut order: a group where its first toolpath comes, holding all of
/// its toolpaths (a group spread out by an older job comes together there).
pub fn units(listed: &[Listed]) -> Vec<Unit> {
    let mut out: Vec<Unit> = Vec::new();
    for (id, group) in listed {
        match group {
            None => out.push(Unit::Single(*id)),
            Some(name) => {
                let at = out
                    .iter_mut()
                    .find(|u| matches!(u, Unit::Group(g, _) if g == name));
                match at {
                    Some(Unit::Group(_, ids)) => ids.push(*id),
                    _ => out.push(Unit::Group(name.clone(), vec![*id])),
                }
            }
        }
    }
    out
}

/// The cut order `units` make.
pub fn order(units: &[Unit]) -> Vec<EntityId> {
    units.iter().flat_map(|u| u.ids().iter().copied()).collect()
}

/// A row of the toolpath list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Row {
    /// A group's heading.
    Group(String),
    /// A toolpath, and whether it sits in a group.
    Toolpath(EntityId, bool),
}

/// The list's rows: each unit in cut order, a group's heading over its toolpaths, then the
/// groups with no toolpaths yet (`groups`, as made).
pub fn rows(listed: &[Listed], groups: &[String]) -> Vec<Row> {
    let units = units(listed);
    let mut rows = Vec::new();
    for unit in &units {
        match unit {
            Unit::Group(name, ids) => {
                rows.push(Row::Group(name.clone()));
                rows.extend(ids.iter().map(|&id| Row::Toolpath(id, true)));
            }
            Unit::Single(id) => rows.push(Row::Toolpath(*id, false)),
        }
    }
    for name in groups {
        let used = units
            .iter()
            .any(|u| matches!(u, Unit::Group(g, _) if g == name));
        if !used {
            rows.push(Row::Group(name.clone()));
        }
    }
    rows
}

/// The order after toolpath `id` moves one place up (`up`) or down: inside its group it
/// trades places with the next of the group; in no group it goes past the next unit, a whole
/// group at once. None at the end of the list or of its group.
pub fn moved_toolpath(listed: &[Listed], id: EntityId, up: bool) -> Option<Vec<EntityId>> {
    let mut units = units(listed);
    let at = units.iter().position(|u| u.ids().contains(&id))?;
    if let Unit::Group(_, ids) = &mut units[at] {
        let i = ids.iter().position(|&m| m == id)?;
        let j = if up { i.checked_sub(1)? } else { i + 1 };
        if j >= ids.len() {
            return None;
        }
        ids.swap(i, j);
    } else {
        let to = if up { at.checked_sub(1)? } else { at + 1 };
        if to >= units.len() {
            return None;
        }
        units.swap(at, to);
    }
    Some(order(&units))
}

/// The order after group `name` moves past the unit before it (`up`) or after it, all its
/// toolpaths together. None at an end of the list, or for a group with no toolpaths.
pub fn moved_group(listed: &[Listed], name: &str, up: bool) -> Option<Vec<EntityId>> {
    let mut units = units(listed);
    let at = units
        .iter()
        .position(|u| matches!(u, Unit::Group(g, _) if g == name))?;
    let to = if up { at.checked_sub(1)? } else { at + 1 };
    if to >= units.len() {
        return None;
    }
    units.swap(at, to);
    Some(order(&units))
}

/// Where toolpaths dragged in the list are dropped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// Just before this toolpath, joining its group (or leaving theirs when it is in none).
    Before(EntityId),
    /// At the end of this group.
    Into(String),
    /// At the end of the list, in no group.
    End,
}

/// The order and the groups after `moving` is dropped on `target`: those toolpaths, in the
/// order they cut, go there together and take its group. None when `target` is one of them.
pub fn dropped(
    listed: &[Listed],
    moving: &[EntityId],
    target: &Target,
) -> Option<(Vec<EntityId>, Option<String>)> {
    if matches!(target, Target::Before(id) if moving.contains(id)) {
        return None;
    }
    let group = match target {
        Target::Before(id) => listed.iter().find(|(t, _)| t == id)?.1.clone(),
        Target::Into(name) => Some(name.clone()),
        Target::End => None,
    };
    // the toolpaths as they will be, the moved ones in their new group
    let regrouped: Vec<Listed> = listed
        .iter()
        .map(|(id, g)| {
            let g = if moving.contains(id) {
                group.clone()
            } else {
                g.clone()
            };
            (*id, g)
        })
        .collect();
    let mut rest: Vec<EntityId> = order(&units(&regrouped))
        .into_iter()
        .filter(|id| !moving.contains(id))
        .collect();
    let moved: Vec<EntityId> = listed
        .iter()
        .map(|(id, _)| *id)
        .filter(|id| moving.contains(id))
        .collect();
    let at = match target {
        Target::Before(id) => rest.iter().position(|t| t == id)?,
        Target::Into(name) => {
            let last = rest.iter().rposition(|t| {
                regrouped
                    .iter()
                    .any(|(r, g)| r == t && g.as_ref() == Some(name))
            });
            // an empty group: its toolpaths go at the end of the list
            last.map_or(rest.len(), |i| i + 1)
        }
        Target::End => rest.len(),
    };
    rest.splice(at..at, moved);
    Some((rest, group))
}

/// The order after `ids` go in group `name`: the group comes together where its first
/// toolpath cuts, so a new group stands where the first of `ids` was.
pub fn grouped(listed: &[Listed], ids: &[EntityId], name: &str) -> Vec<EntityId> {
    let regrouped: Vec<Listed> = listed
        .iter()
        .map(|(id, g)| {
            let g = if ids.contains(id) {
                Some(name.to_owned())
            } else {
                g.clone()
            };
            (*id, g)
        })
        .collect();
    order(&units(&regrouped))
}

/// The order after `ids` leave their groups: each just after the group it was in, so it
/// still cuts where it did, now on its own.
pub fn ungrouped(listed: &[Listed], ids: &[EntityId]) -> Vec<EntityId> {
    let mut out = Vec::new();
    for unit in units(listed) {
        let (kept, out_of): (Vec<EntityId>, Vec<EntityId>) =
            unit.ids().iter().partition(|id| !ids.contains(id));
        out.extend(kept);
        out.extend(out_of);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(n: &[u64]) -> Vec<EntityId> {
        n.iter().copied().map(EntityId).collect()
    }

    /// 1 and 4 in "Outside", 2 and 3 in no group, 5 in "Holes".
    fn job() -> Vec<Listed> {
        vec![
            (EntityId(1), Some("Outside".into())),
            (EntityId(2), None),
            (EntityId(3), None),
            (EntityId(4), Some("Outside".into())),
            (EntityId(5), Some("Holes".into())),
        ]
    }

    #[test]
    fn a_group_shows_where_its_first_toolpath_cuts_holding_all_of_its_toolpaths() {
        let rows = rows(&job(), &["Outside".into(), "Spare".into()]);
        assert_eq!(
            rows,
            [
                Row::Group("Outside".into()),
                Row::Toolpath(EntityId(1), true),
                Row::Toolpath(EntityId(4), true),
                Row::Toolpath(EntityId(2), false),
                Row::Toolpath(EntityId(3), false),
                Row::Group("Holes".into()),
                Row::Toolpath(EntityId(5), true),
                Row::Group("Spare".into()),
            ]
        );
    }

    #[test]
    fn a_toolpath_in_no_group_steps_past_a_whole_group() {
        assert_eq!(
            moved_toolpath(&job(), EntityId(2), true),
            Some(ids(&[2, 1, 4, 3, 5]))
        );
        assert_eq!(
            moved_toolpath(&job(), EntityId(3), false),
            Some(ids(&[1, 4, 2, 5, 3]))
        );
        assert_eq!(moved_toolpath(&job(), EntityId(5), false), None, "last");
    }

    #[test]
    fn a_toolpath_in_a_group_moves_inside_it_only() {
        assert_eq!(
            moved_toolpath(&job(), EntityId(4), true),
            Some(ids(&[4, 1, 2, 3, 5]))
        );
        assert_eq!(
            moved_toolpath(&job(), EntityId(4), false),
            None,
            "end of its group"
        );
        assert_eq!(moved_toolpath(&job(), EntityId(1), true), None);
    }

    #[test]
    fn a_group_moves_with_all_its_toolpaths() {
        assert_eq!(
            moved_group(&job(), "Holes", true),
            Some(ids(&[1, 4, 2, 5, 3]))
        );
        assert_eq!(
            moved_group(&job(), "Outside", false),
            Some(ids(&[2, 1, 4, 3, 5]))
        );
        assert_eq!(moved_group(&job(), "Outside", true), None, "first");
        assert_eq!(moved_group(&job(), "Spare", true), None, "no toolpaths");
    }

    #[test]
    fn dropping_on_a_toolpath_puts_it_before_and_in_that_group() {
        let (order, group) = dropped(&job(), &ids(&[3]), &Target::Before(EntityId(4))).unwrap();
        assert_eq!(order, ids(&[1, 3, 4, 2, 5]));
        assert_eq!(group.as_deref(), Some("Outside"));
        let (order, group) = dropped(&job(), &ids(&[5]), &Target::Before(EntityId(2))).unwrap();
        assert_eq!(order, ids(&[1, 4, 5, 2, 3]));
        assert_eq!(group, None, "out of its group");
        assert!(dropped(&job(), &ids(&[2]), &Target::Before(EntityId(2))).is_none());
    }

    #[test]
    fn grouping_puts_the_toolpaths_together_at_the_end_of_the_group() {
        let (order, group) =
            dropped(&job(), &ids(&[2, 5]), &Target::Into("Outside".into())).unwrap();
        assert_eq!(order, ids(&[1, 4, 2, 5, 3]));
        assert_eq!(group.as_deref(), Some("Outside"));
        // a new group stands where the first of its toolpaths was
        assert_eq!(grouped(&job(), &ids(&[3, 5]), "New"), ids(&[1, 4, 2, 3, 5]));
        assert_eq!(grouped(&job(), &ids(&[2, 1]), "New"), ids(&[1, 2, 3, 4, 5]));
        let (order, group) = dropped(&job(), &ids(&[1]), &Target::End).unwrap();
        assert_eq!(order, ids(&[2, 3, 4, 5, 1]));
        assert_eq!(group, None);
    }

    #[test]
    fn ungrouping_leaves_a_toolpath_just_after_its_old_group() {
        assert_eq!(ungrouped(&job(), &ids(&[1])), ids(&[4, 1, 2, 3, 5]));
        assert_eq!(
            ungrouped(&job(), &ids(&[2])),
            ids(&[1, 4, 2, 3, 5]),
            "in none already"
        );
    }
}
