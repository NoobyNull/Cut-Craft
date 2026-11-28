//! The toolpath list: its rows, menus, reordering and the toolpath summary.

use super::*;

#[test]
fn a_dragged_value_in_the_3d_settings_changes_the_component_as_it_goes() {
    let mut h = Harness::new();
    h.open_document(cut_craft_core::testing::domed());
    h.given_document(|doc| doc.selected_components = [EntityId(1)].into());
    h.given_orbit(view3d::Orbit::TOP);
    h.show_view(View::ThreeD);
    h.invoke("transform");
    h.frame(vec![]);
    h.frame(vec![]);
    let settings = handle_3d_at(&h, Handle3d::Settings);
    h.click(settings, Modifiers::NONE);
    h.frame(vec![]);
    let tilt = h.text_at("Tilt").expect("the settings popup");
    h.click(tilt, Modifiers::NONE);
    h.frame(vec![]);
    let angle = |h: &Harness| {
        let k = &h.doc().components[&EntityId(1)];
        k.shaping.tilt.map(|t| t.angle)
    };
    assert!(angle(&h).is_some(), "tilted");
    let undo_from = h.undo_steps();
    let held = h.text_at("Combine with other components ...").unwrap();
    h.press(held, true, Modifiers::NONE);
    for step in 1..=3 {
        let to = 10.0 * f64::from(step);
        h.edit_3d_settings(|p| p.tilt.angle = to);
        h.frame(vec![]);
        assert_eq!(angle(&h), Some(to), "follows the drag: {step}");
    }
    h.press(held, false, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.undo_steps(), undo_from + 1, "one undo");
    h.invoke("undo");
    assert_ne!(angle(&h), Some(30.0), "the drag undone at once");
}

#[test]
fn the_components_tab_is_the_component_list() {
    let mut h = Harness::new();
    h.invoke("design_layout");
    h.show_design_tab("Components");
    h.frame(vec![]);
    assert!(h.text_at("No components yet").is_some());
}

#[test]
fn the_toolpath_list_shows_hides_edits_and_reorders_its_rows() {
    let mut h = Harness::new();
    h.open_document(cut_craft_core::testing::three_toolpaths());
    let mut c = Change::builder(h.doc());
    for (id, name) in [(1, "Pocket"), (2, "Drill"), (3, "Cut Out")] {
        let mut t = h.doc().toolpaths[&EntityId(id)].clone();
        t.name = name.into();
        c.replace_toolpath(EntityId(id), t);
    }
    h.given(&c.finish());
    h.given_job_screen();
    h.pin(Page::Toolpaths);
    h.frame(vec![]);
    h.frame(vec![]);
    let names = |h: &Harness| -> Vec<String> {
        h.doc().toolpaths.values().map(|t| t.name.clone()).collect()
    };
    // the tick left of a name hides it
    let drill = h.text_at("Drill").unwrap();
    h.click(drill - egui::vec2(24.0, 0.0), Modifiers::NONE);
    assert!(!h.doc().toolpaths[&EntityId(2)].visible);
    // a click selects, the down arrow moves it below the next one
    h.click(drill, Modifiers::NONE);
    assert_eq!(h.selected_toolpath(), Some(EntityId(2)));
    let down = h.text_at("Move Down").unwrap();
    h.click(down, Modifiers::NONE);
    assert_eq!(names(&h), ["Pocket", "Cut Out", "Drill"]);
    assert_eq!(
        h.selected_toolpath(),
        Some(EntityId(3)),
        "it stays selected"
    );
    assert!(
        !h.doc().toolpaths[&EntityId(3)].visible,
        "hidden moves with it"
    );
    // a double-click opens its form
    h.frame(vec![]);
    let pocket = h.text_at("Pocket").unwrap();
    h.double_click(pocket);
    h.frame(vec![]);
    assert_eq!(h.selected_toolpath(), Some(EntityId(1)));
    assert!(h.form_id().is_some(), "the toolpath's form is open");
}

