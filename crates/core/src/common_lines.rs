//! Common lines: a span of one vector lying exactly on a span of another vector on the
//! same layer (the same two end points, and the same line between them). A copy pasted in
//! place, a Circular or Array Copy that lands on its original, or two shapes drawn edge to
//! edge all leave one; a profile would cut it twice. The 2D view draws them in red so
//! they are seen. Vectors on different layers never share a common line.

use std::collections::HashMap;

use crate::document::{Document, EntityId, LayerId};
use crate::geometry::{Path, Point, Segment, arc_sweep};

/// How near (mm) two end points must be to count as the same point.
const SAME_POINT: f64 = 0.001;
/// How near (mm) two spans' middles must be for them to lie on the same line.
const SAME_LINE: f64 = 0.01;

/// One span of a vector: which vector, which span, where it starts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Span {
    pub id: EntityId,
    pub index: usize,
    pub from: Point,
    pub segment: Segment,
}

impl Span {
    /// This span alone, as a path to draw.
    pub fn path(&self) -> Path {
        Path {
            start: self.from,
            segments: vec![self.segment],
            closed: false,
        }
    }
}

/// A span's layer and its two ends, either way round: spans that may lie on each other.
type Ends = (LayerId, [i64; 2], [i64; 2]);

/// Every span of a shown vector that lies on a span of another shown vector on the same
/// layer, or on another span of its own.
pub fn common_lines(doc: &Document) -> Vec<Span> {
    let mut by_ends: HashMap<Ends, Vec<(Span, Point)>> = HashMap::new();
    // a construction line lying on a vector is how one is meant to be used: never flagged
    let flagged = |id: EntityId| doc.shown(id) && !doc.is_construction(id);
    for (&id, path) in doc.vectors.iter().filter(|(id, _)| flagged(**id)) {
        let layer = doc.layer_of(id);
        let mut from = path.start;
        for (index, &segment) in path.segments.iter().enumerate() {
            let to = segment.end();
            let span = Span {
                id,
                index,
                from,
                segment,
            };
            from = to;
            let middle = middle(span.from, &segment);
            // a span with no length (a doubled node) lies on nothing
            if span.from.distance(to) < SAME_POINT && span.from.distance(middle) < SAME_POINT {
                continue;
            }
            let (a, b) = (grid(span.from), grid(to));
            let key = (layer, a.min(b), a.max(b));
            by_ends.entry(key).or_default().push((span, middle));
        }
    }
    let mut common: Vec<Span> = by_ends
        .into_values()
        .flat_map(|spans| {
            let on_another = |i: usize| {
                spans
                    .iter()
                    .enumerate()
                    .any(|(j, other)| j != i && other.1.distance(spans[i].1) < SAME_LINE)
            };
            (0..spans.len())
                .filter(|&i| on_another(i))
                .map(|i| spans[i].0)
                .collect::<Vec<_>>()
        })
        .collect();
    common.sort_by_key(|s| (s.id, s.index));
    common
}

/// A point to the nearest [`SAME_POINT`], so the same point found twice gives one key.
#[expect(
    clippy::cast_possible_truncation,
    reason = "job sizes are far below i64"
)]
fn grid(p: Point) -> [i64; 2] {
    [
        (p.x / SAME_POINT).round() as i64,
        (p.y / SAME_POINT).round() as i64,
    ]
}

/// The point halfway along a span: the same whichever way the span runs.
fn middle(from: Point, segment: &Segment) -> Point {
    match *segment {
        Segment::Line { to } => (from + to) / 2.0,
        Segment::Arc { to, center, ccw } => {
            let half = arc_sweep(from, to, center, ccw) / 2.0;
            center + glam::DMat2::from_angle(half) * (from - center)
        }
        Segment::Cubic { c1, c2, to } => (from + 3.0 * c1 + 3.0 * c2 + to) / 8.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Layer;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn square(at: f64) -> Path {
        Path::polyline(
            &[
                p(at, 0.0),
                p(at + 10.0, 0.0),
                p(at + 10.0, 10.0),
                p(at, 10.0),
            ],
            true,
        )
    }

    fn add(doc: &mut Document, path: Path) -> EntityId {
        let id = doc.next_id();
        doc.vectors.insert(id, path);
        id
    }

    #[test]
    fn a_copy_lying_on_its_original_is_common_all_round() {
        let mut doc = Document::default();
        let a = add(&mut doc, square(0.0));
        // drawn the other way round, from another corner: still the same line
        let b = add(&mut doc, square(0.0).reversed());
        let common = common_lines(&doc);
        assert_eq!(common.iter().filter(|s| s.id == a).count(), 4);
        assert_eq!(common.iter().filter(|s| s.id == b).count(), 4);
    }

    #[test]
    fn two_squares_side_by_side_share_only_the_edge_between_them() {
        let mut doc = Document::default();
        let a = add(&mut doc, square(0.0));
        let b = add(&mut doc, square(10.0));
        let common = common_lines(&doc);
        assert_eq!(common.len(), 2, "{common:?}");
        let edge = |id| common.iter().find(|s| s.id == id).expect("its shared edge");
        for id in [a, b] {
            let s = edge(id);
            assert!(s.from.x == 10.0 && s.segment.end().x == 10.0, "{s:?}");
        }
    }

    #[test]
    fn the_same_ends_but_a_different_line_is_not_common() {
        let mut doc = Document::default();
        add(
            &mut doc,
            Path::polyline(&[p(0.0, 0.0), p(10.0, 0.0)], false),
        );
        // a half circle between the same two points
        add(
            &mut doc,
            Path {
                start: p(0.0, 0.0),
                segments: vec![Segment::Arc {
                    to: p(10.0, 0.0),
                    center: p(5.0, 0.0),
                    ccw: true,
                }],
                closed: false,
            },
        );
        // a line that lies over part of the first but ends elsewhere
        add(&mut doc, Path::polyline(&[p(0.0, 0.0), p(5.0, 0.0)], false));
        assert_eq!(common_lines(&doc), vec![]);
    }

    #[test]
    fn arcs_and_curves_lying_on_each_other_are_common_either_way_round() {
        let mut doc = Document::default();
        let circle = Path::circle(p(50.0, 50.0), 20.0);
        add(&mut doc, circle.clone());
        add(&mut doc, circle.reversed());
        let curve = Path {
            start: p(0.0, 0.0),
            segments: vec![Segment::Cubic {
                c1: p(5.0, 20.0),
                c2: p(15.0, -20.0),
                to: p(20.0, 0.0),
            }],
            closed: false,
        };
        add(&mut doc, curve.clone());
        add(&mut doc, curve.reversed());
        let spans = circle.segments.len() * 2 + 2;
        assert_eq!(common_lines(&doc).len(), spans);
    }

    #[test]
    fn only_vectors_on_the_same_shown_layer_share_a_common_line() {
        let mut doc = Document::default();
        add(&mut doc, square(0.0));
        let b = add(&mut doc, square(0.0));
        let other = LayerId(77);
        doc.layers.push(Layer {
            id: other,
            name: "Other".into(),
            ..doc.layers[0].clone()
        });
        doc.vector_layers.insert(b, other);
        assert_eq!(common_lines(&doc), vec![], "different layers");
        doc.vector_layers.remove(&b);
        assert_eq!(common_lines(&doc).len(), 8, "the same layer");
        doc.layers[0].visible = false;
        assert_eq!(common_lines(&doc), vec![], "a hidden layer shows nothing");
    }
}
