//! Making, changing and exporting 3D models.

use super::{
    Mark, Step, Tour, borders_installed, depth_installed, fresh_job, model_circle, model_dome,
    model_dome_circle, model_dome2, model_none, model_picture, model_profile, model_rails,
    model_rect, model_result, model_texture, model_two_rails, on_design_tab, sample_border,
};

pub(super) const TOURS: &[Tour] = &[
    Tour {
        tool: "create_shape_round",
        steps: &[
            Step {
                caption: "Create Shape raises a closed shape into a 3D form. Draw the shape and select it: here a circle.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_circle(app, ctx);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, open **Model** and press **Create Shape: Round** (under Shape Creation).",
                setup: |app, _| on_design_tab(app, 3),
                mark: Mark::At("tool:create_shape_round"),
            },
            Step {
                caption: "Set the **Angle** the sides rise at and a **Height** limit if you want a flat top, then press **Create**. The circle becomes a dome.",
                setup: |app, ctx| app.invoke(ctx, "create_shape_round"),
                mark: Mark::None,
            },
            Step {
                caption: "The circle rises into a dome, shown here in the 3D view.",
                setup: |app, _| model_result(app, &[("total_height", 12.0)]),
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "create_shape_angled",
        steps: &[
            Step {
                caption: "An angled shape rises in straight slopes, like a roof or a pyramid. Select the closed shape.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_circle(app, ctx);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Model**, press **Create Shape: Angled**.",
                setup: |app, _| on_design_tab(app, 3),
                mark: Mark::At("tool:create_shape_angled"),
            },
            Step {
                caption: "Set the **Angle** of the slopes, then press **Create**.",
                setup: |app, ctx| app.invoke(ctx, "create_shape_angled"),
                mark: Mark::None,
            },
            Step {
                caption: "The circle rises into a cone with straight sloping sides.",
                setup: |app, _| model_result(app, &[("total_height", 12.0)]),
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "create_shape_smooth",
        steps: &[
            Step {
                caption: "A smooth shape rises with soft, pillowy sides. Select the closed shape.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_circle(app, ctx);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Model**, press **Create Shape: Smooth**.",
                setup: |app, _| on_design_tab(app, 3),
                mark: Mark::At("tool:create_shape_smooth"),
            },
            Step {
                caption: "Set the height and how soft the sides are, then press **Create**.",
                setup: |app, ctx| app.invoke(ctx, "create_shape_smooth"),
                mark: Mark::None,
            },
            Step {
                caption: "The circle rises into a soft, rounded cushion.",
                setup: |app, _| model_result(app, &[("total_height", 12.0)]),
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "create_shape_flat",
        steps: &[
            Step {
                caption: "A flat shape is a raised plateau with straight sides, for lettering or a base. Select the closed shape.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_circle(app, ctx);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Model**, press **Create Shape: Flat**.",
                setup: |app, _| on_design_tab(app, 3),
                mark: Mark::At("tool:create_shape_flat"),
            },
            Step {
                caption: "Set the **Height** of the plateau, then press **Create**.",
                setup: |app, ctx| app.invoke(ctx, "create_shape_flat"),
                mark: Mark::None,
            },
            Step {
                caption: "The circle becomes a flat plateau with straight sides.",
                setup: |app, _| model_result(app, &[("height", 10.0)]),
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "extrude_and_weave",
        steps: &[
            Step {
                caption: "Extrude sweeps a cross-section along a path, like a moulding round a frame. Select the path (rail) first, then the open cross-section.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_rails(app, ctx);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Model**, press **Extrude and Weave**.",
                setup: |app, _| on_design_tab(app, 3),
                mark: Mark::At("tool:extrude_and_weave"),
            },
            Step {
                caption: "Choose how the cross-section follows the rail, then press **Create**.",
                setup: |app, ctx| app.invoke(ctx, "extrude_and_weave"),
                mark: Mark::None,
            },
            Step {
                caption: "The quarter-round runs all the way round the rectangle: a picture frame moulding.",
                setup: |app, _| model_result(app, &[]),
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "two_rail_sweep",
        steps: &[
            Step {
                caption: "Two Rail Sweep stretches a cross-section between two rails all the way along, as for a curved handle or a boat hull. Select the two rails, then the cross-section.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_two_rails(app, ctx);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Model**, press **Two Rail Sweep**.",
                setup: |app, _| on_design_tab(app, 3),
                mark: Mark::At("tool:two_rail_sweep"),
            },
            Step {
                caption: "Choose how the cross-section is scaled along the way, then press **Create**.",
                setup: |app, ctx| app.invoke(ctx, "two_rail_sweep"),
                mark: Mark::None,
            },
            Step {
                caption: "The cross-section is swept along the rails into a 3D part.",
                setup: |app, _| model_result(app, &[]),
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "vector_unwrapper",
        steps: &[
            Step {
                caption: "On a rotary job, Vector Unwrapper turns a cross-section of the finished shape into the profile Two Rail Sweep needs. Select the cross-section.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_circle(app, ctx);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Model**, press **Vector Unwrapper**.",
                setup: |app, _| on_design_tab(app, 3),
                mark: Mark::At("tool:vector_unwrapper"),
            },
            Step {
                caption: "Pick the centre to unwrap round, then press **Apply**. Use the profile with [Two Rail Sweep](help:two_rail_sweep).",
                setup: |app, ctx| app.invoke(ctx, "vector_unwrapper"),
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "turn_and_spin",
        steps: &[
            Step {
                caption: "Turn and Spin makes a round 3D shape from a profile line: a lathe shape like a spindle, or a dish spun round one end. Select the open profile.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_profile(app, ctx);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Model**, press **Turn and Spin**.",
                setup: |app, _| on_design_tab(app, 3),
                mark: Mark::At("tool:turn_and_spin"),
            },
            Step {
                caption: "Choose **Turn** for a lathe shape or **Spin** for a dish or dome, then press **Create**.",
                setup: |app, ctx| app.invoke(ctx, "turn_and_spin"),
                mark: Mark::None,
            },
            Step {
                caption: "The profile is turned into a round 3D shape, like a spindle on a lathe.",
                setup: |app, _| model_result(app, &[]),
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "texture_area",
        steps: &[
            Step {
                caption: "Texture Area tiles a small 3D piece over an area, like scales or studs. Select the piece in the Components list and the area's outline.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_texture(app, ctx);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Model**, press **Create Texture Area**.",
                setup: |app, _| on_design_tab(app, 3),
                mark: Mark::At("tool:texture_area"),
            },
            Step {
                caption: "Set the spacing and the shifts between rows, then press **Create**.",
                setup: |app, ctx| app.invoke(ctx, "texture_area"),
                mark: Mark::None,
            },
            Step {
                caption: "Copies of the piece are tiled across the area inside the shape.",
                setup: |app, _| model_result(app, &[]),
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "decorative_relief",
        steps: &[
            Step {
                caption: "Decorative Relief fills a shape with raised scrolls and leaves, ready to carve. Select the closed shape.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_rect(app, ctx);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Model**, press **Create Decorative Relief**.",
                setup: |app, _| on_design_tab(app, 3),
                mark: Mark::At("tool:decorative_relief"),
            },
            Step {
                caption: "Pick the style and its height, then press **Create**.",
                setup: |app, ctx| app.invoke(ctx, "decorative_relief"),
                mark: Mark::None,
            },
            Step {
                caption: "Raised scrolls and leaves fill the rectangle, ready to carve.",
                setup: |app, _| model_result(app, &[]),
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "add_zero_plane",
        steps: &[
            Step {
                caption: "Add Zero Plane makes a flat base at height 0 over the whole board, for other 3D parts to sit on.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_none(app, ctx);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Model**, press **Add Zero Plane**.",
                setup: |app, _| on_design_tab(app, 3),
                mark: Mark::At("tool:add_zero_plane"),
            },
            Step {
                caption: "A flat component covering the material joins the Components list.",
                setup: |app, ctx| app.invoke(ctx, "add_zero_plane"),
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "create_component_from_bitmap",
        steps: &[
            Step {
                caption: "Turn a grey picture into a 3D relief: light parts high, dark parts low. Import the picture first.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_picture(app, ctx);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Model**, press **Create Component from Bitmap**.",
                setup: |app, _| on_design_tab(app, 3),
                mark: Mark::At("tool:create_component_from_bitmap"),
            },
            Step {
                caption: "Set the height and press **Create**. Use [Edit Picture](help:edit_picture) first to adjust the tones.",
                setup: |app, ctx| app.invoke(ctx, "create_component_from_bitmap"),
                mark: Mark::None,
            },
            Step {
                caption: "The picture becomes a relief: the sky and sun stand high and the dark hills sit low.",
                setup: |app, _| model_result(app, &[]),
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "addons",
        steps: &[
            Step {
                caption: "Open **Gadgets ▸ Add-ons...** and press **Install** beside the add-on you want. One that is not signed asks before it installs.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    depth_installed(app);
                    app.invoke(ctx, "addons");
                },
                mark: Mark::None,
            },
            Step {
                caption: "Once it is installed it is listed in **Gadgets ▸ My Gadgets**, and a **Gadgets** section after the other tools holds it. Click it there to use it.",
                setup: |app, _| {
                    app.addons.open = false;
                    on_design_tab(app, 4);
                },
                mark: Mark::At("tool:relief_from_photo"),
            },
        ],
    },
    Tour {
        tool: "relief_from_photo",
        steps: &[
            Step {
                caption: "Relief from Photo shapes a photo into a carved relief, with the background cut away. It needs the Depth add-on to work out the depth.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_picture(app, ctx);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Model**, press **Relief from Photo**.",
                setup: |app, _| on_design_tab(app, 3),
                mark: Mark::At("tool:relief_from_photo"),
            },
            Step {
                caption: "Choose how high the subject stands and how much detail to keep, then press **Create**.",
                setup: |app, ctx| app.invoke(ctx, "relief_from_photo"),
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "generate_border",
        steps: &[
            Step {
                caption: "Generate Border makes one repeating length of picture-frame moulding in a style you pick, ready to carve. It needs the free Borders add-on and a Google key of your own.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    borders_installed(app);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Model**, press **Generate Border**.",
                setup: |app, _| on_design_tab(app, 3),
                mark: Mark::At("tool:generate_border"),
            },
            Step {
                caption: "Pick a **Style**, type anything to add under **Details**, and set the border's **Length** and **Depth**.",
                setup: |app, ctx| app.invoke(ctx, "generate_border"),
                mark: Mark::At("field:style"),
            },
            Step {
                caption: "Type your Google key, press **Save Key**, then press **Generate**. It takes about half a minute.",
                setup: |_, _| {},
                mark: Mark::At("button:Generate"),
            },
            Step {
                caption: "The border is saved in your **Library**, under **Generated ▸ Borders**, its photo on its tile. Double-click it, or drag it onto the job, to place it like any 3D model.",
                setup: |app, ctx| {
                    app.form = None;
                    sample_border(app);
                    app.invoke(ctx, "clipart_library");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "sculpt",
        steps: &[
            Step {
                caption: "Sculpting changes a 3D part with a brush, like modelling clay. Select the part in the Components list.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_dome(app, ctx);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Model**, press **Sculpt** (under Shape Editing).",
                setup: |app, _| on_design_tab(app, 3),
                mark: Mark::At("tool:sculpt"),
            },
            Step {
                caption: "Pick smooth, smudge, add or remove, set the brush size, and drag over the part in the 3D view.",
                setup: |app, ctx| app.invoke(ctx, "sculpt"),
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "smooth_selected_components",
        steps: &[
            Step {
                caption: "Smooth softens a 3D part's shape. Select the part in the Components list.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_dome(app, ctx);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Model**, press **Smooth Selected Components**.",
                setup: |app, _| on_design_tab(app, 3),
                mark: Mark::At("tool:smooth_selected_components"),
            },
            Step {
                caption: "Set the number of passes and press **Apply**: more passes, softer shape.",
                setup: |app, ctx| app.invoke(ctx, "smooth_selected_components"),
                mark: Mark::None,
            },
            Step {
                caption: "The part's surface is softer, with the small steps smoothed out.",
                setup: |app, _| model_result(app, &[]),
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "emboss",
        steps: &[
            Step {
                caption: "Emboss makes a 3D part much lower while keeping its detail, for a shallow carving. Select the part.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_dome(app, ctx);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Model**, press **Emboss Component**.",
                setup: |app, _| on_design_tab(app, 3),
                mark: Mark::At("tool:emboss"),
            },
            Step {
                caption: "Set **Scale height**, the new height as a share of the old, and press **Apply**.",
                setup: |app, ctx| app.invoke(ctx, "emboss"),
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "offset_model",
        steps: &[
            Step {
                caption: "Offset Model grows or shrinks the whole visible model by a distance, all round.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_dome(app, ctx);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Model**, press **Offset Model**.",
                setup: |app, _| on_design_tab(app, 3),
                mark: Mark::At("tool:offset_model"),
            },
            Step {
                caption: "Set the distance (negative shrinks it) and press **Apply**.",
                setup: |app, ctx| app.invoke(ctx, "offset_model"),
                mark: Mark::None,
            },
            Step {
                caption: "The whole model has grown outwards by the distance.",
                setup: |app, _| model_result(app, &[]),
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "slice",
        steps: &[
            Step {
                caption: "Slice cuts a model too thick for your board into layers, each carved from its own board and glued up.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_dome(app, ctx);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Model**, press **Slice Model**.",
                setup: |app, _| on_design_tab(app, 3),
                mark: Mark::At("tool:slice"),
            },
            Step {
                caption: "Set each slice's thickness and press **Apply**: each slice becomes its own part.",
                setup: |app, ctx| app.invoke(ctx, "slice"),
                mark: Mark::None,
            },
            Step {
                caption: "The model is split into slices, each its own part to carve from its own board.",
                setup: |app, _| model_result(app, &[("slice_thickness", 6.0)]),
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "add_draft",
        steps: &[
            Step {
                caption: "Add Draft makes the model's steep walls slope, so it lifts out of a mould.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_dome(app, ctx);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Model**, press **Add Draft to Model**.",
                setup: |app, _| on_design_tab(app, 3),
                mark: Mark::At("tool:add_draft"),
            },
            Step {
                caption: "Set the draft angle and press **Apply**.",
                setup: |app, ctx| app.invoke(ctx, "add_draft"),
                mark: Mark::None,
            },
            Step {
                caption: "The steep walls now slope outwards, so the part lifts out of a mould.",
                setup: |app, _| model_result(app, &[]),
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "scale_model_z_height",
        steps: &[
            Step {
                caption: "Scale Model Z Height squashes or stretches the whole model to a new height.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_dome(app, ctx);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Model**, press **Scale Model Z Height**.",
                setup: |app, _| on_design_tab(app, 3),
                mark: Mark::At("tool:scale_model_z_height"),
            },
            Step {
                caption: "Type the new height and press **Apply**.",
                setup: |app, ctx| app.invoke(ctx, "scale_model_z_height"),
                mark: Mark::None,
            },
            Step {
                caption: "The model is squashed or stretched to exactly the height you typed.",
                setup: |app, _| model_result(app, &[]),
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "replace_below",
        steps: &[
            Step {
                caption: "Replace Below removes or flattens the parts of a 3D part lower than a height. Select the part.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_dome(app, ctx);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Model**, press **Replace Below**.",
                setup: |app, _| on_design_tab(app, 3),
                mark: Mark::At("tool:replace_below"),
            },
            Step {
                caption: "Set the height, choose remove or flatten, then press **Apply**.",
                setup: |app, ctx| app.invoke(ctx, "replace_below"),
                mark: Mark::None,
            },
            Step {
                caption: "The lower part of the shape is cleared, leaving only what stands above the height.",
                setup: |app, _| model_result(app, &[("below", 6.0)]),
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "split",
        steps: &[
            Step {
                caption: "Split cuts a 3D part in two with a shape, like a cookie cutter. Select the part and the closed shape.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_dome_circle(app, ctx);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Model**, press **Split Component**.",
                setup: |app, _| on_design_tab(app, 3),
                mark: Mark::At("tool:split"),
            },
            Step {
                caption: "Press **Apply**: the inside and the outside become two parts.",
                setup: |app, ctx| app.invoke(ctx, "split"),
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "bake_components",
        steps: &[
            Step {
                caption: "Bake joins all the visible 3D parts into one.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_dome2(app, ctx);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Model**, press **Bake Components**.",
                setup: |app, _| on_design_tab(app, 3),
                mark: Mark::At("tool:bake_components"),
            },
            Step {
                caption: "The visible parts are replaced by one part holding the same model.",
                setup: |app, ctx| app.invoke(ctx, "bake_components"),
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "component_levels",
        steps: &[
            Step {
                caption: "Levels stack 3D parts: each level is combined on its own, then the levels are combined, so one part can sit on another.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_dome2(app, ctx);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Model**, press **Component Levels**.",
                setup: |app, _| on_design_tab(app, 3),
                mark: Mark::At("tool:component_levels"),
            },
            Step {
                caption: "Add levels and set how each one combines with those under it.",
                setup: |app, ctx| app.invoke(ctx, "component_levels"),
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "reset_inside_selected_vectors",
        steps: &[
            Step {
                caption: "Reset Inside clears the model inside the selected shapes back to flat. Select the closed shapes.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_dome_circle(app, ctx);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Model**, press **Reset Inside Selected Vectors**.",
                setup: |app, _| on_design_tab(app, 3),
                mark: Mark::At("tool:reset_inside_selected_vectors"),
            },
            Step {
                caption: "Inside the shape, the model is flat again.",
                setup: |app, ctx| app.invoke(ctx, "reset_inside_selected_vectors"),
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "reset_outside_selected_vectors",
        steps: &[
            Step {
                caption: "Reset Outside clears the model outside the selected shapes back to flat. Select the closed shapes.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_dome_circle(app, ctx);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Model**, press **Reset Outside Selected Vectors**.",
                setup: |app, _| on_design_tab(app, 3),
                mark: Mark::At("tool:reset_outside_selected_vectors"),
            },
            Step {
                caption: "Outside the shape, the model is flat again.",
                setup: |app, ctx| app.invoke(ctx, "reset_outside_selected_vectors"),
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "component_boundary",
        steps: &[
            Step {
                caption: "Draw an outline round the 3D parts, to cut them out or to limit a toolpath. Select the parts, or none for all.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_dome(app, ctx);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Model**, press **Create Vector Boundary Around Selected Components**.",
                setup: |app, _| on_design_tab(app, 3),
                mark: Mark::At("tool:component_boundary"),
            },
            Step {
                caption: "A closed outline follows the edge of the parts.",
                setup: |app, ctx| app.invoke(ctx, "component_boundary"),
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "cross_section",
        steps: &[
            Step {
                caption: "A cross-section draws the model's profile along a line, to check it or reuse it as a sweep's profile.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_dome(app, ctx);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Model**, press **Create Cross Section Vector**.",
                setup: |app, _| on_design_tab(app, 3),
                mark: Mark::At("tool:cross_section"),
            },
            Step {
                caption: "Drag a line across the model: its profile is drawn as a new line.",
                setup: |app, ctx| app.invoke(ctx, "cross_section"),
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "create_component_from_visible_model",
        steps: &[
            Step {
                caption: "Make one new 3D part from everything visible, keeping the parts it came from hidden.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_dome(app, ctx);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Model**, press **Create Component from Visible Model**.",
                setup: |app, _| on_design_tab(app, 3),
                mark: Mark::At("tool:create_component_from_visible_model"),
            },
            Step {
                caption: "A new part holding the model joins the Components list; the others are hidden.",
                setup: |app, ctx| app.invoke(ctx, "create_component_from_visible_model"),
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "export_as_stl",
        steps: &[
            Step {
                caption: "Save the 3D model as an STL file, for a 3D printer or another program.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_dome(app, ctx);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Model**, press **Export Model as STL** (under Export Model) and name the file.",
                setup: |app, _| on_design_tab(app, 3),
                mark: Mark::At("tool:export_as_stl"),
            },
        ],
    },
    Tour {
        tool: "export_as_grayscale_bitmap",
        steps: &[
            Step {
                caption: "Save the 3D model as a grey picture: light is high, dark is low.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_dome(app, ctx);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Model**, press **Export Model as Grayscale Picture** and name the file.",
                setup: |app, _| on_design_tab(app, 3),
                mark: Mark::At("tool:export_as_grayscale_bitmap"),
            },
        ],
    },
    Tour {
        tool: "export_as_3d_clipart",
        steps: &[
            Step {
                caption: "Save the 3D model as clipart, to use again in other jobs.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    model_dome(app, ctx);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Model**, press **Export Model as 3D Clipart** and name the file.",
                setup: |app, _| on_design_tab(app, 3),
                mark: Mark::At("tool:export_as_3d_clipart"),
            },
        ],
    },
];
