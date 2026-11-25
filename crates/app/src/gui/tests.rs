use super::MenuItem;

#[test]
fn tools_without_a_crate_still_say_what_they_do() {
    // a flyout's variant of a drawing tool, and an icon opening another tool's form
    for id in [
        "draw_circle_from_edge",
        "draw_rectangle_from_center",
        "create_shape_round",
        "create_shape_angled",
        "create_shape_smooth",
        "create_shape_flat",
        "edit_text",
    ] {
        let about = super::about(id).unwrap_or_default();
        assert!(about.split_whitespace().count() > 6, "{id}: {about:?}");
    }
}

#[test]
fn status_text_goes_on_one_line() {
    assert_eq!(super::one_line("Ready"), "Ready");
    assert_eq!(
        super::one_line("0331. Flowers.STL\nDouble-click to add it\r\n\n  "),
        "0331. Flowers.STL · Double-click to add it"
    );
}

#[test]
fn separators_split_a_menu_into_categories() {
    let items = [
        MenuItem::separator(),
        MenuItem::with_action("Undo", "undo"),
        MenuItem::with_action("Redo", "redo"),
        MenuItem::separator(),
        MenuItem::separator(),
        MenuItem::with_action("Delete", "delete"),
    ];
    let names: Vec<Vec<&str>> = super::categories(&items)
        .iter()
        .map(|c| c.iter().map(|i| i.name.as_str()).collect())
        .collect();
    assert_eq!(names, [vec!["Undo", "Redo"], vec!["Delete"]]);
}

#[test]
fn every_design_group_is_on_exactly_one_sub_tab_or_always_shown() {
    let toolbar = super::Toolbar::new();
    let placed: Vec<&str> = super::ALWAYS_SHOWN
        .iter()
        .chain(super::DESIGN_SUBTABS.iter().flat_map(|t| t.groups.iter()))
        .copied()
        .collect();
    for group in toolbar.all_design_groups() {
        let n = placed.iter().filter(|p| **p == group.name).count();
        assert_eq!(n, 1, "{} is placed {n} times", group.name);
    }
    for name in &placed {
        assert!(
            toolbar.all_design_groups().iter().any(|g| g.name == *name),
            "{name} is no group"
        );
    }
    assert_eq!(
        super::DESIGN_SUBTABS[0].name,
        "Create",
        "drawing comes first"
    );
}

#[test]
fn every_toolpath_group_is_on_exactly_one_sub_tab_or_always_shown() {
    let toolbar = super::Toolbar::new();
    let placed: Vec<&str> = super::TOOLPATH_ALWAYS_SHOWN
        .iter()
        .chain(super::TOOLPATH_SUBTABS.iter().flat_map(|t| t.groups.iter()))
        .copied()
        .collect();
    for group in &toolbar.toolpath_groups {
        let n = placed.iter().filter(|p| **p == group.name).count();
        assert_eq!(n, 1, "{} is placed {n} times", group.name);
    }
    for name in &placed {
        assert!(
            toolbar.toolpath_groups.iter().any(|g| g.name == *name),
            "{name} is no group"
        );
    }
    let names: Vec<&str> = super::TOOLPATH_SUBTABS.iter().map(|t| t.name).collect();
    assert_eq!(
        names,
        ["Create", "Preview", "Analyse", "Output"],
        "in the order a job goes"
    );
}

/// Regrouping the Toolpaths page (Matthew, 2026-11-13) lost no tool and put none twice.
#[test]
fn the_toolpaths_page_keeps_every_tool_it_had() {
    let before = [
        "material_setup",
        "materials",
        "profile_toolpath",
        "pocket_toolpath",
        "drilling_toolpath",
        "drill_bank_toolpath",
        "quick_engrave_toolpath",
        "vcarve_engrave_toolpath",
        "vcarve_inlay_toolpath",
        "inlay_toolpath",
        "chamfer_toolpath",
        "fluting_toolpath",
        "prism_toolpath",
        "thread_milling_toolpath",
        "keyhole_toolpath",
        "drag_knife_toolpath",
        "moulding_toolpath",
        "texture_toolpath",
        "photo_vcarve_toolpath",
        "sketch_carving_toolpath",
        "surfacing_toolpath",
        "3d_roughing_toolpath",
        "3d_finishing_toolpath",
        "3d_pencil_toolpath",
        "rounding_toolpath",
        "rotary_roughing_toolpath",
        "rotary_finishing_toolpath",
        "rotary_machining_toolpath",
        "rotary_lathe_toolpath",
        "laser_cut_fill_toolpath",
        "laser_picture_toolpath",
        "array_copy_toolpath",
        "merged_toolpath",
        "load_template",
        "save_selected_toolpath_as_template",
        "save_all_visible_toolpaths_as_template",
        "save_toolpaths",
        "toolpath_summary",
        "tool_database",
        "keep_out_zones",
        "toolpath_tiling",
        "create_job_sheet",
        "print_job_sheet",
        "part_labels",
        "cut_list",
    ];
    let mut now: Vec<&str> = super::toolpath_groups()
        .iter()
        .flat_map(|g| g.tools.iter().copied())
        .collect();
    let count = now.len();
    now.sort_unstable();
    now.dedup();
    assert_eq!(now.len(), count, "no tool twice");
    let mut before = before.to_vec();
    before.sort_unstable();
    assert_eq!(now, before);
}

