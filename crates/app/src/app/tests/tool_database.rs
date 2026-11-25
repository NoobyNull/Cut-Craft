//! The Tool Database: bits, their feeds, and the tools toolpath forms pick from it.

use super::*;

/// Vendor tool databases are not part of the repo: `CUT_CRAFT_VENDOR_FILES` names a
/// folder holding some to try them.
#[test]
fn a_vendor_tool_database_imports_when_one_is_given() {
    let mut h = Harness::new();
    let before = h.app.tools.tools.len();
    h.app
        .import_tools(std::path::Path::new("/nonexistent.vtdb"));
    assert_eq!(h.app.tools.tools.len(), before, "nothing added on failure");
    let Some(dir) = std::env::var_os("CUT_CRAFT_VENDOR_FILES") else {
        return;
    };
    let Some(file) = std::fs::read_dir(dir).ok().and_then(|d| {
        d.flatten()
            .map(|e| e.path())
            .find(|p| p.extension().is_some_and(|x| x == "vtdb"))
    }) else {
        return;
    };
    h.app.import_tools(&file);
    assert!(h.app.tools.tools.len() > before + 10, "{}", file.display());
    let name = file.file_stem().unwrap().to_string_lossy();
    assert!(
        h.app
            .tools
            .tools
            .iter()
            .any(|t| t.group.starts_with(&*name)),
        "in a group named after the file"
    );
}

/// Relief from Photo with Only What the Bit Can Carve ticked: the form draws its note on
/// the job's resolution, and the relief is still one relief made again in place.
#[test]
fn relief_from_photo_carves_only_what_the_bit_can() {
    use cut_craft_core::bitmap::Bitmap;
    use cut_craft_core::geometry::{Point, Rect};
    let mut h = Harness::new();
    let mut c = Change::builder(&h.app.doc);
    // white all over, with one black column: a groove 1 mm wide
    c.add_bitmap(Bitmap {
        name: "map".into(),
        columns: 40,
        rows: 4,
        grey: (0..160)
            .map(|i| if i % 40 == 20 { 0 } else { 255 })
            .collect(),
        rect: Rect::new(Point::new(0.0, 0.0), Point::new(40.0, 4.0)),
        depth: Vec::new(),
        rgb: Vec::new(),
    });
    h.app.record(Ok(c.finish()), "added");
    h.app.invoke(&h.ctx, "relief_from_photo");
    h.frame(vec![]);
    let form = h.app.form.as_mut().unwrap();
    for (key, value) in [
        ("depth_from", "picture".into()),
        ("max_height", 10.0.into()),
        ("lift", 0.0.into()),
        ("background_cut_off", 0.0.into()),
        ("depth_cut_off", 0.0.into()),
        ("fine_detail", 0.0.into()),
        ("smoothness", 0.into()),
    ] {
        let value: serde_json::Value = value;
        form.params[key] = value;
    }
    let params = form.params.clone();
    h.app.apply_form("relief_from_photo", &params).unwrap();
    let lowest = |h: &Harness| {
        let k = h.app.doc.components.values().next().unwrap();
        k.height_range().unwrap().0
    };
    let as_drawn = lowest(&h);
    // ticked before a bit is picked: the form says to Select… one
    let form = h.app.form.as_mut().unwrap();
    form.params["only_what_the_bit_can_carve"] = true.into();
    let params = form.params.clone();
    let refused = h.app.apply_form("relief_from_photo", &params).unwrap_err();
    assert!(refused.contains("Select…"), "{refused}");
    // the bit comes from the Tool Database, as on a toolpath form; no passes or rest there
    h.frame(vec![]);
    assert!(h.text_at("Select…").is_some(), "the form's tool");
    assert!(h.text_at("Edit Passes…").is_none());
    h.app.choose_tool("6 mm Ball Nose");
    h.app.form.as_mut().unwrap().dirty = true;
    h.frame(vec![]);
    assert_eq!(h.app.doc.components.len(), 1, "made again in place");
    assert!(
        lowest(&h) > as_drawn + 2.0,
        "the groove fills in: {as_drawn} -> {}",
        lowest(&h)
    );
}

