//! The Canvas window layout: the whole window is the drawing. A thin header holds the
//! tabs and a Find a Tool box; the tools, the open tool's form and the job's lists float
//! over the view (`docs/superpowers/specs/2026-11-24-shared-tool-model-design.md`).

use super::installed::GADGETS;
use super::{CutCraftApp, INSPECTOR_TABS, RIBBON_TABS, TOOL_ICON_ROWS, egui, gui};
use crate::options::WindowLayout;
use crate::tool_names::{group_name, page_tab_name};
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_i18n::tr;

/// How much of the view's height the floating form may take.
const FORM_SHARE: f32 = 0.8;
/// How much of the view's height an open list (Sheets, Layers, Toolpaths…) may take.
const LIST_SHARE: f32 = 0.6;

/// What the Canvas layout has open: the palette's group (by name, so it stays the same
/// group when another machine offers other groups) whose tools fly out beside it, and the
/// chip (an index into `INSPECTOR_TABS`) whose list floats above the chips.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct CanvasState {
    pub group: Option<&'static str>,
    pub chip: Option<usize>,
}

/// A floating box's frame: a window's look, without its title bar.
fn floating(ui: &egui::Ui) -> egui::Frame {
    egui::Frame::window(ui.style()).inner_margin(ui.spacing().item_spacing.y)
}

impl CutCraftApp {
    /// Canvas: a thin header (tabs and Find a Tool), then floating boxes over the view: the
    /// palette of tool groups on the left with its flyout, the open form on the right, and
    /// chips along the foot that open Sheets, Layers, Toolpaths and the rest.
    pub(super) fn canvas_layout(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("canvas_header").show(ui, |ui| self.canvas_header(ui));
        // the room the view has, under the header and over the status bar
        let room = ui.available_rect_before_wrap();
        let row = ui.spacing().interact_size.y;
        let margin = ui.spacing().item_spacing.y;
        // under the view's own bar (2D / 3D, the split, sheet and layer, snapping)
        let top = room.top() + 2.0 * row + 3.0 * margin;
        let tab = RIBBON_TABS[self.ribbon_tab];
        if tab == "Favourites" || tab == GADGETS {
            self.canvas.group = None;
            let left = egui::pos2(room.left() + margin, top);
            self.canvas_flyout(ui, left, tab, &self.canvas_list(tab));
        } else {
            self.canvas_palette(ui, egui::pos2(room.left() + margin, top));
        }
        if self.form.is_some() {
            let width = super::DESIGN_PANEL_WIDTHS * ui.spacing().interact_size.x;
            let height = FORM_SHARE * (room.bottom() - top);
            egui::Area::new(egui::Id::unique("canvas_form"))
                .pivot(egui::Align2::RIGHT_TOP)
                .default_pos(egui::pos2(room.right() - margin, top))
                .show(ui.ctx(), |ui| {
                    floating(ui).show(ui, |ui| {
                        cut_craft_bug_report::mark(ui, "Canvas form");
                        ui.set_width(width);
                        ui.set_height(height);
                        self.form_panel(ui);
                    });
                });
        }
        self.canvas_chips(ui, room);
    }