#[test]
fn a_cut_list_imports_as_parts_and_nest_parts_opens_to_move_them() {
    let mut h = Harness::new();
    let dir = std::env::temp_dir().join(format!("cut-craft-cut-list-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("kitchen.csv");
    std::fs::write(&file, "Name,Length,Width,Qty\nSide,72,56,2\n").unwrap();
    h.import_cut_list(&file);
    assert_eq!(h.doc().vectors.len(), 2);
    assert_eq!(h.doc().part_names.values().next().unwrap(), "Side");
    assert_eq!(h.form_id(), Some("nest_parts"), "Nest Parts opens next");
    assert_eq!(h.form_value("remove_original_parts"), true);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_layers_list_arrows_move_the_active_layer_and_merge_visible_is_offered() {
    let mut h = Harness::new();
    h.open_document(cut_craft_core::testing::two_layers());
    let first = h.doc().layers[0].id;
    h.given_document(|doc| doc.current_layer = first);
    h.pin(Page::Design);
    h.show_design_tab("Layers");
    h.frame(vec![]);
    h.frame(vec![]);
    let up = *h
        .texts_at("Move Up")
        .first()
        .expect("the arrows are over the list");
    h.click(up, Modifiers::NONE);
    assert_eq!(
        h.doc().layers.last().map(|l| l.id),
        Some(first),
        "now at the top"
    );
    let menu = h.layer_menu(0);
    let merge = leaf(&menu, &["Merge Visible"]);
    assert!(merge.is_enabled, "another layer is visible");
    // layers with nothing on this sheet can be left out of the list
    let empty = (h.doc().layers.iter())
        .find(|l| h.doc().vectors_on(l.id).next().is_none() && l.id != first)
        .map(|l| l.name.clone());
    if let Some(name) = empty {
        h.show_only_layers_on_sheet();
        h.frame(vec![]);
        assert!(h.text_at(&name).is_none(), "{name} is left out");
    }
}

#[test]
fn favourites_list_each_tool_on_its_own_even_from_one_flyout() {
    let mut h = Harness::new();
    h.pin(Page::Design);
    h.given_options(|o| {
        o.favourites.design = vec!["draw_circle".into(), "draw_ellipse".into()];
        o.favourites.design_only = true;
    });
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Favourites").is_some());
    for arrow in flyout_arrows(&h) {
        h.frame(vec![Event::PointerMoved(arrow - vec2(12.0, 12.0))]);
        h.frame(vec![]);
        let hint = cut_craft_bug_report::hint::current(&h.ctx).unwrap_or_default();
        assert!(
            !hint.contains("corner arrow"),
            "a flyout among favourites: {hint}"
        );
    }
}

#[test]
fn a_model_dragged_in_the_2d_view_is_drawn_where_it_is_going() {
    let mut h = Harness::new();
    let rect = picked_model(&mut h);
    let o = h.doc().job.area().min;
    let at = |h: &Harness, p: Point| h.at(p.x - o.x, p.y - o.y);
    let (from, to) = (
        at(&h, rect.center()),
        at(&h, rect.center() + Point::new(40.0, 0.0)),
    );
    h.frame(vec![Event::PointerMoved(from)]);
    h.press(from, true, Modifiers::NONE);
    h.frame(vec![Event::PointerMoved(from.lerp(to, 0.5))]);
    h.frame(vec![Event::PointerMoved(to)]);
    let drawn = textured_meshes(&h);
    assert_eq!(drawn.len(), 1, "{drawn:?}");
    assert!(
        (drawn[0].center().x - to.x).abs() < 2.0,
        "{drawn:?} at {to:?}"
    );
    h.press(to, false, Modifiers::NONE);
    h.frame(vec![]);
}

#[test]
fn an_older_pocket_with_a_rest_list_still_shows_it() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.click(h.at(60.0, 35.0), Modifiers::NONE);
    h.invoke("pocket_toolpath");
    h.pin(Page::Toolpaths);
    let big = serde_json::to_value(h.tool_database().get("12 mm End Mill").unwrap()).unwrap();
    // as an older file holds it: a rest list the form no longer starts with
    let mut older = h.form_settings().clone();
    older["rest"] = serde_json::json!({ "after": [big], "minimum_detail": 0.0, "overlap": 0.0 });
    h.set_form_settings(older);
    h.frame(vec![]);
    h.frame(vec![]);
    h.form_tab(crate::form::form_tabs::FormTab::Where);
    assert!(
        h.text_at("Rest Machining").is_some(),
        "kept, so it can be changed"
    );
}

#[test]
fn the_toolpath_list_stays_under_every_toolpaths_tab() {
    use super::panels::{SAVE_FOR_MACHINE, SAVE_NEEDS};
    let mut h = Harness::new();
    h.pin(Page::Toolpaths);
    h.frame(vec![]);
    h.frame(vec![]);
    // Make opens first, the job and its tools over the tabs, the list under them
    assert!(h.text_at("Job and Bits").is_some());
    assert!(h.text_at("Create a toolpath: pick how to cut").is_some());
    assert!(h.text_at("Toolpath List").is_some());
    assert!(
        h.text_at(SAVE_FOR_MACHINE).is_none(),
        "saving has its own tab"
    );
    // the tab row stays where it is whichever tab is open
    let tabs: Vec<(&str, egui::Pos2)> = ["Preview", "Analyse", "Output", "Create"]
        .into_iter()
        .map(|name| {
            // the Design page has a Create sub-tab too: the one on the right
            let at = h
                .shapes
                .iter()
                .filter_map(|c| match &c.shape {
                    egui::Shape::Text(t) if t.galley.text() == name => {
                        Some(t.pos + t.galley.size() / 2.0)
                    }
                    _ => None,
                })
                .max_by(|a, b| a.x.total_cmp(&b.x))
                .unwrap();
            (name, at)
        })
        .collect();
    for (name, at) in tabs {
        h.click(at, Modifiers::NONE);
        h.frame(vec![]);
        assert_eq!(h.toolpath_tab(), name);
        assert!(
            h.text_at("Toolpath List").is_some(),
            "the list under {name}"
        );
    }
    // Output: the big button, and what it waits for
    h.show_toolpath_tab("Output");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at(SAVE_NEEDS).is_some(), "says what to do first");
    assert!(h.text_at("Paperwork for the Job").is_some());
    h.click(h.text_at(SAVE_FOR_MACHINE).unwrap(), Modifiers::NONE);
    assert!(!h.save_form_open(), "nothing to save yet");
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.invoke("profile_toolpath");
    let params = h.form_settings().clone();
    h.run("profile_toolpath", &params).unwrap();
    h.close_form();
    h.show_toolpath_tab("Output");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Profile").is_some(), "the new toolpath, in sight");
    assert!(h.text_at(SAVE_NEEDS).is_none());
    h.click(h.text_at(SAVE_FOR_MACHINE).unwrap(), Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.save_form_open(), "the same Save Toolpaths form");
    // Preview: every preview a button of its own, none in a More… menu
    h.close_save_form();
    h.show_toolpath_tab("Preview");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Undo Last Preview").is_some());
    assert!(h.text_at("More…").is_none());
    // Analyse: the checks before cutting on the shown toolpaths
    h.show_toolpath_tab("Analyse");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Checks before cutting").is_some());
    assert!(
        h.text_at("✔ Every toolpath matches what it was made from")
            .is_some()
    );
}

#[test]
fn showing_only_favourites_keeps_the_tabs_and_the_toolpath_list() {
    let mut h = named_toolpaths();
    let only = format!("{}toolpaths:true", gui::FAVOURITES_ONLY);
    h.invoke(&only);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(
        h.text_at("No favourites yet: right-click a tool ▸ Add to Favourites")
            .is_some(),
        "favourites in place of the strategies"
    );
    for text in ["Create", "Preview", "Output", "Toolpath List", "Drill"] {
        assert!(h.text_at(text).is_some(), "{text} still there");
    }
}

#[test]
fn a_toolpath_dragged_onto_another_goes_just_before_it() {
    let mut h = named_toolpaths();
    let (from, to) = (h.text_at("Cut Out").unwrap(), h.text_at("Pocket").unwrap());
    h.frame(vec![Event::PointerMoved(from)]);
    h.press(from, true, Modifiers::NONE);
    for k in 1..=8_u8 {
        h.frame(vec![Event::PointerMoved(from.lerp(to, f32::from(k) / 8.0))]);
    }
    h.press(to, false, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(toolpath_names(&h), ["Cut Out", "Pocket", "Drill"]);
}

#[test]
fn del_after_picking_a_toolpath_in_the_list_deletes_that_toolpath_not_the_vectors() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.invoke("profile_toolpath");
    let params = h.form_settings().clone();
    h.run("profile_toolpath", &params).unwrap();
    h.close_form();
    h.invoke("show_toolpaths_tab");
    h.frame(vec![]);
    h.frame(vec![]);
    let name = h.doc().toolpaths.values().next().unwrap().name.clone();
    let row = h.text_at(&name).expect("the toolpath's row");
    h.click(row, Modifiers::NONE);
    assert!(h.selected_toolpath().is_some(), "picked in the list");
    assert!(
        !h.doc().selection.is_empty(),
        "the rectangle is still selected"
    );
    h.frame(vec![key_with(Key::Delete, Modifiers::NONE)]);
    assert!(h.doc().toolpaths.is_empty(), "the picked toolpath went");
    assert_eq!(h.doc().vectors.len(), 1, "the drawing is kept");
    // a click in the drawing after that: Del deletes the vectors again
    h.invoke("profile_toolpath");
    let params = h.form_settings().clone();
    h.run("profile_toolpath", &params).unwrap();
    h.close_form();
    h.frame(vec![]);
    let row = h.text_at(&name).expect("the toolpath's row");
    h.click(row, Modifiers::NONE);
    let all: Vec<EntityId> = h.doc().vectors.keys().copied().collect();
    h.select(all.clone());
    h.click(Pos2::new(640.0, 400.0), Modifiers::NONE);
    h.select(all);
    h.frame(vec![key_with(Key::Delete, Modifiers::NONE)]);
    assert_eq!(h.doc().toolpaths.len(), 1);
    assert!(h.doc().vectors.is_empty(), "Del is the drawing's again");
}

#[test]
fn ticking_a_toolpath_off_in_the_list_is_one_undo_step() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    let params = h.opening_settings("drilling_toolpath");
    h.run("drilling_toolpath", &params).unwrap();
    let (&id, t) = h.doc().toolpaths.iter().next().unwrap();
    let name = t.name.clone();
    h.invoke("show_toolpaths_tab");
    h.frame(vec![]);
    h.frame(vec![]);
    let at = h.text_at(&name).expect("the toolpath in the list");
    let depth = h.undo_steps();
    // the tick box: the small square just left of the name
    let tick = (h.shapes.iter())
        .filter_map(|c| match &c.shape {
            egui::Shape::Rect(r)
                if (r.rect.center().y - at.y).abs() < 4.0
                    && r.rect.max.x < at.x
                    && r.rect.width() < 20.0 =>
            {
                Some(r.rect.center())
            }
            _ => None,
        })
        .max_by(|a, b| a.x.total_cmp(&b.x))
        .expect("the tick box");
    h.click(tick, Modifiers::NONE);
    assert!(!h.doc().toolpaths[&id].visible, "hidden");
    assert_eq!(h.undo_steps(), depth + 1, "one step to undo");
    h.invoke("undo");
    assert!(h.doc().toolpaths[&id].visible, "one undo shows it again");
}

