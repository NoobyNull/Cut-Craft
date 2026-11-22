//! The side panels and windows: Tool Database, layers, sheets, clipart, the component tree,
//! playback and the 3D view.

use super::RING_SHARE;
use super::{
    Align, Anchor, Change, Cow, CutCraftApp, EXPORT_LEVEL, EntityId, FineKey, HEIGHT_DRAG_EVERY,
    Handle, Handle3d, LayerId, Layout, MESH_CELLS, MODELING_PLANE_ALPHA, MODELING_PLANE_MARGIN,
    Material, Menus, PLAYBACK_SPEEDS, Page, Point, Screen, Settings3d, TOOL_BOX_MARGIN,
    TOOL_ICON_ROWS, TOOL_SHANK, VIEW_ZOOM_QUARTERS, Value, View, ViewTool, egui, file_name, files,
    gui, handles, menu_items, sheet_length, view2d, view3d, wrap_for,
};
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_bug_report::number::Number;
use cut_craft_core::document::{MachinedArea, Units};
use cut_craft_i18n::tr;

mod components;
mod handles_3d;
mod layers;
mod paint_3d;
mod playback;
mod toolpaths_page;
mod tools;
mod view_3d;

/// The push-pin's two drawings (`scripts/make_icons.py`).
pub(super) const PIN_ICON: &str = "view/pin.png";
pub(super) const PINNED_ICON: &str = "view/pinned.png";
/// How many times a button's height the 3D view's height buttons are: big enough to tell
/// their pictures apart at a glance.
pub(super) const HEIGHT_BUTTON: f32 = 1.75;

/// What tr!("panels-by-rule") beside a toolpath in the list means.
pub(super) const FOLLOWS_RULE_HINT: &str = "This toolpath follows a Vector Selector: each time it is calculated it takes the vectors \
     that pass its filters (open or closed, circles, layers), so vectors added later are cut \
     too once it is recalculated";

/// What the selection count in the status bar and on toolpath forms means.
pub(super) const SELECTION_COUNT_HINT: &str = "A closed vector goes all the way round and ends where it starts, so it has an inside to \
     pocket or carve. An open vector is a line with two loose ends: Pocket, V-Carve and the \
     other area toolpaths leave it out. Join open ends with Join Open Ends, and see them with \
     View ▸ Show Open Vector Ends";

/// Each toolpath's place in the cut order.
type Places = std::collections::BTreeMap<EntityId, usize>;

/// A toolpath's row in the list: its tick, its preview swatch (given its place and the
/// material's colour, when the preview takes toolpath colours) and its name, indented in a
/// group, marked ⚠ in red with what its collet or spindle hits (`collided`) when the last
/// preview found that. The name's response, the tick's new state if changed, and the colour
/// picked if any.
fn toolpath_row(
    ui: &mut egui::Ui,
    toolpath: &cut_craft_core::toolpath::Toolpath,
    (selected, collided): (bool, Option<&str>),
    swatch: Option<(usize, [u8; 3])>,
    indented: bool,
) -> (egui::Response, Option<bool>, Option<[u8; 3]>) {
    let (mut ticked, mut rgb) = (None, None);
    let label = ui
        .horizontal(|ui| {
            if indented {
                ui.add_space(ui.spacing().indent);
            }
            let mut visible = toolpath.visible;
            if ui
                .checkbox(&mut visible, "")
                .on_hover_hint(tr!("panels-show-or-hide-this-toolpath"))
                .changed()
            {
                ticked = Some(visible);
            }
            if let Some((place, material)) = swatch {
                rgb = preview_swatch(ui, toolpath, place, material);
            }
            let row = if let Some(collided) = collided {
                let name = egui::RichText::new(format!("⚠ {}", toolpath.name))
                    .color(ui.visuals().error_fg_color);
                let stale = if toolpath.stale {
                    "\nIt is also out of date: recalculate it."
                } else {
                    ""
                };
                ui.selectable_label(selected, name)
                    .on_hover_hint(format!("{}\n{collided}{stale}", toolpath.tool.name))
            } else if toolpath.stale {
                let name = egui::RichText::new(format!("⚠ {}", toolpath.name))
                    .color(ui.visuals().warn_fg_color);
                ui.selectable_label(selected, name).on_hover_hint(tr!(
                    "panels-toolpath-out-of-date",
                    tool = toolpath.tool.name.as_str()
                ))
            } else {
                ui.selectable_label(selected, &toolpath.name)
                    .on_hover_hint(&toolpath.tool.name)
            };
            // made from whatever passes its Vector Selector, not a fixed set of vectors
            if cut_craft_core::selector::Selector::of(&toolpath.params).is_some() {
                ui.weak(tr!("panels-by-rule"))
                    .on_hover_hint(FOLLOWS_RULE_HINT);
            }
            row
        })
        .inner;
    (label, ticked, rgb)
}

