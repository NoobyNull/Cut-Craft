//! The toolpath list: its rows, menus, reordering and the toolpath summary.

use super::*;

#[test]
fn a_dragged_value_in_the_3d_settings_changes_the_component_as_it_goes() {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::domed();
    h.app.doc.selected_components = [EntityId(1)].into();
    h.app.orbit = view3d::Orbit::TOP;
    h.app.view = View::ThreeD;
    h.app.invoke(&h.ctx, "transform");
    h.frame(vec![]);
    h.frame(vec![]);
    let settings = h
        .app
        .handles_3d
        .iter()
        .find(|(k, _)| *k == Handle3d::Settings)
        .map(|(_, at)| *at)
        .expect("the editing icons");
    h.click(settings, Modifiers::NONE);
    h.frame(vec![]);
    let tilt = h.text_at("Tilt").expect("the settings popup");
    h.click(tilt, Modifiers::NONE);
    h.frame(vec![]);
    let angle = |h: &Harness| {
        let k = &h.app.doc.components[&EntityId(1)];
        k.shaping.tilt.map(|t| t.angle)
    };
    assert!(angle(&h).is_some(), "tilted");
    let undo_from = h.app.history.undo_depth();
    let held = h.text_at("Combine with other components ...").unwrap();
    h.press(held, true, Modifiers::NONE);
    for step in 1..=3 {
        let to = 10.0 * f64::from(step);
        let s = h.app.settings_3d.as_mut().unwrap();
        s.params.tilt.angle = to;
        s.dirty = true;
        h.frame(vec![]);
        assert_eq!(angle(&h), Some(to), "follows the drag: {step}");
    }
    h.press(held, false, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.app.history.undo_depth(), undo_from + 1, "one undo");
    h.app.invoke(&h.ctx, "undo");
    assert_ne!(angle(&h), Some(30.0), "the drag undone at once");
}

#[test]
fn the_components_tab_is_the_component_list() {
    let mut h = Harness::new();
    h.app.pages = Pages::design_layout();
    h.app.design_tab = DESIGN_TABS.iter().position(|&t| t == "Components").unwrap();
    h.frame(vec![]);
    assert!(h.text_at("No components yet").is_some());
}

#[test]
fn the_toolpath_list_shows_hides_edits_and_reorders_its_rows() {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::three_toolpaths();
    let mut c = Change::builder(&h.app.doc);
    for (id, name) in [(1, "Pocket"), (2, "Drill"), (3, "Cut Out")] {
        let mut t = h.app.doc.toolpaths[&EntityId(id)].clone();
        t.name = name.into();
        c.replace_toolpath(EntityId(id), t);
    }
    h.app.doc.apply(&c.finish()).unwrap();
    h.app.screen = Screen::Job;
    h.app.pages.set_pinned(Page::Toolpaths, true);
    h.frame(vec![]);
    h.frame(vec![]);
    let names = |h: &Harness| -> Vec<String> {
        h.app
            .doc
            .toolpaths
            .values()
            .map(|t| t.name.clone())
            .collect()
    };
    // the tick left of a name hides it
    let drill = h.text_at("Drill").unwrap();
    h.click(drill - egui::vec2(24.0, 0.0), Modifiers::NONE);
    assert!(!h.app.doc.toolpaths[&EntityId(2)].visible);
    // a click selects, the down arrow moves it below the next one
    h.click(drill, Modifiers::NONE);
    assert_eq!(h.app.selected_toolpath, Some(EntityId(2)));
    let down = h.text_at("Move Down").unwrap();
    h.click(down, Modifiers::NONE);
    assert_eq!(names(&h), ["Pocket", "Cut Out", "Drill"]);
    assert_eq!(
        h.app.selected_toolpath,
        Some(EntityId(3)),
        "it stays selected"
    );
    assert!(
        !h.app.doc.toolpaths[&EntityId(3)].visible,
        "hidden moves with it"
    );
    // a double-click opens its form
    h.frame(vec![]);
    let pocket = h.text_at("Pocket").unwrap();
    h.double_click(pocket);
    h.frame(vec![]);
    assert_eq!(h.app.selected_toolpath, Some(EntityId(1)));
    assert!(h.app.form.is_some(), "the toolpath's form is open");
}

