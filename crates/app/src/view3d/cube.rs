//! The view cube in the 3D view's corner: its faces, and turning the view to one.

use super::{
    Align2, Color32, DVec3, EDGE_ON, FontId, Orbit, Painter, Pos2, Rect, Shape, Vec2, Visuals,
};

/// The view cube in a corner of the 3D view: a cube turned with the view whose faces, edges
/// and corners each turn the view to look from there (View Controller).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewCube {
    /// Where its middle shows.
    pub centre: Pos2,
    /// Half its edge, points.
    pub half: f32,
}

/// The cube's faces: the way out of each, the two ways across it, and its name.
pub(super) const FACES: [(DVec3, DVec3, DVec3, &str); 6] = [
    (DVec3::Z, DVec3::X, DVec3::Y, "Top"),
    (DVec3::NEG_Z, DVec3::X, DVec3::NEG_Y, "Bottom"),
    (DVec3::NEG_Y, DVec3::X, DVec3::Z, "Front"),
    (DVec3::Y, DVec3::NEG_X, DVec3::Z, "Back"),
    (DVec3::X, DVec3::Y, DVec3::Z, "Right"),
    (DVec3::NEG_X, DVec3::NEG_Y, DVec3::Z, "Left"),
];

impl ViewCube {
    /// In the top right of `area`, `margin` in from its edges: each face as wide as the
    /// longest name in `font` with `margin` either side.
    pub fn new(area: Rect, painter: &Painter, font: &FontId, margin: f32) -> Self {
        let longest = FACES
            .iter()
            .map(|f| {
                let galley = painter.layout_no_wrap(f.3.to_owned(), font.clone(), Color32::BLACK);
                galley.size().x
            })
            .fold(0.0, f32::max);
        let half = longest / 2.0 + margin;
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

    /// The square it turns in: its corners never reach further from its middle.
    pub fn rect(self) -> Rect {
        Rect::from_center_size(self.centre, Vec2::splat(2.0 * self.half * 3f32.sqrt()))
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

    /// Its faces in sight from `orbit`, each cut in thirds both ways: each part's corners on
    /// screen and the direction it looks from, a face's middle from that face, the strips
    /// along its edges from those edges and the squares in its corners from those corners.
    pub(super) fn tiles(self, orbit: Orbit) -> Vec<([Pos2; 4], DVec3)> {
        let third = |k: i32| -1.0 + 2.0 * f64::from(k) / 3.0;
        let mut tiles = vec![];
        for (out, across, up, _) in FACES {
            if orbit.project(out).1 <= EDGE_ON {
                continue;
            }
            for (i, j) in (0..3).flat_map(|i| (0..3).map(move |j| (i, j))) {
                let corners = [(i, j), (i + 1, j), (i + 1, j + 1), (i, j + 1)]
                    .map(|(u, v)| self.screen(orbit, out + across * third(u) + up * third(v)));
                let from = out + across * f64::from(i - 1) + up * f64::from(j - 1);
                tiles.push((corners, from.normalize()));
            }
        }
        tiles
    }

    /// The direction the part of the cube under `at` looks from; `None` off the cube.
    pub fn direction_at(self, orbit: Orbit, at: Pos2) -> Option<DVec3> {
        let inside = |corners: &[Pos2; 4]| {
            let turns: Vec<f32> = (0..4)
                .map(|k| {
                    let (a, b) = (corners[k], corners[(k + 1) % 4]);
                    (b - a).x * (at - a).y - (b - a).y * (at - a).x
                })
                .collect();
            turns.iter().all(|&t| t >= 0.0) || turns.iter().all(|&t| t <= 0.0)
        };
        self.tiles(orbit)
            .into_iter()
            .find(|(corners, _)| inside(corners))
            .map(|(_, from)| from)
    }

    /// Draws it as seen from `orbit`, the parts that look from `hovered` highlighted and each
    /// face's name on it where there is room.
    pub fn paint(
        self,
        painter: &Painter,
        orbit: Orbit,
        hovered: Option<DVec3>,
        visuals: &Visuals,
        font: &FontId,
    ) {
        for (corners, from) in self.tiles(orbit) {
            let lit = hovered.is_some_and(|h| h.abs_diff_eq(from, f64::EPSILON.sqrt()));
            let fill = if lit {
                visuals.selection.bg_fill
            } else {
                visuals.widgets.inactive.bg_fill
            };
            painter.add(Shape::convex_polygon(
                corners.to_vec(),
                fill,
                eframe::egui::Stroke::NONE,
            ));
        }
        let (stroke, text) = (
            visuals.widgets.noninteractive.bg_stroke,
            visuals.text_color(),
        );
        for (out, across, up, name) in FACES {
            let facing = orbit.project(out).1;
            if facing <= EDGE_ON {
                continue;
            }
            let outline = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
                .map(|(u, v)| self.screen(orbit, out + across * u + up * v));
            painter.add(Shape::closed_line(outline.to_vec(), stroke));
            // the name where the face, as turned, has at least the name's area
            let galley = painter.layout_no_wrap(name.to_owned(), font.clone(), text);
            let room = facing * f64::from((2.0 * self.half).powi(2));
            if f64::from(galley.size().x * galley.size().y) <= room {
                let at = Align2::CENTER_CENTER
                    .anchor_size(self.screen(orbit, out), galley.size())
                    .min;
                painter.galley(at, galley, text);
            }
        }
        self.paint_axes(painter, orbit, font);
    }

    /// The job's X, Y and Z along the cube's edges from its front-left-bottom corner, a
    /// little past the cube, in the axis colours used everywhere, each with an arrow head
    /// and its letter: which way each axis runs, whichever way the view is turned.
    pub(super) fn paint_axes(self, painter: &Painter, orbit: Orbit, font: &FontId) {
        use crate::axes::Axis;
        let corner = DVec3::splat(-1.0);
        let from = self.screen(orbit, corner);
        let width = (self.half / 8.0).max(2.0);
        let head = self.half / 2.5;
        // the letters a little bigger than the faces' names, to be read at a glance
        let font = &FontId::new(font.size * 1.25, font.family.clone());
        for axis in Axis::ALL {
            let to = self.screen(orbit, corner + axis.direction() * AXIS_REACH);
            let colour = axis.colour();
            if from.distance(to) < head {
                // looking straight along it: its letter alone, over the corner
                crate::axes::tag(painter, to, axis.letter(), colour, font);
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
}

/// How far the view cube's axis arrows reach from its corner, in half edges: a little past
/// the cube's far side.
pub(super) const AXIS_REACH: f64 = 2.4;