#[test]
fn toolpath_forms_take_their_tool_from_the_database_and_follow_its_edits() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "pocket_toolpath");
    h.app.choose_tool("3 mm End Mill");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Edit…").is_some());
    assert!(
        h.text_at("Number of Flutes").is_none(),
        "the tool's settings are not edited in the form"
    );
    assert_eq!(h.app.form.as_ref().unwrap().params["tool"]["diameter"], 3.0);
    h.app.edit_tool(&h.ctx, Some("3 mm End Mill".into()));
    let manager = h
        .app
        .tool_manager
        .as_mut()
        .expect("the Tool Database opens");
    let Some(cut_craft_tool_database::Pick::Tool(i)) = manager.pick else {
        panic!("on the form's tool")
    };
    assert_eq!(manager.draft.tools[i].name, "3 mm End Mill");
    manager.draft.tools[i].pass_depth = 1.25;
    let edited = manager.draft.clone();
    h.app.tools = edited;
    h.app.refresh_form_tool();
    assert_eq!(
        h.app.form.as_ref().unwrap().params["tool"]["pass_depth"],
        1.25,
        "saving the database updates the open form"
    );
}

#[test]
fn the_tool_database_window_keeps_edits_only_on_ok() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "tool_database");
    h.frame(vec![]);
    let count = h.app.tools.tools.len();
    h.app.tool_manager.as_mut().unwrap().draft.tools.remove(0);
    h.frame(vec![]);
    assert_eq!(h.app.tools.tools.len(), count, "not until OK");
    let ok = *h.texts_at("OK").last().expect("the window has OK");
    h.click(ok, Modifiers::NONE);
    assert!(h.app.tool_manager.is_none());
    assert_eq!(h.app.tools.tools.len(), count - 1);
    h.app.invoke(&h.ctx, "tool_database");
    h.app.tool_manager.as_mut().unwrap().draft.tools.clear();
    h.frame(vec![]);
    let cancel = *h.texts_at("Cancel").last().expect("and Cancel");
    h.click(cancel, Modifiers::NONE);
    assert_eq!(h.app.tools.tools.len(), count - 1, "Cancel drops the edits");
}

#[test]
fn three_d_finishing_picks_its_tools_in_the_tool_database() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "3d_finishing_toolpath");
    h.app.pages.set_pinned(Page::Toolpaths, true);
    h.frame(vec![]);
    h.frame(vec![]);
    let first = h.app.form.as_ref().unwrap().params["tools"][0]["name"].clone();
    // no drop-down of tools: each has Select… and Edit…, and Add a tool… opens the database
    assert!(h.text_at("Select…").is_some(), "the first tool's Select…");
    let add = h.text_at("Add a tool…").expect("an Add button");
    h.click(add, Modifiers::NONE);
    select_in_tool_database(&mut h, "6 mm Ball Nose");
    let form = h.app.form.as_ref().unwrap();
    assert_eq!(form.params["tools"][0]["name"], first, "kept");
    assert_eq!(
        form.params["tools"][1]["name"], "6 mm Ball Nose",
        "added after it"
    );
    let ball = h.app.tools.get("6 mm Ball Nose").unwrap();
    assert_eq!(
        form.params["tools"][1]["feed_rate"],
        serde_json::json!(ball.feed_rate),
        "its own feeds and speeds come with it"
    );
    // the first tool's Select… puts another in its place
    let select = *h.texts_at("Select…").first().unwrap();
    h.click(select, Modifiers::NONE);
    select_in_tool_database(&mut h, "3 mm End Mill");
    let form = h.app.form.as_ref().unwrap();
    assert_eq!(form.params["tools"][0]["name"], "3 mm End Mill");
    assert_eq!(form.params["tools"][1]["name"], "6 mm Ball Nose");
    assert_eq!(form.params["tools"].as_array().unwrap().len(), 2);
}

