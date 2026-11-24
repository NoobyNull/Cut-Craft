//! The toolpath list's order: the toolpaths in the order they cut, each group's together
//! under its heading (groups can hold groups), and the new orders that moving a toolpath or
//! a group, grouping toolpaths or dropping one somewhere else ask for. Toolpaths cut in id
//! order; the list shows them in that order, so what is seen is what is cut. A group, with
//! everything inside it, always cuts as one block.

use cut_craft_core::document::{Document, EntityId};
use std::collections::{BTreeMap, BTreeSet};

/// A toolpath and its group, in cut order.
pub type Listed = (EntityId, Option<String>);

/// What the list is made from: the toolpaths in cut order with their groups, the groups as
/// made, and the group each sub-group sits in.
#[derive(Clone, Debug, Default)]
pub struct Listing {
    pub toolpaths: Vec<Listed>,
    pub groups: Vec<String>,
    pub parents: BTreeMap<String, String>,
}

/// A row's thing: a group or a toolpath.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Item {
    Group(String),
    Toolpath(EntityId),
}

/// A part of the list: a group holding toolpaths and groups, or a toolpath.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node {
    Group(String, Vec<Node>),
    Toolpath(EntityId),
}

impl Node {
    fn is(&self, item: &Item) -> bool {
        match (self, item) {
            (Self::Group(a, _), Item::Group(b)) => a == b,
            (Self::Toolpath(a), Item::Toolpath(b)) => a == b,
            _ => false,
        }
    }

    fn toolpaths(&self, out: &mut Vec<EntityId>) {
        match self {
            Self::Group(_, inside) => inside.iter().for_each(|n| n.toolpaths(out)),
            Self::Toolpath(id) => out.push(*id),
        }
    }
}

/// A row of the toolpath list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub item: Item,
    /// How many groups it sits inside.
    pub depth: usize,
    /// For each group it sits inside, outermost first: whether that group's guide line runs
    /// on past this row (more of that group comes below).
    pub lines: Vec<bool>,
    /// The last thing in its group (its guide line turns into it and stops).
    pub last: bool,
    /// A group's toolpaths, its sub-groups' too; 0 for a toolpath.
    pub count: usize,
    /// A group holding anything, so it has an arrow to open or close it.
    pub holds: bool,
}

/// Where things dragged in the list are dropped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// Just before this toolpath or group, in the group it is in.
    Before(Item),
    /// At the end of this group.
    Into(String),
    /// At the end of the list, in no group.
    End,
}

/// What is being dragged: the picked toolpaths, or a group with all inside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Moving {
    Toolpaths(Vec<EntityId>),
    Group(String),
}

/// A new arrangement: the cut order, the group each named toolpath goes in (empty: none),
/// and the group each named group goes inside (empty: the top).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Arranged {
    pub order: Vec<EntityId>,
    pub groups: Vec<(EntityId, String)>,
    pub parents: Vec<(String, String)>,
}

/// The children of group `name` in `at`, the group added at the end when it isn't there.
fn child<'a>(at: &'a mut Vec<Node>, name: &str) -> &'a mut Vec<Node> {
    let i = if let Some(i) = at
        .iter()
        .position(|n| matches!(n, Node::Group(g, _) if g == name))
    {
        i
    } else {
        at.push(Node::Group(name.to_owned(), vec![]));
        at.len() - 1
    };
    match &mut at[i] {
        Node::Group(_, inside) => inside,
        Node::Toolpath(_) => unreachable!("found as a group"),
    }
}

/// Where `item` is: the places down to it, one per level.
fn find(nodes: &[Node], item: &Item) -> Option<Vec<usize>> {
    for (i, node) in nodes.iter().enumerate() {
        if node.is(item) {
            return Some(vec![i]);
        }
        if let Node::Group(_, inside) = node
            && let Some(mut path) = find(inside, item)
        {
            path.insert(0, i);
            return Some(path);
        }
    }
    None
}

