//! Copy Along Vectors: copies of the selected objects spaced along a vector. The last vector
//! picked is the one to copy along; everything picked before it is copied.

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::geometry::Point;
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use glam::DAffine2;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct CopyAlongVectors;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Spacing {
    /// Copies, spread evenly from end to end (all round, on a closed vector)
    #[default]
    Count,
    /// Distance along the vector from one copy to the next, starting at its start
    Distance,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub spacing: Spacing,
    /// How many copies, when spacing by count
    #[schemars(range(min = 1))]
    pub copies: u32,
    /// Between copies, when spacing by distance
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub distance: f64,
    /// Turn each copy to follow the vector, as if the objects travelled along it facing +X
    pub align_to_vector: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            spacing: Spacing::Count,
            copies: 5,
            distance: 20.0,
            align_to_vector: true,
        }
    }
}

/// How finely the vector to copy along is followed.
// ponytail: fixed; copies land within this of the true curve
const FOLLOW: f64 = 0.01;

impl Command for CopyAlongVectors {
    const ID: &'static str = "copy_along_vectors";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Copy Along Vectors",
            group: "Offset and Layout",
            icon: "drawing/create_vectors/copy_along_vectors.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let picked: Vec<EntityId> = doc.selection.iter().copied().collect();
        let Some((&along, objects)) = picked.split_last().filter(|(_, rest)| !rest.is_empty())
        else {
            return Err(CommandError::Failed(
                "select the objects to copy, then last the vector to copy them along".into(),
            ));
        };
        let guide = doc.vectors[&along].clone();
        let mut points = guide.flatten(FOLLOW);
        if guide.closed && points.first() != points.last() {
            points.push(points[0]);
        }
        let walk = Walk::new(points);
        if walk.length <= 0.0 {
            return Err(CommandError::Failed(
                "the vector to copy along has no length".into(),
            ));
        }
        let distances: Vec<f64> = match p.spacing {
            Spacing::Count => {
                if p.copies == 0 {
                    return Err(CommandError::InvalidParams(
                        "copies must be at least 1".into(),
                    ));
                }
                let n = f64::from(p.copies);
                // a closed vector's end is its start: share the length out all round
                let gaps = if guide.closed || p.copies == 1 {
                    n
                } else {
                    n - 1.0
                };
                (0..p.copies)
                    .map(|i| walk.length * f64::from(i) / gaps)
                    .collect()
            }
            Spacing::Distance => {
                ensure_positive("distance", p.distance)?;
                let mut at = vec![];
                let mut d = 0.0;
                while d <= walk.length + 1e-9 && at.len() < MAX_COPIES {
                    at.push(d);
                    d += p.distance;
                }
                at
            }
        };
        let middle = objects
            .iter()
            .map(|id| doc.vectors[id].bounds())
            .reduce(cut_craft_core::geometry::Rect::union)
            .expect("at least one object")
            .center();
        let transforms: Vec<DAffine2> = distances
            .into_iter()
            .map(|d| {
                let (point, heading) = walk.at(d);
                let turn = if p.align_to_vector {
                    DAffine2::from_angle(heading)
                } else {
                    DAffine2::IDENTITY
                };
                DAffine2::from_translation(point) * turn * DAffine2::from_translation(-middle)
            })
            .collect();
        Ok(Change::copy_vectors(doc, objects, &transforms))
    }
}

/// A distance step so small next to the vector would make more copies than anyone means.
const MAX_COPIES: usize = 10_000;

/// Positions along a polyline by distance from its start.
struct Walk {
    points: Vec<Point>,
    /// Distance from the start to each point.
    at: Vec<f64>,
    length: f64,
}

impl Walk {
    fn new(points: Vec<Point>) -> Self {
        let mut at = vec![0.0];
        for w in points.windows(2) {
            at.push(at[at.len() - 1] + (w[1] - w[0]).length());
        }
        let length = at[at.len() - 1];
        Self { points, at, length }
    }

