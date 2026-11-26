//! The 3D view's handles: moving, sizing and turning models with them, the 3D settings window,
//! shaded pictures and sculpting.

use super::{
    Anchor, Change, Cow, CutCraftApp, EntityId, HEIGHT_BUTTON, HEIGHT_DRAG_EVERY, Handle, Handle3d,
    HoverHint, Number, Plane3d, Point, Settings3d, VIEW_ZOOM_QUARTERS, View, ViewTool, box_corners,
    combine_radios_3d, egui, handles, save_part, top_over, tr, view2d, view3d, wrap_for,
};

impl CutCraftApp {
    /// Transform mode's handles in the 3D view, over whatever is selected: the 3D
    /// Transform Handles (see [`Self::transform_handles_3d`]) for vectors and components
    /// alike, and for a component, the component editing icons, round buttons
    /// along the near edge of its box; a drag on the first three sets its total, shape or
    /// base height, and the last opens its settings. The model's own view has them, not a
    /// preview. With nothing selected the status bar says how to start.
    pub(in super::super) fn handles_3d(
        &mut self,
        ui: &mut egui::Ui,
        area: egui::Rect,
        fit: egui::Rect,
    ) {
        self.handles_3d.clear();
        let importing = self.form.as_ref().is_some_and(|f| f.model.is_some());
        if self.preview.material.is_some()
            || self.player.playback.is_some()
            || importing
            || self.view_tool != Some(ViewTool::Transform)
        {
            // a drag the mode was left in the middle of ends here, as far as it went
            self.handle_drag.end();
            return;
        }
        let Some(bounds) = self.doc.selection_bounds() else {
            self.handle_drag.let_go();
            if ui.rect_contains_pointer(area) {
                cut_craft_bug_report::hint::hint(
                    ui.ctx(),
                    "Transform: click a component or vector to select it, then drag its \
                     handles to move, scale or rotate it",
                );
            }
            return;
        };
        let Some(surface) = self.surface_3d().map(Cow::into_owned) else {
            return;
        };
        let camera = view3d::Camera::new(&surface, self.orbit, self.framing, fit, None);
        let plane = Plane3d {
            camera: &camera,
            origin: surface.origin,
            z: top_over(&surface, bounds),
        };
        let painter = ui.painter_at(area);
        let chosen = self
            .doc
            .selected_components
            .iter()
            .rev()
            .find(|id| self.doc.components.contains_key(id))
            .copied();
        if let Some(id) = chosen {
            self.component_icons_3d(ui, &painter, &plane, id);
        }
        self.transform_handles_3d(ui, &painter, &plane, bounds);
        if let Some(id) = chosen {
            self.handle_drag_3d(ui, fit, id);
        }
    }

    /// The component editing icons along the near edge of component `id`'s box, just
    /// below it (the transform handles sit on the edge itself).
    pub(super) fn component_icons_3d(
        &mut self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        plane: &Plane3d,
        id: EntityId,
    ) {
        let extent = self.doc.components[&id].extent();
        let (near, far) = (
            plane.screen(extent.min),
            plane.screen(Point::new(extent.max.x, extent.min.y)),
        );
        // big enough to tell the pictures apart (HEIGHT_BUTTON times a button's height), a
        // button's size apart; an edge seen end-on runs to the right
        let edge = ui.spacing().interact_size.y * HEIGHT_BUTTON;
        let mut along = (far - near).normalized();
        if !along.is_finite() || along == egui::Vec2::ZERO {
            along = egui::Vec2::X;
        }
        let mut below = along.rot90();
        if below.dot(near - plane.screen(extent.center())) < 0.0 {
            below = -below;
        }
        for (i, handle) in Handle3d::ALL.into_iter().enumerate() {
            #[expect(clippy::cast_precision_loss, reason = "four handles")]
            let centre = near + along * edge * (0.75 + 1.25 * i as f32) + below * edge * 0.8;
            let rect = egui::Rect::from_center_size(centre, egui::Vec2::splat(edge));
            let response = ui
                .interact(
                    rect,
                    ui.scope_id().with(("handle_3d", i)),
                    egui::Sense::click_and_drag(),
                )
                .on_hover_hint(handle.hint(self.options.snapping.steps()));
            let look = ui.style().interact(&response);
            let fill = if response.hovered() || response.dragged() {
                look.bg_fill
            } else {
                ui.visuals().extreme_bg_color
            };
            painter.circle(centre, edge / 2.0, fill, look.bg_stroke);
            if let Some(icon) = handle.icon() {
                self.icons.image(icon).paint_at(ui, rect);
            }
            self.handles_3d.push((handle, centre));
            if response.drag_started()
                && let Some(origin) = ui.input(|i| i.pointer.press_origin())
            {
                self.handle_drag.start(handle, origin);
            }
            if response.clicked() && handle == Handle3d::Settings {
                let params = cmd_component_properties::Params::of(id, &self.doc.components[&id]);
                self.settings_3d = Some(Settings3d {
                    component: id,
                    params,
                    dirty: false,
                    dragged: None,
                    at: rect.left_bottom(),
                    radios: vec![],
                    anchors: None,
                });
            }
        }
    }

