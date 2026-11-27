//! The view cube in the 3D view's corner: a dice with bevelled edges and corners whose 26
//! parts each turn the view to look from there, and the buttons round it (Home, the two
//! quarter turns and, looking straight at a face, the four nudges to the next face).

use super::{Color32, DVec3, EDGE_ON, FontId, Orbit, Painter, Pos2, Rect, Shape, Vec2, Visuals};
use std::sync::LazyLock;

/// The view cube in a corner of the 3D view: a dice turned with the view whose faces, bevelled
/// edges and corners each turn the view to look from there (View Controller).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewCube {
    /// Where its middle shows.
    pub centre: Pos2,
    /// Half its edge, points.
    pub half: f32,
}

/// How the cube is drawn (Options ▸ 3D View Settings ▸ View cube style).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CubeLook {
    /// Light and flat, its parts told apart by a little shading: easiest to read small.
    #[default]
    Flat,
    /// Shaded as a solid block lit from the upper left.
    Shaded,
}

/// The buttons round the cube.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CubeButton {
    /// Back to the Home view, the job fitted.
    Home,
    /// A quarter turn anticlockwise about the line of sight.
    TurnLeft,
    /// A quarter turn clockwise about the line of sight.
    TurnRight,
    /// The face beyond that side of the screen brought to the front.
    Nudge(Nudge),
}

/// Which side of the screen a nudge brings the next face from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Nudge {
    Up,
    Down,
    Left,
    Right,
}

impl Nudge {
    pub const ALL: [Self; 4] = [Self::Up, Self::Down, Self::Left, Self::Right];

    /// Its way on the screen (x right, y up).
    pub fn screen(self) -> DVec3 {
        match self {
            Self::Up => DVec3::Y,
            Self::Down => DVec3::NEG_Y,
            Self::Left => DVec3::NEG_X,
            Self::Right => DVec3::X,
        }
    }
}

/// How far each bevel cuts into a face from its edge, in half edges: 0.3 makes each bevel
/// 15% of the cube across, wide enough to hit and leaving the face room for its name.
pub const BEVEL: f64 = 0.3;
/// [`BEVEL`] for screen sizes.
const BEVEL_F32: f32 = 0.3;
/// How far a corner's bevel cuts in past where the edges' bevels meet, in half edges.
const CORNER: f64 = 0.9 * BEVEL;

/// The cube's faces: the way out of each, the two ways across it (its name reads along the
/// first, upright along the second), and its name's key.
pub const FACES: [(DVec3, DVec3, DVec3, &str); 6] = [
    (DVec3::Z, DVec3::X, DVec3::Y, "view-cube-top"),
    (DVec3::NEG_Z, DVec3::X, DVec3::NEG_Y, "view-cube-bottom"),
    (DVec3::NEG_Y, DVec3::X, DVec3::Z, "view-cube-front"),
    (DVec3::Y, DVec3::NEG_X, DVec3::Z, "view-cube-back"),
    (DVec3::X, DVec3::Y, DVec3::Z, "view-cube-right"),
    (DVec3::NEG_X, DVec3::NEG_Y, DVec3::Z, "view-cube-left"),
];

/// One of the cube's 26 parts: the way it looks from (a face's, an edge's or a corner's,
/// unit length) and its outline on the cube (each coordinate -1 to 1), anticlockwise seen
/// from outside.
pub(super) struct Part {
    pub from: DVec3,
    pub outline: Vec<DVec3>,
}

/// Faces, bevelled edges and corners: each part the plane square to its direction, cut by
/// all the others.
pub(super) static PARTS: LazyLock<Vec<Part>> = LazyLock::new(|| {
    let ways: Vec<DVec3> = (-1..=1)
        .flat_map(|x| (-1..=1).flat_map(move |y| (-1..=1).map(move |z| (x, y, z))))
        .filter(|&w| w != (0, 0, 0))
        .map(|(x, y, z)| DVec3::new(f64::from(x), f64::from(y), f64::from(z)))
        .collect();
    let plane = |w: DVec3| {
        let reach = match kind(w) {
            0 => 1.0,
            1 => 2.0 - BEVEL,
            _ => 3.0 - 1.5 * BEVEL - CORNER,
        };
        (w.normalize(), reach / w.length())
    };
    ways.iter()
        .map(|&w| {
            let (n, d) = plane(w);
            let across = n.any_orthonormal_vector();
            let up = n.cross(across);
            let mut outline: Vec<DVec3> = (0..4)
                .map(|k| {
                    let a = f64::from(k) * std::f64::consts::FRAC_PI_2;
                    n * d + (across * a.cos() + up * a.sin()) * 4.0
                })
                .collect();
            for &other in ways.iter().filter(|&&o| o != w) {
                let (m, e) = plane(other);
                outline = clipped(&outline, m, e);
            }
            outline.dedup_by(|a, b| a.distance(*b) < 1e-9);
            if outline.len() > 1 && outline[0].distance(outline[outline.len() - 1]) < 1e-9 {
                outline.pop();
            }
            Part { from: n, outline }
        })
        .collect()
});