#[test]
fn rest_machining_picks_its_earlier_tools_in_the_tool_database() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.invoke(&h.ctx, "profile_toolpath");
    h.app.pages.set_pinned(Page::Toolpaths, true);
    h.app.choose_tool("3 mm End Mill");
    h.frame(vec![]);
    // what Rest Machining's Add a tool… asks for: Select Tool, for its list
    h.app
        .ask_listed(&h.ctx, ToolList::Rest, ListAsk::Select(ListSlot::New));
    select_in_tool_database(&mut h, "6 mm End Mill");
    let form = h.app.form.as_ref().unwrap();
    assert_eq!(form.params["rest"]["after"][0]["name"], "6 mm End Mill");
    assert_eq!(
        form.params["tool"]["name"], "3 mm End Mill",
        "the form's own tool is left as it was"
    );
    // and its Select… replaces that one
    h.app
        .ask_listed(&h.ctx, ToolList::Rest, ListAsk::Select(ListSlot::At(0)));
    select_in_tool_database(&mut h, "6 mm Ball Nose");
    let after = &h.app.form.as_ref().unwrap().params["rest"]["after"];
    assert_eq!(after.as_array().unwrap().len(), 1);
    assert_eq!(after[0]["name"], "6 mm Ball Nose");
}

#[test]
fn three_d_pencil_picks_the_bits_that_finished_before_in_the_tool_database() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "3d_pencil_toolpath");
    h.app.pages.set_pinned(Page::Toolpaths, true);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(
        h.text_at("Bits that finished before").is_some(),
        "its own list, under its own heading"
    );
    assert!(
        h.text_at("Rest Machining").is_none(),
        "not a second list of earlier bits"
    );
    h.app
        .ask_listed(&h.ctx, ToolList::Before, ListAsk::Select(ListSlot::At(0)));
    select_in_tool_database(&mut h, "6 mm End Mill");
    let form = h.app.form.as_ref().unwrap();
    assert_eq!(form.params["bits_before"][0]["name"], "6 mm End Mill");
    assert_eq!(form.params["bits_before"].as_array().unwrap().len(), 1);
    assert_ne!(
        form.params["tool"]["name"], "6 mm End Mill",
        "the pencil bit is left as it was"
    );
    h.app
        .ask_listed(&h.ctx, ToolList::Before, ListAsk::Select(ListSlot::New));
    select_in_tool_database(&mut h, "6 mm Ball Nose");
    let form = h.app.form.as_ref().unwrap();
    assert_eq!(form.params["bits_before"][1]["name"], "6 mm Ball Nose");
}

#[test]
fn the_tool_database_window_shows_the_chip_load() {
    let mut h = Harness::new();
    h.app.tools = ToolDatabase {
        tools: vec![Tool::default()],
        ..ToolDatabase::default()
    };
    h.app.invoke(&h.ctx, "tool_database");
    h.app.tool_manager.as_mut().unwrap().pick = Some(cut_craft_tool_database::Pick::Tool(0));
    h.frame(vec![]);
    h.frame(vec![]);
    // the details scroll: down to the feeds
    let details = h.text_at("Tool Info").expect("the tool's details");
    for _ in 0..3 {
        h.frame(vec![
            Event::PointerMoved(details),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(0.0, -400.0),
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Wheel,
                modifiers: Modifiers::NONE,
            },
        ]);
    }
    h.frame(vec![]);
    // 1000 mm/min over 18000 rpm x 2 flutes
    assert!(
        h.text_at("Chip load: 0.028 mm").is_some(),
        "the window shows the feed per tooth"
    );
}

#[test]
fn the_tool_database_says_when_changes_wait_and_apply_keeps_them() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "tool_database");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at(super::options_apply::ALL_APPLIED).is_some());
    let manager = h.app.tool_manager.as_mut().unwrap();
    manager.new_material = "Cork".into();
    manager.add_material();
    h.frame(vec![]);
    assert!(h.text_at(super::options_apply::NOT_APPLIED).is_some());
    assert!(
        !h.app.tools.materials.iter().any(|m| m == "Cork"),
        "not kept yet"
    );
    click_button(&mut h, "Apply");
    assert!(h.app.tools.materials.iter().any(|m| m == "Cork"), "kept");
    h.frame(vec![]);
    assert!(h.text_at(super::options_apply::ALL_APPLIED).is_some());
}