#[test]
fn a_toolpath_tool_is_found_on_its_sub_tab() {
    let toolbar = super::Toolbar::new();
    let tab = |name: &str| super::TOOLPATH_SUBTABS.iter().position(|t| t.name == name);
    assert_eq!(
        toolbar.toolpath_subtab_of("profile_toolpath", 0),
        tab("Create")
    );
    assert_eq!(
        toolbar.toolpath_subtab_of("3d_roughing_toolpath", 3),
        tab("Create")
    );
    assert_eq!(
        toolbar.toolpath_subtab_of("merged_toolpath", 0),
        tab("Create")
    );
    assert_eq!(
        toolbar.toolpath_subtab_of("toolpath_summary", 0),
        tab("Analyse")
    );
    assert_eq!(
        toolbar.toolpath_subtab_of("save_toolpaths", 0),
        tab("Output")
    );
    assert_eq!(toolbar.toolpath_subtab_of("part_labels", 0), tab("Output"));
    assert_eq!(
        toolbar.toolpath_subtab_of("tool_database", 0),
        None,
        "always shown"
    );
    assert_eq!(toolbar.toolpath_subtab_of("draw_circle", 0), None);
}

#[test]
fn toolpath_sub_tabs_move_out_of_the_row_apart_from_the_design_pages() {
    let none: Vec<String> = vec![];
    let save = super::TOOLPATHS
        .placed(
            &none,
            &format!("{}Output:section", super::TOOLPATH_SUBTAB_PLACE),
        )
        .expect("a toolpaths action");
    assert_eq!(save, ["Output"]);
    assert_eq!(super::TOOLPATHS.layout(&save), (vec![0, 1, 2], vec![3]));
    assert_eq!(
        super::TOOLPATHS.open(3, &save),
        Some(0),
        "Save is not in the row"
    );
    // each page's actions are its own
    assert!(
        super::DESIGN
            .placed(
                &none,
                &format!("{}Output:section", super::TOOLPATH_SUBTAB_PLACE)
            )
            .is_none()
    );
    assert!(
        super::TOOLPATHS
            .placed(&none, &format!("{}Model:section", super::SUBTAB_PLACE))
            .is_none()
    );
    let menu = super::TOOLPATHS.menu("Preview", &save);
    assert!(menu[0].action.starts_with(super::TOOLPATH_SUBTAB_PLACE));
}

#[test]
fn a_sub_tab_shows_its_own_groups_and_far_fewer_buttons() {
    let toolbar = super::Toolbar::new();
    let names =
        |tab: usize| -> Vec<&str> { toolbar.subtab_groups(tab).iter().map(|g| g.name).collect() };
    assert_eq!(
        names(2),
        ["Align Objects", "Offset and Layout", "Array Objects"]
    );
    assert!(names(99).is_empty(), "a tab out of range shows nothing");
    let always: Vec<&str> = toolbar
        .named(super::ALWAYS_SHOWN)
        .iter()
        .map(|g| g.name)
        .collect();
    assert_eq!(always, super::ALWAYS_SHOWN);
    let faces = crate::flyout::Faces::new();
    let count = |groups: Vec<super::ToolGroup>| -> usize {
        groups
            .iter()
            .map(|g| crate::flyout::buttons(g.tools, &faces).len())
            .sum()
    };
    let whole = count(toolbar.all_design_groups());
    for tab in 0..super::DESIGN_SUBTABS.len() {
        let mut shown = toolbar.named(super::ALWAYS_SHOWN);
        shown.extend(toolbar.subtab_groups(tab));
        let n = count(shown);
        // every tool has a button somewhere (Matthew, 2026-11-10), which took the Model
        // sub-tab past the 36 the others keep under; Generate Border made it 41
        assert!(
            n <= 41 && n * 2 < whole,
            "{}: {n} of {whole}",
            super::DESIGN_SUBTABS[tab].name
        );
    }
}

