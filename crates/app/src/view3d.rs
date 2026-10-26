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
/// How near the cursor, in pixels, a surface point picks at least.
// ponytail: fixed until the 3D view has settings
const PICK_REACH: f32 = 4.0;

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

    /// The job direction that is `v` on screen (x right, y up, z towards the viewer): the
    /// inverse of [`Orbit::project`].
    pub fn to_world(self, v: DVec3) -> DVec3 {
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let (y, z) = (
            v.y * cos_pitch - v.z * sin_pitch,
            v.y * sin_pitch + v.z * cos_pitch,
        );
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        DVec3::new(v.x * cos_yaw + y * sin_yaw, y * cos_yaw - v.x * sin_yaw, z)
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

/// Brightness of the surface over a cell, 0 to 1: lit from `light`, darker deeper.
#[expect(clippy::cast_precision_loss, reason = "cell steps are 1 or 2")]
pub fn brightness(material: &Material, column: usize, row: usize, light: DVec3) -> f64 {
    let m = material;
    let own = m.heights[row * m.columns + column];
    // an empty neighbour (NaN: no model there) counts as level with this cell
    let height = |column: usize, row: usize| {
        let h = m.heights[row * m.columns + column];
        if h.is_nan() { own } else { h }
    };
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
    lit(normal, light) * (1.0 - DEPTH_SHADE * depth)
}

/// Brightness of a surface facing `normal`, lit from `light`.
fn lit(normal: DVec3, light: DVec3) -> f64 {
    AMBIENT + (1.0 - AMBIENT) * normal.normalize().dot(light.normalize()).max(0.0)
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

/// How the block is drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    pub colour: Color32,
    /// The block's sides from the surface down to its bottom.
    pub sides: bool,
    /// Where the light comes from, in job directions.
    pub light: DVec3,
    /// Rotary jobs: drawn round the axis; the sides are then left out.
    pub wrap: Option<Wrap>,
}

impl Look {
    /// Light from the upper left and above as the job lies, or (`follow`) as the viewer sees it.
    pub fn light_from(orbit: Orbit, follow: bool) -> DVec3 {
        if follow { orbit.to_world(LIGHT) } else { LIGHT }
    }
}

/// A rotary job's material drawn round its axis instead of flat (View ▸ Auto-Wrapping View).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wrap {
    /// The axis runs along the material's x, else its y; the other way goes round.
    pub along_x: bool,
    /// The material's radius.
    pub radius: f64,
    /// The height in the material that is its outer surface.
    pub surface: f64,
}

impl Wrap {
    /// A point of the flat material (x, y from its corner, z a height) on the cylinder whose
    /// circumference is `round`: the axis along x (or y) through the origin, angle 0 up.
    pub fn place(self, p: DVec3, round: f64) -> DVec3 {
        let out = (self.radius + p.z - self.surface).max(0.0);
        let (along, across) = if self.along_x { (p.x, p.y) } else { (p.y, p.x) };
        let angle = across / round * std::f64::consts::TAU;
        let (side, up) = (out * angle.sin(), out * angle.cos());
        if self.along_x {
            DVec3::new(along, side, up)
        } else {
            DVec3::new(side, along, up)
        }
    }
}

/// The box the drawn material fills: the flat block, or the cylinder round its axis.
fn extent(material: &Material, wrap: Option<Wrap>) -> (DVec3, DVec3) {
    let size = plan_size(material);
    match wrap {
        None => {
            let top_z = material
                .heights
                .iter()
                .copied()
                .fold(material.bottom, f64::max);
            (
                DVec3::new(0.0, 0.0, material.bottom),
                DVec3::new(size.x, size.y, top_z),
            )
        }
        Some(w) if w.along_x => (
            DVec3::new(0.0, -w.radius, -w.radius),
            DVec3::new(size.x, w.radius, w.radius),
        ),
        Some(w) => (
            DVec3::new(-w.radius, 0.0, -w.radius),
            DVec3::new(w.radius, size.y, w.radius),
        ),
    }
}

