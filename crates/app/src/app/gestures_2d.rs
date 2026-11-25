//! Mouse and touch in the 2D view: zoom and pan, pinch, strokes, click and double-click selection,
//! and stepping in and out of groups.

use super::views::INSIDE_GROUP;
use super::{
    Canvas, Change, CutCraftApp, Drag, EntityId, Gesture, STROKE_TOOLS, Value, ViewTool, egui,
};

impl CutCraftApp {
    /// The wheel zooms about the cursor, and tilting it pans sideways; a pinch zooms about
    /// the cursor too; in Trackpad mode two fingers pan (each as Options ▸ Mouse and
    /// Shortcuts says); the pan buttons (the wheel, to start with) or the Pan tool pan.
    pub(super) fn zoom_and_pan(&mut self, ui: &egui::Ui, canvas: &mut Canvas) {
        let area = canvas.area;
        if let Some(at) = canvas.hover() {
            let scroll = ui.input(|i| i.smooth_scroll_delta);
            let pinch = self.pinch(ui.ctx());
            let moved = self
                .options
                .controls
                .view_move(scroll, pinch, area.height());
            if moved.zoom != 1.0 {
                canvas.transform = canvas.transform.zoomed_about(at, moved.zoom);
                self.camera = Some(canvas.transform);
            }
            if moved.pan != egui::Vec2::ZERO {
                canvas.transform = canvas.transform.panned(moved.pan);
                self.camera = Some(canvas.transform);
            }
        }
        let pan = match self.view_tool {
            Some(ViewTool::Pan) => canvas.dragged_by(egui::PointerButton::Primary),
            _ => None,
        }
        .or_else(|| (self.pan_buttons().into_iter()).find_map(|b| canvas.dragged_by(b)));
        if let Some(by) = pan {
            canvas.transform = canvas.transform.panned(by);
            self.camera = Some(canvas.transform);
        }
    }

    /// This frame's pinch on a trackpad or touch screen, as a zoom factor (1 for none). Ctrl
    /// and the wheel count too (some trackpads send a pinch so), except while Sculpting,
    /// whose brush they size.
    pub(super) fn pinch(&self, ctx: &egui::Context) -> f32 {
        let sculpting = self.form.as_ref().is_some_and(|f| f.id == "sculpt");
        ctx.input(|i| {
            if sculpting {
                (i.raw.events.iter())
                    .filter_map(|e| match e {
                        egui::Event::Zoom(factor) => Some(*factor),
                        _ => None,
                    })
                    .product()
            } else {
                i.zoom_delta()
            }
        })
    }

    /// Remembers where the pointer goes while dragging, for stroke tools.
    pub(super) fn track_stroke(&mut self, gesture: Option<Gesture>) {
        if let Some(Gesture::Dragging(drag)) = gesture {
            // a new drag starts a new stroke
            if self.stroke.first() != Some(&drag.start) {
                self.stroke = vec![drag.start];
            }
            if self.stroke.last() != Some(&drag.end) {
                self.stroke.push(drag.end);
            }
        }
    }

    /// A stroke tool's params with the points dragged through so far.
    pub(super) fn with_stroke(&self, id: &str, params: &Value) -> Value {
        let mut params = params.clone();
        if STROKE_TOOLS.contains(&id) {
            let points: Vec<_> = self
                .stroke
                .iter()
                .map(|q| serde_json::json!({ "x": q.x, "y": q.y }))
                .collect();
            params["points"] = Value::from(points);
        }
        params
    }

    /// Finishes a drag with a drawing tool: the form shows the dragged values, then they apply.
    pub(super) fn draw(&mut self, id: &'static str, drag: Drag) {
        let Some(cmd) = self.registry.get(id) else {
            return;
        };
        let Some((current, way)) = self
            .form
            .as_ref()
            .map(|f| (self.with_stroke(id, &f.params), f.way))
        else {
            return;
        };
        let drag = way.apply(drag);
        self.stroke.clear();
        let before = self.entity_ids();
        let result = cmd
            .drag_params(drag, &current)
            .map_err(|e| e.to_string())
            .and_then(|params| {
                if let Some(form) = &mut self.form {
                    form.params.clone_from(&params);
                }
                self.apply(id, &params)
            });
        if result.is_ok() {
            self.remember_drawn(&before);
        }
        if let Some(form) = &mut self.form {
            form.error = result.err();
        }
    }

