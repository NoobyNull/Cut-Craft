//! The toolpath list as a tree: the grip (≡) each row is dragged by, the room for its place
//! in the tree with the guide lines showing which group holds it, and a group's arrow that
//! opens or closes it.

use super::{HoverHint, egui, tr};
use crate::toolpath_order::Row;

/// The width of one level of the tree, and of a group's arrow.
pub(super) const LEVEL: f32 = 16.0;

/// The grip at the left of a row: three short lines, a grab hand over them, dragged to
/// change the cut order.
pub(super) fn grip(ui: &mut egui::Ui) -> egui::Response {
    let height = ui.spacing().interact_size.y;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(12.0, height), egui::Sense::drag());
    let response = response
        .on_hover_cursor(egui::CursorIcon::Grab)
        .on_hover_hint(tr!("panels-drag-to-change-the-cut-order"));
    let colour = if response.hovered() || response.dragged() {
        ui.visuals().strong_text_color()
    } else {
        ui.visuals().weak_text_color()
    };
    let stroke = egui::Stroke::new(1.5, colour);
    for dy in [-3.5, 0.0, 3.5] {
        let y = rect.center().y + dy;
        ui.painter()
            .hline(rect.left() + 1.0..=rect.right() - 1.0, y, stroke);
    }
    response
}

/// The room for `row`'s place in the tree, a level for each group it sits in, then its
/// arrow's place. For a group holding anything, the arrow (open: pointing down) and its
/// response; a click on it opens or closes the group. Returns the room taken, for
/// [`guide_lines`].
pub(super) fn tree_room(
    ui: &mut egui::Ui,
    row: &Row,
    open: bool,
) -> (egui::Rect, Option<egui::Response>) {
    let height = ui.spacing().interact_size.y;
    #[expect(clippy::cast_precision_loss, reason = "a handful of levels")]
    let width = LEVEL * (row.depth + 1) as f32;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    if !row.holds {
        return (rect, None);
    }
    let place = egui::Rect::from_min_max(
        egui::pos2(rect.right() - LEVEL, rect.top()),
        rect.right_bottom(),
    );
    let id = ui.scope_id().with(("toolpath group arrow", &row.item));
    let arrow = ui
        .interact(place, id, egui::Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_hint(tr!("panels-open-or-close-this-group"));
    let icon = egui::Rect::from_center_size(place.center(), egui::vec2(9.0, 9.0));
    let rotation = if open {
        0.0
    } else {
        -std::f32::consts::FRAC_PI_2
    };
    let colour = ui.style().interact(&arrow).fg_stroke.color;
    ui.painter()
        .add(egui::Shape::rotated_triangle(icon, rotation, colour));
    (rect, Some(arrow))
}

/// The guide lines for `row`, laid out in `room` (from [`tree_room`]) of a row standing in
/// `row_rect`: a line down from each group it sits in, turning into it from its own group.
pub(super) fn guide_lines(ui: &egui::Ui, row: &Row, room: egui::Rect, row_rect: egui::Rect) {
    let stroke = egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color);
    // across the gap between rows too, so the lines join up
    let gap = ui.spacing().item_spacing.y / 2.0;
    let (top, bottom, middle) = (
        row_rect.top() - gap,
        row_rect.bottom() + gap,
        row_rect.center().y,
    );
    let painter = ui.painter();
    for (k, &runs_on) in row.lines.iter().enumerate() {
        #[expect(clippy::cast_precision_loss, reason = "a handful of levels")]
        let x = room.left() + LEVEL * k as f32 + LEVEL / 2.0;
        if k + 1 < row.lines.len() {
            if runs_on {
                painter.vline(x, top..=bottom, stroke);
            }
        } else {
            painter.vline(x, top..=if runs_on { bottom } else { middle }, stroke);
            // into the row, past its arrow's place when it has none
            let reach = if row.holds { LEVEL * 0.6 } else { LEVEL * 1.4 };
            painter.hline(x..=x + reach, middle, stroke);
        }
    }
}

/// Where a drop would go, drawn over the list: a line at the top of the row, starting at
/// its level, or a box round a group's name it would go into.
pub(super) fn drop_mark(
    ui: &egui::Ui,
    row_rect: egui::Rect,
    from_x: f32,
    into: Option<egui::Rect>,
) {
    let stroke = egui::Stroke::new(2.0, ui.visuals().selection.stroke.color);
    if let Some(name) = into {
        ui.painter()
            .rect_stroke(name, 2.0, stroke, egui::StrokeKind::Outside);
    } else {
        let y = row_rect.top() - ui.spacing().item_spacing.y / 2.0;
        ui.painter().hline(from_x..=row_rect.right(), y, stroke);
        ui.painter()
            .circle_filled(egui::pos2(from_x, y), 3.0, stroke.color);
    }
}
