//! Generate Border's form, beyond its settings: the person's own Google key, how to get the
//! Borders add-on, and Generate, which runs it in the background (`crate::borders`) and
//! adds the border it saves to the Library, under Generated ▸ Borders.

use super::CutCraftApp;
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_i18n::tr;
use eframe::egui;
use serde_json::Value;

/// The form's id.
pub(super) const ID: &str = "generate_border";

/// Generate Border's state between frames.
/// The key kept, as last read.
#[derive(Default)]
enum Key {
    #[default]
    NotRead,
    None,
    Kept(String),
}

#[derive(Default)]
pub(super) struct BorderForm {
    /// Where the key is kept (none in tests).
    pub keys: Option<crate::borders::KeyStore>,
    /// The key kept, once read.
    key: Key,
    /// The key being typed in, before Save Key.
    typed: String,
    /// What the last Save Key or Generate said, and whether it went wrong.
    note: Option<(String, bool)>,
    generating: Option<crate::borders::Generating>,
    /// Generate pressed, its settings: started on the next frame.
    pub pending: Option<Value>,
}

impl BorderForm {
    fn key(&mut self) -> Option<&str> {
        if matches!(self.key, Key::NotRead) {
            self.key = match self.keys.as_ref().and_then(crate::borders::KeyStore::load) {
                Some(key) => Key::Kept(key),
                None => Key::None,
            };
        }
        match &self.key {
            Key::Kept(key) => Some(key),
            _ => None,
        }
    }
}

