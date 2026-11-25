//! Snapping: where a point snaps to, the snap marks, kept snap points and the Snap Options window.

use super::{
    Canvas, Change, CutCraftApp, Drag, Gesture, Point, STROKE_TOOLS, ViewTool, dialog_size, egui,
};
use cut_craft_core::snap::Snap;

impl CutCraftApp {
    /// Drawing and node drags snap to the snap points (Shift holds snapping off).
    #[cfg(test)]
    pub(super) fn snapped(
        &self,
        ui: &egui::Ui,
        gesture: Option<Gesture>,
        scale: f32,
    ) -> Option<Gesture> {
        self.snapping(ui, gesture, scale, None).0
    }

    /// The gesture snapped as [`Self::snapped`] says, and what the pointer (a drag's end, or
    /// `pointer` while hovering) snaps to, for the view's marker: drawing tools' clicks snap
    /// too, and a polyline's next point snaps tangent or square on from the last.
    pub(super) fn snapping(
        &self,
        ui: &egui::Ui,
        gesture: Option<Gesture>,
        scale: f32,
        pointer: Option<Point>,
    ) -> (Option<Gesture>, Option<Snap>, Vec<Snap>) {
        // drawing, node editing and measuring snap (Measure reads from one snap to another)
        let snapping = self.drawing_tool().is_some()
            || matches!(self.view_tool, Some(ViewTool::NodeEdit | ViewTool::Measure));
        if !snapping || ui.input(|i| i.modifiers.shift) {
            return (gesture, None, vec![]);
        }
        let reach = self.options.snapping.snapping_radius / f64::from(scale);
        // a drawn shape's far end also snaps square onto spans and tangent to arcs; a node
        // dragged starts from the node itself, and a stroke's samples are not a shape's
        // end, so those snap as points
        let tool = self.drawing_tool();
        let drawing = tool.is_some_and(|id| !STROKE_TOOLS.contains(&id));
        // a polyline's next point runs on from its last
        let last = (tool == Some("draw_polyline"))
            .then(|| self.polyline.points.last().copied())
            .flatten();
        // on a rotary job the ghosts past each seam snap too, as the drawing they copy
        let wrap = self.wrap();
        let plain =
            |from: Option<Point>, at: Point| self.options.snapping.find(&self.doc, from, at, reach);
        let find = |from: Option<Point>, at: Point| match wrap {
            Some(w) => w.snap(at, from, plain),
            None => plain(from, at),
        };
        let to = |hit: Option<Snap>, at: Point| hit.map_or(at, |h| h.at);
        let mut marker = None;
        // where a point was placed on a snap: kept there once drawn
        let mut placed = vec![];
        // in Node Edit, a press on a Bézier handle stays on the handle: snapping it onto a
        // nearby node would turn the handle drag into a node drag
        let handle_at = |at: Point| -> Option<f64> {
            (self.view_tool == Some(ViewTool::NodeEdit))
                .then(|| self.nearest_handle(at))
                .flatten()
        };
        let mut snap = |mut drag: Drag| {
            let mut first = find(last, drag.start);
            if handle_at(drag.start).is_some_and(|d| d <= reach) {
                first = None;
            }
            drag.start = to(first, drag.start);
            let hit = find(drawing.then_some(drag.start), drag.end);
            drag.end = to(hit, drag.end);
            marker = hit;
            (drag, first.into_iter().chain(hit))
        };
        let gesture = gesture.map(|g| match g {
            Gesture::Dragging(drag) => Gesture::Dragging(snap(drag).0),
            Gesture::Dropped { drag, shift } => {
                let (drag, hits) = snap(drag);
                placed.extend(hits);
                Gesture::Dropped { drag, shift }
            }
            // a drawing tool's click places a point, so it snaps; node editing's picks
            Gesture::Click { at, shift } if tool.is_some() => {
                let hit = find(last, at);
                placed.extend(hit);
                Gesture::Click {
                    at: to(hit, at),
                    shift,
                }
            }
            click @ Gesture::Click { .. } => click,
        });
        if !matches!(gesture, Some(Gesture::Dragging(_))) {
            marker = pointer.and_then(|at| find(last.filter(|_| drawing), at));
        }
        (gesture, marker, placed)
    }

    /// The snap marker where the pointer snapped: a shape for each kind (a square on an end,
    /// a triangle on a middle, a circle where a line touches a curve, a right angle where it
    /// meets one square on, a plus on a centre, a cross elsewhere) and its name beside it;
    /// the status bar says what it means.
    pub(super) fn paint_snap(ui: &egui::Ui, canvas: &Canvas, hit: Snap) {
        const SIZE: f32 = 6.0;
        let colour = egui::Color32::from_rgb(0, 160, 60);
        let painter = ui.painter_at(canvas.area);
        let at = canvas.transform.to_screen(hit.at);
        Self::snap_mark(&painter, at, hit.kind, SIZE, egui::Stroke::new(2.0, colour));
        painter.text(
            at + egui::vec2(SIZE + 4.0, SIZE + 4.0),
            egui::Align2::LEFT_TOP,
            hit.kind.label(),
            egui::FontId::proportional(12.0),
            colour,
        );
    }