    /// The 3D Transform Handles over the selection's box, level with the highest
    /// surface inside it so they sit on what is selected rather than under it: the 2D
    /// view's handles at the same places (a cross in the middle moves, white corners and
    /// black edges scale) and a rotation arrow on a stem above the far edge. Dragged as the
    /// 2D view's are, with the same keys, and applied once the pointer is let go (one
    /// undo); while dragging, the box as it will be. Escape cancels a drag.
    // ponytail: the manual's edit box for an exact value, and moving the rotation centre,
    // are not offered; the Move, Scale and Rotate forms take exact values
    pub(super) fn transform_handles_3d(
        &mut self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        plane: &Plane3d,
        bounds: cut_craft_core::geometry::Rect,
    ) {
        let visuals = ui.visuals();
        let (stroke, dark) = (visuals.selection.stroke, visuals.text_color());
        let edge = ui.spacing().interact_size.y;
        let half = edge * view2d::NODE_HANDLE_SHARE / 2.0;
        // the selection's box itself, so the handles read as its corners and sides
        let corners = box_corners(bounds).map(|c| plane.screen(c)).to_vec();
        painter.add(egui::Shape::dashed_line(
            &[corners.as_slice(), &corners[..1]].concat(),
            stroke,
            6.0,
            4.0,
        ));
        // the arrow a button's height out from the far edge's middle, on a stem
        let arrow = Handle::Rotate { x: 0, y: 1 };
        let far = plane.screen(handles::position(bounds, arrow));
        let mut out = (far - plane.screen(bounds.center())).normalized();
        if !out.is_finite() || out == egui::Vec2::ZERO {
            out = -egui::Vec2::Y;
        }
        let tip = far + out * edge;
        painter.line_segment([far, tip], stroke);
        let all = handles::handles(false).into_iter().chain([arrow]);
        for (i, handle) in all.enumerate() {
            let at = if handle == arrow {
                tip
            } else {
                plane.screen(handles::position(bounds, handle))
            };
            let square = egui::Rect::from_center_size(at, egui::Vec2::splat(half * 2.0));
            match handle {
                // arrows both ways along the job's X and Y as they lie on screen
                Handle::Move => {
                    let middle = bounds.center();
                    let way = |d: Point| plane.screen(middle + d) - plane.screen(middle);
                    let along = [way(Point::new(1.0, 0.0)), way(Point::new(0.0, 1.0))];
                    // as big as a button, to be seen over the model
                    super::super::views::move_cross(painter, at, along, edge, stroke.width);
                }
                Handle::Scale { x, y } if x != 0 && y != 0 => {
                    painter.rect_filled(square, 0.0, visuals.extreme_bg_color);
                    painter.rect_stroke(square, 0, stroke, egui::StrokeKind::Middle);
                }
                Handle::Scale { .. } => {
                    painter.rect_filled(square, 0.0, dark);
                }
                Handle::Rotate { .. } => {
                    // a ring with an arrow head on top, blue as it turns about Z
                    let blue = crate::axes::Axis::Z.colour();
                    painter.circle_stroke(at, half, egui::Stroke::new(stroke.width * 1.5, blue));
                    let top = at - egui::vec2(0.0, half);
                    painter.add(egui::Shape::convex_polygon(
                        vec![
                            top + egui::vec2(half, 0.0),
                            top + egui::vec2(0.0, half / 2.0),
                            top - egui::vec2(0.0, half / 2.0),
                        ],
                        blue,
                        egui::Stroke::NONE,
                    ));
                }
            }
            // picked within the 2D view's reach of it: half a button's height
            let response = ui.interact(
                egui::Rect::from_center_size(at, egui::Vec2::splat(edge)),
                ui.scope_id().with(("transform_3d", i)),
                egui::Sense::click_and_drag(),
            );
            // the status bar says what it does and the keys that change it
            if response.hovered() || response.dragged() {
                cut_craft_bug_report::hint::hint(
                    ui.ctx(),
                    &handle.hint(self.options.snapping.steps()),
                );
            }
            self.handles_3d.push((Handle3d::Transform(handle), at));
            if response.drag_started()
                && let Some(from) = ui.input(|i| i.pointer.press_origin())
            {
                self.handle_drag.start(Handle3d::Transform(handle), from);
            }
        }
        self.transform_drag_3d(ui, painter, plane, bounds);
    }

