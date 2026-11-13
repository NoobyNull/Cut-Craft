//! Cutting: one window for how a Profile or Pocket toolpath cuts. Which side of the vectors
//! (Profile), the direction, single or both directions, the rings' order (an offset Pocket),
//! straight or trochoidal and its loops. A picture of what the choices make together is in the
//! window and, smaller, in the form beside the Cutting… button.

use cut_craft_bug_report::hint::HoverHint;
use cut_craft_bug_report::number::Number;
use cut_craft_core::entry::{Loops, Turn, trochoid};
use cut_craft_core::geometry::Point;
use eframe::egui::{self, Pos2, Vec2, pos2, vec2};
use serde_json::Value;

/// How many loops the loops strip shows at the least, and at the most.
const SHOWN: f64 = 3.0;
const MOST: f64 = 60.0;

/// How much further apart the combination picture draws the loops than they are.
const LOOP_SPREAD: f64 = 4.0;

/// The form's picture, and the window's, in rows.
const SMALL_ROWS: f32 = 5.0;
const LARGE_ROWS: f32 = 12.0;

/// Where the cut goes, as the picture draws it.
#[derive(Clone, Copy, PartialEq)]
enum At {
    Outside,
    Inside,
    On,
    Pocket,
}

/// The choices read from a form's settings.
#[expect(
    clippy::struct_excessive_bools,
    reason = "the choices as the form keeps them"
)]
struct Cutting {
    at: At,
    climb: bool,
    both: bool,
    trochoidal: bool,
    loops: Loops,
    /// An offset pocket's ring order; a raster pocket's is `None`, as is a profile's
    rings: Option<String>,
    raster: bool,
    /// An adaptive pocket: a steady bite, spiralling out, then once round the wall
    adaptive: bool,
}

impl Cutting {
    fn read(params: &Value) -> Self {
        let text = |key: &str| params[key].as_str().unwrap_or_default().to_owned();
        let pocket = params.get("rings").is_some();
        let raster = pocket && text("strategy") == "raster";
        let adaptive = pocket && text("strategy") == "adaptive";
        Self {
            at: match (pocket, text("side").as_str()) {
                (true, _) => At::Pocket,
                (_, "inside") => At::Inside,
                (_, "on") => At::On,
                _ => At::Outside,
            },
            climb: text("direction") != "conventional",
            both: text("ways") == "both",
            trochoidal: text("motion") == "trochoidal" && !adaptive,
            loops: serde_json::from_value(params["loops"].clone()).unwrap_or_default(),
            rings: (pocket && !raster && !adaptive).then(|| text("rings")),
            raster,
            adaptive,
        }
    }

    /// Whether the first pass goes clockwise, seen from above: climb is clockwise round a
    /// part or along a line, anticlockwise round a hole or in a pocket.
    fn clockwise(&self) -> bool {
        self.climb == matches!(self.at, At::Outside | At::On)
    }

    /// What the choices make in plain words, before the detail [`Self::told`] gives.
    fn headline(&self) -> String {
        let way = if self.clockwise() {
            "clockwise"
        } else {
            "anticlockwise"
        };
        match self.at {
            At::Outside => format!("Cuts round the outside of each shape, going {way}"),
            At::Inside => format!("Cuts round the inside of each shape, going {way}"),
            At::On => format!("Cuts along the line itself, going {way}"),
            At::Pocket if self.raster => "Clears the pocket in rows".to_owned(),
            At::Pocket if self.adaptive => {
                "Clears the pocket with a thin, steady bite, then once round the wall".to_owned()
            }
            At::Pocket => format!("Clears the pocket in rings, going {way}"),
        }
    }

