//! Node editing on paths: a path's nodes are its start and each span's end (a closed path's
//! last node is its start again, so it is not counted twice). Spans are numbered from 0;
//! span `k` runs from node `k` to node `k + 1`.

use crate::CommandError;
use crate::document::{Document, EntityId};
use crate::geometry::{Path, Point, Segment, arc_sweep, polyline_distance};
use crate::params::Xy;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Where a node-editing command acts: the node or span of `vector` nearest `at`, within
/// `reach` (the 2D view's pick distance).
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct NodeAt {
    pub vector: EntityId,
    pub at: Xy,
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub reach: f64,
}

impl Default for NodeAt {
    fn default() -> Self {
        Self {
            vector: EntityId(1),
            at: Xy::new(10.0, 10.0),
            reach: 1.0,
        }
    }
}

impl NodeAt {
    /// The vector, if it exists and can be edited.
    pub fn path<'a>(&self, doc: &'a Document) -> Result<&'a Path, CommandError> {
        doc.vectors
            .get(&self.vector)
            .filter(|_| doc.editable(self.vector))
            .ok_or_else(|| CommandError::Failed("that vector cannot be edited".into()))
    }

    /// The node nearest the point.
    pub fn node(&self, path: &Path) -> Result<usize, CommandError> {
        node_near(path, self.at.into(), self.reach)
            .ok_or_else(|| CommandError::Failed("there is no node there".into()))
    }

    /// The span nearest the point.
    pub fn span(&self, path: &Path) -> Result<usize, CommandError> {
        span_near(path, self.at.into(), self.reach)
            .ok_or_else(|| CommandError::Failed("there is no span there".into()))
    }
}

/// The nodes of `path`.
pub fn nodes(path: &Path) -> Vec<Point> {
    let mut v = vec![path.start];
    v.extend(path.segments.iter().map(Segment::end));
    if path.closed && v.len() > 1 {
        v.pop();
    }
    v
}

/// The node nearest `at`, if one is within `reach`.
pub fn node_near(path: &Path, at: Point, reach: f64) -> Option<usize> {
    nodes(path)
        .iter()
        .enumerate()
        .map(|(k, q)| (k, q.distance(at)))
        .filter(|(_, d)| *d <= reach)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(k, _)| k)
}

/// The span nearest `at`, if one passes within `reach`.
pub fn span_near(path: &Path, at: Point, reach: f64) -> Option<usize> {
    let mut from = path.start;
    let mut best = None;
    for (k, s) in path.segments.iter().enumerate() {
        let one = Path {
            start: from,
            segments: vec![*s],
            closed: false,
        };
        let d = polyline_distance(&one.flatten(reach / 10.0), at);
        if d <= reach && best.is_none_or(|(_, b)| d < b) {
            best = Some((k, d));
        }
        from = s.end();
    }
    best.map(|(k, _)| k)
}

/// Where span `k` starts.
fn span_start(path: &Path, k: usize) -> Point {
    if k == 0 {
        path.start
    } else {
        path.segments[k - 1].end()
    }
}

/// An arc from `from` to `to` bulging as much as `old` did from its own ends.
fn same_bulge(old_from: Point, old: Segment, from: Point, to: Point) -> Segment {
    let Segment::Arc {
        to: old_to,
        center,
        ccw,
    } = old
    else {
        return old;
    };
    let sweep = arc_sweep(old_from, old_to, center, ccw);
    let bulge = (sweep / 4.0).tan() * if ccw { 1.0 } else { -1.0 };
    if bulge.abs() < 1e-12 || from.distance(to) < 1e-12 {
        return Segment::Line { to };
    }
    let centre = (from + to) / 2.0 + (to - from).perp() * ((1.0 - bulge * bulge) / (4.0 * bulge));
    Segment::Arc {
        to,
        center: centre,
        ccw,
    }
}

