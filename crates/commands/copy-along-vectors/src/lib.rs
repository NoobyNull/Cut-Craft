//! Copy Along Vectors: copies of an object spaced along vectors. The first object picked
//! (with its group) is the one copied; every other selected vector is a path to copy it
//! along.

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::geometry::{Point, Rect};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use glam::DAffine2;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct CopyAlongVectors;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Spacing {
    /// A number of copies, spread evenly from end to end (all round, on a closed vector)
    #[default]
    Count,
    /// Distance along the vector from one copy to the next, starting at its start
    Distance,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[expect(clippy::struct_excessive_bools, reason = "one tick box per option")]
pub struct Params {
    pub spacing: Spacing,
    /// How many copies on each vector, when spacing by count
    #[schemars(range(min = 1))]
    pub copies: u32,
    /// Between copies, when spacing by distance
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub distance: f64,
    /// Stretch or shrink the distance a little so copies land on both ends
    pub force_even_spacing: bool,
    /// Turn each copy to follow the vector, as if the object travelled along it facing +X
    pub align_to_vector: bool,
    /// Run along each vector from its end instead of its start
    pub reverse_direction: bool,
    /// Put the copies on a new layer of their own
    pub create_copies_on_new_layer: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            spacing: Spacing::Count,
            copies: 5,
            distance: 20.0,
            force_even_spacing: false,
            align_to_vector: true,
            reverse_direction: false,
            create_copies_on_new_layer: false,
        }
    }
}

/// How finely the vectors to copy along are followed.
// ponytail: fixed; copies land within this of the true curve
const FOLLOW: f64 = 0.01;

impl Command for CopyAlongVectors {
    const ID: &'static str = "copy_along_vectors";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Copy Along Vectors",
            group: "Offset and Layout",
            icon: "drawing/create_vectors/copy_along_vectors.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let picked: Vec<EntityId> = doc.selection.iter().copied().collect();
        let object = picked
            .first()
            .map(|&first| doc.group_members(first))
            .unwrap_or_default();
        let paths: Vec<EntityId> = picked
            .iter()
            .copied()
            .filter(|id| !object.contains(id))
            .collect();
        if object.is_empty() || paths.is_empty() {
            return Err(CommandError::Failed(
                "select the object to copy first, then the vectors to copy it along".into(),
            ));
        }
        let middle = object
            .iter()
            .map(|id| doc.vectors[id].bounds())
            .reduce(Rect::union)
            .expect("at least one object")
            .center();
        let mut transforms = vec![];
        for id in &paths {
            let guide = &doc.vectors[id];
            let mut points = guide.flatten(FOLLOW);
            if guide.closed && points.first() != points.last() {
                points.push(points[0]);
            }
            if p.reverse_direction {
                points.reverse();
            }
            let walk = Walk::new(points);
            if walk.length <= 0.0 {
                return Err(CommandError::Failed(
                    "a vector to copy along has no length".into(),
                ));
            }
            for d in distances(p, walk.length, guide.closed)? {
                let (point, heading) = walk.at(d);
                let turn = if p.align_to_vector {
                    DAffine2::from_angle(heading)
                } else {
                    DAffine2::IDENTITY
                };
                transforms.push(
                    DAffine2::from_translation(point) * turn * DAffine2::from_translation(-middle),
                );
            }
        }
        if !p.create_copies_on_new_layer {
            return Ok(Change::copy_vectors(doc, &object, &transforms));
        }
        let mut c = Change::builder(doc);
        let layer = c.add_layer("Copy Along Vectors");
        let mut selected = object.clone();
        for t in &transforms {
            for id in &object {
                let copy = c.add(doc.vectors[id].transformed(t));
                c.move_to_layer(copy, layer);
                selected.push(copy);
            }
        }
        c.select(selected);
        Ok(c.finish())
    }
}

