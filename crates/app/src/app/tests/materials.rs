//! Materials: the Materials Library, Material Setup and the uncut material.

use super::*;

#[test]
fn hiding_or_locking_a_layer_keeps_its_material() {
    let mut h = Harness::new();
    let mut layer = cmd_layer_properties::Params::from(h.doc().layers[0].clone());
    layer.material = "Oak".into();
    h.run("layer_properties", &serde_json::to_value(&layer).unwrap())
        .unwrap();
    h.click_layer(LayerId(1), "toggle_visible");
    h.click_layer(LayerId(1), "toggle_locked");
    let l = &h.doc().layers[0];
    assert!(!l.visible && l.locked);
    assert_eq!(
        l.material, "Oak",
        "the eye and lock leave the material alone"
    );
}

#[test]
fn a_tool_chosen_for_a_toolpath_cuts_with_the_active_material() {
    let mut h = Harness::new();
    let mut tool = h.tool_database().get("3 mm End Mill").unwrap().clone();
    tool.set_cutting(cut_craft_core::toolpath::CuttingData {
        material: "Oak".into(),
        feed_rate: 450.0,
        ..tool.cutting("")
    });
    h.given_tool(tool);
    h.given_active_material("Oak");
    h.invoke("pocket_toolpath");
    h.choose_tool("3 mm End Mill");
    assert_eq!(h.form_value("tool")["feed_rate"], 450.0);
}

#[test]
fn a_tool_chosen_for_a_toolpath_cuts_the_hardest_material_its_vectors_are_on() {
    let mut h = Harness::new();
    let mut tool = h.tool_database().get("3 mm End Mill").unwrap().clone();
    for (material, feed_rate) in [("Pine", 1500.0), ("Hard Maple", 700.0)] {
        tool.set_cutting(cut_craft_core::toolpath::CuttingData {
            material: material.into(),
            feed_rate,
            ..tool.cutting("")
        });
    }
    h.given_tool(tool);
    h.given_active_material("Oak");
    // the job and its layer name their material: it wins over the active material
    h.given_document(|doc| {
        doc.sheets[0].material = "Pine".into();
        doc.layers[0].material = "Pine".into();
    });
    let feed = |h: &mut Harness| {
        h.invoke("pocket_toolpath");
        h.choose_tool("3 mm End Mill");
        h.form_value("tool")["feed_rate"].clone()
    };
    assert_eq!(feed(&mut h), 1500.0);
    // one selected vector on a layer of maple: the harder material's feeds
    h.rectangle(0.0, 0.0, 20.0, 20.0);
    let id = *h.doc().vectors.keys().next_back().unwrap();
    let maple = cut_craft_core::document::LayerId(9);
    let mut layer = cut_craft_core::document::Layer::new(maple, "Maple");
    layer.material = "Hard Maple".into();
    h.given_document(|doc| {
        doc.layers.push(layer);
        doc.vector_layers.insert(id, maple);
    });
    h.select([id]);
    assert_eq!(feed(&mut h), 700.0);
}

#[test]
fn a_model_imported_past_the_material_edge_shows_that_part_as_a_red_ghost() {
    let mut h = Harness::new();
    let file = pyramid_stl("past-edge");
    h.import(&file);
    h.frame(vec![]);
    assert!(
        h.importing_beyond().is_some_and(<[_]>::is_empty),
        "centred, nothing is past the edge"
    );
    // half of the 20 mm pyramid past the material's right edge
    let area = h.doc().job.area();
    h.set_form_value("/placing/center", Value::Bool(false));
    h.set_form_value(
        "/placing/position",
        serde_json::json!({ "x": area.max.x - 10.0, "y": area.min.y + 10.0 }),
    );
    h.frame(vec![]);
    let beyond = h.importing_beyond().unwrap();
    assert!(!beyond.is_empty(), "the part past the edge is kept to show");
    assert!(
        beyond
            .iter()
            .flat_map(|(t, _)| t)
            .all(|v| v.x >= area.max.x - 1e-6),
        "only what is past it"
    );
    let model = h.importing_model().unwrap();
    assert!(
        model
            .iter()
            .flat_map(|(t, _)| t)
            .all(|v| v.x <= area.max.x + 1e-6),
        "the model drawn as usual stops at the edge"
    );
    // past the preview limit the component shows over the material, the ghost still past it
    h.given_options(|o| o.import.preview_faces = 4);
    h.touch_form();
    h.frame(vec![]);
    assert!(!h.import_drawn_whole());
    assert!(
        h.importing_beyond()
            .is_some_and(|b| !b.is_empty() && b.len() <= 4),
        "the ghost keeps to the limit too: {:?}",
        h.importing_beyond().map(<[_]>::len)
    );
    let _ = std::fs::remove_file(file);
}

