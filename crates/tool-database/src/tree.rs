//! The tool tree: groups are `/`-separated paths, named by tools ([`Tool::group`]) and listed
//! with their notes ([`ToolDatabase::groups`]).

use cut_craft_core::toolpath::{Tool, ToolDatabase, ToolGroup};

/// The group a path sits in; `""` at the top.
pub fn parent(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(p, _)| p)
}

/// A path's own name, its last level.
pub fn leaf(path: &str) -> &str {
    path.rsplit_once('/').map_or(path, |(_, l)| l)
}

/// Whether `path` is `group` or inside it.
pub fn within(path: &str, group: &str) -> bool {
    path == group
        || path
            .strip_prefix(group)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// Every group, each after the groups it is in: the listed ones in order, then any a tool
/// names that is not listed.
pub fn groups(db: &ToolDatabase) -> Vec<String> {
    let mut all: Vec<String> = vec![];
    let named = db.tools.iter().map(|t| t.group.as_str());
    for path in db.groups.iter().map(|g| g.path.as_str()).chain(named) {
        let mut at = 0;
        // each level from the top, so a group always follows its parent
        while !path.is_empty() {
            let end = path[at..].find('/').map_or(path.len(), |i| at + i);
            let level = &path[..end];
            if !all.iter().any(|g| g == level) {
                all.push(level.to_owned());
            }
            if end == path.len() {
                break;
            }
            at = end + 1;
        }
    }
    all
}

/// The groups directly inside `group` (`""` for the top), from [`groups`].
pub fn subgroups<'a>(all: &'a [String], group: &str) -> Vec<&'a String> {
    all.iter().filter(|g| parent(g) == group).collect()
}

/// The tools directly inside `group`, by index.
pub fn tools_in(db: &ToolDatabase, group: &str) -> Vec<usize> {
    (0..db.tools.len())
        .filter(|&i| db.tools[i].group == group)
        .collect()
}

