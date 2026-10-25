//! 3D components: height-map shapes that combine, in the order they were made, into the model.

use crate::CommandError;
use crate::document::{Document, EntityId};
use crate::geometry::{Point, Rect};
use crate::simulation::Material;
use crate::toolpath::Tool;
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

/// Heights over a grid of square cells in the job.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Component {
    pub name: String,
    pub combine: Combine,
    /// Lower-left corner of the grid, job mm.
    pub origin: Point,
    pub cell: f64,
    pub columns: usize,
    pub rows: usize,
    /// Row by row from the bottom; `None` where the component has no shape.
    // ponytail: plain JSON numbers; a packed encoding if job files grow too big
    pub heights: Vec<Option<f64>>,
    /// Hidden components are left out of the model (added after the first job format).
    #[serde(default = "shown")]
    pub visible: bool,
}

fn shown() -> bool {
    true
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
            self.heights[row * self.columns + column]
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
            heights: material.heights.iter().map(|&h| Some(h)).collect(),
            visible: true,
        }
    }

    /// The area its grid covers, in job mm.
    #[expect(clippy::cast_precision_loss, reason = "cell counts are small")]
    pub fn extent(&self) -> Rect {
        let size = Point::new(self.columns as f64, self.rows as f64) * self.cell;
        Rect::new(self.origin, self.origin + size)
    }

    /// Lowest and highest of its heights, or `None` when it has no shape.
    pub fn height_range(&self) -> Option<(f64, f64)> {
        self.heights.iter().flatten().fold(None, |range, &h| {
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
        (0..self.rows)
            .rev()
            .flat_map(|row| &self.heights[row * self.columns..(row + 1) * self.columns])
            .map(|h| h.map(|z| ((z - lo) / span * 255.0).round().clamp(0.0, 255.0) as u8))
            .collect()
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
        [one] => Some((ids.clone(), doc.components[one].clone())),
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

/// The model's surface over the job: every component combined onto a flat base at 0.
pub fn model(doc: &Document, cells_across: usize) -> Material {
    let mut surface = Material::new(&doc.job, cells_across);
    for row in 0..surface.rows {
        for column in 0..surface.columns {
            let centre = surface.centre(column, row);
            surface.heights[row * surface.columns + column] = doc
                .components
                .values()
                .filter(|c| c.visible)
                .fold(0.0, |under, c| {
                    c.height_at(centre)
                        .map_or(under, |height| c.combine.apply(under, height))
                });
        }
    }
    surface
}

/// The model as a 3D toolpath machines it: cells a twentieth of `tool` across, but no finer
/// than the job's modeling resolution, its top at z = 0 (the material surface). It must
/// exist and fit in the material.
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
    for height in &mut surface.heights {
        *height -= top;
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
