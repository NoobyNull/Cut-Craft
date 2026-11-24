#[test]
fn a_tool_number_another_bit_has_is_said_in_red_but_0_is_shared_freely() {
    let bit = |name: &str, number: u32| Tool {
        name: name.into(),
        number,
        ..Tool::default()
    };
    let tools = vec![
        bit("6 mm End Mill", 3),
        bit("60° V-Bit", 3),
        bit("1/4\" Ball Nose", 0),
        bit("1/8\" End Mill", 0),
    ];
    assert_eq!(sharing_number(&tools, 0), ["60° V-Bit"]);
    assert!(sharing_number(&tools, 2).is_empty(), "0: changed by hand");
    let note = number_taken(3, &sharing_number(&tools, 0)).unwrap();
    assert!(
        note.starts_with("Bit number 3 is also 60° V-Bit's. A tool changer"),
        "{note}"
    );
    assert_eq!(number_taken(3, &[]), None);
}

#[test]
fn renames_are_kept_from_the_first_name_to_the_last() {
    let mut m = super::Manager::new(&cut_craft_core::toolpath::ToolDatabase::default());
    // typed a letter at a time
    m.note_rename("6 mm", "6 mm A");
    m.note_rename("6 mm A", "6 mm AT");
    m.note_rename("V-Bit", "V");
    m.note_rename("V", "V-Bit");
    assert_eq!(
        m.take_renamed(),
        vec![("6 mm".to_owned(), "6 mm AT".to_owned())]
    );
    assert_eq!(m.take_renamed(), vec![]);
}

use super::*;

fn frame(manager: &mut Manager, ctx: &egui::Context) -> Option<Outcome> {
    let look = Look {
        picture: &|_| None,
        profile: None,
    };
    let mut outcome = None;
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1200.0, 800.0),
        )),
        ..Default::default()
    };
    // no renderer takes a headless frame's texture changes
    ctx.run_pass(input, |ctx| outcome = manager.show(ctx, &look))
        .textures_delta
        .clear();
    outcome
}

#[test]
fn a_tapered_ball_nose_is_a_ball_nose_with_a_taper_and_a_wider_shank() {
    let mut tool = Tool {
        shape: ToolShape::EndMill,
        diameter: 0.5,
        ..Tool::default()
    };
    make_tapered_ball(&mut tool);
    assert_eq!(tool.shape, ToolShape::BallNose);
    assert!(tapered_ball(&tool));
    assert!(tool.tapered().is_some(), "it cuts as a tapered tool");
    assert_eq!(picture_path(&tool), "tool_database/tapered_ball_nose.png");
    // its own taper and shank are kept
    tool.taper = 4.7;
    tool.shank_diameter = 6.0;
    make_tapered_ball(&mut tool);
    assert!((tool.taper - 4.7).abs() < 1e-9 && (tool.shank_diameter - 6.0).abs() < 1e-9);
}

#[test]
fn drawing_each_tool_keeps_none_of_the_drawing_s_guesses() {
    let ctx = egui::Context::default();
    let db = ToolDatabase::starter();
    let mut m = Manager::new(&db);
    for i in 0..db.tools.len() {
        m.pick = Some(Pick::Tool(i));
        frame(&mut m, &ctx);
        frame(&mut m, &ctx);
    }
    // the starter tools give no flute or shank length; drawn, they are still unset
    assert_eq!(m.draft.tools, db.tools);
}

#[test]
fn the_drawing_sits_right_of_tool_info_when_both_fit() {
    let (info, card) = split(1200.0, 8.0).expect("wide: side by side");
    assert_eq!(card, SHAPE_MAX);
    assert_eq!(info + card + 8.0, 1200.0);
    let (info, card) = split(820.0, 8.0).expect("still fits");
    assert!(info >= INFO_MIN && card >= SHAPE_MIN, "{info} {card}");
    assert_eq!(split(700.0, 8.0), None, "narrow: the drawing goes under");
}

#[test]
fn the_window_draws_groups_tools_and_holds_its_size() {
    let ctx = egui::Context::default();
    let mut m = Manager::new(&ToolDatabase::starter());
    frame(&mut m, &ctx);
    m.pick = Some(Pick::Tool(0));
    frame(&mut m, &ctx);
    let first = ctx.memory(|mem| mem.area_rect(tool_window()));
    assert!(first.is_some(), "the window is drawn");
    for _ in 0..3 {
        frame(&mut m, &ctx);
    }
    let later = ctx.memory(|mem| mem.area_rect(tool_window()));
    assert_eq!(first, later, "panels inside the window do not grow it");
    m.pick = Some(Pick::Group("Metric Tools".into()));
    assert_eq!(frame(&mut m, &ctx), None);
}

