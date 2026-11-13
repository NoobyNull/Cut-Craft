//! Drawing vectors in the 3D view: the 2D view's drawing tools, with the
//! pointer taken to where it meets the model (or the material's top plane off it).

use super::views::Modes;
use super::{Cow, CutCraftApp, Gesture, Material, Path, Point, egui, view2d, view3d};
use std::sync::Arc;

impl CutCraftApp {
    /// The drawing tool the 3D view's left button draws with: the open form's, when it is
    /// drawn with the mouse, no view tool is on, and the job is not shown wrapped round its
    /// rotary axis. Sculpting has its own drag.
    pub(super) fn drawing_in_3d(&self) -> Option<&'static str> {
        let wrapped = self.doc.job.rotary.is_some() && self.display.auto_wrapping;
        let importing = self.form.as_ref().is_some_and(|f| f.model.is_some());
        self.drawing_tool()
            .filter(|id| *id != "sculpt")
            .filter(|_| self.view_tool.is_none() && !wrapped && !importing)
    }

    /// A gesture in the 3D view with drawing tool `id`, once the view is drawn: what it
    /// would draw shown on the model, the status bar's next step, and the gesture handed to
    /// the tool as the 2D view hands it.
    pub(super) fn draw_3d(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        area: egui::Rect,
        fit: egui::Rect,
        id: &'static str,
    ) {
        let Some(surface) = self.surface_3d().map(Cow::into_owned) else {
            return;
        };
        let camera = view3d::Camera::new(&surface, self.orbit, self.framing, fit, None);
        // the pointer meets the model as finely as the settled view draws it
        let model = self.drawing_surface();
        let solid = model.as_deref().unwrap_or(&surface);
        let to_job = |at: egui::Pos2| camera.on_surface(solid, at).map(|p| p + solid.origin);
        let gesture = view2d::gesture_of(ui, response, to_job);
        #[expect(clippy::cast_possible_truncation, reason = "a screen scale fits f32")]
        let scale = camera.scale() as f32;
        let hovered = response.hover_pos().and_then(to_job);
        let (gesture, snap, _) = self.snapping(ui, gesture, scale, hovered);
        // the 2D view's pick radius: half the theme's interactive height, on screen
        let reach = f64::from(ui.spacing().interact_size.y / 2.0) / camera.scale();
        self.track_stroke(gesture);
        self.cursor = hovered;
        // drawing: a crosshair, to place points by
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        }
        // the preview runs to where a click would land
        let aim = snap.map(|s| s.at).or(hovered);
        let preview = self.preview_3d(gesture, aim, id, reach);
        let stroke = ui.visuals().selection.stroke;
        let painter = ui.painter_at(area);
        for path in &preview {
            let points = path.flatten(cut_craft_core::polygon::TOLERANCE);
            for run in lines_on(&surface, (self.orbit, self.framing), fit, &points) {
                painter.add(egui::Shape::line(run, stroke));
            }
        }
        if response.hovered()
            && let Some(step) = self.step_hint(Modes::default())
        {
            let hint = match snap {
                Some(hit) => format!(
                    "{step}; {}; right-drag turns the view",
                    Self::snap_hint(hit)
                ),
                None => format!("{step}; right-drag turns the view"),
            };
            cut_craft_bug_report::hint::hint(ui.ctx(), &hint);
        }
        let modifiers = ui.input(|i| i.modifiers);
        self.drawing_gesture(id, gesture, reach, modifiers);
    }

    /// The model at the detail the settled 3D view draws it ([`Self::fine_surface`]), kept
    /// while the document and job stay as they are; `None` while a preview shows, which is
    /// hit as [`Self::surface_3d`] has it.
    pub(super) fn drawing_surface(&mut self) -> Option<Arc<Material>> {
        if self.simulation.is_some() || self.playback.is_some() || self.doc.components.is_empty() {
            return None;
        }
        let cells = self.options.view_3d.still_cells();
        let key = (self.history.revision(), self.doc.job, cells);
        if let Some((kept, model)) = &self.drawn_on
            && *kept == key
        {
            return Some(Arc::clone(model));
        }
        let model = Arc::new(self.fine_surface());
        self.drawn_on = Some((key, Arc::clone(&model)));
        Some(model)
    }

    /// What drawing tool `id` would make with `gesture`, as the 2D view previews it.
    fn preview_3d(
        &self,
        gesture: Option<Gesture>,
        pointer: Option<Point>,
        id: &'static str,
        reach: f64,
    ) -> Vec<Path> {
        match gesture {
            gesture if id == "draw_polyline" => self.polyline_overlay(gesture, pointer),
            Some(Gesture::Dragging(drag)) => match self.text_arc_drag(drag, reach) {
                Some(bent) => self.text_preview(&bent),
                None => self.preview(id, drag),
            },
            _ => vec![],
        }
    }
}

/// The parts of the line through job `points` that the 3D view of `surface` shows, on
/// screen: flat at the height a vector there is drawn ([`view3d::vector_plane`]), less what
/// the surface hides.
pub(super) fn lines_on(
    surface: &Material,
    (orbit, framing): (view3d::Orbit, view3d::Framing),
    fit: egui::Rect,
    points: &[Point],
) -> Vec<Vec<egui::Pos2>> {
    let screen = view3d::screen_of(surface, orbit, framing, fit, None);
    let seen = view3d::seen_of(surface, orbit, framing);
    let z = view3d::vector_plane(surface, points);
    let line: Vec<_> = points
        .iter()
        .map(|p| (*p - surface.origin).extend(z))
        .collect();
    view3d::visible_runs(&line, surface.cell, &seen)
        .into_iter()
        .map(|run| run.into_iter().map(&screen).collect())
        .collect()
}
