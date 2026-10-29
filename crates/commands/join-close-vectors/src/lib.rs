//! Join / Close Vectors (Aspire's help: Join / Close Vector with a Straight Line, with a
//! Smooth Curve, Join/Close by Moving End Points). Two open vectors selected are joined into
//! one at their closest ends; one is closed. A straight line bridges the gap, or a smooth
//! curve carrying on from each end, or the ends both move to the point halfway between.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Path, Point, Segment, arc_to_cubics};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct JoinCloseVectors;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    #[default]
    StraightLine,
    SmoothCurve,
    MoveEndPoints,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    #[schemars(extend("x-widget" = "row"))]
    pub method: Method,
}

impl Command for JoinCloseVectors {
    const ID: &'static str = "join_close_vectors";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Join / Close Vectors",
            group: "Edit Objects",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let open: Vec<_> = doc
            .selection
            .iter()
            .filter(|id| !doc.vectors[id].closed)
            .copied()
            .collect();
        match open.as_slice() {
            [_] => {
                // one vector: closed as Close / Extend Vector closes it
                use cmd_close_extend_vector::{
                    CloseExtendVector, Method as Close, Params as Closing,
                };
                let method = match p.method {
                    Method::StraightLine => Close::Line,
                    Method::SmoothCurve => Close::Smooth,
                    Method::MoveEndPoints => Close::CommonPoint,
                };
                CloseExtendVector::run(
                    doc,
                    &Closing {
                        method,
                        max_gap: 0.0,
                    },
                )
            }
            [a, b] => {
                let joined = join(&doc.vectors[a], &doc.vectors[b], p.method);
                let mut c = Change::builder(doc);
                c.replace(*a, joined);
                c.remove(*b);
                c.select([*a]);
                Ok(c.finish())
            }
            _ => Err(CommandError::Failed(
                "select one open vector to close, or two to join".into(),
            )),
        }
    }
}

fn end(path: &Path) -> Point {
    path.segments.last().map_or(path.start, Segment::end)
}

/// `a` and `b` as one open path, joined at their closest ends.
fn join(a: &Path, b: &Path, method: Method) -> Path {
    // `a` turned to end at the near end, `b` to start there
    let pairs = [
        (false, false, end(a).distance(b.start)),
        (false, true, end(a).distance(end(b))),
        (true, false, a.start.distance(b.start)),
        (true, true, a.start.distance(end(b))),
    ];
    let (flip_a, flip_b, _) = pairs
        .into_iter()
        .min_by(|x, y| x.2.total_cmp(&y.2))
        .expect("four pairs");
    let mut first = if flip_a { a.reversed() } else { a.clone() };
    let mut second = if flip_b { b.reversed() } else { b.clone() };
    let (from, to) = (end(&first), second.start);
    let gap = from.distance(to);
    match method {
        Method::StraightLine => first.segments.push(Segment::Line { to }),
        Method::SmoothCurve => {
            // a third of the gap along each end's own direction
            let out = arriving(&first).normalize_or_zero() * gap / 3.0;
            let into = leaving(&second).normalize_or_zero() * gap / 3.0;
            first.segments.push(Segment::Cubic {
                c1: from + out,
                c2: to - into,
                to,
            });
        }
        Method::MoveEndPoints => {
            let middle = (from + to) / 2.0;
            move_end(&mut first, middle);
            move_start(&mut second, middle);
        }
    }
    first.segments.extend(second.segments);
    first.closed = false;
    first
}

/// The way `path` heads as it leaves its start.
fn leaving(path: &Path) -> Point {
    match path.segments.first() {
        Some(&Segment::Line { to }) => to - path.start,
        Some(&Segment::Cubic { c1, to, .. }) => {
            if c1 == path.start {
                to - path.start
            } else {
                c1 - path.start
            }
        }
        Some(&Segment::Arc { center, ccw, .. }) => turn(path.start - center, ccw),
        None => Point::ZERO,
    }
}

/// The way `path` heads as it arrives at its end.
fn arriving(path: &Path) -> Point {
    let n = path.segments.len();
    let before = if n >= 2 {
        path.segments[n - 2].end()
    } else {
        path.start
    };
    match path.segments.last() {
        Some(&Segment::Line { to }) => to - before,
        Some(&Segment::Cubic { c2, to, .. }) => {
            if c2 == to {
                to - before
            } else {
                to - c2
            }
        }
        Some(&Segment::Arc { to, center, ccw }) => turn(to - center, ccw),
        None => Point::ZERO,
    }
}

