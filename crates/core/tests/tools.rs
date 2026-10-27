use cut_craft_core::toolpath::{Tool, ToolDatabase, ToolShape};

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
