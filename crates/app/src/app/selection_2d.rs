//! The selection in the 2D view: moving and transforming it, its handles, and the status bar's
//! next-step hints.

use super::views::{
    EMPTY_SHEET_STEP, INSIDE_GROUP, Modes, ROTATE_OUT, SELECTION_HANDLE_SHARE, move_cross,
};
use super::{
    Canvas, Change, CutCraftApp, DAffine2, Drag, EntityId, Gesture, Grab, Handle, Overlay, Path,
    Point, Rect, ViewTool, egui, gui, handles,
};
use crate::flyout::DragWay;

impl CutCraftApp {
    /// Transform mode while dragging: the selection as the handle would leave it, else the
    /// selection window.
    pub(super) fn transform_overlay(&self, ui: &egui::Ui, drag: Drag, reach: f64) -> Overlay {
        if let Some(t) = self.handle_drag(ui, drag, reach) {
            let doc = &self.doc;
            let paths = doc
                .selection
                .iter()
                .map(|id| doc.vectors[id].transformed(&t))
                .collect();
            return Overlay::Moving {
                paths,
                components: self.moving_components(doc.selected_components.iter(), t),
            };
        }
        Overlay::Window {
            start: drag.start,
            end: drag.end,
            crossing: drag.end.x < drag.start.x,
        }
    }

    /// What a select-mode drag starting at `at` takes hold of: the selection when it starts
    /// inside its box, else the vector or model under it (picked as the drag ends), else
    /// nothing (the drag selects by window).
    pub(super) fn grabbed(&self, at: Point, reach: f64) -> Option<Grab> {
        let inside = self.doc.selection_bounds().is_some_and(|b| {
            Rect::new(b.min - Point::splat(reach), b.max + Point::splat(reach)).contains(at)
        });
        if inside {
            return Some(Grab::Selection);
        }
        if let Some(id) = self.doc.pick(at, reach) {
            return Some(Grab::Vector(id));
        }
        self.doc.pick_component(at).map(Grab::Component)
    }

    /// The outlines a grab moves: vectors as they are, models as their boxes.
    pub(super) fn grabbed_outlines(&self, grab: Grab) -> Vec<Path> {
        let boxed = |r: Rect| {
            Path::polyline(
                &[
                    r.min,
                    Point::new(r.max.x, r.min.y),
                    r.max,
                    Point::new(r.min.x, r.max.y),
                ],
                true,
            )
        };
        match grab {
            Grab::Vector(id) => (self.picked_with(id).iter())
                .filter_map(|m| self.doc.vectors.get(m).cloned())
                .collect(),
            Grab::Component(id) => vec![boxed(self.doc.components[&id].extent())],
            Grab::Selection => {
                let vectors = self
                    .doc
                    .selection
                    .iter()
                    .map(|id| self.doc.vectors[id].clone());
                let models = self
                    .doc
                    .selected_components
                    .iter()
                    .filter_map(|id| self.doc.components.get(id))
                    .map(|k| boxed(k.extent()));
                vectors.chain(models).collect()
            }
        }
    }

    /// A select-mode drag in progress: what it moves, followed by the pointer, else the
    /// selection window.
    pub(super) fn select_drag_overlay(&self, drag: Drag, axis: bool, reach: f64) -> Overlay {
        let Some(grab) = self.grabbed(drag.start, reach) else {
            return Overlay::Window {
                start: drag.start,
                end: drag.end,
                crossing: drag.end.x < drag.start.x,
            };
        };
        let by = cut_craft_core::glam::DAffine2::from_translation(
            self.select_move_by(drag, axis, reach),
        );
        let models: Vec<EntityId> = match grab {
            Grab::Vector(_) => vec![],
            Grab::Component(id) => vec![id],
            Grab::Selection => self.doc.selected_components.iter().copied().collect(),
        };
        Overlay::Moving {
            paths: self
                .grabbed_outlines(grab)
                .iter()
                .map(|p| p.transformed(&by))
                .collect(),
            components: self.moving_components(models.iter(), by),
        }
    }