    /// What the choices make, a line each.
    fn told(&self) -> Vec<String> {
        let way = |cw: bool| if cw { "clockwise" } else { "anticlockwise" };
        let cut = |climb: bool| if climb { "climb" } else { "conventional" };
        let round = match self.at {
            At::Outside => "round the part",
            At::Inside => "round the hole",
            At::On => "along the line",
            At::Pocket => "round the pocket",
        };
        if self.adaptive {
            return vec![
                format!(
                    "Spirals out from the roomiest spot, cutting {} all the way",
                    cut(self.climb)
                ),
                "Turns tighter where the bite would grow, so corners are cut in thin slices"
                    .to_owned(),
                format!(
                    "Then once round the wall, {}, for a clean edge",
                    way(self.clockwise())
                ),
            ];
        }
        let mut lines = vec![format!(
            "Pass 1: {}, {} {round}",
            cut(self.climb),
            way(self.clockwise())
        )];
        if self.both {
            lines.push(format!(
                "Pass 2: back the other way, {}, {}; and so on",
                cut(!self.climb),
                way(!self.clockwise())
            ));
        }
        if let Some(rings) = &self.rings
            && !self.trochoidal
        {
            lines.push(match rings.as_str() {
                "outline_in" => format!("Rings from the outline in: they cut {}", cut(!self.climb)),
                "alternate" => format!(
                    "Rings middle out ({}), then outline in ({}), pass by pass",
                    cut(self.climb),
                    cut(!self.climb)
                ),
                _ => format!("Rings from the middle out: they cut {}", cut(self.climb)),
            });
        }
        if self.trochoidal {
            let anticlockwise = self.loops.turn == Turn::Anticlockwise;
            lines.push(format!(
                "In loops turning {}: each loop cuts {}",
                way(!anticlockwise),
                cut(anticlockwise)
            ));
        }
        lines
    }
}

/// Under Profile's Machine Vectors… or Pocket's Clear Pocket: the picture of the cutting
/// chosen, what it does, and Cutting… to change it, with its window while open. Whether the
/// settings changed.
pub(super) fn group_end(
    ui: &mut egui::Ui,
    group: &str,
    params: &mut Value,
    units: cut_craft_core::document::Units,
) -> bool {
    if group == crate::form::WHICH_SIDE {
        ui.label(side_said(params["side"].as_str().unwrap_or_default()));
        return false;
    }
    let ours = match group {
        "Machine Vectors…" => params.get("side").is_some(),
        "Clear Pocket" => params.get("rings").is_some(),
        _ => false,
    };
    if !ours {
        return false;
    }
    let id = egui::Id::unique("cutting");
    let mut open = ui.data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
    let row = ui.spacing().interact_size.y;
    ui.horizontal(|ui| {
        combination(ui, &Cutting::read(params), row * SMALL_ROWS);
        ui.vertical(|ui| {
            let cutting = Cutting::read(params);
            ui.label(cutting.headline());
            for line in cutting.told() {
                ui.small(line);
            }
            if ui
                .button("Cutting…")
                .on_hover_hint("Direction, passes, motion and loops, with a picture of them")
                .clicked()
            {
                open = true;
            }
        });
    });
    let before = params.clone();
    if open {
        let diameter = params["tool"]["diameter"].as_f64().unwrap_or(6.0).max(0.1);
        egui::Window::new("Cutting")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .show(ui.ctx(), |ui| window(ui, params, diameter, units));
    }
    ui.data_mut(|d| d.insert_temp(id, open));
    before != *params
}

/// What a profile cut on `side` of the vectors makes, in plain words for someone who has not
/// cut one before.
pub(super) fn side_said(side: &str) -> &'static str {
    match side {
        "inside" => {
            "Cuts a hole: the tool goes round the inside of each shape, so the hole is \
                     the size drawn."
        }
        "on" => {
            "Follows the line: the middle of the tool runs along it, so the cut is as wide \
                 as the tool. Use it for open lines and grooves."
        }
        _ => {
            "Cuts a part out: the tool goes round the outside of each shape, so the part is \
              the size drawn."
        }
    }
}

/// The window's inside: the choices, the picture, then the loops when trochoidal.
fn window(
    ui: &mut egui::Ui,
    params: &mut Value,
    diameter: f64,
    units: cut_craft_core::document::Units,
) {
    choices(ui, params);
    let cutting = Cutting::read(params);
    let row = ui.spacing().interact_size.y;
    ui.horizontal(|ui| {
        combination(ui, &cutting, row * LARGE_ROWS);
        ui.vertical(|ui| {
            for line in cutting.told() {
                ui.label(line);
            }
        });
    });
    if cutting.trochoidal {
        ui.separator();
        let mut loops = cutting.loops;
        loops_editor(ui, &mut loops, diameter, units);
        if loops != cutting.loops {
            params["loops"] = serde_json::to_value(loops).expect("loops serialise");
        }
    }
}