/// Whether the part that looks from `w` is a face (0), an edge (1) or a corner (2).
fn kind(w: DVec3) -> u32 {
    w.abs().cmpgt(DVec3::splat(1e-9)).bitmask().count_ones() - 1
}

/// The polygon `outline` kept where `p · n <= d`.
fn clipped(outline: &[DVec3], n: DVec3, d: f64) -> Vec<DVec3> {
    let mut kept = vec![];
    for (k, &a) in outline.iter().enumerate() {
        let b = outline[(k + 1) % outline.len()];
        let (da, db) = (a.dot(n) - d, b.dot(n) - d);
        if da <= 1e-12 {
            kept.push(a);
        }
        if (da < -1e-12 && db > 1e-12) || (da > 1e-12 && db < -1e-12) {
            kept.push(a + (b - a) * (da / (da - db)));
        }
    }
    kept
}

impl ViewCube {
    /// In the top right of `area`, `margin` in from its edges, `half` points to half an edge.
    pub fn new(area: Rect, half: f32, margin: f32) -> Self {
        let reach = Self {
            centre: Pos2::ZERO,
            half,
        }
        .rect()
        .width()
            / 2.0;
        Self {
            centre: area.right_top() + Vec2::new(-(reach + margin), reach + margin),
            half,
        }
    }

    /// The square the cube turns in, with its buttons round it: its corners (√3 half edges
    /// out) never reach further from its middle.
    pub fn rect(self) -> Rect {
        Rect::from_center_size(self.centre, Vec2::splat(2.0 * self.half * 1.9))
    }