/// `name`, else `name 2`, `name 3`... whichever `taken` says is free.
fn unique(name: &str, taken: &dyn Fn(&str) -> bool) -> String {
    let mut n = 1;
    loop {
        let candidate = if n == 1 {
            name.to_owned()
        } else {
            format!("{name} {n}")
        };
        if !taken(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

fn join(group: &str, name: &str) -> String {
    if group.is_empty() {
        name.to_owned()
    } else {
        format!("{group}/{name}")
    }
}

/// A new, empty group inside `group`; its path.
pub fn add_group(db: &mut ToolDatabase, group: &str) -> String {
    let all = groups(db);
    let path = unique(&join(group, "New Group"), &|p| all.iter().any(|g| g == p));
    db.groups.push(ToolGroup {
        path: path.clone(),
        notes: String::new(),
    });
    path
}

/// A new end mill in `group`, with the defaults; its index.
pub fn add_tool(db: &mut ToolDatabase, group: &str) -> usize {
    let name = unique("New Tool", &|n| db.get(n).is_some());
    db.tools.push(Tool {
        name,
        group: group.to_owned(),
        ..Tool::default()
    });
    db.tools.len() - 1
}

/// A copy of tool `i` just after it, its name made unique; the copy's index.
pub fn copy_tool(db: &mut ToolDatabase, i: usize) -> usize {
    let mut tool = db.tools[i].clone();
    tool.name = unique(&format!("{} (copy)", tool.name), &|n| db.get(n).is_some());
    db.tools.insert(i + 1, tool);
    i + 1
}

/// Renames `group` (and everything in it) to `name` at the same level; its new path.
pub fn rename_group(db: &mut ToolDatabase, group: &str, name: &str) -> String {
    let renamed = join(parent(group), name);
    let moved = |path: &mut String| {
        if within(path, group) {
            *path = format!("{renamed}{}", &path[group.len()..]);
        }
    };
    db.groups.iter_mut().for_each(|g| moved(&mut g.path));
    db.tools.iter_mut().for_each(|t| moved(&mut t.group));
    renamed
}

/// Takes `group` out, with every group and tool inside it.
pub fn delete_group(db: &mut ToolDatabase, group: &str) {
    db.groups.retain(|g| !within(&g.path, group));
    db.tools.retain(|t| !within(&t.group, group));
}

/// `group`'s notes, listing the group if only its tools named it.
pub fn notes_mut<'a>(db: &'a mut ToolDatabase, group: &str) -> &'a mut String {
    let at = if let Some(at) = db.groups.iter().position(|g| g.path == group) {
        at
    } else {
        db.groups.push(ToolGroup {
            path: group.to_owned(),
            notes: String::new(),
        });
        db.groups.len() - 1
    };
    &mut db.groups[at].notes
}

/// `group` (with what is inside it) on its own, as a database to export.
pub fn export_group(db: &ToolDatabase, group: &str) -> ToolDatabase {
    ToolDatabase {
        tools: db
            .tools
            .iter()
            .filter(|t| within(&t.group, group))
            .cloned()
            .collect(),
        groups: db
            .groups
            .iter()
            .filter(|g| within(&g.path, group))
            .cloned()
            .collect(),
        materials: db.materials.clone(),
        material: String::new(),
    }
}

/// Adds `incoming` inside a new top-level group named `name` (made unique), so imported tools
/// stay apart from the database's own; tool names already taken get a number. Its materials
/// join the material list.
pub fn import(db: &mut ToolDatabase, incoming: ToolDatabase, name: &str) -> String {
    for m in &incoming.materials {
        db.add_material(m);
    }
    let all = groups(db);
    let top = unique(name, &|p| all.iter().any(|g| g == p));
    db.groups.push(ToolGroup {
        path: top.clone(),
        notes: String::new(),
    });
    for mut g in incoming.groups {
        g.path = join(&top, &g.path);
        db.groups.push(g);
    }
    for mut tool in incoming.tools {
        tool.group = if tool.group.is_empty() {
            top.clone()
        } else {
            join(&top, &tool.group)
        };
        tool.name = unique(&tool.name, &|n| db.get(n).is_some());
        db.tools.push(tool);
    }
    top
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> ToolDatabase {
        ToolDatabase::starter()
    }

    #[test]
    fn groups_follow_their_parents_and_include_ones_only_tools_name() {
        let mut d = db();
        d.tools[0].group = "Extra/Deep/Down".into();
        let all = groups(&d);
        assert_eq!(&all[..2], ["Metric Tools", "Imperial Tools"]);
        let at = |p: &str| all.iter().position(|g| g == p).unwrap();
        assert!(at("Extra") < at("Extra/Deep") && at("Extra/Deep") < at("Extra/Deep/Down"));
        assert!(at("Metric Tools") < at("Metric Tools/End Mills"));
        let top: Vec<&str> = subgroups(&all, "").iter().map(|s| s.as_str()).collect();
        assert_eq!(top, ["Metric Tools", "Imperial Tools", "Extra"]);
        assert_eq!(leaf("Extra/Deep"), "Deep");
        assert!(within("Extra/Deep", "Extra") && !within("Extras", "Extra"));
    }

    #[test]
    fn renaming_a_group_moves_what_is_in_it() {
        let mut d = db();
        let before = tools_in(&d, "Metric Tools/End Mills").len();
        let path = rename_group(&mut d, "Metric Tools", "Millimetre");
        assert_eq!(path, "Millimetre");
        assert_eq!(tools_in(&d, "Millimetre/End Mills").len(), before);
        assert!(groups(&d).iter().all(|g| !g.starts_with("Metric Tools")));
    }

    #[test]
    fn deleting_a_group_deletes_what_is_in_it() {
        let mut d = db();
        let imperial = d
            .tools
            .iter()
            .filter(|t| within(&t.group, "Imperial Tools"))
            .count();
        let total = d.tools.len();
        delete_group(&mut d, "Imperial Tools");
        assert_eq!(d.tools.len(), total - imperial);
        assert!(!groups(&d).iter().any(|g| within(g, "Imperial Tools")));
    }

    #[test]
    fn new_and_copied_items_get_unused_names() {
        let mut d = ToolDatabase::default();
        assert_eq!(add_group(&mut d, ""), "New Group");
        assert_eq!(add_group(&mut d, ""), "New Group 2");
        assert_eq!(add_group(&mut d, "New Group"), "New Group/New Group");
        let a = add_tool(&mut d, "New Group");
        let b = add_tool(&mut d, "New Group");
        assert_eq!(
            (d.tools[a].name.as_str(), d.tools[b].name.as_str()),
            ("New Tool", "New Tool 2")
        );
        let c = copy_tool(&mut d, a);
        assert_eq!((c, d.tools[c].name.as_str()), (1, "New Tool (copy)"));
        assert_eq!(d.tools[c].group, "New Group");
        notes_mut(&mut d, "New Group").push_str("hello");
        assert_eq!(export_group(&d, "New Group").groups[0].notes, "hello");
        assert_eq!(export_group(&d, "New Group").tools.len(), 3);
    }

    #[test]
    fn imports_go_in_a_group_of_their_own() {
        let mut d = db();
        let incoming = export_group(&db(), "Metric Tools");
        let n = incoming.tools.len();
        let top = import(&mut d, incoming.clone(), "Vendor");
        assert_eq!(top, "Vendor");
        assert_eq!(
            tools_in(&d, "Vendor/Metric Tools/End Mills").len(),
            tools_in(&db(), "Metric Tools/End Mills").len()
        );
        assert_eq!(import(&mut d, incoming, "Vendor"), "Vendor 2");
        let names: std::collections::HashSet<_> = d.tools.iter().map(|t| &t.name).collect();
        assert_eq!(names.len(), d.tools.len(), "tool names stay unique");
        assert_eq!(d.tools.len(), db().tools.len() + 2 * n);
    }

    #[test]
    fn imported_materials_join_the_list_once() {
        let mut d = db();
        d.add_material("Oak");
        let mut incoming = export_group(&db(), "Metric Tools");
        incoming.add_material("oak");
        incoming.add_material("MDF");
        import(&mut d, incoming, "Vendor");
        assert_eq!(d.materials, ["Oak", "MDF"]);
    }
}
