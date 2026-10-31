//! The frame itself: the window's bars and drop-downs, the 2D view and its gestures,
//! snapping, node editing, transform handles, autosave and the Options windows.

use super::{
    APP_NAME, Align, BRUSH_FINE_STEP, BRUSH_STEP, Canvas, Change, CutCraftApp, DAffine2,
    DESIGN_TABS, DROP_SHADE, Drag, EntityId, Gesture, Grab, Handle, Info, JOB_EXTENSION, Layout,
    OLD_JOB_EXTENSION, Overlay, Path, Point, RESOLUTIONS, Rect, SCROLL_NOTCH, STROKE_TOOLS, Screen,
    Split, TOOL_ICON_ROWS, UNITS, VIEW_BUTTONS, VIEW_ZOOM_QUARTERS, Value, View, ViewTool,
    ViewTransform, bordered, dialog_size, egui, file_dialog, file_name, files, gui, handles,
    inspected, inspected_rows, measured, measured_rows, paint_focus_ring, side_panel, view2d,
};
use crate::options::WindowLayout;
use cut_craft_core::document::Units;

impl CutCraftApp {
    /// One pass of the whole window on the root `ui`; apart from the eframe entry so tests
    /// can drive it headlessly.
    pub(super) fn pass(&mut self, ui: &mut egui::Ui) {
        let ctx = &ui.ctx().clone();
        self.draw_window(ui);
        self.dropped_files(ctx);
        self.follow_form(ctx);
        self.autosave(ctx);
        self.report_bugs(ctx);
        self.space_mouse_input(ctx);
        if let Some(mut shots) = self.shots.take() {
            shots.step(self, ctx);
            self.shots = Some(shots);
        }
    }

    /// Whether Model ▸ Model Resolution's item `id` is the job's resolution; `None` for any
    /// other item.
    pub(super) fn resolution_checked(&self, id: &str) -> Option<bool> {
        RESOLUTIONS
            .iter()
            .find(|(item, ..)| *item == id)
            .map(|&(_, resolution, _)| self.doc.job.resolution == resolution)
    }

    /// Whether Edit ▸ Units' item `id` is the job's units; `None` for any other item.
    pub(super) fn units_checked(&self, id: &str) -> Option<bool> {
        UNITS
            .iter()
            .find(|(item, ..)| *item == id)
            .map(|&(_, units, _)| self.doc.job.units == units)
    }

    /// Edit ▸ Units: what every length reads in from now on. Lengths are kept in
    /// millimetres, so the part stays the same size; tools read in their own units.
    pub(super) fn set_units(&mut self, id: &str) {
        let Some(&(_, units, name)) = UNITS.iter().find(|(item, ..)| *item == id) else {
            return;
        };
        let mut c = Change::builder(&self.doc);
        c.set_job(cut_craft_core::document::Job {
            units,
            ..self.doc.job
        });
        let cell = c.finish();
        let done = format!("Units: {name}; every length now reads in {name}, tools in their own");
        self.record(Ok(cell), &done);
    }

    /// The pointer's position over the 2D view in the job's units: the figures, and the
    /// unit they are in.
    pub(super) fn coordinates(&self) -> Option<(String, &'static str)> {
        let p = self.cursor?;
        Some(match self.doc.job.units {
            Units::Mm => (format!("X {:8.2}  Y {:8.2}", p.x, p.y), "mm"),
            Units::Inch => (format!("X {:8.3}  Y {:8.3}", p.x / 25.4, p.y / 25.4), "in"),
        })
    }

    /// Model ▸ Model Resolution: the job's level of detail for models made from now on, as
    /// Job Setup sets it.
    pub(super) fn set_resolution(&mut self, id: &str) {
        let Some(&(_, resolution, name)) = RESOLUTIONS.iter().find(|(item, ..)| *item == id) else {
            return;
        };
        let mut c = Change::builder(&self.doc);
        c.set_job(cut_craft_core::document::Job {
            resolution,
            ..self.doc.job
        });
        let cell = c.finish();
        // models already made keep their cells; their detail came from what they were made from
        let done =
            format!("Model resolution: {name}; new models (and those imported again) use it");
        self.record(Ok(cell), &done);
    }