/// The part of `picture` inside `area` (in its pixels) saved as a PNG at `path`.
fn save_part(
    picture: &egui::ColorImage,
    area: egui::Rect,
    path: &std::path::Path,
) -> Result<(), String> {
    part_of(picture, area, "the 3D view")?
        .save(path)
        .map_err(|e| crate::messages::could_not_save_picture(path, &e))
}

/// The `area` (pixels) of the window's `picture`: `what` is the view it should show, named
/// when it was not on screen.
pub(super) fn part_of(
    picture: &egui::ColorImage,
    area: egui::Rect,
    what: &str,
) -> Result<image::RgbaImage, String> {
    let [width, height] = picture.size;
    let clamp = |v: f32, most: usize| {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "pixels"
        )]
        let v = v.round().max(0.0) as usize;
        v.min(most)
    };
    let (x0, y0) = (clamp(area.min.x, width), clamp(area.min.y, height));
    let (x1, y1) = (clamp(area.max.x, width), clamp(area.max.y, height));
    if x1 <= x0 || y1 <= y0 {
        return Err(format!("{what} was not on screen to save"));
    }
    let mut out = image::RgbaImage::new(
        u32::try_from(x1 - x0).map_err(|e| e.to_string())?,
        u32::try_from(y1 - y0).map_err(|e| e.to_string())?,
    );
    for (y, row) in (y0..y1).enumerate() {
        for (x, column) in (x0..x1).enumerate() {
            let c = picture.pixels[row * width + column];
            let (x, y) = (u32::try_from(x).unwrap_or(0), u32::try_from(y).unwrap_or(0));
            out.put_pixel(x, y, image::Rgba(c.to_srgba_unmultiplied()));
        }
    }
    Ok(out)
}

/// Whether `path` is a mesh, imported through Orientate 3D Model.
/// Asks before Delete from Disk, naming what goes; whether to go ahead.
fn confirm_delete(doomed: &[std::path::PathBuf]) -> bool {
    const LISTED: usize = 8;
    let mut names: Vec<String> = (doomed.iter().take(LISTED))
        .map(|f| format!("  {}", file_name(f)))
        .collect();
    if doomed.len() > LISTED {
        names.push(format!("  and {} more", doomed.len() - LISTED));
    }
    let what = if doomed.len() == 1 {
        "this file".to_owned()
    } else {
        format!("these {} files", doomed.len())
    };
    let answer = super::message_dialog()
        .set_title("Delete from Disk")
        .set_description(format!(
            "Delete {what} from your disk?\n\n{}\n\nThis can't be undone: they don't go \
             to the bin.",
            names.join("\n")
        ))
        .set_level(rfd::MessageLevel::Warning)
        .set_buttons(rfd::MessageButtons::OkCancelCustom(
            "Delete".into(),
            "Cancel".into(),
        ))
        .show();
    matches!(answer, rfd::MessageDialogResult::Custom(c) if c == "Delete")
}

fn is_model(path: &std::path::Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("stl") || e.eq_ignore_ascii_case("obj"))
        // a generated border: its depth map, read as a model at its recipe's size
        || cut_craft_border_tile::of_depth_map(path).is_some()
}

