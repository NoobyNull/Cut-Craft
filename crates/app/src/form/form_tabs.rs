//! A tool form split into tabs across its top, so its settings are a few short pages
//! instead of one long scroll: the same tabs, in the same order, on every toolpath form.
//!
//! Each setting's tab comes from its block (`x-group`), its strip (`x-tab`) or its key, by
//! one table ([`tab_of`]); a setting the table does not name goes on the toolpath's own tab.
//! A tab with nothing on it is left out, and Defaults is always last.

use super::{AFTER_TOOL, CUTTING_DEPTHS, Field, HoverHint, KNIFE, Value, WHICH_SIDE, egui, tr};

/// One of a tool form's tabs, in the order they stand.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FormTab {
    /// The bit, the bits that clear before it, how deep and which side: what cuts, and
    /// how far.
    #[default]
    Cutting,
    /// The toolpath's own settings (Clear Pocket, Strategy, Fill…), named for the tool.
    Own,
    /// What is cut: the vector selector, rest machining, boundaries, locked faces, Project
    /// onto 3D Model.
    Where,
    /// How the bit gets there and moves: ramps, leads, order, start points, corners, cut
    /// direction, tabs, pecks.
    Movement,
    /// What is left for a last pass: allowances.
    Finishing,
    /// The toolpath's name, and the form's saved settings.
    Defaults,
}

/// Every tab, in the order they stand.
pub const ALL: [FormTab; 6] = [
    FormTab::Cutting,
    FormTab::Own,
    FormTab::Where,
    FormTab::Movement,
    FormTab::Finishing,
    FormTab::Defaults,
];

/// The blocks whose settings say what cuts and how deep.
const CUTTING_GROUPS: [&str; 5] = [
    CUTTING_DEPTHS,
    WHICH_SIDE,
    KNIFE,
    "Carving bit",
    "Larger Area Clearance Bit",
];

/// The blocks that say what is cut, and what is left alone.
const WHERE_GROUPS: [&str; 5] = [
    "Machining Limit Boundary",
    "Boundary",
    "Rest Machining",
    "What the bits before left",
    "Along the part",
];

/// The blocks about how the bit moves.
const MOVEMENT_GROUPS: [&str; 6] = [
    "Ramp",
    "Ramp Plunge Moves",
    "Ordering",
    "Cut Direction",
    "Tabs",
    "Peck Drilling",
];

/// The blocks about what a last pass takes off.
const FINISHING_GROUPS: [&str; 2] = ["Machining Allowance", "Machine Vectors…"];

/// The tab a setting stands on.
pub fn tab_of(f: &Field) -> FormTab {
    let in_group = |groups: &[&str]| f.group.as_deref().is_some_and(|g| groups.contains(&g));
    match f.key.as_str() {
        "name" => return FormTab::Defaults,
        "tool" | "tools" | "clearance_tools" => return FormTab::Cutting,
        "bits_before" | "project_onto_model" => return FormTab::Where,
        // the order the vectors are cut in, and where each starts
        "use_vector_selection_order" | "use_selection_order" | "use_vector_start_points" => {
            return FormTab::Movement;
        }
        // a rotary toolpath's stock left on, outside any block
        "allowance" if f.group.is_none() => return FormTab::Finishing,
        key if key == AFTER_TOOL => return FormTab::Where,
        _ => {}
    }
    if f.group.as_deref() == Some(cmd_surfacing_toolpath::AREA_GROUP) || in_group(&CUTTING_GROUPS) {
        FormTab::Cutting
    } else if in_group(&WHERE_GROUPS) {
        FormTab::Where
    } else if f.tab.is_some() || in_group(&MOVEMENT_GROUPS) {
        // Profile's Leads, Order, Start At and Corners: blocks of their own on Movement
        FormTab::Movement
    } else if in_group(&FINISHING_GROUPS) {
        FormTab::Finishing
    } else {
        FormTab::Own
    }
}

/// Whether a form is split into tabs: a toolpath's, which picks its bit.
pub fn tabbed(fields: &[Field]) -> bool {
    super::picks_bits(fields)
}

/// The tabs `fields` fill, in order, Defaults always last.
pub fn tabs_of(fields: &[Field]) -> Vec<FormTab> {
    ALL.into_iter()
        .filter(|&t| t == FormTab::Defaults || fields.iter().any(|f| tab_of(f) == t))
        .collect()
}

/// The fields on `tab`, in the form's order.
pub fn on_tab(fields: &[Field], tab: FormTab) -> Vec<&Field> {
    fields.iter().filter(|f| tab_of(f) == tab).collect()
}

/// Whether a field's value is not the one a new form opens with. The bits are left out:
/// a form always has one picked, and it is not a setting of the form's own.
fn changed(f: &Field, params: &Value, defaults: &Value) -> bool {
    !super::drawn_by_tool(&f.key) && f.key != "name" && params.get(&f.key) != defaults.get(&f.key)
}

/// The tabs holding a setting changed from what a new form opens with (`defaults`): each
/// gets a blue dot.
pub fn changed_tabs(fields: &[Field], params: &Value, defaults: &Value) -> Vec<FormTab> {
    let mut tabs: Vec<FormTab> = fields
        .iter()
        .filter(|f| changed(f, params, defaults))
        .map(tab_of)
        .collect();
    tabs.sort();
    tabs.dedup();
    tabs
}

/// The keys of the settings changed from what a new form opens with: each gets a blue bar.
pub fn changed_keys(fields: &[Field], params: &Value, defaults: &Value) -> Vec<String> {
    fields
        .iter()
        .filter(|f| changed(f, params, defaults))
        .map(|f| f.key.clone())
        .collect()
}

