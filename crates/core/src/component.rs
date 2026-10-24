//! 3D components: height-map shapes that combine, in the order they were made, into the model.

use crate::CommandError;
use crate::document::Document;
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