#[test]
fn a_fine_bit_note_after_a_preview_keeps_the_count_beside_it() {
    let mut h = Harness::new();
    h.app.wait_before_showing = std::time::Duration::ZERO;
    h.app.display.animate_preview = false;
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    for id in ["profile_toolpath", "pocket_toolpath"] {
        h.app.invoke(&h.ctx, id);
        let params = h.app.form.as_ref().unwrap().params.clone();
        h.app.apply_form(id, &params).unwrap();
        until_not_busy(&mut h);
        h.app.form = None;
    }
    for t in h.app.doc.toolpaths.values_mut() {
        t.tool.diameter = 0.05;
    }
    h.app.preview_all(false);
    until_not_busy(&mut h);
    let message = &h.app.status.message;
    assert!(
        message.starts_with("Previewed 2 toolpaths. ⚠ "),
        "{message}"
    );
    assert!(message.contains("is finer than the preview's"));
}

#[test]
fn the_drag_knife_form_says_a_knife_needs_no_flute_length() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 40.0, 40.0);
    h.click(h.at(50.0, 30.0), Modifiers::NONE);
    h.app.invoke(&h.ctx, "drag_knife_toolpath");
    h.app.pages.set_pinned(Page::Toolpaths, true);
    h.frame(vec![]);
    h.frame(vec![]);
    // no "Depth not checked: set this bit's Flute Length", and no end mill or pass
    assert_eq!(h.app.form_reach(), None);
    assert!(h.text_at(super::KNIFE_LINE).is_some());
    assert!(h.text_at("0.5 mm end mill, 3 mm a pass").is_none());
    let params = &h.app.form.as_ref().unwrap().params;
    assert_eq!(
        params["pressure"].as_f64(),
        Some(50.0),
        "a share of the spring travel, not a material depth"
    );
    assert!(params.get("cut_depth").is_none());
}

#[test]
fn pocket_clears_with_bigger_bits_picked_in_the_tool_database() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.click(h.at(60.0, 35.0), Modifiers::NONE);
    h.app.invoke(&h.ctx, "pocket_toolpath");
    h.app.pages.set_pinned(Page::Toolpaths, true);
    h.app.choose_tool("3 mm End Mill");
    h.frame(vec![]);
    h.frame(vec![]);
    // one picker on the form: no separate Rest Machining list beside clearing
    assert!(
        h.text_at("Rest Machining").is_none(),
        "clearing does its job"
    );
    assert!(
        h.text_at("Add a tool…").is_none(),
        "no menus of tools on the form"
    );
    let tick = h
        .text_at("Clear with bigger bits first")
        .expect("clearing, off to start with");
    assert_eq!(
        h.app.form.as_ref().unwrap().params["clearance_tools"],
        serde_json::json!([])
    );
    // ticking it opens the Tool Database to pick the clearing bit in
    h.click(tick, Modifiers::NONE);
    h.frame(vec![]);
    h.frame(vec![]);
    let manager = h.app.tool_manager.as_mut().expect("Select Tool");
    assert!(manager.selecting);
    let big = manager
        .draft
        .tools
        .iter()
        .position(|t| t.name == "12 mm End Mill")
        .unwrap();
    manager.pick = Some(cut_craft_tool_database::Pick::Tool(big));
    h.frame(vec![]);
    h.frame(vec![]);
    let select = *h.texts_at("Select").last().expect("Select, in place of OK");
    h.click(select, Modifiers::NONE);
    h.frame(vec![]);
    h.frame(vec![]);
    let form = h.app.form.as_ref().unwrap();
    assert_eq!(
        form.params["tool"]["name"], "3 mm End Mill",
        "the finishing bit stays"
    );
    assert_eq!(form.params["clearance_tools"][0]["name"], "12 mm End Mill");
    assert_eq!(
        form.params["clearance_tools"][0]["feed_rate"],
        serde_json::json!(h.app.tools.get("12 mm End Mill").unwrap().feed_rate),
        "with its own feeds and speeds"
    );
    // another, in between: kept largest first
    h.click(
        h.text_at("Add a smaller clearing bit…")
            .expect("another bit"),
        Modifiers::NONE,
    );
    h.frame(vec![]);
    h.frame(vec![]);
    let manager = h.app.tool_manager.as_mut().expect("Select Tool");
    let middle = manager
        .draft
        .tools
        .iter()
        .position(|t| t.name == "6 mm End Mill")
        .unwrap();
    manager.pick = Some(cut_craft_tool_database::Pick::Tool(middle));
    h.frame(vec![]);
    h.frame(vec![]);
    h.click(*h.texts_at("Select").last().unwrap(), Modifiers::NONE);
    h.frame(vec![]);
    h.frame(vec![]);
    let names: Vec<&str> = h.app.form.as_ref().unwrap().params["clearance_tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["12 mm End Mill", "6 mm End Mill"]);
}

