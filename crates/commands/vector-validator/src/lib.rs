//! Vector Validator: finds vectors that would trouble toolpaths or modelling (open,
//! crossing themselves, lying on top of another, or too small) and selects them.

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::geometry::{Path, Point, Segment};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct VectorValidator;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[expect(clippy::struct_excessive_bools, reason = "one tick box per check")]
pub struct Params {
    /// Open vectors: they cannot be pocketed, modelled or profiled inside or outside
    pub open: bool,
    /// Closed vectors that cross themselves
    pub self_intersecting: bool,
    /// Vectors lying on top of another one
    pub duplicates: bool,
    /// Vectors, or spans within them, shorter than the tolerance
    pub tiny: bool,
    /// How close counts as the same point
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub tolerance: f64,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            open: true,
            self_intersecting: true,
            duplicates: true,
            tiny: true,
            tolerance: 0.01,
        }
    }
}

impl Command for VectorValidator {
    const ID: &'static str = "vector_validator";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Vector Validator",
            group: "Edit Objects",
            icon: "drawing/create_vectors/vector_validator.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("tolerance", p.tolerance)?;
        // the selection when there is one, else every vector that can be edited
        let checked: Vec<EntityId> = if doc.selection.is_empty() {
            doc.vectors
                .keys()
                .copied()
                .filter(|&id| doc.editable(id))
                .collect()
        } else {
            doc.selection.iter().copied().collect()
        };
        let flat: Vec<(EntityId, Vec<Point>)> = checked
            .iter()
            .map(|&id| (id, doc.vectors[&id].flatten(p.tolerance)))
            .collect();
        let mut problems = vec![];
        for (i, (id, points)) in flat.iter().enumerate() {
            let path = &doc.vectors[id];
            let bad = (p.open && !path.closed)
                || (p.tiny && is_tiny(path, points, p.tolerance))
                || (p.self_intersecting && path.closed && crosses_itself(points))
                || (p.duplicates
                    && flat[..i]
                        .iter()
                        .any(|(_, other)| same_points(points, other, p.tolerance)));
            if bad {
                problems.push(*id);
            }
        }
        if problems.is_empty() {
            return Err(CommandError::Failed(format!(
                "no problems found in {} vector{}",
                checked.len(),
                if checked.len() == 1 { "" } else { "s" }
            )));
        }
        let mut c = Change::builder(doc);
        c.select(problems);
        Ok(c.finish())
    }
}

fn is_tiny(path: &Path, points: &[Point], tolerance: f64) -> bool {
    let size = path.bounds().size();
    size.x.max(size.y) < tolerance
        || std::iter::once(path.start)
            .chain(path.segments.iter().map(Segment::end))
            .collect::<Vec<_>>()
            .windows(2)
            .any(|w| (w[1] - w[0]).length() < tolerance)
        || points.len() < 2
}

/// Whether `a` and `b` trace the same points, in the same or the opposite direction.
fn same_points(a: &[Point], b: &[Point], tolerance: f64) -> bool {
    let close = |x: &Point, y: &Point| (*x - *y).length() <= tolerance;
    a.len() == b.len()
        && (a.iter().zip(b).all(|(x, y)| close(x, y))
            || a.iter().zip(b.iter().rev()).all(|(x, y)| close(x, y)))
}

/// Whether the closed ring through `points` crosses itself: two of its edges that are not
/// neighbours meet. Edges are swept left to right so only overlapping ones are compared.
fn crosses_itself(points: &[Point]) -> bool {
    let mut ring: Vec<Point> = points.to_vec();
    if ring.len() > 1 && ring.first() == ring.last() {
        ring.pop();
    }
    let n = ring.len();
    if n < 4 {
        return false;
    }
    let edge = |i: usize| (ring[i], ring[(i + 1) % n]);
    let mut order: Vec<usize> = (0..n).collect();
    let left = |i: usize| ring[i].x.min(ring[(i + 1) % n].x);
    let right = |i: usize| ring[i].x.max(ring[(i + 1) % n].x);
    order.sort_by(|&a, &b| left(a).total_cmp(&left(b)));
    for (k, &i) in order.iter().enumerate() {
        for &j in &order[k + 1..] {
            if left(j) > right(i) {
                break;
            }
            let neighbours = (i + 1) % n == j || (j + 1) % n == i;
            if !neighbours && edges_cross(edge(i), edge(j)) {
                return true;
            }
        }
    }
    false
}

