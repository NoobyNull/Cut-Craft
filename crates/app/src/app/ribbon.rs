//! The Ribbon interface (Options ▸ Appearance ▸ Interface style), laid out as Fusion's:
//! a quick-access bar and tabs across the top, each tab's groups below them showing as many
//! icons as the width has room for over the group's name, which drops down all its tools by
//! name; a browser of the job down the left; the open tool's form on the right.

use super::{CutCraftApp, DESIGN_PANEL_WIDTHS, TOOL_ICON_ROWS, egui, gui};

/// The ribbon's tabs and the toolbar groups each holds, by name.
pub(super) const RIBBON_TABS: [(&str, &[&str]); 4] = [
    (
        "Create",
        &["Create Vectors", "Offset and Layout", "Array Objects"],
    ),
    (
        "Modify",
        &["Edit Objects", "Transform Objects", "Align Objects"],
    ),
    ("Model", &["Shape Creation", "Shape Editing"]),
    (
        "Toolpaths",
        &[
            "Material",
            "Toolpath Operations",
            "3D Toolpaths",
            "Laser",
            "Toolpath Templates",
            "Output",
        ],
    ),
];
/// The group whose icons are the quick-access bar, before the tabs.
pub(super) const QUICK_ACCESS: &str = "File Operations";
/// A group's drop-down arrow, as shares of its square: egui's combo box arrow.
const ARROW_SHARE: [f32; 2] = [0.7, 0.45];
/// Groups the ribbon leaves out: the view has these buttons on it already.
#[cfg(test)]
pub(super) const LEFT_OUT: [&str; 1] = ["2D View Control"];

impl CutCraftApp {
    /// The job's ribbon, browser and form around the views.
    pub(super) fn ribbon_pages(&mut self, ui: &mut egui::Ui) {
        let width = DESIGN_PANEL_WIDTHS * ui.global_style().spacing.interact_size.x;
        egui::Panel::top("ribbon").show(ui, |ui| self.ribbon(ui));
        egui::Panel::left("browser")
            .default_size(width)
            .show(ui, |ui| self.browser(ui));
        if self.form.is_some() {
            egui::Panel::right("ribbon_form")
                .default_size(width)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| self.command_form(ui));
                });
        }
    }

    /// A toolbar group by name.
    pub(super) fn toolbar_group(&self, name: &str) -> Option<gui::ToolGroup> {
        let t = &self.toolbar;
        t.groups
            .iter()
            .chain(&t.modeling_groups)
            .chain(&t.toolpath_groups)
            .find(|g| g.name == name)
            .copied()
    }

    fn ribbon(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "Ribbon");
        let mut clicked = None;
        ui.horizontal(|ui| {
            // the quick-access bar: small icons, as Fusion's file and save buttons
            let small = ui.spacing().interact_size.y;
            for &id in self
                .toolbar_group(QUICK_ACCESS)
                .map_or(&[][..], |g| g.tools)
            {
                if let Some(id) = self.icon_tool(ui, id, small) {
                    clicked = Some(id);
                }
            }
            ui.separator();
            for (i, (name, _)) in RIBBON_TABS.iter().enumerate() {
                if ui.selectable_label(i == self.ribbon_tab, *name).clicked() {
                    self.ribbon_tab = i;
                }
            }
        });
        let (_, names) = RIBBON_TABS[self.ribbon_tab.min(RIBBON_TABS.len() - 1)];
        let groups: Vec<gui::ToolGroup> =
            names.iter().filter_map(|n| self.toolbar_group(n)).collect();
        let edge = TOOL_ICON_ROWS * ui.spacing().interact_size.y;
        let spacing = ui.spacing().clone();
        let slot = edge + 2.0 * spacing.button_padding.x + spacing.item_spacing.x;
        let font = egui::TextStyle::Button.resolve(ui.style());
        let labels: Vec<f32> = groups
            .iter()
            .map(|g| {
                let text = ui.painter().layout_no_wrap(
                    g.name.to_owned(),
                    font.clone(),
                    egui::Color32::PLACEHOLDER,
                );
                // the name, its arrow and the gap between them, inside the button's padding
                text.size().x
                    + font.size
                    + spacing.icon_spacing
                    + 2.0 * spacing.button_padding.x
                    + spacing.item_spacing.x
            })
            .collect();
        // icon tools fill the strip; ones without an icon are in the drop-down only
        let icons: Vec<Vec<&'static str>> = groups
            .iter()
            .map(|g| {
                g.tools
                    .iter()
                    .copied()
                    .filter(|&id| self.has_icon(id))
                    .collect()
            })
            .collect();
        let counts: Vec<usize> = icons.iter().map(Vec::len).collect();
        // each group is followed by a separator
        let gaps = as_width(groups.len()) * (2.0 * spacing.item_spacing.x + 1.0);
        let shown = fit(&labels, &counts, slot, ui.available_width() - gaps);
        // every group's icons in a row of the same height, so the names line up beneath
        let row = egui::vec2(0.0, edge + 2.0 * spacing.button_padding.y);
        let centred =
            egui::Layout::left_to_right(egui::Align::Center).with_main_align(egui::Align::Center);
        ui.horizontal_top(|ui| {
            for (i, group) in groups.iter().enumerate() {
                let width = labels[i].max(as_width(shown[i]) * slot);
                ui.allocate_ui_with_layout(
                    egui::vec2(width, ui.available_height()),
                    egui::Layout::top_down(egui::Align::Center),
                    |ui| {
                        let row = egui::vec2(width, row.y);
                        ui.allocate_ui_with_layout(row, centred, |ui| {
                            ui.set_min_size(row);
                            for &id in &icons[i][..shown[i]] {
                                if let Some(id) = self.icon_tool(ui, id, edge) {
                                    clicked = Some(id);
                                }
                            }
                        });
                        group_menu(ui, group.name, font.size, |ui| {
                            for &id in group.tools {
                                let label = self.tool_label(id);
                                let button = egui::Button::new(label);
                                if ui.add_enabled(self.available(id), button).clicked() {
                                    clicked = Some(id);
                                }
                            }
                        });
                    },
                );
                ui.separator();
            }
        });
        if let Some(id) = clicked {
            self.invoke(ui.ctx(), id);
        }
    }

    /// A tool's icon button, picked out while it is the tool in hand; its id when clicked.
    fn icon_tool(&self, ui: &mut egui::Ui, id: &'static str, edge: f32) -> Option<&'static str> {
        let label = self.tool_label(id);
        let explicit = self.registry.get(id).map_or("", |c| c.meta().icon);
        let button = self
            .icon_button(explicit, id, &label, edge)
            .selected(self.status.current_tool == label);
        ui.add_enabled(self.available(id), button)
            .on_hover_text(&label)
            .on_disabled_hover_text(format!("{label}: not implemented yet"))
            .clicked()
            .then_some(id)
    }

    fn has_icon(&self, id: &str) -> bool {
        let explicit = self.registry.get(id).map_or("", |c| c.meta().icon);
        self.icons.find(explicit, id).is_some()
    }

    fn tool_label(&self, id: &str) -> String {
        self.registry
            .get(id)
            .map_or_else(|| gui::title_case(id), |c| c.meta().label.to_owned())
    }

    /// Fusion's browser: the job's parts as sections that fold away.
    fn browser(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "Browser");
        egui::ScrollArea::vertical().show(ui, |ui| {
            let section =
                |name: &str, open: bool| egui::CollapsingHeader::new(name).default_open(open);
            section("Sheets", false).show(ui, |ui| self.sheets_panel(ui));
            section("Layers", true).show(ui, |ui| self.layers_panel(ui));
            section("Components", false).show(ui, |ui| self.component_list(ui));
            section("Toolpaths", true).show(ui, |ui| self.toolpath_list(ui));
            section("Clipart", false).show(ui, |ui| self.clipart_panel(ui));
        });
    }
}

