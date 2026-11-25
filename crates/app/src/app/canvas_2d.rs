//! The 2D view itself: painting it, the empty sheet's hint, what clicks and drags act on, zoom box,
//! form marks, the datum and guide lines.

use super::views::{
    EMPTY_SHEET_HINT, EMPTY_SHEET_RECENT, EMPTY_SHEET_SAMPLE, EMPTY_SHEET_TITLE, Modes,
};
use super::{
    Align, Canvas, Change, CutCraftApp, Drag, Gesture, Grab, Overlay, Path, Point, Rect, ViewTool,
    ViewTransform, egui, measured, view2d,
};
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_bug_report::number::Number;
use cut_craft_i18n::tr;

impl CutCraftApp {
    /// The 2D view's drawing, and the snap grid's points while snapping to it.
    fn paint_2d(&self, ui: &egui::Ui, canvas: &view2d::Canvas, overlay: &view2d::Overlay) {
        let look = &self.options.appearance;
        let dark = ui.visuals().dark_mode;
        let colours = look.canvas(dark);
        canvas.paint_sheet(ui, colours.sheet_color);
        // Join Open Ends marks the ends it joins, whether or not the view shows them
        let display = crate::display::Display {
            open_ends: self.display.open_ends || self.joining_ends(),
            ..self.display
        };
        canvas.paint(
            ui,
            &self.doc,
            overlay,
            self.selected_toolpath,
            &display,
            (self.job_epoch, self.history.revision()),
            &look.lines(dark),
        );
        if self.options.snapping.snap_to_grid {
            canvas.paint_grid(
                ui,
                self.options.snapping.grid_spacing,
                colours.grid_color,
                look.grid_point_size,
            );
        }
    }

    /// The 2D view moved on screen (a split's divider, a corner picture): the zoom and pan
    /// move with it, so the drawing stays where it was in the view.
    fn follow_view_2d(&mut self, ui: &egui::Ui) {
        let at = ui.available_rect_before_wrap().min;
        if let (Some(was), Some(camera)) = (self.view_2d_at, self.camera.as_mut()) {
            camera.origin += at - was;
        }
        self.view_2d_at = Some(at);
    }