/// `path` with node `k` moved to `to`; Bézier handles move with it, arcs keep their bulge.
#[must_use]
pub fn move_node(path: &Path, k: usize, to: Point) -> Path {
    let n = path.segments.len();
    let mut out = path.clone();
    let delta = to - nodes(path)[k];
    // the span arriving at node k, and the one leaving it
    let arriving = if k == 0 {
        path.closed.then(|| n - 1)
    } else {
        Some(k - 1)
    };
    let leaving = (k < n).then_some(k);
    if k == 0 {
        out.start = to;
    }
    if let Some(a) = arriving {
        let from = span_start(&out, a);
        out.segments[a] = match path.segments[a] {
            Segment::Line { .. } => Segment::Line { to },
            Segment::Cubic { c1, c2, .. } => Segment::Cubic {
                c1,
                c2: c2 + delta,
                to,
            },
            arc @ Segment::Arc { .. } => same_bulge(span_start(path, a), arc, from, to),
        };
    }
    if let Some(l) = leaving {
        out.segments[l] = match path.segments[l] {
            Segment::Cubic { c1, c2, to: end } => Segment::Cubic {
                c1: c1 + delta,
                c2,
                to: end,
            },
            arc @ Segment::Arc { to: end, .. } => same_bulge(span_start(path, l), arc, to, end),
            line => line,
        };
    }
    out
}

/// `path` without node `k`: its two spans become one straight line. `None` when too few
/// nodes would be left.
pub fn delete_node(path: &Path, k: usize) -> Option<Path> {
    let n = path.segments.len();
    let min = if path.closed { 3 } else { 2 };
    if nodes(path).len() <= min {
        return None;
    }
    let mut out = path.clone();
    if path.closed && k == 0 {
        // start one node on, and drop the span into the old start
        out.start = path.segments[0].end();
        out.segments.remove(0);
        let last = out.segments.len() - 1;
        out.segments[last] = Segment::Line { to: out.start };
    } else if k == 0 {
        out.start = path.segments[0].end();
        out.segments.remove(0);
    } else if k == n {
        out.segments.pop();
    } else {
        let to = path.segments[k].end();
        out.segments.remove(k);
        out.segments[k - 1] = Segment::Line { to };
    }
    Some(out)
}

/// `path` with a new node on its span nearest `at`, splitting that span in two there.
#[expect(clippy::many_single_char_names, reason = "de Casteljau's points")]
pub fn insert_node(path: &Path, at: Point, reach: f64) -> Option<Path> {
    let k = span_near(path, at, reach)?;
    let from = span_start(path, k);
    let mut out = path.clone();
    let pieces = match path.segments[k] {
        Segment::Line { to } => {
            let d = to - from;
            let t = ((at - from).dot(d) / d.length_squared().max(1e-24)).clamp(0.0, 1.0);
            let mid = from + d * t;
            vec![Segment::Line { to: mid }, Segment::Line { to }]
        }
        Segment::Arc { to, center, ccw } => {
            let r = from.distance(center);
            let mid = center + (at - center).normalize_or_zero() * r;
            vec![
                Segment::Arc {
                    to: mid,
                    center,
                    ccw,
                },
                Segment::Arc { to, center, ccw },
            ]
        }
        Segment::Cubic { c1, c2, to } => {
            // the nearest parameter by sampling, then de Casteljau
            let at_t = |t: f64| {
                let u = 1.0 - t;
                u * u * u * from + 3.0 * u * u * t * c1 + 3.0 * u * t * t * c2 + t * t * t * to
            };
            let t = (1..200)
                .map(|i| f64::from(i) / 200.0)
                .min_by(|a, b| at_t(*a).distance(at).total_cmp(&at_t(*b).distance(at)))
                .unwrap_or(0.5);
            let (a, b, c) = (from.lerp(c1, t), c1.lerp(c2, t), c2.lerp(to, t));
            let (d, e) = (a.lerp(b, t), b.lerp(c, t));
            let mid = d.lerp(e, t);
            vec![
                Segment::Cubic {
                    c1: a,
                    c2: d,
                    to: mid,
                },
                Segment::Cubic { c1: e, c2: c, to },
            ]
        }
    };
    out.segments.splice(k..=k, pieces);
    Some(out)
}

