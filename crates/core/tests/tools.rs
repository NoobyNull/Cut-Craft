use cut_craft_core::toolpath::{CuttingData, RateUnits, Tool, ToolDatabase, ToolShape};

fn tool(name: &str, diameter: f64) -> Tool {
    Tool {
        name: name.into(),
        diameter,
        ..Tool::default()
    }
}

#[test]
fn the_starter_database_has_uniquely_named_tools_including_the_default() {
    let db = ToolDatabase::starter();
    assert!(db.tools.len() > 1);
    let mut names: Vec<&str> = db.tools.iter().map(|t| t.name.as_str()).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), db.tools.len());
    // the default tool's settings, filed in the starter groups
    let mut default = db
        .get(&Tool::default().name)
        .cloned()
        .expect("the default tool");
    assert_eq!(default.group, "Metric Tools/End Mills");
    default.group.clear();
    assert_eq!(default, Tool::default());
}

#[test]
fn putting_a_tool_replaces_the_one_with_its_name_or_adds_it() {
    let mut db = ToolDatabase::default();
    db.put(tool("A", 3.0));
    db.put(tool("B", 6.0));
    db.put(tool("A", 4.0));
    assert_eq!(db.tools, vec![tool("A", 4.0), tool("B", 6.0)]);
}

#[test]
fn the_database_round_trips_through_json() {
    let db = ToolDatabase::starter();
    let back = ToolDatabase::from_json(&db.to_json()).unwrap();
    assert_eq!(back, db);
    assert!(ToolDatabase::from_json(b"not json").is_err());
}

#[test]
fn the_starter_database_has_each_shape_of_tool() {
    let db = ToolDatabase::starter();
    for shape in [
        ToolShape::EndMill,
        ToolShape::BallNose,
        ToolShape::VBit,
        ToolShape::RadiusedEndMill,
    ] {
        assert!(db.tools.iter().any(|t| t.shape == shape), "{shape:?}");
    }
}

#[test]
fn tool_profiles_rise_from_the_tip() {
    let tool = |shape| Tool {
        shape,
        diameter: 6.0,
        angle: 60.0,
        ..Tool::default()
    };
    assert_eq!(tool(ToolShape::EndMill).profile(2.0), Some(0.0));
    let ball = tool(ToolShape::BallNose).profile(3.0).unwrap();
    assert!((ball - 3.0).abs() < 1e-12, "a ball's edge is a radius up");
    let v = tool(ToolShape::VBit).profile(1.0).unwrap();
    assert!((v - 1.0 / 30f64.to_radians().tan()).abs() < 1e-12);
    assert_eq!(
        tool(ToolShape::EndMill).profile(3.5),
        None,
        "beyond the tool"
    );
}

#[test]
fn tools_saved_before_shapes_existed_are_end_mills() {
    let mut json = serde_json::to_value(Tool::default()).unwrap();
    let fields = json.as_object_mut().unwrap();
    fields.remove("shape");
    fields.remove("angle");
    let tool: Tool = serde_json::from_value(json).unwrap();
    assert_eq!(tool.shape, ToolShape::EndMill);
}

#[test]
fn a_radiused_end_mill_is_flat_then_rounds_up_by_its_tip_radius() {
    let tool = Tool {
        shape: ToolShape::RadiusedEndMill,
        diameter: 6.0,
        tip_radius: 1.0,
        ..Tool::default()
    };
    assert_eq!(tool.profile(1.5), Some(0.0), "flat across 4 mm");
    let half = tool.profile(2.5).unwrap();
    assert!((half - (1.0 - 0.75f64.sqrt())).abs() < 1e-12, "{half}");
    assert!(
        (tool.profile(3.0).unwrap() - 1.0).abs() < 1e-12,
        "a tip radius up at the edge"
    );
}

#[test]
fn tools_saved_before_tip_radii_get_one_millimetre() {
    let mut json = serde_json::to_value(Tool::default()).unwrap();
    json.as_object_mut()
        .unwrap()
        .remove("tip_radius")
        .expect("tip radius is saved");
    let tool: Tool = serde_json::from_value(json).unwrap();
    assert!((tool.tip_radius - 1.0).abs() < 1e-12);
}

/// A 1 mm tapered ball nose: 10° each side, widening to a 4 mm shank.
fn tapered_ball() -> Tool {
    Tool {
        shape: ToolShape::BallNose,
        diameter: 1.0,
        taper: 10.0,
        shank_diameter: 4.0,
        ..Tool::default()
    }
}

