//! Painting the 3D view: picking rings and anchors, the background, the models, the toolpaths and
//! the overlays.

use super::{
    Anchor, Cow, CutCraftApp, EntityId, Material, Point, RING_MARK_GAP, RING_SHARE, TOOL_SHANK,
    egui, faces_geometry, modeling_plane, origin_3d, origin_arrow, ring_arrows, view2d, view3d,
    wrap_for,
};

impl CutCraftApp {
    /// The rotation handles while Orientate 3D Model shows the model in a flat job: red,
    /// green and blue rings round it turn it about X, Y and Z (the axis colours used
    /// everywhere), and the grey sphere in its middle about all three. Each ring has arrows
    /// both ways round where it stands out most, the + way marked (X+ and X−): dragging
    /// along the ring that way turns the model by a positive angle in that ring's box alone.
    /// Before any turn each ring is square to the job's own axis; turned, the Y ring tilts
    /// with the Z turn and the X ring with the Z and Y turns, so a ring always goes round
    /// the axis its box turns about (help: Orientate 3D Model, Turning the model).
    pub(super) fn rings_3d(&mut self, ui: &egui::Ui, area: egui::Rect, fit: egui::Rect) {
        use crate::axes::Axis;
        use cut_craft_core::glam::DVec3;
        self.turn_rings.clear();
        let model = self
            .form
            .as_ref()
            .filter(|f| f.id == "orientate_3d_model" && f.model_whole)
            .and_then(|f| f.model.clone());
        let Some(model) = model.filter(|_| self.doc.job.rotary.is_none()) else {
            return;
        };
        let corners = model.iter().flat_map(|(t, _)| t);
        let (Some(lo), Some(hi)) = (
            corners.clone().copied().reduce(DVec3::min),
            corners.copied().reduce(DVec3::max),
        ) else {
            return;
        };
        let Some(surface) = self.surface_3d().map(Cow::into_owned) else {
            return;
        };
        let camera = view3d::Camera::new(&surface, self.orbit, self.framing, fit, None);
        let origin = DVec3::new(surface.origin.x, surface.origin.y, 0.0);
        let screen = |p: DVec3| camera.screen(p - origin);
        let (middle, reach) = ((lo + hi) / 2.0, (hi - lo).max_element() * RING_SHARE);
        let painter = ui.painter_at(area);
        let width = 2.0 * ui.visuals().widgets.noninteractive.fg_stroke.width;
        let centre = screen(middle);
        let font = egui::TextStyle::Body.resolve(ui.style());
        let head = ui.spacing().interact_size.y / 2.0;
        // a ring's points every few degrees, in the plane square to its axis, in the order a
        // positive turn about the axis goes (right-hand rule: X turns Y up towards Z, Y turns
        // Z over towards X, Z turns X round towards Y)
        let steps = 72;
        let mut looks = vec![];
        let turns = self.form.as_ref().map_or([0.0; 3], |f| {
            ["x", "y", "z"].map(|k| f.params["turn"][k].as_f64().unwrap_or(0.0))
        });
        for axis in Axis::ALL {
            // tilted with the turns made after its own, so it goes round the axis its box
            // turns the model about
            let frame = crate::axes::ring_frame(turns, axis);
            let points: Vec<egui::Pos2> = (0..=steps)
                .map(|i| {
                    let a = std::f64::consts::TAU * f64::from(i) / f64::from(steps);
                    let (c, s) = (a.cos() * reach, a.sin() * reach);
                    middle
                        + frame
                            * match axis {
                                Axis::X => DVec3::new(0.0, c, s),
                                Axis::Y => DVec3::new(s, 0.0, c),
                                Axis::Z => DVec3::new(c, s, 0.0),
                            }
                })
                .map(screen)
                .collect();
            let name = axis.letter();
            let key = match axis {
                Axis::X => "x",
                Axis::Y => "y",
                Axis::Z => "z",
            };
            let hot = self.turn_rings.held() == Some(key);
            let colour = axis.colour();
            let stroke = egui::Stroke::new(if hot { 2.0 * width } else { width }, colour);
            painter.add(egui::Shape::line(points.clone(), stroke));
            self.turn_rings.add_ring(key, points);
            looks.push((stroke, name));
        }
        // each ring's + and − arrows, kept clear of each other's
        let rings = self.turn_rings.ring_points();
        let marks = crate::axes::rings_marks(&rings, centre, RING_MARK_GAP * head);
        for ((points, mark), (stroke, name)) in rings.iter().zip(marks).zip(looks) {
            if let Some((at, _)) = mark {
                ring_arrows(&painter, points, at, centre, (stroke, head), &font, name);
            }
        }
        let radius = ui.spacing().interact_size.y / 2.0;
        painter.circle(
            centre,
            radius,
            ui.visuals().widgets.inactive.bg_fill,
            ui.visuals().widgets.inactive.bg_stroke,
        );
        self.turn_rings.set_sphere(centre, radius);
    }