#[test]
fn the_import_sub_tab_gathers_every_import() {
    let toolbar = super::Toolbar::new();
    let tab = super::DESIGN_SUBTABS
        .iter()
        .position(|t| t.name == "Import")
        .expect("an Import sub-tab");
    let tools: Vec<&str> = toolbar
        .subtab_groups(tab)
        .iter()
        .flat_map(|g| g.tools.iter().copied())
        .collect();
    for id in [
        "import_vectors_bitmap",
        "import_vectors",
        "import_component_3d_model",
        "clipart_library",
        "import_toolpaths_file",
        "import_cut_list",
    ] {
        assert!(tools.contains(&id), "{id} is on Import");
        assert_eq!(toolbar.subtab_of(id, 0), Some(tab), "{id}");
        // each its own button, named on its own, so none hides behind another
        assert!(crate::flyout::flyout_of(id).is_none(), "{id} in a flyout");
        assert!(super::about(id).is_some(), "{id} says what it does");
    }
    assert_eq!(
        super::built_in_label("import_vectors_bitmap"),
        Some("Import Vectors or Picture")
    );
    let file = toolbar.named(&["File Operations"]);
    assert!(
        !file[0].tools.iter().any(|t| t.starts_with("import")),
        "imports live on the Import sub-tab only"
    );
}

#[test]
fn sub_tabs_go_in_the_row_or_a_section_of_their_own() {
    let none: Vec<String> = vec![];
    let all = super::DESIGN_SUBTABS.len();
    assert_eq!(super::DESIGN.layout(&none), ((0..all).collect(), vec![]));
    let model = vec!["Model".to_owned(), "No such tab".to_owned()];
    let (row, apart) = super::DESIGN.layout(&model);
    assert_eq!(apart, [3]);
    assert_eq!(row.len(), all - 1);
    // the row shows its open tab, else its first; nothing when it is empty
    assert_eq!(super::DESIGN.open(1, &model), Some(1));
    assert_eq!(super::DESIGN.open(3, &model), Some(0));
    let every = super::DESIGN
        .placed(&none, "subtab_place:*:section")
        .unwrap();
    assert_eq!(super::DESIGN.open(0, &every), None);
    assert_eq!(every.len(), all);
    // moved one at a time, kept in tab order
    let one = super::DESIGN
        .placed(&none, "subtab_place:Import:section")
        .unwrap();
    let two = super::DESIGN
        .placed(&one, "subtab_place:Edit:section")
        .unwrap();
    assert_eq!(two, ["Edit", "Import"]);
    assert_eq!(
        super::DESIGN
            .placed(&two, "subtab_place:Import:row")
            .unwrap(),
        ["Edit"]
    );
    assert_eq!(
        super::DESIGN.placed(&two, "subtab_place:*:row"),
        Some(vec![])
    );
    assert_eq!(super::DESIGN.placed(&two, "subtab_place:Nothing:row"), None);
    assert_eq!(super::DESIGN.placed(&two, "favourite:design:add:x"), None);
    // the menu offers the move that applies, and ticks the layout in use
    let menu = super::DESIGN.menu("Edit", &two);
    assert_eq!(menu[0].name, "Put Back in the Tab Row");
    assert_eq!(menu[0].action, "subtab_place:Edit:row");
    let menu = super::DESIGN.menu("Create", &none);
    assert_eq!(menu[0].name, "Show as Its Own Section");
    let ticked: Vec<&str> = menu
        .iter()
        .filter(|i| i.is_checked)
        .map(|i| i.name.as_str())
        .collect();
    assert_eq!(ticked, ["All Tabs in One Row"]);
}

#[test]
fn a_tool_is_found_on_its_sub_tab() {
    let toolbar = super::Toolbar::new();
    assert_eq!(toolbar.subtab_of("align_left", 0), Some(2));
    assert_eq!(toolbar.subtab_of("move", 0), Some(1));
    assert_eq!(toolbar.subtab_of("sculpt", 0), Some(3));
    assert_eq!(toolbar.subtab_of("save", 0), None, "always shown");
    assert_eq!(toolbar.subtab_of("no_such_tool", 0), None);
    // Distort Object is on Edit and on Model: it stays where it was started from
    assert_eq!(toolbar.subtab_of("distort_object", 3), Some(3));
    assert_eq!(toolbar.subtab_of("distort_object", 1), Some(1));
    assert_eq!(toolbar.subtab_of("distort_object", 0), Some(1));
}

#[test]
fn title_case_capitalizes_each_word() {
    assert_eq!(super::title_case("join_vectors"), "Join Vectors");
    assert_eq!(super::title_case("2d_view"), "2d View");
}
