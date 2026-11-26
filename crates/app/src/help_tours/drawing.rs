//! Drawing, text and editing shapes.

use super::{
    Mark, RECT_H, RECT_W, RECT_X, RECT_Y, Step, Tour, first_vector, fresh_job, on_design_tab,
    shapes_none, shapes_pair, shapes_picture, shapes_rect, shapes_star, shapes_text,
    shapes_text_curve, shapes_text_rect,
};

pub(super) const TOURS: &[Tour] = &[
    Tour {
        tool: "draw_polyline",
        steps: &[
            Step {
                caption: "Draw lines through points you click: a zigzag, an outline, a path.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_none(app);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Create**, press **Draw Line / Polyline**.",
                setup: |app, _| on_design_tab(app, 0),
                mark: Mark::At("tool:draw_polyline"),
            },
            Step {
                caption: "Click each corner in the view, or drag for a curved span. Click the first point again to close the shape, or press Esc to finish an open line. You can also type the points in the form.",
                setup: |app, ctx| {
                    app.invoke(ctx, "draw_polyline");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "draw_rectangle_from_center",
        steps: &[
            Step {
                caption: "Draw a rectangle out from its middle, the same distance each way.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_none(app);
                },
                mark: Mark::None,
            },
            Step {
                caption: "Under **Create**, press **Draw Rectangle**: one button draws every way.",
                setup: |app, _| on_design_tab(app, 0),
                mark: Mark::At("tool:draw_rectangle"),
            },
            Step {
                caption: "At the top of its form, under **Draw from**, pick **Center**. Its button opens that way next time.",
                setup: |app, ctx| {
                    app.invoke(ctx, "draw_rectangle");
                    app.pick_way("draw_rectangle");
                },
                mark: Mark::At("way:draw_rectangle_from_center"),
            },
            Step {
                caption: "Click the middle, then drag out to a corner, or type the size in the form.",
                setup: |app, ctx| {
                    app.invoke(ctx, "draw_rectangle_from_center");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "draw_circle_from_edge",
        steps: &[
            Step {
                caption: "Draw a circle from one edge to the other, rather than from its middle.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_none(app);
                },
                mark: Mark::None,
            },
            Step {
                caption: "Under **Create**, press **Draw Circle**: one button draws every way.",
                setup: |app, _| on_design_tab(app, 0),
                mark: Mark::At("tool:draw_circle"),
            },
            Step {
                caption: "At the top of its form, under **Draw from**, pick **Edge**. Its button opens that way next time.",
                setup: |app, ctx| {
                    app.invoke(ctx, "draw_circle");
                    app.pick_way("draw_circle");
                },
                mark: Mark::At("way:draw_circle_from_edge"),
            },
            Step {
                caption: "Drag across the circle from edge to edge, or type it in the form.",
                setup: |app, ctx| {
                    app.invoke(ctx, "draw_circle_from_edge");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "draw_ellipse",
        steps: &[
            Step {
                caption: "Draw an oval: a stretched circle, for plaques and frames.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_none(app);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Create**, press **Draw Ellipse**.",
                setup: |app, _| on_design_tab(app, 0),
                mark: Mark::At("tool:draw_ellipse"),
            },
            Step {
                caption: "Drag from the middle out to a corner of the box it fits in, or type the centre, width and height in the form.",
                setup: |app, ctx| {
                    app.invoke(ctx, "draw_ellipse");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "draw_polygon",
        steps: &[
            Step {
                caption: "Draw a shape with equal sides: a triangle, a hexagon, an octagon.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_none(app);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Create**, press **Draw Polygon**.",
                setup: |app, _| on_design_tab(app, 0),
                mark: Mark::At("tool:draw_polygon"),
            },
            Step {
                caption: "Set the number of **Sides**, then drag from the middle out, or type the centre and radius.",
                setup: |app, ctx| {
                    app.invoke(ctx, "draw_polygon");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "draw_star",
        steps: &[
            Step {
                caption: "Draw a star with as many points as you like.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_none(app);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Create**, press **Draw Star**.",
                setup: |app, _| on_design_tab(app, 0),
                mark: Mark::At("tool:draw_star"),
            },
            Step {
                caption: "Set the number of **Points** and how deep the points go (inner radius), then drag from the middle out, or type the sizes.",
                setup: |app, ctx| {
                    app.invoke(ctx, "draw_star");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "draw_arc",
        steps: &[
            Step {
                caption: "Draw part of a circle: an arc.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_none(app);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Create**, press **Draw Arc**.",
                setup: |app, _| on_design_tab(app, 0),
                mark: Mark::At("tool:draw_arc"),
            },
            Step {
                caption: "Set the centre, radius, and the start and end angles. It runs anticlockwise from the start to the end.",
                setup: |app, ctx| {
                    app.invoke(ctx, "draw_arc");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "draw_curve",
        steps: &[
            Step {
                caption: "Draw a smooth curve that passes through every point you click.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_none(app);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Create**, press **Draw Curve**.",
                setup: |app, _| on_design_tab(app, 0),
                mark: Mark::At("tool:draw_curve"),
            },
            Step {
                caption: "Click the points in the view. The curve bends smoothly through each one; press Esc to finish.",
                setup: |app, ctx| {
                    app.invoke(ctx, "draw_curve");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "draw_freehand",
        steps: &[
            Step {
                caption: "Draw by hand, like a pencil: the line follows the pointer and is smoothed.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_none(app);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Create**, press **Draw Freehand**.",
                setup: |app, _| on_design_tab(app, 0),
                mark: Mark::At("tool:draw_freehand"),
            },
            Step {
                caption: "Hold the button down and draw in the view.",
                setup: |app, ctx| {
                    app.invoke(ctx, "draw_freehand");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "edit_text",
        steps: &[
            Step {
                caption: "Change words you made with Draw Text: select the text.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_text(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Create**, press **Edit Text**.",
                setup: |app, _| on_design_tab(app, 0),
                mark: Mark::At("tool:edit_text"),
            },
            Step {
                caption: "The text's form opens: change the words, font or size, and the letters on the job follow.",
                setup: |app, ctx| {
                    app.invoke(ctx, "edit_text");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "convert_text_to_curves",
        steps: &[
            Step {
                caption: "Turn text into plain shapes, so you can change the letters' outlines with node editing. Select the text.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_text(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Create**, press **Convert Text to Curves**.",
                setup: |app, _| on_design_tab(app, 0),
                mark: Mark::At("tool:convert_text_to_curves"),
            },
            Step {
                caption: "The letters are now shapes. They can no longer be edited as text.",
                setup: |app, ctx| {
                    app.invoke(ctx, "convert_text_to_curves");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "text_on_a_curve",
        steps: &[
            Step {
                caption: "Lay words along a curve, as on a badge or a sign's arch. Select the text, then Shift-click the curve.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_text_curve(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Create**, press **Text on a Curve**.",
                setup: |app, _| on_design_tab(app, 0),
                mark: Mark::At("tool:text_on_a_curve"),
            },
            Step {
                caption: "Choose whether the letters sit above, on or below the curve, and whether they turn with it. The text stays editable.",
                setup: |app, ctx| {
                    app.invoke(ctx, "text_on_a_curve");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "auto_layout_text",
        steps: &[
            Step {
                caption: "Fit text to fill a shape as big as it will go, wrapping it onto lines. Select the text and the shape.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_text_rect(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Create**, press **Auto Layout Text**.",
                setup: |app, _| on_design_tab(app, 0),
                mark: Mark::At("tool:auto_layout_text"),
            },
            Step {
                caption: "Set the border to leave round the edge. The words are sized and wrapped to fill the shape.",
                setup: |app, ctx| {
                    app.invoke(ctx, "auto_layout_text");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "dimensions",
        steps: &[
            Step {
                caption: "Dimensions draw measurements on the job: lengths, angles and radii, with arrows and the number.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_rect(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Create**, press **Dimensions**.",
                setup: |app, _| on_design_tab(app, 0),
                mark: Mark::At("tool:dimensions"),
            },
            Step {
                caption: "Choose the kind (length, horizontal, vertical, angle, radius), then click the points to measure.",
                setup: |app, ctx| {
                    app.invoke(ctx, "dimensions");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "create_fillets",
        steps: &[
            Step {
                caption: "Round the sharp corners of a shape, or cut dog-bones into inside corners so a round bit can reach them. Select the shape.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_rect(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Create**, press **Create Fillets**.",
                setup: |app, _| on_design_tab(app, 0),
                mark: Mark::At("tool:create_fillets"),
            },
            Step {
                caption: "Choose a rounded corner or a dog-bone, set the radius, then click the corners (or do every corner at once).",
                setup: |app, ctx| {
                    app.invoke(ctx, "create_fillets");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "vector_boundary",
        steps: &[
            Step {
                caption: "Draw one outline round several shapes, for a border or a backing board. Select the shapes.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_pair(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Create**, press **Vector Boundary**.",
                setup: |app, _| on_design_tab(app, 0),
                mark: Mark::At("tool:vector_boundary"),
            },
            Step {
                caption: "Choose how close the outline follows them, and how far out it sits.",
                setup: |app, ctx| {
                    app.invoke(ctx, "vector_boundary");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "vector_texture",
        steps: &[
            Step {
                caption: "Fill a shape with rows of wavy lines or a decorative pattern, to carve as a texture. Select the shape.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_rect(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Create**, press **Create Vector Texture**.",
                setup: |app, _| on_design_tab(app, 0),
                mark: Mark::At("tool:vector_texture"),
            },
            Step {
                caption: "Choose the pattern and its spacing, waves and angle. Change the **Seed** for another random variation.",
                setup: |app, ctx| {
                    app.invoke(ctx, "vector_texture");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "embellish",
        steps: &[
            Step {
                caption: "Embellish adds folk-style swirls and ornament designed to fit the shape. Select a closed shape for a fill, or an open line to run along.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_rect(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Create**, press **Embellish**.",
                setup: |app, _| on_design_tab(app, 0),
                mark: Mark::At("tool:embellish"),
            },
            Step {
                caption: "Pick a style and its settings. Warnings appear in red at the foot of the form.",
                setup: |app, ctx| {
                    app.invoke(ctx, "embellish");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "trace_bitmap",
        steps: &[
            Step {
                caption: "Trace turns a picture into shapes you can cut. Import the picture and select it.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_picture(app);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Create**, press **Trace Bitmap**.",
                setup: |app, _| on_design_tab(app, 0),
                mark: Mark::At("tool:trace_bitmap"),
            },
            Step {
                caption: "Choose black and white or colour, and how many colours. The preview shows the shapes; press **Apply** to keep them.",
                setup: |app, ctx| {
                    app.invoke(ctx, "trace_bitmap");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "quick_trace",
        steps: &[
            Step {
                caption: "Quick Trace turns a picture into shapes in one click, with settings chosen to suit it. Select the picture.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_picture(app);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Create**, press **Quick Trace**.",
                setup: |app, _| on_design_tab(app, 0),
                mark: Mark::At("tool:quick_trace"),
            },
            Step {
                caption: "The shapes are drawn over the picture. For more control, use [Trace Bitmap](help:trace_bitmap).",
                setup: |app, ctx| {
                    app.invoke(ctx, "quick_trace");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "document_variables",
        steps: &[
            Step {
                caption: "Variables are named numbers kept with the job, such as {thickness}, to use in any number box.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_none(app);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Create**, press **Document Variables**.",
                setup: |app, _| on_design_tab(app, 0),
                mark: Mark::At("tool:document_variables"),
            },
            Step {
                caption: "Add a name and a value. Type {name} in a number box to use it; change it here and every box using it follows.",
                setup: |app, ctx| {
                    app.invoke(ctx, "document_variables");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "node_edit",
        steps: &[
            Step {
                caption: "Node editing moves the points (nodes) a shape is made of, to reshape it. Select the shape.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_star(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Edit**, press **Node Edit**.",
                setup: |app, _| on_design_tab(app, 1),
                mark: Mark::At("tool:node_edit"),
            },
            Step {
                caption: "Drag a node (a square) to move it. Right-click a node or a span for more: add or delete nodes, make a span straight or curved. A curved span shows round handles: drag one to bend it ([Convert Span to Bezier](help:span_to_bezier), [Move Handle](help:move_handle)).",
                setup: |app, ctx| {
                    app.invoke(ctx, "node_edit");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "span_to_bezier",
        steps: &[
            Step {
                caption: "Convert Span to Bezier turns one side of a shape (a span) into a \
                          curve you can bend. Select the shape.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_rect(app);
                },
                mark: Mark::Shape(RECT_X, RECT_Y + RECT_H / 2.0),
            },
            Step {
                caption: "On the Design page, under **Edit**, press **Node Edit** (or press \
                          N). Small squares appear on the shape's corners: its nodes.",
                setup: |app, ctx| {
                    on_design_tab(app, 1);
                    app.invoke(ctx, "node_edit");
                },
                mark: Mark::At("tool:node_edit"),
            },
            Step {
                caption: "Right-click the side you want to curve and pick **Convert Span to \
                          Bezier**. Two round handles appear on it, each joined by a line to \
                          the corner it pulls from.",
                setup: |app, _| {
                    let at = serde_json::json!({
                        "vector": first_vector(app),
                        "at": { "x": RECT_X + RECT_W / 2.0, "y": RECT_Y },
                        "reach": 2.0,
                    });
                    let _ = app.apply("span_to_bezier", &at);
                },
                mark: Mark::None,
            },
            Step {
                caption: "Drag a round handle. The side bends towards it while the corners \
                          stay put. Drag the other handle to shape the other end of the curve.",
                setup: |app, _| {
                    let params = serde_json::json!({
                        "from": {
                            "vector": first_vector(app),
                            "at": { "x": RECT_X + RECT_W / 3.0, "y": RECT_Y },
                            "reach": 2.0,
                        },
                        "to": { "x": RECT_X + RECT_W / 3.0, "y": RECT_Y - 45.0 },
                    });
                    let _ = app.apply("move_handle", &params);
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "transform",
        steps: &[
            Step {
                caption: "Transform moves, turns and resizes with the handles round the selection. Select the shape.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_star(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Edit**, press **Transform**.",
                setup: |app, _| on_design_tab(app, 1),
                mark: Mark::At("tool:transform"),
            },
            Step {
                caption: "Drag a corner handle to resize, the round handle to turn it, or the middle to move it.",
                setup: |app, ctx| {
                    app.invoke(ctx, "transform");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "group",
        steps: &[
            Step {
                caption: "Grouping joins shapes so they select and move as one. Select them.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_pair(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Edit**, press **Group**.",
                setup: |app, _| on_design_tab(app, 1),
                mark: Mark::At("tool:group"),
            },
            Step {
                caption: "They now move together. Ungroup takes them apart again.",
                setup: |app, ctx| {
                    app.invoke(ctx, "group");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "measure_inspect",
        steps: &[
            Step {
                caption: "Measure distances and angles, and read a shape's size.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_rect(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Edit**, press **Measure / Inspect**.",
                setup: |app, _| on_design_tab(app, 1),
                mark: Mark::At("tool:measure_inspect"),
            },
            Step {
                caption: "Click two points to measure between them, or click a shape to see its size.",
                setup: |app, ctx| {
                    app.invoke(ctx, "measure_inspect");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "fit_curves_to_vectors",
        steps: &[
            Step {
                caption: "Fit Curves redraws rough shapes (from a trace or a scan) as smooth arcs and curves. Select them.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_star(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Edit**, press **Fit Curves to Vectors**.",
                setup: |app, _| on_design_tab(app, 1),
                mark: Mark::At("tool:fit_curves_to_vectors"),
            },
            Step {
                caption: "Choose arcs, curves or straight lines and how closely they follow the shape.",
                setup: |app, ctx| {
                    app.invoke(ctx, "fit_curves_to_vectors");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "vector_validator",
        steps: &[
            Step {
                caption: "Vector Validator finds shapes that would stop a toolpath: lines crossing themselves, doubled shapes, zero-length spans.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_star(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Design page, under **Edit**, press **Vector Validator**.",
                setup: |app, _| on_design_tab(app, 1),
                mark: Mark::At("tool:vector_validator"),
            },
            Step {
                caption: "It selects the faulty shapes so you can fix them, and can fix zero-length spans for you.",
                setup: |app, ctx| {
                    app.invoke(ctx, "vector_validator");
                },
                mark: Mark::None,
            },
        ],
    },
];
