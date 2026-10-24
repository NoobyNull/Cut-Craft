//! The main window: menus, Design and Toolpaths panels, and the 2D/3D views over the
//! document, its undo history and the command registry.

use crate::files::{self, Session};
use crate::form::{self, Field};
use crate::gui;
use crate::icons::Icons;
use crate::pages::{Page, Pages, Shown};
use crate::view2d::{Canvas, Gesture, Overlay, ViewTransform};
use crate::{commands, shortcut, view3d};
use cut_craft_core::document::{Document, EntityId, LayerId};
use cut_craft_core::file::{JOB_EXTENSION, OLD_JOB_EXTENSION};
use cut_craft_core::geometry::{Path, Point, Rect};
use cut_craft_core::simulation::Material;
use cut_craft_core::toolpath::{Tool, ToolDatabase};
use cut_craft_core::{Change, Drag, History, Registry};
use eframe::egui::{self, Align, Key, KeyboardShortcut, Layout, Modifiers};
use serde_json::Value;
use std::borrow::Cow;
use std::f64::consts::{FRAC_PI_2, PI};

const DESIGN_TABS: [&str; 5] = ["Drawing", "Modeling", "Sheets", "Layers", "Clipart"];
/// Actions the app handles itself rather than through a command.
const BUILT_INS: [&str; 31] = [
    "undo",
    "redo",
    "exit",
    "new",
    "open",
    "save",
    "save_as",
    "import_vectors_bitmap",
    "recalculate_toolpath",
    "recalculate_visible_toolpaths",
    "recalculate_all_toolpaths",
    "delete_toolpath",
    "save_toolpaths",
    "install_post_processor",
    "close",
    "select",
    "select_all_vectors_on_current_layer",
    "job_size_and_position",
    "show_toolpaths_tab",
    "design_layout",
    "machining_layout",
    "interface_layout",
    "pan",
    "zoom_box",
    "zoom_to_job",
    "zoom_to_drawing",
    "zoom_selected",
    "tool_database",
    "preview_toolpath",
    "preview_all_toolpaths",
    "reset_preview",
];
/// Simulation cells along the longer side of the job.
// ponytail: one quality until the Preview Simulation Quality menu is wired
const PREVIEW_CELLS: usize = 1000;
/// 3D view mesh cells along the longer side of the job.
// ponytail: fixed until the 3D view has a detail setting
const MESH_CELLS: usize = 250;
/// Aspire's layout keys: F11 designing (Design pinned), F12 machining (Toolpaths pinned).
const LAYOUT_KEYS: [(&str, &str); 2] = [("design_layout", "F11"), ("machining_layout", "F12")];
/// The zoom buttons over the 2D view: action, label and icon.
const VIEW_BUTTONS: [(&str, &str, &str); 3] = [
    ("zoom_box", "Zoom Box", "view/zoom_box.svg"),
    ("zoom_to_job", "Zoom to Job", "view/zoom_to_job.svg"),
    (
        "zoom_selected",
        "Zoom to Selection",
        "view/zoom_selected.svg",
    ),
];
/// Scrolling a view's height divided by this doubles the zoom.
// ponytail: fixed rate until a preferences dialog exists
const VIEW_ZOOM_QUARTERS: f32 = 4.0;
/// Aspire's startup tasks: action, label and icon.
const STARTUP_TASKS: [(&str, &str, &str); 3] = [
    ("new", "Create a new file", "file/new.svg"),
    (
        "new_from_template",
        "New file from template",
        "toolpaths/load_toolpath_template.svg",
    ),
    ("open", "Open an existing file", "file/open.svg"),
];
/// Beside each recently opened file.
const RECENT_FILE_ICON: &str = "file/new.svg";
/// The design panel's starting width, in the theme's interactive widths: room for Job Setup's
/// pictures beside its choices.
// ponytail: fixed until panel widths are saved with the window layout
const DESIGN_PANEL_WIDTHS: f32 = 7.0;
/// Size of a form section's picture, in rows of the theme's interactive height.
// ponytail: fixed ratio until a preferences dialog exists
const SECTION_PICTURE_ROWS: f32 = 3.0;
/// The Cut Craft mark, shown with no job open.
const LOGO: &str = "assets/cut-craft.svg";
/// How much of the middle of the window the mark takes, by its shorter side.
// ponytail: fixed share until there is a preferences dialog
const SPLASH_SHARE: f32 = 1.0 / 3.0;
const APP_NAME: &str = "Cut Craft";
/// How the desktop knows the window: the same name as the launcher, `cut-craft.desktop`
/// (see `scripts/install-launcher`), so the taskbar shows its icon.
const APP_ID: &str = "cut-craft";
/// Tool icon edge, in rows of the theme's interactive height.
// ponytail: fixed ratio until a preferences dialog exists
const TOOL_ICON_ROWS: f32 = 1.75;

/// What the window shows: Aspire's start page, Job Setup for a new job, or the job.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    /// No job open: the startup tasks and recently opened files.
    Start,
    /// Job Setup for a new job; OK makes it, Cancel goes back to the start page.
    NewJob,
    Job,
}

/// A 2D view tool that takes the left button from drawing and selecting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ViewTool {
    Pan,
    /// One box, then back to selecting.
    ZoomBox,
}

#[derive(PartialEq)]
enum View {
    TwoD,
    ThreeD,
}

/// A command whose settings form is open.
struct OpenForm {
    id: &'static str,
    label: &'static str,
    icon: &'static str,
    fields: Vec<Field>,
    params: Value,
    error: Option<String>,
    /// Job Setup for a new job: OK and Cancel instead of Apply and Close.
    new_job: bool,
    /// The tool page it shows in: Toolpaths for toolpath strategies, else Design.
    page: Page,
}

struct CutCraftApp {
    screen: Screen,
    doc: Document,
    history: History,
    registry: Registry,
    icons: Icons,
    menus: gui::MenuSystem,
    /// The menus with no job open.
    start_menus: gui::MenuSystem,
    toolbar: gui::Toolbar,
    status: gui::StatusBar,
    design_tab: usize,
    view: View,
    cursor: Option<Point>,
    form: Option<OpenForm>,
    /// How the 2D view mapped the job last frame.
    view_transform: Option<ViewTransform>,
    /// Where the 2D view was on screen last frame.
    view_area: Option<egui::Rect>,
    /// The user's zoom and pan; `None` fits the job to the view.
    camera: Option<ViewTransform>,
    view_tool: Option<ViewTool>,
    /// The Design and Toolpaths pages: pinned open, or hidden to a side tab.
    pages: Pages,
    session: Session,
    /// Toolpath picked in the Toolpaths panel.
    selected_toolpath: Option<EntityId>,
    /// Installed post-processor; without one, toolpaths save as generic G-code.
    post: Option<cut_craft_post::Post>,
    tools: ToolDatabase,
    /// Where `tools` is saved; `None` keeps it in memory (tests, or a file that failed to load).
    tools_path: Option<std::path::PathBuf>,
    tool_window: bool,
    /// The tool shown in the Tool Database window.
    tool_selected: usize,
    /// The material as cut by the previewed toolpaths, and its picture once drawn.
    simulation: Option<Material>,
    /// `simulation` at the 3D view's mesh resolution.
    simulation_coarse: Option<Material>,
    orbit: view3d::Orbit,
    /// Jobs opened or saved lately, newest first.
    recent: Vec<std::path::PathBuf>,
    /// Where `recent` is saved; `None` keeps it in memory (tests).
    recent_path: Option<std::path::PathBuf>,
    /// Window title last sent to the viewport.
    title: String,
}

impl CutCraftApp {
    fn new() -> Self {
        let menus = gui::MenuSystem::new();
        Self {
            screen: Screen::Start,
            start_menus: menus.without_job(),
            doc: Document::default(),
            history: History::default(),
            registry: commands::registry(),
            icons: Icons::load(&Icons::default_dir()),
            menus,
            toolbar: gui::Toolbar::new(),
            status: gui::StatusBar::new(),
            design_tab: 0,
            view: View::TwoD,
            cursor: None,
            form: None,
            view_transform: None,
            view_area: None,
            camera: None,
            view_tool: None,
            pages: Pages::default(),
            session: Session::default(),
            selected_toolpath: None,
            post: None,
            tools: ToolDatabase::starter(),
            tools_path: None,
            tool_window: false,
            tool_selected: 0,
            simulation: None,
            simulation_coarse: None,
            orbit: view3d::Orbit::default(),
            recent: vec![],
            recent_path: None,
            title: String::new(),
        }
    }

    fn available(&self, id: &str) -> bool {
        if let Some((_, command, _)) = gui::command_action(id) {
            return self.registry.get(command).is_some();
        }
        match id {
            "undo" => self.history.can_undo(),
            "close" => self.screen != Screen::Start,
            "zoom_selected" => !self.doc.selection.is_empty(),
            id if id.starts_with("open_recent:") => true,
            "redo" => self.history.can_redo(),
            "recalculate_toolpath" | "delete_toolpath" | "preview_toolpath" => {
                self.selected_toolpath.is_some()
            }
            "recalculate_all_toolpaths" | "preview_all_toolpaths" => !self.doc.toolpaths.is_empty(),
            "recalculate_visible_toolpaths" => self.doc.toolpaths.values().any(|t| t.visible),
            "reset_preview" => self.simulation.is_some(),
            "save_toolpaths" => {
                !self.doc.toolpaths.is_empty()
                    && (self.post.is_some() || self.registry.exporter("export_gcode").is_some())
            }
            _ => {
                BUILT_INS.contains(&id)
                    || self.registry.get(id).is_some()
                    || self.registry.exporter(id).is_some()
            }
        }
    }

    /// Menu items, tool buttons and shortcuts all end up here: built-in actions first, then
    /// the command registry.
    fn invoke(&mut self, ctx: &egui::Context, id: &str) {
        // right-click menu items: a command with params chosen by the menu
        if let Some((form, command, params)) = gui::command_action(id) {
            if form {
                self.start_command(command);
                if let Some(open) = self.form.as_mut().filter(|f| f.id == command) {
                    open.params = params;
                }
            } else if let Err(e) = self.apply(command, &params) {
                self.status.set_message(&e);
            }
            return;
        }
        match id {
            "undo" => {
                if self.history.undo(&mut self.doc) {
                    self.status.set_message("Undone");
                }
            }
            "redo" => {
                if self.history.redo(&mut self.doc) {
                    self.status.set_message("Redone");
                }
            }
            // closing goes through the close request, which checks for unsaved work
            "exit" => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            "new"
            | "open"
            | "close"
            | "save"
            | "save_as"
            | "import_vectors_bitmap"
            | "install_post_processor" => self.file_action(id),
            "tool_database" => self.tool_window = true,
            // the Select tool: put away whatever tool is in hand
            "select" => {
                (self.form, self.view_tool) = (None, None);
                "Select".clone_into(&mut self.status.current_tool);
            }
            // menu items that are an existing tool reached another way
            "select_all_vectors_on_current_layer" => {
                let layer = serde_json::json!({ "layer": self.doc.current_layer });
                if let Err(e) = self.apply("select_layer_vectors", &layer) {
                    self.status.set_message(&e);
                }
            }
            "job_size_and_position" => self.start_command("job_setup"),
            "show_toolpaths_tab" => self.pages.open(Page::Toolpaths),
            "design_layout" => self.pages = Pages::design_layout(),
            "machining_layout" => self.pages = Pages::machining_layout(),
            "interface_layout" => self.pages.switch_layout(),
            "pan" | "zoom_box" | "zoom_to_job" | "zoom_to_drawing" | "zoom_selected" => {
                self.view_action(ctx, id);
            }
            "recalculate_toolpath"
            | "recalculate_visible_toolpaths"
            | "recalculate_all_toolpaths"
            | "delete_toolpath"
            | "save_toolpaths" => {
                self.toolpath_action(id);
            }
            "preview_toolpath" => {
                if let Some(id) = self.selected_toolpath {
                    self.preview_toolpath(id);
                }
            }
            // the cascade: start again from uncut material, then preview each toolpath
            "preview_all_toolpaths" => {
                self.simulation = None;
                let visible = self.doc.toolpaths.iter().filter(|(_, t)| t.visible);
                for id in visible.map(|(&id, _)| id).collect::<Vec<_>>() {
                    self.preview_toolpath(id);
                }
            }
            "reset_preview" => (self.simulation, self.simulation_coarse) = (None, None),
            id if id.starts_with("open_recent:") => {
                if let Ok(index) = id["open_recent:".len()..].parse() {
                    self.open_recent(index);
                }
            }
            id if self.registry.exporter(id).is_some() => self.export(id),
            _ => self.start_command(id),
        }
    }

