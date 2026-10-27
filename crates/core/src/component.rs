//! 3D components: height-map shapes that combine, in the order they were made, into the model.

use crate::CommandError;
use crate::document::{Document, EntityId};
use crate::geometry::{Point, Rect};
use crate::params::Xy;
use crate::simulation::Material;
use crate::toolpath::Tool;
use glam::DAffine2;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// How a component's heights combine with the model under it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Combine {
    /// Raised on top of what is under it
    #[default]
    Add,
    /// Sunk into what is under it
    Subtract,
    /// The higher of the two
    Merge,
    /// The lower of the two
    Low,
    /// The two multiplied: a 0 to 1 component fades what is under it
    Multiply,
}

impl Combine {
    pub fn apply(self, under: f64, height: f64) -> f64 {
        match self {
            Self::Add => under + height,
            Self::Subtract => under - height,
            Self::Merge => under.max(height),
            Self::Low => under.min(height),
            Self::Multiply => under * height,
        }
    }
}

/// Whether two height grids match cell by cell, a hole (NaN) matching a hole.
#[expect(
    clippy::float_cmp,
    reason = "the same height exactly, as a Vec<Option<f64>> compared"
)]
pub fn same_heights(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(x, y)| x == y || (x.is_nan() && y.is_nan()))
}

/// A cell's height, or `fallback` where it has no shape.
pub fn height_or(h: f64, fallback: f64) -> f64 {
    if h.is_nan() { fallback } else { h }
}

/// Heights over a grid of square cells in the job.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Component {
    pub name: String,
    pub combine: Combine,
    /// Lower-left corner of the grid, job mm.
    pub origin: Point,
    pub cell: f64,
    pub columns: usize,
    pub rows: usize,
    /// Row by row from the bottom; NaN where the component has no shape (half the memory of
    /// an `Option<f64>`, and what [`Material`] uses too).
    #[serde(with = "crate::packed::heights")]
    pub heights: Vec<f64>,
    /// Hidden components are left out of the model (added after the first job format).
    #[serde(default = "shown")]
    pub visible: bool,
    /// Shape and base height, fade and tilt: applied as heights are read, until baked.
    #[serde(default)]
    pub shaping: Shaping,
    /// The heights before it was first sculpted, which the Undo Brush brings back; cleared
    /// when the grid changes.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::packed::heights::option"
    )]
    pub unsculpted: Option<Vec<f64>>,
    /// The level it is on (added with levels; older files put everything on the first).
    #[serde(default)]
    pub level: u64,
    /// The component group it is in, if any (added with groups).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<Group>,
}

impl PartialEq for Component {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.combine == other.combine
            && self.origin == other.origin
            && self.cell == other.cell
            && self.columns == other.columns
            && self.rows == other.rows
            && same_heights(&self.heights, &other.heights)
            && self.visible == other.visible
            && self.shaping == other.shaping
            && match (&self.unsculpted, &other.unsculpted) {
                (None, None) => true,
                (Some(a), Some(b)) => same_heights(a, b),
                _ => false,
            }
            && self.level == other.level
            && self.group == other.group
    }
}

/// A component group: its members combine with each other first, then the result combines
/// with what is under it, where its top member is in the tree. Each member carries it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Group {
    pub id: u64,
    pub name: String,
    pub combine: Combine,
}

/// A component's dynamic properties (Aspire's Component Properties): they change how its
/// heights enter the model without changing the heights, until the component is baked.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Shaping {
    /// Height as a percentage of the component's own
    #[schemars(title = "Shape Height", extend("x-unit" = "%"))]
    pub shape_height: f64,
    /// Raised on a flat plane this thick
    #[schemars(title = "Base Height", extend("x-unit" = "mm"))]
    pub base_height: f64,
    /// Fades the heights down from one point toward another
    pub fade: Option<Fade>,
    /// Tilts the component up toward a point
    pub tilt: Option<Tilt>,
}

impl Default for Shaping {
    fn default() -> Self {
        Self {
            shape_height: 100.0,
            base_height: 0.0,
            fade: None,
            tilt: None,
        }
    }
}