#[test]
fn a_tapered_ball_nose_rises_up_its_taper_to_the_shank() {
    let tool = tapered_ball();
    let a = 10f64.to_radians();
    // the ball meets the taper where it is as steep as the taper
    let (turn, rise) = (0.5 * a.cos(), 0.5 - 0.5 * a.sin());
    let ball = tool.profile(0.3).unwrap();
    assert!(
        (ball - (0.5 - (0.25f64 - 0.09).sqrt())).abs() < 1e-12,
        "{ball}"
    );
    assert!((tool.profile(turn).unwrap() - rise).abs() < 1e-12);
    let cone = tool.profile(1.5).unwrap();
    assert!(
        (cone - (rise + (1.5 - turn) / a.tan())).abs() < 1e-12,
        "{cone}"
    );
    assert!((tool.reach() - 2.0).abs() < 1e-12, "out to the shank");
    assert!(tool.profile(2.0).is_some() && tool.profile(2.01).is_none());
    // a ball nose without a taper ends at its diameter, whatever its shank
    let plain = Tool {
        taper: 0.0,
        ..tapered_ball()
    };
    assert!((plain.reach() - 0.5).abs() < 1e-12);
    assert_eq!(plain.profile(0.6), None);
}

#[test]
fn a_tapered_flat_bottom_is_flat_then_rises_up_its_taper() {
    let tool = Tool {
        shape: ToolShape::EndMill,
        ..tapered_ball()
    };
    assert_eq!(tool.profile(0.4), Some(0.0));
    let cone = tool.profile(1.5).unwrap();
    assert!(
        (cone - 1.0 / 10f64.to_radians().tan()).abs() < 1e-12,
        "{cone}"
    );
}

#[test]
fn tools_saved_before_tapers_are_straight_with_no_flute_length() {
    let mut json = serde_json::to_value(Tool::default()).unwrap();
    let fields = json.as_object_mut().unwrap();
    for field in ["taper", "shank_diameter", "flute_length"] {
        fields.remove(field);
    }
    let tool: Tool = serde_json::from_value(json).unwrap();
    assert_eq!(
        (tool.taper, tool.shank_diameter, tool.flute_length),
        (0.0, 0.0, 0.0)
    );
}

/// A 6 mm end mill with its own cutting data for oak.
fn with_oak() -> Tool {
    let mut tool = Tool::default();
    tool.set_cutting(CuttingData {
        material: "Oak".into(),
        feed_rate: 600.0,
        spindle_speed: 16000.0,
        ..Tool::default().cutting("")
    });
    tool
}

#[test]
fn a_tool_cuts_each_material_with_its_own_data_else_its_default() {
    let tool = with_oak();
    assert_eq!(tool.feed_rate, 1000.0, "the default data is the tool's own");
    let oak = tool.for_material("oak");
    assert_eq!(
        (oak.feed_rate, oak.spindle_speed),
        (600.0, 16000.0),
        "matched whatever the case"
    );
    assert_eq!(oak.pass_depth, tool.pass_depth);
    assert_eq!(
        tool.for_material("MDF").feed_rate,
        1000.0,
        "no data: the default"
    );
    assert_eq!(tool.for_material("").feed_rate, 1000.0);
    // a material with no data shows the default's, under its own name
    let mdf = tool.cutting("MDF");
    assert_eq!((mdf.material.as_str(), mdf.feed_rate), ("MDF", 1000.0));
}

#[test]
fn setting_cutting_data_changes_the_materials_set_or_the_default() {
    let mut tool = with_oak();
    tool.set_cutting(CuttingData {
        feed_rate: 700.0,
        ..tool.cutting("Oak")
    });
    assert_eq!(
        tool.materials.len(),
        1,
        "oak's set is changed, not added again"
    );
    assert_eq!(tool.cutting("Oak").feed_rate, 700.0);
    tool.set_cutting(CuttingData {
        plunge_rate: 123.0,
        ..tool.cutting("")
    });
    assert_eq!(
        tool.plunge_rate, 123.0,
        "the default is the tool's own fields"
    );
    assert_eq!(tool.materials.len(), 1);
}

#[test]
fn a_tool_picked_from_the_database_cuts_with_the_active_material() {
    let mut db = ToolDatabase {
        tools: vec![with_oak()],
        ..ToolDatabase::default()
    };
    let name = Tool::default().name;
    assert_eq!(db.pick(&name).unwrap().feed_rate, 1000.0);
    db.material = "Oak".into();
    assert_eq!(db.pick(&name).unwrap().feed_rate, 600.0);
    assert!(db.pick("no such tool").is_none());
}

