//! The 3D view: its surface and models, the buttons and mouse over it, and the meshes it shows.

use super::{
    COLLISION_RED, Cow, CutCraftApp, EntityId, FineKey, MESH_CELLS, MachinedArea, Material, Value,
    View, egui, faces_geometry, real_3d, tr, view3d,
};

impl CutCraftApp {
    /// A playing preview's material at the moving detail, painted, as the 3D view draws it:
    /// made again only once it has cut on (or the detail or the job changed).
    pub(super) fn playback_surface(&mut self) -> Option<Material> {
        let playback = self.player.playback.as_ref()?;
        let key = (
            self.shown_revision(playback),
            self.options.performance.still_cells(),
            self.history.revision(),
        );
        if let Some((kept, surface)) = &self.player.view
            && *kept == key
        {
            return Some(surface.clone());
        }
        let coarse = view3d::on_sheet(playback.material(), self.doc.job.area(), key.1);
        let surface = self.painted(coarse);
        self.player.view = Some((key, surface.clone()));
        self.player.builds += 1;
        Some(surface)
    }

    /// The playback's revision the 3D view is to show (`playback_shown`).
    pub(super) fn shown_revision(&self, playback: &cut_craft_core::playback::Playback) -> u64 {
        self.player
            .shown
            .map_or_else(|| playback.revision(), |(shown, _)| shown)
    }

