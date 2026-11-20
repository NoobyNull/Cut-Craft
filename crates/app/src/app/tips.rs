//! Did You Know?: short tips for someone new to CAD/CAM, one shown when Cut Craft starts
//! (Edit ▸ Options ▸ General turns that off) and all of them from Help ▸ Did You Know? and
//! the Help window's own page of them. Cut Craft's own (see `re/SPEC.md`, Did You Know?).

use super::{CutCraftApp, Info, SPLASH_LOGO, egui};
use cut_craft_i18n::tr;
use std::fmt::Write as _;

/// What a tip is about: the Help window's page of tips lists them under these, in this order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Area {
    Start,
    Drawing,
    Modelling,
    Toolpaths,
    Cutting,
    Program,
    Safety,
}

impl Area {
    /// Every subject, in the order the page lists them.
    pub const ALL: [Self; 7] = [
        Self::Start,
        Self::Drawing,
        Self::Modelling,
        Self::Toolpaths,
        Self::Cutting,
        Self::Program,
        Self::Safety,
    ];

    /// The page's heading for this subject.
    pub fn heading(self) -> &'static str {
        match self {
            Self::Start => "Getting started",
            Self::Drawing => "Drawing and editing",
            Self::Modelling => "3D models",
            Self::Toolpaths => "Toolpaths",
            Self::Cutting => "Preview, save and send",
            Self::Program => "Settings, help and updates",
            Self::Safety => "Cutting safely",
        }
    }
}