#[test]
fn scaling_an_import_to_fit_keeps_it_where_it_was_aligned_in_the_material() {
    let mut h = Harness::new();
    let file = pyramid_stl("scale-aligned");
    h.import(&file);
    h.frame(vec![]);
    h.frame(vec![]);
    let area = h.doc().job.area();
    let cell = h.doc().job.model_cell();
    let model = |h: &Harness| {
        let k = h.doc().components.values().next().unwrap();
        (k.extent(), k.height_range().unwrap())
    };
    let fit = |h: &mut Harness| {
        let fitted = h.form_value("fit_material").as_bool().unwrap();
        h.set_form_value("/fit_material", Value::Bool(!fitted));
        // right first time: no stale frame with the model sunk or out of place
        h.frame(vec![]);
    };
    // imported centred at 1:1, then scaled to fit: still centred, from the material's corner,
    // its bottom still on the zero plane
    fit(&mut h);
    let (e, (_, hi)) = model(&h);
    assert!(e.size().x > 30.0, "scaled up: {e:?}");
    assert!(
        (e.center() - area.center()).length() < 2.0 * cell,
        "{e:?} in {area:?}"
    );
    // the 20 long, 5 tall pyramid grown as much in height: all of it above the zero plane
    let tall = e.size().x / 20.0 * 5.0;
    assert!(
        (hi - tall).abs() < 0.5,
        "not sunk under the zero plane: {hi} of {tall}"
    );
    // back to 1:1, aligned to the right, then scaled to fit: still against the right side
    fit(&mut h);
    h.set_form_value("/placing/align/across", Value::from(1.0));
    h.frame(vec![]);
    let (e, _) = model(&h);
    assert!(
        e.size().x < 21.0 && (e.max.x - area.max.x).abs() < 2.0 * cell,
        "{e:?}"
    );
    fit(&mut h);
    let (e, _) = model(&h);
    assert!(
        e.size().x > 30.0 && (e.max.x - area.max.x).abs() < 2.0 * cell,
        "{e:?}"
    );
    let shown = h.form_value("placing")["position"]["x"].as_f64();
    assert!(
        (shown.unwrap() - e.min.x).abs() < 2.0 * cell,
        "the number says where it is"
    );
    // a position typed leaves it there when it is resized
    fit(&mut h);
    h.set_form_value("/placing/position/x", Value::from(area.min.x + 3.0));
    h.frame(vec![]);
    fit(&mut h);
    let (e, _) = model(&h);
    assert!((e.min.x - (area.min.x + 3.0)).abs() < 2.0 * cell, "{e:?}");
    let _ = std::fs::remove_file(file);
}