#[test]
fn a_smaller_recent_file_list_only_hides_the_rest_until_it_grows_again() {
    let mut h = Harness::new();
    h.given_options(|o| o.general.recent_file_list_size = 9);
    h.given_recent_files(
        (0..9)
            .map(|i| std::path::PathBuf::from(format!("job{i}.ccjob")))
            .collect(),
    );
    h.invoke("options");
    h.frame(vec![]);
    h.frame(vec![]);
    let at = *h.texts_at("General Settings").last().expect("listed");
    h.click(at, Modifiers::NONE);
    h.frame(vec![]);
    let reset = h.text_at("Reset to defaults").expect("the page's reset");
    h.click(reset, Modifiers::NONE);
    h.frame(vec![]);
    click_button(&mut h, "Apply");
    assert_eq!(h.options().recent_count(), files::RECENT_COUNT);
    assert_eq!(h.recent_shown().len(), files::RECENT_COUNT);
    h.given_options(|o| o.general.recent_file_list_size = 9);
    assert_eq!(h.recent_shown().len(), 9, "all nine back");
}

#[test]
fn an_open_edit_toolpath_form_follows_its_toolpath_through_a_reorder_and_its_undo() {
    let mut h = named_toolpaths();
    // the job put in as a new one is
    h.given_it_opened_as_a_new_job();
    h.frame(vec![]);
    h.select_toolpath(EntityId(1));
    h.edit_toolpath();
    assert_eq!(h.form_editing(), Some(EntityId(1)));
    // the Pocket moved down below the Drill: it is now at id 2
    h.run(
        "arrange_toolpaths",
        &serde_json::json!({ "order": [2, 1, 3] }),
    )
    .unwrap();
    h.frame(vec![]);
    assert_eq!(h.doc().toolpaths[&EntityId(2)].name, "Pocket");
    assert_eq!(
        h.form_editing(),
        Some(EntityId(2)),
        "still edits the Pocket"
    );
    h.invoke("undo");
    h.frame(vec![]);
    assert_eq!(h.doc().toolpaths[&EntityId(1)].name, "Pocket");
    assert_eq!(h.form_editing(), Some(EntityId(1)));
    h.invoke("redo");
    h.frame(vec![]);
    assert_eq!(h.form_editing(), Some(EntityId(2)));
}

