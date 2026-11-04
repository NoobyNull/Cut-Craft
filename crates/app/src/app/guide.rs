//! What to do next with a tool, in a beginner's words. The Design and Toolpaths pages show it
//! in the room left under their icons (the hovered tool's description and first step, else
//! the tool in hand's next step), the status bar carries the same line, each tooltip ends
//! with it, and a tool's form opens with it at the top. The first step comes from the tool
//! itself: run on an empty job, a tool that works on a selection says what to select
//! ("select the closed vectors to pocket"), so the words are the command's own and never go
//! stale as tools are added.

use super::{CutCraftApp, Layout, egui};
use cut_craft_core::document::Document;
use cut_craft_core::{CommandError, DynCommand};

/// A tool's guidance: its name, what it does, and what to do next (nothing for a tool that
/// just runs, such as Undo).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Tip {
    pub label: String,
    pub about: String,
    pub next: Option<String>,
}

impl Tip {
    /// The status bar's line for the tool: its name, then its next step, else what it does.
    pub(super) fn status(&self) -> String {
        match (&self.next, self.about.is_empty()) {
            (Some(next), _) => format!("{}: {next}", self.label),
            (None, false) => format!("{}: {}", self.label, self.about),
            (None, true) => self.label.clone(),
        }
    }
}

/// The first step for a tool that wants a selection but says only that nothing is selected.
pub(super) const SELECT_FIRST: &str =
    "Select what to work on: click a vector in the 2D view, Shift-click to add more";

/// The least room under the icons worth showing the tip box in, in rows: below it the status
/// bar alone carries the line.
const MIN_TIP_ROWS: f32 = 4.0;

/// Where the hovered tool's tip is kept: the tip and the pass that set it.
fn id() -> egui::Id {
    egui::Id::unique("hovered_tool_tip")
}

/// Sets this frame's hovered tool.
pub(super) fn show(ctx: &egui::Context, tip: &Tip) {
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| d.insert_temp(id(), (pass, tip.clone())));
}

/// A hovered tool button's guidance: the tip box under the icons shows `tip`, the status bar
/// its next step.
pub(super) fn hover(response: &egui::Response, tip: &Tip) {
    if response.hovered() {
        show(&response.ctx, tip);
        cut_craft_bug_report::hint::hint(&response.ctx, &tip.status());
    }
}

/// The tool hovered this frame or the last (the box may be drawn before the buttons).
pub(super) fn hovered(ctx: &egui::Context) -> Option<Tip> {
    let pass = ctx.cumulative_pass_nr();
    ctx.data(|d| d.get_temp::<(u64, Tip)>(id()))
        .filter(|(set, _)| pass.saturating_sub(*set) <= 1)
        .map(|(_, tip)| tip)
}

/// The first step a tool asks for, from what it says when run with nothing to work on: a
/// tool that says what to select ("select two rails, then one or more open cross-sections")
/// is quoted; one that says only that nothing is selected gets [`SELECT_FIRST`]; any other
/// complaint (a missing toolpath, a bad setting) is no step at all.
pub(super) fn step_from_error(error: &CommandError) -> Option<String> {
    match error {
        CommandError::NothingSelected => Some(SELECT_FIRST.to_owned()),
        CommandError::Failed(why) if why.to_lowercase().starts_with("select ") => {
            Some(capitalised(why))
        }
        CommandError::Failed(_) | CommandError::InvalidParams(_) => None,
    }
}

/// The first step `command` asks for on an empty job, if it works on a selection.
pub(super) fn first_step(command: &dyn DynCommand) -> Option<String> {
    command
        .run_json(&Document::default(), &command.default_params())
        .err()
        .and_then(|e| step_from_error(&e))
}