    /// Where the cube's point `p` (each coordinate -1 to 1) shows from `orbit`.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "screen coordinates fit f32"
    )]
    pub fn screen(self, orbit: Orbit, p: DVec3) -> Pos2 {
        let (at, _) = orbit.project(p);
        self.centre + Vec2::new(at.x as f32, -at.y as f32) * self.half
    }

    /// Its parts in sight from `orbit`: each one's outline on screen and the direction it
    /// looks from.
    pub(super) fn tiles(self, orbit: Orbit) -> Vec<(Vec<Pos2>, DVec3)> {
        PARTS
            .iter()
            .filter(|part| orbit.project(part.from).1 > EDGE_ON)
            .map(|part| {
                let outline = part.outline.iter().map(|&p| self.screen(orbit, p));
                (outline.collect(), part.from)
            })
            .collect()
    }

    /// The middle of the part that looks from `from` (any length), on the cube.
    #[cfg(test)]
    pub fn part_middle(from: DVec3) -> DVec3 {
        let from = from.normalize();
        PARTS
            .iter()
            .find(|p| p.from.abs_diff_eq(from, 1e-9))
            .map_or(from, |p| {
                #[expect(clippy::cast_precision_loss, reason = "a few corners")]
                let n = p.outline.len() as f64;
                p.outline.iter().copied().sum::<DVec3>() / n
            })
    }

    /// The direction the part of the cube under `at` looks from; `None` off the cube.
    pub fn direction_at(self, orbit: Orbit, at: Pos2) -> Option<DVec3> {
        let inside = |corners: &[Pos2]| {
            let n = corners.len();
            let turns = (0..n).map(|k| {
                let (a, b) = (corners[k], corners[(k + 1) % n]);
                (b - a).x * (at - a).y - (b - a).y * (at - a).x
            });
            let turns: Vec<f32> = turns.collect();
            turns.iter().all(|&t| t >= 0.0) || turns.iter().all(|&t| t <= 0.0)
        };
        self.tiles(orbit)
            .into_iter()
            .find(|(corners, _)| inside(corners))
            .map(|(_, from)| from)
    }

    /// How wide each button round the cube is, points.
    pub fn button_size(self) -> f32 {
        (self.half * 0.66).max(16.0)
    }

    /// Where `button` is: Home in the top left corner, the turns down the top right one,
    /// each nudge just off its side of a face seen straight on.
    pub fn button_rect(self, button: CubeButton) -> Rect {
        let (size, reach) = (self.button_size(), self.rect().width() / 2.0);
        let corner = reach - size / 2.0;
        let beside = self.half + size * 0.7;
        let at = match button {
            CubeButton::Home => Vec2::new(-corner, -corner),
            CubeButton::TurnLeft => Vec2::new(corner, -corner),
            CubeButton::TurnRight => Vec2::new(corner, -corner + size * 1.15),
            CubeButton::Nudge(Nudge::Up) => Vec2::new(0.0, -beside),
            CubeButton::Nudge(Nudge::Down) => Vec2::new(0.0, beside),
            CubeButton::Nudge(Nudge::Left) => Vec2::new(-beside, 0.0),
            CubeButton::Nudge(Nudge::Right) => Vec2::new(beside, 0.0),
        };
        Rect::from_center_size(self.centre + at, Vec2::splat(size))
    }

    /// The buttons shown from `orbit`, each with whether it can be pressed: the turns work
    /// only looking straight at a face, and the nudges show only then.
    pub fn buttons(orbit: Orbit) -> Vec<(CubeButton, bool)> {
        let head_on = orbit.head_on();
        let mut shown = vec![
            (CubeButton::Home, true),
            (CubeButton::TurnLeft, head_on),
            (CubeButton::TurnRight, head_on),
        ];
        if head_on {
            shown.extend(Nudge::ALL.map(|n| (CubeButton::Nudge(n), true)));
        }
        shown
    }

    /// The button under `at` from `orbit` that can be pressed.
    #[cfg(test)]
    pub fn button_at(self, orbit: Orbit, at: Pos2) -> Option<CubeButton> {
        Self::buttons(orbit)
            .into_iter()
            .find(|&(b, on)| on && self.button_rect(b).contains(at))
            .map(|(b, _)| b)
    }

    /// Draws it as seen from `orbit` in `look`, the part that looks from `hovered`
    /// highlighted, each face's name (`names`, in [`FACES`] order) printed along it.
    #[expect(
        clippy::too_many_arguments,
        reason = "one drawing, its inputs spelled out"
    )]
    pub fn paint(
        self,
        painter: &Painter,
        orbit: Orbit,
        hovered: Option<DVec3>,
        visuals: &Visuals,
        look: CubeLook,
        names: &[String; 6],
        font: &FontId,
    ) {
        let dark = visuals.dark_mode;
        let outline =
            eframe::egui::Stroke::new(1.0, Color32::from_gray(if dark { 25 } else { 70 }));
        let mut fills = vec![];
        for (corners, from) in self.tiles(orbit) {
            let lit = hovered.is_some_and(|h| h.abs_diff_eq(from, 1e-9));
            let fill = if lit {
                visuals.selection.bg_fill
            } else {
                shade(orbit, from, look, dark)
            };
            fills.push((from, fill));
            painter.add(Shape::convex_polygon(corners, fill, outline));
        }
        for ((out, across, up, _), name) in FACES.iter().zip(names) {
            let facing = orbit.project(*out).1;
            let Some(&(_, fill)) = fills.iter().find(|(f, _)| f.abs_diff_eq(*out, 1e-9)) else {
                continue;
            };
            if facing < 0.2 {
                continue;
            }
            self.paint_name(painter, orbit, (*out, *across, *up), name, fill, font);
        }
        self.paint_axes(painter, orbit, font);
    }

    /// A face's `name` printed along it, as large as fits its middle, dark on a light face
    /// and light on a dark one.
    fn paint_name(
        self,
        painter: &Painter,
        orbit: Orbit,
        (out, across, up): (DVec3, DVec3, DVec3),
        name: &str,
        fill: Color32,
        font: &FontId,
    ) {
        let colour = if u16::from(fill.r()) + u16::from(fill.g()) + u16::from(fill.b()) > 3 * 140 {
            Color32::from_gray(25)
        } else {
            Color32::from_gray(240)
        };
        // as large as fits the face's middle as it is turned and tilted: its width along the
        // face, its height across it
        let middle = self.screen(orbit, out);
        let along = self.screen(orbit, out + across) - middle;
        let up = self.screen(orbit, out + up) - middle;
        let inner = 2.0 * (1.0 - BEVEL_F32);
        let measured = painter.layout_no_wrap(name.to_owned(), font.clone(), colour);
        let (wide, high) = (measured.size().x.max(1.0), measured.size().y.max(1.0));
        let fits = (inner * along.length() * 0.85 / wide).min(inner * up.length() * 0.55 / high);
        let size = font.size * fits.min(1.4);
        if size < 6.0 {
            return;
        }
        let font = FontId::new(size, font.family.clone());
        let galley = painter.layout_no_wrap(name.to_owned(), font, colour);
        let angle = along.y.atan2(along.x);
        let half = galley.size() / 2.0;
        let (sin, cos) = angle.sin_cos();
        let turned = Vec2::new(half.x * cos - half.y * sin, half.x * sin + half.y * cos);
        let text =
            eframe::egui::epaint::TextShape::new(middle - turned, galley, colour).with_angle(angle);
        painter.add(text);
    }

    /// The job's X, Y and Z from a point off the cube's bottom left corner, in the axis
    /// colours used everywhere, each with an arrow head and its letter: which way each axis
    /// runs, whichever way the view is turned, clear of the cube's names.
    pub(super) fn paint_axes(self, painter: &Painter, orbit: Orbit, font: &FontId) {
        use crate::axes::Axis;
        let reach = self.rect().width() / 2.0;
        let from = self.centre + Vec2::new(-reach, reach) * 0.95;
        let length = self.half * AXIS_REACH;
        let width = (self.half / 12.0).max(1.5);
        let head = self.half / 3.5;
        let font = &FontId::new(font.size, font.family.clone());
        for axis in Axis::ALL {
            let (on_screen, _) = orbit.project(axis.direction());
            #[expect(clippy::cast_possible_truncation, reason = "screen offsets fit f32")]
            let to = from + Vec2::new(on_screen.x as f32, -on_screen.y as f32) * length;
            let colour = axis.colour();
            if from.distance(to) < head {
                // looking straight along it: its letter alone, beside the corner
                crate::axes::tag(
                    painter,
                    from + Vec2::new(head, -head),
                    axis.letter(),
                    colour,
                    font,
                );
                continue;
            }
            crate::axes::arrow(
                painter,
                from,
                to,
                eframe::egui::Stroke::new(width, colour),
                head,
            );
            let out = (to - from).normalized() * head;
            crate::axes::tag(painter, to + out, axis.letter(), colour, font);
        }
    }

    /// Draws `button` in its place, `hovered` lit and greyed when it can't be pressed.
    pub fn paint_button(
        self,
        ui: &eframe::egui::Ui,
        button: CubeButton,
        (on, hovered): (bool, bool),
        visuals: &Visuals,
        icon: Option<&eframe::egui::Image<'static>>,
    ) {
        let (painter, rect) = (ui.painter(), self.button_rect(button));
        let widget = if !on {
            &visuals.widgets.noninteractive
        } else if hovered {
            &visuals.widgets.hovered
        } else {
            &visuals.widgets.inactive
        };
        let round = matches!(button, CubeButton::Home).then_some(rect.width() * 0.22);
        match round {
            Some(r) => painter.rect(
                rect,
                r,
                widget.weak_bg_fill,
                widget.bg_stroke,
                eframe::egui::StrokeKind::Inside,
            ),
            None => painter.circle(
                rect.center(),
                rect.width() / 2.0,
                widget.weak_bg_fill,
                widget.bg_stroke,
            ),
        };
        let ink = if on {
            widget.fg_stroke.color
        } else {
            visuals.weak_text_color().gamma_multiply(0.6)
        };
        let inner = rect.shrink(rect.width() * 0.18);
        if let Some(image) = icon {
            let tint = if on {
                Color32::WHITE
            } else {
                Color32::from_white_alpha(90)
            };
            image.clone().tint(tint).paint_at(ui, inner);
            return;
        }
        draw_mark(painter, button, inner, (ink, widget.weak_bg_fill));
    }
}

