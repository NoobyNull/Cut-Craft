//! Static UI data: Drawing tab tool groups, the main menu, right-click menus, status bar text.

use cmd_space_horizontally::Order as SpaceH;
use cmd_space_vertically::Order as SpaceV;
use cut_craft_core::Command;
use cut_craft_core::component::{Combine, Component, Group, Level, Mirror};
use cut_craft_core::document::{Document, EntityId, Layer};
use cut_craft_core::params::Items;
use cut_craft_core::toolpath::Toolpath;
mod context_menus;
mod menu_bar;
pub use context_menus::*;
mod tool_groups;
use tool_groups::{drawing_groups, modeling_groups, toolpath_groups};

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
/// shows a couple of dozen buttons instead of every icon at once down
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
        groups: &["Shape Creation", "Shape Editing", "Export Model"],
    },
    SubTab {
        name: "Import",
        hint: "Bring in drawings, pictures, 3D models, clipart, toolpaths and cut lists",
        groups: &["Import"],
    },
];

/// The Toolpaths page's sub-tabs, in order, the same way as the Design page's: making a
/// toolpath, the list of those made, previewing them and saving them for the machine each
/// get the page's whole height, rather than the list taking half of it and pushing the
/// last tool groups off the bottom (`re/SPEC.md`, Toolpaths sub-tabs).
pub const TOOLPATH_SUBTABS: &[SubTab] = &[
    SubTab {
        name: "Create",
        hint: "Pick how to cut: each button makes a toolpath from the vectors or model you \
               selected",
        groups: &[
            "Cut Out and Clear",
            "Holes",
            "Carve and Engrave",
            "Edges and Inlays",
            "3D Toolpaths",
            "Laser",
            "From Other Toolpaths",
            "Templates",
        ],
    },
    SubTab {
        name: "Preview",
        hint: "Watch the toolpaths cut the material",
        groups: &[],
    },
    SubTab {
        name: "Analyse",
        hint: "Check the toolpaths before cutting: what would stop or spoil the cut, what is \
               out of date, and how long each takes",
        groups: &["Reports"],
    },
    SubTab {
        name: "Output",
        hint: "Write the file your machine runs or send it there, and the sheets and labels \
               that go with it",
        groups: &["Save for the Machine", "Paperwork for the Job", "Big Jobs"],
    },
];

/// The groups the Toolpaths page shows over its sub-tabs whichever is open: the material
/// and the bits, wanted at any stage.
pub const TOOLPATH_ALWAYS_SHOWN: &[&str] = &["Job and Tools"];

/// Moving a Toolpaths sub-tab between the tab row and a section of its own, as
/// [`SUBTAB_PLACE`] does for the Design page's.
pub const TOOLPATH_SUBTAB_PLACE: &str = "toolpath_subtab_place:";

/// A page's sub-tabs, the groups shown over them, and the action that moves one between the
/// tab row and a section of its own: the Design and Toolpaths pages follow the same rules.
#[derive(Clone, Copy, Debug)]
pub struct SubTabSet {
    pub tabs: &'static [SubTab],
    pub always: &'static [&'static str],
    pub place: &'static str,
}

/// The Design page's sub-tabs.
pub const DESIGN: SubTabSet = SubTabSet {
    tabs: DESIGN_SUBTABS,
    always: ALWAYS_SHOWN,
    place: SUBTAB_PLACE,
};

/// The Toolpaths page's sub-tabs.
pub const TOOLPATHS: SubTabSet = SubTabSet {
    tabs: TOOLPATH_SUBTABS,
    always: TOOLPATH_ALWAYS_SHOWN,
    place: TOOLPATH_SUBTAB_PLACE,
};

impl SubTabSet {
    /// The sub-tabs in the tab row and those shown as sections of their own under it, by
    /// index in `tabs`, each in that order. `sections` names the sub-tabs moved out of the
    /// row (kept in the options); names that are no sub-tab are ignored.
    pub fn layout(&self, sections: &[String]) -> (Vec<usize>, Vec<usize>) {
        (0..self.tabs.len()).partition(|&i| !sections.iter().any(|s| s == self.tabs[i].name))
    }