#[test]
fn a_rotary_preview_plays_the_file_turning_the_material_with_a_in_step_with_its_controls() {
    use cut_craft_core::document::{Along, Job};
    let mut h = Harness::new();
    h.given_document(|doc| doc.job = Job::rotary(100.0, 40.0, Along::X));
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.invoke("profile_toolpath");
    let params = h.form_settings().clone();
    h.run("profile_toolpath", &params).unwrap();
    h.close_form();
    h.given_animated_preview(true);
    h.invoke("preview_all_toolpaths");
    let uncut = {
        let turning = h.turning().expect("the file played turning");
        turning.material().count()
    };
    h.frame(vec![]);
    h.frame(vec![]);
    // in step with the Preview Control: as far through the file as it is through the job
    let fraction = |h: &Harness| {
        let p = h.playback().unwrap();
        let r = h.turning().unwrap();
        (p.at() / p.length(), r.at() / r.length())
    };
    // playing, it cuts toward the same point each frame, never past it (a busy computer
    // may leave it a frame behind)
    let (job, file) = fraction(&h);
    assert!(
        job > 0.0 && file > 0.0 && file <= job + 1e-9,
        "{job} {file}"
    );
    h.click(h.text_at("Pause").unwrap(), Modifiers::NONE);
    let caught_up = |h: &mut Harness| {
        for _ in 0..50 {
            h.frame(vec![]);
            let (job, file) = fraction(h);
            if (job - file).abs() < 1e-9 {
                return true;
            }
        }
        false
    };
    assert!(
        caught_up(&mut h),
        "paused, it gets there: {:?}",
        fraction(&h)
    );
    let progress = h.text_at("Progress").expect("a Progress slider");
    let from = progress + egui::vec2(80.0, 0.0);
    h.drag(from, from + egui::vec2(200.0, 0.0));
    assert!(caught_up(&mut h), "dragged, it follows: {:?}", fraction(&h));
    h.set_playback_speed(1000.0);
    h.click(h.text_at("Run").unwrap(), Modifiers::NONE);
    for _ in 0..600 {
        if h.playback().is_none() {
            break;
        }
        h.frame(vec![]);
    }
    // the material catches up a frame's budget at a time (a slow computer takes a few)
    for _ in 0..200 {
        h.frame(vec![]);
        if h.turning().is_some_and(cut_craft_rotary::Replay::finished) {
            break;
        }
    }
    let turning = h.turning().expect("kept with the controls");
    assert!(turning.finished(), "played through");
    assert!(turning.material().count() < uncut, "it cut");
    h.click(h.text_at("Close").unwrap(), Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.turning().is_none(), "gone with the controls");
    // a flat job plays as before
    h.given_document(|doc| doc.job = Job::default());
    h.invoke("preview_all_toolpaths");
    assert!(h.playback().is_some() && h.turning().is_none());
}

#[test]
fn toolpaths_preview_colors_opens_the_material_colours() {
    let mut h = Harness::new();
    let item = (h.menu_bar().items())
        .into_iter()
        .find(|i| i.name == "Preview Colors...")
        .map(|i| i.action.clone())
        .expect("Toolpaths ▸ Preview Colors...");
    h.invoke(&item);
    assert_eq!(h.form_id(), Some("materials"));
    assert_eq!(h.form_page(), Page::Toolpaths, "beside Material Setup");
    h.frame(vec![]);
    assert!(
        h.text_at("Rest Machining").is_none(),
        "it cuts nothing, so nothing to rest machine"
    );
    let walnut = serde_json::json!({ "material": "walnut" });
    h.run("materials", &walnut).unwrap();
    assert_ne!(
        h.doc().job.colour,
        cut_craft_core::document::Job::default().colour
    );
}

#[test]
fn materials_shows_its_form_in_a_job_and_waits_for_one_on_the_start_page() {
    let mut h = Harness::new();
    h.frame(vec![]);
    h.invoke("materials");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(
        h.text_at("Pine").is_some(),
        "the Toolpaths page flies out with it"
    );
    let start = Harness::at_start();
    assert!(!start.available("materials"), "no job to colour");
    assert!(
        !start.available("wrapped_fluting_layout"),
        "nor for a gadget"
    );
}

#[test]
fn the_materials_library_says_when_changes_wait_and_apply_keeps_them() {
    let mut h = Harness::new();
    h.open_materials_library();
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at(super::options_apply::ALL_APPLIED).is_some());
    let count = h.materials_kept();
    h.new_material_in_library();
    h.frame(vec![]);
    assert!(h.text_at(super::options_apply::NOT_APPLIED).is_some());
    assert_eq!(h.materials_kept(), count, "not kept yet");
    click_button(&mut h, "Apply");
    assert_eq!(h.materials_kept(), count + 1, "kept");
    h.frame(vec![]);
    assert!(h.text_at(super::options_apply::ALL_APPLIED).is_some());
}

