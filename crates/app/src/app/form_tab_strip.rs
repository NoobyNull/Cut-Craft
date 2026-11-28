//! A toolpath form's tabs ([`crate::form::form_tabs`]): the strip across its top, which tab
//! is shown, and its Defaults tab.

use super::{CutCraftApp, Page, egui};
use crate::form::form_tabs::{self, Dot, FormTab};
use crate::options::{CalculateButton, ToolForms};
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_i18n::tr;

/// Which tab a tabbed form shows.
#[derive(Debug, Default)]
pub(super) struct Shown {
    pub(super) tab: FormTab,
    /// The warnings the form last turned to the tab of, so it turns once for each, and the
    /// user can go to another tab while they stand.
    turned_for: Option<String>,
}

impl CutCraftApp {
    /// Whether the open form shows its settings a tab at a time: a toolpath's, unless
    /// Options ▸ Appearance ▸ Tool forms says One page.
    pub(super) fn form_is_tabbed(&self) -> bool {
        self.options.appearance.tool_forms == ToolForms::Tabs
            && self
                .form
                .as_ref()
                .is_some_and(|f| f.page == Page::Toolpaths && form_tabs::tabbed(&f.fields))
    }

    /// The tab the open form shows, when it is tabbed.
    pub(super) fn form_tab(&self) -> Option<FormTab> {
        self.form_is_tabbed()
            .then(|| self.form.as_ref().map(|f| f.tabs.tab))
            .flatten()
    }

    /// What a new form of the open tool opens with: the settings a changed one is told
    /// apart from.
    fn form_defaults(&self) -> Option<serde_json::Value> {
        let form = self.form.as_ref()?;
        Some(self.registry.get(form.id)?.default_params())
    }

    /// Keeps which of the open form's settings are changed from a new form's, for the
    /// blue bar beside each; none on a form that is not a toolpath's.
    pub(super) fn note_changed_settings(&self, ctx: &egui::Context) {
        let keys = match (self.form.as_ref(), self.form_defaults()) {
            (Some(form), Some(defaults))
                if form.page == Page::Toolpaths && form_tabs::tabbed(&form.fields) =>
            {
                form_tabs::changed_keys(&form.fields, &form.params, &defaults)
            }
            _ => vec![],
        };
        form_tabs::set_changed(ctx, keys);
    }

    /// The open form's strip of tabs: a red dot on a tab a warning is about (the form
    /// turns to it once, when the warning comes), a blue one on a tab with a changed
    /// setting. A help tour pointing at a setting turns to its tab.
    pub(super) fn form_tab_strip(&mut self, ui: &mut egui::Ui) {
        let defaults = self.form_defaults().unwrap_or_default();
        let Some(form) = self.form.as_ref() else {
            return;
        };
        let about_bit =
            Self::drill_sideways(form).is_some() || Self::wrong_kind_of_tool(form).is_some();
        // a profile cut right through with nothing to hold the part: said under Tabs
        let units = self.doc.job.units;
        let stock = super::command_form::form_stock(
            &self.doc,
            &form.params,
            units,
            super::machine_setup::bed(&self.machines),
        );
        let loose = form.id == "profile_toolpath" && super::tabs::cuts_loose(&form.params, stock);
        let wanted = crate::marks::wanted_name(ui.ctx());
        let Some(form) = self.form.as_mut() else {
            return;
        };
        let loose_words = loose.then(super::tabs::loose);
        let said: Vec<&str> = [
            form.error.as_deref(),
            form.warning.as_deref(),
            loose_words.as_deref(),
        ]
        .into_iter()
        .flatten()
        .collect();
        let warned = form_tabs::warned_tabs(&form.fields, &said, about_bit);
        let changed = form_tabs::changed_tabs(&form.fields, &form.params, &defaults);
        let now = said.join("\n");
        if said.is_empty() && !about_bit {
            form.tabs.turned_for = None;
        } else if let Some(&first) = warned.first()
            && form.tabs.turned_for.as_deref() != Some(now.as_str())
        {
            form.tabs.tab = first;
            form.tabs.turned_for = Some(now);
        }
        if let Some(tab) = wanted.and_then(|w| form_tabs::tab_of_mark(&form.fields, &w)) {
            form.tabs.tab = tab;
        }
        let tabs = form_tabs::tabs_of(&form.fields);
        if !tabs.contains(&form.tabs.tab) {
            form.tabs.tab = tabs.first().copied().unwrap_or(FormTab::Cutting);
        }
        let own = form_tabs::own_name(
            form.id,
            &crate::tool_names::form_name(&self.registry, form.id, form.label),
        );
        let dots = |t: FormTab| {
            if warned.contains(&t) {
                Some(Dot::Warned)
            } else if changed.contains(&t) {
                Some(Dot::Changed)
            } else {
                None
            }
        };
        form_tabs::strip(ui, &tabs, &mut form.tabs.tab, &own, &dots);
    }

    /// Whether the open form's Calculate is hidden on the tab shown: Options ▸ Appearance ▸
    /// Calculate button says Last tab only, and this is not the last; such a tab has Next.
    pub(super) fn calculate_waits_for_last_tab(&self) -> bool {
        let Some(tab) = self.form_tab() else {
            return false;
        };
        let Some(form) = self.form.as_ref() else {
            return false;
        };
        self.options.appearance.calculate_button == CalculateButton::LastTab
            && form_tabs::tabs_of(&form.fields).last() != Some(&tab)
    }

    /// Next, on a tab whose Calculate waits for the last tab: the tab after it.
    pub(super) fn next_form_tab(&mut self) {
        if let Some(form) = self.form.as_mut() {
            let tabs = form_tabs::tabs_of(&form.fields);
            if let Some(at) = tabs.iter().position(|&t| t == form.tabs.tab) {
                form.tabs.tab = tabs.get(at + 1).copied().unwrap_or(form.tabs.tab);
            }
        }
    }

    /// The Defaults tab, under its settings (the toolpath's name): this form's settings
    /// saved and loaded, and Reset.
    pub(super) fn defaults_tab(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.strong(tr!("form-tab-saved-settings"));
            let (save, load) = super::form_panel::settings_file_row(ui);
            let reset = ui
                .button(tr!("form-reset"))
                .on_hover_hint(tr!("form-tab-reset-hint"))
                .clicked();
            ui.weak(tr!("form-tab-reset-note"));
            if save || load {
                self.form_settings_file(save);
            } else if reset {
                // the bit picked stays: Reset puts back the form's settings, not the bit
                let bit = self.form.as_ref().map(|f| f.params["tool"].clone());
                self.reset_form();
                if let (Some(bit), Some(form)) = (bit, self.form.as_mut())
                    && !bit.is_null()
                {
                    form.params["tool"] = bit;
                }
            }
        });
    }
}
