//! The 3D view: the simulated material as a lit block, turned, tilted, panned and zoomed.

use cut_craft_core::glam::{DVec2, DVec3};
use cut_craft_core::simulation::Material;
use eframe::egui::{
    Align2, Color32, FontId, Mesh, Painter, Pos2, Rect, Rgba, Shape, Vec2, Visuals,
};

pub mod gpu;

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
/// How wide (across the view's shorter side) the perspective view sees, radians.
// ponytail: fixed until Options has a camera setting; 45 degrees is the usual lens
const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_4;
/// View cube faces nearer edge-on than this (the cosine of their angle to the viewer) are
/// left out: they have no area to click.
const EDGE_ON: f64 = 1e-6;

/// Where the view looks from: `yaw` turns the job about the vertical, `pitch` tilts it from
/// straight down (0) through side on (a right angle) to straight up from below (half a
/// turn), and `roll` turns the picture about the line of sight (0 keeps the job's up up),
/// radians. Together they can face any way (Z-X-Z angles), so free rotation needs nothing
/// more.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Orbit {
    pub yaw: f64,
    pub pitch: f64,
    pub roll: f64,
}

impl Default for Orbit {
    fn default() -> Self {
        // ponytail: one starting angle until the view menu has standard views
        Self {
            yaw: -20f64.to_radians(),
            pitch: 50f64.to_radians(),
            roll: 0.0,
        }
    }
}

/// How dragging (and a SpaceMouse) turns the 3D view.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Rotation {
    /// About the job's vertical and tilting between straight down and straight up: the job's
    /// up stays up, as on a machine bed.
    Turntable,
    /// Any way, without stops: the job can roll and go upside down.
    #[default]
    Free,
}

impl Orbit {
    /// Looking straight down on the job.
    pub const TOP: Self = Self {
        yaw: 0.0,
        pitch: 0.0,
        roll: 0.0,
    };
    /// Side on, from the front (-y) edge.
    pub const FRONT: Self = Self {
        yaw: 0.0,
        pitch: std::f64::consts::FRAC_PI_2,
        roll: 0.0,
    };
    /// Side on, from the right (+x) edge.
    pub const RIGHT: Self = Self {
        yaw: -std::f64::consts::FRAC_PI_2,
        pitch: std::f64::consts::FRAC_PI_2,
        roll: 0.0,
    };

    /// Turned by a drag of `by` across a view `size` as `rotation` says: across the whole
    /// view is half a turn, sideways turning and up/down tilting (a turntable between
    /// straight down and straight up, so past side on to underneath, the job's up staying
    /// up), or freely about the screen's own axes, without stops.
    #[must_use]
    pub fn dragged_by(self, by: Vec2, size: Vec2, rotation: Rotation) -> Self {
        let turn = |d: f32, across: f32| f64::from(d / across.max(1.0)) * std::f64::consts::PI;
        // a drag down tilts the job's top towards the viewer
        self.turned_by(
            DVec3::new(-turn(by.y, size.y), turn(by.x, size.x), 0.0),
            rotation,
        )
    }

    /// Turned about the screen's axes as [`Orbit::turned`] says, freely, or as a turntable
    /// takes it: the turn about the screen's y spins the job about its vertical, the one
    /// about x tilts it (between straight down and straight up), and roll is left out.
    #[must_use]
    pub fn turned_by(self, by: DVec3, rotation: Rotation) -> Self {
        match rotation {
            Rotation::Free => self.turned(by),
            Rotation::Turntable => Self {
                yaw: self.yaw + by.y,
                pitch: (self.pitch - by.x).clamp(0.0, std::f64::consts::PI),
                roll: 0.0,
            },
        }
    }

    /// Turned about the screen's axes (x right, y up, z towards the viewer) by these angles,
    /// radians: the job's front moves right for a positive turn about y, down for one about
    /// x, and the picture turns anticlockwise for one about z.
    #[must_use]
    pub fn turned(self, by: DVec3) -> Self {
        let screen = cut_craft_core::glam::DQuat::from_rotation_x(by.x)
            * cut_craft_core::glam::DQuat::from_rotation_y(by.y)
            * cut_craft_core::glam::DQuat::from_rotation_z(by.z);
        Self::from_matrix(cut_craft_core::glam::DMat3::from_quat(screen) * self.matrix())
    }

    /// Job directions to the screen's (x right, y up, z towards the viewer).
    fn matrix(self) -> cut_craft_core::glam::DMat3 {
        let column = |v: DVec3| {
            let (p, depth) = self.project(v);
            DVec3::new(p.x, p.y, depth)
        };
        cut_craft_core::glam::DMat3::from_cols(column(DVec3::X), column(DVec3::Y), column(DVec3::Z))
    }

    /// The view whose [`Orbit::matrix`] is `m`.
    fn from_matrix(m: cut_craft_core::glam::DMat3) -> Self {
        // the job direction towards the viewer gives yaw and pitch, as `facing` does
        let towards = m.transpose() * DVec3::Z;
        let level = Self {
            roll: 0.0,
            ..Self::facing(towards)
        };
        // and what is on the screen's right then, how far the picture is turned
        let right = level.project(m.transpose() * DVec3::X).0;
        Self {
            roll: (-right.y).atan2(right.x),
            ..level
        }
    }

    /// Looking from `towards` (a job direction, any length); straight down or up keep the
    /// plan's x to the right.
    pub fn facing(towards: DVec3) -> Self {
        let d = towards.normalize();
        // `towards_viewer` is (-sin pitch sin yaw, -sin pitch cos yaw, cos pitch)
        let yaw = if d.x.hypot(d.y) <= f64::EPSILON {
            0.0
        } else {
            (-d.x).atan2(-d.y)
        };
        Self {
            yaw,
            pitch: d.z.clamp(-1.0, 1.0).acos(),
            roll: 0.0,
        }
    }

    /// The job direction towards the viewer.
    pub fn towards_viewer(self) -> DVec3 {
        self.to_world(DVec3::Z)
    }

    /// The job direction that is `v` on screen (x right, y up, z towards the viewer): the
    /// inverse of [`Orbit::project`].
    pub fn to_world(self, v: DVec3) -> DVec3 {
        let (sin_roll, cos_roll) = self.roll.sin_cos();
        let v = DVec3::new(
            v.x * cos_roll + v.y * sin_roll,
            v.y * cos_roll - v.x * sin_roll,
            v.z,
        );
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
        let (sx, sy) = (x, y * cos_pitch + p.z * sin_pitch);
        let (sin_roll, cos_roll) = self.roll.sin_cos();
        (
            DVec2::new(sx * cos_roll - sy * sin_roll, sx * sin_roll + sy * cos_roll),
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
    /// Seen in perspective (nearer is larger) rather than orthographic.
    pub perspective: bool,
}

impl Default for Framing {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: Vec2::ZERO,
            perspective: false,
        }
    }
}

impl Framing {
    /// Fitted to the view again, in the same projection.
    #[must_use]
    pub fn fitted(self) -> Self {
        Self {
            perspective: self.perspective,
            ..Self::default()
        }
    }