impl CutCraftApp {
    /// Generate pressed: the border asked for in the background, or why it can't be yet
    /// (said in red at the foot of the form, never as a pop-up).
    fn generate_border(&mut self, params: Value, ctx: &egui::Context) {
        let refuse = |app: &mut Self, why: String| {
            app.border.note = Some((why, true));
        };
        if self.border.generating.is_some() {
            return;
        }
        let params: cmd_generate_border::Params = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return refuse(self, e.to_string()),
        };
        if let Err(e) = params.check() {
            return refuse(self, e.to_string());
        }
        let Some(program) = self.addons.dir().and_then(crate::borders::find) else {
            return refuse(self, tr!("border-needs-addon"));
        };
        let Some(key) = self.border.key().map(str::to_owned) else {
            return refuse(self, tr!("border-needs-key"));
        };
        let depth = self.addons.dir().and_then(crate::depth::find);
        if params.depth_from == cmd_generate_border::DepthFrom::DepthAddOn && depth.is_none() {
            return refuse(self, tr!("border-needs-depth-addon"));
        }
        let Some(root) = crate::borders::library_root() else {
            return refuse(self, tr!("border-no-documents"));
        };
        // each program it starts is asked about: the Borders add-on's, and the Depth
        // add-on's when the depth comes from it
        if !self.may_run_add_on(crate::borders::ADDON_ID, "The Borders add-on", &program) {
            return;
        }
        if params.depth_from == cmd_generate_border::DepthFrom::DepthAddOn
            && let Some(depth) = &depth
            && !self.may_run_add_on(crate::depth::ADDON_ID, "The Depth add-on", &depth.program)
        {
            return;
        }
        let job = crate::borders::Job {
            program,
            key,
            depth,
            params,
            folder: crate::borders::folder(&root),
        };
        match crate::borders::Generating::start(job, ctx) {
            Ok(running) => {
                self.border.note = None;
                self.border.generating = Some(running);
            }
            Err(e) => self.border.note = Some((e, true)),
        }
    }

    /// Once a frame: Generate's border started once pressed, and once made, put in the
    /// Library and shown there.
    pub(super) fn follow_border(&mut self, ctx: &egui::Context) {
        if let Some(params) = self.border.pending.take() {
            self.generate_border(params, ctx);
        }
        let Some(answer) = self
            .border
            .generating
            .as_ref()
            .and_then(crate::borders::Generating::answer)
        else {
            return;
        };
        self.border.generating = None;
        match answer {
            Ok(files) => {
                let name = files
                    .depth
                    .file_stem()
                    .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
                if let Some(root) = crate::borders::library_root()
                    && let Err(e) = self.clipart.add(root)
                {
                    self.status.set_message(&e);
                }
                if let Some(folder) = files.depth.parent() {
                    self.clipart.choose(Some(folder.to_path_buf()));
                }
                let said = tr!("border-saved", name = name);
                self.status.set_message(&said);
                self.border.note = Some((said, false));
            }
            Err(e) => self.border.note = Some((e, true)),
        }
    }

    /// Under Generate Border's settings: what is sent where, the key, and how it is going.
    pub(super) fn border_section(&mut self, ui: &mut egui::Ui) {
        let installed = self.addons.dir().and_then(crate::borders::find).is_some();
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.strong(tr!("border-google-title"));
            ui.label(tr!("border-sent-to-google"));
            self.border_key_box(ui);
        });
        if !installed {
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.label(tr!("border-needs-addon"));
                self.border_install(ui);
            });
        }
        if let Some(running) = &self.border.generating {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(tr!(running.stage().key()));
            });
        }
        if let Some((note, wrong)) = &self.border.note {
            if *wrong {
                ui.colored_label(ui.visuals().error_fg_color, note);
            } else {
                ui.label(note);
            }
        }
    }

    /// The Google key box: Save Key keeps what was typed in the password store; once one is
    /// kept, only that it is (never the key itself), and Forget Key.
    fn border_key_box(&mut self, ui: &mut egui::Ui) {
        let Some(keys) = self.border.keys.clone() else {
            return;
        };
        if self.border.key().is_some() {
            ui.horizontal(|ui| {
                ui.label(tr!("border-key-kept"));
                if ui
                    .button(tr!("border-forget-key"))
                    .on_hover_hint(tr!("border-forget-key.tooltip"))
                    .clicked()
                {
                    self.border.note = keys.forget().err().map(|e| (e, true));
                    self.border.key = Key::NotRead;
                }
            });
            return;
        }
        ui.horizontal(|ui| {
            ui.label(tr!("border-key"));
            ui.add(
                egui::TextEdit::singleline(&mut self.border.typed)
                    .password(true)
                    .desired_width(ui.available_width() * 0.6),
            )
            .on_hover_hint(tr!("border-key.tooltip"));
            let typed = !self.border.typed.trim().is_empty();
            if ui
                .add_enabled(typed, egui::Button::new(tr!("border-save-key")))
                .on_hover_hint(tr!("border-save-key.tooltip"))
                .clicked()
            {
                let typed = std::mem::take(&mut self.border.typed);
                self.border.key = Key::NotRead;
                self.border.note = Some(match keys.save(&typed) {
                    Ok(crate::borders::Kept::PasswordStore) => (tr!("border-key-saved"), false),
                    Ok(crate::borders::Kept::PrivateFile) => (tr!("border-key-saved-file"), false),
                    Err(e) => (e, true),
                });
            }
        });
        ui.hyperlink_to(tr!("border-get-key"), crate::borders::KEY_PAGE);
    }

    /// Install Borders Add-on, or how its install is going.
    fn border_install(&mut self, ui: &mut egui::Ui) {
        use crate::addons::Phase;
        match &self.addons.phase {
            Phase::Finding(_) | Phase::Installing { .. } | Phase::Asking(_) => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(tr!("border-addon-installing"));
                });
            }
            phase => {
                if let Phase::Failed(e) = phase {
                    ui.colored_label(ui.visuals().error_fg_color, e);
                }
                let can = self.addons.dir().is_some();
                if ui
                    .add_enabled(can, egui::Button::new(tr!("border-addon-install")))
                    .on_hover_hint(tr!("border-addon-install.tooltip"))
                    .on_disabled_hover_text(tr!("depth-addon-install.disabled"))
                    .clicked()
                {
                    let borders = cut_craft_updater::addon::OFFICIAL
                        .iter()
                        .find(|a| a.id == crate::borders::ADDON_ID)
                        .expect("Borders is an official add-on");
                    self.addons.install_from(ui.ctx(), borders.repo);
                }
            }
        }
    }
}
