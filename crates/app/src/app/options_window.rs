//! The Options window: its categories, asking about unapplied changes, and applying them.

use super::{CutCraftApp, Value, dialog_size, egui};
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_i18n::tr;

impl CutCraftApp {
    /// Edit ▸ Options: the sections; changes wait for Apply, and Cancel throws them away.
    #[expect(
        clippy::too_many_lines,
        reason = "the one Options window, with the extras some pages show under their settings"
    )]
    pub(super) fn options_window(&mut self, ctx: &egui::Context) {
        let mut open = self.options_window;
        let fields = self.options_categories();
        let mut values = serde_json::to_value(self.edited_options()).expect("options serialise");
        let dirty = self.options_dirty();
        let closing = self.options_closing;
        let mut button = None;
        let mut answer = None;
        let mut changed = None;
        let mut reset = None;
        if let Some(key) = self.options_goto.take()
            && let Some(at) = fields.iter().position(|f| f.key == key)
        {
            self.options_page = at;
        }
        // Mouse and Shortcuts lists every action's shortcuts under its settings
        let on_shortcuts = fields
            .get(self.options_page)
            .is_some_and(|f| f.key == super::shortcuts_page::CATEGORY);
        let rows = if on_shortcuts {
            self.shortcut_rows(&self.shortcut_search)
        } else {
            vec![]
        };
        let names = self.shortcut_actions();
        let label_of = |id: &str| {
            names
                .iter()
                .find(|(a, _)| a == id)
                .map_or_else(|| id.to_owned(), |(_, label)| label.clone())
        };
        let on_job_defaults = fields
            .get(self.options_page)
            .is_some_and(|f| f.key == crate::options::JOB_DEFAULTS);
        let (job_fields, mut job_params, job_error) = if on_job_defaults {
            self.job_defaults_state()
        } else {
            (vec![], Value::Null, None)
        };
        let on_language = fields
            .get(self.options_page)
            .is_some_and(|f| f.key == crate::options::LANGUAGE);
        if on_language && self.language_packs.is_none() {
            let dir = self.addons.dir().map(std::path::Path::to_path_buf);
            self.language_packs = Some(dir.map_or_default(|dir| {
                crate::languages::installed(&dir)
                    .into_iter()
                    .map(|p| {
                        let percent = crate::languages::coverage(&dir, &p);
                        (p, percent)
                    })
                    .collect()
            }));
        } else if !on_language {
            self.language_packs = None;
        }
        let language_packs = self.language_packs.clone().unwrap_or_default();
        let chosen_language = self.edited_options().language.pack.clone();
        let mut language_picked = None;
        let capture = self.capture.clone();
        let edited_web_api = self.edited_options().web_api.clone();
        let web_api_state = self.web_api_state().clone();
        let search = &mut self.shortcut_search;
        let mut shortcut_edit = None;
        let mut web_api_pressed = None;
        let unsigned_remembered = self.trust.remembered();
        let mut forget_unsigned = false;
        let mut job_edited = false;
        let (icons, materials) = (&self.icons, &self.materials);
        let show_advanced = &mut self.show_advanced;
        let page = &mut self.options_page;
        *page = (*page).min(fields.len().saturating_sub(1));
        let (id, size) = (egui::Id::unique("options"), dialog_size(ctx));
        // the categories down the left, the chosen one's settings beside them
        cut_craft_bug_report::dialog_with_panels(ctx, id, "Options", size, &mut open, |ui| {
            // the window's own close box asks first too, while changes wait
            cut_craft_bug_report::unapplied::hold_close(ui, dirty);
            egui::Panel::bottom("options_buttons").show(ui, |ui| {
                button = super::options_apply::footer(ui, dirty);
            });
            if closing {
                answer = super::options_apply::question(ui.ctx());
            }
            egui::Panel::left("options_categories")
                .resizable(false)
                .show(ui, |ui| {
                    for (i, f) in fields.iter().enumerate() {
                        if ui.selectable_label(i == *page, &f.label).clicked() {
                            *page = i;
                        }
                    }
                    ui.separator();
                    ui.checkbox(show_advanced, tr!("views-show-advanced-settings"))
                        .on_hover_text(tr!("views-list-settings-for-how-hard"));
                });
            egui::CentralPanel::default().show(ui, |ui| {
                let Some(category) = fields.get(*page) else {
                    return;
                };
                if category.key == "advanced" {
                    ui.colored_label(ui.visuals().warn_fg_color, crate::options::WARNING);
                    ui.label(crate::options::WARNING_MORE);
                    ui.separator();
                }
                let units = self.doc.job.units;
                let look = crate::form::Look::plain(units, crate::form::material(&self.doc, units));
                egui::Panel::bottom("options_reset").show(ui, |ui| {
                    if ui
                        .button(tr!("views-reset-to-defaults"))
                        .on_hover_text(tr!("views-put-every-setting-on-this"))
                        .clicked()
                    {
                        reset = Some(category.key.clone());
                    }
                });
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let one = std::slice::from_ref(category);
                    if crate::form::show(ui, one, &mut values, &look) {
                        changed = serde_json::from_value::<crate::options::Options>(values).ok();
                    }
                    if category.key == crate::options::JOB_DEFAULTS {
                        job_edited = super::setup_material::job_defaults_section(
                            ui,
                            &job_fields,
                            &mut job_params,
                            job_error.as_deref(),
                            icons,
                            materials,
                        );
                    }
                    if category.key == crate::options::LANGUAGE {
                        language_picked = super::language_page::show(
                            ui,
                            &language_packs,
                            &chosen_language,
                            *show_advanced,
                        );
                    }
                    if category.key == "confirmations" {
                        forget_unsigned = unsigned_code_section(ui, unsigned_remembered);
                    }
                    if category.key == "web_api" {
                        web_api_pressed =
                            super::web_api::options_section(ui, &web_api_state, &edited_web_api);
                    }
                    if category.key == super::shortcuts_page::CATEGORY {
                        shortcut_edit = super::shortcuts_page::show(
                            ui,
                            &rows,
                            capture.as_ref(),
                            search,
                            &label_of,
                        );
                    }
                });
            });
        });
        if let Some(page) = reset {
            if page == crate::options::JOB_DEFAULTS {
                self.job_defaults_form = None;
            }
            changed = Some(self.edited_options().reset(&page));
        }
        // leaving the page stops waiting for a shortcut
        if !on_shortcuts {
            self.capture = None;
        }
        if let Some(pack) = language_picked {
            let mut options = changed
                .take()
                .unwrap_or_else(|| self.edited_options().clone());
            options.language.pack = pack;
            changed = Some(options);
        }
        // kept until Apply
        if let Some(options) = changed {
            self.options_edited(options);
        }
        if job_edited {
            self.job_defaults_edited(job_params);
        }
        // the close box is Close: it asks first while changes wait
        if !open {
            button = Some(super::options_apply::Button::Close);
        }
        if let Some(button) = button {
            self.options_button(ctx, button);
        }
        if let Some(answer) = answer {
            self.options_answer(ctx, answer);
        }
        if forget_unsigned {
            self.trust.forget();
            self.status.set_message(&tr!("options-unsigned-forgotten"));
        }
        if let Some(pressed) = web_api_pressed {
            self.web_api_pressed(pressed);
        }
        // after the page's settings, so they don't put back the shortcuts as they were
        if let Some(edit) = shortcut_edit {
            self.apply_shortcut_edit(edit);
        }
    }

    /// The Options window's categories, each with its settings: Advanced only once asked
    /// for, and its Wayland choice only on Wayland.
    fn options_categories(&self) -> Vec<crate::form::Field> {
        let schema = serde_json::to_value(schemars::schema_for!(crate::options::Options))
            .expect("a schema serialises");
        let mut fields = crate::form::fields(&schema);
        for f in &mut fields {
            if let crate::form::Kind::Group(inner) = &mut f.kind
                && f.key == "advanced"
                && !crate::options::Advanced::wayland()
            {
                inner.retain(|f| f.key != "wayland_native");
            }
        }
        if !self.show_advanced {
            fields.retain(|f| f.key != "advanced");
        }
        fields
    }

    /// Options changed in the Options window: what they change, done, and kept.
    /// Save Toolpaths' question when the files would replace G-code from an earlier save:
    /// Replace, Archive (kept aside) or Cancel, with the answer kept as the default if asked.
    pub(super) fn earlier_question(&mut self, ctx: &egui::Context) {
        use crate::options::EarlierGcode;
        const SHOWN: usize = 8;
        let Some(ask) = self.earlier_ask.as_mut() else {
            return;
        };
        let mut answer: Option<Option<EarlierGcode>> = None;
        egui::Modal::new(egui::Id::unique("earlier_gcode")).show(ctx, |ui| {
            ui.set_width(ui.spacing().interact_size.x * 11.0);
            ui.heading(tr!("views-replace-the-earlier-g-code"));
            ui.label(if ask.sending {
                tr!("views-these-files-from-an-earlier-2")
            } else {
                tr!("views-these-files-from-an-earlier")
            });
            for file in ask.earlier.iter().take(SHOWN) {
                ui.label(format!("• {}", super::file_name(file)));
            }
            if ask.earlier.len() > SHOWN {
                ui.label(tr!("views-and-more", count = ask.earlier.len() - SHOWN));
            }
            ui.add_space(ui.spacing().item_spacing.y);
            ui.checkbox(&mut ask.remember, tr!("views-do-this-every-time"))
                .on_hover_hint(tr!("views-keep-this-answer-and-stop"));
            ui.separator();
            ui.horizontal(|ui| {
                if ui
                    .button(tr!("views-replace"))
                    .on_hover_hint(tr!("views-delete-the-earlier-files-older"))
                    .clicked()
                {
                    answer = Some(Some(EarlierGcode::Replace));
                }
                if ui
                    .button(tr!("views-archive"))
                    .on_hover_hint(tr!("views-move-the-earlier-files-into"))
                    .clicked()
                {
                    answer = Some(Some(EarlierGcode::Archive));
                }
                if ui
                    .button(tr!("views-cancel"))
                    .on_hover_hint(tr!("views-save-nothing-and-leave-the"))
                    .clicked()
                {
                    answer = Some(None);
                }
            });
        });
        if let Some(choice) = answer {
            let remember = ask.remember;
            self.answer_earlier(choice, remember);
        }
    }

    pub(super) fn options_changed(
        &mut self,
        ctx: &egui::Context,
        mut options: crate::options::Options,
    ) {
        // the Preview tab's detail stays as it was moved, unless the default changed
        let now = self.display.detail;
        options.apply_to(&mut self.display);
        if options.performance.preview_detail == self.options.performance.preview_detail {
            self.display.detail = now;
        }
        ctx.set_theme(options.appearance.theme.preference());
        let frame = options.appearance.system_title_bar;
        if frame != self.options.appearance.system_title_bar {
            if super::mac::is_mac(ctx) {
                // a Mac's window can't change its title bar while open
                self.status
                    .set_message(&cut_craft_i18n::tr!("status-title-bar-next-start"));
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(frame));
            }
        }
        // the checks ran on the moves thinned as they were: checked again as now saved
        let (new, old) = (&options.toolpaths, &self.options.toolpaths);
        if (new.tolerance_2d, new.tolerance_3d, new.tolerance_vcarve)
            != (old.tolerance_2d, old.tolerance_3d, old.tolerance_vcarve)
        {
            self.forget_save_checks();
        }
        if options.toolpaths.preview_multiplier != self.options.toolpaths.preview_multiplier {
            self.playback_speed = options.toolpaths.preview_speed();
        }
        let language =
            (options.language != self.options.language).then(|| options.language.pack.clone());
        options.apply();
        // the window never changes the toolbars of your own: ones made while it was open stay
        options.toolbars.clone_from(&self.options.toolbars);
        self.options = options;
        self.save_options();
        if let Some(pack) = language {
            self.use_language(&pack);
        }
        // clipping turned on or off: the toolpaths made again with or without it
        if self.sync_machine_keep_out() {
            self.keep_out_changed();
        }
    }
}

/// Options ▸ Confirmations: how much unsigned code the user said to run without asking,
/// and a button that forgets those answers (at once: not a setting Apply keeps). Whether it
/// was pressed.
fn unsigned_code_section(ui: &mut egui::Ui, remembered: usize) -> bool {
    ui.separator();
    ui.strong(tr!("options-unsigned-heading"));
    ui.label(tr!("options-unsigned-remembered", count = remembered));
    ui.add_enabled(
        remembered > 0,
        egui::Button::new(tr!("options-unsigned-forget")),
    )
    .on_hover_text(tr!("options-unsigned-forget.tooltip"))
    .clicked()
}