    pub(super) fn view_2d(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "2D view");
        self.forget_gone_group();
        let job = self.shown_job();
        self.follow_view_2d(ui);
        let mut canvas = Canvas::new(ui, &job, self.camera, self.wrap());
        self.clipart_dropped_2d(ui, &canvas);
        let area = canvas.area;
        self.zoom_and_pan(ui, &mut canvas);
        let reach = canvas.pick_tolerance(ui);
        let gesture = canvas.gesture(ui);
        // a guide being dragged takes the drag over
        let guided = self.guide_gesture(ui, &canvas, reach);
        let gesture = gesture.filter(|_| !guided);
        let (gesture, snap, placed) =
            self.snapping(ui, gesture, canvas.transform.scale, canvas.cursor());
        self.place_on_snaps(gesture, &placed);
        // drawing: a crosshair, to place points by
        if self.drawing_tool().is_some() && canvas.hover().is_some() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        }
        self.track_stroke(gesture);
        let modes = self.modes();
        let Modes {
            handled,
            tracing,
            filleting,
            placing_tabs,
            joining_ends,
        } = modes;
        let overlay = match (gesture, self.view_tool, self.drawing_tool()) {
            // Trace Bitmap: a box dragged over the picture is the part traced
            (Some(Gesture::Dragging(drag)), None, None) if tracing => Overlay::Window {
                start: drag.start,
                end: drag.end,
                crossing: true,
            },
            // Create Fillets: the fillet the corner under the pointer would get, or the
            // fillet a click there would take out
            (None | Some(Gesture::Click { .. }), None, None) if filleting => {
                self.fillet_overlay(canvas.cursor(), reach)
            }
            (None | Some(Gesture::Click { .. }), None, None) if placing_tabs => {
                self.tab_overlay(reach)
            }
            // Join Open Ends: the end picked first, and the join a click would make
            (None | Some(Gesture::Click { .. }), None, None) if joining_ends => {
                self.join_ends_overlay(canvas.cursor(), reach)
            }
            (_, Some(ViewTool::NodeEdit), _) => self.node_overlay(gesture, reach),
            (Some(Gesture::Dragging(drag)), Some(ViewTool::Measure), _) => {
                self.status
                    .set_message(&measured(drag.start, drag.end, self.doc.job.units));
                Overlay::Preview(vec![Path::polyline(&[drag.start, drag.end], false)])
            }
            (Some(Gesture::Dragging(drag)), Some(ViewTool::ZoomBox), _) => Overlay::Window {
                start: drag.start,
                end: drag.end,
                crossing: false,
            },
            (_, Some(ViewTool::Pan), _) => Overlay::None,
            (Some(Gesture::Dragging(drag)), Some(ViewTool::Transform), _) => {
                self.transform_overlay(ui, drag, reach)
            }
            (gesture, None, Some("draw_polyline")) => Overlay::Preview(
                self.polyline_overlay(gesture, snap.map(|s| s.at).or(canvas.cursor())),
            ),
            (Some(Gesture::Dragging(drag)), _, Some(id)) => {
                Overlay::Preview(match self.text_arc_drag(drag, reach) {
                    Some(bent) => self.text_preview(&bent),
                    None => self.preview(id, drag),
                })
            }
            // select mode: a drag from a scale or rotate handle transforms, as in Transform
            (Some(Gesture::Dragging(drag)), None, None)
                if handled && self.on_handle(drag.start, reach) =>
            {
                self.transform_overlay(ui, drag, reach)
            }
            (Some(Gesture::Dragging(drag)), _, None) => {
                let axis = ui.input(|i| i.modifiers.shift) || drag.constrain;
                self.select_drag_overlay(drag, axis, reach)
            }
            _ => Overlay::None,
        };
        self.paint_2d(ui, &canvas, &overlay);
        if let Some(id) = self.empty_sheet_hint(ui, &canvas) {
            self.invoke(ui.ctx(), &id);
        }
        self.datum_mark(ui, &canvas, area, job.offset);
        self.handles_and_hints(ui, &canvas, reach, modes);
        self.paint_kept_snaps(ui, &canvas, reach);
        if let Some(hit) = snap.filter(|_| canvas.hover().is_some() || gesture.is_some()) {
            Self::paint_snap(ui, &canvas, hit);
            // after what the tool does next, so that stays first
            let hint = match self.step_hint(modes) {
                Some(step) => format!("{step}; {}", Self::snap_hint(hit)),
                None => Self::snap_hint(hit),
            };
            cut_craft_bug_report::hint::hint(ui.ctx(), &hint);
        }
        self.paint_form_marks(ui, &canvas);
        canvas.rulers(ui, job.units);
        self.view_menu(ui, &canvas, reach);
        self.view_buttons(ui, area);
        self.view_transform = Some(canvas.transform);
        self.view_area = Some(area);
        self.cursor = canvas.cursor();
        self.act_2d(ui, gesture, &canvas, reach, area, modes);
    }

    /// A rotary job's wrap, as the 2D view shows it: none on a flat job, or with Options ▸
    /// Appearance's rotary wrap bands off.
    pub(super) fn wrap(&self) -> Option<view2d::Wrap> {
        view2d::Wrap::of(&self.shown_job(), self.options.appearance.rotary_wrap)
    }

    /// Whether the sheet has nothing on it yet: no vectors, pictures or 3D components.
    pub(super) fn nothing_drawn(&self) -> bool {
        self.doc.vectors.is_empty() && self.doc.bitmaps.is_empty() && self.doc.components.is_empty()
    }

    /// The hint in the middle of an empty sheet, until the first thing is drawn or dropped,
    /// with no tool in hand.
    /// With links to what the start page offered: a recent file, the sample project; the
    /// one clicked, as an action to invoke.
    fn empty_sheet_hint(&self, ui: &egui::Ui, canvas: &Canvas) -> Option<String> {
        if !self.nothing_drawn()
            || self.form.is_some()
            || self.view_tool.is_some()
            || self.drawing_tool().is_some()
        {
            return None;
        }
        let job = self.shown_job().area();
        let (a, b) = (
            canvas.transform.to_screen(job.min),
            canvas.transform.to_screen(job.max),
        );
        let sheet = egui::Rect::from_two_pos(a, b).intersect(canvas.area);
        if sheet.width() < ui.spacing().interact_size.x * 2.0 {
            return None;
        }
        let colour = ui.visuals().weak_text_color();
        let size = ui.text_style_height(&egui::TextStyle::Body);
        let mut text = egui::text::LayoutJob::default();
        text.append(
            EMPTY_SHEET_TITLE,
            0.0,
            egui::TextFormat::simple(egui::FontId::proportional(size * 1.4), colour),
        );
        text.append(
            &format!("\n{EMPTY_SHEET_HINT}"),
            0.0,
            egui::TextFormat::simple(egui::FontId::proportional(size), colour),
        );
        text.wrap.max_width = sheet.width() * 0.8;
        text.halign = Align::Center;
        let galley = ui.fonts_mut(|f| f.layout_job(text));
        let at = sheet.center() - egui::vec2(0.0, galley.size().y / 2.0);
        let below = egui::pos2(sheet.center().x, at.y + galley.size().y + size);
        ui.painter()
            .with_clip_rect(sheet)
            .galley(at, galley, colour);
        // what the start page offered, where the start page no longer is
        if !sheet.contains(below + egui::vec2(0.0, size * 2.0)) {
            return None;
        }
        let recent = self.recent_shown();
        let mut clicked = None;
        egui::Area::new(egui::Id::unique("empty_sheet_links"))
            .fixed_pos(below)
            .pivot(egui::Align2::CENTER_TOP)
            .show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    if !recent.is_empty() {
                        ui.menu_button(EMPTY_SHEET_RECENT, |ui| {
                            for (i, path) in recent.iter().enumerate() {
                                if ui
                                    .button(super::file_name(path))
                                    .on_hover_hint(path.display().to_string())
                                    .clicked()
                                {
                                    clicked = Some(format!("open_recent:{i}"));
                                }
                            }
                        });
                        ui.weak("·");
                    }
                    if ui
                        .link(EMPTY_SHEET_SAMPLE)
                        .on_hover_hint(tr!("start-empty-sheet-sample-tip"))
                        .clicked()
                    {
                        clicked = Some("try_sample_project".to_owned());
                    }
                });
            });
        clicked
    }

    /// What a gesture in the 2D view does, once the view is drawn.
    fn act_2d(
        &mut self,
        ui: &egui::Ui,
        gesture: Option<Gesture>,
        canvas: &Canvas,
        reach: f64,
        area: egui::Rect,
        modes: Modes,
    ) {
        let Modes {
            handled,
            tracing,
            filleting,
            placing_tabs,
            joining_ends,
        } = modes;
        // a double-click with nothing in hand steps into a group, or edits text
        if handled
            && canvas.response().double_clicked()
            && let Some(at) = canvas.cursor()
        {
            self.double_click_2d(at, canvas.pick_tolerance(ui));
            return;
        }
        // selecting or transforming, a slow click or one the hand shifted a pixel or two is
        // a click, not a move or a resize (a third of the reach is about three pixels)
        let gesture = match gesture {
            Some(Gesture::Dropped { drag, shift })
                if matches!(self.view_tool, None | Some(ViewTool::Transform))
                    && self.drawing_tool().is_none()
                    && !tracing
                    && drag.start.distance(drag.end) < reach / 3.0 =>
            {
                Some(Gesture::Click {
                    at: drag.start,
                    shift,
                })
            }
            gesture => gesture,
        };
        match (gesture, self.view_tool, self.drawing_tool()) {
            (Some(Gesture::Dropped { drag, .. }), Some(ViewTool::ZoomBox), _) => {
                self.zoom_box(ui, area, drag);
            }
            (Some(Gesture::Dropped { drag, shift }), Some(ViewTool::NodeEdit), _) => {
                self.node_drop(drag, shift, reach);
            }
            (_, Some(ViewTool::Measure), _) => self.measure(gesture, reach),
            (Some(Gesture::Click { at, shift }), Some(ViewTool::NodeEdit), _) => {
                self.click_select(at, shift, reach);
            }
            (gesture, Some(ViewTool::Transform), _) => self.transform_gesture(ui, gesture, reach),
            (gesture, None, Some(id)) => {
                let modifiers = ui.input(|i| i.modifiers);
                self.drawing_gesture(id, gesture, reach, modifiers);
            }
            (Some(Gesture::Dropped { drag, .. }), None, None) if tracing => {
                self.set_trace_area(Some(Rect::new(drag.start, drag.end)));
            }
            (Some(Gesture::Click { at, .. }), None, None) if tracing => self.trace_click(at),
            (Some(Gesture::Dropped { drag, .. }), None, None)
                if handled && self.on_handle(drag.start, reach) =>
            {
                if let Some(t) = self.handle_drag(ui, drag, reach) {
                    let change = Change::transform_selected(&self.doc, |_| t);
                    self.record(change, "Transformed");
                }
            }
            (Some(Gesture::Dropped { drag, shift }), None, None) => {
                self.plain_drop(drag, shift, reach, placing_tabs);
            }
            (Some(Gesture::Click { at, .. }), None, None) if filleting => {
                self.fillet_click(at, reach);
            }
            (Some(Gesture::Click { at, .. }), None, None) if placing_tabs => {
                self.place_tab(at, reach);
            }
            (Some(Gesture::Click { at, .. }), None, None) if joining_ends => {
                self.join_ends_click(at, reach);
            }
            (Some(Gesture::Click { at, shift }), None, None) => {
                self.click_select(at, shift, canvas.pick_tolerance(ui));
            }
            _ => {}
        }
    }

    /// Zoom Box let go: the view zooms to the box dragged, and the tool ends.
    fn zoom_box(&mut self, ui: &egui::Ui, area: egui::Rect, drag: Drag) {
        let size = drag.end - drag.start;
        if size.x.abs() > 0.0 && size.y.abs() > 0.0 {
            let margin = ui.spacing().interact_size.y;
            self.camera = Some(ViewTransform::fit(
                area,
                margin,
                Rect::new(drag.start, drag.end),
            ));
        }
        self.view_tool = None;
        self.status.current_tool = self.tool_label("select");
    }

    /// What open forms mark in the 2D view: Draw Text's arc handles, Sketch Carving's areas,
    /// V-Carve's area to carve, Trace Bitmap's area.
    fn paint_form_marks(&mut self, ui: &egui::Ui, canvas: &Canvas) {
        self.paint_arc_handles(ui, canvas);
        self.paint_sketch_areas(ui, canvas);
        self.paint_carve_area(ui, canvas);
        self.paint_trace_area(ui, canvas);
    }

    /// While V-Carve's form is open, the area it would carve shaded over the vectors: the
    /// shapes for Fill (every other band of nested loops), the groove beside the edge for
    /// Inside and Outside, along the lines for On the Line. Found again only when the
    /// settings, the job or the selection change.
    fn paint_carve_area(&mut self, ui: &egui::Ui, canvas: &Canvas) {
        let Some(form) = self
            .form
            .as_ref()
            .filter(|f| f.id == "vcarve_engrave_toolpath")
        else {
            self.carve_area = None;
            return;
        };
        let fresh = self.carve_area.as_ref().is_some_and(|c| {
            c.params == form.params
                && c.revision == self.history.revision()
                && c.selection == self.doc.selection
        });
        if !fresh {
            let triangles = serde_json::from_value(form.params.clone())
                .ok()
                .map_or_default(|p| {
                    let (_, paths) = self.doc.selected_paths(|_| true);
                    cut_craft_core::medial::triangles(&cmd_vcarve_engrave_toolpath::carved_area(
                        &paths, &p,
                    ))
                });
            self.carve_area = Some(super::CarveArea {
                params: form.params.clone(),
                revision: self.history.revision(),
                selection: self.doc.selection.clone(),
                triangles,
            });
        }
        let Some(area) = &self.carve_area else {
            return;
        };
        let colour = ui
            .visuals()
            .selection
            .bg_fill
            .gamma_multiply(super::CARVE_AREA_ALPHA);
        let mut mesh = egui::Mesh::default();
        for t in &area.triangles {
            let base = u32::try_from(mesh.vertices.len()).unwrap_or(u32::MAX);
            for &p in t {
                mesh.colored_vertex(canvas.transform.to_screen(p), colour);
            }
            mesh.add_triangle(base, base + 1, base + 2);
        }
        ui.painter_at(canvas.area).add(mesh);
    }

    /// While Job Setup is open a red square marks the datum.
    fn datum_mark(&self, ui: &egui::Ui, canvas: &Canvas, area: egui::Rect, datum: Point) {
        if self.form.as_ref().is_some_and(|f| f.id == "job_setup") {
            let edge = ui.spacing().interact_size.y / 2.0;
            let at = canvas.transform.to_screen(datum);
            ui.painter_at(area).rect_filled(
                egui::Rect::from_center_size(at, egui::vec2(edge, edge)),
                0.0,
                ui.visuals().error_fg_color,
            );
        }
    }

    /// The unlocked guide a Select-mode drag from `at` takes: only where no vector, selection
    /// or handle is, so a shape lying on a guide is dragged rather than the guide.
    fn guide_to_drag(&self, at: Point, reach: f64) -> Option<usize> {
        let selecting = self.view_tool.is_none() && self.drawing_tool().is_none();
        let other = || {
            self.handle_at(at, reach).is_some()
                || matches!(
                    self.grabbed(at, reach),
                    Some(Grab::Selection | Grab::Vector(_))
                )
        };
        (selecting && !other())
            .then(|| (self.doc.guides.iter()).position(|g| !g.locked && g.distance(at) <= reach))
            .flatten()
    }

    /// Guides: dragged out of a ruler (top: across, left: up and down), dragged along when
    /// unlocked, and dragged back onto a ruler to delete them. True while one is dragged.
    pub(super) fn guide_gesture(&mut self, ui: &egui::Ui, canvas: &Canvas, reach: f64) -> bool {
        if canvas.drag_started()
            && let Some(origin) = ui.input(|i| i.pointer.press_origin())
        {
            let at = canvas.transform.to_job(origin);
            self.guide_drag = match canvas.ruler_at(ui, origin) {
                Some(vertical) => Some((vertical, None)),
                None => self
                    .guide_to_drag(at, reach)
                    .map(|i| (self.doc.guides[i].vertical, Some(i))),
            };
        }
        let Some((vertical, index)) = self.guide_drag else {
            // resting on a guide a drag would take: a cursor that says it moves
            if let Some(i) = canvas.cursor().and_then(|at| self.guide_to_drag(at, reach)) {
                let g = self.doc.guides[i];
                ui.ctx().set_cursor_icon(match (g.tilted(), g.vertical) {
                    (true, _) => egui::CursorIcon::Move,
                    (false, true) => egui::CursorIcon::ResizeColumn,
                    (false, false) => egui::CursorIcon::ResizeRow,
                });
            }
            return false;
        };
        // Escape (egui lets go of the drag) or the drag otherwise lost: no guide
        if !canvas.response().dragged() && !canvas.drag_stopped() {
            self.guide_drag = None;
            return false;
        }
        let Some(pointer) = ui.input(|i| i.pointer.latest_pos()) else {
            return true;
        };
        let at = canvas.transform.to_job(pointer);
        // the guide as the drag leaves it: moved through the pointer, keeping its angle
        let moved = match index.and_then(|i| self.doc.guides.get(i)) {
            Some(g) => g.through(at),
            None if vertical => cut_craft_core::document::Guide::vertical(at.x),
            None => cut_craft_core::document::Guide::horizontal(at.y),
        };
        if !canvas.drag_stopped() {
            ui.painter_at(canvas.area)
                .line_segment(canvas.guide_ends(&moved), ui.visuals().selection.stroke);
            return true;
        }
        self.guide_drag = None;
        let onto_ruler = canvas.ruler_at(ui, pointer).is_some();
        let mut guides = self.doc.guides.clone();
        match (index, onto_ruler) {
            (Some(i), true) => {
                guides.remove(i);
            }
            (Some(i), false) => guides[i] = moved,
            (None, false) => guides.push(moved),
            (None, true) => {}
        }
        self.set_guides(guides);
        true
    }

    /// Guide Properties, on a right-clicked guide: its position (where it crosses its axis)
    /// and angle, typed; Lock Guide; new guides parallel to it, at an absolute position or an
    /// offset from it, one or several a step apart; Delete Guide.
    #[expect(clippy::too_many_lines, reason = "one window, its rows in order")]
    pub(super) fn guide_window(&mut self, ctx: &egui::Context) {
        let Some(mut props) = self.guide_props.clone() else {
            return;
        };
        let Some(g) = self.doc.guides.get(props.index).copied() else {
            self.guide_props = None;
            return;
        };
        let mut open = true;
        let (mut apply, mut create, mut delete) = (false, false, false);
        let units = self.doc.job.units;
        egui::Window::new(tr!("views-guide-properties"))
            .id(egui::Id::unique("guide_properties"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                cut_craft_bug_report::mark(ui, "Guide Properties");
                ui.label(if g.vertical {
                    tr!("views-a-vertical-guide-its-position")
                } else {
                    tr!("views-a-horizontal-guide-its-position")
                });
                ui.horizontal(|ui| {
                    ui.label(tr!("views-position"));
                    crate::form::length_edit(
                        ui,
                        &tr!("views-position"),
                        &mut props.position,
                        units,
                        f64::NEG_INFINITY,
                    );
                });
                ui.horizontal(|ui| {
                    ui.label(tr!("views-angle"));
                    let hint = "Degrees anticlockwise from the X axis";
                    ui.add(egui::Slider::new(&mut props.angle, -180.0..=180.0).show_value(false))
                        .accessible_name(tr!("views-angle"))
                        .on_hover_hint(hint);
                    ui.add(
                        Number::new(&mut props.angle)
                            .range(-180.0..=180.0)
                            .suffix("°")
                            .name("Angle"),
                    )
                    .on_hover_hint(hint);
                });
                ui.checkbox(&mut props.locked, tr!("views-lock-guide"))
                    .on_hover_hint(tr!("views-a-locked-guide-cannot-be"));
                apply = ui.button(tr!("views-apply")).clicked();
                ui.separator();
                ui.strong(tr!("views-new-guide"));
                ui.horizontal(|ui| {
                    // an angled guide gives only relative parallels
                    ui.add_enabled_ui(!g.tilted(), |ui| {
                        ui.radio_value(&mut props.relative, false, tr!("views-absolute"));
                    });
                    ui.radio_value(&mut props.relative, true, tr!("views-relative"));
                });
                ui.horizontal(|ui| {
                    ui.label(if props.relative {
                        tr!("views-offset")
                    } else {
                        tr!("views-position")
                    });
                    let name = tr!(if props.relative {
                        "views-new-guide-offset"
                    } else {
                        "views-new-guide-position"
                    });
                    crate::form::length_edit(
                        ui,
                        &name,
                        &mut props.new_at,
                        units,
                        f64::NEG_INFINITY,
                    );
                    ui.label(tr!("views-count"));
                    ui.add(
                        Number::new(&mut props.count)
                            .range(1..=100)
                            .name(tr!("views-count")),
                    );
                });
                create = ui
                    .button(tr!("views-create-new-guide"))
                    .on_hover_hint(tr!("views-parallel-to-this-guide-several"))
                    .clicked();
                delete = ui.button(tr!("views-delete-guide")).clicked();
            });
        if g.tilted() {
            props.relative = true;
        }
        self.guide_props = open.then_some(props);
        if apply {
            self.guide_props_apply();
        }
        if create {
            self.guide_props_create();
        }
        if delete {
            self.guide_props_delete();
        }
    }

    /// Guide Properties' Apply: the position, angle and lock typed.
    pub(super) fn guide_props_apply(&mut self) {
        let Some(props) = self.guide_props.clone() else {
            return;
        };
        let mut guides = self.doc.guides.clone();
        let Some(g) = guides.get_mut(props.index) else {
            return;
        };
        *g = cut_craft_core::document::Guide {
            at: props.position,
            locked: props.locked,
            ..g.with_angle(props.angle)
        };
        self.set_guides(guides);
    }

    /// Guide Properties' Create New Guide: `count` guides parallel to this one, `new_at`
    /// apart from it (relative) or the first at that position and the rest the same step on.
    pub(super) fn guide_props_create(&mut self) {
        let Some(props) = self.guide_props.clone() else {
            return;
        };
        let mut guides = self.doc.guides.clone();
        let Some(&g) = guides.get(props.index) else {
            return;
        };
        let step = if props.relative {
            props.new_at
        } else {
            props.new_at - g.at
        };
        for k in 1..=props.count {
            let by = step * f64::from(k);
            let made = if props.relative {
                g.parallel(by)
            } else {
                cut_craft_core::document::Guide { at: g.at + by, ..g }
            };
            guides.push(cut_craft_core::document::Guide {
                locked: false,
                ..made
            });
        }
        self.set_guides(guides);
    }

    /// Guide Properties' Delete Guide, closing the window.
    pub(super) fn guide_props_delete(&mut self) {
        let Some(props) = self.guide_props.take() else {
            return;
        };
        let mut guides = self.doc.guides.clone();
        if props.index < guides.len() {
            guides.remove(props.index);
            self.set_guides(guides);
        }
    }

    /// Replaces the guides, undoably.
    pub(super) fn set_guides(&mut self, guides: Vec<cut_craft_core::document::Guide>) {
        let mut c = Change::builder(&self.doc);
        c.set_guides(guides);
        self.record(Ok(c.finish()), "Guides changed");
    }
}
