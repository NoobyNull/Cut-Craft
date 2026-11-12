//! The tool drawn from its own sizes in the Tool Database: its cutting edge, flutes, neck
//! and shank to scale, each size typed in a box on its own dimension line. Aspire shows a
//! fixed picture with lettered sizes (D, A, R); Cut Craft draws the tool so a novice sees
//! what each number is, and what is not set yet.

use crate::manager::length;
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_bug_report::number::Number;
use cut_craft_core::toolpath::{Tool, ToolShape};
use eframe::egui;

/// Points across the edge from the axis to the outside.
const EDGE_STEPS: u32 = 48;
/// A drawn flute length when none is set, in diameters: most end mills' cut 2 to 4.
const GUESS_FLUTES: f64 = 3.0;
/// A drawn shank when its length is not set, over the flutes and neck below it.
const GUESS_SHANK: f64 = 1.5;

/// The tool's outline, millimetres up from its tip, what is drawn from a guess marked.
#[derive(Clone, Debug, PartialEq)]
pub struct Outline {
    /// The right half's edge: (out from the axis, up from the tip), from the tip on the axis
    /// to the top of the flutes.
    pub edge: Vec<[f64; 2]>,
    /// How far out the cutting part reaches: half the diameter, the shank for a tapered tool
    pub radius: f64,
    /// How wide the flutes' body is, out from the axis: `radius`, but a thread mill's core
    pub body: f64,
    /// A thread mill's tooth standing out from its core: its lower corner, tip and upper
    /// corner
    pub tooth: Option<[[f64; 2]; 3]>,
    pub flute_top: f64,
    pub neck_top: f64,
    pub shank_radius: f64,
    pub top: f64,
    /// Not set, drawn from a rule of thumb
    pub guessed_flutes: bool,
    pub guessed_shank: bool,
    pub guessed_shank_diameter: bool,
}

/// The outline of `tool`; `None` with no diameter. Nothing here is written to the tool:
/// unset sizes are only drawn (dashed), never kept, so no check trusts a guess.
pub fn outline(tool: &Tool) -> Option<Outline> {
    if !(tool.diameter.is_finite() && tool.diameter > 0.0) {
        return None;
    }
    let radius = tool.reach();
    let height_at = |r: f64| tool.profile(r).unwrap_or(0.0).max(0.0);
    // a thread mill's tooth stands out from a narrower core, which is what runs up to the
    // shank
    let tooth = (tool.shape == ToolShape::ThreadMill).then(|| {
        let core = radius - tool.tooth_size.clamp(0.0, radius);
        let half = tool.tooth_height() / 2.0;
        let tip = tool.tooth_offset.max(half);
        [[core, tip - half], [radius, tip], [core, tip + half]]
    });
    let body = tooth.map_or(radius, |t| t[0][0]);
    let edge_top = tooth.map_or_else(|| height_at(radius), |t| t[2][1]);
    let flutes = tool.known_flute_length();
    let flute_top = flutes.unwrap_or(match tool.shape {
        // a V-bit cuts with its cone
        ToolShape::VBit => edge_top,
        _ => edge_top.max(GUESS_FLUTES * tool.diameter),
    });
    let mut edge: Vec<[f64; 2]> = (0..=EDGE_STEPS)
        .map(|i| body * f64::from(i) / f64::from(EDGE_STEPS))
        .map(|r| [r, if tooth.is_some() { 0.0 } else { height_at(r) }])
        .take_while(|p| p[1] <= flute_top + 1e-9)
        .collect();
    // flutes shorter than the edge (a V-bit's cone) end where they stop, then out to the
    // side
    if edge
        .last()
        .is_some_and(|p| p[1] < flute_top - 1e-9 || p[0] < body)
    {
        let out = edge.last().map_or(0.0, |p| p[0]);
        edge.push([out, flute_top]);
    }
    if edge.last().is_some_and(|p| p[0] < body) {
        edge.push([body, flute_top]);
    }
    if tooth.is_some() {
        // the core's flat bottom is two points, not a row of them
        edge.dedup_by(|b, a| a[1] == b[1] && b[1] == 0.0 && b[0] < body);
    }
    let neck_top = flute_top + tool.neck_length.max(0.0);
    let shank_set = tool.shank_diameter.is_finite() && tool.shank_diameter > 0.0;
    let shank_radius = if shank_set {
        tool.shank_diameter / 2.0
    } else {
        body
    };
    let shank_known = tool.shank_length.is_finite() && tool.shank_length > 0.0;
    let shank = if shank_known {
        tool.shank_length
    } else {
        (GUESS_SHANK * neck_top).max(6.0 * shank_radius)
    };
    Some(Outline {
        edge,
        radius,
        body,
        tooth,
        flute_top,
        neck_top,
        shank_radius,
        top: neck_top + shank,
        guessed_flutes: flutes.is_none(),
        guessed_shank: !shank_known,
        guessed_shank_diameter: !shank_set,
    })
}