/// Heights kept at `from` and lowered by `percent` at `to`, linearly between.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Fade {
    pub from: Xy,
    pub to: Xy,
    #[schemars(extend("x-unit" = "%"))]
    pub percent: f64,
}

/// The component tilted about the line through `pivot` square to `toward`, so that `toward`
/// rises by `angle` degrees.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Tilt {
    pub pivot: Xy,
    pub toward: Xy,
    #[schemars(extend("x-unit" = "deg"))]
    pub angle: f64,
}

impl Shaping {
    /// The same shaping with its anchors carried by `t`.
    #[must_use]
    pub fn transformed(&self, t: &DAffine2) -> Self {
        let carry = |p: Xy| {
            let q = t.transform_point2(p.into());
            Xy::new(q.x, q.y)
        };
        Self {
            fade: self.fade.map(|f| Fade {
                from: carry(f.from),
                to: carry(f.to),
                ..f
            }),
            tilt: self.tilt.map(|k| Tilt {
                pivot: carry(k.pivot),
                toward: carry(k.toward),
                ..k
            }),
            ..self.clone()
        }
    }

    /// Whether it leaves heights as they are.
    pub fn is_identity(&self) -> bool {
        *self == Self::default()
    }

    /// A raw height `z` at `p`, shaped.
    pub fn apply(&self, p: Point, z: f64) -> f64 {
        let along = |from: Xy, to: Xy| {
            let (from, to) = (Point::from(from), Point::from(to));
            let axis = to - from;
            let length = axis.length();
            (length > f64::EPSILON).then(|| ((p - from).dot(axis) / length, length))
        };
        let mut z = z * self.shape_height / 100.0;
        if let Some(f) = self.fade
            && let Some((d, length)) = along(f.from, f.to)
        {
            z *= 1.0 - (d / length).clamp(0.0, 1.0) * f.percent / 100.0;
        }
        z += self.base_height;
        if let Some(t) = self.tilt
            && let Some((d, _)) = along(t.pivot, t.toward)
        {
            // ponytail: a plane through the pivot line, lowering behind it too [inf]
            z += d * t.angle.to_radians().tan();
        }
        z
    }
}

fn shown() -> bool {
    true
}

/// A level of the component tree: its components combine with each other first, then the
/// result combines with the levels below it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Level {
    pub name: String,
    /// How the level combines with the levels below it
    #[schemars(title = "Combine Mode")]
    pub combine: Combine,
    /// Hidden levels are left out of the model
    pub visible: bool,
    /// Raises the whole level on a flat plane this thick
    #[schemars(title = "Base Height", extend("x-unit" = "mm"))]
    pub base_height: f64,
    /// Level Clipping: the level is kept only inside these closed vectors (none: no clipping)
    #[schemars(skip)]
    pub clip: Vec<crate::geometry::Path>,
    /// The vectors the clipping was made from, so Clipping ▸ Update can read them again
    #[schemars(skip)]
    pub clip_vectors: Vec<EntityId>,
    /// Mirrors one half or quarter of the job onto the rest
    #[schemars(title = "Mirror Mode")]
    pub mirror: Mirror,
    /// Rotary jobs: what runs off one side of the unrolled job comes back on the other
    pub wrapping: bool,
}

impl Default for Level {
    fn default() -> Self {
        Self {
            name: "Level 1".into(),
            combine: Combine::Add,
            visible: true,
            base_height: 0.0,
            clip: vec![],
            clip_vectors: vec![],
            mirror: Mirror::Off,
            wrapping: false,
        }
    }
}

/// A level's mirror mode: which part of the job is drawn, the rest being its reflection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Mirror {
    #[default]
    Off,
    #[schemars(title = "Left to Right")]
    LeftToRight,
    #[schemars(title = "Right to Left")]
    RightToLeft,
    #[schemars(title = "Top to Bottom")]
    TopToBottom,
    #[schemars(title = "Bottom to Top")]
    BottomToTop,
    #[schemars(title = "Top Left Quadrant")]
    TopLeft,
    #[schemars(title = "Top Right Quadrant")]
    TopRight,
    #[schemars(title = "Bottom Left Quadrant")]
    BottomLeft,
    #[schemars(title = "Bottom Right Quadrant")]
    BottomRight,
}

