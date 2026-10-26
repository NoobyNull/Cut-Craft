//! Vector Validator: finds vectors that would stop toolpaths being made (overlapping,
//! crossing themselves or each other, or with zero-length spans) and selects them, or fixes
//! the zero-length spans.

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::geometry::{Path, Point, Segment};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct VectorValidator;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[expect(clippy::struct_excessive_bools, reason = "one tick box per check")]
pub struct Params {
    /// Vectors lying on top of another one
    pub overlapping: bool,
    /// Vectors crossing themselves or each other
    pub intersections: bool,
    /// Spans, or whole vectors, shorter than the tolerance
    pub zero_length_spans: bool,
    /// Open vectors: they cannot be pocketed or modelled (not one of Aspire's checks)
    pub open: bool,
    /// Instead of selecting problems, remove the zero-length spans from the vectors checked
    pub fix_zero_length_spans: bool,
    /// How close counts as the same point
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub tolerance: f64,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            overlapping: true,
            intersections: true,
            zero_length_spans: true,
            open: false,
            fix_zero_length_spans: false,
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
        // the selection when there is one, else every vector on a visible, unlocked layer
        let checked: Vec<EntityId> = if doc.selection.is_empty() {
            doc.vectors
                .keys()
                .copied()
                .filter(|&id| doc.editable(id))
                .collect()
        } else {
            doc.selection.iter().copied().collect()
        };
        if p.fix_zero_length_spans {
            return fix_spans(doc, &checked, p.tolerance);
        }
        let flat: Vec<Vec<Point>> = checked
            .iter()
            .map(|id| doc.vectors[id].flatten(p.tolerance))
            .collect();
        let mut bad = vec![false; checked.len()];
        if p.intersections {
            let closed: Vec<bool> = checked.iter().map(|id| doc.vectors[id].closed).collect();
            for owner in crossing(&flat, &closed) {
                bad[owner] = true;
            }
        }
        for (i, id) in checked.iter().enumerate() {
            let path = &doc.vectors[id];
            bad[i] |= (p.open && !path.closed)
                || (p.zero_length_spans && has_zero_length_span(path, p.tolerance))
                || (p.overlapping
                    && flat[..i]
                        .iter()
                        .any(|other| same_points(&flat[i], other, p.tolerance)));
        }
        let problems: Vec<EntityId> = checked
            .iter()
            .zip(&bad)
            .filter(|(_, bad)| **bad)
            .map(|(id, _)| *id)
            .collect();
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

fn ends(path: &Path) -> Vec<Point> {
    std::iter::once(path.start)
        .chain(path.segments.iter().map(Segment::end))
        .collect()
}

fn has_zero_length_span(path: &Path, tolerance: f64) -> bool {
    let size = path.bounds().size();
    size.x.max(size.y) < tolerance
        || ends(path)
            .windows(2)
            .any(|w| (w[1] - w[0]).length() < tolerance)
}

/// Removes every span shorter than `tolerance` from the `checked` vectors.
fn fix_spans(doc: &Document, checked: &[EntityId], tolerance: f64) -> Result<Change, CommandError> {
    let mut c = Change::builder(doc);
    let mut fixed = 0;
    for id in checked {
        let path = &doc.vectors[id];
        let mut segments: Vec<Segment> = vec![];
        let mut from = path.start;
        for segment in &path.segments {
            if (segment.end() - from).length() >= tolerance {
                segments.push(*segment);
                from = segment.end();
            }
        }
        // too little left to be a vector: leave it for the person to delete
        if segments.len() == path.segments.len() || segments.is_empty() {
            continue;
        }
        if path.closed {
            // the last span kept now ends where the dropped ones led: back on the start
            match segments.last_mut().expect("not empty") {
                Segment::Line { to } | Segment::Arc { to, .. } | Segment::Cubic { to, .. } => {
                    *to = path.start;
                }
            }
        }
        c.replace(
            *id,
            Path {
                start: path.start,
                segments,
                closed: path.closed,
            },
        );
        fixed += 1;
    }
    if fixed == 0 {
        return Err(CommandError::Failed(
            "no zero-length spans to fix in the vectors checked".into(),
        ));
    }
    Ok(c.finish())
}

/// Whether `a` and `b` trace the same points, in the same or the opposite direction.
fn same_points(a: &[Point], b: &[Point], tolerance: f64) -> bool {
    let close = |x: &Point, y: &Point| (*x - *y).length() <= tolerance;
    a.len() == b.len()
        && (a.iter().zip(b).all(|(x, y)| close(x, y))
            || a.iter().zip(b.iter().rev()).all(|(x, y)| close(x, y)))
}

/// One straight edge of a flattened vector.
struct Edge {
    owner: usize,
    index: usize,
    a: Point,
    b: Point,
}

/// Which of the polylines `flat` cross themselves or another: two of their edges meet that
/// are not neighbours along one polyline. Edges are swept left to right so only ones that
/// overlap in x are compared.
fn crossing(flat: &[Vec<Point>], closed: &[bool]) -> Vec<usize> {
    let mut edges = vec![];
    let mut count = vec![0; flat.len()];
    for (owner, points) in flat.iter().enumerate() {
        let mut ring = points.clone();
        if closed[owner] && ring.len() > 1 && ring.first() == ring.last() {
            ring.pop();
        }
        let n = ring.len();
        let spans = if closed[owner] {
            n
        } else {
            n.saturating_sub(1)
        };
        count[owner] = n;
        for index in 0..spans {
            let (a, b) = (ring[index], ring[(index + 1) % n]);
            edges.push(Edge { owner, index, a, b });
        }
    }
    let left = |e: &Edge| e.a.x.min(e.b.x);
    let right = |e: &Edge| e.a.x.max(e.b.x);
    edges.sort_by(|e, f| left(e).total_cmp(&left(f)));
    let mut found = vec![false; flat.len()];
    for (k, e) in edges.iter().enumerate() {
        for f in &edges[k + 1..] {
            if left(f) > right(e) {
                break;
            }
            if e.owner == f.owner {
                let n = count[e.owner];
                let next = |i: usize| (i + 1) % n;
                if next(e.index) == f.index || next(f.index) == e.index {
                    continue;
                }
            }
            if edges_cross((e.a, e.b), (f.a, f.b)) {
                found[e.owner] = true;
                found[f.owner] = true;
            }
        }
    }
    (0..flat.len()).filter(|&i| found[i]).collect()
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

    cut_craft_core::contract_tests!(VectorValidator, crossed);

    /// A job with a problem to find: a bow tie.
    fn crossed() -> Document {
        document_with(vec![bow_tie()])
    }

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    /// `paths` with nothing selected, so the validator checks them all.
    fn unselected(paths: Vec<Path>) -> Document {
        let mut doc = document_with(paths);
        let mut c = Change::builder(&doc);
        c.select([]);
        doc.apply(&c.finish()).unwrap();
        doc
    }

    /// Which of `paths` (by position) the validator picks.
    fn found(paths: Vec<Path>, params: &Params) -> Result<Vec<usize>, String> {
        let mut doc = unselected(paths);
        let ids: Vec<EntityId> = doc.vectors.keys().copied().collect();
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

    fn with_zero_span() -> Path {
        Path::polyline(
            &[
                p(50.0, 50.0),
                p(60.0, 50.0),
                p(60.0, 50.0),
                p(60.0, 60.0),
                p(50.0, 60.0),
            ],
            true,
        )
    }

    #[test]
    fn finds_each_kind_of_problem() {
        let paths = vec![
            square(0.0, 0.0, 10.0),
            bow_tie(),
            square(30.0, 30.0, 5.0),
            square(30.0, 30.0, 5.0).reversed(),
            with_zero_span(),
            Path::polyline(&[p(80.0, 0.0), p(90.0, 0.0)], false),
        ];
        // the bow tie is drawn over the first square, whose corners it touches but does not
        // cross; the second copy of the small square overlaps the first
        assert_eq!(found(paths, &Params::default()).unwrap(), [1, 3, 4]);
    }

    #[test]
    fn vectors_crossing_each_other_are_both_found() {
        let a = square(0.0, 0.0, 10.0);
        let b = square(5.0, 5.0, 10.0);
        let apart = square(40.0, 40.0, 5.0);
        assert_eq!(
            found(vec![a, b, apart], &Params::default()).unwrap(),
            [0, 1]
        );
        // touching at a corner is not crossing
        let touching = vec![square(0.0, 0.0, 10.0), square(10.0, 10.0, 10.0)];
        assert!(found(touching, &Params::default()).is_err());
    }

    #[test]
    fn each_check_can_be_turned_off_and_open_is_an_extra() {
        let only = |overlapping, intersections, zero_length_spans, open| Params {
            overlapping,
            intersections,
            zero_length_spans,
            open,
            ..Params::default()
        };
        let paths = || {
            vec![
                Path::polyline(&[p(50.0, 0.0), p(60.0, 0.0)], false),
                bow_tie(),
                square(30.0, 30.0, 5.0),
                square(30.0, 30.0, 5.0),
                with_zero_span(),
            ]
        };
        assert_eq!(
            found(paths(), &only(false, false, false, true)).unwrap(),
            [0]
        );
        assert_eq!(
            found(paths(), &only(false, true, false, false)).unwrap(),
            [1]
        );
        assert_eq!(
            found(paths(), &only(true, false, false, false)).unwrap(),
            [3]
        );
        assert_eq!(
            found(paths(), &only(false, false, true, false)).unwrap(),
            [4]
        );
    }

    #[test]
    fn fixing_removes_zero_length_spans_and_keeps_the_shape() {
        let mut doc = unselected(vec![with_zero_span(), square(0.0, 0.0, 10.0)]);
        let fix = Params {
            fix_zero_length_spans: true,
            ..Params::default()
        };
        doc.apply(&VectorValidator::run(&doc, &fix).unwrap())
            .unwrap();
        let fixed = doc.vectors.values().next().unwrap();
        assert!(fixed.closed);
        assert_eq!(fixed.segments.len(), 4);
        assert_eq!(fixed.bounds(), with_zero_span().bounds());
        // nothing left to fix, and nothing left to find
        assert!(VectorValidator::run(&doc, &fix).is_err());
        assert!(VectorValidator::run(&doc, &Params::default()).is_err());
    }

    #[test]
    fn a_clean_job_is_reported_not_selected() {
        let err = found(vec![square(0.0, 0.0, 10.0)], &Params::default()).unwrap_err();
        assert!(err.contains("no problems found in 1 vector"), "{err}");
    }

    #[test]
    fn checks_only_the_selection_when_there_is_one() {
        // document_with selects what it adds; add a bow tie after, unselected
        let mut doc = document_with(vec![square(20.0, 20.0, 10.0)]);
        let mut c = Change::builder(&doc);
        c.add(bow_tie());
        doc.apply(&c.finish()).unwrap();
        assert!(VectorValidator::run(&doc, &Params::default()).is_err());
    }

    #[test]
    fn curves_are_checked_too() {
        let circle = Path::circle(p(0.0, 0.0), 5.0);
        assert_eq!(
            crossing(&[circle.flatten(0.01)], &[true]),
            Vec::<usize>::new()
        );
    }
}
