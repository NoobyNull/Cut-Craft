//! The drawing tools and their forms: shapes, embellishments, fills and importing into shapes.

use super::*;

#[test]
fn a_tool_in_other_units_leaves_the_form_in_the_jobs_units() {
    let mut h = Harness::new();
    h.invoke("pocket_toolpath");
    // an inch bit picked in a mm job: the bit's units are its own, not the form's
    h.set_form_value("/tool/units", Value::from("Inch"));
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
    h.given_options(|o| o.general.layout = crate::options::WindowLayout::Rail);
    let menu = h.row_menu("draw_circle");
    assert_eq!(menu[0].name, "Add to Favourites");
    h.invoke(&menu[0].action);
    let menu = h.row_menu("pocket_toolpath");
    h.invoke(&menu[0].action);
    assert_eq!(h.options().favourites.design, ["draw_circle"]);
    assert_eq!(h.options().favourites.toolpaths, ["pocket_toolpath"]);
    assert_eq!(h.row_menu("draw_circle")[0].name, "Remove from Favourites");
    h.frame(vec![]);
    let star = h.text_at("★").expect("the Rail's favourites button");
    h.press(star, true, Modifiers::NONE);
    h.press(star, false, Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.rail_favourites_open());
    for name in ["Favourites", "Draw Circle", "Pocket Toolpath"] {
        assert!(h.text_at(name).is_some(), "{name}");
    }
    assert!(h.text_at("Draw Rectangle").is_none(), "only the favourites");
}

#[test]
fn a_drawn_shape_changes_with_its_form_until_it_is_let_go() {
    use cut_craft_core::geometry::Segment;
    let arcs = |h: &Harness| {
        let only: Vec<_> = h.doc().vectors.values().collect();
        assert_eq!(only.len(), 1, "one shape, not a second on top");
        only[0]
            .segments
            .iter()
            .filter(|s| matches!(s, Segment::Arc { .. }))
            .count()
    };
    let mut h = Harness::new();
    h.invoke("draw_rectangle");
    h.frame(vec![]);
    let (from, to) = (h.at(100.0, 100.0), h.at(200.0, 160.0));
    h.drag(from, to);
    assert_eq!(arcs(&h), 0, "drawn square");
    // issue #2: choosing rounded corners afterwards changes the rectangle just drawn, as
    // soon as it is chosen (issue #3), and Apply has nothing more to do
    let rounded = h.text_at("Rounded").expect("the corner choices");
    h.click(rounded, Modifiers::NONE);
    assert_eq!(arcs(&h), 4, "the same rectangle, rounded");
    let params = h.form_settings().clone();
    h.apply_form("draw_rectangle", &params).unwrap();
    assert_eq!(arcs(&h), 4);
    let rect = h.doc().vectors.values().next().unwrap().bounds();
    // drawn from screen pixels: the same to a hundredth of a millimetre
    assert!((rect.size().x - 100.0).abs() < 1e-2, "same size: {rect:?}");
    h.invoke("undo");
    assert_eq!(arcs(&h), 0, "one undo puts the square back");
    // once let go, Apply draws a new shape
    h.invoke("redo");
    h.select([]);
    h.apply_form("draw_rectangle", &params).unwrap();
    assert_eq!(h.doc().vectors.len(), 2);
}