    /// A left drag starting on a rotation handle takes it for the drag; let go, none.
    pub(super) fn take_ring(&mut self, ui: &egui::Ui, response: &egui::Response) {
        if response.drag_started_by(egui::PointerButton::Primary) {
            let pressed = ui.input(|i| i.pointer.press_origin());
            self.turn_rings
                .press(pressed, ui.spacing().interact_size.y / 2.0);
        } else if !response.dragged() {
            self.turn_rings.let_go();
        }
        // while a ring is held, the way + runs round it under the pointer and how far it
        // reaches from its middle: the drag turns the model the way it goes round the ring
        self.turn_rings.follow(ui.input(|i| i.pointer.latest_pos()));
    }

    /// Set in the settings popup: two clicks on the model place the fade's or tilt's
    /// anchors; the status bar says which point to
    /// click, and the first is marked while the second is awaited.
    pub(super) fn anchors_3d(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        area: egui::Rect,
        fit: egui::Rect,
    ) {
        let Some((which, first)) = self.settings_3d.as_ref().and_then(|s| s.anchors) else {
            return;
        };
        self.status.set_message(&which.prompt(first.is_some()));
        if response.clicked()
            && let Some(at) = response.interact_pointer_pos()
            && let Some(p) = self.point_at_3d(fit, at)
        {
            self.place_anchor(which, first, p);
        }
        if let Some(first) = first
            && let Some(surface) = self.surface_3d().map(Cow::into_owned)
        {
            let screen = view3d::screen_of(&surface, self.orbit, self.framing, fit, None);
            let z = surface.at(first).unwrap_or(0.0);
            let at = screen(cut_craft_core::glam::DVec3::new(
                first.x - surface.origin.x,
                first.y - surface.origin.y,
                z,
            ));
            let r = ui.spacing().interact_size.y * view2d::NODE_HANDLE_SHARE / 2.0;
            ui.painter_at(area)
                .circle_filled(at, r, ui.visuals().selection.stroke.color);
        }
    }