/// `change` (an import) with what it adds moved to be centred on `at`.
fn centred_on(mut change: Change, at: cut_craft_core::geometry::Point) -> Change {
    use cut_craft_core::Edit;
    use cut_craft_core::geometry::Rect;
    let boxes = change.edits.iter().filter_map(|e| match e {
        Edit::Add { path, .. } => Some(path.bounds()),
        Edit::AddComponent { component, .. } => Some(component.extent()),
        Edit::AddBitmap { bitmap, .. } => Some(bitmap.rect),
        _ => None,
    });
    let Some(all) = boxes.reduce(Rect::union) else {
        return change;
    };
    let by = at - all.center();
    let t = cut_craft_core::glam::DAffine2::from_translation(by);
    for edit in &mut change.edits {
        match edit {
            Edit::Add { path, .. } => *path = path.transformed(&t),
            Edit::AddComponent { component, .. } => **component = component.transformed(&t),
            Edit::AddBitmap { bitmap, .. } => {
                bitmap.rect = Rect {
                    min: bitmap.rect.min + by,
                    max: bitmap.rect.max + by,
                };
            }
            _ => {}
        }
    }
    change
}

/// A toolpath's colour in a Toolpath Color preview, as a swatch beside it in the toolpath
/// list (the material's `colour` when it has no fill); the colour picked, if one was.
fn preview_swatch(
    ui: &mut egui::Ui,
    toolpath: &cut_craft_core::toolpath::Toolpath,
    place: usize,
    colour: [u8; 3],
) -> Option<[u8; 3]> {
    let given = toolpath.preview_colour(place);
    let mut rgb = given.unwrap_or(colour);
    let tip = if given.is_some() {
        "Its colour in the preview (right-click it for No Fill)"
    } else {
        "No Fill: previewed in the material's colour"
    };
    // ponytail: a drag in the picker is an edit a frame; coalesce them if the undo list
    // gets long
    ui.color_edit_button_srgb(&mut rgb)
        .on_hover_hint(tip)
        .changed()
        .then_some(rgb)
}

/// The modeling plane as a sheet of light blue (the zero plane a model is placed on), a
/// little past the material on every side so it shows under the model.
fn modeling_plane(
    ui: &egui::Ui,
    screen: &dyn Fn(cut_craft_core::glam::DVec3) -> egui::Pos2,
    lo: cut_craft_core::glam::DVec3,
    hi: cut_craft_core::glam::DVec3,
) -> egui::Mesh {
    use cut_craft_core::glam::DVec3;
    let margin = (hi - lo).max_element() * MODELING_PLANE_MARGIN;
    let (lo, hi) = (lo - DVec3::splat(margin), hi + DVec3::splat(margin));
    let fill = ui
        .visuals()
        .selection
        .bg_fill
        .gamma_multiply(MODELING_PLANE_ALPHA);
    let mut mesh = egui::Mesh::default();
    for corner in [
        DVec3::new(lo.x, lo.y, 0.0),
        DVec3::new(hi.x, lo.y, 0.0),
        DVec3::new(hi.x, hi.y, 0.0),
        DVec3::new(lo.x, hi.y, 0.0),
    ] {
        mesh.colored_vertex(screen(corner), fill);
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    mesh
}

/// The combine modes as the small shape icons, the chosen one pressed and the names on
/// hover; where each is drawn goes to `s.radios`.
fn combine_radios_3d(ui: &mut egui::Ui, icons: &crate::icons::Icons, s: &mut Settings3d) {
    let edge = ui.spacing().interact_size.y;
    s.radios.clear();
    ui.horizontal(|ui| {
        for (name, mode, icon) in gui::COMBINE_MODES {
            let image = icons.image(icon).fit_to_exact_size(egui::Vec2::splat(edge));
            let radio = egui::Button::image(image)
                .selected(s.params.combine == mode)
                .min_size(egui::Vec2::splat(edge));
            let response = ui.add(radio).on_hover_hint(name);
            if response.clicked() {
                s.params.combine = mode;
                s.dirty = true;
            }
            s.radios.push((mode, response.rect.center()));
        }
    });
}

/// A flyout button's arrow: a small filled triangle in its bottom-right corner.
fn flyout_arrow(ui: &egui::Ui, response: &egui::Response) {
    let r = response.rect;
    let side = r.width().min(r.height()) * crate::flyout::ARROW_SHARE * 0.6;
    let corner = r.right_bottom() - egui::vec2(3.0, 3.0);
    let colour = ui.style().interact(response).fg_stroke.color;
    ui.painter().add(egui::Shape::convex_polygon(
        vec![
            corner,
            corner - egui::vec2(0.0, side),
            corner - egui::vec2(side, 0.0),
        ],
        colour,
        egui::Stroke::NONE,
    ));
}

/// The level plane the 3D view's transform handles sit on, at height `z` in the material.
struct Plane3d<'a> {
    camera: &'a view3d::Camera,
    origin: Point,
    z: f64,
}