    /// The sub-tab the row shows open: `open` while it is in the row, else the row's first;
    /// none when every sub-tab is a section.
    pub fn open(&self, open: usize, sections: &[String]) -> Option<usize> {
        let (row, _) = self.layout(sections);
        row.contains(&open).then_some(open).or(row.first().copied())
    }

    /// A sub-tab's right-click menu, on its tab in the row or its section's heading.
    pub fn menu(&self, name: &str, sections: &[String]) -> Vec<MenuItem> {
        let place = self.place;
        let in_section = sections.iter().any(|s| s == name);
        let (row, apart) = self.layout(sections);
        let mut menu = vec![if in_section {
            MenuItem::with_action("Put Back in the Tab Row", &format!("{place}{name}:row"))
        } else {
            MenuItem::with_action("Show as Its Own Section", &format!("{place}{name}:section"))
        }];
        menu.push(MenuItem::separator());
        menu.push(MenuItem {
            is_checked: apart.is_empty(),
            ..MenuItem::with_action("All Tabs in One Row", &format!("{place}*:row"))
        });
        menu.push(MenuItem {
            is_checked: row.is_empty(),
            ..MenuItem::with_action("Every Tab as Its Own Section", &format!("{place}*:section"))
        });
        menu
    }

    /// `sections` after a move action of this set, kept in `tabs` order; `None` for an
    /// action that is not one.
    pub fn placed(&self, sections: &[String], action: &str) -> Option<Vec<String>> {
        let (name, place) = action.strip_prefix(self.place)?.rsplit_once(':')?;
        let apart = match place {
            "section" => true,
            "row" => false,
            _ => return None,
        };
        if name != "*" && !self.tabs.iter().any(|t| t.name == name) {
            return None;
        }
        Some(
            self.tabs
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
}

/// Moving a Design sub-tab between the tab row and a section of its own: then the sub-tab's
/// name and `section` or `row`, or `*` and either for every sub-tab.
pub const SUBTAB_PLACE: &str = "subtab_place:";

/// Names and descriptions of the built-in tools on the Import sub-tab, which have no
/// command crate to give them: the names run words together ("Import Vectors Bitmap")
/// and say nothing about the files they take.
pub const BUILT_IN_ABOUTS: &[(&str, &str, &str)] = &[
    (
        "setup_material",
        "Setup Material",
        "Set the material for this job: single sided, double sided or rotary, and its size. \
         Z zero, X0 Y0 and the rest come from Options ▸ Job Defaults",
    ),
    (
        "machine_files",
        "Machine Files",
        "See the files on the machine's card or shared folder, and upload, rename, delete or \
         make folders there, without walking over to the machine",
    ),
    (
        "import_vectors_bitmap",
        "Import Vectors or Picture",
        "Bring in a drawing (DXF, SVG or a Vectric job's vectors) or a picture (PNG, JPG, \
         BMP, GIF or TIFF) to trace, carve round or draw over. An SVG sized in pixels is \
         read at 96 pixels per inch, and a note says the size it came in at",
    ),
    (
        "import_vectors",
        "Import Vectors",
        "Bring in a drawing made in another program: DXF, SVG or a Vectric job's vectors",
    ),
    (
        "import_into_shape",
        "Import Into Selected Shape",
        "Select a closed shape first (a circle, plaque or heart), then bring in a drawing or \
         picture sized to fit inside it: scaled, stretched or bent to follow its outline. \
         Select an open line instead to bend the drawing along it, like lettering on an arc",
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
         width and how many, ready for Nest Parts. Sizes are in the units after the Length \
         and Width headings, such as Length (in) or Length (mm), else in the job's units. \
         The note after importing names the units used for each, so a heading missing its \
         unit shows up",
    ),
    // the Model sub-tab's exports, which are file formats rather than commands
    (
        "export_as_stl",
        "Export Model as STL",
        "Save the 3D model as an STL file, for a 3D printer or another program",
    ),
    (
        "export_as_grayscale_bitmap",
        "Export Model as Grayscale Picture",
        "Save the 3D model as a PNG picture where lighter is higher, for another carving \
         program",
    ),
    (
        "export_as_3d_clipart",
        "Export Model as 3D Clipart",
        "Save the selected 3D parts as clipart, to drop into other jobs from the Library",
    ),
    // the Toolpaths page's templates and printouts, which were only in the Toolpaths menu
    (
        "load_template",
        "Load Toolpath Template",
        "Add the toolpaths saved in a template to this job, ready to pick the vectors (or \
         picture) each one cuts and calculate. A Merged or Array Copy toolpath uses the \
         toolpaths saved with it; one saved without them says to tick the ones to use",
    ),
    (
        "save_selected_toolpath_as_template",
        "Save Toolpath as Template",
        "Save the selected toolpath's settings and tools, to make the same cut on another job",
    ),
    (
        "save_all_visible_toolpaths_as_template",
        "Save All Shown Toolpaths as Template",
        "Save the settings and tools of every toolpath shown in the list, to make the same \
         cuts on another job",
    ),
    (
        "create_job_sheet",
        "Save Job Sheet",
        "Save a page for the machine: the job, the material, each toolpath's tool and how long \
         it takes",
    ),
    (
        "print_job_sheet",
        "Print Job Sheet",
        "Open the job sheet in the web browser to print it: the job, the material, each \
         toolpath's tool and how long it takes",
    ),
    (
        "part_labels",
        "Print Part Labels",
        "Print a label for each named part, to stick on it once it is cut out",
    ),
    (
        "clearing_settings",
        "Clearing Settings",
        "How adaptive and trochoidal clearing cut in this job: the light step, the loops, \
         and whether rows and loops keep climb on the way back",
    ),
    (
        "advanced_rotary_mode",
        "Turn On Advanced Rotary Mode",
        "Open Options at Advanced rotary mode: each rotary toolpath then cuts the whole blank, \
         not only what the toolpaths above it leave",
    ),
    (
        "cut_list",
        "Save Cut List",
        "Save a list of the job's parts and their sizes, as a spreadsheet (CSV)",
    ),
    (
        "show_open_vector_ends",
        "Show Open Vector Ends",
        "Put a circle on each end of every open vector, so a gap that stops a shape being \
         pocketed or carved stands out",
    ),
    (
        "show_construction_lines",
        "Show Construction Lines",
        "Show or hide the dashed construction lines: lines that only place things and are \
         never cut. Hidden, they cannot be picked either",
    ),
];

/// The icons of built-in tools whose file is not named for their id.
const BUILT_IN_ICONS: &[(&str, &str)] = &[
    ("load_template", "toolpaths/load_toolpath_template.png"),
    (
        "save_selected_toolpath_as_template",
        "toolpaths/save_toolpath_template.png",
    ),
    (
        "save_all_visible_toolpaths_as_template",
        "toolpaths/save_all_as_template.png",
    ),
];

/// A built-in tool's own icon, where no file is named for its id.
pub fn built_in_icon(id: &str) -> Option<&'static str> {
    BUILT_IN_ICONS
        .iter()
        .find(|(i, _)| *i == id)
        .map(|(_, icon)| *icon)
}

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
        subtab_of_in(DESIGN_SUBTABS, &self.all_design_groups(), id, open)
    }

    /// The Toolpaths page's groups called `names`, in that order.
    pub fn toolpath_named(&self, names: &[&str]) -> Vec<ToolGroup> {
        names
            .iter()
            .filter_map(|name| {
                self.toolpath_groups
                    .iter()
                    .find(|g| g.name == *name)
                    .copied()
            })
            .collect()
    }

    /// Toolpaths sub-tab `tab`'s own groups; none for a tab out of range.
    pub fn toolpath_subtab_groups(&self, tab: usize) -> Vec<ToolGroup> {
        self.toolpath_named(TOOLPATH_SUBTABS.get(tab).map_or(&[][..], |t| t.groups))
    }

    /// The Toolpaths sub-tab `id` is on, if any, as [`Self::subtab_of`].
    pub fn toolpath_subtab_of(&self, id: &str, open: usize) -> Option<usize> {
        subtab_of_in(TOOLPATH_SUBTABS, &self.toolpath_groups, id, open)
    }
}

