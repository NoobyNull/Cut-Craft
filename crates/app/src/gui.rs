//! Static UI data: Drawing tab tool groups, the main menu, right-click menus, status bar text.

use cmd_space_horizontally::Order as SpaceH;
use cmd_space_vertically::Order as SpaceV;
use cut_craft_core::Command;
use cut_craft_core::component::{Combine, Component, Group, Level, Mirror};
use cut_craft_core::document::{Document, EntityId, Layer};
use cut_craft_core::params::Items;
use cut_craft_core::toolpath::Toolpath;

/// Drawing tab layout: groups of command ids. Implemented commands supply their own label and
/// icon; the rest show their humanized id, disabled.
pub struct Toolbar {
    pub groups: Vec<ToolGroup>,
    /// The Toolpaths panel, laid out the same way.
    pub toolpath_groups: Vec<ToolGroup>,
    /// The Modeling tab.
    pub modeling_groups: Vec<ToolGroup>,
}

#[derive(Clone, Copy)]
pub struct ToolGroup {
    pub name: &'static str,
    pub tools: &'static [&'static str],
}

impl Toolbar {
    pub fn new() -> Self {
        Self {
            groups: drawing_groups(),
            toolpath_groups: toolpath_groups(),
            modeling_groups: modeling_groups(),
        }
    }
}

fn group(name: &'static str, tools: &'static [&'static str]) -> ToolGroup {
    ToolGroup { name, tools }
}

/// A sub-tab of the Design page: a few of its tool groups, one job at a time, so the page
/// shows a couple of dozen buttons instead of every icon at once. Aspire shows them all down
/// one long page; a beginner finds a tool faster under a word that says what it is for
/// (`re/SPEC.md`, Design sub-tabs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubTab {
    pub name: &'static str,
    /// What its tools are for, in a beginner's words: the sub-tab's tooltip.
    pub hint: &'static str,
    /// The groups it shows, by name, in order.
    pub groups: &'static [&'static str],
}

/// The groups the Design page shows over the sub-tabs whichever is open: the job's files and
/// the view, wanted at any stage.
pub const ALWAYS_SHOWN: &[&str] = &["File Operations", "2D View Control"];

/// The Design page's sub-tabs, in order; the first is open when the app starts.
pub const DESIGN_SUBTABS: &[SubTab] = &[
    SubTab {
        name: "Create",
        hint: "Draw shapes, lines, curves and text",
        groups: &["Create Vectors"],
    },
    SubTab {
        name: "Edit",
        hint: "Select, reshape, join, trim, move, rotate and resize what you drew",
        groups: &[
            "Select and Edit",
            "Join, Combine and Trim",
            "Transform Objects",
        ],
    },
    SubTab {
        name: "Arrange",
        hint: "Line things up, space them out, offset them, nest them and make copies",
        groups: &["Align Objects", "Offset and Layout", "Array Objects"],
    },
    SubTab {
        name: "Model",
        hint: "Build and shape the 3D parts to carve",
        groups: &["Shape Creation", "Shape Editing"],
    },
    SubTab {
        name: "Import",
        hint: "Bring in drawings, pictures, 3D models, clipart, toolpaths and cut lists",
        groups: &["Import"],
    },
];

/// The Design sub-tabs in the tab row and those shown as sections of their own under it, by
/// index in [`DESIGN_SUBTABS`], each in that order. `sections` names the sub-tabs moved out
/// of the row (Options' `design_sections`); names that are no sub-tab are ignored.
pub fn subtab_layout(sections: &[String]) -> (Vec<usize>, Vec<usize>) {
    (0..DESIGN_SUBTABS.len()).partition(|&i| !sections.iter().any(|s| s == DESIGN_SUBTABS[i].name))
}

/// The sub-tab the row shows open: `open` while it is in the row, else the row's first; none
/// when every sub-tab is a section.
pub fn open_subtab(open: usize, sections: &[String]) -> Option<usize> {
    let (row, _) = subtab_layout(sections);
    row.contains(&open).then_some(open).or(row.first().copied())
}

/// Moving a Design sub-tab between the tab row and a section of its own: then the sub-tab's
/// name and `section` or `row`, or `*` and either for every sub-tab.
pub const SUBTAB_PLACE: &str = "subtab_place:";

/// A Design sub-tab's right-click menu, on its tab in the row or its section's heading.
pub fn subtab_menu(name: &str, sections: &[String]) -> Vec<MenuItem> {
    let in_section = sections.iter().any(|s| s == name);
    let (row, apart) = subtab_layout(sections);
    let mut menu = vec![if in_section {
        MenuItem::with_action(
            "Put Back in the Tab Row",
            &format!("{SUBTAB_PLACE}{name}:row"),
        )
    } else {
        MenuItem::with_action(
            "Show as Its Own Section",
            &format!("{SUBTAB_PLACE}{name}:section"),
        )
    }];
    menu.push(MenuItem::separator());
    menu.push(MenuItem {
        is_checked: apart.is_empty(),
        ..MenuItem::with_action("All Tabs in One Row", &format!("{SUBTAB_PLACE}*:row"))
    });
    menu.push(MenuItem {
        is_checked: row.is_empty(),
        ..MenuItem::with_action(
            "Every Tab as Its Own Section",
            &format!("{SUBTAB_PLACE}*:section"),
        )
    });
    menu
}

/// `sections` after a [`SUBTAB_PLACE`] action, kept in [`DESIGN_SUBTABS`] order; `None` for
/// an action that is not one.
pub fn placed(sections: &[String], action: &str) -> Option<Vec<String>> {
    let (name, place) = action.strip_prefix(SUBTAB_PLACE)?.rsplit_once(':')?;
    let apart = match place {
        "section" => true,
        "row" => false,
        _ => return None,
    };
    if name != "*" && !DESIGN_SUBTABS.iter().any(|t| t.name == name) {
        return None;
    }
    Some(
        DESIGN_SUBTABS
            .iter()
            .map(|t| t.name)
            .filter(|&t| {
                if name == "*" || t == name {
                    apart
                } else {
                    sections.iter().any(|s| s == t)
                }
            })
            .map(str::to_owned)
            .collect(),
    )
}

/// Names and descriptions of the built-in tools on the Import sub-tab, which have no
/// command crate to give them: Aspire's names run words together ("Import Vectors Bitmap")
/// and say nothing about the files they take.
const BUILT_IN_ABOUTS: &[(&str, &str, &str)] = &[
    (
        "import_vectors_bitmap",
        "Import Vectors or Picture",
        "Bring in a drawing (DXF, SVG or a Vectric job's vectors) or a picture (PNG, JPG, \
         BMP, GIF or TIFF) to trace, carve round or draw over",
    ),
    (
        "import_vectors",
        "Import Vectors",
        "Bring in a drawing made in another program: DXF, SVG or a Vectric job's vectors",
    ),
    (
        "import_component_3d_model",
        "Import 3D Model",
        "Bring in a 3D model (STL or OBJ) or saved 3D clipart, to size, place and carve",
    ),
    (
        "clipart_library",
        "Clipart Library",
        "Open the Library tab: your folders of ready-made drawings, pictures and 3D clipart, \
         to drag onto the job",
    ),
    (
        "import_toolpaths_file",
        "Import Toolpaths (G-code)",
        "Read a G-code file (.nc, .tap, .gcode and the like) back in as toolpaths, to preview \
         and check it before it runs",
    ),
    (
        "import_cut_list",
        "Import Cut List",
        "Draw a part for each row of a spreadsheet's cut list (saved as CSV): name, length, \
         width and how many, ready for Nest Parts",
    ),
];

/// A built-in tool's own name, where [`title_case`] of its id would not do.
pub fn built_in_label(id: &str) -> Option<&'static str> {
    BUILT_IN_ABOUTS
        .iter()
        .find(|(i, _, _)| *i == id)
        .map(|(_, label, _)| *label)
}

impl Toolbar {
    /// Every group the Design page can show: the drawing then the modeling tools.
    pub fn all_design_groups(&self) -> Vec<ToolGroup> {
        self.groups
            .iter()
            .chain(&self.modeling_groups)
            .copied()
            .collect()
    }

    /// Sub-tab `tab`'s own groups; none for a tab out of range.
    pub fn subtab_groups(&self, tab: usize) -> Vec<ToolGroup> {
        self.named(DESIGN_SUBTABS.get(tab).map_or(&[][..], |t| t.groups))
    }

    /// The groups called `names`, in that order.
    pub fn named(&self, names: &[&str]) -> Vec<ToolGroup> {
        let all = self.all_design_groups();
        names
            .iter()
            .filter_map(|name| all.iter().find(|g| g.name == *name).copied())
            .collect()
    }

    /// The sub-tab `id` is on, if any: where to look for a tool picked some other way. A
    /// tool on two sub-tabs (Distort Object is under Transform Objects and Shape Editing)
    /// stays on `open` when it is there, else is found on the first.
    pub fn subtab_of(&self, id: &str, open: usize) -> Option<usize> {
        let all = self.all_design_groups();
        let on = |t: &SubTab| {
            t.groups
                .iter()
                .any(|name| all.iter().any(|g| g.name == *name && g.tools.contains(&id)))
        };
        if DESIGN_SUBTABS.get(open).is_some_and(on) {
            return Some(open);
        }
        DESIGN_SUBTABS.iter().position(on)
    }
}

