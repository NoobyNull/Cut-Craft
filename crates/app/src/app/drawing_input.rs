//! Drawing in the 2D view: sculpting keys, drawing gestures, editing text on the sheet, sketch
//! areas, arc handles and drawing polylines.

use super::{
    BRUSH_FINE_STEP, BRUSH_STEP, Canvas, Change, CutCraftApp, Drag, EntityId, Gesture, Path, Point,
    SCROLL_NOTCH, Value, egui, view2d,
};

impl CutCraftApp {
    /// Sculpting's keys while its form is open: 1 to 5 pick the tool, the
    /// wheel sizes the brush (finer with Ctrl), Shift and the wheel set the strength, and
    /// Alt and the wheel zoom the view as the wheel alone otherwise does.
    pub(super) fn sculpt_keys(&mut self, ctx: &egui::Context) {
        use egui::Key;
        const TOOLS: [&str; 5] = ["smooth", "smudge", "deposit", "remove", "undo_brush"];
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let Some(form) = self.form.as_mut().filter(|f| f.id == "sculpt") else {
            return;
        };
        let keys = [Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5];
        for (key, tool) in keys.into_iter().zip(TOOLS) {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key)) {
                form.params["tool"] = Value::from(tool);
            }
        }
        let over_view = ctx.pointer_hover_pos().is_some_and(|at| {
            self.view_area.is_some_and(|a| a.contains(at))
                || self.view_3d_area.is_some_and(|a| a.contains(at))
        });
        // the wheel's notches this frame (a line each, or a line's worth of points)
        let (modifiers, notches) = ctx.input(|i| {
            let notches: f32 = (i.raw.events.iter())
                .filter_map(|e| match e {
                    egui::Event::MouseWheel { unit, delta, .. } => Some(match unit {
                        egui::MouseWheelUnit::Line => delta.x + delta.y,
                        egui::MouseWheelUnit::Point => (delta.x + delta.y) / SCROLL_NOTCH,
                        egui::MouseWheelUnit::Page => (delta.x + delta.y) * SCROLL_NOTCH,
                    }),
                    _ => None,
                })
                .sum();
            (i.modifiers, notches)
        });
        if !over_view || modifiers.alt || notches == 0.0 {
            return;
        }
        // the wheel is the brush's, not the view's
        ctx.input_mut(|i| i.smooth_scroll_delta = egui::Vec2::ZERO);
        let (field, step) = match (modifiers.shift, modifiers.ctrl) {
            (true, _) => ("strength", BRUSH_STEP),
            (false, true) => ("diameter", BRUSH_FINE_STEP),
            (false, false) => ("diameter", BRUSH_STEP),
        };
        let value = form.params[field].as_f64().unwrap_or(0.0);
        let scaled = value * (1.0 + step).powf(f64::from(notches));
        let scaled = if field == "strength" {
            scaled.clamp(1.0, 100.0)
        } else {
            scaled.max(0.1)
        };
        form.params[field] = Value::from((scaled * 100.0).round() / 100.0);
    }

    /// A gesture with drawing tool `id`: the polyline's own clicks and drags, a Shift-click
    /// editing a shape the tool drew, or a finished drag drawing one.
    pub(super) fn drawing_gesture(
        &mut self,
        id: &'static str,
        gesture: Option<Gesture>,
        reach: f64,
        modifiers: egui::Modifiers,
    ) {
        match gesture {
            gesture if id == "draw_polyline" => self.polyline_gesture(gesture, reach),
            Some(Gesture::Click { at, .. }) if id == "draw_text" => {
                self.text_click(at, reach, modifiers);
            }
            Some(Gesture::Dropped { drag, .. })
                if id == "draw_text" && self.text_arc_drag(drag, reach).is_some() =>
            {
                if let Some(bent) = self.text_arc_drag(drag, reach) {
                    self.set_text_params(&bent);
                }
            }
            Some(Gesture::Click { at, shift: true }) => self.shift_click_shape(id, at, reach),
            Some(Gesture::Dropped { drag, .. }) => self.draw(id, drag),
            _ => {}
        }
    }

    /// A click with Draw Text open: between two letters it brings them together (Shift:
    /// apart; Ctrl: by more; Alt: every pair in the block), between two lines it brings the
    /// lines together (Shift apart, Ctrl by more); anywhere else it places the anchor, or
    /// with Shift picks text to edit.
    pub(super) fn text_click(&mut self, at: Point, reach: f64, modifiers: egui::Modifiers) {
        /// A click's step: a share of the text height between letters, of the pitch
        /// between lines; Ctrl makes it four.
        const KERN_STEP: f64 = 0.05;
        const LINE_STEP: f64 = 0.1;
        let Some(form) = self.form.as_ref().filter(|f| f.id == "draw_text") else {
            return;
        };
        let Ok(mut p) = serde_json::from_value::<cmd_draw_text::Params>(form.params.clone()) else {
            return;
        };
        let way = if modifiers.shift { 1.0 } else { -1.0 } * if modifiers.ctrl { 4.0 } else { 1.0 };
        match cmd_draw_text::gap_at(&p, at, reach) {
            Ok(Some(cmd_draw_text::Gap::Letters { line, index })) => {
                let by = way * KERN_STEP * p.height;
                if modifiers.alt {
                    for (line, index) in cmd_draw_text::letter_gaps(&p).unwrap_or_default() {
                        p.spacing.kern(line, index, by);
                    }
                } else {
                    p.spacing.kern(line, index, by);
                }
            }
            Ok(Some(cmd_draw_text::Gap::Lines)) => {
                p.spacing.line_pitch = (p.spacing.line_pitch + way * LINE_STEP).max(LINE_STEP);
            }
            _ if modifiers.shift => return self.shift_click_shape("draw_text", at, reach),
            _ => p.anchor = cut_craft_core::params::Xy::new(at.x, at.y),
        }
        let value = serde_json::to_value(&p).expect("text settings serialise");
        if let Some(form) = self.form.as_mut() {
            form.params = value.clone();
        }
        // text already made follows at once: it is several vectors, not a shape that follows
        // the form by itself
        if p.replaces.is_some()
            && let Err(e) = self.apply_form("draw_text", &value)
        {
            self.status.set_message(&e);
        }
    }

    /// Draw Text open on text made before: its settings, as the form has them.
    fn edited_text(&self) -> Option<cmd_draw_text::Params> {
        let form = self.form.as_ref().filter(|f| f.id == "draw_text")?;
        let p: cmd_draw_text::Params = serde_json::from_value(form.params.clone()).ok()?;
        p.replaces.is_some().then_some(p)
    }

    /// A drag from one of the green handles over text Draw Text is editing: the text's
    /// settings with the arc the drag gives it (up bends it up, down bends it down).
    pub(super) fn text_arc_drag(&self, drag: Drag, reach: f64) -> Option<cmd_draw_text::Params> {
        let p = self.edited_text()?;
        let handles = cmd_draw_text::arc_handles(&p)?;
        handles
            .iter()
            .any(|h| h.distance(drag.start) <= reach)
            .then(|| {
                let rise = cmd_draw_text::arc_rise(&p) + drag.end.y - drag.start.y;
                cmd_draw_text::Params {
                    arc: cmd_draw_text::arc_from_rise(&p, rise),
                    ..p
                }
            })
    }

    /// The vectors the text settings `p` make, without making them.
    pub(super) fn text_preview(&self, p: &cmd_draw_text::Params) -> Vec<Path> {
        let value = serde_json::to_value(p).expect("text settings serialise");
        self.registry
            .get("draw_text")
            .and_then(|cmd| cmd.run_json(&self.doc, &value).ok())
            .map_or_default(|change| change.added().cloned().collect())
    }

    /// The text settings `p` into Draw Text's form, and the text made again with them.
    fn set_text_params(&mut self, p: &cmd_draw_text::Params) {
        let value = serde_json::to_value(p).expect("text settings serialise");
        if let Some(form) = self.form.as_mut() {
            form.params = value.clone();
        }
        if let Err(e) = self.apply_form("draw_text", &value) {
            self.status.set_message(&e);
        }
    }

    /// While Sketch Carving's form is open, the areas it would carve in translucent green,
    /// highlighted; found again only when the settings or the job change.
    pub(super) fn paint_sketch_areas(&mut self, ui: &egui::Ui, canvas: &Canvas) {
        let Some(form) = self
            .form
            .as_ref()
            .filter(|f| f.id == "sketch_carving_toolpath")
        else {
            self.sketch_areas = None;
            return;
        };
        let fresh = self.sketch_areas.as_ref().is_some_and(|s| {
            s.params == form.params
                && s.revision == self.history.revision()
                && s.selection == self.doc.selection
        });
        if !fresh {
            let areas = serde_json::from_value(form.params.clone())
                .ok()
                .and_then(|p| cmd_sketch_carving_toolpath::areas(&self.doc, &p).ok())
                .unwrap_or_default();
            self.sketch_areas = Some(super::SketchAreas {
                params: form.params.clone(),
                revision: self.history.revision(),
                selection: self.doc.selection.clone(),
                areas,
            });
        }
        let Some(sketch) = &self.sketch_areas else {
            return;
        };
        let painter = ui.painter_at(canvas.area);
        let green = egui::Color32::GREEN.gamma_multiply(super::SKETCH_AREA_ALPHA);
        for r in &sketch.areas {
            let rect = egui::Rect::from_two_pos(
                canvas.transform.to_screen(r.min),
                canvas.transform.to_screen(r.max),
            );
            painter.rect_filled(rect, 0.0, green);
        }
    }

    /// The two green handles over text Draw Text is editing: drag them to bend it.
    pub(super) fn paint_arc_handles(&self, ui: &egui::Ui, canvas: &Canvas) {
        let Some(handles) = self
            .edited_text()
            .as_ref()
            .and_then(cmd_draw_text::arc_handles)
        else {
            return;
        };
        let half = ui.spacing().interact_size.y * view2d::NODE_HANDLE_SHARE / 2.0;
        let painter = ui.painter_at(canvas.area);
        for h in handles {
            let square = egui::Rect::from_center_size(
                canvas.transform.to_screen(h),
                egui::vec2(half, half) * 2.0,
            );
            painter.rect_filled(square, 0.0, egui::Color32::GREEN);
            painter.rect_stroke(
                square,
                0,
                ui.visuals().window_stroke,
                egui::StrokeKind::Middle,
            );
        }
    }

    /// Shift-click in drawing tool `id`'s form: a shape that tool drew, under `at`, is
    /// edited in the form (Shift-click). A tool that keeps its settings with
    /// the group it made (Dimensions) gives them back from there.
    pub(super) fn shift_click_shape(&mut self, id: &'static str, at: Point, reach: f64) {
        let picked = self.doc.pick(at, reach);
        let recorded = picked.and_then(|v| {
            let group = self.doc.vector_groups.get(&v)?;
            let record = self.doc.texts.get(group)?;
            (record["command"] == id).then(|| {
                let mut params = record.clone();
                params["replaces"] = Value::from(*group);
                (v, params)
            })
        });
        let shape = recorded.or_else(|| {
            let v = picked?;
            let (command, params) = self
                .registry
                .drawn_by(&self.doc, self.doc.vectors.get(&v)?)?;
            (command == id).then_some((v, params))
        });
        match shape {
            Some((v, params)) => self.edit_shape(v, id, params),
            None => self
                .status
                .set_message(&cut_craft_i18n::tr!("status-shift-click-to-edit")),
        }
    }

    /// Draw Line / Polyline's keys while it is the tool: S swaps curves and straight lines;
    /// with a polyline begun, Backspace takes back a point (or a typed character), Tab
    /// closes it, Space finishes it and starts another (unless a point is being typed),
    /// Esc finishes it (and closes the form), and numbers typed then Enter place the next
    /// point.
    pub(super) fn polyline_keys(&mut self, ctx: &egui::Context) {
        use egui::{Key, Modifiers};
        if self.drawing_tool() != Some("draw_polyline") {
            if !self.polyline.is_empty() {
                self.polyline = crate::polyline::Draft::default();
            }
            return;
        }
        // egui moved its focus with the Tab that closed the last polyline: the drawing keeps
        // the keys
        if std::mem::take(&mut self.polyline.tabbed) {
            ctx.memory_mut(|m| {
                if let Some(id) = m.focused() {
                    m.surrender_focus(id);
                }
            });
        }
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let key = |k: Key| ctx.input_mut(|i| i.consume_key(Modifiers::NONE, k));
        if key(Key::S) {
            self.polyline.toggle_curves();
            self.status
                .set_message(&cut_craft_i18n::tr!(if self.polyline.curved {
                    "status-drawing-curves"
                } else {
                    "status-drawing-lines"
                }));
        }
        // what is typed goes to the next point, not to the shortcuts
        let typed: String = ctx.input(|i| {
            i.events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::Text(t) => Some(t.clone()),
                    _ => None,
                })
                .collect()
        });
        let mut typed: String = typed
            .chars()
            .filter(|c| c.is_ascii_digit() || ".,-aAlL ".contains(*c))
            .collect();
        // a space only goes between what is typed ("90A 2L"); on its own it finishes
        if self.polyline.typed.is_empty() {
            typed = typed.trim_start().to_owned();
        }
        if !typed.is_empty() {
            for k in [Key::A, Key::L, Key::Comma, Key::Period, Key::Minus]
                .into_iter()
                .chain((0..=9).filter_map(|n| Key::from_name(&n.to_string())))
            {
                key(k);
            }
            self.polyline.typed.push_str(&typed);
            self.status.set_message(&cut_craft_i18n::tr!(
                "status-next-point",
                typed = self.polyline.typed.clone()
            ));
        }
        if key(Key::Enter) && !self.polyline.typed.is_empty() {
            let unit = crate::form::unit_mm(self.doc.job.units);
            let toward = ctx
                .pointer_latest_pos()
                .zip(self.view_transform)
                .map_or(cut_craft_core::geometry::Point::ZERO, |(at, t)| {
                    t.to_job(at)
                });
            match self.polyline.typed_point(unit, toward) {
                Some(at) => self.polyline.click(at),
                None => self
                    .status
                    .set_message(&cut_craft_i18n::tr!("status-type-a-point")),
            }
            self.polyline.typed.clear();
        }
        if self.polyline.is_empty() {
            return;
        }
        if key(Key::Backspace) && self.polyline.typed.pop().is_none() {
            self.polyline.back();
        }
        if key(Key::Tab) {
            self.finish_polyline(true);
            self.polyline.tabbed = true;
        }
        // while a point is being typed, Space is part of it, not the end of the polyline
        if key(Key::Space) && self.polyline.typed.is_empty() {
            self.finish_polyline(false);
        }
        // Escape is left for the shortcuts, which then close the form
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.finish_polyline(false);
        }
    }

    /// Edit Polyline: Draw Line / Polyline goes on drawing from an open vector's end (its
    /// start, `from_start`), and what is drawn extends it.
    pub(super) fn edit_polyline(&mut self, id: EntityId, from_start: bool) {
        let Some(path) = self.doc.vectors.get(&id).filter(|p| !p.closed) else {
            return;
        };
        let path = if from_start {
            path.reversed()
        } else {
            path.clone()
        };
        let end = path
            .segments
            .last()
            .map_or(path.start, cut_craft_core::geometry::Segment::end);
        self.view_tool = None;
        self.start_command("draw_polyline");
        let curved = self.polyline.curved;
        self.polyline = crate::polyline::Draft::default();
        (self.polyline.curved, self.polyline.extends) = (curved, Some((id, path)));
        self.polyline.click(end);
        self.status
            .set_message(&cut_craft_i18n::tr!("status-drawing-on-from-end"));
    }

    /// The polyline drawn so far made (closed or not), and a new one begun in the same mode.
    pub(super) fn finish_polyline(&mut self, closed: bool) {
        let draft = std::mem::take(&mut self.polyline);
        self.polyline.curved = draft.curved;
        if draft.extends.is_some() {
            if let Some((id, path)) = draft.extended(closed) {
                let mut c = Change::builder(&self.doc);
                c.replace(id, path);
                c.select([id]);
                self.record(Ok(c.finish()), "Polyline extended");
            }
            return;
        }
        if let Some(params) = draft.params(closed) {
            if let Some(form) = &mut self.form {
                form.params.clone_from(&params);
            }
            if let Err(e) = self.apply("draw_polyline", &params) {
                self.status.set_message(&e);
            }
        }
    }

    /// A click adds the next point (on the first, it closes the polyline; on the last, as
    /// the second click of a double-click, it finishes it open); a drag adds one with a
    /// smooth span.
    fn polyline_gesture(&mut self, gesture: Option<Gesture>, reach: f64) {
        match gesture {
            Some(Gesture::Click { at, .. }) => {
                let near = |p: Option<&cut_craft_core::geometry::Point>| {
                    p.is_some_and(|p| p.distance(at) <= reach)
                };
                // the polyline's first point: an extended vector's start
                let first = match &self.polyline.extends {
                    Some((_, base)) => Some(&base.start),
                    None => self
                        .polyline
                        .points
                        .first()
                        .filter(|_| self.polyline.points.len() >= 3),
                };
                if near(first) {
                    self.finish_polyline(true);
                } else if near(self.polyline.points.last()) {
                    // a double-click ends the polyline open there (S swaps to curves);
                    // on a lone first point there is nothing to finish yet
                    if self.polyline.points.len() >= 2 || self.polyline.extends.is_some() {
                        self.finish_polyline(false);
                    }
                } else {
                    self.polyline.click(at);
                }
            }
            Some(Gesture::Dropped { drag, .. }) => self.polyline.drag(drag.start, drag.end),
            _ => {}
        }
    }

    /// The polyline being drawn, on to the pointer (or along a drag in progress).
    /// `pointer` is where the pointer is over the view, in job coordinates.
    pub(super) fn polyline_overlay(
        &self,
        gesture: Option<Gesture>,
        pointer: Option<Point>,
    ) -> Vec<Path> {
        let mut draft = self.polyline.clone();
        let pointer = match gesture {
            Some(Gesture::Dragging(drag)) => {
                draft.drag(drag.start, drag.end);
                None
            }
            _ => pointer,
        };
        draft
            .path(false, pointer.filter(|_| !draft.is_empty()))
            .into_iter()
            .collect()
    }

    /// Clipart dragged from the Clipart tab and dropped on the 2D view goes where it was
    /// dropped; while Toolpath Tiling is open a double-click on a tile makes it the active
    /// one (on it again, none).
    pub(super) fn clipart_dropped_2d(&mut self, ui: &egui::Ui, canvas: &Canvas) {
        if canvas.response().double_clicked()
            && let Some(form) = self.form.as_mut().filter(|f| f.id == "toolpath_tiling")
            && let Some(tiling) = self.doc.tiling
            && let Some(at) = ui.ctx().pointer_latest_pos()
        {
            let at = canvas.transform.to_job(at);
            let tiles = tiling.tiles(self.doc.job.area());
            if let Some(i) = tiles.iter().position(|t| t.contains(at)) {
                let picked = u32::try_from(i + 1).unwrap_or(0);
                let active = if tiling.active == picked { 0 } else { picked };
                form.params["tiling"]["active"] = Value::from(active);
                let params = form.params.clone();
                if let Err(e) = self.apply("toolpath_tiling", &params) {
                    self.status.set_message(&e);
                }
            }
        }
        if let Some(path) = canvas
            .response()
            .dnd_release_payload::<std::path::PathBuf>()
            && let Some(at) = ui.ctx().pointer_latest_pos()
        {
            let at = canvas.transform.to_job(at);
            self.import_clipart(&path, Some(at), None);
        }
    }
}