/// `path` cut open at node `k`: a closed path becomes one open path from there round to
/// there; an open one becomes the parts either side.
pub fn cut_at(path: &Path, k: usize) -> Vec<Path> {
    let n = path.segments.len();
    if path.closed {
        let start = nodes(path)[k];
        let mut segments = path.segments[k..].to_vec();
        segments.extend_from_slice(&path.segments[..k]);
        return vec![Path {
            start,
            segments,
            closed: false,
        }];
    }
    if k == 0 || k >= n {
        return vec![path.clone()];
    }
    let first = Path {
        start: path.start,
        segments: path.segments[..k].to_vec(),
        closed: false,
    };
    let second = Path {
        start: span_start(path, k),
        segments: path.segments[k..].to_vec(),
        closed: false,
    };
    vec![first, second]
}

/// A closed `path` starting at node `k` instead.
pub fn make_start(path: &Path, k: usize) -> Option<Path> {
    path.closed.then(|| {
        let mut open = cut_at(path, k).remove(0);
        open.closed = true;
        open
    })
}

/// `path` with span `k` a straight line.
#[must_use]
pub fn span_to_line(path: &Path, k: usize) -> Path {
    let mut out = path.clone();
    out.segments[k] = Segment::Line {
        to: path.segments[k].end(),
    };
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Path {
        Path::polyline(
            &[
                Point::new(0.0, 0.0),
                Point::new(10.0, 0.0),
                Point::new(10.0, 10.0),
                Point::new(0.0, 10.0),
            ],
            true,
        )
    }

    #[test]
    fn a_closed_square_has_four_nodes_and_moving_one_moves_both_its_spans() {
        let s = square();
        assert_eq!(nodes(&s).len(), 4);
        assert_eq!(node_near(&s, Point::new(9.0, 9.5), 2.0), Some(2));
        let moved = move_node(&s, 0, Point::new(-2.0, -2.0));
        assert_eq!(moved.start, Point::new(-2.0, -2.0));
        assert_eq!(
            moved.segments[3].end(),
            Point::new(-2.0, -2.0),
            "the closing span follows"
        );
        let corner = move_node(&s, 2, Point::new(12.0, 12.0));
        assert_eq!(corner.segments[1].end(), Point::new(12.0, 12.0));
    }

    #[test]
    fn arcs_keep_their_bulge_and_beziers_carry_their_handles() {
        let circle = Path::circle(Point::ZERO, 10.0);
        let moved = move_node(&circle, 1, Point::new(-20.0, 0.0));
        let Segment::Arc { center, .. } = moved.segments[0] else {
            panic!("still an arc")
        };
        assert!(
            (center - Point::new(-5.0, 0.0)).length() < 1e-9,
            "a half circle on the new chord"
        );
        let curve = Path {
            start: Point::ZERO,
            segments: vec![Segment::Cubic {
                c1: Point::new(1.0, 1.0),
                c2: Point::new(9.0, 1.0),
                to: Point::new(10.0, 0.0),
            }],
            closed: false,
        };
        let moved = move_node(&curve, 1, Point::new(10.0, 5.0));
        let Segment::Cubic { c1, c2, .. } = moved.segments[0] else {
            panic!("still a curve")
        };
        assert_eq!((c1, c2), (Point::new(1.0, 1.0), Point::new(9.0, 6.0)));
    }

    #[test]
    fn deleting_inserting_cutting_and_restarting() {
        let s = square();
        let three = delete_node(&s, 2).unwrap();
        assert_eq!(nodes(&three).len(), 3);
        assert!(
            delete_node(&three, 0).is_none(),
            "a closed path keeps three nodes"
        );
        let five = insert_node(&s, Point::new(5.0, 0.2), 1.0).unwrap();
        assert_eq!(nodes(&five)[1], Point::new(5.0, 0.0));
        let open = cut_at(&s, 2);
        assert_eq!(open.len(), 1);
        assert!(!open[0].closed && open[0].start == Point::new(10.0, 10.0));
        assert_eq!(
            open[0].segments.last().unwrap().end(),
            Point::new(10.0, 10.0)
        );
        let two = cut_at(&open[0], 2);
        assert_eq!(two.len(), 2);
        let restarted = make_start(&s, 1).unwrap();
        assert!(restarted.closed && restarted.start == Point::new(10.0, 0.0));
        let curvy =
            insert_node(&Path::circle(Point::ZERO, 10.0), Point::new(0.0, 11.0), 2.0).unwrap();
        assert!((nodes(&curvy)[1] - Point::new(0.0, 10.0)).length() < 1e-9);
    }
}
