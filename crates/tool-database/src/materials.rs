//! The Materials Library window, laid out as the Tool Database: the classes, each over its
//! materials softest first, on the left; the picked material's or class's settings on the
//! right, with the bits that have feeds and speeds for it. Add Tool picks a bit with the
//! tool selector and gives it feeds for the material. Edits are made to copies; only Apply
//! and OK hand them back.

use crate::manager::{length, percent, rate};
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_bug_report::number::Number;
use cut_craft_core::document::Units;
use cut_craft_core::material_class::MaterialClass;
use cut_craft_core::material_library::{ClassFeeds, Material, MaterialLibrary};
use cut_craft_core::toolpath::{CuttingData, RateUnits, ToolDatabase};
use eframe::egui;

/// What is picked in the tree.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MaterialPick {
    Class(MaterialClass),
    /// A material, by its index in the library.
    Material(usize),
}

/// What the window asks of the app.
#[derive(Debug, PartialEq)]
pub enum MaterialsOutcome {
    /// Keep the edited library and tools; leave the window open.
    Apply,
    /// Keep them and close the window.
    Ok,
    /// Close, dropping the edits.
    Cancel,
    /// Open the tool selector on [`MaterialsWindow::tools`]; the bit picked goes to
    /// [`MaterialsWindow::add_tool`].
    AddTool,
}

/// The open window.
pub struct MaterialsWindow {
    pub library: MaterialLibrary,
    /// The tools, whose feeds and speeds for the materials are edited here too.
    pub tools: ToolDatabase,
    pub pick: Option<MaterialPick>,
}

/// The window's share of the screen when it first opens.
const WINDOW_SHARE: f32 = 0.6;

impl MaterialsWindow {
    pub fn new(library: &MaterialLibrary, tools: &ToolDatabase) -> Self {
        Self {
            library: library.clone(),
            tools: tools.clone(),
            pick: None,
        }
    }

    /// The picked material's name, when one is picked.
    fn material(&self) -> Option<&Material> {
        match self.pick {
            Some(MaterialPick::Material(i)) => self.library.materials.get(i),
            _ => None,
        }
    }

    /// The bit `name` (picked with the tool selector) gets feeds and speeds for what is
    /// picked: its data as it cuts the material now, or for a class its data at both ends.
    pub fn add_tool(&mut self, name: &str) {
        let Some(i) = self.tools.index_of(name) else {
            return;
        };
        match self.pick {
            Some(MaterialPick::Material(m)) => {
                let Some(material) = self.library.materials.get(m).map(|m| m.name.clone()) else {
                    return;
                };
                let tool = &self.tools.tools[i];
                let data = CuttingData {
                    material: material.clone(),
                    ..tool.for_material_in(&material, &self.library).cutting("")
                };
                let tool = &mut self.tools.tools[i];
                if !tool
                    .materials
                    .iter()
                    .any(|d| d.material.eq_ignore_ascii_case(&material))
                {
                    tool.materials.push(data);
                }
                self.tools.add_material(&material);
            }
            Some(MaterialPick::Class(class)) => {
                let tool = &mut self.tools.tools[i];
                if !tool.class_feeds.iter().any(|f| f.class == class) {
                    let data = tool.cutting("");
                    tool.class_feeds.push(ClassFeeds {
                        class,
                        softest: data.clone(),
                        hardest: data,
                    });
                }
            }
            None => {}
        }
    }

    /// A new material in the picked class (Softwoods when none is), picked.
    pub fn new_material(&mut self) {
        let class = match self.pick {
            Some(MaterialPick::Class(c)) => c,
            Some(MaterialPick::Material(i)) => self.library.materials[i].class,
            None => MaterialClass::Softwoods,
        };
        let mut name = "New Material".to_owned();
        let mut n = 2;
        while self.library.get(&name).is_some() {
            name = format!("New Material {n}");
            n += 1;
        }
        self.library.materials.push(Material {
            name,
            class,
            hardness: 0.0,
            notes: String::new(),
        });
        self.pick = Some(MaterialPick::Material(self.library.materials.len() - 1));
    }

    /// Deletes the picked material, with every bit's feeds kept for it.
    pub fn delete_material(&mut self) {
        let Some(MaterialPick::Material(i)) = self.pick else {
            return;
        };
        let gone = self.library.materials.remove(i).name;
        for tool in &mut self.tools.tools {
            tool.materials
                .retain(|d| !d.material.eq_ignore_ascii_case(&gone));
        }
        self.pick = None;
    }