    /// The job point on the drawn surface at the point of the 3D view's picture `fit` under
    /// `at`; `None` off the surface.
    pub(in super::super) fn point_at_3d(
        &mut self,
        fit: egui::Rect,
        at: egui::Pos2,
    ) -> Option<Point> {
        let (job, previewing) = (
            self.shown_job(),
            self.preview.material.is_some() || self.showing_uncut_material(),
        );
        let (orbit, framing, on) = (self.orbit, self.framing, self.display.auto_wrapping);
        let surface = self.surface_3d()?.into_owned();
        let wrap = wrap_for(&job, on, previewing, &surface);
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

    /// A click placing a fade's or tilt's anchor: the first is kept until the second, which
    /// turns the fade or tilt on between them through the settings popup.
    pub(super) fn place_anchor(&mut self, which: Anchor, first: Option<Point>, p: Point) {
        if let Some(s) = &mut self.settings_3d {
            s.place_anchor(which, first, p);
        }
    }

    /// Every shown component at the point of the 3D view's picture `fit` under `at`,
    /// topmost first.
    pub(in super::super) fn components_at_3d(
        &mut self,
        fit: egui::Rect,
        at: egui::Pos2,
    ) -> Vec<EntityId> {
        let Some(p) = self.point_at_3d(fit, at) else {
            return vec![];
        };
        self.doc
            .components
            .iter()
            .rev()
            .filter(|(_, k)| k.visible && k.height_at(p).is_some())
            .map(|(&id, _)| id)
            .collect()
    }

    /// Behind the 3D view: Options' solid colour, gradient or picture (the gradient when the
    /// picture cannot be read).
    pub(super) fn paint_background(
        &self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        area: egui::Rect,
    ) {
        use crate::options::BackgroundStyle;
        let v = &self.options.view_3d;
        let colour = |[r, g, b]: [u8; 3]| egui::Color32::from_rgb(r, g, b);
        let (top, bottom) = (
            colour(v.background_color),
            colour(v.gradient_background_color),
        );
        match v.background_style {
            BackgroundStyle::Solid => {
                painter.rect_filled(area, 0.0, top);
            }
            BackgroundStyle::Image if !v.image_file_path.is_empty() => {
                painter.add(view3d::background(area, top, bottom));
                let uri = format!("file://{}", v.image_file_path);
                egui::Image::new(uri)
                    .fit_to_exact_size(area.size())
                    .paint_at(ui, area);
            }
            _ => {
                painter.add(view3d::background(area, top, bottom));
            }
        }
    }

    /// The 3D view's picture: background, block and the overlays the View menu turns on.
    /// A full 3D model being imported into a rotary job, turned round the axis inside a
    /// wireframe of the stock, over the wrapped view.
    #[expect(clippy::too_many_arguments, reason = "one frame's view, taken apart")]
    pub(super) fn paint_turned(
        &mut self,
        ui: &egui::Ui,
        area: egui::Rect,
        surface: &cut_craft_core::simulation::Material,
        wrap: view3d::Wrap,
        camera: &view3d::Camera,
        look: &view3d::Look,
        gpu: Option<&mut Option<view3d::gpu::Draw>>,
    ) {
        use cut_craft_core::glam::DVec3;
        let Some(model) = self.turned_models() else {
            return;
        };
        let painter = ui.painter_at(area);
        let screen = |p| camera.screen(p);
        // from the job's millimetres (the axis at y = z = 0 for an X axis) to the wrapped
        // view's: along from the material's start; across and up as they are, the model at
        // its true size inside the stock
        let start = if wrap.along_x {
            DVec3::new(-surface.origin.x, 0.0, 0.0)
        } else {
            DVec3::new(0.0, -surface.origin.y, 0.0)
        };
        let mark = ui.visuals().error_fg_color;
        if let Some(draw) = gpu {
            let geometry = self.model_geometry(&model);
            *draw = Some(view3d::gpu::Draw::new(
                geometry, camera, area, start, look, mark,
            ));
        } else {
            let shown: Vec<cmd_orientate_3d_model::Seen> = model
                .iter()
                .map(|(t, hidden)| (t.map(|p| p + start), *hidden))
                .collect();
            painter.add(view3d::triangles(&shown, screen, self.orbit, look, mark));
        }
        if !self.display.material_block {
            return;
        }
        let job = self.shown_job().area().size();
        let length = if wrap.along_x { job.x } else { job.y };
        let stroke = egui::Stroke::new(ui.visuals().widgets.noninteractive.fg_stroke.width, mark);
        // the material as Job Setup describes it: round, square or a board
        let outline = self.doc.job.rotary.as_ref().and_then(view3d::outline_of);
        for [a, b] in view3d::stock_edges(wrap.along_x, wrap.radius, outline, 0.0, length) {
            painter.line_segment([screen(a), screen(b)], stroke);
        }
    }

    /// What [`Self::paint_turned`] draws: the model being imported (its form's own faces)
    /// and every other full 3D model as it really is; kept while nothing changes, so the
    /// GPU's copy is made once. `None` with nothing to draw.
    pub(super) fn turned_models(
        &mut self,
    ) -> Option<std::sync::Arc<Vec<cmd_orientate_3d_model::Seen>>> {
        let importing = self
            .form
            .as_ref()
            .filter(|f| f.model_whole)
            .and_then(|f| Some((f.model.clone()?, f.drawn)));
        let key = (
            self.history.revision(),
            importing
                .as_ref()
                .map(|(m, _)| std::sync::Arc::as_ptr(m) as usize),
        );
        if let Some((kept, model)) = &self.meshes.turned
            && *kept == key
        {
            return Some(std::sync::Arc::clone(model));
        }
        let (own, skip) = importing.map_or((None, None), |(m, id)| (Some(m), id));
        let mut all = self.rotary_models(skip);
        if let Some(own) = &own {
            if all.is_empty() {
                // only the import: its own faces, kept as they are
                self.meshes.turned = Some((key, std::sync::Arc::clone(own)));
                return Some(std::sync::Arc::clone(own));
            }
            all.extend(own.iter().copied());
        }
        if all.is_empty() {
            self.meshes.turned = None;
            return None;
        }
        let all = std::sync::Arc::new(all);
        self.meshes.turned = Some((key, std::sync::Arc::clone(&all)));
        Some(all)
    }

    /// The imported model's triangles drawn for this view, kept from frame to frame while
    /// the model, the job and the view stay the same.
    pub(super) fn model_mesh(
        &mut self,
        model: &std::sync::Arc<Vec<cmd_orientate_3d_model::Seen>>,
        view: (view3d::Orbit, view3d::Framing, egui::Rect, view3d::Look),
        mark: egui::Color32,
        screen: impl Fn(cut_craft_core::glam::DVec3) -> egui::Pos2,
    ) -> std::sync::Arc<egui::Mesh> {
        let (orbit, framing, fit, look) = view;
        let key = super::super::ModelKey {
            model: std::sync::Arc::as_ptr(model) as usize,
            job: self.shown_job(),
            orbit,
            framing,
            fit,
            look,
            mark,
        };
        if let Some(mesh) = self.meshes.model.get(&key) {
            return std::sync::Arc::clone(mesh);
        }
        let mesh = std::sync::Arc::new(view3d::triangles(model, screen, orbit, &look, mark));
        std::sync::Arc::clone(self.meshes.model.keep(key, mesh))
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one frame of the 3D view, in drawing order"
    )]
    pub(in super::super) fn paint_3d(&mut self, ui: &egui::Ui, area: egui::Rect) {
        let fit = area.shrink(ui.spacing().item_spacing.x * 2.0);
        let shown = self.display;
        let [r, g, b] = self.shown_job().colour;
        let look = view3d::Look {
            colour: if shown.colour_shaded {
                egui::Color32::from_rgb(r, g, b)
            } else {
                ui.visuals().widgets.inactive.fg_stroke.color
            },
            // the block's sides: a preview's, the uncut material's, the block the model
            // sits on
            sides: shown.material_block
                && (self.preview.material.is_some()
                    || self.player.playback.is_some()
                    || self.showing_uncut_material()
                    || self.model_on_material()),
            light: view3d::Look::light_from(self.orbit, shown.light_follows),
            wrap: None,
            // a wood from Toolpaths ▸ Preview Colors shows its grain
            grain: cmd_materials::Material::of([r, g, b])
                .grain()
                .filter(|_| shown.colour_shaded)
                .map(|(spacing, depth)| view3d::Grain { spacing, depth }),
            shadows: shown.shadows,
        };
        let (orbit, framing) = (self.orbit, self.framing);
        let painter = ui.painter_at(area);
        if shown.shaded_background {
            self.paint_background(ui, &painter, area);
        }
        let job = self.shown_job();
        // the uncut material wraps as a preview's stock does: its top at the surface
        let previewing = self.preview.material.is_some()
            || self.player.playback.is_some()
            || self.showing_uncut_material();
        // a playing preview's tool: where its tip is, and the parts it is drawn in
        let tool_at = self
            .player
            .playback
            .as_ref()
            .and_then(cut_craft_core::playback::Playback::tool)
            .map(|(tip, tool)| (tip, view3d::tool_parts(tool)));
        let Some(surface) = self.surface_3d().map(Cow::into_owned) else {
            return;
        };
        // a rotary job's full 3D models (and one being imported) are shown as they really
        // are round the axis, so the job is drawn wrapped while there are any
        let turned = self.shows_turned();
        // a rotary job's preview as the machine runs it is always shown round its axis
        let spinning = self.turning.is_some() && job.rotary.is_some();
        let wrap = wrap_for(
            &job,
            shown.auto_wrapping || turned || spinning,
            previewing,
            &surface,
        );
        let look = view3d::Look { wrap, ..look };
        // the modeling plane a model being imported sits on, under the material so it shows
        // round it and through what the material leaves open
        let importing = self.form.as_ref().is_some_and(|f| f.model.is_some());
        if importing && wrap.is_none() {
            let screen = view3d::screen_of(&surface, orbit, framing, fit, None);
            let area = job.area();
            let at = |p: cut_craft_core::geometry::Point| {
                cut_craft_core::glam::DVec3::new(
                    p.x - surface.origin.x,
                    p.y - surface.origin.y,
                    0.0,
                )
            };
            painter.add(modeling_plane(ui, &screen, at(area.min), at(area.max)));
        }
        // on the GPU when the window has one (the headless tests draw egui meshes): its
        // callback takes this place in the painter's order once the model is known too
        let camera = view3d::Camera::new(&surface, orbit, framing, fit, wrap);
        let mark = ui.visuals().error_fg_color;
        let slot = self
            .gpu_3d
            .is_some()
            .then(|| painter.add(egui::Shape::Noop));
        let (mut surface_draw, mut model_draw, mut ghost_draw) = (None, None, None);
        if spinning {
            surface_draw = self.paint_turning(ui, area, &camera, &look, slot.is_some());
        } else if slot.is_some() {
            let geometry = self.surface_geometry(ui.ctx(), look.wrap, look.sides);
            let at = cut_craft_core::glam::DVec3::ZERO;
            surface_draw = Some(view3d::gpu::Draw::new(
                geometry, &camera, area, at, &look, mark,
            ));
        } else {
            painter.add(self.surface_mesh(ui, &surface, (orbit, framing), look, fit));
        }
        let gpu = slot.is_some().then_some(&mut model_draw);
        if let Some(wrap) = wrap {
            // ponytail: the overlays are drawn flat, so they are left off the wrapped view
            if turned {
                self.paint_turned(ui, area, &surface, wrap, &camera, &look, gpu);
            }
        } else {
            self.paint_flat(ui, area, &surface, &camera, look, tool_at, gpu);
            ghost_draw = self.paint_ghost(ui, area, &surface, &camera, look, slot.is_some());
        }
        // the model's body, under its surface: also while a model is imported, whose own
        // faces (all of them, or past the face limit those out of reach) are drawn as well
        let body_draw = (slot.is_some() && self.preview.material.is_none())
            .then(|| self.meshes.solid_gpu.clone())
            .flatten()
            .map(|geometry| {
                let at = cut_craft_core::glam::DVec3::ZERO;
                view3d::gpu::Draw::new(geometry, &camera, area, at, &look, mark)
            });
        // a finished preview at its own detail, in its place over the sheet (on the card)
        let pixels = ui.ctx().pixels_per_point();
        let tiled = body_draw
            .is_none()
            .then(|| self.preview_draw(&camera, area, &look, mark, (surface.origin, pixels)))
            .flatten();
        if let (Some(slot), Some(gpu)) = (slot, &mut self.gpu_3d) {
            let shape = gpu.paint(area, surface_draw, model_draw, body_draw, ghost_draw, tiled);
            painter.set(slot, shape);
        }
    }