#[test]
fn a_material_added_becomes_active_and_the_window_draws_its_data() {
    let ctx = egui::Context::default();
    let mut m = Manager::new(&ToolDatabase::starter());
    m.draft.add_material("Oak");
    m.new_material = " oak ".into();
    m.add_material();
    assert_eq!(
        (m.draft.materials.len(), m.draft.material.as_str()),
        (1, "Oak"),
        "the listed one, whatever the case"
    );
    assert_eq!(m.new_material, "");
    let thread_mill = m
        .draft
        .tools
        .iter()
        .position(|t| t.shape == ToolShape::ThreadMill)
        .unwrap();
    m.pick = Some(Pick::Tool(thread_mill));
    frame(&mut m, &ctx);
    assert!(
        m.draft.tools[thread_mill].materials.is_empty(),
        "only drawing it makes no data"
    );
}

#[test]
fn a_material_is_duplicated_with_its_cutting_data_and_settings_copy_between_materials() {
    let mut m = Manager::new(&ToolDatabase::starter());
    m.draft.add_material("Oak");
    m.draft.material = "Oak".into();
    let mut oak = m.draft.tools[0].cutting("");
    oak.material = "Oak".into();
    oak.feed_rate = 777.0;
    m.draft.tools[0].set_cutting(oak);
    m.new_material = " Ash ".into();
    m.duplicate_material();
    assert_eq!(m.draft.materials, ["Oak", "Ash"]);
    assert_eq!(
        (m.draft.material.as_str(), m.new_material.as_str()),
        ("Ash", "")
    );
    assert!((m.draft.tools[0].cutting("Ash").feed_rate - 777.0).abs() < 1e-9);
    assert!(
        !m.draft.tools[1].has_data_for("Ash"),
        "no Oak data, none copied"
    );
    // without the data, named after the original, and made unique
    m.draft.material = "Oak".into();
    m.copy_cutting_data = false;
    m.duplicate_material();
    assert_eq!(m.draft.materials, ["Oak", "Ash", "Oak copy"]);
    assert!(!m.draft.tools[0].has_data_for("Oak copy"));
    m.draft.material = "Oak".into();
    m.duplicate_material();
    assert_eq!(m.draft.material, "Oak copy 2");
    // Copy Settings From another material, or from the default
    let tool = &mut m.draft.tools[0];
    tool.copy_cutting("Oak", "Oak copy");
    assert!((tool.cutting("Oak copy").feed_rate - 777.0).abs() < 1e-9);
    tool.copy_cutting("", "Oak");
    assert!((tool.cutting("Oak").feed_rate - tool.feed_rate).abs() < 1e-9);
}

/// Every piece of text one frame draws.
fn drawn(manager: &mut Manager, ctx: &egui::Context) -> Vec<String> {
    let look = Look {
        picture: &|_| None,
        profile: None,
    };
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1200.0, 800.0),
        )),
        ..Default::default()
    };
    let mut output = ctx.run_pass(input, |ctx| {
        manager.show(ctx, &look);
    });
    output.textures_delta.clear();
    output
        .shapes
        .iter()
        .filter_map(|c| match &c.shape {
            egui::Shape::Text(t) => Some(t.galley.text().to_owned()),
            _ => None,
        })
        .collect()
}

#[test]
fn the_search_box_lists_the_tools_it_finds_by_name() {
    let ctx = egui::Context::default();
    let mut m = Manager::new(&ToolDatabase::starter());
    let all: Vec<String> = m.draft.tools.iter().map(|t| t.name.clone()).collect();
    let ball: Vec<&String> = all.iter().filter(|n| n.contains("Ball")).collect();
    assert!(!ball.is_empty() && ball.len() < all.len());
    m.search = "ball".into();
    drawn(&mut m, &ctx);
    let shown = drawn(&mut m, &ctx);
    for name in &all {
        assert_eq!(shown.contains(name), name.contains("Ball"), "{name}");
    }
    // a renamed tool is found by its new name
    m.draft.tools[0].name = "Zebra Cutter".into();
    m.search = "zeb".into();
    assert!(drawn(&mut m, &ctx).contains(&"Zebra Cutter".to_owned()));
}