/// The sub-tab of `tabs` whose groups (found in `all`) hold `id`: `open` when it is one.
fn subtab_of_in(tabs: &[SubTab], all: &[ToolGroup], id: &str, open: usize) -> Option<usize> {
    let on = |t: &SubTab| {
        t.groups
            .iter()
            .any(|name| all.iter().any(|g| g.name == *name && g.tools.contains(&id)))
    };
    if tabs.get(open).is_some_and(on) {
        return Some(open);
    }
    tabs.iter().position(on)
}

/// `"join_vectors"` → `"Join Vectors"`: the label of a tool that is not implemented yet.
pub fn title_case(id: &str) -> String {
    id.split('_')
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map_or_default(|f| f.to_uppercase().chain(c).collect::<String>())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Status bar text
mod abouts {
    include!(concat!(env!("OUT_DIR"), "/abouts.rs"));
}

/// Every command a crate under `crates/commands` makes, by id.
#[cfg(test)]
pub fn command_ids() -> impl Iterator<Item = &'static str> {
    abouts::ABOUTS.iter().map(|(id, _)| *id)
}

/// What command `id` does (its crate's first sentence), for its tooltip and the status bar.
pub fn about(id: &str) -> Option<&'static str> {
    // the hints first: a crate holding several commands says one thing for all of them
    crate::hints::hint(id)
        .or_else(|| {
            abouts::ABOUTS
                .iter()
                .find(|(command, _)| *command == id)
                .map(|(_, about)| *about)
        })
        .or_else(|| {
            BUILT_IN_ABOUTS
                .iter()
                .find(|(i, _, _)| *i == id)
                .map(|(_, _, about)| *about)
        })
        .or_else(|| crate::flyout::variant(id).map(|v| v.hint))
}