fn turn(radius: Point, ccw: bool) -> Point {
    if ccw { radius.perp() } else { -radius.perp() }
}

/// `path` with its last point moved to `to` (an arc there becomes curves first, as an arc
/// cannot move one end and stay an arc).
fn move_end(path: &mut Path, to: Point) {
    let n = path.segments.len();
    let from = if n >= 2 {
        path.segments[n - 2].end()
    } else {
        path.start
    };
    if let Some(&Segment::Arc {
        to: end,
        center,
        ccw,
    }) = path.segments.last()
    {
        let curves = arc_to_cubics(from, end, center, ccw);
        path.segments.splice(n - 1.., curves);
    }
    match path.segments.last_mut() {
        Some(Segment::Line { to: end }) => *end = to,
        Some(Segment::Cubic { c2, to: end, .. }) => {
            *c2 += to - *end;
            *end = to;
        }
        _ => {}
    }
}

/// `path` with its start moved to `to`.
fn move_start(path: &mut Path, to: Point) {
    if let Some(&Segment::Arc {
        to: end,
        center,
        ccw,
    }) = path.segments.first()
    {
        let curves = arc_to_cubics(path.start, end, center, ccw);
        path.segments.splice(0..1, curves);
    }
    if let Some(Segment::Cubic { c1, .. }) = path.segments.first_mut() {
        *c1 += to - path.start;
    }
    path.start = to;
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{document_with, rejection, run_on};

    fn line(a: (f64, f64), b: (f64, f64)) -> Path {
        Path::polyline(&[Point::new(a.0, a.1), Point::new(b.0, b.1)], false)
    }

    /// Two lines along y = 0, ending 10 apart: 0..10 and 30..20 (the second backwards).
    fn two_lines() -> Document {
        document_with(vec![
            line((0.0, 0.0), (10.0, 0.0)),
            line((30.0, 0.0), (20.0, 0.0)),
        ])
    }

    cut_craft_core::contract_tests!(JoinCloseVectors, two_lines);

    fn joined(method: Method) -> Path {
        let mut doc = two_lines();
        run_on::<JoinCloseVectors>(&mut doc, &Params { method });
        assert_eq!(doc.vectors.len(), 1, "two became one");
        doc.vectors.values().next().unwrap().clone()
    }

    #[test]
    fn joined_with_a_line_across_the_closest_ends() {
        let p = joined(Method::StraightLine);
        assert!(!p.closed);
        assert_eq!((p.start, end(&p)), (Point::ZERO, Point::new(30.0, 0.0)));
        assert_eq!(
            p.segments[1],
            Segment::Line {
                to: Point::new(20.0, 0.0)
            }
        );
    }

    #[test]
    fn joined_with_a_curve_carrying_on_from_each_end() {
        let p = joined(Method::SmoothCurve);
        let Segment::Cubic { c1, c2, to } = p.segments[1] else {
            panic!("{:?}", p.segments)
        };
        assert_eq!(
            (c1, c2, to),
            (
                Point::new(10.0 + 10.0 / 3.0, 0.0),
                Point::new(20.0 - 10.0 / 3.0, 0.0),
                Point::new(20.0, 0.0)
            )
        );
    }

    #[test]
    fn joined_by_moving_the_ends_to_their_middle() {
        let p = joined(Method::MoveEndPoints);
        assert_eq!(p.segments.len(), 2, "no bridge");
        assert_eq!(p.segments[0].end(), Point::new(15.0, 0.0));
        assert_eq!(end(&p), Point::new(30.0, 0.0));
    }

    #[test]
    fn one_vector_is_closed_and_three_are_refused() {
        let mut doc = document_with(vec![Path::polyline(
            &[Point::ZERO, Point::new(10.0, 0.0), Point::new(10.0, 10.0)],
            false,
        )]);
        run_on::<JoinCloseVectors>(&mut doc, &Params::default());
        assert!(doc.vectors.values().next().unwrap().closed);
        let three = document_with(vec![
            line((0.0, 0.0), (1.0, 0.0)),
            line((2.0, 0.0), (3.0, 0.0)),
            line((4.0, 0.0), (5.0, 0.0)),
        ]);
        assert!(JoinCloseVectors::run(&three, &Params::default()).is_err());
        assert!(rejection::<JoinCloseVectors>(&Params::default()).contains("select one open"));
    }
}