/// Aspire's Drawing tab. Group names and which tool sits in which group follow the help
/// topics and the trial (see `re/SPEC.md`). Ids match
/// the icon file names, so unimplemented tools still show their icon.
#[expect(clippy::too_many_lines, reason = "the drawing tab's tool list")]
fn drawing_groups() -> Vec<ToolGroup> {
    vec![
        group(
            "File Operations",
            &[
                "new",
                "open",
                "save",
                "export_vectors",
                "paste_in_place",
                "open_folder",
                "job_setup",
            ],
        ),
        // every way to bring something into the job, on the Import sub-tab: Aspire spreads
        // them over the File menu, the Drawing and Modeling tabs and the Clipart tab
        group(
            "Import",
            &[
                "import_vectors_bitmap",
                "import_vectors",
                "import_component_3d_model",
                "clipart_library",
                "import_toolpaths_file",
                "import_cut_list",
            ],
        ),
        group(
            "2D View Control",
            &[
                "pan",
                "zoom_box",
                "zoom_to_job",
                "zoom_selected",
                "interface_layout",
            ],
        ),
        group(
            "Create Vectors",
            &[
                "draw_polyline",
                "draw_rectangle",
                "draw_rectangle_from_center",
                "draw_circle",
                "draw_circle_from_edge",
                "draw_ellipse",
                "draw_polygon",
                "draw_star",
                "draw_arc",
                "draw_curve",
                "draw_freehand",
                "draw_text",
                "edit_text",
                "convert_text_to_curves",
                "text_on_a_curve",
                "auto_layout_text",
                "dimensions",
                "create_fillets",
                "vector_boundary",
                "vector_texture",
                "trace_bitmap",
            ],
        ),
        // Aspire's Edit Objects section, in two: picking and reshaping, then the tools that
        // make one vector of several or cut them (each a job a beginner names differently)
        group(
            "Select and Edit",
            &[
                "select",
                "node_edit",
                "transform",
                "group",
                "measure_inspect",
                "fit_curves_to_vectors",
                "vector_validator",
                "edit_picture",
                "crop_bitmap",
            ],
        ),
        group(
            "Join, Combine and Trim",
            &[
                "join_vectors",
                "join_straight_line",
                "join_smooth_curve",
                "join_move_end_points",
                "close_extend_vector",
                "weld_vectors",
                "subtract_vectors",
                "overlap_intersect",
                "interactive_vector_trim",
                "trim_objects",
            ],
        ),
        // Aspire's Align Objects section (help: Alignment Tools): the common alignments
        group(
            "Align Objects",
            &[
                "center_in_material",
                "center_horizontally_in_material",
                "center_vertically_in_material",
                "center",
                "align_left",
                "align_right",
                "align_top",
                "align_bottom",
                "center_horizontally",
                "center_vertically",
                "left_to_right",
                "right_to_left",
                "top_to_bottom",
                "bottom_to_top",
            ],
        ),
        group(
            "Transform Objects",
            &["move", "rotate", "set_size", "mirror", "distort_object"],
        ),
        group(
            "Offset and Layout",
            &[
                "offset_vectors",
                "nest_parts",
                "name_part",
                "engrave_part_names",
                "plate_production",
            ],
        ),
        group(
            "Array Objects",
            &["array_copy", "circular_copy", "copy_along_vectors"],
        ),
    ]
}

/// The Modeling tab, laid out the same way.
fn modeling_groups() -> Vec<ToolGroup> {
    vec![
        group(
            "Shape Creation",
            &[
                "create_shape_round",
                "create_shape_angled",
                "create_shape_smooth",
                "create_shape_flat",
                "extrude_and_weave",
                "two_rail_sweep",
                "vector_unwrapper",
                "turn_and_spin",
                "texture_area",
                "decorative_relief",
                "add_zero_plane",
                "create_component_from_bitmap",
                "relief_from_photo",
            ],
        ),
        group(
            "Shape Editing",
            &[
                "sculpt",
                "smooth_selected_components",
                "emboss",
                "distort_object",
                "offset_model",
                "slice",
                "add_draft",
                "scale_model_z_height",
                "replace_below",
                "split",
                "bake_components",
                "component_levels",
            ],
        ),
    ]
}

/// The Toolpaths panel, laid out the same way.
fn toolpath_groups() -> Vec<ToolGroup> {
    vec![
        group("Material", &["material_setup", "materials"]),
        group(
            "Toolpath Operations",
            &[
                "profile_toolpath",
                "pocket_toolpath",
                "drilling_toolpath",
                "drill_bank_toolpath",
                "quick_engrave_toolpath",
                "vcarve_engrave_toolpath",
                "vcarve_inlay_toolpath",
                "inlay_toolpath",
                "chamfer_toolpath",
                "fluting_toolpath",
                "prism_toolpath",
                "thread_milling_toolpath",
                "keyhole_toolpath",
                "drag_knife_toolpath",
                "moulding_toolpath",
                "texture_toolpath",
                "photo_vcarve_toolpath",
                "sketch_carving_toolpath",
            ],
        ),
        group(
            "3D Toolpaths",
            &[
                "3d_roughing_toolpath",
                "3d_finishing_toolpath",
                "rounding_toolpath",
            ],
        ),
        group(
            "Laser",
            &["laser_cut_fill_toolpath", "laser_picture_toolpath"],
        ),
        group(
            "Toolpath Templates",
            &["array_copy_toolpath", "merged_toolpath"],
        ),
        group(
            "Output",
            &[
                "save_toolpaths",
                "toolpath_summary",
                "tool_database",
                "keep_out_zones",
                "toolpath_tiling",
            ],
        ),
    ]
}

/// `"join_vectors"` → `"Join Vectors"`: the label of a tool that is not implemented yet.
pub fn title_case(id: &str) -> String {
    id.split('_')
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map(|f| f.to_uppercase().chain(c).collect::<String>())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Status bar text
mod abouts {
    include!(concat!(env!("OUT_DIR"), "/abouts.rs"));
}

/// What command `id` does (its crate's first sentence), for its tooltip and the status bar.
pub fn about(id: &str) -> Option<&'static str> {
    abouts::ABOUTS
        .iter()
        .find(|(command, _)| *command == id)
        .map(|(_, about)| *about)
        .or_else(|| {
            BUILT_IN_ABOUTS
                .iter()
                .find(|(i, _, _)| *i == id)
                .map(|(_, _, about)| *about)
        })
}

pub struct StatusBar {
    pub message: String,
    pub current_tool: String,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            message: "Ready".to_string(),
            current_tool: "Select".to_string(),
        }
    }

    pub fn set_message(&mut self, message: &str) {
        self.message = message.to_string();
    }
}

/// `text` on one line for the status bar: its lines joined with " · ", blank ones dropped.
pub fn one_line(text: &str) -> String {
    let lines: Vec<&str> = (text.lines().map(str::trim))
        .filter(|l| !l.is_empty())
        .collect();
    lines.join(" · ")
}

/// Menu system for application controls
pub struct MenuSystem {
    pub menus: Vec<Menu>,
}

/// The node-editing popup for the node or span at `at` (Aspire's popup menus 142, 317, 635).
/// Edit Polyline's action: drawing on from an open vector's end node, `edit_polyline:` then
/// the vector's id and `start` or `end`.
pub const EDIT_POLYLINE: &str = "edit_polyline:";

pub fn node_menu(doc: &Document, at: &cut_craft_core::nodes::NodeAt) -> Vec<MenuItem> {
    // Node Position starts at the node itself, not at the click near it
    let node = at
        .path(doc)
        .ok()
        .and_then(|path| Some(cut_craft_core::nodes::nodes(path)[at.node(path).ok()?]))
        .map_or(at.at, |p| cut_craft_core::params::Xy::new(p.x, p.y));
    let mut items = vec![
        MenuItem::run::<cmd_insert_point::InsertPoint>("Insert Point", at),
        MenuItem::run::<cmd_delete_point::DeletePoint>("Delete Point", at),
        MenuItem::run::<cmd_smooth_point::SmoothPoint>("Smooth Point", at),
        MenuItem::form::<cmd_move_node::MoveNode>(
            "Node Position...",
            &cmd_move_node::Params {
                from: at.clone(),
                to: node,
            },
        ),
        MenuItem::separator(),
        MenuItem::run::<cmd_convert_span::SpanToLine>("Convert Span to Line", at),
        MenuItem::run::<cmd_convert_span::SpanToBezier>("Convert Span to Bezier", at),
        MenuItem::run::<cmd_convert_span::SpanToArc>("Convert Span to Arc", at),
        MenuItem::separator(),
        MenuItem::run::<cmd_cut_vector::CutVector>("Cut Vector", at),
        MenuItem::run::<cmd_make_start_point::MakeStartPoint>("Make Start Point", at),
        MenuItem::run::<cmd_reverse_direction::ReverseDirection>("Reverse Direction", at),
        MenuItem::run::<cmd_close_with_line::CloseWithLine>("Close With a Line", at),
    ];
    // an open vector's end node draws on from there
    let end = at.path(doc).ok().filter(|p| !p.closed).and_then(|path| {
        let last = cut_craft_core::nodes::nodes(path).len().checked_sub(1)?;
        match at.node(path).ok()? {
            0 => Some("start"),
            n if n == last => Some("end"),
            _ => None,
        }
    });
    if let Some(end) = end {
        items.push(MenuItem::with_action(
            "Edit Polyline",
            &format!("{EDIT_POLYLINE}{}:{end}", at.vector.0),
        ));
    }
    items.extend([
        MenuItem::separator(),
        MenuItem::with_action("Exit Node Editing", "node_edit"),
    ]);
    items
}

impl MenuSystem {
    /// Adds the Lua gadgets to the Gadgets menu, below the built-in ones: Cut Craft's own,
    /// then the user's in a My Gadgets submenu that says where to put them when it is empty.
    pub fn add_gadgets(
        &mut self,
        loaded: &crate::gadgets::Loaded,
        folder: Option<&std::path::Path>,
    ) {
        let item = |g: &crate::gadgets::Listed| MenuItem::with_action(&g.label, &g.id);
        let mut mine: Vec<MenuItem> = loaded.mine.iter().map(item).collect();
        if mine.is_empty() {
            let hint = folder.map_or_else(
                || "Put .lua gadget files in Cut Craft's settings folder".to_owned(),
                |f| format!("Put .lua gadget files in {}", f.display()),
            );
            mine.push(MenuItem {
                is_enabled: false,
                ..MenuItem::with_action(&hint, "")
            });
        }
        let Some(menu) = self.menus.iter_mut().find(|m| m.name == "Gadgets") else {
            return;
        };
        // Add-ons... stays last, after the gadgets
        let addons = menu.items.iter().position(|i| i.action == "addons");
        let tail: Vec<MenuItem> = addons
            .map(|at| menu.items.drain(at.saturating_sub(1)..).collect())
            .unwrap_or_default();
        menu.items.push(MenuItem::separator());
        menu.items.extend(loaded.built_in.iter().map(item));
        menu.items.push(MenuItem::submenu("My Gadgets", mine));
        menu.items.extend(tail);
    }