    /// A model being imported: its parts past the material's edges, in see-through red
    /// (Options ▸ Out-of-bounds opacity) over the rest, so a model placed partly off the
    /// material is seen whole. Given to the GPU when the view has one, else painted.
    pub(super) fn paint_ghost(
        &mut self,
        ui: &egui::Ui,
        area: egui::Rect,
        surface: &Material,
        camera: &view3d::Camera,
        look: view3d::Look,
        on_gpu: bool,
    ) -> Option<view3d::gpu::Draw> {
        let Some(ghost) = self
            .form
            .as_ref()
            .and_then(|f| f.beyond.clone())
            .filter(|g| !g.is_empty())
        else {
            self.meshes.ghost_gpu = None;
            return None;
        };
        let red = self.options.import.out_of_bounds_colour();
        let look = view3d::Look {
            colour: red,
            sides: false,
            wrap: None,
            grain: None,
            shadows: false,
            ..look
        };
        let origin = cut_craft_core::glam::DVec3::new(-surface.origin.x, -surface.origin.y, 0.0);
        if on_gpu {
            let geometry = faces_geometry(&mut self.meshes.ghost_gpu, &ghost);
            return Some(view3d::gpu::Draw::new(
                geometry, camera, area, origin, &look, red,
            ));
        }
        let screen = |p| camera.screen(p + origin);
        let mesh = view3d::triangles(&ghost, screen, self.orbit, &look, red);
        ui.painter_at(area).add(mesh);
        None
    }

