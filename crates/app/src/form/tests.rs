use super::*;
use crate::commands::registry;
use cut_craft_core::document::Units;

fn schema(id: &str) -> Value {
    registry().get(id).unwrap().schema()
}

fn keys(fields: &[Field]) -> Vec<&str> {
    fields.iter().map(|f| f.key.as_str()).collect()
}

fn field<'a>(fields: &'a [Field], key: &str) -> &'a Field {
    fields.iter().find(|f| f.key == key).unwrap()
}

#[test]
fn decorative_reliefs_style_uses_the_fills_pictures_and_has_new_variation() {
    let relief = fields(&schema("decorative_relief"));
    assert_eq!(relief[0].key, "style");
    let Kind::Choice(styles, Widget::Icons { picture, named }) = &relief[0].kind else {
        panic!("{:?}", relief[0].kind);
    };
    assert!(*named);
    assert_eq!(picture.as_deref(), Some("drawing_forms/fill_{}.png"));
    let labels: Vec<&str> = styles.iter().map(|c| c.label.as_str()).collect();
    assert_eq!(labels, ["Baroque", "Western leaf"]);
    assert_eq!(field(&relief, "seed").kind, Kind::Seed);
}

#[test]
fn vector_textures_style_is_a_row_of_named_pictures_and_its_seed_has_new_variation() {
    let texture = fields(&schema("vector_texture"));
    assert_eq!(texture[0].key, "style", "the style comes first");
    let Kind::Choice(styles, Widget::Icons { picture, named }) = &texture[0].kind else {
        panic!("{:?}", texture[0].kind);
    };
    assert!(*named);
    assert_eq!(picture.as_deref(), Some("drawing_forms/fill_{}.png"));
    let labels: Vec<&str> = styles.iter().map(|c| c.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Wavy lines",
            "Floral",
            "Baroque",
            "Western leaf",
            "Frost",
            "Snowflakes",
            "Fractal",
            "Stones",
            "Ripples",
            "Hammered",
            "Bandanna"
        ]
    );
    assert_eq!(field(&texture, "seed").kind, Kind::Seed);
    // the wavy lines' own settings only show for them, the pattern's for the rest
    let wavy = serde_json::json!({ "style": "wavy_lines" });
    let floral = serde_json::json!({ "style": "floral" });
    let shown = |key, params| {
        field(&texture, key)
            .when
            .as_ref()
            .is_none_or(|w| w.shown(params))
    };
    assert!(shown("wavelength", &wavy) && !shown("wavelength", &floral));
    assert!(!shown("gap", &wavy) && shown("gap", &floral));
}

#[test]
fn laser_forms_show_only_the_settings_of_the_strategy_picked() {
    let f = fields(&schema("laser_cut_fill_toolpath"));
    let shown = |key, strategy: &str| {
        let params = serde_json::json!({ "strategy": strategy });
        field(&f, key)
            .when
            .as_ref()
            .is_none_or(|w| w.shown(&params))
    };
    for hatch in ["stepover", "hatch_angle", "cross_hatch", "outline"] {
        assert!(shown(hatch, "hatch_fill"), "{hatch}");
        assert!(
            !shown(hatch, "cut_outside") && !shown(hatch, "cut_on"),
            "{hatch}"
        );
    }
    assert!(shown("allowance", "cut_outside") && shown("allowance", "cut_inside"));
    assert!(!shown("allowance", "hatch_fill") && !shown("allowance", "cut_on"));
    let f = fields(&schema("laser_picture_toolpath"));
    let shown = |key, strategy: &str| {
        let params = serde_json::json!({ "strategy": strategy });
        field(&f, key)
            .when
            .as_ref()
            .is_none_or(|w| w.shown(&params))
    };
    for lines in ["line_angle", "line_spacing"] {
        assert!(shown(lines, "raster") && shown(lines, "hatch"), "{lines}");
        assert!(!shown(lines, "selected"), "{lines}");
    }
}

#[test]
fn finishing_shows_its_rest_machining_settings_only_with_two_tools_or_more() {
    let f = fields(&schema("3d_finishing_toolpath"));
    let tools = |n: usize| serde_json::json!({ "tools": vec![serde_json::json!({}); n] });
    for key in ["minimum_detail", "boundary_offset"] {
        let rest = field(&f, key);
        assert_eq!(rest.group.as_deref(), Some("Rest Machining"), "{key}");
        let shown = |n| rest.when.as_ref().is_none_or(|w| w.shown(&tools(n)));
        assert!(!shown(1), "{key}: one tool has nothing to rest machine");
        assert!(shown(2) && shown(3), "{key}");
        // one of a set, edited on its own: cut after the tools before it
        let later = serde_json::json!({ "tools": [], "after": [{}] });
        assert!(rest.when.as_ref().is_none_or(|w| w.shown(&later)), "{key}");
    }
    assert_eq!(field(&f, "boundary_offset").label, "Rest overlap");
    assert_eq!(
        field(&f, "machining_boundary_offset").label,
        "Boundary Offset"
    );
}

#[test]
fn embellishs_settings_start_with_one_style_list_and_have_a_new_design_button() {
    let f = fields(&schema("embellish"));
    assert_eq!(keys(&f)[0], "style");
    let Kind::Choice(styles, _) = &field(&f, "style").kind else {
        panic!("{:?}", field(&f, "style").kind);
    };
    let labels: Vec<&str> = styles.iter().map(|c| c.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Folk mix",
            "Scrolls",
            "Flowers",
            "Band and beads",
            "Rope",
            "Chain",
            "Twist",
            "Braid",
            "Bead and reel",
            "Laurel leaves",
            "Feathers",
            "Chevrons"
        ]
    );
    // each setting only shows for the styles it changes
    let style = |s: &str| serde_json::json!({ "style": s });
    let (rope, chain, mix) = (style("rope"), style("chain"), style("folk_mix"));
    let (scrolls, flowers) = (style("scrolls"), style("flowers"));
    let shown = |key, params| field(&f, key).when.as_ref().is_none_or(|w| w.shown(params));
    assert!(shown("border_width", &rope) && shown("border_width", &mix));
    assert!(shown("twist", &rope) && !shown("twist", &chain));
    assert!(shown("scroll_size", &scrolls) && !shown("scroll_size", &chain));
    assert!(shown("flowers", &flowers) && !shown("flowers", &scrolls));
    let twist = style("twist");
    assert!(shown("sequence", &mix) && shown("sequence", &rope) && !shown("sequence", &twist));
    assert_eq!(field(&f, "sequence").kind, Kind::Sequence);
    for key in ["refinement", "tightness"] {
        assert_eq!(
            field(&f, key).kind,
            Kind::Number {
                unit: None,
                min: Some(1.0),
                max: Some(10.0),
                integer: true,
                steps: vec![],
                slider: true,
                percent_when: None,
            },
            "{key}"
        );
    }
    assert!(matches!(
        field(&f, "height").kind,
        Kind::Number { slider: false, .. }
    ));
    assert_eq!(field(&f, "seed").kind, Kind::Seed);
}

