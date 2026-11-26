//! A shape drawn more than one way has one button: Draw Rectangle draws from a corner or
//! from the center, Draw Circle from the center or across from an edge. **Draw from** at the
//! top of the shape's form picks the way, and the shape's button opens the way picked last
//! (Matthew, 2026-11-24). Options ▸ General ▸ *Other ways to draw a shape* set to *As
//! buttons* gives each way its button back, as before. Find Tool, menus, favourites and help name
//! every way either way, so none is lost ([`crate::flyout::WAYS`]).

use super::{CutCraftApp, HoverHint, egui};
use crate::flyout;
use cut_craft_i18n::tr;

impl CutCraftApp {
    /// Whether tool `id` gets a button on the tool pages, the Rail's lists and the Ribbon:
    /// offered on this machine, and not another way to draw a shape that its form offers.
    pub(super) fn has_button(&self, id: &str) -> bool {
        self.offered(id) && flyout::has_button(id, self.options.general.other_ways)
    }

    /// Whether the ways are picked in the shape's form rather than by buttons of their own.
    fn ways_in_form(&self) -> bool {
        self.options.general.other_ways == crate::options::OtherWays::InTheForm
    }

    /// Shape `id`'s button: its form, drawing the way picked last in it (the usual way
    /// when each way has its own button, which says which it is).
    pub(super) fn start_shape(&mut self, id: &str) {
        let last = self
            .options
            .shape_ways
            .get(id)
            .filter(|_| self.ways_in_form())
            .and_then(|way| flyout::variant(way))
            .filter(|v| v.command == id);
        match last {
            Some(variant) => self.start_variant(variant.id),
            None => self.start_command(id),
        }
    }

    /// Draws the open form's shape way `id` from now on ([`flyout::ways_of`]): its drag is
    /// read that way, the form and status bar name it, and the shape's button opens it next
    /// time.
    pub(super) fn pick_way(&mut self, id: &str) {
        let Some(form) = self.form.as_mut() else {
            return;
        };
        let Some(way) = flyout::ways_of(form.id).iter().find(|w| w.id == id) else {
            return;
        };
        form.way = way.drag;
        form.label = match flyout::variant(id) {
            Some(variant) => variant.label,
            None => self
                .registry
                .get(form.id)
                .map_or(form.label, |c| c.meta().label),
        };
        let command = form.id.to_owned();
        self.status.current_tool = self.tool_label(id);
        self.remember_way(&command, id);
    }

    /// Keeps way `id` as the way shape `command` was drawn last.
    pub(super) fn remember_way(&mut self, command: &str, id: &str) {
        if self.options.shape_ways.get(command).map(String::as_str) != Some(id) {
            self.options
                .shape_ways
                .insert(command.to_owned(), id.to_owned());
            self.save_options();
        }
    }

    /// **Draw from** under the open form's heading, for a shape drawn more than one way:
    /// each way by name, the one in use picked. A click draws the shape that way.
    pub(super) fn way_picker(&mut self, ui: &mut egui::Ui) {
        let Some(form) = &self.form else { return };
        let ways = flyout::ways_of(form.id);
        if ways.is_empty() {
            return;
        }
        let current = form.way;
        let mut picked = None;
        ui.horizontal(|ui| {
            ui.label(tr!("form-way-caption"));
            for way in ways {
                let response = ui
                    .selectable_label(way.drag == current, tr!(way.name))
                    .on_hover_hint(tr!(way.hint));
                crate::marks::note(ui, &format!("way:{}", way.id), response.rect);
                if response.clicked() {
                    picked = Some(way.id);
                }
            }
        });
        if let Some(id) = picked {
            self.pick_way(id);
        }
    }

    /// How the open form's drag is read when its shape is drawn another way (Draw Circle
    /// from Edge), for the status bar and the form's Next line.
    pub(super) fn way_step(&self) -> Option<String> {
        let form = self
            .form
            .as_ref()
            .filter(|f| f.way != flyout::DragWay::AsIs)?;
        let variant = flyout::VARIANTS
            .iter()
            .find(|v| v.command == form.id && v.way == form.way)?;
        let edit = t!("; Shift-click one this tool drew to edit it");
        Some(format!("{}{edit}", variant.hint))
    }

    /// The line a shape's button adds to its tooltip while its other ways are in its form:
    /// which ways, and where to pick them.
    pub(super) fn ways_note(&self, id: &str) -> Option<String> {
        let ways = flyout::ways_of(id);
        if ways.is_empty() || !self.ways_in_form() {
            return None;
        }
        let names: Vec<String> = ways.iter().map(|w| tr!(w.name)).collect();
        Some(tr!("form-way-tooltip", ways = names.join(" / ")))
    }

    /// Whether tool `id`'s button shows as in hand: its tool is, or another way to draw its
    /// shape that has no button of its own.
    pub(super) fn tool_in_hand(&self, id: &str) -> bool {
        let current = &self.status.current_tool;
        *current == self.tool_label(id)
            || flyout::ways_of(id).iter().any(|w| {
                !flyout::has_button(w.id, self.options.general.other_ways)
                    && *current == self.tool_label(w.id)
            })
    }
}
