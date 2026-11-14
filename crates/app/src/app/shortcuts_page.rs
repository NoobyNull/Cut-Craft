//! Options ▸ Mouse and Shortcuts: setting any action's shortcut to a key or to the mouse's
//! wheel click and thumb buttons, and which buttons pan the views when held and dragged.

use super::{CutCraftApp, ShortcutCapture};
use crate::shortcut::{self, Captured, Trigger};
use eframe::egui;

/// The key of the Options category this page draws under.
pub(super) const CATEGORY: &str = "controls";

/// One action's row in the list.
pub(super) struct Row {
    pub id: String,
    pub label: String,
    pub shortcuts: Vec<Trigger>,
    /// Its shortcuts are set in Options rather than Cut Craft's own.
    pub changed: bool,
}

/// What the person did in the list, done once the window is drawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Edit {
    /// Add: wait for a key or button for this action.
    Start(String),
    /// Stop waiting, changing nothing.
    Cancel,
    Remove(String, Trigger),
    /// Back to Cut Craft's own shortcuts for this action.
    Reset(String),
    /// Take the clashing shortcut from the other action and give it to this one.
    Move,
    /// Leave the clashing shortcut where it is, and wait for another.
    PickAnother,
}

impl CutCraftApp {
    /// While a shortcut is being set, takes the first key or mouse button pressed for it,
    /// before anything else sees it; any other keys are kept from running anything.
    pub(super) fn capture_shortcut(&mut self, ctx: &egui::Context) {
        let Some(capture) = &self.capture else {
            return;
        };
        let events = ctx.input(|i| i.events.clone());
        // a clash waits on the person's answer, on the page's buttons
        let got = if capture.clash.is_some() {
            None
        } else {
            shortcut::capture(&events)
        };
        let id = capture.id.clone();
        match got {
            None => {}
            Some(Captured::Cancel) => self.capture = None,
            Some(Captured::Refused(why)) => self.capture_refused(why),
            Some(Captured::Trigger(t)) if id == shortcut::PAN_DRAG => match t {
                // a button held while dragging: the keys held with it don't matter
                Trigger::Mouse(_, b) => {
                    self.offer(ctx, &id, Trigger::Mouse(egui::Modifiers::NONE, b));
                }
                Trigger::Key(_) => self.capture_refused(PAN_NEEDS_A_BUTTON),
            },
            Some(Captured::Trigger(t)) => self.offer(ctx, &id, t),
        }
        ctx.input_mut(|i| {
            i.events.retain(|e| {
                !matches!(
                    e,
                    egui::Event::Key { .. }
                        | egui::Event::Text(_)
                        | egui::Event::Copy
                        | egui::Event::Cut
                        | egui::Event::Paste(_)
                )
            });
        });
    }

    fn capture_refused(&mut self, why: &'static str) {
        if let Some(c) = &mut self.capture {
            c.refused = Some(why);
        }
    }

    /// `t` pressed for `id`: set it, unless another action has it, which is asked about first.
    fn offer(&mut self, ctx: &egui::Context, id: &str, t: Trigger) {
        let bindings = self.bindings(ctx);
        if let Some(other) = shortcut::clash(&bindings, t, id) {
            let other = other.to_owned();
            if let Some(c) = &mut self.capture {
                c.clash = Some((t, other));
                c.refused = None;
            }
        } else {
            self.add_shortcut(id, t);
            self.capture = None;
        }
    }

    /// `id`'s shortcuts as they are now: those set in Options, else its own.
    pub(super) fn shortcuts_of(&self, id: &str) -> Vec<Trigger> {
        shortcut::effective(self.default_bindings(), &self.options.controls.shortcuts)
            .into_iter()
            .filter(|(a, _)| a == id)
            .map(|(_, t)| t)
            .collect()
    }