/// A button's own picture, drawn: a house, a quarter turn round a square, or a triangle.
fn draw_mark(painter: &Painter, button: CubeButton, rect: Rect, (ink, back): (Color32, Color32)) {
    let (mid, size) = (rect.center(), rect.width() / 2.0);
    let stroke = eframe::egui::Stroke::new((size / 5.0).max(1.5), ink);
    match button {
        CubeButton::Home => {
            let at = |x: f32, y: f32| mid + Vec2::new(x, y) * size;
            painter.add(Shape::convex_polygon(
                vec![at(-1.0, 0.0), at(0.0, -0.95), at(1.0, 0.0)],
                ink,
                eframe::egui::Stroke::NONE,
            ));
            let body = Rect::from_min_max(at(-0.7, -0.05), at(0.7, 0.9));
            painter.rect_filled(body, 0.0, ink);
            let door = Rect::from_min_max(at(-0.2, 0.35), at(0.2, 0.9));
            painter.rect_filled(door, 0.0, back);
        }
        CubeButton::TurnLeft | CubeButton::TurnRight => {
            painter.rect_stroke(
                Rect::from_center_size(mid, Vec2::splat(size * 0.6)),
                0.0,
                eframe::egui::Stroke::new(stroke.width * 0.7, ink),
                eframe::egui::StrokeKind::Middle,
            );
            // three quarters of a ring, its arrow head where the turn goes; the left turn is
            // the right one mirrored
            let mirror = if button == CubeButton::TurnRight {
                1.0
            } else {
                -1.0
            };
            let steps = 24;
            let ring: Vec<Pos2> = (0..=steps)
                .map(|k| {
                    #[expect(clippy::cast_precision_loss, reason = "a few steps")]
                    let along = k as f32 / steps as f32;
                    let angle = -std::f32::consts::FRAC_PI_2 * 0.2 + along * 4.4;
                    mid + Vec2::new(angle.cos() * mirror, angle.sin()) * size * 0.85
                })
                .collect();
            let (end, before) = (ring[steps], ring[steps - 2]);
            painter.add(Shape::line(ring, stroke));
            crate::axes::head(painter, end, end - before, size * 0.6, ink);
        }
        CubeButton::Nudge(nudge) => {
            let way = nudge.screen();
            #[expect(clippy::cast_possible_truncation, reason = "unit directions")]
            let ahead = Vec2::new(way.x as f32, -way.y as f32);
            let across = ahead.rot90();
            painter.add(Shape::convex_polygon(
                vec![
                    mid + ahead * size * 0.7,
                    mid - ahead * size * 0.45 + across * size * 0.7,
                    mid - ahead * size * 0.45 - across * size * 0.7,
                ],
                ink,
                eframe::egui::Stroke::NONE,
            ));
        }
    }
}

