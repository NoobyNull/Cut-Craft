//! Join Open Ends: click the end of an open vector, then the end it should meet, and the two
//! are joined there with a straight line, by running both ends on until they meet, or with
//! arcs that carry on smoothly from each end. Click both ends of one vector to close it.

use cmd_join_close_vectors::{arriving, leaving};
use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::geometry::{Path, Point, Segment};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct JoinOpenEnds;

/// How the gap between the two ends is bridged.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    /// A straight line from one end to the other
    #[default]
    #[schemars(title = "Straight Line")]
    StraightLine,
    /// Both ends run on the way they are heading until they meet, making a sharp corner
    #[schemars(title = "Extend to Meet")]
    ExtendToMeet,
    /// Arcs that carry on smoothly from each end, with no corner where they meet
    #[schemars(title = "Arc")]
    Arc,
}

/// Joins open vectors where you click. Each open end has a circle on it: click one end, then
/// the end to join it to. Click both ends of one vector to close it into a shape that can be
/// pocketed or carved.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// How the gap between the two ends is bridged
    pub method: Method,
    /// The first end clicked: the nearest open end within `within` of it
    #[serde(default)]
    #[schemars(skip)]
    pub first: Option<Point>,
    /// The second end clicked, joined to the first
    #[serde(default)]
    #[schemars(skip)]
    pub second: Option<Point>,
    /// How near a click must be to an end
    #[serde(default)]
    #[schemars(skip)]
    pub within: f64,
}

/// One end of an open vector.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct End {
    pub id: EntityId,
    /// The vector's start, rather than its end.
    pub at_start: bool,
    pub point: Point,
}

impl Command for JoinOpenEnds {
    const ID: &'static str = "join_open_ends";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Join Open Ends",
            group: "Edit Objects",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let (keep, gone, path) = joined(doc, p)?;
        let mut c = Change::builder(doc);
        c.replace(keep, path);
        if let Some(gone) = gone {
            c.remove(gone);
        }
        Ok(c.finish())
    }
}

/// Every end of the shown, unlocked open vectors.
pub fn open_ends(doc: &Document) -> Vec<End> {
    doc.vectors
        .iter()
        .filter(|(id, path)| {
            !path.closed && !path.segments.is_empty() && doc.shown(**id) && doc.editable(**id)
        })
        .flat_map(|(&id, path)| {
            [
                End {
                    id,
                    at_start: true,
                    point: path.start,
                },
                End {
                    id,
                    at_start: false,
                    point: last(path),
                },
            ]
        })
        .collect()
}

/// The open end nearest `at`, if one is within `within` of it.
pub fn end_near(doc: &Document, at: Point, within: f64) -> Option<End> {
    nearest(open_ends(doc), at, within)
}

fn nearest(ends: impl IntoIterator<Item = End>, at: Point, within: f64) -> Option<End> {
    ends.into_iter()
        .filter(|e| e.point.distance(at) <= within)
        .min_by(|a, b| a.point.distance(at).total_cmp(&b.point.distance(at)))
}

/// What joining `p`'s two ends makes: the vector kept, the one joined into it (none when
/// one vector is closed), and the kept vector as it becomes.
pub fn joined(
    doc: &Document,
    p: &Params,
) -> Result<(EntityId, Option<EntityId>, Path), CommandError> {
    const CLICK: &str = "click the end of an open vector (each has a circle on it), then the \
                         end to join it to";
    let (Some(first_at), Some(second_at)) = (p.first, p.second) else {
        return Err(CommandError::Failed(CLICK.into()));
    };
    let ends = open_ends(doc);
    let first = nearest(ends.iter().copied(), first_at, p.within)
        .ok_or_else(|| CommandError::Failed(format!("there is no open end there: {CLICK}")))?;
    let others = ends
        .iter()
        .copied()
        .filter(|e| (e.id, e.at_start) != (first.id, first.at_start));
    let Some(second) = nearest(others, second_at, p.within) else {
        let why = if first.point.distance(second_at) <= p.within {
            "that is the end already picked. Click the other end to join it to"
        } else {
            "there is no open end there. Click the end to join to: each has a circle on it"
        };
        return Err(CommandError::Failed(why.into()));
    };
    // the first vector turned to finish at its clicked end
    let a = &doc.vectors[&first.id];
    let a = if first.at_start {
        a.reversed()
    } else {
        a.clone()
    };
    if second.id == first.id {
        // both ends of one vector: closed from its end back round to its start
        let mut path = a.clone();
        bridge(&mut path, leaving(&a), a.start, p.method)?;
        path.closed = true;
        return Ok((first.id, None, path));
    }
    // the second turned to carry on from its clicked end
    let b = &doc.vectors[&second.id];
    let b = if second.at_start {
        b.clone()
    } else {
        b.reversed()
    };
    let mut path = a;
    bridge(&mut path, leaving(&b), b.start, p.method)?;
    path.segments.extend(b.segments);
    path.closed = false;
    Ok((first.id, Some(second.id), path))
}