    /// Sets `id`'s shortcuts, kept in Options; Cut Craft's own set is kept as no change.
    fn set_shortcuts(&mut self, id: &str, set: &[Trigger]) {
        let own: Vec<Trigger> = (self.default_bindings().into_iter())
            .filter(|(a, _)| a == id)
            .map(|(_, t)| t)
            .collect();
        let shortcuts = &mut self.options.controls.shortcuts;
        if set == own.as_slice() {
            shortcuts.remove(id);
        } else {
            shortcuts.insert(id.to_owned(), set.iter().map(|t| t.text()).collect());
        }
        self.save_options();
    }

    pub(super) fn add_shortcut(&mut self, id: &str, t: Trigger) {
        let mut set = self.shortcuts_of(id);
        if !set.contains(&t) {
            set.push(t);
        }
        self.set_shortcuts(id, &set);
    }

    pub(super) fn remove_shortcut(&mut self, id: &str, t: Trigger) {
        let mut set = self.shortcuts_of(id);
        set.retain(|s| *s != t);
        self.set_shortcuts(id, &set);
    }

    /// Every action's row, narrowed to those whose name or shortcut has `search` in it;
    /// panning first, then by name.
    pub(super) fn shortcut_rows(&self, search: &str) -> Vec<Row> {
        let all = shortcut::effective(self.default_bindings(), &self.options.controls.shortcuts);
        let search = search.trim().to_lowercase();
        self.shortcut_actions()
            .into_iter()
            .map(|(id, label)| {
                let shortcuts: Vec<Trigger> = (all.iter())
                    .filter(|(a, _)| *a == id)
                    .map(|(_, t)| *t)
                    .collect();
                let changed = self.options.controls.shortcuts.contains_key(&id);
                Row {
                    id,
                    label,
                    shortcuts,
                    changed,
                }
            })
            .filter(|r| {
                search.is_empty()
                    || starts_a_word(&r.label, &search)
                    || (r.shortcuts.iter()).any(|t| t.text().to_lowercase().starts_with(&search))
            })
            .collect()
    }

    /// Does what the person did in the list.
    pub(super) fn apply_shortcut_edit(&mut self, edit: Edit) {
        match edit {
            Edit::Start(id) => {
                self.capture = Some(ShortcutCapture {
                    id,
                    refused: None,
                    clash: None,
                });
            }
            Edit::Cancel => self.capture = None,
            Edit::Remove(id, t) => self.remove_shortcut(&id, t),
            Edit::Reset(id) => {
                self.options.controls.shortcuts.remove(&id);
                self.save_options();
            }
            Edit::Move => {
                if let Some(ShortcutCapture {
                    id,
                    clash: Some((t, other)),
                    ..
                }) = self.capture.take()
                {
                    self.remove_shortcut(&other, t);
                    self.add_shortcut(&id, t);
                }
            }
            Edit::PickAnother => {
                if let Some(c) = &mut self.capture {
                    c.clash = None;
                }
            }
        }
    }
}

/// Whether `search` (lower case) starts a word of `label`: "pan" finds Pan, not Convert
/// Span.
fn starts_a_word(label: &str, search: &str) -> bool {
    format!(" {}", label.to_lowercase()).contains(&format!(" {search}"))
}

/// Why a key can't pan.
const PAN_NEEDS_A_BUTTON: &str = "Panning is done by holding a mouse button and dragging, so \
    it needs a button: press the wheel, or a thumb button.";

/// What the list says while it waits for a key or button.
const WAITING: &str = "Press a key (with Ctrl, Alt or Shift if you like) or a mouse button: \
    the wheel, or a thumb button. Esc stops.";

