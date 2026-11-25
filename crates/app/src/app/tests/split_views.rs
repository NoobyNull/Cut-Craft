use super::*;

/// Where `text` is drawn last: the small view's bar is drawn over the big view.
fn last_text(h: &Harness, text: &str) -> Pos2 {
    *h.texts_at(text)
        .last()
        .unwrap_or_else(|| panic!("{text} on screen"))
}

#[test]
fn the_top_bar_puts_one_view_in_a_corner_and_swap_makes_it_the_big_one() {
    let mut h = Harness::new();
    h.frame(vec![]);
    h.app.invoke(&h.ctx, "split_picture");
    assert_eq!(h.app.split, Some(Split::Picture));
    h.frame(vec![]);
    h.frame(vec![]);
    assert_eq!(h.app.view, View::TwoD);
    let name = last_text(&h, "3D View (F3)");
    let room = h.ctx.content_rect();
    assert!(
        name.x > room.center().x && name.y > room.center().y,
        "the 3D view sits small at the bottom right: {name:?}"
    );
    h.click(last_text(&h, "⇄"), Modifiers::NONE);
    assert_eq!(
        h.app.view,
        View::ThreeD,
        "⇄ makes the small view the big one"
    );
    h.frame(vec![]);
    assert!(
        !h.texts_at("2D View (F2)").is_empty(),
        "2D is now the small one"
    );
    h.app.invoke(&h.ctx, "split_picture");
    assert_eq!(h.app.split, None, "a second click shows one view again");
}

#[test]
fn the_small_views_bar_drags_it_to_another_corner() {
    let mut h = Harness::new();
    h.app.split = Some(Split::Picture);
    h.frame(vec![]);
    h.frame(vec![]);
    let name = last_text(&h, "3D View (F3)");
    let room = h.ctx.content_rect();
    h.drag(name, room.left_top() + egui::vec2(200.0, 200.0));
    assert_eq!(h.app.split_layout.corner, egui::Align2::LEFT_TOP);
}

#[test]
fn the_divider_shares_the_room_and_dragged_to_the_edge_ends_the_split() {
    let mut h = Harness::new();
    h.app.split = Some(Split::SideBySide);
    h.frame(vec![]);
    h.frame(vec![]);
    let swap = last_text(&h, "⇄");
    let on_divider = swap + egui::vec2(0.0, 80.0);
    let room = h.ctx.content_rect();
    let to = egui::pos2(swap.x - 0.15 * room.width(), on_divider.y);
    h.drag(on_divider, to);
    let share = h.app.split_layout.share;
    assert!(
        share < 0.45 && share > 0.2,
        "the divider moved left: {share}"
    );
    assert_eq!(h.app.split, Some(Split::SideBySide));

    h.click(last_text(&h, "⇄"), Modifiers::NONE);
    assert!(
        h.app.split_layout.three_d_first,
        "⇄ swaps the views' places"
    );

    h.frame(vec![]);
    let swap = last_text(&h, "⇄");
    let on_divider = swap + egui::vec2(0.0, 80.0);
    h.drag(on_divider, egui::pos2(room.left() + 2.0, on_divider.y));
    assert_eq!(h.app.split, None, "dragged to the edge, the split ends");
    assert_eq!(
        h.app.view,
        View::TwoD,
        "the 3D view on the left went; the 2D view has the room"
    );
}

#[test]
fn a_zoomed_2d_view_keeps_its_drawing_when_the_view_moves() {
    let mut h = Harness::new();
    h.frame(vec![]);
    let middle = view_middle(&h);
    h.app.camera = h.app.view_transform.map(|t| t.zoomed_about(middle, 2.0));
    h.frame(vec![]);
    let before = h.at(0.0, 0.0) - h.app.view_2d_at.unwrap();
    h.app.split = Some(Split::SideBySide);
    h.app.split_layout.three_d_first = true;
    h.frame(vec![]);
    h.frame(vec![]);
    let after = h.at(0.0, 0.0) - h.app.view_2d_at.unwrap();
    assert!((before - after).length() < 0.5, "{before:?} then {after:?}");
}

#[test]
fn the_small_view_snaps_only_when_let_go() {
    let mut h = Harness::new();
    h.app.split = Some(Split::Picture);
    h.frame(vec![]);
    h.frame(vec![]);
    let from = last_text(&h, "3D View (F3)");
    let to = h.ctx.content_rect().left_top() + egui::vec2(200.0, 200.0);
    h.frame(vec![Event::PointerMoved(from)]);
    h.press(from, true, Modifiers::NONE);
    h.frame(vec![Event::PointerMoved(from.lerp(to, 0.5))]);
    h.frame(vec![Event::PointerMoved(to)]);
    assert_eq!(
        h.app.split_layout.corner,
        egui::Align2::RIGHT_BOTTOM,
        "while held it only shows where it will go"
    );
    h.press(to, false, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.app.split_layout.corner, egui::Align2::LEFT_TOP);
}