#[test]
fn the_preview_paints_machined_areas_in_the_materials_a_fill_or_each_toolpaths_colour() {
    let mut h = Harness::new();
    h.given_animated_preview(false);
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    for id in ["drilling_toolpath", "profile_toolpath"] {
        h.invoke(id);
        let params = h.form_settings().clone();
        h.run(id, &params).unwrap();
    }
    let ids: Vec<EntityId> = h.doc().toolpaths.keys().copied().collect();
    h.invoke("preview_all_toolpaths");
    let paints = |h: &mut Harness| -> std::collections::BTreeSet<u32> {
        h.surface_3d().unwrap().paint.iter().copied().collect()
    };
    let packed = |[r, g, b]: [u8; 3]| u32::from_be_bytes([1, r, g, b]);
    assert!(
        paints(&mut h).is_empty(),
        "the material's colour paints nothing"
    );
    // Toolpaths ▸ Preview Colors: one fill colour over every machined area
    let mut p = h.opening_settings("materials");
    p["machined_area"] = "global_fill_color".into();
    p["fill_colour"] = serde_json::json!([0, 0, 255]);
    h.run("materials", &p).unwrap();
    assert_eq!(paints(&mut h), [0, packed([0, 0, 255])].into());
    // each toolpath's own colour: the drilling given red, the profile with no fill
    p["machined_area"] = "toolpath_color".into();
    h.run("materials", &p).unwrap();
    h.frame(vec![]);
    let colour = |id: EntityId, preview: serde_json::Value| serde_json::json!({ "toolpath": id, "name": "t", "visible": true, "preview": preview });
    h.run(
        "toolpath_properties",
        &colour(ids[0], serde_json::json!({ "colour": [255, 0, 0] })),
    )
    .unwrap();
    h.run("toolpath_properties", &colour(ids[1], "no_fill".into()))
        .unwrap();
    assert_eq!(paints(&mut h), [0, packed([255, 0, 0])].into());
    assert!(
        h.doc().toolpaths[&ids[1]].preview_colour(1).is_none(),
        "no fill: the profile shows the material"
    );
}

#[test]
fn a_toolpath_made_with_cut_right_through_reads_as_the_materials_thickness() {
    let mut h = profile_form();
    let thickness = h.doc().job.thickness;
    h.set_form_value("/through", true.into());
    h.frame(vec![]);
    h.frame(vec![]);
    let params = h.form_settings();
    assert_eq!(params["depth"].as_f64(), Some(thickness));
    assert_eq!(params["through"], false, "an ordinary depth to change");
}

#[test]
fn the_texture_form_says_when_it_will_texture_the_whole_material() {
    use cut_craft_core::document::VectorCount;
    let filled = serde_json::json!({ "use_selected_vectors_as_pattern": false });
    let note = |closed, open| {
        form_panel::texture_area_note("texture_toolpath", &filled, VectorCount { closed, open })
    };
    assert!(note(0, 0).is_some_and(|n| n.contains("whole material")));
    let open_only = note(0, 2).unwrap();
    assert!(
        open_only.contains("whole material") && open_only.contains("open"),
        "{open_only}"
    );
    assert_eq!(note(1, 2), None, "the closed one is textured");
    let pattern = serde_json::json!({ "use_selected_vectors_as_pattern": true });
    assert_eq!(
        form_panel::texture_area_note(
            "texture_toolpath",
            &pattern,
            VectorCount { closed: 0, open: 0 }
        ),
        None
    );
}

#[test]
fn a_new_job_shows_its_uncut_material_in_the_3d_view() {
    let mut h = Harness::new();
    h.show_view(View::ThreeD);
    h.frame(vec![]);
    assert!(
        !h.texts().iter().any(|t| t.contains("Nothing in 3D yet")),
        "no empty view to fill first"
    );
    let _drawn = h.view_3d_area();
    assert!(h.showing_uncut_material());
    let job = h.doc().job;
    let block = h.surface_3d().expect("the material");
    assert!(
        block.heights.iter().all(|&z| z == 0.0),
        "uncut: its whole top at the material's top"
    );
    assert_eq!(block.bottom, -job.thickness, "as thick as Job Setup says");
    assert_eq!(block.origin, job.area().min);
    let across = f64::from(u32::try_from(block.columns).unwrap()) * block.cell;
    assert!(
        (across - job.width).abs() <= block.cell,
        "as wide as the job"
    );
}

