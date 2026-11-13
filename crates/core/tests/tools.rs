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
fn tools_saved_before_units_read_in_millimetres() {
    let mut json = serde_json::to_value(Tool::default()).unwrap();
    json.as_object_mut()
        .unwrap()
        .remove("units")
        .expect("units are saved");
    let tool: Tool = serde_json::from_value(json).unwrap();
    assert_eq!(tool.units, cut_craft_core::document::Units::Mm);
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

#[test]
fn tools_saved_with_one_stepover_finish_with_it_too() {
    let mut json = serde_json::to_value(Tool {
        stepover: 0.3,
        ..Tool::default()
    })
    .unwrap();
    json.as_object_mut().unwrap().remove("finish_stepover");
    // cutting data for a material saved before the split
    json["materials"] = serde_json::json!([{
        "material": "Oak", "pass_depth": 2.0, "stepover": 0.25,
        "feed_rate": 600.0, "plunge_rate": 200.0, "spindle_speed": 16000.0
    }]);
    let tool: Tool = serde_json::from_value(json).unwrap();
    assert_eq!(tool.stepover, 0.3, "the clearing stepover is the old one");
    assert_eq!(tool.finishing_stepover(), 0.3, "and so is the finishing");
    let oak = tool.for_material("Oak");
    assert_eq!((oak.stepover, oak.finishing_stepover()), (0.25, 0.25));
}

#[test]
fn a_tool_clears_and_finishes_with_its_own_stepovers() {
    let mut tool = Tool {
        stepover: 0.4,
        finish_stepover: 0.1,
        ..Tool::default()
    };
    assert_eq!((tool.stepover, tool.finishing_stepover()), (0.4, 0.1));
    // a material's set keeps both
    tool.set_cutting(CuttingData {
        material: "Oak".into(),
        stepover: 0.3,
        finish_stepover: 0.05,
        ..tool.cutting("")
    });
    let oak = tool.for_material("Oak");
    assert_eq!((oak.stepover, oak.finishing_stepover()), (0.3, 0.05));
    assert_eq!(tool.cutting("").finish_stepover, 0.1);
    assert_eq!(tool.cutting("MDF").finish_stepover, 0.1, "the default's");
    // and it round-trips
    let back: Tool = serde_json::from_str(&serde_json::to_string(&tool).unwrap()).unwrap();
    assert_eq!(back, tool);
}

#[test]
fn starter_ball_noses_clear_coarse_and_finish_fine() {
    let db = ToolDatabase::starter();
    let ball = db.get("6 mm Ball Nose").unwrap();
    assert!(
        ball.stepover > ball.finishing_stepover(),
        "{} then {}",
        ball.stepover,
        ball.finishing_stepover()
    );
    assert_eq!(ball.finishing_stepover(), 0.1);
}

#[test]
fn tools_loaded_in_a_tool_changer_take_their_slot_as_their_number() {
    let mut db = ToolDatabase {
        tools: vec![tool("A", 3.0), tool("B", 6.0), tool("C", 12.0)],
        ..ToolDatabase::default()
    };
    db.tools[2].number = 7;
    let mut slots = vec![None, Some("B".to_owned()), Some("A".to_owned()), None];
    assert!(db.number_by_slots(&slots), "numbers changed");
    let numbers: Vec<u32> = db.tools.iter().map(|t| t.number).collect();
    assert_eq!(numbers, [3, 2, 7], "a tool not loaded keeps its own");
    assert!(!db.number_by_slots(&slots), "nothing more to change");
    // A taken out keeps its last number while T3 is empty
    slots[2] = None;
    db.number_by_slots(&slots);
    assert_eq!(db.tools[0].number, 3);
    // C put in T3: A gives up 3, so no two tools share a number
    slots[2] = Some("C".to_owned());
    db.number_by_slots(&slots);
    let numbers: Vec<u32> = db.tools.iter().map(|t| t.number).collect();
    assert_eq!(numbers, [0, 2, 3]);
}

#[test]
fn tools_saved_before_drawn_profiles_have_no_neck_or_shank_length() {
    let mut json = serde_json::to_value(Tool::default()).unwrap();
    let fields = json.as_object_mut().unwrap();
    assert!(!fields.contains_key("neck_length"), "0 is not written");
    assert!(!fields.contains_key("shank_length"));
    let tool: Tool = serde_json::from_value(json).unwrap();
    assert_eq!((tool.neck_length, tool.shank_length), (0.0, 0.0));
    let set = Tool {
        neck_length: 4.0,
        shank_length: 25.4,
        ..Tool::default()
    };
    let back: Tool = serde_json::from_str(&serde_json::to_string(&set).unwrap()).unwrap();
    assert_eq!((back.neck_length, back.shank_length), (4.0, 25.4));
}

#[test]
fn a_flute_length_is_known_only_when_one_was_given() {
    let mut t = Tool::default();
    assert_eq!(t.known_flute_length(), None, "0 is not set");
    t.flute_length = 22.225;
    assert_eq!(t.known_flute_length(), Some(22.225));
    for unset in [-1.0, f64::NAN, f64::INFINITY] {
        t.flute_length = unset;
        assert_eq!(t.known_flute_length(), None, "{unset}");
    }
}

#[test]
fn catalogue_tools_are_not_my_tools_and_old_databases_are_all_mine() {
    let mut db = ToolDatabase {
        tools: vec![
            Tool {
                group: "Metric Tools/End Mills".into(),
                ..tool("mine", 6.0)
            },
            Tool {
                group: "Amana/End Mills".into(),
                ..tool("theirs", 6.35)
            },
            Tool {
                group: "Amana".into(),
                ..tool("theirs too", 3.0)
            },
        ],
        ..ToolDatabase::default()
    };
    assert_eq!(
        db.my_tools().count(),
        3,
        "no catalogues: every tool is mine"
    );
    let json = serde_json::to_value(&db).unwrap();
    assert!(json.get("catalogues").is_none(), "nothing written for none");
    db.catalogues.push("Amana".into());
    let mine: Vec<&str> = db.my_tools().map(|t| t.name.as_str()).collect();
    assert_eq!(mine, ["mine"]);
    assert!(db.is_catalogue(&db.tools[1]) && db.is_catalogue(&db.tools[2]));
    assert!(!db.is_catalogue(&db.tools[0]));
    // a group only starting with the catalogue's name is not in it
    let near = Tool {
        group: "Amana Copies".into(),
        ..tool("near", 1.0)
    };
    assert!(!db.is_catalogue(&near));
    let back: ToolDatabase = serde_json::from_value(serde_json::to_value(&db).unwrap()).unwrap();
    assert_eq!(back.catalogues, ["Amana"]);
}

#[test]
fn a_tool_s_own_group_is_its_maker_then_its_type() {
    let amana = Tool {
        manufacturer: "Amana".into(),
        shape: ToolShape::VBit,
        ..tool("v", 12.7)
    };
    assert_eq!(amana.own_group(), "Amana/V-Bits");
    assert_eq!(tool("plain", 6.0).own_group(), "Generic/End Mills");
    let bowl = Tool {
        shape: ToolShape::RadiusedEndMill,
        manufacturer: "  Whiteside ".into(),
        ..tool("bowl", 25.4)
    };
    assert_eq!(bowl.own_group(), "Whiteside/Radiused End Mills");
}

#[test]
fn a_catalogue_tool_copied_into_my_tools_keeps_its_name_and_the_copy_is_used() {
    let theirs = Tool {
        group: "Amana/End Mills".into(),
        feed_rate: 1000.0,
        ..tool("46171 End Mill", 6.35)
    };
    let mut db = ToolDatabase {
        tools: vec![theirs.clone()],
        catalogues: vec!["Amana".into()],
        ..ToolDatabase::default()
    };
    // until copied, the catalogue's is the one found
    assert_eq!(db.index_of("46171 End Mill"), Some(0));
    db.tools.push(Tool {
        group: "Amana/End Mills".into(),
        ..theirs.clone()
    });
    db.tools[1].group = "Mine".into();
    db.tools[1].feed_rate = 1500.0;
    assert_eq!(db.index_of("46171 End Mill"), Some(1), "My Tools first");
    assert_eq!(db.get("46171 End Mill").unwrap().feed_rate, 1500.0);
    // a put changes the copy, never the catalogue
    let mut changed = db.get("46171 End Mill").unwrap().clone();
    changed.feed_rate = 1800.0;
    db.put(changed);
    assert_eq!(db.tools[0].feed_rate, 1000.0);
    assert_eq!(db.tools[1].feed_rate, 1800.0);
    // a slot numbers the copy only
    db.number_by_slots(&[Some("46171 End Mill".into())]);
    assert_eq!((db.tools[0].number, db.tools[1].number), (0, 1));
}

#[test]
fn putting_a_catalogue_tool_back_adds_it_to_my_tools() {
    let mut db = ToolDatabase {
        tools: vec![Tool {
            group: "Catalogues/Amana/V-Bits".into(),
            shape: ToolShape::VBit,
            manufacturer: "Amana".into(),
            ..tool("V", 12.7)
        }],
        catalogues: vec!["Catalogues/Amana".into()],
        ..ToolDatabase::default()
    };
    let mut edited = db.tools[0].clone();
    edited.angle = 60.0;
    db.put(edited);
    assert_eq!(db.tools.len(), 2);
    assert_eq!(db.tools[0].angle, 90.0, "the catalogue is unchanged");
    assert_eq!(db.tools[1].group, "Amana/V-Bits");
    assert!(
        !db.is_catalogue(&db.tools[1]),
        "My Tools' Amana is not the catalogue"
    );
}

#[test]
fn the_starter_tools_have_a_surfacing_bit_that_cuts_flat_and_steps_over_wide() {
    let db = ToolDatabase::starter();
    let bit = db.get("1 in Surfacing Bit").unwrap();
    assert_eq!(bit.shape, ToolShape::SurfacingBit);
    assert!(bit.shape.is_flat());
    assert_eq!(bit.group, "Imperial Tools/Surfacing Bits");
    assert!(bit.stepover >= 0.7);
    assert_eq!(
        bit.profile(bit.diameter / 2.0 - 0.1),
        Some(0.0),
        "flat to its edge"
    );
}