#[test]
fn tools_without_data_for_the_material_are_greyed_or_hidden() {
    let ctx = egui::Context::default();
    let mut m = Manager::new(&ToolDatabase::starter());
    m.draft.add_material("Oak");
    m.draft.material = "Oak".into();
    let mut oak = m.draft.tools[0].cutting("");
    oak.material = "Oak".into();
    m.draft.tools[0].set_cutting(oak);
    let (set, unset) = (m.draft.tools[0].name.clone(), m.draft.tools[1].name.clone());
    assert!(m.draft.tools[0].has_data_for("Oak") && !m.draft.tools[1].has_data_for("oak"));
    assert!(
        m.draft.tools[1].has_data_for(""),
        "the default always has data"
    );
    drawn(&mut m, &ctx);
    let shown = drawn(&mut m, &ctx);
    assert!(
        shown.contains(&set) && shown.contains(&unset),
        "greyed, still listed"
    );
    m.hide_unset = true;
    let shown = drawn(&mut m, &ctx);
    assert!(shown.contains(&set) && !shown.contains(&unset), "hidden");
}

#[test]
fn edits_shown_for_a_material_become_its_data_and_leave_the_default() {
    let mut tool = Tool::default();
    let mut shown = tool.for_material("Oak");
    let before = shown.cutting("");
    keep_cutting(&mut tool, &shown, "Oak", &before);
    assert!(tool.materials.is_empty(), "nothing changed, nothing kept");
    shown.feed_rate = 600.0;
    shown.rate_units = RateUnits::InchPerMin;
    keep_cutting(&mut tool, &shown, "Oak", &before);
    assert!((tool.cutting("Oak").feed_rate - 600.0).abs() < 1e-9);
    assert!((tool.feed_rate - 1000.0).abs() < 1e-9, "the default stays");
    assert_eq!(
        tool.rate_units,
        RateUnits::InchPerMin,
        "units are the tool's"
    );
    let mut default = tool.for_material("");
    let before = default.cutting("");
    default.plunge_rate = 50.0;
    keep_cutting(&mut tool, &default, "", &before);
    assert!((tool.plunge_rate - 50.0).abs() < 1e-9);
    assert_eq!(tool.materials.len(), 1);
}

#[test]
fn select_tool_opens_on_the_forms_tool_and_hands_back_the_one_shown() {
    let db = ToolDatabase::starter();
    let name = db.tools[1].name.clone();
    let mut m = Manager::selecting(&db, &name);
    assert!(m.selecting);
    assert_eq!(m.pick, Some(Pick::Tool(1)), "shown first");
    assert_eq!(m.select(), Some(Outcome::Select(name)));
    m.pick = Some(Pick::Tool(0));
    assert_eq!(m.select(), Some(Outcome::Select(db.tools[0].name.clone())));
    // a group shown, nothing to select
    m.pick = Some(Pick::Group(String::new()));
    assert_eq!(m.select(), None);
    assert_eq!(Manager::selecting(&db, "no such tool").pick, None);
    assert!(!Manager::new(&db).selecting);
}

#[test]
fn tools_load_into_one_slot_each_and_follow_renames_and_deletes() {
    let mut changer = Changer {
        machine: "Router".into(),
        slots: vec![None; 5],
    };
    changer.load("6 mm End Mill", Some(2));
    changer.load("3 mm End Mill", Some(2));
    assert_eq!(
        changer.slot_of("3 mm End Mill"),
        Some(2),
        "it takes the slot"
    );
    assert_eq!(
        changer.slot_of("6 mm End Mill"),
        None,
        "what was there comes out"
    );
    changer.load("3 mm End Mill", Some(5));
    assert_eq!(
        (changer.slots[1].clone(), changer.slot_of("3 mm End Mill")),
        (None, Some(5))
    );
    changer.load("3 mm End Mill", Some(9));
    assert_eq!(
        changer.slot_of("3 mm End Mill"),
        None,
        "no such slot: unloaded"
    );
    let mut m = Manager::new(&ToolDatabase::starter());
    let name = m.draft.tools[0].name.clone();
    changer.load(&name, Some(1));
    m.changer = Some(changer);
    m.pick = Some(Pick::Tool(0));
    m.act(TreeAction::Delete);
    assert_eq!(
        m.changer.as_ref().unwrap().slots[0],
        None,
        "a deleted tool leaves its slot"
    );
}

