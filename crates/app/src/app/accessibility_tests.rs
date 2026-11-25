//! What a screen reader is handed: the app's accessibility tree, read headlessly.

use super::*;
use eframe::egui::accesskit::{Role, TreeUpdate};

/// One frame with the accessibility tree on; the tree it built.
fn tree(h: &mut Harness) -> TreeUpdate {
    h.accessibility_tree()
}

/// Every number box's name, as a screen reader would say it.
fn number_boxes(tree: &TreeUpdate) -> Vec<String> {
    tree.nodes
        .iter()
        .filter(|(_, n)| n.role() == Role::SpinButton)
        .map(|(_, n)| n.label().unwrap_or_default().to_owned())
        .collect()
}

/// A name that is only a unit ("mm", "°") tells a screen reader user nothing.
fn only_a_unit(name: &str) -> bool {
    let name = name.trim();
    name.is_empty()
        || name.chars().count() <= 6 && !name.contains(char::is_uppercase)
        || name.starts_with(',')
}

fn assert_all_named(scene: &str, tree: &TreeUpdate) {
    let names = number_boxes(tree);
    assert!(!names.is_empty(), "{scene}: no number boxes found");
    let bad: Vec<_> = names.iter().filter(|n| only_a_unit(n)).collect();
    assert!(
        bad.is_empty(),
        "{scene}: number boxes a screen reader would hear only as their unit: {bad:?} \
         (all: {names:?})"
    );
}

#[test]
fn job_setup_number_boxes_are_heard_by_their_labels() {
    let mut h = Harness::new();
    h.invoke("job_size_and_position");
    let tree = tree(&mut h);
    assert_all_named("Job Setup", &tree);
    let names = number_boxes(&tree);
    assert!(
        names
            .iter()
            .any(|n| n.starts_with("Thickness") && n.ends_with(", mm")),
        "{names:?}"
    );
}

#[test]
fn toolpath_form_number_boxes_are_heard_by_their_labels() {
    let mut h = Harness::new();
    h.invoke("profile_toolpath");
    let tree = tree(&mut h);
    assert_all_named("Profile", &tree);
    let names = number_boxes(&tree);
    assert!(names.contains(&"Cut Depth, mm".to_owned()), "{names:?}");
}

#[test]
fn drawing_number_boxes_are_heard_by_their_labels() {
    let mut h = Harness::new();
    h.invoke("draw_rectangle");
    assert_all_named("Draw Rectangle", &tree(&mut h));
}

#[test]
fn number_box_arrows_are_not_tab_stops() {
    let mut h = Harness::new();
    h.invoke("profile_toolpath");
    let tree = tree(&mut h);
    let arrows: Vec<_> = tree
        .nodes
        .iter()
        .filter(|(_, n)| {
            n.label()
                .is_some_and(|l| l.starts_with("Increase ") || l.starts_with("Decrease "))
        })
        .collect();
    assert_ne!(arrows.len(), 0);
    assert!(
        arrows
            .iter()
            .all(|(_, n)| !n.supports_action(egui::accesskit::Action::Focus))
    );
}

/// The screen reader hand-off is built in: without eframe's `accesskit` feature the tree
/// above is made but never reaches Narrator, NVDA, Orca or `VoiceOver`.
#[test]
fn screen_reader_support_is_built_in() {
    let workspace = include_str!("../../../../Cargo.toml");
    let eframe = workspace
        .lines()
        .find(|l| l.starts_with("eframe = "))
        .expect("eframe in the workspace");
    assert!(eframe.contains("\"accesskit\""), "{eframe}");
}