impl Mirror {
    /// Where the level's heights at `p` come from, reflected about the middle of `job`.
    pub fn source(self, p: Point, job: Rect) -> Point {
        let c = job.center();
        // (keep the left half, keep the bottom half): Some(true) keeps the low side
        let (x, y) = match self {
            Self::Off => (None, None),
            Self::LeftToRight => (Some(true), None),
            Self::RightToLeft => (Some(false), None),
            Self::TopToBottom => (None, Some(false)),
            Self::BottomToTop => (None, Some(true)),
            Self::TopLeft => (Some(true), Some(false)),
            Self::TopRight => (Some(false), Some(false)),
            Self::BottomLeft => (Some(true), Some(true)),
            Self::BottomRight => (Some(false), Some(true)),
        };
        let reflect = |v: f64, middle: f64, keep_low: Option<bool>| match keep_low {
            Some(true) if v > middle => 2.0 * middle - v,
            Some(false) if v < middle => 2.0 * middle - v,
            _ => v,
        };
        Point::new(reflect(p.x, c.x, x), reflect(p.y, c.y, y))
    }
}

/// The level every component starts on, and the one a job always has.
pub fn default_levels() -> std::collections::BTreeMap<u64, Level> {
    [(0, Level::default())].into()
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "cell counts and indices are small and non-negative"
)]
impl Component {
    /// `height` sampled at the centres of `cell`-sized squares covering `area`.
    pub fn from_fn(
        name: &str,
        combine: Combine,
        area: Rect,
        cell: f64,
        height: impl Fn(Point) -> Option<f64>,
    ) -> Self {
        let count = |length: f64| ((length / cell).ceil() as usize).max(1);
        let (columns, rows) = (count(area.size().x), count(area.size().y));
        let heights = (0..rows)
            .flat_map(|row| (0..columns).map(move |column| (column, row)))
            .map(|(column, row)| {
                height(area.min + Point::new(column as f64 + 0.5, row as f64 + 0.5) * cell)
                    .unwrap_or(f64::NAN)
            })
            .collect();
        Self {
            name: name.into(),
            combine,
            origin: area.min,
            cell,
            columns,
            rows,
            heights,
            visible: true,
            shaping: Shaping::default(),
            unsculpted: None,
            level: 0,
            group: None,
        }
    }

    /// Height of the cell under `p`; `None` off the grid or where there is no shape.
    pub fn height_at(&self, p: Point) -> Option<f64> {
        let local = (p - self.origin) / self.cell;
        if local.x < 0.0 || local.y < 0.0 {
            return None;
        }
        let (column, row) = (local.x as usize, local.y as usize);
        if column < self.columns && row < self.rows {
            self.shaped(row * self.columns + column)
        } else {
            None
        }
    }
}

/// The model at the job's modeling resolution.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a cell count"
)]
pub fn model_at_resolution(doc: &Document) -> Material {
    let longest = doc.job.width.max(doc.job.height);
    model(
        doc,
        ((longest / doc.job.model_cell()).ceil() as usize).max(1),
    )
}

impl Component {
    /// A component holding `material`'s heights (each cell a shape), over the same cells.
    pub fn from_material(name: &str, combine: Combine, material: &Material) -> Self {
        Self {
            name: name.into(),
            combine,
            origin: material.origin,
            cell: material.cell,
            columns: material.columns,
            rows: material.rows,
            heights: material.heights.clone(),
            visible: true,
            shaping: Shaping::default(),
            unsculpted: None,
            level: 0,
            group: None,
        }
    }

    /// The area its grid covers, in job mm.
    #[expect(clippy::cast_precision_loss, reason = "cell counts are small")]
    pub fn extent(&self) -> Rect {
        let size = Point::new(self.columns as f64, self.rows as f64) * self.cell;
        Rect::new(self.origin, self.origin + size)
    }

