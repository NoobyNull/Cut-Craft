//! The sheets, layers, components and toolpath lists, node editing's right-click menu and the
//! menu commands (activate sheet to merge visible layers): each shown before and after, the menu
//! named in the words.

use super::{
    ARC_R, ARC_X, ARC_Y, CIRCLE_R, CIRCLE_X, CIRCLE_Y, COMPONENTS, LAYERS, Mark, Page, RECT_H,
    RECT_W, RECT_X, RECT_Y, SHEETS, Step, Tour, View, apply_form, centre_line, component_group,
    dome, double_sided, draw, fresh_job, gcode_file, menu_form, model_close_up, model_dome,
    model_dome2, model_result, name_toolpaths, node_params, on_list, on_node, open_arc,
    pick_toolpath, picture, rect, run, select, select_all, set, shapes_circle, shapes_lines2,
    shapes_rect, small_profile, star, swap_toolpaths, toolpath_ids, two_layers, two_shapes,
    two_toolpaths, with,
};

pub(super) const TOURS: &[Tour] = &[
    Tour {
        tool: "activate_sheet",
        steps: &[
            Step {
                caption: "A job can hold several sheets of material. Open **Sheets** at the \
                          top of the Design page to see them listed.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    run(app, "add_sheet", serde_json::json!({ "name": "Sheet 2" }));
                    on_list(app, SHEETS);
                },
                mark: Mark::At("sheet:Sheet 2"),
            },
            Step {
                caption: "Click a sheet's name, or right-click it and choose **Activate**. The \
                          view now shows that sheet and what is on it.",
                setup: |app, _| run(app, "activate_sheet", serde_json::json!({ "sheet": 1 })),
                mark: Mark::At("sheet:Sheet 2"),
            },
        ],
    },
    Tour {
        tool: "add_layer",
        steps: &[
            Step {
                caption: "Open **Layers** at the top of the Design page and press **Add \
                          Layer** under the list.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    rect(app);
                    app.doc.selection.clear();
                    on_list(app, LAYERS);
                },
                mark: Mark::At("tool:add_layer"),
            },
            Step {
                caption: "Type a name that says what the layer holds, such as Engraving or Cut \
                          Out.",
                setup: |app, ctx| app.invoke(ctx, "add_layer"),
                mark: Mark::At("field:name"),
            },
            Step {
                caption: "Press **Create**. The new layer goes to the top of the list and is \
                          made current, so what you draw next goes on it.",
                setup: |app, _| {
                    apply_form(app);
                    on_list(app, LAYERS);
                },
                mark: Mark::At("layer:Layer 2"),
            },
        ],
    },
    Tour {
        tool: "add_sheet",
        steps: &[
            Step {
                caption: "Open **Sheets** at the top of the Design page and press **Add New \
                          Sheet...** under the list.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    on_list(app, SHEETS);
                },
                mark: Mark::At("tool:add_sheet"),
            },
            Step {
                caption: "Give the sheet a name. It starts the same size as the sheet it was \
                          made from.",
                setup: |app, ctx| app.invoke(ctx, "add_sheet"),
                mark: Mark::At("field:name"),
            },
            Step {
                caption: "Press **Create**. The new sheet is listed; click it to work on it, and \
                          change its size in **Job Setup**.",
                setup: |app, _| {
                    apply_form(app);
                    on_list(app, SHEETS);
                },
                mark: Mark::At("sheet:Sheet 2"),
            },
        ],
    },
    Tour {
        tool: "arrange_toolpaths",
        steps: &[
            Step {
                caption: "The Toolpath List cuts from the top down. Click the toolpath you want \
                          to cut at another time.",
                setup: |app, ctx| {
                    two_toolpaths(app, ctx);
                    pick_toolpath(app, 1);
                },
                mark: Mark::At("toolpath:Cut out"),
            },
            Step {
                caption: "Press **Move Up** or **Move Down** above the list, or drag the \
                          toolpath to its new place.",
                setup: |_, _| {},
                mark: Mark::At("button:toolpaths_move_up"),
            },
            Step {
                caption: "It now cuts in its new place: the file you save runs the toolpaths in \
                          this order.",
                setup: |app, _| {
                    swap_toolpaths(app);
                    pick_toolpath(app, 0);
                },
                mark: Mark::At("toolpath:Cut out"),
            },
        ],
    },
    Tour {
        tool: "bitmap_properties",
        steps: &[
            Step {
                caption: "Click the picture to select it, then right-click it and choose \
                          **Bitmap Properties**.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    picture(app);
                },
                mark: Mark::None,
            },
            Step {
                caption: "Type where its corner goes (**X**, **Y**) and how big it is \
                          (**Width**, **Height**).",
                setup: |app, ctx| app.invoke(ctx, "bitmap_properties"),
                mark: Mark::At("field:width"),
            },
            Step {
                caption: "Press **Apply**. The picture moves and resizes; the picture itself \
                          stays as it was.",
                setup: |_, _| {},
                mark: Mark::At("button:Apply"),
            },
        ],
    },
    Tour {
        tool: "break_text_into_lines",
        steps: &[
            Step {
                caption: "Select a block of text with more than one line, made with **Draw \
                          Text**.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    let words = serde_json::json!({
                        "text": "CUT\nCRAFT", "height": 40.0, "anchor": { "x": 80.0, "y": 180.0 }
                    });
                    let words = with(app, "draw_text", &words);
                    draw(app, "draw_text", words);
                    select_all(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "Right-click it and choose **Text ▸ Break Block into Lines**. Each \
                          line is now text of its own, to move or edit apart.",
                setup: |app, _| {
                    let group = cmd_draw_text::selected_text_group(&app.doc).unwrap_or(0);
                    run(
                        app,
                        "break_text_into_lines",
                        serde_json::json!({ "group": group }),
                    );
                    app.doc.selection.clear();
                },
                mark: Mark::Everything,
            },
        ],
    },
    Tour {
        tool: "celtic_weave",
        steps: &[
            Step {
                caption: "On the **Gadgets** menu, choose **Celtic Weave Creator**. Set the \
                          knot grid's **Columns**, **Rows** and cell size.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    app.invoke(ctx, "celtic_weave");
                },
                mark: Mark::At("field:columns"),
            },
            Step {
                caption: "Tick **Create cross-section** for a half-round profile beside the knot, \
                          ready for Extrude and Weave.",
                setup: |_, _| {},
                mark: Mark::At("field:cross_section"),
            },
            Step {
                caption: "Press **Create**. The knot's loops are drawn as smooth closed vectors, \
                          the rails Extrude and Weave turns into a weave.",
                setup: |app, _| {
                    set(app, "cross_section", true.into());
                    apply_form(app);
                },
                mark: Mark::Everything,
            },
        ],
    },
    Tour {
        tool: "close_with_line",
        steps: &[
            Step {
                caption: "Select an open vector, then press **Node Edit** on the Design page \
                          (under **Edit**).",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    open_arc(app);
                    app.invoke(ctx, "node_edit");
                },
                mark: Mark::Shape(ARC_X, ARC_Y + ARC_R),
            },
            Step {
                caption: "Right-click on the vector and choose **Close With a Line**. A \
                          straight line now joins its end back to its start.",
                setup: |app, _| on_node(app, "close_with_line", ARC_X, ARC_Y + ARC_R),
                mark: Mark::Shape(ARC_X, ARC_Y + ARC_R),
            },
        ],
    },
    Tour {
        tool: "component_properties",
        steps: &[
            Step {
                caption: "Open **Components** at the top of the Design page, right-click the \
                          component and choose **Properties**.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_dome(app, ctx);
                    on_list(app, COMPONENTS);
                },
                mark: Mark::At("component:Shape"),
            },
            Step {
                caption: "**Shape height** makes it taller or lower, as a percentage; **Base \
                          height** raises it on a flat base.",
                setup: |app, ctx| app.invoke(ctx, "component_properties"),
                mark: Mark::At("field:shape_height"),
            },
            Step {
                caption: "**Combine Mode** says how it meets the model under it: Add, \
                          Subtract, Merge, Low or Multiply.",
                setup: |_, _| {},
                mark: Mark::At("field:combine"),
            },
        ],
    },
    Tour {
        tool: "copy_to_other_side",
        steps: &[
            Step {
                caption: "In a double-sided job, select the vectors to cut from the other side \
                          as well.",
                setup: |app, ctx| {
                    double_sided(app, ctx);
                    draw(app, "draw_star", star());
                    select_all(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "Right-click them and choose **Copy To ▸ Other Side**. **Swap Sides** \
                          shows the copy there, mirrored so it lines up through the board.",
                setup: |app, ctx| {
                    run(app, "copy_to_other_side", serde_json::json!({}));
                    app.invoke(ctx, "swap_sides");
                    app.doc.selection.clear();
                },
                mark: Mark::Everything,
            },
        ],
    },
    Tour {
        tool: "copy_toolpath_to_sheet",
        steps: &[
            Step {
                caption: "Right-click a toolpath in the Toolpath List and choose **Copy to \
                          Sheet**, then the sheet it goes on.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    run(app, "add_sheet", serde_json::json!({ "name": "Sheet 2" }));
                    small_profile(app, ctx);
                    name_toolpaths(app, &["Cut out"]);
                    app.pages.set_pinned(Page::Toolpaths, true);
                },
                mark: Mark::At("toolpath:Cut out"),
            },
            Step {
                caption: "The toolpath and copies of its vectors are now on that sheet too, \
                          ready to recalculate there.",
                setup: |app, _| {
                    let toolpath = toolpath_ids(app)[0];
                    let params = serde_json::json!({ "toolpath": toolpath, "sheet": 1 });
                    run(app, "copy_toolpath_to_sheet", params);
                    run(app, "activate_sheet", serde_json::json!({ "sheet": 1 }));
                },
                mark: Mark::At("toolpath:Cut out"),
            },
        ],
    },
    Tour {
        tool: "create_shape",
        steps: &[
            Step {
                caption: "Select one or more closed vectors: the shape rises from their edges.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_circle(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Model page, press one of the **Create Shape** buttons, then \
                          pick the **Profile**: Round, Angular, Smooth, Flat or Custom.",
                setup: |app, ctx| app.invoke(ctx, "create_shape"),
                mark: Mark::At("field:profile"),
            },
            Step {
                caption: "Set the **Angle** the sides rise at, and a **Height** where the top \
                          levels off.",
                setup: |_, _| {},
                mark: Mark::At("field:angle"),
            },
            Step {
                caption: "Press **Create**. The shape is added to the model as a component.",
                setup: |app, _| model_result(app, &[("total_height", 12.0)]),
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "cut_vector",
        steps: &[
            Step {
                caption: "Select the vector, press **Node Edit**, then right-click the node \
                          where it should open.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_rect(app);
                    app.invoke(ctx, "node_edit");
                },
                mark: Mark::Shape(RECT_X, RECT_Y + RECT_H / 2.0),
            },
            Step {
                caption: "Choose **Cut Vector**. The shape is open at that node now: drag one \
                          end away and the gap shows.",
                setup: |app, _| {
                    on_node(app, "cut_vector", RECT_X, RECT_Y);
                    let from = node_params(app, RECT_X, RECT_Y);
                    let to = serde_json::json!({ "x": from["at"]["x"].as_f64().unwrap_or(0.0) + 40.0, "y": from["at"]["y"].as_f64().unwrap_or(0.0) + 40.0 });
                    run(
                        app,
                        "move_node",
                        serde_json::json!({ "from": from, "to": to }),
                    );
                },
                mark: Mark::Shape(RECT_X, RECT_Y + RECT_H / 2.0),
            },
        ],
    },
    Tour {
        tool: "delete",
        steps: &[
            Step {
                caption: "Select what you want to remove: vectors, text or pictures.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    two_shapes(app);
                    select(app, CIRCLE_X - CIRCLE_R, CIRCLE_Y);
                },
                mark: Mark::Shape(CIRCLE_X - CIRCLE_R, CIRCLE_Y),
            },
            Step {
                caption: "Press **Delete** on the keyboard (or **Edit ▸ Delete**). Changed your \
                          mind? **Undo** (Ctrl+Z) brings it back.",
                setup: |app, ctx| app.invoke(ctx, "delete"),
                mark: Mark::Everything,
            },
        ],
    },
    Tour {
        tool: "delete_component",
        steps: &[
            Step {
                caption: "Open **Components** at the top of the Design page and right-click \
                          the component to remove.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_dome2(app, ctx);
                    on_list(app, COMPONENTS);
                },
                mark: Mark::At("component:Shape"),
            },
            Step {
                caption: "Choose **Delete**. It is gone from the model; **Undo** brings it \
                          back.",
                setup: |app, _| {
                    let first = app.doc.components.keys().next().copied();
                    if let Some(id) = first {
                        run(
                            app,
                            "delete_component",
                            serde_json::json!({ "component": id }),
                        );
                    }
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "delete_items",
        steps: &[
            Step {
                caption: "Open **Layers** at the top of the Design page and right-click a \
                          layer, then choose **Delete**.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    two_layers(app);
                    run(app, "add_layer", serde_json::json!({ "name": "Layer 3" }));
                    on_list(app, LAYERS);
                },
                mark: Mark::At("layer:Layer 3"),
            },
            Step {
                caption: "Pick **This**, **Visible**, **Invisible** or **Empty**. Empty clears \
                          away every layer with nothing on it.",
                setup: |app, _| {
                    let params =
                        serde_json::json!({ "items": "layers", "item": 3, "which": "empty" });
                    run(app, "delete_items", params);
                    on_list(app, LAYERS);
                },
                mark: Mark::At("layer:Layer 2"),
            },
        ],
    },
    Tour {
        tool: "delete_layer",
        steps: &[
            Step {
                caption: "Open **Layers** at the top of the Design page and right-click the \
                          layer to remove.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    two_layers(app);
                    on_list(app, LAYERS);
                },
                mark: Mark::At("layer:Layer 2"),
            },
            Step {
                caption: "Choose **Delete Layer**. The layer goes, and every vector on it \
                          goes with it.",
                setup: |app, _| {
                    run(app, "delete_layer", serde_json::json!({ "layer": 2 }));
                    on_list(app, LAYERS);
                },
                mark: Mark::At("layer:Layer 1"),
            },
        ],
    },
    Tour {
        tool: "delete_level",
        steps: &[
            Step {
                caption: "Open **Components** at the top of the Design page and right-click the \
                          level to remove.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    run(
                        app,
                        "insert_level",
                        serde_json::json!({ "name": "Level 2" }),
                    );
                    on_list(app, COMPONENTS);
                },
                mark: Mark::At("level:Level 2"),
            },
            Step {
                caption: "Choose **Delete Level**. The level and the components on it are \
                          gone; a job always keeps one level.",
                setup: |app, _| {
                    let last = app.doc.levels.keys().last().copied().unwrap_or(0);
                    run(app, "delete_level", serde_json::json!({ "level": last }));
                    on_list(app, COMPONENTS);
                },
                mark: Mark::At("level:Level 1"),
            },
        ],
    },
    Tour {
        tool: "delete_point",
        steps: &[
            Step {
                caption: "Select the shape and press **Node Edit**, then right-click the node \
                          to take out.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_rect(app);
                    app.invoke(ctx, "node_edit");
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "Choose **Delete Point**. The node goes, and a straight line joins \
                          the nodes either side.",
                setup: |app, _| on_node(app, "delete_point", RECT_X + RECT_W, RECT_Y + RECT_H),
                mark: Mark::Everything,
            },
        ],
    },
    Tour {
        tool: "delete_sheet",
        steps: &[
            Step {
                caption: "Open **Sheets** at the top of the Design page and right-click the \
                          sheet to remove.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    run(app, "add_sheet", serde_json::json!({ "name": "Sheet 2" }));
                    on_list(app, SHEETS);
                },
                mark: Mark::At("sheet:Sheet 2"),
            },
            Step {
                caption: "Choose **Delete**. The sheet goes with everything on it; a job \
                          always keeps one sheet.",
                setup: |app, _| {
                    run(app, "delete_sheet", serde_json::json!({ "sheet": 1 }));
                    on_list(app, SHEETS);
                },
                mark: Mark::At("sheet:Sheet 1"),
            },
        ],
    },
    Tour {
        tool: "delete_toolpath",
        steps: &[
            Step {
                caption: "Click the toolpath to remove in the Toolpath List.",
                setup: |app, ctx| {
                    two_toolpaths(app, ctx);
                    pick_toolpath(app, 0);
                },
                mark: Mark::At("toolpath:Engrave"),
            },
            Step {
                caption: "Press **Delete** above the list (or the Delete key). The vectors it \
                          was made from stay.",
                setup: |_, _| {},
                mark: Mark::At("button:toolpaths_delete"),
            },
            Step {
                caption: "It is gone from the list. **Undo** (Ctrl+Z) brings it back.",
                setup: |app, ctx| app.invoke(ctx, "delete_toolpath"),
                mark: Mark::At("toolpath:Cut out"),
            },
        ],
    },
    Tour {
        tool: "duplicate_component",
        steps: &[
            Step {
                caption: "Open **Components** at the top of the Design page and right-click the \
                          component to copy.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_dome(app, ctx);
                    on_list(app, COMPONENTS);
                },
                mark: Mark::At("component:Shape"),
            },
            Step {
                caption: "Choose **Duplicate**. The copy sits in the same place, at the top of \
                          its level in the list: click it there, then move it.",
                setup: |app, _| {
                    let first = app.doc.components.keys().next().copied();
                    if let Some(id) = first {
                        run(
                            app,
                            "duplicate_component",
                            serde_json::json!({ "component": id }),
                        );
                    }
                    on_list(app, COMPONENTS);
                },
                mark: Mark::At("component:Shape"),
            },
        ],
    },
    Tour {
        tool: "duplicate_toolpath",
        steps: &[
            Step {
                caption: "Right-click a toolpath in the Toolpath List and choose **Duplicate**.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    small_profile(app, ctx);
                    name_toolpaths(app, &["Cut out"]);
                    app.pages.set_pinned(Page::Toolpaths, true);
                },
                mark: Mark::At("toolpath:Cut out"),
            },
            Step {
                caption: "A copy is added, named with the next number: change its settings to \
                          try something new without losing the first.",
                setup: |app, _| {
                    let toolpath = toolpath_ids(app)[0];
                    run(
                        app,
                        "duplicate_toolpath",
                        serde_json::json!({ "toolpath": toolpath }),
                    );
                },
                mark: Mark::At("toolpath:Cut out (1)"),
            },
        ],
    },
    Tour {
        tool: "group_components",
        steps: &[
            Step {
                caption: "Open **Components** and pick two or more components on one level \
                          (Shift-click to add one).",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_dome2(app, ctx);
                    on_list(app, COMPONENTS);
                },
                mark: Mark::At("component:Shape"),
            },
            Step {
                caption: "Right-click one and choose **Group**, then give the group a name.",
                setup: |app, ctx| app.invoke(ctx, "group_components"),
                mark: Mark::At("field:name"),
            },
            Step {
                caption: "Press **Apply**. They are one group in the list: they combine with \
                          each other first, then with the rest.",
                setup: |app, _| {
                    apply_form(app);
                    on_list(app, COMPONENTS);
                },
                mark: Mark::At("group:Group"),
            },
        ],
    },
    Tour {
        tool: "group_properties",
        steps: &[
            Step {
                caption: "Open **Components**, right-click a group's name and choose \
                          **Properties**.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_dome2(app, ctx);
                    run(
                        app,
                        "group_components",
                        serde_json::json!({ "name": "Group" }),
                    );
                    on_list(app, COMPONENTS);
                },
                mark: Mark::At("group:Group"),
            },
            Step {
                caption: "Rename the group, and pick how it combines with what is under it. \
                          Every component in it takes that mode.",
                setup: |app, ctx| {
                    let group = component_group(app);
                    let params = serde_json::json!({ "group": group, "name": "Group" });
                    menu_form(app, ctx, "group_properties", &params);
                },
                mark: Mark::At("field:combine"),
            },
        ],
    },
    Tour {
        tool: "group_toolpaths",
        steps: &[
            Step {
                caption: "In the Toolpath List, click one toolpath and Ctrl-click the others to \
                          put together.",
                setup: |app, ctx| {
                    two_toolpaths(app, ctx);
                    pick_toolpath(app, 0);
                    let second = toolpath_ids(app)[1];
                    app.more_toolpaths.insert(second);
                },
                mark: Mark::At("toolpath:Engrave"),
            },
            Step {
                caption: "Press **Group** above the list. The group is cut one toolpath after \
                          another, and moves as one.",
                setup: |_, _| {},
                mark: Mark::At("button:toolpaths_group"),
            },
            Step {
                caption: "The group is listed with its toolpaths under it. Drag another toolpath \
                          onto its name to add it.",
                setup: |app, _| {
                    let toolpaths = toolpath_ids(app);
                    let params = serde_json::json!({ "toolpaths": toolpaths, "group": "Group 1" });
                    run(app, "group_toolpaths", params);
                    app.selected_toolpath = None;
                    app.more_toolpaths.clear();
                },
                mark: Mark::At("toolpath_group:Group 1"),
            },
        ],
    },
    Tour {
        tool: "import_toolpaths",
        steps: &[
            Step {
                caption: "Choose **File ▸ Import ▸ Import Toolpaths...** and pick a G-code \
                          file your machine runs.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    let file = gcode_file();
                    let params = serde_json::json!({ "name": "Sign", "file": file });
                    menu_form(app, ctx, "import_toolpaths", &params);
                },
                mark: Mark::At("field:file"),
            },
            Step {
                caption: "Pick the **Tool** the file cuts with: its shape and size draw the \
                          preview.",
                setup: |_, _| {},
                mark: Mark::At("field:tool"),
            },
            Step {
                caption: "Press **Apply**. The file is a toolpath in the list now, to preview, \
                          check and save again.",
                setup: |app, _| {
                    apply_form(app);
                    app.pages.set_pinned(Page::Toolpaths, true);
                },
                mark: Mark::At("toolpath:Sign"),
            },
        ],
    },
    Tour {
        tool: "insert_level",
        steps: &[
            Step {
                caption: "Open **Components** at the top of the Design page and right-click a \
                          level's name.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    on_list(app, COMPONENTS);
                },
                mark: Mark::At("level:Level 1"),
            },
            Step {
                caption: "Choose **Insert New Level** and give the level a name.",
                setup: |app, ctx| app.invoke(ctx, "insert_level"),
                mark: Mark::At("field:name"),
            },
            Step {
                caption: "Press **Create**. The new level goes on top: it combines with the \
                          levels below it.",
                setup: |app, _| {
                    apply_form(app);
                    on_list(app, COMPONENTS);
                },
                mark: Mark::At("level:Level 2"),
            },
        ],
    },
    Tour {
        tool: "insert_point",
        steps: &[
            Step {
                caption: "Select the shape and press **Node Edit**, then right-click on a span \
                          where a new node should go.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_rect(app);
                    app.invoke(ctx, "node_edit");
                },
                mark: Mark::Shape(RECT_X + RECT_W / 2.0, RECT_Y),
            },
            Step {
                caption: "Choose **Insert Point**. A new node splits the span there; drag it \
                          to reshape the side.",
                setup: |app, _| on_node(app, "insert_point", RECT_X + RECT_W / 2.0, RECT_Y),
                mark: Mark::Shape(RECT_X + RECT_W / 2.0, RECT_Y),
            },
        ],
    },
    Tour {
        tool: "join_close_vectors",
        steps: &[
            Step {
                caption: "Select two open vectors to join, or one to close.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_lines2(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "Right-click and choose **Join / Close Vectors...**, then pick how the \
                          gap is filled.",
                setup: |app, ctx| app.invoke(ctx, "join_close_vectors"),
                mark: Mark::At("field:method"),
            },
            Step {
                caption: "Press **Apply**. The two are one vector now, joined at their closest \
                          ends.",
                setup: |app, _| apply_form(app),
                mark: Mark::Everything,
            },
        ],
    },
    Tour {
        tool: "layer_properties",
        steps: &[
            Step {
                caption: "Open **Layers** at the top of the Design page and double-click a \
                          layer's name (or click its coloured square).",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    two_layers(app);
                    on_list(app, LAYERS);
                },
                mark: Mark::At("layer:Layer 1"),
            },
            Step {
                caption: "Change its **Name**, and name the **Material** its vectors are cut \
                          from.",
                setup: |app, ctx| {
                    let params =
                        serde_json::json!({ "layer": 1, "name": "Layer 1", "visible": true });
                    menu_form(app, ctx, "layer_properties", &params);
                },
                mark: Mark::At("field:material"),
            },
            Step {
                caption: "Tick **Own Colour** and pick a **Colour** to tell this layer's \
                          vectors apart, then press **Apply** to keep the changes.",
                setup: |app, _| set(app, "own_colour", true.into()),
                mark: Mark::At("field:own_colour"),
            },
        ],
    },
    Tour {
        tool: "level_clipping",
        steps: &[
            Step {
                caption: "Draw a closed vector round the part of the model to keep, and select \
                          it.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    dome(app, ctx);
                    let keep = serde_json::json!({ "x": 110.0, "y": 60.0, "width": 180.0, "height": 120.0 });
                    let keep = with(app, "draw_rectangle", &keep);
                    draw(app, "draw_rectangle", keep);
                    app.form = None;
                    select(app, 110.0, 100.0);
                    app.view = View::TwoD;
                },
                mark: Mark::Shape(110.0, 100.0),
            },
            Step {
                caption: "Open **Components**, right-click the level and choose **Clipping ▸ \
                          Apply**.",
                setup: |app, _| on_list(app, COMPONENTS),
                mark: Mark::At("level:Level 1"),
            },
            Step {
                caption: "Only the part inside the vector shows; the components are not \
                          changed. **Clipping ▸ Remove** shows it all again.",
                setup: |app, _| {
                    let level = app.doc.levels.keys().next().copied().unwrap_or(0);
                    let params = serde_json::json!({ "level": level, "action": "apply" });
                    run(app, "level_clipping", params);
                    model_close_up(app);
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "level_properties",
        steps: &[
            Step {
                caption: "Open **Components**, right-click a level's name and choose \
                          **Properties**.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    on_list(app, COMPONENTS);
                },
                mark: Mark::At("level:Level 1"),
            },
            Step {
                caption: "Rename it, pick how it combines with the levels below, and raise it \
                          with a **Base height**.",
                setup: |app, ctx| {
                    let level = app.doc.levels.keys().next().copied().unwrap_or(0);
                    let params =
                        serde_json::json!({ "level": level, "name": "Level 1", "visible": true });
                    menu_form(app, ctx, "level_properties", &params);
                },
                mark: Mark::At("field:base_height"),
            },
            Step {
                caption: "**Mirror Mode** copies the level's model across the middle, so you \
                          model one half or quarter.",
                setup: |_, _| {},
                mark: Mark::At("field:mirror"),
            },
        ],
    },
    Tour {
        tool: "make_construction_line",
        steps: &[
            Step {
                caption: "Select the vectors that only place things, such as a centre line or \
                          a curve to bend along.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_circle(app);
                    centre_line(app);
                },
                mark: Mark::Shape(CIRCLE_X, 40.0),
            },
            Step {
                caption: "Right-click and choose **Make Construction Line**. It is drawn dashed, \
                          and is never cut or saved for the machine.",
                setup: |app, ctx| {
                    app.invoke(ctx, "make_construction_line");
                    app.doc.selection.clear();
                },
                mark: Mark::Shape(CIRCLE_X, 40.0),
            },
        ],
    },
    Tour {
        tool: "make_real_line",
        steps: &[
            Step {
                caption: "Select a construction line: the dashed ones place things but are \
                          never cut.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_circle(app);
                    centre_line(app);
                    app.invoke(ctx, "make_construction_line");
                },
                mark: Mark::Shape(CIRCLE_X, 40.0),
            },
            Step {
                caption: "Right-click and choose **Make Real Line**. It is an ordinary vector \
                          again, to cut and save.",
                setup: |app, ctx| {
                    app.invoke(ctx, "make_real_line");
                    app.doc.selection.clear();
                },
                mark: Mark::Shape(CIRCLE_X, 40.0),
            },
        ],
    },
    Tour {
        tool: "make_start_point",
        steps: &[
            Step {
                caption: "Select the closed shape and press **Node Edit**, then right-click \
                          the node the cut should start from.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_rect(app);
                    app.invoke(ctx, "node_edit");
                },
                mark: Mark::Shape(RECT_X + RECT_W, RECT_Y + RECT_H / 2.0),
            },
            Step {
                caption: "Choose **Make Start Point**. Toolpaths made from it now start, and \
                          plunge, at that node.",
                setup: |app, _| on_node(app, "make_start_point", RECT_X + RECT_W, RECT_Y + RECT_H),
                mark: Mark::Shape(RECT_X + RECT_W, RECT_Y + RECT_H / 2.0),
            },
        ],
    },
    Tour {
        tool: "merge_visible_layers",
        steps: &[
            Step {
                caption: "Open **Layers**, hide any layer to keep apart, then right-click the \
                          layer to merge into.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    two_layers(app);
                    on_list(app, LAYERS);
                },
                mark: Mark::At("layer:Layer 1"),
            },
            Step {
                caption: "Choose **Merge Visible**. Every shown layer's vectors move onto it, \
                          and the emptied layers go.",
                setup: |app, _| {
                    run(
                        app,
                        "merge_visible_layers",
                        serde_json::json!({ "layer": 1 }),
                    );
                    on_list(app, LAYERS);
                },
                mark: Mark::At("layer:Layer 1"),
            },
        ],
    },
];