/// The choices, a row of radio buttons each, Rings for an offset pocket. A profile's side is
/// on its form, under the cut depth, where it is seen first.
fn choices(ui: &mut egui::Ui, params: &mut Value) {
    // adaptive clearing steers its own way: only the direction is the user's
    let adaptive = params["strategy"] == "adaptive";
    egui::Grid::new("cutting").num_columns(2).show(ui, |ui| {
        choose(
            ui,
            params,
            "direction",
            "Direction",
            &[
                (
                    "climb",
                    "Climb",
                    "The cutter's edge meets the material head on",
                ),
                ("conventional", "Conventional", "The other way round"),
            ],
        );
        if adaptive {
            return;
        }
        choose(
            ui,
            params,
            "ways",
            "Passes",
            &[
                ("one", "Single direction", "Every pass the direction's way"),
                (
                    "both",
                    "Both directions",
                    "Each pass back the way the last came",
                ),
            ],
        );
        if params["strategy"] == "offset" {
            choose(
                ui,
                params,
                "rings",
                "Rings",
                &[
                    (
                        "middle_out",
                        "From the middle out",
                        "Cut the direction chosen",
                    ),
                    (
                        "outline_in",
                        "From the outline in",
                        "Cut the other direction",
                    ),
                    (
                        "alternate",
                        "Alternate",
                        "Middle out, then outline in, pass by pass",
                    ),
                ],
            );
        }
        choose(
            ui,
            params,
            "motion",
            "Motion",
            &[
                (
                    "straight",
                    "Straight",
                    "Along the path, the cutter's whole side cutting",
                ),
                (
                    "trochoidal",
                    "Trochoidal",
                    "In small loops, a sliver of the edge cutting",
                ),
            ],
        );
    });
}

/// A row of radio buttons for `params[key]`.
pub(super) fn choose(
    ui: &mut egui::Ui,
    params: &mut Value,
    key: &str,
    label: &str,
    options: &[(&str, &str, &str)],
) {
    ui.label(label);
    ui.horizontal(|ui| {
        for &(value, name, help) in options {
            if ui
                .radio(params[key] == value, name)
                .on_hover_hint(help)
                .clicked()
            {
                params[key] = value.into();
            }
        }
    });
    ui.end_row();
}

