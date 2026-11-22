//! More of the lists' and menus' commands (move node to wrapped spiral layout): each shown
//! before and after, the menu named in the words.

use super::{
    ARC_R, ARC_X, ARC_Y, CIRCLE_R, CIRCLE_X, CIRCLE_Y, COMPONENTS, LAYERS, Mark, Page, RECT_H,
    RECT_W, RECT_X, RECT_Y, SHEETS, Step, Tour, apply_form, circle, component_group, double_sided,
    draw, fresh_job, holes_inside, menu_form, model_close_up, model_dome, model_dome2,
    name_toolpaths, node_params, on_design_tab, on_list, on_node, open_arc, pick_toolpath, rect,
    rotary_job, run, select, select_all, set, shapes_circle, shapes_lines, shapes_pair,
    shapes_rect, shapes_star, shapes_three, small_profile, stacked_three, star, stl_file,
    swap_toolpaths, toolpath_ids, two_layers, two_shapes, two_toolpaths,
};

pub(super) const TOURS: &[Tour] = &[
    Tour {
        tool: "move_node",
        steps: &[
            Step {
                caption: "Select the shape and press **Node Edit**, then right-click the node \
                          to place exactly.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_rect(app);
                    app.invoke(ctx, "node_edit");
                },
                mark: Mark::Shape(RECT_X + RECT_W, RECT_Y + RECT_H / 2.0),
            },
            Step {
                caption: "Choose **Node Position...** and type where the node goes.",
                setup: |app, ctx| {
                    let params = node_params(app, RECT_X + RECT_W, RECT_Y + RECT_H);
                    let params = serde_json::json!({ "from": params, "to": params["at"] });
                    menu_form(app, ctx, "move_node", &params);
                },
                mark: Mark::At("field:to"),
            },
            Step {
                caption: "Press **Apply**. The node moves there and the spans either side \
                          follow it.",
                setup: |app, _| {
                    let mut params = node_params(app, RECT_X + RECT_W, RECT_Y + RECT_H);
                    let to = serde_json::json!({ "x": params["at"]["x"].as_f64().unwrap_or(0.0) + 40.0, "y": params["at"]["y"].as_f64().unwrap_or(0.0) + 30.0 });
                    params = serde_json::json!({ "from": params, "to": to });
                    run(app, "move_node", params);
                },
                mark: Mark::Everything,
            },
        ],
    },
    Tour {
        tool: "move_nodes",
        steps: &[
            Step {
                caption: "In **Node Edit**, drag a box round the nodes to move: they are picked \
                          together.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_rect(app);
                    app.invoke(ctx, "node_edit");
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "Drag one of them: they all move by the same amount, keeping their \
                          places to each other.",
                setup: |app, _| {
                    let a = node_params(app, RECT_X + RECT_W, RECT_Y);
                    let b = node_params(app, RECT_X + RECT_W, RECT_Y + RECT_H);
                    let params =
                        serde_json::json!({ "nodes": [a, b], "by": { "x": 60.0, "y": 0.0 } });
                    run(app, "move_nodes", params);
                },
                mark: Mark::Everything,
            },
        ],
    },
    Tour {
        tool: "move_to_layer",
        steps: &[
            Step {
                caption: "Select the vectors to move to another layer.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    two_layers(app);
                    select(app, RECT_X, RECT_Y + RECT_H / 2.0);
                },
                mark: Mark::Shape(RECT_X, RECT_Y + RECT_H / 2.0),
            },
            Step {
                caption: "Right-click them and choose **Move to Layer**, then the layer. In \
                          **Layers**, right-click a layer ▸ **Move Selection Here** does the \
                          same.",
                setup: |app, _| {
                    run(app, "move_to_layer", serde_json::json!({ "layer": 2 }));
                    on_list(app, LAYERS);
                },
                mark: Mark::At("layer:Layer 2"),
            },
        ],
    },
    Tour {
        tool: "move_to_level",
        steps: &[
            Step {
                caption: "Open **Components** and pick the component to move to another level.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_dome(app, ctx);
                    run(
                        app,
                        "insert_level",
                        serde_json::json!({ "name": "Level 2" }),
                    );
                    app.doc.selected_components = app.doc.components.keys().copied().collect();
                    on_list(app, COMPONENTS);
                },
                mark: Mark::At("component:Shape"),
            },
            Step {
                caption: "Right-click it and choose **Move to Level**, then the level. It now \
                          combines with that level's components.",
                setup: |app, _| {
                    let last = app.doc.levels.keys().last().copied().unwrap_or(0);
                    run(app, "move_to_level", serde_json::json!({ "level": last }));
                    on_list(app, COMPONENTS);
                },
                mark: Mark::At("level:Level 2"),
            },
        ],
    },
    Tour {
        tool: "open_gadgets_folder",
        steps: &[Step {
            caption: "Gadgets you install show on the Design page under **Gadgets**. **Gadgets \
                      ▸ My Gadgets ▸ Open My Gadgets Folder** opens the folder they go in.",
            setup: |app, ctx| {
                fresh_job(app, ctx);
                on_design_tab(app, 0);
            },
            mark: Mark::None,
        }],
    },
    Tour {
        tool: "rename_toolpath_group",
        steps: &[
            Step {
                caption: "In the Toolpath List, right-click a group's name and choose \
                          **Rename**.",
                setup: |app, ctx| {
                    two_toolpaths(app, ctx);
                    let toolpaths = toolpath_ids(app);
                    let params = serde_json::json!({ "toolpaths": toolpaths, "group": "Group 1" });
                    run(app, "group_toolpaths", params);
                },
                mark: Mark::At("toolpath_group:Group 1"),
            },
            Step {
                caption: "Type a name that says what the group cuts, such as Sign or Box \
                          sides.",
                setup: |app, ctx| {
                    let params = serde_json::json!({ "group": "Group 1", "name": "Group 1" });
                    menu_form(app, ctx, "rename_toolpath_group", &params);
                },
                mark: Mark::At("field:name"),
            },
            Step {
                caption: "Press **Apply**. The group has its new name in the list.",
                setup: |app, _| {
                    set(app, "name", "Sign".into());
                    apply_form(app);
                },
                mark: Mark::At("toolpath_group:Sign"),
            },
        ],
    },
    Tour {
        tool: "reorder_component",
        steps: &[
            Step {
                caption: "Open **Components** and pick a component: the list combines from \
                          the bottom up.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_dome2(app, ctx);
                    for (component, name) in
                        app.doc.components.values_mut().zip(["Dome 1", "Dome 2"])
                    {
                        component.name = name.into();
                    }
                    let last = app.doc.components.keys().last().copied();
                    app.doc.selected_components = last.into_iter().collect();
                    on_list(app, COMPONENTS);
                },
                mark: Mark::At("component:Dome 2"),
            },
            Step {
                caption: "Press **Move Up** or **Move Down** over the list to move it one place \
                          among its level's components.",
                setup: |_, _| {},
                mark: Mark::At("tool:reorder_component"),
            },
        ],
    },
    Tour {
        tool: "reorder_layer",
        steps: &[
            Step {
                caption: "Open **Layers** and click the layer to move, so it is the active one.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    two_layers(app);
                    on_list(app, LAYERS);
                },
                mark: Mark::At("layer:Layer 2"),
            },
            Step {
                caption: "Press **Move Up** or **Move Down** over the list. The order is only \
                          how the layers are listed and drawn.",
                setup: |_, _| {},
                mark: Mark::At("tool:reorder_layer"),
            },
            Step {
                caption: "The layer is one place lower in the list now (Move Up takes it \
                          back up).",
                setup: |app, _| {
                    let params = serde_json::json!({ "layer": 2, "direction": "down" });
                    run(app, "reorder_layer", params);
                    on_list(app, LAYERS);
                },
                mark: Mark::At("layer:Layer 2"),
            },
        ],
    },
    Tour {
        tool: "reorder_level",
        steps: &[
            Step {
                caption: "Open **Components** and right-click the level to move: levels \
                          combine from the bottom up.",
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
                caption: "Choose **Move Level Down** (or Up). Its components go with it, and \
                          the model is built again in the new order.",
                setup: |app, _| {
                    let last = app.doc.levels.keys().last().copied().unwrap_or(0);
                    let params = serde_json::json!({ "level": last, "direction": "down" });
                    run(app, "reorder_level", params);
                    on_list(app, COMPONENTS);
                },
                mark: Mark::At("level:Level 2"),
            },
        ],
    },
    Tour {
        tool: "reorder_toolpath",
        steps: &[
            Step {
                caption: "Click the toolpath to move in the Toolpath List: they cut from the \
                          top down.",
                setup: |app, ctx| {
                    two_toolpaths(app, ctx);
                    pick_toolpath(app, 1);
                },
                mark: Mark::At("toolpath:Cut out"),
            },
            Step {
                caption: "Press **Move Up** to cut it sooner, or **Move Down** to cut it \
                          later.",
                setup: |_, _| {},
                mark: Mark::At("button:toolpaths_move_up"),
            },
            Step {
                caption: "It has moved one place, and the file you save cuts it there.",
                setup: |app, _| {
                    swap_toolpaths(app);
                    pick_toolpath(app, 0);
                },
                mark: Mark::At("toolpath:Cut out"),
            },
        ],
    },
    Tour {
        tool: "reverse_direction",
        steps: &[
            Step {
                caption: "Select an open vector and press **Node Edit**: the cut runs from its \
                          start to its end.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    open_arc(app);
                    app.invoke(ctx, "node_edit");
                },
                mark: Mark::Shape(ARC_X, ARC_Y + ARC_R),
            },
            Step {
                caption: "Right-click it and choose **Reverse Direction**. Its start and end \
                          trade places, so it is cut the other way.",
                setup: |app, _| on_node(app, "reverse_direction", ARC_X, ARC_Y + ARC_R),
                mark: Mark::Shape(ARC_X, ARC_Y + ARC_R),
            },
        ],
    },
    Tour {
        tool: "segment_3d_model",
        steps: &[
            Step {
                caption: "Choose **File ▸ Import ▸ Segment 3D Model...** and pick the model's \
                          STL file.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    let file = stl_file();
                    let params = serde_json::json!({ "file": file, "turn_over": true });
                    menu_form(app, ctx, "segment_3d_model", &params);
                },
                mark: Mark::At("field:file"),
            },
            Step {
                caption: "Pick the axis the planes are square to (**Segmenting plane square to**), and the **Number of cuts**.",
                setup: |_, _| {},
                mark: Mark::At("field:axis"),
            },
            Step {
                caption: "Tick **Turn the second segment over** so its far side faces up to be \
                          machined.",
                setup: |_, _| {},
                mark: Mark::At("field:turn_over"),
            },
            Step {
                caption: "Press **Apply**. The segments are laid side by side as components, \
                          their tops at the material's surface.",
                setup: |app, _| {
                    apply_form(app);
                    model_close_up(app);
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "select_all_duplicate_vectors",
        steps: &[
            Step {
                caption: "A drawing brought in can have shapes drawn twice, one on top of the \
                          other: they would be cut twice.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    two_shapes(app);
                    draw(app, "draw_circle", circle());
                    app.form = None;
                    app.doc.selection.clear();
                },
                mark: Mark::None,
            },
            Step {
                caption: "Choose **Edit ▸ Selection ▸ Select All Duplicate Vectors**. The \
                          copies are selected: press **Delete** to keep one of each.",
                setup: |app, ctx| app.invoke(ctx, "select_all_duplicate_vectors"),
                mark: Mark::Shape(CIRCLE_X - CIRCLE_R, CIRCLE_Y),
            },
        ],
    },
    Tour {
        tool: "select_all_open_vectors",
        steps: &[
            Step {
                caption: "Open vectors, whose ends do not meet, can't be pocketed or cut out \
                          inside or outside.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    rect(app);
                    shapes_lines(app);
                    app.doc.selection.clear();
                },
                mark: Mark::None,
            },
            Step {
                caption: "Choose **Edit ▸ Selection ▸ Select All Open Vectors** to find them \
                          all, then join or close them.",
                setup: |app, ctx| app.invoke(ctx, "select_all_open_vectors"),
                mark: Mark::Shape(140.0, 80.0),
            },
        ],
    },
    Tour {
        tool: "select_all_vectors",
        steps: &[
            Step {
                caption: "Press **Ctrl+A** (or **Edit ▸ Selection ▸ Select All Vectors**) to \
                          select every vector you can change.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_three(app);
                    app.doc.selection.clear();
                    app.invoke(ctx, "select_all_vectors");
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "Vectors on hidden or locked layers are left out. **Unselect All** (or \
                          Esc) clears the selection.",
                setup: |_, _| {},
                mark: Mark::Everything,
            },
        ],
    },
    Tour {
        tool: "select_layer_vectors",
        steps: &[
            Step {
                caption: "Open **Layers** and right-click the layer whose vectors you want.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    two_layers(app);
                    on_list(app, LAYERS);
                },
                mark: Mark::At("layer:Layer 1"),
            },
            Step {
                caption: "Choose **Select Layer Vectors**. Every vector on that layer is \
                          selected, and nothing else.",
                setup: |app, _| {
                    run(
                        app,
                        "select_layer_vectors",
                        serde_json::json!({ "layer": 1 }),
                    );
                    on_list(app, LAYERS);
                },
                mark: Mark::Shape(RECT_X, RECT_Y + RECT_H / 2.0),
            },
        ],
    },
    Tour {
        tool: "select_vectors_inside",
        steps: &[
            Step {
                caption: "Select a closed shape with other vectors inside it.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    rect(app);
                    holes_inside(app);
                    app.form = None;
                    select(app, RECT_X, RECT_Y + RECT_H / 2.0);
                },
                mark: Mark::Shape(RECT_X, RECT_Y + RECT_H / 2.0),
            },
            Step {
                caption: "Right-click and choose **Select Vectors Inside**. Everything inside \
                          the shape is selected in its place.",
                setup: |app, ctx| app.invoke(ctx, "select_vectors_inside"),
                mark: Mark::Shape(132.0, 140.0),
            },
        ],
    },
    Tour {
        tool: "send_to_sheet",
        steps: &[
            Step {
                caption: "Select the vectors, then open **Sheets** and right-click the sheet \
                          they should go on.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    run(app, "add_sheet", serde_json::json!({ "name": "Sheet 2" }));
                    shapes_star(app);
                    on_list(app, SHEETS);
                },
                mark: Mark::At("sheet:Sheet 2"),
            },
            Step {
                caption: "Choose **Copy Selected Vectors Here** or **Move Selected Vectors \
                          Here**. They keep their layers and groups there.",
                setup: |app, _| {
                    let params = serde_json::json!({ "sheet": 1, "move_vectors": true });
                    run(app, "send_to_sheet", params);
                    run(app, "activate_sheet", serde_json::json!({ "sheet": 1 }));
                    app.doc.selection.clear();
                },
                mark: Mark::Everything,
            },
        ],
    },
    Tour {
        tool: "set_current_layer",
        steps: &[
            Step {
                caption: "Open **Layers** at the top of the Design page: the current layer is \
                          highlighted.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    two_layers(app);
                    on_list(app, LAYERS);
                },
                mark: Mark::At("layer:Layer 2"),
            },
            Step {
                caption: "Click another layer's name (or right-click ▸ **Activate**). New \
                          vectors, text and imports go on it now.",
                setup: |app, _| {
                    run(app, "set_current_layer", serde_json::json!({ "layer": 1 }));
                    on_list(app, LAYERS);
                },
                mark: Mark::At("layer:Layer 1"),
            },
        ],
    },
    Tour {
        tool: "sheet_properties",
        steps: &[
            Step {
                caption: "Open **Sheets**, right-click a sheet and choose **Rename**.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    on_list(app, SHEETS);
                },
                mark: Mark::At("sheet:Sheet 1"),
            },
            Step {
                caption: "Type a name that says what is cut from it, such as Back panel.",
                setup: |app, ctx| {
                    let params = serde_json::json!({ "sheet": 0, "name": "Sheet 1" });
                    menu_form(app, ctx, "sheet_properties", &params);
                },
                mark: Mark::At("field:name"),
            },
            Step {
                caption: "Press **Apply**. The sheet is listed by its new name; its size is \
                          set in **Job Setup**.",
                setup: |app, _| {
                    set(app, "name", "Back panel".into());
                    apply_form(app);
                    on_list(app, SHEETS);
                },
                mark: Mark::At("sheet:Back panel"),
            },
        ],
    },
    Tour {
        tool: "show_hide",
        steps: &[
            Step {
                caption: "Right-click a layer in **Layers** (or a toolpath or component in its \
                          list) and choose **Show** or **Hide**.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    two_layers(app);
                    on_list(app, LAYERS);
                },
                mark: Mark::At("layer:Layer 2"),
            },
            Step {
                caption: "**Show ▸ Only This** hides every other one, so you see just this \
                          layer's vectors. **Show ▸ All** brings them all back.",
                setup: |app, _| {
                    let params = serde_json::json!({ "items": "layers", "item": 2, "action": "show_only_this" });
                    run(app, "show_hide", params);
                    on_list(app, LAYERS);
                },
                mark: Mark::At("layer:Layer 1"),
            },
        ],
    },
    Tour {
        tool: "smooth_point",
        steps: &[
            Step {
                caption: "Select the shape and press **Node Edit**, then right-click a sharp \
                          node.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_rect(app);
                    app.invoke(ctx, "node_edit");
                },
                mark: Mark::Shape(RECT_X, RECT_Y + RECT_H / 2.0),
            },
            Step {
                caption: "Choose **Smooth Point**. The corner rounds into a smooth curve; drag \
                          its handles to shape it.",
                setup: |app, _| on_node(app, "smooth_point", RECT_X + RECT_W, RECT_Y + RECT_H),
                mark: Mark::Shape(RECT_X, RECT_Y + RECT_H / 2.0),
            },
        ],
    },
    Tour {
        tool: "space_horizontally",
        steps: &[
            Step {
                caption: "Select three or more shapes, clicking them in the order you want them \
                          from left to right.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_three(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "Choose **Edit ▸ Align Selected Objects ▸ Left To Right**. The gaps \
                          between them are now even across.",
                setup: |app, _| {
                    let params = serde_json::json!({ "order": "left_to_right", "inside_last_vector": false });
                    run(app, "space_horizontally", params);
                },
                mark: Mark::Everything,
            },
        ],
    },
    Tour {
        tool: "space_vertically",
        steps: &[
            Step {
                caption: "Select three or more shapes, clicking them in the order you want them \
                          from bottom to top.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    stacked_three(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "Choose **Edit ▸ Align Selected Objects ▸ Bottom To Top**. The gaps \
                          between them are now even up the job.",
                setup: |app, _| {
                    let params = serde_json::json!({ "order": "bottom_to_top", "inside_last_vector": false });
                    run(app, "space_vertically", params);
                },
                mark: Mark::Everything,
            },
        ],
    },
    Tour {
        tool: "span_to_arc",
        steps: &[
            Step {
                caption: "Select the shape and press **Node Edit**, then right-click the span \
                          to bend.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_rect(app);
                    app.invoke(ctx, "node_edit");
                },
                mark: Mark::Shape(RECT_X + RECT_W / 2.0, RECT_Y + RECT_H),
            },
            Step {
                caption: "Choose **Convert Span to Arc**. The side is a round arc now: drag its \
                          middle to make it bulge more or less.",
                setup: |app, _| {
                    on_node(app, "span_to_arc", RECT_X + RECT_W / 2.0, RECT_Y + RECT_H);
                },
                mark: Mark::Shape(RECT_X, RECT_Y + RECT_H / 2.0),
            },
        ],
    },
    Tour {
        tool: "span_to_line",
        steps: &[
            Step {
                caption: "Select a curved shape and press **Node Edit**, then right-click the \
                          curved span to straighten.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_circle(app);
                    app.invoke(ctx, "node_edit");
                },
                mark: Mark::Shape(CIRCLE_X - CIRCLE_R, CIRCLE_Y),
            },
            Step {
                caption: "Choose **Convert Span to Line**. That span is a straight line between \
                          its two nodes now.",
                setup: |app, _| on_node(app, "span_to_line", CIRCLE_X, CIRCLE_Y - CIRCLE_R),
                mark: Mark::Shape(CIRCLE_X - CIRCLE_R, CIRCLE_Y),
            },
        ],
    },
    Tour {
        tool: "swap_sides",
        steps: &[
            Step {
                caption: "A double-sided job has a top and a bottom side, each with its own \
                          vectors, models and toolpaths.",
                setup: |app, ctx| {
                    double_sided(app, ctx);
                    draw(app, "draw_star", star());
                    app.form = None;
                    app.doc.selection.clear();
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "Choose **Edit ▸ Swap Sides** (or click the side's name at the top of \
                          the view) to work on the other side.",
                setup: |app, ctx| app.invoke(ctx, "swap_sides"),
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "toolpath_properties",
        steps: &[
            Step {
                caption: "Right-click a toolpath in the Toolpath List and choose **Rename**.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    small_profile(app, ctx);
                    name_toolpaths(app, &["Profile"]);
                    app.pages.set_pinned(Page::Toolpaths, true);
                },
                mark: Mark::At("toolpath:Profile"),
            },
            Step {
                caption: "Type a name that says what it cuts, such as Cut out, and press \
                          **Apply**.",
                setup: |app, ctx| {
                    let toolpath = toolpath_ids(app)[0];
                    let params = serde_json::json!({ "toolpath": toolpath, "name": "Profile", "visible": true });
                    menu_form(app, ctx, "toolpath_properties", &params);
                },
                mark: Mark::At("field:name"),
            },
            Step {
                caption: "The tick before a toolpath shows or hides it. Its coloured square, \
                          shown when the material's Machined Area Color is Toolpath Color, sets its colour.",
                setup: |app, _| {
                    set(app, "name", "Cut out".into());
                    apply_form(app);
                },
                mark: Mark::At("toolpath:Cut out"),
            },
        ],
    },
    Tour {
        tool: "ungroup",
        steps: &[
            Step {
                caption: "Select a group: a click picks all of it, as one.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_pair(app);
                    app.invoke(ctx, "group");
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "Right-click and choose **Ungroup ▸ Ungroup onto Original Object \
                          Layers**. Each piece can be picked on its own again.",
                setup: |app, ctx| {
                    app.invoke(ctx, "ungroup");
                    select(app, CIRCLE_X - CIRCLE_R, CIRCLE_Y);
                },
                mark: Mark::Shape(CIRCLE_X - CIRCLE_R, CIRCLE_Y),
            },
        ],
    },
    Tour {
        tool: "ungroup_components",
        steps: &[
            Step {
                caption: "Open **Components** and right-click a group's name, or a component \
                          in it.",
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
                caption: "Choose **Ungroup**. Its components are on their own again, where they \
                          were.",
                setup: |app, _| {
                    let group = component_group(app);
                    run(
                        app,
                        "ungroup_components",
                        serde_json::json!({ "group": group }),
                    );
                    on_list(app, COMPONENTS);
                },
                mark: Mark::At("component:Shape"),
            },
        ],
    },
    Tour {
        tool: "ungroup_merged_toolpath",
        steps: &[
            Step {
                caption: "Right-click a merged toolpath (made with **Toolpaths ▸ Merge Visible \
                          Toolpaths**) in the list.",
                setup: |app, ctx| {
                    two_toolpaths(app, ctx);
                    app.invoke(ctx, "merged_toolpath");
                    set(app, "name", "Merged".into());
                    apply_form(app);
                },
                mark: Mark::At("toolpath:Merged"),
            },
            Step {
                caption: "Choose **Ungroup**. The merged toolpath goes, and the toolpaths it \
                          was made from are shown again.",
                setup: |app, _| {
                    let merged = toolpath_ids(app).last().copied();
                    if let Some(toolpath) = merged {
                        run(
                            app,
                            "ungroup_merged_toolpath",
                            serde_json::json!({ "toolpath": toolpath }),
                        );
                    }
                },
                mark: Mark::At("toolpath:Cut out"),
            },
        ],
    },
    Tour {
        tool: "ungroup_onto_current_layer",
        steps: &[
            Step {
                caption: "Click the layer the pieces should go on in **Layers**, then select \
                          the group.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_pair(app);
                    app.invoke(ctx, "group");
                    run(app, "add_layer", serde_json::json!({ "name": "Layer 2" }));
                    select_all(app);
                    on_list(app, LAYERS);
                },
                mark: Mark::At("layer:Layer 2"),
            },
            Step {
                caption: "Right-click and choose **Ungroup ▸ Ungroup onto Current Layer**. The \
                          pieces come apart, all on that layer.",
                setup: |app, ctx| {
                    app.invoke(ctx, "ungroup_onto_current_layer");
                    on_list(app, LAYERS);
                },
                mark: Mark::At("layer:Layer 2"),
            },
        ],
    },
    Tour {
        tool: "ungroup_toolpaths",
        steps: &[
            Step {
                caption: "Click a group's name in the Toolpath List.",
                setup: |app, ctx| {
                    two_toolpaths(app, ctx);
                    let toolpaths = toolpath_ids(app);
                    let params = serde_json::json!({ "toolpaths": toolpaths, "group": "Group 1" });
                    run(app, "group_toolpaths", params);
                    app.selected_group = Some("Group 1".into());
                },
                mark: Mark::At("toolpath_group:Group 1"),
            },
            Step {
                caption: "Press **Ungroup** above the list (or right-click the group ▸ \
                          **Ungroup**). Its toolpaths stay, out of the group.",
                setup: |_, _| {},
                mark: Mark::At("button:toolpaths_ungroup"),
            },
            Step {
                caption: "The group is gone and its toolpaths are listed on their own again.",
                setup: |app, _| {
                    let params =
                        serde_json::json!({ "group": "Group 1", "delete_toolpaths": false });
                    run(app, "ungroup_toolpaths", params);
                    app.selected_group = None;
                },
                mark: Mark::At("toolpath:Engrave"),
            },
        ],
    },
    Tour {
        tool: "unselect_all",
        steps: &[
            Step {
                caption: "Selected vectors are drawn dashed, and a tool acts on them.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_three(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "Choose **Edit ▸ Selection ▸ Unselect All** (or press Esc, or click \
                          an empty spot). Nothing is selected now.",
                setup: |app, ctx| app.invoke(ctx, "unselect_all"),
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "vector_selector",
        steps: &[
            Step {
                caption: "Choose **Edit ▸ Selection ▸ Vector Selector...** to select vectors by \
                          what they are, not by clicking.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    rect(app);
                    holes_inside(app);
                    app.form = None;
                    app.doc.selection.clear();
                    app.invoke(ctx, "vector_selector");
                },
                mark: Mark::At("field:closed_kind"),
            },
            Step {
                caption: "Pick circles only, or circles of one size (**Circle diameter**): every hole of one \
                          size, ready to drill.",
                setup: |app, _| set(app, "closed_kind", "circles".into()),
                mark: Mark::At("field:circles"),
            },
            Step {
                caption: "Press **Apply**. Every vector that passes is selected, on the layers \
                          you chose.",
                setup: |app, _| apply_form(app),
                mark: Mark::Shape(132.0, 140.0),
            },
        ],
    },
    Tour {
        tool: "wrapped_fluting_layout",
        steps: &[
            Step {
                caption: "In a rotary job, choose **Gadgets ▸ Wrapped Fluting Layout** and set \
                          the **Number of flutes** along the piece.",
                setup: |app, ctx| {
                    rotary_job(app, ctx);
                    app.invoke(ctx, "wrapped_fluting_layout");
                },
                mark: Mark::At("field:flutes"),
            },
            Step {
                caption: "Set the **Start offset** and **End offset**, and tick **Cove at start** or \
                          **Cove at end** for a groove round that end.",
                setup: |_, _| {},
                mark: Mark::At("field:cove_start"),
            },
            Step {
                caption: "Press **Create**. The flutes are laid out as vectors on the unrolled \
                          surface, for a Profile or Fluting toolpath.",
                setup: |app, _| apply_form(app),
                mark: Mark::Everything,
            },
        ],
    },
    Tour {
        tool: "wrapped_spiral_layout",
        steps: &[
            Step {
                caption: "In a rotary job, choose **Gadgets ▸ Wrapped Spiral Layout** and set \
                          the **Number of strands** twisting round the piece.",
                setup: |app, ctx| {
                    rotary_job(app, ctx);
                    app.invoke(ctx, "wrapped_spiral_layout");
                },
                mark: Mark::At("field:strands"),
            },
            Step {
                caption: "Set the **Distance** (with Pitch: how far a strand goes in one turn) \
                          and the **Twist**: which way it turns.",
                setup: |_, _| {},
                mark: Mark::At("field:distance"),
            },
            Step {
                caption: "Press **Create**. The strands are laid out on the unrolled surface; \
                          wrapped round, they are a spiral.",
                setup: |app, _| apply_form(app),
                mark: Mark::Everything,
            },
        ],
    },
];