#[test]
fn a_list_of_lengths_keeps_its_unit() {
    // Slice's own list of thicknesses: each one a length, shown in inches in an inch job
    let f = fields(&schema("slice"));
    let Kind::List(item) = &field(&f, "thicknesses").kind else {
        panic!("{:?}", field(&f, "thicknesses").kind);
    };
    assert!(
        matches!(&**item, Kind::Number { unit: Some(u), .. } if u == "mm"),
        "{item:?}"
    );
}

#[test]
fn set_sizes_width_is_a_length_until_percent_is_ticked() {
    let f = fields(&schema("set_size"));
    for key in ["width", "height"] {
        let Kind::Number {
            unit, percent_when, ..
        } = &field(&f, key).kind
        else {
            panic!("{key}");
        };
        assert_eq!(unit.as_deref(), Some("mm"), "{key}");
        let when = percent_when.as_ref().expect("a percentage when ticked");
        assert!(when.shown(&serde_json::json!({ "percent": true })));
        assert!(!when.shown(&serde_json::json!({ "percent": false })));
    }
}

#[test]
fn number_boxes_step_and_drag_in_amounts_that_fit_the_unit_shown() {
    use cut_craft_bug_report::number::nudged;
    // 3D finishing accuracy, 1..20 µm, in an inch job: one click is not the whole range
    let (lo, hi) = (1.0 / 25_400.0, 20.0 / 25_400.0);
    let (step, speed) = feel(Some("in"), false, lo..=hi);
    let once = nudged(5.0 / 25_400.0, step, true, &(lo..=hi)) * 25_400.0;
    assert!(once < 10.0, "5 µm went to {once} µm in one click");
    assert!(speed <= step);
    // a cut depth in inches drags in thousandths, not inches
    let (step, speed) = feel(Some("in"), false, 0.0..=f64::INFINITY);
    assert!((step - 0.01).abs() < 1e-12);
    assert!(speed <= 0.001 + 1e-12, "{speed} in a point");
    let (_, speed) = feel(Some("mm"), false, 0.0..=f64::INFINITY);
    assert!(speed <= 0.01 + 1e-12, "{speed} mm a point");
    // whole numbers still drag one a point
    assert_eq!(feel(None, true, 1.0..=100.0), (1.0, 1.0));
}

#[test]
fn show_rules_written_as_numbers_or_bools_hide_their_fields() {
    let f = fields(&schema("segment_3d_model"));
    let when = field(&f, "position")
        .when
        .as_ref()
        .expect("Position has a rule");
    assert!(when.shown(&serde_json::json!({ "cuts": 1 })));
    assert!(!when.shown(&serde_json::json!({ "cuts": 3 })));
    let f = fields(&schema("extrude_and_weave"));
    let when = field(&f, "weave_by")
        .when
        .as_ref()
        .expect("Weave by has a rule");
    assert!(!when.shown(&serde_json::json!({ "weave": false })));
    // every rule in every form is read, none dropped
    for (id, schema) in every_form() {
        for s in rule_holders(&schema) {
            for key in [
                "x-show-when",
                "x-hide-when",
                "x-enable-when",
                "x-percent-when",
            ] {
                if let Some(rule) = s.get(key) {
                    assert!(When::from_rule(rule, true).is_some(), "{id}: {key} {rule}");
                }
            }
        }
    }
}

/// Every object in `schema`, where a rule may sit.
fn rule_holders(schema: &Value) -> Vec<&Value> {
    let mut found = vec![schema];
    match schema {
        Value::Object(map) => map.values().for_each(|v| found.extend(rule_holders(v))),
        Value::Array(list) => list.iter().for_each(|v| found.extend(rule_holders(v))),
        _ => {}
    }
    found
}

#[test]
fn fields_used_only_with_a_tick_are_greyed_without_it() {
    let live = |f: &[Field], key, local: Value, top: Value| {
        field(f, key)
            .enabled
            .as_ref()
            .expect("greyed by a rule")
            .judged(&local, &top)
    };
    let job = fields(&schema("job_setup"));
    let Kind::Group(datum) = &field(&job, "datum").kind else {
        panic!("the datum section");
    };
    for key in ["x", "y"] {
        // judged on the datum section's own tick, not the form's top level
        let top = serde_json::json!({ "datum": {} });
        let ticked = serde_json::json!({ "use_offset": true });
        let unticked = serde_json::json!({ "use_offset": false });
        assert!(live(datum, key, ticked, top.clone()));
        assert!(!live(datum, key, unticked, top));
    }
    let flute = fields(&schema("fluting_toolpath"));
    let percent = serde_json::json!({ "use_ramp_percent": true });
    let length = serde_json::json!({ "use_ramp_percent": false });
    assert!(live(&flute, "ramp_percent", Value::Null, percent.clone()));
    assert!(!live(&flute, "ramp_length", Value::Null, percent));
    assert!(live(&flute, "ramp_length", Value::Null, length));
    let ramp_percent = field(&flute, "ramp_percent");
    assert_eq!(ramp_percent.label, "Ramp percent");
    assert!(matches!(&ramp_percent.kind, Kind::Number { unit: Some(u), .. } if u == "%"));
    let weave = fields(&schema("extrude_and_weave"));
    let scale = serde_json::json!({ "scale_to_exact_height": false });
    assert!(!live(&weave, "height", Value::Null, scale));
    let component = fields(&schema("component_properties"));
    let parent = serde_json::json!({ "appearance": "same_as_parent" });
    let solid = serde_json::json!({ "appearance": "solid_color" });
    assert!(!live(&component, "colour", Value::Null, parent));
    assert!(live(&component, "colour", Value::Null, solid));
}

