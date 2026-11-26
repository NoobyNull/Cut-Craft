//! The Canvas layout: the view has the whole window, and the tools, the open form and
//! the job's lists float over it.

use super::*;
use crate::options::WindowLayout;

fn canvas() -> Harness {
    let mut h = Harness::new();
    h.given_options(|o| o.general.layout = WindowLayout::Canvas);
    h.frame(vec![]);
    h.frame(vec![]);
    h
}

fn click_text(h: &mut Harness, text: &str) {
    let at = h
        .text_at(text)
        .unwrap_or_else(|| panic!("{text} not shown"));
    h.frame(vec![Event::PointerMoved(at)]);
    h.press(at, true, Modifiers::NONE);
    h.press(at, false, Modifiers::NONE);
    h.frame(vec![]);
}

#[test]
fn the_view_has_the_whole_width() {
    let h = canvas();
    let view = h.text_at("2D View (F2)").expect("the view's own bar");
    assert!(
        view.x < 0.1 * h.window.x,
        "no side panel pushes the view over: {view:?}"
    );
}

#[test]
fn a_palette_group_lists_its_tools_and_a_pick_floats_its_form() {
    let mut h = canvas();
    assert!(h.text_at("Draw Circle").is_none(), "no list open at first");
    h.open_canvas_group_list("Create Vectors");
    // a box that just opened is measured first, then drawn
    h.frame(vec![]);
    h.frame(vec![]);
    click_text(&mut h, "Draw Circle");
    assert_eq!(h.form_id(), Some("draw_circle"));
    assert!(h.open_canvas_group().is_none(), "the list closes on a pick");
    let form = h
        .text_at("Draw Circle")
        .expect("the form floats over the view");
    assert!(form.x > 0.5 * h.window.x, "on the right: {form:?}");
}

#[test]
fn a_chip_opens_its_list_over_the_view_and_closes_again() {
    let mut h = canvas();
    assert!(h.open_canvas_chip().is_none());
    click_text(&mut h, "Layers");
    let layers = INSPECTOR_TABS.iter().position(|t| *t == "Layers");
    assert_eq!(h.open_canvas_chip(), layers);
    assert!(h.text_at("Layer 1").is_some(), "the layer list shows");
    let chip = h
        .texts_at("Layers")
        .into_iter()
        .max_by(|a, b| a.y.total_cmp(&b.y))
        .unwrap();
    h.frame(vec![Event::PointerMoved(chip)]);
    h.press(chip, true, Modifiers::NONE);
    h.press(chip, false, Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.open_canvas_chip().is_none());
}

