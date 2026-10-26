//! Vector Unwrapper (rotary jobs): a closed cross-section of the turned shape becomes the
//! open profile Two Rail Sweep needs. Round the chosen centre, the distance out to the
//! cross-section at each angle is laid out along the job's circumference, with legs down to
//! its baseline at each end; two drive rails along the job's sides go on the "Unwrapped
//! Vectors Drive Rails" layer. The rails and the profile are left selected in the order Two
//! Rail Sweep takes them.

use cut_craft_core::document::{Along, Document, Layer};
use cut_craft_core::geometry::{Path, Point};
use cut_craft_core::params::Xy;
use cut_craft_core::trace::{self, Fit};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct VectorUnwrapper;

/// Where the rotation axis crosses the cross-section.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Centre {
    /// The middle of the cross-section's bounding box
    #[default]
    #[schemars(title = "Use center of contour")]
    Contour,
    /// The middle of the job
    #[schemars(title = "Use center of workspace")]
    Workspace,
    /// The point below
    #[schemars(title = "Use custom point")]
    Custom,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    #[schemars(title = "Center of Rotation")]
    pub centre: Centre,
    /// Use custom point only
    pub point: Xy,
    /// Fit curves through the profile so it has far fewer nodes
    #[schemars(title = "Refine unwrapped vectors")]
    pub refine: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            centre: Centre::Contour,
            point: Xy::default(),
            refine: true,
        }
    }
}

/// The layer the drive rails go on, as Aspire names it.
const RAILS_LAYER: &str = "Unwrapped Vectors Drive Rails";
/// Angles sampled round the cross-section.
// ponytail: every half degree [inf]
const SAMPLES: usize = 720;
/// How closely Refine fits the profile, mm.
const REFINE_TOLERANCE: f64 = 0.01;

/// How far out from `centre` the cross-section reaches in direction `d`: the farthest
/// crossing of the ray with its edges, or 0 when the ray misses it.
fn reach(ring: &[Point], centre: Point, d: Point) -> f64 {
    let mut far: f64 = 0.0;
    for (i, &a) in ring.iter().enumerate() {
        let b = ring[(i + 1) % ring.len()];
        let edge = b - a;
        let det = d.perp_dot(edge);
        if det.abs() < 1e-12 {
            continue;
        }
        let to_a = a - centre;
        let along_ray = to_a.perp_dot(edge) / det;
        let along_edge = to_a.perp_dot(d) / det;
        if along_ray >= 0.0 && (0.0..=1.0).contains(&along_edge) {
            far = far.max(along_ray);
        }
    }
    far
}