/// Screen positions of points in the material's own coordinates (x, y from its corner, z up
/// from the surface) as [`mesh`] draws it: the block's bounding sphere fitted into `area`,
/// so turning never changes its size, then zoomed and panned by `framing`.
#[expect(
    clippy::cast_possible_truncation,
    reason = "screen coordinates fit f32"
)]
pub fn screen_of(
    material: &Material,
    orbit: Orbit,
    framing: Framing,
    area: Rect,
    wrap: Option<Wrap>,
) -> impl Fn(DVec3) -> Pos2 {
    let (lo, hi) = extent(material, wrap);
    let radius = ((hi - lo).length() / 2.0).max(f64::EPSILON);
    let (middle, _) = orbit.project((lo + hi) / 2.0);
    let scale =
        f64::from(area.width().min(area.height())) / (2.0 * radius) * f64::from(framing.zoom);
    let centre = area.center() + framing.pan;
    move |p: DVec3| {
        let offset = (orbit.project(p).0 - middle) * scale;
        Pos2::new(centre.x + offset.x as f32, centre.y - offset.y as f32)
    }
}

/// The height a vector is drawn at: flat, in one plane, at the highest the surface reaches
/// under it (within half a cell, so an outline on a shape's edge counts the wall's top); 0 off
/// the material.
pub fn vector_plane(material: &Material, points: &[DVec2]) -> f64 {
    let r = material.cell / 2.0;
    points
        .iter()
        .flat_map(|&p| {
            [(0.0, 0.0), (r, 0.0), (-r, 0.0), (0.0, r), (0.0, -r)]
                .map(|(dx, dy)| material.at(p + DVec2::new(dx, dy)))
        })
        .flatten()
        .filter(|h| !h.is_nan())
        .reduce(f64::max)
        .unwrap_or(0.0)
}

/// The material's width and height.
#[expect(clippy::cast_precision_loss, reason = "cell counts")]
pub fn plan_size(material: &Material) -> DVec2 {
    DVec2::new(material.columns as f64, material.rows as f64) * material.cell
}

