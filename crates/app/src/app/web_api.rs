//! Options ▸ Web API, the app's side: the server started and stopped as Options say, and each
//! call it hands over run here, on the window's own thread, as the matching click would be.

use super::{CutCraftApp, Screen, egui};
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_core::Change;
use cut_craft_web_api::job;
use cut_craft_web_api::{Answer, ApiError, Call, Pending, Server, State};
use serde_json::{Value, json};
use std::collections::BTreeSet;

/// What the status bar says while the Web API listens, in green.
pub(super) const RUNNING: &str = "WEB API Running";
/// What it says, in red, while the Web API is switched on but not listening.
pub(super) const NOT_RUNNING: &str = "WEB API Not Running";

/// Actions a call may not press: closing Cut Craft would end the very API that asked.
const NOT_FROM_THE_API: &[&str] = &["exit"];

/// The Web API's state kept by the app.
#[derive(Default)]
pub(super) struct WebApi {
    server: Option<Server>,
    /// The settings it was last started with, `None` while switched off.
    started_with: Option<cut_craft_web_api::Settings>,
    /// What the status bar shows.
    pub state: Option<State>,
    /// Calls waiting for the app to be free (a slow job running), oldest first.
    waiting: std::collections::VecDeque<Pending>,
}

impl CutCraftApp {
    /// Each frame: the server matched to Options (started, stopped or restarted), then the
    /// calls that came in run, in order.
    pub(super) fn web_api_frame(&mut self, ctx: &egui::Context) {
        self.sync_web_api(ctx);
        self.serve_web_api(ctx);
    }

    /// What the status bar shows about the Web API.
    pub(super) fn web_api_state(&self) -> &State {
        self.web_api.state.as_ref().unwrap_or(&State::Off)
    }

    /// Options ▸ Web API ▸ Make New Token: calls with the old one are turned away from now.
    pub(super) fn new_web_api_token(&mut self) {
        self.options.web_api.token = cut_craft_web_api::new_token();
        self.save_options();
        self.status
            .set_message("Made a new Web API token: scripts need the new one from now on");
    }

    /// Options ▸ Web API ▸ Try Again: starts it afresh with the same settings.
    pub(super) fn retry_web_api(&mut self) {
        self.web_api.server = None;
        self.web_api.started_with = None;
        self.web_api.state = None;
    }

    fn sync_web_api(&mut self, ctx: &egui::Context) {
        let o = &mut self.options.web_api;
        if o.enabled && o.token.is_empty() {
            o.token = cut_craft_web_api::new_token();
            self.save_options();
        }
        let wanted = self.options.web_api.server_settings();
        let api = &mut self.web_api;
        if api.state.is_some() && wanted == api.started_with {
            // its thread stopping by itself is a fault: say so rather than show green
            if api.server.as_ref().is_some_and(|s| !s.listening()) {
                api.server = None;
                api.state = Some(State::Failed(
                    "The Web API stopped by itself. Press Try Again in Options ▸ Web API.".into(),
                ));
            }
            return;
        }
        // the old one goes first, so a restart on the same port finds it free
        api.server = None;
        api.waiting.clear();
        api.started_with.clone_from(&wanted);
        api.state = Some(match wanted {
            None => State::Off,
            Some(settings) => {
                let ctx = ctx.clone();
                match Server::start(&settings, move || ctx.request_repaint()) {
                    Ok(server) => {
                        let at = server.address();
                        api.server = Some(server);
                        State::Running(at)
                    }
                    Err(why) => {
                        log::warn!("Web API: {why}");
                        State::Failed(why)
                    }
                }
            }
        });
    }

    fn serve_web_api(&mut self, ctx: &egui::Context) {
        if let Some(server) = &self.web_api.server {
            self.web_api.waiting.extend(server.take());
        }
        while let Some(pending) = self.web_api.waiting.pop_front() {
            // a slow job (a toolpath calculating) finishes first; asking what is going on
            // is always answered
            if self.busy.is_some() && pending.call != Call::Status {
                self.web_api.waiting.push_front(pending);
                break;
            }
            let answer = self.web_call(ctx, &pending.call);
            pending.answer(answer);
        }
    }

    /// Runs one call and says how it went.
    pub(super) fn web_call(&mut self, ctx: &egui::Context, call: &Call) -> Answer {
        match call {
            Call::Status => Ok(json!({
                "version": env!("CARGO_PKG_VERSION"),
                "job_open": self.screen == Screen::Job,
                "busy": self.busy.is_some(),
                "units": job::units_name(self.doc.job.units),
                "form": self.form.as_ref().map(|f| f.id),
                "message": self.status.message,
            })),
            Call::Commands => Ok(job::commands(&self.registry)),
            Call::Command(id) => job::command(&self.registry, &self.doc, id),
            Call::Actions => Ok(self.web_actions()),
            Call::Job => {
                self.job_needed()?;
                Ok(job::summary(&self.doc))
            }
            Call::Invoke(action) => self.web_invoke(ctx, action),
            Call::Run { command, params } => self.web_run(command, params),
            Call::Select { pick, add } => {
                self.job_needed()?;
                let mut ids = if *add {
                    self.doc.selection.iter().copied().collect()
                } else {
                    vec![]
                };
                ids.extend(job::pick(&self.doc, pick)?);
                let count = ids.iter().collect::<BTreeSet<_>>().len();
                let mut c = Change::builder(&self.doc);
                c.select(ids);
                self.record(Ok(c.finish()), &selected_message(count));
                Ok(json!({
                    "selection": self.doc.selection.iter().map(|v| v.0).collect::<Vec<_>>(),
                    "message": self.status.message,
                }))
            }
        }
    }