#[test]
fn job_setups_datum_x_and_y_follow_the_datum_sections_own_tick_when_drawn() {
    // the rule's key sits inside the datum section, not at the form's top level, so
    // drawing the form must judge it there
    let job = fields(&schema("job_setup"));
    let colours = |use_offset: bool| {
        let mut params = registry()
            .get("job_setup")
            .map(cut_craft_core::DynCommand::default_params)
            .expect("Job Setup's form");
        params["datum"]["use_offset"] = Value::Bool(use_offset);
        let ctx = egui::Context::default();
        let picture = |_: &str| None;
        let look = Look {
            units: Units::Mm,
            picture: &picture,
            material: cut_craft_core::calc::Values::default(),
            tool: None,
            group_end: None,
            materials: None,
            pattern: None,
        };
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| show(ui, &job, &mut params, &look));
            });
        });
        out.textures_delta.clear();
        let texts: Vec<(String, egui::Color32)> = out
            .shapes
            .iter()
            .filter_map(|c| match &c.shape {
                egui::Shape::Text(t) => Some((t.galley.text().to_owned(), t.fallback_color)),
                _ => None,
            })
            .collect();
        let tick = texts
            .iter()
            .position(|(t, _)| t == "Use offset")
            .expect("the datum section's Use offset tick is drawn");
        let after = |label: &str| {
            texts[tick..]
                .iter()
                .find(|(t, _)| t == label)
                .unwrap_or_else(|| panic!("{label} drawn after Use offset: {texts:?}"))
                .1
        };
        (texts[tick].1, after("X"), after("Y"))
    };
    let (live, x, y) = colours(true);
    assert_eq!((x, y), (live, live), "ticked: X and Y can be edited");
    let (live, x, y) = colours(false);
    assert!(x != live && y != live, "unticked: X and Y are greyed");
}

#[test]
fn interactive_vector_trim_shows_no_at_position_to_type() {
    // the click on the canvas says where: typed X and Y would trim nothing
    let f = fields(&schema("interactive_vector_trim"));
    assert_eq!(keys(&f), ["reach", "rejoin_trimmed_sections"]);
}

#[test]
fn micrometres_show_in_inches_in_an_inch_job() {
    assert_eq!(shown_in("µm", Units::Inch), Some((25_400.0, "in")));
    assert_eq!(shown_in("µm", Units::Mm), None);
}

/// Every form the app draws from a schema: each command's, and the app's own dialogs'.
fn every_form() -> Vec<(String, Value)> {
    let mut forms: Vec<(String, Value)> = registry()
        .commands()
        .map(|c| (c.id().to_owned(), c.schema()))
        .collect();
    for (name, schema) in [
        ("Options", schemars::schema_for!(crate::options::Options)),
        (
            "Job sheet",
            schemars::schema_for!(fmt_create_job_sheet::Style),
        ),
        ("Part labels", schemars::schema_for!(fmt_part_labels::Style)),
        (
            "STL export",
            schemars::schema_for!(fmt_export_as_stl::Settings),
        ),
        (
            "Snapping",
            schemars::schema_for!(cut_craft_core::snap::SnapOptions),
        ),
    ] {
        forms.push((name.to_owned(), serde_json::to_value(schema).unwrap()));
    }
    forms
}

/// Each number field in `fields` (inside groups and lists too) as its path and its kind.
fn numbers<'a>(prefix: &str, fields: &'a [Field], out: &mut Vec<(String, &'a Kind)>) {
    for f in fields {
        // the Tool Database's tools are drawn by their own block and keep their own units
        if f.key == "tool" || TOOL_LISTS.contains(&f.key.as_str()) {
            continue;
        }
        let path = format!("{prefix}{}", f.key);
        let mut kind = &f.kind;
        while let Kind::List(item) = kind {
            kind = item;
        }
        match kind {
            Kind::Number { .. } => out.push((path, kind)),
            Kind::Group(inner) => numbers(&format!("{path}."), inner, out),
            _ => {}
        }
    }
}

/// Units a number may be shown in that are not lengths, so never change with mm / inches.
const NOT_LENGTHS: [&str; 14] = [
    "%",
    "min",
    "MB",
    "°",
    "deg",
    "rpm",
    "s",
    "days",
    "px",
    "frames/s",
    // a turning speed (the 3D view's Auto-rotate)
    "°/s",
    "×",
    "kerf widths",
    // a printed line's width in points, the same whatever the job's units
    "pt",
];

/// Numbers with no unit that are not measurements: counts, ids, shares and strengths. A
/// new number that is none of these needs an `x-unit` (`mm` for a length), so it is
/// shown converted in an inch job, or a place here saying why not.
const UNITLESS: [&str; 65] = [
    // counts
    "extra_passes",
    "sides",
    "points",
    "passes",
    "rows",
    "columns",
    "copies",
    "items",
    "number_of_slices",
    "lines",
    "objects",
    "rails",
    "cuts",
    "plates",
    "flutes",
    "strands",
    "digits",
    "decimal_places",
    "start",
    "step",
    "count",
    // which item: an id or a place in a list
    "layer",
    "toolpath",
    "toolpaths",
    "component",
    "item",
    "bitmap",
    "group",
    "vector",
    "sheet",
    "sources",
    "column",
    "row",
    "active",
    // shares of 1, strengths and settings with no unit
    "smoothing",
    "seed",
    "background_cut_off",
    "depth_cut_off",
    "fine_detail",
    "sharpen",
    "smoothness",
    "similarity",
    "threshold",
    "corner_fit",
    "detail_smoothness",
    "refinement",
    "tightness",
    "contrast",
    "brightness",
    "gamma",
    // the app's own settings: counts, limits and sizes on screen
    "recent_file_list_size",
    "preview_faces",
    "cores",
    "dxf_spline_detail",
    "library_most_files",
    "previews_on_card",
    "nest_most_sheets",
    "nest_most_fillers",
    "triangles",
    "speed",
    "grid_point_size",
    "picture_pixels_per_inch",
    "gadget_million_instructions",
    "gadget_most_commands",
    "grid_rows",
];

/// Numbers with no unit that are measurements of something other than the job's
/// lengths, each with why it does not change with mm / inches.
const UNITLESS_BY_FORM: [(&str, &str, &str); 11] = [
    (
        "generate_border",
        "depth_curve",
        "a power the heights are raised to",
    ),
    ("arrange_toolpaths", "order", "toolpath ids, not lengths"),
    ("rotary_machining_toolpath", "faces", "a count of sides"),
    ("rotary_machining_toolpath", "tilts", "a count of angles"),
    (
        "sketch_carving_toolpath",
        "line_thickness",
        "a strength, 0 to 100",
    ),
    (
        "laser_picture_toolpath",
        "line_spacing",
        "in widths of the laser's kerf",
    ),
    (
        "document_variables",
        "variables.value",
        "whatever the user's sums need",
    ),
    ("materials", "lithophane_light", "a share of the light"),
    ("Options", "rapid_rate", "set with its own feed-rate units"),
    ("Options", "scale_factor", "a multiplier"),
    ("Options", "web_api.port", "a network port number"),
];