#[test]
fn a_tool_loaded_into_a_slot_takes_the_slot_as_its_number() {
    let mut m = Manager::new(&ToolDatabase::starter());
    m.changer = Some(Changer {
        machine: "Router".into(),
        slots: vec![None; 6],
    });
    m.draft.tools[1].number = 9;
    m.load_into(1, Some(4));
    assert_eq!(m.draft.tools[1].number, 4, "T4 holds it: tool 4");
    assert_eq!(
        m.changer.as_ref().unwrap().slot_of(&m.draft.tools[1].name),
        Some(4)
    );
    m.load_into(1, Some(2));
    assert_eq!(m.draft.tools[1].number, 2, "moved to T2");
    m.load_into(1, None);
    assert_eq!(
        m.draft.tools[1].number, 2,
        "taken out, it keeps its last number"
    );
}

#[test]
fn deleting_a_group_says_how_many_of_the_job_s_toolpaths_cut_with_its_tools() {
    let db = ToolDatabase::starter();
    let mut m = Manager::new(&db);
    let used = db.tools[0].clone();
    m.pick = Some(Pick::Group(used.group.clone()));
    m.act(TreeAction::Delete);
    let question = m.delete_question().expect("it asks");
    assert!(!question.contains("toolpath"), "none use them: {question}");
    m.job_tools = vec![used.name.clone(), used.name.clone(), "Elsewhere".into()];
    let question = m.delete_question().expect("it asks");
    assert!(
        question.contains("2 toolpaths in this job cut with them"),
        "{question}"
    );
}

#[test]
fn a_question_never_moves_the_tree_or_the_buttons_under_it() {
    let ctx = egui::Context::default();
    let mut m = Manager::new(&ToolDatabase::starter());
    let group = m.draft.tools[0].group.clone();
    m.pick = Some(Pick::Group(group));
    frame(&mut m, &ctx);
    frame(&mut m, &ctx);
    let before = m.tree_rect;
    m.act(TreeAction::Delete);
    assert!(m.delete_question().is_some());
    frame(&mut m, &ctx);
    frame(&mut m, &ctx);
    assert_eq!(m.tree_rect, before, "the delete question has its own area");
    m.answer_delete(false);
    m.draft.tools.remove(0);
    m.open_materials_library();
    assert!(m.library_question().is_some());
    frame(&mut m, &ctx);
    frame(&mut m, &ctx);
    assert_eq!(m.tree_rect, before, "and so has Materials Library…'s");
}

#[test]
fn deleting_a_group_asks_first_naming_its_hidden_and_loaded_tools() {
    let db = ToolDatabase::starter();
    let mut m = Manager::new(&db);
    let group = "Metric Tools".to_owned();
    let inside: Vec<String> = db
        .tools
        .iter()
        .filter(|t| tree::within(&t.group, &group))
        .map(|t| t.name.clone())
        .collect();
    assert!(inside.len() > 1);
    m.changer = Some(Changer {
        machine: "Router".into(),
        slots: vec![Some(inside[0].clone()), None],
    });
    m.hide_unset = true;
    m.draft.material = "No Such Material".into();
    m.pick = Some(Pick::Group(group.clone()));
    m.act(TreeAction::Delete);
    assert_eq!(m.draft.tools.len(), db.tools.len(), "nothing gone yet");
    let question = m.delete_question().expect("it asks");
    assert!(question.contains("Metric Tools"), "{question}");
    assert!(
        question.contains(&format!("{} bits", inside.len())),
        "{question}"
    );
    assert!(question.contains("hidden"), "{question}");
    assert!(question.contains("tool changer"), "{question}");
    m.answer_delete(false);
    assert_eq!(m.draft.tools.len(), db.tools.len(), "Keep keeps them");
    assert!(m.delete_question().is_none());
    m.act(TreeAction::Delete);
    m.answer_delete(true);
    assert!(
        m.draft
            .tools
            .iter()
            .all(|t| !tree::within(&t.group, &group))
    );
    assert_eq!(
        m.changer.as_ref().unwrap().slots[0],
        None,
        "its slot is emptied"
    );
}