#[test]
fn the_uncut_material_follows_job_setup_and_the_model_sits_on_it() {
    let mut h = Harness::new();
    h.show_view(View::ThreeD);
    h.frame(vec![]);
    h.given_document(|doc| doc.job.thickness = 30.0);
    let block = h.surface_3d().unwrap();
    assert_eq!(block.bottom, -30.0, "a new thickness is shown at once");
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    let shape = h.default_settings("create_shape");
    h.run("create_shape", &shape).unwrap();
    h.frame(vec![]);
    assert!(!h.showing_uncut_material(), "there is a model now");
    assert!(h.model_on_material());
    let model = h.surface_3d().unwrap();
    let top = model
        .heights
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    // Material Setup starts with Gap Above Model 0: the model's top flush with the top
    assert!(
        top.abs() < 1e-9,
        "the model's top at the material's top: {top}"
    );
    let off = model
        .at(cut_craft_core::geometry::Point::new(
            job_corner(&h).x + 1.0,
            job_corner(&h).y + 1.0,
        ))
        .unwrap();
    assert!(
        off < 0.0,
        "round the model, cut down to its zero plane: {off}"
    );
    assert!(model.heights.iter().all(|z| !z.is_nan()), "the whole block");
    assert_eq!(model.bottom, -30.0);
}

#[test]
fn material_setups_model_position_moves_the_model_in_the_3d_view_on_apply() {
    use cut_craft_core::document::{GapFrom, ModelPosition};
    let mut h = Harness::new();
    h.show_view(View::ThreeD);
    h.given_document(|doc| doc.job.thickness = 30.0);
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    let shape = h.default_settings("create_shape");
    h.run("create_shape", &shape).unwrap();
    h.frame(vec![]);
    let range = |h: &mut Harness| {
        let m = h.surface_3d().unwrap();
        let lo = m.heights.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = m.heights.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        (lo, hi)
    };
    let (lo, hi) = range(&mut h);
    let tall = hi - lo;
    assert!(tall > 0.0 && tall < 30.0, "a model that fits: {tall}");
    // Gap Below Model 0: its bottom on the material's bottom
    let mut p = h.opening_settings("material_setup");
    p["position"] = serde_json::to_value(ModelPosition {
        from: GapFrom::Below,
        gap: 0.0,
    })
    .unwrap();
    h.run("material_setup", &p).unwrap();
    h.frame(vec![]);
    let (lo, hi) = range(&mut h);
    assert!((lo + 30.0).abs() < 1e-6, "bottom on the bottom: {lo}");
    assert!((hi - lo - tall).abs() < 1e-6, "the same model, moved");
    // 2 mm under the top
    p["position"] = serde_json::to_value(ModelPosition {
        from: GapFrom::Above,
        gap: 2.0,
    })
    .unwrap();
    h.run("material_setup", &p).unwrap();
    h.frame(vec![]);
    let (_, hi) = range(&mut h);
    assert!((hi + 2.0).abs() < 1e-6, "top 2 mm down: {hi}");
}

#[test]
fn the_first_click_on_the_toolpaths_page_opens_material_setup_once_per_job() {
    let mut h = named_toolpaths();
    h.given_material_not_asked();
    assert!(!h.doc().material_checked);
    h.click(h.text_at("Drill").unwrap(), Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.form_id(), Some("material_setup"), "the material first");
    assert!(h.doc().material_checked);
    h.close_form();
    h.frame(vec![]);
    h.click(h.text_at("Drill").unwrap(), Modifiers::NONE);
    assert!(h.form_id().is_none(), "asked once");
    assert_eq!(h.selected_toolpath(), Some(EntityId(2)));
}