#[test]
fn every_measurement_on_every_form_changes_with_the_units() {
    let mut found = vec![];
    for (form, schema) in every_form() {
        let all = fields(&schema);
        let mut listed = vec![];
        numbers("", &all, &mut listed);
        for (path, kind) in listed {
            let Kind::Number { unit, .. } = kind else {
                unreachable!()
            };
            let last = path.rsplit('.').next().unwrap_or(&path);
            match unit.as_deref() {
                // a length or a feed: shown converted in an inch job, not relabelled
                Some(unit) if !NOT_LENGTHS.contains(&unit) => {
                    if shown_in(unit, Units::Inch).is_none() {
                        found.push(format!("{form}: {path} in {unit} is not converted"));
                    }
                }
                Some(_) => {}
                None if UNITLESS.contains(&last)
                    || last.ends_with("_percent")
                    || UNITLESS_BY_FORM
                        .iter()
                        .any(|(f, p, _)| *f == form && *p == path) => {}
                None => found.push(format!(
                    "{form}: {path} has no unit: add `x-unit` (\"mm\" for a length) so \
                         it shows in inches in an inch job, or list it as unitless"
                )),
            }
        }
    }
    assert!(found.is_empty(), "{}", found.join("\n"));
}

/// The text a form shows, drawn in `units`.
fn drawn(fields: &[Field], params: &mut Value, units: Units) -> Vec<String> {
    let ctx = egui::Context::default();
    let picture = |_: &str| None;
    let look = Look {
        units,
        picture: &picture,
        material: cut_craft_core::calc::Values::default(),
        tool: None,
        group_end: None,
        materials: None,
        pattern: None,
    };
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| show(ui, fields, params, &look));
        });
    });
    out.textures_delta.clear();
    out.shapes
        .iter()
        .filter_map(|c| match &c.shape {
            egui::Shape::Text(t) => Some(t.galley.text().to_owned()),
            _ => None,
        })
        .collect()
}

#[test]
fn no_form_drawn_in_inches_shows_millimetres() {
    let mut found = vec![];
    for (form, schema) in every_form() {
        let all = fields(&schema);
        let mut params = registry().get(&form).map_or_else(
            || Value::Object(serde_json::Map::new()),
            cut_craft_core::DynCommand::default_params,
        );
        for text in drawn(&all, &mut params, Units::Inch) {
            if text.contains(" mm") || text.contains("µm") {
                found.push(format!("{form}: {text:?}"));
            }
        }
    }
    assert!(found.is_empty(), "{}", found.join("\n"));
}

#[test]
fn a_length_shows_divided_by_an_inch_and_set_size_shows_percent_as_is() {
    let f = fields(&schema("set_size"));
    let mut mm = serde_json::json!({ "width": 127.0, "height": 25.4, "percent": false });
    // a number box draws its value and its unit as two pieces of text
    let shows = |texts: &[String], value: &str, unit: &str| {
        texts.windows(2).any(|w| w[0] == value && w[1] == unit)
    };
    let texts = drawn(&f, &mut mm, Units::Inch);
    assert!(
        shows(&texts, "5", " in") && shows(&texts, "1", " in"),
        "{texts:?}"
    );
    let texts = drawn(&f, &mut mm, Units::Mm);
    assert!(
        shows(&texts, "127", " mm") && shows(&texts, "25.4", " mm"),
        "{texts:?}"
    );
    let mut share = serde_json::json!({ "width": 50.0, "height": 50.0, "percent": true });
    let texts = drawn(&f, &mut share, Units::Inch);
    assert!(shows(&texts, "50", " %"), "{texts:?}");
    assert!(!texts.iter().any(|t| t == " in"), "{texts:?}");
}

#[test]
fn a_new_variation_is_another_seed_and_the_same_one_each_time() {
    assert_ne!(next_seed(1), 1);
    assert_eq!(next_seed(1), next_seed(1));
    assert_ne!(next_seed(next_seed(1)), next_seed(1));
}

#[test]
fn toolpath_fields_are_arranged_in_the_standard_order() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "name": { "type": "string" },
            "side": { "type": "string", "x-group": "Chamfer" },
            "seed": { "type": "number" },
            "start_depth": { "type": "number", "x-group": "Cutting Depths" },
            "cut": { "type": "string", "x-group": WHICH_SIDE },
            "width": { "type": "number", "x-group": "Chamfer" },
            "ramp": { "type": "boolean", "x-tab": "Ramp" },
            "project_onto_model": { "type": "boolean" },
            "order": { "type": "boolean", "x-tab": "Order" },
            "tool": { "type": "object", "properties": {} }
        }
    });
    assert_eq!(
        keys(&arrange(fields(&schema))),
        [
            "start_depth",
            "cut",
            "tool",
            "side",
            "width",
            "seed",
            "ramp",
            "order",
            "project_onto_model",
            "name"
        ]
    );
}

/// Surfacing's What to Flatten comes before its depth: what is cut, then how deep.
#[test]
fn every_toolpath_form_opens_with_its_depths_and_tool_and_ends_with_its_name() {
    let chamfer = fields(&schema("chamfer_toolpath"));
    let width = field(&chamfer, "width");
    assert_eq!(width.group.as_deref(), Some("Chamfer"));
    assert!(
        matches!(&width.kind, Kind::Number { unit: Some(u), .. } if u == "mm"),
        "a group mark keeps the field's other marks"
    );
    for command in registry().iter() {
        let f = fields(&command.schema());
        if f.iter().all(|f| f.key != "tool") {
            continue;
        }
        let f = arrange(f);
        let k = keys(&f);
        let depths = f
            .iter()
            .take_while(|f| {
                [
                    Some(cmd_surfacing_toolpath::AREA_GROUP),
                    Some(CUTTING_DEPTHS),
                    Some(WHICH_SIDE),
                    Some(KNIFE),
                ]
                .contains(&f.group.as_deref())
            })
            .count();
        assert_eq!(k.get(depths), Some(&"tool"), "{}: {k:?}", command.id());
        if k.contains(&"name") {
            assert_eq!(k.last(), Some(&"name"), "{}", command.id());
        }
    }
    let surfacing = arrange(fields(&schema("surfacing_toolpath")));
    assert_eq!(keys(&surfacing).first(), Some(&"area"));
}