    fn file_action(&mut self, id: &str) {
        match id {
            // a new job starts in Job Setup, as Aspire's does
            "new" => {
                if self.confirm_discard() {
                    self.reset_job();
                    self.start_command("job_setup");
                    if let Some(form) = &mut self.form {
                        form.new_job = true;
                    }
                    self.screen = Screen::NewJob;
                }
            }
            "open" => {
                if self.confirm_discard()
                    && let Some(path) = job_dialog().pick_file()
                {
                    self.open_path(&path);
                }
            }
            "close" => {
                if self.confirm_discard() {
                    self.reset_job();
                    self.screen = Screen::Start;
                }
            }
            "save" => match self.session.path.clone() {
                Some(path) => self.save_to(&path),
                None => self.file_action("save_as"),
            },
            "save_as" => {
                if let Some(path) = job_dialog()
                    .set_file_name(format!("untitled.{JOB_EXTENSION}"))
                    .save_file()
                {
                    self.save_to(&path.with_extension(JOB_EXTENSION));
                }
            }
            "import_vectors_bitmap" => {
                let mut dialog = rfd::FileDialog::new();
                for importer in self.registry.importers() {
                    dialog = dialog.add_filter(importer.format.label, importer.format.extensions);
                }
                if let Some(path) = dialog.pick_file() {
                    let change = files::import(&self.registry, &self.doc, &path)
                        .map_err(cut_craft_core::CommandError::Failed);
                    self.record(change, &format!("Imported {}", path.display()));
                }
            }
            "install_post_processor" => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Post-processor", &["pp"])
                    .pick_file()
                {
                    match files::load_post(&path) {
                        Ok(post) => {
                            self.status
                                .set_message(&format!("Post-processor: {}", post.name));
                            self.post = Some(post);
                        }
                        Err(e) => self.status.set_message(&e),
                    }
                }
            }
            _ => unreachable!("not a file action: {id}"),
        }
    }

    fn toolpath_action(&mut self, id: &str) {
        match id {
            "recalculate_toolpath" => {
                if let Some(id) = self.selected_toolpath {
                    let result = self.registry.recalculate(&self.doc, id);
                    self.record(result, "Toolpath recalculated");
                }
            }
            // the cascade: Recalculate on each shown toolpath
            "recalculate_visible_toolpaths" => {
                let shown = self.doc.toolpaths.iter().filter(|(_, t)| t.visible);
                let ids: Vec<EntityId> = shown.map(|(&id, _)| id).collect();
                let result =
                    Change::chain_over(&self.doc, &ids, |d, &id| self.registry.recalculate(d, id));
                self.record(result, "Visible toolpaths recalculated");
            }
            "recalculate_all_toolpaths" => {
                let result = self.registry.recalculate_all(&self.doc);
                self.record(result, "All toolpaths recalculated");
            }
            // the toolpath list's button: the Delete Toolpath command on the selected one
            "delete_toolpath" => {
                if let Some(id) = self.selected_toolpath.take()
                    && let Err(e) =
                        self.apply("delete_toolpath", &serde_json::json!({ "toolpath": id }))
                {
                    self.status.set_message(&e);
                }
            }
            "save_toolpaths" => match &self.post {
                Some(post) => {
                    let extension = post.extension.clone();
                    let dialog =
                        rfd::FileDialog::new().add_filter(&post.name, &[extension.as_str()]);
                    if let Some(path) = dialog.save_file() {
                        let path = path.with_extension(&extension);
                        let job = self.session.path.as_deref();
                        let result = files::save_through(post, &self.doc, &path, job);
                        self.status.set_message(&result.map_or_else(
                            |e| e,
                            |()| format!("Saved toolpaths to {}", path.display()),
                        ));
                    }
                }
                None => self.export("export_gcode"),
            },
            _ => unreachable!("not a toolpath action: {id}"),
        }
    }

    /// Runs a registered command: straight away if it has no settings, else opens its form.
    fn start_command(&mut self, id: &str) {
        let Some(cmd) = self.registry.get(id) else {
            self.status
                .set_message(&format!("{}: not implemented yet", gui::title_case(id)));
            return;
        };
        let page = self.page_of(cmd.id());
        let (meta, fields, params) = (
            cmd.meta(),
            form::fields(&cmd.schema()),
            cmd.initial_params(&self.doc),
        );
        meta.label.clone_into(&mut self.status.current_tool);
        self.view_tool = None;
        if fields.is_empty() {
            if let Err(e) = self.apply(cmd.id(), &params) {
                self.status.set_message(&e);
            }
        } else {
            self.form = Some(OpenForm {
                id: cmd.id(),
                label: meta.label,
                icon: meta.icon,
                fields,
                params,
                error: None,
                new_job: false,
                page,
            });
            // a form shows in its page, so bring an auto-hidden page out
            if self.pages.shown(page) == Shown::Hidden {
                self.pages.open(page);
            }
        }
    }

    fn save_to(&mut self, path: &std::path::Path) {
        match self.session.save(&self.doc, &self.history, path) {
            Ok(()) => {
                self.status
                    .set_message(&format!("Saved {}", path.display()));
                self.remember(path);
            }
            Err(e) => self.status.set_message(&e),
        }
    }

    /// Opens a job file in place of whatever is open (already confirmed); whether it opened.
    fn open_path(&mut self, path: &std::path::Path) -> bool {
        let mut session = Session::default();
        match session.open(path) {
            Ok((doc, history)) => {
                self.reset_job();
                (self.doc, self.history, self.session) = (doc, history, session);
                self.screen = Screen::Job;
                self.remember(path);
                true
            }
            Err(e) => {
                self.status.set_message(&e);
                false
            }
        }
    }

    /// Opens one of the recent files; one that no longer opens leaves the list.
    fn open_recent(&mut self, index: usize) {
        let Some(path) = self.recent.get(index).cloned() else {
            return;
        };
        if self.confirm_discard() && !self.open_path(&path) {
            self.recent.retain(|p| p != &path);
            self.save_recent();
        }
    }

    fn remember(&mut self, path: &std::path::Path) {
        files::remember(&mut self.recent, path);
        self.save_recent();
    }

    fn save_recent(&mut self) {
        if let Some(path) = &self.recent_path
            && let Err(e) = files::save_recent(&self.recent, path)
        {
            self.status.set_message(&e);
        }
    }

    fn load_recent(&mut self) {
        self.recent_path = files::recent_files_path();
        if let Some(path) = &self.recent_path {
            self.recent = files::load_recent(path);
        }
    }

    /// Back to nothing: no job, no form, no preview.
    fn reset_job(&mut self) {
        (self.doc, self.history, self.session, self.form) = Default::default();
        (self.camera, self.view_tool) = (None, None);
        (
            self.selected_toolpath,
            self.simulation,
            self.simulation_coarse,
        ) = (None, None, None);
    }

    /// OK in Job Setup for a new job: the job is made, with nothing to undo yet.
    fn job_setup_ok(&mut self) {
        let Some(mut form) = self.form.take() else {
            return;
        };
        let cmd = self
            .registry
            .get(form.id)
            .expect("only registered commands have forms");
        let made = cmd
            .run_json(&self.doc, &form.params)
            .map_err(|e| e.to_string())
            .and_then(|change| self.doc.apply(&change).map_err(|e| e.to_string()));
        match made {
            Ok(()) => {
                (self.history, self.session) = Default::default();
                self.screen = Screen::Job;
                self.status.set_message("New job created");
                "Select".clone_into(&mut self.status.current_tool);
            }
            Err(e) => {
                form.error = Some(e);
                self.form = Some(form);
            }
        }
    }

    fn job_setup_cancel(&mut self) {
        self.reset_job();
        "Select".clone_into(&mut self.status.current_tool);
        self.screen = Screen::Start;
    }

    fn export(&mut self, id: &str) {
        let Some(format) = self.registry.exporter(id).map(|e| e.format) else {
            return;
        };
        let dialog = rfd::FileDialog::new().add_filter(format.label, format.extensions);
        if let Some(path) = dialog.save_file() {
            let path = path.with_extension(format.extensions[0]);
            let result = files::export(&self.registry, id, &self.doc, &path);
            self.status.set_message(
                &result.map_or_else(|e| e, |()| format!("Exported {}", path.display())),
            );
        }
    }

    /// True when it is fine to throw the current job away: no unsaved work, or the user chose
    /// to save it or not to. False if they cancelled.
    fn confirm_discard(&mut self) -> bool {
        if !self.session.is_dirty(&self.history) {
            return true;
        }
        let answer = rfd::MessageDialog::new()
            .set_title(APP_NAME)
            .set_description(format!(
                "Save changes to {}?",
                self.session.title(APP_NAME, &self.history)
            ))
            .set_buttons(rfd::MessageButtons::YesNoCancel)
            .show();
        match answer {
            rfd::MessageDialogResult::Yes => {
                match self
                    .session
                    .path
                    .clone()
                    .or_else(|| job_dialog().save_file())
                {
                    Some(path) => self.save_to(&path.with_extension(JOB_EXTENSION)),
                    None => return false,
                }
                !self.session.is_dirty(&self.history)
            }
            rfd::MessageDialogResult::No => true,
            _ => false,
        }
    }

    /// Applies a computed change for undo, reporting how it went in the status bar.
    fn record(&mut self, change: Result<Change, cut_craft_core::CommandError>, done: &str) {
        let result = change.map_err(|e| e.to_string()).and_then(|c| {
            self.history
                .apply(&mut self.doc, c)
                .map_err(|e| e.to_string())
        });
        self.status
            .set_message(&result.map_or_else(|e| e, |()| done.to_owned()));
    }

    /// Runs a command and records it for undo.
    fn apply(&mut self, id: &str, params: &Value) -> Result<(), String> {
        let cmd = self
            .registry
            .get(id)
            .expect("only registered commands are applied");
        let change = cmd.run_json(&self.doc, params).map_err(|e| e.to_string())?;
        self.history
            .apply(&mut self.doc, change)
            .map_err(|e| e.to_string())?;
        self.status
            .set_message(&format!("{} applied", cmd.meta().label));
        Ok(())
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        // single-letter shortcuts must not fire while typing in a form field
        if !ctx.wants_keyboard_input() {
            let job = self.screen == Screen::Job;
            let fired: Vec<String> = self
                .bindings()
                .into_iter()
                .filter(|(id, _)| job || ["new", "open"].contains(&id.as_str()))
                .filter(|(_, keys)| ctx.input_mut(|i| i.consume_shortcut(keys)))
                .map(|(id, _)| id)
                .collect();
            for id in fired {
                self.invoke(ctx, &id);
            }
        }
        if ctx
            .input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::Escape)))
        {
            if self.screen == Screen::NewJob {
                self.job_setup_cancel();
            }
            self.form = None;
            self.view_tool = None;
            self.pages.done_with(Page::Design);
            self.pages.done_with(Page::Toolpaths);
        }
        ctx.input(|i| {
            if i.key_pressed(Key::F2) {
                self.view = View::TwoD;
            }
            if i.key_pressed(Key::F3) {
                self.view = View::ThreeD;
            }
        });
    }

    /// Every key binding: each command's own shortcut, plus the built-ins as the menus spell them.
    fn bindings(&self) -> Vec<(String, KeyboardShortcut)> {
        let commands = self
            .registry
            .iter()
            .filter_map(|c| Some((c.id().to_owned(), shortcut::parse(c.meta().shortcut)?)));
        let built_ins = self
            .menus
            .items()
            .into_iter()
            .filter(|i| BUILT_INS.contains(&i.action.as_str()))
            .filter_map(|i| Some((i.action.clone(), shortcut::parse(&i.shortcut)?)));
        let layouts = LAYOUT_KEYS
            .iter()
            .filter_map(|&(id, keys)| Some((id.to_owned(), shortcut::parse(keys)?)));
        commands.chain(built_ins).chain(layouts).collect()
    }

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        let mut clicked = None;
        let recent: Vec<gui::MenuItem> = self
            .recent
            .iter()
            .enumerate()
            .map(|(i, path)| {
                gui::MenuItem::with_action(&file_name(path), &format!("open_recent:{i}"))
            })
            .collect();
        egui::menu::bar(ui, |ui| {
            for menu in &self.current_menus().menus {
                ui.menu_button(&menu.name, |ui| {
                    menu_items(
                        ui,
                        &menu.items,
                        &Menus {
                            available: &|id| self.available(id),
                            shortcut_of: &|id| self.registry.get(id).map(|c| c.meta().shortcut),
                            recent: &recent,
                        },
                        &mut clicked,
                    );
                });
            }
        });
        if let Some(id) = clicked {
            self.invoke(ui.ctx(), &id);
        }
    }

    /// The menu bar with or without a job open.
    fn current_menus(&self) -> &gui::MenuSystem {
        if self.screen == Screen::Start {
            &self.start_menus
        } else {
            &self.menus
        }
    }

    #[cfg(test)]
    fn menu_names(&self) -> Vec<&str> {
        self.current_menus()
            .menus
            .iter()
            .map(|m| m.name.as_str())
            .collect()
    }

    /// No job open: Aspire's startup tasks and the jobs opened lately.
    fn start_page(&mut self, ui: &mut egui::Ui) {
        let edge = ui.spacing().interact_size.y;
        let mut clicked = None;
        ui.strong("Startup Tasks");
        for (id, label, icon) in STARTUP_TASKS {
            let button = match self.icons.find(icon, id) {
                Some(path) => egui::Button::image_and_text(
                    Icons::image(path).fit_to_exact_size(egui::vec2(edge, edge)),
                    label,
                ),
                None => egui::Button::new(label),
            }
            .frame(false);
            let response = ui
                .add_enabled(self.available(id), button)
                .on_disabled_hover_text(format!("{label}: not implemented yet"));
            if response.clicked() {
                clicked = Some(id.to_owned());
            }
        }
        ui.separator();
        ui.strong("Recently opened files");
        if self.recent.is_empty() {
            ui.weak("None yet");
        }
        let glyph = self.icons.find(RECENT_FILE_ICON, "");
        for (i, path) in self.recent.iter().enumerate() {
            let name = file_name(path);
            let button = match glyph {
                Some(glyph) => egui::Button::image_and_text(
                    Icons::image(glyph).fit_to_exact_size(egui::vec2(edge, edge)),
                    name,
                ),
                None => egui::Button::new(name),
            }
            .frame(false);
            if ui
                .add(button)
                .on_hover_text(path.display().to_string())
                .clicked()
            {
                clicked = Some(format!("open_recent:{i}"));
            }
        }
        if let Some(id) = clicked {
            self.invoke(ui.ctx(), &id);
        }
    }

    /// The middle of the window with no job open: the Cut Craft mark.
    fn splash(ui: &mut egui::Ui) {
        let logo = std::path::Path::new(LOGO);
        let space = ui.available_size();
        let edge = space.min_elem() * SPLASH_SHARE;
        ui.vertical_centered(|ui| {
            ui.add_space((space.y - edge) / 2.0);
            if logo.exists() {
                ui.add(Icons::image(logo).fit_to_exact_size(egui::vec2(edge, edge)));
            }
            ui.heading(APP_NAME);
        });
    }

    /// The Design page: the chosen design tab's tools, under any form opened from them.
    fn design_page(&mut self, ui: &mut egui::Ui) {
        self.page_header(ui, Page::Design, DESIGN_TABS[self.design_tab]);
        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.form.as_ref().is_some_and(|f| f.page == Page::Design) {
                self.command_form(ui);
            }
            match DESIGN_TABS[self.design_tab] {
                "Drawing" => {
                    let groups = self.toolbar.groups.clone();
                    self.tool_groups(ui, &groups, Page::Design);
                }
                "Modeling" => {
                    let groups = self.toolbar.modeling_groups.clone();
                    self.tool_groups(ui, &groups, Page::Design);
                    self.component_list(ui);
                }
                "Layers" => self.layers_panel(ui),
                tab => {
                    ui.weak(format!("{tab} tools not implemented yet"));
                }
            }
        });
    }

    /// A page's title with its push-pin and Aspire's Switch Layout button.
    fn page_header(&mut self, ui: &mut egui::Ui, page: Page, title: &str) {
        let row = ui.spacing().interact_size.y;
        let (mut pin, mut switch) = (false, false);
        ui.horizontal(|ui| {
            ui.strong(title);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let pinned = self.pages.pinned(page);
                pin = ui
                    .selectable_label(pinned, "📌")
                    .on_hover_text(if pinned {
                        "Unpin: hide this page to its side tab"
                    } else {
                        "Pin this page open"
                    })
                    .clicked();
                switch = ui
                    .add(self.icon_button("view/interface_layout.svg", "", "Switch Layout", row))
                    .on_hover_text("Switch Layout (F11 designing, F12 machining)")
                    .clicked();
            });
        });
        ui.separator();
        if pin {
            let pinned = self.pages.pinned(page);
            self.pages.set_pinned(page, !pinned);
        }
        if switch {
            self.pages.switch_layout();
        }
    }

    /// The side tabs and the docked pages around the view; flown-out pages over it.
    fn job_pages(&mut self, ctx: &egui::Context) {
        let style = ctx.style();
        let (row, margin) = (style.spacing.interact_size.y, style.spacing.item_spacing.y);
        let width = DESIGN_PANEL_WIDTHS * style.spacing.interact_size.x;
        let strip = || egui::Frame::side_top_panel(&style).inner_margin(margin);
        let left_tabs = egui::SidePanel::left("design_tabs")
            .resizable(false)
            .exact_width(row + 2.0 * margin)
            .frame(strip())
            .show(ctx, |ui| self.design_tabs(ui))
            .response
            .rect;
        let right_tabs = egui::SidePanel::right("toolpath_tabs")
            .resizable(false)
            .exact_width(row + 2.0 * margin)
            .frame(strip())
            .show(ctx, |ui| self.toolpath_tabs(ui))
            .response
            .rect;
        if self.pages.shown(Page::Design) == Shown::Docked {
            egui::SidePanel::left("design_panel")
                .default_width(width)
                .show(ctx, |ui| self.design_page(ui));
        }
        if self.pages.shown(Page::Toolpaths) == Shown::Docked {
            egui::SidePanel::right("toolpaths_panel")
                .default_width(width)
                .show(ctx, |ui| self.toolpaths_page(ui));
        }
        let room = ctx.available_rect();
        // a click on the view puts a flown-out page away, unless its form is open
        let pressed = ctx.input(|i| {
            i.pointer
                .primary_pressed()
                .then(|| i.pointer.interact_pos())
                .flatten()
        });
        for (page, tabs) in [(Page::Design, left_tabs), (Page::Toolpaths, right_tabs)] {
            if self.pages.shown(page) != Shown::Flyout {
                continue;
            }
            let out = self.flyout(ctx, page, room);
            let form_here = self.form.as_ref().is_some_and(|f| f.page == page);
            let on_view = |at: egui::Pos2| {
                room.contains(at)
                    && ctx.layer_id_at(at).map(|l| l.order) != Some(egui::Order::Foreground)
            };
            if let Some(at) = pressed
                && on_view(at)
                && !out.contains(at)
                && !tabs.contains(at)
                && !form_here
            {
                self.pages.done_with(page);
            }
        }
    }

    /// The tool page a command's form shows in.
    fn page_of(&self, id: &str) -> Page {
        let toolpaths = self
            .toolbar
            .toolpath_groups
            .iter()
            .any(|g| g.tools.contains(&id));
        if toolpaths {
            Page::Toolpaths
        } else {
            Page::Design
        }
    }

    /// A tool picked from a page: a page that flew out for it goes away again, unless the
    /// tool opened a form there.
    fn pick(&mut self, ctx: &egui::Context, page: Page, id: &str) {
        self.invoke(ctx, id);
        if self.form.is_none() {
            self.pages.done_with(page);
        }
    }

    /// Aspire's design tabs, down the left edge.
    fn design_tabs(&mut self, ui: &mut egui::Ui) {
        let shown = self.pages.shown(Page::Design) != Shown::Hidden;
        let (hovered, clicked) = side_tabs(ui, &DESIGN_TABS, shown.then_some(self.design_tab));
        // an auto-hidden page comes out on hover; a docked one changes tab on click
        let unpinned = !self.pages.pinned(Page::Design);
        if let Some(tab) = clicked.or(hovered.filter(|_| unpinned)) {
            self.design_tab = tab;
            self.pages.open(Page::Design);
        }
    }

    /// The Toolpaths tab, down the right edge.
    fn toolpath_tabs(&mut self, ui: &mut egui::Ui) {
        let shown = self.pages.shown(Page::Toolpaths) != Shown::Hidden;
        let (hovered, clicked) = side_tabs(ui, &["Toolpaths"], shown.then_some(0));
        let unpinned = !self.pages.pinned(Page::Toolpaths);
        if clicked.or(hovered.filter(|_| unpinned)).is_some() {
            self.pages.open(Page::Toolpaths);
        }
    }

    /// An auto-hidden page flown out over the view from its side; returns where it is.
    fn flyout(&mut self, ctx: &egui::Context, page: Page, room: egui::Rect) -> egui::Rect {
        let width = DESIGN_PANEL_WIDTHS * ctx.style().spacing.interact_size.x;
        let frame = egui::Frame::side_top_panel(&ctx.style());
        let height = room.height() - frame.inner_margin.sum().y - frame.outer_margin.sum().y;
        let left = match page {
            Page::Design => room.left(),
            Page::Toolpaths => room.right() - width - frame.inner_margin.sum().x,
        };
        egui::Area::new(egui::Id::new(("flyout", page)))
            .fixed_pos(egui::pos2(left, room.top()))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                frame.show(ui, |ui| {
                    ui.set_width(width);
                    ui.set_height(height);
                    match page {
                        Page::Design => self.design_page(ui),
                        Page::Toolpaths => self.toolpaths_page(ui),
                    }
                });
            })
            .response
            .rect
    }

    /// The open form's buttons: OK and Cancel for a new job's setup, else Apply and Close.
    fn form_buttons(&mut self, ui: &mut egui::Ui) {
        let Some(open) = &self.form else { return };
        let new_job = open.new_job;
        let (apply_label, close_label) = if new_job {
            ("OK", "Cancel")
        } else {
            ("Apply", "Close")
        };
        let (mut apply, mut close) = (false, false);
        ui.horizontal(|ui| {
            apply = ui.button(apply_label).clicked();
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                close = ui.button(close_label).clicked();
            });
        });
        match (new_job, apply, close) {
            (true, true, _) => self.job_setup_ok(),
            (true, _, true) => self.job_setup_cancel(),
            (false, true, _) => {
                let (id, params) = (open.id, open.params.clone());
                let result = self.apply(id, &params);
                if let Some(open) = &mut self.form {
                    open.error = result.err();
                }
            }
            (false, _, true) => {
                let page = open.page;
                self.form = None;
                self.pages.done_with(page);
            }
            _ => {}
        }
    }

    fn command_form(&mut self, ui: &mut egui::Ui) {
        let Some(open) = &mut self.form else { return };
        ui.horizontal(|ui| {
            if let Some(path) = self.icons.find(open.icon, open.id) {
                let edge = ui.spacing().interact_size.y;
                ui.add(Icons::image(path).fit_to_exact_size(egui::vec2(edge, edge)));
            }
            ui.strong(open.label);
        });
        let (mut chosen, mut save_tool) = (None, false);
        if open.params.get("tool").is_some_and(Value::is_object) {
            let current = open.params["tool"]["name"].as_str().unwrap_or_default();
            ui.horizontal(|ui| {
                egui::ComboBox::from_label("Tool database")
                    .selected_text(current)
                    .show_ui(ui, |ui| {
                        for tool in &self.tools.tools {
                            if ui
                                .selectable_label(tool.name == current, &tool.name)
                                .clicked()
                            {
                                chosen = Some(tool.name.clone());
                            }
                        }
                    });
                save_tool = ui.button("Save to database").clicked();
            });
        }
        // a form that sets the units (Job Setup) shows its lengths in them as they change
        let edge = SECTION_PICTURE_ROWS * ui.spacing().interact_size.y;
        let picture = |path: &str| {
            self.icons
                .find(path, "")
                .map(|p| Icons::image(p).fit_to_exact_size(egui::vec2(edge, edge)))
        };
        let look = form::Look {
            units: form::units_chosen(&open.params).unwrap_or(self.doc.job.units),
            picture: &picture,
        };
        form::show(ui, &open.fields, &mut open.params, &look);
        if let Some(error) = &open.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        // Job Setup for a new job keeps its buttons pinned under the panel instead
        if !open.new_job {
            self.form_buttons(ui);
            ui.separator();
        }
        if let Some(name) = chosen {
            self.choose_tool(&name);
        }
        if save_tool {
            self.save_form_tool();
        }
    }

    /// Reads the user's tool database; a broken one is reported and left untouched on disk.
    fn load_tools(&mut self) {
        self.tools_path = files::tool_database_path();
        if let Some(path) = &self.tools_path {
            match files::load_tools(path) {
                Ok(tools) => self.tools = tools,
                Err(e) => {
                    self.status
                        .set_message(&format!("{e}; tool changes will not be saved"));
                    self.tools_path = None;
                }
            }
        }
    }

    /// Copies a database tool into the open form.
    fn choose_tool(&mut self, name: &str) {
        if let (Some(form), Some(tool)) = (&mut self.form, self.tools.get(name)) {
            form.params["tool"] = serde_json::to_value(tool).expect("tools serialize");
        }
    }

    /// Stores the open form's tool in the database, replacing one of the same name.
    fn save_form_tool(&mut self) {
        let tool = self
            .form
            .as_ref()
            .and_then(|f| serde_json::from_value::<Tool>(f.params["tool"].clone()).ok());
        match tool {
            Some(tool) => {
                let done = format!("Saved {} to the tool database", tool.name);
                self.tools.put(tool);
                self.tools_changed(&done);
            }
            None => self
                .status
                .set_message("the form has no valid tool to save"),
        }
    }

    fn delete_tool(&mut self, index: usize) {
        if index < self.tools.tools.len() {
            let tool = self.tools.tools.remove(index);
            self.tools_changed(&format!("Deleted {} from the tool database", tool.name));
        }
    }

    /// Writes the tool database, if it has a file, and reports `done` or the failure.
    fn tools_changed(&mut self, done: &str) {
        let result = self
            .tools_path
            .as_deref()
            .map_or(Ok(()), |path| files::save_tools(&self.tools, path));
        self.status
            .set_message(&result.map_or_else(|e| e, |()| done.to_owned()));
    }

    /// Pick a tool to edit or delete; edits are saved as they are made.
    fn tool_database_window(&mut self, ctx: &egui::Context) {
        let (mut open, mut delete, mut edited) = (self.tool_window, None, false);
        egui::Window::new("Tool Database")
            .open(&mut open)
            .show(ctx, |ui| {
                let Some(last) = self.tools.tools.len().checked_sub(1) else {
                    ui.weak("No tools: save one from a toolpath form");
                    return;
                };
                self.tool_selected = self.tool_selected.min(last);
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_source("tool_database_tools")
                        .selected_text(&self.tools.tools[self.tool_selected].name)
                        .show_ui(ui, |ui| {
                            for (i, tool) in self.tools.tools.iter().enumerate() {
                                ui.selectable_value(&mut self.tool_selected, i, &tool.name);
                            }
                        });
                    if ui.button("Delete").clicked() {
                        delete = Some(self.tool_selected);
                    }
                });
                let fields = form::fields(
                    &serde_json::to_value(schemars::schema_for!(Tool)).expect("schemas serialize"),
                );
                let tool = &mut self.tools.tools[self.tool_selected];
                let mut value = serde_json::to_value(&*tool).expect("tools serialize");
                let look = form::Look {
                    units: self.doc.job.units,
                    picture: &|_| None,
                };
                if form::show(ui, &fields, &mut value, &look)
                    && let Ok(changed) = serde_json::from_value(value)
                {
                    *tool = changed;
                    edited = true;
                }
            });
        self.tool_window = open;
        if let Some(index) = delete {
            self.delete_tool(index);
        } else if edited {
            self.tools_changed("Tool database saved");
        }
    }

    /// Icon buttons for groups of command ids: implemented ones run, the rest are greyed out.
    fn tool_groups(&mut self, ui: &mut egui::Ui, groups: &[gui::ToolGroup], page: Page) {
        let edge = TOOL_ICON_ROWS * ui.spacing().interact_size.y;
        let mut clicked = None;
        // plain sections, as Aspire's tool pages have: a heading over its tools
        for group in groups {
            ui.strong(group.name);
            ui.horizontal_wrapped(|ui| {
                for &id in group.tools {
                    let cmd = self.registry.get(id);
                    let label =
                        cmd.map_or_else(|| gui::title_case(id), |c| c.meta().label.to_owned());
                    let explicit = cmd.map_or("", |c| c.meta().icon);
                    let button = self
                        .icon_button(explicit, id, &label, edge)
                        .selected(self.status.current_tool == label);
                    let response = ui
                        .add_enabled(self.available(id), button)
                        .on_hover_text(&label)
                        .on_disabled_hover_text(format!("{label}: not implemented yet"));
                    if response.clicked() {
                        clicked = Some(id);
                    }
                }
            });
            ui.add_space(ui.spacing().item_spacing.y);
        }
        if let Some(id) = clicked {
            self.pick(ui.ctx(), page, id);
        }
    }

    /// An icon button, or a text one when the icon is missing.
    fn icon_button(&self, icon: &str, id: &str, label: &str, edge: f32) -> egui::Button<'static> {
        match self.icons.find(icon, id) {
            Some(path) => {
                egui::Button::image(Icons::image(path).fit_to_exact_size(egui::vec2(edge, edge)))
            }
            None => egui::Button::new(label.to_owned()),
        }
    }

    /// Layers, top first: visibility and lock toggles, the current layer highlighted, and
    /// Aspire's layer menu on a right-click.
    fn layers_panel(&mut self, ui: &mut egui::Ui) {
        let edge = ui.spacing().interact_size.y;
        let label = |id: &str| self.registry.get(id).map_or("", |c| c.meta().label);
        let mut clicked: Option<(LayerId, &str)> = None;
        let mut chosen = None;
        for layer in self.doc.layers.iter().rev() {
            ui.horizontal(|ui| {
                let (eye, eye_tip) = if layer.visible {
                    ("layers/visible.svg", "Hide")
                } else {
                    ("layers/hidden.svg", "Show")
                };
                let (lock, lock_tip) = if layer.locked {
                    ("layers/locked.svg", "Unlock")
                } else {
                    ("layers/unlocked.svg", "Lock")
                };
                if ui
                    .add(self.icon_button(eye, "", eye_tip, edge))
                    .on_hover_text(eye_tip)
                    .clicked()
                {
                    clicked = Some((layer.id, "toggle_visible"));
                }
                if ui
                    .add(self.icon_button(lock, "", lock_tip, edge))
                    .on_hover_text(lock_tip)
                    .clicked()
                {
                    clicked = Some((layer.id, "toggle_locked"));
                }
                let count = self.doc.vectors_on(layer.id).count();
                let row = ui
                    .selectable_label(self.doc.current_layer == layer.id, &layer.name)
                    .on_hover_text(format!("{count} vectors"));
                if row.clicked() {
                    clicked = Some((layer.id, "set_current_layer"));
                }
                if row.double_clicked() {
                    clicked = Some((layer.id, "layer_properties"));
                }
                if let Some(id) = self.popup(&row, || gui::layer_menu(&self.doc, layer)) {
                    chosen = Some(id);
                }
            });
        }
        let add = ui.button(label("add_layer")).clicked();
        if let Some((layer, action)) = clicked {
            self.layer_action(ui.ctx(), layer, action);
        }
        if add {
            self.invoke(ui.ctx(), "add_layer");
        }
        if let Some(id) = chosen {
            self.invoke(ui.ctx(), &id);
        }
    }

    /// A right-click menu on `response`, built when it opens: the action chosen, if any.
    fn popup(
        &self,
        response: &egui::Response,
        menu: impl FnOnce() -> Vec<gui::MenuItem>,
    ) -> Option<String> {
        let mut clicked = None;
        response.context_menu(|ui| {
            let menus = Menus {
                available: &|id| self.available(id),
                shortcut_of: &|id| self.registry.get(id).map(|c| c.meta().shortcut),
                recent: &[],
            };
            menu_items(ui, &menu(), &menus, &mut clicked);
        });
        clicked
    }

    /// A layer-panel action: the toggles are undoable; choosing a layer is view state like the
    /// selection, so it is not.
    fn layer_action(&mut self, ctx: &egui::Context, layer: LayerId, action: &str) {
        let Some(l) = self.doc.layer(layer).cloned() else {
            return;
        };
        let properties = |visible: bool, locked: bool| serde_json::json!({ "layer": l.id, "name": l.name, "visible": visible, "locked": locked });
        let result = match action {
            "toggle_visible" => self.apply("layer_properties", &properties(!l.visible, l.locked)),
            "toggle_locked" => self.apply("layer_properties", &properties(l.visible, !l.locked)),
            "set_current_layer" => self
                .registry
                .get(action)
                .expect("set_current_layer is registered")
                .run_json(&self.doc, &serde_json::json!({ "layer": layer }))
                .map_err(|e| e.to_string())
                .map(|change| {
                    self.doc
                        .apply(&change)
                        .expect("the panel lists existing layers");
                }),
            // the properties form starts from the current layer
            "layer_properties" => {
                self.layer_action(ctx, layer, "set_current_layer");
                self.invoke(ctx, action);
                Ok(())
            }
            _ => unreachable!("not a layer panel action: {action}"),
        };
        if let Err(e) = result {
            self.status.set_message(&e);
        }
    }

    /// Cuts the toolpath from the previewed material (uncut material if there is none yet).
    fn preview_toolpath(&mut self, id: EntityId) {
        let Some(toolpath) = self.doc.toolpaths.get(&id) else {
            return;
        };
        self.simulation
            .get_or_insert_with(|| Material::new(&self.doc.job, PREVIEW_CELLS))
            .cut(toolpath);
        self.simulation_coarse = None;
        self.view = View::ThreeD;
        self.status
            .set_message(&format!("Previewed {}", toolpath.name));
    }

    /// Components top first: the model is built from the bottom of the list up. Aspire's
    /// component menu on a right-click.
    fn component_list(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.strong("Components");
        if self.doc.components.is_empty() {
            ui.weak("No components yet");
        }
        let mut chosen = None;
        for (&id, component) in self.doc.components.iter().rev() {
            let text = format!("{} ({:?})", component.name, component.combine);
            let row = ui.add(egui::Label::new(text).sense(egui::Sense::click()));
            if let Some(action) = self.popup(&row, || gui::component_menu(id, component)) {
                chosen = Some(action);
            }
        }
        if let Some(action) = chosen {
            self.invoke(ui.ctx(), &action);
        }
    }

    /// What the 3D view shows: the toolpath preview if there is one, else the model.
    // ponytail: rebuilt on every call, measured at about 0.1 ms of a 7.5 ms frame; cache it on
    // the history revision if that grows
    fn surface_3d(&mut self) -> Option<Cow<'_, Material>> {
        if let Some(material) = &self.simulation {
            return Some(Cow::Borrowed(
                self.simulation_coarse
                    .get_or_insert_with(|| view3d::coarse(material, MESH_CELLS)),
            ));
        }
        (!self.doc.components.is_empty())
            .then(|| Cow::Owned(cut_craft_core::component::model(&self.doc, MESH_CELLS)))
    }

    /// The preview or model as a block, turned by dragging sideways and tilted up/down.
    fn view_3d(&mut self, ui: &mut egui::Ui) {
        if self.simulation.is_none() && self.doc.components.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.weak("Nothing in 3D yet: create a shape or preview the toolpaths")
            });
            return;
        }
        let (area, response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::drag());
        // dragging across the whole view turns half a turn
        let delta = response.drag_delta();
        self.orbit.yaw += f64::from(delta.x / area.width().max(1.0)) * PI;
        self.orbit.pitch = (self.orbit.pitch + f64::from(delta.y / area.height().max(1.0)) * PI)
            .clamp(0.0, FRAC_PI_2);
        let [r, g, b] = self.shown_job().colour;
        let (orbit, colour) = (self.orbit, egui::Color32::from_rgb(r, g, b));
        let fit = area.shrink(ui.spacing().item_spacing.x * 2.0);
        if let Some(surface) = self.surface_3d() {
            ui.painter_at(area)
                .add(view3d::mesh(&surface, orbit, colour, fit));
        }
    }

    /// The Toolpaths page: strategies, any form opened from them, and the toolpath list.
    fn toolpaths_page(&mut self, ui: &mut egui::Ui) {
        self.page_header(ui, Page::Toolpaths, "Toolpaths");
        let groups = self.toolbar.toolpath_groups.clone();
        egui::ScrollArea::vertical().show(ui, |ui| {
            if self
                .form
                .as_ref()
                .is_some_and(|f| f.page == Page::Toolpaths)
            {
                self.command_form(ui);
            }
            self.tool_groups(ui, &groups, Page::Toolpaths);
            ui.separator();
            ui.strong("Toolpath List");
            if self.doc.toolpaths.is_empty() {
                ui.weak("No toolpaths yet");
            }
            let mut chosen = None;
            for (&id, toolpath) in &self.doc.toolpaths {
                let selected = self.selected_toolpath == Some(id);
                let label = ui
                    .selectable_label(selected, &toolpath.name)
                    .on_hover_text(&toolpath.tool.name);
                if label.clicked() {
                    self.selected_toolpath = if selected { None } else { Some(id) };
                }
                // Aspire's toolpath menu acts on the row right-clicked, so it selects it
                if label.secondary_clicked() {
                    self.selected_toolpath = Some(id);
                }
                if let Some(action) = self.popup(&label, || gui::toolpath_menu(id, toolpath)) {
                    chosen = Some(action);
                }
            }
            if let Some(action) = chosen {
                self.invoke(ui.ctx(), &action);
            }
            let actions = [gui::ToolGroup {
                name: "Toolpath List Actions",
                tools: &[
                    "recalculate_toolpath",
                    "recalculate_all_toolpaths",
                    "delete_toolpath",
                    "preview_toolpath",
                    "preview_all_toolpaths",
                    "reset_preview",
                ],
            }];
            self.tool_groups(ui, &actions, Page::Toolpaths);
        });
    }

    fn status_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(&self.status.message);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if let Some(p) = self.cursor {
                    ui.monospace(format!("X {:8.2}  Y {:8.2} mm", p.x, p.y));
                    ui.separator();
                }
                ui.label(format!("Tool: {}", self.status.current_tool));
            });
        });
    }
}