    /// Over the unwrapped surface: the model being imported in the material's outline, a
    /// playing preview's tool, and the overlays.
    #[expect(clippy::too_many_arguments, reason = "one frame's view, taken apart")]
    pub(super) fn paint_flat(
        &mut self,
        ui: &egui::Ui,
        area: egui::Rect,
        surface: &Material,
        camera: &view3d::Camera,
        look: view3d::Look,
        tool_at: Option<(cut_craft_core::glam::DVec3, Vec<Vec<[f64; 2]>>)>,
        gpu: Option<&mut Option<view3d::gpu::Draw>>,
    ) {
        let (job, painter) = (self.shown_job(), ui.painter_at(area));
        let (orbit, framing) = (self.orbit, self.framing);
        let fit = area.shrink(ui.spacing().item_spacing.x * 2.0);
        let screen = |p| camera.screen(p);
        let local = |p: cut_craft_core::geometry::Point, z: f64| {
            cut_craft_core::glam::DVec3::new(p.x - surface.origin.x, p.y - surface.origin.y, z)
        };
        // an imported model whole, what a cutter cannot reach marked, inside the material's
        // outline to judge its size by, over the modeling plane it is placed on
        if let Some(model) = self.form.as_ref().and_then(|f| f.model.clone()) {
            let origin = local(cut_craft_core::geometry::Point::new(0.0, 0.0), 0.0);
            let mark = ui.visuals().error_fg_color;
            if let Some(draw) = gpu {
                let geometry = self.model_geometry(&model);
                *draw = Some(view3d::gpu::Draw::new(
                    geometry, camera, area, origin, &look, mark,
                ));
            } else {
                let view = (orbit, framing, fit, look);
                painter.add(self.model_mesh(&model, view, mark, |p| screen(p + origin)));
            }
            // the material's outline, unless View ▸ Draw Material Block is off
            if self.display.material_block {
                let job_area = job.area();
                let (lo, hi) = (local(job_area.min, 0.0), local(job_area.max, job.thickness));
                let stroke = ui.visuals().widgets.noninteractive.fg_stroke;
                for [a, b] in view3d::box_edges(lo, hi) {
                    painter.line_segment([screen(a), screen(b)], stroke);
                }
            }
        }
        self.overlays_3d(ui, &painter, surface, (orbit, framing), fit);
        self.picked_outline_3d(ui, &painter, surface, (orbit, framing), fit);
        // a playing preview's tool standing upright on its tip, its own shape and size: the
        // cutting end highlighted, the shank above it steel; over the vectors, which would
        // otherwise print onto it
        // ponytail: drawn whole, over the surface it is cutting into
        if let Some((tip, parts)) = tool_at {
            let at = local(tip.truncate(), tip.z);
            let selection = ui.visuals().selection;
            for (i, rings) in parts.iter().enumerate() {
                let fill = if i == 0 {
                    selection.bg_fill
                } else {
                    TOOL_SHANK
                };
                painter.add(egui::Shape::convex_polygon(
                    view3d::turned_outline(screen, at, rings),
                    fill,
                    selection.stroke,
                ));
            }
        }
    }

