//! The first tours: a job's material, drawing, selecting, the first toolpaths and saving them.

use super::{
    CIRCLE_R, CIRCLE_X, CIRCLE_Y, Mark, Page, Point, RECT_H, RECT_X, RECT_Y, RECT2_X, Step, Tour,
    calculate, circle, draw, fresh_job, on_design_tab, preview_3d, rectangle, select, star,
    two_shapes, with,
};

pub(super) const TOURS: &[Tour] = &[
    Tour {
        tool: "setup_material",
        steps: &[
            Step {
                caption: "Cut Craft opens with a blank job, made with your Job Defaults. To \
                          change this job's material, press **Setup Material** (under File \
                          Operations, at the top of the Design page).",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    on_design_tab(app, 0);
                },
                mark: Mark::At("tool:setup_material"),
            },
            Step {
                caption: "Pick the kind of job: **Single Sided** (cut from the top), **Double \
                          Sided** (turned over to cut the back) or **Rotary** (a cylinder on a \
                          rotary axis).",
                setup: |app, ctx| app.invoke(ctx, "setup_material"),
                mark: Mark::At("field:job_type"),
            },
            Step {
                caption: "Measure your board and type its **Width**, **Height** and \
                          **Thickness**. Get the thickness right: it decides how deep a cut \
                          must go to come through.",
                setup: |_, _| {},
                mark: Mark::At("field:thickness"),
            },
            Step {
                caption: "The line under them says where **Z zero** and **X0 Y0** are. **More \
                          settings for this job…** changes them for this job only; **Change my \
                          defaults…** changes them for every new job.",
                setup: |_, _| {},
                mark: Mark::At("button:More settings for this job…"),
            },
            Step {
                caption: "Press **Apply**. The sheet on screen is now your board, ready to draw \
                          on.",
                setup: |_, _| {},
                mark: Mark::At("button:Apply"),
            },
        ],
    },
    Tour {
        tool: "job_setup",
        steps: &[
            Step {
                caption: "Job Setup holds every setting of this job's material. Press **Setup \
                          Material** (under File Operations).",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    on_design_tab(app, 0);
                },
                mark: Mark::At("tool:setup_material"),
            },
            Step {
                caption: "Press **More settings for this job…** under the size: the rest of \
                          Job Setup opens below it.",
                setup: |app, ctx| app.invoke(ctx, "setup_material"),
                mark: Mark::At("button:More settings for this job…"),
            },
            Step {
                caption: "Choose where **Z zero** is: the top of the material (most common) or \
                          the machine bed. Zero the bit in the same place on your machine before \
                          cutting.",
                setup: |app, _| app.show_all_job_settings(),
                mark: Mark::At("field:z_zero"),
            },
            Step {
                caption: "Pick the **XY zero** point: the corner (or the middle) of the board \
                          you will set as X0 Y0 on the machine.",
                setup: |_, _| {},
                mark: Mark::At("field:datum"),
            },
            Step {
                caption: "Press **Apply**. Only this job changes: new jobs start with Edit ▸ \
                          Options ▸ Job Defaults.",
                setup: |_, _| {},
                mark: Mark::At("button:Apply"),
            },
        ],
    },
    Tour {
        tool: "draw_rectangle",
        steps: &[
            Step {
                caption: "On the Design page, press **Draw Rectangle**.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    app.pages.set_pinned(Page::Design, true);
                },
                mark: Mark::At("tool:draw_rectangle"),
            },
            Step {
                caption: "Click on the sheet where one corner goes, move to the opposite \
                          corner, and click again.",
                setup: |app, ctx| {
                    app.invoke(ctx, "draw_rectangle");
                    let params = with(app, "draw_rectangle", &rectangle());
                    draw(app, "draw_rectangle", params);
                },
                mark: Mark::Shape(RECT_X, RECT_Y + RECT_H / 2.0),
            },
            Step {
                caption: "For an exact size, type the **Width** and **Height** in the form. The \
                          rectangle on the sheet follows what you type.",
                setup: |_, _| {},
                mark: Mark::At("field:width"),
            },
            Step {
                caption: "**Corners** can be square, rounded or cut in; **Radius** sets how big \
                          the rounding is.",
                setup: |app, _| {
                    let mut params = rectangle();
                    params["corners"] = "rounded".into();
                    params["radius"] = 20.0.into();
                    let params = with(app, "draw_rectangle", &params);
                    draw(app, "draw_rectangle", params);
                },
                mark: Mark::At("field:corners"),
            },
        ],
    },
    Tour {
        tool: "draw_text",
        steps: &[
            Step {
                caption: "On the Design page, press **Draw Text**.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    app.pages.set_pinned(Page::Design, true);
                },
                mark: Mark::At("tool:draw_text"),
            },
            Step {
                caption: "Click on the sheet where the words start, then type them in the \
                          **Text** box.",
                setup: |app, ctx| {
                    app.invoke(ctx, "draw_text");
                    let params = with(
                        app,
                        "draw_text",
                        &serde_json::json!({
                            "text": "CUT CRAFT",
                            "height": 40.0,
                            "anchor": { "x": 40.0, "y": 140.0 }
                        }),
                    );
                    draw(app, "draw_text", params);
                },
                mark: Mark::At("field:text"),
            },
            Step {
                caption: "Pick a **Font** and set the **Height** of the capital letters. The \
                          words on the sheet change as you go.",
                setup: |_, _| {},
                mark: Mark::At("field:height"),
            },
        ],
    },
    Tour {
        tool: "select",
        steps: &[
            Step {
                caption: "Press **Select** (or Esc) to pick shapes. Most tools work on the \
                          shapes you have selected.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    two_shapes(app);
                    app.pages.set_pinned(Page::Design, true);
                    // Select is on the Edit tab
                    app.design_subtab = 1;
                },
                mark: Mark::At("tool:select"),
            },
            Step {
                caption: "Click a shape's line to select it. Handles appear round it: drag them \
                          to move, size or turn it.",
                setup: |app, _| select(app, CIRCLE_X - CIRCLE_R, CIRCLE_Y),
                mark: Mark::Shape(CIRCLE_X - CIRCLE_R, CIRCLE_Y),
            },
            Step {
                caption: "Hold **Shift** and click another shape to add it. Dragging a box \
                          round shapes selects them all; click empty space to select nothing.",
                setup: |app, _| {
                    let o = app.doc.job.area().min;
                    let picked = [
                        Point::new(CIRCLE_X - CIRCLE_R, CIRCLE_Y),
                        Point::new(RECT2_X, RECT_Y + RECT_H / 2.0),
                    ];
                    app.doc.selection = picked
                        .iter()
                        .filter_map(|p| app.doc.pick(o + *p, 2.0))
                        .collect();
                },
                mark: Mark::Everything,
            },
        ],
    },
    Tour {
        tool: "profile_toolpath",
        steps: &[
            Step {
                caption: "Draw the part you want to cut out, and click it to select it.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    draw(app, "draw_circle", circle());
                    app.form = None;
                    select(app, CIRCLE_X - CIRCLE_R, CIRCLE_Y);
                },
                mark: Mark::Shape(CIRCLE_X - CIRCLE_R, CIRCLE_Y),
            },
            Step {
                caption: "On the Toolpaths page, press **Profile** (under Cut Out and Clear).",
                setup: |app, _| app.pages.set_pinned(Page::Toolpaths, true),
                mark: Mark::At("tool:profile_toolpath"),
            },
            Step {
                caption: "Set the **Cut Depth** (D). To cut a part free, make it the thickness \
                          of the material.",
                setup: |app, ctx| {
                    app.invoke(ctx, "profile_toolpath");
                    let thickness = app.doc.job.thickness;
                    if let Some(open) = app.form.as_mut() {
                        open.params["depth"] = thickness.into();
                    }
                },
                mark: Mark::At("field:depth"),
            },
            Step {
                caption: "Choose which side the bit runs: **Outside** cuts out a part the size \
                          you drew, **Inside** cuts a hole that size, **On** follows the line.",
                setup: |_, _| {},
                mark: Mark::At("field:side"),
            },
            Step {
                caption: "Add **Tabs**: small bridges that hold the part in the board until the \
                          last pass, so it cannot fly loose.",
                setup: |_, _| {},
                mark: Mark::At("field:tabs"),
            },
            Step {
                caption: "Press **Calculate** to make the toolpath.",
                setup: |_, _| {},
                mark: Mark::At("button:Calculate"),
            },
            Step {
                caption: "The new line is where the middle of the bit will travel: just outside \
                          the shape, so the part comes out the size you drew.",
                setup: calculate,
                mark: Mark::Shape(CIRCLE_X - CIRCLE_R, CIRCLE_Y),
            },
            Step {
                caption: "Press **Preview All Toolpaths** to watch it cut in 3D before you send \
                          it to the machine.",
                setup: preview_3d,
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "vcarve_engrave_toolpath",
        steps: &[
            Step {
                caption: "Select the letters or shapes to carve. V-carving suits lettering and \
                          shapes with sharp corners.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    draw(app, "draw_star", star());
                    app.form = None;
                    app.doc.selection = app.doc.vectors.keys().copied().collect();
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Toolpaths page, press **V-Carve** (under Carve and Engrave).",
                setup: |app, _| app.pages.set_pinned(Page::Toolpaths, true),
                mark: Mark::At("tool:vcarve_engrave_toolpath"),
            },
            Step {
                caption: "Choose how to carve. **Fill** carves the whole shape, as deep as each \
                          part's width needs, so the sides meet in a sharp crease.",
                setup: |app, ctx| app.invoke(ctx, "vcarve_engrave_toolpath"),
                mark: Mark::At("field:place"),
            },
            Step {
                caption: "Check the bit is a **V-bit**. **Select…** picks one from your Tool \
                          Database; its angle sets how deep each part is carved.",
                setup: |_, _| {},
                mark: Mark::At("button:Select…"),
            },
            Step {
                caption: "Press **Calculate**. Wide parts are carved deeper, narrow points \
                          shallower.",
                setup: |_, _| {},
                mark: Mark::At("button:Calculate"),
            },
            Step {
                caption: "Press **Preview All Toolpaths** to see the carving in 3D.",
                setup: |app, ctx| {
                    calculate(app, ctx);
                    preview_3d(app, ctx);
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "drilling_toolpath",
        steps: &[
            Step {
                caption: "Draw a small circle where each hole goes, then select them all. Each \
                          circle's middle is where the bit goes down.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    for x in [100.0, 180.0, 260.0, 340.0] {
                        let hole = serde_json::json!({
                            "center": { "x": x, "y": 150.0 }, "radius": 8.0
                        });
                        draw(app, "draw_circle", hole);
                    }
                    app.form = None;
                    app.doc.selection = app.doc.vectors.keys().copied().collect();
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Toolpaths page, press **Drilling** (under Holes).",
                setup: |app, _| app.pages.set_pinned(Page::Toolpaths, true),
                mark: Mark::At("tool:drilling_toolpath"),
            },
            Step {
                caption: "Set the **Cut Depth** (D): how deep each hole goes.",
                setup: |app, ctx| {
                    app.invoke(ctx, "drilling_toolpath");
                    if let Some(open) = app.form.as_mut() {
                        open.params["depth"] = 12.0.into();
                    }
                },
                mark: Mark::At("field:depth"),
            },
            Step {
                caption: "Turn on **Peck** for deep holes: the bit lifts out now and then to \
                          clear the chips, so it does not jam or burn.",
                setup: |_, _| {},
                mark: Mark::At("field:peck"),
            },
            Step {
                caption: "Press **Calculate** to make the toolpath. Each circle becomes one \
                          straight plunge.",
                setup: |_, _| {},
                mark: Mark::At("button:Calculate"),
            },
        ],
    },
    Tour {
        tool: "save_toolpaths",
        steps: &[
            Step {
                caption: "When the toolpaths look right in the preview, press **Save Toolpaths** \
                          (Save for the Machine).",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    draw(app, "draw_circle", circle());
                    app.form = None;
                    select(app, CIRCLE_X - CIRCLE_R, CIRCLE_Y);
                    let params = app
                        .registry
                        .get("profile_toolpath")
                        .map(cut_craft_core::DynCommand::default_params);
                    if let Some(params) = params {
                        let _ = app.apply("profile_toolpath", &params);
                    }
                    app.doc.selection.clear();
                    app.pages.set_pinned(Page::Toolpaths, true);
                    // saving is on the Output tab
                    app.toolpath_subtab = 3;
                },
                mark: Mark::At("tool:save_toolpaths"),
            },
            Step {
                caption: "Pick your machine's **post processor** (the kind of file it reads), \
                          then press **Save** and choose where the file goes, such as the card \
                          or stick you take to the machine.",
                setup: |app, ctx| app.invoke(ctx, "save_toolpaths"),
                mark: Mark::None,
            },
        ],
    },
];