/// What command `id` does at length (its crate's first paragraph, the lead `Name:` that only
/// repeats its name taken off), for its help page when its params have no doc comment.
pub fn intro(id: &str, label: &str) -> Option<String> {
    let docs = crate::hints::hint(id)
        .or_else(|| {
            abouts::INTROS
                .iter()
                .find(|(command, _)| *command == id)
                .map(|(_, docs)| *docs)
        })
        .or_else(|| about(id))?;
    let body = docs
        .strip_prefix(label)
        .and_then(|rest| rest.strip_prefix(':'))
        .map_or(docs, str::trim_start);
    let mut chars = body.chars();
    let first = chars.next()?;
    Some(first.to_uppercase().chain(chars).collect())
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

/// The node-editing popup for the node or span at `at`.
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

/// The Gadgets menu's submenu of what the user installed.
pub const MY_GADGETS: &str = "My Gadgets";
/// Gadgets ▸ My Gadgets ▸ Open My Gadgets Folder: where the user's own `.lua` gadgets go.
pub const OPEN_GADGETS_FOLDER: &str = "open_gadgets_folder";

/// Gadgets ▸ My Gadgets: each gadget and add-on the user installed, `(name, action)`, or a
/// line saying there is none yet; then the way to add more.
fn my_gadgets_items(installed: &[(String, String)]) -> Vec<MenuItem> {
    let mut items: Vec<MenuItem> = installed
        .iter()
        .map(|(name, action)| MenuItem::with_action(name, action))
        .collect();
    if items.is_empty() {
        items.push(MenuItem {
            is_enabled: false,
            ..MenuItem::with_action("Nothing installed yet", "")
        });
    }
    items.extend([
        MenuItem::separator(),
        MenuItem::with_action("Open My Gadgets Folder", OPEN_GADGETS_FOLDER),
        MenuItem::with_action("Add-ons...", "addons"),
    ]);
    items
}

impl MenuSystem {
    /// Adds Cut Craft's own Lua gadgets to the Gadgets menu, below the built-in ones, then
    /// the My Gadgets submenu listing what the user installed (see [`Self::set_my_gadgets`]).
    pub fn add_gadgets(
        &mut self,
        built_in: &[crate::gadgets::Listed],
        installed: &[(String, String)],
    ) {
        let Some(menu) = self.menus.iter_mut().find(|m| m.name == "Gadgets") else {
            return;
        };
        // Add-ons... stays last, after the gadgets
        let addons = menu.items.iter().position(|i| i.action == "addons");
        let tail: Vec<MenuItem> =
            addons.map_or_default(|at| menu.items.drain(at.saturating_sub(1)..).collect());
        menu.items.push(MenuItem::separator());
        menu.items.extend(
            built_in
                .iter()
                .map(|g| MenuItem::with_action(&g.label, &g.id)),
        );
        menu.items
            .push(MenuItem::submenu(MY_GADGETS, my_gadgets_items(installed)));
        menu.items.extend(tail);
    }

    /// Gadgets ▸ My Gadgets lists `installed` (each name with the action that runs it) in
    /// place of what it listed, as add-ons come and go.
    pub fn set_my_gadgets(&mut self, installed: &[(String, String)]) {
        let mine = self
            .menus
            .iter_mut()
            .filter(|m| m.name == "Gadgets")
            .flat_map(|m| m.items.iter_mut())
            .find(|i| i.name == MY_GADGETS);
        if let Some(mine) = mine {
            mine.children = my_gadgets_items(installed);
        }
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

    /// The 3D view's right-click menu: the 2D/3D view menu (help: Right Mouse Click
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
            // ponytail: the Text ▸ also removes text from a curve and breaks a block
            // into lines; neither exists yet
            MenuItem::submenu(
                "Text",
                vec![MenuItem::with_action(
                    "Convert to Vectors",
                    "convert_text_to_curves",
                )],
            )
            .enabled(text),
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

    /// The 2D view's right-click menu: the selection menu, plus Move to Layer,
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
            MenuItem::with_action(
                if crate::app::line_to_import_along(doc) {
                    "Import Along This Line…"
                } else {
                    "Import Into This Shape…"
                },
                "import_into_shape",
            )
            .enabled(crate::app::shape_to_import_into(doc)),
            construction_item(doc),
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
/// The selection's construction line item: Make Real Line when every selected vector is a
/// construction line already, else Make Construction Line.
fn construction_item(doc: &Document) -> MenuItem {
    if cmd_construction_lines::all_construction(doc) {
        MenuItem::with_action("Make Real Line", "make_real_line")
    } else {
        MenuItem::with_action("Make Construction Line", "make_construction_line")
            .enabled(!doc.selection.is_empty())
    }
}

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
    /// Its picture, an icon path, where the menu has one (the combine modes).
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

/// Show ▸ and Hide ▸ for one layer, toolpath or component: the Show / Hide family.
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

/// Delete ▸ for one layer, toolpath or component: the Delete family.
fn delete(items: Items, item: u64, which: &[(&str, cmd_delete_items::Which)]) -> MenuItem {
    use cmd_delete_items::{DeleteItems, Params};
    let run = |&(name, which): &(&str, _)| {
        MenuItem::run::<DeleteItems>(name, &Params { items, item, which })
    };
    MenuItem::submenu("Delete", which.iter().map(run).collect())
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

#[cfg(test)]
mod tests;

#[cfg(test)]
mod menu_tests {
    use super::MenuSystem;

    #[test]
    fn every_strategy_in_the_toolpaths_tab_is_a_command() {
        let registry = crate::commands::registry();
        for group in super::toolpath_groups() {
            if [
                "Job and Tools",
                "Reports",
                "Save for the Machine",
                "Paperwork for the Job",
                "Big Jobs",
            ]
            .contains(&group.name)
            {
                continue; // app actions, not commands
            }
            // the templates' load and save are app actions beside the template commands
            for id in group
                .tools
                .iter()
                .filter(|id| super::built_in_label(id).is_none())
            {
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
