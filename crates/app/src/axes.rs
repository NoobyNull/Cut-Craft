//! The X, Y and Z axes as the whole app shows them: one colour each (X red, Y green, Z
//! blue), the same in the 2D view, the 3D view, the view cube and the forms' X, Y and Z
//! boxes, and arrows that point both ways along an axis, the + way marked.

use eframe::egui::{self, Color32, Painter, Pos2, Stroke, Vec2};

/// One of the job's three axes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    /// All three, in order.
    pub const ALL: [Self; 3] = [Self::X, Self::Y, Self::Z];

    /// Its colour everywhere it is shown: red X, green Y, blue Z, each bright enough to read
    /// on the dark and the light theme and on the material.
    #[must_use]
    pub const fn colour(self) -> Color32 {
        match self {
            Self::X => Color32::from_rgb(230, 60, 60),
            Self::Y => Color32::from_rgb(50, 170, 70),
            Self::Z => Color32::from_rgb(40, 130, 235),
        }
    }

    /// Its letter.
    #[must_use]
    pub const fn letter(self) -> &'static str {
        match self {
            Self::X => "X",
            Self::Y => "Y",
            Self::Z => "Z",
        }
    }

    /// The axis a setting is about: one keyed `x`, `y` or `z`, or one marked `x-axis`.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "x" => Some(Self::X),
            "y" => Some(Self::Y),
            "z" => Some(Self::Z),
            _ => None,
        }
    }

    /// Its unit direction in the job: X right, Y away from you, Z up.
    #[must_use]
    pub fn direction(self) -> cut_craft_core::glam::DVec3 {
        use cut_craft_core::glam::DVec3;
        match self {
            Self::X => DVec3::X,
            Self::Y => DVec3::Y,
            Self::Z => DVec3::Z,
        }
    }
}

/// A label for a setting about `axis`: its name in the axis's colour, bold.
#[must_use]
pub fn label(text: &str, axis: Axis) -> egui::RichText {
    egui::RichText::new(text).strong().color(axis.colour())
}

/// A filled arrow head with its tip at `tip`, pointing along `along`, `size` long.
pub fn head(painter: &Painter, tip: Pos2, along: Vec2, size: f32, colour: Color32) {
    let along = along.normalized();
    if !along.is_finite() || along == Vec2::ZERO {
        return;
    }
    let wing = along.rot90() * size * 0.5;
    let back = tip - along * size;
    painter.add(egui::Shape::convex_polygon(
        vec![tip, back + wing, back - wing],
        colour,
        Stroke::NONE,
    ));
}

/// A line from `from` to `to` with an arrow head at `to`.
pub fn arrow(painter: &Painter, from: Pos2, to: Pos2, stroke: Stroke, size: f32) {
    painter.line_segment([from, to - (to - from).normalized() * size * 0.5], stroke);
    head(painter, to, to - from, size, stroke.color);
}

/// A line along one axis through `middle` from `minus` to `plus` with an arrow head at each
/// end and a small + by the + end, so which way is + reads at a glance.
pub fn both_ways(painter: &Painter, minus: Pos2, plus: Pos2, stroke: Stroke, size: f32) {
    let along = (plus - minus).normalized();
    if !along.is_finite() {
        return;
    }
    painter.line_segment(
        [minus + along * size * 0.5, plus - along * size * 0.5],
        stroke,
    );
    head(painter, plus, along, size, stroke.color);
    head(painter, minus, -along, size, stroke.color);
}

/// Text at `at`, centred, in `colour`, with a faint shadow so it reads over anything.
pub fn tag(painter: &Painter, at: Pos2, text: &str, colour: Color32, font: &egui::FontId) {
    let shadow = Color32::from_black_alpha(160);
    let align = egui::Align2::CENTER_CENTER;
    painter.text(at + egui::vec2(1.0, 1.0), align, text, font.clone(), shadow);
    painter.text(at, align, text, font.clone(), colour);
}

