//! What every rotary carving toolpath sets up before it works out a pass: the model round
//! the axis, the stretch along the part it cuts, and what Job Setup leaves to hold the part.

use cut_craft_core::CommandError;
use cut_craft_core::document::{Along, Document, Rotary, Units};
use cut_craft_core::toolpath::Tool;
use cut_craft_rotary::{Stock, Voxels};

use crate::{Limit, Solid, Turned, Work, model_along, parts, supporting};

/// The part as a rotary carving toolpath sees it, ready to work passes out over
/// ([`Self::work`]).
pub struct Carving {
    pub rotary: Rotary,
    pub turned: Turned,
    pub solid: Option<Solid>,
    /// What is left of the material to hold the part, kept like a model.
    pub support: Option<Solid>,
    pub tool: Tool,
    pub allowance: f64,
    pub along_x: bool,
    /// The material's length along the axis, and its circumference round it.
    pub length: f64,
    pub circumference: f64,
    /// The material's radius, and how far out it swings as it turns.
    pub radius: f64,
    pub top: f64,
    /// Where along the part the passes start and end, and the step between them.
    pub start: f64,
    pub end: f64,
    pub step: f64,
    pub units: Units,
    /// Where on the unrolled material the bit may cut: none for all of it.
    pub limit: Option<Limit>,
}

/// Where a carving toolpath cuts along the part, how far apart its passes are, and what it
/// leaves on the model.
#[derive(Clone, Copy, Debug)]
pub struct Reach {
    /// From the job's start; `end` 0 runs to the job's end.
    pub start: f64,
    pub end: f64,
    pub step: f64,
    pub allowance: f64,
    /// Cut away the centre stock Job Setup leaves along the axis.
    pub cut_support: bool,
}

impl Carving {
    /// The model in `doc` round its turning axis, for `tool`, as `reach` says.
    ///
    /// # Errors
    /// When the job is not rotary, or End is not past Start.
    pub fn of(doc: &Document, tool: &Tool, reach: Reach) -> Result<Self, CommandError> {
        let Some(rotary) = doc.job.rotary else {
            return Err(CommandError::Failed(
                "this toolpath turns the material: set the job up as rotary in Job Setup".into(),
            ));
        };
        let along_x = rotary.along == Along::X;
        let (length, circumference) = if along_x {
            (doc.job.width, doc.job.height)
        } else {
            (doc.job.height, doc.job.width)
        };
        let radius = rotary.diameter / 2.0;
        cut_craft_core::progress::stage("Placing the model");
        let (rest, solid) = parts(doc, tool, reach.allowance)?;
        // as fine as the model's own cells (as placing the model makes them), or the bit's
        let cell = (tool.diameter / 16.0).max(
            rest.as_ref()
                .map_or((tool.diameter / 20.0).max(doc.job.model_cell()), |m| m.cell),
        );
        let turned = Turned::of(
            rest.as_ref(),
            radius,
            along_x,
            (length, circumference),
            cell,
            doc.job.area().min,
        );
        let start = reach.start.clamp(0.0, length);
        let end = if reach.end > 0.0 {
            reach.end.min(length)
        } else {
            length
        };
        if end <= start {
            return Err(CommandError::InvalidParams(
                "the rings must end further along than they start".into(),
            ));
        }
        let mut carving = Self {
            rotary,
            turned,
            solid,
            support: None,
            tool: tool.clone(),
            allowance: reach.allowance,
            along_x,
            length,
            circumference,
            radius,
            top: Stock::of(&doc.job).map_or(radius, |stock| stock.swing().max(radius)),
            start,
            end,
            step: reach.step,
            units: doc.job.units,
            limit: None,
        };
        // what Job Setup leaves to hold the part, kept like a model
        let origin = if along_x {
            doc.job.area().min.x
        } else {
            doc.job.area().min.y
        };
        carving.support = supporting(
            rotary.support,
            tool,
            carving.top,
            length,
            origin,
            model_along(&carving.work()),
            reach.cut_support,
        );
        Ok(carving)
    }

    /// The passes are worked out on this.
    #[must_use]
    pub fn work(&self) -> Work<'_> {
        Work {
            turned: &self.turned,
            solid: self.solid.as_ref(),
            support: self.support.as_ref(),
            tool: &self.tool,
            allowance: self.allowance,
            along_x: self.along_x,
            circumference: self.circumference,
            start: self.start,
            end: self.end,
            step: self.step,
            top: self.top,
            units: self.units,
            limit: self.limit.as_ref(),
        }
    }

    /// Kept within `limit` (none for the whole material): the passes worked out only as far
    /// along the part as it reaches, and a bit's width either side.
    ///
    /// # Errors
    /// When the limit lies wholly outside Start and End.
    pub fn limited(mut self, limit: Option<Limit>) -> Result<Self, CommandError> {
        if let Some(l) = &limit {
            let reach = self.tool.diameter / 2.0;
            self.start = self.start.max(l.from - reach);
            self.end = self.end.min(l.to + reach);
            if self.end <= self.start {
                return Err(CommandError::InvalidParams(
                    "the machining limit lies outside Start and End: nothing to cut".into(),
                ));
            }
        }
        self.limit = limit;
        Ok(self)
    }
}

/// How far apart `tool`'s finishing passes are: its finishing stepover of its diameter.
///
/// # Errors
/// When the stepover is not more than 0% and at most 100%.
pub fn finishing_step(tool: &Tool) -> Result<f64, CommandError> {
    cut_craft_core::ensure_positive("bit diameter", tool.diameter)?;
    let stepover = tool.finishing_stepover();
    if !(stepover > 0.0 && stepover <= 1.0) {
        return Err(CommandError::InvalidParams(format!(
            "the finishing stepover of {} must be more than 0% and at most 100%: set it in \
             the Bit Library",
            tool.name
        )));
    }
    Ok(stepover * tool.diameter)
}

impl Carving {
    /// What is left of the material for `tool` to cut, kept in cells no finer than a twelfth
    /// of it (what is left between is not worth a pass): the whole blank when `whole_blank`
    /// (Settings' advanced rotary mode), else what the toolpaths above the one being made
    /// leave of it.
    #[must_use]
    pub fn left_of(&self, doc: &Document, stock: &Stock, whole_blank: bool) -> Voxels {
        let mut material = Voxels::of(
            stock,
            Voxels::cell_for(stock, crate::CELLS).max(self.tool.diameter / 12.0),
        );
        if !whole_blank {
            cut_craft_core::progress::stage("Working out what the toolpaths above leave");
            crate::cut_by(
                &mut material,
                &doc.toolpaths_before_made(),
                self.corner(doc),
                self.radius,
                self.along_x,
            );
        }
        material
    }

    /// Where the material's corner is in the job: the passes are worked out from it.
    #[must_use]
    pub fn corner(&self, doc: &Document) -> cut_craft_core::glam::DVec3 {
        let corner = doc.job.area().min;
        cut_craft_core::glam::DVec3::new(corner.x, corner.y, 0.0)
    }
}