#[test]
fn find_a_tool_is_in_the_header_and_enter_runs_the_first_match() {
    let mut h = canvas();
    let hint = cut_craft_i18n::tr!("layouts-canvas-find");
    assert!(h.text_at(&hint).is_some(), "the box shows in the header");
    h.invoke("find_tool");
    h.frame(vec![]);
    h.frame(vec![Event::Text("circle".into())]);
    h.frame(vec![]);
    assert!(h.text_at("Draw Circle").is_some(), "matches drop down");
    assert!(
        h.text_at(&cut_craft_i18n::tr!("layouts-find-tool"))
            .is_none(),
        "no Find Tool window in this layout"
    );
    h.frame(vec![Event::Key {
        key: Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    h.frame(vec![]);
    assert_eq!(h.form_id(), Some("draw_circle"));
    assert!(!h.finding_a_tool());
}

#[test]
fn enter_in_an_empty_find_box_runs_nothing() {
    let mut h = canvas();
    h.invoke("find_tool");
    h.frame(vec![]);
    h.frame(vec![key(Key::Enter)]);
    h.frame(vec![]);
    assert!(h.form_id().is_none(), "{:?}", h.form_id());
}

#[test]
fn the_library_components_and_toolpaths_actions_open_their_chips() {
    let mut h = canvas();
    for (id, tab) in [
        ("clipart_library", "Library"),
        ("component_levels", "Components"),
        ("show_toolpaths_tab", "Toolpaths"),
    ] {
        h.invoke(id);
        let chip = INSPECTOR_TABS.iter().position(|t| *t == tab);
        assert_eq!(h.open_canvas_chip(), chip, "{id}");
        h.frame(vec![]);
    }
}

#[test]
fn every_palette_group_can_be_reached_in_a_short_window() {
    let mut h = canvas();
    h.show_ribbon_tab("Toolpaths");
    h.window = egui::vec2(h.window.x, 420.0);
    h.frame(vec![]);
    h.frame(vec![]);
    let palette = h
        .ctx
        .memory(|m| m.area_rect(egui::Id::unique("canvas_palette")))
        .expect("the palette is drawn");
    assert!(
        palette.bottom() <= 420.0,
        "the palette scrolls within the window: {palette:?}"
    );
}

/// The palette's place on screen.
fn palette(h: &Harness) -> egui::Rect {
    h.ctx
        .memory(|m| m.area_rect(egui::Id::unique("canvas_palette")))
        .expect("the palette is drawn")
}

#[test]
fn pointing_at_a_palette_group_lists_its_tools_and_moving_away_closes_them() {
    let mut h = canvas();
    assert_eq!(
        h.options().general.canvas_flyouts,
        crate::options::CanvasFlyouts::OnHover
    );
    let first = palette(&h).left_top() + egui::vec2(20.0, 20.0);
    h.frame(vec![Event::PointerMoved(first)]);
    h.frame(vec![]);
    let group = h.canvas_groups()[0].name;
    assert_eq!(h.open_canvas_group(), Some(group), "no click needed");
    h.frame(vec![]);
    let far = egui::pos2(0.6 * h.window.x, 0.5 * h.window.y);
    h.frame(vec![Event::PointerMoved(far)]);
    h.frame(vec![]);
    assert!(h.open_canvas_group().is_none(), "the list closes");
}

#[test]
fn on_click_a_palette_group_waits_for_its_click() {
    let mut h = canvas();
    h.given_options(|o| o.general.canvas_flyouts = crate::options::CanvasFlyouts::OnClick);
    let first = palette(&h).left_top() + egui::vec2(20.0, 20.0);
    h.frame(vec![Event::PointerMoved(first)]);
    h.frame(vec![]);
    assert!(h.open_canvas_group().is_none(), "pointing opens nothing");
    h.press(first, true, Modifiers::NONE);
    h.press(first, false, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.open_canvas_group(), Some(h.canvas_groups()[0].name));
}

#[test]
fn a_tall_enough_window_shows_the_palette_in_one_column() {
    let mut h = canvas();
    h.show_ribbon_tab("Toolpaths");
    h.frame(vec![]);
    h.frame(vec![]);
    let tall = palette(&h);
    h.window = egui::vec2(h.window.x, 420.0);
    h.frame(vec![]);
    h.frame(vec![]);
    let short = palette(&h);
    assert!(
        short.width() > 1.5 * tall.width(),
        "{tall:?} then {short:?}"
    );
}

#[test]
fn a_long_favourites_list_in_a_short_window_stays_inside_it() {
    let mut h = canvas();
    let every: Vec<String> = h
        .canvas_groups()
        .iter()
        .flat_map(|group| group.tools.iter().map(|id| (*id).to_string()))
        .collect();
    assert!(every.len() > 20, "{}", every.len());
    h.given_options(|o| o.favourites.design = every);
    h.show_ribbon_tab("Favourites");
    h.window = egui::vec2(900.0, 420.0);
    h.frame(vec![]);
    h.frame(vec![]);
    let list = h
        .ctx
        .memory(|m| m.area_rect(egui::Id::unique("canvas_flyout")))
        .expect("the favourites are listed");
    assert!(
        list.right() <= 900.0 && list.bottom() <= 420.0,
        "the extra columns scroll sideways: {list:?}"
    );
}

/// A left-click on a shape while the view's right-click menu is open shuts the menu
/// (issue #783, seen in this layout).
#[test]
fn clicking_a_shape_while_the_right_click_menu_is_open_closes_it() {
    let mut h = canvas();
    h.rectangle(10.0, 10.0, 40.0, 40.0);
    h.right_click(h.at(150.0, 150.0));
    assert!(h.text_at("Report a Bug Here…").is_some(), "the menu is open");
    h.click(h.at(10.0, 30.0), Modifiers::NONE);
    assert!(h.text_at("Report a Bug Here…").is_none(), "the menu is shut");
}