/// Where a ring's + and − arrows go: given its points on screen in + order (the way a
/// positive angle turns it, by the right-hand rule) and its middle `centre`, the point
/// where it stands out most from the middle, at least `gap` from every point in `avoid`
/// (the other rings' arrows, so their marks never sit on each other) when it can be, and
/// the way + runs there.
#[must_use]
pub fn ring_marks(
    points: &[Pos2],
    centre: Pos2,
    avoid: &[Pos2],
    gap: f32,
) -> Option<(usize, Vec2)> {
    if points.len() < 3 {
        return None;
    }
    let off = |i: usize| points[i].distance_sq(centre);
    let clear = |i: usize| avoid.iter().all(|a| a.distance(points[i]) >= gap);
    let best = |ok: &dyn Fn(usize) -> bool| {
        (0..points.len() - 1)
            .filter(|&i| ok(i))
            .max_by(|&a, &b| off(a).total_cmp(&off(b)))
    };
    let outer = best(&clear).or_else(|| best(&|_| true))?;
    let along = points[outer + 1] - points[outer];
    Some((outer, along.normalized()))
}

/// Each ring's arrows ([`ring_marks`]), the rings taken in turn, each kept `gap` from the
/// ones before.
#[must_use]
pub fn rings_marks(rings: &[&[Pos2]], centre: Pos2, gap: f32) -> Vec<Option<(usize, Vec2)>> {
    let mut taken: Vec<Pos2> = vec![];
    rings
        .iter()
        .map(|points| {
            let mark = ring_marks(points, centre, &taken, gap);
            if let Some((i, _)) = mark {
                taken.push(points[i]);
            }
            mark
        })
        .collect()
}

/// Fine turns about X, Y and Z (degrees, applied X first, then Y, then Z, as Import 3D
/// Model applies them) after the model is turned a further `degrees` about the job's own
/// `axis`, the way that axis's ring shows: the same three turns given back, so a ring turns
/// the model about the axis it is drawn round whatever the other turns are.
#[must_use]
pub fn turned_about(turns: [f64; 3], axis: Axis, degrees: f64) -> [f64; 3] {
    use cut_craft_core::glam::{DMat3, EulerRot};
    let [x, y, z] = turns.map(f64::to_radians);
    let now = DMat3::from_euler(EulerRot::ZYX, z, y, x);
    let then = DMat3::from_axis_angle(axis.direction(), degrees.to_radians()) * now;
    let (z, y, x) = then.to_euler(EulerRot::ZYX);
    // a millionth of a degree is noise from the sums, not a turn anyone made
    let tidy = |a: f64| {
        let d = (a.to_degrees() * 1e6).round() / 1e6;
        if d == 0.0 { 0.0 } else { d }
    };
    [tidy(x), tidy(y), tidy(z)]
}

/// How far a drag `by` turns a ring whose screen tangent (the + way) at the pointer is
/// `along` and which reaches `radius` points from its middle, in degrees: the part of the
/// drag along the ring, as an arc of that radius.
#[must_use]
pub fn ring_turn(by: Vec2, along: Vec2, radius: f32) -> f64 {
    let along = along.normalized();
    if !along.is_finite() || radius <= f32::EPSILON {
        return 0.0;
    }
    f64::from(by.dot(along) / radius).to_degrees()
}