fn job_dialog() -> rfd::FileDialog {
    rfd::FileDialog::new().add_filter("Cut Craft job", &[JOB_EXTENSION, OLD_JOB_EXTENSION])
}

/// What drawing the menus needs from the app.
struct Menus<'a> {
    available: &'a dyn Fn(&str) -> bool,
    shortcut_of: &'a dyn Fn(&str) -> Option<&'static str>,
    /// Filled in where the menus have a Recent Files entry.
    recent: &'a [gui::MenuItem],
}

/// Menu entries, recursing into submenus. Sets `clicked` to the chosen item's action.
fn menu_items(
    ui: &mut egui::Ui,
    items: &[gui::MenuItem],
    menus: &Menus,
    clicked: &mut Option<String>,
) {
    for item in items {
        if item.is_separator() {
            ui.separator();
        } else if item.action == "recent_files" {
            ui.add_enabled_ui(!menus.recent.is_empty(), |ui| {
                ui.menu_button(&item.name, |ui| {
                    menu_items(ui, menus.recent, menus, clicked);
                });
            });
        } else if !item.children.is_empty() {
            ui.menu_button(&item.name, |ui| {
                menu_items(ui, &item.children, menus, clicked);
            });
        } else {
            let button = egui::Button::new(&item.name)
                .shortcut_text((menus.shortcut_of)(&item.action).unwrap_or(&item.shortcut))
                .selected(item.is_checked);
            let response = ui
                .add_enabled(item.is_enabled && (menus.available)(&item.action), button)
                .on_disabled_hover_text("Not implemented yet");
            if response.clicked() {
                *clicked = Some(item.action.clone());
                ui.close_menu();
            }
        }
    }
}