/// The tabs a warning or error in `said` is about: those holding a setting, or a block,
/// it names, and the bit's when `about_bit` (a drill on a toolpath that cuts sideways, a
/// router bit on a laser's). Each gets a red dot.
pub fn warned_tabs(fields: &[Field], said: &[&str], about_bit: bool) -> Vec<FormTab> {
    let said: Vec<String> = said.iter().map(|s| s.to_lowercase()).collect();
    let names = |text: &str| -> bool {
        let text = text.trim().to_lowercase();
        // a word of two letters ("D") would be found everywhere
        text.chars().count() > 2 && said.iter().any(|s| names_whole(s, &text))
    };
    let mut tabs: Vec<FormTab> = fields
        .iter()
        .filter(|f| {
            names(&f.label)
                || f.group
                    .as_deref()
                    .is_some_and(|g| names(&super::setting_text(g)))
                || f.tab
                    .as_deref()
                    .is_some_and(|t| names(&super::setting_text(t)))
        })
        .map(tab_of)
        .collect();
    if about_bit {
        tabs.push(FormTab::Cutting);
    }
    tabs.sort();
    tabs.dedup();
    tabs
}

/// Whether `said` has `words` in it as whole words: "cut" is not in "cuts".
fn names_whole(said: &str, words: &str) -> bool {
    let edge = |c: Option<char>| c.is_none_or(|c| !c.is_alphanumeric());
    said.match_indices(words).any(|(at, _)| {
        edge(said[..at].chars().next_back()) && edge(said[at + words.len()..].chars().next())
    })
}

/// The tab holding the setting a help tour points at (`field:<key>`), for the form to turn
/// to it.
pub fn tab_of_mark(fields: &[Field], mark: &str) -> Option<FormTab> {
    let key = mark.strip_prefix("field:")?;
    fields.iter().find(|f| f.key == key).map(tab_of)
}

/// A tab's name: the toolpath's own tab is named for the tool (`own`).
pub fn name(tab: FormTab, own: &str) -> String {
    match tab {
        FormTab::Cutting => tr!("form-tab-cutting"),
        FormTab::Own => own.to_owned(),
        FormTab::Where => tr!("form-tab-where"),
        FormTab::Movement => tr!("form-tab-movement"),
        FormTab::Finishing => tr!("form-tab-finishing"),
        FormTab::Defaults => tr!("form-tab-defaults"),
    }
}

/// The name of toolpath `id`'s own tab, short (Pocket, Profile, V-Carve); `fallback`, the
/// form's name, for a tool with none (an add-on's).
pub fn own_name(id: &str, fallback: &str) -> String {
    cut_craft_i18n::tr_or(&own_key(id), fallback)
}

/// The key of toolpath `id`'s own tab name: `form-tab-own-` and its id, hyphenated.
pub fn own_key(id: &str) -> String {
    cut_craft_i18n::key_of("form-tab-own", id)
}

/// What a tab holds, on hover.
pub fn hint(tab: FormTab) -> String {
    match tab {
        FormTab::Cutting => tr!("form-tab-cutting.tooltip"),
        FormTab::Own => tr!("form-tab-own-hint"),
        FormTab::Where => tr!("form-tab-where.tooltip"),
        FormTab::Movement => tr!("form-tab-movement.tooltip"),
        FormTab::Finishing => tr!("form-tab-finishing.tooltip"),
        FormTab::Defaults => tr!("form-tab-defaults.tooltip"),
    }
}

/// How a tab is marked: a red dot for a warning on it, else a blue one for a changed
/// setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dot {
    Warned,
    Changed,
}

/// The strip of tabs across a form's top, `picked` the one shown; a dot on each tab
/// `dots` names. Whether another tab was picked.
pub fn strip(
    ui: &mut egui::Ui,
    tabs: &[FormTab],
    picked: &mut FormTab,
    own: &str,
    dots: &dyn Fn(FormTab) -> Option<Dot>,
) -> bool {
    let before = *picked;
    ui.horizontal_wrapped(|ui| {
        for &tab in tabs {
            let response = ui
                .selectable_label(*picked == tab, name(tab, own))
                .on_hover_hint(hint(tab));
            crate::marks::note(ui, &format!("form-tab:{tab:?}"), response.rect);
            if let Some(dot) = dots(tab) {
                let colour = match dot {
                    Dot::Warned => ui.visuals().error_fg_color,
                    Dot::Changed => changed_colour(ui.visuals().dark_mode),
                };
                let r = ui.spacing().interact_size.y * 0.12;
                let at = response.rect.right_top() + egui::vec2(-r, r);
                ui.painter().circle_filled(at, r, colour);
            }
            if response.clicked() {
                *picked = tab;
            }
        }
    });
    ui.separator();
    *picked != before
}

/// The blue of a changed setting's bar and its tab's dot: the selection blue, which reads
/// on both themes.
pub fn changed_colour(dark: bool) -> egui::Color32 {
    if dark {
        egui::Color32::from_rgb(0x5a, 0x9c, 0xf0)
    } else {
        egui::Color32::from_rgb(0x2a, 0x7b, 0xe4)
    }
}

/// The keys of the settings changed from a new form's, as [`set_changed`] last kept them:
/// the grid draws a blue bar beside each.
pub(super) fn changed_now(ctx: &egui::Context) -> Vec<String> {
    ctx.data(|d| d.get_temp::<Vec<String>>(changed_id()))
        .unwrap_or_default()
}

/// Keeps which settings are changed from a new form's, for this frame's grid
/// ([`changed_now`]); empty for a form that marks none.
pub fn set_changed(ctx: &egui::Context, keys: Vec<String>) {
    ctx.data_mut(|d| d.insert_temp(changed_id(), keys));
}

fn changed_id() -> egui::Id {
    egui::Id::unique("form_changed_settings")
}

#[cfg(test)]
mod tests;