/// The picture of what the choices make: the vector (a rounded square) with the material
/// round it, the first pass green with arrows the way it goes (in loops when trochoidal), a
/// second pass dashed the other way for both directions, and a pocket's rings numbered in
/// the order they are cut.
#[expect(clippy::too_many_lines, reason = "one picture, drawn in order")]
fn combination(ui: &mut egui::Ui, c: &Cutting, side: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(side), egui::Sense::hover());
    let visuals = ui.visuals().clone();
    let painter = ui.painter_at(rect);
    let (centre, half) = (rect.center(), side / 2.0);
    let width = side / 60.0;
    let vector = egui::Stroke::new(width, visuals.text_color());
    let tool = egui::Stroke::new(width * 1.5, visuals.selection.stroke.color);
    let material = visuals.widgets.inactive.weak_bg_fill;
    let hole = visuals.extreme_bg_color;
    let round = |s: f32| ring(centre, s, s * 0.3);
    // the material, and the vector
    let (edge, path) = match c.at {
        At::Outside => (0.4, 0.62),
        At::Inside => (0.8, 0.58),
        At::On => (0.6, 0.6),
        At::Pocket => (0.85, 0.66),
    };
    let solid = matches!(c.at, At::Inside | At::Pocket);
    painter.rect_filled(rect, 0.0, if solid { material } else { hole });
    painter.add(egui::Shape::convex_polygon(
        round(half * edge),
        if solid { hole } else { material },
        egui::Stroke::NONE,
    ));
    painter.add(egui::Shape::closed_line(round(half * edge), vector));
    let cw = c.clockwise();
    let along = |s: f32, cw: bool| {
        let mut pts = round(s);
        if !cw {
            pts.reverse();
        }
        pts
    };
    let arrows = |pts: &[Pos2], stroke: egui::Stroke, count: usize| {
        for k in 0..count {
            #[expect(clippy::cast_precision_loss, reason = "a few arrows")]
            let f = (k as f32 + 0.3) / count as f32;
            if let Some((at, dir)) = point_along(pts, f) {
                painter.arrow(at - dir * side / 30.0, dir * side / 15.0, stroke);
            }
        }
    };
    if c.adaptive {
        // a spiral out from the middle, then round the edge
        let turn = if cw { -1.0 } else { 1.0 };
        let spiral: Vec<Pos2> = (0..=120u8)
            .map(|k| {
                let t = f32::from(k) / 120.0;
                let a = turn * t * std::f32::consts::TAU * 3.0;
                centre + half * 0.55 * t * vec2(a.cos(), a.sin())
            })
            .collect();
        painter.add(egui::Shape::line(spiral.clone(), tool));
        arrows(&spiral, tool, 2);
        let pts = along(half * path, cw);
        painter.add(egui::Shape::closed_line(pts.clone(), tool));
        arrows(&pts, tool, 3);
    } else if c.raster {
        // lines across, every other one back, then round the edge
        for k in 0..5u8 {
            let y = centre.y + half * (f32::from(k) - 2.0) * 0.22;
            let (a, b) = (centre.x - half * 0.55, centre.x + half * 0.55);
            let (from, to) = if k % 2 == 0 { (a, b) } else { (b, a) };
            painter.arrow(pos2(from, y), vec2(to - from, 0.0), tool);
        }
        let pts = along(half * path, cw);
        painter.add(egui::Shape::closed_line(pts.clone(), tool));
        arrows(&pts, tool, 3);
    } else if let (Some(order), false) = (&c.rings, c.trochoidal) {
        // three rings, each numbered by when it is cut (both passes for alternate)
        let font = egui::FontId::proportional(side / 18.0);
        for (i, s) in [0.22, 0.44, 0.66].into_iter().enumerate() {
            let pts = along(half * s, cw);
            painter.add(egui::Shape::closed_line(pts.clone(), tool));
            arrows(&pts, tool, 1);
            let label = match order.as_str() {
                "outline_in" => format!("{}", 3 - i),
                "alternate" => format!("{}·{}", i + 1, 6 - i),
                _ => format!("{}", i + 1),
            };
            painter.text(
                centre + vec2(0.0, -half * s),
                egui::Align2::CENTER_CENTER,
                label,
                font.clone(),
                visuals.strong_text_color(),
            );
        }
    } else {
        let pts = along(half * path, cw);
        if c.trochoidal {
            let anticlockwise = c.loops.turn == Turn::Anticlockwise;
            let r = f64::from(half) * 0.07 * (c.loops.width / 100.0).clamp(0.4, 2.5);
            // drawn spread out (the real step is in the strip below), still closer for a
            // smaller step
            let advance = (2.0 * r * c.loops.step / c.loops.width * LOOP_SPREAD).max(r * 0.3);
            let line: Vec<Point> = pts
                .iter()
                .map(|p| Point::new(f64::from(p.x), f64::from(p.y)))
                .collect();
            // the screen's y runs down, so the loops are worked out turning the other way
            #[expect(clippy::cast_possible_truncation, reason = "a place on the screen")]
            let looped: Vec<Pos2> = trochoid(&line, true, r, advance, !anticlockwise)
                .into_iter()
                .map(|p| pos2(p.x as f32, p.y as f32))
                .collect();
            painter.add(egui::Shape::line(
                looped,
                egui::Stroke::new(width, visuals.selection.stroke.color),
            ));
        } else {
            painter.add(egui::Shape::closed_line(pts.clone(), tool));
        }
        arrows(
            &pts,
            egui::Stroke::new(width * 2.0, visuals.warn_fg_color),
            3,
        );
    }
    if c.both {
        // the next pass, a little deeper, back the other way
        let s = half * (path + if solid { -0.1 } else { 0.1 });
        let mut back = along(s, !cw);
        back.push(back[0]);
        let dashed = egui::Stroke::new(width, visuals.weak_text_color());
        painter.extend(egui::Shape::dashed_line(
            &back,
            dashed,
            side / 40.0,
            side / 60.0,
        ));
        arrows(&back, dashed, 2);
    }
}

