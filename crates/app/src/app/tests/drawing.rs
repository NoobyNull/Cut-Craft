//! The drawing tools and their forms: shapes, embellishments, fills and importing into shapes.

use super::*;

#[test]
fn a_tool_in_other_units_leaves_the_form_in_the_jobs_units() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "pocket_toolpath");
    // an inch bit picked in a mm job: the bit's units are its own, not the form's
    h.app.form.as_mut().unwrap().params["tool"]["units"] = Value::from("Inch");
    // the Toolpaths page flies out unseen on its first frame
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Pocket Toolpath").is_some(), "the form is drawn");
    let texts: Vec<String> = h
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(t) => Some(t.galley.text().to_owned()),
            _ => None,
        })
        .collect();
    assert!(
        texts.iter().any(|t| t.starts_with("6 mm end mill")),
        "the bit reads in the job's mm: {texts:?}"
    );
    assert!(
        !texts.iter().any(|t| t.trim() == "in"),
        "no field of a mm job's form turns to inches: {texts:?}"
    );
}

#[test]
fn a_rail_rows_menu_adds_a_favourite_and_the_star_lists_them() {
    let mut h = Harness::new();
    h.app.options.general.layout = crate::options::WindowLayout::Rail;
    let menu = h.app.rail_menu("draw_circle");
    assert_eq!(menu[0].name, "Add to Favourites");
    h.app.invoke(&h.ctx, &menu[0].action);
    let menu = h.app.rail_menu("pocket_toolpath");
    h.app.invoke(&h.ctx, &menu[0].action);
    assert_eq!(h.app.options.favourites.design, ["draw_circle"]);
    assert_eq!(h.app.options.favourites.toolpaths, ["pocket_toolpath"]);
    assert_eq!(
        h.app.rail_menu("draw_circle")[0].name,
        "Remove from Favourites"
    );
    h.frame(vec![]);
    let star = h.text_at("★").expect("the Rail's favourites button");
    h.press(star, true, Modifiers::NONE);
    h.press(star, false, Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.app.rail_favourites);
    for name in ["Favourites", "Draw Circle", "Pocket Toolpath"] {
        assert!(h.text_at(name).is_some(), "{name}");
    }
    assert!(h.text_at("Draw Rectangle").is_none(), "only the favourites");
}

#[test]
fn a_drawn_shape_changes_with_its_form_until_it_is_let_go() {
    use cut_craft_core::geometry::Segment;
    let arcs = |app: &CutCraftApp| {
        let only: Vec<_> = app.doc.vectors.values().collect();
        assert_eq!(only.len(), 1, "one shape, not a second on top");
        only[0]
            .segments
            .iter()
            .filter(|s| matches!(s, Segment::Arc { .. }))
            .count()
    };
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "draw_rectangle");
    h.frame(vec![]);
    let (from, to) = (h.at(100.0, 100.0), h.at(200.0, 160.0));
    h.drag(from, to);
    assert_eq!(arcs(&h.app), 0, "drawn square");
    // issue #2: choosing rounded corners afterwards changes the rectangle just drawn, as
    // soon as it is chosen (issue #3), and Apply has nothing more to do
    let rounded = h.text_at("Rounded").expect("the corner choices");
    h.click(rounded, Modifiers::NONE);
    assert_eq!(arcs(&h.app), 4, "the same rectangle, rounded");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply_form("draw_rectangle", &params).unwrap();
    assert_eq!(arcs(&h.app), 4);
    let rect = h.app.doc.vectors.values().next().unwrap().bounds();
    // drawn from screen pixels: the same to a hundredth of a millimetre
    assert!((rect.size().x - 100.0).abs() < 1e-2, "same size: {rect:?}");
    h.app.invoke(&h.ctx, "undo");
    assert_eq!(arcs(&h.app), 0, "one undo puts the square back");
    // once let go, Apply draws a new shape
    h.app.invoke(&h.ctx, "redo");
    h.app.doc.selection.clear();
    h.app.apply_form("draw_rectangle", &params).unwrap();
    assert_eq!(h.app.doc.vectors.len(), 2);
}