/// The drawing's size, points: its width at most, narrower in a narrow window.
const SIZE: egui::Vec2 = egui::vec2(460.0, 340.0);
/// Room left of the tool for the angle, tip radius and taper boxes.
const LEFT: f32 = 120.0;
/// Room right of the tool for the length boxes.
const RIGHT: f32 = 150.0;
/// Room above the tool for the shank diameter and below it for the diameter.
const ABOVE: f32 = 46.0;
const BELOW: f32 = 54.0;
/// A box and its caption.
const BOX: egui::Vec2 = egui::vec2(120.0, 40.0);

/// Where the drawing is: millimetres from the tool's tip to points.
struct View {
    rect: egui::Rect,
    axis: f32,
    tip: f32,
    scale: f32,
}

impl View {
    fn new(rect: egui::Rect, shape: &Outline) -> Self {
        let draw = egui::Rect::from_min_max(
            rect.min + egui::vec2(LEFT, ABOVE),
            rect.max - egui::vec2(RIGHT, BELOW),
        );
        let widest = shape.radius.max(shape.shank_radius);
        #[expect(clippy::cast_possible_truncation, reason = "a scale in points")]
        let scale = (f64::from(draw.height()) / shape.top)
            .min(f64::from(draw.width()) / (2.0 * widest)) as f32;
        Self {
            rect,
            axis: draw.center().x,
            tip: draw.bottom(),
            scale,
        }
    }

    #[expect(clippy::cast_possible_truncation, reason = "millimetres to points")]
    fn at(&self, r: f64, z: f64) -> egui::Pos2 {
        egui::pos2(
            self.axis + r as f32 * self.scale,
            self.tip - z as f32 * self.scale,
        )
    }
}

/// The tool drawn to scale, each size in a box on its dimension line; whether any changed.
pub fn editor(ui: &mut egui::Ui, tool: &mut Tool) -> bool {
    let Some(shape) = outline(tool) else {
        ui.vertical(|ui| {
            ui.weak("Give the tool a diameter to see it drawn");
            ui.label("Diameter");
        });
        return length(ui, &mut tool.diameter, tool.units);
    };
    let width = (ui.available_width() - 1.0)
        .floor()
        .clamp(LEFT + RIGHT + 60.0, SIZE.x);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, SIZE.y), egui::Sense::hover());
    let view = View::new(rect, &shape);
    paint(ui, &view, &shape);
    let mut changed = lengths(ui, &view, &shape, tool);
    changed |= widths(ui, &view, &shape, tool);
    changed |= left_sizes(ui, &view, &shape, tool);
    changed
}