#[test]
fn a_cut_list_imports_as_parts_and_nest_parts_opens_to_move_them() {
    let mut h = Harness::new();
    let dir = std::env::temp_dir().join(format!("cut-craft-cut-list-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("kitchen.csv");
    std::fs::write(&file, "Name,Length,Width,Qty\nSide,72,56,2\n").unwrap();
    h.app.import_cut_list(&file);
    assert_eq!(h.app.doc.vectors.len(), 2);
    assert_eq!(h.app.doc.part_names.values().next().unwrap(), "Side");
    let form = h.app.form.as_ref().expect("Nest Parts opens next");
    assert_eq!(form.id, "nest_parts");
    assert_eq!(form.params["remove_original_parts"], true);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_layers_list_arrows_move_the_active_layer_and_merge_visible_is_offered() {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::two_layers();
    let first = h.app.doc.layers[0].id;
    h.app.doc.current_layer = first;
    h.app.pages.set_pinned(Page::Design, true);
    h.app.design_tab = DESIGN_TABS.iter().position(|&t| t == "Layers").unwrap();
    h.frame(vec![]);
    h.frame(vec![]);
    let up = *h
        .texts_at("Move Up")
        .first()
        .expect("the arrows are over the list");
    h.click(up, Modifiers::NONE);
    assert_eq!(
        h.app.doc.layers.last().map(|l| l.id),
        Some(first),
        "now at the top"
    );
    let menu = gui::layer_menu(&h.app.doc, &h.app.doc.layers[0]);
    let merge = leaf(&menu, &["Merge Visible"]);
    assert!(merge.is_enabled, "another layer is visible");
    // layers with nothing on this sheet can be left out of the list
    let empty = h
        .app
        .doc
        .layers
        .iter()
        .find(|l| h.app.doc.vectors_on(l.id).next().is_none() && l.id != first)
        .map(|l| l.name.clone());
    if let Some(name) = empty {
        h.app.layers_on_sheet = true;
        h.frame(vec![]);
        assert!(h.text_at(&name).is_none(), "{name} is left out");
    }
}

#[test]
fn favourites_list_each_tool_on_its_own_even_from_one_flyout() {
    let mut h = Harness::new();
    h.app.pages.set_pinned(Page::Design, true);
    h.app.options.favourites.design = vec!["draw_circle".into(), "draw_ellipse".into()];
    h.app.options.favourites.design_only = true;
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
    let o = h.app.doc.job.area().min;
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
    h.app.invoke(&h.ctx, "pocket_toolpath");
    h.app.pages.set_pinned(Page::Toolpaths, true);
    let big = serde_json::to_value(h.app.tools.get("12 mm End Mill").unwrap()).unwrap();
    h.app.form.as_mut().unwrap().params["rest"] =
        serde_json::json!({ "after": [big], "minimum_detail": 0.0, "overlap": 0.0 });
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(
        h.text_at("Rest Machining").is_some(),
        "kept, so it can be changed"
    );
}

#[test]
fn the_toolpath_list_stays_under_every_toolpaths_tab() {
    use super::panels::{SAVE_FOR_MACHINE, SAVE_NEEDS};
    let mut h = Harness::new();
    h.app.pages.set_pinned(Page::Toolpaths, true);
    h.frame(vec![]);
    h.frame(vec![]);
    let tab = |name: &str| {
        gui::TOOLPATH_SUBTABS
            .iter()
            .position(|t| t.name == name)
            .unwrap()
    };
    // Make opens first, the job and its tools over the tabs, the list under them
    assert!(h.text_at("Job and Tools").is_some());
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
        assert_eq!(h.app.toolpath_subtab, tab(name));
        assert!(
            h.text_at("Toolpath List").is_some(),
            "the list under {name}"
        );
    }
    // Output: the big button, and what it waits for
    h.app.toolpath_subtab = tab("Output");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at(SAVE_NEEDS).is_some(), "says what to do first");
    assert!(h.text_at("Paperwork for the Job").is_some());
    h.click(h.text_at(SAVE_FOR_MACHINE).unwrap(), Modifiers::NONE);
    assert!(h.app.save_form.is_none(), "nothing to save yet");
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.invoke(&h.ctx, "profile_toolpath");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("profile_toolpath", &params).unwrap();
    h.app.form = None;
    h.app.toolpath_subtab = tab("Output");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Profile").is_some(), "the new toolpath, in sight");
    assert!(h.text_at(SAVE_NEEDS).is_none());
    h.click(h.text_at(SAVE_FOR_MACHINE).unwrap(), Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.app.save_form.is_some(), "the same Save Toolpaths form");
    // Preview: every preview a button of its own, none in a More… menu
    h.app.save_form = None;
    h.app.toolpath_subtab = tab("Preview");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Undo Last Preview").is_some());
    assert!(h.text_at("More…").is_none());
    // Analyse: the checks before cutting on the shown toolpaths
    h.app.toolpath_subtab = tab("Analyse");
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
    h.app.invoke(&h.ctx, &only);
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
    h.app.invoke(&h.ctx, "profile_toolpath");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("profile_toolpath", &params).unwrap();
    h.app.form = None;
    h.app.invoke(&h.ctx, "show_toolpaths_tab");
    h.frame(vec![]);
    h.frame(vec![]);
    let name = h.app.doc.toolpaths.values().next().unwrap().name.clone();
    let row = h.text_at(&name).expect("the toolpath's row");
    h.click(row, Modifiers::NONE);
    assert!(h.app.selected_toolpath.is_some(), "picked in the list");
    assert!(
        !h.app.doc.selection.is_empty(),
        "the rectangle is still selected"
    );
    h.frame(vec![key_with(Key::Delete, Modifiers::NONE)]);
    assert!(h.app.doc.toolpaths.is_empty(), "the picked toolpath went");
    assert_eq!(h.app.doc.vectors.len(), 1, "the drawing is kept");
    // a click in the drawing after that: Del deletes the vectors again
    h.app.invoke(&h.ctx, "profile_toolpath");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("profile_toolpath", &params).unwrap();
    h.app.form = None;
    h.frame(vec![]);
    let row = h.text_at(&name).expect("the toolpath's row");
    h.click(row, Modifiers::NONE);
    h.app.doc.selection = h.app.doc.vectors.keys().copied().collect();
    h.click(Pos2::new(640.0, 400.0), Modifiers::NONE);
    h.app.doc.selection = h.app.doc.vectors.keys().copied().collect();
    h.frame(vec![key_with(Key::Delete, Modifiers::NONE)]);
    assert_eq!(h.app.doc.toolpaths.len(), 1);
    assert!(h.app.doc.vectors.is_empty(), "Del is the drawing's again");
}

#[test]
fn ticking_a_toolpath_off_in_the_list_is_one_undo_step() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    let params = h
        .app
        .registry
        .get("drilling_toolpath")
        .unwrap()
        .initial_params(&h.app.doc);
    h.app.apply("drilling_toolpath", &params).unwrap();
    let (&id, t) = h.app.doc.toolpaths.iter().next().unwrap();
    let name = t.name.clone();
    h.app.invoke(&h.ctx, "show_toolpaths_tab");
    h.frame(vec![]);
    h.frame(vec![]);
    let at = h.text_at(&name).expect("the toolpath in the list");
    let depth = h.app.history.undo_depth();
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
    assert!(!h.app.doc.toolpaths[&id].visible, "hidden");
    assert_eq!(h.app.history.undo_depth(), depth + 1, "one step to undo");
    h.app.invoke(&h.ctx, "undo");
    assert!(h.app.doc.toolpaths[&id].visible, "one undo shows it again");
}