#[test]
fn material_setup_hides_the_thickness_on_a_rotary_job() {
    use cut_craft_core::document::{Along, Job};
    let mut h = Harness::new();
    h.invoke("material_setup");
    for _ in 0..3 {
        h.frame(vec![]);
    }
    assert!(h.text_at("Material Thickness").is_some(), "a flat job's");
    h.close_form();
    h.given_document(|doc| doc.job = Job::rotary(300.0, 50.0, Along::X));
    h.invoke("material_setup");
    for _ in 0..3 {
        h.frame(vec![]);
    }
    assert!(h.text_at("Material Thickness").is_none());
}

#[test]
fn preview_all_played_starts_again_from_uncut_material() {
    let mut h = playing_preview();
    h.click(h.text_at("Stop").unwrap(), Modifiers::NONE);
    // an old cut, far from the square's profile
    let top = h.previewed_height();
    h.given_previewed_height(top - 5.0);
    h.invoke("preview_all_toolpaths");
    let playing = h.playback().expect("it plays");
    assert_eq!(playing.material().heights[0], top, "old cuts gone");
    // Preview of one toolpath still cuts into what is there
    h.finish_playback();
    h.given_previewed_height(top - 5.0);
    let id = *h.doc().toolpaths.keys().next().unwrap();
    h.preview_toolpath(id);
    let playing = h.playback().expect("it plays");
    assert_eq!(playing.material().heights[0], top - 5.0, "cut into");
}

#[test]
fn previewing_a_group_played_starts_from_uncut_material() {
    let mut h = playing_preview();
    h.click(h.text_at("Stop").unwrap(), Modifiers::NONE);
    let top = h.previewed_height();
    h.given_previewed_height(top - 5.0);
    let ids: Vec<_> = h.doc().toolpaths.keys().copied().collect();
    h.preview_toolpaths(&ids);
    let playing = h.playback().expect("it plays");
    assert_eq!(playing.material().heights[0], top, "old cuts gone");
}

#[test]
fn material_setup_shows_both_gaps_and_says_in_red_when_the_model_sticks_out() {
    use cut_craft_core::document::{GapFrom, ModelPosition};
    let mut h = Harness::new();
    h.given_document(|doc| doc.job.thickness = 20.0);
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    let shape = h.default_settings("create_shape");
    h.run("create_shape", &shape).unwrap();
    h.invoke("material_setup");
    for _ in 0..3 {
        h.frame(vec![]);
    }
    let tall = h.model_height();
    assert!(tall > 0.0 && tall < 20.0, "measured: {tall}");
    assert!(
        h.texts()
            .iter()
            .any(|t| t.starts_with("Gap above the model:")),
        "both gaps said under the setting"
    );
    assert!(!h.texts().iter().any(|t| t.starts_with("⚠ The model")));
    // a gap below past the room left: kept, and said in red at the foot
    let below = ModelPosition {
        from: GapFrom::Below,
        gap: 20.0 - tall + 1.0,
    };
    h.set_form_value("/position", serde_json::to_value(below).unwrap());
    h.frame(vec![]);
    assert!(
        h.texts()
            .iter()
            .any(|t| t.starts_with("⚠ The model sticks 1.0 mm out of the top")),
        "{:?}",
        h.texts()
    );
}

#[test]
fn setup_material_takes_job_setups_place_in_file_operations() {
    let toolbar = gui::Toolbar::new();
    let file = toolbar
        .all_design_groups()
        .into_iter()
        .find(|g| g.name == "File Operations")
        .expect("File Operations");
    assert_eq!(
        file.tools,
        [
            "new",
            "open",
            "save",
            "export_vectors",
            "paste_in_place",
            "open_folder",
            "setup_material",
            "notes"
        ],
        "in Job Setup's place, so nothing moves"
    );
    assert_eq!(
        gui::built_in_label("setup_material"),
        Some("Setup Material")
    );
    assert!(icon_set().contains("file/setup_material.png"));
}