#[test]
fn pocket_says_on_the_form_when_a_clearing_bit_is_not_bigger_than_the_finishing_bit() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.click(h.at(60.0, 35.0), Modifiers::NONE);
    h.app.invoke(&h.ctx, "pocket_toolpath");
    h.app.pages.set_pinned(Page::Toolpaths, true);
    h.app.choose_tool("6 mm End Mill");
    let small = serde_json::to_value(h.app.tools.get("3 mm End Mill").unwrap()).unwrap();
    h.app.form.as_mut().unwrap().params["clearance_tools"] = serde_json::json!([small]);
    h.frame(vec![]);
    h.frame(vec![]);
    let said = h.shapes.iter().any(|clipped| {
        matches!(&clipped.shape, egui::Shape::Text(t)
            if t.galley.text().contains("must be bigger than the finishing bit"))
    });
    assert!(said, "the problem shows before Calculate");
}

#[test]
fn suggest_picks_clearing_bits_for_a_big_pocket_and_says_how_much_quicker() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 150.0, 150.0);
    h.click(h.at(160.0, 85.0), Modifiers::NONE);
    assert!(!h.app.doc.selection.is_empty(), "the rectangle is selected");
    h.app.invoke(&h.ctx, "pocket_toolpath");
    h.app.pages.set_pinned(Page::Toolpaths, true);
    h.app.choose_tool("3 mm End Mill");
    h.app.form.as_mut().unwrap().params["depth"] = 6.0.into();
    h.frame(vec![]);
    h.frame(vec![]);
    h.click(
        h.text_at("Suggest clearing bits")
            .expect("Suggest, beside the clearing bits"),
        Modifiers::NONE,
    );
    h.frame(vec![]);
    h.frame(vec![]);
    let form = h.app.form.as_ref().unwrap();
    let picked = form.params["clearance_tools"].as_array().unwrap();
    assert!(!picked.is_empty(), "a bigger bit clears this pocket first");
    for bit in picked {
        assert!(bit["diameter"].as_f64().unwrap() > 3.0, "{bit}");
    }
    let said = h.shapes.iter().any(|clipped| {
        matches!(&clipped.shape, egui::Shape::Text(t)
            if t.galley.text().contains("min with 3 mm End Mill alone"))
    });
    assert!(said, "how much quicker, on the form");
}

#[test]
fn suggest_takes_bits_from_my_tools_only() {
    let mut h = Harness::new();
    let mut catalogue = h.app.tools.get("12 mm End Mill").unwrap().clone();
    catalogue.name = "Vendor 16 mm End Mill".into();
    catalogue.diameter = 16.0;
    catalogue.group = "Vendor/End Mills".into();
    h.app.tools.tools.push(catalogue);
    h.app.tools.catalogues = vec!["Vendor".into()];
    let names: Vec<String> = h
        .app
        .suggest_candidates()
        .into_iter()
        .map(|c| c.tool.name)
        .collect();
    assert!(names.iter().any(|n| n == "12 mm End Mill"), "{names:?}");
    assert!(
        !names.iter().any(|n| n.starts_with("Vendor")),
        "a read-only catalogue's bit is not suggested: {names:?}"
    );
    assert!(
        !names.iter().any(|n| *n == Tool::laser().name),
        "nor a laser: {names:?}"
    );
}