    /// A click in the 2D view: picks the vector under it, else the picture; Shift adds or
    /// takes away.
    pub(super) fn click_select(
        &mut self,
        at: cut_craft_core::geometry::Point,
        shift: bool,
        tolerance: f64,
    ) {
        // a snap mark: picked, with its vector, for Delete to let its point go
        self.picked_constraint = None;
        if let Some(i) = self.kept_snap_at(at, tolerance) {
            let snap = self.doc.constraints[i];
            self.select([snap.vector], shift);
            self.picked_constraint = Some(super::PickedSnap {
                snap,
                selection: self.doc.selection.clone(),
                tool: self.view_tool,
                revision: self.history.revision(),
            });
            let kind = snap.to.kind();
            self.status.set_message(&format!(
                "Snap picked: this point is kept on the {}. Press Delete to let it go.",
                kind.label().to_lowercase()
            ));
            return;
        }
        // on a rotary job, a click on a ghost past a seam picks the vector it copies
        let pick = |p| self.doc.pick(p, tolerance);
        let hit = self.wrap().map_or_else(|| pick(at), |w| w.pick(at, pick));
        // a picture is picked where no vector is: click picks it, Shift-click adds or
        // takes it away
        let picture = hit.is_none().then(|| self.doc.pick_bitmap(at)).flatten();
        match (picture, shift) {
            (Some(b), true) => {
                if !self.doc.selected_bitmaps.remove(&b) {
                    self.doc.selected_bitmaps.insert(b);
                }
            }
            (picture, false) => self.doc.selected_bitmaps = picture.into_iter().collect(),
            (None, true) => {}
        }
        // a component's depth map is picked where no vector or picture is
        let component = (hit.is_none() && picture.is_none())
            .then(|| self.doc.pick_component(at))
            .flatten();
        match (component, shift) {
            (Some(k), true) => {
                if !self.doc.selected_components.remove(&k) {
                    self.doc.selected_components.insert(k);
                }
            }
            (component, false) => self.doc.selected_components = component.into_iter().collect(),
            (None, true) => {}
        }
        // a plain click on a picture or component, which are in no group, steps out
        if !shift && (picture.is_some() || component.is_some()) {
            self.doc.inside_group = None;
        }
        // inside a closed shape, away from any line, picture or component: the shape and
        // its holes, as one part
        let part = if hit.is_none() && picture.is_none() && component.is_none() {
            self.doc.pick_part(at, tolerance)
        } else {
            Vec::new()
        };
        if !part.is_empty() {
            let mut ids = self.doc.selection.clone();
            if !shift {
                ids.clear();
            }
            // Shift-clicking a part already picked takes it away
            if shift && part.iter().all(|id| ids.contains(id)) {
                for id in part.iter().flat_map(|&id| self.picked_with(id)) {
                    ids.remove(&id);
                }
            } else {
                ids.extend(part);
            }
            self.select(ids, false);
        } else if shift {
            // a grouped vector brings or takes its whole group
            let mut ids = self.doc.selection.clone();
            if let Some(id) = hit {
                let members = self.picked_with(id);
                if ids.contains(&id) {
                    for m in &members {
                        ids.remove(m);
                    }
                } else {
                    ids.extend(members);
                }
            }
            self.select(ids, false);
        } else {
            self.select(hit, false);
        }
    }

    /// The vectors a click on `id` picks: its whole group, or, stepped inside a group, the
    /// one item in it that holds `id`.
    pub(super) fn picked_with(&self, id: EntityId) -> Vec<EntityId> {
        self.doc
            .pick_inside(id, self.doc.inside_group)
            .unwrap_or_else(|| self.doc.group_members(id))
    }

    /// A double-click in the 2D view with nothing in hand, at `at`: on a group, steps inside
    /// it (picking the item clicked); on text, opens Edit Text on it; off everything while
    /// inside a group, steps out one level.
    pub(super) fn double_click_2d(&mut self, at: cut_craft_core::geometry::Point, reach: f64) {
        let inside = self.doc.inside_group;
        let Some(hit) = self.doc.pick(at, reach) else {
            if let Some(group) = inside {
                self.step_out_of_group(group);
            }
            return;
        };
        let Some(group) = self.doc.group_to_enter(hit, inside) else {
            return;
        };
        let text = self
            .doc
            .texts
            .get(&group)
            .is_some_and(|t| t.get("text").is_some());
        if text {
            // the text picked whole, in the group it is in, and Edit Text opened on it
            self.doc.inside_group = self.doc.group_parents.get(&group).copied();
            let letters = self.doc.members_under(group);
            self.select(letters, false);
            self.start_command("draw_text");
            return;
        }
        self.doc.inside_group = Some(group);
        self.select([hit], false);
        self.status.set_message(INSIDE_GROUP);
    }

    /// Steps out of the group stepped into once it is gone (undone, deleted, ungrouped).
    pub(super) fn forget_gone_group(&mut self) {
        let doc = &self.doc;
        if let Some(group) = doc.inside_group
            && !doc
                .vectors
                .keys()
                .any(|&v| doc.group_chain(v).contains(&group))
        {
            self.doc.inside_group = None;
        }
    }

    /// Leaves group `group` (stepped inside it) for the group around it, if any.
    pub(super) fn step_out_of_group(&mut self, group: u64) {
        let around = self.doc.group_parents.get(&group).copied();
        self.doc.inside_group = around;
        let members = self.doc.members_under(group);
        self.select(members, false);
        self.status.set_message(if around.is_some() {
            "Stepped out to the group around it"
        } else {
            "Stepped out of the group: a click picks whole groups again"
        });
    }

    /// Selection is view state: applied directly, never recorded for undo.
    /// Picking a grouped vector picks its whole group, or, stepped inside a group, the one
    /// item in it (a group inside it, whole, or the vector alone).
    pub(super) fn select(&mut self, ids: impl IntoIterator<Item = EntityId>, add: bool) {
        let ids: Vec<EntityId> = ids.into_iter().collect();
        // picking something outside the group stepped into steps out of it
        let inside = self.doc.inside_group;
        if inside.is_some()
            && ids
                .iter()
                .any(|&id| self.doc.pick_inside(id, inside).is_none())
        {
            self.doc.inside_group = None;
        }
        let doc = &self.doc;
        let whole = ids
            .into_iter()
            .flat_map(|id| doc.pick_inside(id, doc.inside_group).unwrap_or_default());
        let mut c = Change::builder(doc);
        if add {
            c.select(doc.selection.iter().copied().chain(whole));
        } else {
            c.select(whole);
        }
        self.doc
            .apply(&c.finish())
            .expect("picked ids come from the document");
    }
}
