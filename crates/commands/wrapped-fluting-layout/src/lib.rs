//! Wrapped Fluting Layout (an Aspire gadget): on a rotary job's unrolled surface, flutes along
//! the axis evenly spaced round the cylinder, and coves round it at either end if asked. It
//! lays out vectors for Profile or Fluting toolpaths; it makes no toolpaths itself.

use cut_craft_core::document::{Along, Document};
use cut_craft_core::geometry::{Path, Point};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct WrappedFlutingLayout;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    #[schemars(title = "Number of flutes", range(min = 1))]
    pub flutes: u32,
    /// From the start of the work piece to where the flutes start
    #[schemars(title = "Start offset", extend("x-unit" = "mm"), range(min = 0))]
    pub start: f64,
    /// From the end of the work piece back to where the flutes end
    #[schemars(title = "End offset", extend("x-unit" = "mm"), range(min = 0))]
    pub end: f64,
    /// A groove round the cylinder where the flutes start
    #[schemars(title = "Cove at start")]
    pub cove_start: bool,
    /// A groove round the cylinder where the flutes end
    #[schemars(title = "Cove at end")]
    pub cove_end: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            flutes: 8,
            start: 20.0,
            end: 20.0,
            cove_start: false,
            cove_end: false,
        }
    }
}

/// A rotary job's unrolled surface: along the axis `lo..hi`, round it `round_lo..round_hi`,
/// and a point at (along, round).
pub struct Unrolled {
    pub lo: f64,
    pub hi: f64,
    pub round_lo: f64,
    pub round_hi: f64,
    along: Along,
}

impl Unrolled {
    pub fn of(doc: &Document) -> Result<Self, CommandError> {
        let Some(rotary) = doc.job.rotary else {
            return Err(CommandError::Failed(
                "this gadget lays out a rotary job (Job Setup)".into(),
            ));
        };
        let a = doc.job.area();
        Ok(match rotary.along {
            Along::X => Self {
                lo: a.min.x,
                hi: a.max.x,
                round_lo: a.min.y,
                round_hi: a.max.y,
                along: Along::X,
            },
            Along::Y => Self {
                lo: a.min.y,
                hi: a.max.y,
                round_lo: a.min.x,
                round_hi: a.max.x,
                along: Along::Y,
            },
        })
    }

    pub fn at(&self, along: f64, round: f64) -> Point {
        match self.along {
            Along::X => Point::new(along, round),
            Along::Y => Point::new(round, along),
        }
    }

    pub fn round(&self) -> f64 {
        self.round_hi - self.round_lo
    }

    /// The span along the axis between the offsets.
    pub fn between(&self, start: f64, end: f64) -> Result<(f64, f64), CommandError> {
        let (from, to) = (self.lo + start, self.hi - end);
        if from >= to {
            return Err(CommandError::InvalidParams(
                "the offsets leave nothing of the work piece".into(),
            ));
        }
        Ok((from, to))
    }

    /// A cove: a line right round the cylinder at `along`.
    pub fn cove(&self, along: f64) -> Path {
        Path::polyline(
            &[self.at(along, self.round_lo), self.at(along, self.round_hi)],
            false,
        )
    }
}

impl Command for WrappedFlutingLayout {
    const ID: &'static str = "wrapped_fluting_layout";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Wrapped Fluting Layout",
            group: "Gadgets",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let job = Unrolled::of(doc)?;
        if p.flutes == 0 {
            return Err(CommandError::InvalidParams("at least one flute".into()));
        }
        let (from, to) = job.between(p.start, p.end)?;
        let gap = job.round() / f64::from(p.flutes);
        let mut c = Change::builder(doc);
        let mut ids = vec![];
        for i in 0..p.flutes {
            // centred in its share of the circumference
            let round = job.round_lo + gap * (f64::from(i) + 0.5);
            ids.push(c.add(Path::polyline(
                &[job.at(from, round), job.at(to, round)],
                false,
            )));
        }
        for (wanted, at) in [(p.cove_start, from), (p.cove_end, to)] {
            if wanted {
                ids.push(c.add(job.cove(at)));
            }
        }
        c.select(ids);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::document::Job;
    use cut_craft_core::testing::{rejection, run_on};

    fn rotary() -> Document {
        Document {
            job: Job::rotary(300.0, 50.0, Along::X),
            ..Document::default()
        }
    }

    cut_craft_core::contract_tests!(WrappedFlutingLayout, rotary);

    #[test]
    fn flutes_run_along_evenly_spaced_round() {
        let mut doc = rotary();
        let p = Params {
            flutes: 4,
            cove_start: true,
            ..Params::default()
        };
        run_on::<WrappedFlutingLayout>(&mut doc, &p);
        assert_eq!(doc.vectors.len(), 5, "4 flutes and a cove");
        let a = doc.job.area();
        let flute = doc.vectors.values().next().unwrap().bounds();
        assert!((flute.min.x - (a.min.x + 20.0)).abs() < 1e-9);
        assert!((flute.max.x - (a.max.x - 20.0)).abs() < 1e-9);
        assert!(
            (flute.min.y - (a.min.y + a.size().y / 8.0)).abs() < 1e-9,
            "centred in its quarter"
        );
    }

    #[test]
    fn needs_a_rotary_job_and_room() {
        assert!(rejection::<WrappedFlutingLayout>(&Params::default()).contains("rotary"));
        let too_far = Params {
            start: 200.0,
            end: 200.0,
            ..Params::default()
        };
        assert!(
            WrappedFlutingLayout::run(&rotary(), &too_far)
                .unwrap_err()
                .to_string()
                .contains("offsets")
        );
    }
}