fn edges_cross((a, b): (Point, Point), (c, d): (Point, Point)) -> bool {
    let side = |p: Point, q: Point, r: Point| (q - p).perp_dot(r - p);
    let (d1, d2) = (side(c, d, a), side(c, d, b));
    let (d3, d4) = (side(a, b, c), side(a, b, d));
    let opposite = |x: f64, y: f64| (x > 0.0 && y < 0.0) || (x < 0.0 && y > 0.0);
    opposite(d1, d2) && opposite(d3, d4)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{document_with, square};

    cut_craft_core::contract_tests!(VectorValidator);

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    /// Which of `paths` (by position) the validator picks, with nothing selected first.
    fn found(paths: Vec<Path>, params: &Params) -> Result<Vec<usize>, String> {
        let mut doc = document_with(paths);
        let ids: Vec<EntityId> = doc.vectors.keys().copied().collect();
        let mut c = Change::builder(&doc);
        c.select([]);
        doc.apply(&c.finish()).unwrap();
        let change = VectorValidator::run(&doc, params).map_err(|e| e.to_string())?;
        doc.apply(&change).unwrap();
        Ok(doc
            .selection
            .iter()
            .map(|id| ids.iter().position(|x| x == id).unwrap())
            .collect())
    }

    fn bow_tie() -> Path {
        Path::polyline(
            &[p(0.0, 0.0), p(10.0, 10.0), p(10.0, 0.0), p(0.0, 10.0)],
            true,
        )
    }

    #[test]
    fn finds_each_kind_of_problem() {
        let open = Path::polyline(&[p(50.0, 0.0), p(60.0, 0.0)], false);
        let speck = square(70.0, 70.0, 0.001);
        let paths = vec![
            square(0.0, 0.0, 10.0),
            open,
            bow_tie(),
            square(30.0, 30.0, 5.0),
            square(30.0, 30.0, 5.0).reversed(),
            speck,
        ];
        assert_eq!(found(paths, &Params::default()).unwrap(), [1, 2, 4, 5]);
    }

    #[test]
    fn each_check_can_be_turned_off() {
        let only = |open, self_intersecting, duplicates, tiny| Params {
            open,
            self_intersecting,
            duplicates,
            tiny,
            ..Params::default()
        };
        let paths = || {
            vec![
                Path::polyline(&[p(50.0, 0.0), p(60.0, 0.0)], false),
                bow_tie(),
                square(30.0, 30.0, 5.0),
                square(30.0, 30.0, 5.0),
            ]
        };
        assert_eq!(
            found(paths(), &only(true, false, false, false)).unwrap(),
            [0]
        );
        assert_eq!(
            found(paths(), &only(false, true, false, false)).unwrap(),
            [1]
        );
        assert_eq!(
            found(paths(), &only(false, false, true, false)).unwrap(),
            [3]
        );
    }

    #[test]
    fn crossing_is_told_from_touching_and_curves_are_checked_too() {
        assert!(crosses_itself(&bow_tie().flatten(0.01)));
        assert!(!crosses_itself(&square(0.0, 0.0, 10.0).flatten(0.01)));
        let circle = Path::circle(p(0.0, 0.0), 5.0);
        assert!(!crosses_itself(&circle.flatten(0.01)));
    }

    #[test]
    fn a_clean_job_is_reported_not_selected() {
        let err = found(vec![square(0.0, 0.0, 10.0)], &Params::default()).unwrap_err();
        assert!(err.contains("no problems found in 1 vector"), "{err}");
    }

    #[test]
    fn checks_only_the_selection_when_there_is_one() {
        // document_with selects what it adds; add a bow tie after, unselected
        let mut doc = document_with(vec![square(0.0, 0.0, 10.0)]);
        let mut c = Change::builder(&doc);
        c.add(bow_tie());
        doc.apply(&c.finish()).unwrap();
        assert!(VectorValidator::run(&doc, &Params::default()).is_err());
    }
}
