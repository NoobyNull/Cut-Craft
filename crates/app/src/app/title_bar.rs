//! Cut Craft's own title bar, drawn in the menu bar's row so the window is one surface:
//! the space beside the menus moves the window, its buttons minimise, maximise and close
//! it, and the window's edges resize it. Options ▸ Appearance gives back the desktop's.

use super::egui;
use egui::{CursorIcon, Rect, ResizeDirection, Sense, StrokeKind, Vec2, ViewportCommand, vec2};

/// A window button's width, in its heights: wider than tall, as desktops draw them.
const BUTTON_ASPECT: f32 = 1.5;
/// A window button's glyph, as a share of the button's height.
const GLYPH_SHARE: f32 = 0.4;
/// How far the restore glyph's two windows sit apart, as a share of the glyph.
const RESTORE_OFFSET: f32 = 0.25;

#[derive(Clone, Copy, PartialEq)]
enum Button {
    Minimise,
    Maximise,
    Close,
}

fn maximised(ctx: &egui::Context) -> bool {
    ctx.input(|i| i.viewport().maximized.unwrap_or(false))
}

/// After the menus, in the menu bar: the window buttons at the right, the window's title
/// centred, and the space between to drag the window by (twice clicked, it maximises).
pub(super) fn controls(ui: &mut egui::Ui, title: &str) {
    let ctx = ui.ctx().clone();
    let maximised = maximised(&ctx);
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        for button in [Button::Close, Button::Maximise, Button::Minimise] {
            if window_button(ui, button, maximised).clicked() {
                ctx.send_viewport_cmd(match button {
                    Button::Minimise => ViewportCommand::Minimized(true),
                    Button::Maximise => ViewportCommand::Maximized(!maximised),
                    Button::Close => ViewportCommand::Close,
                });
            }
        }
        // the buttons' row, from the menus to the buttons
        let rest = Rect::from_x_y_ranges(
            ui.available_rect_before_wrap().x_range(),
            ui.min_rect().y_range(),
        );
        let drag = ui.interact(
            rest,
            ui.scope_id().with("title_drag"),
            Sense::click_and_drag(),
        );
        if drag.drag_started_by(egui::PointerButton::Primary) {
            ctx.send_viewport_cmd(ViewportCommand::StartDrag);
        }
        if drag.double_clicked() {
            ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximised));
        }
        let colour = ui.visuals().text_color();
        let font = egui::TextStyle::Body.resolve(ui.style());
        let galley = ui.painter().layout_no_wrap(title.to_owned(), font, colour);
        // centred on the window, or failing that in the space it has; left out if neither fits
        let on_window = egui::pos2(ctx.content_rect().center().x, rest.center().y);
        let place = [on_window, rest.center()]
            .map(|centre| Rect::from_center_size(centre, galley.size()))
            .into_iter()
            .find(|at| rest.contains_rect(*at));
        if let Some(at) = place {
            ui.painter().galley(at.min, galley, colour);
        }
    });
}

fn window_button(ui: &mut egui::Ui, button: Button, maximised: bool) -> egui::Response {
    let height = ui.spacing().interact_size.y;
    let (rect, response) =
        ui.allocate_exact_size(vec2(height * BUTTON_ASPECT, height), Sense::click());
    let label = match button {
        Button::Minimise => "Minimise",
        Button::Maximise if maximised => "Restore",
        Button::Maximise => "Maximise",
        Button::Close => "Close",
    };
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), label));
    let visuals = ui.style().interact(&response);
    if response.hovered() {
        let fill = if button == Button::Close {
            ui.visuals().error_fg_color
        } else {
            visuals.bg_fill
        };
        ui.painter().rect_filled(rect, visuals.corner_radius, fill);
    }
    let glyph = Rect::from_center_size(rect.center(), Vec2::splat(height * GLYPH_SHARE));
    let stroke = visuals.fg_stroke;
    let painter = ui.painter();
    match button {
        Button::Minimise => {
            painter.line_segment([glyph.left_center(), glyph.right_center()], stroke);
        }
        Button::Maximise if maximised => {
            let offset = glyph.width() * RESTORE_OFFSET;
            let front =
                Rect::from_min_max(glyph.min + vec2(0.0, offset), glyph.max - vec2(offset, 0.0));
            for window in [front.translate(vec2(offset, -offset)), front] {
                painter.rect_stroke(window, 0, stroke, StrokeKind::Inside);
            }
        }
        Button::Maximise => {
            painter.rect_stroke(glyph, 0, stroke, StrokeKind::Inside);
        }
        Button::Close => {
            painter.line_segment([glyph.left_top(), glyph.right_bottom()], stroke);
            painter.line_segment([glyph.right_top(), glyph.left_bottom()], stroke);
        }
    }
    response.on_hover_text(label)
}

/// With no desktop frame, the window's edges resize it: within a grab's reach of an edge or
/// corner the pointer shows the resize cursor, and pressing there starts the desktop's resize.
pub(super) fn resize_edges(ctx: &egui::Context) {
    if maximised(ctx) || ctx.input(|i| i.viewport().fullscreen.unwrap_or(false)) {
        return;
    }
    let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) else {
        return;
    };
    let reach = ctx.global_style().interaction.resize_grab_radius_side;
    let window = ctx.content_rect();
    let west = pos.x < window.left() + reach;
    let east = pos.x > window.right() - reach;
    let north = pos.y < window.top() + reach;
    let south = pos.y > window.bottom() - reach;
    let (direction, cursor) = match (north, south, west, east) {
        (true, _, true, _) => (ResizeDirection::NorthWest, CursorIcon::ResizeNwSe),
        (true, _, _, true) => (ResizeDirection::NorthEast, CursorIcon::ResizeNeSw),
        (_, true, true, _) => (ResizeDirection::SouthWest, CursorIcon::ResizeNeSw),
        (_, true, _, true) => (ResizeDirection::SouthEast, CursorIcon::ResizeNwSe),
        (true, ..) => (ResizeDirection::North, CursorIcon::ResizeVertical),
        (_, true, ..) => (ResizeDirection::South, CursorIcon::ResizeVertical),
        (_, _, true, _) => (ResizeDirection::West, CursorIcon::ResizeHorizontal),
        (_, _, _, true) => (ResizeDirection::East, CursorIcon::ResizeHorizontal),
        _ => return,
    };
    ctx.set_cursor_icon(cursor);
    if ctx.input(|i| i.pointer.primary_pressed()) {
        ctx.send_viewport_cmd(ViewportCommand::BeginResize(direction));
    }
}