    /// Zoomed by `factor` keeping the point `at` (from the view's centre) still.
    #[must_use]
    pub fn zoomed_about(self, at: Vec2, factor: f32) -> Self {
        // ponytail: limits keep the block findable; a "zoom to selection" can widen them
        let zoom = (self.zoom * factor).clamp(0.05, 200.0);
        Self {
            zoom,
            pan: at - (at - self.pan) * (zoom / self.zoom),
            ..self
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

/// `colour` at `brightness` (0 to 1), shaded in linear light.
#[expect(clippy::cast_possible_truncation, reason = "brightness fits f32")]
fn shaded(colour: Color32, brightness: f64) -> Color32 {
    let (base, b) = (Rgba::from(colour), brightness as f32);
    Color32::from(Rgba::from_rgb(base.r() * b, base.g() * b, base.b() * b))
}

/// `material` with at most `most` cells along its longer side, each the deepest of the
/// cells it covers so narrow cuts stay visible, and marked and painted as that cell is.
#[expect(clippy::cast_precision_loss, reason = "a small step")]
pub fn coarse(material: &Material, most: usize) -> Material {
    let m = material;
    let step = m.columns.max(m.rows).div_ceil(most.max(1)).max(1);
    let (columns, rows) = (m.columns.div_ceil(step), m.rows.div_ceil(step));
    let span = |block: usize, count: usize| block * step..((block + 1) * step).min(count);
    // each block's deepest cell; a block of cells all taken away (NaN) has none
    let deepest: Vec<Option<usize>> = (0..rows)
        .flat_map(|row| (0..columns).map(move |column| (column, row)))
        .map(|(column, row)| {
            span(row, m.rows)
                .flat_map(|r| span(column, m.columns).map(move |c| r * m.columns + c))
                .filter(|&i| !m.heights[i].is_nan())
                .min_by(|&a, &b| m.heights[a].total_cmp(&m.heights[b]))
        })
        .collect();
    let of = |cells: &[u32]| -> Vec<u32> {
        if cells.is_empty() {
            return vec![];
        }
        deepest.iter().map(|d| d.map_or(0, |i| cells[i])).collect()
    };
    Material {
        origin: m.origin,
        columns,
        rows,
        cell: m.cell * step as f64,
        bottom: m.bottom,
        heights: deepest
            .iter()
            .map(|d| d.map_or(f64::NAN, |i| m.heights[i]))
            .collect(),
        cut_by: of(&m.cut_by),
        cutting: m.cutting,
        paint: of(&m.paint),
        other_side: m.other_side.clone(),
        // the underside under each block's deepest cell, as its top
        under: if m.under.is_empty() {
            vec![]
        } else {
            deepest
                .iter()
                .map(|d| d.map_or(f64::NAN, |i| m.under[i]))
                .collect()
        },
    }
}

/// `material` painted for the preview: each cell cut by a toolpath in the colour `fill`
/// gives that toolpath's id, if it gives one.
pub fn painted(mut material: Material, fill: impl Fn(u32) -> Option<Color32>) -> Material {
    let paint: Vec<u32> = material
        .cut_by
        .iter()
        .map(|&id| {
            (id != 0)
                .then(|| fill(id))
                .flatten()
                .map_or(0, |c| u32::from_be_bytes([1, c.r(), c.g(), c.b()]))
        })
        .collect();
    material.paint = if paint.iter().any(|&p| p != 0) {
        paint
    } else {
        vec![]
    };
    material
}

/// `material` painted as a lithophane lit from behind: white where nothing is left, `colour`
/// (the material's) at full thickness, and between by the share left, raised to a power the
/// `light` sets: 0 is high contrast (the colour soon), 1 light (white long) [inf].
#[expect(clippy::cast_possible_truncation, reason = "a share of the thickness")]
pub fn lit_through(mut material: Material, colour: Color32, light: f64) -> Material {
    let power = 4f64.powf(2.0 * light.clamp(0.0, 1.0) - 1.0);
    let thickness = -material.bottom;
    let (white, base) = (Rgba::WHITE, Rgba::from(colour));
    material.paint = material
        .heights
        .iter()
        .map(|&height| {
            if height.is_nan() || thickness <= 0.0 {
                return 0;
            }
            let left = ((height - material.bottom) / thickness)
                .clamp(0.0, 1.0)
                .powf(power);
            let c = Color32::from(white * (1.0 - left as f32) + base * left as f32);
            u32::from_be_bytes([1, c.r(), c.g(), c.b()])
        })
        .collect();
    material
}

/// A cell's preview colour, if it is painted.
fn paint_of(material: &Material, cell: usize) -> Option<Color32> {
    let [flag, red, green, blue] = material.paint.get(cell)?.to_be_bytes();
    (flag != 0).then(|| Color32::from_rgb(red, green, blue))
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
    /// Wood's grain on the top, for a wood from Toolpaths ▸ Materials.
    pub grain: Option<Grain>,
}

/// How far grain lines drift from straight, in line spacings, and over how many spacings
/// along the grain: a long slow drift and a shorter one at an unrelated length, so the
/// lines never settle into a repeating ripple.
const GRAIN_DRIFTS: [(f64, f64); 2] = [(1.6, 53.0), (0.45, 17.3)];
/// Higher makes the dark lines thinner beside the light wood between them.
const GRAIN_SHARPNESS: i32 = 6;
/// How far the lines move across the board for each mm the surface goes down into it, as a
/// plain-sawn board's rings do: carved hollows show rings round them.
const GRAIN_DEPTH_SHIFT: f64 = 3.0;

/// How far lines bunch together and spread apart across the board (early and late wood),
/// in line spacings, and how many spacings each bunching spans: two unrelated lengths.
const GRAIN_BUNCHES: [(f64, f64); 2] = [(0.6, 7.0), (0.3, 3.1)];
/// The mesh cells a line spacing needs for its lines to be drawn at full strength; fewer,
/// and the grain fades rather than shimmering between cells.
const GRAIN_CELLS_PER_LINE: f64 = 6.0;

/// Wood grain: dark lines along X, `spacing` mm apart, `depth` darker than the wood.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grain {
    pub spacing: f64,
    pub depth: f32,
}

impl Grain {
    /// As drawn on a mesh of `cell` mm cells: fainter where they are too coarse for it.
    pub fn on_cells(self, cell: f64) -> Self {
        #[expect(clippy::cast_possible_truncation, reason = "a share in 0..=1")]
        let strength = (self.spacing / cell / GRAIN_CELLS_PER_LINE).min(1.0) as f32;
        Self {
            depth: self.depth * strength,
            ..self
        }
    }

    /// How lit the wood is at `p` (job space): 1 between lines, `1 - depth` on one.
    pub fn at(self, p: DVec3) -> f64 {
        use std::f64::consts::TAU;
        let s = self.spacing;
        let waves = |along: f64, waves: &[(f64, f64)]| -> f64 {
            waves
                .iter()
                .map(|&(size, length)| size * s * (TAU * along / (length * s)).sin())
                .sum()
        };
        let across =
            p.y + waves(p.x, &GRAIN_DRIFTS) + waves(p.y, &GRAIN_BUNCHES) + GRAIN_DEPTH_SHIFT * p.z;
        let line = (0.5 + 0.5 * (TAU * across / s).cos()).powi(GRAIN_SHARPNESS);
        1.0 - f64::from(self.depth) * line
    }
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

/// How the view sees the material: its bounding sphere fitted into the view, so turning
/// never changes its size, then zoomed and panned. In perspective the eye stands just far
/// enough off for the sphere to fill [`FIELD_OF_VIEW`].
#[derive(Clone, Copy, Debug)]
pub(crate) struct Camera {
    orbit: Orbit,
    /// The bounding sphere's middle, in the material's own coordinates, and its radius.
    middle: DVec3,
    radius: f64,
    /// In perspective, the eye's distance from `middle`.
    eye: Option<f64>,
    /// Points on screen per mm (at `middle`'s depth, in perspective).
    scale: f64,
    /// Where `middle` shows.
    centre: Pos2,
}

impl Camera {
    pub(crate) fn new(
        material: &Material,
        orbit: Orbit,
        framing: Framing,
        area: Rect,
        wrap: Option<Wrap>,
    ) -> Self {
        let (lo, hi) = extent(material, wrap);
        let radius = ((hi - lo).length() / 2.0).max(f64::EPSILON);
        let half = FIELD_OF_VIEW / 2.0;
        // the sphere's outline, seen from the eye, is half the field of view off its middle
        let eye = framing.perspective.then(|| radius / half.sin());
        // how far from the middle, on the plane through it, the view's edge is
        let reach = eye.map_or(radius, |e| e * half.tan());
        Self {
            orbit,
            middle: (lo + hi) / 2.0,
            radius,
            eye,
            scale: f64::from(area.width().min(area.height())) / (2.0 * reach)
                * f64::from(framing.zoom),
            centre: area.center() + framing.pan,
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "screen coordinates fit f32"
    )]
    pub(crate) fn screen(&self, p: DVec3) -> Pos2 {
        let (at, depth) = self.orbit.project(p - self.middle);
        // nearer the eye is larger; nothing drawn is as near as the eye, outside the sphere
        let nearer = self.eye.map_or(1.0, |e| e / (e - depth).max(f64::EPSILON));
        let offset = at * nearer * self.scale;
        Pos2::new(
            self.centre.x + offset.x as f32,
            self.centre.y - offset.y as f32,
        )
    }

    /// Points on screen per mm (at the middle's depth, in perspective).
    pub(crate) fn scale(&self) -> f64 {
        self.scale
    }

    /// The line of sight through `at` on screen: the point on it at the middle's depth, and
    /// its direction away from the viewer.
    fn sight(&self, at: Pos2) -> (DVec3, DVec3) {
        let off = at - self.centre;
        let seen = self.middle
            + self.orbit.to_world(DVec3::new(
                f64::from(off.x) / self.scale,
                -f64::from(off.y) / self.scale,
                0.0,
            ));
        let along = self
            .eye_at()
            .map_or(-self.orbit.towards_viewer(), |eye| seen - eye)
            .normalize();
        (seen, along)
    }

    /// The point on the level plane at height `z` (0 is the material's top) under `at` on
    /// screen, `None` when the plane is seen edge-on or, in perspective, `at` is above its
    /// horizon.
    pub(crate) fn on_plane(&self, at: Pos2, z: f64) -> Option<DVec2> {
        // the point at the middle's depth under `at`, then along the line of sight to z
        let (seen, along) = self.sight(at);
        if along.z.abs() <= EDGE_ON {
            return None;
        }
        // in perspective, a point above the horizon looks away from the plane: the line
        // of sight only meets it behind the eye
        if let Some(eye) = self.eye_at()
            && (z - eye.z) / along.z <= 0.0
        {
            return None;
        }
        Some((seen - along * ((seen.z - z) / along.z)).truncate())
    }

    /// Where the line of sight through `at` first meets the solid of `material` (unwrapped),
    /// as a point on its plan: what a vector drawn there in the 3D view lies on. Off the
    /// material, the top plane under `at` ([`Camera::on_plane`]).
    // ponytail: stepped half a cell at a time, then halved down to a hair between the last
    // step outside and the first inside, as `seen_of` steps; walk the cells if it shows
    pub(crate) fn on_surface(&self, material: &Material, at: Pos2) -> Option<DVec2> {
        let m = material;
        let (seen, along) = self.sight(at);
        let (lo, _) = extent(m, None);
        let inside = |t: f64| {
            let q = seen + along * t;
            q.z >= lo.z && m.at(m.origin + q.truncate()).is_some_and(|h| q.z <= h)
        };
        // from outside the bounding sphere (or the eye, nearer) to past its far side
        let from = self
            .eye_at()
            .map_or(-2.0 * self.radius, |eye| -(seen - eye).length());
        let step = m.cell / 2.0;
        let mut t = from;
        while t < 2.0 * self.radius {
            if inside(t) {
                let (mut outside, mut within) = ((t - step).max(from), t);
                for _ in 0..40 {
                    let mid = f64::midpoint(outside, within);
                    if inside(mid) {
                        within = mid;
                    } else {
                        outside = mid;
                    }
                }
                return Some((seen + along * within).truncate());
            }
            t += step;
        }
        self.on_plane(at, 0.0)
    }

    /// Material coordinates to clip space over `area` (x and y across it, -1 to 1), placing
    /// points as [`Camera::screen`] does in `area`; depth runs from 0 at `reach` radii
    /// nearer than the middle to 1 as far behind it.
    fn clip(&self, area: Rect, reach: f64) -> cut_craft_core::glam::DMat4 {
        use cut_craft_core::glam::{DMat4, DVec4};
        let rotation = self.orbit.matrix();
        let (right, up, depth) = (rotation.row(0), rotation.row(1), rotation.row(2));
        let half = area.size() / 2.0;
        let off = self.centre - area.center();
        let reach = self.radius * reach;
        // the fourth clip coordinate: 1, or in perspective the distance from the eye
        let (nearer, fourth) = match self.eye {
            None => (1.0, DVec4::W),
            Some(eye) => (eye, (-depth).extend(eye)),
        };
        let along = |axis: DVec3, half: f32, off: f32| {
            axis.extend(0.0) * (self.scale * nearer / f64::from(half))
                + fourth * (f64::from(off) / f64::from(half))
        };
        let into = match self.eye {
            None => (-depth / (2.0 * reach)).extend(0.5),
            Some(eye) => {
                // far / (far - near) times (distance - near), near and far `reach` about the middle
                let spread = (eye + reach) / (2.0 * reach);
                (-depth * spread).extend(spread * reach)
            }
        };
        let rows = DMat4::from_cols(
            along(right, half.x, off.x),
            along(up, half.y, -off.y),
            into,
            fourth,
        );
        rows.transpose() * DMat4::from_translation(-self.middle)
    }

    /// Where the eye is, in perspective.
    fn eye_at(&self) -> Option<DVec3> {
        self.eye
            .map(|e| self.middle + self.orbit.towards_viewer() * e)
    }

    /// The direction from `p` towards the eye.
    fn towards_eye(&self, p: DVec3) -> DVec3 {
        self.eye_at()
            .map_or(self.orbit.towards_viewer(), |eye| (eye - p).normalize())
    }

    /// Whether the plane through `p` facing `outward` faces the eye.
    fn faces(&self, p: DVec3, outward: DVec3) -> bool {
        self.towards_eye(p).dot(outward) > 0.0
    }
}

/// Screen positions of points in the material's own coordinates (x, y from its corner, z up
/// from the surface) as [`mesh`] draws it: the block's bounding sphere fitted into `area`,
/// so turning never changes its size, then zoomed and panned (and seen in perspective) by
/// `framing`.
pub fn screen_of(
    material: &Material,
    orbit: Orbit,
    framing: Framing,
    area: Rect,
    wrap: Option<Wrap>,
) -> impl Fn(DVec3) -> Pos2 {
    let camera = Camera::new(material, orbit, framing, area, wrap);
    move |p: DVec3| camera.screen(p)
}

/// Whether points in the material's own coordinates are in sight, or hidden by the surface
/// (or the block under it) as [`mesh`] draws it unwrapped: the overlays drawn over the mesh
/// leave out what this hides.
// ponytail: a ray from the point towards the eye, stepped half a cell at a time over the
// height grid until it is above (or below) everything or off the material. The surface is a
// height grid, so this is exact to a cell; the overlays stay egui's own lines rather than
// going through the GPU's depth buffer with the surface. Walk the cells (DDA) instead if long
// side-on rays ever show in a profile.
pub fn seen_of(material: &Material, orbit: Orbit, framing: Framing) -> impl Fn(DVec3) -> bool {
    let m = material;
    // only the eye is wanted, which is the same whatever size the view is
    let camera = Camera::new(m, orbit, framing, Rect::ZERO, None);
    let (lo, hi) = extent(m, None);
    let step = m.cell / 2.0;
    // off the material and heading further off: nothing more can be in the way
    let leaving = |at: f64, heading: f64, size: f64| {
        (at < 0.0 && heading <= 0.0) || (at > size && heading >= 0.0)
    };
    move |p: DVec3| {
        let towards = camera.towards_eye(p);
        let far = camera
            .eye_at()
            .map_or(f64::INFINITY, |eye| (eye - p).length());
        // a cell clear of the point first, so a line lying on the surface is not hidden by
        // the cell it lies on
        let mut t = m.cell;
        while t < far {
            let q = p + towards * t;
            let past = (towards.z >= 0.0 && q.z >= hi.z) || (towards.z <= 0.0 && q.z <= lo.z);
            if past || leaving(q.x, towards.x, hi.x) || leaving(q.y, towards.y, hi.y) {
                return true;
            }
            let under = m.at(m.origin + q.truncate()).is_some_and(|h| q.z < h);
            if under && q.z > lo.z {
                return false;
            }
            t += step;
        }
        true
    }
}

/// The parts of the line through `points` that are `seen`, each a line of its own: every
/// segment is cut into pieces no longer than `step` first, so the surface can hide part of
/// one.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "a piece count"
)]
pub fn visible_runs(points: &[DVec3], step: f64, seen: impl Fn(DVec3) -> bool) -> Vec<Vec<DVec3>> {
    let mut samples: Vec<DVec3> = points.first().copied().into_iter().collect();
    for pair in points.windows(2) {
        let pieces = ((pair[1] - pair[0]).length() / step.max(f64::EPSILON))
            .ceil()
            .max(1.0) as usize;
        samples.extend((1..=pieces).map(|i| pair[0].lerp(pair[1], i as f64 / pieces as f64)));
    }
    let mut runs = vec![vec![]];
    for p in samples {
        if seen(p) {
            runs.last_mut().expect("never empty").push(p);
        } else if runs.last().is_some_and(|run| !run.is_empty()) {
            runs.push(vec![]);
        }
    }
    runs.retain(|run| run.len() >= 2);
    runs
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

/// The outline on screen of an upright cylinder standing on `tip` (the moving tool in a
/// playing preview): the convex hull of its top and bottom rims, `screen` placing points.
#[expect(clippy::cast_precision_loss, reason = "a few rim points")]
pub fn cylinder(screen: impl Fn(DVec3) -> Pos2, tip: DVec3, radius: f64, height: f64) -> Vec<Pos2> {
    // ponytail: 24 points round each rim
    const RIM: usize = 24;
    let mut points: Vec<Pos2> = (0..RIM)
        .flat_map(|i| {
            let a = std::f64::consts::TAU * i as f64 / RIM as f64;
            let round = DVec3::new(a.cos() * radius, a.sin() * radius, 0.0);
            [screen(tip + round), screen(tip + round + DVec3::Z * height)]
        })
        .collect();
    // convex hull (monotone chain) of the projected rims
    points.sort_by(|p, q| p.x.total_cmp(&q.x).then(p.y.total_cmp(&q.y)));
    let turn = |o: Pos2, a: Pos2, b: Pos2| (a - o).x * (b - o).y - (a - o).y * (b - o).x;
    let mut hull: Vec<Pos2> = vec![];
    for pass in [false, true] {
        let start = hull.len();
        let ordered: Vec<Pos2> = if pass {
            points.iter().rev().copied().collect()
        } else {
            points.clone()
        };
        for p in ordered {
            while hull.len() >= start + 2
                && turn(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0.0
            {
                hull.pop();
            }
            hull.push(p);
        }
        hull.pop();
    }
    hull
}

/// The material's width and height.
#[expect(clippy::cast_precision_loss, reason = "cell counts")]
pub fn plan_size(material: &Material) -> DVec2 {
    DVec2::new(material.columns as f64, material.rows as f64) * material.cell
}

/// A background shading from `top` to `bottom`, behind the block.
pub fn background(area: Rect, top: Color32, bottom: Color32) -> Mesh {
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
#[expect(clippy::cast_precision_loss, reason = "cell indices fit f64")]
pub fn mesh(material: &Material, orbit: Orbit, framing: Framing, look: &Look, area: Rect) -> Mesh {
    let m = material;
    let shade = |brightness: f64| shaded(look.colour, brightness);
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
            let grain = |c, r| look.grain.map_or(1.0, |g| g.on_cells(m.cell).at(top(c, r)));
            let colour = |c: usize, r: usize| {
                let b = brightness(m, c, r, look.light);
                match paint_of(m, r * m.columns + c) {
                    Some(paint) => shaded(paint, b),
                    None => shade(b * grain(c, r)),
                }
            };
            quads.push((
                corners.map(|(c, r)| top(c, r)),
                corners.map(|(c, r)| colour(c, r)),
            ));
        }
    }
    // painter's algorithm: far quads first
    let mut projected: Vec<(f64, [DVec3; 4], [Color32; 4])> = quads
        .into_iter()
        .map(|(corners, colours)| {
            let depth = corners.iter().map(|&c| orbit.project(c).1).sum::<f64>();
            (depth, corners, colours)
        })
        .collect();
    projected.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut quads: Vec<_> = projected.into_iter().map(|(_, q, c)| (q, c)).collect();
    let camera = Camera::new(m, orbit, framing, area, look.wrap);
    if look.sides && look.wrap.is_none() {
        quads.extend(block_faces(m, &top, &camera, orbit, &shade, look.light));
    }
    let mut mesh = Mesh::default();
    for (corners, colours) in quads {
        let first = u32::try_from(mesh.vertices.len()).expect("mesh fits u32 indices");
        for (corner, colour) in corners.into_iter().zip(colours) {
            mesh.colored_vertex(camera.screen(corner), colour);
        }
        mesh.add_triangle(first, first + 1, first + 2);
        mesh.add_triangle(first, first + 2, first + 3);
    }
    mesh
}

/// The block's sides that face the viewer, from the surface down to its underside, and the
/// underside seen from below: flat, or (a double-sided preview) cut from below. The block
/// is all behind them, so they go after the top.
fn block_faces(
    m: &Material,
    top: &dyn Fn(usize, usize) -> DVec3,
    camera: &Camera,
    orbit: Orbit,
    shade: &dyn Fn(f64) -> Color32,
    light: DVec3,
) -> Vec<([DVec3; 4], [Color32; 4])> {
    let mut quads = vec![];
    let (last_column, last_row) = (m.columns - 1, m.rows - 1);
    let sides: [(DVec3, Vec<(usize, usize)>); 4] = [
        (DVec3::NEG_Y, (0..m.columns).map(|c| (c, 0)).collect()),
        (DVec3::Y, (0..m.columns).map(|c| (c, last_row)).collect()),
        (DVec3::NEG_X, (0..m.rows).map(|r| (0, r)).collect()),
        (DVec3::X, (0..m.rows).map(|r| (last_column, r)).collect()),
    ];
    // the underside: flat at the bottom, or (a double-sided preview) cut from below
    let floor = |c: usize, r: usize| {
        let p = top(c, r);
        let z = m.under.get(r * m.columns + c).copied().unwrap_or(m.bottom);
        DVec3::new(p.x, p.y, z)
    };
    for (outward, edge) in sides {
        if !camera.faces(floor(edge[0].0, edge[0].1), outward) {
            continue;
        }
        let colour = shade(lit(outward, light));
        for pair in edge.windows(2) {
            let (a, b) = (top(pair[0].0, pair[0].1), top(pair[1].0, pair[1].1));
            let (fa, fb) = (floor(pair[0].0, pair[0].1), floor(pair[1].0, pair[1].1));
            if a.z.is_nan() || b.z.is_nan() || fa.z.is_nan() || fb.z.is_nan() {
                continue;
            }
            quads.push(([a, b, fb, fa], [colour; 4]));
        }
    }
    if m.under.is_empty() {
        let bottom = [
            (0, 0),
            (last_column, 0),
            (last_column, last_row),
            (0, last_row),
        ]
        .map(|(c, r)| floor(c, r));
        if camera.faces(bottom[0], DVec3::NEG_Z) {
            quads.push((bottom, [shade(lit(DVec3::NEG_Z, light)); 4]));
        }
    } else if camera.faces(floor(0, 0), DVec3::NEG_Z) {
        quads.extend(underside(m, &floor, camera, orbit, shade, light));
    }
    quads
}

/// The underside's quads cut from below, seen from below (it faces the viewer), lit as
/// they face and far to near.
fn underside(
    m: &Material,
    floor: &dyn Fn(usize, usize) -> DVec3,
    camera: &Camera,
    orbit: Orbit,
    shade: &dyn Fn(f64) -> Color32,
    light: DVec3,
) -> Vec<([DVec3; 4], [Color32; 4])> {
    let mut quads = vec![];
    for row in 1..m.rows {
        for column in 1..m.columns {
            // wound the other way round from the top, as seen from below
            let corners = [
                (column - 1, row - 1),
                (column - 1, row),
                (column, row),
                (column, row - 1),
            ]
            .map(|(c, r)| floor(c, r));
            if corners.iter().any(|p| p.z.is_nan()) {
                continue;
            }
            let normal = (corners[1] - corners[0]).cross(corners[3] - corners[0]);
            let normal = if normal.z > 0.0 { -normal } else { normal };
            if !camera.faces(corners[0], normal) {
                continue;
            }
            quads.push((corners, [shade(lit(normal, light)); 4]));
        }
    }
    // far first
    quads.sort_by(|a, b| {
        let depth = |q: &[DVec3; 4]| q.iter().map(|&c| orbit.project(c).1).sum::<f64>();
        depth(&a.0).total_cmp(&depth(&b.0))
    });
    quads
}

/// The model's body under its surface, so it shows solid rather than as a skin: walls
/// hanging from the edge of the surface the grid draws (its quads between four cells that
/// have a model) down to the modeling plane (z = 0), and a bottom there; wrapped round the
/// axis (`wrap`), the walls go down to the axis and need no bottom. Triangles in the
/// surface's coordinates, wound to face out.
#[expect(clippy::cast_precision_loss, reason = "cell counts")]
pub fn solid(material: &Material, wrap: Option<Wrap>) -> Vec<cmd_orientate_3d_model::Seen> {
    let (cols, rows, cell) = (material.columns, material.rows, material.cell);
    if cols < 2 || rows < 2 {
        return vec![];
    }
    let height = |col: usize, row: usize| material.heights[row * cols + col];
    // the quads the surface draws, round again past the end the way the job wraps
    let (wraps_c, wraps_r) = (
        wrap.is_some_and(|w| !w.along_x),
        wrap.is_some_and(|w| w.along_x),
    );
    let quads = |n: usize, wraps: bool| if wraps { n } else { n - 1 };
    let quad = |col: usize, row: usize| {
        col < quads(cols, wraps_c)
            && row < quads(rows, wraps_r)
            && [(0, 0), (1, 0), (1, 1), (0, 1)]
                .iter()
                .all(|&(dc, dr)| !height((col + dc) % cols, (row + dr) % rows).is_nan())
    };
    // the quad before, if there is one
    let before = |i: usize, n: usize, wraps: bool| match i {
        0 => wraps.then(|| n - 1),
        i => Some(i - 1),
    };
    let floor = wrap.map_or(0.0, |w| w.surface - w.radius);
    let round = |along_x: bool| if along_x { rows } else { cols } as f64 * cell;
    let flat = |col: usize, row: usize, z: f64| {
        DVec3::new((col as f64 + 0.5) * cell, (row as f64 + 0.5) * cell, z)
    };
    let at = |col: usize, row: usize, z: f64| {
        let p = flat(col, row, z);
        wrap.map_or(p, |w| w.place(p, round(w.along_x)))
    };
    let mut out = vec![];
    // a wall below the edge between two cells, facing `outward` before wrapping
    let mut wall = |from: (usize, usize), to: (usize, usize), outward: DVec3| {
        let along = flat(to.0, to.1, 0.0) - flat(from.0, from.1, 0.0);
        let (from, to) = if along.cross(DVec3::Z).dot(outward) < 0.0 {
            (to, from)
        } else {
            (from, to)
        };
        let top = (
            at(from.0, from.1, height(from.0, from.1)),
            at(to.0, to.1, height(to.0, to.1)),
        );
        let low = (at(from.0, from.1, floor), at(to.0, to.1, floor));
        out.push(([low.0, low.1, top.1], false));
        out.push(([low.0, top.1, top.0], false));
    };
    for row in 0..rows {
        for col in 0..cols {
            let (right, up) = ((col + 1) % cols, (row + 1) % rows);
            let here = quad(col, row);
            // the edge along the bottom of this quad, drawn where only one side has a quad
            let below = before(row, rows, wraps_r).is_some_and(|r| quad(col, r));
            match (here, below) {
                (true, false) => wall((col, row), (right, row), DVec3::NEG_Y),
                (false, true) => wall((col, row), (right, row), DVec3::Y),
                _ => {}
            }
            // the edge along its left side
            let left = before(col, cols, wraps_c).is_some_and(|c| quad(c, row));
            match (here, left) {
                (true, false) => wall((col, row), (col, up), DVec3::NEG_X),
                (false, true) => wall((col, row), (col, up), DVec3::X),
                _ => {}
            }
        }
    }
    if wrap.is_none() {
        // the bottom, a strip for each run of quads along a row, facing down
        for row in 0..rows - 1 {
            let mut col = 0;
            while col < cols - 1 {
                if !quad(col, row) {
                    col += 1;
                    continue;
                }
                let first = col;
                while col < cols - 1 && quad(col, row) {
                    col += 1;
                }
                let corners = [
                    flat(first, row, floor),
                    flat(first, row + 1, floor),
                    flat(col, row + 1, floor),
                    flat(col, row, floor),
                ];
                out.push(([corners[0], corners[1], corners[2]], false));
                out.push(([corners[0], corners[2], corners[3]], false));
            }
        }
    }
    out
}

/// Triangles in the material's coordinates (as [`screen_of`] takes them), lit as the
/// surface is and drawn far to near with their backs left out; the `true` ones in `mark`.
// ponytail: sorted every frame; cache by view as fine_mesh does if big models drag
pub fn triangles(
    tris: &[cmd_orientate_3d_model::Seen],
    screen: impl Fn(DVec3) -> Pos2,
    orbit: Orbit,
    look: &Look,
    mark: Color32,
) -> Mesh {
    let towards = orbit.towards_viewer();
    let mut seen: Vec<(f64, &[DVec3; 3], Color32, DVec3)> = tris
        .iter()
        .filter_map(|(t, marked)| {
            let normal = (t[1] - t[0]).cross(t[2] - t[0]).normalize_or_zero();
            let depth = t.iter().map(|&c| orbit.project(c).1).sum::<f64>();
            let colour = if *marked { mark } else { look.colour };
            (normal.dot(towards) > 0.0).then_some((depth, t, colour, normal))
        })
        .collect();
    seen.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut mesh = Mesh::default();
    for (_, t, colour, normal) in seen {
        let shade = shaded(colour, lit(normal, look.light));
        let first = u32::try_from(mesh.vertices.len()).expect("mesh fits u32 indices");
        for &corner in t {
            mesh.colored_vertex(screen(corner), shade);
        }
        mesh.add_triangle(first, first + 1, first + 2);
    }
    mesh
}

/// The lines along a rotary stock's wireframe, and the sides of each end's ring.
const CYLINDER_SIDES: usize = 24;

/// A wireframe of the cylinder of `radius` round the axis (y = z = 0 along x, else x = z = 0
/// along y) from `from` to `to` along it: a ring at each end and lines along between.
#[expect(clippy::cast_precision_loss, reason = "a handful of sides")]
pub fn cylinder_edges(along_x: bool, radius: f64, from: f64, to: f64) -> Vec<[DVec3; 2]> {
    let at = |side: usize, along: f64| {
        let angle = std::f64::consts::TAU * side as f64 / CYLINDER_SIDES as f64;
        let (across, up) = (radius * angle.sin(), radius * angle.cos());
        if along_x {
            DVec3::new(along, across, up)
        } else {
            DVec3::new(across, along, up)
        }
    };
    (0..CYLINDER_SIDES)
        .flat_map(|i| {
            [
                [at(i, from), at(i + 1, from)],
                [at(i, to), at(i + 1, to)],
                [at(i, from), at(i, to)],
            ]
        })
        .collect()
}

/// The twelve edges of the box from `lo` to `hi`.
pub fn box_edges(lo: DVec3, hi: DVec3) -> Vec<[DVec3; 2]> {
    let corner = |i: usize| {
        DVec3::new(
            if i & 1 == 1 { hi.x } else { lo.x },
            if i & 2 == 2 { hi.y } else { lo.y },
            if i & 4 == 4 { hi.z } else { lo.z },
        )
    };
    // corners one bit apart share an edge
    (0..8)
        .flat_map(|i| [1, 2, 4].map(move |bit| (i, i | bit)))
        .filter(|(i, j)| i != j)
        .map(|(i, j)| [corner(i), corner(j)])
        .collect()
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

/// The view cube in a corner of the 3D view: a cube turned with the view whose faces, edges
/// and corners each turn the view to look from there (Aspire's View Controller).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewCube {
    /// Where its middle shows.
    pub centre: Pos2,
    /// Half its edge, points.
    pub half: f32,
}

/// The cube's faces: the way out of each, the two ways across it, and its name.
const FACES: [(DVec3, DVec3, DVec3, &str); 6] = [
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
    fn tiles(self, orbit: Orbit) -> Vec<([Pos2; 4], DVec3)> {
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
    }
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

    #[test]
    fn a_lithophane_is_white_where_nothing_is_left_and_the_material_where_it_is_whole() {
        use cut_craft_core::document::Job;
        let job = Job {
            thickness: 10.0,
            ..Job::default()
        };
        let mut m = cut_craft_core::simulation::Material::new(&job, 4);
        m.heights[0] = 0.0;
        m.heights[1] = m.bottom;
        m.heights[2] = m.bottom / 2.0;
        let colour = Color32::from_rgb(200, 100, 0);
        let lit = lit_through(m.clone(), colour, 0.5);
        assert_eq!(paint_of(&lit, 0), Some(colour));
        assert_eq!(paint_of(&lit, 1), Some(Color32::WHITE));
        let mid = paint_of(&lit, 2).unwrap();
        assert!(
            mid.g() > 100 && mid.b() > 0 && mid.b() < 255,
            "between: {mid:?}"
        );
        // more light: the half-thick cell is lighter still
        let lighter = paint_of(&lit_through(m, colour, 1.0), 2).unwrap();
        assert!(lighter.b() > mid.b(), "{lighter:?} vs {mid:?}");
    }

    #[test]
    fn a_preview_is_painted_by_who_cut_each_cell_and_coarse_keeps_the_deepest_cells_paint() {
        let mut m = Material::new(
            &cut_craft_core::document::Job {
                width: 4.0,
                height: 4.0,
                ..cut_craft_core::document::Job::default()
            },
            4,
        );
        // cell 0 cut deepest by toolpath 3, cell 1 by toolpath 9 (unpainted)
        (m.heights[0], m.cut_by[0]) = (-2.0, 3);
        (m.heights[1], m.cut_by[1]) = (-1.0, 9);
        let red = Color32::from_rgb(255, 0, 0);
        let p = painted(m.clone(), |id| (id == 3).then_some(red));
        assert_eq!(paint_of(&p, 0), Some(red));
        assert_eq!(paint_of(&p, 1), None, "no fill");
        assert_eq!(paint_of(&p, 5), None, "uncut");
        assert!(
            painted(m.clone(), |_| None).paint.is_empty(),
            "nothing to paint"
        );
        let c = coarse(&p, 2);
        assert_eq!(
            (c.cut_by[0], paint_of(&c, 0)),
            (3, Some(red)),
            "the deepest cell's"
        );
    }
    use cut_craft_core::document::Job;
    use cut_craft_core::geometry::Point;
    use eframe::egui::pos2;

    fn plain() -> Look {
        Look {
            colour: Color32::WHITE,
            sides: true,
            light: LIGHT,
            wrap: None,
            grain: None,
        }
    }

    #[test]
    fn grain_darkens_lines_along_x_that_shift_going_down_into_the_board() {
        let g = Grain {
            spacing: 4.0,
            depth: 0.3,
        };
        let on = g.at(DVec3::ZERO);
        assert!((on - 0.7).abs() < 1e-6, "a line is `depth` darker: {on}");
        let between = (1..40)
            .map(|i| g.at(DVec3::new(0.0, f64::from(i) * 0.1, 0.0)))
            .fold(0.0, f64::max);
        assert!(between > 0.99, "the wood between lines is not: {between}");
        let deeper = g.at(DVec3::new(0.0, 0.0, -2.0 / GRAIN_DEPTH_SHIFT));
        assert!(
            deeper > on + 0.2,
            "going down, the line moves off: {deeper}"
        );
        let coarse = g.on_cells(g.spacing).depth;
        assert!(
            coarse < g.depth / 2.0,
            "fades on cells too coarse for it: {coarse}"
        );
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
            roll: 0.0,
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
    #[expect(clippy::cast_possible_truncation, reason = "small test coordinates")]
    fn the_tool_is_an_upright_cylinder_on_its_tip() {
        // looking straight down: a circle; from the side: a rectangle as tall as the tool
        let down = |p: DVec3| Pos2::new(p.x as f32, -p.y as f32);
        let ring = cylinder(down, DVec3::ZERO, 3.0, 12.0);
        assert!(
            ring.len() >= 12
                && ring
                    .iter()
                    .all(|p| (p.to_vec2().length() - 3.0).abs() < 1e-3)
        );
        let side = |p: DVec3| Pos2::new(p.x as f32, -p.z as f32);
        let outline = cylinder(side, DVec3::ZERO, 3.0, 12.0);
        let (lo, hi) = outline.iter().fold((f32::MAX, f32::MIN), |(lo, hi), p| {
            (lo.min(p.y), hi.max(p.y))
        });
        assert!((hi - lo - 12.0).abs() < 1e-3, "{lo}..{hi}");
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

    #[test]
    fn the_model_is_solid_down_to_the_modeling_plane() {
        // 10 x 10 cells of 1 mm, the model a 4 x 4 block of them 5 high, nothing round it
        let job = Job {
            width: 10.0,
            height: 10.0,
            ..Job::default()
        };
        let mut m = Material::new(&job, 10);
        for (i, h) in m.heights.iter_mut().enumerate() {
            let (c, r) = (i % 10, i / 10);
            *h = if (3..7).contains(&c) && (3..7).contains(&r) {
                5.0
            } else {
                f64::NAN
            };
        }
        let tris = solid(&m, None);
        // its surface spans 3 x 3 quads: four walls of 3 quads, 2 triangles each, and the
        // bottom as 3 strips of 2
        assert_eq!(tris.len(), 4 * 3 * 2 + 3 * 2, "{}", tris.len());
        let middle = DVec3::new(5.0, 5.0, 2.5);
        for (t, _) in &tris {
            let normal = (t[1] - t[0]).cross(t[2] - t[0]);
            let centre = (t[0] + t[1] + t[2]) / 3.0;
            assert!(normal.dot(centre - middle) > 0.0, "faces out: {t:?}");
            assert!(
                t.iter().all(|p| (0.0..=5.0).contains(&p.z)),
                "plane to surface"
            );
        }
        assert!(
            tris.iter()
                .any(|(t, _)| t.iter().all(|p| p.z.abs() < 1e-12)),
            "a bottom"
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
    fn a_screen_point_goes_back_to_the_top_plane_under_it() {
        let m = slotted();
        let area = Rect::from_min_size(pos2(10.0, 20.0), Vec2::new(400.0, 300.0));
        for perspective in [false, true] {
            let framing = Framing {
                zoom: 1.5,
                pan: Vec2::new(7.0, -3.0),
                perspective,
            };
            let camera = Camera::new(&m, Orbit::default(), framing, area, None);
            let p = DVec3::new(30.0, 70.0, 0.0);
            let back = camera.on_plane(camera.screen(p), 0.0).unwrap();
            // screen positions are f32
            assert!(
                (back - p.truncate()).length() < 1e-3,
                "perspective {perspective}: {back}"
            );
        }
        let edge_on = Camera::new(&m, Orbit::FRONT, Framing::default(), area, None);
        assert_eq!(
            edge_on.on_plane(area.center(), 0.0),
            None,
            "the plane seen edge-on"
        );
        // nearly side on in perspective, the top of the view looks over the horizon
        let low = Orbit {
            pitch: 80f64.to_radians(),
            ..Orbit::FRONT
        };
        let framing = Framing {
            perspective: true,
            ..Framing::default()
        };
        let camera = Camera::new(&m, low, framing, area, None);
        assert_eq!(
            camera.on_plane(area.center_top(), 0.0),
            None,
            "above the horizon, not a point behind the eye"
        );
        assert!(camera.on_plane(area.center_bottom(), 0.0).is_some());
        // a raised plane: the point drawn at that height goes back to it
        let camera = Camera::new(&m, Orbit::default(), framing, area, None);
        let p = DVec3::new(30.0, 70.0, 6.0);
        let back = camera.on_plane(camera.screen(p), 6.0).unwrap();
        assert!((back - p.truncate()).length() < 1e-3, "{back}");
    }

    #[test]
    fn a_screen_point_goes_to_the_surface_it_shows_not_the_plane_behind_it() {
        let m = slotted();
        let area = Rect::from_min_size(pos2(10.0, 20.0), Vec2::new(400.0, 300.0));
        for perspective in [false, true] {
            let framing = Framing {
                zoom: 1.5,
                pan: Vec2::new(7.0, -3.0),
                perspective,
            };
            let camera = Camera::new(&m, Orbit::default(), framing, area, None);
            // the slot's floor, 5 mm down, seen at a slant: the top plane under the same
            // pixel is millimetres off it
            for p in [
                DVec3::new(50.0, 50.0, -5.0),
                DVec3::new(20.0, 30.0, 0.0),
                DVec3::new(75.0, 10.0, 0.0),
            ] {
                let at = camera.screen(p);
                let back = camera.on_surface(&m, at).unwrap();
                assert!(
                    (back - p.truncate()).length() < 0.01,
                    "perspective {perspective}: {p} came back as {back}"
                );
                if p.z < 0.0 {
                    let plane = camera.on_plane(at, 0.0).unwrap();
                    assert!((plane - p.truncate()).length() > 1.0, "{plane}");
                }
            }
            // off the material, the top plane carries on
            let off = DVec3::new(130.0, 50.0, 0.0);
            let back = camera.on_surface(&m, camera.screen(off)).unwrap();
            assert!((back - off.truncate()).length() < 0.01, "{back}");
        }
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
    #[ignore = "timing only: cargo test --release -- --ignored mesh_timing --nocapture"]
    fn mesh_timing() {
        use cut_craft_core::component::{Combine, Component};
        let mut doc = cut_craft_core::document::Document::default();
        let o = doc.job.area().min;
        let rect = cut_craft_core::geometry::Rect::new(
            o + cut_craft_core::geometry::Point::new(150.0, 100.0),
            o + cut_craft_core::geometry::Point::new(450.0, 250.0),
        );
        let mut c = cut_craft_core::Change::builder(&doc);
        c.add_component(Component::from_fn(
            "m",
            Combine::Add,
            rect,
            doc.job.model_cell(),
            |p| Some((p.x * 0.3).sin() + (p.y * 0.2).cos() + 3.0),
        ));
        doc.apply(&c.finish()).unwrap();
        let area = Rect::from_min_size(Pos2::ZERO, eframe::egui::vec2(1400.0, 900.0));
        let look = Look {
            colour: Color32::GRAY,
            light: DVec3::new(-1.0, 1.0, 1.0),
            sides: true,
            wrap: None,
            grain: None,
        };
        for cells in [250, 500, 1000, 1500] {
            let t = std::time::Instant::now();
            let m = cut_craft_core::component::model_shape(&doc, cells);
            let built = t.elapsed();
            let t = std::time::Instant::now();
            let mesh = mesh(&m, Orbit::default(), Framing::default(), &look, area);
            eprintln!(
                "{cells} cells: model {built:?}, mesh {:?}, {} triangles",
                t.elapsed(),
                mesh.indices.len() / 3
            );
        }
    }

    #[test]
    fn coarse_cells_of_taken_away_material_stay_empty() {
        let mut m = slotted();
        m.heights
            .iter_mut()
            .take(m.columns * 8)
            .for_each(|h| *h = f64::NAN);
        let c = coarse(&m, 30);
        assert!(c.heights[0].is_nan(), "a block all gone is gone");
        assert!(c.heights[c.columns].is_nan(), "rows 0..8 are 2 coarse rows");
        assert!(c.heights[2 * c.columns].is_finite(), "the rest stays");
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
            roll: 0.0,
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
            roll: 0.0,
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
            roll: 0.0,
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
    fn seen_from_below_a_cut_underside_is_drawn_cell_by_cell() {
        let area = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 300.0));
        let below = Orbit::facing(DVec3::NEG_Z);
        let flat = mesh(&slotted(), below, Framing::default(), &plain(), area);
        let mut cut = slotted();
        cut.under = vec![cut.bottom; cut.heights.len()];
        cut.under[0] = cut.bottom + 2.0;
        let shown = mesh(&cut, below, Framing::default(), &plain(), area);
        assert!(
            shown.indices.len() > flat.indices.len() * 3 / 2,
            "the underside's own grid: {} {}",
            shown.indices.len(),
            flat.indices.len()
        );
        let above = mesh(&cut, Orbit::TOP, Framing::default(), &plain(), area);
        let plain_above = mesh(&slotted(), Orbit::TOP, Framing::default(), &plain(), area);
        assert_eq!(
            above.indices.len(),
            plain_above.indices.len(),
            "hidden from above"
        );
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

    /// Whether `a` and `b` put job points in the same places on screen.
    fn same_view(a: Orbit, b: Orbit) -> bool {
        [DVec3::X, DVec3::Y, DVec3::Z, DVec3::new(1.0, 2.0, 3.0)]
            .into_iter()
            .all(|p| {
                let (pa, da) = a.project(p);
                let (pb, db) = b.project(p);
                (pa - pb).length() < 1e-9 && (da - db).abs() < 1e-9
            })
    }

    #[test]
    fn any_way_of_facing_survives_its_matrix() {
        for (yaw, pitch, roll) in [
            (0.3, 1.1, 0.0),
            (-2.0, 2.5, 0.7),
            (1.0, 0.0, 0.4),
            (0.0, 3.0, -2.9),
        ] {
            let o = Orbit { yaw, pitch, roll };
            let back = Orbit::from_matrix(o.matrix());
            assert!(same_view(o, back), "{o:?} came back as {back:?}");
            // and to_world undoes project
            let p = DVec3::new(0.4, -1.3, 2.2);
            let (on, depth) = o.project(p);
            assert!((o.to_world(DVec3::new(on.x, on.y, depth)) - p).length() < 1e-9);
        }
    }

    #[test]
    fn free_rotation_turns_as_the_turntable_does_side_on_and_goes_past_the_top() {
        let size = Vec2::new(400.0, 300.0);
        for by in [Vec2::new(30.0, 0.0), Vec2::new(0.0, 20.0)] {
            let turntable = Orbit::FRONT.dragged_by(by, size, Rotation::Turntable);
            let free = Orbit::FRONT.dragged_by(by, size, Rotation::Free);
            assert!(
                same_view(turntable, free),
                "{by:?}: {turntable:?} against {free:?}"
            );
        }
        // up past straight down: the turntable stops, free rotation carries on over
        let over = Vec2::new(0.0, -60.0);
        let stopped = Orbit::TOP.dragged_by(over, size, Rotation::Turntable);
        let carried = Orbit::TOP.dragged_by(over, size, Rotation::Free);
        assert!(
            same_view(stopped, Orbit::TOP),
            "the turntable stops at the top"
        );
        assert!(!same_view(carried, Orbit::TOP), "free rotation does not");
        assert!(carried.towards_viewer().z > 0.0 && carried.towards_viewer().z < 1.0);
    }

    #[test]
    fn dragging_turns_and_tilts_from_straight_down_past_side_on_to_straight_up() {
        let size = Vec2::new(400.0, 300.0);
        let o = Orbit::TOP.dragged_by(Vec2::new(200.0, -300.0), size, Rotation::Turntable);
        assert!((o.yaw - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
        assert_eq!(o.pitch, 0.0, "cannot tilt past straight down");
        let under = Orbit::FRONT.dragged_by(Vec2::new(0.0, 50.0), size, Rotation::Turntable);
        assert!(under.pitch > std::f64::consts::FRAC_PI_2, "past side on");
        assert!(under.towards_viewer().z < 0.0, "from underneath");
        assert_eq!(
            Orbit::TOP
                .dragged_by(Vec2::new(0.0, 900.0), size, Rotation::Turntable)
                .pitch,
            std::f64::consts::PI
        );
    }

    #[test]
    fn facing_looks_from_each_face_edge_and_corner() {
        assert_eq!(Orbit::facing(DVec3::Z), Orbit::TOP);
        let same = |a: Orbit, b: Orbit| a.towards_viewer().abs_diff_eq(b.towards_viewer(), 1e-12);
        assert!(same(Orbit::facing(DVec3::NEG_Y), Orbit::FRONT));
        assert!(same(Orbit::facing(DVec3::X * 3.0), Orbit::RIGHT));
        for from in [
            DVec3::NEG_Z,
            DVec3::new(1.0, -1.0, 0.0),
            DVec3::new(-1.0, 1.0, 1.0),
            DVec3::new(1.0, 1.0, -1.0),
        ] {
            let towards = Orbit::facing(from).towards_viewer();
            assert!(
                towards.abs_diff_eq(from.normalize(), 1e-12),
                "{from}: {towards}"
            );
        }
        let below = Orbit::facing(DVec3::NEG_Z);
        let (x, _) = below.project(DVec3::X);
        assert!(x.abs_diff_eq(DVec2::X, 1e-12), "x still to the right");
    }

    #[test]
    fn in_perspective_nearer_is_larger_and_the_block_still_fits() {
        let area = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 300.0));
        let m = slotted();
        let ortho = Framing::default();
        let perspective = Framing {
            perspective: true,
            ..ortho
        };
        // from the front, the front (y 0) and back (y 100) edges' lengths
        let edges = |framing: Framing| {
            let screen = screen_of(&m, Orbit::FRONT, framing, area, None);
            let across = |y: f64| {
                screen(DVec3::new(0.0, y, 0.0)).distance(screen(DVec3::new(100.0, y, 0.0)))
            };
            (across(0.0), across(100.0))
        };
        let (front, back) = edges(ortho);
        assert!((front - back).abs() < 1e-3, "orthographic: the same");
        let (front, back) = edges(perspective);
        assert!(front > back * 1.1, "perspective: {front} against {back}");
        for orbit in [Orbit::TOP, Orbit::default(), Orbit::FRONT] {
            let drawn = mesh(&m, orbit, perspective, &plain(), area);
            assert!(
                drawn
                    .vertices
                    .iter()
                    .all(|v| area.expand(1e-3).contains(v.pos)),
                "{orbit:?}"
            );
        }
    }

    #[test]
    fn from_below_the_block_shows_its_bottom_last() {
        let area = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 300.0));
        let m = slotted();
        let count = |orbit| {
            mesh(&m, orbit, Framing::default(), &plain(), area)
                .vertices
                .len()
        };
        let below = Orbit::facing(DVec3::new(0.3, -1.0, -0.5));
        let above = Orbit::facing(DVec3::new(0.3, -1.0, 0.5));
        assert_eq!(count(below), count(above) + 4, "one quad more: the bottom");
        let drawn = mesh(&m, below, Framing::default(), &plain(), area);
        let bottom = Rgba::from(drawn.vertices.last().unwrap().color);
        let lit_bottom = lit(DVec3::NEG_Z, LIGHT);
        assert!(
            (f64::from(bottom.r()) - lit_bottom).abs() < 0.01,
            "drawn last, over the surface"
        );
    }

    /// 100 x 100 mm in 1 mm cells, its top at 0, with a 20 mm high ridge from x = 40 to 60.
    fn ridged() -> Material {
        let job = Job {
            width: 100.0,
            height: 100.0,
            ..Job::default()
        };
        let mut m = Material::new(&job, 100);
        for row in 0..m.rows {
            for column in 40..60 {
                m.heights[row * m.columns + column] = 20.0;
            }
        }
        m
    }

    #[test]
    fn the_surface_hides_what_is_behind_it_and_nothing_else() {
        let m = ridged();
        let seen = |orbit: Orbit, framing: Framing, p: DVec3| seen_of(&m, orbit, framing)(p);
        let (ortho, perspective) = (
            Framing::default(),
            Framing {
                perspective: true,
                ..Framing::default()
            },
        );
        let left = DVec3::new(20.0, 50.0, 0.0);
        let ridge_top = DVec3::new(50.0, 50.0, 20.0);
        let (left_view, above_right) = (
            Orbit::facing(DVec3::NEG_X),
            Orbit::facing(DVec3::new(1.0, 0.0, 0.3)),
        );
        for framing in [ortho, perspective] {
            assert!(!seen(Orbit::RIGHT, framing, left), "behind the ridge");
            assert!(seen(Orbit::TOP, framing, left), "from above");
            assert!(seen(left_view, framing, left), "on the near side");
            assert!(seen(above_right, framing, ridge_top), "on top of it");
        }
        // from low on the right the ridge hides the near part of the floor behind it
        let low = Orbit::facing(DVec3::new(1.0, 0.0, 0.5));
        assert!(!seen(low, ortho, DVec3::new(35.0, 50.0, 0.0)));
        assert!(
            seen(low, ortho, DVec3::new(-30.0, 50.0, 60.0)),
            "high off it"
        );
        let below = Orbit::facing(DVec3::NEG_Z);
        assert!(!seen(below, ortho, left), "the block is under the top");
        assert!(
            seen(below, ortho, DVec3::new(-5.0, 50.0, 0.0)),
            "off the block"
        );
    }

    #[test]
    fn a_line_is_cut_where_it_is_hidden() {
        // a line across the ridge's far side, seen from the right: the part behind it goes
        let m = ridged();
        let seen = seen_of(&m, Orbit::RIGHT, Framing::default());
        let line = [DVec3::new(10.0, 20.0, 0.0), DVec3::new(10.0, 80.0, 0.0)];
        assert!(visible_runs(&line, m.cell, &seen).is_empty(), "all behind");
        let across = [DVec3::new(10.0, 50.0, 0.0), DVec3::new(90.0, 50.0, 0.0)];
        let runs = visible_runs(&across, m.cell, &seen);
        assert_eq!(runs.len(), 1, "{runs:?}");
        assert!(
            runs[0].iter().all(|p| p.x >= 59.0),
            "only right of the ridge"
        );
        let all = |_| true;
        assert_eq!(visible_runs(&across, 1.0, all)[0].len(), 81, "a point a mm");
        let gap = |p: DVec3| !(40.0..60.0).contains(&p.x);
        assert_eq!(visible_runs(&across, 1.0, gap).len(), 2, "split in two");
    }

    #[test]
    fn the_view_cube_turns_to_what_is_under_the_pointer() {
        let cube = ViewCube {
            centre: pos2(100.0, 100.0),
            half: 30.0,
        };
        let iso = Orbit::default();
        // the middles of the faces in sight, and a corner and an edge of the top
        for from in [DVec3::Z, DVec3::NEG_Y, DVec3::X] {
            let at = cube.screen(iso, from);
            assert_eq!(cube.direction_at(iso, at), Some(from), "{from}");
        }
        let corner = cube.screen(iso, DVec3::new(0.8, -0.8, 1.0));
        let towards = cube.direction_at(iso, corner).unwrap();
        assert!(towards.abs_diff_eq(DVec3::new(1.0, -1.0, 1.0).normalize(), 1e-12));
        let edge = cube.screen(iso, DVec3::new(0.0, 0.8, 1.0));
        let towards = cube.direction_at(iso, edge).unwrap();
        assert!(towards.abs_diff_eq(DVec3::new(0.0, 1.0, 1.0).normalize(), 1e-12));
        assert_eq!(cube.direction_at(iso, pos2(0.0, 0.0)), None, "off it");
        assert!(cube.rect().contains(cube.screen(iso, DVec3::ONE)));
        // face on, the faces seen edge-on take no clicks
        assert_eq!(
            cube.direction_at(Orbit::RIGHT, cube.screen(Orbit::RIGHT, DVec3::X)),
            Some(DVec3::X)
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
            grain: None,
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