/// The list of shortcuts: a search box, then each action with its shortcuts, an Add button
/// and a Reset button where it was changed. `labels` names actions by id, for a clash.
pub(super) fn show(
    ui: &mut egui::Ui,
    rows: &[Row],
    capture: Option<&ShortcutCapture>,
    search: &mut String,
    label_of: &dyn Fn(&str) -> String,
) -> Option<Edit> {
    let mut edit = None;
    ui.add_space(ui.spacing().item_spacing.y);
    ui.strong("Shortcuts");
    ui.label(
        "Press Add beside an action, then the key or mouse button to run it with. The \
         thumb buttons (Back and Forward) and the wheel click can be used as well as keys. \
         Press × on a shortcut to take it away.",
    );
    ui.horizontal(|ui| {
        ui.label("Find");
        ui.add(
            egui::TextEdit::singleline(search)
                .hint_text("an action or a shortcut, such as Pan or Ctrl+Z"),
        );
    });
    // each action's name in a column of its own, its shortcuts and buttons beside it, going
    // on to another line there when they don't fit rather than breaking a shortcut's name
    let name_width = ui
        .available_width()
        .min(LABEL_WIDTH * ui.spacing().interact_size.x);
    for (n, row) in rows.iter().enumerate() {
        let frame = if n % 2 == 0 {
            egui::Frame::new().fill(ui.visuals().faint_bg_color)
        } else {
            egui::Frame::new()
        };
        frame.show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let height = ui.spacing().interact_size.y;
                ui.allocate_ui_with_layout(
                    egui::vec2(name_width, height),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.set_min_width(name_width);
                        ui.add(egui::Label::new(&row.label).truncate());
                    },
                );
                ui.horizontal_wrapped(|ui| {
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                    if let Some(e) = row_buttons(ui, row, capture) {
                        edit = Some(e);
                    }
                });
            });
            if let Some(c) = capture.filter(|c| c.id == row.id) {
                waiting_note(ui, c, label_of, &mut edit);
            }
        });
    }
    edit
}

/// A row's shortcuts, each with × to take it away, then Add… (Cancel while waiting) and
/// Reset where it was changed.
fn row_buttons(ui: &mut egui::Ui, row: &Row, capture: Option<&ShortcutCapture>) -> Option<Edit> {
    let mut edit = None;
    if row.shortcuts.is_empty() {
        ui.weak("none");
    }
    for t in &row.shortcuts {
        if ui
            .button(format!("{}  ×", t.text()))
            .on_hover_text("Take this shortcut away")
            .clicked()
        {
            edit = Some(Edit::Remove(row.id.clone(), *t));
        }
    }
    if capture.is_some_and(|c| c.id == row.id) {
        if ui.button("Cancel").clicked() {
            edit = Some(Edit::Cancel);
        }
    } else if ui
        .button("Add…")
        .on_hover_text("Then press the key or mouse button to use")
        .clicked()
    {
        edit = Some(Edit::Start(row.id.clone()));
    }
    if row.changed
        && ui
            .button("Reset")
            .on_hover_text("Back to Cut Craft's own shortcut for this")
            .clicked()
    {
        edit = Some(Edit::Reset(row.id.clone()));
    }
    edit
}

/// How wide the actions' names are, in the theme's control widths.
const LABEL_WIDTH: f32 = 4.0;

/// Under the row waiting for a shortcut: what to press, or why the last press was not
/// taken, or which action already has it and what to do about that.
fn waiting_note(
    ui: &mut egui::Ui,
    c: &ShortcutCapture,
    label_of: &dyn Fn(&str) -> String,
    edit: &mut Option<Edit>,
) {
    if let Some((t, other)) = &c.clash {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            format!(
                "⚠ {} already runs {}. Move it here, so it runs this instead?",
                t.text(),
                label_of(other)
            ),
        );
        ui.horizontal(|ui| {
            if ui.button("Move it here").clicked() {
                *edit = Some(Edit::Move);
            }
            if ui.button("Pick another").clicked() {
                *edit = Some(Edit::PickAnother);
            }
        });
        return;
    }
    if let Some(why) = c.refused {
        ui.colored_label(ui.visuals().warn_fg_color, why);
    } else {
        ui.colored_label(ui.visuals().hyperlink_color, WAITING);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_search_finds_words_by_their_start() {
        assert!(starts_a_word("Pan (hold and drag)", "pan"));
        assert!(starts_a_word("Zoom to Drawing", "zoom to"));
        assert!(starts_a_word("Zoom to Drawing", "draw"));
        assert!(!starts_a_word("Convert Span to Arc", "pan"));
    }
}