    /// One snap marker of `kind`, `size` pixels from its middle to its edge.
    fn snap_mark(
        painter: &egui::Painter,
        at: egui::Pos2,
        kind: cut_craft_core::snap::SnapKind,
        size: f32,
        stroke: egui::Stroke,
    ) {
        use cut_craft_core::snap::SnapKind;
        let v = egui::vec2;
        match kind {
            SnapKind::End | SnapKind::JobPoint => {
                painter.rect_stroke(
                    egui::Rect::from_center_size(at, v(2.0 * size, 2.0 * size)),
                    0.0,
                    stroke,
                    egui::StrokeKind::Middle,
                );
            }
            SnapKind::Mid => {
                let points = vec![
                    at + v(0.0, -size),
                    at + v(size, 0.8 * size),
                    at + v(-size, 0.8 * size),
                ];
                painter.add(egui::Shape::closed_line(points, stroke));
            }
            SnapKind::Tangent => {
                painter.circle_stroke(at, size, stroke);
                painter.line_segment([at + v(-size, -size), at + v(size, -size)], stroke);
            }
            SnapKind::Perpendicular => {
                painter.line_segment([at + v(-size, size), at + v(size, size)], stroke);
                painter.line_segment([at + v(-size, size), at + v(-size, -size)], stroke);
                painter.line_segment([at + v(-size, 0.0), at + v(0.0, 0.0)], stroke);
                painter.line_segment([at + v(0.0, 0.0), at + v(0.0, size)], stroke);
            }
            SnapKind::ArcCenter | SnapKind::ObjectCenter => {
                painter.line_segment([at + v(-size, 0.0), at + v(size, 0.0)], stroke);
                painter.line_segment([at + v(0.0, -size), at + v(0.0, size)], stroke);
                painter.circle_stroke(at, size / 2.0, stroke);
            }
            SnapKind::Guide | SnapKind::Aligned | SnapKind::Grid => {
                painter.line_segment([at + v(-size, -size), at + v(size, size)], stroke);
                painter.line_segment([at + v(-size, size), at + v(size, -size)], stroke);
            }
        }
    }

    /// The marks of the points kept on their snaps: small blue snap markers; the picked one
    /// orange and named, as is one under the pointer, which the status bar explains.
    pub(super) fn paint_kept_snaps(&self, ui: &egui::Ui, canvas: &Canvas, reach: f64) {
        const SIZE: f32 = 4.0;
        let blue = egui::Color32::from_rgb(30, 110, 220);
        let orange = egui::Color32::from_rgb(240, 130, 0);
        let painter = ui.painter_at(canvas.area);
        let hover = canvas.cursor();
        let picked_snap = self.picked_snap();
        for (i, at, kind) in self.doc.constraint_marks() {
            if !self.doc.shown(self.doc.constraints[i].vector) {
                continue;
            }
            let picked = picked_snap == Some(i);
            let hovered = hover.is_some_and(|p| p.distance(at) <= reach);
            let colour = if picked { orange } else { blue };
            let screen = canvas.transform.to_screen(at);
            let width = if picked { 2.5 } else { 1.5 };
            Self::snap_mark(
                &painter,
                screen,
                kind,
                SIZE,
                egui::Stroke::new(width, colour),
            );
            if picked || hovered {
                painter.text(
                    screen + egui::vec2(SIZE + 4.0, -SIZE - 4.0),
                    egui::Align2::LEFT_BOTTOM,
                    format!("Kept on {}", kind.label()),
                    egui::FontId::proportional(12.0),
                    colour,
                );
            }
            if hovered && !picked {
                cut_craft_bug_report::hint::hint(
                    ui.ctx(),
                    &format!(
                        "This point stays on the {} it was snapped to, even when that moves. \
                         Click the mark (which picks its shape too) and press Delete to let \
                         it move freely.",
                        kind.label().to_lowercase()
                    ),
                );
            }
        }
    }

    /// The picked snap mark (an index into the job's constraints), while it still is: the
    /// selection, the tool and the job as they were when it was picked, and its point
    /// still kept.
    pub(super) fn picked_snap(&self) -> Option<usize> {
        let picked = self.picked_constraint.as_ref()?;
        if picked.selection != self.doc.selection
            || picked.tool != self.view_tool
            || picked.revision != self.history.revision()
        {
            return None;
        }
        self.doc.constraints.iter().position(|c| *c == picked.snap)
    }