    fn menu(&self, name: &str) -> &Menu {
        self.menus
            .iter()
            .find(|m| m.name == name)
            .unwrap_or_else(|| panic!("there is a {name} menu"))
    }

    /// Edit ▸ `name`, reused as it is.
    fn edit(&self, name: &str) -> MenuItem {
        let items = &self.menu("Edit").items;
        let item = items.iter().find(|i| i.name == name);
        item.cloned()
            .unwrap_or_else(|| panic!("there is an Edit ▸ {name}"))
    }

    /// The 3D view's right-click menu: Aspire 12's 2D/3D view menu (help: Right Mouse Click
    /// Menu) headed by the components `under` the click, topmost first and the selected
    /// ticked (a click picks that one, so a buried component can be picked); its vector
    /// items greyed while no vector is selected, and Component ▸ the chosen component's own
    /// menu.
    pub fn view_3d_menu(&self, doc: &Document, under: &[EntityId]) -> Vec<MenuItem> {
        let edit = |name: &str| self.edit(name);
        let chosen = doc
            .selected_components
            .iter()
            .rev()
            .find(|id| doc.components.contains_key(id))
            .map(|&id| (id, &doc.components[&id]));
        let vectors = !doc.selection.is_empty();
        let text = doc.selection.iter().any(|id| {
            doc.vector_groups
                .get(id)
                .is_some_and(|g| doc.texts.contains_key(g))
        });
        let layers = doc.layers.iter().map(|l| {
            MenuItem::run::<cmd_move_to_layer::MoveToLayer>(
                &l.name,
                &cmd_move_to_layer::Params { layer: l.id },
            )
        });
        let mut menu: Vec<MenuItem> = under
            .iter()
            .filter_map(|id| {
                let name = &doc.components.get(id)?.name;
                let pick = format!("{SELECT_COMPONENT}{}", id.0);
                Some(
                    MenuItem::with_action(name, &pick)
                        .checked(doc.selected_components.contains(id)),
                )
            })
            .collect();
        if !menu.is_empty() {
            menu.push(MenuItem::separator());
        }
        let [duplicate, group, ungroup, component, move_to] =
            component_items(doc, chosen, layers.collect());
        menu.extend([
            edit("Cut"),
            edit("Copy"),
            edit("Paste"),
            duplicate,
            MenuItem::separator(),
            edit("Delete"),
            MenuItem::separator(),
            group,
            ungroup,
            MenuItem::separator(),
            MenuItem {
                name: "Select".into(),
                ..edit("Selection")
            },
            MenuItem::with_action("Activate Layer of Selection", "activate_layer_of_selection")
                .enabled(vectors),
            MenuItem::separator(),
            // ponytail: Aspire's Text ▸ also removes text from a curve and breaks a block
            // into lines; neither exists yet
            MenuItem::submenu(
                "Text",
                vec![MenuItem::with_action(
                    "Convert to Vectors",
                    "convert_text_to_curves",
                )],
            )
            .enabled(text),
            MenuItem::with_action("Convert To Vectors", "convert_text_to_curves").enabled(text),
            MenuItem::separator(),
            MenuItem::form::<cmd_join_close_vectors::JoinCloseVectors>(
                "Join / Close Vectors...",
                &cmd_join_close_vectors::Params::default(),
            )
            .enabled(vectors),
            MenuItem::with_action("Node Edit Mode", "node_edit").enabled(vectors),
            MenuItem::separator(),
            // Appearance fades a picture's greyscale in the 2D view: Bitmap Properties
            MenuItem::with_action("Appearance", "bitmap_properties")
                .enabled(!doc.selected_bitmaps.is_empty()),
            component,
            MenuItem::submenu(
                "Copy To",
                vec![
                    MenuItem::with_action("Other Side", "copy_to_other_side")
                        .enabled(doc.job.double_sided.is_some()),
                ],
            ),
            move_to,
        ]);
        menu
    }

    /// The 2D view's right-click menu: Aspire's selection menu (MENU 318), plus Move to Layer,
    /// the vector booleans and Align. Edit menu items are reused as they are.
    pub fn selection_menu(&self, doc: &Document) -> Vec<MenuItem> {
        let edit = |name: &str| self.edit(name);
        let layers = doc.layers.iter().map(|l| {
            MenuItem::run::<cmd_move_to_layer::MoveToLayer>(
                &l.name,
                &cmd_move_to_layer::Params { layer: l.id },
            )
        });
        vec![
            edit("Undo"),
            edit("Redo"),
            MenuItem::separator(),
            edit("Cut"),
            edit("Copy"),
            edit("Paste"),
            edit("Delete"),
            MenuItem::separator(),
            MenuItem::with_action("Select Vectors Inside", "select_vectors_inside"),
            MenuItem::submenu("Move to Layer", layers.collect()),
            MenuItem::with_action("Copy to Other Side", "copy_to_other_side")
                .enabled(doc.job.double_sided.is_some()),
            MenuItem::separator(),
            MenuItem::with_action("Weld", "weld_vectors"),
            MenuItem::with_action("Subtract", "subtract_vectors"),
            MenuItem::with_action("Overlap", "overlap_intersect"),
            edit("Join Vectors"),
            MenuItem::with_action("Group", "group"),
            MenuItem::submenu(
                "Ungroup",
                vec![
                    MenuItem::with_action("Ungroup onto Original Object Layers", "ungroup"),
                    MenuItem::with_action(
                        "Ungroup onto Current Layer",
                        "ungroup_onto_current_layer",
                    ),
                ],
            ),
            MenuItem {
                name: "Align".into(),
                ..edit("Align Selected Objects")
            },
            MenuItem::submenu("Text", text_items(doc)),
            MenuItem::separator(),
            MenuItem::with_action("Name Part...", "name_part"),
            MenuItem::with_action("Object Properties", "object_properties"),
            MenuItem::with_action("Bitmap Properties", "bitmap_properties")
                .enabled(!doc.selected_bitmaps.is_empty()),
            MenuItem::separator(),
            guides_submenu(),
        ]
    }

    /// Every command item, submenus flattened.
    pub fn items(&self) -> Vec<&MenuItem> {
        self.menus.iter().flat_map(|m| leaves(&m.items)).collect()
    }
}

/// The 2D view's menu over a keep-out zone, before the selection's: the zone's `label`,
/// where it is edited (the job's Keep-Out Zones, or Machine Configuration for a `machine`
/// clamp), and showing or hiding the labels.
pub fn keep_out_menu(label: &str, machine: bool, labels_shown: bool) -> Vec<MenuItem> {
    let edit = if machine {
        MenuItem::with_action("Edit Machine Keep-Out Zones…", "machine_configuration")
    } else {
        MenuItem::with_action("Edit Keep-Out Zones…", "keep_out_zones")
    };
    vec![
        MenuItem::with_action(label, "").enabled(false),
        edit,
        MenuItem::with_action(
            if labels_shown {
                "Hide Keep-Out Zone Labels"
            } else {
                "Show Keep-Out Zone Labels"
            },
            "show_keep_out_zone_labels",
        ),
        MenuItem::separator(),
    ]
}

/// The 2D view's menu on a guide: its properties, locking it, deleting it.
pub fn guide_menu(index: usize, locked: bool) -> Vec<MenuItem> {
    vec![
        MenuItem::with_action("Guide Properties…", &format!("{GUIDE}props:{index}")),
        MenuItem::with_action(
            if locked { "Unlock Guide" } else { "Lock Guide" },
            &format!("{GUIDE}lock:{index}"),
        ),
        MenuItem::with_action("Delete Guide", &format!("{GUIDE}delete:{index}")),
    ]
}

/// The 2D view's Guidelines submenu: a guide inserted where the menu opened.
pub fn guides_submenu() -> MenuItem {
    MenuItem::submenu(
        "Guidelines",
        vec![
            MenuItem::with_action("Insert Horizontal Guide", &format!("{GUIDE}insert:h")),
            MenuItem::with_action("Insert Vertical Guide", &format!("{GUIDE}insert:v")),
            MenuItem::with_action("Insert Angled Guide…", &format!("{GUIDE}insert:a")),
        ],
    )
}

/// The selection's Text submenu: editing the selected text, and breaking a block of lines
/// into a text per line.
fn text_items(doc: &Document) -> Vec<MenuItem> {
    let group = cmd_draw_text::selected_text_group(doc);
    let lines = group
        .and_then(|g| doc.texts.get(&g))
        .and_then(|t| t["text"].as_str())
        .map_or(0, |t| t.lines().count());
    vec![
        MenuItem::with_action("Edit Text", "edit_text"),
        MenuItem::run::<cmd_draw_text::BreakTextIntoLines>(
            "Break Block into Lines",
            &cmd_draw_text::BreakParams {
                group: group.unwrap_or(0),
            },
        )
        .enabled(lines > 1),
    ]
}