#[test]
fn a_tool_renamed_in_the_tool_database_is_renamed_in_the_jobs_toolpaths_too() {
    let mut h = Harness::new();
    two_tools(&mut h);
    h.app.rename_toolpath_tools(
        &[("6 mm End Mill".to_owned(), "Spiral 6".to_owned())],
        "Tool database saved",
    );
    let names: Vec<(String, String)> = h
        .app
        .doc
        .toolpaths
        .values()
        .map(|t| {
            (
                t.tool.name.clone(),
                t.params["tool"]["name"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
            )
        })
        .collect();
    assert!(
        names.contains(&("Spiral 6".to_owned(), "Spiral 6".to_owned())),
        "{names:?}"
    );
    assert!(names.iter().any(|(n, _)| n == "3 mm End Mill"), "{names:?}");
    assert!(h.app.doc.toolpaths.values().all(|t| !t.stale));
    // so the tool changer slot it was loaded in still gives its number
    let ids: Vec<_> = h.app.doc.toolpaths.keys().copied().collect();
    h.app.machines.add();
    let machine = h.app.machines.active_mut().unwrap();
    machine.set_slot_count(2);
    machine.slots[0] = Some("Spiral 6".into());
    machine.slots[1] = Some("3 mm End Mill".into());
    let found = h
        .app
        .preflight(&[ids], crate::files::ToolChanges::ToolChanger);
    assert!(
        found.iter().all(|f| !f.message.contains("not loaded")),
        "{found:?}"
    );
    // undone in one step
    h.app.invoke(&h.ctx, "undo");
    assert!(
        h.app
            .doc
            .toolpaths
            .values()
            .any(|t| t.tool.name == "6 mm End Mill")
    );
}

#[test]
fn a_tool_renamed_in_the_tool_database_is_renamed_in_other_saved_jobs_when_they_open() {
    let dir = std::env::temp_dir().join(format!("cut-craft-renamed-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut h = Harness::new();
    two_tools(&mut h);
    // another job, saved before the bit was renamed
    let other = dir.join("other.ccjob");
    h.app.save_to(&other);
    // renamed in the Tool Database window, then OK
    h.app.invoke(&h.ctx, "tool_database");
    h.frame(vec![]);
    let manager = h.app.tool_manager.as_mut().unwrap();
    let at = manager.draft.index_of("6 mm End Mill").unwrap();
    manager.draft.tools[at].name = "Spiral 6".into();
    manager.note_rename("6 mm End Mill", "Spiral 6");
    h.frame(vec![]);
    let ok = *h.texts_at("OK").last().expect("the window has OK");
    h.click(ok, Modifiers::NONE);
    h.frame(vec![]);
    let names = |h: &Harness| -> Vec<String> {
        (h.app.doc.toolpaths.values())
            .map(|t| t.tool.name.clone())
            .collect()
    };
    assert!(
        names(&h).contains(&"Spiral 6".to_owned()),
        "{:?}",
        names(&h)
    );
    // the other job takes the new name when it opens, so its changer slot is found
    assert!(h.app.open_path(&other));
    assert!(
        names(&h).contains(&"Spiral 6".to_owned()),
        "{:?}",
        names(&h)
    );
    assert!(!names(&h).contains(&"6 mm End Mill".to_owned()));
    // a new bit given the old name is not mistaken for the renamed one
    let mut new = h.app.tools.get("Spiral 6").unwrap().clone();
    new.name = "6 mm End Mill".into();
    h.app.tools.tools.push(new);
    assert!(h.app.open_path(&other));
    assert!(names(&h).contains(&"6 mm End Mill".to_owned()));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_drill_shows_as_drill_only_in_the_tool_database_and_can_be_changed_there() {
    let mut h = Harness::new();
    let mut drill = h.app.tools.tools[0].clone();
    drill.name = "Drill 6mm".into();
    drill.drill_only = true;
    h.app.tools.tools.push(drill);
    let at = h.app.tools.tools.len() - 1;
    h.app.invoke(&h.ctx, "tool_database");
    h.app.tool_manager.as_mut().unwrap().pick = Some(cut_craft_tool_database::Pick::Tool(at));
    h.frame(vec![]);
    h.frame(vec![]);
    let tick = h.text_at("Drill only").expect("shown beside its tool type");
    h.click(tick, Modifiers::NONE);
    h.frame(vec![]);
    assert!(!h.app.tool_manager.as_ref().unwrap().draft.tools[at].drill_only);
}

#[test]
fn a_laser_form_shows_its_laser_from_the_tool_database_and_says_when_a_bit_is_picked() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "laser_cut_fill_toolpath");
    h.frame(vec![]);
    h.frame(vec![]);
    let texts = h.texts();
    assert!(
        texts
            .iter()
            .any(|t| t == "Laser, 0.15 mm kerf, 1500 mm/min"),
        "{texts:?}"
    );
    assert!(!texts.iter().any(|t| t.contains("end mill")), "{texts:?}");
    assert!(
        !texts.iter().any(|t| t.contains("Not in the Tool Database")),
        "the laser is in it"
    );
    assert!(!texts.iter().any(|t| t.contains("is not a laser")));
    // a router bit picked here is said in red, never refused
    h.app.choose_tool("6 mm End Mill");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(
        h.texts()
            .iter()
            .any(|t| t.contains("6 mm End Mill is not a laser")),
        "{:?}",
        h.texts()
    );
    // and a laser on a router's toolpath
    h.app.invoke(&h.ctx, "profile_toolpath");
    h.frame(vec![]);
    h.app.choose_tool(&Tool::laser().name);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(
        h.texts()
            .iter()
            .any(|t| t.contains("is a laser, which cannot cut this toolpath")),
        "{:?}",
        h.texts()
    );
}

#[test]
fn a_tool_database_or_materials_library_that_could_not_be_read_is_never_said_saved() {
    let mut h = Harness::new();
    let dir = std::env::temp_dir().join(format!("cut-craft-broken-db-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (tools, materials) = (dir.join("tools.json"), dir.join("materials.json"));
    std::fs::write(&tools, "{ not json").unwrap();
    std::fs::write(&materials, "{ not json").unwrap();
    h.app.load_tools_at(Some(tools.clone()));
    h.app.load_materials_at(Some(materials.clone()));
    h.app.tools_changed("Tool database saved");
    let message = h.app.status.message.clone();
    assert!(message.starts_with("Tool database not saved"), "{message}");
    assert!(message.contains("tools.json"), "{message}");
    assert!(message.contains("rename that file"), "{message}");
    let saved = h.app.save_materials();
    let e = saved.unwrap_err();
    assert!(e.starts_with("Materials Library not saved"), "{e}");
    assert!(e.contains("materials.json"), "{e}");
    // the broken files are left as they were
    assert_eq!(std::fs::read_to_string(&tools).unwrap(), "{ not json");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_materials_library_s_add_tool_keeps_a_catalogue_bit_copied_into_my_tools() {
    use cut_craft_tool_database::materials::{MaterialPick, MaterialsWindow};
    let mut h = Harness::new();
    let mut bit = h.app.tools.tools[0].clone();
    bit.name = "Maker Bit".into();
    bit.group = "Catalogues/Maker".into();
    h.app.tools.catalogues.push("Catalogues/Maker".into());
    h.app.tools.tools.push(bit);
    let catalogue = h.app.tools.tools.len() - 1;
    let mut window = MaterialsWindow::new(&h.app.materials, &h.app.tools);
    let maple = window
        .library
        .materials
        .iter()
        .position(|m| m.name == "Hard Maple")
        .unwrap();
    window.pick = Some(MaterialPick::Material(maple));
    h.app.materials_window = Some(window);
    // Add Tool, then Copy to My Tools and Select on the catalogue bit
    let mut picker = cut_craft_tool_database::Manager::selecting(&h.app.tools, "Maker Bit");
    cut_craft_tool_database::tree::copy_to_mine(&mut picker.draft, catalogue);
    h.app.tool_manager = Some(picker);
    h.app.picking = Some(super::PickFor::Material);
    assert!(
        h.app
            .library_outcome(&cut_craft_tool_database::Outcome::Select(
                "Maker Bit".into()
            ))
    );
    h.app.materials_kept(true);
    let has_maple =
        |t: &cut_craft_core::toolpath::Tool| t.materials.iter().any(|d| d.material == "Hard Maple");
    let mine = h.app.tools.get("Maker Bit").unwrap();
    assert!(!h.app.tools.is_catalogue(mine), "the My Tools copy is kept");
    assert!(has_maple(mine), "with the material's feeds");
    assert!(
        !has_maple(&h.app.tools.tools[catalogue]),
        "the read-only catalogue entry is left as it was"
    );
}

#[test]
fn a_broken_materials_json_still_saves_the_bits_feeds_to_tools_json() {
    use cut_craft_tool_database::materials::{MaterialPick, MaterialsWindow};
    let mut h = Harness::new();
    let dir = std::env::temp_dir().join(format!("cut-craft-broken-lib-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (tools, materials) = (dir.join("tools.json"), dir.join("materials.json"));
    std::fs::write(&materials, "{ not json").unwrap();
    h.app.tools_path = Some(tools.clone());
    h.app.load_materials_at(Some(materials.clone()));
    let mut window = MaterialsWindow::new(&h.app.materials, &h.app.tools);
    let maple = window
        .library
        .materials
        .iter()
        .position(|m| m.name == "Hard Maple")
        .unwrap();
    window.pick = Some(MaterialPick::Material(maple));
    let bit = window.tools.tools[0].name.clone();
    window.add_tool(&bit);
    h.app.materials_window = Some(window);
    h.app.materials_kept(true);
    let saved = std::fs::read_to_string(&tools).unwrap_or_default();
    assert!(saved.contains("Hard Maple"), "the feeds reach tools.json");
    let message = h.app.status.message.clone();
    assert!(
        message.starts_with("Materials Library not saved"),
        "{message}"
    );
    assert!(message.contains("feeds were saved"), "{message}");
    assert_eq!(std::fs::read_to_string(&materials).unwrap(), "{ not json");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_renamed_tool_s_toolpaths_never_say_an_unreadable_tool_database_was_saved() {
    let mut h = Harness::new();
    two_tools(&mut h);
    let dir = std::env::temp_dir().join(format!("cut-craft-broken-rename-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let tools = dir.join("tools.json");
    std::fs::write(&tools, "{ not json").unwrap();
    h.app.load_tools_at(Some(tools.clone()));
    h.app.tools_kept(
        "Tool database saved",
        &[("6 mm End Mill".to_owned(), "Spiral 6".to_owned())],
    );
    let message = h.app.status.message.clone();
    assert!(message.starts_with("Tool database not saved"), "{message}");
    assert!(!message.contains("Tool database saved"), "{message}");
    let names: Vec<_> = h.app.doc.toolpaths.values().map(|t| &t.tool.name).collect();
    assert!(names.iter().any(|n| *n == "Spiral 6"), "{names:?}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_fine_bit_is_noted_under_the_controls_while_its_preview_plays() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    for _ in 0..2 {
        h.app.invoke(&h.ctx, "profile_toolpath");
        let params = h.app.form.as_ref().unwrap().params.clone();
        h.app.apply("profile_toolpath", &params).unwrap();
        h.app.form = None;
    }
    // only the first toolpath's bit is fine
    let first = *h.app.doc.toolpaths.keys().next().expect("two toolpaths");
    h.app.doc.toolpaths.get_mut(&first).unwrap().tool.diameter = 0.05;
    h.app.display.animate_preview = true;
    h.app.invoke(&h.ctx, "preview_all_toolpaths");
    h.frame(vec![]);
    h.frame(vec![]);
    // paused near each end, so on the fine toolpath and on the wide one: not stopped
    h.app.player.playing = false;
    for share in [0.01, 0.99] {
        let playback = h.app.player.playback.as_mut().expect("playing");
        let at = playback.length() * share;
        playback.seek(at);
        h.frame(vec![]);
        assert!(h.app.player.playback.is_some() && h.app.preview.material.is_none());
        assert!(
            h.texts()
                .iter()
                .any(|t| t.contains("is finer than the preview's")),
            "the note shows while it plays, whichever toolpath is cutting ({share})"
        );
    }
}

#[test]
fn a_new_jobs_default_bit_has_its_flute_length_so_the_form_warns_of_nothing() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.invoke(&h.ctx, "profile_toolpath");
    h.app.pages.set_pinned(Page::Toolpaths, true);
    h.frame(vec![]);
    let line = h.app.form_reach().expect("a reach line");
    assert!(!line.text().contains("not checked"), "{line:?}");
}
