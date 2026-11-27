//! A rotary toolpath's machining limit: where on the unrolled material (the job's sheet,
//! wrapped round the part) its bit may cut. The Material is the whole of it; the Model, a
//! level's components or selected closed vectors keep it within their outline, as on a flat
//! job, wrapped round the part as the sheet is.

use cut_craft_core::CommandError;
use cut_craft_core::component::{MachiningBoundary, machining_boundary};
use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::geometry::{Point, inside};
use cut_craft_core::glam::DVec3;
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

/// How finely the way from one point of a pass to the next is looked along, mm.
const STEP: f64 = 0.1;

/// The most places looked at on the way from one point to the next.
const MOST_STEPS: usize = 40;

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
    pub fn holds_point(&self, along_x: bool, point: &(DVec3, f64)) -> bool {
        let (along, round) = self.place(along_x, point);
        self.holds(along, round)
    }

    /// Whether the bit may cut at `to`, coming straight from `from` (where it was cutting
    /// just before, if it was): `to` inside the limit, and the way there not leaving it for
    /// a gap wider than [`STEP`].
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "a few dozen steps"
    )]
    pub fn holds_step(
        &self,
        along_x: bool,
        from: Option<&(DVec3, f64)>,
        to: &(DVec3, f64),
    ) -> bool {
        if !self.holds_point(along_x, to) {
            return false;
        }
        let Some(from) = from else {
            return true;
        };
        let (a, b) = (self.place(along_x, from), self.place(along_x, to));
        // the short way round
        let c = self.circumference;
        let mut turn = (b.1 - a.1).rem_euclid(c);
        if turn > c / 2.0 {
            turn -= c;
        }
        let (along, round) = (b.0 - a.0, turn);
        // looked at every tenth of a millimetre on the way (no more than a few dozen times)
        let steps = ((along.hypot(round) / STEP).ceil() as usize).clamp(1, MOST_STEPS);
        (1..steps).all(|k| {
            let t = k as f64 / steps as f64;
            self.holds(a.0 + along * t, a.1 + round * t)
        })
    }

    /// Where on the unrolled sheet a point of a pass cuts: along, and round (mm round the
    /// surface) where its tip is, held across or not.
    fn place(&self, along_x: bool, &(point, across): &(DVec3, f64)) -> (f64, f64) {
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
        (along, round)
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
                if self.holds_step(along_x, run.last(), &point) {
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

/// Why a rotary toolpath's machining limit leaves it nothing to cut, said in red: no closed
/// vectors selected, no level of that name, or outlines with nothing inside. The toolpath is
/// then made with no moves rather than refused, so the job keeps it and recalculating once the
/// limit is put right fills it in. `None` when the limit has something in it (or is the whole
/// Material, or the job isn't rotary).
#[must_use]
pub fn nothing_in(
    doc: &Document,
    boundary: MachiningBoundary,
    offset: f64,
    level: &str,
) -> Option<String> {
    Limit::of(doc, boundary, offset, level).err()?;
    let missing = boundary == MachiningBoundary::SelectedLevel
        && !doc.levels.values().any(|l| l.name == level);
    Some(if missing {
        format!(
            "Nothing is cut: there is no level called \"{level}\" for the machining limit. \
             Pick the level in Level, or another limit, then press Calculate."
        )
    } else {
        format!("Nothing is cut. {EMPTY}")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 0 to 40 along and 0 to 100 round, with a slot from 20.4 to 20.6 along taken out of it.
    fn slotted() -> Limit {
        let ring = |points: &[(f64, f64)]| points.iter().map(|&(x, y)| Point::new(x, y)).collect();
        Limit {
            rings: vec![
                ring(&[(0.0, 0.0), (40.0, 0.0), (40.0, 100.0), (0.0, 100.0)]),
                ring(&[(20.4, 10.0), (20.6, 10.0), (20.6, 90.0), (20.4, 90.0)]),
            ],
            circumference: 125.0,
            radius: 20.0,
            from: 0.0,
            to: 40.0,
        }
    }

    fn at(along: f64, round: f64) -> (DVec3, f64) {
        (DVec3::new(along, round, 0.0), 0.0)
    }

    #[test]
    fn a_gap_narrower_than_the_points_apart_is_never_cut_across() {
        let limit = slotted();
        assert!(limit.holds_point(true, &at(20.0, 20.0)));
        assert!(limit.holds_point(true, &at(21.0, 20.0)));
        assert!(!limit.holds_step(true, Some(&at(20.0, 20.0)), &at(21.0, 20.0)));
        // away from the slot, and coming down fresh, it cuts
        assert!(limit.holds_step(true, Some(&at(10.0, 20.0)), &at(11.0, 20.0)));
        assert!(limit.holds_step(true, None, &at(21.0, 20.0)));
        // the short way round the part, not the long way back across everything
        let round = Limit {
            rings: vec![slotted().rings[0].clone()],
            circumference: 100.0,
            ..slotted()
        };
        assert!(round.holds_step(true, Some(&at(10.0, 99.5)), &at(10.0, 100.5)));
    }

    #[test]
    fn a_limit_with_nothing_in_it_says_so_instead_of_refusing() {
        use cut_craft_core::document::{Along, Job};
        let doc = Document {
            job: Job::rotary(60.0, 40.0, Along::X),
            ..Document::default()
        };
        assert_eq!(nothing_in(&doc, MachiningBoundary::Material, 0.0, ""), None);
        let none = nothing_in(&doc, MachiningBoundary::SelectedVectors, 0.0, "").unwrap();
        assert!(none.starts_with("Nothing is cut"), "{none}");
        assert!(none.contains("select the closed vectors"), "{none}");
        let gone = nothing_in(&doc, MachiningBoundary::SelectedLevel, 0.0, "Leaves").unwrap();
        assert!(gone.contains("no level called \"Leaves\""), "{gone}");
        assert!(gone.contains("Pick the level"), "says what to do: {gone}");
    }
}