#[test]
fn the_materials_library_button_asks_first_when_it_would_keep_deletions() {
    let db = ToolDatabase::starter();
    let mut m = Manager::new(&db);
    // nothing deleted: it opens, keeping the edits as Apply
    m.open_materials_library();
    assert!(matches!(m.pending.take(), Some(Outcome::MaterialsLibrary)));
    let gone = m.draft.tools.remove(0).name;
    m.open_materials_library();
    assert!(m.pending.is_none(), "it asks first");
    let question = m.library_question().expect("a question");
    assert!(question.contains("1 deleted bit"), "{question}");
    assert!(question.contains(&gone), "{question}");
    m.answer_library(false);
    assert!(m.pending.is_none() && m.library_question().is_none());
    assert_eq!(m.draft.tools.len(), db.tools.len() - 1, "the edits stay");
    m.open_materials_library();
    m.answer_library(true);
    assert!(matches!(m.pending.take(), Some(Outcome::MaterialsLibrary)));
    // once applied, the deletion is kept already: no question
    m.applied();
    m.open_materials_library();
    assert!(matches!(m.pending.take(), Some(Outcome::MaterialsLibrary)));
}

#[test]
fn a_renamed_tool_is_not_asked_about_as_deleted() {
    let mut m = Manager::new(&ToolDatabase::starter());
    let before = m.draft.tools[0].name.clone();
    m.draft.tools[0].name = "Renamed".into();
    m.note_rename(&before, "Renamed");
    m.open_materials_library();
    assert!(
        matches!(m.pending.take(), Some(Outcome::MaterialsLibrary)),
        "{:?}",
        m.library_question()
    );
    // renamed and then deleted: deleted
    m.draft.tools.remove(0);
    m.open_materials_library();
    let question = m.library_question().expect("a question");
    assert!(question.contains(&before), "{question}");
}

#[test]
fn tree_buttons_add_copy_and_delete_in_the_copy_only() {
    let db = ToolDatabase::starter();
    let mut m = Manager::new(&db);
    m.pick = Some(Pick::Group("Metric Tools/End Mills".into()));
    m.act(TreeAction::NewTool);
    let Some(Pick::Tool(i)) = m.pick else {
        panic!("the new tool is picked")
    };
    assert_eq!(m.draft.tools[i].group, "Metric Tools/End Mills");
    m.act(TreeAction::Copy);
    assert_eq!(m.draft.tools.len(), db.tools.len() + 2);
    m.act(TreeAction::Delete);
    assert_eq!(m.draft.tools.len(), db.tools.len() + 1);
    m.pick = Some(Pick::Group("Imperial Tools".into()));
    let Some(Outcome::Export(out)) = m.act(TreeAction::Export) else {
        panic!("exports")
    };
    assert!(!out.tools.is_empty() && out.tools.iter().all(|t| t.group.starts_with("Imperial")));
    assert_eq!(m.act(TreeAction::Import), Some(Outcome::Import));
    m.import(out, "Vendor");
    assert_eq!(m.pick, Some(Pick::Group("Catalogues/Vendor".into())));
    assert_eq!(
        ToolDatabase::starter(),
        db,
        "the database itself is untouched"
    );
}

/// The starter tools as My Bits, with an "Amana" catalogue of two tools; the
/// catalogue's first tool's index.
fn with_catalogue() -> (Manager, usize) {
    let mut db = ToolDatabase::starter();
    let catalogue = ToolDatabase {
        tools: vec![
            Tool {
                name: "46171".into(),
                group: "Up-cut".into(),
                ..Tool::default()
            },
            Tool {
                name: "45700".into(),
                shape: ToolShape::VBit,
                ..Tool::default()
            },
        ],
        ..ToolDatabase::default()
    };
    let first = db.tools.len();
    tree::import(&mut db, catalogue, "Amana");
    (Manager::new(&db), first)
}

#[test]
fn a_catalogue_tool_is_copied_into_my_tools_not_edited_or_loaded() {
    let (mut m, i) = with_catalogue();
    let before = m.draft.tools.len();
    m.changer = Some(Changer {
        machine: "Router".into(),
        slots: vec![None; 4],
    });
    m.load_into(i, Some(1));
    assert_eq!(
        m.changer.as_ref().unwrap().slots[0],
        None,
        "a catalogue's tool is not loaded"
    );
    m.pick = Some(Pick::Tool(i));
    m.act(TreeAction::Copy);
    let Some(Pick::Tool(mine)) = m.pick else {
        panic!("the copy is picked")
    };
    assert_eq!(m.draft.tools.len(), before + 1);
    assert_eq!(m.draft.tools[mine].name, "46171");
    assert_eq!(m.draft.tools[mine].group, "Amana/End Mills");
    m.load_into(mine, Some(1));
    assert_eq!(m.changer.as_ref().unwrap().slot_of("46171"), Some(1));
    // a catalogue tool or its groups are not deleted one by one; the catalogue is
    m.pick = Some(Pick::Tool(i));
    m.act(TreeAction::Delete);
    assert_eq!(m.draft.tools.len(), before + 1);
    m.pick = Some(Pick::Group("Catalogues/Amana".into()));
    m.act(TreeAction::Delete);
    m.answer_delete(true);
    assert_eq!(
        m.draft.tools.len(),
        before - 1,
        "the catalogue goes, the copy stays"
    );
    assert_eq!(m.draft.catalogues, Vec::<String>::new());
    assert_eq!(m.changer.as_ref().unwrap().slot_of("46171"), Some(1));
}

