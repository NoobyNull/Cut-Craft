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
    h.palette_rect().expect("the palette is drawn")
}

/// The palette's first group's icon, under its grip.
fn first_group(h: &Harness) -> egui::Pos2 {
    palette(h).left_top() + egui::vec2(20.0, 30.0)
}

#[test]
fn pointing_at_a_palette_group_lists_its_tools_and_moving_away_closes_them() {
    let mut h = canvas();
    assert_eq!(
        h.options().general.canvas_flyouts,
        crate::options::CanvasFlyouts::OnHover
    );
    let first = first_group(&h);
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
    let first = first_group(&h);
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
    h.given_options(|o| {
        o.favourites.design = every;
        o.favourites.bar = crate::bars::Place::Layout;
    });
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

#[test]
fn the_palette_is_a_bar_its_grip_moves_snaps_and_turns() {
    let mut h = canvas();
    let start = palette(&h);
    assert!(start.height() > start.width(), "a column at first");
    // its grip, the dots along its top, moves it
    let grip = egui::pos2(start.center().x, start.top() + 7.0);
    h.drag(grip, grip + egui::vec2(200.0, 60.0));
    h.frame(vec![]);
    let moved = palette(&h);
    assert!(
        (moved.left() - start.left() - 200.0).abs() < 2.0,
        "{start:?} to {moved:?}"
    );
    assert!(matches!(
        h.options().canvas_palette,
        crate::bars::Place::Floating(f) if f.offset.is_some() && f.down
    ));
    // turned, it runs across
    h.invoke("toolbar:palette:turn");
    h.frame(vec![]);
    h.frame(vec![]);
    let across = palette(&h);
    assert!(across.width() > across.height(), "{across:?}");
    // snapped to the left, the drawing makes room for it
    let before = h.view_rect();
    h.invoke("toolbar:palette:snap:left");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.view_rect().left() > before.left() + 10.0, "pushed over");
    // and its groups still list their tools beside it
    h.open_canvas_group_list("Create Vectors");
    h.frame(vec![]);
    h.frame(vec![]);
    let circle = h.text_at("Draw Circle").expect("listed");
    assert!(
        circle.x > h.view_rect().left(),
        "beside the palette, over the drawing"
    );
}

/// A left-click on a shape while the view's right-click menu is open shuts the menu
/// (issue #783, seen in this layout).
#[test]
fn clicking_a_shape_while_the_right_click_menu_is_open_closes_it() {
    let mut h = canvas();
    h.rectangle(10.0, 10.0, 40.0, 40.0);
    h.right_click(h.at(150.0, 150.0));
    assert!(
        h.text_at("Report a Bug Here…").is_some(),
        "the menu is open"
    );
    h.click(h.at(10.0, 30.0), Modifiers::NONE);
    assert!(
        h.text_at("Report a Bug Here…").is_none(),
        "the menu is shut"
    );
}

/// A bar that comes to hold more grows to show it, rather than scrolling in the size it had:
/// the palette, squeezed into a short window, is one long column again in a tall one (it
/// stayed squeezed, a scrollbar on its first group, Matthew 2026-11-27).
#[test]
fn a_bar_grows_back_when_the_window_does() {
    let mut h = canvas();
    h.frame(vec![]);
    let tall = palette(&h);
    h.window = egui::vec2(h.window.x, 420.0);
    h.frame(vec![]);
    h.frame(vec![]);
    let window = h.window;
    h.window = egui::vec2(window.x, 1000.0);
    for _ in 0..3 {
        h.frame(vec![]);
    }
    let again = palette(&h);
    assert!(
        (again.height() - tall.height()).abs() < 2.0 && (again.width() - tall.width()).abs() < 2.0,
        "{tall:?} then {again:?}"
    );
}