#[test]
fn a_smaller_recent_file_list_only_hides_the_rest_until_it_grows_again() {
    let mut h = Harness::new();
    h.app.options.general.recent_file_list_size = 9;
    h.app.recent = (0..9)
        .map(|i| std::path::PathBuf::from(format!("job{i}.ccjob")))
        .collect();
    h.app.invoke(&h.ctx, "options");
    h.frame(vec![]);
    h.frame(vec![]);
    let at = *h.texts_at("General Settings").last().expect("listed");
    h.click(at, Modifiers::NONE);
    h.frame(vec![]);
    let reset = h.text_at("Reset to defaults").expect("the page's reset");
    h.click(reset, Modifiers::NONE);
    h.frame(vec![]);
    click_button(&mut h, "Apply");
    assert_eq!(h.app.options.recent_count(), files::RECENT_COUNT);
    assert_eq!(h.app.recent_shown().len(), files::RECENT_COUNT);
    h.app.options.general.recent_file_list_size = 9;
    assert_eq!(h.app.recent_shown().len(), 9, "all nine back");
}

#[test]
fn an_open_edit_toolpath_form_follows_its_toolpath_through_a_reorder_and_its_undo() {
    let mut h = named_toolpaths();
    // the job put in as a new one is
    h.app.job_epoch += 1;
    h.frame(vec![]);
    h.app.selected_toolpath = Some(EntityId(1));
    h.app.edit_toolpath();
    assert_eq!(h.app.form.as_ref().unwrap().editing, Some(EntityId(1)));
    // the Pocket moved down below the Drill: it is now at id 2
    h.app
        .apply(
            "arrange_toolpaths",
            &serde_json::json!({ "order": [2, 1, 3] }),
        )
        .unwrap();
    h.frame(vec![]);
    assert_eq!(h.app.doc.toolpaths[&EntityId(2)].name, "Pocket");
    assert_eq!(
        h.app.form.as_ref().unwrap().editing,
        Some(EntityId(2)),
        "still edits the Pocket"
    );
    h.app.invoke(&h.ctx, "undo");
    h.frame(vec![]);
    assert_eq!(h.app.doc.toolpaths[&EntityId(1)].name, "Pocket");
    assert_eq!(h.app.form.as_ref().unwrap().editing, Some(EntityId(1)));
    h.app.invoke(&h.ctx, "redo");
    h.frame(vec![]);
    assert_eq!(h.app.form.as_ref().unwrap().editing, Some(EntityId(2)));
}

