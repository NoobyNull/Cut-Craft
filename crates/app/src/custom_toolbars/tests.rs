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
    let menu = item_menu(&toolbars, 0, 0);
    assert!(!menu[0].is_enabled, "the first cannot go earlier");
    assert!(act(&mut toolbars, &menu[1].action), "{}", menu[1].action);
    assert_eq!(items(&toolbars[0]), ["tool:b", "tool:a", "group:C"]);
    // a drag drops it anywhere along, the far end too
    assert!(act(&mut toolbars, "toolbar:move:0:0:9"));
    assert_eq!(items(&toolbars[0]), ["tool:a", "group:C", "tool:b"]);
    let menu = item_menu(&toolbars, 0, 2);
    assert!(!menu[1].is_enabled, "the last cannot go later");
    assert_eq!(menu[2].name, "Remove from Toolbar");
    assert!(act(&mut toolbars, &menu[2].action));
    assert_eq!(items(&toolbars[0]), ["tool:a", "group:C"]);
}

#[test]
fn a_toolbar_is_renamed_floated_and_deleted() {
    let mut toolbars = vec![];
    act(&mut toolbars, "toolbar:new:tool:a");
    assert!(act(&mut toolbars, "toolbar:rename:0:Signs: big ones"));
    assert_eq!(toolbars[0].name, "Signs: big ones");
    assert!(
        !act(&mut toolbars, "toolbar:rename:0:   "),
        "a name is needed"
    );
    let menu = toolbar_menu(&toolbars, 0);
    assert_eq!(menu[0].action, format!("{RENAME}0"));
    assert!(!menu[1].is_checked);
    assert!(act(&mut toolbars, &menu[1].action));
    assert!(toolbars[0].floating);
    assert!(
        toolbar_menu(&toolbars, 0)[1].is_checked,
        "ticked while floating"
    );
    let back = toolbar_menu(&toolbars, 0)[1].action.clone();
    assert!(act(&mut toolbars, &back));
    assert!(!toolbars[0].floating, "and back");
    assert!(act(&mut toolbars, &menu[2].action));
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
        "float:3:true",
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
    let toolbars = vec![Toolbar {
        name: "Signs".into(),
        items: vec![
            Item::Tool("draw_text".into()),
            Item::Group("Create Vectors".into()),
        ],
        floating: true,
        at: Some([10.0, 20.0]),
    }];
    let json = serde_json::to_string(&toolbars).unwrap();
    assert_eq!(
        json,
        r#"[{"name":"Signs","items":[{"tool":"draw_text"},{"group":"Create Vectors"}],"floating":true,"at":[10.0,20.0]}]"#
    );
    let back: Vec<Toolbar> = serde_json::from_str(&json).unwrap();
    assert_eq!(back, toolbars);
}
