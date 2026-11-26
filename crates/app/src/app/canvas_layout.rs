//! The Canvas window layout: the whole window is the drawing. A thin header holds the
//! tabs and a Find a Tool box; the tools, the open tool's form and the job's lists float
//! over the view (`docs/superpowers/specs/2026-11-24-shared-tool-model-design.md`).

use super::bars::{Bar, Room};
use super::custom_toolbars::PALETTE;
use super::installed::GADGETS;
use super::{CutCraftApp, INSPECTOR_TABS, RIBBON_TABS, TOOL_ICON_ROWS, egui, gui};
use crate::bars::Place;
use crate::options::{CanvasFlyouts, WindowLayout};
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

/// The id of the Canvas palette's bar.
pub(super) fn palette_id() -> egui::Id {
    egui::Id::unique("canvas_palette")
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
        let row = ui.spacing().interact_size.y;
        let margin = ui.spacing().item_spacing.y;
        // the bars snapped to a side, the palette first, then the rest; the palette where
        // it starts, under the view's own bar (2D / 3D, the split, sheet and layer, snapping)
        let mut bars = Room::new(ui.available_rect_before_wrap());
        let first = ui.available_rect_before_wrap();
        let start = first.left_top() + egui::vec2(margin, 2.0 * row + 3.0 * margin);
        let tab = RIBBON_TABS[self.ribbon_tab];
        let palette = self.toolbars.ribbon.is_none() && tab != "Favourites" && tab != GADGETS;
        if palette && matches!(self.options.canvas_palette, Place::Snapped { .. }) {
            self.canvas_palette(ui, &mut bars, start);
        }
        self.placed_bars(ui, &mut bars, 4.0 * TOOL_ICON_ROWS * row);
        // the room the view has, under the header and the snapped bars, over the status bar
        let room = ui.available_rect_before_wrap();
        let top = room.top() + 2.0 * row + 3.0 * margin;
        if self.toolbars.ribbon.is_some() {
            self.canvas.group = None;
            let frame = floating(ui);
            self.canvas_toolbar(ui, egui::pos2(room.left() + margin, top), frame);
        } else if !palette {
            self.canvas.group = None;
            let left = egui::pos2(room.left() + margin, top);
            let _ = self.canvas_flyout(
                ui,
                left,
                egui::Align2::LEFT_TOP,
                tab,
                &self.canvas_list(tab),
            );
        } else if !matches!(self.options.canvas_palette, Place::Snapped { .. }) {
            // past any bar snapped to the left or top, which may have pushed the view over
            self.canvas_palette(ui, &mut bars, egui::pos2(room.left() + margin, top));
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
        ui.horizontal(|ui| {
            if self.tab_row(ui) {
                self.canvas.group = None;
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

    /// The palette: one icon for each group, down its column (or across, snapped to the
    /// top or the foot, or turned); a click lists that group's tools beside it, another
    /// click closes the list. A bar like any other: its grip moves it, snaps it, locks it
    /// or turns it (Options keep where).
    fn canvas_palette(&mut self, ui: &mut egui::Ui, bars: &mut Room, start: egui::Pos2) {
        let groups = self.canvas_groups();
        let row = ui.spacing().interact_size.y;
        let edge = TOOL_ICON_ROWS * row;
        let hover = self.options.general.canvas_flyouts == CanvasFlyouts::OnHover;
        // down to just above the chips at the foot; a window too short for every group
        // goes on to more columns
        let room = (ui.ctx().content_rect().bottom() - start.y - 3.0 * row).max(edge);
        let per_column = palette_rows(room, edge + ui.spacing().item_spacing.y);
        let place = self.options.canvas_palette;
        let menu = crate::bars::menu(
            &place,
            &format!("{}{PALETTE}", crate::custom_toolbars::ACTION),
            None,
        );
        let name = tr!("layouts-canvas-palette");
        let id = palette_id();
        let mut contents = |app: &mut Self, ui: &mut egui::Ui, across: bool| {
            app.palette_icons(ui, &groups, across, per_column, edge, hover);
        };
        let bar = Bar {
            id,
            name: &name,
            menu: &menu,
        };
        let shown = match place {
            Place::Snapped { side } => self.snapped_bar(ui, bars, &bar, side, &mut contents),
            Place::Floating(floating) => {
                self.floating_bar(ui.ctx(), &bar, floating, start, &mut contents)
            }
            // the palette has no tab of its own; one put there floats
            Place::Layout => {
                self.options.canvas_palette = Place::column();
                return;
            }
        };
        if let Some(action) = shown.action {
            self.toolbars.pending = Some(action);
        }
        if let Some(moved) = shown.moved {
            self.options.canvas_palette = Place::Floating(moved);
            self.save_options();
        }
        let Some(bar) = shown.rect else {
            return;
        };
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
        // beside the palette, on the side with the more room
        let gap = ui.spacing().item_spacing.x;
        let window = ui.ctx().content_rect();
        let (at, pivot) = if place.runs_across() {
            if bar.center().y < window.center().y {
                (
                    bar.left_bottom() + egui::vec2(0.0, gap),
                    egui::Align2::LEFT_TOP,
                )
            } else {
                (
                    bar.left_top() - egui::vec2(0.0, gap),
                    egui::Align2::LEFT_BOTTOM,
                )
            }
        } else if bar.center().x < window.center().x {
            (
                bar.right_top() + egui::vec2(gap, 0.0),
                egui::Align2::LEFT_TOP,
            )
        } else {
            (
                bar.left_top() - egui::vec2(gap, 0.0),
                egui::Align2::RIGHT_TOP,
            )
        };
        let list = self.canvas_flyout(ui, at, pivot, group.name, &tools);
        // on hover, the list closes once the pointer moves away from both it and the palette
        let near = |rect: egui::Rect, at: egui::Pos2| rect.expand(2.0 * gap).contains(at);
        if hover
            && ui.input(|i| i.pointer.delta() != egui::Vec2::ZERO)
            && let Some(at) = ui.ctx().pointer_hover_pos()
            && !near(bar, at)
            && !near(list, at)
            && !egui::Popup::is_any_open(ui.ctx())
        {
            self.canvas.group = None;
        }
    }

    /// The palette's group icons, in columns `per_column` long running down, or in one row
    /// across; pointing at one (or clicking, Options ▸ General ▸ Canvas tool lists open)
    /// lists its tools.
    fn palette_icons(
        &mut self,
        ui: &mut egui::Ui,
        groups: &[gui::ToolGroup],
        across: bool,
        per_column: usize,
        edge: f32,
        hover: bool,
    ) {
        let lines: Vec<&[gui::ToolGroup]> = if across {
            vec![groups]
        } else {
            groups.chunks(per_column).collect()
        };
        let line = |app: &mut Self, ui: &mut egui::Ui, column: &[gui::ToolGroup]| {
            for group in column {
                let name = group_name(group.name);
                let icon = group
                    .tools
                    .iter()
                    .find_map(|&id| app.icons.find(app.explicit_icon(id), id));
                let button = match icon {
                    Some(path) => egui::Button::image(
                        app.icons
                            .image(path)
                            .fit_to_exact_size(egui::vec2(edge, edge)),
                    ),
                    None => {
                        egui::Button::new(name.as_str()).wrap_mode(egui::TextWrapMode::Truncate)
                    }
                }
                .min_size(egui::vec2(edge, edge))
                .selected(app.canvas.group == Some(group.name));
                let response = ui.add(button);
                // on hover the list beside it names the group
                let response = if hover {
                    response
                } else {
                    response.on_hover_hint(&name)
                };
                if let Some(action) = app.group_menu_on(&response, group.name) {
                    app.invoke(ui.ctx(), &action);
                }
                if hover && response.hovered() {
                    app.canvas.group = Some(group.name);
                } else if response.clicked() {
                    app.canvas.group =
                        (hover || app.canvas.group != Some(group.name)).then_some(group.name);
                }
            }
        };
        if across {
            ui.horizontal(|ui| line(self, ui, groups));
        } else {
            ui.horizontal_top(|ui| {
                for column in lines {
                    ui.vertical(|ui| line(self, ui, column));
                }
            });
        }
    }

    /// A list of tools by icon and name, floating at `at` under its name, with the pointed-at
    /// tool's guidance beside it; picking one closes the list. Where it was drawn.
    fn canvas_flyout(
        &mut self,
        ui: &mut egui::Ui,
        at: egui::Pos2,
        pivot: egui::Align2,
        name: &str,
        tools: &[&'static str],
    ) -> egui::Rect {
        let row = ui.spacing().interact_size.y;
        let width = super::DESIGN_PANEL_WIDTHS * ui.spacing().interact_size.x * 0.6;
        let shown = egui::Area::new(egui::Id::unique("canvas_flyout"))
            .pivot(pivot)
            .fixed_pos(at)
            .show(ui.ctx(), |ui| {
                floating(ui).show(ui, |ui| {
                    cut_craft_bug_report::mark(ui, "Canvas flyout");
                    ui.strong(group_name(name));
                    ui.separator();
                    if tools.is_empty() {
                        ui.weak(super::layouts::NO_FAVOURITES);
                    }
                    // every tool shows: down to the chips, then on to another column, with
                    // the pointed-at tool's guidance beside them
                    // opening upward (from a palette at the foot) it has the room above
                    let room = if pivot.y() == egui::Align::Max {
                        at.y - ui.ctx().content_rect().top() - 5.0 * row
                    } else {
                        ui.ctx().content_rect().bottom() - ui.cursor().top() - 3.0 * row
                    };
                    let per_column = palette_rows(room, 1.7 * row + ui.spacing().item_spacing.y);
                    let mut picked = false;
                    // more columns than the window is wide scroll sideways
                    let across = ui.ctx().content_rect().right()
                        - ui.cursor().left()
                        - width
                        - 4.0 * ui.spacing().item_spacing.x
                        - 2.0 * row;
                    ui.horizontal_top(|ui| {
                        egui::ScrollArea::horizontal()
                            .max_width(across.max(row))
                            .show(ui, |ui| {
                                ui.horizontal_top(|ui| {
                                    // names keep to one line, however far the columns go
                                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                                    for column in tools.chunks(per_column) {
                                        ui.vertical(|ui| picked |= self.tool_rows(ui, column));
                                    }
                                });
                            });
                        ui.separator();
                        ui.vertical(|ui| {
                            ui.set_width(width);
                            self.tip_contents(ui);
                        });
                    });
                    if picked {
                        self.canvas.group = None;
                    }
                });
            });
        shown.response.rect
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

/// How many palette groups go down a column `room` high, each `step` tall: all that fit,
/// at least one.
fn palette_rows(room: f32, step: f32) -> usize {
    // whole groups only; the float is small and positive
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let rows = (room / step.max(1.0)).floor().max(1.0) as usize;
    rows
}