    /// A transform handle's drag in the 3D view: the box as it will be while dragging, the
    /// transform applied (one undo) when the pointer is let go over the handles' plane.
    pub(super) fn transform_drag_3d(
        &mut self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        plane: &Plane3d,
        bounds: cut_craft_core::geometry::Rect,
    ) {
        let Some((Handle3d::Transform(handle), from)) = self.handle_drag.held() else {
            return;
        };
        let stroke = ui.visuals().selection.stroke;
        let (now, released, keys) = ui.input(|i| {
            (
                i.pointer.latest_pos(),
                i.pointer.primary_released(),
                handles::Keys {
                    alt: i.modifiers.alt,
                    shift: i.modifiers.shift,
                    ctrl: i.modifiers.ctrl || i.modifiers.command,
                    steps: self.options.snapping.steps(),
                },
            )
        });
        let Some((start, end)) = plane.back(from).zip(now.and_then(|at| plane.back(at))) else {
            // off the handles' plane (over the horizon in perspective, or the plane seen
            // edge-on): nothing to show, and letting go there changes nothing
            if released {
                self.handle_drag.let_go();
                self.status
                    .set_message(&cut_craft_i18n::tr!("status-let-go-off-material"));
            } else {
                self.status
                    .set_message(&cut_craft_i18n::tr!("status-pointer-off-material"));
            }
            return;
        };
        let t = handles::transform(bounds, handle, start, end, keys);
        let moved = box_corners(bounds)
            .map(|c| plane.screen(t.transform_point2(c)))
            .to_vec();
        painter.add(egui::Shape::closed_line(moved, stroke));
        if released {
            self.handle_drag.let_go();
            let change = Change::transform_selected(&self.doc, |_| t);
            self.record(change, "Transformed");
        }
    }

    /// Escape during a drag on a 3D view handle: the drag ends with nothing changed (a
    /// height already set as it went is undone), Transform mode stays.
    pub(in super::super) fn cancel_drag_3d(&mut self) {
        let Some(applied) = self.handle_drag.cancel() else {
            return;
        };
        if applied {
            self.history.undo(&mut self.doc);
        }
        self.status
            .set_message(&cut_craft_i18n::tr!("status-cancelled-nothing-done"));
    }

