//! Fluting Toolpath: cuts along the selected vectors from their start points, ramping the
//! depth so the cut tapers in (and out), as for decorative flutes.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::Point;
use cut_craft_core::toolpath::{Tool, cut_points, follow, made_by, project_onto_model};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct FlutingToolpath;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FluteType {
    /// From the start depth at the start of each vector down to full depth at its end
    #[default]
    RampOverCompleteLength,
    /// Down to full depth over the ramp, then along at full depth
    RampAtStart,
    /// Down at the start and back up at the end
    RampAtStartAndEnd,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RampType {
    /// A straight slope
    #[default]
    Linear,
    /// A curve that levels out into the full depth
    Smooth,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
    /// Below the material surface where the flute begins (the floor of an earlier pocket)
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub start_depth: f64,
    /// Below the start depth at the deepest point
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub flute_depth: f64,
    pub flute_type: FluteType,
    pub ramp_type: RampType,
    /// Size the ramps as a percentage of the longest they can be, instead of a length
    pub use_ramp_percent: bool,
    /// Length of each ramp along the vector (capped at the longest it can be)
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub ramp_length: f64,
    /// Percentage of the longest ramp: the whole vector at the start only, half of it at both ends
    #[schemars(range(min = 0, max = 100))]
    pub ramp_percent: f64,
    pub tool: Tool,
    /// After calculating, drop the toolpath onto the 3D model: its depths are then below the
    /// model's surface, going no lower than the bottom of the material
    #[serde(default)]
    #[schemars(title = "Project toolpath onto 3D Model")]
    pub project_onto_model: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Fluting".into(),
            start_depth: 0.0,
            flute_depth: 3.0,
            flute_type: FluteType::RampAtStartAndEnd,
            ramp_type: RampType::Smooth,
            use_ramp_percent: true,
            ramp_length: 10.0,
            ramp_percent: 50.0,
            tool: Tool::default(),
            project_onto_model: false,
        }
    }
}

/// Segments each ramp is split into so a smooth ramp follows its curve.
const RAMP_STEPS: u32 = 32;

impl Params {
    /// Length of the ramp(s) on a vector `length` long.
    fn ramp(&self, length: f64) -> f64 {
        let longest = match self.flute_type {
            FluteType::RampOverCompleteLength => return length,
            FluteType::RampAtStart => length,
            FluteType::RampAtStartAndEnd => length / 2.0,
        };
        if self.use_ramp_percent {
            longest * self.ramp_percent / 100.0
        } else {
            self.ramp_length.min(longest)
        }
    }

    /// How far down the flute is (0 at the start depth, 1 at full depth) at `s` along a vector
    /// `length` long whose ramps are `ramp` long.
    fn fraction(&self, s: f64, length: f64, ramp: f64) -> f64 {
        let shape = |t: f64| {
            let t = t.clamp(0.0, 1.0);
            match self.ramp_type {
                RampType::Linear => t,
                // ponytail: quarter sine (level at full depth); Aspire's exact curve is unknown
                RampType::Smooth => (t * std::f64::consts::FRAC_PI_2).sin(),
            }
        };
        let down = shape(s / ramp);
        match self.flute_type {
            FluteType::RampAtStartAndEnd => down.min(shape((length - s) / ramp)),
            _ => down,
        }
    }
}

/// `points`, each with its distance along them, with extra points inserted at each distance
/// in `at` (sorted).
fn insert_at(points: &[Point], at: &[f64]) -> Vec<(Point, f64)> {
    let mut out = vec![(points[0], 0.0)];
    let mut along = 0.0;
    let mut next = at.iter().copied().peekable();
    for pair in points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let len = a.distance(b);
        while let Some(s) = next.next_if(|&s| s < along + len) {
            if s > along + 1e-9 {
                out.push((a.lerp(b, (s - along) / len), s));
            }
        }
        along += len;
        out.push((b, along));
    }
    out
}

