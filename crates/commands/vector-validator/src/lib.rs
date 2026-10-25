//! Vector Validator: finds what stops toolpaths being made from vectors (paths crossing
//! themselves or each other, zero-length spans) in the selection, or all shown vectors when
//! nothing is selected. The vectors with problems end up selected, as Aspire's marks; zero-
//! length spans can be fixed.

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::geometry::{Path, Point, Segment};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub struct VectorValidator;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// Remove spans that start and end at the same point
    #[schemars(title = "Fix zero length spans")]
    pub fix_zero_length_spans: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            fix_zero_length_spans: true,
        }
    }
}

/// Spans shorter than this count as zero length.
const ZERO: f64 = 1e-6;

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
        let ids: Vec<EntityId> = if doc.selection.is_empty() {
            doc.vectors
                .keys()
                .copied()
                .filter(|id| doc.shown(*id))
                .collect()
        } else {
            doc.selection.iter().copied().collect()
        };
        if ids.is_empty() {
            return Err(CommandError::Failed("there are no vectors to check".into()));
        }
        let lines: Vec<Vec<Point>> = ids
            .iter()
            .map(|id| doc.vectors[id].flatten(cut_craft_core::polygon::TOLERANCE))
            .collect();
        let mut bad = BTreeSet::new();
        for (i, a) in lines.iter().enumerate() {
            if crosses(a, a, true) {
                bad.insert(ids[i]);
            }
            for (j, b) in lines.iter().enumerate().skip(i + 1) {
                if crosses(a, b, false) {
                    bad.insert(ids[i]);
                    bad.insert(ids[j]);
                }
            }
        }
        let mut c = Change::builder(doc);
        for &id in &ids {
            let path = &doc.vectors[&id];
            let fixed = without_zero_spans(path);
            if fixed.segments.len() != path.segments.len() {
                if p.fix_zero_length_spans && !fixed.segments.is_empty() {
                    c.replace(id, fixed);
                } else {
                    bad.insert(id);
                }
            }
        }
        c.select(bad);
        Ok(c.finish())
    }
}

fn without_zero_spans(path: &Path) -> Path {
    let mut from = path.start;
    let mut out = path.clone();
    out.segments.retain(|s| {
        let keep = s.end().distance(from) > ZERO || matches!(s, Segment::Arc { .. });
        from = s.end();
        keep
    });
    out
}

/// Whether two polylines cross (or, when `same`, a polyline crosses itself away from its
/// neighbouring segments).
// ponytail: every pair of segments, O(n²); a sweep line when big imports make it slow
fn crosses(a: &[Point], b: &[Point], same: bool) -> bool {
    let n = a.len().saturating_sub(1);
    for i in 0..n {
        for j in 0..b.len().saturating_sub(1) {
            if same {
                let closed_wrap = a.first() == a.last() && i == 0 && j + 2 == a.len();
                if j <= i + 1 || closed_wrap {
                    continue;
                }
            }
            if proper(a[i], a[i + 1], b[j], b[j + 1]) {
                return true;
            }
        }
    }
    false
}

/// Segments pq and rs cross at a point inside both.
fn proper(p: Point, q: Point, r: Point, s: Point) -> bool {
    let side = |a: Point, b: Point, c: Point| (b - a).perp_dot(c - a);
    let (d1, d2) = (side(r, s, p), side(r, s, q));
    let (d3, d4) = (side(p, q, r), side(p, q, s));
    d1 * d2 < 0.0 && d3 * d4 < 0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{document_with, run_on, square};

    cut_craft_core::contract_tests!(VectorValidator);

    #[test]
    fn crossing_vectors_and_bow_ties_are_selected() {
        let bow_tie = Path::polyline(
            &[
                Point::new(50.0, 0.0),
                Point::new(60.0, 10.0),
                Point::new(60.0, 0.0),
                Point::new(50.0, 10.0),
            ],
            true,
        );
        let mut doc = document_with(vec![
            square(0.0, 0.0, 10.0),
            square(5.0, 5.0, 10.0),
            square(30.0, 0.0, 5.0),
            bow_tie,
        ]);
        doc.selection.clear();
        run_on::<VectorValidator>(&mut doc, &Params::default());
        let picked: Vec<u64> = doc.selection.iter().map(|id| id.0).collect();
        assert_eq!(
            picked,
            vec![1, 2, 4],
            "the overlapping squares and the bow tie"
        );
    }

    #[test]
    fn zero_length_spans_are_removed_or_marked() {
        let mut path = square(0.0, 0.0, 10.0);
        path.segments.insert(
            1,
            Segment::Line {
                to: path.segments[0].end(),
            },
        );
        let mut fixed = document_with(vec![path.clone()]);
        run_on::<VectorValidator>(&mut fixed, &Params::default());
        assert_eq!(fixed.vectors.values().next().unwrap().segments.len(), 4);
        assert!(fixed.selection.is_empty(), "nothing left wrong");
        let mut marked = document_with(vec![path]);
        run_on::<VectorValidator>(
            &mut marked,
            &Params {
                fix_zero_length_spans: false,
            },
        );
        assert_eq!(marked.selection.len(), 1);
        assert!(VectorValidator::run(&Document::default(), &Params::default()).is_err());
    }
}