/// The tool itself: flutes shaded, set sizes outlined solid and guessed ones dashed.
fn paint(ui: &egui::Ui, view: &View, shape: &Outline) {
    let painter = ui.painter_at(view.rect);
    let ink = ui.visuals().text_color();
    let faint = ui.visuals().weak_text_color();
    let cutting = egui::Color32::from_rgb(232, 140, 48);
    let at = |r, z| view.at(r, z);
    // the flutes as columns under the edge, each side, in one mesh (the edge can be
    // concave, and a mesh leaves no seams between them)
    let flute_fill = cutting.gamma_multiply(if shape.guessed_flutes { 0.45 } else { 0.9 });
    let mut flutes = egui::Mesh::default();
    for w in shape.edge.windows(2) {
        let (p, q) = (w[0], w[1]);
        for sign in [1.0, -1.0] {
            let first = u32::try_from(flutes.vertices.len()).unwrap_or(u32::MAX);
            for corner in [
                at(sign * p[0], p[1]),
                at(sign * q[0], q[1]),
                at(sign * q[0], shape.flute_top),
                at(sign * p[0], shape.flute_top),
            ] {
                flutes.colored_vertex(corner, flute_fill);
            }
            flutes.add_triangle(first, first + 1, first + 2);
            flutes.add_triangle(first, first + 2, first + 3);
        }
    }
    painter.add(flutes);
    // a thread mill's tooth, each side
    for sign in [1.0, -1.0] {
        if let Some(tooth) = shape.tooth {
            painter.add(egui::Shape::convex_polygon(
                tooth.iter().map(|p| at(sign * p[0], p[1])).collect(),
                flute_fill,
                egui::Stroke::new(1.5, ui.visuals().text_color()),
            ));
        }
    }
    let body_fill = faint.gamma_multiply(0.35);
    for (half, from, to) in [
        (shape.body, shape.flute_top, shape.neck_top),
        (shape.shank_radius, shape.neck_top, shape.top),
    ] {
        painter.rect_filled(
            egui::Rect::from_two_pos(at(-half, from), at(half, to)),
            0.0,
            body_fill,
        );
    }
    let line = egui::Stroke::new(1.5, ink);
    let draw = |points: Vec<egui::Pos2>, guessed: bool| {
        if guessed {
            painter.extend(egui::Shape::dashed_line(&points, line, 5.0, 4.0));
        } else {
            painter.add(egui::Shape::line(points, line));
        }
    };
    for sign in [1.0, -1.0] {
        draw(
            shape.edge.iter().map(|p| at(sign * p[0], p[1])).collect(),
            false,
        );
        draw(
            vec![
                at(sign * shape.body, shape.flute_top),
                at(sign * shape.body, shape.neck_top),
                at(sign * shape.shank_radius, shape.neck_top),
            ],
            false,
        );
        draw(
            vec![
                at(sign * shape.shank_radius, shape.neck_top),
                at(sign * shape.shank_radius, shape.top),
            ],
            shape.guessed_shank || shape.guessed_shank_diameter,
        );
    }
    draw(
        vec![
            at(-shape.shank_radius, shape.top),
            at(shape.shank_radius, shape.top),
        ],
        shape.guessed_shank,
    );
    // where the flutes stop
    let flute_line = vec![
        at(-shape.body, shape.flute_top),
        at(shape.body, shape.flute_top),
    ];
    if shape.guessed_flutes {
        draw(flute_line, true);
    } else {
        painter.add(egui::Shape::line(flute_line, egui::Stroke::new(1.0, faint)));
    }
}

/// The lengths up the right side, flutes, neck and shank, each box beside its stretch.
fn lengths(ui: &mut egui::Ui, view: &View, shape: &Outline, tool: &mut Tool) -> bool {
    let painter = ui.painter_at(view.rect);
    let dim = egui::Stroke::new(1.0, ui.visuals().weak_text_color());
    let line_x = view.at(shape.radius.max(shape.shank_radius), 0.0).x + 14.0;
    for height in [0.0, shape.flute_top, shape.neck_top, shape.top] {
        let y = view.at(0.0, height).y;
        painter.line_segment(
            [egui::pos2(line_x - 6.0, y), egui::pos2(line_x + 6.0, y)],
            dim,
        );
    }
    painter.line_segment(
        [
            egui::pos2(line_x, view.at(0.0, 0.0).y),
            egui::pos2(line_x, view.at(0.0, shape.top).y),
        ],
        dim,
    );
    // the boxes pushed apart where the stretches are short, kept in the drawing
    let mids = [
        f64::midpoint(0.0, shape.flute_top),
        f64::midpoint(shape.flute_top, shape.neck_top),
        f64::midpoint(shape.neck_top, shape.top),
    ];
    let mut ys = mids.map(|m| view.at(0.0, m).y);
    for i in (0..2).rev() {
        ys[i] = ys[i].max(ys[i + 1] + BOX.y + 6.0);
    }
    let over = (ys[0] - (view.rect.bottom() - BOX.y / 2.0)).max(0.0);
    for y in &mut ys {
        *y -= over;
    }
    let box_x = line_x + 18.0;
    let units = tool.units;
    let set = [
        !shape.guessed_flutes,
        // no neck is a neck of 0, not a size missing
        true,
        !shape.guessed_shank,
    ];
    let rows: [(&str, &str, &mut f64); 3] = [
        (
            "Flute length (cuts)",
            "The part of the bit with cutting edges: the bit can cut no deeper than this, \
             or the plain part above rubs and burns the wood. Toolpaths that go deeper are \
             flagged after Calculate. Measure your bit, or look for 'cutting length' on its \
             packaging; most are 2 to 4 times the diameter. Drawn dashed until set",
            &mut tool.flute_length,
        ),
        (
            "Neck (doesn't cut)",
            "A plain part between the flutes and the shank, on bits that have one; 0 for \
             none",
            &mut tool.neck_length,
        ),
        (
            "Shank (in the collet)",
            "The smooth part held in the collet, from the flutes (or neck) to the top. \
             Drawn dashed until set",
            &mut tool.shank_length,
        ),
    ];
    let mut changed = false;
    for (i, (caption, hint, value)) in rows.into_iter().enumerate() {
        let from = egui::pos2(line_x, view.at(0.0, mids[i]).y);
        painter.line_segment([from, egui::pos2(box_x - 2.0, ys[i])], dim);
        let area = egui::Rect::from_min_size(egui::pos2(box_x, ys[i] - BOX.y / 2.0), BOX);
        changed |= labelled(ui, area, caption, hint, set[i], value, units);
    }
    changed
}