/// The modeling and toolpath tools each have a bar of their own, both shown at once: no
/// header tab swaps one for the other.
#[test]
fn the_toolpaths_have_a_bar_of_their_own_beside_the_modeling_bar() {
    let mut h = canvas();
    let modeling = palette(&h);
    let toolpaths = h.toolpaths_bar_rect().expect("the toolpaths bar is drawn");
    assert!(modeling.center().x < 0.5 * h.window.x, "{modeling:?}");
    assert!(toolpaths.center().x > 0.5 * h.window.x, "{toolpaths:?}");
    let names =
        |groups: Vec<gui::ToolGroup>| -> Vec<&str> { groups.iter().map(|g| g.name).collect() };
    let modeling_groups = names(h.canvas_groups());
    assert!(modeling_groups.contains(&"Create Vectors"));
    assert!(
        modeling_groups.contains(&"Export Model"),
        "Model's groups too"
    );
    let toolpath_groups = names(h.canvas_toolpath_groups());
    assert!(toolpath_groups.contains(&"Cut Out and Clear"));
    assert!(toolpath_groups.contains(&"Save for the Machine"));
    // a toolpath group lists its tools beside its bar, on the drawing's side
    h.open_canvas_group_list("Holes");
    h.frame(vec![]);
    h.frame(vec![]);
    let drilling = h.text_at("Drilling Toolpath").expect("Holes is listed");
    assert!(
        drilling.x < toolpaths.left(),
        "{drilling:?} left of {toolpaths:?}"
    );
    assert!(
        palette(&h).width() < modeling.width() + 1.0,
        "the modeling bar stays as it was"
    );
}

#[test]
fn the_header_has_no_design_model_or_toolpaths_tab() {
    let mut h = canvas();
    // the header's row is the Find a Tool box's; the chip along the foot is the one
    // Toolpaths left, and the menus over the header keep theirs
    let find = h
        .text_at(&cut_craft_i18n::tr!("layouts-canvas-find"))
        .expect("the header");
    for name in ["Design", "Model", "Toolpaths"] {
        let row: Vec<_> = h
            .texts_at(name)
            .into_iter()
            .filter(|p| (p.y - find.y).abs() < 4.0)
            .collect();
        assert!(row.is_empty(), "{name}: {row:?}");
    }
    // nor does an old Toolpaths tab, kept from before, hide either bar
    h.show_ribbon_tab("Toolpaths");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.palette_rect().is_some() && h.toolpaths_bar_rect().is_some());
}

#[test]
fn either_bar_can_be_hidden_in_options() {
    for (modeling, toolpaths) in [(true, false), (false, true)] {
        let mut h = Harness::new();
        h.given_options(|o| {
            o.general.layout = WindowLayout::Canvas;
            o.general.canvas_modeling_bar = modeling;
            o.general.canvas_toolpaths_bar = toolpaths;
        });
        h.frame(vec![]);
        h.frame(vec![]);
        assert_eq!(h.palette_rect().is_some(), modeling);
        assert_eq!(h.toolpaths_bar_rect().is_some(), toolpaths);
    }
}

#[test]
fn the_toolpaths_bar_snaps_like_any_bar() {
    let mut h = canvas();
    h.invoke("toolbar:toolpaths_bar:snap:right");
    assert_eq!(
        h.options().canvas_toolpaths,
        crate::bars::Place::Snapped {
            side: crate::bars::Side::Right
        }
    );
    assert!(
        h.options().canvas_palette.is_column(),
        "the modeling bar stays"
    );
    h.frame(vec![]);
    h.frame(vec![]);
    let bar = h.toolpaths_bar_rect();
    assert!(bar.is_none_or(|r| r.right() > 0.9 * h.window.x), "{bar:?}");
}

#[test]
fn the_save_group_has_view_gcode_and_machine_files() {
    let h = canvas();
    let save = h
        .canvas_toolpath_groups()
        .into_iter()
        .find(|g| g.name == "Save for the Machine")
        .expect("the group");
    assert_eq!(
        save.tools,
        ["save_toolpaths", "view_gcode", "machine_files"]
    );
    assert_eq!(h.tool_label("view_gcode"), "View G-code");
}