#[test]
fn reshaping_a_made_shape_by_hand_stops_it_being_remade_from_its_settings() {
    // sculpting or embossing a Create Shape dome and then pressing E would rebuild it from
    // its outline and throw the work away: once its heights are edited, it is just a model
    for tool in [
        "emboss",
        "smooth_selected_components",
        "replace_below",
        "sculpt",
    ] {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        let shape = h.app.registry.get("create_shape").unwrap().default_params();
        h.app.apply("create_shape", &shape).unwrap();
        let id = *h.app.doc.components.keys().next().unwrap();
        h.app.doc.selection.clear();
        h.app.doc.selected_components = [id].into_iter().collect();
        assert_eq!(
            h.app.made_to_edit(),
            Some("create_shape"),
            "{tool}: made here"
        );
        let mut params = h.app.registry.get(tool).unwrap().default_params();
        if tool == "sculpt" {
            params["points"] = serde_json::json!([{ "x": 30.0, "y": 30.0 }]);
        }
        if tool == "replace_below" {
            params["below"] = 0.5.into();
        }
        h.app.apply(tool, &params).unwrap();
        assert!(
            h.app.doc.components.values().all(|k| k.shape.is_none()),
            "{tool} leaves a model, not a recipe to remake"
        );
        let ids: Vec<_> = h.app.doc.components.keys().copied().collect();
        h.app.doc.selected_components = ids.into_iter().collect();
        assert_eq!(h.app.made_to_edit(), None, "{tool}: E no longer remakes it");
    }
}

#[test]
fn every_measurement_picture_is_in_the_icon_set() {
    fn walk(fields: &[form::Field], out: &mut Vec<String>) {
        for f in fields {
            out.extend(f.diagram.clone());
            if let form::Kind::Group(inner) = &f.kind {
                walk(inner, out);
            }
        }
    }
    let known = icon_set();
    let mut pictures = vec![];
    for c in commands::registry().commands() {
        walk(&form::fields(&c.schema()), &mut pictures);
    }
    for p in &pictures {
        assert!(
            known.contains(p),
            "{p} is not in assets/icons: run scripts/build-icons"
        );
    }
    assert!(
        pictures.len() >= 19,
        "the toolpaths' depth pictures: {pictures:?}"
    );
}

#[test]
fn start_page_and_form_pictures_are_in_the_icon_set() {
    let known = icon_set();
    for (_, icon) in STARTUP_TASKS {
        assert!(known.contains(icon), "{icon}");
    }
    assert!(known.contains(RECENT_FILE_ICON));
    // every picture these forms can show, whatever is chosen
    let forms = [
        "job_setup",
        "orientate_3d_model",
        "draw_rectangle",
        "draw_circle",
        "draw_polygon",
        "draw_star",
    ];
    let fields: Vec<form::Field> = forms
        .iter()
        .flat_map(|id| form::fields(&commands::registry().get(id).unwrap().schema()))
        .collect();
    let mut pictures = 0;
    for f in &fields {
        let Some(template) = &f.icon else { continue };
        let choices = |kind: &form::Kind| match kind {
            form::Kind::Choice(options, _) => options.iter().map(|o| o.value.clone()).collect(),
            _ => vec![],
        };
        let values: Vec<Value> = match &f.kind {
            form::Kind::Choice(..) => choices(&f.kind).into_iter().map(Value::from).collect(),
            form::Kind::Group(inner) => {
                let each: Vec<Value> = inner
                    .iter()
                    .flat_map(|i| {
                        choices(&i.kind)
                            .into_iter()
                            .map(|c| serde_json::json!({ i.key.as_str(): c }))
                    })
                    .collect();
                // a section of numbers only has the one picture
                if each.is_empty() {
                    vec![Value::Null]
                } else {
                    each
                }
            }
            _ => vec![Value::Null],
        };
        for value in values {
            let path = form::icon_path(template, &value);
            if !path.contains('{') {
                assert!(known.contains(&path), "{} shows {path}", f.key);
                pictures += 1;
            }
        }
    }
    assert!(
        pictures >= 9 + 3 + 12,
        "Job Setup's, Orientate 3D Model's and the shape tools' pictures: {pictures}"
    );
}

#[test]
fn a_tool_from_a_flown_out_page_hides_it_unless_it_opens_a_form() {
    let mut h = Harness::new();
    h.app.pages = Pages::machining_layout();
    h.app.pages.open(Page::Design);
    h.app.pick(&h.ctx, Page::Design, "draw_rectangle");
    assert_eq!(
        h.app.pages.shown(Page::Design),
        Shown::Flyout,
        "its form shows there"
    );
    h.app.form = None;
    h.app.pick(&h.ctx, Page::Design, "select_all_vectors");
    assert_eq!(h.app.pages.shown(Page::Design), Shown::Hidden);
}