    fn job_needed(&self) -> Result<(), ApiError> {
        if self.screen == Screen::Job {
            Ok(())
        } else {
            Err(ApiError::refused(
                "No job is open. Open or start one in Cut Craft's window first.",
            ))
        }
    }

    /// `GET actions`: every menu item and tool button, by id and name, each with whether it
    /// can be used now (a greyed-out item cannot).
    fn web_actions(&self) -> Value {
        let mut seen = BTreeSet::new();
        let mut list = vec![];
        let tools = self
            .all_groups()
            .into_iter()
            .flat_map(|g| g.tools.iter().map(|&t| (t.to_owned(), self.tool_label(t))));
        for (id, name) in menu_items(self).into_iter().chain(tools) {
            if NOT_FROM_THE_API.contains(&id.as_str()) || !seen.insert(id.clone()) {
                continue;
            }
            let available = self.available(&id);
            list.push(json!({ "id": id, "name": name, "available": available }));
        }
        Value::Array(list)
    }

    /// `POST invoke`: presses a button or menu item, as a click does.
    fn web_invoke(&mut self, ctx: &egui::Context, action: &str) -> Answer {
        if NOT_FROM_THE_API.contains(&action) {
            return Err(ApiError::refused(format!(
                "\"{action}\" cannot be pressed from the Web API: it would close Cut Craft."
            )));
        }
        let known = self.registry.get(action).is_some()
            || menu_items(self).iter().any(|(id, _)| id == action)
            || self.all_groups().iter().any(|g| g.tools.contains(&action));
        if !known {
            return Err(ApiError::not_found(format!(
                "There is no button or menu item \"{action}\". GET {}actions lists them.",
                cut_craft_web_api::PREFIX
            )));
        }
        if !self.available(action) {
            return Err(ApiError::refused(format!(
                "{} cannot be used now (it is greyed out in the window).",
                self.tool_label(action)
            )));
        }
        cut_craft_bug_report::logs::step(&format!("Web API pressed {action}"));
        self.invoke(ctx, action);
        Ok(json!({
            "message": self.status.message,
            "form": self.form.as_ref().map(|f| f.id),
        }))
    }

    /// `POST run`: a tool run with the caller's settings over what its form starts with, as
    /// one Undo step, with no form.
    fn web_run(&mut self, command: &str, given: &Value) -> Answer {
        self.job_needed()?;
        let params = job::run_params(&self.registry, &self.doc, command, given)?;
        cut_craft_bug_report::logs::step(&format!("Web API ran {command}"));
        let vectors: BTreeSet<_> = self.doc.vectors.keys().copied().collect();
        let toolpaths: BTreeSet<_> = self.doc.toolpaths.keys().copied().collect();
        self.apply(command, &params).map_err(ApiError::refused)?;
        let added = |now: &mut dyn Iterator<Item = &cut_craft_core::document::EntityId>,
                     before: &BTreeSet<_>| {
            now.filter(|id| !before.contains(*id))
                .map(|id| id.0)
                .collect::<Vec<_>>()
        };
        Ok(json!({
            "message": self.status.message,
            "added_vectors": added(&mut self.doc.vectors.keys(), &vectors),
            "added_toolpaths": added(&mut self.doc.toolpaths.keys(), &toolpaths),
            "selection": self.doc.selection.iter().map(|v| v.0).collect::<Vec<_>>(),
        }))
    }
}

/// The status bar's note after a call picks vectors.
fn selected_message(count: usize) -> String {
    match count {
        0 => "Nothing selected (Web API)".into(),
        1 => "1 vector selected (Web API)".into(),
        n => format!("{n} vectors selected (Web API)"),
    }
}

/// Every item on the app's menus that does something, by action id, with its name.
fn menu_items(app: &CutCraftApp) -> Vec<(String, String)> {
    fn walk(items: &[crate::gui::MenuItem], out: &mut Vec<(String, String)>) {
        for item in items {
            if item.children.is_empty() {
                if !item.name.is_empty() && !item.action.is_empty() {
                    out.push((item.action.clone(), item.name.clone()));
                }
            } else {
                walk(&item.children, out);
            }
        }
    }
    let mut out = vec![];
    for menu in &app.menus.menus {
        walk(&menu.items, &mut out);
    }
    out
}

