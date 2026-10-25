//! The 3D view: the simulated material as a lit block, turned, tilted, panned and zoomed.

use cut_craft_core::glam::{DVec2, DVec3};
use cut_craft_core::simulation::Material;
use eframe::egui::{Color32, Mesh, Pos2, Rect, Rgba, Vec2};

// ponytail: fixed lighting until the 3D view has light settings
/// Where the light comes from: upper left (-x, +y) and above.
const LIGHT: DVec3 = DVec3::new(-1.0, 1.0, 1.0);
/// Brightness of surfaces facing away from the light.
const AMBIENT: f64 = 0.35;
/// How much darker the full material thickness down is than the top.
const DEPTH_SHADE: f64 = 0.5;

/// Where the view looks from: `yaw` turns the job about the vertical, `pitch` tilts it from
/// straight down (0) towards side on (a right angle), radians.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Orbit {
    pub yaw: f64,
    pub pitch: f64,
}

impl Default for Orbit {
    fn default() -> Self {
        // ponytail: one starting angle until the view menu has standard views
        Self {
            yaw: -20f64.to_radians(),
            pitch: 50f64.to_radians(),
        }
    }
}

impl Orbit {
    /// Looking straight down on the job.
    pub const TOP: Self = Self {
        yaw: 0.0,
        pitch: 0.0,
    };
    /// Side on, from the front (-y) edge.
    pub const FRONT: Self = Self {
        yaw: 0.0,
        pitch: std::f64::consts::FRAC_PI_2,
    };
    /// Side on, from the right (+x) edge.
    pub const RIGHT: Self = Self {
        yaw: -std::f64::consts::FRAC_PI_2,
        pitch: std::f64::consts::FRAC_PI_2,
    };

    /// Turned by a drag of `by` across a view `size`: across the whole view is half a turn,
    /// sideways turning and up/down tilting (between straight down and side on).
    #[must_use]
    pub fn dragged(self, by: Vec2, size: Vec2) -> Self {
        let turn = |d: f32, across: f32| f64::from(d / across.max(1.0)) * std::f64::consts::PI;
        Self {
            yaw: self.yaw + turn(by.x, size.x),
            pitch: (self.pitch + turn(by.y, size.y)).clamp(0.0, std::f64::consts::FRAC_PI_2),
        }
    }

    /// `p` on the screen plane (x right, y up) and its distance towards the viewer.
    pub fn project(self, p: DVec3) -> (DVec2, f64) {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let (x, y) = (p.x * cos_yaw - p.y * sin_yaw, p.x * sin_yaw + p.y * cos_yaw);
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        (
            DVec2::new(x, y * cos_pitch + p.z * sin_pitch),
            p.z * cos_pitch - y * sin_pitch,
        )
    }
}

/// Zoom and pan on top of fitting the block to the view: `zoom` 1 and no `pan` fit it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Framing {
    pub zoom: f32,
    /// Of the block's centre from the view's, in points.
    pub pan: Vec2,
}

impl Default for Framing {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: Vec2::ZERO,
        }
    }
}

impl Framing {
    /// Zoomed by `factor` keeping the point `at` (from the view's centre) still.
    #[must_use]
    pub fn zoomed_about(self, at: Vec2, factor: f32) -> Self {
        // ponytail: limits keep the block findable; a "zoom to selection" can widen them
        let zoom = (self.zoom * factor).clamp(0.05, 200.0);
        Self {
            zoom,
            pan: at - (at - self.pan) * (zoom / self.zoom),
        }
    }

    #[must_use]
    pub fn panned(self, by: Vec2) -> Self {
        Self {
            pan: self.pan + by,
            ..self
        }
    }
}

/// What a drag in the 3D view does, from the buttons held (Aspire's mouse controls).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragMode {
    Rotate,
    Pan,
}

/// Middle-drag, Ctrl+right-drag or left and right together pan; left or right alone rotate.
/// `held` is the left, right and middle buttons.
pub fn drag_mode(held: [bool; 3], ctrl: bool) -> Option<DragMode> {
    let [left, right, middle] = held;
    match (left, right, middle) {
        (_, _, true) | (true, true, _) => Some(DragMode::Pan),
        (false, true, _) if ctrl => Some(DragMode::Pan),
        (true, _, _) | (_, true, _) => Some(DragMode::Rotate),
        _ => None,
    }
}

