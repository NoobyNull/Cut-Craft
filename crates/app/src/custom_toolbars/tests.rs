use super::*;

fn act(toolbars: &mut Vec<Toolbar>, action: &str) -> bool {
    let action = action.strip_prefix(ACTION).expect("a toolbar action");
    apply(toolbars, action)
}

fn items(toolbar: &Toolbar) -> Vec<String> {
    toolbar.items.iter().map(Item::key).collect()
}

#[test]
fn a_new_toolbar_holds_the_tool_or_group_it_was_made_from() {
    let mut toolbars = vec![];
    let circle = Item::Tool("draw_circle".into());
    let menu = add_menu(&toolbars, &circle);
    assert_eq!(menu.name, "Add to Toolbar");
    let new = menu.children.last().unwrap();
    assert_eq!(new.name, "New Toolbar");
    assert!(act(&mut toolbars, &new.action));
    assert_eq!(toolbars[0].name, "My Tools");
    assert_eq!(items(&toolbars[0]), ["tool:draw_circle"]);
    // a group too, on a toolbar of its own with the next free name
    let group = Item::Group("Create Vectors".into());
    let new = add_menu(&toolbars, &group).children.last().unwrap().clone();
    assert!(act(&mut toolbars, &new.action));
    assert_eq!(toolbars[1].name, "My Tools 2");
    assert_eq!(items(&toolbars[1]), ["group:Create Vectors"]);
}

#[test]
fn adding_puts_it_on_the_end_once() {
    let mut toolbars = vec![];
    act(&mut toolbars, "toolbar:new:tool:draw_circle");
    let line = Item::Tool("draw_line".into());
    let menu = add_menu(&toolbars, &line);
    assert_eq!(menu.children[0].name, "My Tools");
    assert!(act(&mut toolbars, &menu.children[0].action));
    assert_eq!(items(&toolbars[0]), ["tool:draw_circle", "tool:draw_line"]);
    // once on, it is ticked and cannot be added again
    let again = add_menu(&toolbars, &line);
    assert!(again.children[0].is_checked && !again.children[0].is_enabled);
    assert!(!act(&mut toolbars, &again.children[0].action));
    assert_eq!(toolbars[0].items.len(), 2);
}

#[test]
fn an_item_moves_along_its_toolbar_and_comes_off() {
    let mut toolbars = vec![];
    act(&mut toolbars, "toolbar:new:tool:a");
    act(&mut toolbars, "toolbar:add:0:tool:b");
    act(&mut toolbars, "toolbar:add:0:group:C");
    let menu = item_menu(&toolbars, 0, 0, Home::Header);
    assert!(!menu[0].is_enabled, "the first cannot go earlier");
    assert!(act(&mut toolbars, &menu[1].action), "{}", menu[1].action);
    assert_eq!(items(&toolbars[0]), ["tool:b", "tool:a", "group:C"]);
    // a drag drops it anywhere along, the far end too
    assert!(act(&mut toolbars, "toolbar:move:0:0:9"));
    assert_eq!(items(&toolbars[0]), ["tool:a", "group:C", "tool:b"]);
    let menu = item_menu(&toolbars, 0, 2, Home::Header);
    assert!(!menu[1].is_enabled, "the last cannot go later");
    assert_eq!(menu[2].name, "Remove from Toolbar");
    assert!(act(&mut toolbars, &menu[2].action));
    assert_eq!(items(&toolbars[0]), ["tool:a", "group:C"]);
}

fn find<'a>(menu: &'a [MenuItem], name: &str) -> &'a MenuItem {
    menu.iter()
        .find(|item| item.name == name)
        .unwrap_or_else(|| panic!("no {name}"))
}