#[test]
fn cancel_on_a_form_keeps_a_reorder_made_in_the_toolpath_list_while_it_was_open() {
    let mut h = named_toolpaths();
    h.app.invoke(&h.ctx, "draw_circle");
    h.frame(vec![]);
    let circle = |x: f64| serde_json::json!({ "center": { "x": x, "y": 150.0 }, "radius": 20.0 });
    h.app.apply("draw_circle", &circle(100.0)).unwrap();
    // the Pocket moved down in the toolpath list beside the form
    h.app.selected_toolpath = Some(EntityId(1));
    h.frame(vec![]);
    let down = *h
        .texts_at("Move Down")
        .last()
        .expect("the list's Move Down");
    h.click(down, Modifiers::NONE);
    assert_eq!(h.app.doc.toolpaths[&EntityId(2)].name, "Pocket");
    assert!(h.app.form.is_some(), "the form is still open");
    h.app.apply("draw_circle", &circle(200.0)).unwrap();
    assert_eq!(h.app.doc.vectors.len(), 2);
    h.app.cancel_form();
    assert_eq!(
        h.app.doc.toolpaths[&EntityId(2)].name,
        "Pocket",
        "the reorder is kept"
    );
    assert_eq!(
        h.app.doc.vectors.len(),
        0,
        "what the form did before the reorder and after it is undone"
    );
    // one Undo takes back the reorder, as the list's own step
    h.app.invoke(&h.ctx, "undo");
    assert_eq!(h.app.doc.toolpaths[&EntityId(1)].name, "Pocket");
    assert_eq!(h.app.doc.vectors.len(), 0);
}

#[test]
fn cancel_with_only_a_list_change_since_the_form_opened_has_nothing_to_take_back() {
    let mut h = named_toolpaths();
    h.app.invoke(&h.ctx, "draw_circle");
    h.frame(vec![]);
    h.app.selected_toolpath = Some(EntityId(1));
    h.frame(vec![]);
    let down = *h
        .texts_at("Move Down")
        .last()
        .expect("the list's Move Down");
    h.click(down, Modifiers::NONE);
    h.frame(vec![]);
    let form = h.app.form.as_ref().unwrap();
    assert_eq!(form.list_steps.len(), 1, "the list's step is noted");
    assert_eq!(
        h.app.history.undo_depth(),
        form.undo_from + 1,
        "only the list's step since it opened"
    );
    h.app.cancel_form();
    assert_eq!(h.app.doc.toolpaths[&EntityId(2)].name, "Pocket");
}