/// Brightness of the surface over a cell, 0 to 1: lit from the upper left, darker deeper.
#[expect(clippy::cast_precision_loss, reason = "cell steps are 1 or 2")]
pub fn brightness(material: &Material, column: usize, row: usize) -> f64 {
    let m = material;
    let height = |column: usize, row: usize| m.heights[row * m.columns + column];
    // slope by central differences, one-sided at the edges
    let slope =
        |before: usize, after: usize, rise: f64| rise / (m.cell * (after - before).max(1) as f64);
    let (left, right) = (column.saturating_sub(1), (column + 1).min(m.columns - 1));
    let (below, above) = (row.saturating_sub(1), (row + 1).min(m.rows - 1));
    let normal = DVec3::new(
        -slope(left, right, height(right, row) - height(left, row)),
        -slope(below, above, height(column, above) - height(column, below)),
        1.0,
    );
    let depth = if m.bottom < 0.0 {
        height(column, row) / m.bottom
    } else {
        0.0
    };
    lit(normal) * (1.0 - DEPTH_SHADE * depth)
}

/// Brightness of a surface facing `normal`.
fn lit(normal: DVec3) -> f64 {
    AMBIENT + (1.0 - AMBIENT) * normal.normalize().dot(LIGHT.normalize()).max(0.0)
}

/// `material` with at most `most` cells along its longer side, each the deepest of the
/// cells it covers so narrow cuts stay visible.
#[expect(clippy::cast_precision_loss, reason = "a small step")]
pub fn coarse(material: &Material, most: usize) -> Material {
    let m = material;
    let step = m.columns.max(m.rows).div_ceil(most.max(1)).max(1);
    let (columns, rows) = (m.columns.div_ceil(step), m.rows.div_ceil(step));
    let span = |block: usize, count: usize| block * step..((block + 1) * step).min(count);
    let heights = (0..rows)
        .flat_map(|row| (0..columns).map(move |column| (column, row)))
        .map(|(column, row)| {
            span(row, m.rows)
                .flat_map(|r| span(column, m.columns).map(move |c| m.heights[r * m.columns + c]))
                .fold(f64::INFINITY, f64::min)
        })
        .collect();
    Material {
        origin: m.origin,
        columns,
        rows,
        cell: m.cell * step as f64,
        bottom: m.bottom,
        heights,
    }
}