#[test]
fn the_drag_knifes_own_settings_come_right_after_its_depths() {
    let knife = arrange(fields(&schema("drag_knife_toolpath")));
    let k = keys(&knife);
    let at = |key| k.iter().position(|x| *x == key).unwrap();
    assert!(at("passes") < at("blade_offset"), "{k:?}");
    assert!(at("tolerance_angle") < at("tool"), "{k:?}");
}

#[test]
fn titles_name_fields_and_choices() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "width": { "title": "Width (X)", "type": "number" },
            "z_zero": { "oneOf": [
                { "const": "material_surface", "title": "Material Surface", "description": "Tool tip set on the top" },
                { "const": "machine_bed" }
            ]}
        }
    });
    let f = fields(&schema);
    assert_eq!(field(&f, "width").label, "Width (X)");
    let Kind::Choice(options, _) = &field(&f, "z_zero").kind else {
        panic!("a choice")
    };
    assert_eq!(options[0].label, "Material Surface");
    assert_eq!(options[0].help.as_deref(), Some("Tool tip set on the top"));
    assert_eq!(
        options[1].label, "Machine bed",
        "untitled choices fall back to the value"
    );
}

#[test]
fn chamfer_shows_angle_for_a_ball_nose_and_overcut_for_a_v_bit() {
    // a V-bit's slope is half its angle, and only a V-bit's point is moved by an overcut
    let f = fields(&schema("chamfer_toolpath"));
    let with = |shape: &str| serde_json::json!({ "tool": { "shape": shape } });
    let shown = |key: &str, shape: &str| {
        field(&f, key)
            .when
            .as_ref()
            .is_none_or(|w| w.shown(&with(shape)))
    };
    assert!(!shown("angle", "v_bit"));
    assert!(shown("angle", "ball_nose"));
    assert!(shown("overcut", "v_bit"));
    assert!(!shown("overcut", "ball_nose"));
}

#[test]
fn a_choice_can_be_offered_only_while_another_says_so() {
    // Pocket's Profile Pass: First only for Raster, adaptive clearing always goes round last
    let f = fields(&schema("pocket_toolpath"));
    let Kind::Choice(options, _) = &field(&f, "profile_pass").kind else {
        panic!("a choice")
    };
    let offered = |strategy: &str| -> Vec<String> {
        let params = serde_json::json!({ "strategy": strategy });
        offered(options, &params)
            .into_iter()
            .map(|o| o.value)
            .collect()
    };
    assert_eq!(offered("raster"), ["first", "last", "none"]);
    assert_eq!(offered("adaptive"), ["last", "none"]);
    let first = options.iter().find(|o| o.value == "first").unwrap();
    assert!(
        !first
            .help
            .as_deref()
            .unwrap_or_default()
            .contains("raster lines")
            || first.when.is_some()
    );
}

#[test]
fn a_pockets_form_offers_first_for_raster_and_never_leaves_a_hidden_pick() {
    // the form judges a choice's rule on the form's own settings, as drawn
    let f = fields(&schema("pocket_toolpath"));
    let pocket = |strategy: &str, pass: &str| {
        let mut params = registry()
            .get("pocket_toolpath")
            .map(cut_craft_core::DynCommand::default_params)
            .expect("Pocket's form");
        params["strategy"] = Value::String(strategy.into());
        params["profile_pass"] = Value::String(pass.into());
        let texts = drawn(&f, &mut params, Units::Mm);
        (texts, params["profile_pass"].clone())
    };
    let (raster, pass) = pocket("raster", "first");
    assert!(raster.iter().any(|t| t == "First"), "{raster:?}");
    assert_eq!(pass, "first", "a Raster pocket keeps First");
    let (adaptive, pass) = pocket("adaptive", "first");
    assert!(!adaptive.iter().any(|t| t == "First"), "{adaptive:?}");
    assert_eq!(
        pass, "last",
        "First switched to Adaptive shows as Last, which it cuts"
    );
}

#[test]
fn quick_engraves_old_passes_show_only_while_above_one() {
    // a job saved when Passes repeated the pattern keeps a pass count the bit's passes
    // cannot go below, so it shows until set back to 1
    let f = fields(&schema("quick_engrave_toolpath"));
    let drawn_with = |passes: u32| {
        let mut params = registry()
            .get("quick_engrave_toolpath")
            .map(cut_craft_core::DynCommand::default_params)
            .expect("Quick Engrave's form");
        params["passes"] = passes.into();
        drawn(&f, &mut params, Units::Mm)
    };
    let label = "Passes at Least";
    assert!(
        drawn_with(3).iter().any(|t| t == label),
        "{:?}",
        drawn_with(3)
    );
    assert!(!drawn_with(1).iter().any(|t| t == label));
}

#[test]
fn marked_groups_are_sections_and_marked_arrays_are_colours() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "size": { "type": "object", "x-section": true, "properties": { "w": { "type": "number" } } },
            "at": { "type": "object", "properties": { "x": { "type": "number" } } },
            "colour": { "type": "array", "items": { "type": "integer" }, "x-kind": "colour" }
        }
    });
    let f = fields(&schema);
    assert!(field(&f, "size").section);
    assert!(matches!(&field(&f, "size").kind, Kind::Group(inner) if inner.len() == 1));
    assert!(!field(&f, "at").section, "unmarked groups stay inline");
    assert_eq!(field(&f, "colour").kind, Kind::Colour);
}

#[test]
fn choices_take_a_widget_hint() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "units": { "enum": ["mm", "inch"], "x-widget": "row" },
            "resolution": { "enum": ["standard", "high"], "x-widget": "drop-down" },
            "datum": { "enum": ["bottom_left", "centre"], "x-widget": "corners" },
            "side": { "enum": ["inside", "outside"] }
        }
    });
    let f = fields(&schema);
    let widget = |key| match &field(&f, key).kind {
        Kind::Choice(_, widget) => widget.clone(),
        other => panic!("{key} is {other:?}"),
    };
    assert_eq!(widget("units"), Widget::Row);
    assert_eq!(widget("resolution"), Widget::DropDown);
    assert_eq!(widget("datum"), Widget::Corners);
    assert_eq!(widget("side"), Widget::Auto);
}

#[test]
fn fields_show_or_hide_with_another_choice() {
    let s = serde_json::json!({
        "type": "object",
        "properties": {
            "kind": { "enum": ["flat", "rotary"] },
            "diameter": { "type": "number", "x-show-when": { "kind": "rotary" } },
            "thickness": { "type": "number", "x-hide-when": { "kind": ["rotary"] } },
        }
    });
    let fields = fields(&s);
    let when = |key| field(&fields, key).when.as_ref().unwrap();
    let (flat, rotary) = (
        serde_json::json!({ "kind": "flat" }),
        serde_json::json!({ "kind": "rotary" }),
    );
    assert!(!when("diameter").shown(&flat) && when("diameter").shown(&rotary));
    assert!(when("thickness").shown(&flat) && !when("thickness").shown(&rotary));
    assert!(field(&fields, "kind").when.is_none());
}

