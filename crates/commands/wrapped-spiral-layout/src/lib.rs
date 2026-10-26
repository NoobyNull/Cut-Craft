//! Wrapped Spiral Layout (an Aspire gadget): rope or barley-twist strands on a rotary job's
//! unrolled surface. Each strand starts evenly spaced round the cylinder and runs along the
//! axis turning by the pitch; it runs past the job's edge, and wrapping makes it a spiral.
//! Coves round the ends if asked. It lays out vectors for Profile toolpaths only.

use cmd_wrapped_fluting_layout::Unrolled;
use cut_craft_core::document::Document;
use cut_craft_core::geometry::Path;
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct WrappedSpiralLayout;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Twist {
    #[default]
    #[schemars(title = "Right hand")]
    Right,
    #[schemars(title = "Left hand")]
    Left,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Spacing {
    /// How far along the axis a strand goes in one turn
    #[default]
    #[schemars(title = "Spiral Pitch")]
    Pitch,
    /// How far apart neighbouring strands are along the axis
    #[schemars(title = "Spacing between strands")]
    BetweenStrands,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    #[schemars(title = "Number of strands", range(min = 1))]
    pub strands: u32,
    #[schemars(title = "Start offset", extend("x-unit" = "mm"), range(min = 0))]
    pub start: f64,
    #[schemars(title = "End offset", extend("x-unit" = "mm"), range(min = 0))]
    pub end: f64,
    #[schemars(extend("x-widget" = "row"))]
    pub spacing: Spacing,
    /// The pitch, or the spacing between strands, as chosen above
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub distance: f64,
    #[schemars(extend("x-widget" = "row"))]
    pub twist: Twist,
    #[schemars(title = "Cove at start")]
    pub cove_start: bool,
    #[schemars(title = "Cove at end")]
    pub cove_end: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            strands: 3,
            start: 20.0,
            end: 20.0,
            spacing: Spacing::Pitch,
            distance: 60.0,
            twist: Twist::Right,
            cove_start: false,
            cove_end: false,
        }
    }
}

impl Command for WrappedSpiralLayout {
    const ID: &'static str = "wrapped_spiral_layout";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Wrapped Spiral Layout",
            group: "Gadgets",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let job = Unrolled::of(doc)?;
        if p.strands == 0 || p.distance <= 0.0 {
            return Err(CommandError::InvalidParams(
                "at least one strand, and a pitch or spacing above 0".into(),
            ));
        }
        let (from, to) = job.between(p.start, p.end)?;
        let strands = f64::from(p.strands);
        let pitch = match p.spacing {
            Spacing::Pitch => p.distance,
            Spacing::BetweenStrands => p.distance * strands,
        };
        let round = job.round();
        let hand = match p.twist {
            Twist::Right => 1.0,
            Twist::Left => -1.0,
        };
        let turn = hand * round * (to - from) / pitch;
        let mut c = Change::builder(doc);
        let mut ids = vec![];
        for i in 0..p.strands {
            let start = job.round_lo + round * f64::from(i) / strands;
            ids.push(c.add(Path::polyline(
                &[job.at(from, start), job.at(to, start + turn)],
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

/// How many times round each strand goes.
pub fn revolutions(doc: &Document, p: &Params) -> Option<f64> {
    let job = Unrolled::of(doc).ok()?;
    let (from, to) = job.between(p.start, p.end).ok()?;
    let pitch = match p.spacing {
        Spacing::Pitch => p.distance,
        Spacing::BetweenStrands => p.distance * f64::from(p.strands),
    };
    (pitch > 0.0).then(|| (to - from) / pitch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::document::{Along, Job};
    use cut_craft_core::testing::{rejection, run_on};

    fn rotary() -> Document {
        Document {
            job: Job::rotary(300.0, 50.0, Along::X),
            ..Document::default()
        }
    }

    cut_craft_core::contract_tests!(WrappedSpiralLayout, rotary);

    #[test]
    fn strands_turn_once_per_pitch() {
        let mut doc = rotary();
        let p = Params {
            strands: 2,
            ..Params::default()
        };
        run_on::<WrappedSpiralLayout>(&mut doc, &p);
        assert_eq!(doc.vectors.len(), 2);
        // 260 mm between the offsets at a 60 mm pitch: 4⅓ turns of the circumference
        assert!((revolutions(&doc, &p).unwrap() - 260.0 / 60.0).abs() < 1e-9);
        let strand = doc.vectors.values().next().unwrap().bounds();
        let round = doc.job.area().size().y;
        assert!(
            (strand.size().y - round * 260.0 / 60.0).abs() < 1e-6,
            "runs past the edge"
        );
    }

    #[test]
    fn spacing_between_strands_sets_the_pitch_and_left_hand_turns_the_other_way() {
        let doc = rotary();
        let p = Params {
            spacing: Spacing::BetweenStrands,
            distance: 20.0,
            ..Params::default()
        };
        assert!(
            (revolutions(&doc, &p).unwrap() - 260.0 / 60.0).abs() < 1e-9,
            "3 strands 20 apart"
        );
        let mut left = rotary();
        run_on::<WrappedSpiralLayout>(
            &mut left,
            &Params {
                twist: Twist::Left,
                ..Params::default()
            },
        );
        let strand = left.vectors.values().next().unwrap();
        let ends = strand.flatten(0.01);
        assert!(ends.last().unwrap().y < ends[0].y, "left hand goes down");
    }

    #[test]
    fn needs_a_rotary_job() {
        assert!(rejection::<WrappedSpiralLayout>(&Params::default()).contains("rotary"));
    }
}