#[test]
fn a_tools_form_takes_the_place_of_the_page() {
    let mut h = Harness::new();
    h.app.pages.set_pinned(Page::Design, true);
    h.app.pages.set_pinned(Page::Toolpaths, true);
    h.frame(vec![]);
    assert!(h.text_at("Create Vectors").is_some());
    assert!(h.text_at("Job and Tools").is_some());
    h.app.invoke(&h.ctx, "draw_circle");
    h.frame(vec![]);
    assert!(h.text_at("Create Vectors").is_none(), "design tools hidden");
    assert!(h.text_at("Job and Tools").is_some(), "the other page stays");
    h.app.invoke(&h.ctx, "profile_toolpath");
    h.frame(vec![]);
    assert!(
        h.text_at("Create Vectors").is_some(),
        "back once its form went"
    );
    assert!(h.text_at("Job and Tools").is_none(), "toolpath page hidden");
}

#[test]
fn crossing_vectors_go_to_the_vector_validator_or_on_as_options_say() {
    use crate::options::IntersectionCheck;
    use cut_craft_core::geometry::{Path, Point};
    let mut h = Harness::new();
    let bow_tie = Path::polyline(
        &[
            Point::new(10.0, 10.0),
            Point::new(50.0, 50.0),
            Point::new(50.0, 10.0),
            Point::new(10.0, 50.0),
        ],
        true,
    );
    let mut c = Change::builder(&h.app.doc);
    let id = c.add(bow_tie);
    c.select(vec![id]);
    let change = c.finish();
    h.app.history.apply(&mut h.app.doc, change).unwrap();
    let params = h
        .app
        .registry
        .get("pocket_toolpath")
        .unwrap()
        .initial_params(&h.app.doc);
    h.app.options.confirmations.toolpath_intersections = IntersectionCheck::OpenVectorValidator;
    let why = h.app.apply("pocket_toolpath", &params).unwrap_err();
    assert!(why.contains("cross"), "{why}");
    assert!(h.app.doc.toolpaths.is_empty());
    assert_eq!(h.app.form.as_ref().map(|f| f.id), Some("vector_validator"));
    assert!(h.app.doc.selection.contains(&id));
    h.app.options.confirmations.toolpath_intersections = IntersectionCheck::DontCheck;
    h.app.apply("pocket_toolpath", &params).unwrap();
    assert_eq!(h.app.doc.toolpaths.len(), 1);
}

#[test]
fn another_design_tab_closes_the_design_form() {
    let mut h = Harness::new();
    h.app.pages.set_pinned(Page::Design, true);
    h.app.start_command("document_variables");
    assert!(h.app.form.is_some());
    h.app.set_design_tab(h.app.design_tab);
    assert!(h.app.form.is_some(), "the same tab keeps it");
    let layers = DESIGN_TABS.iter().position(|&t| t == "Layers").unwrap();
    h.app.set_design_tab(layers);
    assert!(h.app.form.is_none(), "the Layers tab shows, not the form");
}

#[test]
fn e_on_an_embellishment_opens_embellish_on_it_and_remakes_it_in_place() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 120.0, 120.0);
    let mut params = h.app.registry.get("embellish").unwrap().default_params();
    params["objects"] = 3.into();
    h.app.apply("embellish", &params).unwrap();
    assert_eq!(h.app.doc.components.len(), 1);
    let layers = h.app.doc.layers.len();
    let id = *h.app.doc.components.keys().next().unwrap();
    // its component
    h.app.doc.selection = cut_craft_core::selection::Selection::default();
    h.app.doc.selected_components = [id].into();
    assert!(h.app.available("edit_object"));
    h.frame(vec![key(Key::E)]);
    let form = h.app.form.as_mut().expect("Embellish opened");
    assert_eq!(form.id, "embellish");
    assert_eq!(form.params["objects"], 3);
    let mut edited = form.params.clone();
    h.frame(vec![]);
    assert!(
        h.text_at("Apply").is_some() && h.text_at("Create").is_none(),
        "it changes the one it opened on"
    );
    edited["objects"] = 8.into();
    h.app.apply_form("embellish", &edited).unwrap();
    assert_eq!(h.app.doc.components.len(), 1, "remade, not added");
    assert_eq!(h.app.doc.layers.len(), layers, "on its own layer still");
    // one of its pieces
    h.app.form = None;
    let layer = h.app.doc.layers.last().unwrap().id;
    let piece = *h
        .app
        .doc
        .vectors
        .keys()
        .find(|v| h.app.doc.layer_of(**v) == layer)
        .unwrap();
    h.app.doc.selected_components.clear();
    h.app.doc.selection = [piece].into();
    h.frame(vec![key(Key::E)]);
    let form = h.app.form.as_ref().expect("Embellish opened");
    assert_eq!(form.id, "embellish");
    assert_eq!(form.params["objects"], 8);
}