    /// The point `d` along, and the heading (radians) of the edge it is on.
    fn at(&self, d: f64) -> (Point, f64) {
        let d = d.clamp(0.0, self.length);
        // the first edge ending at or past `d` that has some length
        let edge = (1..self.points.len())
            .find(|&i| self.at[i] >= d && self.at[i] > self.at[i - 1])
            .unwrap_or(self.points.len() - 1);
        let (a, b) = (self.points[edge - 1], self.points[edge]);
        let span = self.at[edge] - self.at[edge - 1];
        let t = if span > 0.0 {
            (d - self.at[edge - 1]) / span
        } else {
            0.0
        };
        (a.lerp(b, t), (b - a).to_angle())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Path;
    use cut_craft_core::testing::{document_with, square};

    cut_craft_core::contract_tests!(CopyAlongVectors);

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    /// A 4 wide, 2 high bar at the origin.
    fn bar() -> Path {
        Path::polyline(&[p(0.0, 0.0), p(4.0, 0.0), p(4.0, 2.0), p(0.0, 2.0)], true)
    }

    /// (centre, size) of each copy, rounded to µm; the object and the guide are left out.
    fn copies(object: Path, along: Path, params: &Params) -> Vec<((f64, f64), (f64, f64))> {
        let mut doc = document_with(vec![object, along]);
        doc.apply(&CopyAlongVectors::run(&doc, params).unwrap())
            .unwrap();
        let round = |v: f64| (v * 1000.0).round() / 1000.0 + 0.0;
        doc.vectors
            .values()
            .skip(2)
            .map(|path| {
                let b = path.bounds();
                let (c, s) = (b.center(), b.size());
                ((round(c.x), round(c.y)), (round(s.x), round(s.y)))
            })
            .collect()
    }

    #[test]
    fn a_count_spreads_copies_from_end_to_end() {
        let line = Path::polyline(&[p(10.0, 0.0), p(50.0, 0.0)], false);
        let placed = copies(bar(), line, &Params::default());
        let centres: Vec<_> = placed.iter().map(|(c, _)| *c).collect();
        assert_eq!(
            centres,
            [
                (10.0, 0.0),
                (20.0, 0.0),
                (30.0, 0.0),
                (40.0, 0.0),
                (50.0, 0.0)
            ]
        );
        assert!(placed.iter().all(|(_, size)| *size == (4.0, 2.0)));
    }

    #[test]
    fn aligned_copies_turn_with_the_vector() {
        let up = || Path::polyline(&[p(0.0, 10.0), p(0.0, 50.0)], false);
        let aligned = copies(bar(), up(), &Params::default());
        assert!(aligned.iter().all(|(_, size)| *size == (2.0, 4.0)));
        let straight = copies(
            bar(),
            up(),
            &Params {
                align_to_vector: false,
                ..Params::default()
            },
        );
        assert!(straight.iter().all(|(_, size)| *size == (4.0, 2.0)));
    }

    #[test]
    fn a_closed_vector_shares_the_copies_out_all_round() {
        let ring = square(100.0, 100.0, 10.0);
        let placed = copies(
            bar(),
            ring,
            &Params {
                copies: 4,
                align_to_vector: false,
                ..Params::default()
            },
        );
        let centres: Vec<_> = placed.iter().map(|(c, _)| *c).collect();
        assert_eq!(
            centres,
            [
                (100.0, 100.0),
                (110.0, 100.0),
                (110.0, 110.0),
                (100.0, 110.0)
            ]
        );
    }

    #[test]
    fn a_distance_steps_from_the_start_while_the_vector_lasts() {
        let line = Path::polyline(&[p(0.0, 0.0), p(40.0, 0.0)], false);
        let placed = copies(
            bar(),
            line,
            &Params {
                spacing: Spacing::Distance,
                distance: 15.0,
                ..Params::default()
            },
        );
        let xs: Vec<f64> = placed.iter().map(|((x, _), _)| *x).collect();
        assert_eq!(xs, [0.0, 15.0, 30.0]);
    }

    #[test]
    fn needs_objects_then_a_vector() {
        let doc = document_with(vec![bar()]);
        assert!(CopyAlongVectors::run(&doc, &Params::default()).is_err());
        let doc = document_with(vec![
            bar(),
            Path::polyline(&[p(5.0, 5.0), p(5.0, 5.0)], false),
        ]);
        let err = CopyAlongVectors::run(&doc, &Params::default())
            .map(|_| ())
            .unwrap_err()
            .to_string();
        assert!(err.contains("no length"), "{err}");
    }
}