/// The diameter under the tip and the shank's over the top.
fn widths(ui: &mut egui::Ui, view: &View, shape: &Outline, tool: &mut Tool) -> bool {
    let painter = ui.painter_at(view.rect);
    let dim = egui::Stroke::new(1.0, ui.visuals().weak_text_color());
    let y = view.tip + 12.0;
    let (l, r) = (view.at(-shape.radius, 0.0).x, view.at(shape.radius, 0.0).x);
    painter.line_segment([egui::pos2(l, y), egui::pos2(r, y)], dim);
    for end in [l, r] {
        painter.line_segment([egui::pos2(end, y - 5.0), egui::pos2(end, y + 5.0)], dim);
    }
    let area = egui::Rect::from_center_size(egui::pos2(view.axis, y + 6.0 + BOX.y / 2.0), BOX);
    let diameter_hint = if tool.tapered().is_some() {
        "Across the tip, where the taper starts"
    } else {
        "How wide the bit cuts, across its flutes"
    };
    let mut diameter = tool.diameter;
    let mut changed = labelled(
        ui,
        area,
        "Diameter",
        diameter_hint,
        true,
        &mut diameter,
        tool.units,
    );
    // a tool keeps a width to draw
    if diameter > 0.0 {
        tool.diameter = diameter;
    }
    let y = view.at(0.0, shape.top).y - 10.0;
    let (l, r) = (
        view.at(-shape.shank_radius, 0.0).x,
        view.at(shape.shank_radius, 0.0).x,
    );
    painter.line_segment([egui::pos2(l, y), egui::pos2(r, y)], dim);
    let area = egui::Rect::from_min_size(
        egui::pos2(r + 10.0, (y - BOX.y / 2.0).max(view.rect.top())),
        BOX,
    );
    let shank_hint = if tool.taper > 0.0 {
        "Where the taper ends: the shank's width. Toolpaths keep all of it off the model"
    } else {
        "The shank's width: 1/4 in or 1/8 in on most router bits, 6 or 8 mm in metric"
    };
    changed |= labelled(
        ui,
        area,
        "Shank diameter",
        shank_hint,
        !shape.guessed_shank_diameter,
        &mut tool.shank_diameter,
        tool.units,
    );
    changed
}

/// `add`'s box at the drawing's left, `height` up the tool, a line from it to `pointing`.
fn on_left(
    ui: &mut egui::Ui,
    view: &View,
    height: f64,
    pointing: egui::Pos2,
    add: impl FnOnce(&mut egui::Ui) -> bool,
) -> bool {
    let rect = view.rect;
    let y = view
        .at(0.0, height)
        .y
        .clamp(rect.top() + BOX.y / 2.0, rect.bottom() - BOX.y / 2.0);
    let area = egui::Rect::from_min_size(
        egui::pos2(rect.left(), y - BOX.y / 2.0),
        BOX - egui::vec2(10.0, 0.0),
    );
    let faint = egui::Stroke::new(1.0, ui.visuals().weak_text_color());
    ui.painter_at(rect)
        .line_segment([egui::pos2(area.right(), y), pointing], faint);
    // a child not laid out in `ui`: the drawing already took the room
    add(&mut ui.new_child(column(area)))
}