/// The way + runs along a ring at the point of it nearest `at`: its points on screen in
/// + order.
#[must_use]
pub fn ring_tangent(points: &[Pos2], at: Pos2) -> Option<Vec2> {
    points
        .windows(2)
        .filter(|w| w[0] != w[1])
        .min_by(|a, b| {
            let off = |w: &[Pos2]| {
                let (p, q) = (w[0], w[1]);
                let t = ((at - p).dot(q - p) / (q - p).length_sq()).clamp(0.0, 1.0);
                at.distance_sq(p + (q - p) * t)
            };
            off(a).total_cmp(&off(b))
        })
        .map(|w| (w[1] - w[0]).normalized())
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{pos2, vec2};

    #[test]
    fn each_axis_has_its_own_colour_red_green_blue() {
        let [x, y, z] = Axis::ALL.map(Axis::colour);
        assert!(x.r() > x.g() && x.r() > x.b(), "X is red: {x:?}");
        assert!(y.g() > y.r() && y.g() > y.b(), "Y is green: {y:?}");
        assert!(z.b() > z.r() && z.b() > z.g(), "Z is blue: {z:?}");
    }

    #[test]
    fn settings_keyed_x_y_or_z_are_about_that_axis() {
        assert_eq!(Axis::named("x"), Some(Axis::X));
        assert_eq!(Axis::named("Y"), Some(Axis::Y));
        assert_eq!(Axis::named("z"), Some(Axis::Z));
        assert_eq!(Axis::named("width"), None);
    }

    #[test]
    fn a_ring_turns_the_model_about_the_jobs_own_axis_whatever_the_other_turns() {
        use cut_craft_core::glam::{DMat3, DVec3, EulerRot};
        let fine = |[x, y, z]: [f64; 3]| {
            DMat3::from_euler(
                EulerRot::ZYX,
                z.to_radians(),
                y.to_radians(),
                x.to_radians(),
            )
        };
        // nothing else turned: only that axis's own number moves
        assert_eq!(turned_about([0.0; 3], Axis::X, 10.0), [10.0, 0.0, 0.0]);
        assert_eq!(
            turned_about([0.0, 5.0, 0.0], Axis::Y, 10.0),
            [0.0, 15.0, 0.0]
        );
        // spun half round Z first, the red ring still turns it about the job's +X
        for (start, axis) in [([0.0, 0.0, 180.0], Axis::X), ([30.0, 20.0, 45.0], Axis::Y)] {
            let after = turned_about(start, axis, 10.0);
            let step = fine(after) * fine(start).transpose();
            let want = DMat3::from_axis_angle(axis.direction(), 10f64.to_radians());
            for c in [DVec3::X, DVec3::Y, DVec3::Z] {
                assert!(
                    (step * c - want * c).length() < 1e-6,
                    "{start:?} {axis:?} {after:?}"
                );
            }
        }
    }

    #[test]
    fn a_drag_along_a_ring_the_plus_way_turns_it_plus_and_back_minus() {
        let along = vec2(0.0, -1.0);
        let quarter = std::f32::consts::FRAC_PI_2 * 100.0;
        let plus = ring_turn(vec2(0.0, -quarter), along, 100.0);
        assert!((plus - 90.0).abs() < 1e-3, "{plus}");
        let minus = ring_turn(vec2(0.0, quarter), along, 100.0);
        assert!((minus + 90.0).abs() < 1e-3, "{minus}");
        assert!(
            ring_turn(vec2(50.0, 0.0), along, 100.0).abs() < 1e-9,
            "across it: none"
        );
    }

    #[test]
    fn a_rings_marks_go_where_it_stands_out_most_pointing_the_plus_way() {
        // a flat ellipse drawn anticlockwise on screen (y down): widest at its ends
        let points: Vec<Pos2> = (0..=36u8)
            .map(|i| {
                let a = std::f32::consts::TAU * f32::from(i) / 36.0;
                pos2(100.0 * a.cos(), -30.0 * a.sin())
            })
            .collect();
        let (at, along) = ring_marks(&points, Pos2::ZERO, &[], 0.0).unwrap();
        // kept clear of another ring's arrows already there
        let (moved, _) = ring_marks(&points, Pos2::ZERO, &[points[at]], 50.0).unwrap();
        assert!(points[moved].distance(points[at]) >= 50.0);
        assert!(points[at].x.abs() > 99.0, "{:?}", points[at]);
        assert!(along.y.abs() > 0.9, "along the ring there: {along:?}");
        let tangent = ring_tangent(&points, pos2(0.0, -31.0)).unwrap();
        assert!(
            tangent.x < -0.9,
            "the + way at the top runs left: {tangent:?}"
        );
    }
}
