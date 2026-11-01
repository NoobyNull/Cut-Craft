//! Edit Loops: a window over a trochoidal toolpath's form for its loops. How wide they are,
//! how far each moves on, and which way they turn, with a picture of them at the tool's size.

use cut_craft_core::entry::{Loops, Turn, trochoid};
use cut_craft_core::geometry::Point;
use eframe::egui;
use serde_json::Value;

/// The groups holding a toolpath's Motion, under which Edit Loops… goes.
const UNDER: [&str; 2] = ["Machine Vectors…", "Clear Pocket"];

/// How many loops the picture shows at the least, and at the most.
const SHOWN: f64 = 3.0;
const MOST: f64 = 60.0;

/// The Edit Loops… button under a trochoidal toolpath's cut settings, and its window while
/// open. Whether the loops changed.
pub(super) fn edit_loops(ui: &mut egui::Ui, group: &str, params: &mut Value) -> bool {
    if !UNDER.contains(&group) || params["motion"] != "trochoidal" {
        return false;
    }
    let Ok(mut loops) = serde_json::from_value::<Loops>(params["loops"].clone()) else {
        return false;
    };
    let id = egui::Id::unique("edit_loops");
    let mut open = ui.data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
    if ui
        .button("Edit Loops…")
        .on_hover_text(
            "How wide the trochoidal loops are, how far each moves on, which way they turn",
        )
        .clicked()
    {
        open = true;
    }
    let before = loops;
    if open {
        let diameter = params["tool"]["diameter"].as_f64().unwrap_or(6.0).max(0.1);
        egui::Window::new("Edit Loops")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .show(ui.ctx(), |ui| editor(ui, &mut loops, diameter));
    }
    ui.data_mut(|d| d.insert_temp(id, open));
    let changed = before != loops;
    if changed {
        params["loops"] = serde_json::to_value(loops).expect("loops serialise");
    }
    changed
}

/// The window's inside: the settings over the picture.
fn editor(ui: &mut egui::Ui, loops: &mut Loops, diameter: f64) {
    egui::Grid::new("loops").num_columns(2).show(ui, |ui| {
        ui.label("Loop Width")
            .on_hover_text("How wide each loop is, of the tool's diameter");
        ui.add(egui::Slider::new(&mut loops.width, 10.0..=400.0).suffix(" %"));
        ui.end_row();
        ui.label("Loop Step")
            .on_hover_text("How far each loop moves on from the last, of the tool's diameter");
        ui.add(
            egui::Slider::new(&mut loops.step, 1.0..=100.0)
                .logarithmic(true)
                .suffix(" %"),
        );
        ui.end_row();
        ui.label("Loops Turn");
        ui.horizontal(|ui| {
            ui.radio_value(&mut loops.turn, Turn::Anticlockwise, "Anticlockwise")
                .on_hover_text("Each loop cuts climb");
            ui.radio_value(&mut loops.turn, Turn::Clockwise, "Clockwise")
                .on_hover_text("Each loop cuts conventional");
        });
        ui.end_row();
    });
    let (radius, advance) = loops.sized(diameter);
    ui.weak(format!(
        "Loops {:.2} mm across, each {:.2} mm on; the cutter {:.2} mm",
        2.0 * radius,
        advance,
        diameter
    ));
    picture(
        ui,
        radius,
        advance,
        diameter,
        loops.turn == Turn::Anticlockwise,
    );
}

/// The path (a line), the loops along it, and the cutter at the last loop's start, to scale,
/// the way on shown by an arrow.
fn picture(ui: &mut egui::Ui, radius: f64, advance: f64, diameter: f64, anticlockwise: bool) {
    let row = ui.spacing().interact_size.y;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(row * 20.0, row * 6.0), egui::Sense::hover());
    let visuals = ui.visuals().clone();
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, visuals.extreme_bg_color);
    let reach = radius + diameter / 2.0;
    // the cutter's reach either side of the line fills the height; as many loops as fit
    // across (a few at least)
    let gap = f64::from(ui.spacing().item_spacing.x);
    let wide = f64::from(rect.width()) - 2.0 * gap;
    let scale = ((f64::from(rect.height()) - 2.0 * gap) / (2.0 * reach))
        .min(wide / (SHOWN * advance + 2.0 * reach));
    let length = (wide / scale - 2.0 * reach).min(MOST * advance);
    #[expect(clippy::cast_possible_truncation, reason = "a place on the screen")]
    let at = |p: Point| {
        egui::pos2(
            rect.left() + (gap + (reach + p.x) * scale) as f32,
            rect.center().y - (p.y * scale) as f32,
        )
    };
    let line = [Point::ZERO, Point::new(length, 0.0)];
    painter.line_segment(
        [at(line[0]), at(line[1])],
        visuals.widgets.noninteractive.fg_stroke,
    );
    let path: Vec<egui::Pos2> = trochoid(&line, false, radius, advance, anticlockwise)
        .into_iter()
        .map(at)
        .collect();
    painter.add(egui::Shape::line(
        path.clone(),
        egui::Stroke::new(row / 12.0, visuals.selection.stroke.color),
    ));
    // the cutter where the last loop begins
    if let Some(&start) = path.iter().rev().nth(24) {
        #[expect(clippy::cast_possible_truncation, reason = "a size on the screen")]
        let r = (diameter / 2.0 * scale) as f32;
        painter.circle_stroke(
            start,
            r,
            egui::Stroke::new(row / 12.0, visuals.warn_fg_color),
        );
    }
    let end = at(line[1]);
    painter.arrow(
        end - egui::vec2(row, 0.0),
        egui::vec2(row, 0.0),
        visuals.widgets.noninteractive.fg_stroke,
    );
}