/// The angle, corner radius and taper, on the left of the tool beside what they size.
fn left_sizes(ui: &mut egui::Ui, view: &View, shape: &Outline, tool: &mut Tool) -> bool {
    let mut changed = false;
    if matches!(tool.shape, ToolShape::VBit | ToolShape::ThreadMill) {
        // a V-bit's cone halfway up, a thread mill's tooth at its tip
        let (out, height) = match shape.tooth {
            Some(t) => (t[1][0], t[1][1]),
            None => (
                shape.radius / 2.0,
                shape.edge.last().map_or(0.0, |p| p[1]) / 2.0,
            ),
        };
        let pointing = view.at(-out, height);
        let caption = if tool.shape == ToolShape::VBit {
            "Included angle"
        } else {
            "Angle"
        };
        changed |= on_left(ui, view, height, pointing, |ui| {
            ui.weak(caption)
                .on_hover_hint("The angle across the point, from one side to the other");
            ui.add(Number::new(&mut tool.angle).range(1.0..=179.0).suffix("°"))
                .changed()
        });
    }
    if tool.shape == ToolShape::RadiusedEndMill {
        let pointing = view.at(-shape.radius, 0.0);
        let units = tool.units;
        changed |= on_left(ui, view, 0.0, pointing, |ui| {
            ui.weak("Corner radius")
                .on_hover_hint("How rounded the bit's bottom corners are");
            length(ui, &mut tool.tip_radius, units)
        });
    }
    if matches!(tool.shape, ToolShape::BallNose | ToolShape::EndMill) {
        let height = shape.flute_top / 2.0;
        let pointing = view.at(-shape.radius, height);
        changed |= on_left(ui, view, height, pointing, |ui| {
            ui.weak("Taper angle").on_hover_hint(
                "For a tapered bit, how far each side leans from straight; 0 for straight",
            );
            ui.add(Number::new(&mut tool.taper).range(0.0..=45.0).suffix("°"))
                .changed()
        });
    }
    changed
}

/// A box's own top-down place, not laid out in its parent (the drawing took the room).
fn column(area: egui::Rect) -> egui::UiBuilder {
    egui::UiBuilder::new()
        .max_rect(area)
        .layout(egui::Layout::top_down(egui::Align::Min))
}

