//! Edit Passes: a window over a toolpath form showing each pass down to the cut depth as a
//! line through the material, where passes are clicked on (or off), dragged, spread evenly,
//! or made to leave a thin last pass. Kept on the form's tool as `passes` and `last_pass`.

use super::trimmed;
use crate::form;
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_bug_report::number::Number;
use cut_craft_core::document::Units;
use cut_craft_core::toolpath::Tool;
use eframe::egui;
use serde_json::Value;

/// The Edit Passes… button under a form's tool, and its window while open. Whether the
/// passes changed.
pub(super) fn edit_passes(ui: &mut egui::Ui, params: &mut Value, units: Units) -> bool {
    // ponytail: forms whose depth is `depth`; the V-carve and 3D forms keep theirs elsewhere
    let Some(depth) = params["depth"].as_f64().filter(|d| *d > 0.0) else {
        return false;
    };
    let Ok(mut tool) = serde_json::from_value::<Tool>(params["tool"].clone()) else {
        return false;
    };
    let id = egui::Id::unique("edit_passes");
    let mut open = ui.data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
    if ui
        .button("Edit Passes…")
        .on_hover_hint("See and set each pass down to the cut depth")
        .clicked()
    {
        open = true;
    }
    let before = (tool.passes.clone(), tool.last_pass);
    if open {
        egui::Window::new("Edit Passes")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .show(ui.ctx(), |ui| editor(ui, &mut tool, depth, units));
    }
    ui.data_mut(|d| d.insert_temp(id, open));
    let changed = before != (tool.passes.clone(), tool.last_pass);
    if changed {
        params["tool"] = serde_json::to_value(&tool).expect("a tool serialises");
    }
    changed
}

/// The window's inside: the picture of the passes beside the ways to set them.
fn editor(ui: &mut egui::Ui, tool: &mut Tool, depth: f64, units: Units) {
    let (per, unit) = form::shown_in("mm", units).unwrap_or((1.0, "mm"));
    let show = |mm: f64| format!("{} {unit}", trimmed(mm / per));
    let depths: Vec<f64> = tool.pass_depths(depth).iter().map(|z| -z).collect();
    ui.label(format!(
        "Cut Depth {} in {} passes",
        show(depth),
        depths.len()
    ));
    ui.horizontal(|ui| {
        if let Some(set) = picture(ui, &depths, depth, &show) {
            tool.passes = set;
            tool.last_pass = 0.0;
        }
        ui.vertical(|ui| controls(ui, tool, depth, depths.len(), per, unit));
    });
    ui.weak("Click the picture to add a pass or take one away; drag a pass to move it");
}

