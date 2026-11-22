//! Each tool's tour for its help page's step pictures ([`super::help_pictures`]): the steps a
//! beginner takes, each set up in the app as they would leave it, with what to ring and the
//! words that go over its picture. A step builds on the one before.

use super::help_pictures::{Mark, Step, Tour};
use super::{CutCraftApp, View};
use crate::pages::Page;
use cut_craft_core::geometry::Point;
use eframe::egui;
use std::fmt::Write as _;

/// Every tool's tour.
pub const TOURS: &[Tour] = &[
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
        tool: "rotary_machining_toolpath",
        steps: &[
            Step {
                caption: "4-Axis Rotary Machining finishes a model all the way round a turning \
                          cylinder. Start a **rotary** job and place a full 3D model in it: \
                          here, a turned bead.",
                setup: turned_bead,
                mark: Mark::None,
            },
            Step {
                caption: "On the Toolpaths page, press **4-Axis Rotary Machining** (under 3D \
                          Toolpaths).",
                setup: |app, _| {
                    app.view = View::TwoD;
                    app.pages.set_pinned(Page::Toolpaths, true);
                },
                mark: Mark::At("tool:rotary_machining_toolpath"),
            },
            Step {
                caption: "Pick a ball nose bit: its finishing stepover spaces the rings round \
                          the part.",
                setup: |app, ctx| app.invoke(ctx, "rotary_machining_toolpath"),
                mark: Mark::At("field:tool"),
            },
            Step {
                caption: "Say where along the part the rings **Start** and **End**, clear of \
                          the chuck and the tailstock.",
                setup: |_, _| {},
                mark: Mark::At("field:start"),
            },
            Step {
                caption: "Tick **Registration mark** for a small notch at A=0, so the part can \
                          go back exactly as it was if the job is started again.",
                setup: |_, _| {},
                mark: Mark::At("field:registration_mark"),
            },
            Step {
                caption: "Press **Calculate** to make the toolpath: a ring round the part at \
                          each step along it.",
                setup: |_, _| {},
                mark: Mark::At("button:Calculate"),
            },
        ],
    },
    Tour {
        tool: "rotary_lathe_toolpath",
        steps: &[
            Step {
                caption: "Rotary Lathe spins the material like a lathe while the bit stays over \
                          the turning axis. Start a **rotary** job: here, a bead in a 60 mm bar.",
                setup: turned_bead,
                mark: Mark::None,
            },
            Step {
                caption: "On the Toolpaths page, press **Rotary Lathe** (under 3D Toolpaths).",
                setup: |app, _| {
                    app.view = View::TwoD;
                    app.pages.set_pinned(Page::Toolpaths, true);
                },
                mark: Mark::At("tool:rotary_lathe_toolpath"),
            },
            Step {
                caption: "Pick a square end mill: it leaves a flat face. Its pass depth sets \
                          each step down, and its stepover the turns along the part.",
                setup: |app, ctx| app.invoke(ctx, "rotary_lathe_toolpath"),
                mark: Mark::At("field:tool"),
            },
            Step {
                caption: "Choose what it does: **Face** an end down to a stub, **Turn** the part \
                          round to its outline, or **Part off** with one groove.",
                setup: |_, _| {},
                mark: Mark::At("field:operation"),
            },
            Step {
                caption: "Say where along the part it cuts with **Start** and **End**, measured \
                          from the chuck end.",
                setup: |_, _| {},
                mark: Mark::At("field:start"),
            },
            Step {
                caption: "**Holding stub** is how thick a stub is left on the axis, keeping the \
                          part joined on until it is cut free.",
                setup: |_, _| {},
                mark: Mark::At("field:holding_stub"),
            },
            Step {
                caption: "Press **Calculate**: a turn round the part at each step along it, a \
                          level at a time, only where there is material.",
                setup: |_, _| {},
                mark: Mark::At("button:Calculate"),
            },
        ],
    },
    Tour {
        tool: "rounding_toolpath",
        steps: &[
            Step {
                caption: "Rounding turns a square or round blank into a smooth cylinder on a \
                          rotary axis. Press **Setup Material** and pick **Rotary**, with the \
                          finished diameter.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    app.invoke(ctx, "setup_material");
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
    Tour {
        tool: "new",
        steps: &[
            Step {
                caption: "Start a new, blank job, made with your Job Defaults (Edit ▸ Options ▸ Job Defaults).",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_none(app);
                },
                mark: Mark::None,
            },
            Step {
                caption: "Press **New** (under File Operations, at the top of the Design page). A blank sheet opens: change its size with [Setup Material](help:setup_material).",
                setup: |app, _| on_design_tab(app, 0),
                mark: Mark::At("tool:new"),
            },
        ],
    },
    Tour {
        tool: "open",
        steps: &[
            Step {
                caption: "Open a job you saved before (a .ccjob file).",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_none(app);
                },
                mark: Mark::None,
            },
            Step {
                caption: "Press **Open** (under File Operations) and pick the file.",
                setup: |app, _| on_design_tab(app, 0),
                mark: Mark::At("tool:open"),
            },
        ],
    },
    Tour {
        tool: "save",
        steps: &[
            Step {
                caption: "Save the job: the drawing, the 3D model and the toolpaths, all in one file. Save often.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_circle(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "Press **Save** (under File Operations). The first time, name the file; after that it saves over it.",
                setup: |app, _| on_design_tab(app, 0),
                mark: Mark::At("tool:save"),
            },
        ],
    },
    Tour {
        tool: "export_vectors",
        steps: &[
            Step {
                caption: "Send your drawing to another program, as DXF, SVG or EPS. Select what to send, or nothing for everything.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_circle(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "Press **Export Vectors** (under File Operations), choose the type and name the file.",
                setup: |app, _| on_design_tab(app, 0),
                mark: Mark::At("tool:export_vectors"),
            },
        ],
    },
    Tour {
        tool: "paste_in_place",
        steps: &[
            Step {
                caption: "Paste in Place puts what you copied exactly where it was, rather than where the pointer is: handy for copying between sheets or jobs. Copy something first (Ctrl+C).",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_circle(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "Press **Paste in Place** (under File Operations).",
                setup: |app, _| on_design_tab(app, 0),
                mark: Mark::At("tool:paste_in_place"),
            },
            Step {
                caption: "The copy lands on top of where the original was, ready to move or change.",
                setup: |app, ctx| {
                    app.invoke(ctx, "copy");
                    app.invoke(ctx, "paste_in_place");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "open_folder",
        steps: &[
            Step {
                caption: "Open the folder the job is saved in, to find its files.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_none(app);
                },
                mark: Mark::None,
            },
            Step {
                caption: "Press **Open Folder** (under File Operations). Your file manager opens there.",
                setup: |app, _| on_design_tab(app, 0),
                mark: Mark::At("tool:open_folder"),
            },
        ],
    },
    Tour {
        tool: "import_vectors_bitmap",
        steps: &[
            Step {
                caption: "Bring in a drawing (DXF, SVG) or a picture (PNG, JPG and more) to cut, trace or draw over. You can also just drop the file on the sheet.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_none(app);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, open **Import** and press **Import Vectors or Picture**, then pick the file.",
                setup: |app, _| on_design_tab(app, 4),
                mark: Mark::At("tool:import_vectors_bitmap"),
            },
        ],
    },
    Tour {
        tool: "import_vectors",
        steps: &[
            Step {
                caption: "Bring in a drawing made in another program: DXF or SVG.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_none(app);
                },
                mark: Mark::None,
            },
            Step {
                caption: "Under **Import**, press **Import Vectors** and pick the file. A DXF asks which units it was drawn in.",
                setup: |app, _| on_design_tab(app, 4),
                mark: Mark::At("tool:import_vectors"),
            },
            Step {
                caption: "Each layer of a DXF comes in as a layer of your job, with its name and colour. Open the **Layers** tab to see them, show or hide them, or change their colour.",
                setup: |app, ctx| {
                    use cut_craft_core::Importer as _;
                    const SHEET: &[u8] = include_bytes!(
                        "../../formats/import-dxf/tests/fixtures/nesting-sheet-layers.dxf"
                    );
                    let sheet = fmt_import_dxf::ImportDxf::import(&app.doc, SHEET);
                    app.record(sheet, "Imported");
                    app.design_tab = super::DESIGN_TABS
                        .iter()
                        .position(|&t| t == "Layers")
                        .unwrap_or(0);
                    // the whole drawing in view, then nothing selected, so its colours show
                    app.invoke(ctx, "zoom_selected");
                    app.doc.selection.clear();
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "import_into_shape",
        steps: &[
            Step {
                caption: "Fit a drawing or picture inside a shape you already have: select the shape first (a circle, plaque or heart).",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_circle(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "Under **Import**, press **Import Into Selected Shape** and pick the file. Choose to scale it, stretch it or bend it to the outline. Select an open line instead to bend the drawing along it.",
                setup: |app, _| on_design_tab(app, 4),
                mark: Mark::At("tool:import_into_shape"),
            },
        ],
    },
    Tour {
        tool: "import_component_3d_model",
        steps: &[
            Step {
                caption: "Bring in a 3D model (STL or OBJ) or saved 3D clipart to carve.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_none(app);
                },
                mark: Mark::None,
            },
            Step {
                caption: "Under **Import**, press **Import 3D Model** and pick the file, then size and place it.",
                setup: |app, _| on_design_tab(app, 4),
                mark: Mark::At("tool:import_component_3d_model"),
            },
        ],
    },
    Tour {
        tool: "clipart_library",
        steps: &[
            Step {
                caption: "The clipart library holds ready-made drawings, pictures and 3D models in your folders.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_none(app);
                },
                mark: Mark::None,
            },
            Step {
                caption: "On the Design page, under **Import**, press **Clipart Library**.",
                setup: |app, _| on_design_tab(app, 4),
                mark: Mark::At("tool:clipart_library"),
            },
            Step {
                caption: "The Library opens: drag a piece onto the job.",
                setup: |app, ctx| {
                    app.invoke(ctx, "clipart_library");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "import_toolpaths_file",
        steps: &[
            Step {
                caption: "Read a G-code file back in as toolpaths, to preview and check it before it runs.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_none(app);
                },
                mark: Mark::None,
            },
            Step {
                caption: "Under **Import**, press **Import Toolpaths (G-code)** and pick the file.",
                setup: |app, _| on_design_tab(app, 4),
                mark: Mark::At("tool:import_toolpaths_file"),
            },
        ],
    },
    Tour {
        tool: "import_cut_list",
        steps: &[
            Step {
                caption: "Draw a rectangle for each part in a spreadsheet's cut list (saved as CSV): name, length, width and how many.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_none(app);
                },
                mark: Mark::None,
            },
            Step {
                caption: "Under **Import**, press **Import Cut List** and pick the file. Then use [Nest Parts](help:nest_parts) to lay them out on the board.",
                setup: |app, _| on_design_tab(app, 4),
                mark: Mark::At("tool:import_cut_list"),
            },
        ],
    },
    Tour {
        tool: "pan",
        steps: &[
            Step {
                caption: "Pan slides the view around without changing anything.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_circle(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "Press **Pan** (under 2D View Control).",
                setup: |app, _| on_design_tab(app, 0),
                mark: Mark::At("tool:pan"),
            },
            Step {
                caption: "Drag in the view to slide it. The mouse wheel zooms in and out wherever you are.",
                setup: |app, ctx| {
                    app.invoke(ctx, "pan");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "zoom_box",
        steps: &[
            Step {
                caption: "Zoom Box zooms in on the part you draw a box round.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_circle(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "Press **Zoom Box** (under 2D View Control).",
                setup: |app, _| on_design_tab(app, 0),
                mark: Mark::At("tool:zoom_box"),
            },
            Step {
                caption: "Drag a box round the part to see close up.",
                setup: |app, ctx| {
                    app.invoke(ctx, "zoom_box");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "zoom_to_job",
        steps: &[
            Step {
                caption: "Zoom to Job fits the whole board in the view again.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_circle(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "Press **Zoom to Job** (under 2D View Control).",
                setup: |app, _| on_design_tab(app, 0),
                mark: Mark::At("tool:zoom_to_job"),
            },
            Step {
                caption: "The whole job fits the view.",
                setup: |app, ctx| {
                    app.invoke(ctx, "zoom_to_job");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "zoom_selected",
        steps: &[
            Step {
                caption: "Zoom Selected fits what you selected in the view. Select it first.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_circle(app);
                },
                mark: Mark::Everything,
            },
            Step {
                caption: "Press **Zoom Selected** (under 2D View Control).",
                setup: |app, _| on_design_tab(app, 0),
                mark: Mark::At("tool:zoom_selected"),
            },
            Step {
                caption: "The view zooms to fit the selection.",
                setup: |app, ctx| {
                    app.invoke(ctx, "zoom_selected");
                },
                mark: Mark::None,
            },
        ],
    },
    Tour {
        tool: "interface_layout",
        steps: &[
            Step {
                caption: "Choose how the panels are laid out, and save layouts of your own.",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    shapes_none(app);
                },
                mark: Mark::None,
            },
            Step {
                caption: "Press **Interface Layout** (under 2D View Control).",
                setup: |app, _| on_design_tab(app, 0),
                mark: Mark::At("tool:interface_layout"),
            },
            Step {
                caption: "Pick a layout, or save the one you have.",
                setup: |app, ctx| {
                    app.invoke(ctx, "interface_layout");
                },
                mark: Mark::None,
            },
        ],
    },
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
                caption: "Under **Create**, press **Draw Rectangle From Centre** (it shares a button with Draw Rectangle).",
                setup: |app, _| on_design_tab(app, 0),
                mark: Mark::At("tool:draw_rectangle_from_center"),
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
                caption: "Under **Create**, press **Draw Circle From Edge** (it shares a button with Draw Circle).",
                setup: |app, _| on_design_tab(app, 0),
                mark: Mark::At("tool:draw_circle_from_edge"),
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
    // The sheets, layers, components and toolpath lists, node editing's right-click menu and
    // the menu commands: each shown before and after, the menu named in the words.
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
    // the last tour's windows closed, so none is left in the next tour's pictures
    (app.tool_manager, app.info, app.part_labels_window) = (None, None, false);
    // the Design tab down the left (not Sheets, Layers or the Library)
    app.design_tab = 0;
    (app.save_form, app.gcode_view) = (None, None);
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

/// A rotary job 60 mm across and 160 mm long along X, holding a turned bead: round, fatter
/// in the middle, all the way round the axis.
fn turned_bead(app: &mut CutCraftApp, ctx: &egui::Context) {
    use cut_craft_core::component::{Combine, Component};
    use cut_craft_core::document::{Along, Job};
    fresh_job(app, ctx);
    let job = Job {
        clearance: 5.0,
        ..Job::rotary(160.0, 60.0, Along::X)
    };
    let mut change = cut_craft_core::Change::builder(&app.doc);
    change.set_job(job);
    app.record(Ok(change.finish()), "Rotary job");
    let bead = Component::from_fn("Bead", Combine::Add, app.doc.job.area(), 0.5, |p| {
        let t = p.x / 160.0;
        Some(18.0 + 12.0 * (std::f64::consts::PI * t).sin().powi(2))
    });
    let mut change = cut_craft_core::Change::builder(&app.doc);
    change.add_new_component(bead);
    app.record(Ok(change.finish()), "Bead added");
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

/// The Design page pinned open on sub-tab `tab` (0 Create, 1 Edit, 2 Arrange, 4 Import).
/// The Depth add-on, as Add-ons leaves it once installed, in an add-ons folder of the
/// pictures' own (so not the user's).
fn depth_installed(app: &mut CutCraftApp) {
    let dir = std::env::temp_dir().join("cut-craft-help-pictures-addons");
    let depth = dir.join("depth");
    let _ = std::fs::create_dir_all(&depth);
    let _ = std::fs::write(
        depth.join("addon.toml"),
        "id = \"depth\"\nname = \"Depth (Relief from Photo)\"\nkind = \"tool\"\nversion = \"0.1.0\"",
    );
    let _ = std::fs::write(
        depth.join("installed.json"),
        r#"{"repo":{"owner":"NoobyNull","name":"CutCraft-Depth"},"tag":"v0.1.0","version":"0.1.0","signed":false}"#,
    );
    app.addons = crate::addons::Addons::new(Some(dir));
    app.refresh_installed();
}

/// The Borders add-on, as if installed (a stand-in program, never run in a tour).
fn borders_installed(app: &mut CutCraftApp) {
    let dir = std::env::temp_dir().join("cut-craft-help-pictures-addons");
    let borders = dir.join(crate::borders::ADDON_ID);
    let _ = std::fs::create_dir_all(&borders);
    let _ = std::fs::write(
        borders.join("addon.toml"),
        "id = \"borders\"\nname = \"Borders (Generate Border)\"\nkind = \"tool\"\nversion = \"0.1.0\"",
    );
    let _ = std::fs::write(borders.join(crate::borders::program_name()), b"");
    let _ = std::fs::write(
        borders.join("installed.json"),
        r#"{"repo":{"owner":"Power-Whittle","name":"CutCraft-Borders"},"tag":"v0.1.0","version":"0.1.0","signed":false}"#,
    );
    // a key box of its own, empty: never the computer's password store
    let key = dir.join("google-key");
    let _ = std::fs::remove_file(&key);
    app.border.keys = Some(crate::borders::KeyStore::File(key));
    app.addons = crate::addons::Addons::new(Some(dir));
    app.refresh_installed();
}

/// A border as Generate saves it, in a Library of its own (not the user's): the hand-drawn
/// sample from `scripts/border_sample_picture.py`.
fn sample_border(app: &mut CutCraftApp) {
    let root = std::env::temp_dir().join("cut-craft-help-pictures-library");
    let folder = crate::borders::folder(&root);
    let files = cut_craft_border_tile::files(&folder, "Classical Baroque");
    let _ = std::fs::create_dir_all(&folder);
    let _ = std::fs::write(&files.depth, include_bytes!("../samples/border-depth.png"));
    let _ = std::fs::write(&files.picture, include_bytes!("../samples/border.jpg"));
    let recipe = cut_craft_border_tile::Recipe::new("classical_baroque", "16:9", 160.0, 90.0, 6.0);
    if let Ok(json) = serde_json::to_string_pretty(&recipe) {
        let _ = std::fs::write(&files.recipe, json);
    }
    app.clipart = crate::clipart::Clipart::default();
    app.clipart.folders = vec![root];
    app.clipart.choose(Some(folder));
}

fn on_design_tab(app: &mut CutCraftApp, tab: usize) {
    app.pages.set_pinned(Page::Design, true);
    app.design_subtab = tab;
}

/// Nothing drawn: the tool needs nothing selected.
fn shapes_none(_: &mut CutCraftApp) {}

/// Everything drawn so far, selected in the order drawn.
fn select_all(app: &mut CutCraftApp) {
    app.form = None;
    app.doc.selection = app.doc.vectors.keys().copied().collect();
}

/// A circle, selected.
fn shapes_circle(app: &mut CutCraftApp) {
    draw(app, "draw_circle", circle());
    select_all(app);
}

/// A star, selected.
fn shapes_star(app: &mut CutCraftApp) {
    draw(app, "draw_star", star());
    select_all(app);
}

/// The first vector drawn, as a node-editing command names it.
fn first_vector(app: &CutCraftApp) -> serde_json::Value {
    serde_json::json!(app.doc.vectors.keys().next().map_or(0, |id| id.0))
}

/// A rectangle, selected.
fn shapes_rect(app: &mut CutCraftApp) {
    rect(app);
    select_all(app);
}

/// A circle and a smaller rectangle overlapping it, both selected (the rectangle last).
fn shapes_pair(app: &mut CutCraftApp) {
    draw(app, "draw_circle", circle());
    let over = serde_json::json!({ "x": 230.0, "y": 90.0, "width": 160.0, "height": 110.0 });
    let over = with(app, "draw_rectangle", &over);
    draw(app, "draw_rectangle", over);
    select_all(app);
}

/// Three circles of different sizes, unevenly spaced and out of line, all selected.
fn shapes_three(app: &mut CutCraftApp) {
    for (x, y, r) in [
        (80.0, 120.0, 30.0),
        (150.0, 190.0, 45.0),
        (330.0, 140.0, 35.0),
    ] {
        let c = serde_json::json!({ "center": { "x": x, "y": y }, "radius": r });
        draw(app, "draw_circle", c);
    }
    select_all(app);
}

/// Three open lines whose ends nearly meet, all selected.
fn shapes_lines(app: &mut CutCraftApp) {
    let spans = [
        [(80.0, 80.0), (200.0, 80.0)],
        [(206.0, 84.0), (206.0, 220.0)],
        [(200.0, 226.0), (80.0, 226.0)],
    ];
    for [(x0, y0), (x1, y1)] in spans {
        let line = serde_json::json!({
            "points": [{ "x": x0, "y": y0 }, { "x": x1, "y": y1 }],
            "closed": false,
            "curves": []
        });
        draw(app, "draw_polyline", with(app, "draw_polyline", &line));
    }
    select_all(app);
}

/// Words made with Draw Text, selected.
fn shapes_text(app: &mut CutCraftApp) {
    let words = serde_json::json!({
        "text": "CUT CRAFT", "height": 40.0, "anchor": { "x": 60.0, "y": 140.0 }
    });
    let words = with(app, "draw_text", &words);
    draw(app, "draw_text", words);
    select_all(app);
}

/// Words, then an arch to lay them on, both selected (the arch last).
fn shapes_text_curve(app: &mut CutCraftApp) {
    let words = serde_json::json!({
        "text": "CUT CRAFT", "height": 24.0, "anchor": { "x": 120.0, "y": 60.0 }
    });
    let words = with(app, "draw_text", &words);
    draw(app, "draw_text", words);
    let arch = serde_json::json!({
        "center": { "x": 200.0, "y": 90.0 }, "radius": 120.0,
        "start_angle": 20.0, "end_angle": 160.0
    });
    let arch = with(app, "draw_arc", &arch);
    draw(app, "draw_arc", arch);
    select_all(app);
}

/// Words and a plaque round them, both selected.
fn shapes_text_rect(app: &mut CutCraftApp) {
    shapes_rect(app);
    shapes_text(app);
}

/// The made-up picture, selected.
fn shapes_picture(app: &mut CutCraftApp) {
    picture(app);
}

/// The made-up picture and a circle over it, both selected.
fn shapes_picture_circle(app: &mut CutCraftApp) {
    picture(app);
    draw(app, "draw_circle", circle());
    select_all(app);
}

/// A small star, then a curve to copy it along, both selected (the star first).
fn shapes_copy_along(app: &mut CutCraftApp) {
    let small = serde_json::json!({
        "center": { "x": 60.0, "y": 150.0 }, "outer_radius": 20.0, "inner_radius": 9.0,
        "points": 5, "rotation": 90.0
    });
    let small = with(app, "draw_star", &small);
    draw(app, "draw_star", small);
    let path = serde_json::json!({
        "points": [{ "x": 100.0, "y": 80.0 }, { "x": 220.0, "y": 220.0 }, { "x": 360.0, "y": 90.0 }],
        "closed": false,
        "curves": []
    });
    draw(app, "draw_polyline", with(app, "draw_polyline", &path));
    select_all(app);
}

/// Two open lines whose ends nearly meet, both selected.
fn shapes_lines2(app: &mut CutCraftApp) {
    shapes_lines(app);
    let last = app.doc.vectors.keys().last().copied();
    app.doc.selection.retain(|id| Some(*id) != last);
}

/// Nothing drawn: the Model tool needs nothing selected.
fn model_none(_: &mut CutCraftApp, _: &egui::Context) {}

/// A circle, selected.
fn model_circle(app: &mut CutCraftApp, _: &egui::Context) {
    shapes_circle(app);
}

/// A rectangle (the rail) and an open quarter-round (the cross-section), selected in that order.
fn model_rails(app: &mut CutCraftApp, _: &egui::Context) {
    moulding_shapes(app);
    select_all(app);
}

/// An open profile line, selected: the half outline of a spindle.
fn model_profile(app: &mut CutCraftApp, _: &egui::Context) {
    let points: Vec<_> = (0..=12)
        .map(|i| {
            let t = f64::from(i) / 12.0;
            let x = 120.0 + 160.0 * t;
            let y = 150.0 + 25.0 + 30.0 * (t * std::f64::consts::PI * 2.0).sin().abs();
            serde_json::json!({ "x": x, "y": y })
        })
        .collect();
    let profile = serde_json::json!({ "points": points, "closed": false, "curves": [] });
    draw(app, "draw_polyline", with(app, "draw_polyline", &profile));
    select_all(app);
}

/// A dome-shaped 3D part, picked in the Components list.
fn model_dome(app: &mut CutCraftApp, ctx: &egui::Context) {
    dome(app, ctx);
    model_close_up(app);
    app.doc.selected_components = app.doc.components.keys().copied().collect();
}

/// The dome, picked, and a smaller circle over it, selected.
fn model_dome_circle(app: &mut CutCraftApp, ctx: &egui::Context) {
    model_dome(app, ctx);
    let inner =
        serde_json::json!({ "center": { "x": CIRCLE_X, "y": CIRCLE_Y }, "radius": CIRCLE_R * 0.5 });
    draw(app, "draw_circle", inner);
    app.form = None;
    let last = app.doc.vectors.keys().last().copied();
    app.doc.selection = last.into_iter().collect();
}

/// A rectangle, selected.
fn model_rect(app: &mut CutCraftApp, _: &egui::Context) {
    shapes_rect(app);
}

/// The made-up picture, selected.
fn model_picture(app: &mut CutCraftApp, _: &egui::Context) {
    picture(app);
}

/// Two dome-shaped 3D parts side by side, both picked in the Components list.
fn model_dome2(app: &mut CutCraftApp, ctx: &egui::Context) {
    dome(app, ctx);
    let small = serde_json::json!({ "center": { "x": 330.0, "y": 220.0 }, "radius": 40.0 });
    draw(app, "draw_circle", small);
    app.form = None;
    let last = app.doc.vectors.keys().last().copied();
    app.doc.selection = last.into_iter().collect();
    app.invoke(ctx, "create_shape_round");
    set(app, "total_height", 8.0.into());
    if let Some(open) = &app.form {
        let (id, params) = (open.id, open.params.clone());
        let _ = app.apply(id, &params);
    }
    app.form = None;
    app.doc.selection.clear();
    app.doc.selected_components = app.doc.components.keys().copied().collect();
    model_close_up(app);
}

/// The open tool's settings, changed by `settings`, applied; the result shown close up in the
/// 3D view.
fn model_result(app: &mut CutCraftApp, settings: &[(&str, f64)]) {
    for (key, value) in settings {
        set(app, key, (*value).into());
    }
    if let Some(open) = &app.form {
        let (id, params) = (open.id, open.params.clone());
        let _ = app.apply(id, &params);
    }
    app.form = None;
    app.doc.selection.clear();
    model_close_up(app);
}

/// The 3D view, closer in on the model.
fn model_close_up(app: &mut CutCraftApp) {
    app.view = View::ThreeD;
    app.framing = crate::view3d::Framing::default().zoomed_about(egui::vec2(-70.0, -10.0), 2.2);
}

/// Two gently curving rails, then an open half-round cross-section, selected in that order.
fn model_two_rails(app: &mut CutCraftApp, _: &egui::Context) {
    for y in [110.0, 190.0] {
        let points: Vec<_> = (0..=10)
            .map(|i| {
                let t = f64::from(i) / 10.0;
                serde_json::json!({ "x": 60.0 + 240.0 * t, "y": y + 25.0 * (t * std::f64::consts::PI).sin() })
            })
            .collect();
        let rail = serde_json::json!({ "points": points, "closed": false, "curves": [] });
        draw(app, "draw_polyline", with(app, "draw_polyline", &rail));
    }
    let points: Vec<_> = (0..=8)
        .map(|i| {
            let a = f64::from(i) / 8.0 * std::f64::consts::PI;
            serde_json::json!({ "x": 350.0 - 25.0 * a.cos(), "y": 150.0 + 20.0 * a.sin() })
        })
        .collect();
    let section = serde_json::json!({ "points": points, "closed": false, "curves": [] });
    draw(app, "draw_polyline", with(app, "draw_polyline", &section));
    select_all(app);
}

/// A small dome (the piece to tile), picked in the Components list, and a rectangle (the
/// area to fill), selected.
fn model_texture(app: &mut CutCraftApp, ctx: &egui::Context) {
    let piece = serde_json::json!({ "center": { "x": 40.0, "y": 40.0 }, "radius": 12.0 });
    draw(app, "draw_circle", piece);
    app.form = None;
    let last = app.doc.vectors.keys().last().copied();
    app.doc.selection = last.into_iter().collect();
    app.invoke(ctx, "create_shape_round");
    set(app, "total_height", 5.0.into());
    if let Some(open) = &app.form {
        let (id, params) = (open.id, open.params.clone());
        let _ = app.apply(id, &params);
    }
    app.form = None;
    app.doc.selected_components = app.doc.components.keys().copied().collect();
    rect(app);
    let last = app.doc.vectors.keys().last().copied();
    app.doc.selection = last.into_iter().collect();
}

/// The Design page's lists, by their place along its top.
const SHEETS: usize = 1;
const LAYERS: usize = 2;
const COMPONENTS: usize = 3;

/// The Design page showing list `tab` ([`SHEETS`], [`LAYERS`] or [`COMPONENTS`]), no form
/// over it.
fn on_list(app: &mut CutCraftApp, tab: usize) {
    app.form = None;
    app.pages.set_pinned(Page::Design, true);
    app.design_tab = tab;
}

/// Runs command `id` with `params`, as a right-click menu's item does, no form left open.
#[expect(
    clippy::needless_pass_by_value,
    reason = "the tours write the settings in place, as json!(...)"
)]
fn run(app: &mut CutCraftApp, id: &str, params: serde_json::Value) {
    let params = with(app, id, &params);
    let _ = app.apply(id, &params);
    app.form = None;
}

/// Presses the open form's Apply and closes it.
fn apply_form(app: &mut CutCraftApp) {
    if let Some(open) = &app.form {
        let (id, params) = (open.id, open.params.clone());
        let _ = app.apply(id, &params);
    }
    app.form = None;
}

/// Opens command `id`'s form filled in with `params` over its own settings, as a right-click
/// menu's item does.
fn menu_form(app: &mut CutCraftApp, ctx: &egui::Context, id: &str, params: &serde_json::Value) {
    let params = with(app, id, params);
    app.invoke(ctx, &format!("form:{id}:{params}"));
}

/// Where node editing's right-click at `(x, y)` of the job is, on the vector there.
fn node_params(app: &CutCraftApp, x: f64, y: f64) -> serde_json::Value {
    let at = app.doc.job.area().min + Point::new(x, y);
    let vector = app.doc.pick(at, 2.0).map_or(0, |id| id.0);
    serde_json::json!({ "vector": vector, "at": { "x": at.x, "y": at.y }, "reach": 5.0 })
}

/// Runs node editing's command `id` on the node or span at `(x, y)` of the job.
fn on_node(app: &mut CutCraftApp, id: &str, x: f64, y: f64) {
    let params = node_params(app, x, y);
    let _ = app.apply(id, &params);
}

/// The rectangle on Layer 1, then Layer 2 (made current) with a circle on it; nothing
/// selected.
fn two_layers(app: &mut CutCraftApp) {
    rect(app);
    app.form = None;
    run(app, "add_layer", serde_json::json!({ "name": "Layer 2" }));
    let round = serde_json::json!({ "center": { "x": 440.0, "y": 150.0 }, "radius": 60.0 });
    draw(app, "draw_circle", round);
    app.form = None;
    app.doc.selection.clear();
}

/// A new job with two toolpaths, Engrave round a circle and Cut out round a rectangle, and
/// the Toolpaths page showing.
fn two_toolpaths(app: &mut CutCraftApp, ctx: &egui::Context) {
    fresh_job(app, ctx);
    small_profile(app, ctx);
    let outline = serde_json::json!({ "x": 200.0, "y": 60.0, "width": 200.0, "height": 150.0 });
    let outline = with(app, "draw_rectangle", &outline);
    draw(app, "draw_rectangle", outline);
    app.form = None;
    select(app, 200.0, 100.0);
    app.invoke(ctx, "profile_toolpath");
    calculate(app, ctx);
    app.form = None;
    app.doc.selection.clear();
    name_toolpaths(app, &["Engrave", "Cut out"]);
    app.pages.set_pinned(Page::Toolpaths, true);
}

/// Every toolpath, in the list's order.
fn toolpath_ids(app: &CutCraftApp) -> Vec<cut_craft_core::document::EntityId> {
    app.doc.toolpaths.keys().copied().collect()
}

/// The toolpaths named `names`, in the list's order.
fn name_toolpaths(app: &mut CutCraftApp, names: &[&str]) {
    for (toolpath, name) in app.doc.toolpaths.values_mut().zip(names) {
        toolpath.name = (*name).into();
    }
}

/// Toolpath number `n` (from 0) clicked in the list, and only it.
fn pick_toolpath(app: &mut CutCraftApp, n: usize) {
    app.selected_toolpath = toolpath_ids(app).get(n).copied();
    app.more_toolpaths.clear();
}

/// The toolpaths' cut order turned round.
fn swap_toolpaths(app: &mut CutCraftApp) {
    let mut order = toolpath_ids(app);
    order.reverse();
    run(
        app,
        "arrange_toolpaths",
        serde_json::json!({ "order": order, "groups": {} }),
    );
}

/// The first component's group.
fn component_group(app: &CutCraftApp) -> u64 {
    app.doc
        .components
        .values()
        .find_map(|k| k.group.as_ref().map(|g| g.id))
        .unwrap_or(0)
}

/// A new double-sided job.
fn double_sided(app: &mut CutCraftApp, ctx: &egui::Context) {
    fresh_job(app, ctx);
    let job = cut_craft_core::document::Job {
        double_sided: Some(cut_craft_core::document::DoubleSided::default()),
        ..app.doc.job
    };
    let mut change = cut_craft_core::Change::builder(&app.doc);
    change.set_job(job);
    app.record(Ok(change.finish()), "Double-sided job");
}

/// A new rotary job 60 mm across and 160 mm long along X, unrolled in the 2D view.
fn rotary_job(app: &mut CutCraftApp, ctx: &egui::Context) {
    use cut_craft_core::document::{Along, Job};
    fresh_job(app, ctx);
    let job = Job {
        clearance: 5.0,
        ..Job::rotary(160.0, 60.0, Along::X)
    };
    let mut change = cut_craft_core::Change::builder(&app.doc);
    change.set_job(job);
    app.record(Ok(change.finish()), "Rotary job");
}

/// The open arc's middle and radius.
const ARC_X: f64 = 200.0;
const ARC_Y: f64 = 90.0;
const ARC_R: f64 = 120.0;

/// An open arc, bulging up, selected.
fn open_arc(app: &mut CutCraftApp) {
    let arch = serde_json::json!({
        "center": { "x": ARC_X, "y": ARC_Y }, "radius": ARC_R,
        "start_angle": 20.0, "end_angle": 160.0
    });
    let arch = with(app, "draw_arc", &arch);
    draw(app, "draw_arc", arch);
    select_all(app);
}

/// A line straight up through the circle's middle, selected alone.
fn centre_line(app: &mut CutCraftApp) {
    let line = serde_json::json!({
        "points": [{ "x": CIRCLE_X, "y": 40.0 }, { "x": CIRCLE_X, "y": 260.0 }],
        "closed": false,
        "curves": []
    });
    draw(app, "draw_polyline", with(app, "draw_polyline", &line));
    app.form = None;
    let last = app.doc.vectors.keys().last().copied();
    app.doc.selection = last.into_iter().collect();
}

/// Three circles inside the rectangle, two of one size and a smaller one.
fn holes_inside(app: &mut CutCraftApp) {
    for (x, y, r) in [
        (152.0, 140.0, 20.0),
        (240.0, 140.0, 20.0),
        (196.0, 185.0, 10.0),
    ] {
        let hole = serde_json::json!({ "center": { "x": x, "y": y }, "radius": r });
        draw(app, "draw_circle", hole);
    }
    app.form = None;
}

/// Three circles one above another, unevenly spaced, all selected.
fn stacked_three(app: &mut CutCraftApp) {
    for (x, y, r) in [
        (200.0, 60.0, 25.0),
        (215.0, 115.0, 20.0),
        (190.0, 240.0, 30.0),
    ] {
        let c = serde_json::json!({ "center": { "x": x, "y": y }, "radius": r });
        draw(app, "draw_circle", c);
    }
    select_all(app);
}

/// A small G-code file cutting a square, written for Import Toolpaths to read.
fn gcode_file() -> String {
    let dir = std::env::temp_dir().join("cut-craft-help");
    let path = dir.join("sign.nc");
    let gcode = "G21 G90\nT1 M6\nS18000 M3\nG0 Z5\nG0 X20 Y20\nG1 Z-3 F300\n\
                 G1 X120 F1200\nG1 Y120\nG1 X20\nG1 Y20\nG0 Z5\nM5\nM30\n";
    let _ = std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(&path, gcode));
    path.display().to_string()
}

/// A small STL of two pyramids base to base (undercut all round), for Segment 3D Model.
fn stl_file() -> String {
    let dir = std::env::temp_dir().join("cut-craft-help");
    let path = dir.join("gem.stl");
    let corners = [(40.0, 0.0), (0.0, 40.0), (-40.0, 0.0), (0.0, -40.0)];
    let mut stl = String::from("solid gem\n");
    for i in 0..4 {
        let (a, b) = (corners[i], corners[(i + 1) % 4]);
        for tip in [30.0, -30.0] {
            let (p, q) = if tip > 0.0 { (a, b) } else { (b, a) };
            let _ = write!(
                stl,
                "facet normal 0 0 0\nouter loop\nvertex {} {} 0\nvertex {} {} 0\nvertex 0 0 {tip}\n\
                 endloop\nendfacet\n",
                p.0, p.1, q.0, q.1
            );
        }
    }
    stl.push_str("endsolid gem\n");
    let _ = std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(&path, stl));
    path.display().to_string()
}