/// A value dragged on a drawing form is one undo: each step takes back the drag's last one.
/// Only that step: an Undo and another edit made while the button is still held are not
/// taken back by the drag going on.
#[test]
fn a_drag_on_a_form_takes_back_only_its_own_last_step() {
    let mut h = Harness::new();
    h.invoke("draw_rectangle");
    h.frame(vec![]);
    let (from, to) = (h.at(100.0, 100.0), h.at(200.0, 160.0));
    h.drag(from, to);
    assert_eq!(h.doc().vectors.len(), 1);
    let dragged = *h.doc().vectors.keys().next().unwrap();
    // held on the form, a value dragged: its first step
    let on_form = h.text_at("Rounded").expect("the form is open");
    h.frame(vec![mouse(PointerButton::Primary, on_form, true)]);
    h.set_form_value("/width", Value::from(80.0));
    h.frame_holding(Modifiers::NONE, vec![]);
    // still held: that step undone, and another edit in its place
    h.invoke("undo");
    let params = serde_json::json!({
        "x": 290.0, "y": 290.0, "width": 20.0, "height": 20.0, "corners": "square", "radius": 0.0
    });
    h.run("draw_rectangle", &params).unwrap();
    assert_eq!(h.doc().vectors.len(), 2);
    h.select([dragged]);
    // the drag goes on: it must not take back the square drawn meanwhile
    h.set_form_value("/width", Value::from(90.0));
    h.frame_holding(Modifiers::NONE, vec![]);
    let square_stays = h.doc().vectors.values().any(|v| {
        let b = v.bounds();
        (b.center().x - 300.0).abs() < 1e-6 && (b.size().x - 20.0).abs() < 1e-6
    });
    assert!(square_stays, "the square drawn meanwhile stays");
    assert_eq!(h.doc().vectors.len(), 2, "beside the one dragged");
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
        let shape = h.default_settings("create_shape");
        h.run("create_shape", &shape).unwrap();
        let id = *h.doc().components.keys().next().unwrap();
        h.select([]);
        h.given_document(|doc| doc.selected_components = [id].into_iter().collect());
        assert_eq!(h.made_to_edit(), Some("create_shape"), "{tool}: made here");
        let mut params = h.default_settings(tool);
        if tool == "sculpt" {
            params["points"] = serde_json::json!([{ "x": 30.0, "y": 30.0 }]);
        }
        if tool == "replace_below" {
            params["below"] = 0.5.into();
        }
        h.run(tool, &params).unwrap();
        assert!(
            h.doc().components.values().all(|k| k.shape.is_none()),
            "{tool} leaves a model, not a recipe to remake"
        );
        let ids: Vec<_> = h.doc().components.keys().copied().collect();
        h.given_document(|doc| doc.selected_components = ids.into_iter().collect());
        assert_eq!(h.made_to_edit(), None, "{tool}: E no longer remakes it");
    }
}

#[test]
fn every_measurement_picture_is_in_the_icon_set() {
    // a picture of what is picked: one for each choice
    fn walk(fields: &[form::Field], out: &mut Vec<String>) {
        for f in fields {
            if let Some(template) = &f.diagram {
                match &f.kind {
                    form::Kind::Choice(options, _) if template.contains("{}") => {
                        out.extend(
                            options
                                .iter()
                                .map(|o| form::icon_path(template, &Value::from(o.value.clone()))),
                        );
                    }
                    _ => out.push(template.clone()),
                }
            }
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
            "{p} is not in assets/icons: add its SVG there"
        );
        // drawn as shapes, so they stay sharp on a scaled screen; letters as outlines,
        // since the SVG reader is given no fonts
        assert!(
            crate::icons::is_svg(p),
            "{p}: a measurement picture is an SVG"
        );
        let svg = crate::icons::built_in::BUILT_IN
            .iter()
            .find(|(key, _)| key == p)
            .map(|(_, bytes)| String::from_utf8_lossy(bytes))
            .expect("built in");
        assert!(!svg.contains("<text"), "{p}: letters drawn as outlines");
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
    h.invoke("machining_layout");
    h.open_page(Page::Design);
    h.pick_on_page(Page::Design, "draw_rectangle");
    assert_eq!(
        h.pages().shown(Page::Design),
        Shown::Flyout,
        "its form shows there"
    );
    h.close_form();
    h.pick_on_page(Page::Design, "select_all_vectors");
    assert_eq!(h.pages().shown(Page::Design), Shown::Hidden);
}

#[test]
fn a_tools_form_takes_the_place_of_the_page() {
    let mut h = Harness::new();
    h.pin(Page::Design);
    h.pin(Page::Toolpaths);
    h.frame(vec![]);
    assert!(h.text_at("Create Vectors").is_some());
    assert!(h.text_at("Job and Bits").is_some());
    h.invoke("draw_circle");
    h.frame(vec![]);
    assert!(h.text_at("Create Vectors").is_none(), "design tools hidden");
    assert!(h.text_at("Job and Bits").is_some(), "the other page stays");
    h.invoke("profile_toolpath");
    h.frame(vec![]);
    assert!(
        h.text_at("Create Vectors").is_some(),
        "back once its form went"
    );
    assert!(h.text_at("Job and Bits").is_none(), "toolpath page hidden");
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
    let mut c = Change::builder(h.doc());
    let id = c.add(bow_tie);
    c.select(vec![id]);
    h.make(c.finish(), "draw");
    let params = h.opening_settings("pocket_toolpath");
    h.given_options(|o| {
        o.confirmations.toolpath_intersections = IntersectionCheck::OpenVectorValidator;
    });
    let why = h.run("pocket_toolpath", &params).unwrap_err();
    assert!(why.contains("cross"), "{why}");
    assert!(h.doc().toolpaths.is_empty());
    assert_eq!(h.form_id(), Some("vector_validator"));
    assert!(h.doc().selection.contains(&id));
    h.given_options(|o| o.confirmations.toolpath_intersections = IntersectionCheck::DontCheck);
    h.run("pocket_toolpath", &params).unwrap();
    assert_eq!(h.doc().toolpaths.len(), 1);
}

#[test]
fn another_design_tab_closes_the_design_form() {
    let mut h = Harness::new();
    h.pin(Page::Design);
    h.open_tool_form("document_variables");
    assert!(h.form_id().is_some());
    let same = h.design_tab();
    h.show_design_tab(same);
    assert!(h.form_id().is_some(), "the same tab keeps it");
    h.show_design_tab("Layers");
    assert!(h.form_id().is_none(), "the Layers tab shows, not the form");
}

#[test]
fn e_on_an_embellishment_opens_embellish_on_it_and_remakes_it_in_place() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 120.0, 120.0);
    let mut params = h.default_settings("embellish");
    params["objects"] = 3.into();
    h.run("embellish", &params).unwrap();
    assert_eq!(h.doc().components.len(), 1);
    let layers = h.doc().layers.len();
    let id = *h.doc().components.keys().next().unwrap();
    // its component
    h.select([]);
    h.given_document(|doc| doc.selected_components = [id].into());
    assert!(h.available("edit_object"));
    h.frame(vec![key(Key::E)]);
    assert_eq!(h.form_id(), Some("embellish"), "Embellish opened");
    assert_eq!(h.form_value("objects"), 3);
    let mut edited = h.form_settings().clone();
    h.frame(vec![]);
    assert!(
        h.text_at("Apply").is_some() && h.text_at("Create").is_none(),
        "it changes the one it opened on"
    );
    edited["objects"] = 8.into();
    h.apply_form("embellish", &edited).unwrap();
    assert_eq!(h.doc().components.len(), 1, "remade, not added");
    assert_eq!(h.doc().layers.len(), layers, "on its own layer still");
    // one of its pieces
    h.close_form();
    let layer = h.doc().layers.last().unwrap().id;
    let piece = *(h.doc().vectors.keys())
        .find(|v| h.doc().layer_of(**v) == layer)
        .unwrap();
    h.given_document(|doc| doc.selected_components.clear());
    h.select([piece]);
    h.frame(vec![key(Key::E)]);
    assert_eq!(h.form_id(), Some("embellish"), "Embellish opened");
    assert_eq!(h.form_value("objects"), 8);
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
    h.given_form_error("select the vectors to profile");
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
        assert!(h.has_icon(icon), "{icon} is in assets/icons");
    }
}

