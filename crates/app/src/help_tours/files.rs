//! Templates, job sheets, files, importing and the view.

use super::{
    Mark, Page, Step, Tour, fresh_job, on_design_tab, set, shapes_circle, shapes_none,
    small_profile, two_shapes,
};

pub(super) const TOURS: &[Tour] = &[
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
                    app.toolpath_pick
                        .set(app.doc.toolpaths.keys().next().copied());
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
                        "../../../formats/import-dxf/tests/fixtures/nesting-sheet-layers.dxf"
                    );
                    let sheet = fmt_import_dxf::ImportDxf::import(&app.doc, SHEET);
                    app.record(sheet, "Imported");
                    app.design_tab = super::super::DESIGN_TABS
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
                caption: "Pick how the window is arranged: Ribbon, Rail or Classic.",
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
                caption: "Click the layout you want, then press **Use This Layout**.",
                setup: |app, ctx| {
                    app.invoke(ctx, "interface_layout");
                },
                mark: Mark::None,
            },
        ],
    },
];