impl Plane3d<'_> {
    /// Where the job's point `p` on this plane shows.
    fn screen(&self, p: Point) -> egui::Pos2 {
        use cut_craft_core::glam::DVec3;
        self.camera
            .screen(DVec3::new(p.x - self.origin.x, p.y - self.origin.y, self.z))
    }

    /// The job's point on this plane under `at` on screen, if the plane is there.
    fn back(&self, at: egui::Pos2) -> Option<Point> {
        self.camera.on_plane(at, self.z).map(|p| p + self.origin)
    }
}

/// The highest surface over `bounds` (sampled on a 9 by 9 grid), or the material's top
/// when that is higher or the surface is not there.
fn top_over(surface: &Material, bounds: cut_craft_core::geometry::Rect) -> f64 {
    const SAMPLES: u32 = 9;
    let size = bounds.size();
    (0..SAMPLES)
        .flat_map(|i| (0..SAMPLES).map(move |j| (i, j)))
        .filter_map(|(i, j)| {
            let along = |k: u32| f64::from(k) / f64::from(SAMPLES - 1);
            surface.at(bounds.min + Point::new(size.x * along(i), size.y * along(j)))
        })
        .fold(0.0, f64::max)
}

/// The corners of `bounds`, round from its minimum.
fn box_corners(bounds: cut_craft_core::geometry::Rect) -> [Point; 4] {
    [
        bounds.min,
        Point::new(bounds.max.x, bounds.min.y),
        bounds.max,
        Point::new(bounds.min.x, bounds.max.y),
    ]
}

/// `faces` on the GPU, made once and `kept` while the faces stay the same.
fn faces_geometry(
    kept: &mut Option<super::ModelGpu>,
    faces: &std::sync::Arc<Vec<cmd_orientate_3d_model::Seen>>,
) -> std::sync::Arc<view3d::gpu::Geometry> {
    if let Some(kept) = kept
        && std::sync::Arc::ptr_eq(&kept.model, faces)
    {
        return std::sync::Arc::clone(&kept.geometry);
    }
    let geometry = std::sync::Arc::new(view3d::gpu::model(faces));
    *kept = Some(super::ModelGpu {
        model: std::sync::Arc::clone(faces),
        geometry: std::sync::Arc::clone(&geometry),
    });
    geometry
}

/// View G-code: what saving would write, shown in a window (Output tab, Save Toolpaths).
pub(super) const VIEW_GCODE: &str = "View G-code…";
pub(super) const VIEW_GCODE_ABOUT: &str = "Read the G-code saving would write, through the \
     machine's post-processor, without saving a file. Copy it from there if you need it";

/// The Save Toolpaths button under the toolpath list.
pub(super) const SAVE_FOR_MACHINE: &str = "Save Toolpaths for the Machine…";

/// Under it while there is nothing to save.
pub(super) const SAVE_NEEDS: &str = "Make a toolpath first, then save it here";

/// The share of the Toolpaths page the toolpath list takes until its divider is dragged.
const LIST_SHARE: f32 = 0.4;

/// The stock in the way of the collet, spindle or shank, in the preview: a strong red, so it
/// stands out on any material or toolpath colour.
pub(super) const COLLISION_RED: egui::Color32 = egui::Color32::from_rgb(230, 30, 30);

/// The toolpath list's actions in its More… menu (Recalculate All only while none is out of
/// date, when it is a button of its own).
const MORE_LIST_ACTIONS: [&str; 6] = [
    "preview_toolpath",
    "preview_visible_toolpaths",
    "undo_last_preview",
    "recalculate_toolpath",
    "recalculate_all_toolpaths",
    "delete_toolpath",
];