    /// Cell `index`'s height with the shaping applied; `None` where there is no shape.
    pub fn shaped(&self, index: usize) -> Option<f64> {
        let z = self.heights[index];
        if z.is_nan() {
            return None;
        }
        Some(if self.shaping.is_identity() {
            z
        } else {
            self.shaping.apply(self.centre_of(index), z)
        })
    }

    /// Every cell's height with the shaping applied, row by row from the bottom; NaN where
    /// there is no shape.
    pub fn shaped_heights(&self) -> Vec<f64> {
        (0..self.heights.len())
            .map(|i| self.shaped(i).unwrap_or(f64::NAN))
            .collect()
    }

    /// The same shape with its shaping made permanent (Component Properties' Bake).
    #[must_use]
    pub fn baked(&self) -> Self {
        Self {
            heights: self.shaped_heights(),
            shaping: Shaping::default(),
            ..self.clone()
        }
    }

    /// Lowest and highest of its shaped heights, or `None` when it has no shape.
    pub fn height_range(&self) -> Option<(f64, f64)> {
        self.shaped_heights()
            .into_iter()
            .filter(|h| !h.is_nan())
            .fold(None, |range, h| {
                Some(range.map_or((h, h), |(lo, hi): (f64, f64)| (lo.min(h), hi.max(h))))
            })
    }

    /// The component as a picture, top row first: each shaped cell's height as a grey level
    /// (0 at its lowest, 255 at its highest), `None` where it has no shape.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 0..=255"
    )]
    pub fn shade(&self) -> Vec<Option<u8>> {
        let (lo, hi) = self.height_range().unwrap_or((0.0, 0.0));
        let span = (hi - lo).max(f64::EPSILON);
        let heights = self.shaped_heights();
        (0..self.rows)
            .rev()
            .flat_map(|row| &heights[row * self.columns..(row + 1) * self.columns])
            .map(|&z| {
                (!z.is_nan()).then(|| ((z - lo) / span * 255.0).round().clamp(0.0, 255.0) as u8)
            })
            .collect()
    }

    /// The component carried by `t` in XY (moved, rotated, mirrored or scaled; heights kept),
    /// resampled onto cells of the same size over where it lands; fade and tilt anchors go
    /// with it.
    #[must_use]
    pub fn transformed(&self, t: &DAffine2) -> Self {
        let e = self.extent();
        let corners = [
            e.min,
            Point::new(e.max.x, e.min.y),
            e.max,
            Point::new(e.min.x, e.max.y),
        ]
        .map(|p| t.transform_point2(p));
        let (lo, hi) = corners
            .iter()
            .fold((corners[0], corners[0]), |(lo, hi), &p| {
                (lo.min(p), hi.max(p))
            });
        let back = t.inverse();
        let mut out = Self::from_fn(
            &self.name,
            self.combine,
            Rect::new(lo, hi),
            self.cell,
            |p| self.sample(back.transform_point2(p)),
        );
        out.visible = self.visible;
        out.shaping = self.shaping.transformed(t);
        out.level = self.level;
        out.group.clone_from(&self.group);
        out
    }

    /// The unshaped height at `p`: bilinear between the four cell centres round it where they
    /// all have shape, else the height of the cell `p` is in.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "cell indices are small"
    )]
    fn sample(&self, p: Point) -> Option<f64> {
        let raw = |column: isize, row: isize| {
            let inside = column >= 0
                && row >= 0
                && (column as usize) < self.columns
                && (row as usize) < self.rows;
            inside
                .then(|| self.heights[row as usize * self.columns + column as usize])
                .filter(|h| !h.is_nan())
        };
        let local = (p - self.origin) / self.cell;
        if local.x < 0.0 || local.y < 0.0 {
            return None;
        }
        let own = raw(local.x as isize, local.y as isize)?;
        let grid = local - 0.5;
        let (column, row) = (grid.x.floor() as isize, grid.y.floor() as isize);
        let frac = grid - grid.floor();
        let around = [
            raw(column, row),
            raw(column + 1, row),
            raw(column, row + 1),
            raw(column + 1, row + 1),
        ];
        Some(match around {
            [
                Some(low_left),
                Some(low_right),
                Some(up_left),
                Some(up_right),
            ] => {
                let bottom = low_left + (low_right - low_left) * frac.x;
                let top = up_left + (up_right - up_left) * frac.x;
                bottom + (top - bottom) * frac.y
            }
            _ => own,
        })
    }

    /// Where cell `index` is: its centre in job mm.
    #[expect(clippy::cast_precision_loss, reason = "cell indices are small")]
    pub fn centre_of(&self, index: usize) -> Point {
        let (column, row) = (index % self.columns, index / self.columns);
        self.origin + Point::new(column as f64 + 0.5, row as f64 + 0.5) * self.cell
    }
}