/// A rounded square `s` from `c` to each side, corners `k` round, clockwise on the screen
/// from the top.
fn ring(c: Pos2, s: f32, k: f32) -> Vec<Pos2> {
    let corners = [
        (1.0, -1.0, -90.0_f32),
        (1.0, 1.0, 0.0),
        (-1.0, 1.0, 90.0),
        (-1.0, -1.0, 180.0),
    ];
    let mut pts = vec![];
    for (sx, sy, a0) in corners {
        let corner = c + vec2(sx * (s - k), sy * (s - k));
        for i in 0..=6u8 {
            let a = (a0 + 15.0 * f32::from(i)).to_radians();
            pts.push(corner + vec2(a.cos(), a.sin()) * k);
        }
    }
    pts
}

/// The point `f` of the way round the closed `pts`, and the way on there.
fn point_along(pts: &[Pos2], f: f32) -> Option<(Pos2, Vec2)> {
    let closed: Vec<Pos2> = pts.iter().chain(pts.first()).copied().collect();
    let length: f32 = closed.windows(2).map(|w| w[0].distance(w[1])).sum();
    let mut left = f * length;
    for w in closed.windows(2) {
        let d = w[0].distance(w[1]);
        if left <= d && d > 0.0 {
            return Some((w[0] + (w[1] - w[0]) * (left / d), (w[1] - w[0]) / d));
        }
        left -= d;
    }
    None
}

/// The loops' sizes in a line, in the job's `units`.
fn loops_said(
    radius: f64,
    advance: f64,
    diameter: f64,
    units: cut_craft_core::document::Units,
) -> String {
    format!(
        "Loops {} across, each {} on; the cutter {}",
        units.length(2.0 * radius, 2),
        units.length(advance, 2),
        units.length(diameter, 2)
    )
}

/// The loops' settings over a strip of them at the cutter's size.
fn loops_editor(
    ui: &mut egui::Ui,
    loops: &mut Loops,
    diameter: f64,
    units: cut_craft_core::document::Units,
) {
    egui::Grid::new("loops").num_columns(2).show(ui, |ui| {
        ui.label("Loop Width")
            .on_hover_hint("How wide each loop is, of the tool's diameter");
        slider(ui, &mut loops.width, 10.0..=400.0, false);
        ui.end_row();
        ui.label("Loop Step")
            .on_hover_hint("How far each loop moves on from the last, of the tool's diameter");
        slider(ui, &mut loops.step, 1.0..=100.0, true);
        ui.end_row();
        ui.label("Loops Turn");
        ui.horizontal(|ui| {
            ui.radio_value(&mut loops.turn, Turn::Anticlockwise, "Anticlockwise")
                .on_hover_hint("Each loop cuts climb");
            ui.radio_value(&mut loops.turn, Turn::Clockwise, "Clockwise")
                .on_hover_hint("Each loop cuts conventional");
        });
        ui.end_row();
    });
    let (radius, advance) = loops.sized(diameter);
    ui.weak(loops_said(radius, advance, diameter, units));
    loops_strip(
        ui,
        radius,
        advance,
        diameter,
        loops.turn == Turn::Anticlockwise,
    );
}

/// The path (a line), the loops along it, and the cutter at the last loop's start, to scale,
/// the way on shown by an arrow.
fn loops_strip(ui: &mut egui::Ui, radius: f64, advance: f64, diameter: f64, anticlockwise: bool) {
    let row = ui.spacing().interact_size.y;
    let (rect, _) = ui.allocate_exact_size(vec2(row * 20.0, row * 6.0), egui::Sense::hover());
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
        pos2(
            rect.left() + (gap + (reach + p.x) * scale) as f32,
            rect.center().y - (p.y * scale) as f32,
        )
    };
    let line = [Point::ZERO, Point::new(length, 0.0)];
    painter.line_segment(
        [at(line[0]), at(line[1])],
        visuals.widgets.noninteractive.fg_stroke,
    );
    let path: Vec<Pos2> = trochoid(&line, false, radius, advance, anticlockwise)
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
        end - vec2(row, 0.0),
        vec2(row, 0.0),
        visuals.widgets.noninteractive.fg_stroke,
    );
}

