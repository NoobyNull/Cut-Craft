//! Bars of tools on screen, the same for every bar (your own toolbars, the Canvas palette):
//! a grip of dots at the start, no words, dragged to move a floating bar and right-clicked
//! for the bar's menu ([`crate::bars::menu`]); a bar snapped to a side of the drawing,
//! pushing the drawing over or lying over its edge (Options ▸ General ▸ Bars snapped to a
//! side); a bar floating over the drawing, where it was left.

use super::{CutCraftApp, egui, gui};
use crate::bars::{Floating, Home, Side};
use crate::options::{SnappedBars, WindowLayout};
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_i18n::tr;

/// What drawing a bar gave back: an action picked from its grip's menu, and the place it
/// was dragged to, if it was let go this frame.
#[derive(Default)]
pub(super) struct Shown {
    pub action: Option<String>,
    pub moved: Option<Floating>,
    pub rect: Option<egui::Rect>,
}

/// Which bar is being drawn: its id, its name (said when its grip is pointed at) and its
/// grip's menu.
#[derive(Clone, Copy)]
pub(super) struct Bar<'a> {
    pub id: egui::Id,
    pub name: &'a str,
    pub menu: &'a [gui::MenuItem],
}

/// A bar's tools, drawn running across (`true`) or down.
pub(super) type Contents<'a> = &'a mut dyn FnMut(&mut CutCraftApp, &mut egui::Ui, bool);

/// Where the snapped bars may go this frame, and how much of each side the bars laid over
/// the drawing have taken so far (top, foot, left, right), so the next lies beside them.
pub(super) struct Room {
    rect: egui::Rect,
    pub used: [f32; 4],
}

impl Room {
    pub fn new(rect: egui::Rect) -> Self {
        Self {
            rect,
            used: [0.0; 4],
        }
    }
}

/// What the grip says when pointed at.
fn grip_hint(name: &str, movable: bool) -> String {
    if movable {
        tr!("bars-grip-hint", bar = name)
    } else {
        tr!("bars-grip-hint-fixed", bar = name)
    }
}

impl CutCraftApp {
    /// The bars beside or over the drawing: those snapped to a side, then those floating,
    /// starting near the drawing's top left, `left` further right (clear of the Canvas
    /// palette). `room` is the drawing's, and what the snapped ones took of it.
    pub(super) fn placed_bars(&mut self, ui: &mut egui::Ui, room: &mut Room, left: f32) {
        self.placed_toolbars(ui, room, None);
        // under the view's own bar (2D / 3D, the split, sheet and layer, snapping)
        let row = ui.spacing().interact_size.y;
        let margin = ui.spacing().item_spacing.y;
        let free = ui.available_rect_before_wrap();
        let start = free.left_top()
            + egui::vec2(
                room.used[2] + margin + left,
                room.used[0] + 2.0 * row + 3.0 * margin,
            );
        self.placed_toolbars(ui, room, Some(start));
    }

    /// Where this layout puts a bar back: a header tab, a Rail list or a Classic section.
    pub(super) fn bar_home(&self) -> Home {
        match self.options.general.layout {
            WindowLayout::Ribbon | WindowLayout::Canvas => Home::Header,
            WindowLayout::Rail => Home::Rail,
            WindowLayout::Classic => Home::DesignTab,
        }
    }