/// A size's caption over its box in `area`, "not set" beside a size that isn't.
fn labelled(
    ui: &mut egui::Ui,
    area: egui::Rect,
    caption: &str,
    hint: &str,
    set: bool,
    value: &mut f64,
    units: cut_craft_core::document::Units,
) -> bool {
    let mut changed = false;
    let ui = &mut ui.new_child(column(area));
    {
        ui.spacing_mut().item_spacing.y = 2.0;
        ui.weak(caption).on_hover_hint(hint);
        ui.horizontal(|ui| {
            changed = length(ui, value, units);
            if !set {
                ui.colored_label(ui.visuals().warn_fg_color, "not set")
                    .on_hover_hint(hint);
            }
        });
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn end_mill(diameter: f64) -> Tool {
        Tool {
            diameter,
            ..Tool::default()
        }
    }

    #[test]
    fn an_end_mill_with_its_sizes_set_is_drawn_to_them() {
        let t = Tool {
            flute_length: 22.0,
            neck_length: 3.0,
            shank_length: 30.0,
            shank_diameter: 6.35,
            ..end_mill(3.0)
        };
        let o = outline(&t).unwrap();
        assert_eq!(o.edge.first(), Some(&[0.0, 0.0]), "flat from the axis");
        assert_eq!(
            o.edge.last(),
            Some(&[1.5, 22.0]),
            "up the side to the flutes' top"
        );
        assert_eq!(
            (o.flute_top, o.neck_top, o.top, o.shank_radius),
            (22.0, 25.0, 55.0, 3.175)
        );
        assert!(!o.guessed_flutes && !o.guessed_shank && !o.guessed_shank_diameter);
    }

    #[test]
    fn unset_sizes_are_drawn_from_a_guess_and_never_kept() {
        let t = end_mill(6.0);
        let o = outline(&t).unwrap();
        assert!(o.guessed_flutes && o.guessed_shank && o.guessed_shank_diameter);
        assert_eq!(o.flute_top, 18.0, "three diameters");
        assert_eq!(o.shank_radius, 3.0, "as wide as the flutes");
        assert_eq!(o.top, 18.0 + 27.0);
        assert_eq!(t.known_flute_length(), None);
        assert_eq!((t.shank_length, t.shank_diameter), (0.0, 0.0));
    }

    #[test]
    fn a_v_bit_without_a_flute_length_is_drawn_as_its_cone() {
        let t = Tool {
            shape: ToolShape::VBit,
            angle: 90.0,
            ..end_mill(12.0)
        };
        let o = outline(&t).unwrap();
        assert!((o.flute_top - 6.0).abs() < 1e-9, "{}", o.flute_top);
        let last = o.edge.last().unwrap();
        assert!((last[0] - 6.0).abs() < 1e-9 && (last[1] - 6.0).abs() < 1e-9);
        // flutes shorter than the cone stop it there and step out to the side
        let short = Tool {
            flute_length: 3.0,
            ..t
        };
        let o = outline(&short).unwrap();
        assert_eq!(o.flute_top, 3.0);
        assert!(o.edge.iter().all(|p| p[1] <= 3.0 + 1e-9));
        assert_eq!(o.edge.last(), Some(&[6.0, 3.0]));
    }

    #[test]
    fn a_ball_nose_is_round_at_the_tip() {
        let t = Tool {
            shape: ToolShape::BallNose,
            flute_length: 10.0,
            ..end_mill(6.0)
        };
        let o = outline(&t).unwrap();
        let mid = o.edge[o.edge.len() / 2];
        assert!(mid[1] > 0.0 && mid[1] < 3.0, "{mid:?}");
        assert_eq!(o.edge.last(), Some(&[3.0, 10.0]));
    }

    #[test]
    fn a_thread_mill_is_a_narrow_core_with_its_tooth_standing_out() {
        let t = Tool {
            shape: ToolShape::ThreadMill,
            angle: 60.0,
            tooth_size: 1.0,
            tooth_offset: 2.0,
            flute_length: 15.0,
            ..end_mill(10.0)
        };
        let o = outline(&t).unwrap();
        assert_eq!(o.radius, 5.0, "the diameter is tip to tip");
        assert_eq!(o.body, 4.0, "the core is the tooth size in from the tip");
        assert_eq!(o.edge, vec![[0.0, 0.0], [4.0, 0.0], [4.0, 15.0]]);
        let half = 30f64.to_radians().tan();
        let tooth = o.tooth.unwrap();
        assert_eq!(tooth[1], [5.0, 2.0], "its tip at the offset");
        assert!((tooth[0][1] - (2.0 - half)).abs() < 1e-12 && tooth[0][0] == 4.0);
        assert!((tooth[2][1] - (2.0 + half)).abs() < 1e-12 && tooth[2][0] == 4.0);
    }

    #[test]
    fn a_form_tool_is_drawn_from_its_profile() {
        let t = Tool {
            shape: ToolShape::FormTool,
            form: vec![[0.0, 0.0], [4.0, 0.0], [8.0, 4.0], [12.7, 9.0]],
            flute_length: 12.0,
            ..end_mill(25.4)
        };
        let o = outline(&t).unwrap();
        assert_eq!((o.radius, o.body, o.tooth), (12.7, 12.7, None));
        // the edge follows the profile: flat, then up to 9 at the outside
        assert!(
            o.edge
                .iter()
                .any(|p| (p[0] - 6.0).abs() < 0.2 && (p[1] - 2.0).abs() < 0.3)
        );
        assert!(
            o.edge
                .iter()
                .any(|p| (p[0] - 12.7).abs() < 1e-9 && (p[1] - 9.0).abs() < 1e-9)
        );
    }

    #[test]
    fn a_tool_with_no_diameter_is_not_drawn() {
        assert_eq!(outline(&end_mill(0.0)), None);
        assert_eq!(outline(&end_mill(f64::NAN)), None);
    }
}