    /// Files dragged over the window are shown a drop will take them; dropped, each is opened,
    /// imported or installed as the menus would.
    pub(super) fn dropped_files(&mut self, ctx: &egui::Context) {
        let (hovering, dropped) = ctx.input(|i| {
            (
                !i.raw.hovered_files.is_empty(),
                i.raw
                    .dropped_files
                    .iter()
                    .map(|f| f.path().to_path_buf())
                    .collect::<Vec<_>>(),
            )
        });
        if hovering {
            let screen = ctx.content_rect();
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::unique("file_drop"),
            ));
            let shade = ctx
                .global_style()
                .visuals
                .extreme_bg_color
                .gamma_multiply(DROP_SHADE);
            painter.rect_filled(screen, 0.0, shade);
            painter.text(
                screen.center(),
                egui::Align2::CENTER_CENTER,
                "Drop to open or import",
                egui::TextStyle::Heading.resolve(&ctx.global_style()),
                ctx.global_style().visuals.strong_text_color(),
            );
        }
        for path in dropped {
            self.drop_file(&path);
        }
    }

    /// One dropped file: a job opens; a post-processor installs; a tool database's tools are
    /// added; anything else is imported into the open job.
    pub(super) fn drop_file(&mut self, path: &std::path::Path) {
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let job = [
            JOB_EXTENSION,
            OLD_JOB_EXTENSION,
            cut_craft_core::file::TEMPLATE_EXTENSION,
        ];
        match ext.as_str() {
            e if job.contains(&e) => {
                if self.confirm_discard() {
                    self.open_path(path);
                }
            }
            "pp" => self.install_post(path),
            e if files::TOOL_FILE_EXTENSIONS.contains(&e) && e != "json" => self.import_tools(path),
            _ if self.screen != Screen::Job => self.status.set_message(&format!(
                "Open or make a job to import {} into",
                file_name(path)
            )),
            _ => self.import_file(path),
        }
    }

    /// Right-click ▸ Report a Bug Here…, over whatever was drawn.
    pub(super) fn report_bugs(&mut self, ctx: &egui::Context) {
        // the reporter is taken out while the app answers its questions
        let mut reporter = std::mem::replace(
            &mut self.reporter,
            cut_craft_bug_report::Reporter::new("", None),
        );
        let state = || self.bug_state();
        let pick = || {
            file_dialog()
                .add_filter("Pictures", &["png", "jpg", "jpeg", "gif", "bmp"])
                .pick_files()
                .unwrap_or_default()
        };
        let host = cut_craft_bug_report::Host {
            state: &state,
            pick_pictures: &pick,
        };
        let message = reporter.frame(ctx, &host);
        self.reporter = reporter;
        if let Some(message) = message {
            self.status.set_message(&message);
        }
    }

    /// What a bug report says the app was doing.
    pub(super) fn bug_state(&self) -> String {
        let job = &self.doc.job;
        let form = self.form.as_ref().map_or("none", |f| f.id);
        [
            format!(
                "Cut Craft {} ({}), {}",
                env!("CARGO_PKG_VERSION"),
                option_env!("CUT_CRAFT_COMMIT").unwrap_or("unknown commit"),
                std::env::consts::OS
            ),
            format!("Screen: {:?}; view: {:?}; split: {:?}", self.screen, self.view, self.split),
            format!(
                "Design tab: {}; form: {form}; view tool: {:?}",
                DESIGN_TABS.get(self.design_tab).copied().unwrap_or_default(),
                self.view_tool
            ),
            format!(
                "Job: {} x {} x {} mm ({:?}); {} vectors ({} selected), {} components, {} toolpaths",
                job.width,
                job.height,
                job.thickness,
                job.units,
                self.doc.vectors.len(),
                self.doc.selection.len(),
                self.doc.components.len(),
                self.doc.toolpaths.len()
            ),
            format!("Status: {}", self.status.message),
        ]
        .join("\n")
    }

    pub(super) fn draw_window(&mut self, ui: &mut egui::Ui) {
        let ctx = &ui.ctx().clone();
        self.icons.preload(ctx, self.options.general.preload_icons);
        // picks are view state: undo or another sheet can take what they name away
        let doc = &mut self.doc;
        doc.selected_components
            .retain(|id| doc.components.contains_key(id));
        doc.selected_bitmaps
            .retain(|id| doc.bitmaps.contains_key(id));
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.confirm_discard() {
                self.clear_recovery();
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
        }
        paint_focus_ring(ctx);
        let title = match self.screen {
            Screen::Start => APP_NAME.to_owned(),
            _ => self.session.title(APP_NAME, &self.history),
        };
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
        self.polyline_keys(ctx);
        self.sculpt_keys(ctx);
        self.shaded_image_arrived();
        self.shortcuts(ctx);
        egui::Panel::top("menu_bar").show(ui, |ui| self.menu_bar(ui));
        egui::Panel::bottom("status_bar").show(ui, |ui| self.status_bar(ui));
        if !self.options.appearance.system_title_bar {
            super::title_bar::resize_edges(ctx);
        }
        self.tool_database_window(ctx);
        self.machine_windows(ctx);
        self.info_window(ctx);
        self.save_toolpaths_window(ctx);
        self.measure_window(ctx);
        self.snap_options_window(ctx);
        self.job_sheet_window(ctx);
        self.stl_window(ctx);
        self.options_window(ctx);
        self.tool_search(ctx);
        match self.screen {
            Screen::Start => {
                egui::Panel::left("start_page").show(ui, |ui| self.start_page(ui));
                egui::CentralPanel::default().show(ui, Self::splash);
                (self.view_transform, self.cursor) = (None, None);
                return;
            }
            Screen::NewJob => {
                side_panel(egui::Panel::left("design_panel"), ctx).show(ui, |ui| {
                    egui::Panel::bottom("form_buttons").show(ui, |ui| self.form_buttons(ui));
                    egui::ScrollArea::vertical().show(ui, |ui| self.command_form(ui));
                });
            }
            Screen::Job => match self.options.general.layout {
                WindowLayout::Classic => self.job_pages(ui),
                WindowLayout::Rail => self.rail_layout(ui),
                WindowLayout::Ribbon => self.ribbon_layout(ui),
            },
        }
        egui::CentralPanel::default().show(ui, |ui| {
            self.top_bar(ui);
            ui.separator();
            if let Some(split) = self.split {
                let whole = ui.available_rect_before_wrap();
                let (first, second) = match split {
                    Split::Stacked => whole.split_top_bottom_at_fraction(0.5),
                    Split::SideBySide => whole.split_left_right_at_fraction(0.5),
                };
                ui.scope_builder(egui::UiBuilder::new().max_rect(first), |ui| {
                    ui.set_clip_rect(first);
                    self.view_2d(ui);
                });
                ui.scope_builder(egui::UiBuilder::new().max_rect(second), |ui| {
                    ui.set_clip_rect(second);
                    self.view_3d(ui);
                });
                return;
            }
            match self.view {
                View::TwoD => self.view_2d(ui),
                View::ThreeD => {
                    self.view_3d(ui);
                    self.cursor = None;
                }
            }
        });
    }

    /// The top bar's sheet, layer and level drop-downs: a command to apply, if one was
    /// picked; actions to invoke go in `clicked`.
    pub(super) fn drop_downs(
        &self,
        ui: &mut egui::Ui,
        clicked: &mut Option<&'static str>,
    ) -> Option<(&'static str, Value)> {
        let doc = &self.doc;
        let mut apply = None;
        let edge = ui.spacing().interact_size.y;
        // each drop-down as Aspire's: its icon, then the name with a drop-down arrow
        let drop_down =
            |ui: &mut egui::Ui, icon: &str, name: &str, contents: &mut dyn FnMut(&mut egui::Ui)| {
                if let Some(path) = self.icons.find(icon, "") {
                    ui.add(
                        self.icons
                            .image(path)
                            .fit_to_exact_size(egui::vec2(edge, edge)),
                    );
                }
                egui::ComboBox::from_id_salt(icon)
                    .selected_text(name)
                    .show_ui(ui, |ui| contents(ui));
            };
        let sheet = doc
            .sheets
            .get(doc.active_sheet)
            .map_or("", |s| s.name.as_str());
        drop_down(ui, "view/interface_layout.png", sheet, &mut |ui| {
            bordered(ui, |ui| {
                for (i, s) in doc.sheets.iter().enumerate() {
                    if ui.radio(i == doc.active_sheet, &s.name).clicked() {
                        apply = Some(("activate_sheet", serde_json::json!({ "sheet": i })));
                        ui.close();
                    }
                }
            });
            bordered(ui, |ui| {
                if ui.button("Add New Sheet...").clicked() {
                    *clicked = Some("add_sheet");
                    ui.close();
                }
            });
        });
        let layer = doc.layer(doc.current_layer).map_or("", |l| l.name.as_str());
        drop_down(ui, "view/layers.png", layer, &mut |ui| {
            bordered(ui, |ui| {
                for l in doc.layers.iter().rev() {
                    if ui.radio(l.id == doc.current_layer, &l.name).clicked() {
                        let params = serde_json::json!({ "layer": l.id });
                        apply = Some(("set_current_layer", params));
                        ui.close();
                    }
                }
            });
            bordered(ui, |ui| {
                if ui.button("Add New Layer...").clicked() {
                    *clicked = Some("add_layer");
                    ui.close();
                }
            });
        });
        // new components go on the first level
        let level = doc.levels.values().next().map_or("", |l| l.name.as_str());
        drop_down(ui, "view/component_levels.png", level, &mut |ui| {
            bordered(ui, |ui| {
                for (&id, l) in doc.levels.iter().rev() {
                    let mut shown = l.visible;
                    if ui.checkbox(&mut shown, &l.name).changed() {
                        let params = cmd_level_properties::Params {
                            visible: shown,
                            ..cmd_level_properties::Params::of(id, l)
                        };
                        let params = serde_json::to_value(params).expect("params serialize");
                        apply = Some(("level_properties", params));
                    }
                }
            });
        });
        apply
    }

    /// The top bar's geometry, smart and grid snapping toggles, right to left.
    pub(super) fn snap_toggles(
        &self,
        ui: &mut egui::Ui,
        edge: f32,
        clicked: &mut Option<&'static str>,
    ) {
        let s = &self.snap;
        for (id, on, icon, tip) in [
            (
                "toggle_grid_snapping",
                s.snap_to_grid,
                "grid",
                "Grid Snapping",
            ),
            (
                "toggle_smart_snapping",
                s.smart_snapping,
                "smart",
                "Smart Snapping",
            ),
            (
                "toggle_geometry_snapping",
                s.geometry_snapping,
                "geometry",
                "Geometry Snapping",
            ),
        ] {
            let state = if on { "on" } else { "off" };
            let path = format!("snapping/{icon}_snapping_{state}.png");
            let button = self.icon_button(&path, "", tip, edge).selected(on);
            if ui.add(button).on_hover_text(tip).clicked() {
                *clicked = Some(id);
            }
        }
    }

    /// The top bar's split and snapping toggles.
    pub(super) fn top_bar_toggle(&mut self, id: &str) {
        match id {
            "split_stacked" | "split_side_by_side" => {
                let split = if id == "split_stacked" {
                    Split::Stacked
                } else {
                    Split::SideBySide
                };
                self.split = (self.split != Some(split)).then_some(split);
            }
            "toggle_geometry_snapping" => self.snap.geometry_snapping ^= true,
            "toggle_smart_snapping" => self.snap.smart_snapping ^= true,
            _ => self.snap.snap_to_grid ^= true,
        }
    }

    /// Aspire's top bar over the views: 2D / 3D and the split, the sheet, layer and level
    /// drop-downs, and the geometry, smart and grid snapping toggles.
    pub(super) fn top_bar(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "Top bar");
        let edge = ui.spacing().interact_size.y;
        let (mut clicked, mut apply) = (None::<&str>, None::<(&str, Value)>);
        let height = edge + 2.0 * ui.spacing().button_padding.y;
        let (bar, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), height),
            egui::Sense::hover(),
        );
        // left: the views and their split
        ui.scope_builder(egui::UiBuilder::new().max_rect(bar), |ui| {
            ui.horizontal_centered(|ui| {
                ui.selectable_value(&mut self.view, View::TwoD, "2D View (F2)");
                ui.selectable_value(&mut self.view, View::ThreeD, "3D View (F3)");
                for (id, icon, tip, split) in [
                    (
                        "split_stacked",
                        "view/split_over_and_under.png",
                        "2D over 3D",
                        Split::Stacked,
                    ),
                    (
                        "split_side_by_side",
                        "view/split_side_by_side.png",
                        "2D beside 3D",
                        Split::SideBySide,
                    ),
                ] {
                    let button = self
                        .icon_button(icon, "", tip, edge)
                        .selected(self.split == Some(split));
                    if ui.add(button).on_hover_text(tip).clicked() {
                        clicked = Some(id);
                    }
                }
            });
        });
        // right: snapping
        ui.scope_builder(egui::UiBuilder::new().max_rect(bar), |ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                self.snap_toggles(ui, edge, &mut clicked);
            });
        });
        // middle: the side, sheet, layer and level, centred on the bar by last frame's width
        let width_id = ui.scope_id().with("top_bar_middle_width");
        let width: f32 = ui.data(|d| d.get_temp(width_id)).unwrap_or_default();
        let middle = egui::Rect::from_center_size(bar.center(), egui::vec2(width, height));
        let drawn = ui
            .scope_builder(egui::UiBuilder::new().max_rect(middle), |ui| {
                ui.horizontal_centered(|ui| {
                    if self.doc.job.double_sided.is_some() {
                        let side = if self.doc.bottom_side {
                            "Bottom Side"
                        } else {
                            "Top Side"
                        };
                        if ui.button(side).on_hover_text("Swap Sides").clicked() {
                            clicked = Some("swap_sides");
                        }
                    }
                    apply = self.drop_downs(ui, &mut clicked);
                })
                .response
                .rect
                .width()
            })
            .inner;
        if (drawn - width).abs() > 0.5 {
            ui.data_mut(|d| d.insert_temp(width_id, drawn));
            ui.ctx().request_repaint();
        }
        if let Some((id, params)) = apply
            && let Err(e) = self.apply(id, &params)
        {
            self.status.set_message(&e);
        }
        if let Some(id) = clicked {
            self.invoke(ui.ctx(), id);
        }
    }

    /// The job the views show: while Job Setup is open, the one being set up.
    pub(super) fn shown_job(&self) -> cut_craft_core::document::Job {
        self.form
            .as_ref()
            .filter(|f| f.id == "job_setup")
            .and_then(|f| self.registry.get(f.id)?.run_json(&self.doc, &f.params).ok())
            .and_then(|change| change.job().copied())
            .unwrap_or(self.doc.job)
    }

    /// Zoom and pan: Aspire's 2D view controls.
    pub(super) fn view_action(&mut self, ctx: &egui::Context, id: &str) {
        let label = match id {
            "pan" => {
                self.view_tool = Some(ViewTool::Pan);
                "Pan"
            }
            "measure_inspect" => {
                self.view_tool = Some(ViewTool::Measure);
                "Measure"
            }
            "transform" => {
                self.rotating = false;
                if self.view_tool == Some(ViewTool::Transform) {
                    self.view_tool = None;
                    "Select"
                } else {
                    self.view_tool = Some(ViewTool::Transform);
                    "Transform"
                }
            }
            "node_edit" => {
                if self.view_tool == Some(ViewTool::NodeEdit) {
                    self.view_tool = None;
                    "Select"
                } else {
                    self.view_tool = Some(ViewTool::NodeEdit);
                    "Node Edit"
                }
            }
            "zoom_box" => {
                self.view_tool = Some(ViewTool::ZoomBox);
                "Zoom Box"
            }
            "zoom_selected" => {
                let margin = ctx.global_style().spacing.interact_size.y;
                match (self.doc.selection_bounds(), self.view_area) {
                    (Some(bounds), Some(area)) if bounds.size().max_element() > 0.0 => {
                        self.camera = Some(ViewTransform::fit(area, margin, bounds));
                    }
                    _ => self.status.set_message("select something to zoom to"),
                }
                return;
            }
            // zoom to job, zoom to drawing (F6)
            _ => {
                self.camera = None;
                return;
            }
        };
        label.clone_into(&mut self.status.current_tool);
    }

    /// Sculpting's keys while its form is open, as Aspire's: 1 to 5 pick the tool, the
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
    fn drawing_gesture(&mut self, id: &'static str, gesture: Option<Gesture>, reach: f64) {
        match gesture {
            gesture if id == "draw_polyline" => self.polyline_gesture(gesture, reach),
            Some(Gesture::Click { at, shift: true }) => self.shift_click_shape(id, at, reach),
            Some(Gesture::Dropped { drag, .. }) => self.draw(id, drag),
            _ => {}
        }
    }

    /// Shift-click in drawing tool `id`'s form: a shape that tool drew, under `at`, is
    /// edited in the form, as Aspire's Shift-click does.
    fn shift_click_shape(&mut self, id: &'static str, at: Point, reach: f64) {
        let shape = self.doc.pick(at, reach).and_then(|v| {
            let (command, params) = self
                .registry
                .drawn_by(&self.doc, self.doc.vectors.get(&v)?)?;
            (command == id).then_some((v, params))
        });
        match shape {
            Some((v, params)) => self.edit_shape(v, id, params),
            None => self
                .status
                .set_message("Shift-click a shape this tool drew to edit it"),
        }
    }

    /// Draw Line / Polyline's keys while it is the tool: S swaps curves and straight lines;
    /// with a polyline begun, Backspace takes back a point (or a typed character), Tab
    /// closes it, Space finishes it and starts another, Esc finishes it (and closes the
    /// form), and numbers typed then Enter place the next point.
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
            let mode = if self.polyline.curved {
                "curves"
            } else {
                "straight lines"
            };
            self.status
                .set_message(&format!("Drawing {mode} (S swaps)"));
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
        let typed: String = typed
            .chars()
            .filter(|c| c.is_ascii_digit() || ".,-aAlL ".contains(*c))
            .collect();
        if !typed.is_empty() {
            for k in [Key::A, Key::L, Key::Comma, Key::Period, Key::Minus]
                .into_iter()
                .chain((0..=9).filter_map(|n| Key::from_name(&n.to_string())))
            {
                key(k);
            }
            self.polyline.typed.push_str(&typed);
            self.status
                .set_message(&format!("Next point: {}", self.polyline.typed));
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
                    .set_message("Type a length, x,y, or an angle A length L"),
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
        if key(Key::Space) {
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
            .set_message("Drawing on from the end: Space or Esc finishes, Tab closes it");
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

    /// A click adds the next point (on the first, it closes the polyline; on the last, it
    /// swaps curves and straight lines); a drag adds one with a smooth span.
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
                    self.polyline.toggle_curves();
                } else {
                    self.polyline.click(at);
                }
            }
            Some(Gesture::Dropped { drag, .. }) => self.polyline.drag(drag.start, drag.end),
            _ => {}
        }
    }

    /// The polyline being drawn, on to the pointer (or along a drag in progress).
    fn polyline_overlay(&self, gesture: Option<Gesture>, canvas: &Canvas) -> Overlay {
        let mut draft = self.polyline.clone();
        let pointer = match gesture {
            Some(Gesture::Dragging(drag)) => {
                draft.drag(drag.start, drag.end);
                None
            }
            _ => canvas.hover().map(|at| canvas.transform.to_job(at)),
        };
        Overlay::Preview(
            draft
                .path(false, pointer.filter(|_| !draft.is_empty()))
                .into_iter()
                .collect(),
        )
    }

    /// Clipart dragged from the Clipart tab and dropped on the 2D view goes where it was
    /// dropped; while Toolpath Tiling is open a double-click on a tile makes it the active
    /// one (on it again, none).
    fn clipart_dropped_2d(&mut self, ui: &egui::Ui, canvas: &Canvas) {
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

    /// The 2D view's drawing, and the snap grid's points while snapping to it.
    fn paint_2d(&self, ui: &egui::Ui, canvas: &view2d::Canvas, overlay: &view2d::Overlay) {
        canvas.paint(
            ui,
            &self.doc,
            overlay,
            self.selected_toolpath,
            &self.display,
        );
        if self.snap.snap_to_grid {
            let look = &self.options.appearance;
            canvas.paint_grid(
                ui,
                self.snap.grid_spacing,
                look.grid_color,
                look.grid_point_size,
            );
        }
    }

    pub(super) fn view_2d(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "2D view");
        let job = self.shown_job();
        let mut canvas = Canvas::new(ui, &job, self.camera);
        self.clipart_dropped_2d(ui, &canvas);
        let area = canvas.area;
        self.zoom_and_pan(ui, &mut canvas);
        let reach = canvas.pick_tolerance(ui);
        let gesture = canvas.gesture(ui);
        // a guide being dragged takes the drag over
        let guided = self.guide_gesture(ui, &canvas, reach);
        let gesture = gesture.filter(|_| !guided);
        let gesture = self.snapped(ui, gesture, canvas.transform.scale);
        self.track_stroke(gesture);
        let placing_tabs = self.placing_tabs();
        let filleting = self.filleting();
        let overlay = match (gesture, self.view_tool, self.drawing_tool()) {
            // Create Fillets: the fillet the corner under the pointer would get, or the
            // fillet a click there would take out
            (None | Some(Gesture::Click { .. }), None, None) if filleting => {
                self.fillet_overlay(canvas.cursor(), reach)
            }
            (None | Some(Gesture::Click { .. }), None, None) if placing_tabs => {
                self.tab_overlay(reach)
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
            (gesture, None, Some("draw_polyline")) => self.polyline_overlay(gesture, &canvas),
            (Some(Gesture::Dragging(drag)), _, Some(id)) => {
                Overlay::Preview(self.preview(id, drag))
            }
            (Some(Gesture::Dragging(drag)), _, None) => self.select_drag_overlay(drag, reach),
            _ => Overlay::None,
        };
        self.paint_2d(ui, &canvas, &overlay);
        // while Job Setup is open a red square marks the datum, as in Aspire
        if self.form.as_ref().is_some_and(|f| f.id == "job_setup") {
            let edge = ui.spacing().interact_size.y / 2.0;
            let at = canvas.transform.to_screen(job.offset);
            ui.painter_at(area).rect_filled(
                egui::Rect::from_center_size(at, egui::vec2(edge, edge)),
                0.0,
                ui.visuals().error_fg_color,
            );
        }
        if self.view_tool == Some(ViewTool::Transform) {
            self.paint_handles(ui, &canvas);
        }
        canvas.rulers(ui, job.units);
        self.view_menu(ui, &canvas, reach);
        self.view_buttons(ui, area);
        self.view_transform = Some(canvas.transform);
        self.view_area = Some(area);
        self.cursor = canvas.cursor();
        match (gesture, self.view_tool, self.drawing_tool()) {
            (Some(Gesture::Dropped { drag, .. }), Some(ViewTool::ZoomBox), _) => {
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
                "Select".clone_into(&mut self.status.current_tool);
            }
            (Some(Gesture::Dropped { drag, shift }), Some(ViewTool::NodeEdit), _) => {
                self.node_drop(drag, shift, reach);
            }
            (_, Some(ViewTool::Measure), _) => self.measure(gesture, reach),
            (Some(Gesture::Click { at, shift }), Some(ViewTool::NodeEdit), _) => {
                self.click_select(at, shift, reach);
            }
            (gesture, Some(ViewTool::Transform), _) => self.transform_gesture(ui, gesture, reach),
            (gesture, None, Some(id)) => self.drawing_gesture(id, gesture, reach),
            (Some(Gesture::Dropped { drag, shift }), None, None) => {
                self.plain_drop(drag, shift, reach, placing_tabs);
            }
            (Some(Gesture::Click { at, .. }), None, None) if filleting => {
                self.fillet_click(at, reach);
            }
            (Some(Gesture::Click { at, .. }), None, None) if placing_tabs => {
                self.place_tab(at, reach);
            }
            (Some(Gesture::Click { at, shift }), None, None) => {
                self.click_select(at, shift, canvas.pick_tolerance(ui));
            }
            _ => {}
        }
    }

    /// Guides: dragged out of a ruler (top: across, left: up and down), dragged along when
    /// unlocked, and dragged back onto a ruler to delete them. True while one is dragged.
    pub(super) fn guide_gesture(&mut self, ui: &egui::Ui, canvas: &Canvas, reach: f64) -> bool {
        if canvas.drag_started()
            && let Some(origin) = ui.input(|i| i.pointer.press_origin())
        {
            let at = canvas.transform.to_job(origin);
            let selecting = self.view_tool.is_none() && self.drawing_tool().is_none();
            self.guide_drag = match canvas.ruler_at(ui, origin) {
                Some(vertical) => Some((vertical, None)),
                None if selecting => self
                    .doc
                    .guides
                    .iter()
                    .position(|g| {
                        let along = if g.vertical { at.x } else { at.y };
                        !g.locked && (along - g.at).abs() <= reach
                    })
                    .map(|i| (self.doc.guides[i].vertical, Some(i))),
                None => None,
            };
        }
        let Some((vertical, index)) = self.guide_drag else {
            return false;
        };
        let Some(pointer) = ui.input(|i| i.pointer.latest_pos()) else {
            return true;
        };
        let at = canvas.transform.to_job(pointer);
        let along = if vertical { at.x } else { at.y };
        if !canvas.drag_stopped() {
            let screen = canvas.transform.to_screen(Point::new(along, along));
            let area = canvas.area;
            let line = if vertical {
                [
                    egui::pos2(screen.x, area.top()),
                    egui::pos2(screen.x, area.bottom()),
                ]
            } else {
                [
                    egui::pos2(area.left(), screen.y),
                    egui::pos2(area.right(), screen.y),
                ]
            };
            ui.painter_at(area)
                .line_segment(line, ui.visuals().selection.stroke);
            return true;
        }
        self.guide_drag = None;
        let onto_ruler = canvas.ruler_at(ui, pointer).is_some();
        let mut guides = self.doc.guides.clone();
        match (index, onto_ruler) {
            (Some(i), true) => {
                guides.remove(i);
            }
            (Some(i), false) => guides[i].at = along,
            (None, false) => guides.push(if vertical {
                cut_craft_core::document::Guide::vertical(along)
            } else {
                cut_craft_core::document::Guide::horizontal(along)
            }),
            (None, true) => {}
        }
        self.set_guides(guides);
        true
    }

    /// Replaces the guides, undoably.
    pub(super) fn set_guides(&mut self, guides: Vec<cut_craft_core::document::Guide>) {
        let mut c = Change::builder(&self.doc);
        c.set_guides(guides);
        self.record(Ok(c.finish()), "Guides changed");
    }

    /// Transform mode while dragging: the selection as the handle would leave it, else the
    /// selection window.
    pub(super) fn transform_overlay(&self, ui: &egui::Ui, drag: Drag, reach: f64) -> Overlay {
        if let Some(t) = self.handle_drag(ui, drag, reach) {
            let doc = &self.doc;
            let moved = doc
                .selection
                .iter()
                .map(|id| doc.vectors[id].transformed(&t));
            return Overlay::Preview(moved.collect());
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
            Grab::Vector(id) => vec![self.doc.vectors[&id].clone()],
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
    pub(super) fn select_drag_overlay(&self, drag: Drag, reach: f64) -> Overlay {
        let Some(grab) = self.grabbed(drag.start, reach) else {
            return Overlay::Window {
                start: drag.start,
                end: drag.end,
                crossing: drag.end.x < drag.start.x,
            };
        };
        let by = cut_craft_core::glam::DAffine2::from_translation(drag.end - drag.start);
        Overlay::Preview(
            self.grabbed_outlines(grab)
                .iter()
                .map(|p| p.transformed(&by))
                .collect(),
        )
    }

    /// A select-mode drag let go: what it took hold of moves (as one undo), as Aspire's
    /// select tool does; a drag from empty space selects by window.
    pub(super) fn select_drop(&mut self, drag: Drag, shift: bool, reach: f64) {
        let Some(grab) = self.grabbed(drag.start, reach) else {
            let ids = self
                .doc
                .window(Rect::new(drag.start, drag.end), drag.end.x < drag.start.x);
            return self.select(ids, shift);
        };
        // a vector or model not yet picked is picked, on its own, as it is dragged
        match grab {
            Grab::Vector(id) => {
                self.doc.selection = [id].into();
                self.doc.selected_components.clear();
            }
            Grab::Component(id) => {
                self.doc.selection.clear();
                self.doc.selected_components = [id].into();
            }
            Grab::Selection => {}
        }
        let by = drag.end - drag.start;
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

    /// The transform a drag in Transform mode makes, if it started on one of the selection's
    /// handles.
    pub(super) fn handle_drag(&self, ui: &egui::Ui, drag: Drag, reach: f64) -> Option<DAffine2> {
        let bounds = self.doc.selection_bounds()?;
        let handle = handles::pick(bounds, self.rotating, drag.start, reach)?;
        let keys = ui.input(|i| handles::Keys {
            alt: i.modifiers.alt,
            shift: i.modifiers.shift,
        });
        Some(handles::transform(
            bounds, handle, drag.start, drag.end, keys,
        ))
    }

    /// Transform mode's handles on the selection: white squares scale, black ones rotate,
    /// and a cross in the middle moves.
    pub(super) fn paint_handles(&self, ui: &egui::Ui, canvas: &Canvas) {
        let Some(bounds) = self.doc.selection_bounds() else {
            return;
        };
        let painter = ui.painter_at(canvas.area);
        let visuals = ui.visuals();
        let stroke = visuals.selection.stroke;
        let half = ui.spacing().interact_size.y * view2d::NODE_HANDLE_SHARE / 2.0;
        for handle in handles::handles(self.rotating) {
            let at = canvas
                .transform
                .to_screen(handles::position(bounds, handle));
            let square = egui::Rect::from_center_size(at, egui::vec2(half, half) * 2.0);
            match handle {
                Handle::Move => {
                    for d in [egui::vec2(half, 0.0), egui::vec2(0.0, half)] {
                        painter.line_segment([at - d, at + d], stroke);
                    }
                }
                Handle::Scale { .. } => {
                    painter.rect_filled(square, 0.0, visuals.extreme_bg_color);
                    painter.rect_stroke(square, 0, stroke, egui::StrokeKind::Middle);
                }
                Handle::Rotate { .. } => {
                    painter.rect_filled(square, 0.0, visuals.text_color());
                }
            }
        }
    }

    /// Aspire's view buttons over the top right of the 2D view, below the ruler.
    pub(super) fn view_buttons(&mut self, ui: &mut egui::Ui, area: egui::Rect) {
        let row = ui.spacing().interact_size.y;
        let edge = TOOL_ICON_ROWS * row;
        let gap = ui.spacing().item_spacing;
        let room = egui::Rect::from_min_max(
            area.min + egui::vec2(row, row) + gap,
            egui::pos2(area.max.x - gap.x, area.max.y),
        );
        let in_hand = |id: &str| match self.view_tool {
            Some(ViewTool::Pan) => id == "pan",
            Some(ViewTool::ZoomBox) => id == "zoom_box",
            Some(ViewTool::NodeEdit) => id == "node_edit",
            Some(ViewTool::Measure) => id == "measure_inspect",
            Some(ViewTool::Transform) => id == "transform",
            None => false,
        };
        let mut clicked = None;
        // Aspire's toolpath drawing toggles over the top left of the 2D view
        ui.scope_builder(egui::UiBuilder::new().max_rect(room), |ui| {
            ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                let shown = self.display.previews_2d;
                let drawing = if shown {
                    "view/2d_toolpath_drawing_on.png"
                } else {
                    "view/2d_toolpath_drawing_off.png"
                };
                for (id, icon, tip, on) in [
                    (
                        "draw_2d_previews",
                        drawing,
                        "Draw 2D Toolpath Previews",
                        shown,
                    ),
                    (
                        "make_2d_previews_solid",
                        "view/solid_2d_toolpath_drawing.png",
                        "Make 2D Previews Solid",
                        self.display.solid_previews,
                    ),
                ] {
                    let button = self.icon_button(icon, "", tip, edge).selected(on);
                    if ui.add(button).on_hover_text(tip).clicked() {
                        clicked = Some(id);
                    }
                }
            });
        });
        ui.scope_builder(egui::UiBuilder::new().max_rect(room), |ui| {
            // laid out from the right, so the first button is the leftmost
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                for (id, label, icon) in VIEW_BUTTONS.into_iter().rev() {
                    let button = self
                        .icon_button(icon, id, label, edge)
                        .selected(in_hand(id));
                    let response = ui
                        .add_enabled(self.available(id), button)
                        .on_hover_text(label)
                        .on_disabled_hover_text(format!("{label}: select something first"));
                    if response.clicked() {
                        clicked = Some(id);
                    }
                }
            });
        });
        if let Some(id) = clicked {
            self.invoke(ui.ctx(), id);
        }
    }

    /// Whether the open form is a profile toolpath placing its tabs by clicking.
    pub(super) fn placing_tabs(&self) -> bool {
        self.form.as_ref().is_some_and(|f| {
            f.id == "profile_toolpath"
                && f.params["tabs"].as_str().is_some_and(|t| t != "none")
                && f.params["tab"]["spacing"] == "placed"
        })
    }

    /// A ring round each clicked tab, the size a click on it must land within.
    fn tab_overlay(&self, reach: f64) -> Overlay {
        Overlay::Preview(
            self.tab_points()
                .into_iter()
                .map(|p| Path::circle(p, reach))
                .collect(),
        )
    }

    /// The open form's clicked tabs.
    fn tab_points(&self) -> Vec<Point> {
        self.form
            .as_ref()
            .and_then(|f| serde_json::from_value(f.params["tab_points"].clone()).ok())
            .unwrap_or_default()
    }

    /// The point on a selected vector nearest `at`, within `reach`.
    fn on_selected(&self, at: Point, reach: f64) -> Option<Point> {
        self.doc
            .selection
            .iter()
            .filter_map(|id| self.doc.vectors.get(id))
            .filter_map(|path| {
                let mut flat = path.flatten(cut_craft_core::polygon::TOLERANCE);
                if path.closed
                    && let Some(&first) = flat.first()
                {
                    flat.push(first);
                }
                cut_craft_core::geometry::polyline_closest(&flat, at)
            })
            .min_by(|p, q| p.distance(at).total_cmp(&q.distance(at)))
            .filter(|p| p.distance(at) <= reach)
    }

    /// A drag with no tool: from a placed tab it moves it, else it selects.
    fn plain_drop(&mut self, drag: Drag, shift: bool, reach: f64, placing_tabs: bool) {
        if placing_tabs && self.on_a_tab(drag.start, reach) {
            self.move_tab(drag, reach);
        } else {
            self.select_drop(drag, shift, reach);
        }
    }

    /// Whether `at` is on one of the open form's placed tabs.
    fn on_a_tab(&self, at: Point, reach: f64) -> bool {
        self.tab_points().iter().any(|t| t.distance(at) <= reach)
    }

    /// A drag from a placed tab moves it along to the selected vector where it is dropped.
    pub(super) fn move_tab(&mut self, drag: Drag, reach: f64) {
        let mut points = self.tab_points();
        let Some(i) = points.iter().position(|p| p.distance(drag.start) <= reach) else {
            return;
        };
        let Some(on) = self.on_selected(drag.end, reach) else {
            self.status
                .set_message("drop a tab on a selected vector to move it there");
            return;
        };
        points[i] = on;
        self.status.set_message("Tab moved");
        if let Some(form) = self.form.as_mut() {
            form.params["tab_points"] = serde_json::json!(points);
        }
    }

    /// A click while placing tabs: on a tab takes it off, on a selected vector adds one there.
    pub(super) fn place_tab(&mut self, at: Point, reach: f64) {
        let mut points = self.tab_points();
        if let Some(i) = points.iter().position(|p| p.distance(at) <= reach) {
            points.remove(i);
            self.status.set_message("Tab removed");
        } else {
            let on = self.on_selected(at, reach);
            let Some(on) = on else {
                self.status.set_message(
                    "click on a selected vector to add a tab, or on a tab to remove it",
                );
                return;
            };
            points.push(on);
            self.status.set_message("Tab added");
        }
        if let Some(form) = self.form.as_mut() {
            form.params["tab_points"] = serde_json::json!(points);
        }
    }

    /// Whether Create Fillets is open: a click on a sharp corner fillets it, a click on a
    /// fillet takes it out.
    pub(super) fn filleting(&self) -> bool {
        self.form.as_ref().is_some_and(|f| f.id == "create_fillets")
    }

    /// Create Fillets' settings aimed at `at`, within `reach` of it.
    fn fillet_params(&self, at: Point, reach: f64) -> Option<cmd_create_fillets::Params> {
        let form = self.form.as_ref().filter(|f| f.id == "create_fillets")?;
        let mut params: cmd_create_fillets::Params =
            serde_json::from_value(form.params.clone()).ok()?;
        params.at = Some(at);
        params.within = reach;
        Some(params)
    }

    /// The vector as a click at `at` would leave it: its corner filleted, or its fillet gone.
    fn fillet_overlay(&self, at: Option<Point>, reach: f64) -> Overlay {
        let Some(at) = at else { return Overlay::None };
        let Some(params) = self.fillet_params(at, reach) else {
            return Overlay::None;
        };
        match cmd_create_fillets::at_click(&self.doc, &params, at) {
            Ok((_, path, _)) => Overlay::Preview(vec![path]),
            Err(_) => Overlay::None,
        }
    }

    /// A click with Create Fillets open: the corner there filleted, or the fillet there
    /// taken out.
    pub(super) fn fillet_click(&mut self, at: Point, reach: f64) {
        let Some(params) = self.fillet_params(at, reach) else {
            return;
        };
        let params = serde_json::to_value(params).expect("fillet settings serialise");
        if let Err(e) = self.apply("create_fillets", &params) {
            self.status.set_message(&e);
        }
    }

    /// The open form's command, if it can be drawn with the mouse.
    pub(super) fn drawing_tool(&self) -> Option<&'static str> {
        let form = self.form.as_ref()?;
        self.registry
            .get(form.id)
            .filter(|c| c.interactive())
            .map(cut_craft_core::DynCommand::id)
    }

    /// Measure / Inspect: a finished drag measures between its ends, a click inspects a vector.
    pub(super) fn measure(&mut self, gesture: Option<Gesture>, reach: f64) {
        let units = self.doc.job.units;
        match gesture {
            Some(Gesture::Dropped { drag, .. }) => {
                self.status
                    .set_message(&measured(drag.start, drag.end, units));
                self.measure_rows = measured_rows(drag.start, drag.end, units);
            }
            Some(Gesture::Click { at, .. }) => {
                if let Some(id) = self.doc.pick(at, reach) {
                    let path = &self.doc.vectors[&id];
                    self.status.set_message(&inspected(path, at, reach, units));
                    self.measure_rows = inspected_rows(path, at, reach, units);
                } else {
                    self.status
                        .set_message("click a vector, or drag between two points");
                }
            }
            _ => {}
        }
    }

    /// Measure / Inspect's form while the tool is on: its last measurement or inspection.
    pub(super) fn measure_window(&mut self, ctx: &egui::Context) {
        if self.view_tool != Some(ViewTool::Measure) {
            return;
        }
        let mut open = true;
        let (id, size) = (egui::Id::unique("measure_inspect"), dialog_size(ctx));
        let rows = &self.measure_rows;
        cut_craft_bug_report::dialog(ctx, id, "Measure / Inspect", size, &mut open, |ui| {
            if rows.is_empty() {
                ui.label("Drag between two points to measure, or click a vector to inspect it.");
            }
            egui::Grid::new("measure_rows")
                .striped(true)
                .show(ui, |ui| {
                    for (name, value) in rows {
                        ui.label(name);
                        ui.monospace(value);
                        ui.end_row();
                    }
                });
        });
        if !open {
            self.view_tool = None;
        }
    }

    /// The 2D view's right-click menu: a node's while node editing, else the selection's.
    pub(super) fn view_menu(&mut self, ui: &egui::Ui, canvas: &Canvas, reach: f64) {
        // a right-click finishes a polyline being drawn, as in Aspire
        if !self.polyline.is_empty() && canvas.response().secondary_clicked() {
            self.finish_polyline(false);
            return;
        }
        let pressed = ui
            .input(|i| i.pointer.press_origin())
            .map(|q| canvas.transform.to_job(q));
        let node_target = pressed
            .filter(|_| self.view_tool == Some(ViewTool::NodeEdit))
            .and_then(|at| self.node_target(at, reach));
        let selection_menu = || match node_target {
            Some(target) => gui::node_menu(&self.doc, &target),
            None => self.menus.selection_menu(&self.doc),
        };
        if let Some(action) = self.popup(canvas.response(), selection_menu) {
            self.invoke(ui.ctx(), &action);
        }
    }

    /// Node handles on the selected vectors, and the dragged node's vector as it would be.
    pub(super) fn node_overlay(&self, gesture: Option<Gesture>, reach: f64) -> Overlay {
        // the dragged node's vector as it would be, drawn over the old one
        let moving = match gesture {
            Some(Gesture::Dragging(drag)) if self.picked_drag(drag, reach).is_some() => self
                .picked_drag(drag, reach)
                .and_then(|params| {
                    let cmd = self.registry.get("move_nodes")?;
                    cmd.run_json(&self.doc, &params).ok()
                })
                .map(|change| change.added().cloned().collect())
                .unwrap_or_default(),
            Some(Gesture::Dragging(drag)) => self
                .node_target(drag.start, reach)
                .and_then(|t| {
                    let path = &self.doc.vectors[&t.vector];
                    let k = cut_craft_core::nodes::node_near(path, drag.start, reach)?;
                    Some(cut_craft_core::nodes::move_node(path, k, drag.end))
                })
                .into_iter()
                .collect(),
            _ => vec![],
        };
        let at = self
            .doc
            .selection
            .iter()
            .flat_map(|id| cut_craft_core::nodes::nodes(&self.doc.vectors[id]))
            .collect();
        let picked = self.picked_nodes.iter().map(|n| n.at.into()).collect();
        Overlay::Nodes { at, picked, moving }
    }

    /// Move Nodes' params when `drag` starts on one of two or more picked nodes.
    pub(super) fn picked_drag(&self, drag: Drag, reach: f64) -> Option<Value> {
        let on_one = self
            .picked_nodes
            .iter()
            .any(|n| Point::from(n.at).distance(drag.start) <= reach);
        (self.picked_nodes.len() > 1 && on_one).then(|| {
            let by = drag.end - drag.start;
            serde_json::json!({
                "nodes": self.picked_nodes,
                "by": cut_craft_core::params::Xy::new(by.x, by.y),
            })
        })
    }

    /// The nodes of the selected, editable vectors inside the box `rect`, for Move Nodes.
    pub(super) fn nodes_in(&self, rect: Rect) -> Vec<cut_craft_core::nodes::NodeAt> {
        self.doc
            .selection
            .iter()
            .filter(|id| self.doc.editable(**id))
            .flat_map(|&vector| {
                cut_craft_core::nodes::nodes(&self.doc.vectors[&vector])
                    .into_iter()
                    .filter(|p| rect.contains(*p))
                    .map(move |p| cut_craft_core::nodes::NodeAt {
                        vector,
                        at: cut_craft_core::params::Xy::new(p.x, p.y),
                        reach: 1e-6,
                    })
            })
            .collect()
    }

    /// A drag from one of several picked nodes moves them all; from a node moves it; a box
    /// round nodes of the selected vectors picks them (Shift adds); any other drag selects,
    /// as with the Select tool.
    pub(super) fn node_drop(&mut self, drag: Drag, shift: bool, reach: f64) {
        if let Some(params) = self.picked_drag(drag, reach) {
            let by = drag.end - drag.start;
            match self.apply("move_nodes", &params) {
                Ok(()) => {
                    for n in &mut self.picked_nodes {
                        n.at = cut_craft_core::params::Xy::new(n.at.x + by.x, n.at.y + by.y);
                    }
                }
                Err(e) => self.status.set_message(&e),
            }
        } else if let Some(params) = self.node_move(drag, reach) {
            self.picked_nodes.clear();
            if let Err(e) = self.apply("move_node", &params) {
                self.status.set_message(&e);
            }
        } else if let picked @ [_, ..] = &self.nodes_in(Rect::new(drag.start, drag.end))[..] {
            if !shift {
                self.picked_nodes.clear();
            }
            self.picked_nodes.extend(picked.iter().cloned());
        } else {
            self.picked_nodes.clear();
            let ids = self
                .doc
                .window(Rect::new(drag.start, drag.end), drag.end.x < drag.start.x);
            self.select(ids, shift);
        }
    }

    /// A selected, editable vector with a node or span within `reach` of `at`.
    pub(super) fn node_target(
        &self,
        at: Point,
        reach: f64,
    ) -> Option<cut_craft_core::nodes::NodeAt> {
        self.doc
            .selection
            .iter()
            .find(|id| {
                let path = &self.doc.vectors[id];
                self.doc.editable(**id)
                    && (cut_craft_core::nodes::node_near(path, at, reach).is_some()
                        || cut_craft_core::nodes::span_near(path, at, reach).is_some())
            })
            .map(|&vector| cut_craft_core::nodes::NodeAt {
                vector,
                at: cut_craft_core::params::Xy::new(at.x, at.y),
                reach,
            })
    }

    /// Move Node's params when `drag` starts on a selected vector's node.
    pub(super) fn node_move(&self, drag: Drag, reach: f64) -> Option<Value> {
        let target = self.node_target(drag.start, reach)?;
        cut_craft_core::nodes::node_near(&self.doc.vectors[&target.vector], drag.start, reach)?;
        let to = cut_craft_core::params::Xy::new(drag.end.x, drag.end.y);
        Some(serde_json::json!({ "from": target, "to": to }))
    }

    /// Paths `id` would create for `drag`, by running the command itself.
    pub(super) fn preview(&self, id: &str, drag: Drag) -> Vec<Path> {
        let (Some(cmd), Some(form)) = (self.registry.get(id), &self.form) else {
            return vec![];
        };
        let mut paths: Vec<Path> = cmd
            .drag_params(drag, &self.with_stroke(id, &form.params))
            .and_then(|params| cmd.run_json(&self.doc, &params))
            .map(|change| change.added().cloned().collect())
            .unwrap_or_default();
        // Freehand's stabilizing circle, round the path's end: the pointer pulls from its edge
        if id == "draw_freehand"
            && let Some(radius) = form.params["stabilize"].as_f64().filter(|r| *r > 0.0)
            && let Some(&end) = cmd_draw_freehand::stabilized(&self.stroke, radius, false).last()
        {
            paths.push(Path::circle(end, radius));
        }
        paths
    }

    /// The quick engrave toolpath Immediate Output saves: the one the form is editing, else
    /// the newest.
    pub(super) fn immediate_output_toolpath(&self) -> Option<EntityId> {
        let editing = self.form.as_ref().and_then(|f| f.editing);
        editing.or_else(|| {
            self.doc
                .toolpaths
                .iter()
                .rev()
                .find(|(_, t)| t.strategy == "quick_engrave_toolpath")
                .map(|(&id, _)| id)
        })
    }

    /// Quick Engrave's Save Toolpath Now: that toolpath alone, through the post picked on
    /// the form, with Save Toolpaths' checks and file dialog.
    pub(super) fn immediate_save(&mut self) {
        let Some(id) = self.immediate_output_toolpath() else {
            return;
        };
        self.selected_toolpath = Some(id);
        self.selected_group = None;
        let form = self.immediate_output.clone();
        self.save_toolpaths(&form);
    }

    /// The Help windows, Toolpath Summary and Snapping Options.
    pub(super) fn open_window(&mut self, id: &str) {
        self.info = match id {
            "keyboard_shortcuts" => Some(Info::Shortcuts),
            "third_party_licenses" => Some(Info::Licenses),
            "toolpath_summary" => Some(Info::Summary),
            "about_cut_craft" => Some(Info::About),
            "help_contents" => Some(Info::Help("contents")),
            _ => {
                self.snap_window = true;
                return;
            }
        };
    }

    /// Drawing and node drags snap to the snap points (Shift holds snapping off).
    pub(super) fn snapped(
        &self,
        ui: &egui::Ui,
        gesture: Option<Gesture>,
        scale: f32,
    ) -> Option<Gesture> {
        let snapping = self.drawing_tool().is_some() || self.view_tool == Some(ViewTool::NodeEdit);
        if !snapping || ui.input(|i| i.modifiers.shift) {
            return gesture;
        }
        let reach = self.snap.snapping_radius / f64::from(scale);
        let snap = |mut drag: Drag| {
            drag.start = self.snap.snap(&self.doc, drag.start, reach);
            drag.end = self.snap.snap(&self.doc, drag.end, reach);
            drag
        };
        gesture.map(|g| match g {
            Gesture::Dragging(drag) => Gesture::Dragging(snap(drag)),
            Gesture::Dropped { drag, shift } => Gesture::Dropped {
                drag: snap(drag),
                shift,
            },
            click @ Gesture::Click { .. } => click,
        })
    }

    /// Edit ▸ Options: Aspire's sections; a change is saved and applied at once.
    pub(super) fn options_window(&mut self, ctx: &egui::Context) {
        let mut open = self.options_window;
        let schema = serde_json::to_value(schemars::schema_for!(crate::options::Options))
            .expect("a schema serialises");
        let mut fields = crate::form::fields(&schema);
        // Advanced only lists on the desktops its settings are for
        if !crate::options::Advanced::applies() {
            fields.retain(|f| f.key != "advanced");
        }
        let mut values = serde_json::to_value(&self.options).expect("options serialise");
        let mut changed = None;
        let page = &mut self.options_page;
        let (id, size) = (egui::Id::unique("options"), dialog_size(ctx));
        // the categories down the left, the chosen one's settings beside them
        cut_craft_bug_report::dialog_with_panels(ctx, id, "Options", size, &mut open, |ui| {
            egui::Panel::left("options_categories")
                .resizable(false)
                .show(ui, |ui| {
                    for (i, f) in fields.iter().enumerate() {
                        if ui.selectable_label(i == *page, &f.label).clicked() {
                            *page = i;
                        }
                    }
                });
            egui::CentralPanel::default().show(ui, |ui| {
                let Some(category) = fields.get(*page) else {
                    return;
                };
                let look = crate::form::Look {
                    units: self.doc.job.units,
                    picture: &|_| None,
                    material: crate::form::material(&self.doc, self.doc.job.units),
                    tool: None,
                };
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let one = std::slice::from_ref(category);
                    if crate::form::show(ui, one, &mut values, &look) {
                        changed = serde_json::from_value::<crate::options::Options>(values).ok();
                    }
                });
            });
        });
        self.options_window = open;
        if let Some(options) = changed {
            options.apply_to(&mut self.display);
            ctx.set_theme(options.appearance.theme.preference());
            let frame = options.appearance.system_title_bar;
            if frame != self.options.appearance.system_title_bar {
                ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(frame));
            }
            self.recent.truncate(options.recent_count());
            self.options = options;
            self.save_options();
        }
    }

    /// Save Toolpaths: the machine and post, which toolpaths go in which file, and Save.
    pub(super) fn save_toolpaths_window(&mut self, ctx: &egui::Context) {
        use crate::files::SaveMode;
        let Some(mut form) = self.save_form.clone() else {
            return;
        };
        let mut open = true;
        let mut save = false;
        let mut machine_changed = None;
        let choices = self.post_choices();
        let default = self.output_post().map(|p| p.name);
        let (picked, group) = (self.selected_toolpath, self.selected_group.as_deref());
        let plan = files::save_plan(&self.doc, form.mode, picked, group, form.group);
        let (id, size) = (egui::Id::unique("save_toolpaths"), dialog_size(ctx));
        cut_craft_bug_report::dialog(ctx, id, "Save Toolpaths", size, &mut open, |ui| {
            ui.strong("Machine and post-processor");
            if !self.machines.machines.is_empty() {
                let active = self.machines.active;
                let shown = self.machines.active().map_or("", |m| m.name.as_str());
                egui::ComboBox::from_label("Machine")
                    .selected_text(shown)
                    .show_ui(ui, |ui| {
                        for (n, m) in self.machines.machines.iter().enumerate() {
                            if ui.selectable_label(n == active, &m.name).clicked() && n != active {
                                machine_changed = Some(n);
                            }
                        }
                    });
            }
            post_combo(ui, "save_post", &mut form.post, &default, &choices);
            ui.separator();
            ui.strong("Toolpaths to save");
            for (mode, label) in [
                (SaveMode::Selected, "Selected toolpath"),
                (SaveMode::VisibleToOneFile, "Visible toolpaths to one file"),
                (
                    SaveMode::VisibleToMultipleFiles,
                    "Visible toolpaths to multiple files",
                ),
            ] {
                ui.radio_value(&mut form.mode, mode, label);
            }
            ui.add_enabled(
                form.mode == SaveMode::VisibleToMultipleFiles,
                egui::Checkbox::new(&mut form.group, "Group where possible"),
            )
            .on_hover_text("Toolpaths in a row that use the same tool go in one file");
            if self.doc.job.double_sided.is_some() {
                ui.checkbox(&mut form.add_side, "Add side to toolpath name");
            }
            ui.separator();
            match &plan {
                Ok(plan) => {
                    for (n, ids) in plan.iter().enumerate() {
                        let names: Vec<String> = ids
                            .iter()
                            .filter_map(|id| self.doc.toolpaths.get(id))
                            .map(|t| format!("{} ({})", t.name, t.tool.name))
                            .collect();
                        ui.label(format!("File {}: {}", n + 1, names.join(", ")));
                    }
                }
                Err(e) => {
                    ui.colored_label(ui.visuals().warn_fg_color, e);
                }
            }
            ui.add_space(ui.spacing().item_spacing.y);
            save = ui
                .add_enabled(plan.is_ok(), egui::Button::new("Save Toolpaths..."))
                .clicked();
        });
        if let Some(n) = machine_changed {
            self.machines.active = n;
            form.post = None;
            self.machines_changed("Machine changed");
        }
        if save {
            self.save_toolpaths(&form);
        }
        self.save_form = open.then_some(form);
    }

    /// Gadgets ▸ Job Setup Sheet Editor: the job sheets' title, colours, logo and sections,
    /// kept with the options.
    pub(super) fn job_sheet_window(&mut self, ctx: &egui::Context) {
        if !self.job_sheet_window {
            return;
        }
        let mut open = true;
        let schema = serde_json::to_value(schemars::schema_for!(fmt_create_job_sheet::Style))
            .expect("a schema serialises");
        let fields = crate::form::fields(&schema);
        let mut values = serde_json::to_value(&self.options.job_sheet).expect("a style serialises");
        let units = self.doc.job.units;
        let (id, size) = (egui::Id::unique("job_sheet_editor"), dialog_size(ctx));
        let mut changed = None;
        let mut pick_logo = false;
        cut_craft_bug_report::dialog(ctx, id, "Job Setup Sheet Editor", size, &mut open, |ui| {
            let picture = |_: &str| None;
            let look = crate::form::Look {
                units,
                picture: &picture,
                material: crate::form::material(&self.doc, units),
                tool: None,
            };
            if crate::form::show(ui, &fields, &mut values, &look)
                && let Ok(style) = serde_json::from_value(values)
            {
                changed = Some(style);
            }
            ui.horizontal(|ui| {
                pick_logo = ui.button("Change Logo...").clicked();
                let has_logo = !self.options.job_sheet.logo.is_empty();
                if ui
                    .add_enabled(has_logo, egui::Button::new("Remove Logo"))
                    .clicked()
                {
                    let mut style = self.options.job_sheet.clone();
                    style.logo.clear();
                    changed = Some(style);
                }
                if ui.button("Restore Default Sheet").clicked() {
                    changed = Some(fmt_create_job_sheet::Style::default());
                }
            });
            ui.weak("A logo is scaled to 165 × 52 pixels at most; landscape pictures suit it.");
        });
        if pick_logo
            && let Some(path) = file_dialog()
                .add_filter("Logo", &["png", "jpg", "jpeg", "gif"])
                .pick_file()
        {
            let ext = path
                .extension()
                .map(|e| e.to_string_lossy().into_owned())
                .unwrap_or_default();
            match std::fs::read(&path)
                .map_err(|e| e.to_string())
                .and_then(|bytes| fmt_create_job_sheet::logo(&bytes, &ext))
            {
                Ok(logo) => {
                    let mut style = self.options.job_sheet.clone();
                    style.logo = logo;
                    changed = Some(style);
                }
                Err(e) => self.status.set_message(&e),
            }
        }
        if let Some(style) = changed {
            self.options.job_sheet = style;
            self.save_options();
        }
        self.job_sheet_window = open;
    }

    /// Export as STL: how the model is triangulated (kept with the options), Triangulate to
    /// see the triangle count and error, and Save STL… to write it.
    pub(super) fn stl_window(&mut self, ctx: &egui::Context) {
        if !self.stl_window {
            return;
        }
        let result = self.stl_result;
        let mut open = true;
        let schema = serde_json::to_value(schemars::schema_for!(fmt_export_as_stl::Settings))
            .expect("a schema serialises");
        let fields = crate::form::fields(&schema);
        let mut values = serde_json::to_value(self.options.stl).expect("settings serialise");
        let units = self.doc.job.units;
        let (id, size) = (egui::Id::unique("export_as_stl"), dialog_size(ctx));
        let (mut changed, mut triangulate, mut save) = (None, false, false);
        cut_craft_bug_report::dialog(ctx, id, "Export Model as STL", size, &mut open, |ui| {
            let picture = |_: &str| None;
            let look = crate::form::Look {
                units,
                picture: &picture,
                material: crate::form::material(&self.doc, units),
                tool: None,
            };
            if crate::form::show(ui, &fields, &mut values, &look)
                && let Ok(settings) = serde_json::from_value(values)
            {
                changed = Some(settings);
            }
            ui.horizontal(|ui| {
                triangulate = ui.button("Triangulate").clicked();
                save = ui.button("Save STL…").clicked();
            });
            if let Some((count, error)) = result {
                ui.label(format!("Num Tri Created: {count}"));
                ui.label(format!("Maximum Error: {}", super::length(error, units)));
            }
        });
        if let Some(settings) = changed {
            self.options.stl = settings;
            self.stl_result = None;
            self.save_options();
        }
        if triangulate || save {
            match fmt_export_as_stl::triangulate(&self.doc, &self.options.stl) {
                Ok(mesh) => {
                    self.stl_result = Some((mesh.triangles.len(), mesh.error));
                    if save {
                        self.save_stl(&mesh.triangles);
                    }
                }
                Err(e) => self.status.set_message(&e.to_string()),
            }
        }
        self.stl_window = open;
    }

    fn save_stl(&mut self, triangles: &[[cut_craft_core::glam::DVec3; 3]]) {
        let Some(path) = file_dialog().add_filter("STL", &["stl"]).save_file() else {
            return;
        };
        let path = path.with_extension("stl");
        let result = std::fs::write(&path, fmt_export_as_stl::binary_stl(triangles));
        self.status.set_message(&result.map_or_else(
            |e| format!("{}: {e}", path.display()),
            |()| format!("Exported {}", path.display()),
        ));
    }

    /// Snapping Options: what drawing and node drags snap to.
    pub(super) fn snap_options_window(&mut self, ctx: &egui::Context) {
        let mut open = self.snap_window;
        let schema = serde_json::to_value(schemars::schema_for!(cut_craft_core::snap::SnapOptions))
            .expect("a schema serialises");
        let fields = crate::form::fields(&schema);
        let mut values = serde_json::to_value(&self.snap).expect("options serialise");
        let units = self.doc.job.units;
        let (id, size) = (egui::Id::unique("snapping_options"), dialog_size(ctx));
        cut_craft_bug_report::dialog(ctx, id, "Snapping Options", size, &mut open, |ui| {
            let picture = |_: &str| None;
            let look = crate::form::Look {
                units,
                picture: &picture,
                material: crate::form::material(&self.doc, units),
                tool: None,
            };
            if crate::form::show(ui, &fields, &mut values, &look)
                && let Ok(options) = serde_json::from_value(values)
            {
                self.snap = options;
            }
        });
        self.snap_window = open;
    }

    /// The wheel zooms about the cursor; the middle button (or the Pan tool) pans.
    pub(super) fn zoom_and_pan(&mut self, ui: &egui::Ui, canvas: &mut Canvas) {
        let area = canvas.area;
        // the wheel zooms about the cursor: a quarter of the view's height doubles the zoom
        if let Some(at) = canvas.hover() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                let factor = 2f32.powf(scroll / (area.height() / VIEW_ZOOM_QUARTERS));
                canvas.transform = canvas.transform.zoomed_about(at, factor);
                self.camera = Some(canvas.transform);
            }
        }
        let pan = match self.view_tool {
            Some(ViewTool::Pan) => canvas.dragged_by(egui::PointerButton::Primary),
            _ => None,
        }
        .or_else(|| canvas.dragged_by(egui::PointerButton::Middle));
        if let Some(by) = pan {
            canvas.transform = canvas.transform.panned(by);
            self.camera = Some(canvas.transform);
        }
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
        let Some(current) = self.form.as_ref().map(|f| self.with_stroke(id, &f.params)) else {
            return;
        };
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
        let hit = self.doc.pick(at, tolerance);
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
        if shift {
            // a grouped vector brings or takes its whole group
            let mut ids = self.doc.selection.clone();
            if let Some(id) = hit {
                let members = self.doc.group_members(id);
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

    /// Selection is view state: applied directly, never recorded for undo.
    /// Picking a grouped vector picks its whole group.
    pub(super) fn select(&mut self, ids: impl IntoIterator<Item = EntityId>, add: bool) {
        let doc = &self.doc;
        let whole = ids.into_iter().flat_map(|id| doc.group_members(id));
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

/// The post-processor to save through: the machine's default (`default`, or why there is
/// none), or one of `choices`.
pub(super) fn post_combo(
    ui: &mut egui::Ui,
    salt: &str,
    post: &mut Option<String>,
    default: &Result<String, String>,
    choices: &[String],
) {
    let shown = post
        .clone()
        .unwrap_or_else(|| default.clone().unwrap_or_else(|e| e));
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt(salt)
            .selected_text(shown)
            .show_ui(ui, |ui| {
                if let Ok(name) = default
                    && ui
                        .selectable_label(post.is_none(), format!("{name} (default)"))
                        .clicked()
                {
                    *post = None;
                }
                for name in choices {
                    let picked = post.as_deref() == Some(name.as_str());
                    if ui.selectable_label(picked, name).clicked() {
                        *post = Some(name.clone());
                    }
                }
            });
        ui.label("Post-processor");
    });
}

/// Quick Engrave's Immediate Output: the toolpath calculated last saved through a post from
/// the form, without closing it. Whether Save Toolpath Now was clicked.
pub(super) fn immediate_output(
    ui: &mut egui::Ui,
    output: &mut super::SaveForm,
    default: &Result<String, String>,
    choices: &[String],
    double_sided: bool,
    ready: bool,
) -> bool {
    let mut save = false;
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.strong("Immediate Output");
        post_combo(ui, "immediate_post", &mut output.post, default, choices);
        if double_sided {
            ui.checkbox(&mut output.add_side, "Add side to toolpath name");
        }
        // ponytail: Aspire's Output Direct to Machine needs a transfer driver; none here
        save = ui
            .add_enabled(ready, egui::Button::new("Save Toolpath Now..."))
            .on_hover_text(
                "Save the toolpath calculated last through the post, keeping the form open",
            )
            .on_disabled_hover_text("Apply first: there is no quick engrave toolpath yet")
            .clicked();
    });
    save
}