    /// The picked components' outlines over the 3D view's (unwrapped) `surface`, thick and
    /// in the selection colour, lying on the model, so a picked model looks picked in any
    /// mode. Traced once per pick and change, then kept.
    pub(super) fn picked_outline_3d(
        &mut self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        surface: &Material,
        (orbit, framing): (view3d::Orbit, view3d::Framing),
        fit: egui::Rect,
    ) {
        let picked: Vec<EntityId> = (self.doc.selected_components.iter())
            .copied()
            .filter(|id| self.doc.components.get(id).is_some_and(|k| k.visible))
            .collect();
        if picked.is_empty() {
            self.picked_outline = None;
            return;
        }
        let key = (picked, self.history.revision());
        if self
            .picked_outline
            .as_ref()
            .is_none_or(|(kept, _)| *kept != key)
        {
            let parts: Vec<_> = key.0.iter().map(|id| &self.doc.components[id]).collect();
            let rings = cut_craft_core::component::silhouette(&parts)
                .iter()
                .map(|path| {
                    let mut points = path.flatten(cut_craft_core::polygon::TOLERANCE);
                    points.extend(points.first().copied());
                    points
                })
                .collect();
            self.picked_outline = Some((key, rings));
        }
        let Some((_, rings)) = &self.picked_outline else {
            return;
        };
        let stroke = ui.visuals().selection.stroke;
        let stroke = egui::Stroke::new(stroke.width.max(1.0) * 3.0, stroke.color);
        let screen = view3d::screen_of(surface, orbit, framing, fit, None);
        let seen = view3d::seen_of(surface, orbit, framing);
        // round the model's foot: the lowest surface within a cell of the outline (what is
        // just outside it), a cell above that so the surface does not hide it
        let (cell, lift) = (surface.cell, surface.cell);
        let foot = |p: cut_craft_core::geometry::Point| {
            [
                (0.0, 0.0),
                (cell, 0.0),
                (-cell, 0.0),
                (0.0, cell),
                (0.0, -cell),
            ]
            .into_iter()
            .filter_map(|(x, y)| surface.at(p + cut_craft_core::geometry::Point::new(x, y)))
            .filter(|z| z.is_finite())
            .reduce(f64::min)
            .unwrap_or(0.0)
        };
        for ring in rings {
            let line: Vec<_> = ring
                .iter()
                .map(|&p| {
                    let z = foot(p);
                    cut_craft_core::glam::DVec3::new(
                        p.x - surface.origin.x,
                        p.y - surface.origin.y,
                        z + lift,
                    )
                })
                .collect();
            // all the way round, thin where the model hides it, so the whole outline shows
            let behind = egui::Stroke::new(stroke.width / 2.0, stroke.color.gamma_multiply(0.6));
            painter.add(egui::Shape::line(
                line.iter().map(|&p| screen(p)).collect(),
                behind,
            ));
            for run in view3d::visible_runs(&line, surface.cell, &seen) {
                let run: Vec<_> = run.into_iter().map(&screen).collect();
                painter.add(egui::Shape::line(run, stroke));
            }
        }
    }