#[test]
fn new_tools_and_groups_picked_in_a_catalogue_go_in_my_tools() {
    let (mut m, i) = with_catalogue();
    m.pick = Some(Pick::Tool(i));
    m.act(TreeAction::NewTool);
    let Some(Pick::Tool(new)) = m.pick else {
        panic!("the new tool is picked")
    };
    assert!(!m.draft.is_catalogue(&m.draft.tools[new]));
    assert_eq!(m.draft.tools[new].group, "Generic/End Mills");
    m.pick = Some(Pick::Group("Catalogues/Amana".into()));
    m.act(TreeAction::NewGroup);
    let Some(Pick::Group(g)) = m.pick.clone() else {
        panic!("the new group is picked")
    };
    assert!(!m.draft.is_catalogue_group(&g), "{g}");
    let mine = m.draft.my_tools().count();
    m.copy_group_to_mine("Catalogues/Amana");
    m.copy_group_to_mine("Catalogues");
    assert_eq!(m.draft.my_tools().count(), mine + 2, "each copied once");
}

#[test]
fn selecting_a_catalogue_tool_copies_it_into_my_tools() {
    let (m, i) = with_catalogue();
    let mut m = Manager::selecting(&m.draft, "46171");
    assert_eq!(m.pick, Some(Pick::Tool(i)));
    let before = m.draft.tools.len();
    assert_eq!(m.select(), Some(Outcome::Select("46171".into())));
    assert_eq!(m.draft.tools.len(), before + 1);
    let mine = m.draft.index_of("46171").unwrap();
    assert!(!m.draft.is_catalogue(&m.draft.tools[mine]));
    assert_eq!(m.pick, Some(Pick::Tool(mine)), "the copy is shown");
    assert_eq!(
        Manager::selecting(&m.draft, "46171").pick,
        Some(Pick::Tool(mine)),
        "the copy opens, not the catalogue's"
    );
}

#[test]
fn a_tool_s_group_follows_its_maker_and_type_until_moved() {
    let ctx = egui::Context::default();
    let (mut m, i) = with_catalogue();
    m.pick = Some(Pick::Tool(i));
    m.act(TreeAction::Copy);
    let Some(Pick::Tool(mine)) = m.pick else {
        panic!()
    };
    frame(&mut m, &ctx);
    m.draft.tools[mine].manufacturer = "Amana Tool".into();
    m.draft.tools[mine].shape = ToolShape::BallNose;
    frame(&mut m, &ctx);
    assert_eq!(m.draft.tools[mine].group, "Amana Tool/Ball Noses");
    m.draft.tools[mine].group = "Favourites".into();
    frame(&mut m, &ctx);
    m.draft.tools[mine].manufacturer = "Whiteside".into();
    frame(&mut m, &ctx);
    assert_eq!(
        m.draft.tools[mine].group, "Favourites",
        "moved by hand: stays"
    );
    // a catalogue's tool is drawn read-only, and the window still draws it
    m.pick = Some(Pick::Tool(i));
    frame(&mut m, &ctx);
    assert_eq!(m.draft.tools[i].group, "Catalogues/Amana/Up-cut");
}

/// A laser toolpath's Select… offers only lasers, any other toolpath's only bits; the Bit
/// Library opened on its own (or for a holder) shows every tool.
#[test]
fn a_toolpath_forms_select_offers_only_the_kind_of_tool_it_cuts_with() {
    let laser = Tool {
        name: "5 W Laser".into(),
        shape: ToolShape::Laser,
        ..Tool::default()
    };
    let bit = Tool {
        name: "6 mm End Mill".into(),
        ..Tool::default()
    };
    let mut m = Manager::new(&ToolDatabase::default());
    assert!(m.may_pick(&laser) && m.may_pick(&bit));
    m.lasers = Some(true);
    assert!(m.may_pick(&laser) && !m.may_pick(&bit));
    m.lasers = Some(false);
    assert!(!m.may_pick(&laser) && m.may_pick(&bit));
}