/// The list the places `path` lead to, and the group holding it (none at the top).
fn level<'a>(nodes: &'a mut Vec<Node>, path: &[usize]) -> (&'a mut Vec<Node>, Option<String>) {
    let mut at = nodes;
    let mut name = None;
    for &i in path {
        match &mut at[i] {
            Node::Group(g, inside) => {
                name = Some(g.clone());
                at = inside;
            }
            Node::Toolpath(_) => unreachable!("a path goes through groups"),
        }
    }
    (at, name)
}

/// `nodes` without the toolpaths in `ids`, which are kept in `taken` as found.
fn take_toolpaths(nodes: &mut Vec<Node>, ids: &[EntityId], taken: &mut Vec<EntityId>) {
    nodes.retain_mut(|n| match n {
        Node::Toolpath(id) if ids.contains(id) => {
            taken.push(*id);
            false
        }
        Node::Toolpath(_) => true,
        Node::Group(_, inside) => {
            take_toolpaths(inside, ids, taken);
            true
        }
    });
}

/// The cut order `nodes` make.
pub fn order(nodes: &[Node]) -> Vec<EntityId> {
    let mut out = Vec::new();
    for node in nodes {
        node.toolpaths(&mut out);
    }
    out
}

impl Listing {
    /// The toolpaths of `doc` in cut order with their groups.
    pub fn of(doc: &Document) -> Self {
        Self {
            toolpaths: doc
                .toolpaths
                .iter()
                .map(|(&id, t)| (id, t.group.clone()))
                .collect(),
            groups: doc.toolpath_groups.clone(),
            parents: doc.toolpath_group_parents.clone(),
        }
    }

    /// The groups as made, then any a toolpath names that the list lacks.
    fn groups_in_use(&self) -> Vec<String> {
        let mut groups = self.groups.clone();
        for name in self.toolpaths.iter().filter_map(|(_, g)| g.as_ref()) {
            if !groups.contains(name) {
                groups.push(name.clone());
            }
        }
        groups
    }

    /// The groups `group` sits in, outermost first, then `group` (a parent that is gone,
    /// or would make a loop, ends the line).
    fn path(&self, group: &str, in_use: &[String]) -> Vec<String> {
        let mut path = vec![group.to_owned()];
        let mut at = group;
        while let Some(parent) = self.parents.get(at) {
            if !in_use.contains(parent) || path.contains(parent) {
                break;
            }
            path.push(parent.clone());
            at = parent;
        }
        path.reverse();
        path
    }

    /// The list as a tree, in cut order: a group where its first toolpath comes, holding
    /// all of its toolpaths and groups (one spread out by an older job comes together
    /// there); groups with no toolpaths yet come last in the group they sit in.
    pub fn tree(&self) -> Vec<Node> {
        let in_use = self.groups_in_use();
        let mut top = Vec::new();
        for (id, group) in &self.toolpaths {
            let mut at = &mut top;
            if let Some(group) = group {
                for name in self.path(group, &in_use) {
                    at = child(at, &name);
                }
            }
            at.push(Node::Toolpath(*id));
        }
        for group in &in_use {
            let mut at = &mut top;
            for name in self.path(group, &in_use) {
                at = child(at, &name);
            }
        }
        top
    }

    /// The list's rows, each group's heading over what it holds; a group in `closed`
    /// shows only its heading.
    pub fn rows(&self, closed: &BTreeSet<String>) -> Vec<Row> {
        /// `through`: for each column but the last of this level's rows, whether a line
        /// from further out runs through them.
        fn add(
            nodes: &[Node],
            depth: usize,
            through: &[bool],
            closed: &BTreeSet<String>,
            rows: &mut Vec<Row>,
        ) {
            for (i, node) in nodes.iter().enumerate() {
                let last = i + 1 == nodes.len();
                let (item, count, holds) = match node {
                    Node::Group(name, inside) => (
                        Item::Group(name.clone()),
                        order(inside).len(),
                        !inside.is_empty(),
                    ),
                    Node::Toolpath(id) => (Item::Toolpath(*id), 0, false),
                };
                let mut lines = through.to_vec();
                if depth > 0 {
                    lines.push(!last);
                }
                rows.push(Row {
                    item,
                    depth,
                    lines,
                    last,
                    count,
                    holds,
                });
                if let Node::Group(name, inside) = node
                    && !closed.contains(name)
                {
                    // this row's own line runs on past its group when more follows it
                    let mut through = through.to_vec();
                    if depth > 0 {
                        through.push(!last);
                    }
                    add(inside, depth + 1, &through, closed, rows);
                }
            }
        }
        let mut rows = Vec::new();
        add(&self.tree(), 0, &[], closed, &mut rows);
        rows
    }

