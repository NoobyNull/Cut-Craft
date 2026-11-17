//! Each tool's tour for its help page's step pictures ([`super::help_pictures`]): the steps a
//! beginner takes, each set up in the app as they would leave it, with what to ring and the
//! words that go over its picture. A step builds on the one before.

use super::help_pictures::{Mark, Step, Tour};
use super::{CutCraftApp, View};
use crate::pages::Page;
use cut_craft_core::geometry::Point;
use eframe::egui;

/// Every tool's tour.
pub const TOURS: &[Tour] = &[
    Tour {
        tool: "job_setup",
        steps: &[
            Step {
                caption: "Press **New** (or File ▸ New) to start a job. The first thing it asks \
                          about is your material.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    app.pages.set_pinned(Page::Design, true);
                },
                mark: Mark::At("tool:new"),
            },
            Step {
                caption: "Measure your board and type its **Width**, **Height** and \
                          **Thickness**. Get the thickness right: it decides how deep a cut \
                          must go to come through.",
                setup: |app, ctx| app.invoke(ctx, "new"),
                mark: Mark::At("field:thickness"),
            },
            Step {
                caption: "Choose where **Z zero** is: the top of the material (most common) or \
                          the machine bed. Zero the bit in the same place on your machine before \
                          cutting.",
                setup: |_, _| {},
                mark: Mark::At("field:z_zero"),
            },
            Step {
                caption: "Pick the **XY zero** point: the corner (or the middle) of the board \
                          you will set as X0 Y0 on the machine.",
                setup: |_, _| {},
                mark: Mark::At("field:datum"),
            },
            Step {
                caption: "Press **OK**. The sheet on screen is now your board, ready to draw on.",
                setup: |_, _| {},
                mark: Mark::At("button:OK"),
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
    Tour {
        tool: "draw_circle",
        steps: &[
            Step {
                caption: "On the Design page, press **Draw Circle**.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    app.pages.set_pinned(Page::Design, true);
                },
                mark: Mark::At("tool:draw_circle"),
            },
            Step {
                caption: "Click on the sheet where the middle of the circle goes, move out, and \
                          click again when it is the size you want.",
                setup: |app, ctx| {
                    app.invoke(ctx, "draw_circle");
                    draw(app, "draw_circle", circle());
                },
                mark: Mark::Shape(CIRCLE_X - CIRCLE_R, CIRCLE_Y),
            },
            Step {
                caption: "To make it an exact size, type the **Radius** (or the centre's X and Y) \
                          in the form. The circle on the sheet follows what you type.",
                setup: |_, _| {},
                mark: Mark::At("field:radius"),
            },
        ],
    },
    Tour {
        tool: "pocket_toolpath",
        steps: &[
            Step {
                caption: "Draw the shape to hollow out on the Design page. Here it is a circle.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    draw(app, "draw_circle", circle());
                    app.form = None;
                    app.doc.selection.clear();
                },
                mark: Mark::Shape(CIRCLE_X - CIRCLE_R, CIRCLE_Y),
            },
            Step {
                caption: "Click the shape to select it. A pocket clears everything inside the \
                          selected shapes.",
                setup: |app, _| select(app, CIRCLE_X - CIRCLE_R, CIRCLE_Y),
                mark: Mark::Shape(CIRCLE_X - CIRCLE_R, CIRCLE_Y),
            },
            Step {
                caption: "On the Toolpaths page, press **Pocket** (under Cut Out and Clear).",
                setup: |app, _| app.pages.set_pinned(Page::Toolpaths, true),
                mark: Mark::At("tool:pocket_toolpath"),
            },
            Step {
                caption: "Set the **Cut Depth** (D): how far below the top of the material the \
                          floor of the pocket ends up. Here it is 10 mm.",
                setup: |app, ctx| {
                    app.invoke(ctx, "pocket_toolpath");
                    if let Some(open) = app.form.as_mut() {
                        open.params["depth"] = 10.0.into();
                    }
                },
                mark: Mark::At("field:depth"),
            },
            Step {
                caption: "Check the bit. **Select…** picks another one from your Tool Database; \
                          its feeds, speeds and depth per pass come with it.",
                setup: |_, _| {},
                mark: Mark::At("button:Select…"),
            },
            Step {
                caption: "Press **Calculate** to make the toolpath.",
                setup: |_, _| {},
                mark: Mark::At("button:Calculate"),
            },
            Step {
                caption: "The orange lines are the path the bit will follow, round and round \
                          until the inside is cleared.",
                setup: |app, _| {
                    if let Some(open) = &app.form {
                        let (id, params) = (open.id, open.params.clone());
                        app.calculate_checked(id, &params);
                    }
                },
                mark: Mark::Shape(CIRCLE_X - CIRCLE_R, CIRCLE_Y),
            },
            Step {
                caption: "Press **Preview All Toolpaths** to watch it cut in 3D before you send \
                          it to the machine.",
                setup: |app, ctx| {
                    app.form = None;
                    app.doc.selection.clear();
                    app.display.animate_preview = false;
                    app.invoke(ctx, "preview_all_toolpaths");
                    app.view = View::ThreeD;
                    app.framing =
                        crate::view3d::Framing::default().zoomed_about(egui::Vec2::ZERO, 1.6);
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "surfacing_toolpath",
        steps: &[
            Step {
                caption: "Surfacing flattens the top of the board, or your spoilboard, with a \
                          wide bit. Nothing needs to be selected to flatten the whole board. On \
                          the Toolpaths page, press **Surfacing** (under Cut Out and Clear).",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    app.pages.set_pinned(Page::Toolpaths, true);
                },
                mark: Mark::At("tool:surfacing_toolpath"),
            },
            Step {
                caption: "Choose what to flatten: the **whole material**, the **spoilboard** on \
                          your machine's bed, or the inside of the shapes you selected.",
                setup: |app, ctx| app.invoke(ctx, "surfacing_toolpath"),
                mark: Mark::At("field:area"),
            },
            Step {
                caption: "Set the **Depth to Take Off** the top. A light skim of a \
                          millimetre or so is usually enough to make it flat.",
                setup: |app, _| set(app, "depth", 1.0.into()),
                mark: Mark::At("field:depth"),
            },
            Step {
                caption: "Pick the **Pattern**: rows along X, rows along Y, or a spiral from the \
                          outside in. The bit always starts beside the board, never plunging \
                          into it.",
                setup: |_, _| {},
                mark: Mark::At("field:pattern"),
            },
            Step {
                caption: "Press **Calculate**. The rows run past every edge so the edges come \
                          out clean.",
                setup: |_, _| {},
                mark: Mark::At("button:Calculate"),
            },
            Step {
                caption: "Press **Preview All Toolpaths** to see the whole top skimmed flat.",
                setup: |app, ctx| {
                    calculate(app, ctx);
                    preview_3d(app, ctx);
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "drag_knife_toolpath",
        steps: &[
            Step {
                caption: "A drag knife cuts thin sheet (vinyl, card, thin plastic) with a \
                          blade that trails behind like a castor. Select the shapes to cut.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    draw(app, "draw_star", star());
                    app.form = None;
                    app.doc.selection = app.doc.vectors.keys().copied().collect();
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Toolpaths page, press **Drag Knife** (under Cut Out and Clear).",
                setup: |app, _| app.pages.set_pinned(Page::Toolpaths, true),
                mark: Mark::At("tool:drag_knife_toolpath"),
            },
            Step {
                caption: "Type the **Blade Offset** from the knife's maker: how far the tip \
                          trails behind the pivot. The knife swings round sharp corners by \
                          this much so the blade faces the new way.",
                setup: |app, ctx| app.invoke(ctx, "drag_knife_toolpath"),
                mark: Mark::At("field:blade_offset"),
            },
            Step {
                caption: "Set the **Pressure**: how hard the blade presses while it cuts. Use \
                          more for thicker or tougher sheet.",
                setup: |_, _| {},
                mark: Mark::At("field:pressure"),
            },
            Step {
                caption: "**Overshoot** runs each cut on a little past a sharp corner, so the \
                          cuts meet and the corner peels away cleanly.",
                setup: |_, _| {},
                mark: Mark::At("field:overshoot"),
            },
            Step {
                caption: "Press **Calculate** to make the toolpath. Switch the spindle off: the \
                          knife does not spin.",
                setup: |_, _| {},
                mark: Mark::At("button:Calculate"),
            },
            Step {
                caption: "The path follows each line, with small loops at the sharp points \
                          where the blade swivels round.",
                setup: calculate,
                mark: Mark::Everything,
            },
        ],
    },
    Tour {
        tool: "drill_bank_toolpath",
        steps: &[
            Step {
                caption: "A drill bank is a head with several drills side by side, found on \
                          some panel machines. Draw a circle for each hole and select them.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    holes(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Toolpaths page, press **Drill Bank** (under Holes).",
                setup: |app, _| app.pages.set_pinned(Page::Toolpaths, true),
                mark: Mark::At("tool:drill_bank_toolpath"),
            },
            Step {
                caption: "Set the **Depth** of the holes. The hole sizes come from the drills \
                          themselves.",
                setup: |app, ctx| {
                    app.invoke(ctx, "drill_bank_toolpath");
                    set(app, "depth", 12.0.into());
                },
                mark: Mark::At("field:depth"),
            },
            Step {
                caption: "The drills come from **Machine Configuration**. Describe your \
                          machine's drill bank there first: each drill's place and size. Then \
                          press **Calculate**: each plunge drops the drills that line up with \
                          holes and keeps the others up.",
                setup: |_, _| {},
                mark: Mark::At("button:Calculate"),
            },
        ],
    },
    Tour {
        tool: "thread_milling_toolpath",
        steps: &[
            Step {
                caption: "Thread milling cuts a screw thread inside a hole (a nut) or round a \
                          post (a bolt). Draw a circle at each thread and select them.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    holes(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Toolpaths page, press **Thread Milling** (under Holes).",
                setup: |app, _| app.pages.set_pinned(Page::Toolpaths, true),
                mark: Mark::At("tool:thread_milling_toolpath"),
            },
            Step {
                caption: "Choose **Internal** for a thread inside a hole, or **External** for a \
                          thread round a post.",
                setup: |app, ctx| app.invoke(ctx, "thread_milling_toolpath"),
                mark: Mark::At("field:thread_type"),
            },
            Step {
                caption: "Set the thread's **Diameter** (its largest size) and **Pitch** (the \
                          distance between ridges), or pick a standard thread.",
                setup: |_, _| {},
                mark: Mark::At("field:diameter"),
            },
            Step {
                caption: "Clear the hole first with a Pocket toolpath. Then press \
                          **Calculate**: the thread mill spirals round each centre.",
                setup: |_, _| {},
                mark: Mark::At("button:Calculate"),
            },
        ],
    },
    Tour {
        tool: "keyhole_toolpath",
        steps: &[
            Step {
                caption: "Keyhole slots let a sign hang flat on screws. On the back of the \
                          sign, draw a small circle where each slot starts and select them.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    for x in [120.0, 280.0] {
                        let hole = serde_json::json!({
                            "center": { "x": x, "y": 150.0 }, "radius": 5.0
                        });
                        draw(app, "draw_circle", hole);
                    }
                    app.form = None;
                    app.doc.selection = app.doc.vectors.keys().copied().collect();
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Toolpaths page, press **Keyhole** (under Holes).",
                setup: |app, _| app.pages.set_pinned(Page::Toolpaths, true),
                mark: Mark::At("tool:keyhole_toolpath"),
            },
            Step {
                caption: "Choose which way the slot runs from the hole (**Direction**). For a \
                          sign hung on the wall, the slot runs up, so the screw head slides to \
                          its top.",
                setup: |app, ctx| app.invoke(ctx, "keyhole_toolpath"),
                mark: Mark::At("field:direction"),
            },
            Step {
                caption: "Set the **Slot Length** and the **Slot Depth**: deep enough that the \
                          cutter's wide part is below the surface.",
                setup: |_, _| {},
                mark: Mark::At("field:slot_length"),
            },
            Step {
                caption: "Press **Calculate**. The keyhole cutter plunges at each circle and \
                          runs along the slot.",
                setup: |_, _| {},
                mark: Mark::At("button:Calculate"),
            },
            Step {
                caption: "The new lines show each slot running from its entry hole.",
                setup: calculate,
                mark: Mark::Shape(115.0, 150.0),
            },
        ],
    },
    Tour {
        tool: "quick_engrave_toolpath",
        steps: &[
            Step {
                caption: "Quick Engrave marks shapes or lettering at a shallow depth. Select \
                          what to engrave.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    draw(app, "draw_star", star());
                    app.form = None;
                    app.doc.selection = app.doc.vectors.keys().copied().collect();
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Toolpaths page, press **Quick Engrave** (under Carve and \
                          Engrave).",
                setup: |app, _| app.pages.set_pinned(Page::Toolpaths, true),
                mark: Mark::At("tool:quick_engrave_toolpath"),
            },
            Step {
                caption: "Choose **Outline** to run along the lines, or **Fill** to mark the \
                          whole inside with rings or hatching.",
                setup: |app, ctx| {
                    app.invoke(ctx, "quick_engrave_toolpath");
                    set(app, "strategy", "fill".into());
                },
                mark: Mark::At("field:strategy"),
            },
            Step {
                caption: "Set the **Depth**: a shallow mark, often half a millimetre.",
                setup: |_, _| {},
                mark: Mark::At("field:depth"),
            },
            Step {
                caption: "Press **Calculate** to make the toolpath.",
                setup: |_, _| {},
                mark: Mark::At("button:Calculate"),
            },
            Step {
                caption: "With Fill, the lines step in from the outline until the star is \
                          covered.",
                setup: calculate,
                mark: Mark::Everything,
            },
        ],
    },
    Tour {
        tool: "fluting_toolpath",
        steps: &[
            Step {
                caption: "Flutes are grooves that taper in and out, as on a column. Draw a line \
                          for each flute and select them. Each flute starts at its line's \
                          first point.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    lines(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Toolpaths page, press **Fluting** (under Carve and Engrave).",
                setup: |app, _| app.pages.set_pinned(Page::Toolpaths, true),
                mark: Mark::At("tool:fluting_toolpath"),
            },
            Step {
                caption: "Set the **Flute Depth**: how deep the groove is at its deepest.",
                setup: |app, ctx| app.invoke(ctx, "fluting_toolpath"),
                mark: Mark::At("field:flute_depth"),
            },
            Step {
                caption: "Choose the **Flute Type**: taper down over the whole line, taper at \
                          the start only, or taper in at the start and out at the end.",
                setup: |app, _| set(app, "flute_type", "ramp_at_start_and_end".into()),
                mark: Mark::At("field:flute_type"),
            },
            Step {
                caption: "Press **Calculate**. A ball nose bit gives rounded flutes, a V-bit \
                          sharp ones.",
                setup: |_, _| {},
                mark: Mark::At("button:Calculate"),
            },
            Step {
                caption: "Press **Preview All Toolpaths**: each flute is shallow at its ends and \
                          deepest in the middle.",
                setup: |app, ctx| {
                    calculate(app, ctx);
                    preview_3d(app, ctx);
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "prism_toolpath",
        steps: &[
            Step {
                caption: "Prism carving leaves each shape standing as a raised, ridged bevel, \
                          like a cut gem. Select the shapes.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    draw(app, "draw_star", star());
                    app.form = None;
                    app.doc.selection = app.doc.vectors.keys().copied().collect();
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Toolpaths page, press **Prism Carving** (under Carve and \
                          Engrave).",
                setup: |app, _| app.pages.set_pinned(Page::Toolpaths, true),
                mark: Mark::At("tool:prism_toolpath"),
            },
            Step {
                caption: "Set the **Prism Depth**: how deep the sloping sides go. Too shallow \
                          and the top is left flat; deep enough and they meet in a sharp \
                          ridge. **Depth for Full Prism** works that depth out for you.",
                setup: |app, ctx| {
                    app.invoke(ctx, "prism_toolpath");
                    set(app, "prism_depth", 6.0.into());
                },
                mark: Mark::At("field:prism_depth"),
            },
            Step {
                caption: "Check the bit is a **V-bit**: its sloping sides make the bevel.",
                setup: |_, _| {},
                mark: Mark::At("button:Select…"),
            },
            Step {
                caption: "Press **Calculate** to make the toolpath.",
                setup: |_, _| {},
                mark: Mark::At("button:Calculate"),
            },
            Step {
                caption: "Press **Preview All Toolpaths**: a sloping bevel runs round the star. A \
                          wider V-bit, or a smaller shape, brings the slopes together in a ridge.",
                setup: |app, ctx| {
                    calculate(app, ctx);
                    preview_3d(app, ctx);
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "texture_toolpath",
        steps: &[
            Step {
                caption: "Texture covers an area with shallow scallops of random depth and \
                          length, like a hand-tooled finish. Select the area's outline.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    rect(app);
                    app.form = None;
                    app.doc.selection = app.doc.vectors.keys().copied().collect();
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Toolpaths page, press **Texture** (under Carve and Engrave).",
                setup: |app, _| app.pages.set_pinned(Page::Toolpaths, true),
                mark: Mark::At("tool:texture_toolpath"),
            },
            Step {
                caption: "Set the **Max Cut Depth** and **Max Cut Length**: the deepest and \
                          longest any scallop can be. Each is shorter and shallower at random.",
                setup: |app, ctx| app.invoke(ctx, "texture_toolpath"),
                mark: Mark::At("field:max_cut_depth"),
            },
            Step {
                caption: "Set the **Stepover** (the gap between rows) and the rows' **Angle**.",
                setup: |_, _| {},
                mark: Mark::At("field:stepover"),
            },
            Step {
                caption: "Press **Calculate**. Change the **Seed** for another random pattern.",
                setup: |_, _| {},
                mark: Mark::At("button:Calculate"),
            },
            Step {
                caption: "Press **Preview All Toolpaths** to see the tooled surface.",
                setup: |app, ctx| {
                    calculate(app, ctx);
                    preview_3d(app, ctx);
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "chamfer_toolpath",
        steps: &[
            Step {
                caption: "A chamfer is a sloped edge. Select the shapes whose edges you want \
                          sloped.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    rect(app);
                    app.form = None;
                    app.doc.selection = app.doc.vectors.keys().copied().collect();
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Toolpaths page, press **Chamfer** (under Edges and Inlays).",
                setup: |app, _| app.pages.set_pinned(Page::Toolpaths, true),
                mark: Mark::At("tool:chamfer_toolpath"),
            },
            Step {
                caption: "Set the **Width** across the slope. With a V-bit, its angle sets how \
                          steep the slope is.",
                setup: |app, ctx| app.invoke(ctx, "chamfer_toolpath"),
                mark: Mark::At("field:width"),
            },
            Step {
                caption: "Choose the **Side**: **Outside** slopes the edge of a part you cut \
                          out, **Inside** slopes the rim of a hole or recess.",
                setup: |_, _| {},
                mark: Mark::At("field:side"),
            },
            Step {
                caption: "Press **Calculate** to make the toolpath.",
                setup: |_, _| {},
                mark: Mark::At("button:Calculate"),
            },
            Step {
                caption: "Press **Preview All Toolpaths** to see the sloped edge.",
                setup: |app, ctx| {
                    calculate(app, ctx);
                    preview_3d(app, ctx);
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "moulding_toolpath",
        steps: &[
            Step {
                caption: "A moulding sweeps a shape (the profile) along a path (the rail), like \
                          a picture frame's edge. Draw the rail, then the profile as an open \
                          line: across is its width, up is its height.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    moulding_shapes(app);
                    app.doc.selection.clear();
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "Select the rail first, then Shift-click the profile last: the open \
                          line picked last is the profile.",
                setup: |app, _| {
                    let ids: Vec<_> = app.doc.vectors.keys().copied().collect();
                    app.doc.selection = ids.into_iter().collect();
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Toolpaths page, press **Moulding** (under Edges and Inlays).",
                setup: |app, _| app.pages.set_pinned(Page::Toolpaths, true),
                mark: Mark::At("tool:moulding_toolpath"),
            },
            Step {
                caption: "Check the bit, usually a ball nose. Its stepover sets how close the \
                          passes run.",
                setup: |app, ctx| app.invoke(ctx, "moulding_toolpath"),
                mark: Mark::At("button:Select…"),
            },
            Step {
                caption: "Press **Calculate**. The passes follow the rail, each as low as the \
                          profile allows.",
                setup: |_, _| {},
                mark: Mark::At("button:Calculate"),
            },
            Step {
                caption: "Press **Preview All Toolpaths** to see the moulding.",
                setup: |app, ctx| {
                    calculate(app, ctx);
                    preview_3d(app, ctx);
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "vcarve_inlay_toolpath",
        steps: &[
            Step {
                caption: "A V-carve inlay is two carvings that fit together: a pocket in one \
                          board and a plug from another, turned over and glued in. Select the \
                          shapes.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    draw(app, "draw_star", star());
                    app.form = None;
                    app.doc.selection = app.doc.vectors.keys().copied().collect();
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Toolpaths page, press **V-Carve Inlay** (under Edges and \
                          Inlays).",
                setup: |app, _| app.pages.set_pinned(Page::Toolpaths, true),
                mark: Mark::At("tool:vcarve_inlay_toolpath"),
            },
            Step {
                caption: "Set the **Pocket Depth**: the deepest the pocket goes.",
                setup: |app, ctx| app.invoke(ctx, "vcarve_inlay_toolpath"),
                mark: Mark::At("field:pocket_depth"),
            },
            Step {
                caption: "Set the **Glue Gap**: the space left under the plug for glue once it \
                          is pressed home.",
                setup: |_, _| {},
                mark: Mark::At("field:glue_gap"),
            },
            Step {
                caption: "Press **Calculate**. You get two toolpaths: the pocket, and the plug \
                          carved round a mirrored copy.",
                setup: |_, _| {},
                mark: Mark::At("button:Calculate"),
            },
            Step {
                caption: "Press **Preview All Toolpaths** to check both halves.",
                setup: |app, ctx| {
                    calculate(app, ctx);
                    preview_3d(app, ctx);
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "inlay_toolpath",
        steps: &[
            Step {
                caption: "An inlay is a part and the hole it fits, both cut with one bit. \
                          Select the shape.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    draw(app, "draw_star", star());
                    app.form = None;
                    app.doc.selection = app.doc.vectors.keys().copied().collect();
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Toolpaths page, press **Inlay** (under Edges and Inlays).",
                setup: |app, _| app.pages.set_pinned(Page::Toolpaths, true),
                mark: Mark::At("tool:inlay_toolpath"),
            },
            Step {
                caption: "Choose which half this toolpath cuts: **Male** is the insert \
                          (straight, or stepped with a shelf round its base), **Female** is the \
                          pocket or hole it fits in. Make one toolpath for each half.",
                setup: |app, ctx| app.invoke(ctx, "inlay_toolpath"),
                mark: Mark::At("field:inlay_type"),
            },
            Step {
                caption: "Set the **Allowance**: the insert is cut this much smaller (or the \
                          pocket this much bigger) so they fit.",
                setup: |_, _| {},
                mark: Mark::At("field:allowance"),
            },
            Step {
                caption: "Press **Calculate**. The star's inside corners are rounded to the \
                          bit's radius, so the two halves match.",
                setup: |_, _| {},
                mark: Mark::At("button:Calculate"),
            },
            Step {
                caption: "The toolpath runs round the shape with the corners rounded.",
                setup: calculate,
                mark: Mark::Everything,
            },
        ],
    },
    Tour {
        tool: "material_setup",
        steps: &[
            Step {
                caption: "Before the first toolpath, tell Cut Craft about the board on the \
                          machine. On the Toolpaths page, press **Material Setup** (under Job \
                          and Tools). It also opens by itself the first time you make a \
                          toolpath.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    app.pages.set_pinned(Page::Toolpaths, true);
                },
                mark: Mark::At("tool:material_setup"),
            },
            Step {
                caption: "Check the **Thickness**: measure the board, because every depth is \
                          worked out from it.",
                setup: |app, ctx| app.invoke(ctx, "material_setup"),
                mark: Mark::At("field:thickness"),
            },
            Step {
                caption: "Choose where the bit's **Z zero** is: on the top of the board, or on \
                          the machine bed under it. Touch the bit off at the same place on the \
                          machine.",
                setup: |_, _| {},
                mark: Mark::At("field:z_zero"),
            },
            Step {
                caption: "Choose the **XY datum**: the corner (or the middle) of the board that \
                          is X0, Y0 on the machine.",
                setup: |_, _| {},
                mark: Mark::At("field:datum"),
            },
            Step {
                caption: "Check how high the bit lifts between cuts (**Rapid Z gaps**), then \
                          press **Apply**.",
                setup: |_, _| {},
                mark: Mark::At("button:Apply"),
            },
        ],
    },
    Tour {
        tool: "materials",
        steps: &[
            Step {
                caption: "Materials shows the job in the wood (or plastic or metal) you will \
                          cut. On the Toolpaths page, press **Materials** (under Job and \
                          Tools).",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    app.pages.set_pinned(Page::Toolpaths, true);
                },
                mark: Mark::At("tool:materials"),
            },
            Step {
                caption: "Pick the **Material**. Woods show their grain in the 3D view; \
                          **Custom** is a plain colour of your own.",
                setup: |app, ctx| {
                    app.invoke(ctx, "materials");
                    set(app, "material", "walnut".into());
                },
                mark: Mark::At("field:material"),
            },
            Step {
                caption: "Choose how the preview colours what the toolpaths cut (**Machined \
                          area**): the material's own colour, one colour, or each toolpath's \
                          colour. Then press **OK**.",
                setup: |_, _| {},
                mark: Mark::At("field:machined_area"),
            },
            Step {
                caption: "The 3D view and the toolpath previews now show the job in that \
                          material.",
                setup: |app, ctx| {
                    if let Some(open) = &app.form {
                        let (id, params) = (open.id, open.params.clone());
                        let _ = app.apply(id, &params);
                    }
                    app.form = None;
                    app.view = View::ThreeD;
                    app.framing = crate::view3d::Framing::default();
                    let _ = ctx;
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "tool_database",
        steps: &[
            Step {
                caption: "The Tool Database keeps your bits, each with its size, shape, feeds \
                          and speeds. On the Toolpaths page, press **Tool Database** (under Job \
                          and Tools).",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    app.pages.set_pinned(Page::Toolpaths, true);
                },
                mark: Mark::At("tool:tool_database"),
            },
            Step {
                caption: "**My Tools** are the bits you own; the makers' catalogues below are \
                          for copying from. Click a bit to see its drawing and settings, or add \
                          your own. Every toolpath's **Select…** picks from My Tools, and \
                          brings that bit's feeds, speeds and depth per pass with it.",
                setup: |app, ctx| app.invoke(ctx, "tool_database"),
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "keep_out_zones",
        steps: &[
            Step {
                caption: "A keep-out zone is an area the bit must never cross, such as a clamp. \
                          Draw round each clamp and select the shapes.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    for x in [40.0, 330.0] {
                        let clamp = serde_json::json!({
                            "x": x, "y": 130.0, "width": 30.0, "height": 40.0
                        });
                        let clamp = with(app, "draw_rectangle", &clamp);
                        draw(app, "draw_rectangle", clamp);
                    }
                    app.form = None;
                    app.doc.selection = app.doc.vectors.keys().copied().collect();
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Toolpaths page, press **Keep-Out Zones** (under Job and Tools).",
                setup: |app, _| app.pages.set_pinned(Page::Toolpaths, true),
                mark: Mark::At("tool:keep_out_zones"),
            },
            Step {
                caption: "Set the **Clearance**: how far beyond the shapes the zone reaches, so \
                          the side of the bit stays clear too.",
                setup: |app, ctx| {
                    app.invoke(ctx, "keep_out_zones");
                    set(app, "clearance", 5.0.into());
                },
                mark: Mark::At("field:clearance"),
            },
            Step {
                caption: "Give the zones a **Name** (they are labelled Clamp 1, Clamp 2…), then \
                          press **OK**. A toolpath that would cut into a zone is not made.",
                setup: |_, _| {},
                mark: Mark::At("field:name"),
            },
        ],
    },
    Tour {
        tool: "3d_roughing_toolpath",
        steps: &[
            Step {
                caption: "3D roughing clears away most of the wood above a 3D model quickly, \
                          leaving a little for the finishing toolpath. Make or load the model \
                          first: here, a dome.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    dome(app, ctx);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Toolpaths page, press **3D Roughing** (under 3D Toolpaths).",
                setup: |app, _| {
                    app.view = View::TwoD;
                    app.pages.set_pinned(Page::Toolpaths, true);
                },
                mark: Mark::At("tool:3d_roughing_toolpath"),
            },
            Step {
                caption: "Set the **Allowance**: the wood left on the model for the finishing \
                          toolpath to take off cleanly.",
                setup: |app, ctx| app.invoke(ctx, "3d_roughing_toolpath"),
                mark: Mark::At("field:allowance"),
            },
            Step {
                caption: "Choose the **Strategy**: **Z level** clears flat steps, one pass \
                          depth at a time; **3D raster** rides over the model in rows.",
                setup: |_, _| {},
                mark: Mark::At("field:strategy"),
            },
            Step {
                caption: "Use a big end mill to clear quickly, then press **Calculate**.",
                setup: |_, _| {},
                mark: Mark::At("button:Calculate"),
            },
            Step {
                caption: "Press **Preview All Toolpaths**: the dome comes out in steps, ready \
                          for finishing.",
                setup: |app, ctx| {
                    calculate(app, ctx);
                    preview_3d(app, ctx);
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "3d_finishing_toolpath",
        steps: &[
            Step {
                caption: "3D finishing runs a ball nose bit over a 3D model in fine rows, as \
                          low as it can go without cutting into it. Make or load the model \
                          first: here, a dome.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    dome(app, ctx);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Toolpaths page, press **3D Finishing** (under 3D Toolpaths).",
                setup: |app, _| {
                    app.view = View::TwoD;
                    app.pages.set_pinned(Page::Toolpaths, true);
                },
                mark: Mark::At("tool:3d_finishing_toolpath"),
            },
            Step {
                caption: "Choose the **Strategy**: **Raster** rows back and forth, or \
                          **Offset** rings following the outline.",
                setup: |app, ctx| app.invoke(ctx, "3d_finishing_toolpath"),
                mark: Mark::At("field:strategy"),
            },
            Step {
                caption: "Pick a ball nose bit. Its stepover sets how close the rows are: \
                          closer rows leave a smoother surface but take longer. Add more bits \
                          to finish the fine detail the first one cannot reach.",
                setup: |_, _| {},
                mark: Mark::At("field:tools"),
            },
            Step {
                caption: "Press **Calculate** to make the toolpath.",
                setup: |_, _| {},
                mark: Mark::At("button:Calculate"),
            },
            Step {
                caption: "Press **Preview All Toolpaths** to see the smooth dome.",
                setup: |app, ctx| {
                    calculate(app, ctx);
                    preview_3d(app, ctx);
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "rounding_toolpath",
        steps: &[
            Step {
                caption: "Rounding turns a square or round blank into a smooth cylinder on a \
                          rotary axis. Start a **rotary** job in Job Setup, with the finished \
                          diameter.",
                setup: |app, ctx| {
                    app.form = None;
                    app.reset_job();
                    app.invoke(ctx, "new");
                    set(app, "job_type", "rotary".into());
                },
                mark: Mark::At("field:job_type"),
            },
            Step {
                caption: "On the Toolpaths page, press **Rounding** (under 3D Toolpaths).",
                setup: |app, ctx| {
                    app.job_setup_ok();
                    (app.design_subtab, app.toolpath_subtab) = (0, 0);
                    app.view = View::TwoD;
                    app.camera = None;
                    app.pages.set_pinned(Page::Toolpaths, true);
                    let _ = ctx;
                },
                mark: Mark::At("tool:rounding_toolpath"),
            },
            Step {
                caption: "Say what you start from (**Blank**: square or round) and its \
                          **Blank size**.",
                setup: |app, ctx| app.invoke(ctx, "rounding_toolpath"),
                mark: Mark::At("field:blank"),
            },
            Step {
                caption: "Choose the **Method**: round the cylinder as it turns, or along it in \
                          rows.",
                setup: |_, _| {},
                mark: Mark::At("field:method"),
            },
            Step {
                caption: "Press **Calculate** to make the toolpath.",
                setup: |_, _| {},
                mark: Mark::At("button:Calculate"),
            },
        ],
    },
    Tour {
        tool: "laser_cut_fill_toolpath",
        steps: &[
            Step {
                caption: "A laser can cut shapes out or mark them with a hatched fill. Select \
                          the shapes.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    draw(app, "draw_star", star());
                    app.form = None;
                    app.doc.selection = app.doc.vectors.keys().copied().collect();
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Toolpaths page, press **Laser Cut / Fill** (under Laser).",
                setup: |app, _| app.pages.set_pinned(Page::Toolpaths, true),
                mark: Mark::At("tool:laser_cut_fill_toolpath"),
            },
            Step {
                caption: "Choose the **Strategy**: cut outside, inside or on the line (the \
                          beam's width is allowed for), or **Hatch fill** to mark the inside \
                          with lines.",
                setup: |app, ctx| {
                    app.invoke(ctx, "laser_cut_fill_toolpath");
                    set(app, "strategy", "hatch_fill".into());
                    set(app, "stepover", 2.0.into());
                },
                mark: Mark::At("field:strategy"),
            },
            Step {
                caption: "Set the **Power** and the number of **Passes**: more passes cut \
                          through thicker material. Try them on a scrap first.",
                setup: |_, _| {},
                mark: Mark::At("field:power"),
            },
            Step {
                caption: "Press **Calculate** to make the toolpath.",
                setup: |_, _| {},
                mark: Mark::At("button:Calculate"),
            },
            Step {
                caption: "The hatch lines fill the star, with an outline round it for a crisp \
                          edge.",
                setup: calculate,
                mark: Mark::Everything,
            },
        ],
    },
    Tour {
        tool: "laser_picture_toolpath",
        steps: &[
            Step {
                caption: "A laser can burn a photo or picture: darker parts get more power. \
                          Import the picture (drop it on the sheet) and click it to select it.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    picture(app);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Toolpaths page, press **Laser Picture** (under Laser).",
                setup: |app, _| app.pages.set_pinned(Page::Toolpaths, true),
                mark: Mark::At("tool:laser_picture_toolpath"),
            },
            Step {
                caption: "Set the **Power** for black. Light parts get less.",
                setup: |app, ctx| app.invoke(ctx, "laser_picture_toolpath"),
                mark: Mark::At("field:power"),
            },
            Step {
                caption: "Tick **Dither image** for materials that only burn or don't (such as \
                          some plastics): the laser switches fully on and off in tiny dots to \
                          give the greys.",
                setup: |_, _| {},
                mark: Mark::At("field:dither_image"),
            },
            Step {
                caption: "Set the **Line spacing**: closer lines give more detail but take \
                          longer. Then press **Calculate**.",
                setup: |app, _| set(app, "line_spacing", 10.0.into()),
                mark: Mark::At("field:line_spacing"),
            },
            Step {
                caption: "The toolpath follows the picture: more power where it is darker. \
                          It joins the toolpath list, ready to save for the laser.",
                setup: calculate,
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "photo_vcarve_toolpath",
        steps: &[
            Step {
                caption: "Photo V-carving carves a picture as grooves with a V-bit: deeper, so \
                          wider, where it is darker. Import the picture and select it.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    picture(app);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Toolpaths page, press **Photo V-Carve** (under Carve and \
                          Engrave).",
                setup: |app, _| app.pages.set_pinned(Page::Toolpaths, true),
                mark: Mark::At("tool:photo_vcarve_toolpath"),
            },
            Step {
                caption: "Set the **Max carving depth**: how deep the grooves go where the \
                          picture is black.",
                setup: |app, ctx| {
                    app.invoke(ctx, "photo_vcarve_toolpath");
                    set(app, "max_carving_depth", 3.0.into());
                },
                mark: Mark::At("field:max_carving_depth"),
            },
            Step {
                caption: "Choose the **Strategy**: straight lines across (**Raster**), crossing \
                          lines (**Hatch**), or along vectors you drew.",
                setup: |_, _| {},
                mark: Mark::At("field:strategy"),
            },
            Step {
                caption: "Press **Calculate**. Use a V-bit with a fine point for detail.",
                setup: |_, _| {},
                mark: Mark::At("button:Calculate"),
            },
            Step {
                caption: "Press **Preview All Toolpaths** to see the picture in grooves.",
                setup: |app, ctx| {
                    calculate(app, ctx);
                    preview_3d(app, ctx);
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "sketch_carving_toolpath",
        steps: &[
            Step {
                caption: "Sketch carving carves the lines of a picture, like a pencil sketch: \
                          wherever it is darker than around it. Import the picture and select \
                          it.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    picture(app);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Toolpaths page, press **Sketch Carving** (under Carve and \
                          Engrave).",
                setup: |app, _| app.pages.set_pinned(Page::Toolpaths, true),
                mark: Mark::At("tool:sketch_carving_toolpath"),
            },
            Step {
                caption: "Choose the **Source**: the picture, or the 3D model's dips and \
                          edges.",
                setup: |app, ctx| app.invoke(ctx, "sketch_carving_toolpath"),
                mark: Mark::At("field:source"),
            },
            Step {
                caption: "Set the **Line thickness**: heavier lines, but less detail, as it \
                          goes up.",
                setup: |_, _| {},
                mark: Mark::At("field:line_thickness"),
            },
            Step {
                caption: "Press **Calculate**, then **Preview All Toolpaths** to see the \
                          sketch.",
                setup: |_, _| {},
                mark: Mark::At("button:Calculate"),
            },
            Step {
                caption: "The sketch's lines carved into the board.",
                setup: |app, ctx| {
                    calculate(app, ctx);
                    preview_3d(app, ctx);
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "array_copy_toolpath",
        steps: &[
            Step {
                caption: "Array Copy repeats a toolpath in a grid, to cut several of the same \
                          part from one board. Make the toolpath for one part first: here a \
                          Profile round a circle.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    small_profile(app, ctx);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "On the Toolpaths page, press **Array Copy** (under From Other \
                          Toolpaths).",
                setup: |app, _| {
                    app.form = None;
                    app.pages.set_pinned(Page::Toolpaths, true);
                },
                mark: Mark::At("tool:array_copy_toolpath"),
            },
            Step {
                caption: "Set how many **Rows** and **Columns** of copies, counting the first.",
                setup: |app, ctx| {
                    app.invoke(ctx, "array_copy_toolpath");
                    set(app, "rows", 2.into());
                    set(app, "columns", 3.into());
                },
                mark: Mark::At("field:rows"),
            },
            Step {
                caption: "Set the distances between copies across (**X**) and up (**Y**) the \
                          job.",
                setup: |app, _| {
                    set(app, "x", 110.0.into());
                    set(app, "y", 110.0.into());
                },
                mark: Mark::At("field:x"),
            },
            Step {
                caption: "Press **Calculate**. The copies follow the first: change and \
                          recalculate it, and they change too.",
                setup: |_, _| {},
                mark: Mark::At("button:Calculate"),
            },
            Step {
                caption: "Six parts from one toolpath.",
                setup: calculate,
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "merged_toolpath",
        steps: &[
            Step {
                caption: "Merged joins toolpaths that use the same bit into one, in the order \
                          that travels least. Make the toolpaths first: here a Profile round the \
                          circle…",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    two_shapes(app);
                    select(app, CIRCLE_X - CIRCLE_R, CIRCLE_Y);
                    app.invoke(ctx, "profile_toolpath");
                    calculate(app, ctx);
                },
                mark: Mark::Shape(CIRCLE_X - CIRCLE_R, CIRCLE_Y),
            },
            Step {
                caption: "…and a Profile round the rectangle, with the same bit.",
                setup: |app, ctx| {
                    app.form = None;
                    select(app, RECT2_X, RECT_Y + RECT_H / 2.0);
                    app.invoke(ctx, "profile_toolpath");
                    calculate(app, ctx);
                },
                mark: Mark::Shape(RECT2_X, RECT_Y + RECT_H / 2.0),
            },
            Step {
                caption: "On the Toolpaths page, press **Merged** (under From Other \
                          Toolpaths). It merges the toolpaths shown in the list.",
                setup: |app, _| {
                    app.form = None;
                    app.doc.selection.clear();
                    app.pages.set_pinned(Page::Toolpaths, true);
                },
                mark: Mark::At("tool:merged_toolpath"),
            },
            Step {
                caption: "Tick the orders to try: left to right, bottom to top, in a grid, or \
                          nearest next. The quickest is kept. **Merge by part** keeps each \
                          part's cuts together.",
                setup: |app, ctx| app.invoke(ctx, "merged_toolpath"),
                mark: Mark::At("field:shortest_path"),
            },
            Step {
                caption: "Press **Calculate**: one toolpath now cuts both, and the originals \
                          are hidden.",
                setup: |_, _| {},
                mark: Mark::At("button:Calculate"),
            },
        ],
    },
    Tour {
        tool: "load_template",
        steps: &[
            Step {
                caption: "A toolpath template brings saved toolpath settings and bits into this \
                          job. On the Toolpaths page, press **Load Toolpath Template** (under \
                          Templates) and pick the template file.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    two_shapes(app);
                    app.pages.set_pinned(Page::Toolpaths, true);
                },
                mark: Mark::At("tool:load_template"),
            },
            Step {
                caption: "Its toolpaths join the list, ready for you to pick the vectors each \
                          one cuts: select the shapes, edit the toolpath and press \
                          **Calculate**.",
                setup: |_, _| {},
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "save_selected_toolpath_as_template",
        steps: &[
            Step {
                caption: "Save a toolpath you like as a template, to make the same cut on \
                          other jobs. Click it in the toolpath list to select it.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    small_profile(app, ctx);
                },
                mark: Mark::None,
            },
            Step {
                caption: "Press **Save Toolpath as Template** (under Templates) and name the \
                          file. It keeps the settings and the bits, not the shapes.",
                setup: |app, _| {
                    app.form = None;
                    app.selected_toolpath = app.doc.toolpaths.keys().next().copied();
                    app.pages.set_pinned(Page::Toolpaths, true);
                },
                mark: Mark::At("tool:save_selected_toolpath_as_template"),
            },
        ],
    },
    Tour {
        tool: "save_all_visible_toolpaths_as_template",
        steps: &[
            Step {
                caption: "Save every toolpath shown in the list as one template: a whole \
                          job's cuts, such as a sign's lettering, pocket and cut-out, ready for \
                          the next sign. Untick any you do not want in it.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    small_profile(app, ctx);
                },
                mark: Mark::None,
            },
            Step {
                caption: "Press **Save All Shown Toolpaths as Template** (under Templates) and \
                          name the file. Use **Load Toolpath Template** to bring them into \
                          another job.",
                setup: |app, _| {
                    app.form = None;
                    app.pages.set_pinned(Page::Toolpaths, true);
                },
                mark: Mark::At("tool:save_all_visible_toolpaths_as_template"),
            },
        ],
    },
    Tour {
        tool: "toolpath_summary",
        steps: &[
            Step {
                caption: "Once the toolpaths are made, open the **Analyse** tab on the \
                          Toolpaths page and press **Toolpath Summary**.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    small_profile(app, ctx);
                },
                mark: Mark::None,
            },
            Step {
                caption: "Press **Toolpath Summary** (under Reports).",
                setup: |app, _| {
                    app.form = None;
                    app.pages.set_pinned(Page::Toolpaths, true);
                    app.toolpath_subtab = 2;
                },
                mark: Mark::At("tool:toolpath_summary"),
            },
            Step {
                caption: "It lists each toolpath with its bit and how long it takes to cut, \
                          and the whole job's time.",
                setup: |app, ctx| app.invoke(ctx, "toolpath_summary"),
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "create_job_sheet",
        steps: &[Step {
            caption: "A job sheet is a page to take to the machine: the job, the material, \
                          and each toolpath's bit and time. Open the **Output** tab on the \
                          Toolpaths page and press **Save Job Sheet** (under Paperwork for the \
                          Job), then name the file.",
            setup: |app, ctx| {
                fresh_job(app, ctx);
                small_profile(app, ctx);
                app.pages.set_pinned(Page::Toolpaths, true);
                app.toolpath_subtab = 3;
            },
            mark: Mark::At("tool:create_job_sheet"),
        }],
    },
    Tour {
        tool: "print_job_sheet",
        steps: &[Step {
            caption: "To print the job sheet, open the **Output** tab on the Toolpaths page \
                          and press **Print Job Sheet** (under Paperwork for the Job). It opens \
                          in your web browser; print it from there.",
            setup: |app, ctx| {
                fresh_job(app, ctx);
                small_profile(app, ctx);
                app.pages.set_pinned(Page::Toolpaths, true);
                app.toolpath_subtab = 3;
            },
            mark: Mark::At("tool:print_job_sheet"),
        }],
    },
    Tour {
        tool: "part_labels",
        steps: &[
            Step {
                caption: "Part labels are stickers for the parts you cut out, one for each \
                          named part. Open the **Output** tab on the Toolpaths page and press \
                          **Print Part Labels** (under Paperwork for the Job).",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    small_profile(app, ctx);
                    app.pages.set_pinned(Page::Toolpaths, true);
                    app.toolpath_subtab = 3;
                },
                mark: Mark::At("tool:part_labels"),
            },
            Step {
                caption: "Choose what goes on each label and the label sheet, then print.",
                setup: |app, ctx| {
                    app.form = None;
                    app.invoke(ctx, "part_labels");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "cut_list",
        steps: &[Step {
            caption: "A cut list is a spreadsheet of the job's parts and their sizes. Open \
                          the **Output** tab on the Toolpaths page and press **Save Cut List** \
                          (under Paperwork for the Job), then name the file. It opens in any \
                          spreadsheet program.",
            setup: |app, ctx| {
                fresh_job(app, ctx);
                small_profile(app, ctx);
                app.pages.set_pinned(Page::Toolpaths, true);
                app.toolpath_subtab = 3;
            },
            mark: Mark::At("tool:cut_list"),
        }],
    },
    Tour {
        tool: "toolpath_tiling",
        steps: &[
            Step {
                caption: "Tiling cuts a job bigger than your machine in pieces: the board is \
                          moved along between tiles. Open the **Output** tab on the Toolpaths \
                          page and press **Toolpath Tiling** (under Big Jobs).",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    small_profile(app, ctx);
                    app.pages.set_pinned(Page::Toolpaths, true);
                    app.toolpath_subtab = 3;
                },
                mark: Mark::At("tool:toolpath_tiling"),
            },
            Step {
                caption: "Tick it on, and set how the job is split into tiles: their size and \
                          how much they overlap.",
                setup: |app, ctx| {
                    app.form = None;
                    app.invoke(ctx, "toolpath_tiling");
                    set(app, "enabled", true.into());
                },
                mark: Mark::At("field:tiling"),
            },
            Step {
                caption: "Press **Apply**. The tiles are drawn on the job, and **Save \
                          Toolpaths** writes one file for each tile.",
                setup: |_, _| {},
                mark: Mark::At("button:Apply"),
            },
        ],
    },
];

const CIRCLE_X: f64 = 200.0;
const CIRCLE_Y: f64 = 150.0;
const CIRCLE_R: f64 = 90.0;

fn circle() -> serde_json::Value {
    serde_json::json!({ "center": { "x": CIRCLE_X, "y": CIRCLE_Y }, "radius": CIRCLE_R })
}

const RECT_X: f64 = 60.0;
const RECT_Y: f64 = 60.0;
const RECT_W: f64 = 260.0;
/// The rectangle beside the circle in [`two_shapes`].
const RECT2_X: f64 = CIRCLE_X + CIRCLE_R + 40.0;
const RECT_H: f64 = 160.0;
const STAR_X: f64 = 200.0;
const STAR_Y: f64 = 150.0;
const STAR_R: f64 = 100.0;

fn rectangle() -> serde_json::Value {
    serde_json::json!({ "x": RECT_X, "y": RECT_Y, "width": RECT_W, "height": RECT_H })
}

fn star() -> serde_json::Value {
    serde_json::json!({
        "center": { "x": STAR_X, "y": STAR_Y },
        "outer_radius": STAR_R,
        "inner_radius": STAR_R * 0.45,
        "points": 5,
        "rotation": 90.0
    })
}

/// A circle and, off to its right, a rectangle.
fn two_shapes(app: &mut CutCraftApp) {
    draw(app, "draw_circle", circle());
    let mut rect = rectangle();
    rect["x"] = RECT2_X.into();
    let rect = with(app, "draw_rectangle", &rect);
    draw(app, "draw_rectangle", rect);
    app.form = None;
    app.doc.selection.clear();
}

/// Tool `id`'s own settings with `overrides` laid over them.
fn with(app: &CutCraftApp, id: &str, overrides: &serde_json::Value) -> serde_json::Value {
    let mut params = app.registry.get(id).map_or(
        serde_json::Value::Null,
        cut_craft_core::DynCommand::default_params,
    );
    if let (Some(base), Some(over)) = (params.as_object_mut(), overrides.as_object()) {
        for (k, v) in over {
            base.insert(k.clone(), v.clone());
        }
    }
    params
}

/// Presses the open form's Calculate.
fn calculate(app: &mut CutCraftApp, _: &egui::Context) {
    if let Some(open) = &app.form {
        let (id, params) = (open.id, open.params.clone());
        app.calculate_checked(id, &params);
    }
}

/// Every toolpath previewed, cut into the material in the 3D view.
fn preview_3d(app: &mut CutCraftApp, ctx: &egui::Context) {
    app.form = None;
    app.doc.selection.clear();
    app.display.animate_preview = false;
    app.invoke(ctx, "preview_all_toolpaths");
    app.view = View::ThreeD;
    app.framing = crate::view3d::Framing::default().zoomed_about(egui::vec2(-70.0, -10.0), 2.2);
}

/// A new job, nothing drawn, the 2D view.
fn fresh_job(app: &mut CutCraftApp, ctx: &egui::Context) {
    app.form = None;
    // the last tour's windows closed
    (app.tool_manager, app.info, app.part_labels_window) = (None, None, false);
    // the last tour's job goes without asking whether to save it
    app.reset_job();
    app.invoke(ctx, "new");
    app.job_setup_ok();
    // each page's first tab, as a fresh start shows them
    (app.design_subtab, app.toolpath_subtab) = (0, 0);
    app.view = View::TwoD;
    app.camera = None;
}

/// Runs drawing tool `id` with `params`, as its form would.
fn draw(app: &mut CutCraftApp, id: &str, params: serde_json::Value) {
    let _ = app.apply(id, &params);
    if let Some(open) = app.form.as_mut().filter(|f| f.id == id) {
        open.params = params;
    }
}

/// Selects the shape through `(x, y)` of the job.
fn select(app: &mut CutCraftApp, x: f64, y: f64) {
    let at = app.doc.job.area().min + Point::new(x, y);
    app.doc.selection = app.doc.pick(at, 2.0).into_iter().collect();
}

/// Sets setting `key` of the open form to `value`.
fn set(app: &mut CutCraftApp, key: &str, value: serde_json::Value) {
    if let Some(open) = app.form.as_mut() {
        open.params[key] = value;
    }
}

/// Four small circles in a row, all selected: holes to drill or thread.
fn holes(app: &mut CutCraftApp) {
    for x in [100.0, 180.0, 260.0, 340.0] {
        let hole = serde_json::json!({ "center": { "x": x, "y": 150.0 }, "radius": 8.0 });
        draw(app, "draw_circle", hole);
    }
    app.form = None;
    app.doc.selection = app.doc.vectors.keys().copied().collect();
}

/// Three upright open lines, all selected: flutes.
fn lines(app: &mut CutCraftApp) {
    for x in [140.0, 200.0, 260.0] {
        let line = serde_json::json!({
            "points": [{ "x": x, "y": 50.0 }, { "x": x, "y": 250.0 }],
            "closed": false,
            "curves": []
        });
        draw(app, "draw_polyline", with(app, "draw_polyline", &line));
    }
    app.form = None;
    app.doc.selection = app.doc.vectors.keys().copied().collect();
}

/// A rectangle (the rail) and, to its right, an open quarter-round (the profile).
fn moulding_shapes(app: &mut CutCraftApp) {
    rect(app);
    // 40 mm across, 12 mm high: a quarter round
    let (x, y, across, high) = (RECT_X + RECT_W + 30.0, RECT_Y, 40.0, 12.0);
    let points: Vec<_> = (0..=8)
        .map(|i| {
            let a = f64::from(i) / 8.0 * std::f64::consts::FRAC_PI_2;
            serde_json::json!({ "x": x + across * (1.0 - a.cos()), "y": y + high * a.sin() })
        })
        .collect();
    let profile = serde_json::json!({ "points": points, "closed": false, "curves": [] });
    draw(app, "draw_polyline", with(app, "draw_polyline", &profile));
    app.form = None;
}

/// The [`rectangle`], drawn.
fn rect(app: &mut CutCraftApp) {
    let rect = with(app, "draw_rectangle", &rectangle());
    draw(app, "draw_rectangle", rect);
}

/// A circle made into a dome-shaped 3D model.
fn dome(app: &mut CutCraftApp, ctx: &egui::Context) {
    draw(app, "draw_circle", circle());
    app.form = None;
    select(app, CIRCLE_X - CIRCLE_R, CIRCLE_Y);
    app.invoke(ctx, "create_shape_round");
    // lower than the material is thick
    set(app, "total_height", 12.0.into());
    if let Some(open) = &app.form {
        let (id, params) = (open.id, open.params.clone());
        let _ = app.apply(id, &params);
    }
    app.form = None;
    app.doc.selection.clear();
    app.view = View::ThreeD;
}

/// A small circle near the job's corner with a Profile toolpath round it.
fn small_profile(app: &mut CutCraftApp, ctx: &egui::Context) {
    let part = serde_json::json!({ "center": { "x": 70.0, "y": 70.0 }, "radius": 40.0 });
    draw(app, "draw_circle", part);
    app.form = None;
    select(app, 30.0, 70.0);
    app.invoke(ctx, "profile_toolpath");
    calculate(app, ctx);
    app.form = None;
}

/// A made-up picture, selected: a sun over hills under a sky that darkens upwards.
fn picture(app: &mut CutCraftApp) {
    use cut_craft_core::bitmap::Bitmap;
    let (columns, rows) = (240_u32, 150_u32);
    let grey = (0..rows)
        .flat_map(|r| {
            (0..columns).map(move |c| {
                landscape(
                    f64::from(c) / f64::from(columns),
                    f64::from(r) / f64::from(rows),
                )
            })
        })
        .collect();
    let mut change = cut_craft_core::Change::builder(&app.doc);
    let rect =
        cut_craft_core::geometry::Rect::new(Point::new(40.0, 50.0), Point::new(360.0, 250.0));
    let id = change.add_bitmap(Bitmap {
        name: "Hills".into(),
        columns: columns as usize,
        rows: rows as usize,
        grey,
        rect,
        depth: Vec::new(),
        rgb: Vec::new(),
    });
    app.record(Ok(change.finish()), "Picture added");
    app.doc.selected_bitmaps = [id].into();
}

/// The made-up picture's grey at `(x, y)` across and down it (0 to 1): 0 black, 255 white.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn landscape(x: f64, y: f64) -> u8 {
    let far = 0.55 + 0.08 * (x * 9.0).sin() + 0.05 * (x * 23.0).cos();
    let near = 0.72 + 0.1 * (x * 5.0 + 1.0).sin();
    let sun = ((x - 0.7).powi(2) + (y - 0.3).powi(2)).sqrt() < 0.12;
    let tone = if y > near {
        0.15 + 0.1 * (x * 60.0).sin().abs()
    } else if y > far {
        0.4
    } else if sun {
        0.97
    } else {
        0.55 + 0.4 * y
    };
    (tone.clamp(0.0, 1.0) * 255.0) as u8
}