/// Distances along a vector `length` long at which copies go.
fn distances(p: &Params, length: f64, closed: bool) -> Result<Vec<f64>, CommandError> {
    // a closed vector's end is its start: one copy fewer than the gaps
    let spread = |gaps: u32| -> Vec<f64> {
        let copies = if closed { gaps } else { gaps + 1 };
        (0..copies)
            .map(|i| length * f64::from(i) / f64::from(gaps.max(1)))
            .collect()
    };
    match p.spacing {
        Spacing::Count => {
            if p.copies == 0 {
                return Err(CommandError::InvalidParams(
                    "copies must be at least 1".into(),
                ));
            }
            Ok(match (p.copies, closed) {
                (1, _) => vec![0.0],
                (n, true) => spread(n),
                (n, false) => spread(n - 1),
            })
        }
        Spacing::Distance => {
            ensure_positive("distance", p.distance)?;
            if p.force_even_spacing {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "a positive count, capped"
                )]
                let gaps = ((length / p.distance).round().max(1.0) as u32).min(MAX_COPIES);
                return Ok(spread(gaps));
            }
            let mut at = vec![];
            let mut d = 0.0;
            // on a closed vector the copy at its end would sit on the one at its start
            while (d < length - 1e-9 || (!closed && d <= length + 1e-9))
                && at.len() < MAX_COPIES as usize
            {
                at.push(d);
                d += p.distance;
            }
            Ok(at)
        }
    }
}

/// A distance step so small next to the vector would make more copies than anyone means.
const MAX_COPIES: u32 = 10_000;

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
    fn force_even_spacing_lands_copies_on_both_ends() {
        let line = Path::polyline(&[p(0.0, 0.0), p(40.0, 0.0)], false);
        let placed = copies(
            bar(),
            line,
            &Params {
                spacing: Spacing::Distance,
                distance: 15.0,
                force_even_spacing: true,
                ..Params::default()
            },
        );
        // 40 / 15 rounds to 3 gaps of 13.333
        let xs: Vec<f64> = placed.iter().map(|((x, _), _)| *x).collect();
        assert_eq!(xs, [0.0, 13.333, 26.667, 40.0]);
    }

    #[test]
    fn reversing_runs_from_the_end() {
        let line = Path::polyline(&[p(10.0, 0.0), p(50.0, 0.0)], false);
        let placed = copies(
            bar(),
            line,
            &Params {
                copies: 3,
                reverse_direction: true,
                ..Params::default()
            },
        );
        let xs: Vec<f64> = placed.iter().map(|((x, _), _)| *x).collect();
        assert_eq!(xs, [50.0, 30.0, 10.0]);
    }

    #[test]
    fn every_other_selected_vector_is_a_path_and_new_layer_is_offered() {
        let lines = || {
            vec![
                bar(),
                Path::polyline(&[p(0.0, 10.0), p(20.0, 10.0)], false),
                Path::polyline(&[p(0.0, 20.0), p(20.0, 20.0)], false),
            ]
        };
        let mut doc = document_with(lines());
        let two = Params {
            copies: 2,
            ..Params::default()
        };
        doc.apply(&CopyAlongVectors::run(&doc, &two).unwrap())
            .unwrap();
        assert_eq!(doc.vectors.len(), 3 + 4, "two copies on each of two paths");
        let mut doc = document_with(lines());
        let layers = doc.layers.len();
        let apart = Params {
            create_copies_on_new_layer: true,
            ..two
        };
        doc.apply(&CopyAlongVectors::run(&doc, &apart).unwrap())
            .unwrap();
        assert_eq!(doc.layers.len(), layers + 1);
        let new_layer = doc.layers.last().unwrap().id;
        let copies: Vec<_> = doc.vectors.keys().skip(3).copied().collect();
        assert_eq!(copies.len(), 4);
        assert!(copies.iter().all(|id| doc.layer_of(*id) == new_layer));
    }

    #[test]
    fn needs_an_object_then_a_vector() {
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
