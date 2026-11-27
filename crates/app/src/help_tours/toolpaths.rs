//! The toolpath tours, with the material, tool and keep-out settings they use.

use super::{
    CIRCLE_R, CIRCLE_X, CIRCLE_Y, Mark, Page, RECT_H, RECT_Y, RECT2_X, Step, Tour, View, calculate,
    circle, dome, draw, egui, fitted, fresh_job, holes, lines, moulding_shapes, picture,
    preview_3d, rect, select, set, small_profile, star, turned_bead, two_shapes, with,
};
use cut_craft_core::Needs;

pub(super) const TOURS: &[Tour] = &[
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
                caption: "Check the bit, first in the form, drawn beside its name. **Select…** \
                          picks another one from your Bit Library; its feeds, speeds and depth \
                          per pass come with it.",
                setup: |app, ctx| app.invoke(ctx, "pocket_toolpath"),
                mark: Mark::At("field:tool"),
            },
            Step {
                caption: "Set the **Cut Depth** (D): how far below the top of the material the \
                          floor of the pocket ends up. Here it is 10 mm.",
                setup: |app, _| {
                    if let Some(open) = app.form.as_mut() {
                        open.params["depth"] = 10.0.into();
                    }
                },
                mark: Mark::At("field:depth"),
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
                caption: "The Bit Library keeps your bits, each with its size, shape, feeds \
                          and speeds. On the Toolpaths page, press **Bit Library** (under Job \
                          and Tools).",
                setup: |app, ctx| {
                    fresh_job(app, ctx);
                    app.pages.set_pinned(Page::Toolpaths, true);
                },
                mark: Mark::At("tool:tool_database"),
            },
            Step {
                caption: "**My Bits** are the bits you own; the makers' catalogues below are \
                          for copying from. Click a bit to see its drawing and settings, or add \
                          your own. Every toolpath's **Select…** picks from My Bits, and \
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
                caption: "On the Toolpaths page, press **Keep-Out Zones** (under Job and Bits).",
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
                          Toolpaths). It shows once **4th axis** is ticked in Machine \
                          Configuration.",
                setup: |app, _| {
                    fitted(app, Needs::Rotary);
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
                caption: "On the Toolpaths page, press **Rotary Lathe** (under 3D Toolpaths). \
                          It shows once **4th axis** is ticked in Machine Configuration.",
                setup: |app, _| {
                    fitted(app, Needs::Rotary);
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
                caption: "On the Toolpaths page, press **Laser Cut / Fill** (under Laser). It \
                          shows once **Laser fitted** is ticked in Machine Configuration.",
                setup: |app, _| {
                    fitted(app, Needs::Laser);
                    app.pages.set_pinned(Page::Toolpaths, true);
                },
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
                caption: "On the Toolpaths page, press **Laser Picture** (under Laser). It shows \
                          once **Laser fitted** is ticked in Machine Configuration.",
                setup: |app, _| {
                    fitted(app, Needs::Laser);
                    app.pages.set_pinned(Page::Toolpaths, true);
                },
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
];