    /// Renames material `i`, its feeds on every bit following.
    fn rename(&mut self, i: usize, from: &str) {
        let to = self.library.materials[i].name.clone();
        for tool in &mut self.tools.tools {
            for d in &mut tool.materials {
                if d.material.eq_ignore_ascii_case(from) {
                    d.material.clone_from(&to);
                }
            }
        }
        for m in &mut self.tools.materials {
            if m.eq_ignore_ascii_case(from) {
                m.clone_from(&to);
            }
        }
        if self.tools.material.eq_ignore_ascii_case(from) {
            self.tools.material = to;
        }
    }

    /// Draws the window; what it asks of the app, if anything.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<MaterialsOutcome> {
        let mut open = true;
        let mut outcome = None;
        let size = ctx.content_rect().size() * WINDOW_SHARE;
        cut_craft_bug_report::dialog_with_panels(
            ctx,
            egui::Id::unique("materials_library_window"),
            "Materials Library",
            size,
            &mut open,
            |ui| {
                cut_craft_bug_report::mark(ui, "Materials Library window");
                egui::Panel::bottom("materials_buttons").show(ui, |ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Cancel").clicked() {
                            outcome = Some(MaterialsOutcome::Cancel);
                        }
                        if ui.button("OK").clicked() {
                            outcome = Some(MaterialsOutcome::Ok);
                        }
                        if ui.button("Apply").clicked() {
                            outcome = Some(MaterialsOutcome::Apply);
                        }
                    });
                });
                egui::Panel::left("materials_tree")
                    .resizable(true)
                    .default_size(size.x * 0.35)
                    .show(ui, |ui| {
                        egui::Panel::bottom("materials_tree_buttons").show(ui, |ui| {
                            ui.horizontal(|ui| {
                                if ui.button("New Material").clicked() {
                                    self.new_material();
                                }
                                let picked = self.material().is_some();
                                if ui
                                    .add_enabled(picked, egui::Button::new("Delete"))
                                    .on_hover_hint(
                                        "Deletes the material and every bit's feeds for it",
                                    )
                                    .clicked()
                                {
                                    self.delete_material();
                                }
                            });
                        });
                        egui::ScrollArea::both().show(ui, |ui| self.tree(ui));
                    });
                egui::CentralPanel::default().show(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        if self.details(ui) {
                            outcome = Some(MaterialsOutcome::AddTool);
                        }
                    });
                });
            },
        );
        if !open {
            outcome = Some(MaterialsOutcome::Cancel);
        }
        outcome
    }

    fn tree(&mut self, ui: &mut egui::Ui) {
        for class in MaterialClass::ALL {
            let id = ui.make_persistent_id(("material_class", class.number()));
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
                .show_header(ui, |ui| {
                    let text = format!("{}. {}", class.number(), class.name());
                    let picked = self.pick == Some(MaterialPick::Class(class));
                    let label = ui.selectable_label(picked, egui::RichText::new(text).strong());
                    if label.on_hover_hint(class.scale()).clicked() {
                        self.pick = Some(MaterialPick::Class(class));
                    }
                })
                .body(|ui| {
                    let mut order: Vec<usize> = (0..self.library.materials.len())
                        .filter(|&i| self.library.materials[i].class == class)
                        .collect();
                    order.sort_by(|&a, &b| {
                        let m = &self.library.materials;
                        m[a].hardness.total_cmp(&m[b].hardness)
                    });
                    for i in order {
                        let m = &self.library.materials[i];
                        ui.horizontal(|ui| {
                            let picked = self.pick == Some(MaterialPick::Material(i));
                            if ui.selectable_label(picked, &m.name).clicked() {
                                self.pick = Some(MaterialPick::Material(i));
                            }
                            if m.hardness > 0.0 {
                                ui.weak(format!("{}", m.hardness));
                            }
                        });
                    }
                });
        }
    }

    /// The picked material's or class's settings and bits; whether Add Tool was clicked.
    fn details(&mut self, ui: &mut egui::Ui) -> bool {
        match self.pick {
            Some(MaterialPick::Material(i)) if i < self.library.materials.len() => {
                self.material_details(ui, i)
            }
            Some(MaterialPick::Class(class)) => self.class_details(ui, class),
            _ => {
                ui.weak("Pick a class or material in the list");
                false
            }
        }
    }

    fn material_details(&mut self, ui: &mut egui::Ui, i: usize) -> bool {
        let before = self.library.materials[i].name.clone();
        let m = &mut self.library.materials[i];
        egui::Grid::new("material_details")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Name");
                ui.text_edit_singleline(&mut m.name);
                ui.end_row();
                ui.label("Class");
                egui::ComboBox::from_id_salt("material_class")
                    .selected_text(format!("{}. {}", m.class.number(), m.class.name()))
                    .show_ui(ui, |ui| {
                        for c in MaterialClass::ALL {
                            let text = format!("{}. {}", c.number(), c.name());
                            ui.selectable_value(&mut m.class, c, text);
                        }
                    });
                ui.end_row();
                ui.label("Hardness").on_hover_hint(
                    "Where it sits in its class: a bit's feeds for the class are placed \
                     between its softest and hardest materials by this",
                );
                ui.horizontal(|ui| {
                    ui.add(Number::new(&mut m.hardness).range(0.0..=f64::MAX));
                    ui.weak(m.class.scale());
                });
                ui.end_row();
                ui.label("Notes");
                ui.text_edit_multiline(&mut m.notes);
                ui.end_row();
            });
        let name = m.name.clone();
        if name != before && !name.trim().is_empty() {
            self.rename(i, &before);
        }
        ui.separator();
        ui.strong("Bits with feeds and speeds for it");
        let library = self.library.clone();
        let mut remove = None;
        for (t, tool) in self.tools.tools.iter_mut().enumerate() {
            let Some(at) = tool
                .materials
                .iter()
                .position(|d| d.material.eq_ignore_ascii_case(&name))
            else {
                continue;
            };
            let mut data = tool.materials[at].clone();
            ui.push_id(("material_tool", t), |ui| {
                ui.horizontal(|ui| {
                    ui.label(&tool.name);
                    if ui
                        .small_button("×")
                        .on_hover_hint("Remove its feeds for this material")
                        .clicked()
                    {
                        remove = Some((t, at));
                    }
                });
                if data_row(ui, &mut data, tool.units, tool.rate_units) {
                    tool.materials[at] = data;
                }
            });
        }
        if let Some((t, at)) = remove {
            self.tools.tools[t].materials.remove(at);
        }
        // the bits that cut it from their class's feeds, to see
        let class = self.library.materials[i].class;
        for tool in &self.tools.tools {
            let own = tool
                .materials
                .iter()
                .any(|d| d.material.eq_ignore_ascii_case(&name));
            if !own && tool.class_feeds.iter().any(|f| f.class == class) {
                let cut = tool.for_material_in(&name, &library);
                ui.weak(format!(
                    "{}: from its {} range, feed {:.0} mm/min at {:.0} r.p.m.",
                    tool.name,
                    class.name(),
                    cut.feed_rate,
                    cut.spindle_speed
                ));
            }
        }
        ui.button("Add Tool…")
            .on_hover_hint(
                "Pick a bit with the tool selector: it gets feeds and speeds for this material",
            )
            .clicked()
    }

    fn class_details(&mut self, ui: &mut egui::Ui, class: MaterialClass) -> bool {
        ui.heading(format!("{}. {}", class.number(), class.name()));
        let members = self.library.in_class(class);
        if let (Some(soft), Some(hard)) = (members.first(), members.last()) {
            ui.weak(format!(
                "From {} ({}) to {} ({}), {}: a material between cuts with feeds between, \
                 by its hardness, unless it has feeds of its own",
                soft.name,
                soft.hardness,
                hard.name,
                hard.hardness,
                class.scale()
            ));
        }
        ui.separator();
        ui.strong("Bits with feeds and speeds for the class");
        let mut remove = None;
        for (t, tool) in self.tools.tools.iter_mut().enumerate() {
            let Some(at) = tool.class_feeds.iter().position(|f| f.class == class) else {
                continue;
            };
            let (units, rates) = (tool.units, tool.rate_units);
            ui.push_id(("class_tool", t), |ui| {
                ui.horizontal(|ui| {
                    ui.label(&tool.name);
                    if ui
                        .small_button("×")
                        .on_hover_hint("Remove its feeds for this class")
                        .clicked()
                    {
                        remove = Some((t, at));
                    }
                });
                let feeds = &mut tool.class_feeds[at];
                ui.push_id("softest", |ui| {
                    ui.weak("Softest");
                    data_row(ui, &mut feeds.softest, units, rates);
                });
                ui.push_id("hardest", |ui| {
                    ui.weak("Hardest");
                    data_row(ui, &mut feeds.hardest, units, rates);
                });
            });
        }
        if let Some((t, at)) = remove {
            self.tools.tools[t].class_feeds.remove(at);
        }
        ui.button("Add Tool…")
            .on_hover_hint(
                "Pick a bit with the tool selector: it gets feeds and speeds for the whole class",
            )
            .clicked()
    }
}