/// A part's colour in `look`: lit by its angle to a light over the viewer's upper left, the
/// faces lightest, the edges' bevels a little darker and the corners darker still, so each
/// part stands out from the next.
fn shade(orbit: Orbit, from: DVec3, look: CubeLook, dark: bool) -> Color32 {
    let (on_screen, towards) = orbit.project(from);
    let light = DVec3::new(-0.45, 0.6, 0.66).normalize();
    let lit = DVec3::new(on_screen.x, on_screen.y, towards)
        .dot(light)
        .max(0.0);
    let kind = f64::from(kind(from));
    let (base, spread, step) = match (look, dark) {
        (CubeLook::Flat, true) => (210.0, 40.0, 30.0),
        (CubeLook::Flat, false) => (236.0, 30.0, 30.0),
        (CubeLook::Shaded, true) => (120.0, 110.0, 14.0),
        (CubeLook::Shaded, false) => (140.0, 105.0, 14.0),
    };
    let grey = (base - spread * (1.0 - lit) - step * kind).clamp(0.0, 255.0);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "kept to 0-255"
    )]
    let g = grey as u8;
    Color32::from_rgb(g, g, g.saturating_add(4))
}

/// How far the view cube's axis arrows reach from their corner, in half edges.
pub(super) const AXIS_REACH: f32 = 0.65;