#[test]
fn cancel_on_a_form_keeps_a_reorder_made_in_the_toolpath_list_while_it_was_open() {
    let mut h = named_toolpaths();
    h.invoke("draw_circle");
    h.frame(vec![]);
    let circle = |x: f64| serde_json::json!({ "center": { "x": x, "y": 150.0 }, "radius": 20.0 });
    h.run("draw_circle", &circle(100.0)).unwrap();
    // the Pocket moved down in the toolpath list beside the form
    h.select_toolpath(EntityId(1));
    h.frame(vec![]);
    let down = *h
        .texts_at("Move Down")
        .last()
        .expect("the list's Move Down");
    h.click(down, Modifiers::NONE);
    assert_eq!(h.doc().toolpaths[&EntityId(2)].name, "Pocket");
    assert!(h.form_id().is_some(), "the form is still open");
    h.run("draw_circle", &circle(200.0)).unwrap();
    assert_eq!(h.doc().vectors.len(), 2);
    h.cancel_form();
    assert_eq!(
        h.doc().toolpaths[&EntityId(2)].name,
        "Pocket",
        "the reorder is kept"
    );
    assert_eq!(
        h.doc().vectors.len(),
        0,
        "what the form did before the reorder and after it is undone"
    );
    // one Undo takes back the reorder, as the list's own step
    h.invoke("undo");
    assert_eq!(h.doc().toolpaths[&EntityId(1)].name, "Pocket");
    assert_eq!(h.doc().vectors.len(), 0);
}