#[test]
fn a_picked_tool_put_back_keeps_its_data_as_the_active_materials() {
    let mut db = ToolDatabase {
        tools: vec![with_oak()],
        material: "Oak".into(),
        ..ToolDatabase::default()
    };
    let name = Tool::default().name;
    let mut tool = db.pick(&name).unwrap();
    tool.feed_rate = 650.0;
    db.put_picked(tool);
    let stored = db.get(&name).unwrap();
    assert_eq!(stored.cutting("Oak").feed_rate, 650.0);
    assert_eq!(stored.feed_rate, 1000.0, "the default stays");
    // with no material active it is the default that changes
    db.material.clear();
    let mut tool = db.pick(&name).unwrap();
    tool.feed_rate = 900.0;
    db.put_picked(tool);
    let stored = db.get(&name).unwrap();
    assert_eq!(
        (stored.feed_rate, stored.cutting("Oak").feed_rate),
        (900.0, 650.0)
    );
}

#[test]
fn materials_are_added_once_and_deleted_with_their_data() {
    let mut db = ToolDatabase {
        tools: vec![with_oak()],
        ..ToolDatabase::default()
    };
    db.add_material("Oak");
    db.add_material("oak");
    db.add_material("MDF");
    db.add_material("");
    assert_eq!(db.materials, ["Oak", "MDF"]);
    db.material = "Oak".into();
    db.delete_material("Oak");
    assert_eq!(db.materials, ["MDF"]);
    assert!(
        db.tools[0].materials.is_empty(),
        "its cutting data goes too"
    );
    assert_eq!(db.material, "", "the default is active again");
}

#[test]
fn tool_databases_saved_before_materials_load_with_none() {
    let mut json = serde_json::to_value(ToolDatabase::starter()).unwrap();
    for tool in json["tools"].as_array_mut().unwrap() {
        let fields = tool.as_object_mut().unwrap();
        for field in ["rate_units", "materials", "tooth_size", "tooth_offset"] {
            fields.remove(field);
        }
    }
    let db: ToolDatabase = serde_json::from_value(json).unwrap();
    assert!(db.materials.is_empty() && db.material.is_empty());
    assert!(db.tools.iter().all(|t| t.materials.is_empty()));
    assert_eq!(db.tools[0].rate_units, RateUnits::MmPerMin);
}

#[test]
fn rate_units_convert_to_millimetres_a_minute() {
    let per_unit = RateUnits::mm_per_min;
    assert!((per_unit(RateUnits::MmPerSec) - 60.0).abs() < 1e-9);
    assert!((per_unit(RateUnits::MmPerMin) - 1.0).abs() < 1e-9);
    assert!((per_unit(RateUnits::MPerMin) - 1000.0).abs() < 1e-9);
    assert!((per_unit(RateUnits::InchPerSec) - 1524.0).abs() < 1e-9);
    assert!((per_unit(RateUnits::InchPerMin) - 25.4).abs() < 1e-9);
    assert!((per_unit(RateUnits::FeetPerMin) - 304.8).abs() < 1e-9);
    let labels: Vec<&str> = RateUnits::ALL.iter().map(|u| u.label()).collect();
    assert_eq!(
        labels,
        [
            "mm/sec",
            "mm/min",
            "m/min",
            "inches/sec",
            "inches/min",
            "feet/min"
        ]
    );
}

/// A 10 mm, 60° single-point thread mill: a 1 mm tooth whose tip is 2 mm up.
fn thread_mill() -> Tool {
    Tool {
        shape: ToolShape::ThreadMill,
        diameter: 10.0,
        angle: 60.0,
        tooth_size: 1.0,
        tooth_offset: 2.0,
        ..Tool::default()
    }
}

#[test]
fn a_thread_mills_tooth_height_follows_its_size_and_angle() {
    let tool = thread_mill();
    let height = 2.0 * 30f64.to_radians().tan();
    assert!((tool.tooth_height() - height).abs() < 1e-12);
    let mut taller = tool.clone();
    taller.set_tooth_height(2.0 * height);
    assert!(
        (taller.tooth_size - 2.0).abs() < 1e-12,
        "the angle stays, so the size changes"
    );
    assert!((taller.angle - 60.0).abs() < 1e-12);
}

#[test]
fn a_thread_mill_is_flat_under_its_core_then_rises_to_its_tooth_tip() {
    let tool = thread_mill();
    let half_height = 30f64.to_radians().tan();
    assert_eq!(tool.profile(3.5), Some(0.0), "the core, 8 mm across");
    let flank = tool.profile(4.5).unwrap();
    assert!((flank - (2.0 - half_height + 0.5 * half_height)).abs() < 1e-12);
    assert!(
        (tool.profile(5.0).unwrap() - 2.0).abs() < 1e-12,
        "the tooth tip"
    );
    assert_eq!(tool.profile(5.1), None);
}

#[test]
fn the_starter_database_has_a_thread_mill() {
    let db = ToolDatabase::starter();
    let tool = db
        .tools
        .iter()
        .find(|t| t.shape == ToolShape::ThreadMill)
        .expect("a thread mill");
    assert!(tool.tooth_size > 0.0 && tool.tooth_offset >= tool.tooth_size / 2.0);
}