/// The material from its top to the cut depth, each pass a line across it with its depth,
/// the last (the cut depth) heavier. The passes above the last as they are after a click
/// or drag, when one changed them.
fn picture(
    ui: &mut egui::Ui,
    depths: &[f64],
    depth: f64,
    show: &dyn Fn(f64) -> String,
) -> Option<Vec<f64>> {
    let row = ui.spacing().interact_size.y;
    let gap = ui.spacing().item_spacing.x;
    let grab = ui.style().interaction.resize_grab_radius_side;
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(row * 7.0, row * 12.0),
        egui::Sense::click_and_drag(),
    );
    // the material over the left half, a row free above and below it
    let block = egui::Rect::from_min_max(
        rect.min + egui::vec2(0.0, row),
        egui::pos2(rect.center().x, rect.bottom() - row),
    );
    #[expect(clippy::cast_possible_truncation, reason = "a place on the screen")]
    let y_of = |d: f64| block.top() + (d / depth) as f32 * block.height();
    let d_of = |y: f32| (f64::from((y - block.top()) / block.height()) * depth).clamp(0.0, depth);
    let visuals = ui.visuals().clone();
    let painter = ui.painter_at(rect);
    painter.rect(
        block,
        0.0,
        visuals.widgets.inactive.weak_bg_fill,
        visuals.widgets.noninteractive.fg_stroke,
        egui::StrokeKind::Inside,
    );
    let font = egui::TextStyle::Small.resolve(ui.style());
    let label = |y: f32, text: String, colour: egui::Color32| {
        painter.text(
            egui::pos2(block.right() + gap, y),
            egui::Align2::LEFT_CENTER,
            text,
            font.clone(),
            colour,
        );
    };
    label(block.top(), "Top".to_owned(), visuals.weak_text_color());
    let last = depths.len().saturating_sub(1);
    for (i, &d) in depths.iter().enumerate() {
        let (stroke, colour) = if i == last {
            (
                egui::Stroke::new(grab / 2.0, visuals.warn_fg_color),
                visuals.warn_fg_color,
            )
        } else {
            (
                egui::Stroke::new(grab / 3.0, visuals.selection.bg_fill),
                visuals.text_color(),
            )
        };
        painter.hline(block.x_range(), y_of(d), stroke);
        label(y_of(d), show(d), colour);
    }
    // where a click would put a pass
    if let Some(at) = response.hover_pos() {
        painter.hline(
            block.x_range(),
            at.y,
            visuals.widgets.noninteractive.bg_stroke,
        );
    }
    let above = &depths[..last];
    let near = |y: f32| above.iter().position(|&d| (y_of(d) - y).abs() <= grab);
    let held = response.id.with("held");
    let at = response.interact_pointer_pos()?;
    let mut set = above.to_vec();
    if response.drag_started() {
        ui.data_mut(|d| d.insert_temp(held, near(at.y)));
    }
    if response.dragged() {
        let i = ui.data(|d| d.get_temp::<Option<usize>>(held)).flatten()?;
        set[i] = d_of(at.y);
    } else if response.clicked() {
        match near(at.y) {
            Some(i) => {
                set.remove(i);
            }
            None => set.push(d_of(at.y)),
        }
    } else {
        return None;
    }
    set.retain(|&d| d > 0.0 && d < depth);
    if set.is_empty() {
        set.push(depth); // one pass, straight to the cut depth
    }
    Some(set)
}

/// The number of passes and Spread Evenly, a last pass thickness, and back to the tool's
/// own pass depth.
fn controls(ui: &mut egui::Ui, tool: &mut Tool, depth: f64, count: usize, per: f64, unit: &str) {
    let id = egui::Id::unique("edit_passes_count");
    let mut wanted = ui.data(|d| d.get_temp::<usize>(id)).unwrap_or(count);
    ui.horizontal(|ui| {
        ui.label("Passes");
        ui.add(Number::new(&mut wanted).range(1..=100));
    });
    ui.data_mut(|d| d.insert_temp(id, wanted));
    if ui
        .button("Spread Evenly")
        .on_hover_hint("That many passes, each as deep as the next")
        .clicked()
    {
        #[expect(clippy::cast_precision_loss, reason = "a small count")]
        let step = depth / wanted as f64;
        #[expect(clippy::cast_precision_loss, reason = "a small count")]
        let set = (1..=wanted).map(|i| step * i as f64).collect();
        tool.passes = set;
        tool.last_pass = 0.0;
    }
    ui.separator();
    let mut thin = tool.last_pass > 0.0;
    if ui
        .checkbox(&mut thin, "Last Pass Thickness")
        .on_hover_hint("A thin last pass for a clean floor; the passes above it even")
        .changed()
    {
        tool.passes.clear();
        tool.last_pass = if thin {
            (depth / 10.0).min(tool.pass_depth)
        } else {
            0.0
        };
    }
    if thin {
        let mut shown = tool.last_pass / per;
        if ui
            .add(
                Number::new(&mut shown)
                    .range(0.0..=depth / per)
                    .speed(0.01)
                    .suffix(format!(" {unit}")),
            )
            .changed()
        {
            tool.passes.clear();
            tool.last_pass = shown * per;
        }
    }
    ui.separator();
    if ui
        .add_enabled(
            !tool.passes.is_empty() || tool.last_pass > 0.0,
            egui::Button::new("From Pass Depth"),
        )
        .on_hover_hint("Even passes no deeper than the tool's pass depth, as before")
        .clicked()
    {
        tool.passes.clear();
        tool.last_pass = 0.0;
    }
}