/// A numbered step of making a job on the Toolpaths page: the number in a filled circle,
/// then what to do.
fn step_heading(ui: &mut egui::Ui, n: u8, text: &str) {
    ui.horizontal(|ui| {
        let row = ui.spacing().interact_size.y;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(row, row), egui::Sense::hover());
        let visuals = ui.visuals();
        ui.painter()
            .circle_filled(rect.center(), row * 0.45, visuals.selection.bg_fill);
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            n.to_string(),
            egui::FontId::proportional(row * 0.6),
            visuals.strong_text_color(),
        );
        ui.strong(text);
    });
}

/// The line under a preview's controls: where the tool is being sent and how far it has
/// got, else how long the rest takes at this speed, with how to stop.
fn playback_note(
    at: f64,
    length: f64,
    goal: Option<f64>,
    playing: bool,
    kept: bool,
    speed: f64,
) -> String {
    let percent = |d: f64| {
        if length > 0.0 {
            d / length * 100.0
        } else {
            100.0
        }
    };
    match goal {
        Some(goal) => format!(
            "Cutting up to {:.0}%: {:.0}% so far. Stop (Esc) stops it here",
            percent(goal),
            percent(at)
        ),
        None if playing => {
            let left = (length - at).max(0.0);
            let watch = if (speed - 1.0).abs() < 1e-9 {
                String::new()
            } else {
                format!(
                    ", {} at ×{}",
                    crate::busy::seconds(left / speed),
                    times(speed)
                )
            };
            format!(
                "Machine time left: about {}{watch}. Speed it up, drag Progress on, or Stop \
                 (Esc) to keep what is cut",
                crate::busy::seconds(left)
            )
        }
        None if kept && at >= length => {
            "Played through. Drag Progress or Close up to look back over it; Run plays it again"
                .to_owned()
        }
        None if kept => format!(
            "Stopped at {:.0}%. Drag Progress or Close up to look again; Run goes on",
            percent(at)
        ),
        None => "Paused. Run goes on; Stop (Esc) keeps what is cut".to_owned(),
    }
}

/// How far either side of the tool the Close up slider reaches, seconds of machine time.
const CLOSE_UP: f64 = 120.0;

/// A speed multiplier as the slider shows it: 0.25, 0.5, 2, 10, 250.
fn times(x: f64) -> String {
    if x < 1.0 {
        format!("{x:.2}").trim_end_matches('0').to_owned()
    } else {
        format!("{x:.0}")
    }
}

/// The status bar's word on a bit picked with Select…, by what it was picked for.
fn picked_message(picking: Option<super::PickFor>, name: &str) -> String {
    match picking {
        Some(super::PickFor::Listed(super::ToolList::Clearing, _)) => {
            format!("Clearing with {name}")
        }
        Some(super::PickFor::Listed(super::ToolList::Rest, _)) => {
            format!("Rest machining after {name}")
        }
        Some(super::PickFor::Listed(super::ToolList::Before, _)) => {
            format!("Pencil cuts only what {name} left")
        }
        Some(super::PickFor::Holder(slot)) => format!("{name} loaded in holder T{slot}"),
        Some(super::PickFor::Listed(super::ToolList::Tools, _) | super::PickFor::Material)
        | None => format!("Cutting with {name}"),
    }
}

/// The room a tool button `edge` across takes in a row, whatever state it is in.
pub(super) fn tool_cell_size(spacing: &egui::style::Spacing, edge: f32) -> egui::Vec2 {
    let pad = spacing.button_padding;
    egui::vec2(
        edge + 2.0 * pad.x,
        (edge + 2.0 * pad.y).max(spacing.interact_size.y),
    )
}

/// Draws a tool button with `add` in a cell of exactly [`tool_cell_size`], so the row around
/// it never moves. A hovered or pressed button is drawn a fraction of a point bigger (its
/// border thickens by the theme's line scale, which egui's whole-point margins don't take
/// back); laid out as it is drawn, that grew the row under the pointer, sent a full row's
/// last button to the next row, and the page jumped between two layouts while it was hovered.
fn tool_cell<R>(ui: &mut egui::Ui, edge: f32, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let (_, rect) = ui.allocate_space(tool_cell_size(ui.spacing(), edge));
    let mut cell = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(Layout::left_to_right(Align::Center)),
    );
    add(&mut cell)
}