    /// What the 3D view shows, at Options' detail while moving: the toolpath preview if
    /// there is one, else the model (kept until it changes: it is asked for several times a
    /// frame, and a big model takes long to mix).
    pub(in super::super) fn surface_3d(&mut self) -> Option<Cow<'_, Material>> {
        let cells = self.options.performance.moving_cells();
        if self.player.playback.is_some() {
            return self.playback_surface().map(Cow::Owned);
        }
        if let Some(material) = &self.preview.material {
            if self
                .preview
                .coarse
                .as_ref()
                .is_none_or(|(made, _)| *made != cells)
            {
                let sheet = view3d::on_sheet(material, self.doc.job.area(), cells);
                self.preview.coarse = Some((cells, sheet));
            }
            let (_, coarse) = self.preview.coarse.as_ref().expect("made above");
            if self.doc.job.machined_area == MachinedArea::MaterialColor
                && coarse.other_side.is_none()
                && self.preview.collisions.is_empty()
            {
                return Some(Cow::Borrowed(coarse));
            }
            return Some(Cow::Owned(self.painted(coarse.clone())));
        }
        // before a preview, the model alone: nothing where no component is; on a job with
        // no model yet, the uncut material Job Setup describes. Made once a change, not
        // every frame
        let key = super::super::RoughKey {
            revision: self.history.revision(),
            job: self.doc.job,
            cells,
            importing: (self.importing_model(), self.importing_whole()),
        };
        if self.meshes.rough.get(&key).is_none() {
            let shape = self.model_shape(cells);
            self.meshes.rough.keep(key, shape);
        }
        self.meshes.rough.get(&key).map(Cow::Borrowed)
    }

    /// Whether the 3D view shows the job's material uncut: no preview, and no model yet
    /// (the block Job Setup describes is there from the start, not made by the user).
    pub(in super::super) fn showing_uncut_material(&self) -> bool {
        self.preview.material.is_none()
            && self.player.playback.is_none()
            && self.doc.components.is_empty()
    }

    /// Whether the 3D view shows the job turned round its axis with its full 3D models as
    /// they really are: a rotary job with one (or one being imported), the model's own view
    /// (not a preview).
    pub(in super::super) fn shows_turned(&self) -> bool {
        if self.doc.job.rotary.is_none()
            || self.preview.material.is_some()
            || self.player.playback.is_some()
        {
            return false;
        }
        let importing = self.form.as_ref().is_some_and(|f| {
            f.id == "orientate_3d_model"
                && f.model.is_some()
                && f.model_whole
                && f.params["model_type"] == "full"
        });
        importing
            || self
                .doc
                .components
                .values()
                .any(|k| k.visible && real_3d(k))
    }

    /// The rotary job's full 3D models as they really are round its axis, in the view's
    /// millimetres (along the job's axis, across and up from it), but for `skip` (the one
    /// being imported, shown from the form's own faces). Nothing is marked out of reach: a
    /// rotary job is cut all the way round.
    pub(in super::super) fn rotary_models(
        &self,
        skip: Option<EntityId>,
    ) -> Vec<cmd_orientate_3d_model::Seen> {
        let Some(rotary) = self.doc.job.rotary else {
            return vec![];
        };
        let along_x = rotary.along == cut_craft_core::document::Along::X;
        self.doc
            .components
            .iter()
            .filter(|(id, k)| Some(**id) != skip && k.visible && real_3d(k))
            .flat_map(|(_, k)| k.solid_triangles())
            .map(|t| {
                // the part's own frame at A=0: along the axis, then across and up from it
                let placed = t.map(|v| {
                    if along_x {
                        v
                    } else {
                        cut_craft_core::glam::DVec3::new(v.y, v.x, v.z)
                    }
                });
                (placed, false)
            })
            .collect()
    }

    /// Whether Import 3D Model shows a model: whole, or (past Options ▸ Model preview faces)
    /// as its component with the faces out of reach over it. Either way it sits on its
    /// modeling plane at 0 inside the material's outline, not seated in the block.
    pub(in super::super) fn importing_model(&self) -> bool {
        self.form.as_ref().is_some_and(|f| f.model.is_some())
    }

    /// Whether an imported model is shown whole, in place of its component.
    pub(in super::super) fn importing_whole(&self) -> bool {
        self.form
            .as_ref()
            .is_some_and(|f| f.model.is_some() && f.model_whole)
    }

    /// The preview's `material` painted as the job's Machined Area Color has it: in the
    /// material's colour, one fill colour, or each toolpath's own.
    pub(super) fn painted(&self, material: Material) -> Material {
        let job = &self.doc.job;
        // a double-sided preview's other side, on the underside
        let material = match job.double_sided {
            Some(sides) => {
                let turn = sides.flip.mirror(job.area());
                material.with_underside(|p| turn.transform_point2(p))
            }
            None => material,
        };
        let rgb = |[r, g, b]: [u8; 3]| egui::Color32::from_rgb(r, g, b);
        let material = match job.machined_area {
            MachinedArea::MaterialColor => material,
            MachinedArea::GlobalFillColor => {
                view3d::painted(material, |_| Some(rgb(job.fill_colour)))
            }
            MachinedArea::ToolpathColor => {
                let fills = self.preview_fills();
                view3d::painted(material, |id| fills.get(&id).copied().flatten().map(rgb))
            }
            MachinedArea::Lithophane => {
                view3d::lit_through(material, rgb(job.colour), job.lithophane_light)
            }
        };
        self.collisions_painted(material)
    }

    /// `material` with the stock the collet, spindle or shank would hit painted red, once
    /// the preview has finished playing (before then the tool has not got there).
    pub(super) fn collisions_painted(&self, mut material: Material) -> Material {
        if self.preview.collisions.is_empty()
            || self.player.playback.as_ref().is_some_and(|p| !p.finished())
        {
            return material;
        }
        let hits: Vec<cut_craft_core::collision::Hit> = self
            .current_collisions()
            .flat_map(|(_, c)| c.hits.iter().copied())
            .collect();
        let struck = cut_craft_core::collision::struck_cells(&material, &hits);
        if struck.is_empty() {
            return material;
        }
        if material.paint.len() != material.heights.len() {
            material.paint = vec![0; material.heights.len()];
        }
        let [r, g, b, _] = COLLISION_RED.to_array();
        for i in struck {
            material.paint[i] = u32::from_be_bytes([1, r, g, b]);
        }
        material
    }

    /// What the last preview found the collet, spindle or shank hitting, for each toolpath
    /// not recalculated since.
    pub(in super::super) fn current_collisions(
        &self,
    ) -> impl Iterator<Item = (EntityId, &super::super::Collided)> {
        self.preview.collisions.iter().filter_map(|(&id, c)| {
            self.doc
                .toolpaths
                .get(&id)
                .is_some_and(|t| super::super::background::fingerprint(t) == c.moves)
                .then_some((id, c))
        })
    }

    /// Each toolpath's colour in a Toolpath Color preview (none: no fill), by the mark its
    /// cells take in the preview.
    pub(in super::super) fn preview_fills(
        &self,
    ) -> std::collections::HashMap<u32, Option<[u8; 3]>> {
        self.doc
            .toolpaths
            .iter()
            .enumerate()
            .map(|(place, (&id, t))| (self.preview_mark(id), t.preview_colour(place)))
            .collect()
    }

    /// The model's shape, `cells` across; while Orientate 3D Model shows its model whole,
    /// without the component the form made of it.
    pub(in super::super) fn model_shape(&mut self, cells: usize) -> Material {
        // nothing modelled yet: the material as it goes on the machine, uncut
        if self.doc.components.is_empty() {
            return Material::new(&self.doc.job, cells);
        }
        let drawn = self
            .form
            .as_ref()
            .filter(|f| f.model.is_some() && f.model_whole)
            .and_then(|f| f.drawn);
        // on a rotary job, full 3D models are drawn as they really are round the axis
        // ([`Self::rotary_models`]), never wrapped again from their unrolled heights
        let real: Vec<EntityId> = if self.doc.job.rotary.is_some() {
            self.doc
                .components
                .iter()
                .filter(|(_, k)| real_3d(k))
                .map(|(id, _)| *id)
                .collect()
        } else {
            vec![]
        };
        // hidden for the moment, not as an edit: nothing to undo
        let left_out: Vec<(EntityId, bool)> = drawn
            .into_iter()
            .chain(real)
            .filter_map(|id| {
                let k = self.doc.components.get_mut(&id)?;
                Some((id, std::mem::replace(&mut k.visible, false)))
            })
            .collect();
        let mut shape = cut_craft_core::component::model_shape(&self.doc, cells);
        for (id, was) in left_out.into_iter().rev() {
            if let Some(k) = self.doc.components.get_mut(&id) {
                k.visible = was;
            }
        }
        // the model in the material where Material Setup's Model Position puts it, as the
        // 3D toolpaths will cut it: where no component is, the model's zero plane
        if self.model_on_material() {
            cut_craft_core::component::seat_in_material(&self.doc.job, &mut shape);
        }
        shape
    }

    /// Whether the 3D view shows the model on the job's whole block of material, its uncut
    /// top round the model and its sides: on a flat job, not while a 3D model is imported
    /// (it is placed in the material, which would hide it) or a preview shows.
    pub(in super::super) fn model_on_material(&self) -> bool {
        self.preview.material.is_none()
            && self.player.playback.is_none()
            && self.doc.job.rotary.is_none()
            && !self.importing_model()
    }

    /// The SpaceMouse's moves since the last frame, turning, panning and zooming the 3D view
    /// while it shows (Options ▸ SpaceMouse); its Fit, Top, Right and Front buttons too.
    pub(in super::super) fn space_mouse_input(&mut self, ctx: &egui::Context) {
        use crate::spacemouse::{ButtonView, Input, button_view, moved};
        let Some(inputs) = self
            .space_mouse
            .as_ref()
            .map(|rx| rx.try_iter().collect::<Vec<_>>())
        else {
            return;
        };
        if self.view != View::ThreeD && self.split.is_none() {
            return;
        }
        let rotation = self.options.view_3d.rotation.view();
        for input in inputs {
            match input {
                Input::Motion {
                    translate,
                    rotate,
                    period,
                } => {
                    // spacenavd's own period, else this frame's; a pause before it is not
                    // a motion
                    let seconds = if period > 0 {
                        f64::from(period) / 1000.0
                    } else {
                        f64::from(ctx.input(|i| i.stable_dt))
                    }
                    .min(crate::spacemouse::LONGEST_STEP);
                    let view = (self.orbit, self.framing);
                    let settings = &self.options.space_mouse;
                    (self.orbit, self.framing) =
                        moved(view, (translate, rotate), seconds, settings, rotation);
                    self.moved_3d = ctx.input(|i| i.time);
                    self.spinning_3d = None;
                }
                Input::Button {
                    number,
                    pressed: true,
                } => match button_view(number) {
                    Some(view) => {
                        if let ButtonView::Look(orbit) = view {
                            self.orbit = orbit;
                        }
                        self.framing = self.framing.fitted();
                    }
                    None => self.status.set_message(&cut_craft_i18n::tr!(
                        "status-spacemouse-button-unused",
                        number = number
                    )),
                },
                Input::Button { .. } => {}
            }
        }
    }

    /// The 3D view's row of standard views, Fit and the perspective toggle.
    pub(super) fn view_3d_buttons(&mut self, ui: &mut egui::Ui) {
        for (label, orbit) in [
            ("Top", view3d::Orbit::TOP),
            ("Front", view3d::Orbit::FRONT),
            ("Right", view3d::Orbit::RIGHT),
            ("Iso", view3d::Orbit::default()),
        ] {
            if ui.small_button(label).clicked() {
                self.orbit = orbit;
                self.framing = self.framing.fitted();
                self.spinning_3d = None;
            }
        }
        if ui.small_button(tr!("panels-fit")).clicked() {
            self.framing = self.framing.fitted();
        }
        // The orthographic / perspective toggle
        let perspective = self.framing.perspective;
        if ui
            .selectable_label(perspective, tr!("panels-perspective"))
            .clicked()
        {
            self.framing.perspective = !perspective;
        }
        let vectors = self.display.vectors_3d;
        if ui
            .selectable_label(vectors, tr!("panels-vectors"))
            .on_hover_text(tr!("panels-show-or-hide-the-vectors"))
            .clicked()
        {
            self.show_vectors_3d(!vectors);
        }
        self.auto_rotate_buttons(ui);
    }

    /// Auto-rotate, last on the 3D view's row: turns the job round and round about its
    /// vertical, to see cuts that the lighting or perspective hides, with its speed beside it
    /// while it spins. Any drag in the view, the view cube or a 3D mouse stops it.
    pub(super) fn auto_rotate_buttons(&mut self, ui: &mut egui::Ui) {
        let spinning = self.spinning_3d.is_some();
        if ui
            .selectable_label(spinning, tr!("panels-auto-rotate"))
            .on_hover_text(tr!("panels-turn-the-job-round-and"))
            .clicked()
        {
            self.spinning_3d = (!spinning).then(|| ui.input(|i| i.time));
        }
        if self.spinning_3d.is_none() {
            return;
        }
        let mut speed = self.options.view_3d.spin_speed();
        let slider = egui::Slider::new(&mut speed, 2.0..=90.0)
            .text("Speed")
            .suffix(" °/s")
            .fixed_decimals(0);
        let changed = ui
            .add(slider)
            .on_hover_text(tr!("panels-how-quickly-the-job-turns"))
            .changed();
        if changed {
            self.options.view_3d.spin_speed = speed;
            self.save_options();
        }
    }

    /// Turns the 3D view on while Auto-rotate spins it, and asks for the next frame: a flat
    /// job about its vertical, a rotary job round its cylinder's axis.
    pub(super) fn spin_3d(&mut self, ctx: &egui::Context) {
        let Some(last) = self.spinning_3d else {
            return;
        };
        let now = ctx.input(|i| i.time);
        // a rotary job drawn wrapped turns round its cylinder's axis, a flat one on the spot
        let axis = match self.doc.job.rotary.filter(|_| self.display.auto_wrapping) {
            Some(r) if r.along == cut_craft_core::document::Along::X => {
                cut_craft_core::glam::DVec3::X
            }
            Some(_) => cut_craft_core::glam::DVec3::Y,
            None => cut_craft_core::glam::DVec3::Z,
        };
        let speed = self.options.view_3d.spin_speed();
        self.orbit = self.orbit.spun_about(axis, now - last, speed);
        self.spinning_3d = Some(now);
        ctx.request_repaint();
    }

    /// Shows or hides the vectors over the 3D view, kept with the options for next time.
    pub(in super::super) fn show_vectors_3d(&mut self, on: bool) {
        self.display.vectors_3d = on;
        self.options.view_3d.draw_vectors = on;
        self.save_options();
    }

    /// The preview or model as a block, with the 3D mouse controls: left- or right-drag
    /// rotates, middle-drag (or Ctrl+right, or left and right together) pans, the wheel zooms
    /// about the cursor, and a double-click or F fits it again.
    /// A drag in the 3D view while Orientate 3D Model is open: with Drag turns on the model,
    /// a turning drag adds to its fine turn (the view's height is half a turn); with Drag
    /// moves the axis on, a pan drag moves the model off the axis (the view's height is
    /// about two diameters). Whether the drag was taken.
    pub(in super::super) fn import_drag(
        &mut self,
        mode: Option<view3d::DragMode>,
        delta: egui::Vec2,
        size: egui::Vec2,
    ) -> bool {
        let (diameter, ring, grip) = (
            self.doc.job.rotary.map(|r| r.diameter),
            self.turn_ring,
            self.ring_grip,
        );
        let Some(form) = self.form.as_mut().filter(|f| f.id == "orientate_3d_model") else {
            return false;
        };
        let add = |value: &mut Value, by: f64| {
            *value = Value::from(value.as_f64().unwrap_or(0.0) + by);
        };
        let (dx, dy) = (f64::from(delta.x / size.x), f64::from(delta.y / size.y));
        match mode {
            Some(view3d::DragMode::Rotate) => {
                let turn = &mut form.params["turn"];
                let (about_x, about_z) = (dy * 180.0, dx * 180.0);
                // a drag from a rotation ring turns about its axis, whatever Drag turns says
                let mode = ring.or(turn["mode"].as_str()).map(str::to_owned);
                match mode.as_deref() {
                    // a ring held turns the model the way the drag goes round it, its own box
                    // alone (the ring is drawn round the axis that box turns about)
                    Some(axis @ ("x" | "y" | "z")) if ring.is_some() && grip.is_some() => {
                        let (along, radius) = grip.unwrap_or_default();
                        add(
                            &mut turn[axis],
                            crate::axes::ring_turn(delta, along, radius),
                        );
                    }
                    Some("x") => add(&mut turn["x"], about_x),
                    Some("y") => add(&mut turn["y"], about_z),
                    Some("z") => add(&mut turn["z"], about_z),
                    Some("xyz") => {
                        add(&mut turn["x"], about_x);
                        add(&mut turn["z"], about_z);
                    }
                    _ => return false,
                }
            }
            Some(view3d::DragMode::Pan) => {
                let Some(diameter) = diameter.filter(|_| form.params["axis"]["drag"] == true)
                else {
                    return false;
                };
                let per_view = 2.0 * diameter;
                add(&mut form.params["axis"]["across"], dx * per_view);
                add(&mut form.params["axis"]["up"], -dy * per_view);
            }
            None => return false,
        }
        form.dirty = true;
        true
    }

    pub(in super::super) fn view_3d(&mut self, ui: &mut egui::Ui) {
        self.framing.field_of_view = self.options.view_3d.field_of_view();
        cut_craft_bug_report::mark(ui, "3D view");
        ui.horizontal(|ui| self.view_3d_buttons(ui));
        let (area, response) =
            ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
        self.view_3d_area = Some(area);
        // the ViewCube over the view's corner, so the view does not see its clicks
        let font = egui::TextStyle::Small.resolve(ui.style());
        let cube = view3d::ViewCube::new(area, ui.painter(), &font, ui.spacing().item_spacing.x);
        let on_cube = ui.interact(
            cube.rect(),
            ui.scope_id().with("view_cube"),
            egui::Sense::click_and_drag(),
        );
        // the pan buttons (the wheel, to start with) pan, as in the 2D view
        let middle = self.pan_button_down(ui.ctx());
        // an f typed into a field is a letter, not Fit; F6 types nothing, so it always fits
        let typing = ui.ctx().egui_wants_keyboard_input();
        let (left, right, ctrl, scroll, fit_key) = ui.input(|i| {
            (
                i.pointer.primary_down(),
                i.pointer.secondary_down(),
                i.modifiers.ctrl,
                i.smooth_scroll_delta,
                (i.key_pressed(egui::Key::F) && !typing) || i.key_pressed(egui::Key::F6),
            )
        });
        let (before, rotation) = (
            (self.orbit, self.framing),
            self.options.view_3d.rotation.view(),
        );
        // while sculpting the left button sculpts; the right turns the view, the middle pans
        let sculpting = self.view_3d_drop_and_sculpt(ui, &response, area);
        // with a drawing tool open the left button draws on the model
        let drawing = self.drawing_in_3d();
        let left = left && !sculpting && drawing.is_none();
        let delta = response.drag_delta();
        self.take_ring(ui, &response);
        if response.dragged() && delta != egui::Vec2::ZERO {
            let mode = view3d::drag_mode([left, right, middle], ctrl);
            // Orientate 3D Model's Interactive Rotation and Rotation Axis Movement take the
            // drag for the model when they are on
            if !self.import_drag(mode, delta, area.size()) {
                match mode {
                    Some(view3d::DragMode::Rotate) => {
                        let turn = self.options.controls.turn(delta);
                        self.orbit = self.orbit.dragged_by(turn, area.size(), rotation);
                    }
                    Some(view3d::DragMode::Pan) => self.framing = self.framing.panned(delta),
                    None => {}
                }
            }
        }
        // the ViewCube: a drag turns the view, a click looks from the face, edge or corner
        // clicked, and a double-click from the other side
        let orbit = self.orbit;
        let pointed = |at: Option<egui::Pos2>| at.and_then(|at| cube.direction_at(orbit, at));
        let hovered = pointed(on_cube.hover_pos());
        // any drag in the view or on the cube, or a click on the cube, stops Auto-rotate
        if response.dragged() || on_cube.dragged() || on_cube.clicked() {
            self.spinning_3d = None;
        }
        self.spin_3d(ui.ctx());
        if on_cube.dragged() {
            let turn = self.options.controls.turn(on_cube.drag_delta());
            self.orbit = (self.orbit).dragged_by(turn, area.size(), rotation);
        } else if on_cube.double_clicked() {
            self.orbit = view3d::Orbit::facing(-self.orbit.towards_viewer());
        } else if let Some(from) =
            pointed(on_cube.interact_pointer_pos()).filter(|_| on_cube.clicked())
        {
            // ponytail: turns at once; the turns smoothly
            self.orbit = view3d::Orbit::facing(from);
        }
        // as in the 2D view: the wheel or a pinch zooms about the pointer, tilting the wheel
        // (or two fingers, in Trackpad mode) pans
        if let Some(at) = response.hover_pos() {
            let pinch = self.pinch(ui.ctx());
            let moved = self
                .options
                .controls
                .view_move(scroll, pinch, area.height());
            if moved.zoom != 1.0 {
                self.framing = self.framing.zoomed_about(at - area.center(), moved.zoom);
            }
            if moved.pan != egui::Vec2::ZERO {
                self.framing = self.framing.panned(moved.pan);
            }
        }
        // a click picks the component under it (its handles shown); a double-click off it fits
        let fit = area.shrink(ui.spacing().item_spacing.x * 2.0);
        self.fit_3d = Some(fit);
        // F fits too, with the pointer over the view
        let fit_now = response.hovered() && fit_key;
        self.double_click_3d(ui, &response, fit, drawing.is_none(), fit_now);
        if response.dragged() || on_cube.dragged() || before != (self.orbit, self.framing) {
            self.moved_3d = ui.input(|i| i.time);
        }
        self.pace_3d(ui.ctx());
        self.paint_3d(ui, area);
        let painter = ui.painter_at(area);
        cube.paint(&painter, self.orbit, hovered, ui.visuals(), &font);
        self.view_cube = Some(cube);
        self.handles_3d(ui, area, fit);
        self.rings_3d(ui, area, fit);
        self.anchors_3d(ui, &response, area, fit);
        if let Some(id) = drawing {
            self.draw_3d(ui, &response, area, fit, id);
        }
        self.view_3d_menu(ui, &response, fit);
    }

    /// A click in the 3D view (not while drawing): on a component it picks it and shows its
    /// handles and height buttons (Transform mode, unless another view tool is in use), Shift
    /// adding or taking it away; off every component it lets go of the picked ones. A
    /// double-click on a preview takes away the piece cut free under it, on a component picks
    /// it as a click does, and off the model fits the view, as `fit_now` (F) does.
    pub(super) fn double_click_3d(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        fit: egui::Rect,
        picking: bool,
        fit_now: bool,
    ) {
        let at = response.interact_pointer_pos().filter(|_| picking);
        let double = at.filter(|_| response.double_clicked());
        // a single click picks while the model shows, not over a preview, where a click
        // is the first half of a double-click taking a piece away
        let clicked = at.filter(|_| {
            response.clicked() && self.preview.material.is_none() && self.player.playback.is_none()
        });
        // while a preview shows, a double-click takes away the piece cut free under it
        let removed = double
            .filter(|_| self.player.playback.is_none() && self.preview.material.is_some())
            .is_some_and(|at| self.remove_preview_piece(fit, at));
        let picked = clicked
            .or(double)
            .filter(|_| !removed)
            .and_then(|at| self.component_at_3d(fit, at));
        let shift = ui.input(|i| i.modifiers.shift);
        if removed {
        } else if let Some(id) = picked {
            self.pick_3d(id, shift);
        } else if clicked.is_some() && !shift {
            self.doc.selected_components.clear();
        }
        if !removed && picked.is_none() && (double.is_some() || fit_now) {
            self.framing = self.framing.fitted();
        }
    }

    /// A click on component `id`: Shift adds or takes it away, else it alone is picked;
    /// with any picked, Transform mode shows their handles and height buttons (when no
    /// other view tool is in use). Clicking again keeps it so.
    pub(super) fn pick_3d(&mut self, id: EntityId, shift: bool) {
        if !shift {
            self.doc.selected_components = [id].into();
        } else if !self.doc.selected_components.remove(&id) {
            self.doc.selected_components.insert(id);
        }
        if !self.doc.selected_components.is_empty() && self.view_tool.is_none() {
            self.toggle_transform();
        }
    }

    /// A right-click picks the topmost component under it (keeping a selection it is part
    /// of) and opens the view menu listing all of them, as the 2D view's does.
    pub(super) fn view_3d_menu(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        fit: egui::Rect,
    ) {
        if response.secondary_clicked()
            && let Some(at) = response.interact_pointer_pos()
        {
            self.under_3d = self.components_at_3d(fit, at);
            if let Some(&id) = self.under_3d.first()
                && !self.doc.selected_components.contains(&id)
            {
                self.doc.selected_components = [id].into();
            }
        }
        let menu = || self.menus.view_3d_menu(&self.doc, &self.under_3d);
        if let Some(action) = self.popup(response, menu) {
            self.invoke(ui.ctx(), &action);
        }
    }

    /// The 3D view's surface: quick (`surface`) while it moves, then once still the
    /// model's own detail, made once.
    pub(in super::super) fn surface_mesh(
        &mut self,
        ui: &egui::Ui,
        surface: &Material,
        (orbit, framing): (view3d::Orbit, view3d::Framing),
        look: view3d::Look,
        fit: egui::Rect,
    ) -> egui::Mesh {
        let since = ui.input(|i| i.time) - self.moved_3d;
        let simplify = self.options.view_3d.simplify_while_moving;
        let settle = self.options.view_3d.settle();
        if self.player.playback.is_none() && (!simplify || since >= settle) {
            return self.fine_mesh(orbit, framing, look, fit);
        }
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_secs_f64(settle));
        view3d::mesh(surface, orbit, framing, &look, fit)
    }

    /// The 3D view's mesh at the model's (or preview's) own detail, kept while nothing it
    /// shows changes.
    pub(in super::super) fn fine_mesh(
        &mut self,
        orbit: view3d::Orbit,
        framing: view3d::Framing,
        look: view3d::Look,
        fit: egui::Rect,
    ) -> egui::Mesh {
        let key = FineKey {
            revision: self.history.revision(),
            cells: self.options.performance.still_cells(),
            job: self.doc.job,
            orbit,
            framing,
            fit,
            look,
            importing: (self.importing_model(), self.importing_whole()),
        };
        if let Some(mesh) = self.meshes.fine.get(&key) {
            return mesh.clone();
        }
        let surface = self.fine_surface();
        let mesh = view3d::mesh(&surface, orbit, framing, &look, fit);
        self.meshes.fine.keep(key, mesh).clone()
    }

    /// The preview's material, or the model's shape, at its own detail up to Options' detail
    /// when still.
    pub(in super::super) fn fine_surface(&mut self) -> Material {
        let most = self.options.performance.still_cells();
        if let Some(material) = &self.preview.material {
            return self.painted(view3d::on_sheet(material, self.doc.job.area(), most));
        }
        let job = &self.doc.job;
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a small cell count"
        )]
        let own = (job.width.max(job.height) / job.model_cell()).ceil() as usize;
        self.model_shape(own.clamp(MESH_CELLS.min(most), most))
    }

    /// The 3D view's surface as the GPU draws it: at its own detail, built once and kept
    /// while nothing it shows changes; while the view moves, at the detail the frame rate
    /// allows (each kept, a coarser one made from the fine one away from the window's
    /// thread); a playing preview's at the moving detail, every frame.
    pub(super) fn surface_geometry(
        &mut self,
        ctx: &egui::Context,
        wrap: Option<view3d::Wrap>,
        sides: bool,
    ) -> std::sync::Arc<view3d::gpu::Geometry> {
        // while a handle drags the model about, the quick surface, kept by the change it
        // shows, so the fine one is made once the drag ends
        let dragging = self.drag_base.is_some();
        let still = self.options.performance.still_cells();
        let cells = if dragging {
            self.moving_cells()
        } else {
            self.turning_cells()
        };
        if let Some(playback) = &self.player.playback {
            self.meshes.solid_gpu = None;
            let key = (
                self.shown_revision(playback),
                self.options.performance.still_cells(),
                self.history.revision(),
                wrap,
                sides,
            );
            if let Some((kept, geometry)) = &self.player.gpu
                && *kept == key
            {
                return std::sync::Arc::clone(geometry);
            }
            let surface = self.playback_surface().expect("a playback");
            let geometry = std::sync::Arc::new(view3d::gpu::surface(&surface, wrap, sides));
            self.player.gpu = Some((key, std::sync::Arc::clone(&geometry)));
            return geometry;
        }
        // a finished preview on the card: the sheet round it, the preview drawn over it
        if let Some(sheet) = self.preview_pair(wrap, sides) {
            self.meshes.solid_gpu = None;
            return sheet;
        }
        let key = self.geometry_key(wrap, sides);
        let solid = self.preview.material.is_none();
        let kept = match &mut self.meshes.surface_gpu {
            Some(kept) if kept.key == key => kept,
            other => other.insert(super::super::SurfaceGpu::new(key)),
        };
        kept.collect();
        if let Some(level) = kept.at(cells) {
            self.meshes.solid_gpu.clone_from(&level.solid);
            return std::sync::Arc::clone(&level.surface);
        }
        // turning the view, coarser than the kept fine surface: made from that on another
        // thread, the nearest kept detail drawn meanwhile
        if !dragging && cells < still && kept.fine.is_some() {
            kept.make(cells, solid, ctx);
            if let Some(level) = kept.nearest(cells) {
                self.meshes.solid_gpu.clone_from(&level.solid);
                return std::sync::Arc::clone(&level.surface);
            }
        }
        let mut surface = match (dragging, solid) {
            // the model alone: the quick surface the view keeps
            (true, true) => match self.surface_3d().map(Cow::into_owned) {
                Some(quick) => quick,
                None => self.model_shape(cells),
            },
            (true, false) => self.model_shape(cells),
            (false, _) => self.fine_surface(),
        };
        // the components' own colours (Solid Color), where the model is theirs: painted as a
        // preview paints the toolpaths' colours
        if solid {
            let doc = &self.doc;
            let paint: Vec<u32> = (0..surface.rows)
                .flat_map(|r| (0..surface.columns).map(move |c| (c, r)))
                .map(|(c, r)| {
                    cut_craft_core::component::colour_at(doc, surface.centre(c, r))
                        .map_or(0, |[r, g, b]| u32::from_be_bytes([1, r, g, b]))
                })
                .collect();
            if paint.iter().any(|&p| p != 0) {
                surface.paint = paint;
            }
        }
        // the fine surface alone stands for every detail coarser than it; a quick one made
        // while dragging is kept as the detail it was asked for
        let (made, fine) = if dragging {
            (cells, false)
        } else {
            (still, true)
        };
        // the model alone (no preview) is drawn solid down to the modeling plane
        let level = super::super::level_of(&surface, made, wrap, sides, solid);
        let kept = self.meshes.surface_gpu.as_mut().expect("kept above");
        if fine {
            kept.fine = Some(std::sync::Arc::new(surface));
        }
        kept.levels.retain(|l| l.cells != made);
        kept.levels.push(level.clone());
        self.meshes.solid_gpu = level.solid;
        level.surface
    }

    /// A finished preview drawn at its own detail by `camera` over `area` (`pixels` to a
    /// point), in its place on the sheet whose corner is `sheet`: the tiles in sight, at the
    /// detail the screen shows ([`view3d::tiles`]); `None` when [`Self::preview_pair`] has
    /// none, or none is in sight.
    pub(super) fn preview_draw(
        &mut self,
        camera: &view3d::Camera,
        area: egui::Rect,
        look: &view3d::Look,
        mark: egui::Color32,
        (sheet, pixels): (cut_craft_core::geometry::Point, f32),
    ) -> Option<view3d::gpu::Tiles> {
        self.preview_pair(look.wrap, look.sides)?;
        let (_, tiled) = self.meshes.surface_gpu.as_mut()?.preview.as_mut()?;
        let screen = |p: cut_craft_core::glam::DVec3| camera.screen(p - sheet.extend(0.0));
        let frame = tiled.frame(screen, area, pixels)?;
        let at = (frame.origin - sheet).extend(0.0);
        Some(view3d::gpu::Tiles {
            draw: view3d::gpu::Draw::new(frame.geometry, camera, area, at, look, mark),
            tiles: frame.tiles,
        })
    }

    /// What the kept surfaces on the card were made for.
    pub(super) fn geometry_key(
        &self,
        wrap: Option<view3d::Wrap>,
        sides: bool,
    ) -> super::super::GeometryKey {
        super::super::GeometryKey {
            revision: self.history.revision(),
            job: self.doc.job,
            wrap,
            sides,
            importing: (self.importing_model(), self.importing_whole()),
            cells: self.options.performance.still_cells(),
        }
    }

    /// A finished preview as the card draws it: the sheet at the 3D view's detail with a
    /// hole where the preview goes, and the preview at its own detail, which can be far
    /// finer than the sheet's (a small part on a big sheet). Made once and kept with the
    /// other surfaces; `None` while one plays, without a card, or for a wrapped or
    /// double-sided job (drawn as the whole sheet, as before).
    pub(super) fn preview_pair(
        &mut self,
        wrap: Option<view3d::Wrap>,
        sides: bool,
    ) -> Option<std::sync::Arc<view3d::gpu::Geometry>> {
        if self.player.playback.is_some()
            || wrap.is_some()
            || self.gpu_3d.is_none()
            || self.doc.job.double_sided.is_some()
        {
            return None;
        }
        let key = self.geometry_key(wrap, sides);
        if let Some(kept) = &self.meshes.surface_gpu
            && kept.key == key
            && let Some((sheet, _)) = &kept.preview
        {
            return Some(std::sync::Arc::clone(sheet));
        }
        let material = self.preview.material.as_ref()?;
        let (area, most) = (self.doc.job.area(), key.cells);
        let sheet = view3d::on_sheet(material, area, most);
        let hole = material.area();
        let preview = material.clone();
        let sheet = view3d::holed(self.painted(sheet), hole);
        let preview = self.painted(preview);
        let sheet = std::sync::Arc::new(view3d::gpu::surface(&sheet, None, sides));
        let budget = crate::preview_detail::kept_on_card(self.display.detail);
        let tiled = view3d::tiles::Tiled::new(&preview, budget);
        let kept = match &mut self.meshes.surface_gpu {
            Some(kept) if kept.key == key => kept,
            other => other.insert(super::super::SurfaceGpu::new(key)),
        };
        kept.preview = Some((std::sync::Arc::clone(&sheet), tiled));
        Some(sheet)
    }

    /// Mesh cells along the job's longer side while the model or the view moves: as the
    /// frame rate allows when Options says so, else Options' detail while moving.
    pub(in super::super) fn moving_cells(&self) -> usize {
        let view = &self.options.performance;
        if view.adjust_to_frame_rate {
            self.detail_3d.0.min(view.still_detail).cells()
        } else {
            view.moving_cells()
        }
    }

    /// Mesh cells the GPU draws the model at while the view turns: as the frame rate allows
    /// when Options says so, else the detail when still (the card turns it unchanged).
    pub(in super::super) fn turning_cells(&self) -> usize {
        let view = &self.options.performance;
        if view.adjust_to_frame_rate {
            self.detail_3d.0.min(view.still_detail).cells()
        } else {
            view.still_cells()
        }
    }

    /// Judges this frame's detail from how long the last one took, once a frame: the
    /// controller sees the view as moving while it turns, a handle drags or a preview
    /// plays, and for the settle time after. Asks for a frame once still, to draw finely.
    pub(super) fn pace_3d(&mut self, ctx: &egui::Context) {
        let pass = ctx.cumulative_pass_nr();
        if self.paced_pass == pass {
            return;
        }
        self.paced_pass = pass;
        let (now, seconds) = ctx.input(|i| (i.time, f64::from(i.unstable_dt)));
        let speed = &self.options.performance;
        let settle = self.options.view_3d.settle();
        let (still, fps) = (speed.still_detail, speed.fps_wanted());
        let since = now - self.moved_3d;
        let moving = since < settle
            || (self.player.playing && self.player.playback.is_some())
            || self.drag_base.is_some()
            || self.drag_3d.is_some();
        let level = self.pacer.frame(seconds, moving, still, fps);
        self.detail_3d = (level, moving);
        if moving && level < still {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(
                (settle - since).max(0.0) + 0.01,
            ));
        }
    }

    /// The status bar's note of the 3D view's detail and frames a second (Options ▸ Show
    /// detail and frame rate in the status bar).
    pub(in super::super) fn frame_rate_note(&self) -> String {
        let (level, moving) = self.detail_3d;
        match self.pacer.fps().filter(|_| moving) {
            Some(fps) => format!("3D detail: {} · {fps:.0} frames/s", level.name()),
            None => format!("3D detail: {}", level.name()),
        }
    }

    /// The imported model as the GPU draws it, kept while the model stays.
    pub(super) fn model_geometry(
        &mut self,
        model: &std::sync::Arc<Vec<cmd_orientate_3d_model::Seen>>,
    ) -> std::sync::Arc<view3d::gpu::Geometry> {
        faces_geometry(&mut self.meshes.model_gpu, model)
    }
}
