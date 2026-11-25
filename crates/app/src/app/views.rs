//! The frame itself: drawing the window, its bars and drop-downs, dropped files, crash
//! reports and the view buttons' actions. The 2D view and the windows have their own files
//! beside this one: `canvas_2d`, `drawing_input`, `selection_2d`, `view_modes`,
//! `node_editing`, `snapping`, `gestures_2d`, `options_window` and `dialog_windows`.

use super::{
    APP_NAME, Align, Change, CutCraftApp, DESIGN_TABS, DROP_SHADE, EntityId, JOB_EXTENSION, Layout,
    OLD_JOB_EXTENSION, RESOLUTIONS, Screen, Split, UNITS, Value, View, ViewTool, ViewTransform,
    bordered, egui, file_name, files, page_width, paint_focus_ring, show_page,
};
use crate::options::WindowLayout;
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_core::document::Units;
use cut_craft_i18n::tr;

/// Where a crash log is kept when the person chose not to send crash logs.
const LAST_CRASH: &str = "last-crash.log";

/// The extensions of the files that, dropped or opened with Cut Craft, open as a job.
const DROPPED_JOBS: [&str; 3] = [
    JOB_EXTENSION,
    OLD_JOB_EXTENSION,
    cut_craft_core::file::TEMPLATE_EXTENSION,
];

/// How far beyond the selection's corners its rotation handles sit, in pick reaches.
pub(super) const ROTATE_OUT: f64 = 2.5;

/// What the status bar says while stepped inside a group.
pub(super) const INSIDE_GROUP: &str = "Inside a group: click an item to move, size or change it on \
     its own, double-click a group in it to go further in or text to edit it; Esc or a \
     double-click on empty space steps out";

/// How big the selection's handles are, as a share of a row: bigger than a node's, as there
/// are only a few and they are what a drag takes hold of.
pub(super) const SELECTION_HANDLE_SHARE: f32 = 0.55;

/// What the 2D view's clicks go to besides selecting: the selection's handles (when nothing
/// else takes them), tracing a bitmap, filleting corners, placing tabs, joining open ends.
#[derive(Clone, Copy, Default)]
#[expect(clippy::struct_excessive_bools, reason = "the view's modes, one each")]
pub(super) struct Modes {
    pub(super) handled: bool,
    pub(super) tracing: bool,
    pub(super) filleting: bool,
    pub(super) placing_tabs: bool,
    pub(super) joining_ends: bool,
}

/// What an empty sheet says in its middle, and the status bar's step for it.
pub(super) const EMPTY_SHEET_TITLE: &str = "Nothing drawn yet";
/// The empty sheet's links, under its hint.
pub(super) const EMPTY_SHEET_RECENT: &str = "Open a recent file";
pub(super) const EMPTY_SHEET_SAMPLE: &str = "Try a sample project";
pub(super) const EMPTY_SHEET_HINT: &str =
    "Draw a shape with the tools on the Design page, or drop a picture, SVG or DXF here.";
pub(super) const EMPTY_SHEET_STEP: &str = "Nothing drawn yet: pick a drawing tool on the Design page, or drop a picture, SVG or DXF \
     on the sheet";

