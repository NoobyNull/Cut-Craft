//! The exact widget under a right-click, from egui's accessibility tree: its kind and label
//! and the labelled parts it sits in, innermost first (`Button "Apply" in Window "Job Setup"`).

use eframe::egui::{self, accesskit};
use std::collections::HashMap;

/// Where the widget under the last right-click is kept for the reporter.
pub(crate) fn id() -> egui::Id {
    egui::Id::unique("bug_report_spotted")
}

/// Turns the accessibility tree on and, after a pass with a right-click, keeps what was
/// under the pointer at [`id`].
// ponytail: the tree is built every frame so the right-click's own pass has it; build it
// only while a report could be wanted if it ever shows in a profile
pub(crate) struct Spotter;

impl egui::Plugin for Spotter {
    fn debug_name(&self) -> &'static str {
        "Report a Bug: the widget clicked"
    }

    fn setup(&mut self, ctx: &egui::Context) {
        ctx.enable_accesskit();
    }

    fn output_hook(&mut self, ctx: &egui::Context, output: &mut egui::FullOutput) {
        let Some(at) = ctx.input(|i| {
            i.pointer
                .secondary_clicked()
                .then(|| i.pointer.interact_pos())
                .flatten()
        }) else {
            return;
        };
        let found = output
            .platform_output
            .accesskit_update
            .as_ref()
            .and_then(|tree| widget_at(tree, at));
        ctx.data_mut(|d| d.insert_temp(id(), found));
    }
}

/// The smallest labelled widget holding `at`, then each labelled part it is in.
pub fn widget_at(tree: &accesskit::TreeUpdate, at: egui::Pos2) -> Option<String> {
    let (x, y) = (f64::from(at.x), f64::from(at.y));
    // strictly inside: a menu opened by the click has its corner exactly at the pointer
    let holds = |n: &accesskit::Node| {
        n.bounds()
            .is_some_and(|b| b.x0 < x && x < b.x1 && b.y0 < y && y < b.y1)
    };
    let area = |n: &accesskit::Node| n.bounds().map_or(f64::INFINITY, |b| b.area());
    let nodes: HashMap<_, _> = tree.nodes.iter().map(|(id, n)| (*id, n)).collect();
    let parents: HashMap<_, _> = tree
        .nodes
        .iter()
        .flat_map(|(id, n)| n.children().iter().map(move |child| (*child, *id)))
        .collect();
    let mut id = tree
        .nodes
        .iter()
        .filter(|(_, n)| holds(n) && described(n).is_some())
        .min_by(|a, b| area(&a.1).total_cmp(&area(&b.1)))?
        .0;
    let mut path = vec![];
    loop {
        path.extend(nodes.get(&id).and_then(|n| described(n)));
        match parents.get(&id) {
            Some(parent) => id = *parent,
            None => break,
        }
    }
    Some(path.join(" in "))
}

/// A node's kind and label (or value, for a text box), when it has either.
fn described(node: &accesskit::Node) -> Option<String> {
    let label = node.label().or_else(|| node.value())?.trim();
    (!label.is_empty()).then(|| format!("{:?} \"{label}\"", node.role()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_button_under_the_pointer_is_named_with_the_window_it_is_in() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut at = egui::Pos2::ZERO;
        let mut pass = || {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 600.0),
                )),
                ..Default::default()
            };
            let mut output = ctx.run_pass(input, |ctx| {
                egui::Window::new("Job Setup").show(ctx, |ui| {
                    ui.label("Width");
                    at = ui.button("Apply").rect.center();
                });
            });
            // no renderer takes a headless pass's texture changes
            output.textures_delta.clear();
            output
        };
        // a window is sized on its first pass and shown from its second
        pass();
        let output = pass();
        let tree = output.platform_output.accesskit_update.expect("a tree");
        let found = widget_at(&tree, at).expect("something there");
        assert!(found.starts_with("Button \"Apply\""), "{found}");
        assert!(found.contains("\"Job Setup\""), "{found}");
        assert_eq!(widget_at(&tree, egui::pos2(799.0, 599.0)), None);
    }
}