    /// The order after `item` moves one place up (`up`) or down in the group it is in,
    /// past the next toolpath or the next whole group. None at an end of its group, or when
    /// the cut order would not change (past a group with no toolpaths).
    pub fn moved(&self, item: &Item, up: bool) -> Option<Vec<EntityId>> {
        let mut tree = self.tree();
        let mut path = find(&tree, item)?;
        let i = path.pop()?;
        let (siblings, _) = level(&mut tree, &path);
        let j = if up { i.checked_sub(1)? } else { i + 1 };
        if j >= siblings.len() {
            return None;
        }
        siblings.swap(i, j);
        let to = order(&tree);
        (to != order(&self.tree())).then_some(to)
    }

    /// The arrangement after `moving` is dropped on `target`: picked toolpaths go there
    /// together, in the order they cut, and take its group; a group goes there with all it
    /// holds, inside the group there. None when `target` is part of what moves.
    pub fn dropped(&self, moving: &Moving, target: &Target) -> Option<Arranged> {
        let mut tree = self.tree();
        let (nodes, toolpaths) = match moving {
            Moving::Toolpaths(ids) => {
                if matches!(target, Target::Before(Item::Toolpath(id)) if ids.contains(id)) {
                    return None;
                }
                let mut taken = Vec::new();
                take_toolpaths(&mut tree, ids, &mut taken);
                // in cut order
                let in_order: Vec<EntityId> = self
                    .toolpaths
                    .iter()
                    .map(|(id, _)| *id)
                    .filter(|id| taken.contains(id))
                    .collect();
                let nodes = in_order.iter().map(|&id| Node::Toolpath(id)).collect();
                (nodes, in_order)
            }
            Moving::Group(name) => {
                let mut path = find(&tree, &Item::Group(name.clone()))?;
                let i = path.pop()?;
                let node = level(&mut tree, &path).0.remove(i);
                // it can't go inside itself: its own rows are gone from the tree now
                let inside_it = match target {
                    Target::Before(item) => find(&tree, item).is_none(),
                    Target::Into(g) => find(&tree, &Item::Group(g.clone())).is_none(),
                    Target::End => false,
                };
                if inside_it {
                    return None;
                }
                (vec![node], vec![])
            }
        };
        let (path, at) = match target {
            Target::Before(item) => {
                let mut path = find(&tree, item)?;
                let at = path.pop()?;
                (path, at)
            }
            Target::Into(g) => {
                let path = find(&tree, &Item::Group(g.clone()))?;
                let at = level(&mut tree, &path).0.len();
                (path, at)
            }
            Target::End => (vec![], tree.len()),
        };
        let (siblings, group) = level(&mut tree, &path);
        siblings.splice(at..at, nodes);
        let group = group.unwrap_or_default();
        Some(Arranged {
            order: order(&tree),
            groups: toolpaths.iter().map(|&id| (id, group.clone())).collect(),
            parents: match moving {
                Moving::Group(name) => vec![(name.clone(), group)],
                Moving::Toolpaths(_) => vec![],
            },
        })
    }

    /// The arrangement after `ids` go in a new group `name`, made where the first of them
    /// cuts, inside the group that one is in.
    pub fn grouped(&self, ids: &[EntityId], name: &str) -> Option<Arranged> {
        let mut tree = self.tree();
        let first = self
            .toolpaths
            .iter()
            .map(|(id, _)| *id)
            .find(|id| ids.contains(id))?;
        let others: Vec<EntityId> = ids.iter().copied().filter(|&id| id != first).collect();
        let mut taken = Vec::new();
        take_toolpaths(&mut tree, &others, &mut taken);
        let mut path = find(&tree, &Item::Toolpath(first))?;
        let i = path.pop()?;
        let (siblings, parent) = level(&mut tree, &path);
        let inside = self
            .toolpaths
            .iter()
            .filter(|(id, _)| ids.contains(id))
            .map(|(id, _)| Node::Toolpath(*id))
            .collect();
        siblings[i] = Node::Group(name.to_owned(), inside);
        Some(Arranged {
            order: order(&tree),
            groups: ids.iter().map(|&id| (id, name.to_owned())).collect(),
            parents: parent.map_or_else(Vec::new, |p| vec![(name.to_owned(), p)]),
        })
    }