#[test]
fn text_marked_multiline_is_a_box_of_lines() {
    let f = fields(&schema("draw_text"));
    assert_eq!(field(&f, "text").kind, Kind::Text { multiline: true });
    let job = fields(&schema("layer_properties"));
    assert_eq!(field(&job, "name").kind, Kind::Text { multiline: false });
    assert_eq!(field(&job, "material").kind, Kind::Material);
    assert_eq!(
        field(&fields(&schema("job_setup")), "material").kind,
        Kind::Material
    );
    assert!(
        matches!(&field(&f, "font").kind, Kind::Choice(fonts, Widget::DropDown) if fonts.iter().any(|c| c.value == "Test Boxes")),
        "the font list is the installed fonts"
    );
}

#[test]
fn the_corner_picker_puts_each_datum_where_it_is() {
    use eframe::egui::vec2;
    assert_eq!(corner_spot("top_right"), Some(vec2(1.0, 0.0)));
    assert_eq!(corner_spot("bottom_left"), Some(vec2(0.0, 1.0)));
    assert_eq!(corner_spot("centre"), Some(vec2(0.5, 0.5)));
    assert_eq!(corner_spot("middle_left"), None);
    let options: Vec<Choice> = ["bottom_left", "top_right", "centre"]
        .map(|v| Choice {
            value: v.into(),
            label: v.into(),
            help: None,
            when: None,
        })
        .into();
    let nearest = |x, y| nearest_corner(&options, vec2(x, y)).map(|c| c.value.as_str());
    assert_eq!(nearest(0.9, 0.2), Some("top_right"));
    assert_eq!(nearest(0.4, 0.6), Some("centre"));
    assert_eq!(nearest(0.1, 0.8), Some("bottom_left"));
}

#[test]
fn properties_forms_do_not_show_the_internal_id_of_what_they_change() {
    // the right-click menu fills the id in: a number box for it could point the form
    // at a different item
    for (id, key) in [
        ("toolpath_properties", "toolpath"),
        ("layer_properties", "layer"),
        ("component_properties", "component"),
        ("smooth_selected_components", "component"),
        ("bitmap_properties", "bitmap"),
        ("edit_picture", "bitmap"),
    ] {
        let f = fields(&schema(id));
        assert!(!keys(&f).contains(&key), "{id} shows {key}");
    }
}

#[test]
fn combine_mode_is_a_row_of_pictures() {
    let f = fields(&schema("component_properties"));
    assert!(matches!(
        &field(&f, "combine").kind,
        Kind::Choice(modes, Widget::Icons { picture: Some(p), .. })
            if modes.len() == 5 && p == "modeling/combine_{}.png"
    ));
}

#[test]
fn orientate_3d_model_is_in_sections() {
    let f = fields(&schema("orientate_3d_model"));
    let sections: Vec<&str> = f
        .iter()
        .filter(|f| f.section)
        .map(|f| f.label.as_str())
        .collect();
    assert_eq!(
        sections,
        [
            "Initial Orientation",
            "Model Size",
            "Position relative to Modeling Plane",
            "Interactive Rotation",
            "Rotation Axis Movement",
            "Model Position in Material"
        ]
    );
    let Kind::Group(orientation) = &field(&f, "orientation").kind else {
        panic!("a group")
    };
    assert!(matches!(
        &field(orientation, "face").kind,
        Kind::Choice(faces, Widget::Faces { picture: Some(p) })
            if faces.len() == 6 && p == "import/initial_orientation.png"
    ));
    assert!(matches!(
        &field(orientation, "rotation").kind,
        Kind::Number { steps, .. } if *steps == [-180.0, -90.0, 0.0, 90.0, 180.0]
    ));
}

#[test]
fn face_labels_stay_on_one_line() {
    let options: Vec<Choice> = ["top", "bottom", "front", "back", "left", "right"]
        .iter()
        .map(|v| Choice {
            value: (*v).into(),
            label: crate::gui::title_case(v),
            help: None,
            when: None,
        })
        .collect();
    let ctx = egui::Context::default();
    // bigger text, as the app's readable style has
    ctx.all_styles_mut(|s| {
        for font in s.text_styles.values_mut() {
            font.size *= 1.15;
        }
    });
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        egui::CentralPanel::default().show(ui, |ui| faces(ui, &options, "top", None));
    });
    // no renderer takes a headless frame's texture changes
    out.textures_delta.clear();
    let mut labels = 0;
    for clipped in &out.shapes {
        if let egui::Shape::Text(t) = &clipped.shape {
            assert_eq!(t.galley.rows.len(), 1, "{:?} wraps", t.galley.text());
            labels += 1;
        }
    }
    assert_eq!(labels, 6);
}

#[test]
fn faces_sit_round_the_picture() {
    // columns left, middle, right; rows above, upper, lower, below the picture
    assert_eq!(face_spot("top"), Some((1, 0)));
    assert_eq!(face_spot("back"), Some((0, 1)));
    assert_eq!(face_spot("left"), Some((0, 2)));
    assert_eq!(face_spot("right"), Some((2, 1)));
    assert_eq!(face_spot("front"), Some((2, 2)));
    assert_eq!(face_spot("bottom"), Some((1, 3)));
    assert_eq!(face_spot("centre"), None);
}

#[test]
fn steps_are_labelled_with_their_sign() {
    assert_eq!(step_label(-90.0), "-90");
    assert_eq!(step_label(0.0), "0");
    assert_eq!(step_label(180.0), "+180");
    assert_eq!(step_label(22.5), "+22.5");
}

#[test]
fn section_pictures_follow_the_choice_made() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "z_zero": { "enum": ["material_surface", "machine_bed"], "x-icon": "job_setup/z_zero_{}.png" }
        }
    });
    assert_eq!(
        field(&fields(&schema), "z_zero").icon.as_deref(),
        Some("job_setup/z_zero_{}.png")
    );
    let pick = |template, value| icon_path(template, &value);
    assert_eq!(
        pick("job_setup/z_zero_{}.png", serde_json::json!("machine_bed")),
        "job_setup/z_zero_machine_bed.png"
    );
    assert_eq!(
        pick(
            "job_setup/xy_datum_{position}.png",
            serde_json::json!({ "position": "centre" })
        ),
        "job_setup/xy_datum_centre.png"
    );
    assert_eq!(
        pick(
            "job_setup/material_size_xyz.png",
            serde_json::json!({ "width": 1 })
        ),
        "job_setup/material_size_xyz.png"
    );
}