    /// The height of the 3D view's origin arrows: the machine's Z zero. While Import 3D Model
    /// shows its model (whole or, past the preview faces, as its component), the model sits
    /// on its own zero plane at 0 and the material's outline is drawn from there up to its
    /// thickness, so zero is measured from that top.
    pub(in super::super) fn origin_height_3d(&self) -> f64 {
        let importing = self.importing_model()
            && self.preview.material.is_none()
            && self.player.playback.is_none();
        origin_3d(&self.doc.job, importing)
    }

    /// The vectors, modeling plane and origin over the 3D view's (unwrapped) `surface`, each
    /// as the View menu says, leaving out what the surface hides, a cell at a time.
    pub(super) fn overlays_3d(
        &self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        surface: &Material,
        (orbit, framing): (view3d::Orbit, view3d::Framing),
        fit: egui::Rect,
    ) {
        let shown = self.display;
        let vectors: Vec<(Vec<cut_craft_core::geometry::Point>, Option<[u8; 3]>)> =
            if shown.vectors_3d {
                self.doc
                    .vectors
                    .iter()
                    // construction lines only place things in the 2D view: nothing to see in 3D
                    .filter(|(id, _)| self.doc.shown(**id) && !self.doc.is_construction(**id))
                    .map(|(id, path)| {
                        let points = path.flatten(cut_craft_core::polygon::TOLERANCE);
                        (points, self.doc.colour_of(*id))
                    })
                    .collect()
            } else {
                vec![]
            };
        let stroke = ui.visuals().widgets.noninteractive.fg_stroke;
        // the vectors in the colour the 2D view draws them in (Options ▸ Appearance, or
        // their layer's own)
        let vector = egui::Stroke::new(
            stroke.width,
            crate::view2d::colour32(self.options.appearance.lines(ui.visuals().dark_mode).vector),
        );
        let screen = view3d::screen_of(surface, orbit, framing, fit, None);
        let local = |p: cut_craft_core::geometry::Point, z: f64| {
            cut_craft_core::glam::DVec3::new(p.x - surface.origin.x, p.y - surface.origin.y, z)
        };
        let seen = view3d::seen_of(surface, orbit, framing);
        let in_sight = |points: &[cut_craft_core::glam::DVec3]| {
            view3d::visible_runs(points, surface.cell, &seen)
                .into_iter()
                .map(|run| run.into_iter().map(&screen).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        };
        // each vector flat, in one plane, on top of what it outlines, in its layer's colour
        for (points, own) in &vectors {
            let z = view3d::vector_plane(surface, points);
            let line: Vec<_> = points.iter().map(|&p| local(p, z)).collect();
            let stroke = own.map_or(vector, |c| {
                egui::Stroke::new(stroke.width, crate::view2d::colour32(c))
            });
            for run in in_sight(&line) {
                painter.add(egui::Shape::line(run, stroke));
            }
        }
        // the keep-out zones in red, closed, over whatever they cover
        let danger = egui::Stroke::new(2.0 * stroke.width, ui.visuals().error_fg_color);
        let zones: Vec<_> = if shown.keep_out_3d {
            self.doc.named_keep_out().collect()
        } else {
            vec![]
        };
        for zone in zones {
            let mut points = zone.path.flatten(cut_craft_core::polygon::TOLERANCE);
            points.extend(points.first().copied());
            let z = view3d::vector_plane(surface, &points);
            let line: Vec<_> = points.iter().map(|&p| local(p, z)).collect();
            for run in in_sight(&line) {
                painter.add(egui::Shape::line(run, danger));
            }
            // its name at its top-left corner, as in the 2D view
            if shown.keep_out_labels {
                let b = zone.path.bounds();
                let corner = local(cut_craft_core::geometry::Point::new(b.min.x, b.max.y), z);
                crate::view2d::zone_label(painter, ui, screen(corner), &zone.label());
            }
        }
        let size = view3d::plan_size(surface);
        // the plane's outline, and always while a model is imported onto it
        if shown.modeling_plane || self.form.as_ref().is_some_and(|f| f.model.is_some()) {
            let corners = [
                (0.0, 0.0),
                (size.x, 0.0),
                (size.x, size.y),
                (0.0, size.y),
                (0.0, 0.0),
            ];
            let line = corners
                .iter()
                .map(|&(x, y)| cut_craft_core::glam::DVec3::new(x, y, 0.0))
                .collect::<Vec<_>>();
            let dash = ui.spacing().item_spacing.x;
            for run in in_sight(&line) {
                painter.add(egui::Shape::dashed_line(&run, stroke, dash, dash / 2.0));
            }
        }
        if shown.origin {
            // X, Y and Z from the job's origin, a tenth of the job long, in the axis colours
            // used everywhere, each with an arrow head and its letter at the + end, at the
            // height Z zero is: the material's top or the bed under it
            let o = local(
                cut_craft_core::geometry::Point::ZERO,
                self.origin_height_3d(),
            );
            let reach = size.x.max(size.y) / 10.0;
            for axis in crate::axes::Axis::ALL {
                let tip = o + axis.direction() * reach;
                let (runs, ends) = (in_sight(&[o, tip]), (screen(o), screen(tip)));
                origin_arrow(ui, painter, runs, ends, axis, stroke.width * 2.0);
            }
        }
    }
}