/// Closed paths round where `parts` have shape, traced over their finest cells and fitted
/// with arcs within half a cell.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "cell counts are small and non-negative"
)]
pub fn silhouette(parts: &[&Component]) -> Vec<crate::geometry::Path> {
    use crate::trace::{self, Fit, Grid};
    let Some(area) = parts.iter().map(|c| c.extent()).reduce(Rect::union) else {
        return vec![];
    };
    let cell = parts.iter().map(|c| c.cell).fold(f64::INFINITY, f64::min);
    let grid = Grid {
        columns: (area.size().x / cell).ceil() as usize,
        rows: (area.size().y / cell).ceil() as usize,
        origin: area.min,
        cell,
    };
    let centre = |c: usize, r: usize| area.min + Point::new(c as f64 + 0.5, r as f64 + 0.5) * cell;
    let rings = trace::outlines(grid, |c, r| {
        parts
            .iter()
            .any(|part| part.height_at(centre(c, r)).is_some())
    });
    // ponytail: arcs within half a cell, corners over 60° kept [inf]; Aspire's own smoothing
    // is not described
    rings
        .iter()
        .map(|ring| trace::fit(ring, true, Fit::Arcs, cell / 2.0, 60_f64.to_radians()))
        .collect()
}

/// What a "selected components" tool works on: the selected components (else the newest
/// one), and the single component they make: the one itself, or several baked together over
/// the job. Removing the returned ids and adding the component replaces them.
pub fn chosen_or_baked(doc: &Document) -> Option<(Vec<EntityId>, Component)> {
    let mut ids: Vec<EntityId> = doc
        .selected_components
        .iter()
        .copied()
        .filter(|id| doc.components.contains_key(id))
        .collect();
    if ids.is_empty() {
        ids.extend(doc.chosen_component());
    }
    match ids.as_slice() {
        [] => None,
        [one] => Some((ids.clone(), doc.components[one].baked())),
        _ => {
            let mut only = doc.clone();
            for (id, c) in &mut only.components {
                c.visible = ids.contains(id);
            }
            let baked =
                Component::from_material("Baked", Combine::Add, &model_at_resolution(&only));
            Some((ids, baked))
        }
    }
}

/// A shown level with its clip rings and what it combines, bottom up.
type Placed<'a> = (&'a Level, Vec<Vec<Point>>, Vec<Entry<'a>>);

/// One step of a level's combining: a lone component, or a group's members together.
enum Entry<'a> {
    One(&'a Component),
    Group(Combine, Vec<&'a Component>),
}

/// A level's components (bottom first) as entries, each group where its top member is.
fn entries<'a>(on: &[&'a Component]) -> Vec<Entry<'a>> {
    let group_of = |c: &Component| c.group.as_ref().map(|g| g.id);
    on.iter()
        .enumerate()
        .filter_map(|(i, c)| match &c.group {
            None => Some(Entry::One(c)),
            Some(g) if on[i + 1..].iter().any(|k| group_of(k) == Some(g.id)) => None,
            Some(g) => Some(Entry::Group(
                g.combine,
                on.iter()
                    .copied()
                    .filter(|k| group_of(k) == Some(g.id))
                    .collect(),
            )),
        })
        .collect()
}