    /// A bar's grip: dots, no words; dragged (when `movable`) to move the bar, right-clicked
    /// for `menu`. Its response and what was picked from the menu.
    fn grip(
        &self,
        ui: &mut egui::Ui,
        across: bool,
        movable: bool,
        name: &str,
        menu: &[gui::MenuItem],
    ) -> (egui::Response, Option<String>) {
        let row = ui.spacing().interact_size.y;
        let edge = super::TOOL_ICON_ROWS * row;
        let size = if across {
            egui::vec2(0.5 * row, edge)
        } else {
            egui::vec2(edge, 0.5 * row)
        };
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
        let colour = if response.hovered() || response.dragged() {
            ui.visuals().strong_text_color()
        } else {
            ui.visuals().weak_text_color()
        };
        // two lines of dots along the bar's start
        let step = 0.25 * row;
        let dots = (if across { rect.height() } else { rect.width() } / step).floor();
        for i in 0..2_u8 {
            // a handful of dots; the floats are small
            #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            for j in 1..dots as usize {
                #[expect(clippy::cast_precision_loss)]
                let along = j as f32 * step;
                let over = if i == 0 { 0.25 } else { 0.75 } * size.min_elem();
                let at = if across {
                    rect.left_top() + egui::vec2(over, along)
                } else {
                    rect.left_top() + egui::vec2(along, over)
                };
                ui.painter().circle_filled(at, 1.2, colour);
            }
        }
        if movable && response.hovered() {
            ui.ctx().set_cursor_icon(if response.dragged() {
                egui::CursorIcon::Grabbing
            } else {
                egui::CursorIcon::Grab
            });
        }
        let response = response.on_hover_hint(grip_hint(name, movable));
        let picked = self.popup(&response, || menu.to_vec());
        (response, picked)
    }

    /// A bar's grip then its tools, across or down.
    fn bar_body(
        &mut self,
        ui: &mut egui::Ui,
        across: bool,
        movable: bool,
        name: &str,
        menu: &[gui::MenuItem],
        contents: Contents<'_>,
    ) -> Option<(egui::Response, Option<String>)> {
        let mut grip = None;
        let mut body = |ui: &mut egui::Ui| {
            grip = Some(self.grip(ui, across, movable, name, menu));
            // what the window has left past the grip: a long bar scrolls rather than
            // running off the window's edge
            let row = ui.spacing().interact_size.y;
            let left = ui.ctx().content_rect().max - ui.cursor().min;
            let scroll = if across {
                egui::ScrollArea::horizontal().max_width((left.x - row).max(3.0 * row))
            } else {
                egui::ScrollArea::vertical().max_height((left.y - row).max(3.0 * row))
            };
            scroll
                .id_salt("bar contents")
                .show(ui, |ui| contents(self, ui, across));
        };
        if across {
            ui.horizontal_top(|ui| body(ui));
        } else {
            ui.vertical(|ui| body(ui));
        }
        grip
    }

    /// A bar floating over the drawing as `floating` places it (at `start` until moved),
    /// moved by dragging its grip unless locked.
    pub(super) fn floating_bar(
        &mut self,
        ctx: &egui::Context,
        bar: &Bar<'_>,
        floating: Floating,
        start: egui::Pos2,
        contents: Contents<'_>,
    ) -> Shown {
        let Bar { id, name, menu } = *bar;
        let window = ctx.content_rect();
        let size = ctx
            .memory(|m| m.area_rect(id))
            .map_or(egui::Vec2::ZERO, |r| r.size());
        let placed = floating.top_left(size, window, start);
        let at = match self.toolbars.drag {
            Some((dragged, at)) if dragged == id => at,
            _ => placed,
        };
        let across = !floating.down;
        let movable = !floating.locked;
        let mut grip = None;
        let area = egui::Area::new(id)
            .fixed_pos(at)
            .constrain(true)
            .show(ctx, |ui| {
                egui::Frame::window(ui.style())
                    .inner_margin(ui.spacing().item_spacing.y)
                    .show(ui, |ui| {
                        cut_craft_bug_report::mark(ui, "Floating bar");
                        grip = self.bar_body(ui, across, movable, name, menu, contents);
                    });
            });
        let mut shown = Shown {
            rect: Some(area.response.rect),
            ..Shown::default()
        };
        let Some((response, action)) = grip else {
            return shown;
        };
        shown.action = action;
        if movable {
            if response.drag_started() {
                self.toolbars.drag = Some((id, at));
            }
            if response.dragged()
                && let Some((dragged, at)) = &mut self.toolbars.drag
                && *dragged == id
            {
                *at += response.drag_delta();
            }
            if response.drag_stopped()
                && let Some((dragged, at)) = self.toolbars.drag.take()
                && dragged == id
            {
                let mut moved = floating;
                if moved.drop_at(at, area.response.rect.size(), window) {
                    shown.moved = Some(moved);
                }
            }
        }
        shown
    }

    /// A bar snapped to `side` of the drawing: its own strip that the drawing makes room
    /// for, or laid over the drawing's edge (Options ▸ General ▸ Bars snapped to a side). A
    /// drag on its grip does nothing; only its menu takes it off the side.
    pub(super) fn snapped_bar(
        &mut self,
        ui: &mut egui::Ui,
        room: &mut Room,
        bar: &Bar<'_>,
        side: Side,
        contents: Contents<'_>,
    ) -> Shown {
        let Bar { id, name, menu } = *bar;
        let across = matches!(side, Side::Top | Side::Bottom);
        let mut picked = None;
        let push = self.options.general.snapped_bars == SnappedBars::PushTheDrawingOver;
        let rect = if push {
            let panel = match side {
                Side::Top => egui::Panel::top(id),
                Side::Bottom => egui::Panel::bottom(id),
                Side::Left => egui::Panel::left(id),
                Side::Right => egui::Panel::right(id),
            };
            panel
                .resizable(false)
                .show(ui, |ui| {
                    cut_craft_bug_report::mark(ui, "Snapped bar");
                    picked = self
                        .bar_body(ui, across, false, name, menu, contents)
                        .and_then(|(_, picked)| picked);
                })
                .response
                .rect
        } else {
            let gap = room.used;
            let r = room.rect;
            let (pivot, at) = match side {
                Side::Top => (
                    egui::Align2::LEFT_TOP,
                    r.left_top() + egui::vec2(0.0, gap[0]),
                ),
                Side::Bottom => (
                    egui::Align2::LEFT_BOTTOM,
                    r.left_bottom() - egui::vec2(0.0, gap[1]),
                ),
                Side::Left => (
                    egui::Align2::LEFT_TOP,
                    r.left_top() + egui::vec2(gap[2], 0.0),
                ),
                Side::Right => (
                    egui::Align2::RIGHT_TOP,
                    r.right_top() - egui::vec2(gap[3], 0.0),
                ),
            };
            egui::Area::new(id)
                .pivot(pivot)
                .fixed_pos(at)
                .show(ui.ctx(), |ui| {
                    egui::Frame::window(ui.style())
                        .inner_margin(ui.spacing().item_spacing.y)
                        .show(ui, |ui| {
                            cut_craft_bug_report::mark(ui, "Snapped bar");
                            picked = self
                                .bar_body(ui, across, false, name, menu, contents)
                                .and_then(|(_, picked)| picked);
                        });
                })
                .response
                .rect
        };
        let used = match side {
            Side::Top | Side::Bottom => rect.height(),
            Side::Left | Side::Right => rect.width(),
        };
        let i = match side {
            Side::Top => 0,
            Side::Bottom => 1,
            Side::Left => 2,
            Side::Right => 3,
        };
        // a bar that pushed the drawing over took its room already
        if !push {
            room.used[i] += used;
        }
        Shown {
            action: picked,
            moved: None,
            rect: Some(rect),
        }
    }
}