    /// A height handle's drag: the component follows the pointer (made again at most every
    /// [`HEIGHT_DRAG_EVERY`] seconds), its total, shape or base
    /// height set through Component Properties each frame the pointer moves (the last
    /// setting undone first, so the whole drag is one undo), its value in the status bar.
    /// A drag up a quarter of the view doubles, as the wheel's zoom does; the base height
    /// moves the material's thickness over that quarter.
    pub(super) fn handle_drag_3d(&mut self, ui: &egui::Ui, fit: egui::Rect, id: EntityId) {
        let Some((handle, origin)) = self.handle_drag.held() else {
            return;
        };
        if !matches!(handle, Handle3d::Total | Handle3d::Height | Handle3d::Base) {
            return;
        }
        let (now, released) = ui.input(|i| (i.pointer.latest_pos(), i.pointer.primary_released()));
        let Some(by) = now.map(|now| now - origin) else {
            return;
        };
        let doc = &self.doc;
        let (base, top, applied) = self.handle_drag.height(|| {
            let k = &doc.components[&id];
            let top = k.height_range().map_or(0.0, |(_, top)| top);
            (cmd_component_properties::Params::of(id, k), top)
        });
        let quarter = fit.size() / VIEW_ZOOM_QUARTERS;
        let factor = |px: f32, over: f32| f64::from(2f32.powf(px / over));
        let mut p = base.clone();
        let message = match handle {
            Handle3d::Total => {
                let f = factor(-by.y, quarter.y);
                p.shape_height = base.shape_height * f;
                p.base_height = base.base_height * f;
                format!("Total height {}", doc.job.units.length(top * f, 2))
            }
            Handle3d::Height => {
                p.shape_height = base.shape_height * factor(-by.y, quarter.y);
                format!("Shape height {:.0}%", p.shape_height)
            }
            _ => {
                let per_pixel = doc.job.thickness / f64::from(quarter.y);
                p.base_height = base.base_height - f64::from(by.y) * per_pixel;
                format!("Base height {}", doc.job.units.length(p.base_height, 2))
            }
        };
        let params = serde_json::to_value(&p).expect("component properties serialize");
        let (same, undo) = (applied == Some(&params), applied.is_some());
        // the model is made again at most ten times a second while the drag goes on (the
        // value in the status bar follows the pointer every frame), and as let go
        let time = ui.input(|i| i.time);
        let since = self.handle_drag.since_applied(time);
        let due = released || since >= HEIGHT_DRAG_EVERY;
        let mut failed = false;
        if !same && due {
            if undo {
                self.history.undo(&mut self.doc);
            }
            let done = match self.apply("component_properties", &params) {
                Ok(()) => Some(params),
                Err(e) => {
                    self.status.set_message(&e);
                    failed = true;
                    None
                }
            };
            self.handle_drag.applied(done, time);
        } else if !same {
            // a value not yet shown: shown once the wait is over, though the pointer stops
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_secs_f64(
                    HEIGHT_DRAG_EVERY - since,
                ));
        }
        if !failed {
            self.status.set_message(&message);
        }
        if released {
            self.handle_drag.end();
        }
    }

    /// The Settings handle's popup: the combine mode, Fade and Tilt with
    /// their anchors' Set, Appearance and its colour, and Close; each change applied
    /// through Component Properties as it is made, a dragged value as it goes (the drag
    /// one undo).
    pub(in super::super) fn settings_3d_window(&mut self, ctx: &egui::Context) {
        use cmd_component_properties::Appearance;
        let Some(s) = &mut self.settings_3d else {
            return;
        };
        if !self.doc.components.contains_key(&s.component) {
            self.settings_3d = None;
            return;
        }
        let mut close = false;
        egui::Window::new("component_settings_3d")
            .title_bar(false)
            .resizable(false)
            .fixed_pos(s.at)
            .show(ctx, |ui| {
                ui.strong(tr!("panels-combine-with-other-components"));
                combine_radios_3d(ui, &self.icons, s);
                egui::Grid::new("component_settings_3d").show(ui, |ui| {
                    let p = &mut s.params;
                    s.dirty |= ui.checkbox(&mut p.fade.on, tr!("panels-fade")).changed();
                    // Set: then two clicks in the 3D view
                    let setting = s.anchors.map(|(which, _)| which);
                    let set_fade = egui::Button::new(tr!("panels-set"))
                        .selected(setting == Some(Anchor::Fade));
                    if ui
                        .add_enabled(p.fade.on, set_fade)
                        .on_hover_hint(Anchor::Fade.prompt(false))
                        .clicked()
                    {
                        s.anchors = Some((Anchor::Fade, None));
                    }
                    let percent = Number::new(&mut p.fade.percent)
                        .range(0.0..=100.0)
                        .suffix(" %")
                        .name("Fade");
                    s.dirty |= ui.add_enabled(p.fade.on, percent).changed();
                    ui.end_row();
                    s.dirty |= ui.checkbox(&mut p.tilt.on, tr!("panels-tilt")).changed();
                    let set_tilt = egui::Button::new(tr!("panels-set"))
                        .selected(setting == Some(Anchor::Tilt));
                    if ui
                        .add_enabled(p.tilt.on, set_tilt)
                        .on_hover_hint(Anchor::Tilt.prompt(false))
                        .clicked()
                    {
                        s.anchors = Some((Anchor::Tilt, None));
                    }
                    let angle = Number::new(&mut p.tilt.angle)
                        .range(-90.0..=90.0)
                        .suffix(" °")
                        .name("Tilt");
                    s.dirty |= ui.add_enabled(p.tilt.on, angle).changed();
                    ui.end_row();
                    ui.label(tr!("panels-appearance"));
                    egui::ComboBox::from_id_salt("appearance_3d")
                        .selected_text(p.appearance.label())
                        .show_ui(ui, |ui| {
                            for a in Appearance::ALL {
                                s.dirty |= ui
                                    .selectable_value(&mut p.appearance, a, a.label())
                                    .changed();
                            }
                        });
                    ui.end_row();
                    ui.label("");
                    ui.horizontal(|ui| {
                        let solid = p.appearance == Appearance::SolidColor;
                        s.dirty |= ui
                            .add_enabled_ui(solid, |ui| ui.color_edit_button_srgb(&mut p.colour))
                            .inner
                            .changed();
                        ui.label(tr!("panels-color"));
                    });
                    ui.end_row();
                });
                close = ui.button(tr!("panels-close")).clicked();
            });
        let pressed = ctx.input(|i| i.pointer.any_down());
        if !pressed {
            s.dragged = None;
        }
        let apply = s.dirty.then(|| s.params.clone());
        let dragged = apply.as_ref().and(s.dragged.take());
        if close {
            self.settings_3d = None;
        } else if apply.is_some() {
            s.dirty = false;
        }
        if let Some(p) = apply {
            // the last frame's step of this drag taken back first, so the drag is one undo
            if dragged == Some(self.history.revision()) {
                self.history.undo(&mut self.doc);
            }
            let before = self.history.undo_depth();
            let params = serde_json::to_value(&p).expect("component properties serialize");
            if let Err(e) = self.apply("component_properties", &params) {
                self.status.set_message(&e);
            }
            let (made, revision) = (self.history.undo_depth() > before, self.history.revision());
            if let (true, Some(s)) = (pressed && made, self.settings_3d.as_mut()) {
                s.dragged = Some(revision);
            }
        }
    }

    /// Takes away the previewed material's piece under `at` in the 3D view's picture `fit`
    /// (double-click on waste); false when the click is off the material or on
    /// material cut right through.
    /// The 3D view's drops, sculpting and saved image: clipart dropped on it goes where it
    /// was dropped on the material, while sculpting a left drag is a stroke (true then), and
    /// Save Shaded Image asks for the window's picture once the view is drawn.
    pub(super) fn view_3d_drop_and_sculpt(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        area: egui::Rect,
    ) -> bool {
        self.ask_for_shaded_image(ui.ctx());
        if let Some(path) = response.dnd_release_payload::<std::path::PathBuf>() {
            let fit = area.shrink(ui.spacing().item_spacing.x * 2.0);
            let at = ui
                .ctx()
                .pointer_latest_pos()
                .and_then(|at| self.point_on_surface(fit, at));
            self.import_clipart(&path, at, None);
        }
        let sculpting = self.form.as_ref().is_some_and(|f| f.id == "sculpt");
        if sculpting {
            self.sculpt_3d(response, area.shrink(ui.spacing().item_spacing.x * 2.0));
        }
        sculpting
    }

    /// View ▸ Save Shaded Image to `path`: the 3D view is shown, its window's picture asked
    /// for once it has been drawn, and the 3D view's part saved when it comes.
    pub(in super::super) fn save_shaded_image(&mut self, path: std::path::PathBuf) {
        self.view = View::ThreeD;
        self.shaded_image = Some(path);
    }

    /// Asks for the window's picture once the 3D view has been drawn for Save Shaded Image;
    /// the 3D view's part of it is saved when it comes.
    pub(super) fn ask_for_shaded_image(&mut self, ctx: &egui::Context) {
        let (Some(path), Some(area)) = (self.shaded_image.take(), self.view_3d_area) else {
            return;
        };
        let (saved, ctx2) = (std::sync::Arc::clone(&self.shaded_saved), ctx.clone());
        let pixels = area * ctx.pixels_per_point();
        let callback = egui::ScreenshotCallback::new(move |picture| {
            let said = save_part(&picture, pixels, &path).map_or_else(
                |e| e,
                |()| format!("Saved the shaded image as {}", path.display()),
            );
            if let Ok(mut slot) = saved.lock() {
                *slot = Some(said);
            }
            ctx2.request_repaint();
        });
        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(callback));
    }

    /// What saving the shaded image said, once it has.
    pub(in super::super) fn shaded_image_arrived(&mut self) {
        let said = self
            .shaded_saved
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(said) = said {
            self.status.set_message(&said);
        }
    }

    /// Sculpting in the 3D view: a left drag over the model is a stroke, its points where
    /// the pointer met the surface, sculpted as one change when the button comes up.
    pub(super) fn sculpt_3d(&mut self, response: &egui::Response, fit: egui::Rect) {
        if response.drag_started_by(egui::PointerButton::Primary) {
            self.stroke.clear();
        }
        if response.dragged_by(egui::PointerButton::Primary)
            && let Some(at) = response
                .interact_pointer_pos()
                .and_then(|at| self.point_on_surface(fit, at))
            && self.stroke.last() != Some(&at)
        {
            self.stroke.push(at);
        }
        if response.drag_stopped_by(egui::PointerButton::Primary)
            && let (Some(&start), Some(&end)) = (self.stroke.first(), self.stroke.last())
        {
            let drag = cut_craft_core::Drag {
                start,
                end,
                from_center: false,
                constrain: false,
            };
            self.draw("sculpt", drag);
        }
    }

    /// The job point of the 3D view's surface under `at` on the picture `fit`, if any.
    pub(super) fn point_on_surface(
        &mut self,
        fit: egui::Rect,
        at: egui::Pos2,
    ) -> Option<cut_craft_core::geometry::Point> {
        let job = self.shown_job();
        let (orbit, framing, on) = (self.orbit, self.framing, self.display.auto_wrapping);
        let surface = self.surface_3d().map(Cow::into_owned)?;
        let wrap = wrap_for(&job, on, true, &surface);
        let (column, row) = view3d::pick(
            &surface,
            orbit,
            framing,
            fit,
            wrap,
            at,
            self.options.view_3d.pick_reach(),
        )?;
        Some(surface.centre(column, row))
    }

    pub(in super::super) fn remove_preview_piece(
        &mut self,
        fit: egui::Rect,
        at: egui::Pos2,
    ) -> bool {
        let job = self.shown_job();
        let (orbit, framing, on) = (self.orbit, self.framing, self.display.auto_wrapping);
        let Some(surface) = self.surface_3d().map(Cow::into_owned) else {
            return false;
        };
        let wrap = wrap_for(&job, on, true, &surface);
        let Some((column, row)) = view3d::pick(
            &surface,
            orbit,
            framing,
            fit,
            wrap,
            at,
            self.options.view_3d.pick_reach(),
        ) else {
            return false;
        };
        let p = surface.centre(column, row);
        let before = self.preview.material.clone();
        let removed = self
            .preview
            .material
            .as_mut()
            .map_or(0, |m| m.remove_piece(p));
        if removed > 0 {
            // scrubbing would put the piece back
            self.player.kept = None;
        }
        if removed > 0 {
            self.preview.before = Some(super::super::BeforePreview(before, self.preview.side));
            self.preview_changed();
            self.status
                .set_message(&cut_craft_i18n::tr!("status-removed-piece-cut-free"));
        }
        removed > 0
    }

    /// The topmost shown component at the point of the 3D view's picture `fit` under `at`.
    pub(in super::super) fn component_at_3d(
        &mut self,
        fit: egui::Rect,
        at: egui::Pos2,
    ) -> Option<EntityId> {
        self.components_at_3d(fit, at).first().copied()
    }
}
