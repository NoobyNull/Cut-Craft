//! Join Vectors.

use aspire_core::document::{Document, EntityId};
use aspire_core::geometry::{Path, Point, Segment};
use aspire_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct JoinVectors;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// Ends closer than this are joined
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub tolerance: f64,
}

impl Default for Params {
    fn default() -> Self {
        Self { tolerance: 0.1 }
    }
}

impl Command for JoinVectors {
    const ID: &'static str = "join_vectors";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Join Vectors",
            group: "Edit Objects",
            icon: "",
            shortcut: "J",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let (ids, mut chains): (Vec<EntityId>, Vec<Path>) = doc
            .selection
            .iter()
            .filter(|id| !doc.vectors[id].closed)
            .map(|&id| (id, doc.vectors[&id].clone()))
            .unzip();
        // ponytail: O(n^3) pairwise search; fine for hand-picked selections, index endpoints if not
        let mut changed = false;
        'merge: loop {
            for i in 0..chains.len() {
                for j in i + 1..chains.len() {
                    if let Some(joined) = try_join(&chains[i], &chains[j], p.tolerance) {
                        chains[i] = joined;
                        chains.swap_remove(j);
                        changed = true;
                        continue 'merge;
                    }
                }
            }
            break;
        }
        for chain in &mut chains {
            let gap = end(chain).distance(chain.start);
            if chain.segments.len() >= 2 && gap <= p.tolerance {
                if gap > 0.0 {
                    chain.segments.push(Segment::Line { to: chain.start });
                }
                chain.closed = true;
                changed = true;
            }
        }
        if !changed {
            return Err(CommandError::Failed(
                "no open vector ends are within the tolerance".into(),
            ));
        }
        Ok(Change::swap(doc, ids, chains))
    }
}

/// Appends `b` to `a`, bridging the (tolerance-sized) gap with a line.
fn append(mut a: Path, b: Path) -> Path {
    let end = a.segments.last().map_or(a.start, Segment::end);
    if end != b.start {
        a.segments.push(Segment::Line { to: b.start });
    }
    a.segments.extend(b.segments);
    a
}

fn end(p: &Path) -> Point {
    p.segments.last().map_or(p.start, Segment::end)
}

/// Joins two chains if any pair of their ends is within `tol`.
fn try_join(a: &Path, b: &Path, tol: f64) -> Option<Path> {
    let near = |p: Point, q: Point| p.distance(q) <= tol;
    if near(end(a), b.start) {
        Some(append(a.clone(), b.clone()))
    } else if near(end(a), end(b)) {
        Some(append(a.clone(), b.reversed()))
    } else if near(a.start, b.start) {
        Some(append(a.reversed(), b.clone()))
    } else if near(a.start, end(b)) {
        Some(append(b.clone(), a.clone()))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use aspire_core::testing::document_with;

    fn line(a: (f64, f64), b: (f64, f64)) -> Path {
        Path::polyline(&[Point::new(a.0, a.1), Point::new(b.0, b.1)], false)
    }

    fn touching_lines() -> Document {
        document_with(vec![
            line((0.0, 0.0), (10.0, 0.0)),
            line((10.0, 0.0), (10.0, 10.0)),
        ])
    }

    fn join(paths: Vec<Path>) -> Result<Document, String> {
        let mut doc = document_with(paths);
        let change = JoinVectors::run(&doc, &Params::default()).map_err(|e| e.to_string())?;
        doc.apply(&change).unwrap();
        Ok(doc)
    }

    #[test]
    fn end_to_start_becomes_one_path() {
        let doc = join(vec![
            line((0.0, 0.0), (10.0, 0.0)),
            line((10.05, 0.0), (10.0, 10.0)),
        ])
        .unwrap();
        assert_eq!(doc.vectors.len(), 1);
        assert!(!doc.vectors.values().next().unwrap().closed);
    }

    #[test]
    fn lines_pointing_the_wrong_way_are_turned_round() {
        let doc = join(vec![
            line((0.0, 0.0), (10.0, 0.0)),
            line((10.0, 10.0), (10.0, 0.0)),
        ])
        .unwrap();
        assert_eq!(doc.vectors.len(), 1);
    }

    #[test]
    fn a_ring_of_lines_closes() {
        let doc = join(vec![
            line((0.0, 0.0), (10.0, 0.0)),
            line((10.0, 0.0), (5.0, 8.0)),
            line((5.0, 8.0), (0.0, 0.0)),
        ])
        .unwrap();
        assert_eq!(doc.vectors.len(), 1);
        assert!(doc.vectors.values().next().unwrap().closed);
    }

    #[test]
    fn far_apart_ends_are_reported() {
        let err = join(vec![
            line((0.0, 0.0), (10.0, 0.0)),
            line((50.0, 0.0), (60.0, 0.0)),
        ])
        .unwrap_err();
        assert_eq!(err, "no open vector ends are within the tolerance");
    }

    aspire_core::contract_tests!(JoinVectors, touching_lines);
}