/// Ends closer than this already meet.
const TOUCHING: f64 = 1e-9;

fn last(path: &Path) -> Point {
    path.segments.last().map_or(path.start, Segment::end)
}

/// `path` carried on from its end to `to`, arriving heading `onward` (the way the vector
/// after `to` leaves it), by `method`.
fn bridge(path: &mut Path, onward: Point, to: Point, method: Method) -> Result<(), CommandError> {
    let from = last(path);
    if from.distance(to) <= TOUCHING {
        return Ok(());
    }
    let (out, into) = (
        arriving(path).normalize_or_zero(),
        onward.normalize_or_zero(),
    );
    let spans = match method {
        Method::StraightLine => vec![Segment::Line { to }],
        Method::ExtendToMeet => extended(from, out, to, into).ok_or_else(|| {
            CommandError::Failed(
                "these ends never meet if they run on: they point apart or side by side. Join \
                 them with Straight Line or Arc instead"
                    .into(),
            )
        })?,
        Method::Arc => biarc(from, out, to, into),
    };
    path.segments.extend(spans);
    Ok(())
}

/// Lines from `from` heading `out`, and into `to` heading `into`, run on until they meet.
fn extended(from: Point, out: Point, to: Point, into: Point) -> Option<Vec<Segment>> {
    if out == Point::ZERO || into == Point::ZERO {
        return None;
    }
    // from + t·out = to − s·into
    let cross = out.perp_dot(into);
    if cross.abs() < 1e-9 {
        // one line: heading straight at each other, a straight run joins them
        let gap = to - from;
        let in_line = gap.perp_dot(out).abs() <= 1e-9 * gap.length().max(1.0);
        return (in_line && gap.dot(out) > 0.0 && out.dot(into) > 0.0)
            .then(|| vec![Segment::Line { to }]);
    }
    let gap = to - from;
    let t = gap.perp_dot(into) / cross;
    let s = out.perp_dot(gap) / cross;
    let slack = 1e-9 * gap.length().max(1.0);
    if t < -slack || s < -slack {
        return None;
    }
    let corner = from + out * t.max(0.0);
    let mut spans = vec![];
    if corner.distance(from) > TOUCHING {
        spans.push(Segment::Line { to: corner });
    }
    spans.push(Segment::Line { to });
    Some(spans)
}

/// Two arcs from `from` heading `out` to `to` heading `into`, meeting smoothly between: a
/// biarc, its two tangent lengths equal.
fn biarc(from: Point, out: Point, to: Point, into: Point) -> Vec<Segment> {
    if out == Point::ZERO || into == Point::ZERO {
        return vec![Segment::Line { to }];
    }
    let v = to - from;
    let t = out + into;
    let denominator = 2.0 * (1.0 - out.dot(into));
    let middle = if denominator.abs() < 1e-12 {
        // both heading the same way
        let along = v.dot(into);
        if along.abs() < 1e-12 {
            // side by side: two half circles, meeting halfway
            (from + to) / 2.0
        } else {
            let d = v.length_squared() / (4.0 * along);
            (from + out * d + to - into * d) / 2.0
        }
    } else {
        let vt = v.dot(t);
        let d = (-vt + (vt * vt + denominator * v.length_squared()).sqrt()) / denominator;
        (from + out * d + to - into * d) / 2.0
    };
    let mut spans = vec![arc(from, out, middle)];
    // the second arc found backwards from `to`, then turned round
    spans.push(match arc(to, -into, middle) {
        Segment::Arc { center, ccw, .. } => Segment::Arc {
            to,
            center,
            ccw: !ccw,
        },
        _ => Segment::Line { to },
    });
    spans
}