/// The picture above a tip's words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Picture {
    /// A scene drawn for this tip alone, by `scripts/make_icons.py` into `assets/icons/tips`.
    Scene(&'static str),
    /// A tool's button icon (by the tool's id), or another of Cut Craft's icons (by its path),
    /// so the tool can be found by it.
    Icon(&'static str),
}

/// A tip: its picture, what it says (Markdown), what it is about and the help page that says
/// more (Tell Me More…).
#[derive(Clone, Copy, Debug)]
pub(super) struct Tip {
    pub area: Area,
    pub picture: Picture,
    pub text: &'static str,
    pub topic: &'static str,
}

/// A tip with a scene of its own.
const fn scene(area: Area, picture: &'static str, text: &'static str, topic: &'static str) -> Tip {
    Tip {
        area,
        picture: Picture::Scene(picture),
        text,
        topic,
    }
}

/// A tip about one tool, showing its icon and opening its help.
const fn tool(area: Area, id: &'static str, text: &'static str) -> Tip {
    Tip {
        area,
        picture: Picture::Icon(id),
        text,
        topic: id,
    }
}

/// A tip showing `icon` (a tool's id or an icon's path), opening help page `topic`.
const fn icon(area: Area, icon: &'static str, text: &'static str, topic: &'static str) -> Tip {
    Tip {
        area,
        picture: Picture::Icon(icon),
        text,
        topic,
    }
}

/// Every tip, in the order startup shows them: the first few are what a beginner needs on day
/// one, the rest make the work quicker, safer or easier to find. Each is true of Cut Craft as
/// it is: check a tip whenever the tool it names changes.
pub(super) const TIPS: &[Tip] = &[
    scene(
        Area::Start,
        "tips/sample_project.png",
        "New to Cut Craft? On the blank sheet it opens with, **Try a sample project** opens a \
         finished sign with its toolpaths already made, so you can see the whole journey from \
         drawing to cutting file before you start your own.",
        "getting_started",
    ),
    scene(
        Area::Start,
        "tips/f1_help.png",
        "Press **F1** with a tool's settings open to read the help for that tool. With nothing \
         open, F1 opens Help Contents, where you can search every tool.",
        "finding_help",
    ),
    scene(
        Area::Start,
        "tips/hover_note.png",
        "Not sure what a setting means? Hover the pointer over its name for a short note about \
         it.",
        "finding_help",
    ),
    scene(
        Area::Start,
        "tips/tool_list.png",
        "The **?** at the top of the Design and Toolpaths pages lists every tool on that page, \
         each with a page of help.",
        "design",
    ),
    scene(
        Area::Start,
        "tips/design_then_toolpaths.png",
        "Cut Craft works in two steps. **Design** (down the left) is where you draw what you \
         want. **Toolpaths** (down the right) is where you tell the machine how to cut it.",
        "getting_started",
    ),
    scene(
        Area::Start,
        "tips/preview_cut.png",
        "Always **Preview** a toolpath before you cut. It shows the bit carving the material on \
         screen, so a mistake costs nothing but a click.",
        "toolpaths",
    ),
    scene(
        Area::Drawing,
        "tips/closed_shape.png",
        "Most toolpaths need a **closed** shape: one whose line meets itself all the way round. \
         **View ▸ Show Open Vector Ends** marks the gaps, and **Edit ▸ Join Open Ends** closes \
         them.",
        "join_open_ends",
    ),
    scene(
        Area::Drawing,
        "tips/common_lines.png",
        "Lines drawn in **red** are common lines: two vectors on the same layer lying exactly \
         on top of each other, as Paste or a copy that lands on its original leaves them. A \
         profile would cut that line twice, so delete or move the extra copy.",
        "design",
    ),
    scene(
        Area::Toolpaths,
        "tips/tool_database.png",
        "Picking a bit from the **Tool Database** fills in its speeds, feeds and depth per pass \
         for you. Set them once for each bit and every toolpath uses them.",
        "tool_database",
    ),
    scene(
        Area::Toolpaths,
        "tips/out_of_date.png",
        "Changed a shape, or the material's thickness in Job Setup, after making its toolpath? \
         The toolpath is marked out of date, and Cut Craft offers to recalculate it. Saving or \
         sending an out-of-date toolpath always stops to ask first.",
        "toolpaths",
    ),
    scene(
        Area::Program,
        "tips/number_sums.png",
        "Number fields take sums. Type **W/2** for half the material's width, or **T** for its \
         thickness, and the answer is worked out when you press Enter.",
        "number_fields",
    ),
    scene(
        Area::Program,
        "tips/number_arrows.png",
        "Every number field has small up and down arrows. Hold **Shift** while clicking them \
         for steps ten times as big.",
        "number_fields",
    ),
    scene(
        Area::Start,
        "tips/units.png",
        "Switching between millimetres and inches converts every measurement in the job, so \
         nothing changes size. Work in whichever your rulers and bits use.",
        "getting_started",
    ),
    scene(
        Area::Start,
        "tips/mouse_zoom.png",
        "Roll the mouse wheel to zoom in and out about the pointer. Drag with the middle mouse \
         button to move around the drawing.",
        "view_2d",
    ),
    scene(
        Area::Start,
        "tips/views_2d_3d.png",
        "**F2** shows the 2D view and **F3** the 3D view. **F11** and **F12** switch between \
         the layout for designing and the layout for machining.",
        "view_2d",
    ),
    scene(
        Area::Program,
        "tips/esc_close.png",
        "Press **Esc** to close the open tool's settings without making anything.",
        "keyboard_shortcuts",
    ),
    scene(
        Area::Drawing,
        "tips/edit_again.png",
        "Select a shape and press **E** to change it in the tool that made it, with the same \
         settings you used.",
        "keyboard_shortcuts",
    ),
    scene(
        Area::Drawing,
        "tips/snapping.png",
        "Drawing snaps to the grid, guides and the ends and middles of lines. **F4** chooses \
         what it snaps to, and holding **Shift** while you draw turns snapping off. A shape \
         stays on what it snapped to until you move it on its own.",
        "snapping",
    ),
    scene(
        Area::Start,
        "tips/undo.png",
        "Made a mistake? **Ctrl+Z** undoes it, as many times as you need. **Ctrl+Y** puts it \
         back.",
        "getting_started",
    ),
    scene(
        Area::Program,
        "tips/favourites.png",
        "Use a tool all the time? Right-click its button ▸ **Add to Favourites**, then \
         right-click the page's tab to show just your favourites.",
        "favourites",
    ),
    scene(
        Area::Toolpaths,
        "tips/bigger_bits_first.png",
        "Pockets with fine detail cut much faster with **Clear with bigger bits first**: a \
         large bit clears the open space and the small bit only cuts the corners it can't \
         reach.",
        "pocket_toolpath",
    ),
    scene(
        Area::Cutting,
        "tips/toolpath_time.png",
        "**Toolpath Summary**, on the Toolpaths page, says how long each toolpath takes to cut, \
         so you know how long the machine will be busy before you start.",
        "toolpath_summary",
    ),
    scene(
        Area::Program,
        "tips/increment_save.png",
        "**File ▸ Increment and Save** saves a new copy with the next number on its name (sign_1, \
         then sign_2), so you can always go back to an earlier version.",
        "saving_jobs",
    ),
    scene(
        Area::Program,
        "tips/recovery_copy.png",
        "Cut Craft keeps a recovery copy of unsaved work every 30 seconds. If the power goes \
         off, your work opens again next time, and the copy is kept until you save or close \
         it. **Edit ▸ Options ▸ General** sets how often.",
        "saving_jobs",
    ),
    scene(
        Area::Drawing,
        "tips/trace_bitmap.png",
        "Have a picture or logo? **Trace Bitmap** turns it into vectors you can carve.",
        "trace_bitmap",
    ),
    scene(
        Area::Modelling,
        "tips/relief_from_photo.png",
        "**Relief from Photo** (Modeling ▸ Shape Creation) turns a picture into a 3D shape to \
         carve.",
        "relief_from_photo",
    ),
    scene(
        Area::Cutting,
        "tips/job_sheet.png",
        "**Toolpaths ▸ Print Job Sheet** prints what the job needs at the machine: the file \
         names, each bit and where it goes, and how long it all takes.",
        "print_job_sheet",
    ),
    scene(
        Area::Program,
        "tips/report_bug.png",
        "Something not working as it should? Right-click ▸ **Report a Bug Here…** sends a \
         report about the part of the window you clicked on.",
        "bug_reports",
    ),
    scene(
        Area::Program,
        "tips/first_setup.png",
        "Got a new machine, or set it up wrong the first time? **Help ▸ Run First-Time Setup \
         Again** asks the starting questions again, starting from your answers, and corrects \
         a machine you set up as not listed, keeping the posts you added to it and the rest \
         of your default job.",
        "first_time_setup",
    ),
    scene(
        Area::Program,
        "tips/keyboard.png",
        "**Help ▸ Keyboard Shortcuts** lists the key for every tool, so you can work without \
         the menus. **Edit ▸ Options ▸ Mouse and Shortcuts** changes them, and can put a tool \
         or panning on your mouse's thumb buttons.",
        "keyboard_shortcuts",
    ),
    tool(
        Area::Drawing,
        "draw_rectangle",
        "**Draw Rectangle** can round or notch the corners as it draws: pick the **Corner Type** \
         and set its **Radius**, and a plaque's outline is done in one step.",
    ),
    tool(
        Area::Drawing,
        "draw_rectangle_from_center",
        "To centre a rectangle on a point, use **Draw Rectangle from Center**: drag from the \
         middle out to a corner.",
    ),
    tool(
        Area::Drawing,
        "draw_circle_from_edge",
        "**Draw Circle from Edge** drags from one side straight across to the other: handy when \
         you know where a circle's edges must touch rather than where its centre is.",
    ),
    tool(
        Area::Drawing,
        "draw_polygon",
        "**Draw Polygon** makes any regular shape: 3 **Sides** for a triangle, 6 for a hexagon. \
         At a **Rotation** of 0, a corner points straight up.",
    ),
    tool(
        Area::Drawing,
        "draw_star",
        "**Draw Star** sets the tips and the notches between them on their own, with **Outer \
         radius** and **Inner radius**: close together for a fat star, far apart for a spiky one.",
    ),
    tool(
        Area::Drawing,
        "draw_freehand",
        "Drawing with a mouse is shaky. **Draw Freehand**'s **Stabilize** setting only pulls the \
         line on once the pointer reaches the edge of a small circle, so little wobbles are \
         ignored.",
    ),
    tool(
        Area::Drawing,
        "draw_curve",
        "**Draw Curve** makes a smooth curve through the points you click, with no handles to \
         drag. Tick **Closed** to join it smoothly back to where it started.",
    ),
    tool(
        Area::Drawing,
        "draw_polyline",
        "With **Draw Line / Polyline**, click for straight spans, or drag between two points to \
         make that span a smooth curve.",
    ),
    tool(
        Area::Drawing,
        "draw_text",
        "**Draw Text** can bend a line of words into an arc: drag the green handles above and \
         below the text, or set **Arc angle**. Above zero it bends up, below zero down.",
    ),
    tool(
        Area::Drawing,
        "edit_text",
        "Spotted a spelling mistake? Double-click the text to open it in **Edit Text** and change \
         its words, font or size, even when it is inside a group.",
    ),
    tool(
        Area::Drawing,
        "convert_text_to_curves",
        "Want to reshape one letter? **Convert Text to Curves** turns text into plain vectors you \
         can node edit. Check the spelling first: afterwards it can't be edited as text.",
    ),
    tool(
        Area::Drawing,
        "text_on_a_curve",
        "**Text on a Curve** lays a line of text along any curve, above, on or below it. The text \
         stays editable and keeps its curve, so you can change the words later.",
    ),
    tool(
        Area::Drawing,
        "auto_layout_text",
        "**Auto Layout Text** wraps your words onto however many lines make them biggest, and \
         fits them inside the selected shape with a border kept clear.",
    ),
    tool(
        Area::Drawing,
        "dimensions",
        "**Dimensions** draws real measurements onto the job (lengths, angles and radii) as \
         vectors: useful on a plan you print. Keep them on a layer you never make toolpaths from.",
    ),
    tool(
        Area::Drawing,
        "create_fillets",
        "A round bit can't cut a sharp inside corner, so a square tenon won't go into its slot. \
         **Create Fillets** can cut dog-bone or T-bone relief into the corners so the joint fits.",
    ),
    tool(
        Area::Drawing,
        "offset_vectors",
        "**Offset Vectors** draws a copy of a shape a set distance outside it (positive) or \
         inside it (negative): an easy border round a sign or a frame round a picture.",
    ),
    tool(
        Area::Drawing,
        "weld_vectors",
        "Letters or shapes overlapping? **Weld Vectors** merges them into one outline and takes \
         away the lines inside, so a profile cuts round the outside only.",
    ),
    tool(
        Area::Drawing,
        "subtract_vectors",
        "**Subtract Vectors** works like a cookie cutter: the other selected shapes are cut out \
         of the largest one.",
    ),
    tool(
        Area::Drawing,
        "overlap_intersect",
        "**Overlap** keeps only the part where all the selected shapes overlap, such as the \
         corner a circle and a square share.",
    ),
    tool(
        Area::Drawing,
        "interactive_vector_trim",
        "**Interactive Vector Trim** removes the piece of line between two crossings with one \
         click. Drag across several pieces and every piece the drag crosses goes.",
    ),
    tool(
        Area::Drawing,
        "join_vectors",
        "Press **J** to **Join Vectors**: selected open vectors whose ends lie close together \
         become one, and it is closed when its ends meet.",
    ),
    tool(
        Area::Drawing,
        "join_smooth_curve",
        "**Join Smooth Curve** bridges the gap between two open ends with a curve that carries on \
         smoothly, where a straight line would leave a kink.",
    ),
    tool(
        Area::Drawing,
        "close_extend_vector",
        "**Close / Extend Vector** can close shapes by extending their end lines until they meet. \
         Its **Max gap** stops it closing ends that are far apart.",
    ),
    tool(
        Area::Drawing,
        "vector_validator",
        "Toolpath won't calculate? Run **Vector Validator**: it finds and selects vectors that \
         overlap, cross each other or have zero-length spans, and can remove those spans for you.",
    ),
    tool(
        Area::Drawing,
        "vector_validator",
        "Checking letters for a V-carve? Tick **V-Carving Mode** in **Vector Validator**: letters \
         overlapping within one text are let be, since V-carving cuts them fine.",
    ),
    tool(
        Area::Drawing,
        "fit_curves_to_vectors",
        "Traced or imported shapes made of thousands of tiny lines cut slowly and roughly. **Edit \
         ▸ Curve Fit Vectors** redraws them as arcs and smooth curves within a tolerance.",
    ),
    icon(
        Area::Drawing,
        "common/copy_duplicate.png",
        "**Edit ▸ Selection ▸ Select All Duplicate Vectors** picks the extra copies of vectors \
         drawn exactly on top of others, so you can delete them and keep one of each.",
        "design",
    ),
    tool(
        Area::Drawing,
        "select",
        "**Edit ▸ Selection ▸ Select Vectors Inside** adds every vector lying wholly inside a \
         selected closed shape, so a part and all its holes are picked in one go.",
    ),
    tool(
        Area::Drawing,
        "node_edit",
        "Press **N** for **Node Edit** to drag the points a vector is made of. Right-click a \
         point or a span to add, delete or change points.",
    ),
    tool(
        Area::Drawing,
        "transform",
        "Click a shape with **Transform** to size it with handles you drag; click it again to \
         switch to turning it.",
    ),
    tool(
        Area::Drawing,
        "move",
        "For exact positions and sizes, press **M** for **Move**, **T** for **Set Size** and \
         **R** for **Rotate**, then type the numbers.",
    ),
    tool(
        Area::Drawing,
        "set_size",
        "In **Set Size**, tick **Link XY** and type only the width or the height: the other is \
         worked out for you, so the shape is never stretched.",
    ),
    tool(
        Area::Drawing,
        "mirror",
        "**Mirror** with **Create mirrored copy** ticked keeps the original and adds a flipped \
         copy: ideal for the left and right halves of a symmetrical design.",
    ),
    tool(
        Area::Drawing,
        "center_in_material",
        "Press **F9** to move the selection to the very middle of the material.",
    ),
    tool(
        Area::Drawing,
        "left_to_right",
        "**Space Left to Right** spreads the selected objects evenly across while the outer ones \
         stay put, so place the two end ones first. A word of text counts as one object.",
    ),
    tool(
        Area::Drawing,
        "array_copy",
        "**Array Copy** makes a grid of copies in one go. **Row displacement** shifts every other \
         row along, for a brick or honeycomb pattern.",
    ),
    tool(
        Area::Drawing,
        "circular_copy",
        "**Circular Copy** spaces copies evenly round a centre, like the numbers on a clock. Tick \
         **Rotate copies** so each one turns with its place on the circle.",
    ),
    tool(
        Area::Drawing,
        "copy_along_vectors",
        "**Copy Along Vectors** spaces copies of an object along a line or curve, such as studs \
         round a border. Tick **Align to vector** so each copy follows the line.",
    ),
    tool(
        Area::Drawing,
        "nest_parts",
        "Cutting lots of parts from one board? **Nest Parts** packs them in a bit's width apart, \
         and with **True shape** small parts go into the holes of bigger ones.",
    ),
    tool(
        Area::Drawing,
        "name_part",
        "**Name Part** gives a part a name such as \"Left side\". **Save Cut List** and **Print \
         Part Labels** use it, so every piece is easy to tell apart once cut.",
    ),
    tool(
        Area::Drawing,
        "engrave_part_names",
        "**Engrave Part Names** writes each part's number and name in its middle, on a Part Names \
         layer, so a Quick Engrave from that layer marks every piece.",
    ),
    tool(
        Area::Drawing,
        "import_cut_list",
        "Have a cut list in a spreadsheet? Save it as CSV and use **Import Cut List**: a part is \
         drawn for each row, ready for **Nest Parts**.",
    ),
    tool(
        Area::Drawing,
        "plate_production",
        "Making a set of name badges? **Plate Production** fills a template's text from a data \
         file, one plate per row, swapping words such as `!!name!!` for each row's own.",
    ),
    tool(
        Area::Drawing,
        "document_variables",
        "Use one value in many places: make a **Document Variable** such as `depth`, then type \
         `{depth}` or `{depth}/2` in any number field. Fields filled in earlier keep their \
         numbers.",
    ),
    tool(
        Area::Drawing,
        "import_into_shape",
        "**Import Into Selected Shape** brings in a drawing or picture sized to fit inside a \
         selected shape, or bent along a selected line, like lettering round an arc.",
    ),
    tool(
        Area::Drawing,
        "clipart_library",
        "The **Library** tab holds ready-made drawings, pictures and 3D clipart. Drag one onto \
         the job to use it.",
    ),
    tool(
        Area::Drawing,
        "paste_in_place",
        "**Ctrl+Shift+V** (Paste in Place) puts what you copied exactly where it was copied from: \
         the way to move shapes into another job at the same spot.",
    ),
    tool(
        Area::Drawing,
        "vector_texture",
        "**Create Vector Texture** fills a shape with wavy lines or decorative fills. Keep its \
         **Smallest gap** wider than your bit cuts, or neighbouring cuts run together.",
    ),
    tool(
        Area::Drawing,
        "embellish",
        "**Embellish** fills a closed shape with ornament designed for it, or runs a pattern such \
         as a rope or leaves along an open line, as a border.",
    ),
    tool(
        Area::Drawing,
        "quick_trace",
        "**Quick Trace** traces a picture with no settings to choose, picking them itself. Open \
         **Trace Bitmap** when you want to see a preview and change them.",
    ),
    tool(
        Area::Drawing,
        "trace_bitmap",
        "**Trace Bitmap** can trace one line down the middle of each stroke instead of round its \
         outline: set **Trace type** for handwriting or line drawings to engrave.",
    ),
    tool(
        Area::Drawing,
        "edit_picture",
        "A faint scan traces badly. **Edit Picture** raises a picture's contrast and brightness \
         before you trace it, and a colour trace sees the change too.",
    ),
    tool(
        Area::Drawing,
        "crop_bitmap",
        "**Crop Bitmap** trims a picture to a selected shape, clearing everything outside it to \
         white, so tracing leaves the background out.",
    ),
    tool(
        Area::Drawing,
        "vector_boundary",
        "**Vector Boundary** draws one closed outline round the selected shapes. Tick **Rubber \
         Band Boundary** for the outline a rubber band stretched round them would make.",
    ),
    tool(
        Area::Drawing,
        "measure_inspect",
        "**Measure** reads the distance and angle between two points you drag between, both ends \
         snapping as drawing does. Click an object instead to see its size.",
    ),
    tool(
        Area::Drawing,
        "notes",
        "**Notes** keep text with the job, such as which board it is for. Start them with a full \
         stop (.) and they open by themselves whenever the job is opened.",
    ),
    tool(
        Area::Drawing,
        "distort_object",
        "**Distort Object** bends vectors, or 3D components, into an envelope, such as the band \
         above a curve: the quick way to an arched sign.",
    ),
    tool(
        Area::Modelling,
        "create_shape_round",
        "**Create Shape** raises a closed vector into 3D: **Round** for a dome, **Angled** for a \
         roof, **Smooth** for gently curving sides and **Flat** for a plateau.",
    ),
    tool(
        Area::Modelling,
        "create_shape_smooth",
        "In **Create Shape**, **Total height** scales the whole shape, base and all, to an exact \
         height, so it fits the material you have.",
    ),
    tool(
        Area::Modelling,
        "create_shape_angled",
        "Tick **Blend to inner vectors** in **Create Shape**, and a shape drawn inside the \
         outline becomes the top instead of a hole: the sides rise from the edge up to it.",
    ),
    tool(
        Area::Modelling,
        "sculpt",
        "**Sculpting** smooths, smudges, adds or removes height as you brush over a component. \
         Keys **1** to **5** switch brushes, and each stroke undoes on its own.",
    ),
    tool(
        Area::Modelling,
        "smooth_selected_components",
        "Model looks bumpy? **Smooth Selected Components** averages each height with its \
         neighbours: more **Passes** make it smoother.",
    ),
    tool(
        Area::Modelling,
        "emboss",
        "**Emboss Component** makes a model much lower while keeping its surface detail, turning \
         a deep 3D model into a shallow carving for thin material.",
    ),
    tool(
        Area::Modelling,
        "scale_model_z_height",
        "**Scale Model Z Height** stretches or squashes the whole model to a new height in one \
         step, every visible component together.",
    ),
    tool(
        Area::Modelling,
        "slice",
        "Model thicker than your board? **Slice Model** divides it into bands, each a component \
         on a level of its own, to carve from separate boards and glue up.",
    ),
    tool(
        Area::Modelling,
        "material_setup",
        "A 3D model is usually thinner than its board. **Material Setup**'s **Model Position in \
         Material** sets the gap above and below it: type one, and the other is what is left.",
    ),
    icon(
        Area::Modelling,
        "job_setup/material_size_xyz.png",
        "Not sure where to put the model in the board? In **Material Setup**, press **⬆ Top at \
         material top**: the least material is cut away, and it is the usual choice.",
        "material_setup",
    ),
    icon(
        Area::Safety,
        "slice",
        "A model drawn in red pokes out of the material: out of the bottom, the bit would cut \
         your spoilboard. Lower the model, or cut it into boards with **Slice Model**.",
        "material_setup",
    ),
    icon(
        Area::Modelling,
        "view/3d_view.png",
        "To change one model's height in the 3D view, double-click it, then double-click it \
         again: four round buttons appear. Drag one up to make it taller, down to make it lower.",
        "view_3d",
    ),
    icon(
        Area::Modelling,
        "toolpaths/recalculate_toolpath.png",
        "**Auto-rotate**, at the end of the row along the top of the 3D view, turns the job round \
         and round, so you can check every side of it.",
        "view_3d",
    ),
    icon(
        Area::Modelling,
        "zoom_to_job",
        "In the 3D view, press **F** or double-click off the job to fit it all in view. Click a \
         face of the cube in the corner to look from that side.",
        "view_3d",
    ),
    icon(
        Area::Modelling,
        "component_levels",
        "Rather type a model's heights than drag them? Right-click it in the 3D view and choose \
         **Component ▸ Properties**: **Shape Height** and **Base Height** are there.",
        "view_3d",
    ),
    tool(
        Area::Modelling,
        "replace_below",
        "**Replace Below** removes or flattens the parts of a component lower than a height you \
         set: a quick way to trim the base off an imported model.",
    ),
    tool(
        Area::Modelling,
        "split",
        "**Split Component** cuts a component in two with a selected closed vector, like a cookie \
         cutter: the inside is named \"- A\" and the outside \"- B\".",
    ),
    tool(
        Area::Modelling,
        "turn_and_spin",
        "**Turn and Spin** makes a lathe-turned shape or a dish from one open line: draw half the \
         outline, and it is swept round to make the rest.",
    ),
    tool(
        Area::Modelling,
        "two_rail_sweep",
        "**Two Rail Sweep** stretches a cross-section between two rails all the way along: \
         frames, handles and curved mouldings.",
    ),
    tool(
        Area::Modelling,
        "extrude_and_weave",
        "**Extrude and Weave** can send rails over and under each other where they cross, \
         alternately along each one, for woven and knotwork designs.",
    ),
    tool(
        Area::Modelling,
        "texture_area",
        "**Create Texture Area** tiles a component over a shape, with gaps, shifts and mirrored \
         tiles, for a repeating 3D texture such as scales or studs.",
    ),
    tool(
        Area::Modelling,
        "decorative_relief",
        "**Create Decorative Relief** fills a shape with carved 3D scrolls. Set **Finishing bit** \
         to the ball nose you will use, and nothing in it is finer than that bit can carve.",
    ),
    tool(
        Area::Modelling,
        "create_component_from_bitmap",
        "**Create Component from Bitmap** turns a picture's tones into heights, white high or \
         black high: a quick relief from a drawing or a depth map.",
    ),
    tool(
        Area::Modelling,
        "relief_from_photo",
        "In **Relief from Photo**, tick **Only what the bit can carve** to see the relief as your \
         bit will really cut it, with grooves too narrow for it filled in.",
    ),
    tool(
        Area::Modelling,
        "add_draft",
        "Making a mould? **Add Draft to Model** leans every wall steeper than the draft angle, so \
         the cast comes out cleanly.",
    ),
    tool(
        Area::Modelling,
        "export_as_stl",
        "**Export Model as STL** saves the 3D model for a 3D printer, so you can print a small \
         test before carving the real thing.",
    ),
    tool(
        Area::Modelling,
        "export_as_3d_clipart",
        "Made a 3D part you will want again? **Export Model as 3D Clipart** saves it, to drop \
         into other jobs from the Library.",
    ),
    tool(
        Area::Modelling,
        "reset_inside_selected_vectors",
        "**Reset Inside Selected Vectors** flattens the model back to its base inside a selected \
         shape: room cleared for a name or a logo.",
    ),
    tool(
        Area::Modelling,
        "job_setup",
        "**Job Setup**'s **Modeling Resolution** sets how finely 3D shapes are worked out. \
         Standard suits most work; High is about three times slower, Highest slower still.",
    ),
    icon(
        Area::Toolpaths,
        "toolpaths/preview_toolpaths.png",
        "The Toolpaths page runs in the order a job goes: **Create** a toolpath, **Preview** it, \
         **Analyse** it for problems, then **Output** the file your machine runs.",
        "toolpaths",
    ),
    icon(
        Area::Toolpaths,
        "toolpaths/side_outside.png",
        "In **Profile Toolpath**, **Outside** cuts a part out, **Inside** cuts a hole and **On \
         the Line** follows the line. Pick the wrong one and the part comes out a bit's width too \
         small or too big.",
        "profile_toolpath",
    ),
    tool(
        Area::Toolpaths,
        "profile_toolpath",
        "Profile's **Allowance Offset** leaves a part a little bigger (positive), to trim later \
         or sand to fit, or cuts into it (negative) for a looser fit.",
    ),
    icon(
        Area::Toolpaths,
        "toolpaths/side_inside.png",
        "Tick **Add tabs to toolpath** on a Profile: small bridges hold the part in place on the \
         last pass, so it can't come loose and be caught by the bit. Cut them off afterwards.",
        "profile_toolpath",
    ),
    icon(
        Area::Toolpaths,
        "diagrams/depth_profile.png",
        "Tab sizes can come from the bit: as thick as its pass depth (at most half the cut) and \
         twice as long, so a 1/4\" bit taking 1/4\" passes leaves 1/2\" x 1/4\" tabs.",
        "profile_toolpath",
    ),
    icon(
        Area::Toolpaths,
        "toolpaths/cut_climb_outside.png",
        "Want tabs in exact places? Set the tabs to **Where clicked**, then, with the form open, \
         click a vector in the 2D view to add a tab there, and click a tab to take it off.",
        "profile_toolpath",
    ),
    icon(
        Area::Toolpaths,
        "toolpaths/passes_one.png",
        "**3D Tabs** rise to the tab's thickness and down the other side without stopping, while \
         **2D Tabs** go up, across and down again.",
        "profile_toolpath",
    ),
    icon(
        Area::Toolpaths,
        "toolpaths/cut_conventional_outside.png",
        "A Profile always cuts the shapes inside others first, so a part stays held by the board \
         round it for as long as it can.",
        "profile_toolpath",
    ),
    icon(
        Area::Toolpaths,
        "toolpaths/motion_trochoidal.png",
        "Every Profile and Pocket has **Entry…**: the bit goes down in a **Helix**, a \
         **Zig-zag**, a **Ramp** or **Straight down**. Straight down is quickest but hardest on \
         the bit, and only for bits that cut at their centre.",
        "pocket_toolpath",
    ),
    icon(
        Area::Toolpaths,
        "toolpaths/loops_anticlockwise.png",
        "An entry fits the room it has: a helix gets smaller in a tight spot, then turns into a \
         zig-zag. Only where there is no room at all does it go straight down, and the checks \
         before cutting say where.",
        "pocket_toolpath",
    ),
    icon(
        Area::Toolpaths,
        "toolpaths/engraving.png",
        "A V-Carve never goes down in a helix, which would carve outside the groove: the bit \
         slides in along the groove's own centre line instead.",
        "vcarve_engrave_toolpath",
    ),
    tool(
        Area::Toolpaths,
        "pocket_toolpath",
        "**Pocket Toolpath** clears in **Offset** rings, **Raster** lines or **Adaptive**, which \
         keeps a thin, steady bite so the bit never cuts its full width in a corner.",
    ),
    icon(
        Area::Toolpaths,
        "diagrams/adaptive_channel.png",
        "With **Adaptive** clearing the bit takes only a thin bite everywhere, so it can go a \
         full pass deep with confidence: gentler on the bit and on the machine.",
        "pocket_toolpath",
    ),
    icon(
        Area::Toolpaths,
        "diagrams/light_step.png",
        "**Toolpaths ▸ Clearing Settings** sets adaptive clearing's **Light Step**, the bite it \
         takes. 5 to 15% of the bit suits most work.",
        "pocket_toolpath",
    ),
    icon(
        Area::Toolpaths,
        "toolpaths/passes_both.png",
        "**Clearing Settings ▸ On the Way Back**: **Keep climb** cuts every row climb, for a \
         cleaner wall; **Alternate** cuts back and forth, which is quicker.",
        "pocket_toolpath",
    ),
    icon(
        Area::Toolpaths,
        "diagrams/edge_pass.png",
        "Pocket's **Leave for Edge Pass** stops the clearing just short of the wall, then one \
         thin, even skim takes it off, for a straighter, cleaner wall.",
        "pocket_toolpath",
    ),
    icon(
        Area::Toolpaths,
        "toolpaths/rings_outline_in.png",
        "Tick Pocket's **Edge Pass at Full Depth** and the wall is finished in one pass at the \
         full depth, with no lines between passes, where the bit's flutes reach that deep.",
        "pocket_toolpath",
    ),
    tool(
        Area::Toolpaths,
        "vcarve_engrave_toolpath",
        "V-Carve's **Carve** setting: **Fill** carves the whole shape as deep as its width needs; \
         **Inside**, **On the Line** and **Outside** carve a groove of the depth you set.",
    ),
    icon(
        Area::Toolpaths,
        "diagrams/depth_vcarve.png",
        "Big letters V-carved deep take ages with a V-bit alone. Set a **Flat depth** and add an \
         end mill under **Clearance Tools**: it clears the flat bottom first, and the V-bit cuts \
         only what is left.",
        "vcarve_engrave_toolpath",
    ),
    tool(
        Area::Toolpaths,
        "vcarve_inlay_toolpath",
        "**V-Carve Inlay Toolpath** makes a pocket and a plug that fit together. The plug goes on \
         a sheet of its own, called Inlay plug, as it is cut from a second board.",
    ),
    icon(
        Area::Toolpaths,
        "diagrams/vcarve_inlay_fit.png",
        "In a V-Carve Inlay, the **Glue gap** is the room left between the top of the plug and \
         the bottom of the pocket once fitted.",
        "vcarve_inlay_toolpath",
    ),
    tool(
        Area::Toolpaths,
        "inlay_toolpath",
        "**Inlay Toolpath** rounds every corner to the bit's radius first, so the part's outside \
         corners match the cavity's, which a round bit can't cut sharp.",
    ),
    tool(
        Area::Toolpaths,
        "drilling_toolpath",
        "**Drilling Toolpath** always makes holes the bit's width, whatever size they are drawn, \
         and warns in red when a hole is much bigger: cut those with a Pocket or Profile instead.",
    ),
    icon(
        Area::Toolpaths,
        "diagrams/depth_drill.png",
        "**Use Peck Drilling** drills a pass depth at a time, rising between pecks, which suits \
         deep holes better than one long plunge.",
        "drilling_toolpath",
    ),
    tool(
        Area::Toolpaths,
        "chamfer_toolpath",
        "**Chamfer Toolpath** cuts a sloped edge with a V-bit's side: a 90° V-bit cuts a 45° \
         chamfer.",
    ),
    tool(
        Area::Toolpaths,
        "fluting_toolpath",
        "**Fluting Toolpath** ramps the depth along each line so the cut tapers in and out, for \
         decorative flutes on columns and panels.",
    ),
    tool(
        Area::Toolpaths,
        "prism_toolpath",
        "**Prism Carving** runs a V-bit round each shape deep enough to leave it standing as a \
         raised, ridged bevel, like chiselled letters.",
    ),
    tool(
        Area::Toolpaths,
        "surfacing_toolpath",
        "New spoilboard or a cupped board? **Surfacing Toolpath** flattens it with a wide bit, \
         always going down beside the work and feeding in from the side, as a surfacing bit must \
         never plunge.",
    ),
    tool(
        Area::Toolpaths,
        "quick_engrave_toolpath",
        "**Quick Engrave** (**Ctrl+Q**, except on a Mac) marks lines at a shallow depth, or fills \
         shapes with hatching: the quickest way to put a name or a mark on a part.",
    ),
    tool(
        Area::Toolpaths,
        "texture_toolpath",
        "**Texture Toolpath** cuts rows of shallow scallops of random depth and length for a \
         hand-tooled finish. Change its **Seed** for another random pattern.",
    ),
    tool(
        Area::Toolpaths,
        "keyhole_toolpath",
        "**Keyhole Toolpath** cuts hanging slots in the back of a sign with a keyhole cutter. \
         **Create preview vectors** draws how the slots will look.",
    ),
    tool(
        Area::Toolpaths,
        "thread_milling_toolpath",
        "**Thread Milling Toolpath** cuts a thread inside a hole or round a post. Tick **Create \
         Circles for Internal Threads** for circles to pocket the hole out first.",
    ),
    tool(
        Area::Toolpaths,
        "drag_knife_toolpath",
        "A drag knife needs a flat surface: make **Drag Knife Toolpath** the first toolpath, or \
         put it straight after a Surfacing toolpath.",
    ),
    tool(
        Area::Toolpaths,
        "3d_roughing_toolpath",
        "Carving a 3D model? Rough it first with **3D Roughing Toolpath**, which leaves an \
         **Allowance**, then finish with **3D Finishing Toolpath** and a ball nose.",
    ),
    tool(
        Area::Toolpaths,
        "3d_finishing_toolpath",
        "**3D Finishing Toolpath** can take several bits at once: each later bit cuts only what \
         the ones before it left (rest machining).",
    ),
    tool(
        Area::Toolpaths,
        "3d_pencil_toolpath",
        "After a 3D finish, **3D Pencil Toolpath** runs a smaller ball nose along only the \
         valleys and inside corners the big bit could not reach: minutes, not hours.",
    ),
    icon(
        Area::Toolpaths,
        "toolpaths/3d_finishing_toolpath.png",
        "For a 3D Pencil toolpath, pick a ball nose about half the size of the finishing bit: a 3 \
         mm ball after a 6 mm one, or 1/16\" after 1/8\".",
        "3d_pencil_toolpath",
    ),
    icon(
        Area::Toolpaths,
        "toolpaths/fluting_toolpath.png",
        "Profile, Pocket, V-Carve, Fluting and more can **Project toolpath onto 3D Model**, \
         cutting their depth below the model's surface: lettering on a dome, say.",
        "toolpaths",
    ),
    tool(
        Area::Toolpaths,
        "keep_out_zones",
        "Clamps on the board? **Keep-Out Zones** marks areas a tool must not pass over, such as \
         round each clamp. A toolpath that would cut into one is not made, unless Options let it \
         be clipped.",
    ),
    icon(
        Area::Toolpaths,
        "common/tool_database.png",
        "Renaming a bit in the **Tool Database** renames it in this job's toolpaths too, and in \
         other jobs when they open.",
        "tool_database",
    ),
    tool(
        Area::Toolpaths,
        "save_selected_toolpath_as_template",
        "Make the same cuts often? **Save Toolpath as Template** keeps a toolpath's settings and \
         bits, and **Load Toolpath Template** adds them to another job, ready to pick vectors and \
         calculate.",
    ),
    tool(
        Area::Toolpaths,
        "merged_toolpath",
        "**Merge Visible Toolpaths** joins toolpaths that use the same bit into one, in whichever \
         order leaves the least travelling between cuts.",
    ),
    tool(
        Area::Toolpaths,
        "array_copy_toolpath",
        "Cutting the same part many times? Make its toolpaths once, then **Array Copy Toolpath** \
         copies them in a grid, and recalculating copies them again.",
    ),
    tool(
        Area::Toolpaths,
        "toolpath_tiling",
        "Job bigger than your machine? **Toolpath Tiling** splits the toolpaths into tiles, a \
         file each, and says in red when a tile is bigger than the machine's bed.",
    ),
    tool(
        Area::Toolpaths,
        "laser_cut_fill_toolpath",
        "**Laser Cut / Fill** keeps the laser off while it comes down to the surface, so no dot \
         is burned where a line starts.",
    ),
    tool(
        Area::Toolpaths,
        "laser_picture_toolpath",
        "Tick **Dither image** in **Laser Picture** and the laser switches full on and off to \
         give a photo's tones: for materials that either burn or don't.",
    ),
    tool(
        Area::Toolpaths,
        "photo_vcarve_toolpath",
        "**Photo V-Carve Toolpath** carves a photo with nothing but a V-bit, as grooves that are \
         deeper, and so wider, where the picture is darker.",
    ),
    tool(
        Area::Toolpaths,
        "rounding_toolpath",
        "On a rotary job, **Rounding Toolpath** turns a square or round blank down to the job's \
         diameter first, ready for the carving.",
    ),
    tool(
        Area::Toolpaths,
        "rotary_machining_toolpath",
        "On a rotary job, **4-Axis Rotary Machining** finishes the model ring by ring all the \
         way round, worked out round the turning axis itself so a bit cutting deep never \
         catches the part beside it.",
    ),
    tool(
        Area::Toolpaths,
        "rotary_lathe_toolpath",
        "On a rotary job, **Rotary Lathe** spins the material like a lathe: face an end flat, \
         turn the part round to its outline, or part it off, leaving a small stub to hold it.",
    ),
    tool(
        Area::Cutting,
        "materials",
        "In the preview, each toolpath cuts in the colour of the swatch beside it in the toolpath \
         list, so you can see which toolpath cut what. **Toolpaths ▸ Preview Colors** sets how.",
    ),
    icon(
        Area::Cutting,
        "toolpaths/preview_toolpaths.png",
        "Want a preview straight away? Turn off **Toolpaths ▸ Toolpath Drawing ▸ Animate \
         Preview** and it shows all at once instead of cutting step by step.",
        "toolpaths",
    ),
    icon(
        Area::Cutting,
        "view/zoom_box.png",
        "Small, fine carving looks rough in the preview? **Toolpaths ▸ Toolpath Drawing ▸ Preview \
         Simulation Quality** draws it finer, from High (twice as slow) up to Maximum.",
        "toolpaths",
    ),
    icon(
        Area::Cutting,
        "view/2d_toolpath_drawing_on.png",
        "**Toolpaths ▸ Toolpath Drawing ▸ Draw Rapid Moves** shows the fast moves the bit makes \
         above the material between cuts, so you can see where it travels.",
        "toolpaths",
    ),
    icon(
        Area::Cutting,
        "view/3d_view.png",
        "**Create Component from Toolpath Preview**, on the Model menu, turns what the preview \
         cut into a 3D component, to carve on or use again.",
        "toolpaths",
    ),
    tool(
        Area::Cutting,
        "save_toolpaths",
        "**View G-code** in Save Toolpaths shows the file it would write, to read or copy, \
         without saving anything.",
    ),
    icon(
        Area::Cutting,
        "toolpaths/hide_toolpaths.png",
        "Save Toolpaths can put every shown (ticked) toolpath in one file, or each in a file of \
         its own: untick any you don't want in this file.",
        "save_toolpaths",
    ),
    icon(
        Area::Cutting,
        "job_setup/single_sided_job.png",
        "No tool changer? Save Toolpaths can write a file for each bit in turn, so you change \
         bits by hand between files: the safest way without one.",
        "save_toolpaths",
    ),
    icon(
        Area::Cutting,
        "file/open_folder.png",
        "Can this computer see your machine's card or shared folder? **Send to Machine** in Save \
         Toolpaths saves the files straight into it. Set the folder in **Machine ▸ Machine \
         Configuration**.",
        "save_toolpaths",
    ),
    icon(
        Area::Cutting,
        "common/open.png",
        "**Machine ▸ Machine Files** shows what is on the machine's card or shared folder, to \
         upload, rename or delete files without walking over to the machine.",
        "save_toolpaths",
    ),
    icon(
        Area::Cutting,
        "toolpaths/save_toolpaths.png",
        "Saving where an earlier save left G-code? It can replace the old files, numbered ones \
         too so none is run by mistake, or archive them. **Edit ▸ Options ▸ Confirmation \
         Choices** sets which.",
        "save_toolpaths",
    ),
    tool(
        Area::Cutting,
        "import_toolpaths_file",
        "Got a G-code file from somewhere else? **Import Toolpaths (G-code)** reads it back in, \
         so you can preview and check it before it runs.",
    ),
    icon(
        Area::Cutting,
        "toolpaths/recalculate_toolpath.png",
        "**Toolpaths ▸ Recalculate All Toolpaths** brings every out-of-date toolpath, marked ⚠ in \
         the list, up to date in one go.",
        "toolpaths",
    ),
    icon(
        Area::Cutting,
        "toolpaths/save_toolpaths.png",
        "Too many post-processors to find yours? **Machine ▸ Post-Processor Management** has \
         filters: tick **mm** or **inch**, **Arcs**, **Straight** or **Rotary (A)**, pick your \
         controller, or type part of a name. They start from your machine each time.",
        "post_processor_management",
    ),
    tool(
        Area::Cutting,
        "cut_list",
        "**Save Cut List** writes the job's parts and their sizes as a spreadsheet (CSV), to \
         check against your boards before you cut.",
    ),
    tool(
        Area::Cutting,
        "create_job_sheet",
        "**Gadgets ▸ Job Setup Sheet Editor** chooses what the job sheet shows: the job, its bits \
         and its toolpaths.",
    ),
    icon(
        Area::Program,
        "file/github_link.png",
        "An update is one click: **Install Now** downloads it, checks it is genuine, installs it \
         and reopens Cut Craft on your job, asking you to save first.",
        "updates",
    ),
    icon(
        Area::Program,
        "toolpaths/recalculate_toolpath.png",
        "**Skip This Version** stops Cut Craft offering that update when it starts. A newer one \
         is still offered, and **Help ▸ Check for Updates** still finds it.",
        "updates",
    ),
    icon(
        Area::Program,
        "common/save.png",
        "Updates never touch your jobs, settings, bits or machines, and nothing is downloaded \
         until you press **Install Now**.",
        "updates",
    ),
    icon(
        Area::Program,
        "view/interface_layout.png",
        "**Edit ▸ Options ▸ Web API** lets a script or program drive Cut Craft. It is off until \
         you turn it on, and every step it takes shows in the window and can be undone.",
        "web_api",
    ),
    icon(
        Area::Program,
        "file/notes.png",
        "With the Web API on, **Open Docs Page** in Options lists every call and tool, ready to \
         try in your web browser, and works without the internet.",
        "web_api",
    ),
    icon(
        Area::Program,
        "view/2d_view.png",
        "While the Web API is on, the status bar shows green **WEB API Running**, or red if it \
         could not start: point at it to see why.",
        "web_api",
    ),
    icon(
        Area::Program,
        "zoom_box",
        "Can't remember where a tool lives? Press **Ctrl+K** for **Find Tool**, type part of its \
         name, and press Enter to open the first match.",
        "find_tool",
    ),
    tool(
        Area::Program,
        "interface_layout",
        "**View ▸ Window Layout** offers Classic (Design and Toolpaths pages), Rail and \
         Inspector, or Ribbon: pick whichever suits your screen.",
    ),
    icon(
        Area::Program,
        "common/materials_appearance.png",
        "Find vectors and toolpaths hard to tell apart? **Edit ▸ Options ▸ Appearance** has \
         color-blind friendly colours, or pick your own.",
        "colors_on_screen",
    ),
    icon(
        Area::Program,
        "common/materials_appearance.png",
        "Changes in **Edit ▸ Options** wait for **Apply**, so you can try a setting and \
         **Cancel** it. **Close** asks first if anything is still waiting.",
        "settings",
    ),
    icon(
        Area::Program,
        "common/edit.png",
        "Hover over a greyed-out menu item or tool to see what it is waiting for, such as a \
         selection or a toolpath picked in the list.",
        "finding_help",
    ),
    icon(
        Area::Program,
        "zoom_selected",
        "**Search help**, along the top of the Help window, finds every page with your words: a \
         tool's name, a setting such as stepover, or what you want to do.",
        "finding_help",
    ),
    icon(
        Area::Program,
        "common/add_tool.png",
        "Every tool's help page shows the picture on its button, and the **?** lists show every \
         tool by its icon, so you can find a tool by how it looks.",
        "finding_help",
    ),
    icon(
        Area::Program,
        "pan",
        "Drag a guide line out of a ruler to line things up against. **View ▸ Guide Lines** \
         shows, locks or deletes them all at once.",
        "guide_lines",
    ),
    icon(
        Area::Program,
        "open",
        "Open a drawing or picture with Cut Craft from your file manager and it is brought into \
         a new, blank job: change the material's size with **Setup Material**.",
        "saving_jobs",
    ),
    icon(
        Area::Program,
        "file/open.png",
        "A job that won't open says why, and stays in **Recent Files** to try again. \"Damaged\" \
         usually means a copy or sync didn't finish: open a backup.",
        "saving_jobs",
    ),
    icon(
        Area::Program,
        "common/reset_undo.png",
        "Started dragging the wrong thing? Press **Esc** while still dragging, in the 2D or 3D \
         view, and the drag is cancelled with nothing changed.",
        "view_2d",
    ),
    icon(
        Area::Program,
        "toolpaths/motion_straight.png",
        "Drag sideways on a number to change it finely: each point you move changes it by a tenth \
         of a step, 0.01 mm or 0.001 in for a length.",
        "number_fields",
    ),
    icon(
        Area::Program,
        "diagrams/size_wh.png",
        "Number fields know units: in a millimetre job type `2*M` for two inches, and in an \
         inch job `3'` for three feet or `6*I` for 6 mm.",
        "number_fields",
    ),
    icon(
        Area::Program,
        "diagrams/arc_radius.png",
        "Need a circle's distance round? Number fields know **P** is π: type 2*P*50 for a 50 mm \
         radius.",
        "number_fields",
    ),
    icon(
        Area::Safety,
        "job_setup/z_zero_material_surface.png",
        "Zero Z where **Material Setup** says, on the material's top or on the machine bed. The \
         G-code's heights depend on it: the wrong one cuts too deep, or in the air.",
        "material_setup",
    ),
    icon(
        Area::Safety,
        "job_setup/clearance_z1.png",
        "Set **Rapid Z Gaps above Material** higher than your clamps and hold-downs, so the bit \
         clears them as it travels between cuts.",
        "job_setup",
    ),
    icon(
        Area::Safety,
        "job_setup/xy_datum_bottom_left.png",
        "**XY Datum Position** says which point of the material is X0, Y0 on the machine. Set \
         your machine's zero on that same point before you run the file.",
        "job_setup",
    ),
    tool(
        Area::Safety,
        "save_toolpaths",
        "Saving or sending toolpaths runs the **checks before cutting** first, and lists anything \
         that would stop or spoil the cut, each with how to put it right.",
    ),
    icon(
        Area::Safety,
        "toolpaths/print_job_sheet.png",
        "Before each toolpath runs, check the bit in the machine is the one the job sheet lists \
         for it: a different bit cuts a different size.",
        "print_job_sheet",
    ),
    tool(
        Area::Safety,
        "inlay_toolpath",
        "An inlay part or hole cut right through has no tabs to hold it: hold it down, or leave a \
         thin skin to cut off by hand, so it can't be thrown loose.",
    ),
    tool(
        Area::Safety,
        "keep_out_zones",
        "**View ▸ Draw Keep-Out Zones in 3D View** shows the areas round your clamps in 3D, so \
         you can see the cuts keep clear of them.",
    ),
    icon(
        Area::Safety,
        "diagrams/drag_knife_pressure.png",
        "**Edit ▸ Options ▸ Confirmation Choices ▸ Warn when material left is over** sets \
         when the checks warn that a bit's taper or shape will leave material on the model.",
        "confirmation_choices",
    ),
];

/// The tip after `n`, starting over after the last.
pub(super) fn next(n: usize) -> usize {
    (n + 1) % TIPS.len()
}

/// The tip before `n`, the last before the first.
pub(super) fn previous(n: usize) -> usize {
    (n + TIPS.len() - 1) % TIPS.len()
}

/// The Help window's page of every tip, by subject, each linked to its help page.
pub(super) fn markdown() -> String {
    let mut md = String::from(
        "[Contents](help:contents)\n\n# Did You Know?\n\nShort tips for getting more out of \
         Cut Craft. One is shown each time Cut Craft starts; turn that off in the tip's window \
         or in **Edit ▸ Options ▸ General**. The tip's window stays in front of Cut Craft \
         until you close it (see [Windows that open beside Cut Craft](help:contents)). Each \
         tip's **Tell me more** opens the help page about it.\n",
    );
    for area in Area::ALL {
        let _ = write!(md, "\n## {}\n\n", area.heading());
        for t in TIPS.iter().filter(|t| t.area == area) {
            let _ = writeln!(md, "- {} [Tell me more](help:{})", t.text, t.topic);
        }
    }
    md
}

/// What the tips window was closed or moved on with.
enum Press {
    Previous,
    Next,
    More(&'static str),
    Close,
}

impl CutCraftApp {
    /// The icon `id` stands for: a tool's button icon, else the icon at that path.
    pub(super) fn tip_icon(&self, id: &str) -> Option<&str> {
        self.icons
            .find(self.explicit_icon(id), id)
            .or_else(|| self.icons.find(id, ""))
    }

    /// The picture `tip` shows, if it is drawn.
    pub(super) fn tip_picture(&self, tip: &Tip) -> Option<&str> {
        match tip.picture {
            Picture::Scene(path) => self.icons.find(path, ""),
            Picture::Icon(id) => self.tip_icon(id),
        }
    }

    /// Help ▸ Did You Know?, or startup: the tips window on `n`.
    pub(super) fn open_tips(&mut self, n: usize) {
        self.tip = Some(n % TIPS.len());
    }

    /// At startup, unless turned off: the next tip, and the one after it next time.
    pub(super) fn tip_at_startup(&mut self) {
        if !self.options.general.show_tips_at_startup || self.first_run.is_some() {
            return;
        }
        let n = self.options.next_tip % TIPS.len();
        self.open_tips(n);
        self.options.next_tip = next(n);
        self.save_options();
    }

    /// First-time setup, then the tip, while open.
    pub(super) fn welcome_windows(&mut self, ctx: &egui::Context) {
        self.first_run_window(ctx);
        self.tips_window(ctx);
    }

    /// The tips window, while it is open.
    pub(super) fn tips_window(&mut self, ctx: &egui::Context) {
        let Some(n) = self.tip else { return };
        let mut open = true;
        let mut show = self.options.general.show_tips_at_startup;
        let mut press = None;
        let size = egui::vec2(28.0, 20.0) * ctx.global_style().spacing.interact_size.y;
        let tip = TIPS[n];
        let key = self.tip_picture(&tip).map(str::to_owned);
        let scene = matches!(tip.picture, Picture::Scene(_));
        let (cache, icons) = (&mut self.tips_cache, &self.icons);
        let id = egui::Id::unique("did_you_know");
        cut_craft_bug_report::dialog(ctx, id, "Did You Know?", size, &mut open, |ui| {
            ui.weak(tr!("tips-tip-of", number = n + 1, count = TIPS.len()));
            // the picture across the top, in a 2:1 space, then the words under it: a scene
            // fills the space; an icon is shown no bigger than it is drawn, in its middle
            let wide = ui
                .available_width()
                .min(22.0 * ui.spacing().interact_size.y);
            let picture = match &key {
                Some(key) => icons.image(key),
                None => egui::Image::from_bytes("bytes://splash-logo.jpg", SPLASH_LOGO),
            };
            let high = if scene || key.is_none() {
                wide / 2.0
            } else {
                (wide / 2.0).min(7.0 * ui.spacing().interact_size.y)
            };
            let gap = (wide / 2.0 - high) / 2.0;
            ui.add_space(gap);
            ui.vertical_centered(|ui| {
                ui.add(picture.fit_to_exact_size(egui::vec2(wide, high)));
            });
            ui.add_space(gap);
            ui.add_space(ui.spacing().item_spacing.y);
            egui_commonmark::CommonMarkViewer::new().show(ui, cache, tip.text);
            ui.add_space(ui.spacing().item_spacing.y);
            ui.horizontal(|ui| {
                if ui.button(tr!("tips-previous")).clicked() {
                    press = Some(Press::Previous);
                }
                if ui.button(tr!("tips-next")).clicked() {
                    press = Some(Press::Next);
                }
                if ui.button(tr!("tips-tell-me-more")).clicked() {
                    press = Some(Press::More(tip.topic));
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.checkbox(&mut show, tr!("tips-show-a-tip-when-cut"));
                if ui.button(tr!("tips-close")).clicked() {
                    press = Some(Press::Close);
                }
            });
        });
        if show != self.options.general.show_tips_at_startup {
            self.options.general.show_tips_at_startup = show;
            self.save_options();
        }
        self.tip = match press {
            _ if !open => None,
            Some(Press::Previous) => Some(previous(n)),
            Some(Press::Next) => Some(next(n)),
            Some(Press::More(topic)) => {
                self.info = Some(Info::Help(topic));
                None
            }
            Some(Press::Close) => None,
            None => Some(n),
        };
    }
}