#[test]
fn lengths_and_feeds_show_in_the_units_chosen() {
    assert_eq!(shown_in("mm", Units::Inch), Some((25.4, "in")));
    assert_eq!(shown_in("mm/min", Units::Inch), Some((25.4, "in/min")));
    assert_eq!(shown_in("mm", Units::Mm), None);
    assert_eq!(shown_in("rpm", Units::Inch), None);
    let job_setup = serde_json::json!({ "size": { "width": 10.0, "units": "Inch" } });
    assert_eq!(units_chosen(&job_setup), Some(Units::Inch));
    assert_eq!(units_chosen(&serde_json::json!({ "width": 1.0 })), None);
}

#[test]
fn a_metric_tool_leaves_an_inch_jobs_toolpath_form_in_inches() {
    // a Profile form in an inch job with a 6 mm end mill picked: the bit's own units are
    // the bit's, so cut depth, allowance and leads stay in the job's inches
    let tool = cut_craft_core::toolpath::Tool {
        units: Units::Mm,
        ..Default::default()
    };
    let profile = serde_json::json!({
        "cut_depth": 6.35,
        "allowance": 0.0,
        "tool": serde_json::to_value(&tool).expect("a tool serializes"),
        "tools": [serde_json::to_value(&tool).expect("a tool serializes")],
    });
    assert_eq!(units_chosen(&profile), None);
}

#[test]
fn fields_keep_declaration_order() {
    let f = fields(&schema("draw_rectangle"));
    assert_eq!(keys(&f), ["x", "y", "width", "height", "corners", "radius"]);
}

#[test]
fn lengths_are_numbers_in_mm_with_their_limits() {
    let f = fields(&schema("draw_rectangle"));
    assert_eq!(
        field(&f, "width").kind,
        Kind::Number {
            unit: Some("mm".into()),
            min: Some(0.0),
            max: None,
            integer: false,
            steps: vec![],
            slider: false,
            percent_when: None,
        }
    );
    assert_eq!(
        field(&f, "x").help.as_deref(),
        Some("Where the left edge sits, along the job's X axis")
    );
}

#[test]
fn enums_become_choices_with_readable_labels() {
    let f = fields(&schema("draw_rectangle"));
    let Kind::Choice(options, _) = &field(&f, "corners").kind else {
        panic!("corners should be a choice")
    };
    let labels: Vec<&str> = options.iter().map(|c| c.label.as_str()).collect();
    assert_eq!(labels, ["Square", "Rounded", "Notched"]);
    assert_eq!(options[2].value, "notched");
}

#[test]
fn shared_point_params_become_a_group_of_x_and_y() {
    let f = fields(&schema("draw_circle"));
    let Kind::Group(inner) = &field(&f, "center").kind else {
        panic!("center should be a group")
    };
    assert_eq!(keys(inner), ["x", "y"]);
}

#[test]
fn counts_are_integers_with_their_range() {
    let f = fields(&schema("draw_polygon"));
    assert_eq!(
        field(&f, "sides").kind,
        Kind::Number {
            unit: None,
            min: Some(3.0),
            max: Some(100.0),
            integer: true,
            steps: vec![],
            slider: false,
            percent_when: None,
        }
    );
}

#[test]
fn lists_of_points_and_flags() {
    let f = fields(&schema("draw_polyline"));
    assert!(
        matches!(&field(&f, "points").kind, Kind::List(item) if matches!(**item, Kind::Group(_)))
    );
    assert_eq!(field(&f, "closed").kind, Kind::Bool);
}

#[test]
fn labels_are_sentence_case() {
    let f = fields(&schema("draw_star"));
    assert_eq!(field(&f, "outer_radius").label, "Outer radius");
}

/// A form drawn with every picture there, its text in order and the pictures asked for.
fn drawn_with_pictures(fields: &[Field], params: &mut Value) -> (Vec<String>, Vec<String>) {
    let ctx = egui::Context::default();
    let asked = std::cell::RefCell::new(vec![]);
    let picture = |path: &str| {
        asked.borrow_mut().push(path.to_owned());
        Some(egui::Image::new(egui::include_image!(
            "../../../../assets/icons/drawing_forms/width.png"
        )))
    };
    let look = Look {
        units: Units::Mm,
        picture: &picture,
        material: cut_craft_core::calc::Values::default(),
        tool: None,
        group_end: None,
        materials: None,
        pattern: None,
    };
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        egui::CentralPanel::default().show(ui, |ui| show(ui, fields, params, &look));
    });
    out.textures_delta.clear();
    let texts = out
        .shapes
        .iter()
        .filter_map(|c| match &c.shape {
            egui::Shape::Text(t) => Some(t.galley.text().to_owned()),
            _ => None,
        })
        .collect();
    (texts, asked.into_inner())
}

fn lettered() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "cut_depth": { "type": "number", "x-unit": "mm", "x-group": "Cutting Depths",
                           "x-dim": "D", "x-diagram": "diagrams/test.png" },
            "swivel_depth": { "type": "number", "x-unit": "mm", "x-group": "Cutting Depths",
                              "x-dim": "D2" },
            "passes": { "type": "integer", "x-group": "Cutting Depths" }
        }
    })
}

#[test]
fn a_measurements_letter_is_read_from_its_schema() {
    let f = fields(&lettered());
    assert_eq!(field(&f, "cut_depth").dim.as_deref(), Some("D"));
    assert_eq!(field(&f, "swivel_depth").dim.as_deref(), Some("D2"));
    assert_eq!(field(&f, "passes").dim, None);
    assert_eq!(
        field(&f, "cut_depth").diagram.as_deref(),
        Some("diagrams/test.png")
    );
}

#[test]
fn a_groups_picture_shows_once_at_its_top_and_each_letter_before_its_name() {
    let f = fields(&lettered());
    let mut params = serde_json::json!({ "cut_depth": 3.0, "swivel_depth": 0.5, "passes": 1 });
    let (texts, mut asked) = drawn_with_pictures(&f, &mut params);
    // egui may lay a first frame out twice
    asked.dedup();
    assert_eq!(asked, ["diagrams/test.png"], "{texts:?}");
    let at = |t: &str| texts.iter().position(|x| x == t);
    let (d2, swivel) = (at("D2").unwrap(), at("Swivel depth").unwrap());
    assert_eq!(
        swivel,
        d2 + 1,
        "the letter sits right before its name: {texts:?}"
    );
    assert!(at("Cutting Depths").unwrap() < at("D").unwrap());
}

