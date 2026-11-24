use super::*;
use crate::gui::Toolbar;
use crate::options::Favourites;
use crate::pages::Page;

fn toolbar() -> Toolbar {
    Toolbar::new()
}

#[test]
fn every_group_belongs_to_one_kind_in_toolbar_order() {
    let toolbar = toolbar();
    let model = ToolModel::new(&toolbar);
    let all: Vec<&str> = model.all_groups().iter().map(|g| g.name).collect();
    let design: Vec<&str> = model.groups(Page::Design).iter().map(|g| g.name).collect();
    let process: Vec<&str> = model
        .groups(Page::Toolpaths)
        .iter()
        .map(|g| g.name)
        .collect();
    // the Create/Edit kind's groups, then the Process kind's, nothing left over or twice
    assert_eq!(all, [design.clone(), process.clone()].concat());
    assert!(design.contains(&"Create Vectors"));
    assert!(
        design.contains(&"Shape Creation"),
        "modeling counts as Create/Edit"
    );
    assert!(process.contains(&"Cut Out and Clear"));
    assert!(design.iter().all(|g| !process.contains(g)));
}

#[test]
fn a_tool_is_of_the_kind_whose_groups_hold_it() {
    let toolbar = toolbar();
    let model = ToolModel::new(&toolbar);
    assert_eq!(model.kind_of("draw_circle"), Page::Design);
    assert_eq!(model.kind_of("pocket_toolpath"), Page::Toolpaths);
    // a gadget or anything not in a group is drawn with the Create/Edit tools
    assert_eq!(model.kind_of("no_such_tool"), Page::Design);
}

#[test]
fn every_sub_tab_names_groups_of_its_own_kind() {
    let toolbar = toolbar();
    let model = ToolModel::new(&toolbar);
    for kind in [Page::Design, Page::Toolpaths] {
        let names: Vec<&str> = model.groups(kind).iter().map(|g| g.name).collect();
        let set = ToolModel::sections(kind);
        for name in set
            .always
            .iter()
            .chain(set.tabs.iter().flat_map(|t| t.groups))
        {
            assert!(names.contains(name), "{name} is not a {kind:?} group");
        }
    }
}

#[test]
fn favourites_come_in_the_order_picked_and_only_while_still_tools_of_that_kind() {
    let toolbar = toolbar();
    let model = ToolModel::new(&toolbar);
    let favourites = Favourites {
        design: vec!["draw_text".into(), "gone_tool".into(), "draw_circle".into()],
        toolpaths: vec!["pocket_toolpath".into(), "draw_circle".into()],
        ..Favourites::default()
    };
    assert_eq!(
        model.favourites(Page::Design, &favourites),
        ["draw_text", "draw_circle"]
    );
    // a Create/Edit tool on the Process list is not one of its tools
    assert_eq!(
        model.favourites(Page::Toolpaths, &favourites),
        ["pocket_toolpath"]
    );
    assert_eq!(
        model.every_favourite(&favourites),
        ["draw_text", "draw_circle", "pocket_toolpath"]
    );
}

#[test]
fn a_group_holding_a_favourite_is_kept_and_uses_add_up() {
    let toolbar = toolbar();
    let model = ToolModel::new(&toolbar);
    let create = model
        .all_groups()
        .into_iter()
        .find(|g| g.name == "Create Vectors")
        .unwrap();
    let mut favourites = Favourites::default();
    assert!(!ToolModel::holds_a_favourite(&create, &favourites));
    favourites.design.push("draw_circle".into());
    assert!(ToolModel::holds_a_favourite(&create, &favourites));

    let uses = [("draw_circle".to_owned(), 3), ("draw_text".to_owned(), 4)]
        .into_iter()
        .collect();
    assert_eq!(ToolModel::uses(&create, &uses), 7);
}

#[test]
fn every_layout_gives_a_tool_the_same_favourites_item() {
    let toolbar = toolbar();
    let model = ToolModel::new(&toolbar);
    let mut favourites = Favourites::default();
    for place in [Place::Grid, Place::Row, Place::Favourites] {
        let menu = model.menu("draw_circle", place, &favourites, false);
        assert_eq!(menu[0].name, "Add to Favourites", "{place:?}");
        assert_eq!(menu[0].action, "favourite:design:add:draw_circle");
    }
    favourites.toolpaths.push("pocket_toolpath".into());
    let menu = model.menu("pocket_toolpath", Place::Row, &favourites, false);
    assert_eq!(menu[0].name, "Remove from Favourites");
    assert_eq!(menu[0].action, "favourite:toolpaths:remove:pocket_toolpath");
}

#[test]
fn what_follows_the_favourites_item_depends_on_where_the_tool_is_shown() {
    let toolbar = toolbar();
    let model = ToolModel::new(&toolbar);
    let favourites = Favourites::default();
    let names = |place, folded| {
        model
            .menu("draw_circle", place, &favourites, folded)
            .iter()
            .map(|m| m.name.clone())
            .collect::<Vec<_>>()
    };
    // a row in a list: nothing to fold, nowhere to move it
    assert_eq!(names(Place::Row, false), ["Add to Favourites"]);
    // among the favourites it moves along them
    assert_eq!(
        names(Place::Favourites, false),
        ["Add to Favourites", "Move Earlier", "Move Later"]
    );
    // a grid of icons can fold its tool groups, ticked while it does
    let grid = model.menu("draw_circle", Place::Grid, &favourites, true);
    assert_eq!(
        grid.last().unwrap().name,
        "Fold Tool Groups into One Button"
    );
    assert!(grid.last().unwrap().is_checked);
}
