//! Both views at once: 2D and 3D over and under or side by side with a divider to drag,
//! or one in a corner of the other (picture in picture); ⇄ swaps them.

use super::{CutCraftApp, Split, View, egui};
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_i18n::tr;

/// The narrowest a view in a split may be dragged; past it the split ends and the other
/// view has the whole room.
const LEAST_SHARE: f32 = 0.08;
/// How much of the room's width the corner view takes.
const PICTURE_SHARE: f32 = 0.3;
/// The divider's thickness, in points.
const DIVIDER: f32 = 6.0;

/// How the two views share the room: the first view's share in a split, whether 3D comes
/// first, and the corner the small view sits in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct SplitLayout {
    pub share: f32,
    pub three_d_first: bool,
    pub corner: egui::Align2,
}

impl Default for SplitLayout {
    fn default() -> Self {
        Self {
            share: 0.5,
            three_d_first: false,
            corner: egui::Align2::RIGHT_BOTTOM,
        }
    }
}

/// Where a drag of the divider leaves the split: `Some(share)` to keep it, clamped to
/// what fits; `None` when it went past either end.
pub(super) fn dragged_share(share: f32) -> Option<f32> {
    (LEAST_SHARE..=1.0 - LEAST_SHARE)
        .contains(&share)
        .then_some(share)
}

/// The corner nearest `at` in `room`.
pub(super) fn nearest_corner(room: egui::Rect, at: egui::Pos2) -> egui::Align2 {
    let x = if at.x < room.center().x {
        egui::Align::Min
    } else {
        egui::Align::Max
    };
    let y = if at.y < room.center().y {
        egui::Align::Min
    } else {
        egui::Align::Max
    };
    egui::Align2([x, y])
}

/// The small view's place in `room`, in `corner`.
pub(super) fn picture_rect(room: egui::Rect, corner: egui::Align2, margin: f32) -> egui::Rect {
    let width = PICTURE_SHARE * room.width();
    let size = egui::vec2(width, (width * 0.7).min(0.5 * room.height()));
    corner.align_size_within_rect(size, room.shrink(margin))
}

impl CutCraftApp {
    /// The two views in the room left, as `self.split` arranges them.
    pub(super) fn split_views(&mut self, ui: &mut egui::Ui, split: Split) {
        let room = ui.available_rect_before_wrap();
        if split == Split::Picture {
            self.picture_in_picture(ui, room);
            return;
        }
        let stacked = split == Split::Stacked;
        let along = if stacked { room.height() } else { room.width() };
        let cut = self.split_layout.share * along - DIVIDER / 2.0;
        let (first, rest) = if stacked {
            room.split_top_bottom_at_y(room.top() + cut)
        } else {
            room.split_left_right_at_x(room.left() + cut)
        };
        let (divider, second) = if stacked {
            rest.split_top_bottom_at_y(rest.top() + DIVIDER)
        } else {
            rest.split_left_right_at_x(rest.left() + DIVIDER)
        };
        let three_d_first = self.split_layout.three_d_first;
        self.view_in(ui, first, !three_d_first);
        self.view_in(ui, second, three_d_first);
        self.split_divider(ui, room, divider, stacked);
    }