#[test]
fn every_toolpath_depth_has_a_letter_and_its_group_a_picture() {
    let mut found = vec![];
    for (form, schema) in every_form() {
        let all = fields(&schema);
        let depths: Vec<&Field> = all
            .iter()
            .filter(|f| f.group.as_deref() == Some(CUTTING_DEPTHS))
            .collect();
        if depths.is_empty() {
            continue;
        }
        if !depths.iter().any(|f| f.diagram.is_some()) {
            found.push(format!(
                "{form}: Cutting Depths has no picture (`x-diagram`)"
            ));
        }
        for f in depths {
            let length = matches!(&f.kind, Kind::Number { unit: Some(u), .. } if u == "mm");
            if length && f.dim.is_none() {
                found.push(format!("{form}: {} has no letter (`x-dim`)", f.key));
            }
        }
    }
    assert!(found.is_empty(), "{}", found.join("\n"));
}

/// Every lettered field, with the picture shown above it (its group's, its own).
/// A field outside any group may be marked on a picture shown higher up the form, above
/// an earlier field outside a group (Set Size's H, under the W that shows the picture).
fn lettered_fields<'a>(fields: &'a [Field], above: bool, out: &mut Vec<(&'a Field, bool)>) {
    for (i, f) in fields.iter().enumerate() {
        if f.dim.is_some() {
            let pictured = above
                || f.diagram.is_some()
                || f.group.as_ref().is_some_and(|g| {
                    fields
                        .iter()
                        .any(|o| o.group.as_ref() == Some(g) && o.diagram.is_some())
                })
                || (f.group.is_none()
                    && fields[..i]
                        .iter()
                        .any(|o| o.group.is_none() && o.diagram.is_some()));
            out.push((f, pictured));
        }
        if let Kind::Group(inner) = &f.kind {
            lettered_fields(inner, above || f.diagram.is_some(), out);
        }
    }
}

/// Whether `a` and `b` are never shown together: one shows on the values the other hides on.
fn apart(a: &Field, b: &Field) -> bool {
    matches!((&a.when, &b.when), (Some(x), Some(y))
            if x.key == y.key && x.values == y.values && x.show != y.show)
}

#[test]
fn letters_are_unique_in_a_form_and_each_is_on_a_picture_above_it() {
    let mut found = vec![];
    for (form, schema) in every_form() {
        let all = fields(&schema);
        let mut listed = vec![];
        lettered_fields(&all, false, &mut listed);
        let mut seen = std::collections::BTreeMap::new();
        for (f, pictured) in listed {
            let dim = f.dim.as_deref().unwrap_or_default();
            // two settings never shown together may share a letter (V-Carve's flat depth
            // for Fill, its groove depth otherwise)
            if let Some(other) = seen.insert(dim, f)
                && !apart(other, f)
            {
                found.push(format!(
                    "{form}: {dim} names both {} and {}",
                    other.key, f.key
                ));
            }
            if !pictured {
                found.push(format!("{form}: {} has letter {dim} but no picture", f.key));
            }
        }
    }
    assert!(found.is_empty(), "{}", found.join("\n"));
}

#[test]
fn settings_that_do_nothing_in_the_current_mode_are_hidden() {
    let shown = |id: &str, key: &str, params: Value| {
        let f = fields(&schema(id));
        field(&f, key)
            .when
            .as_ref()
            .is_none_or(|w| w.shown(&params))
    };
    let count = serde_json::json!({ "spacing": "count" });
    let distance = serde_json::json!({ "spacing": "distance" });
    assert!(shown("copy_along_vectors", "copies", count.clone()));
    assert!(!shown("copy_along_vectors", "copies", distance.clone()));
    for key in ["distance", "force_even_spacing"] {
        assert!(shown("copy_along_vectors", key, distance.clone()), "{key}");
        assert!(!shown("copy_along_vectors", key, count.clone()), "{key}");
    }
    let relative = |r: bool| serde_json::json!({ "relative": r, "rotated": true });
    assert!(!shown("move", "anchor", relative(true)));
    assert!(shown("move", "anchor", relative(false)));
    assert!(
        !shown("move", "rotated", relative(false)),
        "a tick box rule as a bool"
    );
    assert!(shown("move", "angle", relative(true)));
    assert!(
        !shown("move", "angle", relative(false)),
        "the angle only turns relative moves"
    );
    let sharp = |k: bool| serde_json::json!({ "keep_sharp_corners": k });
    assert!(shown("fit_curves_to_vectors", "max_angle", sharp(true)));
    assert!(!shown("fit_curves_to_vectors", "max_angle", sharp(false)));
}

#[test]
fn a_list_of_objects_starts_with_a_blank_one() {
    let f = fields(&schema("segment_3d_model"));
    let Kind::List(item) = &field(&f, "further_cuts").kind else {
        panic!("further cuts should be a list")
    };
    assert_eq!(
        blank(item),
        serde_json::json!({ "segment": "", "axis": "x", "position": 0.0 })
    );
}

#[test]
fn a_label_can_follow_another_choice() {
    let s = serde_json::json!({
        "type": "object",
        "properties": {
            "shape": { "enum": ["round", "square", "board"] },
            "diameter": {
                "type": "number",
                "title": "Diameter (D)",
                "x-titles": { "shape": { "square": "Side (D)", "board": "Thickness (D)" } }
            },
            "length": { "type": "number", "title": "Length (L)" }
        }
    });
    let f = fields(&s);
    let label = |key, shape: &str| {
        let params = serde_json::json!({ "shape": shape });
        field(&f, key).label_for(&params, &Value::Null).to_owned()
    };
    assert_eq!(label("diameter", "round"), "Diameter (D)");
    assert_eq!(label("diameter", "square"), "Side (D)");
    assert_eq!(label("diameter", "board"), "Thickness (D)");
    assert_eq!(label("length", "board"), "Length (L)", "no titles: its own");
}

#[test]
fn job_setup_s_rotary_size_is_named_for_the_material_s_shape() {
    let f = fields(&schema("job_setup"));
    let Kind::Group(size) = &field(&f, "size").kind else {
        panic!("size is a section");
    };
    let label = |shape: &str| {
        let params = serde_json::json!({ "shape": shape });
        field(size, "diameter")
            .label_for(&params, &Value::Null)
            .to_owned()
    };
    assert_eq!(label("round"), "Diameter (D)");
    assert_eq!(label("square"), "Side (D)");
    assert_eq!(label("board"), "Thickness (D)");
}