/// A group's name, which drops down its tools, with an arrow painted after it as egui's
/// combo boxes have (the UI font has no arrow character); `edge` is the arrow's room.
fn group_menu(ui: &mut egui::Ui, name: &str, edge: f32, add: impl FnOnce(&mut egui::Ui)) {
    let room = egui::Atom::custom(egui::IdSalt::new("arrow"), egui::vec2(edge, edge));
    let button = egui::Button::new(name).right_text(room);
    let (response, _) = egui::containers::menu::MenuButton::from_button(button).ui(ui, add);
    let padding = ui.spacing().button_padding.x;
    let at = egui::pos2(
        response.rect.right() - padding - edge / 2.0,
        response.rect.center().y,
    );
    // egui's combo box arrow: a downward triangle in the middle of its square
    let [w, h] = ARROW_SHARE;
    let arrow = egui::Rect::from_center_size(at, egui::vec2(edge * w, edge * h));
    let colour = ui.style().interact(&response).fg_stroke.color;
    ui.painter()
        .add(egui::Shape::rotated_triangle(arrow, 0.0, colour));
}

#[expect(
    clippy::cast_precision_loss,
    reason = "counts of groups and icons are small"
)]
fn as_width(n: usize) -> f32 {
    n as f32
}

/// How many icons each group shows: one more at a time to each group in turn while the row
/// still fits `room`. A group is as wide as its name (`labels`) or its icons (`slot` each).
fn fit(labels: &[f32], counts: &[usize], slot: f32, room: f32) -> Vec<usize> {
    let width = |i: usize, n: usize| labels[i].max(as_width(n) * slot);
    let mut shown = vec![0; counts.len()];
    let mut used: f32 = labels.iter().sum();
    loop {
        let mut grew = false;
        for i in 0..counts.len() {
            let extra = width(i, shown[i] + 1) - width(i, shown[i]);
            if shown[i] < counts[i] && used + extra <= room {
                shown[i] += 1;
                used += extra;
                grew = true;
            }
        }
        if !grew {
            return shown;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fit;

    #[test]
    fn groups_share_the_width_and_never_show_more_than_they_have() {
        // room for the names and six icons: the small group takes its one, the others share
        assert_eq!(
            fit(&[10.0, 10.0, 10.0], &[5, 1, 5], 10.0, 60.0),
            vec![3, 1, 2]
        );
        assert_eq!(
            fit(&[10.0], &[3], 10.0, 1000.0),
            vec![3],
            "all when there is room"
        );
        assert_eq!(
            fit(&[10.0, 10.0], &[4, 4], 10.0, 5.0),
            vec![0, 0],
            "names only"
        );
    }
}
