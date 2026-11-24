//! Layers, sheets and levels, and the job's notes and variables.

use super::*;

#[test]
fn pasted_text_is_a_word_of_its_own_on_its_layer_and_still_text() {
    let mut h = Harness::new();
    let mut c = Change::builder(h.doc());
    let other = c.add_layer("Letters");
    let ids: Vec<EntityId> = [0.0, 10.0]
        .into_iter()
        .map(|x| c.add(Path::circle(Point::new(x, 0.0), 2.0)))
        .collect();
    for &id in &ids {
        c.set_group(id, Some(5));
        c.move_to_layer(id, other);
    }
    c.set_text(5, Some(serde_json::json!({"text": "oo"})));
    c.select(ids.clone());
    let change = c.finish();
    h.given(&change);
    h.frame(vec![Event::Copy]);
    // through the system clipboard, as Ctrl+V brings it back
    h.frame(vec![Event::Paste(h.copied.clone())]);
    let pasted: Vec<EntityId> = h.doc().selection.iter().copied().collect();
    assert_eq!(pasted.len(), 2);
    let group = h.doc().vector_groups.get(&pasted[0]).copied();
    assert!(
        group.is_some() && group != Some(5),
        "the copy is a group of its own"
    );
    assert_eq!(h.doc().vector_groups.get(&pasted[1]).copied(), group);
    assert_eq!(h.doc().texts[&group.unwrap()]["text"], "oo");
    assert_eq!(h.doc().layer_of(pasted[0]), other);
}

#[test]
fn pasted_into_another_job_lands_on_its_current_layer() {
    let mut h = Harness::new();
    let mut c = Change::builder(h.doc());
    let other = c.add_layer("Letters");
    let id = c.add(Path::circle(Point::ZERO, 2.0));
    c.move_to_layer(id, other);
    c.select([id]);
    let change = c.finish();
    h.given(&change);
    h.frame(vec![Event::Copy]);
    // another job, with its own unrelated layer under the same number
    h.start_another_job();
    let mut c = Change::builder(h.doc());
    let hidden = c.add_layer("Hidden");
    let change = c.finish();
    h.given(&change);
    assert_eq!(hidden, other, "the new job's layer took the same number");
    h.frame(vec![Event::Paste(h.copied.clone())]);
    let pasted: Vec<EntityId> = h.doc().selection.iter().copied().collect();
    assert_eq!(pasted.len(), 1);
    assert_ne!(
        h.doc().layer_of(pasted[0]),
        hidden,
        "another job's layer numbers mean nothing here"
    );
}