#[test]
fn reset_puts_embellish_back_to_its_defaults_and_makes_it_again_as_new() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 160.0, 120.0);
    let shape: Vec<EntityId> = h.doc().vectors.keys().copied().collect();
    h.select(shape.iter().copied());
    h.invoke("embellish");
    h.frame(vec![]);
    let defaults = h.default_settings("embellish");
    let mut params = defaults.clone();
    params["seed"] = 99.into();
    params["objects"] = 2.into();
    params["style"] = "rope".into();
    h.set_form_settings(params.clone());
    h.apply_form("embellish", &params).unwrap();
    let outlines = |h: &Harness| -> Vec<String> {
        let mut all: Vec<String> = (h.doc().vectors.iter())
            .filter(|(id, _)| !shape.contains(id))
            .map(|(_, v)| format!("{v:?}"))
            .collect();
        all.sort();
        all
    };
    let changed = outlines(&h);
    let depth = h.undo_steps();
    h.frame(vec![]);
    h.frame(vec![]);
    h.frame(vec![]);
    let at = h.text_at("Reset").expect("a Reset button");
    h.click(at, Modifiers::NONE);
    h.frame(vec![]);
    h.frame(vec![]);
    assert_eq!(h.form_settings(), &defaults, "every setting");
    assert_ne!(outlines(&h), changed, "made again");
    assert_eq!(h.undo_steps(), depth, "in place, one undo still");
    // the same as making it new with the defaults
    let mut fresh = Harness::new();
    fresh.rectangle(10.0, 10.0, 160.0, 120.0);
    let all: Vec<EntityId> = fresh.doc().vectors.keys().copied().collect();
    fresh.select(all);
    fresh.run("embellish", &defaults).unwrap();
    let mut new: Vec<String> = (fresh.doc().vectors.values())
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
    let all: Vec<EntityId> = h.doc().vectors.keys().copied().collect();
    h.select(all);
    h.invoke("embellish");
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
    h.select([]);
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
    // a metric bit kept in mm in the Bit Library reads in inches in an inch job's toolpath
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
    h.invoke("draw_circle");
    h.frame(vec![]);
    assert!(h.text_at("Create").is_some(), "Create, with nothing drawn");
    assert!(h.text_at("Apply").is_none());
    let (from, to) = (h.at(100.0, 100.0), h.at(140.0, 100.0));
    h.drag(from, to);
    h.frame(vec![]);
    assert_eq!(h.doc().vectors.len(), 1);
    assert!(h.text_at("Create").is_none(), "drawn: it follows the form");
    assert!(h.text_at("Apply").is_none());
    assert!(h.text_at("Close").is_some() && h.text_at("Cancel").is_some());
    assert!(
        h.text_at(&commit::live_note(cut_craft_core::Form::Apply, true))
            .is_some(),
        "says why there is no button"
    );
    let new = h.text_at("New Shape").expect("a way to type another");
    h.click(new, Modifiers::NONE);
    h.frame(vec![]);
    let create = h.text_at("Create").expect("Create again");
    h.click(create, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.doc().vectors.len(), 2, "a second circle, the first kept");
}