    /// One view drawn in `rect`: 2D when `two_d`, else 3D.
    fn view_in(&mut self, ui: &mut egui::Ui, rect: egui::Rect, two_d: bool) {
        ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
            ui.set_clip_rect(rect);
            if two_d {
                self.view_2d(ui);
            } else {
                self.view_3d(ui);
            }
        });
    }

    /// The bar between the views: drag it to share the room differently, past either end
    /// to show just the other view; ⇄ on it swaps the views.
    fn split_divider(
        &mut self,
        ui: &mut egui::Ui,
        room: egui::Rect,
        divider: egui::Rect,
        stacked: bool,
    ) {
        let response = ui
            .interact(
                divider,
                ui.scope_id().with("split_divider"),
                egui::Sense::drag(),
            )
            .on_hover_cursor(if stacked {
                egui::CursorIcon::ResizeVertical
            } else {
                egui::CursorIcon::ResizeHorizontal
            })
            .on_hover_hint(tr!("views-split-divider"));
        let colour = if response.hovered() || response.dragged() {
            ui.visuals().widgets.hovered.bg_fill
        } else {
            ui.visuals().widgets.noninteractive.bg_stroke.color
        };
        ui.painter().rect_filled(divider, 0.0, colour);
        if let Some(at) = response
            .interact_pointer_pos()
            .filter(|_| response.dragged())
        {
            let share = if stacked {
                (at.y - room.top()) / room.height()
            } else {
                (at.x - room.left()) / room.width()
            };
            if let Some(share) = dragged_share(share) {
                self.split_layout.share = share;
            } else {
                // the view dragged away goes; the other has the room
                let first_kept = share > 0.5;
                let two_d = first_kept != self.split_layout.three_d_first;
                self.view = if two_d { View::TwoD } else { View::ThreeD };
                self.split = None;
                self.split_layout.share = 0.5;
            }
        }
        let swap = egui::Rect::from_center_size(
            divider.center(),
            egui::Vec2::splat(ui.spacing().interact_size.y),
        );
        if ui
            .put(swap, egui::Button::new("⇄"))
            .on_hover_hint(tr!("views-split-swap"))
            .clicked()
        {
            self.split_layout.three_d_first ^= true;
        }
    }

    /// The view picked (2D or 3D) fills the room; the other sits small in a corner. Drag
    /// the small view's bar to another corner; ⇄ on it swaps which view is big.
    fn picture_in_picture(&mut self, ui: &mut egui::Ui, room: egui::Rect) {
        let big_2d = self.view == View::TwoD;
        self.view_in(ui, room, big_2d);
        let margin = ui.spacing().item_spacing.y;
        let small = picture_rect(room, self.split_layout.corner, margin);
        let row = ui.spacing().interact_size.y;
        let (bar, inside) = small.split_top_bottom_at_y(small.top() + row);
        let stroke = egui::Stroke::new(2.0, ui.visuals().selection.bg_fill);
        ui.painter().rect_filled(bar, 0.0, ui.visuals().window_fill);
        self.view_in(ui, inside, !big_2d);
        ui.painter()
            .rect_stroke(small, 0.0, stroke, egui::StrokeKind::Outside);
        let grab = ui
            .interact(bar, ui.scope_id().with("picture_bar"), egui::Sense::drag())
            .on_hover_cursor(egui::CursorIcon::Grab)
            .on_hover_hint(tr!("views-picture-bar"));
        if grab.drag_stopped()
            && let Some(at) = grab.interact_pointer_pos()
        {
            self.split_layout.corner = nearest_corner(room, at);
        }
        let name = if big_2d {
            tr!("views-3d-view-f3")
        } else {
            tr!("views-2d-view-f2")
        };
        ui.painter().text(
            bar.left_center() + egui::vec2(margin, 0.0),
            egui::Align2::LEFT_CENTER,
            name,
            egui::TextStyle::Small.resolve(ui.style()),
            ui.visuals().text_color(),
        );
        let swap = egui::Rect::from_min_size(
            bar.right_top() - egui::vec2(row, 0.0),
            egui::Vec2::splat(row),
        );
        if ui
            .put(swap, egui::Button::new("⇄"))
            .on_hover_hint(tr!("views-picture-swap"))
            .clicked()
        {
            self.view = if big_2d { View::ThreeD } else { View::TwoD };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_drag_keeps_the_split_until_a_view_is_almost_gone() {
        assert_eq!(dragged_share(0.3), Some(0.3));
        assert_eq!(dragged_share(LEAST_SHARE), Some(LEAST_SHARE));
        assert_eq!(dragged_share(0.02), None);
        assert_eq!(dragged_share(0.98), None);
    }

    #[test]
    fn the_small_view_goes_to_the_corner_nearest_where_it_was_let_go() {
        let room = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 600.0));
        for (at, corner) in [
            (egui::pos2(10.0, 10.0), egui::Align2::LEFT_TOP),
            (egui::pos2(990.0, 10.0), egui::Align2::RIGHT_TOP),
            (egui::pos2(10.0, 590.0), egui::Align2::LEFT_BOTTOM),
            (egui::pos2(990.0, 590.0), egui::Align2::RIGHT_BOTTOM),
        ] {
            assert_eq!(nearest_corner(room, at), corner, "{at:?}");
            let picture = picture_rect(room, corner, 4.0);
            assert!(room.contains_rect(picture), "{corner:?}");
            assert_eq!(nearest_corner(room, picture.center()), corner);
        }
    }
}
