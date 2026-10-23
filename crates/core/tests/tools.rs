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
    assert_eq!(db.get(&Tool::default().name), Some(&Tool::default()));
}

#[test]
fn putting_a_tool_replaces_the_one_with_its_name_or_adds_it() {
    let mut db = ToolDatabase { tools: vec![] };
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
    for shape in [ToolShape::EndMill, ToolShape::BallNose, ToolShape::VBit] {
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