/// The opening splash shows Matthew's CC logo, which carries the name itself: it is a real,
/// square picture big enough to stay sharp at the splash's size on a large screen.
#[test]
fn splash_logo_is_a_square_picture() {
    let logo = image::load_from_memory(SPLASH_LOGO).expect("the splash logo decodes");
    assert_eq!(
        logo.width(),
        logo.height(),
        "square, so the splash can size it by one edge"
    );
    assert!(logo.width() >= 512, "sharp at a third of a large screen");
}

#[test]
fn a_long_forms_apply_button_and_error_stay_in_sight() {
    let mut h = profile_form();
    let apply = h.text_at("Calculate").expect("Calculate");
    assert!(apply.y < 800.0, "Calculate is in the window: {apply:?}");
    assert!(
        h.text_at("Add ramps to toolpath").is_none(),
        "the form runs on out of sight, under the pinned buttons"
    );
    h.app.form.as_mut().unwrap().error = Some("select the vectors to profile".into());
    h.frame(vec![]);
    h.frame(vec![]);
    let error = h
        .text_at("select the vectors to profile")
        .expect("the error");
    let apply = h.text_at("Calculate").unwrap();
    assert!(
        error.y < apply.y && error.y > 600.0,
        "{error:?} over {apply:?}"
    );
}

/// The push-pin is drawn, loose and pinned, not left to a font's 📌.
#[test]
fn the_pin_buttons_are_drawn_icons() {
    let h = Harness::new();
    for icon in [super::panels::PIN_ICON, super::panels::PINNED_ICON] {
        assert!(
            h.app.icons.find(icon, "").is_some(),
            "{icon} is in assets/icons"
        );
    }
}

#[test]
fn reset_puts_embellish_back_to_its_defaults_and_makes_it_again_as_new() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 160.0, 120.0);
    let shape: Vec<EntityId> = h.app.doc.vectors.keys().copied().collect();
    h.app.doc.selection = shape.iter().copied().collect();
    h.app.invoke(&h.ctx, "embellish");
    h.frame(vec![]);
    let defaults = h.app.registry.get("embellish").unwrap().default_params();
    let mut params = defaults.clone();
    params["seed"] = 99.into();
    params["objects"] = 2.into();
    params["style"] = "rope".into();
    h.app.form.as_mut().unwrap().params = params.clone();
    h.app.apply_form("embellish", &params).unwrap();
    let outlines = |h: &Harness| -> Vec<String> {
        let mut all: Vec<String> = h
            .app
            .doc
            .vectors
            .iter()
            .filter(|(id, _)| !shape.contains(id))
            .map(|(_, v)| format!("{v:?}"))
            .collect();
        all.sort();
        all
    };
    let changed = outlines(&h);
    let depth = h.app.history.undo_depth();
    h.frame(vec![]);
    h.frame(vec![]);
    h.frame(vec![]);
    let at = h.text_at("Reset").expect("a Reset button");
    h.click(at, Modifiers::NONE);
    h.frame(vec![]);
    h.frame(vec![]);
    assert_eq!(
        h.app.form.as_ref().unwrap().params,
        defaults,
        "every setting"
    );
    assert_ne!(outlines(&h), changed, "made again");
    assert_eq!(
        h.app.history.undo_depth(),
        depth,
        "in place, one undo still"
    );
    // the same as making it new with the defaults
    let mut fresh = Harness::new();
    fresh.rectangle(10.0, 10.0, 160.0, 120.0);
    fresh.app.doc.selection = fresh.app.doc.vectors.keys().copied().collect();
    fresh.app.apply("embellish", &defaults).unwrap();
    let mut new: Vec<String> = fresh
        .app
        .doc
        .vectors
        .values()
        .skip(1)
        .map(|v| format!("{v:?}"))
        .collect();
    new.sort();
    assert_eq!(outlines(&h).len(), new.len());
}

