//! The frame itself: the window's bars and drop-downs, the 2D view and its gestures,
//! snapping, node editing, transform handles, autosave and the Options windows.

use super::{
    APP_NAME, Align, Canvas, Change, CutCraftApp, DAffine2, DESIGN_PANEL_WIDTHS, DESIGN_TABS,
    DROP_SHADE, Drag, EntityId, Gesture, Grab, Handle, Info, JOB_EXTENSION, Layout,
    OLD_JOB_EXTENSION, Overlay, Path, Point, RESOLUTIONS, Rect, STROKE_TOOLS, Screen, Split,
    TOOL_ICON_ROWS, VIEW_BUTTONS, VIEW_ZOOM_QUARTERS, Value, View, ViewTool, ViewTransform,
    bordered, dialog_size, egui, file_dialog, file_name, files, gui, handles, inspected, measured,
    paint_focus_ring, view2d,
};
use crate::options::WindowLayout;

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
        self.shortcuts(ctx);
        egui::Panel::top("menu_bar").show(ui, |ui| self.menu_bar(ui));
        egui::Panel::bottom("status_bar").show(ui, |ui| self.status_bar(ui));
        if !self.options.appearance.system_title_bar {
            super::title_bar::resize_edges(ctx);
        }
        self.tool_database_window(ctx);
        self.machine_windows(ctx);
        self.info_window(ctx);
        self.snap_options_window(ctx);
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
                let width = DESIGN_PANEL_WIDTHS * ctx.global_style().spacing.interact_size.x;
                egui::Panel::left("design_panel")
                    .default_size(width)
                    .show(ui, |ui| {
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

    pub(super) fn view_2d(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "2D view");
        let job = self.shown_job();
        let mut canvas = Canvas::new(ui, &job, self.camera);
        let area = canvas.area;
        self.zoom_and_pan(ui, &mut canvas);
        let reach = canvas.pick_tolerance(ui);
        let gesture = canvas.gesture(ui);
        // a guide being dragged takes the drag over
        let gesture = if self.guide_gesture(ui, &canvas, reach) {
            None
        } else {
            self.snapped(ui, gesture, canvas.transform.scale)
        };
        self.track_stroke(gesture);
        let placing_tabs = self.placing_tabs();
        let overlay = match (gesture, self.view_tool, self.drawing_tool()) {
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
            (Some(Gesture::Dragging(drag)), _, Some(id)) => {
                Overlay::Preview(self.preview(id, drag))
            }
            (Some(Gesture::Dragging(drag)), _, None) => self.select_drag_overlay(drag, reach),
            _ => Overlay::None,
        };
        canvas.paint(
            ui,
            &self.doc,
            &overlay,
            self.selected_toolpath,
            &self.display,
        );
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
            (Some(Gesture::Dropped { drag, .. }), None, Some(id)) => self.draw(id, drag),
            (Some(Gesture::Dropped { drag, shift }), None, None) => {
                self.select_drop(drag, shift, reach);
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

    /// A click while placing tabs: on a tab takes it off, on a selected vector adds one there.
    pub(super) fn place_tab(&mut self, at: Point, reach: f64) {
        let mut points = self.tab_points();
        if let Some(i) = points.iter().position(|p| p.distance(at) <= reach) {
            points.remove(i);
            self.status.set_message("Tab removed");
        } else {
            let on = self
                .doc
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
                .filter(|p| p.distance(at) <= reach);
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
        match gesture {
            Some(Gesture::Dropped { drag, .. }) => {
                self.status
                    .set_message(&measured(drag.start, drag.end, self.doc.job.units));
            }
            Some(Gesture::Click { at, .. }) => {
                let message = self.doc.pick(at, reach).map_or_else(
                    || "click a vector, or drag between two points".to_owned(),
                    |id| inspected(&self.doc.vectors[&id], at, reach, self.doc.job.units),
                );
                self.status.set_message(&message);
            }
            _ => {}
        }
    }

    /// The 2D view's right-click menu: a node's while node editing, else the selection's.
    pub(super) fn view_menu(&mut self, ui: &egui::Ui, canvas: &Canvas, reach: f64) {
        let pressed = ui
            .input(|i| i.pointer.press_origin())
            .map(|q| canvas.transform.to_job(q));
        let node_target = pressed
            .filter(|_| self.view_tool == Some(ViewTool::NodeEdit))
            .and_then(|at| self.node_target(at, reach));
        let selection_menu = || match node_target {
            Some(target) => gui::node_menu(&target),
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
        Overlay::Nodes { at, moving }
    }

    /// A drag from a node moves it; any other drag selects, as with the Select tool.
    pub(super) fn node_drop(&mut self, drag: Drag, shift: bool, reach: f64) {
        if let Some(params) = self.node_move(drag, reach) {
            if let Err(e) = self.apply("move_node", &params) {
                self.status.set_message(&e);
            }
        } else {
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
        cmd.drag_params(drag, &self.with_stroke(id, &form.params))
            .and_then(|params| cmd.run_json(&self.doc, &params))
            .map(|change| change.added().cloned().collect())
            .unwrap_or_default()
    }

    /// The Help windows, Toolpath Summary and Snapping Options.
    pub(super) fn open_window(&mut self, id: &str) {
        self.info = match id {
            "keyboard_shortcuts" => Some(Info::Shortcuts),
            "third_party_licenses" => Some(Info::Licenses),
            "toolpath_summary" => Some(Info::Summary),
            "about_cut_craft" => Some(Info::About),
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