impl CutCraftApp {
    /// One pass of the whole window on the root `ui`; apart from the eframe entry so tests
    /// can drive it headlessly.
    pub(super) fn pass(&mut self, ui: &mut egui::Ui) {
        let ctx = &ui.ctx().clone();
        // an edit last frame left toolpaths out of date: the status bar says so
        self.note_stale();
        // a job opened or made, or another machine chosen: the machine's zones on it
        self.sync_machine_keep_out();
        // View ▸ Show Construction Lines, on whichever job is open
        self.doc.hide_construction = !self.display.construction;
        // nodes picked on a vector since unselected, undone or put away are let go
        self.prune_picked_nodes();
        // a reorder, or its undo, moved toolpaths between ids: what holds them follows
        self.follow_toolpaths();
        // Swap Sides undone or redone: the preview turns over with the job
        self.follow_side();
        // the Depth add-on's answer, when it has one
        self.take_depth();
        // Generate Border's border, started or made
        self.follow_border(ui.ctx());
        self.draw_window(ui);
        // a form closed having left toolpaths out of date: recalculate them all?
        self.ask_to_recalculate(ctx);
        // a playing preview cut on a frame's worth
        self.advance_playback(ctx);
        // a slow job's progress, over everything else
        self.busy_window(ctx);
        // an import that doesn't say its units, waiting to be told
        self.units_question(ctx);
        // a save that would replace G-code from an earlier save: Replace or Archive?
        self.earlier_question(ctx);
        // what the checks before cutting found: save anyway?
        self.save_anyway_question(ctx);
        // Save or Send pressed in Save Toolpaths: once its window has closed
        self.save_when_form_closed(ctx);
        self.dropped_files(ctx);
        self.follow_form(ctx);
        if !self.tracing() {
            self.trace_view = None;
        }
        self.next_import();
        self.autosave(ctx);
        self.report_bugs(ctx);
        self.space_mouse_input(ctx);
        // calls from the Web API, run as clicks would be
        self.web_api_frame(ctx);
        if let Some(mut shots) = self.shots.take() {
            shots.step(self, ctx);
            self.shots = Some(shots);
        }
        if let Some(mut shots) = self.help_shots.take() {
            shots.step(self, ctx);
            self.help_shots = Some(shots);
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
        let done = format!("Units: {name}; every length now reads in {name}, bits in their own");
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

    /// Whether dropping `path` imports it into the job (it is not a job, a post-processor or
    /// a bit library).
    pub(super) fn drop_imports(path: &std::path::Path) -> bool {
        let ext = path
            .extension()
            .map_or_default(|e| e.to_string_lossy().to_lowercase());
        !(DROPPED_JOBS.contains(&ext.as_str())
            || ext == "pp"
            || (files::TOOL_FILE_EXTENSIONS.contains(&ext.as_str()) && ext != "json"))
    }

    /// One dropped file: a job opens; a post-processor installs; a bit library's tools are
    /// added; anything else is imported into the open job.
    pub(super) fn drop_file(&mut self, path: &std::path::Path) {
        let ext = path
            .extension()
            .map_or_default(|e| e.to_string_lossy().to_lowercase());
        match ext.as_str() {
            e if DROPPED_JOBS.contains(&e) => {
                if self.confirm_discard() {
                    self.open_path(path);
                }
            }
            "pp" => self.install_post(path),
            e if files::TOOL_FILE_EXTENSIONS.contains(&e) && e != "json" => self.import_tools(path),
            _ if self.screen != Screen::Job => self.status.set_message(&cut_craft_i18n::tr!(
                "status-open-a-job-to-import",
                file = file_name(path).clone()
            )),
            _ => self.import_file(path),
        }
    }

    /// A crash last time: sent to the crash-report service or offered as a report, as the
    /// person chose (first-time setup, or Edit ▸ Options ▸ Bug Reports), or, if they said no, kept as `last-crash.log` beside
    /// the settings with a word in the status bar.
    pub(super) fn offer_crash(&mut self, ctx: &egui::Context, crash: String) {
        use crate::options::CrashLogs;
        match self.options.bug_reports.crash_logs {
            CrashLogs::DontSend => {
                let kept = files::config_dir()
                    .map(|d| d.join(LAST_CRASH))
                    .filter(|path| std::fs::write(path, &crash).is_ok());
                self.status.set_message(&kept.map_or_else(
                    || cut_craft_i18n::tr!("status-closed-with-error"),
                    |p| {
                        cut_craft_i18n::tr!(
                            "status-closed-with-error-log",
                            path = p.display().to_string()
                        )
                    },
                ));
            }
            CrashLogs::Send => {
                let install = self
                    .options
                    .bug_reports
                    .install_id
                    .get_or_insert_with(cut_craft_bug_report::relay::new_install_id)
                    .clone();
                self.save_options();
                self.reporter.computer_details = self.options.bug_reports.computer_details;
                self.reporter.send_crash(ctx, crash, &install);
            }
            CrashLogs::Ask => self.reporter.report_crash(crash, false),
        }
    }

    /// Right-click ▸ Report a Bug Here…, over whatever was drawn.
    pub(super) fn report_bugs(&mut self, ctx: &egui::Context) {
        // the reporter is taken out while the app answers its questions
        // what the person said in Options, as it is now
        self.reporter.computer_details = self.options.bug_reports.computer_details;
        let mut reporter = std::mem::replace(
            &mut self.reporter,
            cut_craft_bug_report::Reporter::new("", None),
        );
        let state = || self.bug_state();
        let pick = || {
            super::filtered_dialog(super::Dialog::BugReportPictures, &self.registry)
                .pick_files()
                .unwrap_or_default()
        };
        // in a language pack, a wrong or missing word goes to its translators
        let translation = || {
            let dir = self.addons.dir()?;
            let chosen = &self.options.language.pack;
            let in_use =
                (crate::languages::installed(dir).into_iter()).find(|p| p.id == *chosen)?;
            let url = crate::languages::wording_issues(&in_use);
            Some((in_use.name, url))
        };
        let host = cut_craft_bug_report::Host {
            state: &state,
            pick_pictures: &pick,
            translation: &translation,
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
        if ctx.input(|i| i.viewport().close_requested()) && !self.close_requested() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
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
        self.capture_shortcut(ctx);
        self.polyline_keys(ctx);
        self.sculpt_keys(ctx);
        self.shaded_image_arrived();
        self.shortcuts(ctx);
        egui::Panel::top("menu_bar").show(ui, |ui| self.menu_bar(ui));
        egui::Panel::bottom("status_bar").show(ui, |ui| self.status_bar(ui));
        // a Mac's window always has its own edges to resize by
        if !self.options.appearance.system_title_bar && !super::mac::is_mac(ctx) {
            super::title_bar::resize_edges(ctx);
        }
        self.tool_database_window(ctx);
        self.machine_windows(ctx);
        self.info_window(ctx);
        self.save_toolpaths_window(ctx);
        self.gcode_window(ctx);
        self.machine_files_window(ctx);
        self.measure_window(ctx);
        self.snap_options_window(ctx);
        self.job_sheet_window(ctx);
        self.part_labels_window(ctx);
        self.clearing_window(ctx);
        self.stl_window(ctx);
        self.options_window(ctx);
        self.settings_3d_window(ctx);
        self.tool_search(ctx);
        self.guide_window(ctx);
        self.tool_changer_window(ctx);
        self.fit_import_window(ctx);
        self.updates_window(ctx);
        if self.addons.window(ctx) {
            self.refresh_installed();
        }
        self.welcome_windows(ctx);
        match self.screen {
            Screen::Start => {
                egui::Panel::left("start_page").show(ui, |ui| self.start_page(ui));
                egui::CentralPanel::default().show(ui, Self::splash);
                (self.view_transform, self.cursor) = (None, None);
                return;
            }
            Screen::NewJob => {
                // as wide as the Design page the job opens with
                let width = page_width(ui);
                let rect = show_page(egui::Panel::left("design_panel"), width, ui, |ui| {
                    self.form_panel(ui);
                });
                self.page_rects = [Some(rect), None];
            }
            Screen::Job => self.job_layout(ui),
        }
        egui::CentralPanel::default().show(ui, |ui| {
            self.top_bar(ui);
            ui.separator();
            // a preview playing: its controls over whichever view shows, Stop always to hand
            self.playback_bar(ui);
            if let Some(split) = self.split {
                self.split_views(ui, split);
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

    /// A job's panels around the views, as its window layout arranges them.
    fn job_layout(&mut self, ui: &mut egui::Ui) {
        match self.options.general.layout {
            WindowLayout::Classic => self.job_pages(ui),
            WindowLayout::Rail => self.rail_layout(ui),
            WindowLayout::Ribbon => self.ribbon_layout(ui),
            WindowLayout::Canvas => self.canvas_layout(ui),
        }
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
        // each drop-down: its icon, then the name with a drop-down arrow
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
                if ui.button(tr!("views-add-new-sheet")).clicked() {
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
                if ui.button(tr!("views-add-new-layer")).clicked() {
                    *clicked = Some("add_layer");
                    ui.close();
                }
            });
        });
        // the level new components go on: the picked component's, else the top shown one
        let level = doc
            .levels
            .get(&doc.new_component_level())
            .map_or("", |l| l.name.as_str());
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
        let s = &self.options.snapping;
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
            if ui.add(button).on_hover_hint(tip).clicked() {
                *clicked = Some(id);
            }
        }
    }

    /// The top bar's split and snapping toggles.
    pub(super) fn top_bar_toggle(&mut self, id: &str) {
        match id {
            "split_stacked" | "split_side_by_side" | "split_picture" => {
                let split = match id {
                    "split_stacked" => Split::Stacked,
                    "split_side_by_side" => Split::SideBySide,
                    _ => Split::Picture,
                };
                self.split = (self.split != Some(split)).then_some(split);
            }
            "toggle_geometry_snapping" => self.options.snapping.geometry_snapping ^= true,
            "toggle_smart_snapping" => self.options.snapping.smart_snapping ^= true,
            _ => self.options.snapping.snap_to_grid ^= true,
        }
        if id.starts_with("toggle_") {
            self.save_options();
        }
    }

    /// The top bar over the views: 2D / 3D and the split, the sheet, layer and level
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
                ui.selectable_value(&mut self.view, View::TwoD, tr!("views-2d-view-f2"));
                ui.selectable_value(&mut self.view, View::ThreeD, tr!("views-3d-view-f3"));
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
                    (
                        "split_picture",
                        "view/split_picture_in_picture.png",
                        "One view in a corner of the other",
                        Split::Picture,
                    ),
                ] {
                    let button = self
                        .icon_button(icon, "", tip, edge)
                        .selected(self.split == Some(split));
                    if ui.add(button).on_hover_hint(tip).clicked() {
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
                        if ui
                            .button(side)
                            .on_hover_hint(tr!("views-swap-sides"))
                            .clicked()
                        {
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

    /// Zoom and pan: the 2D view controls.
    pub(super) fn view_action(&mut self, ctx: &egui::Context, id: &str) {
        let tool = match id {
            "pan" => {
                self.view_tool = Some(ViewTool::Pan);
                "pan"
            }
            "measure_inspect" => {
                self.view_tool = Some(ViewTool::Measure);
                "measure_inspect"
            }
            "transform" => {
                self.rotating = false;
                if self.view_tool == Some(ViewTool::Transform) {
                    self.view_tool = None;
                    "select"
                } else {
                    self.view_tool = Some(ViewTool::Transform);
                    "transform"
                }
            }
            "node_edit" => {
                self.node_mirror = [false, false];
                if self.view_tool == Some(ViewTool::NodeEdit) {
                    self.view_tool = None;
                    "select"
                } else {
                    self.view_tool = Some(ViewTool::NodeEdit);
                    "node_edit"
                }
            }
            "zoom_box" => {
                self.view_tool = Some(ViewTool::ZoomBox);
                "zoom_box"
            }
            "zoom_selected" => {
                let margin = ctx.global_style().spacing.interact_size.y;
                match (self.doc.selection_bounds(), self.view_area) {
                    (Some(bounds), Some(area)) if bounds.size().max_element() > 0.0 => {
                        self.camera = Some(ViewTransform::fit(area, margin, bounds));
                    }
                    _ => self
                        .status
                        .set_message(&cut_craft_i18n::tr!("status-select-to-zoom")),
                }
                return;
            }
            // zoom to job, zoom to drawing (F6)
            _ => {
                self.camera = None;
                return;
            }
        };
        self.status.current_tool = self.tool_label(tool);
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
                        .selectable_label(
                            post.is_none(),
                            tr!("views-post-default", name = name.as_str()),
                        )
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
        ui.label(tr!("views-post-processor"));
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
        ui.strong(tr!("views-immediate-output"));
        post_combo(ui, "immediate_post", &mut output.post, default, choices);
        if double_sided {
            ui.checkbox(&mut output.add_side, tr!("views-add-side-to-toolpath-name"));
        }
        // ponytail: the Output Direct to Machine needs a transfer driver; none here
        save = ui
            .add_enabled(ready, egui::Button::new(tr!("views-save-toolpath-now")))
            .on_hover_hint(tr!("views-save-the-toolpath-calculated-last"))
            .on_disabled_hover_text(tr!("views-calculate-first-there-is-no"))
            .clicked();
    });
    save
}

/// Why Save Toolpaths and Send to Machine are greyed while a warning is not ticked.
pub(super) const TICK_TO_SAVE: &str =
    "Tick each ⚠ under Checks before cutting to say you have read it, then save";

/// Save Toolpaths' bit changes: a file for each tool, the tool changer (when the machine
/// has one), or a pause to change the bit by hand.
pub(super) fn tool_changes_section(
    ui: &mut egui::Ui,
    changes: &mut files::ToolChanges,
    has_changer: bool,
) {
    use files::ToolChanges;
    ui.strong(tr!("views-tool-changes"));
    ui.radio_value(
        changes,
        ToolChanges::SeparateFiles,
        tr!("views-separate-file-for-each-tool"),
    )
    .on_hover_hint(tr!("views-the-safest-without-a-tool"));
    ui.add_enabled_ui(has_changer, |ui| {
        ui.radio_value(
            changes,
            ToolChanges::ToolChanger,
            tr!("views-tool-changer-atc"),
        )
        .on_hover_hint(tr!("views-one-file-the-machine-s"))
        .on_disabled_hover_text(tr!("views-no-tool-changer-is-set"));
    });
    ui.radio_value(
        changes,
        ToolChanges::Pause,
        tr!("views-one-file-pause-to-change"),
    )
    .on_hover_hint(tr!("views-the-machine-stops-m0-at"));
}

/// What Save Toolpaths says while tiling is on, with `tiles` tiles.
pub(super) fn tiling_on(tiles: usize) -> String {
    tr!("views-tiling-on", tiles = tiles)
}

/// Save Toolpaths' files: what goes in each, why there is a file for each bit, and that
/// each is saved once per tile while tiling is on.
pub(super) fn files_section(
    ui: &mut egui::Ui,
    doc: &cut_craft_core::document::Document,
    plan: &Result<Vec<Vec<EntityId>>, String>,
    split: Option<&str>,
) {
    let warn = ui.visuals().warn_fg_color;
    // tiling on: each file is saved per tile, never under the name typed alone
    if let Some(tiling) = doc.tiling {
        ui.colored_label(warn, tiling_on(tiling.tiles(doc.job.area()).len()));
    }
    if let Some(why) = split
        && plan.as_ref().is_ok_and(|p| p.len() > 1)
    {
        ui.colored_label(warn, why);
    }
    match plan {
        Ok(plan) => {
            for (n, ids) in plan.iter().enumerate() {
                let names: Vec<String> = ids
                    .iter()
                    .filter_map(|id| doc.toolpaths.get(id))
                    .map(|t| format!("{} ({})", t.name, t.tool.name))
                    .collect();
                ui.label(tr!(
                    "views-file-toolpaths",
                    number = n + 1,
                    toolpaths = names.join(", ")
                ));
            }
        }
        Err(e) => {
            ui.colored_label(warn, e);
        }
    }
}

/// The move handle: a cross of arrows pointing both ways along the job's X (red) and Y
/// (green) through `at`, `along` being which way + X and + Y run on screen, each arm
/// `half` long past the middle and a bit (the 2D and the 3D view alike).
pub(super) fn move_cross(
    painter: &egui::Painter,
    at: egui::Pos2,
    along: [egui::Vec2; 2],
    half: f32,
    width: f32,
) {
    use crate::axes::Axis;
    let reach = half * MOVE_ARM_SHARE;
    for (axis, way) in [Axis::X, Axis::Y].into_iter().zip(along) {
        let way = way.normalized();
        if !way.is_finite() || way == egui::Vec2::ZERO {
            continue;
        }
        let stroke = egui::Stroke::new(width * 1.5, axis.colour());
        crate::axes::both_ways(
            painter,
            at - way * reach,
            at + way * reach,
            stroke,
            reach * 0.45,
        );
    }
}

/// How far the move handle's arrows reach past its middle, in the handle's half size.
const MOVE_ARM_SHARE: f32 = 1.6;