#[test]
fn embellish_drawn_finer_than_it_carves_says_so_in_red_at_the_foot_of_its_form() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 3.0, 3.0);
    h.open_tool_form("embellish");
    let params = h.form_settings().clone();
    h.apply_form("embellish", &params).unwrap();
    assert!(h.form_error().is_none(), "{:?}", h.form_error());
    let warning = h.form_warning().unwrap_or_default();
    assert!(warning.contains("smaller than it will carve"), "{warning}");
    // a shape it fits: the warning goes
    h.close_form();
    h.select([]);
    h.rectangle(30.0, 10.0, 120.0, 120.0);
    h.open_tool_form("embellish");
    let params = h.form_settings().clone();
    h.apply_form("embellish", &params).unwrap();
    assert_eq!(h.form_warning(), None);
}

#[test]
fn embellishs_pattern_maker_adds_samples_to_the_sequence() {
    let mut h = Harness::new();
    // tall enough that the whole form shows without scrolling
    h.window = vec2(1280.0, 2400.0);
    h.rectangle(10.0, 10.0, 160.0, 120.0);
    let all: Vec<EntityId> = h.doc().vectors.keys().copied().collect();
    h.select(all);
    h.invoke("embellish");
    let mut params = h.default_settings("embellish");
    params["style"] = "chevrons".into();
    h.set_form_settings(params);
    for _ in 0..4 {
        h.frame(vec![]);
    }
    let sequence = |h: &Harness| h.form_value("sequence").clone();
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
    h.open_given(&svg);
    h.frame(vec![]);
    assert_eq!(
        h.screen(),
        Screen::NewJob,
        "Job Setup for the job to hold it"
    );
    assert!(
        h.status_message().contains("square.svg"),
        "{}",
        h.status_message()
    );
    h.job_setup_ok();
    for _ in 0..5 {
        h.frame(vec![]);
    }
    assert_eq!(h.screen(), Screen::Job);
    assert!(!h.doc().vectors.is_empty(), "the drawing came in");
    // Cancel in Job Setup: back to the start page, nothing imported later
    let mut h = Harness::at_classic_start();
    h.open_given(&svg);
    h.job_setup_cancel();
    assert_eq!(h.screen(), Screen::Start);
    h.invoke("new");
    h.frame(vec![]);
    h.job_setup_ok();
    for _ in 0..5 {
        h.frame(vec![]);
    }
    assert!(h.doc().vectors.is_empty(), "the cancelled file stays out");
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
    h.open_given(&svg);
    for _ in 0..5 {
        h.frame(vec![]);
    }
    assert_eq!(h.screen(), Screen::Job, "no Job Setup first");
    assert!(!h.doc().vectors.is_empty(), "the drawing came in");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_embellish_remake_that_fails_keeps_what_it_last_made() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 160.0, 120.0);
    let all: Vec<EntityId> = h.doc().vectors.keys().copied().collect();
    h.select(all);
    h.invoke("embellish");
    h.frame(vec![]);
    let params = h.default_settings("embellish");
    h.set_form_settings(params.clone());
    h.apply_form("embellish", &params).unwrap();
    let made = (h.doc().vectors.len(), h.doc().components.len());
    let depth = h.undo_steps();
    // a raise height of nothing cannot be made
    let mut bad = params.clone();
    bad["make"] = "both".into();
    bad["direction"] = "raised".into();
    bad["height"] = 0.0.into();
    assert!(h.apply_form("embellish", &bad).is_err());
    assert_eq!(
        (h.doc().vectors.len(), h.doc().components.len()),
        made,
        "the last ornament is still there"
    );
    assert_eq!(h.undo_steps(), depth, "still one undo");
    assert_eq!(
        h.remade_last(),
        Some(params.clone()),
        "and the form still remakes it in place"
    );
    // a good change after replaces it, not adds a second one
    let mut good = params.clone();
    good["seed"] = 7.into();
    h.apply_form("embellish", &good).unwrap();
    assert_eq!(h.undo_steps(), depth, "in place");
}