    /// The header: the tabs, then on the right the Find a Tool box.
    fn canvas_header(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "Canvas header");
        let gadgets = self.gadget_tools();
        if gadgets.is_empty() && RIBBON_TABS[self.ribbon_tab] == GADGETS {
            self.ribbon_tab = 0;
        }
        ui.horizontal(|ui| {
            for (i, tab) in RIBBON_TABS.iter().enumerate() {
                if (*tab != GADGETS || !gadgets.is_empty())
                    && ui
                        .selectable_value(&mut self.ribbon_tab, i, page_tab_name(tab))
                        .clicked()
                {
                    self.canvas.group = None;
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                self.canvas_search(ui);
            });
        });
    }

    /// The header's Find a Tool box (Ctrl+K puts the keyboard in it): the tools whose names
    /// hold what is typed drop down under it; a click runs one, Enter the first, Escape
    /// clears it.
    fn canvas_search(&mut self, ui: &mut egui::Ui) {
        let row = ui.spacing().interact_size.y;
        let mut text = self.search.clone().unwrap_or_default();
        let edit = ui.add(
            egui::TextEdit::singleline(&mut text)
                .hint_text(tr!("layouts-canvas-find"))
                .desired_width(14.0 * row),
        );
        // Ctrl+K opened it empty: the keyboard goes to the box
        if self.search.as_deref() == Some("") && !edit.has_focus() && !edit.lost_focus() {
            edit.request_focus();
        }
        if edit.changed() || edit.gained_focus() {
            self.search = Some(text.clone());
        }
        if edit.lost_focus() && text.is_empty() {
            self.search = None;
        }
        if text.trim().is_empty() || self.search.is_none() {
            return;
        }
        let matches = self.search_matches(&text);
        if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            if let Some(&(id, _)) = matches.first() {
                self.search = None;
                self.invoke(ui.ctx(), id);
            }
            return;
        }
        let below = edit.rect.left_bottom();
        let ids: Vec<&'static str> = matches.iter().map(|&(id, _)| id).collect();
        egui::Area::new(egui::Id::unique("canvas_search_results"))
            .order(egui::Order::Foreground)
            .fixed_pos(below)
            .show(ui.ctx(), |ui| {
                floating(ui).show(ui, |ui| {
                    cut_craft_bug_report::mark(ui, "Canvas find results");
                    ui.set_width(edit.rect.width());
                    if ids.is_empty() {
                        ui.weak(tr!("layouts-no-tool-has-that-in"));
                    }
                    let picked = egui::ScrollArea::vertical()
                        .max_height(10.0 * row)
                        .show(ui, |ui| self.tool_rows(ui, &ids))
                        .inner;
                    if picked {
                        self.search = None;
                    }
                });
            });
    }

    /// Brings the browser's `tab` (one of `INSPECTOR_TABS`) forward: in the Rail and Ribbon
    /// layouts' browser, and in Canvas as that chip's open list.
    pub(super) fn show_inspector_tab(&mut self, tab: &str) {
        if let Some(i) = INSPECTOR_TABS.iter().position(|&t| t == tab) {
            self.inspector_tab = i;
            if self.options.general.layout == WindowLayout::Canvas {
                self.canvas.chip = Some(i);
            }
        }
    }

    /// The groups of the header's tab with a tool for this machine.
    pub(super) fn canvas_groups(&self) -> Vec<gui::ToolGroup> {
        self.ribbon_groups()
            .0
            .into_iter()
            .filter(|group| group.tools.iter().any(|id| self.has_button(id)))
            .collect()
    }

    /// Favourites' or Gadgets' tools for this machine, listed with no palette. Favourites keep
    /// another way to draw a shape picked for them, as in every layout.
    fn canvas_list(&self, tab: &str) -> Vec<&'static str> {
        let tools = if tab == GADGETS {
            self.gadget_tools()
        } else {
            self.tool_model().every_favourite(&self.options.favourites)
        };
        tools.into_iter().filter(|id| self.offered(id)).collect()
    }

    /// The palette: one icon for each group, top to bottom; a click lists that group's
    /// tools beside it, another click closes the list.
    fn canvas_palette(&mut self, ui: &mut egui::Ui, at: egui::Pos2) {
        let groups = self.canvas_groups();
        let row = ui.spacing().interact_size.y;
        let edge = TOOL_ICON_ROWS * row;
        // down to just above the chips at the foot
        let room = (ui.ctx().content_rect().bottom() - at.y - 3.0 * row).max(edge);
        let shown = egui::Area::new(egui::Id::unique("canvas_palette"))
            .default_pos(at)
            .show(ui.ctx(), |ui| {
                floating(ui).show(ui, |ui| {
                    cut_craft_bug_report::mark(ui, "Canvas palette");
                    // a short window scrolls the palette rather than losing its foot
                    egui::ScrollArea::vertical()
                        .max_height(room)
                        .show(ui, |ui| {
                            for group in &groups {
                                let name = group_name(group.name);
                                let icon = group
                                    .tools
                                    .iter()
                                    .find_map(|&id| self.icons.find(self.explicit_icon(id), id));
                                let button = match icon {
                                    Some(path) => egui::Button::image(
                                        self.icons
                                            .image(path)
                                            .fit_to_exact_size(egui::vec2(edge, edge)),
                                    ),
                                    None => egui::Button::new(name.as_str())
                                        .wrap_mode(egui::TextWrapMode::Truncate),
                                }
                                .min_size(egui::vec2(edge, edge))
                                .selected(self.canvas.group == Some(group.name));
                                if ui.add(button).on_hover_hint(&name).clicked() {
                                    self.canvas.group = (self.canvas.group != Some(group.name))
                                        .then_some(group.name);
                                }
                            }
                        });
                });
            });
        let Some(group) = self
            .canvas
            .group
            .and_then(|name| groups.iter().find(|g| g.name == name))
        else {
            return;
        };
        let tools: Vec<&'static str> = group
            .tools
            .iter()
            .copied()
            .filter(|id| self.has_button(id))
            .collect();
        let beside = shown.response.rect.right_top() + egui::vec2(ui.spacing().item_spacing.x, 0.0);
        self.canvas_flyout(ui, beside, group.name, &tools);
    }

    /// A list of tools by icon and name, floating at `at` under its name, with the pointed-at
    /// tool's guidance under it; picking one closes the list.
    fn canvas_flyout(
        &mut self,
        ui: &mut egui::Ui,
        at: egui::Pos2,
        name: &str,
        tools: &[&'static str],
    ) {
        let row = ui.spacing().interact_size.y;
        let width = super::DESIGN_PANEL_WIDTHS * ui.spacing().interact_size.x * 0.6;
        egui::Area::new(egui::Id::unique("canvas_flyout"))
            .fixed_pos(at)
            .show(ui.ctx(), |ui| {
                floating(ui).show(ui, |ui| {
                    cut_craft_bug_report::mark(ui, "Canvas flyout");
                    ui.set_width(width);
                    ui.strong(group_name(name));
                    ui.separator();
                    if tools.is_empty() {
                        ui.weak(super::layouts::NO_FAVOURITES);
                    }
                    let picked = egui::ScrollArea::vertical()
                        .max_height(16.0 * row)
                        .show(ui, |ui| self.tool_rows(ui, tools))
                        .inner;
                    ui.add_space(ui.spacing().item_spacing.y);
                    self.tip_contents(ui);
                    if picked {
                        self.canvas.group = None;
                    }
                });
            });
    }

    /// The chips along the view's foot, one for each of the job's lists; a click opens that
    /// list floating above them, another click closes it.
    fn canvas_chips(&mut self, ui: &mut egui::Ui, room: egui::Rect) {
        let margin = ui.spacing().item_spacing.y;
        let foot = room.left_bottom() + egui::vec2(margin, -margin);
        let chips = egui::Area::new(egui::Id::unique("canvas_chips"))
            .pivot(egui::Align2::LEFT_BOTTOM)
            .fixed_pos(foot)
            .show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    cut_craft_bug_report::mark(ui, "Canvas chips");
                    for (i, tab) in INSPECTOR_TABS.iter().enumerate() {
                        let open = self.canvas.chip == Some(i);
                        let chip = egui::Button::new(page_tab_name(tab))
                            .corner_radius(ui.spacing().interact_size.y / 2.0)
                            .selected(open);
                        if ui
                            .add(chip)
                            .on_hover_hint(tr!("layouts-canvas-chip", list = page_tab_name(tab)))
                            .clicked()
                        {
                            self.canvas.chip = (!open).then_some(i);
                        }
                    }
                });
            });
        let Some(i) = self.canvas.chip else {
            return;
        };
        let width = super::DESIGN_PANEL_WIDTHS * ui.spacing().interact_size.x;
        let height = LIST_SHARE * room.height();
        let above = chips.response.rect.left_top() - egui::vec2(0.0, margin);
        egui::Area::new(egui::Id::unique("canvas_list"))
            .pivot(egui::Align2::LEFT_BOTTOM)
            .fixed_pos(above)
            .show(ui.ctx(), |ui| {
                floating(ui).show(ui, |ui| {
                    cut_craft_bug_report::mark(ui, "Canvas list");
                    ui.set_width(width);
                    // as tall as its list, up to its share; the toolpath list fills it
                    if INSPECTOR_TABS[i] == "Toolpaths" {
                        ui.set_height(height);
                    } else {
                        ui.set_max_height(height);
                    }
                    self.inspector_tab = i;
                    self.browser_list(ui, INSPECTOR_TABS[i]);
                });
            });
    }
}