/// One set of cutting data in a row: feed, plunge, spindle, pass depth and stepovers.
/// Whether any changed.
fn data_row(ui: &mut egui::Ui, d: &mut CuttingData, units: Units, rates: RateUnits) -> bool {
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        ui.label("Feed");
        changed |= rate(ui, &mut d.feed_rate, rates);
        ui.label("Plunge");
        changed |= rate(ui, &mut d.plunge_rate, rates);
        ui.label("Spindle");
        changed |= ui
            .add(
                Number::new(&mut d.spindle_speed)
                    .range(0.0..=f64::MAX)
                    .suffix(" r.p.m."),
            )
            .changed();
        ui.label("Pass Depth");
        changed |= length(ui, &mut d.pass_depth, units);
        ui.label("Clearing");
        changed |= percent(ui, &mut d.stepover);
        ui.label("Finishing");
        changed |= percent(ui, &mut d.finish_stepover);
    });
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window() -> MaterialsWindow {
        MaterialsWindow::new(&MaterialLibrary::starter(), &ToolDatabase::starter())
    }

    fn at(w: &MaterialsWindow, name: &str) -> usize {
        w.library
            .materials
            .iter()
            .position(|m| m.name == name)
            .unwrap()
    }

    #[test]
    fn add_tool_gives_the_bit_feeds_for_the_material_or_its_class() {
        let mut w = window();
        let bit = w.tools.tools[0].name.clone();
        w.pick = Some(MaterialPick::Material(at(&w, "Hard Maple")));
        w.add_tool(&bit);
        w.add_tool(&bit);
        let tool = w.tools.get(&bit).unwrap();
        assert_eq!(
            tool.materials
                .iter()
                .filter(|d| d.material == "Hard Maple")
                .count(),
            1,
            "once"
        );
        assert!(w.tools.materials.iter().any(|m| m == "Hard Maple"));
        w.pick = Some(MaterialPick::Class(MaterialClass::Softwoods));
        w.add_tool(&bit);
        let tool = w.tools.get(&bit).unwrap();
        assert_eq!(tool.class_feeds.len(), 1);
        assert_eq!(tool.class_feeds[0].class, MaterialClass::Softwoods);
    }

    #[test]
    fn renaming_or_deleting_a_material_takes_its_feeds_on_every_bit() {
        let mut w = window();
        let bit = w.tools.tools[0].name.clone();
        let i = at(&w, "Cherry");
        w.pick = Some(MaterialPick::Material(i));
        w.add_tool(&bit);
        w.library.materials[i].name = "Black Cherry".into();
        w.rename(i, "Cherry");
        let tool = w.tools.get(&bit).unwrap();
        assert!(tool.materials.iter().any(|d| d.material == "Black Cherry"));
        w.delete_material();
        let tool = w.tools.get(&bit).unwrap();
        assert!(!tool.materials.iter().any(|d| d.material == "Black Cherry"));
        assert!(w.library.get("Black Cherry").is_none());
    }

    #[test]
    fn new_materials_go_in_the_picked_class_with_a_name_of_their_own() {
        let mut w = window();
        w.pick = Some(MaterialPick::Class(MaterialClass::Acrylics));
        w.new_material();
        w.pick = Some(MaterialPick::Class(MaterialClass::Acrylics));
        w.new_material();
        let Some(MaterialPick::Material(i)) = w.pick else {
            panic!()
        };
        assert_eq!(w.library.materials[i].name, "New Material 2");
        assert_eq!(w.library.materials[i].class, MaterialClass::Acrylics);
    }

    #[test]
    fn the_window_draws_each_pick() {
        let ctx = egui::Context::default();
        let mut w = window();
        let bit = w.tools.tools[0].name.clone();
        for pick in [
            None,
            Some(MaterialPick::Class(MaterialClass::Hardwoods)),
            Some(MaterialPick::Material(0)),
        ] {
            w.pick = pick;
            w.add_tool(&bit);
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 800.0),
                )),
                ..Default::default()
            };
            ctx.run_pass(input, |ctx| {
                assert_eq!(w.show(ctx), None);
            })
            .textures_delta
            .clear();
        }
    }
}