    /// Each of `ids` still in the job with the transform `t` a drag carries it by (the same
    /// one for each, as the transform let go applies it).
    fn moving_components<'a>(
        &self,
        ids: impl Iterator<Item = &'a EntityId>,
        t: DAffine2,
    ) -> Vec<(EntityId, DAffine2)> {
        ids.filter(|id| self.doc.components.contains_key(id))
            .map(|&id| (id, t))
            .collect()
    }

    /// How far a Select-mode drag moves what it grabbed: from the move cross in the
    /// selection's middle with Shift or Ctrl held (`axis`), along one axis only, as its
    /// hint says; anywhere else, where it was dragged.
    fn select_move_by(&self, drag: Drag, axis: bool, reach: f64) -> Point {
        let mut by = drag.end - drag.start;
        if axis && self.handle_at(drag.start, reach) == Some(Handle::Move) {
            if by.x.abs() > by.y.abs() {
                by.y = 0.0;
            } else {
                by.x = 0.0;
            }
        }
        by
    }

    /// A select-mode drag let go: what it took hold of moves (as one undo), as the
    /// select tool does; a drag from empty space selects by window.
    pub(super) fn select_drop(&mut self, drag: Drag, shift: bool, reach: f64) {
        let Some(grab) = self.grabbed(drag.start, reach) else {
            let ids = self
                .doc
                .window(Rect::new(drag.start, drag.end), drag.end.x < drag.start.x);
            return self.select(ids, shift);
        };
        // a vector or model not yet picked is picked, on its own (a vector with its
        // group, as a click picks it), as it is dragged
        match grab {
            Grab::Vector(id) => {
                self.select([id], false);
                self.doc.selected_components.clear();
            }
            Grab::Component(id) => {
                self.doc.selection.clear();
                self.doc.selected_components = [id].into();
            }
            Grab::Selection => {}
        }
        let by = self.select_move_by(drag, shift || drag.constrain, reach);
        let params =
            serde_json::json!({ "relative": true, "x": by.x, "y": by.y, "anchor": "bottom_left" });
        if let Err(e) = self.apply("move", &params) {
            self.status.set_message(&e);
        }
    }

    /// Transform mode: a drag from a handle transforms the selection, any other drag
    /// selects by window; a click on the middle turns rotation on or off, elsewhere selects.
    pub(super) fn transform_gesture(
        &mut self,
        ui: &egui::Ui,
        gesture: Option<Gesture>,
        reach: f64,
    ) {
        match gesture {
            Some(Gesture::Dropped { drag, shift }) => {
                if let Some(t) = self.handle_drag(ui, drag, reach) {
                    let change = Change::transform_selected(&self.doc, |_| t);
                    self.record(change, "Transformed");
                } else {
                    let window = Rect::new(drag.start, drag.end);
                    let ids = self.doc.window(window, drag.end.x < drag.start.x);
                    self.select(ids, shift);
                }
            }
            Some(Gesture::Click { at, shift }) => {
                let on_middle = self
                    .doc
                    .selection_bounds()
                    .and_then(|b| handles::pick(b, self.rotating, at, reach))
                    == Some(Handle::Move);
                if on_middle {
                    self.rotating ^= true;
                } else {
                    self.click_select(at, shift, reach);
                }
            }
            _ => {}
        }
    }

    /// The selection's handles when shown, and the status bar's hint: a handle's own under
    /// the pointer, else what the tool in hand does next.
    pub(super) fn handles_and_hints(
        &self,
        ui: &egui::Ui,
        canvas: &Canvas,
        reach: f64,
        modes: Modes,
    ) {
        let handled = modes.handled;
        if handled || self.view_tool == Some(ViewTool::Transform) {
            self.paint_handles(ui, canvas);
            // over a handle, the status bar says what it does and the keys that change it
            if let Some(handle) = canvas.cursor().and_then(|at| self.handle_at(at, reach)) {
                cut_craft_bug_report::hint::hint(
                    ui.ctx(),
                    &handle.hint(self.options.snapping.steps()),
                );
            }
        }
        // in the view, the status bar says what the tool in hand does next
        let handles_shown = handled || self.view_tool == Some(ViewTool::Transform);
        if canvas.hover().is_some()
            && !(handles_shown && self.on_any_handle(canvas, reach))
            && let Some(step) = self.step_hint(modes)
        {
            cut_craft_bug_report::hint::hint(ui.ctx(), &step);
        }
    }

    /// Whether the pointer is over one of the selection's handles (which then says its own).
    fn on_any_handle(&self, canvas: &Canvas, reach: f64) -> bool {
        canvas
            .cursor()
            .and_then(|at| self.handle_at(at, reach))
            .is_some()
    }

    /// What the tool in hand does next, for the status bar: a view tool's use, a drawing
    /// tool's next click or drag (a polyline's by how far it has got), or selecting's.
    pub(super) fn step_hint(&self, modes: Modes) -> Option<String> {
        const EDIT: &str = "; Shift-click one this tool drew to edit it";
        let drag = |what: &str| Some(format!("{what}{EDIT}"));
        match self.view_tool {
            Some(ViewTool::Pan) => return Some("Pan: drag to move the view".into()),
            Some(ViewTool::ZoomBox) => {
                return Some("Zoom Box: drag a box round what to zoom in on".into());
            }
            Some(ViewTool::NodeEdit) => {
                return Some(
                    "Node Editing: click a node to pick it, or drag a box round nodes (Shift adds); \
                     drag a picked node to move them; right-click for more"
                        .into(),
                );
            }
            Some(ViewTool::Measure) => {
                return Some(
                    "Measure: drag from one point to another to measure, or click a vector to \
                     inspect it"
                        .into(),
                );
            }
            Some(ViewTool::Transform) => {
                return Some(
                    "Transform: drag a handle to scale or rotate, the middle to move; click the \
                     middle for the rotation handles"
                        .into(),
                );
            }
            None => {}
        }
        if modes.tracing {
            return Some(
                "Trace Bitmap: drag a box over the picture to trace that part, or click a colour \
                 to trace it"
                    .into(),
            );
        }
        if modes.filleting {
            return Some(
                "Create Fillets: click a sharp corner to round it, or a fillet to take it out"
                    .into(),
            );
        }
        if modes.placing_tabs {
            return Some(
                "Tabs: click a selected vector to add a tab, click a tab to take it off, drag a \
                 tab to move it"
                    .into(),
            );
        }
        if modes.joining_ends {
            return Some(if self.join_ends_first().is_some() {
                "Join Open Ends: now click the end to join it to (the same vector's other end \
                 closes it); Escape or a click on the first end lets it go"
                    .into()
            } else {
                "Join Open Ends: click the end of an open vector (each has a circle on it)".into()
            });
        }
        // a form whose tool wants a selection it has not got says what to select
        if let Some(form) = &self.form
            && self.doc.selection.is_empty()
            && let Some(step) = self.first_step_of(form.id)
        {
            return Some(step);
        }
        // a toolbox variant says how it reads the drag
        if let Some(form) = self.form.as_ref().filter(|f| f.way != DragWay::AsIs)
            && let Some(variant) = crate::flyout::VARIANTS
                .iter()
                .find(|v| v.command == form.id && v.way == form.way)
        {
            return drag(variant.hint);
        }
        if let Some(id) = self.drawing_tool() {
            return self.drawing_step(id, self.polyline.is_empty());
        }
        if !modes.handled {
            None
        } else if self.doc.inside_group.is_some() {
            Some(INSIDE_GROUP.into())
        } else if self.nothing_drawn() {
            Some(EMPTY_SHEET_STEP.into())
        } else if self.doc.selection.is_empty() {
            Some(
                "Click a vector to select it, Shift-click to add more, or drag a box round \
                 several (right to left takes those it touches too)"
                    .into(),
            )
        } else {
            Some(
                "Drag the selection to move it, a handle to scale or rotate; Shift-click to add \
                 or take away"
                    .into(),
            )
        }
    }

    /// How drawing tool `id` is used: its next click or drag (a polyline's by whether it has
    /// started); for any other drawing tool, its description and a drag; nothing for a tool
    /// that is not drawn with the mouse.
    pub(super) fn drawing_step(&self, id: &str, polyline_empty: bool) -> Option<String> {
        const EDIT: &str = "; Shift-click one this tool drew to edit it";
        let drag = |what: &str| Some(format!("{what}{EDIT}"));
        match id {
            "draw_polyline" if polyline_empty => Some(
                "Draw Polyline: click the first point (drag for a curved span)".into(),
            ),
            "draw_polyline" => Some(
                "Click the next point (drag for a curved span); click the first point to close it, \
                 or double-click, right-click, Space or Escape to finish it open; Backspace takes a point back; \
                 or type the next one (20, x,y or 90A 20L) and press Enter"
                    .into(),
            ),
            "draw_rectangle" => drag("Draw Rectangle: drag from one corner to the opposite one"),
            "draw_circle" => drag("Draw Circle: drag from the centre out to the radius"),
            "draw_ellipse" => drag("Draw Ellipse: drag a box for it to fit"),
            "draw_polygon" => drag("Draw Polygon: drag from the centre out to a corner"),
            "draw_star" => drag("Draw Star: drag from the centre out to a point"),
            "dimensions" => drag("Dimensions: drag from the first point to the second"),
            "draw_freehand" => Some("Draw Freehand: drag to draw".into()),
            "sculpt" => Some("Sculpting: drag over the model to work it with the brush".into()),
            "cross_section" => Some("Cross Section: drag a line across the model".into()),
            "interactive_vector_trim" => Some(
                "Trim: click a piece of a vector to cut it away back to the crossings either \
                 side, or drag across pieces to cut each one"
                    .into(),
            ),
            "draw_text" => Some(
                "Draw Text: click to place the text; click between two letters to bring them \
                 together (Shift: apart, Ctrl: by more); Shift-click text to edit it"
                    .into(),
            ),
            id => gui::about(id)
                .filter(|_| {
                    self.registry
                        .get(id)
                        .is_some_and(cut_craft_core::DynCommand::interactive)
                })
                .map(|about| format!("{about}: drag in the view")),
        }
    }

    /// What the 2D view's clicks go to now (see [`Modes`]).
    pub(super) fn modes(&self) -> Modes {
        let placing_tabs = self.placing_tabs();
        let filleting = self.filleting();
        let joining_ends = self.joining_ends();
        let tracing = self.tracing();
        // the selection's box and handles, whenever nothing else uses the view's clicks
        let handled = self.view_tool.is_none()
            && self.drawing_tool().is_none()
            && !tracing
            && !filleting
            && !joining_ends
            && !placing_tabs;
        Modes {
            handled,
            tracing,
            filleting,
            placing_tabs,
            joining_ends,
        }
    }

    /// What the tool in hand does next, as [`Self::step_hint`] has it, away from the view.
    pub(super) fn next_step(&self) -> Option<String> {
        self.step_hint(self.modes())
    }

    /// Whether `at` is on one of the selection's scale or rotate handles.
    pub(super) fn on_handle(&self, at: Point, reach: f64) -> bool {
        self.handle_at(at, reach).is_some_and(|h| h != Handle::Move)
    }

    /// The selection's handles there are, and how far the rotation ones sit beyond its
    /// corners: in Transform mode the scale or (after a click on the middle) the rotation
    /// ones, else all of them at once.
    fn selection_handles(&self, reach: f64) -> (Vec<Handle>, f64) {
        if self.view_tool == Some(ViewTool::Transform) {
            (handles::handles(self.rotating), 0.0)
        } else {
            (handles::all(), ROTATE_OUT * reach)
        }
    }

    /// The selection's handle at `at`, if any.
    pub(super) fn handle_at(&self, at: Point, reach: f64) -> Option<Handle> {
        let bounds = self.doc.selection_bounds()?;
        let (among, out) = self.selection_handles(reach);
        handles::pick_among(&among, bounds, at, reach, out)
    }

    /// The transform a drag in Transform mode makes, if it started on one of the selection's
    /// handles.
    pub(super) fn handle_drag(&self, ui: &egui::Ui, drag: Drag, reach: f64) -> Option<DAffine2> {
        let bounds = self.doc.selection_bounds()?;
        let handle = self.handle_at(drag.start, reach)?;
        let steps = self.options.snapping.steps();
        let keys = ui.input(|i| handles::Keys {
            alt: i.modifiers.alt,
            shift: i.modifiers.shift,
            ctrl: i.modifiers.ctrl || i.modifiers.command,
            steps,
        });
        Some(handles::transform(
            bounds, handle, drag.start, drag.end, keys,
        ))
    }

    /// The selection's box and handles: squares scale (the corners and sides), circles beyond
    /// the corners rotate, and a cross in the middle moves.
    pub(super) fn paint_handles(&self, ui: &egui::Ui, canvas: &Canvas) {
        let Some(bounds) = self.doc.selection_bounds() else {
            return;
        };
        let (among, out) = self.selection_handles(canvas.pick_tolerance(ui));
        let painter = ui.painter_at(canvas.area);
        let visuals = ui.visuals();
        let stroke = visuals.selection.stroke;
        let corners = [
            bounds.min,
            Point::new(bounds.max.x, bounds.min.y),
            bounds.max,
            Point::new(bounds.min.x, bounds.max.y),
            bounds.min,
        ]
        .map(|p| canvas.transform.to_screen(p));
        let dash = ui.spacing().interact_size.y / 4.0;
        painter.extend(egui::Shape::dashed_line(&corners, stroke, dash, dash));
        let half = ui.spacing().interact_size.y * SELECTION_HANDLE_SHARE / 2.0;
        for handle in among {
            let at = canvas
                .transform
                .to_screen(handles::placed(bounds, handle, out));
            if let Handle::Rotate { .. } = handle
                && out > 0.0
            {
                // tied to its corner
                let corner = canvas
                    .transform
                    .to_screen(handles::position(bounds, handle));
                painter.line_segment([corner, at], stroke);
            }
            let square = egui::Rect::from_center_size(at, egui::vec2(half, half) * 2.0);
            match handle {
                // arrows both ways along X (red, + to the right) and Y (green, + up)
                Handle::Move => move_cross(
                    &painter,
                    at,
                    [egui::Vec2::X, -egui::Vec2::Y],
                    half,
                    stroke.width,
                ),
                Handle::Scale { .. } => {
                    painter.rect_filled(square, 0.0, visuals.extreme_bg_color);
                    painter.rect_stroke(square, 0, stroke, egui::StrokeKind::Middle);
                }
                // turns about Z: blue, as Z is everywhere
                Handle::Rotate { .. } => {
                    painter.circle(
                        at,
                        half,
                        crate::axes::Axis::Z.colour(),
                        egui::Stroke::new(stroke.width, visuals.extreme_bg_color),
                    );
                }
            }
        }
    }
}