/// Every command item of `items`, submenus flattened.
pub fn leaves(items: &[MenuItem]) -> Vec<&MenuItem> {
    items
        .iter()
        .flat_map(|item| {
            if item.children.is_empty() {
                vec![item]
            } else {
                leaves(&item.children)
            }
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct Menu {
    pub name: String,
    pub items: Vec<MenuItem>,
}

/// Help ▸ Community's links: this, then the link's place in `LINKS`.
pub const COMMUNITY: &str = "community:";

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub name: String,
    pub action: String,
    pub shortcut: String,
    pub is_enabled: bool,
    pub is_checked: bool,
    /// Non-empty for a submenu
    pub children: Vec<MenuItem>,
    /// Its picture, an icon path, where Aspire's menu has one (the combine modes).
    pub icon: Option<String>,
}

impl MenuItem {
    /// Command item; the action id is derived from the name (`"Join Vectors"` -> `"join_vectors"`).
    fn new(name: &str, shortcut: &str) -> Self {
        let words: Vec<String> = name
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .map(String::from)
            .collect();
        Self {
            name: name.to_string(),
            action: words.join("_"),
            shortcut: shortcut.to_string(),
            is_enabled: true,
            is_checked: false,
            children: vec![],
            icon: None,
        }
    }

    /// With a picture.
    #[must_use]
    pub fn icon(self, path: &str) -> Self {
        Self {
            icon: Some(path.to_owned()),
            ..self
        }
    }

    /// An item whose action is not the slug of its name.
    pub fn with_action(name: &str, action: &str) -> Self {
        Self {
            action: action.to_string(),
            ..Self::new(name, "")
        }
    }

    /// Runs command `C` with `params`: `run:<id>:<json>`.
    pub fn run<C: Command>(name: &str, params: &C::Params) -> Self {
        Self::command::<C>(name, "run", params)
    }

    /// Opens command `C`'s form filled in with `params`: `form:<id>:<json>`.
    pub fn form<C: Command>(name: &str, params: &C::Params) -> Self {
        Self::command::<C>(name, "form", params)
    }

    fn command<C: Command>(name: &str, how: &str, params: &C::Params) -> Self {
        let params = serde_json::to_string(params).expect("command params serialize");
        Self::with_action(name, &format!("{how}:{}:{params}", C::ID))
    }

    pub fn submenu(name: &str, children: Vec<MenuItem>) -> Self {
        Self {
            children,
            ..Self::new(name, "")
        }
    }

    /// A separator is an item with an empty name.
    pub fn separator() -> Self {
        Self::new("", "")
    }

    pub fn is_separator(&self) -> bool {
        self.name.is_empty()
    }

    /// Greyed out unless `on`.
    #[must_use]
    pub fn enabled(self, on: bool) -> Self {
        Self {
            is_enabled: on,
            ..self
        }
    }

    /// Ticked when `on`.
    #[must_use]
    pub fn checked(self, on: bool) -> Self {
        Self {
            is_checked: on,
            ..self
        }
    }
}

/// Show ▸ and Hide ▸ for one layer, toolpath or component: Aspire's Show / Hide family.
fn show_hide(items: Items, item: u64) -> [MenuItem; 2] {
    use cmd_show_hide::{Action, Params, ShowHide};
    let run = |name: &str, action| {
        MenuItem::run::<ShowHide>(
            name,
            &Params {
                items,
                item,
                action,
            },
        )
    };
    [
        MenuItem::submenu(
            "Show",
            vec![
                run("This", Action::ShowThis),
                run("Only This", Action::ShowOnlyThis),
                run("All But This", Action::ShowAllButThis),
                run("All", Action::ShowAll),
            ],
        ),
        MenuItem::submenu(
            "Hide",
            vec![run("This", Action::HideThis), run("All", Action::HideAll)],
        ),
    ]
}

/// Delete ▸ for one layer, toolpath or component: Aspire's Delete family.
fn delete(items: Items, item: u64, which: &[(&str, cmd_delete_items::Which)]) -> MenuItem {
    use cmd_delete_items::{DeleteItems, Params};
    let run = |&(name, which): &(&str, _)| {
        MenuItem::run::<DeleteItems>(name, &Params { items, item, which })
    };
    MenuItem::submenu("Delete", which.iter().map(run).collect())
}

/// Aspire's toolpath menu (MENU 968) for toolpath `id`. Right-clicking a toolpath selects it,
/// so Recalculate ▸ This and Preview act on it.
pub fn toolpath_menu(doc: &Document, id: EntityId, toolpath: &Toolpath) -> Vec<MenuItem> {
    use cmd_delete_items::Which;
    let [show, hide] = show_hide(Items::Toolpaths, id.0);
    let properties = cmd_toolpath_properties::Params {
        toolpath: id,
        name: toolpath.name.clone(),
        visible: toolpath.visible,
        preview: cmd_toolpath_properties::PreviewFill::Unchanged,
        notes: None,
    };
    let no_fill = cmd_toolpath_properties::Params {
        preview: cmd_toolpath_properties::PreviewFill::NoFill,
        ..properties.clone()
    };
    vec![
        show,
        hide,
        MenuItem::form::<cmd_toolpath_properties::ToolpathProperties>("Rename", &properties),
        MenuItem::run::<cmd_toolpath_properties::ToolpathProperties>(
            "Preview Color: No Fill",
            &no_fill,
        ),
        MenuItem::run::<cmd_duplicate_toolpath::DuplicateToolpath>(
            "Duplicate",
            &cmd_duplicate_toolpath::Params { toolpath: id },
        ),
        MenuItem::submenu("Group", group_items(doc, id, toolpath)),
        MenuItem::run::<cmd_merged_toolpath::UngroupMergedToolpath>(
            "Ungroup",
            &cmd_merged_toolpath::UngroupParams { toolpath: id },
        )
        .enabled(toolpath.strategy == "merged_toolpath"),
        MenuItem::separator(),
        MenuItem::submenu(
            "Recalculate",
            vec![
                MenuItem::with_action("This", "recalculate_toolpath"),
                MenuItem::with_action("Visible", "recalculate_visible_toolpaths"),
                MenuItem::with_action("All", "recalculate_all_toolpaths"),
            ],
        ),
        MenuItem::with_action("Preview", "preview_toolpath"),
        MenuItem::submenu(
            "Copy to Sheet",
            other_sheets(doc, |sheet| {
                MenuItem::run::<cmd_copy_toolpath_to_sheet::CopyToolpathToSheet>(
                    &doc.sheets[sheet].name,
                    &cmd_copy_toolpath_to_sheet::Params {
                        toolpath: id,
                        sheet,
                    },
                )
            }),
        )
        .enabled(doc.sheets.len() > 1),
        delete(
            Items::Toolpaths,
            id.0,
            &[
                ("This", Which::This),
                ("Invisible", Which::Invisible),
                ("Visible", Which::Visible),
                ("All", Which::All),
            ],
        ),
    ]
}

/// A toolpath's Group submenu: this toolpath or the shown ones into a new group, a new empty
/// group, this toolpath into another group, or out of its group.
fn group_items(doc: &Document, id: EntityId, toolpath: &Toolpath) -> Vec<MenuItem> {
    use cmd_toolpath_group::{GroupParams, GroupToolpaths};
    let into = |name: &str, toolpaths: Vec<EntityId>, group: &str| {
        MenuItem::run::<GroupToolpaths>(
            name,
            &GroupParams {
                toolpaths,
                group: group.to_owned(),
            },
        )
    };
    let new = doc.next_toolpath_group_name();
    let shown = doc
        .toolpaths
        .iter()
        .filter(|(_, t)| t.visible)
        .map(|(&id, _)| id)
        .collect();
    let others: Vec<MenuItem> = doc
        .toolpath_groups_in_use()
        .iter()
        .filter(|g| toolpath.group.as_ref() != Some(g))
        .map(|g| into(g, vec![id], g))
        .collect();
    vec![
        into("Group This", vec![id], &new),
        into("Group Visible", shown, &new),
        into("Create Empty Group", vec![], &new),
        MenuItem::submenu("Move to Group", others.clone()).enabled(!others.is_empty()),
        into("Remove from Group", vec![id], "").enabled(toolpath.group.is_some()),
    ]
}

/// The toolpath list's menu on a group: rename it, take it apart, or delete it with its
/// toolpaths.
pub fn toolpath_group_menu(group: &str) -> Vec<MenuItem> {
    use cmd_toolpath_group::{RenameParams, RenameToolpathGroup, UngroupParams, UngroupToolpaths};
    let rename = RenameParams {
        group: group.to_owned(),
        name: group.to_owned(),
    };
    let ungroup = |delete_toolpaths| UngroupParams {
        group: group.to_owned(),
        delete_toolpaths,
    };
    vec![
        MenuItem::form::<RenameToolpathGroup>("Rename", &rename),
        MenuItem::run::<UngroupToolpaths>("Ungroup", &ungroup(false)),
        MenuItem::run::<UngroupToolpaths>("Delete Group and Its Toolpaths", &ungroup(true)),
    ]
}

/// One item for each sheet but the active one.
fn other_sheets(doc: &Document, item: impl Fn(usize) -> MenuItem) -> Vec<MenuItem> {
    (0..doc.sheets.len())
        .filter(|&i| i != doc.active_sheet)
        .map(item)
        .collect()
}

/// Aspire's level menu (MENU 1046) for level `id`: combine mode ticked, clipping, mirror mode
/// and (rotary jobs) wrapping.
/// A level's Combine Mode submenu, in Aspire's order.
/// A favourites action: then the page (`design` or `toolpaths`), what to do (`add`,
/// `remove`, `earlier`, `later`) and the tool's id.
pub const FAVOURITE: &str = "favourite:";
/// Showing a page's favourites or all its tools: then the page and `true` or `false`.
pub const FAVOURITES_ONLY: &str = "favourites_only:";

/// A page as favourites actions name it.
pub fn page_key(page: crate::pages::Page) -> &'static str {
    match page {
        crate::pages::Page::Design => "design",
        crate::pages::Page::Toolpaths => "toolpaths",
    }
}

/// The page a favourites action names.
pub fn page_of(key: &str) -> Option<crate::pages::Page> {
    match key {
        "design" => Some(crate::pages::Page::Design),
        "toolpaths" => Some(crate::pages::Page::Toolpaths),
        _ => None,
    }
}

/// A tool button's menu: add it to its page's favourites or take it off; shown among the
/// favourites, move it along them too.
pub fn tool_menu(
    page: crate::pages::Page,
    id: &str,
    favourite: bool,
    among: bool,
    folded: bool,
) -> Vec<MenuItem> {
    let action = |what: &str| format!("{FAVOURITE}{}:{what}:{id}", page_key(page));
    let mut menu = vec![if favourite {
        MenuItem::with_action("Remove from Favourites", &action("remove"))
    } else {
        MenuItem::with_action("Add to Favourites", &action("add"))
    }];
    if among {
        menu.push(MenuItem::with_action("Move Earlier", &action("earlier")));
        menu.push(MenuItem::with_action("Move Later", &action("later")));
    } else {
        menu.push(MenuItem::separator());
        menu.push(MenuItem {
            is_checked: folded,
            ..MenuItem::with_action(
                "Fold Tool Groups into One Button",
                &format!("{FOLD_TOOL_GROUPS}{}", !folded),
            )
        });
    }
    menu
}

/// Folding each group of tools that do one job different ways into one button, or spreading
/// them out: then `true` or `false` (Options ▸ General ▸ Fold tool groups into one button).
pub const FOLD_TOOL_GROUPS: &str = "fold_tool_groups:";

/// A tool page's tab menu: its favourites or all its tools, the one shown ticked.
pub fn favourites_menu(page: crate::pages::Page, only: bool) -> Vec<MenuItem> {
    [("Show Favourites", true), ("Show All Tools", false)]
        .into_iter()
        .map(|(name, on)| MenuItem {
            is_checked: on == only,
            ..MenuItem::with_action(name, &format!("{FAVOURITES_ONLY}{}:{on}", page_key(page)))
        })
        .collect()
}

/// The action that imports a model again from its file, followed by the component's id.
pub const REIMPORT_MODEL: &str = "reimport_model:";
/// `select_component:<id>`: the 3D view's menu lists the components under the click.
pub const SELECT_COMPONENT: &str = "select_component:";
/// A guide action: then `props:<index>`, `lock:<index>`, `delete:<index>`, or `insert:h`,
/// `insert:v` or `insert:a` (a horizontal, vertical or angled guide where the menu opened).
pub const GUIDE: &str = "guide:";
/// Save as Sculpting Brush on a component: then its id.
pub const SAVE_BRUSH: &str = "save_brush:";

/// The combine modes in Aspire's order, each with its icon.
pub const COMBINE_MODES: [(&str, Combine, &str); 5] = [
    ("Add", Combine::Add, "modeling/combine_add.png"),
    (
        "Subtract",
        Combine::Subtract,
        "modeling/combine_subtract.png",
    ),
    ("Merge", Combine::Merge, "modeling/combine_merge.png"),
    ("Low", Combine::Low, "modeling/combine_low.png"),
    (
        "Multiply",
        Combine::Multiply,
        "modeling/combine_multiply.png",
    ),
];

/// A level's Mirror Mode submenu, in Aspire's order.
const MIRROR_MODES: [(&str, Mirror); 9] = [
    ("Off", Mirror::Off),
    ("Left to Right", Mirror::LeftToRight),
    ("Right to Left", Mirror::RightToLeft),
    ("Top to Bottom", Mirror::TopToBottom),
    ("Bottom to Top", Mirror::BottomToTop),
    ("Top Left Quadrant", Mirror::TopLeft),
    ("Top Right Quadrant", Mirror::TopRight),
    ("Bottom Left Quadrant", Mirror::BottomLeft),
    ("Bottom Right Quadrant", Mirror::BottomRight),
];

pub fn level_menu(doc: &Document, id: u64, level: &Level) -> Vec<MenuItem> {
    use cmd_level_clipping::{Action, LevelClipping};
    use cmd_level_properties::{LevelProperties, Params};
    let properties = Params::of(id, level);
    let set = |name: &str, params: Params| MenuItem::run::<LevelProperties>(name, &params);
    let mode = |name: &str, combine| {
        set(
            name,
            Params {
                combine,
                ..properties.clone()
            },
        )
        .checked(level.combine == combine)
    };
    let mirror = |name: &str, mirror| {
        set(
            name,
            Params {
                mirror,
                ..properties.clone()
            },
        )
        .checked(level.mirror == mirror)
    };
    let clip = |name: &str, action| {
        MenuItem::run::<LevelClipping>(name, &cmd_level_clipping::Params { level: id, action })
    };
    let reorder = |name: &str, direction| {
        MenuItem::run::<cmd_reorder_level::ReorderLevel>(
            name,
            &cmd_reorder_level::Params {
                level: id,
                direction,
            },
        )
    };
    let clipped = !level.clip.is_empty();
    let [show, hide] = show_hide(Items::Levels, id);
    vec![
        MenuItem::submenu(
            "Combine Mode",
            COMBINE_MODES
                .iter()
                .map(|&(name, c, icon)| mode(name, c).icon(icon))
                .collect(),
        ),
        show,
        hide,
        MenuItem::with_action(
            "Select Components",
            &format!("{}{id}", crate::app::SELECT_LEVEL),
        ),
        MenuItem::submenu(
            "Clipping",
            vec![
                clip("Apply", Action::Apply).checked(clipped),
                clip("Update", Action::Update).enabled(clipped),
                clip("Remove", Action::Remove).enabled(clipped),
            ],
        ),
        MenuItem::submenu(
            "Mirror Mode",
            MIRROR_MODES
                .iter()
                .map(|&(name, m)| mirror(name, m))
                .collect(),
        ),
        set(
            "Wrapping",
            Params {
                wrapping: !level.wrapping,
                ..properties.clone()
            },
        )
        .checked(level.wrapping)
        .enabled(doc.job.rotary.is_some()),
        MenuItem::separator(),
        reorder("Move Level Up", cmd_reorder_level::Direction::Up)
            .enabled(doc.levels.range(id + 1..).next().is_some()),
        reorder("Move Level Down", cmd_reorder_level::Direction::Down)
            .enabled(doc.levels.range(..id).next().is_some()),
        MenuItem::form::<cmd_insert_level::InsertLevel>(
            "Insert New Level",
            &cmd_insert_level::InsertLevel::initial_params(doc),
        ),
        MenuItem::run::<cmd_delete_level::DeleteLevel>(
            "Delete Level",
            &cmd_delete_level::Params { level: id },
        )
        .enabled(doc.levels.len() > 1),
        MenuItem::form::<LevelProperties>("Rename Level", &properties),
        MenuItem::with_action(
            "Export as 3D Clipart",
            &format!("{}{id}", crate::app::EXPORT_LEVEL),
        )
        .enabled(!doc.level_components(id).is_empty()),
        MenuItem::separator(),
        MenuItem::form::<LevelProperties>("Properties", &properties),
    ]
}

/// A component group's menu in the tree: its combine mode ticked, Ungroup, Rename and
/// Properties.
pub fn group_menu(group: &Group) -> Vec<MenuItem> {
    use cmd_group_properties::{GroupProperties, Params};
    let properties = Params::of(group);
    let mode = |name: &str, combine| {
        let params = Params {
            combine,
            ..properties.clone()
        };
        MenuItem::run::<GroupProperties>(name, &params).checked(group.combine == combine)
    };
    vec![
        MenuItem::submenu(
            "Combine Mode",
            COMBINE_MODES
                .iter()
                .map(|&(name, c, icon)| mode(name, c).icon(icon))
                .collect(),
        ),
        MenuItem::run::<cmd_ungroup_components::UngroupComponents>(
            "Ungroup",
            &cmd_ungroup_components::Params { group: group.id },
        ),
        MenuItem::form::<GroupProperties>("Rename", &properties),
        MenuItem::separator(),
        MenuItem::form::<GroupProperties>("Properties", &properties),
    ]
}

/// Move to Level ▸ each level, for the selected components; their level ticked.
fn move_to_level(doc: &Document, component: &Component) -> MenuItem {
    MenuItem::submenu(
        "Move to Level",
        doc.levels
            .iter()
            .map(|(&id, l)| {
                MenuItem::run::<cmd_move_to_level::MoveToLevel>(
                    &l.name,
                    &cmd_move_to_level::Params { level: id },
                )
                .checked(component.level == id)
            })
            .collect(),
    )
}

/// The view menu's items that act on the chosen component (Duplicate, Group, Ungroup,
/// Component ▸, Move To ▸), or for vectors when there is none.
fn component_items(
    doc: &Document,
    chosen: Option<(EntityId, &Component)>,
    layers: Vec<MenuItem>,
) -> [MenuItem; 5] {
    let vectors = !doc.selection.is_empty();
    match chosen {
        Some((id, k)) => [
            MenuItem::run::<cmd_duplicate_component::DuplicateComponent>(
                "Duplicate",
                &cmd_duplicate_component::Params { component: id },
            ),
            MenuItem::form::<cmd_group_components::GroupComponents>(
                "Group",
                &cmd_group_components::Params::default(),
            )
            .enabled(doc.selected_components.len() > 1),
            MenuItem::run::<cmd_ungroup_components::UngroupComponents>(
                "Ungroup",
                &cmd_ungroup_components::Params {
                    group: k.group.as_ref().map_or(0, |g| g.id),
                },
            )
            .enabled(k.group.is_some()),
            {
                // shape to vector outline, with the component's own menu
                let mut items = component_menu(doc, id, k);
                items.push(MenuItem::separator());
                items.push(MenuItem::with_action(
                    "Create Vector Boundary",
                    "component_boundary",
                ));
                MenuItem::submenu("Component", items)
            },
            MenuItem {
                name: "Move To".into(),
                ..move_to_level(doc, k)
            },
        ],
        None => [
            MenuItem::with_action("Duplicate", "duplicate_component").enabled(false),
            MenuItem::with_action("Group", "group").enabled(vectors),
            MenuItem::with_action("Ungroup", "ungroup").enabled(vectors),
            MenuItem::submenu("Component", vec![]).enabled(false),
            MenuItem::submenu("Move To", layers).enabled(vectors),
        ],
    }
}

/// Aspire's component menu (MENU 613) for component `id`, its combine mode ticked.
pub fn component_menu(doc: &Document, id: EntityId, component: &Component) -> Vec<MenuItem> {
    use cmd_component_properties::{ComponentProperties, Params};
    let properties = Params::of(id, component);
    let mode = |name: &str, combine| {
        let params = Params {
            combine,
            ..properties.clone()
        };
        MenuItem::run::<ComponentProperties>(name, &params).checked(component.combine == combine)
    };
    let [show, hide] = show_hide(Items::Components, id.0);
    let mut menu = vec![
        MenuItem::submenu(
            "Combine Mode",
            COMBINE_MODES
                .iter()
                .map(|&(name, c, icon)| mode(name, c).icon(icon))
                .collect(),
        ),
        MenuItem::separator(),
        MenuItem::run::<cmd_duplicate_component::DuplicateComponent>(
            "Duplicate",
            &cmd_duplicate_component::Params { component: id },
        ),
        MenuItem::run::<cmd_delete_component::DeleteComponent>(
            "Delete",
            &cmd_delete_component::Params { component: id },
        ),
        MenuItem::with_action("Save as Sculpting Brush…", &format!("{SAVE_BRUSH}{}", id.0)),
        move_to_level(doc, component),
        MenuItem::form::<cmd_group_components::GroupComponents>(
            "Group",
            &cmd_group_components::Params::default(),
        )
        .enabled(doc.selected_components.len() > 1),
        MenuItem::run::<cmd_ungroup_components::UngroupComponents>(
            "Ungroup",
            &cmd_ungroup_components::Params {
                group: component.group.as_ref().map_or(0, |g| g.id),
            },
        )
        .enabled(component.group.is_some()),
        MenuItem::form::<ComponentProperties>("Rename", &properties),
        show,
        hide,
        MenuItem::separator(),
        MenuItem::form::<ComponentProperties>("Properties", &properties),
    ];
    // an imported model: again from its file, to orientate or size it afresh
    if component.source.is_some() {
        let again = format!("{REIMPORT_MODEL}{}", id.0);
        menu.splice(
            0..0,
            [
                MenuItem::with_action("Re-import Model…", &again),
                MenuItem::separator(),
            ],
        );
    }
    menu
}

/// Aspire's layer menu (MENU 967) for `layer`.
/// A menu's categories: the runs of items between separators, each drawn in a border.
pub fn categories(items: &[MenuItem]) -> Vec<&[MenuItem]> {
    items
        .split(MenuItem::is_separator)
        .filter(|run| !run.is_empty())
        .collect()
}

/// The right-click menu of sheet `sheet` in the Sheets tab (MENU 5030).
pub fn sheet_menu(doc: &Document, sheet: usize) -> Vec<MenuItem> {
    use cmd_sheet_properties::SheetProperties;
    let Some(s) = doc.sheets.get(sheet) else {
        return vec![];
    };
    let props = cmd_sheet_properties::Params {
        sheet,
        name: s.name.clone(),
        visible: s.visible,
    };
    let shown = |name: &str, visible: bool| {
        let params = cmd_sheet_properties::Params {
            visible,
            ..props.clone()
        };
        MenuItem::run::<SheetProperties>(name, &params).enabled(s.visible != visible)
    };
    let send = |name: &str, move_vectors: bool| {
        let params = cmd_send_to_sheet::Params {
            sheet,
            move_vectors,
        };
        MenuItem::run::<cmd_send_to_sheet::SendToSheet>(name, &params)
            .enabled(sheet != doc.active_sheet && !doc.selection.is_empty())
    };
    vec![
        MenuItem::run::<cmd_activate_sheet::ActivateSheet>(
            "Activate",
            &cmd_activate_sheet::Params { sheet },
        )
        .enabled(sheet != doc.active_sheet),
        shown("Show", true),
        shown("Hide", false),
        MenuItem::separator(),
        send("Copy Selected Vectors Here", false),
        send("Move Selected Vectors Here", true),
        MenuItem::separator(),
        MenuItem::form::<cmd_add_sheet::AddSheet>(
            "Insert New Sheet",
            &cmd_add_sheet::Params {
                like: Some(sheet),
                ..cmd_add_sheet::AddSheet::initial_params(doc)
            },
        ),
        // ponytail: a sheet is edited while active, in Job Setup; Aspire's Sheet Editor is that form
        MenuItem::with_action("Edit Sheet...", "job_setup").enabled(sheet == doc.active_sheet),
        MenuItem::form::<SheetProperties>("Rename", &props),
        MenuItem::run::<cmd_delete_sheet::DeleteSheet>(
            "Delete",
            &cmd_delete_sheet::Params { sheet },
        )
        .enabled(doc.sheets.len() > 1),
    ]
}

pub fn layer_menu(doc: &Document, layer: &Layer) -> Vec<MenuItem> {
    use cmd_delete_items::Which;
    use cmd_layer_properties::LayerProperties;
    let id = layer.id;
    let lock = |name: &str, locked: bool| {
        let params = cmd_layer_properties::Params {
            locked,
            ..layer.clone().into()
        };
        MenuItem::run::<LayerProperties>(name, &params).enabled(layer.locked != locked)
    };
    let [show, hide] = show_hide(Items::Layers, u64::from(id.0));
    vec![
        MenuItem::run::<cmd_set_current_layer::SetCurrentLayer>(
            "Activate",
            &cmd_set_current_layer::Params { layer: id },
        ),
        show,
        hide,
        lock("Lock", true),
        lock("Unlock", false),
        MenuItem::separator(),
        MenuItem::with_action("Insert New Layer", "add_layer"),
        delete(
            Items::Layers,
            u64::from(id.0),
            &[
                ("This", Which::This),
                ("Visible", Which::Visible),
                ("Invisible", Which::Invisible),
                ("Empty", Which::Empty),
            ],
        ),
        MenuItem::run::<cmd_delete_layer::DeleteLayer>(
            "Delete Layer",
            &cmd_delete_layer::Params { layer: id },
        )
        .enabled(doc.layers.len() > 1),
        MenuItem::form::<LayerProperties>("Rename", &layer.clone().into()),
        MenuItem::run::<cmd_merge_visible_layers::MergeVisibleLayers>(
            "Merge Visible",
            &cmd_merge_visible_layers::Params { layer: id },
        )
        .enabled(doc.layers.iter().any(|l| l.visible && l.id != id)),
        MenuItem::separator(),
        MenuItem::run::<cmd_select_layer_vectors::SelectLayerVectors>(
            "Select Layer Vectors",
            &cmd_select_layer_vectors::Params { layer: id },
        ),
        MenuItem::run::<cmd_move_to_layer::MoveToLayer>(
            "Move Selection Here",
            &cmd_move_to_layer::Params { layer: id },
        )
        .enabled(!doc.selection.is_empty()),
    ]
}

/// A `run:` or `form:` action taken apart: whether it opens the form, the command id, its params.
pub fn command_action(action: &str) -> Option<(bool, &str, serde_json::Value)> {
    let (form, rest) = match action.split_once(':')? {
        ("run", rest) => (false, rest),
        ("form", rest) => (true, rest),
        _ => return None,
    };
    let (id, params) = rest.split_once(':')?;
    Some((form, id, serde_json::from_str(params).ok()?))
}

impl MenuSystem {
    /// Aspire's main menu (see `re/SPEC.md`, Menus), minus vendor web links and licensing.
    /// Shortcuts follow the keyboard shortcut table there where the two disagree.
    #[expect(clippy::too_many_lines, reason = "the menu table is data, not logic")]
    pub fn new() -> Self {
        let i = |name: &str| MenuItem::new(name, "");
        let k = MenuItem::new;
        let sub = MenuItem::submenu;
        let sep = MenuItem::separator;
        let a = MenuItem::with_action;
        let space_h = |name: &str, order, inside_last_vector| {
            MenuItem::run::<cmd_space_horizontally::SpaceHorizontally>(
                name,
                &cmd_space_horizontally::Params {
                    order,
                    inside_last_vector,
                },
            )
        };
        let space_v = |name: &str, order, inside_last_vector| {
            MenuItem::run::<cmd_space_vertically::SpaceVertically>(
                name,
                &cmd_space_vertically::Params {
                    order,
                    inside_last_vector,
                },
            )
        };
        let menu = |name: &str, items: Vec<MenuItem>| Menu {
            name: name.to_string(),
            items,
        };

        Self {
            menus: vec![
                menu(
                    "File",
                    vec![
                        k("New", "Ctrl+N"),
                        i("New from Template..."),
                        k("Open...", "Ctrl+O"),
                        i("Close"),
                        sep(),
                        k("Save", "Ctrl+S"),
                        i("Save As..."),
                        i("Save As Template..."),
                        k("Increment and Save", "Ctrl+Alt+S"),
                        sep(),
                        i("Print..."),
                        i("Print Preview"),
                        sep(),
                        sub(
                            "Import",
                            vec![
                                k("Import Vectors / Bitmap...", "Ctrl+I"),
                                i("Import Component / 3D Model..."),
                                a("Segment 3D Model...", "segment_3d_model_file"),
                                a("Import Toolpaths...", "import_toolpaths_file"),
                                a("Import Cut List (CSV)...", "import_cut_list"),
                            ],
                        ),
                        sub(
                            "Export",
                            vec![
                                a("EPS", "export_eps"),
                                a("DXF", "export_dxf"),
                                a("AI", "export_ai"),
                                a("SVG", "export_svg"),
                                a("PDF", "export_pdf"),
                            ],
                        ),
                        sep(),
                        i("Open Application Data Folder"),
                        sep(),
                        i("Recent Files"),
                        sep(),
                        i("Exit"),
                    ],
                ),
                menu(
                    "Edit",
                    vec![
                        k("Undo", "Ctrl+Z"),
                        k("Redo", "Ctrl+Y"),
                        sep(),
                        k("Cut", "Ctrl+X"),
                        k("Copy", "Ctrl+C"),
                        k("Paste", "Ctrl+V"),
                        k("Paste in Place", "Ctrl+Shift+V"),
                        k("Delete", "Del"),
                        k("Edit Object", "E"),
                        sep(),
                        k("Find Tool...", "Ctrl+K"),
                        sep(),
                        sub(
                            "Selection",
                            vec![
                                k("Select All Vectors", "Ctrl+A"),
                                i("Select All Open Vectors"),
                                i("Select All Duplicate Vectors"),
                                i("Select All Vectors On Current Layer"),
                                i("Select Vectors Inside"),
                                i("Unselect All"),
                                i("Vector Selector..."),
                            ],
                        ),
                        sub(
                            "Align Selected Objects",
                            vec![
                                k("Center In Material", "F9"),
                                i("Center Horizontally In Material"),
                                i("Center Vertically In Material"),
                                sep(),
                                i("Center"),
                                a("Left", "align_left"),
                                a("Right", "align_right"),
                                a("Top", "align_top"),
                                a("Bottom", "align_bottom"),
                                i("Center Horizontally"),
                                i("Center Vertically"),
                                sep(),
                                space_h("Left To Right", SpaceH::LeftToRight, false),
                                space_h("Right To Left", SpaceH::RightToLeft, false),
                                space_v("Top To Bottom", SpaceV::TopToBottom, false),
                                space_v("Bottom To Top", SpaceV::BottomToTop, false),
                                space_h("Horizontally Inside Last Vector", SpaceH::AsPlaced, true),
                                space_v("Vertically Inside Last Vector", SpaceV::AsPlaced, true),
                            ],
                        ),
                        k("Join Vectors", "J"),
                        a("Curve Fit Vectors", "fit_curves_to_vectors"),
                        a("Nest Selected Objects", "nest_parts"),
                        a("Name Part...", "name_part"),
                        a("Engrave Part Names...", "engrave_part_names"),
                        sep(),
                        i("Job Size and Position"),
                        sub(
                            "Units",
                            vec![a("Millimetres", "units_mm"), a("Inches", "units_inch")],
                        ),
                        i("Swap Sides"),
                        sep(),
                        a("Notes", "notes"),
                        i("Document Variables"),
                        k("Snap Options", "F4"),
                        i("Options"),
                    ],
                ),
                menu(
                    "Model",
                    vec![
                        sub(
                            "Create Component",
                            vec![
                                i("Two Rail Sweep"),
                                i("Extrude and Weave"),
                                i("Turn and Spin"),
                                a("Create Textured Area Component", "texture_area"),
                                a("Create Decorative Relief", "decorative_relief"),
                                i("Add Zero Plane"),
                            ],
                        ),
                        sub(
                            "Create Component from",
                            vec![
                                i("Bake Components"),
                                i("Create Component from Visible Model"),
                                a(
                                    "Create Component with Draft from Visible Model",
                                    "add_draft",
                                ),
                                a("Create an Offset Model", "offset_model"),
                                a("Slice Model", "slice"),
                                i("Create Component from Toolpath Preview"),
                            ],
                        ),
                        sep(),
                        i("Create Component from Bitmap"),
                        i("Import Component / 3D Model..."),
                        i("Export as STL"),
                        i("Export as Grayscale Bitmap"),
                        i("Export as 3D Clipart"),
                        sep(),
                        // Job Setup's Resolution (Aspire's level of detail), here too
                        sub(
                            "Model Resolution",
                            vec![
                                MenuItem::with_action(
                                    "Standard (fastest)",
                                    "model_resolution_standard",
                                ),
                                MenuItem::with_action("High (3x slower)", "model_resolution_high"),
                                MenuItem::with_action(
                                    "Highest (7x slower)",
                                    "model_resolution_highest",
                                ),
                            ],
                        ),
                        sep(),
                        a("Sculpt Visible Model", "sculpt"),
                        i("Smooth Selected Components"),
                        i("Reset Inside Selected Vectors"),
                        i("Reset Outside Selected Vectors"),
                        a("Split Selected Component", "split"),
                        a(
                            "Create Vector Boundary Around Selected Components",
                            "component_boundary",
                        ),
                        a("Create Cross Section Vector", "cross_section"),
                        sep(),
                        a("Emboss Component", "emboss"),
                        i("Scale Model Z Height"),
                        a("Distort Selected Component", "distort_object"),
                        i("Replace Below"),
                        sep(),
                        MenuItem::with_action(
                            "Selected Component Properties",
                            "component_properties",
                        ),
                    ],
                ),
                menu(
                    "Machine",
                    vec![
                        i("Machine Configuration..."),
                        i("Tool Changer..."),
                        sep(),
                        i("Add Machine..."),
                        i("Install Offline Package..."),
                        i("Search for Machine Online..."),
                        sep(),
                        i("Post-Processor Management..."),
                        sep(),
                        i("Install Post-Processor..."),
                        i("Update Post-Processor Database"),
                    ],
                ),
                menu(
                    "Toolpaths",
                    vec![
                        i("Show Toolpaths Tab"),
                        i("Recalculate All Toolpaths"),
                        MenuItem::with_action("Keep-Out Zones...", "keep_out_zones"),
                        MenuItem::with_action("Toolpath Tiling...", "toolpath_tiling"),
                        sub(
                            "Toolpath Drawing",
                            vec![
                                i("Draw 2D Previews"),
                                i("Make 2D Previews Solid"),
                                i("Auto Open 3D View"),
                                i("Animate Preview"),
                                i("Draw Rapid Moves"),
                                i("Draw Plunge Moves"),
                                i("Draw Retract Moves"),
                                i("Draw Join Moves"),
                            ],
                        ),
                        sub(
                            "Preview Simulation Quality",
                            vec![
                                MenuItem::with_action("Standard", "preview_quality_1"),
                                MenuItem::with_action("High (2x Slower)", "preview_quality_2"),
                                MenuItem::with_action("Very High (4x Slower)", "preview_quality_4"),
                                MenuItem::with_action(
                                    "Extremely High (8x Slower)",
                                    "preview_quality_8",
                                ),
                                MenuItem::with_action("Maximum (25x Slower)", "preview_quality_25"),
                            ],
                        ),
                        sep(),
                        sub(
                            "Templates",
                            vec![
                                i("Save Selected Toolpath as Template..."),
                                i("Save All Visible Toolpaths as Template..."),
                                i("Load Template..."),
                            ],
                        ),
                        MenuItem::with_action("Merge Visible Toolpaths", "merged_toolpath"),
                        i("Create Job Sheet"),
                        a("Print Job Sheet...", "print_job_sheet"),
                        a("Print Part Labels...", "part_labels"),
                        a("Save Cut List...", "cut_list"),
                        sep(),
                        i("Materials..."),
                        sep(),
                        i("Tool Database..."),
                    ],
                ),
                menu(
                    "View",
                    vec![
                        k("Zoom to Drawing", "F6"),
                        k("Refresh 2D View", "F5"),
                        sub(
                            "Guide Lines",
                            vec![
                                i("Guides Visible"),
                                i("Delete All Guides"),
                                i("Lock All Existing Guides"),
                                i("Unlock All Existing Guides"),
                            ],
                        ),
                        sep(),
                        i("Draw Origin"),
                        i("Draw Modeling Plane"),
                        i("Draw Material Block"),
                        i("Draw Vectors in 3D View"),
                        i("Draw Keep-Out Zones in 3D View"),
                        i("Show Keep-Out Zone Labels"),
                        sep(),
                        i("Auto-Wrapping View"),
                        i("Color Shaded View"),
                        i("Use Shaded Background"),
                        i("Light Follows Viewer"),
                        i("Use Shadow Shading"),
                        sep(),
                        i("Save Shaded Image"),
                        sep(),
                        k("Tile Windows Horizontal", "PgDn"),
                        k("Tile Windows Vertical", "PgUp"),
                        i("Cascade Windows"),
                        sep(),
                        sub(
                            "Window Layout",
                            vec![
                                MenuItem::with_action("Classic (Aspire's pages)", "layout_classic"),
                                MenuItem::with_action("Rail and Inspector", "layout_rail"),
                                MenuItem::with_action("Ribbon", "layout_ribbon"),
                            ],
                        ),
                    ],
                ),
                menu(
                    "Gadgets",
                    vec![
                        MenuItem::with_action("Wrapped Fluting Layout", "wrapped_fluting_layout"),
                        MenuItem::with_action("Wrapped Spiral Layout", "wrapped_spiral_layout"),
                        MenuItem::with_action("Celtic Weave Creator", "celtic_weave"),
                        MenuItem::with_action("Job Setup Sheet Editor...", "job_sheet_editor"),
                        sep(),
                        MenuItem::with_action("Add-ons...", "addons"),
                    ],
                ),
                menu(
                    "Help",
                    vec![
                        i("Help Contents"),
                        i("Keyboard Shortcuts"),
                        i("Third Party Licenses"),
                        sub(
                            "Community",
                            crate::app::LINKS
                                .iter()
                                .enumerate()
                                .map(|(n, (label, _, icon))| {
                                    MenuItem::with_action(label, &format!("{COMMUNITY}{n}"))
                                        .icon(icon)
                                })
                                .collect(),
                        ),
                        sep(),
                        MenuItem::with_action("Check for Updates...", "check_for_updates"),
                        MenuItem::with_action("Run First-Time Setup Again...", "first_run_setup"),
                        i("About Cut Craft"),
                    ],
                ),
            ],
        }
    }

    /// The menus with no job open (see `re/SPEC.md`, Startup sequence): files, options, machines,
    /// tools and help. Machine, Gadgets and Help are the same as with a job.
    pub fn without_job(&self) -> Self {
        let i = |name: &str| MenuItem::new(name, "");
        let k = MenuItem::new;
        let sep = MenuItem::separator;
        let menu = |name: &str, items: Vec<MenuItem>| Menu {
            name: name.to_string(),
            items,
        };
        let same = |name: &str| self.menu(name).clone();
        Self {
            menus: vec![
                menu(
                    "File",
                    vec![
                        k("New", "Ctrl+N"),
                        i("New from Template..."),
                        k("Open...", "Ctrl+O"),
                        i("Open Application Data Folder"),
                        sep(),
                        i("Recent Files"),
                        sep(),
                        i("Close"),
                        i("Exit"),
                    ],
                ),
                menu("Edit", vec![i("Options")]),
                same("Machine"),
                // Materials colours a job, so it is in the job's Toolpaths menu only
                menu("Toolpaths", vec![i("Tool Database...")]),
                same("Gadgets"),
                same("Help"),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MenuItem;

    #[test]
    fn status_text_goes_on_one_line() {
        assert_eq!(super::one_line("Ready"), "Ready");
        assert_eq!(
            super::one_line("0331. Flowers.STL\nDouble-click to add it\r\n\n  "),
            "0331. Flowers.STL · Double-click to add it"
        );
    }

    #[test]
    fn separators_split_a_menu_into_categories() {
        let items = [
            MenuItem::separator(),
            MenuItem::with_action("Undo", "undo"),
            MenuItem::with_action("Redo", "redo"),
            MenuItem::separator(),
            MenuItem::separator(),
            MenuItem::with_action("Delete", "delete"),
        ];
        let names: Vec<Vec<&str>> = super::categories(&items)
            .iter()
            .map(|c| c.iter().map(|i| i.name.as_str()).collect())
            .collect();
        assert_eq!(names, [vec!["Undo", "Redo"], vec!["Delete"]]);
    }

    #[test]
    fn every_design_group_is_on_exactly_one_sub_tab_or_always_shown() {
        let toolbar = super::Toolbar::new();
        let placed: Vec<&str> = super::ALWAYS_SHOWN
            .iter()
            .chain(super::DESIGN_SUBTABS.iter().flat_map(|t| t.groups.iter()))
            .copied()
            .collect();
        for group in toolbar.all_design_groups() {
            let n = placed.iter().filter(|p| **p == group.name).count();
            assert_eq!(n, 1, "{} is placed {n} times", group.name);
        }
        for name in &placed {
            assert!(
                toolbar.all_design_groups().iter().any(|g| g.name == *name),
                "{name} is no group"
            );
        }
        assert_eq!(
            super::DESIGN_SUBTABS[0].name,
            "Create",
            "drawing comes first"
        );
    }

    #[test]
    fn a_sub_tab_shows_its_own_groups_and_far_fewer_buttons() {
        let toolbar = super::Toolbar::new();
        let names = |tab: usize| -> Vec<&str> {
            toolbar.subtab_groups(tab).iter().map(|g| g.name).collect()
        };
        assert_eq!(
            names(2),
            ["Align Objects", "Offset and Layout", "Array Objects"]
        );
        assert!(names(99).is_empty(), "a tab out of range shows nothing");
        let always: Vec<&str> = toolbar
            .named(super::ALWAYS_SHOWN)
            .iter()
            .map(|g| g.name)
            .collect();
        assert_eq!(always, super::ALWAYS_SHOWN);
        let faces = crate::flyout::Faces::new();
        let count = |groups: Vec<super::ToolGroup>| -> usize {
            groups
                .iter()
                .map(|g| crate::flyout::buttons(g.tools, &faces).len())
                .sum()
        };
        let whole = count(toolbar.all_design_groups());
        for tab in 0..super::DESIGN_SUBTABS.len() {
            let mut shown = toolbar.named(super::ALWAYS_SHOWN);
            shown.extend(toolbar.subtab_groups(tab));
            let n = count(shown);
            assert!(
                n <= 36 && n * 2 < whole,
                "{}: {n} of {whole}",
                super::DESIGN_SUBTABS[tab].name
            );
        }
    }

    #[test]
    fn the_import_sub_tab_gathers_every_import() {
        let toolbar = super::Toolbar::new();
        let tab = super::DESIGN_SUBTABS
            .iter()
            .position(|t| t.name == "Import")
            .expect("an Import sub-tab");
        let tools: Vec<&str> = toolbar
            .subtab_groups(tab)
            .iter()
            .flat_map(|g| g.tools.iter().copied())
            .collect();
        for id in [
            "import_vectors_bitmap",
            "import_vectors",
            "import_component_3d_model",
            "clipart_library",
            "import_toolpaths_file",
            "import_cut_list",
        ] {
            assert!(tools.contains(&id), "{id} is on Import");
            assert_eq!(toolbar.subtab_of(id, 0), Some(tab), "{id}");
            // each its own button, named on its own, so none hides behind another
            assert!(crate::flyout::flyout_of(id).is_none(), "{id} in a flyout");
            assert!(super::about(id).is_some(), "{id} says what it does");
        }
        assert_eq!(
            super::built_in_label("import_vectors_bitmap"),
            Some("Import Vectors or Picture")
        );
        let file = toolbar.named(&["File Operations"]);
        assert!(
            !file[0].tools.iter().any(|t| t.starts_with("import")),
            "imports live on the Import sub-tab only"
        );
    }

    #[test]
    fn sub_tabs_go_in_the_row_or_a_section_of_their_own() {
        let none: Vec<String> = vec![];
        let all = super::DESIGN_SUBTABS.len();
        assert_eq!(super::subtab_layout(&none), ((0..all).collect(), vec![]));
        let model = vec!["Model".to_owned(), "No such tab".to_owned()];
        let (row, apart) = super::subtab_layout(&model);
        assert_eq!(apart, [3]);
        assert_eq!(row.len(), all - 1);
        // the row shows its open tab, else its first; nothing when it is empty
        assert_eq!(super::open_subtab(1, &model), Some(1));
        assert_eq!(super::open_subtab(3, &model), Some(0));
        let every = super::placed(&none, "subtab_place:*:section").unwrap();
        assert_eq!(super::open_subtab(0, &every), None);
        assert_eq!(every.len(), all);
        // moved one at a time, kept in tab order
        let one = super::placed(&none, "subtab_place:Import:section").unwrap();
        let two = super::placed(&one, "subtab_place:Edit:section").unwrap();
        assert_eq!(two, ["Edit", "Import"]);
        assert_eq!(
            super::placed(&two, "subtab_place:Import:row").unwrap(),
            ["Edit"]
        );
        assert_eq!(super::placed(&two, "subtab_place:*:row"), Some(vec![]));
        assert_eq!(super::placed(&two, "subtab_place:Nothing:row"), None);
        assert_eq!(super::placed(&two, "favourite:design:add:x"), None);
        // the menu offers the move that applies, and ticks the layout in use
        let menu = super::subtab_menu("Edit", &two);
        assert_eq!(menu[0].name, "Put Back in the Tab Row");
        assert_eq!(menu[0].action, "subtab_place:Edit:row");
        let menu = super::subtab_menu("Create", &none);
        assert_eq!(menu[0].name, "Show as Its Own Section");
        let ticked: Vec<&str> = menu
            .iter()
            .filter(|i| i.is_checked)
            .map(|i| i.name.as_str())
            .collect();
        assert_eq!(ticked, ["All Tabs in One Row"]);
    }

    #[test]
    fn a_tool_is_found_on_its_sub_tab() {
        let toolbar = super::Toolbar::new();
        assert_eq!(toolbar.subtab_of("align_left", 0), Some(2));
        assert_eq!(toolbar.subtab_of("move", 0), Some(1));
        assert_eq!(toolbar.subtab_of("sculpt", 0), Some(3));
        assert_eq!(toolbar.subtab_of("save", 0), None, "always shown");
        assert_eq!(toolbar.subtab_of("no_such_tool", 0), None);
        // Distort Object is on Edit and on Model: it stays where it was started from
        assert_eq!(toolbar.subtab_of("distort_object", 3), Some(3));
        assert_eq!(toolbar.subtab_of("distort_object", 1), Some(1));
        assert_eq!(toolbar.subtab_of("distort_object", 0), Some(1));
    }

    #[test]
    fn title_case_capitalizes_each_word() {
        assert_eq!(super::title_case("join_vectors"), "Join Vectors");
        assert_eq!(super::title_case("2d_view"), "2d View");
    }
}

#[cfg(test)]
mod menu_tests {
    use super::MenuSystem;

    #[test]
    fn every_strategy_in_the_toolpaths_tab_is_a_command() {
        let registry = crate::commands::registry();
        for group in super::toolpath_groups() {
            if group.name == "Output" {
                continue; // app actions, not commands
            }
            for id in group.tools {
                assert!(
                    registry.get(id).is_some(),
                    "{} lists `{id}`, which is not a command",
                    group.name
                );
            }
        }
    }

    #[test]
    fn align_submenu_items_run_the_align_commands() {
        let registry = crate::commands::registry();
        let menus = MenuSystem::new();
        let edit = menus.menus.iter().find(|m| m.name == "Edit").unwrap();
        let align = edit
            .items
            .iter()
            .find(|i| i.name == "Align Selected Objects")
            .unwrap();
        for item in align
            .children
            .iter()
            // the spacing items run their command with its settings
            .filter(|i| !i.is_separator() && !i.action.starts_with("run:"))
        {
            assert!(
                registry.get(&item.action).is_some(),
                "Align > {} runs `{}`, which is not a command",
                item.name,
                item.action
            );
        }
    }
}