#[test]
fn setup_material_shows_the_kind_and_size_with_the_rest_one_link_away() {
    let mut h = Harness::new();
    h.invoke("setup_material");
    h.frame(vec![]);
    assert_eq!(h.form_id(), Some("job_setup"), "Setup Material's form");
    assert_eq!(h.form_label(), "Setup Material");
    let keys = h.form_fields();
    assert!(
        keys.iter().all(|k| setup_material::BRIEF.contains(k)),
        "{keys:?}"
    );
    assert!(keys.contains(&"job_type") && keys.contains(&"size"));
    assert!(
        h.text_at("600 × 400 × 20 mm, single sided, Z zero on top, X0 Y0 bottom left")
            .is_some()
    );
    let more = h
        .text_at(setup_material::MORE)
        .expect("the link to the rest");
    h.click(more, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.form_label(), "Job Setup", "still open");
    let keys = h.form_fields();
    assert!(keys.contains(&"z_zero") && keys.contains(&"datum") && keys.contains(&"home"));
    // a new size, applied, is this job's
    h.set_form_value("/size/width", 900.0.into());
    let params = h.form_settings().clone();
    h.calculate_checked("job_setup", &params);
    assert_eq!(h.doc().job.width, 900.0);
    assert_eq!(h.options().default_job, None, "new jobs are not changed");
}

/// While Import 3D Model is open the model sits on its zero plane at 0, and the material's
/// outline is drawn from 0 up to its thickness: the arrows go on that outline's top, or on
/// its bottom for Machine Bed, never off the outline.
#[test]
fn the_3d_origin_sits_on_the_material_outline_while_a_model_is_imported() {
    use cut_craft_core::document::ZZero;
    let mut h = Harness::new();
    h.given_document(|doc| doc.job.thickness = 18.0);
    let file = pyramid_stl("origin-import");
    h.import(&file);
    h.frame(vec![]);
    assert!(h.importing_whole(), "the model is shown whole");
    assert!(
        (h.origin_height_3d() - 18.0).abs() < 1e-9,
        "Material Surface: the outline's top, {}",
        h.origin_height_3d()
    );
    h.given_document(|doc| doc.job.z_zero = ZZero::MachineBed);
    assert!(
        h.origin_height_3d().abs() < 1e-9,
        "Machine Bed: the outline's bottom, {}",
        h.origin_height_3d()
    );
    // the form closed, the material's top is back at 0
    h.close_form();
    assert!((h.origin_height_3d() + 18.0).abs() < 1e-9, "the bed");
    h.given_document(|doc| doc.job.z_zero = ZZero::MaterialSurface);
    assert!(h.origin_height_3d().abs() < 1e-9, "the top");
}

/// A model past Options ▸ Model preview faces is shown as its component, only its faces out
/// of reach drawn over it: still on its modeling plane at 0 in the material's outline, as a
/// model shown whole is, never sunk into the block under the plane, the outline and the red
/// marks, with the arrows under it.
#[test]
fn a_big_model_being_imported_sits_on_the_modeling_plane_with_the_origin() {
    use cut_craft_core::document::ZZero;
    let mut h = Harness::new();
    h.given_document(|doc| doc.job.thickness = 18.0);
    h.given_document(|doc| doc.job.z_zero = ZZero::MachineBed);
    h.given_options(|o| o.import.preview_faces = 4);
    let file = pyramid_stl("big-import");
    h.import(&file);
    h.frame(vec![]);
    assert!(!h.importing_whole(), "past the limit: the component shows");
    assert!(!h.model_on_material(), "not seated in the block");
    assert!(
        h.origin_height_3d().abs() < 1e-9,
        "Machine Bed: on the modeling plane, the outline's bottom, {}",
        h.origin_height_3d()
    );
    let surface = h.surface_3d().expect("a model");
    let (lo, hi) = surface
        .heights
        .iter()
        .filter(|z| z.is_finite())
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &z| {
            (lo.min(z), hi.max(z))
        });
    assert!(
        lo > -1e-6 && hi > 1e-6,
        "on the plane, up from it: {lo}..{hi}"
    );
    h.given_document(|doc| doc.job.z_zero = ZZero::MaterialSurface);
    assert!(
        (h.origin_height_3d() - 18.0).abs() < 1e-9,
        "Material Surface: the outline's top"
    );
    let _ = std::fs::remove_file(file);
}