/// A percentage on a slider, its value in a number box with up and down buttons beside it.
fn slider(ui: &mut egui::Ui, percent: &mut f64, range: std::ops::RangeInclusive<f64>, log: bool) {
    ui.horizontal(|ui| {
        ui.add(
            egui::Slider::new(percent, range.clone())
                .logarithmic(log)
                .show_value(false),
        );
        ui.add(Number::new(percent).range(range).suffix(" %"));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn told(params: &Value) -> Vec<String> {
        Cutting::read(params).told()
    }

    #[test]
    fn the_loops_sizes_read_in_the_jobs_units() {
        use cut_craft_core::document::Units;
        assert_eq!(
            loops_said(3.175, 1.27, 6.35, Units::Mm),
            "Loops 6.35 mm across, each 1.27 mm on; the cutter 6.35 mm"
        );
        assert_eq!(
            loops_said(3.175, 1.27, 6.35, Units::Inch),
            "Loops 0.2500 in across, each 0.0500 in on; the cutter 0.2500 in"
        );
    }

    #[test]
    fn an_adaptive_pocket_says_it_steers_itself_and_hides_rings_and_loops() {
        let params = serde_json::json!({
            "rings": "middle_out", "strategy": "adaptive", "direction": "climb",
            "motion": "trochoidal", "ways": "both"
        });
        let cutting = Cutting::read(&params);
        assert!(cutting.adaptive && !cutting.trochoidal && cutting.rings.is_none());
        assert!(cutting.headline().contains("thin, steady bite"));
        let lines = cutting.told();
        assert!(lines[0].contains("climb"), "{lines:?}");
        assert!(lines.iter().all(|l| !l.contains("Pass 2")), "{lines:?}");
        assert!(lines[2].contains("anticlockwise"), "{lines:?}");
    }

    #[test]
    fn the_cutting_is_said_in_plain_words_first() {
        let said = |params: Value| Cutting::read(&params).headline();
        assert_eq!(
            said(serde_json::json!({ "side": "outside", "direction": "climb" })),
            "Cuts round the outside of each shape, going clockwise"
        );
        assert_eq!(
            said(serde_json::json!({ "side": "inside", "direction": "climb" })),
            "Cuts round the inside of each shape, going anticlockwise"
        );
        assert!(said(serde_json::json!({ "side": "on" })).starts_with("Cuts along the line"));
        assert!(side_said("outside").starts_with("Cuts a part out"));
        assert!(side_said("inside").starts_with("Cuts a hole"));
        assert!(side_said("on").starts_with("Follows the line"));
    }

    #[test]
    fn the_choices_together_say_which_way_each_pass_cuts() {
        let lines = told(&serde_json::json!({
            "side": "outside", "direction": "climb", "ways": "both", "motion": "straight"
        }));
        assert_eq!(lines[0], "Pass 1: climb, clockwise round the part");
        assert!(
            lines[1].contains("conventional, anticlockwise"),
            "{lines:?}"
        );
        // in a hole climb goes anticlockwise
        let lines = told(&serde_json::json!({ "side": "inside", "direction": "climb" }));
        assert_eq!(lines[0], "Pass 1: climb, anticlockwise round the hole");
        // a pocket's rings from the outline in cut the other way
        let lines = told(&serde_json::json!({
            "rings": "outline_in", "strategy": "offset", "direction": "climb"
        }));
        assert!(
            lines
                .iter()
                .any(|l| l.contains("outline in: they cut conventional")),
            "{lines:?}"
        );
        // the loops' turn is their own
        let lines = told(&serde_json::json!({
            "side": "outside", "direction": "conventional", "motion": "trochoidal",
            "loops": { "width": 100.0, "step": 10.0, "turn": "anticlockwise" }
        }));
        assert_eq!(
            lines[0],
            "Pass 1: conventional, anticlockwise round the part"
        );
        assert!(
            lines.iter().any(|l| l.contains("each loop cuts climb")),
            "{lines:?}"
        );
    }
}