/// Tabs down a side of the window, text reading downwards as Aspire's side tabs do; the tab
/// hovered and the tab clicked this frame.
fn side_tabs(
    ui: &mut egui::Ui,
    labels: &[&str],
    current: Option<usize>,
) -> (Option<usize>, Option<usize>) {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let pad = ui.spacing().button_padding;
    let (mut hovered, mut clicked) = (None, None);
    for (i, label) in labels.iter().enumerate() {
        let galley = ui.painter().layout_no_wrap(
            (*label).to_owned(),
            font.clone(),
            ui.visuals().text_color(),
        );
        let text = galley.size();
        let size = egui::vec2(ui.available_width(), text.x + 2.0 * pad.x);
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
        let look = ui
            .style()
            .interact_selectable(&response, current == Some(i));
        ui.painter()
            .rect(rect, look.rounding, look.weak_bg_fill, look.bg_stroke);
        // turned a quarter clockwise about its top-left corner, so it reads downwards
        let at = egui::pos2(rect.center().x + text.y / 2.0, rect.top() + pad.x);
        let mut shape = egui::epaint::TextShape::new(at, galley, look.text_color())
            .with_angle(std::f32::consts::FRAC_PI_2);
        shape.override_text_color = Some(look.text_color());
        ui.painter().add(shape);
        if response.hovered() {
            hovered = Some(i);
        }
        if response.clicked() {
            clicked = Some(i);
        }
    }
    (hovered, clicked)
}