/// A background shading from the theme's faint colour at the top to its darkest at the
/// bottom, behind the block.
pub fn background(area: Rect, visuals: &eframe::egui::Visuals) -> Mesh {
    let (top, bottom) = (visuals.faint_bg_color, visuals.extreme_bg_color);
    let mut mesh = Mesh::default();
    for (at, colour) in [
        (area.left_top(), top),
        (area.right_top(), top),
        (area.right_bottom(), bottom),
        (area.left_bottom(), bottom),
    ] {
        mesh.colored_vertex(at, colour);
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    mesh
}

/// The material seen from `orbit`, placed by [`screen_of`] and drawn far to near.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "screen coordinates and brightness fit f32; cell indices fit f64"
)]
pub fn mesh(material: &Material, orbit: Orbit, framing: Framing, look: &Look, area: Rect) -> Mesh {
    let m = material;
    let base = Rgba::from(look.colour);
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
    let mut quads: Vec<([DVec3; 4], [Color32; 4])> = match look.wrap {
        Some(wrap) => wrapped_quads(m, wrap, look.light)
            .into_iter()
            .map(|(corners, brightness)| (corners, [shade(brightness); 4]))
            .collect(),
        None => vec![],
    };
    for row in (1..m.rows).filter(|_| look.wrap.is_none()) {
        for column in 1..m.columns {
            let corners = [
                (column - 1, row - 1),
                (column, row - 1),
                (column, row),
                (column - 1, row),
            ];
            // nothing is drawn where there is no model (NaN)
            if corners
                .iter()
                .any(|&(c, r)| m.heights[r * m.columns + c].is_nan())
            {
                continue;
            }
            quads.push((
                corners.map(|(c, r)| top(c, r)),
                corners.map(|(c, r)| shade(brightness(m, c, r, look.light))),
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
    for (outward, edge) in sides
        .into_iter()
        .filter(|_| look.sides && look.wrap.is_none())
    {
        if orbit.project(outward).1 <= 0.0 {
            continue;
        }
        let colour = shade(lit(outward, look.light));
        for pair in edge.windows(2) {
            let (a, b) = (top(pair[0].0, pair[0].1), top(pair[1].0, pair[1].1));
            if a.z.is_nan() || b.z.is_nan() {
                continue;
            }
            let down = |p: DVec3| DVec3::new(p.x, p.y, m.bottom);
            quads.push(([a, b, down(b), down(a)], [colour; 4]));
        }
    }
    // painter's algorithm: far quads first
    let mut projected: Vec<([DVec3; 4], f64, [Color32; 4])> = quads
        .into_iter()
        .map(|(corners, colours)| {
            let depth = corners.iter().map(|&c| orbit.project(c).1).sum::<f64>();
            (corners, depth, colours)
        })
        .collect();
    projected.sort_by(|a, b| a.1.total_cmp(&b.1));
    let screen = screen_of(m, orbit, framing, area, look.wrap);
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

/// The cell of the drawn surface under `at` nearest the viewer, as (column, row); `None`
/// when the click is off it. Cells count as under the cursor within the larger of one
/// cell's size on screen and [`PICK_REACH`] pixels.
#[expect(clippy::cast_precision_loss, reason = "cell indices fit f64")]
pub fn pick(
    material: &Material,
    orbit: Orbit,
    framing: Framing,
    area: Rect,
    wrap: Option<Wrap>,
    at: Pos2,
) -> Option<(usize, usize)> {
    let m = material;
    let round = if wrap.is_some_and(|w| w.along_x) {
        plan_size(m).y
    } else {
        plan_size(m).x
    };
    let place = |column: usize, row: usize| {
        let p = DVec3::new(
            (column as f64 + 0.5) * m.cell,
            (row as f64 + 0.5) * m.cell,
            m.heights[row * m.columns + column],
        );
        wrap.map_or(p, |w| w.place(p, round))
    };
    let screen = screen_of(m, orbit, framing, area, wrap);
    let step = screen(place(0, 0)).distance(screen(place(1.min(m.columns - 1), 0)));
    let reach = step.max(PICK_REACH);
    let near: Vec<((usize, usize), f64, f32)> = (0..m.rows)
        .flat_map(|row| (0..m.columns).map(move |column| (column, row)))
        .filter_map(|(column, row)| {
            let p = place(column, row);
            let off = screen(p).distance(at);
            (off <= reach).then(|| ((column, row), orbit.project(p).1, off))
        })
        .collect();
    // the frontmost surface there (within a cell), and on it the cell nearest the cursor
    let front = near.iter().map(|n| n.1).fold(f64::NEG_INFINITY, f64::max);
    near.into_iter()
        .filter(|n| n.1 >= front - m.cell)
        .min_by(|a, b| a.2.total_cmp(&b.2))
        .map(|n| n.0)
}

/// The material's surface round the axis as quads, each with its brightness: lit by its own
/// facing, darker deeper, and closed across the seam where the circumference meets itself.
#[expect(clippy::cast_precision_loss, reason = "cell indices fit f64")]
fn wrapped_quads(m: &Material, wrap: Wrap, light: DVec3) -> Vec<([DVec3; 4], f64)> {
    let round = if wrap.along_x {
        plan_size(m).y
    } else {
        plan_size(m).x
    };
    let at = |column: usize, row: usize| {
        wrap.place(
            DVec3::new(
                (column as f64 + 0.5) * m.cell,
                (row as f64 + 0.5) * m.cell,
                m.heights[row * m.columns + column],
            ),
            round,
        )
    };
    // the way round wraps from its last cells back to its first
    let (columns, rows) = if wrap.along_x {
        (m.columns - 1, m.rows)
    } else {
        (m.columns, m.rows - 1)
    };
    let mut quads = vec![];
    for row in 0..rows {
        for column in 0..columns {
            let (next_column, next_row) = ((column + 1) % m.columns, (row + 1) % m.rows);
            let corners = [
                at(column, row),
                at(next_column, row),
                at(next_column, next_row),
                at(column, next_row),
            ];
            let mut normal = (corners[2] - corners[0]).cross(corners[3] - corners[1]);
            // outward: away from the axis
            let middle = corners.iter().copied().sum::<DVec3>() / 4.0;
            let out = if wrap.along_x {
                DVec3::new(0.0, middle.y, middle.z)
            } else {
                DVec3::new(middle.x, 0.0, middle.z)
            };
            if normal.dot(out) < 0.0 {
                normal = -normal;
            }
            let depth = if m.bottom < 0.0 {
                (m.heights[row * m.columns + column] - wrap.surface) / m.bottom
            } else {
                0.0
            };
            let brightness = if normal.length() > 0.0 {
                lit(normal, light) * (1.0 - DEPTH_SHADE * depth.clamp(0.0, 1.0))
            } else {
                AMBIENT
            };
            quads.push((corners, brightness));
        }
    }
    quads
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::document::Job;
    use cut_craft_core::geometry::Point;
    use eframe::egui::pos2;

    fn plain() -> Look {
        Look {
            colour: Color32::WHITE,
            sides: true,
            light: LIGHT,
            wrap: None,
        }
    }

    #[test]
    fn a_vector_is_flat_at_the_top_of_what_it_outlines() {
        // the left half of the job raised 5: an outline along its edge, and a line from the
        // floor onto it, lie in one plane on top
        let job = Job::default();
        let mut m = Material::new(&job, 40);
        let edge = m.origin.x + 20.0 * m.cell;
        for row in 0..m.rows {
            for column in 0..20 {
                m.heights[row * m.columns + column] += 5.0;
            }
        }
        let (top, y) = (m.heights[0], m.origin.y + 3.3 * m.cell);
        for nudge in [-0.4, -1e-9, 0.0, 1e-9, 0.4] {
            let along = [
                Point::new(edge + nudge * m.cell, y),
                Point::new(edge, y + 5.0),
            ];
            assert_eq!(
                vector_plane(&m, &along),
                top,
                "{nudge} of a cell off the edge"
            );
        }
        let across = [
            Point::new(edge + 10.0 * m.cell, y),
            Point::new(edge - 2.0, y),
        ];
        assert_eq!(vector_plane(&m, &across), top);
        let floor = [Point::new(edge + 3.0 * m.cell, y)];
        assert_eq!(vector_plane(&m, &floor), m.at(floor[0]).unwrap());
        assert_eq!(
            vector_plane(&m, &[Point::new(-1e6, 0.0)]),
            0.0,
            "off the material"
        );
    }

    #[test]
    fn to_world_undoes_project() {
        let orbit = Orbit {
            yaw: 0.7,
            pitch: 0.4,
        };
        let p = DVec3::new(3.0, -2.0, 5.0);
        let (screen, depth) = orbit.project(p);
        assert!(orbit.to_world(screen.extend(depth)).abs_diff_eq(p, 1e-9));
    }

    #[test]
    fn hiding_the_sides_leaves_only_the_surface() {
        let area = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 300.0));
        let with = mesh(
            &slotted(),
            Orbit::default(),
            Framing::default(),
            &plain(),
            area,
        );
        let without = mesh(
            &slotted(),
            Orbit::default(),
            Framing::default(),
            &Look {
                sides: false,
                ..plain()
            },
            area,
        );
        assert!(without.indices.len() < with.indices.len());
    }

    #[test]
    fn nothing_is_drawn_where_there_is_no_model() {
        let area = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 300.0));
        let full = mesh(
            &slotted(),
            Orbit::default(),
            Framing::default(),
            &plain(),
            area,
        );
        let mut half = slotted();
        let columns = half.columns;
        for (i, h) in half.heights.iter_mut().enumerate() {
            if i % columns < columns / 2 {
                *h = f64::NAN;
            }
        }
        let shown = mesh(&half, Orbit::default(), Framing::default(), &plain(), area);
        assert!(shown.indices.len() < full.indices.len() * 2 / 3);
        assert!(
            shown
                .vertices
                .iter()
                .all(|v| v.pos.x.is_finite() && v.pos.y.is_finite()),
            "no empty cell reaches the screen"
        );
        assert!(
            brightness(&half, columns / 2, 3, LIGHT).is_finite(),
            "an edge cell is lit"
        );
    }

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
        let flat = brightness(&m, 10, 50, LIGHT);
        assert!(
            brightness(&m, 60, 50, LIGHT) > flat,
            "the wall rising towards +x faces the light"
        );
        assert!(
            brightness(&m, 39, 50, LIGHT) < flat,
            "the wall dropping towards +x faces away"
        );
        assert!(
            brightness(&m, 50, 50, LIGHT) < flat,
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
        let mesh = mesh(&slotted(), orbit, Framing::default(), &plain(), area);
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
            let mesh = mesh(&slotted(), orbit, Framing::default(), &plain(), area);
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

#[cfg(test)]
mod wrap_tests {
    use super::*;
    use cut_craft_core::document::{Along, Job};
    use eframe::egui::pos2;

    fn wrap() -> Wrap {
        Wrap {
            along_x: true,
            radius: 10.0,
            surface: 0.0,
        }
    }

    #[test]
    fn the_surface_goes_round_the_axis_from_the_top() {
        let round = 20.0 * std::f64::consts::PI;
        let top = wrap().place(DVec3::new(5.0, 0.0, 0.0), round);
        assert!(top.abs_diff_eq(DVec3::new(5.0, 0.0, 10.0), 1e-9));
        let quarter = wrap().place(DVec3::new(5.0, round / 4.0, -2.0), round);
        assert!(
            quarter.abs_diff_eq(DVec3::new(5.0, 8.0, 0.0), 1e-9),
            "{quarter}"
        );
    }

    #[test]
    fn a_rotary_material_is_drawn_closed_round_and_inside_the_view() {
        let job = Job::rotary(60.0, 20.0, Along::X);
        let m = Material::new(&job, 60);
        let area = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 300.0));
        let look = Look {
            colour: Color32::WHITE,
            sides: true,
            light: LIGHT,
            wrap: Some(wrap()),
        };
        let quads = wrapped_quads(&m, wrap(), LIGHT);
        assert_eq!(
            quads.len(),
            (m.columns - 1) * m.rows,
            "every row, the seam too"
        );
        let drawn = mesh(&m, Orbit::default(), Framing::default(), &look, area);
        assert_eq!(drawn.vertices.len(), 4 * quads.len());
        assert!(
            drawn
                .vertices
                .iter()
                .all(|v| area.expand(1.0).contains(v.pos))
        );
    }
}

#[cfg(test)]
mod pick_tests {
    use super::*;
    use cut_craft_core::document::Job;
    use eframe::egui::pos2;

    #[test]
    fn from_above_the_cell_under_the_cursor_is_picked_and_off_the_block_nothing() {
        let job = Job {
            width: 100.0,
            height: 50.0,
            ..Job::default()
        };
        let m = Material::new(&job, 100);
        let area = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 300.0));
        let screen = screen_of(&m, Orbit::TOP, Framing::default(), area, None);
        let at = screen(DVec3::new(30.5, 10.5, 0.0));
        assert_eq!(
            pick(&m, Orbit::TOP, Framing::default(), area, None, at),
            Some((30, 10))
        );
        let far = pos2(-50.0, -50.0);
        assert_eq!(
            pick(&m, Orbit::TOP, Framing::default(), area, None, far),
            None
        );
    }
}
