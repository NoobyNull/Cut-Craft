//! Rounding Toolpath: on a rotary job, turns a square or round blank down to the job's
//! diameter in levels, either round the cylinder or along it.

use cut_craft_core::document::{Along, Document};
use cut_craft_core::geometry::Point;
use cut_craft_core::glam::DVec3;
use cut_craft_core::toolpath::{Move, Tool, follow, made_by, pass_depths};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct RoundingToolpath;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Blank {
    /// Square stock; its size is the side
    #[default]
    Square,
    /// Round stock; its size is the diameter
    Round,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    /// A full turn round the cylinder, then a stepover along it, and back
    Radial,
    /// Along the cylinder, then a stepover round it, and back
    #[default]
    Raster,
    /// Along the cylinder only where a square blank still has material
    OptimizedRaster,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
    pub blank: Blank,
    /// Side of a square blank or diameter of a round one
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub blank_size: f64,
    pub method: Method,
    /// Left on the finished cylinder
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub allowance: f64,
    /// Its stepover, pass depth and feed rates are used
    pub tool: Tool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Rounding".into(),
            blank: Blank::Square,
            blank_size: 55.0,
            method: Method::Raster,
            allowance: 0.0,
            tool: Tool::default(),
        }
    }
}

impl Params {
    /// Distance from the axis to the blank's surface at `angle` round it.
    fn blank_radius(&self, angle: f64) -> f64 {
        match self.blank {
            Blank::Round => self.blank_size / 2.0,
            Blank::Square => self.blank_size / 2.0 / angle.cos().abs().max(angle.sin().abs()),
        }
    }

    /// Largest distance from the axis to the blank's surface.
    fn outer_radius(&self) -> f64 {
        match self.blank {
            Blank::Round => self.blank_size / 2.0,
            Blank::Square => self.blank_size / std::f64::consts::SQRT_2,
        }
    }
}

/// Steps from 0 to `length` no more than `step` apart, both ends included.
fn steps(length: f64, step: f64) -> Vec<f64> {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a count"
    )]
    let n = (length / step).ceil().max(1.0) as u32;
    (0..=n)
        .map(|i| length * f64::from(i) / f64::from(n))
        .collect()
}