/// The [`cut_craft_core::eta`] history kind for importing `path`: by its sort of file, as
/// an STL takes far longer than a DXF.
pub(super) fn importing(path: &std::path::Path) -> String {
    let sort = path
        .extension()
        .map_or_default(|e| e.to_string_lossy().to_ascii_lowercase());
    format!("import:{sort}")
}

/// The height of the 3D view's origin arrows: the machine's Z zero, on the material's top or
/// (Job Setup ▸ Z Zero Position ▸ Machine Bed) on the bed under it, a height from the
/// material's top at 0. While a model is being `imported` the frame has the bed at 0 and the
/// top a thickness up, as the material's outline is drawn there.
pub(super) fn origin_3d(job: &cut_craft_core::document::Job, imported: bool) -> f64 {
    job.zero_height() + if imported { job.thickness } else { 0.0 }
}

/// One of the 3D view's origin arrows, `width` wide: its `runs` in sight on screen, in its
/// axis's colour, with an arrow head and its letter at its + end where that end is in sight.
fn origin_arrow(
    ui: &egui::Ui,
    painter: &egui::Painter,
    runs: Vec<Vec<egui::Pos2>>,
    (from, to): (egui::Pos2, egui::Pos2),
    axis: crate::axes::Axis,
    width: f32,
) {
    let colour = axis.colour();
    let head = ui.spacing().interact_size.y / 2.0;
    let font = &egui::TextStyle::Body.resolve(ui.style());
    let end_seen = runs
        .last()
        .and_then(|run| run.last())
        .is_some_and(|end| end.distance(to) < 1.0);
    // faint all the way where the material hides it (all of X and Y on the bed under a centre
    // datum), bold where it is in sight
    painter.line_segment([from, to], (width / 2.0, colour.gamma_multiply(0.5)));
    for run in runs {
        painter.add(egui::Shape::line(run, (width, colour)));
    }
    if end_seen {
        crate::axes::head(painter, to, to - from, head, colour);
        let out = (to - from).normalized() * head;
        crate::axes::tag(painter, to + out, axis.letter(), colour, font);
    }
}

/// A rotation ring's arrows: a short stretch of the ring round its point `outer` (where it
/// stands out most from its middle `centre`, clear of the other rings' arrows), thicker, with an arrow head at each end, the + end marked `X+` and the
/// other `X−` (for `name` X), so which way a drag turns it reads at a glance. `points` run
/// the + way round; `look` is the ring's stroke and the arrow heads' size.
fn ring_arrows(
    painter: &egui::Painter,
    points: &[egui::Pos2],
    outer: usize,
    centre: egui::Pos2,
    look: (egui::Stroke, f32),
    font: &egui::FontId,
    name: &str,
) {
    let (stroke, head) = look;
    // the last point is the first again
    let n = points.len().saturating_sub(1);
    let reach = (n / 18).max(1);
    if n == 0 {
        return;
    }
    // the point `k` along the ring from `reach` before the mark, round past the seam
    let at = |k: usize| points[(outer + n - reach % n + k) % n];
    let stretch: Vec<egui::Pos2> = (0..=2 * reach).map(at).collect();
    let thick = egui::Stroke::new(stroke.width * 2.0, stroke.color);
    painter.add(egui::Shape::line(stretch, thick));
    // the + end, then the − end, each with the point before it on the way out
    let ends = [(2 * reach, 2 * reach - 1, '+'), (0, 1, '\u{2212}')];
    for (k, inner, sign) in ends {
        let (tip, before) = (at(k), at(inner));
        crate::axes::head(painter, tip, tip - before, head, stroke.color);
        let out =
            (tip - centre).normalized() * head * 1.5 + (tip - before).normalized() * head * 2.0;
        crate::axes::tag(
            painter,
            tip + out,
            &format!("{name}{sign}"),
            stroke.color,
            font,
        );
    }
}

/// How far apart the rings' arrows are kept, in arrow heads.
const RING_MARK_GAP: f32 = 9.0;

/// Whether a component is a full 3D model kept as it really is (a rotary job's), raised onto
/// or merged with what is under it: drawn and cut from its real shape, not its heights.
fn real_3d(k: &cut_craft_core::component::Component) -> bool {
    !k.solid.is_empty()
        && matches!(
            k.combine,
            cut_craft_core::component::Combine::Add | cut_craft_core::component::Combine::Merge
        )
}