#[test]
fn embellishs_style_names_have_a_fixed_place_and_none_says_border() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 160.0, 120.0);
    h.app.doc.selection = h.app.doc.vectors.keys().copied().collect();
    h.app.invoke(&h.ctx, "embellish");
    h.frame(vec![]);
    assert!(h.text_at("Rope").is_some());
    assert!(h.text_at("Rope border").is_none());
    assert!(h.text_at("Fill the shape").is_none());
    // the settings start at the same height whatever the Next line or the style says
    let top = |h: &mut Harness| {
        h.frame(vec![]);
        h.text_at("Repeat length").unwrap().y
    };
    let with_shape = top(&mut h);
    h.app.doc.selection = cut_craft_core::selection::Selection::default();
    let without = top(&mut h);
    assert!(
        (with_shape - without).abs() < 0.5,
        "{with_shape} vs {without}"
    );
}

#[test]
fn a_form_tool_reads_in_one_line_in_the_jobs_units() {
    use cut_craft_core::document::Units;
    let tool = Tool {
        diameter: 6.35,
        pass_depth: 3.0,
        ..Tool::default()
    };
    assert_eq!(
        tool_line(&tool, Some(2.0), Units::Mm),
        "6.35 mm end mill, 3 mm a pass, 2 passes"
    );
    assert_eq!(
        tool_line(&tool, Some(1.0), Units::Mm),
        "6.35 mm end mill, 3 mm a pass, 1 pass"
    );
    let v = Tool {
        shape: cut_craft_core::toolpath::ToolShape::VBit,
        angle: 60.0,
        diameter: 12.7,
        pass_depth: 2.0,
        ..Tool::default()
    };
    assert_eq!(
        tool_line(&v, None, Units::Mm),
        "60° V-bit, 12.7 mm across, 2 mm a pass"
    );
    // a metric bit kept in mm in the Tool Database reads in inches in an inch job's toolpath
    assert_eq!(
        tool_line(&v, None, Units::Inch),
        "60° V-bit, 0.5 in across, 0.079 in a pass"
    );
    // and an inch bit in a metric job's toolpath in mm
    let inch_bit = Tool {
        units: Units::Inch,
        ..tool
    };
    assert_eq!(
        tool_line(&inch_bit, None, Units::Mm),
        "6.35 mm end mill, 3 mm a pass"
    );
    let slots = [None, Some("Badger".to_owned())];
    assert_eq!(held_name("Badger", &slots), "T2  Badger");
    assert_eq!(held_name("Other", &slots), "Other");
}

/// A drawing tool's form offers Create until its shape is drawn; then the shape follows the
/// form, so there is nothing to press, and New Shape lets it go to type another.
#[test]
fn a_drawing_tools_form_creates_until_its_shape_is_drawn() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "draw_circle");
    h.frame(vec![]);
    assert!(h.text_at("Create").is_some(), "Create, with nothing drawn");
    assert!(h.text_at("Apply").is_none());
    let (from, to) = (h.at(100.0, 100.0), h.at(140.0, 100.0));
    h.drag(from, to);
    h.frame(vec![]);
    assert_eq!(h.app.doc.vectors.len(), 1);
    assert!(h.text_at("Create").is_none(), "drawn: it follows the form");
    assert!(h.text_at("Apply").is_none());
    assert!(h.text_at("Close").is_some() && h.text_at("Cancel").is_some());
    assert!(
        h.text_at(&commit::live_note("draw_circle", true)).is_some(),
        "says why there is no button"
    );
    let new = h.text_at("New Shape").expect("a way to type another");
    h.click(new, Modifiers::NONE);
    h.frame(vec![]);
    let create = h.text_at("Create").expect("Create again");
    h.click(create, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(
        h.app.doc.vectors.len(),
        2,
        "a second circle, the first kept"
    );
}

#[test]
fn embellish_drawn_finer_than_it_carves_says_so_in_red_at_the_foot_of_its_form() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 3.0, 3.0);
    h.app.start_command("embellish");
    let params = h
        .app
        .form
        .as_ref()
        .expect("Embellish opened")
        .params
        .clone();
    h.app.apply_form("embellish", &params).unwrap();
    let form = h.app.form.as_ref().unwrap();
    assert!(form.error.is_none(), "{:?}", form.error);
    let warning = form.warning.as_deref().unwrap_or_default();
    assert!(warning.contains("smaller than it will carve"), "{warning}");
    // a shape it fits: the warning goes
    h.app.form = None;
    h.app.doc.selection = cut_craft_core::selection::Selection::default();
    h.rectangle(30.0, 10.0, 120.0, 120.0);
    h.app.start_command("embellish");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply_form("embellish", &params).unwrap();
    assert_eq!(h.app.form.as_ref().unwrap().warning, None);
}