impl Command for RoundingToolpath {
    const ID: &'static str = "rounding_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Rounding Toolpath",
            group: "Toolpath Operations",
            icon: "toolpaths/rounding_toolpath.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let Some(rotary) = doc.job.rotary else {
            return Err(CommandError::Failed(
                "rounding needs a rotary job: set one up in Job Setup".into(),
            ));
        };
        ensure_positive("tool diameter", p.tool.diameter)?;
        ensure_positive("pass depth", p.tool.pass_depth)?;
        if !(p.tool.stepover > 0.0 && p.tool.stepover <= 1.0) {
            return Err(CommandError::InvalidParams(
                "stepover must be more than 0 and at most 1".into(),
            ));
        }
        if p.method == Method::OptimizedRaster && p.blank == Blank::Round {
            return Err(CommandError::InvalidParams(
                "the optimized raster is for square blanks".into(),
            ));
        }
        let finished = rotary.diameter / 2.0 + p.allowance.max(0.0);
        let top = p.outer_radius() - rotary.diameter / 2.0;
        if p.outer_radius() <= finished {
            return Err(CommandError::Failed(
                "the blank is no bigger than the finished cylinder".into(),
            ));
        }
        // job coordinates: `along` the axis and `round` it (unrolled at the job's diameter)
        let area = doc.job.area();
        let size = area.max - area.min;
        let (length, circumference) = match rotary.along {
            Along::X => (size.x, size.y),
            Along::Y => (size.y, size.x),
        };
        let at = |along: f64, round: f64, z: f64| {
            let flat = match rotary.along {
                Along::X => Point::new(along, round),
                Along::Y => Point::new(round, along),
            };
            (area.min + flat).extend(z)
        };
        let step = p.tool.stepover * p.tool.diameter;
        let safe = top + doc.job.clearance;
        let surface_radius = rotary.diameter / 2.0;
        let mut moves: Vec<Move> = vec![];
        for z in pass_depths(p.outer_radius() - finished, p.tool.pass_depth)
            .into_iter()
            .map(|down| top + down)
        {
            let radius = surface_radius + z;
            match p.method {
                Method::Radial => {
                    let mut path: Vec<DVec3> = vec![];
                    for (i, a) in steps(length, step).into_iter().enumerate() {
                        let (from, to) = if i % 2 == 0 {
                            (0.0, circumference)
                        } else {
                            (circumference, 0.0)
                        };
                        path.push(at(a, from, z));
                        path.push(at(a, to, z));
                    }
                    moves.extend(follow(&path, safe));
                }
                Method::Raster | Method::OptimizedRaster => {
                    // runs round the cylinder where there is material to cut at this level
                    let mut run: Vec<DVec3> = vec![];
                    let mut forward = true;
                    for u in steps(circumference, step) {
                        let angle = u / surface_radius;
                        let material = p.method == Method::Raster || p.blank_radius(angle) > radius;
                        if !material {
                            moves.extend(follow(&run, safe));
                            run.clear();
                            continue;
                        }
                        let (from, to) = if forward {
                            (0.0, length)
                        } else {
                            (length, 0.0)
                        };
                        run.push(at(from, u, z));
                        run.push(at(to, u, z));
                        forward = !forward;
                    }
                    moves.extend(follow(&run, safe));
                }
            }
        }
        let mut c = Change::builder(doc);
        c.add_toolpath(made_by::<Self>(
            p,
            p.name.clone(),
            p.tool.clone(),
            vec![],
            moves,
        ));
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::document::Job;
    use cut_craft_core::toolpath::{MoveKind, Toolpath};

    /// A 100 mm long, 50 mm diameter rotary job along X.
    fn rotary_job() -> Document {
        Document {
            job: Job::rotary(100.0, 50.0, Along::X),
            ..Document::default()
        }
    }

    cut_craft_core::contract_tests!(RoundingToolpath, rotary_job);

    fn round(p: &Params) -> Toolpath {
        let mut doc = rotary_job();
        let change = RoundingToolpath::run(&doc, p).unwrap();
        doc.apply(&change).unwrap();
        doc.toolpaths.into_values().next().unwrap()
    }

    fn levels(t: &Toolpath) -> Vec<f64> {
        let mut z: Vec<f64> = t
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Plunge)
            .map(|m| m.to.z)
            .collect();
        z.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        z
    }

    #[test]
    fn cuts_down_in_levels_from_the_blank_to_the_finished_diameter() {
        let p = Params {
            blank: Blank::Round,
            blank_size: 60.0,
            ..Params::default()
        };
        let z = levels(&round(&p));
        // a 60 mm blank to 50 mm is 5 mm; 3 mm passes: two levels of 2.5 mm
        assert_eq!(z.len(), 2, "{z:?}");
        assert!((z[0] - 2.5).abs() < 1e-9 && z[1].abs() < 1e-9, "{z:?}");
    }

    #[test]
    fn the_allowance_is_left_on() {
        let p = Params {
            blank: Blank::Round,
            blank_size: 60.0,
            allowance: 0.5,
            ..Params::default()
        };
        let z = levels(&round(&p));
        assert!((z[z.len() - 1] - 0.5).abs() < 1e-9, "{z:?}");
    }

    #[test]
    fn raster_runs_along_the_axis_and_radial_round_it() {
        let cut_dirs = |t: &Toolpath| {
            let cuts: Vec<DVec3> = t
                .moves
                .iter()
                .filter(|m| m.kind != MoveKind::Rapid)
                .map(|m| m.to)
                .collect();
            cuts.windows(2)
                .map(|w| w[1] - w[0])
                .find(|d| d.truncate().length() > 50.0)
                .unwrap()
        };
        let raster = cut_dirs(&round(&Params::default()));
        assert!(
            raster.y.abs() < 1e-9 && (raster.x.abs() - 100.0).abs() < 1e-9,
            "{raster:?}"
        );
        let radial = cut_dirs(&round(&Params {
            method: Method::Radial,
            ..Params::default()
        }));
        assert!(radial.x.abs() < 1e-9, "{radial:?}");
    }

    #[test]
    fn optimized_raster_skips_where_the_square_has_no_material() {
        let full = round(&Params::default()).moves.len();
        let optimized = round(&Params {
            method: Method::OptimizedRaster,
            ..Params::default()
        })
        .moves
        .len();
        assert!(optimized < full, "{optimized} < {full}");
    }

    #[test]
    fn needs_a_rotary_job() {
        let why = RoundingToolpath::run(&Document::default(), &Params::default())
            .unwrap_err()
            .to_string();
        assert!(why.contains("rotary"), "{why}");
    }
}
