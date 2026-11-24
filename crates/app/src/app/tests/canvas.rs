//! The Canvas layout: the view has the whole window, and the tools, the open form and
//! the job's lists float over it.

use super::*;
use crate::options::WindowLayout;

fn canvas() -> Harness {
    let mut h = Harness::new();
    h.app.options.general.layout = WindowLayout::Canvas;
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
    let create = h
        .app
        .canvas_groups()
        .iter()
        .position(|g| g.name == "Create Vectors")
        .unwrap();
    h.app.canvas.group = Some(create);
    // a box that just opened is measured first, then drawn
    h.frame(vec![]);
    h.frame(vec![]);
    click_text(&mut h, "Draw Circle");
    assert_eq!(h.app.form.as_ref().map(|f| f.id), Some("draw_circle"));
    assert!(h.app.canvas.group.is_none(), "the list closes on a pick");
    let form = h
        .text_at("Draw Circle")
        .expect("the form floats over the view");
    assert!(form.x > 0.5 * h.window.x, "on the right: {form:?}");
}

#[test]
fn a_chip_opens_its_list_over_the_view_and_closes_again() {
    let mut h = canvas();
    assert!(h.app.canvas.chip.is_none());
    click_text(&mut h, "Layers");
    let layers = INSPECTOR_TABS.iter().position(|t| *t == "Layers");
    assert_eq!(h.app.canvas.chip, layers);
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
    assert!(h.app.canvas.chip.is_none());
}

#[test]
fn find_a_tool_is_in_the_header_and_enter_runs_the_first_match() {
    let mut h = canvas();
    let hint = cut_craft_i18n::tr!("layouts-canvas-find");
    assert!(h.text_at(&hint).is_some(), "the box shows in the header");
    h.app.invoke(&h.ctx, "find_tool");
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
    assert_eq!(h.app.form.as_ref().map(|f| f.id), Some("draw_circle"));
    assert!(h.app.search.is_none());
}