/// A path as the menus and start page list it: just the file name.
fn file_name(path: &std::path::Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

impl eframe::App for CutCraftApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui(ctx);
    }
}

impl CutCraftApp {
    /// One frame of the whole window; separate from `update` so tests can drive it headlessly.
    fn ui(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested()) && !self.confirm_discard() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        let title = match self.screen {
            Screen::Start => APP_NAME.to_owned(),
            _ => self.session.title(APP_NAME, &self.history),
        };
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
        self.shortcuts(ctx);
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| self.menu_bar(ui));
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| self.status_bar(ui));
        self.tool_database_window(ctx);
        match self.screen {
            Screen::Start => {
                egui::SidePanel::left("start_page").show(ctx, |ui| self.start_page(ui));
                egui::CentralPanel::default().show(ctx, Self::splash);
                (self.view_transform, self.cursor) = (None, None);
                return;
            }
            Screen::NewJob => {
                egui::SidePanel::left("design_panel")
                    .default_width(DESIGN_PANEL_WIDTHS * ctx.style().spacing.interact_size.x)
                    .show(ctx, |ui| {
                        egui::TopBottomPanel::bottom("form_buttons")
                            .show_inside(ui, |ui| self.form_buttons(ui));
                        egui::ScrollArea::vertical().show(ui, |ui| self.command_form(ui));
                    });
            }
            Screen::Job => self.job_pages(ctx),
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.view, View::TwoD, "2D View (F2)");
                ui.selectable_value(&mut self.view, View::ThreeD, "3D View (F3)");
            });
            ui.separator();
            match self.view {
                View::TwoD => self.view_2d(ui),
                View::ThreeD => {
                    self.view_3d(ui);
                    self.cursor = None;
                }
            }
        });
    }

    /// The job the views show: while Job Setup is open, the one being set up.
    fn shown_job(&self) -> cut_craft_core::document::Job {
        self.form
            .as_ref()
            .filter(|f| f.id == "job_setup")
            .and_then(|f| self.registry.get(f.id)?.run_json(&self.doc, &f.params).ok())
            .and_then(|change| change.job().copied())
            .unwrap_or(self.doc.job)
    }

    /// Zoom and pan: Aspire's 2D view controls.
    fn view_action(&mut self, ctx: &egui::Context, id: &str) {
        let label = match id {
            "pan" => {
                self.view_tool = Some(ViewTool::Pan);
                "Pan"
            }
            "zoom_box" => {
                self.view_tool = Some(ViewTool::ZoomBox);
                "Zoom Box"
            }
            "zoom_selected" => {
                let margin = ctx.style().spacing.interact_size.y;
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

    fn view_2d(&mut self, ui: &mut egui::Ui) {
        let job = self.shown_job();
        let mut canvas = Canvas::new(ui, &job, self.camera);
        let area = canvas.area;
        // the wheel zooms about the cursor: a quarter of the view's height doubles the zoom
        if let Some(at) = canvas.hover() {
            let scroll = ui.input(|i| i.raw_scroll_delta.y);
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
        let gesture = canvas.gesture(ui);
        let overlay = match (gesture, self.view_tool, self.drawing_tool()) {
            (Some(Gesture::Dragging(drag)), Some(ViewTool::ZoomBox), _) => Overlay::Window {
                start: drag.start,
                end: drag.end,
                crossing: false,
            },
            (_, Some(ViewTool::Pan), _) => Overlay::None,
            (Some(Gesture::Dragging(drag)), _, Some(id)) => {
                Overlay::Preview(self.preview(id, drag))
            }
            (Some(Gesture::Dragging(drag)), _, None) => Overlay::Window {
                start: drag.start,
                end: drag.end,
                crossing: drag.end.x < drag.start.x,
            },
            _ => Overlay::None,
        };
        canvas.paint(ui, &self.doc, &overlay, self.selected_toolpath);
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
        canvas.rulers(ui, job.units);
        let selection_menu = || self.menus.selection_menu(&self.doc);
        if let Some(action) = self.popup(canvas.response(), selection_menu) {
            self.invoke(ui.ctx(), &action);
        }
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
            (Some(Gesture::Dropped { drag, .. }), None, Some(id)) => self.draw(id, drag),
            (Some(Gesture::Dropped { drag, shift }), None, None) => {
                let ids = self
                    .doc
                    .window(Rect::new(drag.start, drag.end), drag.end.x < drag.start.x);
                self.select(ids, shift);
            }
            (Some(Gesture::Click { at, shift }), None, None) => {
                let hit = self.doc.pick(at, canvas.pick_tolerance(ui));
                if shift {
                    let mut ids = self.doc.selection.clone();
                    if let Some(id) = hit
                        && !ids.remove(&id)
                    {
                        ids.insert(id);
                    }
                    self.select(ids, false);
                } else {
                    self.select(hit, false);
                }
            }
            _ => {}
        }
    }

    /// Aspire's zoom buttons over the top-left of the 2D view, inside the rulers.
    fn view_buttons(&mut self, ui: &mut egui::Ui, area: egui::Rect) {
        let row = ui.spacing().interact_size.y;
        let edge = TOOL_ICON_ROWS * row;
        let corner = area.min + egui::vec2(row, row) + ui.spacing().item_spacing;
        let room = egui::Rect::from_min_size(corner, area.max - corner);
        let mut clicked = None;
        ui.allocate_ui_at_rect(room, |ui| {
            ui.horizontal(|ui| {
                for (id, label, icon) in VIEW_BUTTONS {
                    let button = self
                        .icon_button(icon, id, label, edge)
                        .selected(id == "zoom_box" && self.view_tool == Some(ViewTool::ZoomBox));
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

    /// The open form's command, if it can be drawn with the mouse.
    fn drawing_tool(&self) -> Option<&'static str> {
        let form = self.form.as_ref()?;
        self.registry
            .get(form.id)
            .filter(|c| c.interactive())
            .map(cut_craft_core::DynCommand::id)
    }

    /// Paths `id` would create for `drag`, by running the command itself.
    fn preview(&self, id: &str, drag: Drag) -> Vec<Path> {
        let (Some(cmd), Some(form)) = (self.registry.get(id), &self.form) else {
            return vec![];
        };
        cmd.drag_params(drag, &form.params)
            .and_then(|params| cmd.run_json(&self.doc, &params))
            .map(|change| change.added().cloned().collect())
            .unwrap_or_default()
    }

    /// Finishes a drag with a drawing tool: the form shows the dragged values, then they apply.
    fn draw(&mut self, id: &'static str, drag: Drag) {
        let Some(cmd) = self.registry.get(id) else {
            return;
        };
        let Some(current) = self.form.as_ref().map(|f| f.params.clone()) else {
            return;
        };
        let result = cmd
            .drag_params(drag, &current)
            .map_err(|e| e.to_string())
            .and_then(|params| {
                if let Some(form) = &mut self.form {
                    form.params.clone_from(&params);
                }
                self.apply(id, &params)
            });
        if let Some(form) = &mut self.form {
            form.error = result.err();
        }
    }

    /// Selection is view state: applied directly, never recorded for undo.
    fn select(&mut self, ids: impl IntoIterator<Item = EntityId>, add: bool) {
        let mut c = Change::builder(&self.doc);
        if add {
            c.select(self.doc.selection.iter().copied().chain(ids));
        } else {
            c.select(ids);
        }
        self.doc
            .apply(&c.finish())
            .expect("picked ids come from the document");
    }
}

/// Opens the main window and runs until it is closed.
pub fn run(title: &str) {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(title)
            .with_app_id(APP_ID),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native(
        title,
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            if let Some(rs) = &cc.wgpu_render_state {
                let info = rs.adapter.get_info();
                log::info!(
                    "GPU: {} ({:?}, driver: {})",
                    info.name,
                    info.backend,
                    info.driver
                );
            }
            let mut app = CutCraftApp::new();
            app.load_tools();
            app.load_recent();
            Box::new(app)
        }),
    )
    .expect("event loop error");
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{Event, PointerButton, Pos2, RawInput, Rect, vec2};

    /// Drives the real UI headlessly, one frame per call, with synthetic pointer input.
    struct Harness {
        ctx: egui::Context,
        app: CutCraftApp,
        time: f64,
        /// What the last frame drew.
        shapes: Vec<egui::epaint::ClippedShape>,
    }

    impl Harness {
        /// The app as it starts: the start page, no job open.
        fn at_start() -> Self {
            let mut h = Self {
                ctx: egui::Context::default(),
                app: CutCraftApp::new(),
                time: 0.0,
                shapes: vec![],
            };
            h.frame(vec![]);
            h.frame(vec![]);
            h
        }

        /// A new job made the way a user makes one: New, then OK in Job Setup.
        fn new() -> Self {
            let mut h = Self::at_start();
            h.app.invoke(&h.ctx, "new");
            h.app.job_setup_ok();
            h.frame(vec![]);
            h.frame(vec![]);
            h
        }

        fn frame(&mut self, events: Vec<Event>) {
            self.frame_holding(Modifiers::NONE, events);
        }

        /// One frame with `modifiers` held, as the windowing layer reports them every frame.
        fn frame_holding(&mut self, modifiers: Modifiers, events: Vec<Event>) {
            self.time += 0.1;
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1280.0, 800.0))),
                time: Some(self.time),
                modifiers,
                events,
                ..Default::default()
            };
            self.shapes = self.ctx.run(input, |ctx| self.app.ui(ctx)).shapes;
        }

        /// The middle of `text` where the last frame drew it.
        fn text_at(&self, text: &str) -> Option<Pos2> {
            self.shapes.iter().find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(t) if t.galley.text() == text => {
                    Some(t.pos + t.galley.size() / 2.0)
                }
                _ => None,
            })
        }

        fn right_click(&mut self, at: Pos2) {
            self.frame(vec![Event::PointerMoved(at)]);
            for pressed in [true, false] {
                let button = Event::PointerButton {
                    pos: at,
                    button: PointerButton::Secondary,
                    pressed,
                    modifiers: Modifiers::NONE,
                };
                self.frame(vec![button]);
            }
            self.frame(vec![]);
        }

        /// Screen position of a job coordinate in the 2D view.
        fn at(&self, x: f64, y: f64) -> Pos2 {
            self.app
                .view_transform
                .expect("the 2D view has been drawn")
                .to_screen(Point::new(x, y))
        }

        fn press(&mut self, pos: Pos2, pressed: bool, modifiers: Modifiers) {
            let button = Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed,
                modifiers,
            };
            self.frame_holding(modifiers, vec![button]);
        }

        fn drag(&mut self, from: Pos2, to: Pos2) {
            self.frame(vec![Event::PointerMoved(from)]);
            self.press(from, true, Modifiers::NONE);
            self.frame(vec![Event::PointerMoved(from.lerp(to, 0.5))]);
            self.frame(vec![Event::PointerMoved(to)]);
            self.press(to, false, Modifiers::NONE);
            self.frame(vec![]);
        }

        /// A click with `modifiers` held for the whole gesture, as a held key would be.
        fn click(&mut self, at: Pos2, modifiers: Modifiers) {
            self.frame_holding(modifiers, vec![Event::PointerMoved(at)]);
            self.press(at, true, modifiers);
            self.press(at, false, modifiers);
            self.frame_holding(modifiers, vec![]);
        }

        /// Clicks the item at `path` (names, top down) of the right-click menu `menu` builds.
        fn choose(&mut self, menu: impl Fn(&CutCraftApp) -> Vec<gui::MenuItem>, path: &[&str]) {
            let items = menu(&self.app);
            let item = leaf(&items, path);
            assert!(
                item.is_enabled && self.app.available(&item.action),
                "{path:?} is greyed"
            );
            self.app.invoke(&self.ctx, &item.action);
        }

        /// Draws a rectangle through the command, then closes its form.
        fn rectangle(&mut self, x: f64, y: f64, width: f64, height: f64) {
            let params = serde_json::json!({ "x": x, "y": y, "width": width, "height": height, "corners": "square", "radius": 0.0 });
            self.app.apply("draw_rectangle", &params).unwrap();
            self.app.form = None;
            self.frame(vec![]);
        }
    }

    /// The item at `path` (names, top down) of a menu.
    fn leaf<'a>(menu: &'a [gui::MenuItem], path: &[&str]) -> &'a gui::MenuItem {
        let (name, rest) = path.split_first().expect("a path to an item");
        let item = menu
            .iter()
            .find(|i| i.name == *name)
            .unwrap_or_else(|| panic!("no {name} in the menu"));
        if rest.is_empty() {
            item
        } else {
            leaf(&item.children, rest)
        }
    }

    fn near(a: Point, b: Point) -> bool {
        // one screen pixel is under a millimetre at this window size
        a.distance(b) < 2.0
    }

    #[test]
    fn dragging_with_the_rectangle_tool_draws_that_rectangle() {
        let mut h = Harness::new();
        h.app.invoke(&h.ctx, "draw_rectangle");
        h.frame(vec![]);
        let (from, to) = (h.at(100.0, 100.0), h.at(300.0, 200.0));
        h.drag(from, to);
        assert_eq!(h.app.doc.vectors.len(), 1);
        let b = h.app.doc.vectors.values().next().unwrap().bounds();
        assert!(
            near(b.min, Point::new(100.0, 100.0)) && near(b.max, Point::new(300.0, 200.0)),
            "{b:?}"
        );
        assert!(h.app.history.can_undo());
        let width = h.app.form.as_ref().unwrap().params["width"]
            .as_f64()
            .unwrap();
        assert!(
            (width - 200.0).abs() < 2.0,
            "the form shows the dragged size, got {width}"
        );
    }

    #[test]
    fn clicking_selects_without_adding_undo_steps() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        h.click(h.at(500.0, 300.0), Modifiers::NONE);
        assert!(
            h.app.doc.selection.is_empty(),
            "clicking empty space clears the selection"
        );
        h.click(h.at(60.0, 35.0), Modifiers::NONE);
        assert_eq!(
            h.app.doc.selection.len(),
            1,
            "clicking the right edge selects it"
        );
        assert!(h.app.history.undo(&mut h.app.doc));
        assert!(
            h.app.doc.vectors.is_empty(),
            "the last undo step is the draw, not a click"
        );
    }

    #[test]
    fn shift_click_adds_and_removes() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        h.rectangle(100.0, 10.0, 50.0, 50.0);
        h.click(h.at(10.0, 35.0), Modifiers::NONE);
        h.click(h.at(150.0, 35.0), Modifiers::SHIFT);
        assert_eq!(h.app.doc.selection.len(), 2);
        h.click(h.at(10.0, 35.0), Modifiers::SHIFT);
        assert_eq!(h.app.doc.selection.len(), 1);
    }

    #[test]
    fn left_to_right_windows_and_right_to_left_crosses() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        h.rectangle(100.0, 10.0, 50.0, 50.0);
        h.drag(h.at(0.0, 0.0), h.at(120.0, 80.0));
        assert_eq!(
            h.app.doc.selection.len(),
            1,
            "only the first rectangle is fully inside"
        );
        h.drag(h.at(120.0, 80.0), h.at(0.0, 0.0));
        assert_eq!(
            h.app.doc.selection.len(),
            2,
            "crossing also takes the one it touches"
        );
    }

    #[test]
    fn forms_open_with_values_from_the_document() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 20.0);
        h.app.invoke(&h.ctx, "set_size");
        let params = &h.app.form.as_ref().unwrap().params;
        assert_eq!(
            (params["width"].as_f64(), params["height"].as_f64()),
            (Some(50.0), Some(20.0))
        );
    }

    fn cut_min_x(app: &CutCraftApp) -> f64 {
        cut_craft_core::testing::cut_bounds(app.doc.toolpaths.values().next().unwrap())
            .min
            .x
    }

    #[test]
    fn toolpaths_follow_their_vectors_when_recalculated() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        h.app.invoke(&h.ctx, "profile_toolpath");
        let params = h.app.form.as_ref().unwrap().params.clone();
        h.app.apply("profile_toolpath", &params).unwrap();
        let before = cut_min_x(&h.app);
        let shift =
            serde_json::json!({ "relative": true, "x": 100.0, "y": 0.0, "anchor": "bottom_left" });
        h.app.apply("move", &shift).unwrap();
        h.app.invoke(&h.ctx, "recalculate_all_toolpaths");
        assert!((cut_min_x(&h.app) - before - 100.0).abs() < 1e-6);
        h.app.invoke(&h.ctx, "undo");
        assert!(
            (cut_min_x(&h.app) - before).abs() < 1e-6,
            "recalculating is one undo step"
        );
    }

    #[test]
    fn hiding_a_layer_from_the_panel_is_undoable_and_stops_picking() {
        let mut h = Harness::new();
        h.app.design_tab = DESIGN_TABS.iter().position(|&t| t == "Layers").unwrap();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        h.app.layer_action(&h.ctx, LayerId(1), "toggle_visible");
        h.frame(vec![]);
        h.click(h.at(60.0, 35.0), Modifiers::NONE);
        assert!(h.app.doc.selection.is_empty());
        h.app.invoke(&h.ctx, "undo");
        assert!(h.app.doc.layers[0].visible);
    }

    #[test]
    fn choosing_a_layer_makes_it_current_without_an_undo_step() {
        let mut h = Harness::new();
        h.app
            .apply("add_layer", &serde_json::json!({ "name": "Top" }))
            .unwrap();
        h.app.layer_action(&h.ctx, LayerId(1), "set_current_layer");
        assert_eq!(h.app.doc.current_layer, LayerId(1));
        h.app.invoke(&h.ctx, "undo");
        assert_eq!(
            h.app.doc.layers.len(),
            1,
            "the last undo step added the layer"
        );
    }

    #[test]
    fn the_layer_menu_acts_on_the_layer_clicked() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        h.app
            .apply("add_layer", &serde_json::json!({ "name": "Top" }))
            .unwrap();
        let layer =
            |i: usize| move |app: &CutCraftApp| gui::layer_menu(&app.doc, &app.doc.layers[i]);
        h.choose(layer(1), &["Move Selection Here"]);
        assert!(
            h.app
                .doc
                .vectors
                .keys()
                .all(|&id| h.app.doc.layer_of(id) == LayerId(2))
        );
        h.choose(layer(0), &["Activate"]);
        assert_eq!(h.app.doc.current_layer, LayerId(1));
        h.choose(layer(0), &["Show", "Only This"]);
        let visible: Vec<bool> = h.app.doc.layers.iter().map(|l| l.visible).collect();
        assert_eq!(visible, [true, false]);
        h.choose(layer(0), &["Lock"]);
        let locked = layer(0)(&h.app);
        assert!(!leaf(&locked, &["Lock"]).is_enabled && leaf(&locked, &["Unlock"]).is_enabled);
        h.choose(layer(1), &["Rename"]);
        let form = h.app.form.take().unwrap();
        assert_eq!(
            (form.id, form.params["name"].as_str()),
            ("layer_properties", Some("Top"))
        );
        h.choose(layer(0), &["Delete", "Empty"]);
        let names: Vec<&str> = h.app.doc.layers.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(names, ["Top"], "the first layer had nothing left on it");
    }

    #[test]
    fn the_toolpath_menu_acts_on_the_toolpath_clicked() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        for _ in 0..2 {
            let params = h.app.registry.get("profile_toolpath").unwrap();
            let params = params.initial_params(&h.app.doc);
            h.app.apply("profile_toolpath", &params).unwrap();
        }
        let toolpath = |i: usize| {
            move |app: &CutCraftApp| {
                let (&id, t) = app.doc.toolpaths.iter().nth(i).unwrap();
                gui::toolpath_menu(id, t)
            }
        };
        let min_x = |app: &CutCraftApp| -> Vec<f64> {
            let bounds = app.doc.toolpaths.values();
            bounds
                .map(|t| cut_craft_core::testing::cut_bounds(t).min.x)
                .collect()
        };
        h.choose(toolpath(0), &["Hide", "This"]);
        let visible: Vec<bool> = h.app.doc.toolpaths.values().map(|t| t.visible).collect();
        assert_eq!(visible, [false, true]);
        let before = min_x(&h.app);
        let shift =
            serde_json::json!({ "relative": true, "x": 100.0, "y": 0.0, "anchor": "bottom_left" });
        h.app.apply("move", &shift).unwrap();
        h.choose(toolpath(1), &["Recalculate", "Visible"]);
        let after = min_x(&h.app);
        assert!(
            (after[0] - before[0]).abs() < 1e-6 && (after[1] - before[1] - 100.0).abs() < 1e-6,
            "only the shown toolpath follows its vectors: {before:?} → {after:?}"
        );
        let second = *h.app.doc.toolpaths.keys().nth(1).unwrap();
        h.choose(toolpath(1), &["Rename"]);
        let form = h.app.form.take().unwrap();
        assert_eq!(form.id, "toolpath_properties");
        assert_eq!(form.params["toolpath"], serde_json::json!(second));
        h.choose(toolpath(1), &["Delete", "Invisible"]);
        assert_eq!(
            h.app.doc.toolpaths.keys().copied().collect::<Vec<_>>(),
            [second]
        );
    }

    #[test]
    fn the_component_menu_acts_on_the_component_clicked() {
        use cut_craft_core::component::Combine;
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        let params = h.app.registry.get("create_shape").unwrap();
        let params = params.initial_params(&h.app.doc);
        h.app.apply("create_shape", &params).unwrap();
        let menu = |app: &CutCraftApp| {
            let (&id, component) = app.doc.components.iter().next().unwrap();
            gui::component_menu(id, component)
        };
        let component = |app: &CutCraftApp| app.doc.components.values().next().unwrap().clone();
        assert!(leaf(&menu(&h.app), &["Combine Mode", "Add"]).is_checked);
        h.choose(menu, &["Combine Mode", "Subtract"]);
        assert_eq!(component(&h.app).combine, Combine::Subtract);
        assert!(leaf(&menu(&h.app), &["Combine Mode", "Subtract"]).is_checked);
        h.choose(menu, &["Hide", "This"]);
        assert!(!component(&h.app).visible);
        h.choose(menu, &["Properties"]);
        let form = h.app.form.take().unwrap();
        assert_eq!(
            (form.id, form.params["combine"].as_str()),
            ("component_properties", Some("subtract"))
        );
        h.choose(menu, &["Delete"]);
        assert!(h.app.doc.components.is_empty());
    }

    #[test]
    fn the_selection_menu_runs_edit_tools_on_the_selection() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 40.0, 40.0);
        h.rectangle(30.0, 30.0, 40.0, 40.0);
        h.app
            .apply("add_layer", &serde_json::json!({ "name": "Top" }))
            .unwrap();
        h.app.invoke(&h.ctx, "select_all_vectors");
        let menu = |app: &CutCraftApp| app.menus.selection_menu(&app.doc);
        h.choose(menu, &["Move to Layer", "Top"]);
        let doc = &h.app.doc;
        assert!(doc.vectors.keys().all(|&id| doc.layer_of(id) == LayerId(2)));
        h.choose(menu, &["Weld"]);
        assert_eq!(h.app.doc.vectors.len(), 1);
        h.choose(menu, &["Undo"]);
        assert_eq!(h.app.doc.vectors.len(), 2);
        h.choose(menu, &["Align", "Left"]);
        let left: Vec<f64> = h
            .app
            .doc
            .vectors
            .values()
            .map(|v| v.bounds().min.x)
            .collect();
        assert!((left[0] - left[1]).abs() < 1e-9, "{left:?}");
        let cut = leaf(&menu(&h.app), &["Cut"]).action.clone();
        assert!(!h.app.available(&cut), "Cut is greyed until it is built");
    }

    #[test]
    fn right_clicking_the_view_opens_the_selection_menu() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 40.0, 40.0);
        h.rectangle(30.0, 30.0, 40.0, 40.0);
        h.app.invoke(&h.ctx, "select_all_vectors");
        h.right_click(h.at(90.0, 90.0));
        let weld = h.text_at("Weld").expect("the selection menu is open");
        h.click(weld, Modifiers::NONE);
        assert_eq!(h.app.doc.vectors.len(), 1, "{}", h.app.status.message);
    }

    #[test]
    fn right_clicking_a_toolpath_selects_it_and_opens_its_menu() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        let params = h.app.registry.get("profile_toolpath").unwrap();
        let params = params.initial_params(&h.app.doc);
        h.app.apply("profile_toolpath", &params).unwrap();
        h.app.invoke(&h.ctx, "machining_layout");
        h.frame(vec![]);
        let (&id, toolpath) = h.app.doc.toolpaths.iter().next().unwrap();
        let row = h
            .text_at(&toolpath.name.clone())
            .expect("the toolpath is listed");
        h.right_click(row);
        assert_eq!(h.app.selected_toolpath, Some(id));
        assert!(h.text_at("Recalculate").is_some(), "its menu is open");
    }

    #[test]
    fn toolpath_forms_take_tools_from_the_database_and_save_them_back() {
        let mut h = Harness::new();
        let path =
            std::env::temp_dir().join(format!("cut-craft-tools-{}/tools.json", std::process::id()));
        h.app.tools_path = Some(path.clone());
        h.app.invoke(&h.ctx, "pocket_toolpath");
        h.app.choose_tool("3 mm End Mill");
        h.frame(vec![]);
        let params = &mut h.app.form.as_mut().unwrap().params;
        assert_eq!(params["tool"]["diameter"], 3.0);
        params["tool"]["name"] = "Mine".into();
        h.app.save_form_tool();
        assert_eq!(h.app.tools.get("Mine").map(|t| t.diameter), Some(3.0));
        assert_eq!(files::load_tools(&path).unwrap(), h.app.tools);
    }

    #[test]
    fn the_tool_database_window_lists_and_deletes_tools() {
        let mut h = Harness::new();
        h.app.invoke(&h.ctx, "tool_database");
        h.frame(vec![]);
        assert!(h.app.tool_window);
        let count = h.app.tools.tools.len();
        h.app.delete_tool(0);
        h.frame(vec![]);
        assert_eq!(h.app.tools.tools.len(), count - 1);
    }

    #[test]
    fn previewing_all_toolpaths_is_previewing_each_in_turn() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        for id in ["drilling_toolpath", "profile_toolpath"] {
            h.app.invoke(&h.ctx, id);
            let params = h.app.form.as_ref().unwrap().params.clone();
            h.app.apply(id, &params).unwrap();
        }
        h.app.invoke(&h.ctx, "preview_all_toolpaths");
        assert!(h.app.view == View::ThreeD, "previewing shows the 3D view");
        h.frame(vec![]);
        let all = h.app.simulation.clone().expect("a preview");
        h.app.invoke(&h.ctx, "reset_preview");
        assert!(h.app.simulation.is_none());
        for id in h.app.doc.toolpaths.keys().copied().collect::<Vec<_>>() {
            h.app.selected_toolpath = Some(id);
            h.app.invoke(&h.ctx, "preview_toolpath");
        }
        assert_eq!(h.app.simulation, Some(all));
    }

    #[test]
    fn the_3d_view_shows_the_model_until_there_is_a_preview() {
        let mut h = Harness::new();
        h.app.design_tab = DESIGN_TABS.iter().position(|&t| t == "Modeling").unwrap();
        assert!(
            h.app
                .toolbar
                .modeling_groups
                .iter()
                .any(|g| g.tools.contains(&"create_shape"))
        );
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        let shape = h.app.registry.get("create_shape").unwrap().default_params();
        h.app.apply("create_shape", &shape).unwrap();
        h.app.view = View::ThreeD;
        h.frame(vec![]);
        let raised =
            |app: &mut CutCraftApp| app.surface_3d().unwrap().heights.iter().any(|&z| z > 0.0);
        assert!(raised(&mut h.app), "the model");
        let profile = h
            .app
            .registry
            .get("profile_toolpath")
            .unwrap()
            .default_params();
        h.app.apply("profile_toolpath", &profile).unwrap();
        h.app.invoke(&h.ctx, "preview_all_toolpaths");
        h.frame(vec![]);
        assert!(!raised(&mut h.app), "the preview, cut down from the top");
    }

    #[test]
    fn the_window_is_titled_cut_craft() {
        let h = Harness::new();
        assert_eq!(h.app.title, "Untitled - Cut Craft");
    }

    #[test]
    fn it_starts_on_the_start_page_with_the_no_job_menus() {
        let h = Harness::at_start();
        assert_eq!(h.app.screen, Screen::Start);
        assert_eq!(h.app.title, "Cut Craft", "no job, so no file name");
        assert_eq!(
            h.app.menu_names(),
            ["File", "Edit", "Machine", "Toolpaths", "Gadgets", "Help"]
        );
        assert!(
            h.app.view_transform.is_none(),
            "no 2D view before there is a job"
        );
    }

    #[test]
    fn create_a_new_file_goes_through_job_setup_to_the_job() {
        let mut h = Harness::at_start();
        h.app.invoke(&h.ctx, "new");
        h.frame(vec![]);
        assert_eq!(h.app.screen, Screen::NewJob);
        let form = h.app.form.as_mut().expect("the Job Setup form");
        assert_eq!(form.id, "job_setup");
        form.params["size"]["width"] = 300.0.into();
        assert!(
            (h.app.shown_job().width - 300.0).abs() < 1e-9,
            "the 2D view shows the job being set up"
        );
        h.app.job_setup_ok();
        h.frame(vec![]);
        assert_eq!(h.app.screen, Screen::Job);
        assert!(h.app.form.is_none());
        assert!((h.app.doc.job.width - 300.0).abs() < 1e-9);
        assert!(
            !h.app.history.can_undo(),
            "a new job starts with nothing to undo"
        );
        assert_eq!(
            h.app.status.current_tool, "Select",
            "Job Setup is done with"
        );
        assert!(h.app.menu_names().contains(&"Model"));
        assert!(h.app.view_transform.is_some());
    }

    #[test]
    fn cancelling_job_setup_goes_back_to_the_start_page() {
        let mut h = Harness::at_start();
        h.app.invoke(&h.ctx, "new");
        h.app.job_setup_cancel();
        h.frame(vec![]);
        assert_eq!(h.app.screen, Screen::Start);
        assert!(h.app.form.is_none());
    }

    /// The icons are made from Aspire's and not in git; their manifest is.
    fn icon_set() -> std::collections::HashSet<String> {
        let manifest = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../re/icon_set.json"
        ))
        .expect("re/icon_set.json");
        serde_json::from_slice::<Vec<Value>>(&manifest)
            .unwrap()
            .iter()
            .map(|e| e["path"].as_str().unwrap().to_owned())
            .collect()
    }

    #[test]
    fn start_page_and_job_setup_icons_are_in_the_icon_set() {
        let known = icon_set();
        for (_, _, icon) in STARTUP_TASKS {
            assert!(known.contains(icon), "{icon}");
        }
        assert!(known.contains(RECENT_FILE_ICON));
        // every picture a Job Setup section can show, whatever is chosen
        let fields = form::fields(&commands::registry().get("job_setup").unwrap().schema());
        let mut pictures = 0;
        for f in &fields {
            let Some(template) = &f.icon else { continue };
            let choices = |kind: &form::Kind| match kind {
                form::Kind::Choice(options, _) => options.iter().map(|o| o.value.clone()).collect(),
                _ => vec![],
            };
            let values: Vec<Value> = match &f.kind {
                form::Kind::Choice(..) => choices(&f.kind).into_iter().map(Value::from).collect(),
                form::Kind::Group(inner) => inner
                    .iter()
                    .flat_map(|i| {
                        choices(&i.kind)
                            .into_iter()
                            .map(|c| serde_json::json!({ i.key.as_str(): c }))
                    })
                    .collect(),
                _ => vec![Value::Null],
            };
            for value in values {
                let path = form::icon_path(template, &value);
                if !path.contains('{') {
                    assert!(known.contains(&path), "{} shows {path}", f.key);
                    pictures += 1;
                }
            }
        }
        assert!(
            pictures >= 9,
            "job type, size, Z zero and datum pictures: {pictures}"
        );
    }

    #[test]
    fn closing_the_job_goes_back_to_the_start_page() {
        let mut h = Harness::new();
        h.app.invoke(&h.ctx, "close");
        h.frame(vec![]);
        assert_eq!(h.app.screen, Screen::Start);
    }

    #[test]
    fn saved_and_opened_jobs_are_listed_as_recent_files() {
        let mut h = Harness::new();
        let dir = std::env::temp_dir().join(format!("cut-craft-recent-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        h.app.recent_path = Some(dir.join("recent.json"));
        let job = dir.join("first.ccjob");
        h.app.save_to(&job);
        assert_eq!(h.app.recent, std::slice::from_ref(&job));
        assert_eq!(
            files::load_recent(&dir.join("recent.json")),
            std::slice::from_ref(&job)
        );
        h.app.invoke(&h.ctx, "close");
        h.app.invoke(&h.ctx, "open_recent:0");
        assert_eq!(h.app.screen, Screen::Job);
        assert_eq!(h.app.session.path.as_deref(), Some(job.as_path()));
    }

    #[test]
    fn a_recent_file_that_has_gone_is_dropped() {
        let mut h = Harness::at_start();
        h.app.recent = vec![std::env::temp_dir().join("cut-craft-gone.ccjob")];
        h.app.invoke(&h.ctx, "open_recent:0");
        assert_eq!(h.app.screen, Screen::Start);
        assert_eq!(h.app.recent, Vec::<std::path::PathBuf>::new());
        assert!(h.app.status.message.contains("cut-craft-gone.ccjob"));
    }

    /// The middle of the 2D view on screen.
    fn view_middle(h: &Harness) -> Pos2 {
        h.app
            .view_area
            .expect("the 2D view has been drawn")
            .center()
    }

    #[test]
    fn the_wheel_zooms_about_the_cursor() {
        let mut h = Harness::new();
        let at = view_middle(&h) + vec2(40.0, -30.0);
        let (before, under) = {
            let t = h.app.view_transform.unwrap();
            (t.scale, t.to_job(at))
        };
        h.frame(vec![Event::PointerMoved(at)]);
        h.frame(vec![Event::Scroll(vec2(0.0, 120.0))]);
        h.frame(vec![]);
        let t = h.app.view_transform.unwrap();
        assert!(t.scale > before, "scrolling up zooms in");
        assert!(t.to_job(at).distance(under) < 0.01, "about the cursor");
    }

    #[test]
    fn the_middle_button_and_the_pan_tool_pan() {
        let mut h = Harness::new();
        let p = Point::new(100.0, 100.0);
        let before = h.app.view_transform.unwrap().to_screen(p);
        let from = view_middle(&h);
        h.frame(vec![Event::PointerMoved(from)]);
        let middle = |pressed: bool| Event::PointerButton {
            pos: from + vec2(40.0, 10.0) * f32::from(u8::from(!pressed)),
            button: PointerButton::Middle,
            pressed,
            modifiers: Modifiers::NONE,
        };
        h.frame(vec![middle(true)]);
        h.frame(vec![Event::PointerMoved(from + vec2(20.0, 5.0))]);
        h.frame(vec![Event::PointerMoved(from + vec2(40.0, 10.0))]);
        h.frame(vec![middle(false)]);
        h.frame(vec![]);
        let after = h.app.view_transform.unwrap().to_screen(p);
        assert!(
            (after - before - vec2(40.0, 10.0)).length() < 1.0,
            "{before:?} → {after:?}"
        );
        // the Pan tool does the same with the left button, and draws nothing
        h.app.invoke(&h.ctx, "pan");
        let before = h.app.view_transform.unwrap().to_screen(p);
        h.drag(from, from + vec2(-30.0, 0.0));
        let after = h.app.view_transform.unwrap().to_screen(p);
        assert!(
            (after - before - vec2(-30.0, 0.0)).length() < 1.0,
            "{before:?} → {after:?}"
        );
        assert!(h.app.doc.vectors.is_empty() && h.app.doc.selection.is_empty());
    }

    #[test]
    fn zoom_box_zoom_to_selection_and_zoom_to_job() {
        let mut h = Harness::new();
        let fitted = h.app.view_transform.unwrap().scale;
        h.app.invoke(&h.ctx, "zoom_box");
        h.drag(h.at(100.0, 100.0), h.at(200.0, 150.0));
        let boxed = h.app.view_transform.unwrap();
        assert!(boxed.scale > 3.0 * fitted, "the box fills the view");
        assert!(h.app.doc.selection.is_empty(), "zooming selects nothing");
        let area = h.app.view_area.unwrap();
        assert!(area.contains(boxed.to_screen(Point::new(150.0, 125.0))));
        h.rectangle(400.0, 300.0, 20.0, 10.0);
        h.app.invoke(&h.ctx, "zoom_selected");
        h.frame(vec![]);
        let t = h.app.view_transform.unwrap();
        let (a, b) = (
            t.to_screen(Point::new(400.0, 300.0)),
            t.to_screen(Point::new(420.0, 310.0)),
        );
        assert!(area.contains(a) && area.contains(b));
        assert!(
            (b.x - a.x) > area.width() / 2.0,
            "the selection fills the view"
        );
        h.app.invoke(&h.ctx, "zoom_to_job");
        h.frame(vec![]);
        assert!((h.app.view_transform.unwrap().scale - fitted).abs() < 1e-3);
    }

    #[test]
    fn the_view_buttons_are_the_view_actions() {
        for (id, _, icon) in VIEW_BUTTONS {
            assert!(icon_set().contains(icon), "{icon}");
            assert!(BUILT_INS.contains(&id), "{id}");
        }
    }

    fn key(key: Key) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }
    }

    #[test]
    fn f11_designs_f12_machines_and_switch_layout_swaps() {
        let mut h = Harness::new();
        assert_eq!(h.app.pages, Pages::design_layout());
        h.frame(vec![key(Key::F12)]);
        assert_eq!(h.app.pages, Pages::machining_layout());
        h.frame(vec![key(Key::F11)]);
        assert_eq!(h.app.pages, Pages::design_layout());
        h.app.invoke(&h.ctx, "interface_layout");
        assert_eq!(
            h.app.pages,
            Pages::machining_layout(),
            "the Switch Layout button"
        );
    }

    #[test]
    fn a_tool_from_a_flown_out_page_hides_it_unless_it_opens_a_form() {
        let mut h = Harness::new();
        h.app.pages = Pages::machining_layout();
        h.app.pages.open(Page::Design);
        h.app.pick(&h.ctx, Page::Design, "draw_rectangle");
        assert_eq!(
            h.app.pages.shown(Page::Design),
            Shown::Flyout,
            "its form shows there"
        );
        h.app.form = None;
        h.app.pick(&h.ctx, Page::Design, "select_all_vectors");
        assert_eq!(h.app.pages.shown(Page::Design), Shown::Hidden);
    }

    #[test]
    fn toolpath_forms_open_in_the_toolpaths_page() {
        let mut h = Harness::new();
        h.app.invoke(&h.ctx, "profile_toolpath");
        assert_eq!(h.app.form.as_ref().unwrap().page, Page::Toolpaths);
        assert_eq!(
            h.app.pages.shown(Page::Toolpaths),
            Shown::Flyout,
            "brought out to show its form"
        );
        h.app.invoke(&h.ctx, "draw_circle");
        assert_eq!(h.app.form.as_ref().unwrap().page, Page::Design);
    }

    #[test]
    fn clicking_the_view_puts_a_flown_out_page_away() {
        let mut h = Harness::new();
        h.app.pages.open(Page::Toolpaths);
        h.frame(vec![]);
        h.click(h.at(20.0, 200.0), Modifiers::NONE);
        assert_eq!(h.app.pages.shown(Page::Toolpaths), Shown::Hidden);
        assert_eq!(h.app.pages.shown(Page::Design), Shown::Docked);
    }

    #[test]
    fn menu_items_that_reuse_existing_tools() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 20.0, 20.0);
        h.app
            .apply("add_layer", &serde_json::json!({ "name": "Top" }))
            .unwrap();
        h.rectangle(50.0, 10.0, 20.0, 20.0);
        let on_top = *h.app.doc.selection.iter().next().unwrap();
        h.app.invoke(&h.ctx, "select_all_vectors");
        h.app.invoke(&h.ctx, "select_all_vectors_on_current_layer");
        assert_eq!(
            h.app.doc.selection,
            [on_top].into(),
            "Select Vectors on Layer, current layer"
        );
        h.app.invoke(&h.ctx, "job_size_and_position");
        assert_eq!(h.app.form.as_ref().map(|f| f.id), Some("job_setup"));
        h.app.invoke(&h.ctx, "show_toolpaths_tab");
        assert_ne!(h.app.pages.shown(Page::Toolpaths), Shown::Hidden);
        h.app.invoke(&h.ctx, "zoom_box");
        h.app.invoke(&h.ctx, "select");
        assert!(
            h.app.form.is_none() && h.app.view_tool.is_none(),
            "the Select tool"
        );
    }

    #[test]
    fn run_and_form_items_apply_or_open_a_command_with_their_params() {
        let mut h = Harness::new();
        let add = gui::MenuItem::run::<cmd_add_layer::AddLayer>(
            "Add",
            &cmd_add_layer::Params { name: "Top".into() },
        );
        assert!(h.app.available(&add.action));
        h.app.invoke(&h.ctx, &add.action);
        assert_eq!(
            h.app.doc.layers.last().map(|l| l.name.as_str()),
            Some("Top")
        );
        let mut rename: cmd_layer_properties::Params = h.app.doc.layers[0].clone().into();
        rename.name = "Renamed".into();
        let rename =
            gui::MenuItem::form::<cmd_layer_properties::LayerProperties>("Rename", &rename);
        h.app.invoke(&h.ctx, &rename.action);
        let form = h.app.form.as_ref().unwrap();
        assert_eq!(
            (form.id, form.params["name"].as_str()),
            ("layer_properties", Some("Renamed"))
        );
        assert!(!h.app.available("run:no_such_command:{}"));
    }

    /// Controls that wait on a person: the system's file dialogs, the save-changes question
    /// (the smoke test's job has unsaved work) and quitting.
    fn needs_a_person(app: &CutCraftApp, id: &str) -> bool {
        const ASKS: [&str; 10] = [
            "new",
            "open",
            "close",
            "save",
            "save_as",
            "import_vectors_bitmap",
            "install_post_processor",
            "save_toolpaths",
            "exit",
            "recent_files",
        ];
        ASKS.contains(&id) || app.registry.exporter(id).is_some()
    }

    /// Every enabled menu item and tool button, run through the app as a click would be on a
    /// small job with vectors, a toolpath and a 3D shape: none may panic or be a dead end.
    #[test]
    fn every_enabled_control_works() {
        let mut h = Harness::new();
        h.app.doc.job.width = 100.0;
        h.app.doc.job.height = 100.0;
        h.rectangle(10.0, 10.0, 40.0, 40.0);
        h.rectangle(30.0, 30.0, 40.0, 40.0);
        h.app.invoke(&h.ctx, "select_all_vectors");
        for id in ["profile_toolpath", "create_shape"] {
            let params = h.app.registry.get(id).unwrap().initial_params(&h.app.doc);
            h.app.apply(id, &params).unwrap();
        }
        h.app.selected_toolpath = h.app.doc.toolpaths.keys().next().copied();
        h.frame(vec![]);
        let toolbar = &h.app.toolbar;
        let mut ids: Vec<String> = h
            .app
            .menus
            .items()
            .iter()
            .map(|i| i.action.clone())
            .collect();
        for group in toolbar
            .groups
            .iter()
            .chain(&toolbar.modeling_groups)
            .chain(&toolbar.toolpath_groups)
        {
            ids.extend(group.tools.iter().map(|&t| t.to_owned()));
        }
        ids.extend(VIEW_BUTTONS.iter().map(|(id, ..)| (*id).to_owned()));
        // the right-click menus, on each thing they open on
        let doc = &h.app.doc;
        let mut popups = h.app.menus.selection_menu(doc);
        for layer in &doc.layers {
            popups.extend(gui::layer_menu(doc, layer));
        }
        for (&id, toolpath) in &doc.toolpaths {
            popups.extend(gui::toolpath_menu(id, toolpath));
        }
        for (&id, component) in &doc.components {
            popups.extend(gui::component_menu(id, component));
        }
        let enabled = gui::leaves(&popups).into_iter().filter(|i| i.is_enabled);
        ids.extend(enabled.map(|i| i.action.clone()));
        ids.sort();
        ids.dedup();
        let start = (h.app.doc.clone(), h.app.selected_toolpath);
        let mut ran = 0;
        for id in ids {
            if !h.app.available(&id) || needs_a_person(&h.app, &id) {
                continue;
            }
            (h.app.doc, h.app.selected_toolpath) = start.clone();
            h.app.history = History::default();
            (h.app.form, h.app.view_tool) = (None, None);
            h.app.status.set_message("");
            h.app.invoke(&h.ctx, &id);
            // a tool with a form runs with what the form starts with
            if let Some(form) = h.app.form.take() {
                let outcome = h.app.apply(form.id, &form.params);
                if let Err(why) = outcome {
                    assert!(!why.contains("not implemented"), "{id}: {why}");
                }
            }
            h.frame(vec![]);
            let said = &h.app.status.message;
            assert!(!said.contains("not implemented"), "{id}: {said}");
            ran += 1;
        }
        assert!(ran >= 92, "only {ran} controls ran");
    }

    #[test]
    fn preview_all_leaves_hidden_toolpaths_out() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        for id in ["drilling_toolpath", "profile_toolpath"] {
            let params = h.app.registry.get(id).unwrap().initial_params(&h.app.doc);
            h.app.apply(id, &params).unwrap();
        }
        let profile = *h.app.doc.toolpaths.keys().last().unwrap();
        h.app.selected_toolpath = Some(profile);
        h.app.invoke(&h.ctx, "preview_toolpath");
        let only_profile = h.app.simulation.clone();
        let drilling = *h.app.doc.toolpaths.keys().next().unwrap();
        h.app.doc.toolpaths.get_mut(&drilling).unwrap().visible = false;
        h.app.invoke(&h.ctx, "preview_all_toolpaths");
        assert_eq!(h.app.simulation, only_profile);
    }

    #[test]
    fn deleting_the_selected_toolpath_is_undoable() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        h.app.invoke(&h.ctx, "drilling_toolpath");
        let params = h.app.form.as_ref().unwrap().params.clone();
        h.app.apply("drilling_toolpath", &params).unwrap();
        h.app.selected_toolpath = h.app.doc.toolpaths.keys().next().copied();
        h.app.invoke(&h.ctx, "delete_toolpath");
        assert!(h.app.doc.toolpaths.is_empty());
        h.app.invoke(&h.ctx, "undo");
        assert_eq!(h.app.doc.toolpaths.len(), 1);
    }
}