#[test]
fn a_new_toolbar_floats_and_its_menu_snaps_attaches_renames_and_deletes_it() {
    let mut toolbars = vec![];
    act(&mut toolbars, "toolbar:new:tool:a");
    assert!(
        matches!(toolbars[0].place, Place::Floating(_)),
        "starts floating"
    );
    assert!(act(&mut toolbars, "toolbar:rename:0:Signs: big ones"));
    assert_eq!(toolbars[0].name, "Signs: big ones");
    assert!(
        !act(&mut toolbars, "toolbar:rename:0:   "),
        "a name is needed"
    );
    let menu = toolbar_menu(&toolbars, 0, Home::Header);
    assert_eq!(find(&menu, "Rename Toolbar").action, format!("{RENAME}0"));
    assert!(find(&menu, "Float").is_checked);
    assert!(act(&mut toolbars, &find(&menu, "Snap to Left").action));
    assert_eq!(
        toolbars[0].place,
        Place::Snapped {
            side: bars::Side::Left
        }
    );
    let menu = toolbar_menu(&toolbars, 0, Home::Header);
    assert!(act(&mut toolbars, &find(&menu, "Attach to Header").action));
    assert_eq!(toolbars[0].place, Place::Layout);
    let menu = toolbar_menu(&toolbars, 0, Home::Rail);
    assert!(act(&mut toolbars, &find(&menu, "Detach").action));
    assert!(
        matches!(toolbars[0].place, Place::Floating(_)),
        "and out again"
    );
    assert!(act(&mut toolbars, &find(&menu, "Delete Toolbar").action));
    assert_eq!(toolbars.len(), 0);
}

#[test]
fn nonsense_changes_nothing() {
    let mut toolbars = vec![];
    for action in [
        "new:",
        "new:bolt:x",
        "add:0:tool:a",
        "remove:0:0",
        "move:0:0:1",
        "place:3:float",
        "place:0:explode",
        "delete:0",
        "explode:0",
        "",
    ] {
        assert!(!apply(&mut toolbars, action), "{action}");
    }
    assert_eq!(toolbars.len(), 0);
}

#[test]
fn toolbars_save_and_load_as_written() {
    let toolbars = vec![
        Toolbar {
            name: "Signs".into(),
            items: vec![
                Item::Tool("draw_text".into()),
                Item::Group("Create Vectors".into()),
            ],
            place: Place::Snapped {
                side: bars::Side::Right,
            },
        },
        Toolbar {
            name: "Boxes".into(),
            items: vec![],
            place: Place::Layout,
        },
    ];
    let json = serde_json::to_string(&toolbars).unwrap();
    assert_eq!(
        json,
        r#"[{"name":"Signs","items":[{"tool":"draw_text"},{"group":"Create Vectors"}],"place":"snapped","side":"right"},{"name":"Boxes","items":[],"place":"layout"}]"#
    );
    let back: Vec<Toolbar> = serde_json::from_str(&json).unwrap();
    assert_eq!(back, toolbars);
}

#[test]
fn a_floating_toolbar_kept_before_bars_could_snap_still_floats_where_it_was() {
    let old = r#"[{"name":"Signs","items":[],"floating":true,"at":[10.0,20.0]},{"name":"Boxes"}]"#;
    let toolbars: Vec<Toolbar> = serde_json::from_str(old).unwrap();
    assert_eq!(
        toolbars[0].place,
        Place::Floating(bars::Floating {
            offset: Some([10.0, 20.0]),
            ..bars::Floating::default()
        })
    );
    assert_eq!(toolbars[1].place, Place::Layout, "in the layout, as it was");
}

#[test]
fn a_menu_lists_each_toolbar_by_name_until_there_are_too_many() {
    let mut toolbars = vec![];
    let circle = Item::Tool("draw_circle".into());
    let items = add_items(&toolbars, &circle, false);
    assert_eq!(
        items.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(),
        ["Add to New Toolbar"]
    );
    assert!(act(&mut toolbars, &items[0].action));
    act(&mut toolbars, "toolbar:new:tool:draw_line");
    let items = add_items(&toolbars, &circle, false);
    let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Remove from My Tools",
            "Add to My Tools 2",
            "Add to New Toolbar"
        ]
    );
    assert!(act(&mut toolbars, &items[1].action));
    assert!(toolbars[1].items.contains(&circle));
    assert!(act(&mut toolbars, &items[0].action), "taken off");
    assert_eq!(toolbars[0].items, [] as [Item; 0]);
    // folded by the setting, or once there are more than a handful
    let folded = add_items(&toolbars, &circle, true);
    assert_eq!(folded.len(), 1);
    assert_eq!(folded[0].name, "Add to Toolbar");
    for n in 0..LISTED {
        act(&mut toolbars, &format!("toolbar:new:tool:t{n}"));
    }
    assert_eq!(add_items(&toolbars, &circle, false).len(), 1, "folded");
}