    /// The snap mark (a point kept on its snap) within `reach` of `at`, nearest first.
    pub(super) fn kept_snap_at(&self, at: Point, reach: f64) -> Option<usize> {
        self.doc
            .constraint_marks()
            .into_iter()
            .filter(|(i, _, _)| self.doc.shown(self.doc.constraints[*i].vector))
            .map(|(i, p, _)| (i, p.distance(at)))
            .filter(|(_, d)| *d <= reach)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }

    /// Delete with a snap mark picked: that point is let go, so it moves freely. False when
    /// none is picked.
    pub(super) fn release_picked_snap(&mut self) -> bool {
        let Some(i) = self.picked_snap() else {
            self.picked_constraint = None;
            return false;
        };
        self.picked_constraint = None;
        let mut kept = self.doc.constraints.clone();
        let gone = kept.remove(i);
        let mut c = Change::builder(&self.doc);
        c.set_constraints(kept);
        self.record(
            Ok(c.finish()),
            &format!(
                "Let go of the {} snap: that point now moves freely",
                gone.to.kind().label().to_lowercase()
            ),
        );
        true
    }

    /// Points just placed on snaps by a drawing tool or a node drag (`placed`), remembered
    /// so what is made from them keeps them there. A polyline gathers its points' snaps
    /// from its first click; anything else keeps those of its last click or drag.
    pub(super) fn place_on_snaps(&mut self, gesture: Option<Gesture>, placed: &[Snap]) {
        // a measurement makes nothing to keep on its snaps
        let placing = matches!(
            gesture,
            Some(Gesture::Click { .. } | Gesture::Dropped { .. })
        ) && self.view_tool != Some(ViewTool::Measure);
        if !placing {
            return;
        }
        let kept = placed.iter().filter_map(|s| Some((s.at, s.on?)));
        if self.drawing_tool() == Some("draw_polyline") && !self.polyline.is_empty() {
            self.placed.extend(kept);
        } else {
            self.placed = kept.collect();
        }
    }

    /// The status bar's words for a snap: what it snapped to, and how to place a point
    /// without it.
    pub(super) fn snap_hint(hit: Snap) -> String {
        format!(
            "snapped to {}: {} (Shift places it freely)",
            hit.kind.label(),
            hit.kind.explain()
        )
    }

    /// Snapping Options: what drawing and node drags snap to.
    pub(super) fn snap_options_window(&mut self, ctx: &egui::Context) {
        use cut_craft_bug_report::unapplied::{self, Answer, Button};
        let mut open = self.snap.open;
        let schema = serde_json::to_value(schemars::schema_for!(cut_craft_core::snap::SnapOptions))
            .expect("a schema serialises");
        let fields = crate::form::fields(&schema);
        let shown = self.snap.shown(&self.options.snapping);
        let mut values = serde_json::to_value(shown).expect("options serialise");
        let dirty = self.snap.dirty();
        let closing = self.snap.closing;
        let (mut edited, mut button, mut answer) = (None, None, None);
        let units = self.doc.job.units;
        let (id, size) = (egui::Id::unique("snapping_options"), dialog_size(ctx));
        cut_craft_bug_report::dialog_with_panels(
            ctx,
            id,
            "Snapping Options",
            size,
            &mut open,
            |ui| {
                unapplied::hold_close(ui, dirty);
                egui::Panel::bottom("snapping_buttons").show(ui, |ui| {
                    button = unapplied::footer(ui, dirty);
                });
                if closing {
                    answer = unapplied::question(ui.ctx(), "snapping_apply_question");
                }
                let picture = |_: &str| None;
                let look = crate::form::Look {
                    units,
                    picture: &picture,
                    material: crate::form::material(&self.doc, units),
                    tool: None,
                    group_end: None,
                    materials: None,
                    pattern: None,
                };
                egui::CentralPanel::default().show(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        if crate::form::show(ui, &fields, &mut values, &look) {
                            edited =
                                serde_json::from_value::<cut_craft_core::snap::SnapOptions>(values)
                                    .ok();
                        }
                    });
                });
            },
        );
        // kept until Apply, unless it is as it already is
        if let Some(edited) = edited {
            self.snap.edited(&self.options.snapping, edited);
        }
        // the close box is Close: it asks first while changes wait
        if !open {
            button = Some(Button::Close);
        }
        let apply = |app: &mut Self| {
            if let Some(snapping) = app.snap.take_changes() {
                app.options.snapping = snapping;
                app.save_options();
            }
        };
        match button {
            Some(Button::Apply) => apply(self),
            Some(Button::Close) if self.snap.dirty() => self.snap.closing = true,
            Some(Button::Cancel | Button::Close) => self.snap.close(),
            None => {}
        }
        match answer {
            Some(Answer::ApplyAndClose) => {
                apply(self);
                self.snap.close();
            }
            Some(Answer::Discard) => self.snap.close(),
            Some(Answer::KeepEditing) => self.snap.closing = false,
            None => {}
        }
    }
}