impl CutCraftApp {
    /// The status bar's Web API slot: green while it listens, red while it is switched on but
    /// not listening, nothing while it is off. Always as wide as its longest wording, so the
    /// labels beside it never move. Clicking it opens Options ▸ Web API. Whether it showed.
    pub(super) fn web_api_indicator(&mut self, ui: &mut egui::Ui) -> bool {
        let (text, colour, hint) = match self.web_api_state() {
            State::Off => return false,
            State::Running(at) => (
                RUNNING,
                RUNNING_GREEN,
                format!(
                    "Programs can drive Cut Craft at http://{at}{}. Click here for its settings \
                     and access token.",
                    cut_craft_web_api::PREFIX
                ),
            ),
            State::Failed(why) => (
                NOT_RUNNING,
                ui.visuals().error_fg_color,
                format!("{why} Click here to open those settings."),
            ),
        };
        let font = egui::TextStyle::Body.resolve(ui.style());
        let widest = [RUNNING, NOT_RUNNING]
            .iter()
            .map(|t| {
                ui.painter()
                    .layout_no_wrap((*t).to_owned(), font.clone(), colour)
                    .size()
                    .x
            })
            .fold(0.0, f32::max);
        let size = egui::vec2(widest + 8.0, ui.spacing().interact_size.y);
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            font,
            colour,
        );
        let response = response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_hint(hint);
        if response.clicked() {
            self.options_window = true;
            self.options_goto = Some("web_api");
        }
        true
    }

    /// What was pressed below Options ▸ Web API's settings, done.
    pub(super) fn web_api_pressed(&mut self, pressed: Pressed) {
        match pressed {
            Pressed::Copied => self.status.set_message("Copied"),
            Pressed::NewToken => self.new_web_api_token(),
            Pressed::TryAgain => self.retry_web_api(),
        }
    }
}

/// A button below Options ▸ Web API's settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Pressed {
    Copied,
    NewToken,
    TryAgain,
}

/// Below Options ▸ Web API's settings: whether it is running, the token with Copy and Make
/// New Token, and a first call to try. What was pressed, if anything.
pub(super) fn options_section(
    ui: &mut egui::Ui,
    state: &State,
    settings: &crate::options::WebApi,
) -> Option<Pressed> {
    let mut pressed = None;
    ui.separator();
    match state {
        State::Off => {
            ui.weak("The Web API is off. Tick Turn on the Web API to start it.");
        }
        State::Running(at) => {
            ui.colored_label(RUNNING_GREEN, format!("{RUNNING} at http://{at}"));
        }
        State::Failed(why) => {
            ui.colored_label(ui.visuals().error_fg_color, NOT_RUNNING);
            ui.label(why);
            if ui
                .button("Try Again")
                .on_hover_hint("Start the Web API again with these settings")
                .clicked()
            {
                pressed = Some(Pressed::TryAgain);
            }
        }
    }
    let token = &settings.token;
    let needs_token = settings.allow_network && !token.is_empty();
    ui.add_space(ui.spacing().item_spacing.y);
    if needs_token {
        ui.strong("Access token");
        ui.label(
            "Other computers can reach Cut Craft, so every call must send this. Give it only to \
             programs you trust, and keep it private, like a password.",
        );
    } else {
        ui.label(
            "Programs on this computer need no token. Tick Allow other computers on my network \
             and every call will need an access token, shown here.",
        );
    }
    if needs_token {
        ui.horizontal(|ui| {
            ui.monospace(format!("{}…", token.chars().take(8).collect::<String>()));
            if ui
                .button("Copy")
                .on_hover_hint("Copy the whole token, to paste into your script")
                .clicked()
            {
                ui.ctx().copy_text(token.clone());
                pressed = Some(Pressed::Copied);
            }
            if ui
                .button("Make New Token")
                .on_hover_hint(
                    "Stop the old token working, for example if someone you did not mean to have \
                 it does",
                )
                .clicked()
            {
                pressed = Some(Pressed::NewToken);
            }
        });
    }
    ui.add_space(ui.spacing().item_spacing.y);
    ui.strong("Try it");
    ui.label("Paste this in a terminal: Cut Craft answers with its version and what it is doing.");
    let example = example_call(settings.port, needs_token);
    ui.add(egui::Label::new(egui::RichText::new(&example).monospace()).wrap());
    if ui
        .button("Copy Command")
        .on_hover_hint("Copy this command, to paste in a terminal")
        .clicked()
    {
        ui.ctx().copy_text(example.replace("<token>", token));
        pressed = Some(Pressed::Copied);
    }
    ui.label("Help ▸ Contents ▸ Web API lists every call.");
    pressed
}

/// A first call to try: with `<token>` where the token goes when calls need one.
pub(super) fn example_call(port: u16, token: bool) -> String {
    let auth = if token {
        "-H \"Authorization: Bearer <token>\" "
    } else {
        ""
    };
    format!(
        "curl {auth}http://localhost:{port}{}status",
        cut_craft_web_api::PREFIX
    )
}

/// The status bar's green: readable on the light and the dark theme alike.
const RUNNING_GREEN: egui::Color32 = egui::Color32::from_rgb(0x2e, 0x9e, 0x44);
