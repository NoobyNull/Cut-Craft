//! Pictures, joining and trimming, lining up, moving and copying shapes.

use super::{
    Mark, Step, Tour, fresh_job, on_design_tab, shapes_circle, shapes_copy_along, shapes_lines,
    shapes_lines2, shapes_pair, shapes_picture, shapes_picture_circle, shapes_rect, shapes_star,
    shapes_text_rect, shapes_three,
};

pub(super) const TOURS: &[Tour] = &[
    Tour {
        tool: "edit_picture",
        steps: &[
            Step {
                caption: "Make a picture lighter, darker or more contrasty before you trace or carve it. Select the picture.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_picture(app);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Edit**, press **Edit Picture**.",
                setup: |app, _| on_design_tab(app, 1),
                mark: Mark::At("tool:edit_picture"),
            },
            Step {
                caption: "Move the sliders for contrast, brightness and gamma; tick Invert to swap light and dark.",
                setup: |app, ctx| {
                    app.invoke(ctx, "edit_picture");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "crop_bitmap",
        steps: &[
            Step {
                caption: "Trim a picture to a shape. Select the picture and the shape.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_picture_circle(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Edit**, press **Crop Bitmap**.",
                setup: |app, _| on_design_tab(app, 1),
                mark: Mark::At("tool:crop_bitmap"),
            },
            Step {
                caption: "Everything outside the shape is cleared to white.",
                setup: |app, ctx| {
                    app.invoke(ctx, "crop_bitmap");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "join_vectors",
        steps: &[
            Step {
                caption: "Join open lines whose ends nearly meet into one line, or one closed shape. Select them.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_lines(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Edit**, press **Join Vectors**.",
                setup: |app, _| on_design_tab(app, 1),
                mark: Mark::At("tool:join_vectors"),
            },
            Step {
                caption: "Set how close the ends must be to join.",
                setup: |app, ctx| {
                    app.invoke(ctx, "join_vectors");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "join_open_ends",
        steps: &[
            Step {
                caption: "Join two open ends by clicking them.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_lines(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Edit**, press **Join Open Ends**.",
                setup: |app, _| on_design_tab(app, 1),
                mark: Mark::At("tool:join_open_ends"),
            },
            Step {
                caption: "Click an open end, then the end it should meet. Choose a straight line, running both ends on until they meet, or a curve.",
                setup: |app, ctx| {
                    app.invoke(ctx, "join_open_ends");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "join_straight_line",
        steps: &[
            Step {
                caption: "Join the ends of the selected open lines with straight lines.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_lines2(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Edit**, press **Join With Straight Line**.",
                setup: |app, _| on_design_tab(app, 1),
                mark: Mark::At("tool:join_straight_line"),
            },
            Step {
                caption: "The nearest ends are joined by straight lines.",
                setup: |app, ctx| {
                    app.invoke(ctx, "join_straight_line");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "join_smooth_curve",
        steps: &[
            Step {
                caption: "Join the ends of the selected open lines with smooth curves.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_lines2(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Edit**, press **Join With Smooth Curve**.",
                setup: |app, _| on_design_tab(app, 1),
                mark: Mark::At("tool:join_smooth_curve"),
            },
            Step {
                caption: "The nearest ends are joined by curves that carry on their direction.",
                setup: |app, ctx| {
                    app.invoke(ctx, "join_smooth_curve");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "join_move_end_points",
        steps: &[
            Step {
                caption: "Join the selected open lines by moving their ends together.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_lines2(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Edit**, press **Join By Moving End Points**.",
                setup: |app, _| on_design_tab(app, 1),
                mark: Mark::At("tool:join_move_end_points"),
            },
            Step {
                caption: "The nearest ends are moved to meet.",
                setup: |app, ctx| {
                    app.invoke(ctx, "join_move_end_points");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "close_extend_vector",
        steps: &[
            Step {
                caption: "Close an open line into a shape, so it can be pocketed or cut round. Select the open lines.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_lines(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Edit**, press **Close / Extend Vector**.",
                setup: |app, _| on_design_tab(app, 1),
                mark: Mark::At("tool:close_extend_vector"),
            },
            Step {
                caption: "Choose to close it with a straight line from end to start, or by running its end lines on until they meet.",
                setup: |app, ctx| {
                    app.invoke(ctx, "close_extend_vector");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "weld_vectors",
        steps: &[
            Step {
                caption: "Weld joins overlapping shapes into one outline. Select them.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_pair(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Edit**, press **Weld Vectors**.",
                setup: |app, _| on_design_tab(app, 1),
                mark: Mark::At("tool:weld_vectors"),
            },
            Step {
                caption: "One outline goes round both; the lines inside are gone.",
                setup: |app, ctx| {
                    app.invoke(ctx, "weld_vectors");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "subtract_vectors",
        steps: &[
            Step {
                caption: "Subtract cuts the other shapes out of the biggest, like a cookie cutter. Select them all.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_pair(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Edit**, press **Subtract Vectors**.",
                setup: |app, _| on_design_tab(app, 1),
                mark: Mark::At("tool:subtract_vectors"),
            },
            Step {
                caption: "The biggest shape now has a bite taken out where the others overlapped it.",
                setup: |app, ctx| {
                    app.invoke(ctx, "subtract_vectors");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "overlap_intersect",
        steps: &[
            Step {
                caption: "Overlap keeps only the part where the selected shapes overlap. Select them.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_pair(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Edit**, press **Overlap**.",
                setup: |app, _| on_design_tab(app, 1),
                mark: Mark::At("tool:overlap_intersect"),
            },
            Step {
                caption: "Only the shared part is left.",
                setup: |app, ctx| {
                    app.invoke(ctx, "overlap_intersect");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "interactive_vector_trim",
        steps: &[
            Step {
                caption: "Trim away pieces of lines between where they cross other lines, by clicking them.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_pair(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Edit**, press **Interactive Vector Trim**.",
                setup: |app, _| on_design_tab(app, 1),
                mark: Mark::At("tool:interactive_vector_trim"),
            },
            Step {
                caption: "Click a piece to cut it away up to the nearest crossings, or drag across several.",
                setup: |app, ctx| {
                    app.invoke(ctx, "interactive_vector_trim");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "trim_objects",
        steps: &[
            Step {
                caption: "Trim the selected shapes to the shape you pick last: what is outside it is cut away. Select them, the trimming shape last.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_pair(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Edit**, press **Trim Objects**.",
                setup: |app, _| on_design_tab(app, 1),
                mark: Mark::At("tool:trim_objects"),
            },
            Step {
                caption: "Only the parts inside the last shape are kept.",
                setup: |app, ctx| {
                    app.invoke(ctx, "trim_objects");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "center_in_material",
        steps: &[
            Step {
                caption: "Put the selection in the middle of the board. Select it.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_star(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Arrange**, press **Centre in Material** (under Align Objects).",
                setup: |app, _| on_design_tab(app, 2),
                mark: Mark::At("tool:center_in_material"),
            },
            Step {
                caption: "It moves to the middle of the material.",
                setup: |app, ctx| {
                    app.invoke(ctx, "center_in_material");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "center_horizontally_in_material",
        steps: &[
            Step {
                caption: "Centre the selection left to right on the board.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_star(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Arrange**, press **Centre Horizontally in Material**.",
                setup: |app, _| on_design_tab(app, 2),
                mark: Mark::At("tool:center_horizontally_in_material"),
            },
            Step {
                caption: "It moves to the board's middle, left to right.",
                setup: |app, ctx| {
                    app.invoke(ctx, "center_horizontally_in_material");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "center_vertically_in_material",
        steps: &[
            Step {
                caption: "Centre the selection front to back on the board.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_star(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Arrange**, press **Centre Vertically in Material**.",
                setup: |app, _| on_design_tab(app, 2),
                mark: Mark::At("tool:center_vertically_in_material"),
            },
            Step {
                caption: "It moves to the board's middle, front to back.",
                setup: |app, ctx| {
                    app.invoke(ctx, "center_vertically_in_material");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "center",
        steps: &[
            Step {
                caption: "Centre the selected shapes on each other.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_pair(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Arrange**, press **Centre**.",
                setup: |app, _| on_design_tab(app, 2),
                mark: Mark::At("tool:center"),
            },
            Step {
                caption: "Their middles now line up.",
                setup: |app, ctx| {
                    app.invoke(ctx, "center");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "align_left",
        steps: &[
            Step {
                caption: "Line up the left edges of the selected shapes.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_pair(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Arrange**, press **Align Left**.",
                setup: |app, _| on_design_tab(app, 2),
                mark: Mark::At("tool:align_left"),
            },
            Step {
                caption: "Their left edges now line up.",
                setup: |app, ctx| {
                    app.invoke(ctx, "align_left");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "align_right",
        steps: &[
            Step {
                caption: "Line up the right edges of the selected shapes.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_pair(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Arrange**, press **Align Right**.",
                setup: |app, _| on_design_tab(app, 2),
                mark: Mark::At("tool:align_right"),
            },
            Step {
                caption: "Their right edges now line up.",
                setup: |app, ctx| {
                    app.invoke(ctx, "align_right");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "align_top",
        steps: &[
            Step {
                caption: "Line up the tops of the selected shapes.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_pair(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Arrange**, press **Align Top**.",
                setup: |app, _| on_design_tab(app, 2),
                mark: Mark::At("tool:align_top"),
            },
            Step {
                caption: "Their tops now line up.",
                setup: |app, ctx| {
                    app.invoke(ctx, "align_top");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "align_bottom",
        steps: &[
            Step {
                caption: "Line up the bottoms of the selected shapes.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_pair(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Arrange**, press **Align Bottom**.",
                setup: |app, _| on_design_tab(app, 2),
                mark: Mark::At("tool:align_bottom"),
            },
            Step {
                caption: "Their bottoms now line up.",
                setup: |app, ctx| {
                    app.invoke(ctx, "align_bottom");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "center_horizontally",
        steps: &[
            Step {
                caption: "Line up the middles of the selected shapes, left to right.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_pair(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Arrange**, press **Centre Horizontally**.",
                setup: |app, _| on_design_tab(app, 2),
                mark: Mark::At("tool:center_horizontally"),
            },
            Step {
                caption: "Their middles now line up up and down the job.",
                setup: |app, ctx| {
                    app.invoke(ctx, "center_horizontally");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "center_vertically",
        steps: &[
            Step {
                caption: "Line up the middles of the selected shapes, front to back.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_pair(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Arrange**, press **Centre Vertically**.",
                setup: |app, _| on_design_tab(app, 2),
                mark: Mark::At("tool:center_vertically"),
            },
            Step {
                caption: "Their middles now line up across the job.",
                setup: |app, ctx| {
                    app.invoke(ctx, "center_vertically");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "left_to_right",
        steps: &[
            Step {
                caption: "Space the selected shapes out evenly from left to right.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_three(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Arrange**, press **Space Left to Right**.",
                setup: |app, _| on_design_tab(app, 2),
                mark: Mark::At("tool:left_to_right"),
            },
            Step {
                caption: "The gaps between them are now even.",
                setup: |app, ctx| {
                    app.invoke(ctx, "left_to_right");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "right_to_left",
        steps: &[
            Step {
                caption: "Space the selected shapes out evenly from right to left.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_three(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Arrange**, press **Space Right to Left**.",
                setup: |app, _| on_design_tab(app, 2),
                mark: Mark::At("tool:right_to_left"),
            },
            Step {
                caption: "The gaps between them are now even.",
                setup: |app, ctx| {
                    app.invoke(ctx, "right_to_left");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "top_to_bottom",
        steps: &[
            Step {
                caption: "Space the selected shapes out evenly from top to bottom.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_three(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Arrange**, press **Space Top to Bottom**.",
                setup: |app, _| on_design_tab(app, 2),
                mark: Mark::At("tool:top_to_bottom"),
            },
            Step {
                caption: "The gaps between them are now even.",
                setup: |app, ctx| {
                    app.invoke(ctx, "top_to_bottom");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "bottom_to_top",
        steps: &[
            Step {
                caption: "Space the selected shapes out evenly from bottom to top.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_three(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Arrange**, press **Space Bottom to Top**.",
                setup: |app, _| on_design_tab(app, 2),
                mark: Mark::At("tool:bottom_to_top"),
            },
            Step {
                caption: "The gaps between them are now even.",
                setup: |app, ctx| {
                    app.invoke(ctx, "bottom_to_top");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "move",
        steps: &[
            Step {
                caption: "Move the selection to an exact place, or by an exact distance. Select it.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_star(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "Under **Edit**, press **Move** (under Transform Objects).",
                setup: |app, _| on_design_tab(app, 1),
                mark: Mark::At("tool:move"),
            },
            Step {
                caption: "Type where it goes (X and Y) and which point of it to place there, or how far to move it, then press **Apply**.",
                setup: |app, ctx| {
                    app.invoke(ctx, "move");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "rotate",
        steps: &[
            Step {
                caption: "Turn the selection by an exact angle. Select it.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_star(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Edit**, press **Rotate**.",
                setup: |app, _| on_design_tab(app, 1),
                mark: Mark::At("tool:rotate"),
            },
            Step {
                caption: "Type the **Angle** and the point to turn about, then press **Apply**.",
                setup: |app, ctx| {
                    app.invoke(ctx, "rotate");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "set_size",
        steps: &[
            Step {
                caption: "Make the selection an exact size. Select it.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_star(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Edit**, press **Set Size**.",
                setup: |app, _| on_design_tab(app, 1),
                mark: Mark::At("tool:set_size"),
            },
            Step {
                caption: "Type the width or height; keep **Link** ticked to keep its proportions. Then press **Apply**.",
                setup: |app, ctx| {
                    app.invoke(ctx, "set_size");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "mirror",
        steps: &[
            Step {
                caption: "Flip the selection, as in a mirror. Select it.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_star(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Edit**, press **Mirror**.",
                setup: |app, _| on_design_tab(app, 1),
                mark: Mark::At("tool:mirror"),
            },
            Step {
                caption: "Choose which way to flip, and whether to keep the original, then press **Apply**.",
                setup: |app, ctx| {
                    app.invoke(ctx, "mirror");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "distort_object",
        steps: &[
            Step {
                caption: "Bend the selection into a new shape by moving the corners of its box, or along a curve. Select it.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_rect(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Edit**, press **Distort Object**.",
                setup: |app, _| on_design_tab(app, 1),
                mark: Mark::At("tool:distort_object"),
            },
            Step {
                caption: "Drag the corner handles to bend it.",
                setup: |app, ctx| {
                    app.invoke(ctx, "distort_object");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "offset_vectors",
        steps: &[
            Step {
                caption: "Draw a copy of a shape a set distance outside or inside it: a border, a frame or an inlay. Select it.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_star(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Arrange**, press **Offset Vectors**.",
                setup: |app, _| on_design_tab(app, 2),
                mark: Mark::At("tool:offset_vectors"),
            },
            Step {
                caption: "Set the **Distance** and choose outside or inside, then press **Apply**.",
                setup: |app, ctx| {
                    app.invoke(ctx, "offset_vectors");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "nest_parts",
        steps: &[
            Step {
                caption: "Nesting packs the selected shapes onto the board with as little waste as possible.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_three(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Arrange**, press **Nest Parts**.",
                setup: |app, _| on_design_tab(app, 2),
                mark: Mark::At("tool:nest_parts"),
            },
            Step {
                caption: "Set the bit's width and the gap to leave, and the corner to start from, then press **Apply**.",
                setup: |app, ctx| {
                    app.invoke(ctx, "nest_parts");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "name_part",
        steps: &[
            Step {
                caption: "Name a shape for the cut list and the labels. Select it.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_circle(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Arrange**, press **Name Part**.",
                setup: |app, _| on_design_tab(app, 2),
                mark: Mark::At("tool:name_part"),
            },
            Step {
                caption: "Type the name and press **Apply**.",
                setup: |app, ctx| {
                    app.invoke(ctx, "name_part");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "engrave_part_names",
        steps: &[
            Step {
                caption: "Draw each name as text in the middle of its shape, so a quick engrave marks them all. Name the shapes first.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_circle(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Arrange**, press **Engrave Part Names**.",
                setup: |app, _| on_design_tab(app, 2),
                mark: Mark::At("tool:engrave_part_names"),
            },
            Step {
                caption: "The names are drawn on a Part Names layer, ready for a [Quick Engrave](help:quick_engrave_toolpath).",
                setup: |app, ctx| {
                    app.invoke(ctx, "engrave_part_names");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "plate_production",
        steps: &[
            Step {
                caption: "Make many name plates or badges from one design and a list of names. Select the design.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_text_rect(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Arrange**, press **Plate Production**.",
                setup: |app, _| on_design_tab(app, 2),
                mark: Mark::At("tool:plate_production"),
            },
            Step {
                caption: "Pick the data file; text with !!name!! in it is filled in from each row.",
                setup: |app, ctx| {
                    app.invoke(ctx, "plate_production");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "array_copy",
        steps: &[
            Step {
                caption: "Make a grid of copies of the selection. Select it.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_star(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "Under **Arrange**, press **Array Copy** (under Array Objects).",
                setup: |app, _| on_design_tab(app, 2),
                mark: Mark::At("tool:array_copy"),
            },
            Step {
                caption: "Set how many rows and columns and the gaps between, then press **Create**.",
                setup: |app, ctx| {
                    app.invoke(ctx, "array_copy");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "circular_copy",
        steps: &[
            Step {
                caption: "Make copies spaced round a centre, like the spokes of a wheel. Select the shape.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_star(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Arrange**, press **Circular Copy**.",
                setup: |app, _| on_design_tab(app, 2),
                mark: Mark::At("tool:circular_copy"),
            },
            Step {
                caption: "Set how many copies and the centre, and whether they turn as they go round.",
                setup: |app, ctx| {
                    app.invoke(ctx, "circular_copy");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "copy_along_vectors",
        steps: &[
            Step {
                caption: "Copy a shape along a line. Select the shape first, then Shift-click the line.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_copy_along(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Arrange**, press **Copy Along Vectors**.",
                setup: |app, _| on_design_tab(app, 2),
                mark: Mark::At("tool:copy_along_vectors"),
            },
            Step {
                caption: "Set how many copies or the gap between them, and whether they turn to follow the line.",
                setup: |app, ctx| {
                    app.invoke(ctx, "copy_along_vectors");
                },
                mark: Mark::None,
            },
        ],
    },
];