/// Components combined in turn at `p`, the first onto 0; `None` where none has shape.
fn combined(on: &[&Component], p: Point) -> Option<f64> {
    on.iter().fold(None, |under: Option<f64>, c| {
        c.height_at(p).map_or(under, |height| {
            Some(c.combine.apply(under.unwrap_or(0.0), height))
        })
    })
}

/// The model's surface over the job, on a flat base at 0: each visible level's visible
/// components combined (bottom of the tree first), clipped, mirrored and wrapped as the level
/// says, then the levels combined from the first up.
pub fn model(doc: &Document, cells_across: usize) -> Material {
    model_over(doc, cells_across, 0.0)
}

/// The model alone, as the 3D view shows it before any preview: cells no shown component
/// covers are NaN (nothing there), not the zero plane.
pub fn model_shape(doc: &Document, cells_across: usize) -> Material {
    model_over(doc, cells_across, f64::NAN)
}

/// [`model`], with `empty` in the cells nothing covers.
fn model_over(doc: &Document, cells_across: usize, empty: f64) -> Material {
    use crate::document::Along;
    let mut surface = Material::new(&doc.job, cells_across);
    let job = doc.job.area();
    let first = doc.levels.keys().next().copied().unwrap_or(0);
    let wrap = doc.job.rotary.map(|r| match r.along {
        Along::X => Point::new(0.0, job.size().y),
        Along::Y => Point::new(job.size().x, 0.0),
    });
    let levels: Vec<Placed> = doc
        .levels
        .iter()
        .filter(|(_, l)| l.visible)
        .map(|(&id, l)| {
            let on: Vec<&Component> = doc
                .components
                .values()
                .filter(|c| c.visible)
                .filter(|c| {
                    let level = if doc.levels.contains_key(&c.level) {
                        c.level
                    } else {
                        first
                    };
                    level == id
                })
                .collect();
            // ponytail: clip rings flattened at 0.01 mm
            let rings = l.clip.iter().map(|p| p.flatten(0.01)).collect();
            (l, rings, entries(&on))
        })
        .filter(|(_, _, on)| !on.is_empty())
        .collect();
    let level_at = |on: &[Entry], p: Point| {
        on.iter()
            .fold(None, |under: Option<f64>, entry| match entry {
                Entry::One(c) => c.height_at(p).map_or(under, |height| {
                    Some(c.combine.apply(under.unwrap_or(0.0), height))
                }),
                Entry::Group(combine, members) => combined(members, p).map_or(under, |height| {
                    Some(combine.apply(under.unwrap_or(0.0), height))
                }),
            })
    };
    for row in 0..surface.rows {
        for column in 0..surface.columns {
            let centre = surface.centre(column, row);
            let (mut total, mut covered) = (0.0, false);
            for (level, rings, on) in &levels {
                if !rings.is_empty() && !crate::geometry::inside(rings, centre) {
                    continue;
                }
                let from = level.mirror.source(centre, job);
                let shifts = match wrap {
                    Some(w) if level.wrapping => vec![Point::ZERO, w, -w],
                    _ => vec![Point::ZERO],
                };
                // where a wrapped level meets itself, the higher part is kept
                let height = shifts
                    .into_iter()
                    .filter_map(|s| level_at(on, from + s))
                    .reduce(f64::max);
                if let Some(h) = height {
                    total = level.combine.apply(total, h + level.base_height);
                    covered = true;
                }
            }
            surface.heights[row * surface.columns + column] = if covered { total } else { empty };
        }
    }
    surface
}
/// The model as a 3D toolpath machines it: cells a twentieth of `tool` across, but no finer
/// than the job's modeling resolution, placed in the material as Material Setup says (z = 0
/// is the material surface; by default the model's top is there). It must exist and fit.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a small positive cell count"
)]
pub fn placed_model(doc: &Document, tool: &Tool) -> Result<Material, CommandError> {
    if doc.components.is_empty() {
        return Err(CommandError::Failed(
            "there is no 3D model to machine: create a shape first".into(),
        ));
    }
    let longest = doc.job.width.max(doc.job.height);
    let cell = (tool.diameter / 20.0).max(doc.job.model_cell());
    let cells = ((longest / cell).ceil() as usize).max(1);
    let mut surface = model(doc, cells);
    let top = surface
        .heights
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let lowest = surface
        .heights
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min);
    if top - lowest > doc.job.thickness {
        return Err(CommandError::Failed(format!(
            "the model is {:.1} mm tall but the material is only {:.1} mm thick",
            top - lowest,
            doc.job.thickness
        )));
    }
    let shift = doc.job.model_shift(lowest, top);
    // ponytail: 1 µm slack for rounding
    if top + shift > 1e-6 || lowest + shift < -doc.job.thickness - 1e-6 {
        return Err(CommandError::Failed(
            "the model does not fit in the material where Material Setup puts it".into(),
        ));
    }
    for height in &mut surface.heights {
        *height += shift;
    }
    Ok(surface)
}