#[test]
fn notes_starting_with_a_dot_open_with_the_job() {
    let dir = std::env::temp_dir().join(format!("cut-craft-notes-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut h = Harness::new();
    for (notes, opens) in [(".Read me", true), ("quiet", false)] {
        h.given_document(|doc| doc.notes = notes.into());
        let path = dir.join("notes.ccjob");
        h.save_to(&path);
        h.close_form();
        assert!(h.open(&path));
        assert_eq!(h.form_id() == Some("notes"), opens, "{notes}");
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn hiding_a_layer_from_the_panel_is_undoable_and_stops_picking() {
    let mut h = Harness::new();
    h.show_design_tab("Layers");
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.click_layer(LayerId(1), "toggle_visible");
    h.frame(vec![]);
    h.click(h.at(60.0, 35.0), Modifiers::NONE);
    assert!(h.doc().selection.is_empty());
    h.undo();
    assert!(h.doc().layers[0].visible);
}

#[test]
fn a_layer_used_on_another_sheet_is_never_deleted_from_under_it() {
    let mut h = Harness::new();
    h.open_document(cut_craft_core::testing::two_sheets());
    h.run("add_layer", &serde_json::json!({ "name": "Engraving" }))
        .unwrap();
    let engraving = h.doc().layers.last().unwrap().id;
    let first = h.doc().layers[0].id;
    // on Sheet 2, draw on Engraving, then go back to Sheet 1, where it is empty
    h.run("activate_sheet", &serde_json::json!({ "sheet": 1 }))
        .unwrap();
    h.run(
        "set_current_layer",
        &serde_json::json!({ "layer": engraving }),
    )
    .unwrap();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.run("set_current_layer", &serde_json::json!({ "layer": first }))
        .unwrap();
    h.run("activate_sheet", &serde_json::json!({ "sheet": 0 }))
        .unwrap();
    // Delete ▸ Empty leaves it: it isn't empty on Sheet 2
    let empty = serde_json::json!({ "items": "layers", "item": 0, "which": "empty" });
    let _ = h.run("delete_items", &empty);
    assert!(h.doc().layer(engraving).is_some(), "kept for Sheet 2");
    // Delete Layer says where its work is, rather than lose it
    let refused = h
        .run("delete_layer", &serde_json::json!({ "layer": engraving }))
        .unwrap_err();
    assert!(refused.contains("Sheet 2"), "{refused}");
    // Merge Visible brings Sheet 2's vectors onto the layer merged into
    h.run(
        "merge_visible_layers",
        &serde_json::json!({ "layer": first }),
    )
    .unwrap();
    assert!(h.doc().layer(engraving).is_none());
    h.run("activate_sheet", &serde_json::json!({ "sheet": 1 }))
        .expect("Sheet 2 still opens");
    assert_eq!(h.doc().vectors.len(), 1);
    let id = *h.doc().vectors.keys().next().unwrap();
    assert_eq!(h.doc().layer_of(id), first);
}

#[test]
fn choosing_a_layer_makes_it_current_without_an_undo_step() {
    let mut h = Harness::new();
    h.run("add_layer", &serde_json::json!({ "name": "Top" }))
        .unwrap();
    h.click_layer(LayerId(1), "set_current_layer");
    assert_eq!(h.doc().current_layer, LayerId(1));
    h.undo();
    assert_eq!(
        h.doc().layers.len(),
        1,
        "the last undo step added the layer"
    );
}

#[test]
fn the_layer_menu_acts_on_the_layer_clicked() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.run("add_layer", &serde_json::json!({ "name": "Top" }))
        .unwrap();
    h.choose_on_layer(1, &["Move Selection Here"]);
    assert!(
        h.doc()
            .vectors
            .keys()
            .all(|&id| h.doc().layer_of(id) == LayerId(2))
    );
    h.choose_on_layer(0, &["Activate"]);
    assert_eq!(h.doc().current_layer, LayerId(1));
    h.choose_on_layer(0, &["Show", "Only This"]);
    let visible: Vec<bool> = h.doc().layers.iter().map(|l| l.visible).collect();
    assert_eq!(visible, [true, false]);
    h.choose_on_layer(0, &["Lock"]);
    let locked = h.layer_menu(0);
    assert!(!leaf(&locked, &["Lock"]).is_enabled && leaf(&locked, &["Unlock"]).is_enabled);
    h.choose_on_layer(1, &["Rename"]);
    assert_eq!(
        (h.form_id(), h.form_value("name").as_str()),
        (Some("layer_properties"), Some("Top"))
    );
    h.close_form();
    h.choose_on_layer(0, &["Delete", "Empty"]);
    let names: Vec<&str> = h.doc().layers.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, ["Top"], "the first layer had nothing left on it");
}

#[test]
fn the_sheets_tab_lists_each_sheets_size_and_activates_one() {
    use cut_craft_core::document::Units;
    let mut h = Harness::new();
    h.pin(Page::Design);
    h.show_design_tab(DESIGN_TABS[1]);
    h.open_document(cut_craft_core::testing::two_sheets());
    // the list's grid measures its columns on the first frame
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Sheets List").is_some());
    assert!(h.text_at("Sheet 2").is_some());
    assert!(
        h.text_at(&sheet_length(300.0, Units::Mm)).is_some(),
        "Sheet 2's X"
    );
    assert!(
        h.text_at(&sheet_length(12.0, Units::Mm)).is_some(),
        "Sheet 2's Z"
    );
    assert!(h.text_at("Add New Sheet...").is_some());
    let at = h.text_at("Sheet 2").unwrap();
    h.click(at, Modifiers::NONE);
    assert_eq!(h.doc().active_sheet, 1, "clicking a sheet activates it");
    assert_eq!(h.doc().job.width, 300.0);
}

#[test]
fn appearance_sets_the_2d_sheet_and_grid_colours_for_each_theme() {
    let mut h = Harness::new();
    h.given_options(|o| o.appearance.dark_2d.grid_color = [1, 2, 3]);
    h.invoke("options");
    h.frame(vec![]);
    h.frame(vec![]);
    let at = *h.texts_at("Appearance").last().expect("listed");
    h.click(at, Modifiers::NONE);
    h.frame(vec![]);
    for label in ["2D view in the light theme", "2D view in the dark theme"] {
        assert!(h.text_at(label).is_some(), "{label}");
    }
    assert_eq!(h.texts_at("Sheet color").len(), 2, "one for each theme");
    assert_eq!(h.texts_at("Grid color").len(), 2, "one for each theme");
    let reset = h.text_at("Reset to defaults").expect("the page's reset");
    h.click(reset, Modifiers::NONE);
    h.frame(vec![]);
    click_button(&mut h, "Apply");
    assert_eq!(
        h.options().appearance.dark_2d,
        crate::options::CanvasColours::dark()
    );
}

#[test]
fn clipart_dropped_on_a_view_goes_there_and_import_to_level_puts_it_on_a_level() {
    use crate::clipart::ToLevel;
    use cut_craft_core::geometry::Point;
    let mut h = Harness::new();
    // a 20 mm square drawn as SVG, dropped on the 2D view at (100, 80)
    let svg = std::env::temp_dir().join(format!("cut-craft-drop-{}.svg", std::process::id()));
    std::fs::write(
        &svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="20mm" height="20mm" viewBox="0 0 20 20"><rect x="0" y="0" width="20" height="20"/></svg>"#,
    )
    .unwrap();
    h.drop_clipart(&svg, Some(Point::new(100.0, 80.0)), None);
    let bounds = h
        .doc()
        .vectors
        .values()
        .map(cut_craft_core::geometry::Path::bounds)
        .reduce(cut_craft_core::geometry::Rect::union)
        .expect("the square came in");
    assert!(
        bounds.center().distance(Point::new(100.0, 80.0)) < 1e-6,
        "centred where dropped: {bounds:?}"
    );
    h.undo();
    assert!(h.doc().vectors.is_empty(), "one undo takes the drop away");
    let _ = std::fs::remove_file(svg);
    // a model to a new level named after it, placed where dropped as it is made
    let file = pyramid_stl("to-level");
    let levels = h.doc().levels.len();
    h.drop_clipart(&file, Some(Point::new(150.0, 100.0)), Some(ToLevel::New));
    assert_eq!(h.doc().levels.len(), levels + 1);
    let (&new, level) = h.doc().levels.iter().next_back().unwrap();
    assert!(
        level.name.starts_with("cut-craft-to-level"),
        "{}",
        level.name
    );
    let k = h.doc().components.values().next().expect("the model is in");
    assert_eq!(k.level, new, "on the new level");
    assert!(
        k.extent().center().distance(Point::new(150.0, 100.0)) < 1.0,
        "centred where dropped: {:?}",
        k.extent()
    );
    // moving it in the form keeps it on the level
    h.set_form_value(
        "/placing/position",
        serde_json::json!({ "x": 50.0, "y": 60.0 }),
    );
    h.frame(vec![]);
    assert_eq!(h.doc().components.values().next().unwrap().level, new);
    // and to an existing level
    h.close_form();
    h.drop_clipart(&file, None, Some(ToLevel::Existing(0)));
    assert_ne!(new, 0, "the job starts with level 0");
    assert_eq!(h.doc().level_components(0).len(), 1, "the second model");
    assert_eq!(h.doc().level_components(new).len(), 1, "the first stays");
    let _ = std::fs::remove_file(file);
}

#[test]
fn the_sheet_menu_offers_no_show_or_hide_that_would_do_nothing() {
    let doc = cut_craft_core::testing::two_sheets();
    let menu = gui::sheet_menu(&doc, 1);
    let names: Vec<&str> = menu.iter().map(|m| m.name.as_str()).collect();
    assert!(!names.contains(&"Show"), "{names:?}");
    assert!(!names.contains(&"Hide"), "{names:?}");
    // nor a Shown tick in Rename
    let schema = serde_json::to_value(schemars::schema_for!(cmd_sheet_properties::Params)).unwrap();
    assert!(schema["properties"].get("visible").is_none(), "{schema}");
}

#[test]
fn delete_layer_is_on_the_layer_menu_once_there_is_another_layer() {
    let mut h = Harness::new();
    let only = h.layer_menu(0);
    let item = only
        .iter()
        .find(|i| i.name == "Delete Layer")
        .expect("listed");
    assert!(!item.is_enabled, "the last layer stays");
    h.run("add_layer", &serde_json::json!({ "name": "Top" }))
        .unwrap();
    let last = h.doc().layers.len() - 1;
    let top = h.doc().layers[last].id;
    let menu = h.layer_menu(last);
    let item = menu.iter().find(|i| i.name == "Delete Layer").unwrap();
    assert!(item.is_enabled);
    let action = item.action.clone();
    h.invoke(&action);
    assert!(h.doc().layers.iter().all(|l| l.id != top), "deleted");
}

#[test]
fn an_empty_sheet_says_where_to_start_until_something_is_drawn() {
    let mut h = Harness::new();
    let hint = format!(
        "{}\n{}",
        super::views::EMPTY_SHEET_TITLE,
        super::views::EMPTY_SHEET_HINT
    );
    let title = hint.as_str();
    assert!(h.text_at(title).is_some(), "shown on a new job");
    h.invoke("draw_rectangle");
    h.frame(vec![]);
    assert!(h.text_at(title).is_none(), "not over a tool in hand");
    h.close_form();
    h.frame(vec![]);
    assert!(h.text_at(title).is_some());
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    assert!(h.text_at(title).is_none(), "gone with the first shape");
}

#[test]
fn long_notes_scroll_and_the_variable_explorer_stays_in_sight() {
    use std::fmt::Write as _;
    let mut h = Harness::new();
    let notes = (1..=200).fold(String::new(), |mut text, n| {
        let _ = writeln!(text, "Line {n}");
        text
    });
    h.given_document(|doc| doc.notes = notes);
    h.invoke("notes");
    h.frame(vec![]);
    h.frame(vec![]);
    let explorer = h
        .text_at("Variable Explorer")
        .expect("the Variable Explorer shows");
    let screen = h.ctx.content_rect();
    assert!(
        explorer.y < screen.bottom() - 100.0,
        "{explorer:?} in {screen:?}"
    );
}

#[test]
fn notes_fill_the_panel_and_a_double_clicked_variable_goes_in_live() {
    let mut h = Harness::new();
    h.given_document(|doc| doc.notes = "Plain old notes".into());
    h.invoke("notes");
    h.frame(vec![]);
    h.frame(vec![]);
    // the old notes load as they were, and the box takes the panel's height
    assert_eq!(h.form_value("notes"), "Plain old notes");
    let explorer = h
        .text_at("Variable Explorer")
        .expect("the Variable Explorer shows");
    assert!(
        explorer.y > 400.0,
        "the notes box above it fills the panel: {explorer:?}"
    );
    for button in [
        "Write",
        "Preview",
        "☐",
        "Show these notes when the job opens",
    ] {
        assert!(h.text_at(button).is_some(), "{button}");
    }
    // found by searching, a double-click on the material's Width puts it in at the cursor
    h.search_variables("material width");
    h.frame(vec![]);
    h.frame(vec![]);
    let row = *h
        .texts_at("Width")
        .last()
        .unwrap_or_else(|| panic!("{:?}", h.texts()));
    h.double_click(row);
    h.frame(vec![]);
    assert_eq!(h.form_value("notes"), "Plain old notes{{material.width}}");
    // Preview shows its value now
    let preview = h.text_at("Preview").unwrap();
    h.click(preview, Modifiers::NONE);
    h.frame(vec![]);
    let tall = h.variable_value("material.width");
    assert!(
        h.text_at(&tall).is_some(),
        "{tall} in the preview: {:?}",
        h.texts()
    );
    let apply = h.text_at("Apply").unwrap();
    h.click(apply, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.doc().notes, "Plain old notes{{material.width}}");
}

#[test]
fn the_blank_sheet_links_to_a_recent_file_and_the_sample() {
    let mut h = Harness::new();
    h.frame(vec![]);
    assert!(
        h.text_at(views::EMPTY_SHEET_RECENT).is_none(),
        "no recent files yet"
    );
    let at = h
        .text_at(views::EMPTY_SHEET_SAMPLE)
        .expect("the sample's link");
    h.click(at, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.doc().toolpaths.len(), 2, "the sample opened");
}
