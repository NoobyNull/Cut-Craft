//! A rotary toolpath's machining limit: where on the unrolled material (the job's sheet,
//! wrapped round the part) its bit may cut. The Material is the whole of it; the Model, a
//! level's components or selected closed vectors keep it within their outline, as on a flat
//! job, wrapped round the part as the sheet is.

use cut_craft_core::CommandError;
use cut_craft_core::component::{MachiningBoundary, machining_boundary};
use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::geometry::{Point, inside};
use cut_craft_core::rotary_axis::tip_of;
use cut_craft_core::toolpath::cut_points;

use crate::Pass;

/// Where a rotary toolpath may cut, on the unrolled material.
#[derive(Clone, Debug)]
pub struct Limit {
    /// The outlines (and holes) on the sheet, from the material's corner: x along the part
    /// and y round it, whichever way the job lies.
    rings: Vec<Vec<Point>>,
    circumference: f64,
    radius: f64,
    /// How far along the part the outlines reach, from the material's start.
    pub from: f64,
    pub to: f64,
}

/// Why a limit with nothing in it is refused.
const EMPTY: &str = "The machining limit has nothing in it: select the closed vectors to \
    cut inside before you press Calculate (drawn on the job, they wrap round the part), or pick \
    another limit.";

impl Limit {
    /// The limit `boundary` asks for on `doc` (grown by `offset`; `level` names the level
    /// for Selected Level), for a material of `radius` and `circumference`: none for the
    /// whole Material.
    ///
    /// # Errors
    /// When the level named isn't there, or the limit has nothing in it.
    pub fn of(
        doc: &Document,
        boundary: MachiningBoundary,
        offset: f64,
        level: &str,
    ) -> Result<Option<Self>, CommandError> {
        if boundary == MachiningBoundary::Material {
            return Ok(None);
        }
        let Some(rotary) = doc.job.rotary else {
            return Ok(None);
        };
        let along_x = rotary.along == cut_craft_core::document::Along::X;
        let corner = doc.job.area().min;
        let circumference = if along_x {
            doc.job.height
        } else {
            doc.job.width
        };
        let regions = machining_boundary(doc, boundary, offset, level)?;
        let rings: Vec<Vec<Point>> = regions
            .iter()
            .map(|path| {
                cut_points(path)
                    .into_iter()
                    .map(|p| {
                        let p = p - corner;
                        if along_x { p } else { Point::new(p.y, p.x) }
                    })
                    .collect::<Vec<Point>>()
            })
            .filter(|ring| ring.len() >= 3)
            .collect();
        if rings.is_empty() {
            return Err(CommandError::InvalidParams(EMPTY.into()));
        }
        let (from, to) = rings
            .iter()
            .flatten()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), p| {
                (a.min(p.x), b.max(p.x))
            });
        Ok(Some(Self {
            rings,
            circumference,
            radius: rotary.diameter / 2.0,
            from,
            to,
        }))
    }

    /// Whether the bit may cut at `along` and `round` (mm round the surface, any number of
    /// turns) on the unrolled material.
    #[must_use]
    pub fn holds(&self, along: f64, round: f64) -> bool {
        let round = round.rem_euclid(self.circumference);
        inside(&self.rings, Point::new(along, round))
    }

    /// Whether the bit may cut at a point of a pass: where its tip cuts, held across or not.
    #[must_use]
    pub fn holds_point(&self, along_x: bool, &(point, across): &(cut_craft_core::glam::DVec3, f64)) -> bool {
        let (along, round) = if along_x {
            (point.x, point.y)
        } else {
            (point.y, point.x)
        };
        let round = if across.abs() > 1e-9 {
            tip_of(self.radius, round, point.z, across).0
        } else {
            round
        };
        self.holds(along, round)
    }

    /// `passes` with the stretches outside the limit taken out: a pass leaving it and coming
    /// back is two passes, the second coming down slowly.
    #[must_use]
    pub fn keep(&self, along_x: bool, passes: Vec<(Pass, bool)>) -> Vec<(Pass, bool)> {
        let mut out = vec![];
        for (pass, quick) in passes {
            let mut run: Pass = vec![];
            // only a stretch from the pass's own start comes down as it would have
            let mut from_start = true;
            for point in pass {
                if self.holds_point(along_x, &point) {
                    run.push(point);
                } else {
                    if !run.is_empty() {
                        out.push((std::mem::take(&mut run), quick && from_start));
                    }
                    from_start = false;
                }
            }
            if !run.is_empty() {
                out.push((run, quick && from_start));
            }
        }
        out
    }
}

/// The closed vectors a Selected Vector(s) limit is made from, kept with the toolpath so
/// recalculating uses them again (and changing them marks it out of date).
#[must_use]
pub fn limit_vectors(doc: &Document, boundary: MachiningBoundary) -> Vec<EntityId> {
    match boundary {
        MachiningBoundary::SelectedVectors => doc.selected_paths(|path| path.closed).0,
        _ => vec![],
    }
}