#[test]
fn cancel_with_only_a_list_change_since_the_form_opened_has_nothing_to_take_back() {
    let mut h = named_toolpaths();
    h.invoke("draw_circle");
    h.frame(vec![]);
    h.select_toolpath(EntityId(1));
    h.frame(vec![]);
    let down = *h
        .texts_at("Move Down")
        .last()
        .expect("the list's Move Down");
    h.click(down, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.form_list_steps(), 1, "the list's step is noted");
    assert_eq!(
        h.undo_steps(),
        h.form_opened_at_step() + 1,
        "only the list's step since it opened"
    );
    h.cancel_form();
    assert_eq!(h.doc().toolpaths[&EntityId(2)].name, "Pocket");
}

/// `named_toolpaths` with Pocket in group Outside and Drill in group Holes, the list
/// noting where its grips and arrows are.
fn grouped_toolpaths() -> Harness {
    let mut h = named_toolpaths();
    crate::marks::switch_on(&h.ctx);
    for (id, group) in [(1, "Outside"), (2, "Holes")] {
        let params = serde_json::json!({ "toolpaths": [id], "group": group });
        h.run("group_toolpaths", &params).unwrap();
    }
    h.frame(vec![]);
    h.frame(vec![]);
    h
}

fn mark(h: &Harness, name: &str) -> egui::Pos2 {
    crate::marks::get(&h.ctx, name)
        .unwrap_or_else(|| panic!("{name} drawn"))
        .center()
}