/// `text` with its first letter in upper case, as a sentence starts.
fn capitalised(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// `about` without the `label: ` a command's description starts with, when it does, as a
/// sentence of its own; nothing when it is only the label again.
fn body(label: &str, about: &str) -> String {
    if about == label {
        return String::new();
    }
    let rest = about
        .strip_prefix(label)
        .and_then(|rest| rest.strip_prefix(": "))
        .unwrap_or(about);
    capitalised(rest)
}

/// A tooltip's text: what the tool does, then its next step on a line of its own.
pub(super) fn tooltip(about: &str, next: Option<&str>) -> String {
    match next {
        Some(next) => format!("{about}\n\nNext: {next}"),
        None => about.to_owned(),
    }
}

impl CutCraftApp {
    /// The first step tool `id` asks for (see [`first_step`]), worked out once per tool.
    pub(super) fn first_step_of(&self, id: &str) -> Option<String> {
        if let Some(step) = self.first_steps.borrow().get(id) {
            return step.clone();
        }
        let step = self.registry.get(id).and_then(first_step);
        self.first_steps
            .borrow_mut()
            .insert(id.to_owned(), step.clone());
        step
    }

    /// A tool's button or row once drawn: its guidance on hover (the tip box, the status
    /// bar) and its tooltip, what it does then its next step. Every way a tool is offered
    /// (a page's icons, the Rail's rows, a flyout's list) goes through here.
    pub(super) fn with_guidance(&self, response: egui::Response, id: &str) -> egui::Response {
        let label = self.tool_label(id);
        let about = crate::gui::about(id).unwrap_or(&label);
        let tip = self.tool_tip(id, about);
        hover(&response, &tip);
        response.on_hover_text(tooltip(about, tip.next.as_deref()))
    }

    /// The guidance for tool `id` with description `about`: what it does and, for a tool
    /// that works on a selection, what to select; for a drawing tool, how to draw with it.
    pub(super) fn tool_tip(&self, id: &str, about: &str) -> Tip {
        let label = self.tool_label(id);
        let next = self
            .first_step_of(id)
            .or_else(|| self.drawing_step(id, true))
            .map(|next| body(&label, &next));
        let about = body(&label, about);
        Tip { label, about, next }
    }

    /// The tool in hand's guidance, for the box when no tool is hovered: its form's tool, or
    /// the view tool or selecting.
    pub(super) fn in_hand_tip(&self) -> Tip {
        let label = self.status.current_tool.clone();
        let about = self
            .form
            .as_ref()
            .and_then(|f| gui_about(f.id))
            .map_or_else(String::new, |about| body(&label, about));
        Tip {
            label,
            about,
            next: self.next_step(),
        }
    }

    /// The guidance a tool's form opens with: what to select while its tool has no
    /// selection, how to draw with a drawing tool; nothing for a form that just has settings.
    pub(super) fn form_guidance(&self) -> Option<String> {
        let form = self.form.as_ref()?;
        if self.doc.selection.is_empty()
            && let Some(step) = self.first_step_of(form.id)
        {
            return Some(step);
        }
        self.drawing_step(form.id, self.polyline.is_empty())
            .map(|step| body(form.label, &step))
    }

    /// The tip box under a page's icons, in whatever room is left: the hovered tool's tip,
    /// else the tool in hand's. Nothing when there is no room: the status bar has the line.
    pub(super) fn tip_box(&self, ui: &mut egui::Ui) {
        let row = ui.spacing().interact_size.y;
        if ui.available_height() < row * MIN_TIP_ROWS {
            return;
        }
        egui::ScrollArea::vertical()
            .id_salt("tool_tip_box")
            .auto_shrink([false, true])
            .min_scrolled_height(0.0)
            .max_height(ui.available_height())
            .show(ui, |ui| self.tip_contents(ui));
    }

    /// The tip box itself: the hovered tool's name, what it does and its next step, else the
    /// tool in hand's.
    pub(super) fn tip_contents(&self, ui: &mut egui::Ui) {
        let tip = hovered(ui.ctx()).unwrap_or_else(|| self.in_hand_tip());
        cut_craft_bug_report::mark(ui, "Tool tip box");
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.with_layout(Layout::top_down(super::Align::Min), |ui| {
                ui.strong(&tip.label);
                if !tip.about.is_empty() {
                    ui.label(&tip.about);
                }
                if let Some(next) = &tip.next {
                    ui.horizontal_wrapped(|ui| {
                        ui.strong("Next:");
                        ui.label(next);
                    });
                }
            });
        });
    }
}

/// See [`crate::gui::about`].
fn gui_about(id: &str) -> Option<&'static str> {
    crate::gui::about(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tool_that_says_what_to_select_is_quoted_as_the_first_step() {
        let step = step_from_error(&CommandError::Failed(
            "select the closed vectors to pocket".into(),
        ));
        assert_eq!(step.as_deref(), Some("Select the closed vectors to pocket"));
    }

    #[test]
    fn a_tool_that_only_says_nothing_is_selected_gets_the_general_step() {
        assert_eq!(
            step_from_error(&CommandError::NothingSelected).as_deref(),
            Some(SELECT_FIRST)
        );
    }

    #[test]
    fn other_complaints_are_no_step() {
        assert_eq!(
            step_from_error(&CommandError::Failed("there is no visible model".into())),
            None
        );
        assert_eq!(
            step_from_error(&CommandError::InvalidParams("choose an STL file".into())),
            None
        );
    }

    #[test]
    fn a_tooltip_ends_with_the_next_step_and_a_description_loses_its_label() {
        assert_eq!(
            tooltip("Group: the selected vectors become one", Some("Select two")),
            "Group: the selected vectors become one\n\nNext: Select two"
        );
        assert_eq!(tooltip("Undo", None), "Undo");
        assert_eq!(
            body("Group", "Group: the selected vectors"),
            "The selected vectors"
        );
        assert_eq!(
            body("Weld Vectors", "Weld Vectors"),
            "",
            "no description at all"
        );
        assert_eq!(body("Group", "Groups things"), "Groups things");
    }
}