impl Command for FlutingToolpath {
    const ID: &'static str = "fluting_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Fluting Toolpath",
            group: "Toolpath Operations",
            icon: "toolpaths/fluting_toolpath.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("flute depth", p.flute_depth)?;
        ensure_positive("tool diameter", p.tool.diameter)?;
        if p.start_depth < 0.0 {
            return Err(CommandError::InvalidParams(
                "start depth cannot be negative".into(),
            ));
        }
        if p.flute_type != FluteType::RampOverCompleteLength {
            if !p.use_ramp_percent {
                ensure_positive("ramp length", p.ramp_length)?;
            } else if !(p.ramp_percent > 0.0 && p.ramp_percent < 100.0) {
                return Err(CommandError::InvalidParams(
                    "ramp % must be greater than 0 and less than 100".into(),
                ));
            }
        }
        let (ids, paths) = doc.selected_paths(|_| true);
        if ids.is_empty() {
            return Err(CommandError::Failed("select the vectors to flute".into()));
        }
        let mut moves: Vec<_> = paths
            .iter()
            .flat_map(|path| {
                let mut points = cut_points(path);
                if path.closed {
                    points.push(points[0]);
                }
                let length: f64 = points.windows(2).map(|w| w[0].distance(w[1])).sum();
                if length <= 0.0 {
                    return vec![];
                }
                let ramp = p.ramp(length);
                let steps = (1..=RAMP_STEPS).map(|i| ramp * f64::from(i) / f64::from(RAMP_STEPS));
                let mut at: Vec<f64> = steps.clone().collect();
                if p.flute_type == FluteType::RampAtStartAndEnd {
                    at.extend(steps.map(|s| length - s));
                }
                at.sort_by(f64::total_cmp);
                let cut: Vec<_> = insert_at(&points, &at)
                    .into_iter()
                    .map(|(xy, s)| {
                        let depth = p.start_depth + p.flute_depth * p.fraction(s, length, ramp);
                        xy.extend(-depth)
                    })
                    .collect();
                follow(&cut, doc.job.clearance)
            })
            .collect();
        if p.project_onto_model {
            moves = project_onto_model(doc, &p.tool, &moves)?;
        }
        let mut c = Change::builder(doc);
        c.add_toolpath(made_by::<Self>(
            p,
            p.name.clone(),
            p.tool.clone(),
            ids,
            moves,
        ));
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Path;
    use cut_craft_core::testing::{made_toolpath, rejection, square};
    use cut_craft_core::toolpath::MoveKind;

    cut_craft_core::contract_tests!(FlutingToolpath);

    /// A 100 mm line along x from the origin.
    fn line() -> Path {
        Path::polyline(&[Point::new(0.0, 0.0), Point::new(100.0, 0.0)], false)
    }

    /// Depth (positive, below the surface) of the cut at each x along the line.
    fn depth_at(p: &Params, xs: &[f64]) -> Vec<f64> {
        let tp = made_toolpath::<FlutingToolpath>(vec![line()], p).unwrap();
        let cut: Vec<_> = tp
            .moves
            .iter()
            .filter(|m| m.kind != MoveKind::Rapid)
            .map(|m| m.to)
            .collect();
        xs.iter()
            .map(|&x| {
                let w = cut.windows(2).find(|w| w[0].x <= x && x <= w[1].x).unwrap();
                let t = (x - w[0].x) / (w[1].x - w[0].x);
                -(w[0].z + (w[1].z - w[0].z) * t)
            })
            .collect()
    }