#[cfg(test)]
mod shade_tests {
    use super::*;

    #[test]
    fn shade_runs_from_black_at_the_lowest_to_white_at_the_top_row_first() {
        let k = Component::from_fn(
            "ramp",
            Combine::Add,
            Rect::new(Point::ZERO, Point::new(2.0, 2.0)),
            1.0,
            |p| (p.x < 1.5).then_some(p.y),
        );
        // rows from the bottom: [0.5, 0.5] under [1.5, 1.5]; the right column has no shape
        assert_eq!(k.shade(), vec![Some(255), None, Some(0), None]);
    }
}

#[cfg(test)]
mod shaping_tests {
    use super::*;

    fn flat(z: f64) -> Component {
        Component::from_fn(
            "flat",
            Combine::Add,
            Rect::new(Point::ZERO, Point::new(10.0, 10.0)),
            1.0,
            |_| Some(z),
        )
    }

    fn at(k: &Component, x: f64) -> f64 {
        k.height_at(Point::new(x, 5.5)).unwrap()
    }

    #[test]
    fn shape_height_scales_and_base_height_raises() {
        let mut k = flat(4.0);
        k.shaping.shape_height = 50.0;
        k.shaping.base_height = 1.0;
        assert!((at(&k, 5.5) - 3.0).abs() < 1e-9);
    }

    #[test]
    fn fade_keeps_the_first_point_and_lowers_toward_the_second() {
        let mut k = flat(4.0);
        k.shaping.fade = Some(Fade {
            from: Xy::new(0.5, 5.5),
            to: Xy::new(8.5, 5.5),
            percent: 50.0,
        });
        assert!((at(&k, 0.5) - 4.0).abs() < 1e-9);
        assert!((at(&k, 4.5) - 3.0).abs() < 1e-9);
        assert!(
            (at(&k, 9.5) - 2.0).abs() < 1e-9,
            "past the second point stays faded"
        );
    }

    #[test]
    fn tilt_raises_toward_the_second_point_by_the_angle() {
        let mut k = flat(1.0);
        k.shaping.tilt = Some(Tilt {
            pivot: Xy::new(0.5, 5.5),
            toward: Xy::new(5.5, 5.5),
            angle: 45.0,
        });
        assert!((at(&k, 0.5) - 1.0).abs() < 1e-9);
        assert!((at(&k, 4.5) - 5.0).abs() < 1e-9);
    }

    #[test]
    fn baking_keeps_the_heights_and_resets_the_shaping() {
        let mut k = flat(4.0);
        k.shaping.shape_height = 25.0;
        let baked = k.baked();
        assert!(baked.shaping.is_identity());
        assert!((at(&baked, 3.5) - 1.0).abs() < 1e-9);
        assert_eq!(baked.height_range(), k.height_range());
    }

    #[test]
    fn old_job_files_without_shaping_still_read() {
        let mut json = serde_json::to_value(flat(1.0)).unwrap();
        json.as_object_mut().unwrap().remove("shaping");
        let k: Component = serde_json::from_value(json).unwrap();
        assert!(k.shaping.is_identity());
    }
}