/// Drags from `from` to `to` in small steps, as a hand does.
fn drag_slowly(h: &mut Harness, from: egui::Pos2, to: egui::Pos2) {
    h.frame(vec![Event::PointerMoved(from)]);
    h.press(from, true, Modifiers::NONE);
    for k in 1..=8_u8 {
        h.frame(vec![Event::PointerMoved(from.lerp(to, f32::from(k) / 8.0))]);
    }
    h.press(to, false, Modifiers::NONE);
    h.frame(vec![]);
}

#[test]
fn a_toolpath_dragged_by_its_grip_goes_just_before_the_row_it_is_dropped_on() {
    let mut h = named_toolpaths();
    crate::marks::switch_on(&h.ctx);
    h.frame(vec![]);
    let from = mark(&h, "toolpath_grip:Cut Out");
    let to = h.text_at("Pocket").unwrap();
    drag_slowly(&mut h, from, to);
    assert_eq!(toolpath_names(&h), ["Cut Out", "Pocket", "Drill"]);
}

#[test]
fn a_group_dragged_onto_another_groups_name_goes_inside_it_with_its_toolpaths() {
    let mut h = grouped_toolpaths();
    let from = mark(&h, "toolpath_grip:Holes");
    let to = *h.texts_at("Outside").last().unwrap();
    drag_slowly(&mut h, from, to);
    assert_eq!(
        h.doc().toolpath_group_parent("Holes").as_deref(),
        Some("Outside")
    );
    assert_eq!(
        h.doc().toolpaths[&EntityId(2)].group.as_deref(),
        Some("Holes"),
        "Drill stays in Holes"
    );
    // shown inside Outside: indented further, while the grips stay in one column
    h.frame(vec![]);
    let tick = |h: &Harness, g: &str| mark(h, &format!("toolpath_group_tick:{g}"));
    assert!(tick(&h, "Holes").x > tick(&h, "Outside").x + 10.0);
    let grip = |h: &Harness, g: &str| mark(h, &format!("toolpath_grip:{g}"));
    assert!((grip(&h, "Holes").x - grip(&h, "Outside").x).abs() < 0.5);
    h.invoke("undo");
    assert_eq!(h.doc().toolpath_group_parent("Holes"), None, "one Undo");
}

#[test]
fn a_groups_arrow_closes_it_to_its_name_and_count_without_an_undo_step() {
    let mut h = grouped_toolpaths();
    assert!(h.text_at("Pocket").is_some());
    let steps = h.undo_steps();
    let arrow = mark(&h, "toolpath_arrow:Outside");
    h.click(arrow, Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.text_at("Pocket").is_none(), "closed");
    assert!(h.text_at("1 toolpath").is_some(), "its count shows");
    assert!(h.doc().toolpath_groups_closed.contains("Outside"));
    assert_eq!(h.undo_steps(), steps, "not a change to undo");
    let arrow = mark(&h, "toolpath_arrow:Outside");
    h.click(arrow, Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.text_at("Pocket").is_some(), "open again");
}

#[test]
fn a_groups_tick_hides_everything_inside_it_in_one_undo_step() {
    let mut h = grouped_toolpaths();
    let tick = mark(&h, "toolpath_group_tick:Outside");
    h.click(tick, Modifiers::NONE);
    assert!(!h.doc().toolpaths[&EntityId(1)].visible);
    assert!(h.doc().toolpaths[&EntityId(2)].visible, "Holes untouched");
    h.invoke("undo");
    assert!(h.doc().toolpaths[&EntityId(1)].visible);
}