    fn close(a: &[f64], b: &[f64]) -> bool {
        a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-6)
    }

    fn linear(flute_type: FluteType) -> Params {
        Params {
            start_depth: 1.0,
            flute_depth: 4.0,
            flute_type,
            ramp_type: RampType::Linear,
            ..Params::default()
        }
    }

    #[test]
    fn complete_length_ramps_from_the_start_depth_to_full_depth() {
        let d = depth_at(
            &linear(FluteType::RampOverCompleteLength),
            &[0.0, 50.0, 100.0],
        );
        assert!(close(&d, &[1.0, 3.0, 5.0]), "{d:?}");
    }

    #[test]
    fn ramp_at_start_by_percent_then_runs_at_full_depth() {
        let d = depth_at(&linear(FluteType::RampAtStart), &[0.0, 25.0, 50.0, 100.0]);
        assert!(close(&d, &[1.0, 3.0, 5.0, 5.0]), "{d:?}");
    }

    #[test]
    fn fifty_percent_at_both_ends_ramps_a_quarter_of_the_length_each() {
        let d = depth_at(
            &linear(FluteType::RampAtStartAndEnd),
            &[0.0, 12.5, 25.0, 75.0, 87.5, 100.0],
        );
        assert!(close(&d, &[1.0, 3.0, 5.0, 5.0, 3.0, 1.0]), "{d:?}");
    }

    #[test]
    fn a_ramp_length_too_long_is_capped_at_the_longest_ramp() {
        let p = Params {
            use_ramp_percent: false,
            ramp_length: 500.0,
            ..linear(FluteType::RampAtStartAndEnd)
        };
        let d = depth_at(&p, &[0.0, 25.0, 50.0, 100.0]);
        assert!(close(&d, &[1.0, 3.0, 5.0, 1.0]), "{d:?}");
    }

    #[test]
    fn a_ramp_length_sets_the_ramp_exactly() {
        let p = Params {
            use_ramp_percent: false,
            ramp_length: 20.0,
            ..linear(FluteType::RampAtStart)
        };
        let d = depth_at(&p, &[0.0, 10.0, 20.0, 60.0]);
        assert!(close(&d, &[1.0, 3.0, 5.0, 5.0]), "{d:?}");
    }

    #[test]
    fn a_smooth_ramp_curves_and_levels_into_full_depth() {
        let p = Params {
            ramp_type: RampType::Smooth,
            ..linear(FluteType::RampAtStart)
        };
        let d = depth_at(&p, &[0.0, 25.0, 49.0, 50.0]);
        assert!((d[0] - 1.0).abs() < 1e-6, "{d:?}");
        assert!(d[1] > 3.5, "deeper than a straight ramp halfway: {d:?}");
        assert!(
            d[3] - d[2] < 0.01,
            "nearly level at the end of the ramp: {d:?}"
        );
        assert!((d[3] - 5.0).abs() < 1e-6, "{d:?}");
    }

    #[test]
    fn starts_at_the_vector_start_and_never_cuts_above_the_surface() {
        let tp = made_toolpath::<FlutingToolpath>(vec![line()], &Params::default()).unwrap();
        let plunge = tp
            .moves
            .iter()
            .find(|m| m.kind == MoveKind::Plunge)
            .unwrap();
        assert_eq!(plunge.to.truncate(), Point::new(0.0, 0.0));
        assert!(
            tp.moves
                .iter()
                .filter(|m| m.kind != MoveKind::Rapid)
                .all(|m| m.to.z <= 1e-9)
        );
    }

    #[test]
    fn a_closed_vector_is_fluted_all_the_way_round() {
        let tp = made_toolpath::<FlutingToolpath>(vec![square(0.0, 0.0, 10.0)], &Params::default())
            .unwrap();
        let cut: Vec<_> = tp
            .moves
            .iter()
            .filter(|m| m.kind != MoveKind::Rapid)
            .collect();
        assert_eq!(
            cut.first().unwrap().to,
            cut.last().unwrap().to,
            "ends where it began"
        );
        assert!(
            cut.iter().any(|m| (m.to.z + 3.0).abs() < 1e-9),
            "reaches full depth"
        );
    }

    #[test]
    fn ramp_percent_must_be_between_0_and_100() {
        for pct in [0.0, 100.0] {
            let p = Params {
                ramp_percent: pct,
                ..Params::default()
            };
            assert_eq!(
                rejection::<FlutingToolpath>(&p),
                "invalid parameters: ramp % must be greater than 0 and less than 100"
            );
        }
    }

    #[test]
    fn projected_onto_the_model_it_flutes_below_its_surface() {
        let mut doc = cut_craft_core::testing::domed();
        let across = Path::polyline(&[Point::new(2.0, 20.0), Point::new(38.0, 20.0)], false);
        let change = Change::swap(&doc, [], vec![across]);
        doc.apply(&change).unwrap();
        let p = Params {
            project_onto_model: true,
            ..linear(FluteType::RampOverCompleteLength)
        };
        let change = FlutingToolpath::run(&doc, &p).unwrap();
        doc.apply(&change).unwrap();
        let tp = doc.toolpaths.into_values().next().unwrap();
        let plunge = tp
            .moves
            .iter()
            .find(|m| m.kind == MoveKind::Plunge)
            .unwrap();
        // the start depth below the flat round the 8 mm dome
        assert!((plunge.to.z + 9.0).abs() < 0.01, "{}", plunge.to.z);
        // over the top: 1 mm start + 4 mm flute halfway along, on the dome's top
        let top = tp
            .moves
            .iter()
            .find(|m| (m.to.x - 20.0).abs() < 0.2 && m.kind == MoveKind::Cut)
            .unwrap();
        assert!((top.to.z + 3.0).abs() < 0.1, "{}", top.to.z);
        assert!(tp.moves.iter().all(|m| m.to.z >= -10.0 - 1e-9));
    }

    #[test]
    fn needs_vectors_selected() {
        assert_eq!(
            made_toolpath::<FlutingToolpath>(vec![], &Params::default()).unwrap_err(),
            "select the vectors to flute"
        );
    }
}