impl Command for VectorUnwrapper {
    const ID: &'static str = "vector_unwrapper";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Vector Unwrapper",
            group: "Modeling",
            icon: "",
            shortcut: "",
        }
    }

    #[expect(clippy::cast_precision_loss, reason = "a small sample count")]
    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let rotary = doc.job.rotary.ok_or_else(|| {
            CommandError::Failed("the Vector Unwrapper is for rotary jobs only".into())
        })?;
        let section = match doc.selection.iter().collect::<Vec<_>>().as_slice() {
            [one] if doc.vectors[one].closed => &doc.vectors[one],
            _ => {
                return Err(CommandError::Failed(
                    "select one closed vector: the cross-section to unwrap".into(),
                ));
            }
        };
        let job = doc.job.area();
        let centre = match p.centre {
            Centre::Contour => section.bounds().center(),
            Centre::Workspace => job.center(),
            Centre::Custom => p.point.into(),
        };
        let ring = {
            let mut ring = section.flatten(REFINE_TOLERANCE);
            if ring.len() > 1 && ring[0].distance(ring[ring.len() - 1]) < 1e-9 {
                ring.pop();
            }
            ring
        };
        // the circumference runs across the axis: up the job for an X axis, along it for Y.
        // Angle 0 is straight up the cross-section, turning clockwise [inf]
        let (start, round, out) = match rotary.along {
            Along::X => (
                Point::new(job.center().x, job.min.y),
                Point::new(0.0, job.size().y),
                Point::X,
            ),
            Along::Y => (
                Point::new(job.min.x, job.center().y),
                Point::new(job.size().x, 0.0),
                Point::Y,
            ),
        };
        let profile: Vec<Point> = (0..=SAMPLES)
            .map(|i| {
                let t = i as f64 / SAMPLES as f64;
                let angle = t * std::f64::consts::TAU;
                let d = Point::new(angle.sin(), angle.cos());
                start + round * t + out * reach(&ring, centre, d)
            })
            .collect();
        // the legs down to the baseline at both ends set the profile's height for the sweep
        let points: Vec<Point> = std::iter::once(start)
            .chain(profile)
            .chain(std::iter::once(start + round))
            .collect();
        let path = if p.refine {
            trace::fit(
                &points,
                false,
                Fit::Beziers,
                REFINE_TOLERANCE,
                30_f64.to_radians(),
            )
        } else {
            Path::polyline(&points, false)
        };
        let (rail_1, rail_2) = match rotary.along {
            Along::X => (
                [job.min, Point::new(job.max.x, job.min.y)],
                [Point::new(job.min.x, job.max.y), job.max],
            ),
            Along::Y => (
                [job.min, Point::new(job.min.x, job.max.y)],
                [Point::new(job.max.x, job.min.y), job.max],
            ),
        };
        let mut c = Change::builder(doc);
        let profile = c.add(path);
        let layer = doc
            .layers
            .iter()
            .find(|l| l.name == RAILS_LAYER)
            .map_or_else(|| c.add_layer(RAILS_LAYER), |l: &Layer| l.id);
        let mut rails = vec![];
        for ends in [rail_1, rail_2] {
            let id = c.add(Path::polyline(&ends, false));
            c.move_to_layer(id, layer);
            rails.push(id);
        }
        c.select([rails[0], rails[1], profile]);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::document::{EntityId, Job};
    use cut_craft_core::testing::{rejection, run_on};

    /// A rotary job along X, 100 long, 40 across, with a closed `section` selected.
    fn rotary_with(section: Path) -> Document {
        let mut doc = Document {
            job: Job::rotary(100.0, 40.0, Along::X),
            ..Document::default()
        };
        let mut c = Change::builder(&doc);
        let id = c.add(section);
        c.select([id]);
        doc.apply(&c.finish()).unwrap();
        doc
    }

    fn circle() -> Document {
        rotary_with(Path::circle(Point::new(50.0, 60.0), 15.0))
    }

    cut_craft_core::contract_tests!(VectorUnwrapper, circle);

    fn profile(doc: &Document) -> Vec<Point> {
        let picked: Vec<EntityId> = doc.selection.iter().copied().collect();
        doc.vectors[&picked[2]].flatten(0.01)
    }

    #[test]
    fn a_circle_unwraps_to_a_flat_profile_at_its_radius() {
        let mut doc = circle();
        run_on::<VectorUnwrapper>(&mut doc, &Params::default());
        let job = doc.job.area();
        let base = job.center().x;
        let points = profile(&doc);
        assert!(
            (points[0] - Point::new(base, job.min.y)).length() < 1e-6,
            "a leg starts it"
        );
        let middle: Vec<_> = points
            .iter()
            .filter(|q| q.y > job.min.y + 1.0 && q.y < job.max.y - 1.0)
            .collect();
        assert!(
            middle.iter().all(|q| (q.x - base - 15.0).abs() < 0.05),
            "radius 15 all round"
        );
    }

    #[test]
    fn a_square_reaches_further_at_its_corners() {
        let mut doc = rotary_with(cut_craft_core::testing::square(40.0, 50.0, 20.0));
        run_on::<VectorUnwrapper>(&mut doc, &Params::default());
        let job = doc.job.area();
        let base = job.center().x;
        let height_at = |turn: f64| {
            let y = job.min.y + job.size().y * turn;
            profile(&doc)
                .into_iter()
                .min_by(|a, b| (a.y - y).abs().total_cmp(&(b.y - y).abs()))
                .unwrap()
                .x
                - base
        };
        assert!(
            (height_at(0.01) - 10.0).abs() < 0.1,
            "straight up: the side's middle"
        );
        assert!(
            (height_at(0.125) - 10.0 * 2_f64.sqrt()).abs() < 0.1,
            "45 degrees: a corner"
        );
    }

    #[test]
    fn the_rails_go_on_their_layer_and_two_rail_sweep_order_is_selected() {
        let mut doc = circle();
        run_on::<VectorUnwrapper>(&mut doc, &Params::default());
        let picked: Vec<EntityId> = doc.selection.iter().copied().collect();
        assert_eq!(picked.len(), 3);
        let layer = doc
            .layers
            .iter()
            .find(|l| l.name == RAILS_LAYER)
            .unwrap()
            .id;
        assert_eq!(doc.vectors_on(layer).count(), 2);
        let job = doc.job.area();
        assert!((doc.vectors[&picked[0]].bounds().size().x - job.size().x).abs() < 1e-9);
    }

    #[test]
    fn needs_a_rotary_job_and_one_closed_vector() {
        let mut flat = circle();
        flat.job = Job::default();
        assert!(VectorUnwrapper::run(&flat, &Params::default()).is_err());
        assert!(rejection::<VectorUnwrapper>(&Params::default()).contains("rotary"));
    }
}

#[cfg(test)]
mod sweep {
    use super::*;
    use cut_craft_core::component::model;
    use cut_craft_core::document::Job;
    use cut_craft_core::testing::run_on;

    #[test]
    fn swept_between_its_rails_a_circle_makes_a_cylinder_of_its_radius() {
        let mut doc = Document {
            job: Job::rotary(100.0, 40.0, Along::X),
            ..Document::default()
        };
        let mut c = Change::builder(&doc);
        let id = c.add(Path::circle(Point::new(50.0, 60.0), 15.0));
        c.select([id]);
        doc.apply(&c.finish()).unwrap();
        run_on::<VectorUnwrapper>(&mut doc, &Params::default());
        run_on::<cmd_two_rail_sweep::TwoRailSweep>(
            &mut doc,
            &cmd_two_rail_sweep::Params::default(),
        );
        let surface = model(&doc, 200);
        let job = doc.job.area();
        for turn in [0.2, 0.5, 0.8] {
            let at = Point::new(job.center().x, job.min.y + job.size().y * turn);
            let z = surface.at(at).unwrap();
            assert!((z - 15.0).abs() < 0.3, "{turn}: {z}");
        }
    }
}