/// The arc leaving `from` heading `heading` that reaches `to`; a line when `to` lies
/// straight ahead.
fn arc(from: Point, heading: Point, to: Point) -> Segment {
    let chord = to - from;
    let left = heading.perp();
    let k = left.dot(chord);
    if k.abs() <= 1e-9 * chord.length().max(1.0) {
        return Segment::Line { to };
    }
    let radius = chord.length_squared() / (2.0 * k);
    Segment::Arc {
        to,
        center: from + left * radius,
        ccw: k > 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{document_with, square};

    fn pt(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn line(a: (f64, f64), b: (f64, f64)) -> Path {
        Path::polyline(&[pt(a.0, a.1), pt(b.0, b.1)], false)
    }

    fn params(method: Method, first: (f64, f64), second: (f64, f64)) -> Params {
        Params {
            method,
            first: Some(pt(first.0, first.1)),
            second: Some(pt(second.0, second.1)),
            within: 1.0,
        }
    }

    fn end(path: &Path) -> Point {
        path.segments.last().map_or(path.start, Segment::end)
    }

    fn near(a: Point, b: Point) -> bool {
        a.distance(b) < 1e-6
    }

    /// The document after `p`, with its one vector.
    fn after(doc: &Document, p: &Params) -> (Document, Path) {
        let change = JoinOpenEnds::run(doc, p).expect("joins");
        let mut out = doc.clone();
        out.apply(&change).unwrap();
        assert_eq!(out.vectors.len(), 1, "one vector left");
        let path = out.vectors.values().next().unwrap().clone();
        (out, path)
    }

    #[test]
    fn every_open_vector_has_two_ends_and_closed_ones_none() {
        let doc = document_with(vec![line((0.0, 0.0), (10.0, 0.0)), square(20.0, 20.0, 5.0)]);
        let ends = open_ends(&doc);
        assert_eq!(ends.len(), 2);
        assert!(
            ends.iter()
                .any(|e| e.at_start && near(e.point, pt(0.0, 0.0)))
        );
        assert!(
            ends.iter()
                .any(|e| !e.at_start && near(e.point, pt(10.0, 0.0)))
        );
    }

    #[test]
    fn the_end_nearest_a_click_within_reach_is_picked() {
        let doc = document_with(vec![line((0.0, 0.0), (10.0, 0.0))]);
        let picked = end_near(&doc, pt(9.5, 0.3), 1.0).expect("an end");
        assert!(!picked.at_start);
        assert!(
            end_near(&doc, pt(5.0, 0.0), 1.0).is_none(),
            "the middle is no end"
        );
    }

    #[test]
    fn a_straight_line_joins_two_vectors_into_one() {
        let doc = document_with(vec![
            line((0.0, 0.0), (10.0, 0.0)),
            line((12.0, 0.0), (20.0, 0.0)),
        ]);
        let (_, path) = after(
            &doc,
            &params(Method::StraightLine, (10.0, 0.0), (12.0, 0.0)),
        );
        assert!(!path.closed);
        assert_eq!(path.segments.len(), 3);
        assert!(near(path.start, pt(0.0, 0.0)));
        assert!(near(end(&path), pt(20.0, 0.0)));
    }

    #[test]
    fn the_second_vector_is_turned_round_to_carry_on_from_its_clicked_end() {
        let doc = document_with(vec![
            line((0.0, 0.0), (10.0, 0.0)),
            line((20.0, 0.0), (12.0, 0.0)),
        ]);
        let (_, path) = after(
            &doc,
            &params(Method::StraightLine, (10.0, 0.0), (12.0, 0.0)),
        );
        assert!(near(end(&path), pt(20.0, 0.0)), "{path:?}");
        // clicked the other way round, the first vector turns round instead
        let (_, path) = after(
            &doc,
            &params(Method::StraightLine, (12.0, 0.0), (10.0, 0.0)),
        );
        assert!(near(path.start, pt(20.0, 0.0)), "{path:?}");
        assert!(near(end(&path), pt(0.0, 0.0)), "{path:?}");
    }

    #[test]
    fn clicking_both_ends_of_one_vector_closes_it() {
        let corner = Path::polyline(&[pt(0.0, 0.0), pt(10.0, 0.0), pt(10.0, 10.0)], false);
        let doc = document_with(vec![corner]);
        let (_, path) = after(
            &doc,
            &params(Method::StraightLine, (10.0, 10.0), (0.0, 0.0)),
        );
        assert!(path.closed);
        assert_eq!(path.segments.len(), 3);
        assert!(near(end(&path), path.start));
    }

    #[test]
    fn touching_ends_are_joined_with_no_extra_span() {
        let doc = document_with(vec![
            line((0.0, 0.0), (10.0, 0.0)),
            line((10.0, 0.0), (10.0, 5.0)),
        ]);
        let (_, path) = after(
            &doc,
            &params(Method::StraightLine, (10.0, 0.0), (10.0, 0.0)),
        );
        assert_eq!(path.segments.len(), 2, "{path:?}");
        assert!(near(end(&path), pt(10.0, 5.0)));
    }

    #[test]
    fn extend_to_meet_runs_both_ends_on_to_a_corner() {
        let doc = document_with(vec![
            line((0.0, 0.0), (8.0, 0.0)),
            line((10.0, 2.0), (10.0, 10.0)),
        ]);
        let (_, path) = after(&doc, &params(Method::ExtendToMeet, (8.0, 0.0), (10.0, 2.0)));
        let points: Vec<Point> = std::iter::once(path.start)
            .chain(path.segments.iter().map(Segment::end))
            .collect();
        assert!(points.iter().any(|&q| near(q, pt(10.0, 0.0))), "{points:?}");
        assert!(near(end(&path), pt(10.0, 10.0)));
        assert!(
            path.segments
                .iter()
                .all(|s| matches!(s, Segment::Line { .. })),
            "straight runs"
        );
    }

    #[test]
    fn ends_that_never_meet_say_to_use_another_way() {
        let doc = document_with(vec![
            line((0.0, 0.0), (10.0, 0.0)),
            line((0.0, 5.0), (10.0, 5.0)),
        ]);
        let Err(e) = JoinOpenEnds::run(
            &doc,
            &params(Method::ExtendToMeet, (10.0, 0.0), (10.0, 5.0)),
        ) else {
            panic!("parallel ends never meet");
        };
        let why = e.to_string();
        assert!(
            why.contains("Straight Line") && why.contains("Arc"),
            "{why}"
        );
    }

    #[test]
    fn arcs_carry_on_smoothly_from_both_ends() {
        // two parallel lines heading right: the arcs turn round between them, a half circle
        let doc = document_with(vec![
            line((0.0, 0.0), (10.0, 0.0)),
            line((0.0, 5.0), (10.0, 5.0)),
        ]);
        let (_, path) = after(&doc, &params(Method::Arc, (10.0, 0.0), (10.0, 5.0)));
        let arcs = path
            .segments
            .iter()
            .filter(|s| matches!(s, Segment::Arc { .. }))
            .count();
        assert!(arcs >= 1, "{path:?}");
        let b = path.bounds();
        assert!(
            (b.max.x - 12.5).abs() < 1e-6,
            "a half circle 5 across: {b:?}"
        );
        assert!(near(end(&path), pt(0.0, 5.0)));
    }

    #[test]
    fn an_arc_between_ends_heading_at_each_other_is_a_straight_run() {
        let doc = document_with(vec![
            line((0.0, 0.0), (10.0, 0.0)),
            line((14.0, 0.0), (20.0, 0.0)),
        ]);
        let (_, path) = after(&doc, &params(Method::Arc, (10.0, 0.0), (14.0, 0.0)));
        let b = path.bounds();
        assert!(b.size().y.abs() < 1e-6, "no bulge: {b:?}");
        assert!(near(end(&path), pt(20.0, 0.0)));
    }

    #[test]
    fn a_click_away_from_any_end_says_what_to_click() {
        let doc = document_with(vec![line((0.0, 0.0), (10.0, 0.0))]);
        let Err(e) = JoinOpenEnds::run(&doc, &params(Method::StraightLine, (5.0, 5.0), (0.0, 0.0)))
        else {
            panic!("nothing there");
        };
        assert!(e.to_string().contains("circle"), "{e}");
        let Err(e) = JoinOpenEnds::run(&doc, &Params::default()) else {
            panic!("nothing clicked");
        };
        assert!(e.to_string().to_lowercase().contains("click"), "{e}");
    }

    #[test]
    fn the_same_end_twice_is_not_joined_to_itself() {
        let doc = document_with(vec![line((0.0, 0.0), (10.0, 0.0))]);
        let Err(e) = JoinOpenEnds::run(
            &doc,
            &params(Method::StraightLine, (10.0, 0.0), (10.2, 0.0)),
        ) else {
            panic!("one end");
        };
        assert!(e.to_string().contains("other end"), "{e}");
    }

    #[test]
    fn undo_puts_both_vectors_back() {
        let doc = document_with(vec![
            line((0.0, 0.0), (10.0, 0.0)),
            line((12.0, 0.0), (20.0, 0.0)),
        ]);
        let change = JoinOpenEnds::run(
            &doc,
            &params(Method::StraightLine, (10.0, 0.0), (12.0, 0.0)),
        )
        .unwrap();
        let mut out = doc.clone();
        out.apply(&change).unwrap();
        out.apply(&change.inverse()).unwrap();
        assert_eq!(out, doc);
    }

    #[test]
    fn default_params_round_trip_through_json() {
        cut_craft_core::testing::default_params_round_trip_through_json::<JoinOpenEnds>();
        cut_craft_core::testing::id_is_a_slug_and_meta_is_labelled::<JoinOpenEnds>();
        cut_craft_core::testing::schema_describes_an_object::<JoinOpenEnds>();
    }
}