#[test]
fn embellishs_pattern_maker_adds_samples_to_the_sequence() {
    let mut h = Harness::new();
    // tall enough that the whole form shows without scrolling
    h.window = vec2(1280.0, 2400.0);
    h.rectangle(10.0, 10.0, 160.0, 120.0);
    h.app.doc.selection = h.app.doc.vectors.keys().copied().collect();
    h.app.invoke(&h.ctx, "embellish");
    let mut params = h.app.registry.get("embellish").unwrap().default_params();
    params["style"] = "chevrons".into();
    h.app.form.as_mut().unwrap().params = params;
    for _ in 0..4 {
        h.frame(vec![]);
    }
    let sequence = |h: &Harness| h.app.form.as_ref().unwrap().params["sequence"].clone();
    // the samples sit under the Random size tick, each named by the entry it adds
    let below = h.text_at("Random size (S)").expect("the pattern maker");
    let sample = |h: &Harness, name: &str| {
        h.shapes
            .iter()
            .filter_map(|c| match &c.shape {
                egui::Shape::Text(t) if t.galley.text() == name && t.pos.y > below.y => {
                    Some(t.pos + t.galley.size() / 2.0)
                }
                _ => None,
            })
            .min_by(|a, b| a.distance(below).total_cmp(&b.distance(below)))
    };
    for name in ["2", "1", "0"] {
        let at = sample(&h, name).expect("a sample button");
        h.click(at, Modifiers::NONE);
        h.frame(vec![]);
    }
    assert_eq!(sequence(&h), "210");
    let at = h.text_at("Undo").expect("Undo");
    h.click(at, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(sequence(&h), "21");
    let at = h.text_at("Random").expect("Random");
    h.click(at, Modifiers::NONE);
    h.frame(vec![]);
    assert_ne!(sequence(&h), "");
    let at = h.text_at("Clear").expect("Clear");
    h.click(at, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(sequence(&h), "");
}

#[test]
fn a_drawing_named_on_the_command_line_goes_through_job_setup_into_a_new_job() {
    let dir = std::env::temp_dir().join(format!("cut-craft-argv-svg-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let svg = dir.join("square.svg");
    std::fs::write(
        &svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="20mm" height="20mm" viewBox="0 0 20 20"><rect x="0" y="0" width="20" height="20"/></svg>"#,
    )
    .unwrap();
    let mut h = Harness::at_classic_start();
    h.app.open_given(Some(&svg));
    h.frame(vec![]);
    assert_eq!(
        h.app.screen,
        Screen::NewJob,
        "Job Setup for the job to hold it"
    );
    assert!(
        h.app.status.message.contains("square.svg"),
        "{}",
        h.app.status.message
    );
    h.app.job_setup_ok();
    for _ in 0..5 {
        h.frame(vec![]);
    }
    assert_eq!(h.app.screen, Screen::Job);
    assert!(!h.app.doc.vectors.is_empty(), "the drawing came in");
    // Cancel in Job Setup: back to the start page, nothing imported later
    let mut h = Harness::at_classic_start();
    h.app.open_given(Some(&svg));
    h.app.job_setup_cancel();
    assert_eq!(h.app.screen, Screen::Start);
    h.app.invoke(&h.ctx, "new");
    h.frame(vec![]);
    h.app.job_setup_ok();
    for _ in 0..5 {
        h.frame(vec![]);
    }
    assert!(h.app.doc.vectors.is_empty(), "the cancelled file stays out");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_drawing_named_on_the_command_line_comes_into_a_blank_job() {
    let dir = std::env::temp_dir().join(format!("cut-craft-argv-blank-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let svg = dir.join("square.svg");
    std::fs::write(
        &svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="50mm" height="50mm" viewBox="0 0 50 50"><rect x="5" y="5" width="40" height="40"/></svg>"#,
    )
    .unwrap();
    let mut h = Harness::at_start();
    h.app.open_given(Some(&svg));
    for _ in 0..5 {
        h.frame(vec![]);
    }
    assert_eq!(h.app.screen, Screen::Job, "no Job Setup first");
    assert!(!h.app.doc.vectors.is_empty(), "the drawing came in");
    let _ = std::fs::remove_dir_all(&dir);
}