    /// The arrangement after group `group` goes inside a new group `name`, made where it is.
    pub fn group_wrapped(&self, group: &str, name: &str) -> Option<Arranged> {
        let mut tree = self.tree();
        let mut path = find(&tree, &Item::Group(group.to_owned()))?;
        path.pop();
        let (_, parent) = level(&mut tree, &path);
        let mut parents = vec![(group.to_owned(), name.to_owned())];
        if let Some(p) = parent {
            parents.push((name.to_owned(), p));
        }
        Some(Arranged {
            order: order(&tree),
            groups: vec![],
            parents,
        })
    }

    /// The arrangement after `ids` leave their groups one level: each goes just after the
    /// group it was in, into the group that one sits in, so it still cuts where it did.
    pub fn ungrouped(&self, ids: &[EntityId]) -> Arranged {
        let mut tree = self.tree();
        let mut groups = Vec::new();
        // how many already put after each group, to keep them in cut order
        let mut after: BTreeMap<String, usize> = BTreeMap::new();
        for (id, _) in self.toolpaths.iter().filter(|(id, _)| ids.contains(id)) {
            let Some(mut path) = find(&tree, &Item::Toolpath(*id)) else {
                continue;
            };
            let i = path.pop().unwrap_or_default();
            let Some(g) = path.pop() else {
                continue; // in no group
            };
            let mut to_group = path.clone();
            to_group.push(g);
            let (inside, name) = level(&mut tree, &to_group);
            inside.remove(i);
            let name = name.unwrap_or_default();
            let (siblings, up) = level(&mut tree, &path);
            let n = after.entry(name).or_default();
            *n += 1;
            siblings.insert(g + *n, Node::Toolpath(*id));
            groups.push((*id, up.unwrap_or_default()));
        }
        Arranged {
            order: order(&tree),
            groups,
            parents: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(n: &[u64]) -> Vec<EntityId> {
        n.iter().copied().map(EntityId).collect()
    }

    /// 1 and 4 in "Outside", 2 and 3 in no group, 5 in "Holes"; "Spare" is empty.
    fn job() -> Listing {
        Listing {
            toolpaths: vec![
                (EntityId(1), Some("Outside".into())),
                (EntityId(2), None),
                (EntityId(3), None),
                (EntityId(4), Some("Outside".into())),
                (EntityId(5), Some("Holes".into())),
            ],
            groups: vec!["Outside".into(), "Spare".into()],
            parents: BTreeMap::new(),
        }
    }

    /// Panel holds 1, then Tabs (2 and 3), then 4; 5 in no group.
    fn nested() -> Listing {
        Listing {
            toolpaths: vec![
                (EntityId(1), Some("Panel".into())),
                (EntityId(2), Some("Tabs".into())),
                (EntityId(3), Some("Tabs".into())),
                (EntityId(4), Some("Panel".into())),
                (EntityId(5), None),
            ],
            groups: vec!["Panel".into(), "Tabs".into()],
            parents: [("Tabs".to_owned(), "Panel".to_owned())].into(),
        }
    }

    fn items(rows: &[Row]) -> Vec<(usize, Item)> {
        rows.iter().map(|r| (r.depth, r.item.clone())).collect()
    }

    fn g(name: &str) -> Item {
        Item::Group(name.into())
    }

    fn t(id: u64) -> Item {
        Item::Toolpath(EntityId(id))
    }

    #[test]
    fn a_group_shows_where_its_first_toolpath_cuts_holding_all_of_its_toolpaths() {
        let rows = job().rows(&BTreeSet::new());
        assert_eq!(
            items(&rows),
            [
                (0, g("Outside")),
                (1, t(1)),
                (1, t(4)),
                (0, t(2)),
                (0, t(3)),
                (0, g("Holes")),
                (1, t(5)),
                (0, g("Spare")),
            ]
        );
        assert_eq!(rows[0].count, 2);
        assert!(!rows[7].holds, "Spare is empty: no arrow");
    }

    #[test]
    fn groups_inside_groups_show_as_a_tree_with_guide_lines() {
        let rows = nested().rows(&BTreeSet::new());
        assert_eq!(
            items(&rows),
            [
                (0, g("Panel")),
                (1, t(1)),
                (1, g("Tabs")),
                (2, t(2)),
                (2, t(3)),
                (1, t(4)),
                (0, t(5)),
            ]
        );
        assert_eq!(rows[0].count, 4, "Tabs' toolpaths count too");
        assert!(rows[0].lines.is_empty(), "the top has no lines");
        assert_eq!(rows[1].lines, [true], "more of Panel follows 1");
        // in Tabs: Panel's line runs on to 4, Tabs' own ends at 3
        assert_eq!(rows[3].lines, [true, true]);
        assert_eq!(rows[4].lines, [true, false]);
        assert!(rows[4].last && !rows[3].last);
        assert_eq!(rows[5].lines, [false], "4 ends Panel");
        // a closed group shows only its heading
        let closed = nested().rows(&["Tabs".to_owned()].into());
        assert_eq!(closed.len(), 5);
        assert_eq!(closed[2].item, g("Tabs"));
        assert_eq!(closed[3].item, t(4));
        assert_eq!(
            nested().tree(),
            [
                Node::Group(
                    "Panel".into(),
                    vec![
                        Node::Toolpath(EntityId(1)),
                        Node::Group(
                            "Tabs".into(),
                            vec![Node::Toolpath(EntityId(2)), Node::Toolpath(EntityId(3))]
                        ),
                        Node::Toolpath(EntityId(4)),
                    ]
                ),
                Node::Toolpath(EntityId(5)),
            ]
        );
    }

    #[test]
    fn a_toolpath_in_no_group_steps_past_a_whole_group() {
        let job = job();
        assert_eq!(job.moved(&t(2), true), Some(ids(&[2, 1, 4, 3, 5])));
        assert_eq!(job.moved(&t(3), false), Some(ids(&[1, 4, 2, 5, 3])));
        assert_eq!(job.moved(&t(5), false), None, "last in Holes");
    }

    #[test]
    fn a_toolpath_in_a_group_moves_inside_it_only() {
        let job = job();
        assert_eq!(job.moved(&t(4), true), Some(ids(&[4, 1, 2, 3, 5])));
        assert_eq!(job.moved(&t(4), false), None, "end of its group");
        assert_eq!(job.moved(&t(1), true), None);
        // past a whole sub-group
        assert_eq!(nested().moved(&t(1), false), Some(ids(&[2, 3, 1, 4, 5])));
    }

    #[test]
    fn a_group_moves_with_everything_inside_it() {
        let job = job();
        assert_eq!(job.moved(&g("Holes"), true), Some(ids(&[1, 4, 2, 5, 3])));
        assert_eq!(job.moved(&g("Outside"), false), Some(ids(&[2, 1, 4, 3, 5])));
        assert_eq!(job.moved(&g("Outside"), true), None, "first");
        assert_eq!(job.moved(&g("Spare"), true), None, "no toolpaths");
        let nested = nested();
        assert_eq!(
            nested.moved(&g("Panel"), false),
            Some(ids(&[5, 1, 2, 3, 4]))
        );
        assert_eq!(nested.moved(&g("Tabs"), true), Some(ids(&[2, 3, 1, 4, 5])));
    }

    #[test]
    fn dropping_on_a_toolpath_puts_it_before_and_in_that_group() {
        let job = job();
        let one = |id| Moving::Toolpaths(ids(&[id]));
        let a = job.dropped(&one(3), &Target::Before(t(4))).unwrap();
        assert_eq!(a.order, ids(&[1, 3, 4, 2, 5]));
        assert_eq!(a.groups, [(EntityId(3), "Outside".to_owned())]);
        let a = job.dropped(&one(5), &Target::Before(t(2))).unwrap();
        assert_eq!(a.order, ids(&[1, 4, 5, 2, 3]));
        assert_eq!(a.groups, [(EntityId(5), String::new())], "out of its group");
        assert!(job.dropped(&one(2), &Target::Before(t(2))).is_none());
        // into a sub-group
        let a = nested().dropped(&one(5), &Target::Before(t(3))).unwrap();
        assert_eq!(a.order, ids(&[1, 2, 5, 3, 4]));
        assert_eq!(a.groups, [(EntityId(5), "Tabs".to_owned())]);
    }

    #[test]
    fn dropping_on_a_group_name_puts_them_at_its_end() {
        let job = job();
        let a = job
            .dropped(
                &Moving::Toolpaths(ids(&[2, 5])),
                &Target::Into("Outside".into()),
            )
            .unwrap();
        assert_eq!(a.order, ids(&[1, 4, 2, 5, 3]));
        let a = job
            .dropped(&Moving::Toolpaths(ids(&[1])), &Target::End)
            .unwrap();
        assert_eq!(a.order, ids(&[4, 2, 3, 5, 1]));
        assert_eq!(a.groups, [(EntityId(1), String::new())]);
    }

    #[test]
    fn a_dragged_group_goes_with_everything_inside_it_but_never_inside_itself() {
        let nested = nested();
        let tabs = Moving::Group("Tabs".into());
        // out to the top, after Panel
        let a = nested.dropped(&tabs, &Target::Before(t(5))).unwrap();
        assert_eq!(a.order, ids(&[1, 4, 2, 3, 5]));
        assert_eq!(a.parents, [("Tabs".to_owned(), String::new())]);
        assert!(a.groups.is_empty(), "its toolpaths keep their group");
        // a group into another group: a sub-group
        let job = job();
        let a = job
            .dropped(
                &Moving::Group("Holes".into()),
                &Target::Into("Outside".into()),
            )
            .unwrap();
        assert_eq!(a.order, ids(&[1, 4, 5, 2, 3]));
        assert_eq!(a.parents, [("Holes".to_owned(), "Outside".to_owned())]);
        // never inside itself or a group inside it
        let panel = Moving::Group("Panel".into());
        assert!(
            nested
                .dropped(&panel, &Target::Into("Tabs".into()))
                .is_none()
        );
        assert!(nested.dropped(&panel, &Target::Before(t(2))).is_none());
        assert!(
            nested
                .dropped(&panel, &Target::Into("Panel".into()))
                .is_none()
        );
        assert!(nested.dropped(&panel, &Target::End).is_some());
    }

    #[test]
    fn grouping_makes_the_new_group_where_the_first_toolpath_was_in_its_group() {
        let job = job();
        let a = job.grouped(&ids(&[3, 5]), "New").unwrap();
        assert_eq!(a.order, ids(&[1, 4, 2, 3, 5]));
        assert!(a.parents.is_empty(), "at the top");
        let a = job.grouped(&ids(&[2, 1]), "New").unwrap();
        assert_eq!(a.order, ids(&[1, 2, 4, 3, 5]));
        assert_eq!(a.parents, [("New".to_owned(), "Outside".to_owned())]);
        // a group wrapped in a new one
        let a = nested().group_wrapped("Tabs", "Edge").unwrap();
        assert_eq!(a.order, ids(&[1, 2, 3, 4, 5]));
        assert_eq!(
            a.parents,
            [
                ("Tabs".to_owned(), "Edge".to_owned()),
                ("Edge".to_owned(), "Panel".to_owned())
            ]
        );
    }

    #[test]
    fn ungrouping_moves_a_toolpath_up_one_level_just_after_its_old_group() {
        let a = job().ungrouped(&ids(&[1]));
        assert_eq!(a.order, ids(&[4, 1, 2, 3, 5]));
        assert_eq!(a.groups, [(EntityId(1), String::new())]);
        assert_eq!(job().ungrouped(&ids(&[2])).order, ids(&[1, 4, 2, 3, 5]));
        let a = nested().ungrouped(&ids(&[2, 3]));
        assert_eq!(a.order, ids(&[1, 2, 3, 4, 5]));
        assert_eq!(
            a.groups,
            [
                (EntityId(2), "Panel".to_owned()),
                (EntityId(3), "Panel".to_owned())
            ]
        );
    }
}