/// The material seen from `orbit`, fitted into `area` (then zoomed and panned by `framing`)
/// and drawn far to near.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "screen coordinates and brightness fit f32; cell indices fit f64"
)]
pub fn mesh(
    material: &Material,
    orbit: Orbit,
    framing: Framing,
    colour: Color32,
    area: Rect,
) -> Mesh {
    let m = material;
    let base = Rgba::from(colour);
    let shade = |brightness: f64| {
        let b = brightness as f32;
        Color32::from(Rgba::from_rgb(base.r() * b, base.g() * b, base.b() * b))
    };
    let top = |column: usize, row: usize| {
        DVec3::new(
            (column as f64 + 0.5) * m.cell,
            (row as f64 + 0.5) * m.cell,
            m.heights[row * m.columns + column],
        )
    };
    // quads in job space, a colour per corner
    let mut quads: Vec<([DVec3; 4], [Color32; 4])> = vec![];
    for row in 1..m.rows {
        for column in 1..m.columns {
            let corners = [
                (column - 1, row - 1),
                (column, row - 1),
                (column, row),
                (column - 1, row),
            ];
            quads.push((
                corners.map(|(c, r)| top(c, r)),
                corners.map(|(c, r)| shade(brightness(m, c, r))),
            ));
        }
    }
    // the block's sides that face the viewer, from the surface down to the bottom
    let (last_column, last_row) = (m.columns - 1, m.rows - 1);
    let sides: [(DVec3, Vec<(usize, usize)>); 4] = [
        (DVec3::NEG_Y, (0..m.columns).map(|c| (c, 0)).collect()),
        (DVec3::Y, (0..m.columns).map(|c| (c, last_row)).collect()),
        (DVec3::NEG_X, (0..m.rows).map(|r| (0, r)).collect()),
        (DVec3::X, (0..m.rows).map(|r| (last_column, r)).collect()),
    ];
    for (outward, edge) in sides {
        if orbit.project(outward).1 <= 0.0 {
            continue;
        }
        let colour = shade(lit(outward));
        for pair in edge.windows(2) {
            let (a, b) = (top(pair[0].0, pair[0].1), top(pair[1].0, pair[1].1));
            let down = |p: DVec3| DVec3::new(p.x, p.y, m.bottom);
            quads.push(([a, b, down(b), down(a)], [colour; 4]));
        }
    }
    // painter's algorithm: far quads first
    let mut projected: Vec<([DVec2; 4], f64, [Color32; 4])> = quads
        .into_iter()
        .map(|(corners, colours)| {
            let p = corners.map(|c| orbit.project(c));
            (
                p.map(|(at, _)| at),
                p.iter().map(|(_, d)| d).sum::<f64>(),
                colours,
            )
        })
        .collect();
    projected.sort_by(|a, b| a.1.total_cmp(&b.1));
    // fit the block's bounding sphere into the area, so turning it never changes the size
    let top_z = m.heights.iter().copied().fold(m.bottom, f64::max);
    let extent = DVec3::new(
        m.columns as f64 * m.cell,
        m.rows as f64 * m.cell,
        top_z - m.bottom,
    );
    let radius = (extent.length() / 2.0).max(f64::EPSILON);
    let (middle, _) = orbit.project(DVec3::new(
        extent.x / 2.0,
        extent.y / 2.0,
        f64::midpoint(top_z, m.bottom),
    ));
    let scale =
        f64::from(area.width().min(area.height())) / (2.0 * radius) * f64::from(framing.zoom);
    let centre = area.center() + framing.pan;
    let screen = |p: DVec2| {
        let offset = (p - middle) * scale;
        Pos2::new(centre.x + offset.x as f32, centre.y - offset.y as f32)
    };
    let mut mesh = Mesh::default();
    for (corners, _, colours) in projected {
        let first = u32::try_from(mesh.vertices.len()).expect("mesh fits u32 indices");
        for (corner, colour) in corners.into_iter().zip(colours) {
            mesh.colored_vertex(screen(corner), colour);
        }
        mesh.add_triangle(first, first + 1, first + 2);
        mesh.add_triangle(first, first + 2, first + 3);
    }
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::document::Job;
    use eframe::egui::pos2;

    /// 100 x 100 mm in 1 mm cells, with a 5 mm deep slot from x = 40 to 60 running along y.
    fn slotted() -> Material {
        let job = Job {
            width: 100.0,
            height: 100.0,
            ..Job::default()
        };
        let mut m = Material::new(&job, 100);
        for row in 0..m.rows {
            for column in 40..60 {
                m.heights[row * m.columns + column] = -5.0;
            }
        }
        m
    }

    #[test]
    fn walls_facing_the_light_are_brighter_than_flat_and_the_far_ones_darker() {
        let m = slotted();
        let flat = brightness(&m, 10, 50);
        assert!(
            brightness(&m, 60, 50) > flat,
            "the wall rising towards +x faces the light"
        );
        assert!(
            brightness(&m, 39, 50) < flat,
            "the wall dropping towards +x faces away"
        );
        assert!(
            brightness(&m, 50, 50) < flat,
            "the slot floor is deeper so darker"
        );
    }

    #[test]
    fn coarse_cells_keep_the_deepest_point() {
        let mut m = slotted();
        m.heights[5 * m.columns + 7] = -9.0;
        let c = coarse(&m, 30);
        assert_eq!(
            (c.columns, c.rows),
            (25, 25),
            "4 mm cells: the step that fits 30"
        );
        assert!((c.cell - 4.0).abs() < 1e-12);
        assert_eq!(c.heights[c.columns + 1], -9.0);
        assert_eq!(
            c.heights[12 * c.columns + 12],
            -5.0,
            "x 48 to 52 is in the slot"
        );
        assert_eq!(c.bottom, m.bottom);
    }

    #[test]
    fn from_straight_above_the_projection_is_the_plan() {
        let top = Orbit {
            yaw: 0.0,
            pitch: 0.0,
        };
        let (p, depth) = top.project(DVec3::new(30.0, 40.0, -2.0));
        assert!(p.abs_diff_eq(DVec2::new(30.0, 40.0), 1e-12));
        assert!((depth + 2.0).abs() < 1e-12, "deeper is further away");
    }

    #[test]
    fn tilting_raises_the_far_edge_and_puts_it_behind() {
        let tilted = Orbit {
            yaw: 0.0,
            pitch: 60f64.to_radians(),
        };
        let (front, front_depth) = tilted.project(DVec3::ZERO);
        let (back, back_depth) = tilted.project(DVec3::new(0.0, 100.0, 0.0));
        assert!(back.y > front.y);
        assert!(back_depth < front_depth);
    }

    #[test]
    fn the_mesh_fits_the_area_and_is_drawn_far_to_near() {
        let area = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 300.0));
        let orbit = Orbit {
            yaw: 0.0,
            pitch: 60f64.to_radians(),
        };
        let mesh = mesh(&slotted(), orbit, Framing::default(), Color32::WHITE, area);
        assert_ne!(mesh.indices.len(), 0);
        assert!(
            mesh.vertices
                .iter()
                .all(|v| area.expand(1e-3).contains(v.pos)),
            "inside the area"
        );
        let centre_y = |triangle: &[u32]| -> f32 {
            triangle
                .iter()
                .map(|&i| mesh.vertices[i as usize].pos.y)
                .sum::<f32>()
                / 3.0
        };
        let (first, last) = (
            centre_y(&mesh.indices[..3]),
            centre_y(&mesh.indices[mesh.indices.len() - 3..]),
        );
        assert!(first < last, "far (higher on screen) first, near last");
    }

    #[test]
    fn turning_never_changes_where_the_block_is_or_its_size() {
        let area = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 300.0));
        let width = |orbit| {
            let mesh = mesh(&slotted(), orbit, Framing::default(), Color32::WHITE, area);
            let xs = mesh.vertices.iter().map(|v| v.pos.x);
            xs.clone().fold(f32::INFINITY, f32::min)..xs.fold(f32::NEG_INFINITY, f32::max)
        };
        for orbit in [Orbit::TOP, Orbit::default(), Orbit::FRONT, Orbit::RIGHT] {
            let r = width(orbit);
            assert!(
                r.start >= -1e-3 && r.end <= 400.0 + 1e-3,
                "{orbit:?}: {r:?}"
            );
        }
        let top = width(Orbit::TOP);
        assert!(
            (f32::midpoint(top.start, top.end) - 200.0).abs() < 1.0,
            "centred: {top:?}"
        );
    }

    #[test]
    fn zooming_keeps_the_point_under_the_cursor_still() {
        let at = Vec2::new(50.0, -20.0);
        let f = Framing::default().panned(Vec2::new(10.0, 5.0));
        let z = f.zoomed_about(at, 2.0);
        // a block point at offset o from its centre shows at pan + zoom * o
        let o = (at - f.pan) / f.zoom;
        assert!((z.pan + o * z.zoom - at).length() < 1e-4);
        assert!((z.zoom - 2.0).abs() < 1e-6);
    }

    #[test]
    fn mouse_buttons_pick_rotate_or_pan_as_in_aspire() {
        assert_eq!(
            drag_mode([true, false, false], false),
            Some(DragMode::Rotate)
        );
        assert_eq!(
            drag_mode([false, true, false], false),
            Some(DragMode::Rotate)
        );
        assert_eq!(drag_mode([false, false, true], false), Some(DragMode::Pan));
        assert_eq!(drag_mode([false, true, false], true), Some(DragMode::Pan));
        assert_eq!(drag_mode([true, true, false], false), Some(DragMode::Pan));
        assert_eq!(drag_mode([false, false, false], false), None);
    }

    #[test]
    fn the_standard_views_look_from_where_they_say() {
        // the side nearest the viewer has the greatest depth
        let nearest = |orbit: Orbit, a: DVec3, b: DVec3| orbit.project(a).1 > orbit.project(b).1;
        assert!(nearest(Orbit::TOP, DVec3::Z, DVec3::NEG_Z));
        assert!(nearest(Orbit::FRONT, DVec3::NEG_Y, DVec3::Y));
        assert!(nearest(Orbit::RIGHT, DVec3::X, DVec3::NEG_X));
    }

    #[test]
    fn dragging_turns_and_tilts_within_straight_down_and_side_on() {
        let size = Vec2::new(400.0, 300.0);
        let o = Orbit::TOP.dragged(Vec2::new(200.0, -300.0), size);
        assert!((o.yaw - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
        assert_eq!(o.pitch, 0.0, "cannot tilt past straight down");
        assert_eq!(
            Orbit::TOP.dragged(Vec2::new(0.0, 900.0), size).pitch,
            std::f64::consts::FRAC_PI_2
        );
    }
}