#[test]
fn group_with_a_group_picked_puts_it_inside_a_new_group() {
    let mut h = grouped_toolpaths();
    // the list's own "Holes" (another word on the page may say it too)
    let name = |h: &Harness, g: &str| *h.texts_at(g).last().expect("listed");
    h.click(name(&h, "Holes"), Modifiers::NONE);
    assert_eq!(h.selected_toolpath_group(), Some("Holes"));
    let group = h.text_at("Group").unwrap();
    h.click(group, Modifiers::NONE);
    let made = (h.doc().toolpath_group_parent("Holes")).expect("inside a new group");
    assert!(h.doc().toolpath_groups.contains(&made));
    // Ungroup takes that one level apart: Holes is back at the top
    h.frame(vec![]);
    h.click(name(&h, &made), Modifiers::NONE);
    let ungroup = h.text_at("Ungroup").unwrap();
    h.click(ungroup, Modifiers::NONE);
    assert_eq!(h.doc().toolpath_group_parent("Holes"), None);
}

/// The named toolpaths, the Cut Out a Rotary Roughing (it cuts only what is above it), each
/// cutting differently.
fn roughing_last() -> Harness {
    let mut h = named_toolpaths();
    let mut c = Change::builder(h.doc());
    for id in 1..=3u64 {
        let mut t = h.doc().toolpaths[&EntityId(id)].clone();
        t.moves = vec![cut_craft_core::toolpath::Move {
            kind: cut_craft_core::toolpath::MoveKind::Cut,
            #[expect(clippy::cast_precision_loss, reason = "a small id")]
            to: cut_craft_core::glam::DVec3::new(0.0, 0.0, -(id as f64)),
        }];
        if id == 3 {
            t.strategy = "rotary_roughing_toolpath".into();
        }
        c.replace_toolpath(EntityId(id), t);
    }
    h.given(&c.finish());
    h.frame(vec![]);
    h
}

fn reorder_first_two(h: &mut Harness) {
    h.arrange_toolpaths(crate::toolpath_order::Arranged {
        order: vec![EntityId(2), EntityId(1), EntityId(3)],
        groups: vec![],
        parents: vec![],
    });
    h.frame(vec![]);
    h.frame(vec![]);
}

#[test]
fn a_new_order_that_leaves_a_roughing_out_of_date_offers_to_put_it_back() {
    let mut h = roughing_last();
    reorder_first_two(&mut h);
    assert_eq!(toolpath_names(&h), ["Drill", "Pocket", "Cut Out"]);
    assert!(h.doc().toolpaths[&EntityId(3)].stale);
    assert!(
        h.text_at("Out of date after the new order").is_some(),
        "asked"
    );
    let back = h.text_at("Put it back").expect("the button");
    h.click(back, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(toolpath_names(&h), ["Pocket", "Drill", "Cut Out"]);
    assert!(!h.doc().toolpaths[&EntityId(3)].stale, "up to date again");
    assert!(h.text_at("Out of date after the new order").is_none());
    // kept: it stays out of date, nothing more asked
    reorder_first_two(&mut h);
    let keep = h.text_at("Keep the new order").expect("the button");
    h.click(keep, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(toolpath_names(&h), ["Drill", "Pocket", "Cut Out"]);
    assert!(h.doc().toolpaths[&EntityId(3)].stale);
    assert!(h.text_at("Out of date after the new order").is_none());
}

#[test]
fn the_advanced_rotary_hint_opens_options_with_the_setting_ringed() {
    let mut h = named_toolpaths();
    h.invoke("advanced_rotary_mode");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.options_window_open());
    assert!(crate::marks::spotlit(&h.ctx, "field:advanced_rotary"));
    assert!(h.text_at("Advanced rotary mode").is_some(), "on its page");
    // Options opened the usual way rings nothing
    h.close_options_window();
    h.invoke("options");
    assert!(!crate::marks::spotlit(&h.ctx, "field:advanced_rotary"));
}
