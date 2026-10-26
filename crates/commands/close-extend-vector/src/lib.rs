//! Close / Extend Vector: closes the selected open vectors, with a straight line from end to
//! start or by extending their end lines until they meet.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Path, Point, Segment, arc_to_cubics};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct CloseExtendVector;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    /// A straight line from the end back to the start
    #[default]
    Line,
    /// Lengthen the first and last lines until they meet; when they never do, close with a
    /// straight line
    Extend,
    /// A smooth curve leaving the end and reaching the start along their own directions
    Smooth,
    /// Move the start and the end to the point halfway between them
    CommonPoint,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub method: Method,
    /// Only close vectors whose ends are at most this far apart; 0 closes any
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub max_gap: f64,
}

/// Ends closer than this already meet.
const TOUCHING: f64 = 1e-9;

impl Command for CloseExtendVector {
    const ID: &'static str = "close_extend_vector";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Close / Extend Vector",
            group: "Edit Objects",
            icon: "drawing/create_vectors/close_extend_vector.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if p.max_gap < 0.0 {
            return Err(CommandError::InvalidParams(
                "max gap must not be negative".into(),
            ));
        }
        let (ids, paths) = doc.selected_paths(|path| !path.closed && !path.segments.is_empty());
        if ids.is_empty() {
            return Err(CommandError::Failed("select open vectors to close".into()));
        }
        let mut c = Change::builder(doc);
        let mut closed = 0;
        for (id, path) in ids.into_iter().zip(paths) {
            let gap = (end(&path) - path.start).length();
            if p.max_gap > 0.0 && gap > p.max_gap {
                continue;
            }
            let joined = match p.method {
                Method::Extend if gap > TOUCHING => extended(&path),
                Method::Smooth if gap > TOUCHING => smoothed(&path),
                Method::CommonPoint if gap > TOUCHING => Some(met(path.clone())),
                _ => None,
            };
            c.replace(id, joined.unwrap_or_else(|| with_line(path)));
            closed += 1;
        }
        if closed == 0 {
            return Err(CommandError::Failed(
                "no selected vector's ends are within the max gap".into(),
            ));
        }
        Ok(c.finish())
    }
}

fn end(path: &Path) -> Point {
    path.segments.last().map_or(path.start, Segment::end)
}

/// `path` closed by a line from its end to its start, or just marked closed when they meet.
fn with_line(mut path: Path) -> Path {
    let start = path.start;
    let last = path
        .segments
        .last_mut()
        .expect("open vectors have segments");
    if (last.end() - start).length() <= TOUCHING {
        // snap the end exactly onto the start
        match last {
            Segment::Line { to } | Segment::Arc { to, .. } | Segment::Cubic { to, .. } => {
                *to = start;
            }
        }
    } else {
        path.segments.push(Segment::Line { to: start });
    }
    path.closed = true;
    path
}

/// `path` with its start and end both moved to the point halfway between them, closed there.
/// Curves at either end keep their shape near the other end; arcs there become curves.
fn met(mut path: Path) -> Path {
    let middle = (path.start + end(&path)) / 2.0;
    // arcs cannot move an end and stay arcs: make the end spans curves first
    let as_curves = |segments: &mut Vec<Segment>, at: usize, from: Point| {
        if let Segment::Arc { to, center, ccw } = segments[at] {
            let curves = arc_to_cubics(from, to, center, ccw);
            segments.splice(at..=at, curves);
        }
    };
    let start = path.start;
    as_curves(&mut path.segments, 0, start);
    let last = path.segments.len() - 1;
    let from = if last == 0 {
        path.start
    } else {
        path.segments[last - 1].end()
    };
    as_curves(&mut path.segments, last, from);
    let shift_start = middle - path.start;
    if let Some(Segment::Cubic { c1, .. }) = path.segments.first_mut() {
        *c1 += shift_start;
    }
    let shift_end = middle - end(&path);
    match path
        .segments
        .last_mut()
        .expect("open vectors have segments")
    {
        Segment::Cubic { c2, to, .. } => {
            *c2 += shift_end;
            *to = middle;
        }
        Segment::Line { to } | Segment::Arc { to, .. } => *to = middle,
    }
    path.start = middle;
    path.closed = true;
    path
}

/// `path` closed by a curve leaving its end, and reaching its start, along their own
/// directions; `None` when either direction is unknown (a span of no length).
fn smoothed(path: &Path) -> Option<Path> {
    let (start, finish) = (path.start, end(path));
    let first = *path.segments.first()?;
    let last = *path.segments.last()?;
    let before_last = path
        .segments
        .len()
        .checked_sub(2)
        .map_or(path.start, |i| path.segments[i].end());
    // the direction travelled on leaving the start, and on arriving at the end
    let leaving = match first {
        Segment::Line { to } => to - start,
        Segment::Cubic { c1, .. } => c1 - start,
        Segment::Arc { center, ccw, .. } => turn(start - center, ccw),
    };
    let arriving = match last {
        Segment::Line { to } => to - before_last,
        Segment::Cubic { c2, to, .. } => to - c2,
        Segment::Arc { to, center, ccw } => turn(to - center, ccw),
    };
    let (leaving, arriving) = (leaving.try_normalize()?, arriving.try_normalize()?);
    let reach = (start - finish).length() / 3.0;
    let mut closed = path.clone();
    closed.segments.push(Segment::Cubic {
        c1: finish + arriving * reach,
        c2: start - leaving * reach,
        to: start,
    });
    closed.closed = true;
    Some(closed)
}

/// The direction of travel round an arc at the point `radius` from its centre.
fn turn(radius: Point, ccw: bool) -> Point {
    if ccw { radius.perp() } else { -radius.perp() }
}

/// `path` with its first and last lines lengthened to where they meet, closed there; `None`
/// when either end is not a line or the lines never meet ahead of both ends.
fn extended(path: &Path) -> Option<Path> {
    let count = path.segments.len();
    if count < 2 {
        return None;
    }
    let (Segment::Line { to: second }, Segment::Line { to: last_end }) =
        (path.segments[0], path.segments[count - 1])
    else {
        return None;
    };
    let before_last = path.segments[count - 2].end();
    // the start runs on backwards, away from the second point; the end runs on forwards
    let (from_start, back) = (path.start, path.start - second);
    let (from_end, on) = (last_end, last_end - before_last);
    let cross = back.perp_dot(on);
    if cross.abs() <= TOUCHING * back.length() * on.length() {
        return None;
    }
    let along_start = (from_end - from_start).perp_dot(on) / cross;
    let along_end = (from_end - from_start).perp_dot(back) / cross;
    if along_start < 0.0 || along_end < 0.0 {
        return None;
    }
    let meet = from_start + back * along_start;
    let mut segments = path.segments.clone();
    segments[count - 1] = Segment::Line { to: meet };
    Some(Path {
        start: meet,
        segments,
        closed: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{document_with, square};

    cut_craft_core::contract_tests!(CloseExtendVector);

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    /// An open U: up the left, across the top, down the right; its ends 10 apart.
    fn u_shape() -> Path {
        Path::polyline(
            &[p(0.0, 0.0), p(0.0, 10.0), p(10.0, 10.0), p(10.0, 0.0)],
            false,
        )
    }

    /// Two sides of a triangle whose tip, at (5, 10), is missing.
    fn no_tip() -> Path {
        Path::polyline(
            &[p(2.0, 4.0), p(0.0, 0.0), p(10.0, 0.0), p(8.0, 4.0)],
            false,
        )
    }

    fn closed(paths: Vec<Path>, params: &Params) -> Result<Vec<Path>, String> {
        let mut doc = document_with(paths);
        let change = CloseExtendVector::run(&doc, params).map_err(|e| e.to_string())?;
        doc.apply(&change).unwrap();
        Ok(doc.vectors.values().cloned().collect())
    }

    fn line_ends(path: &Path) -> Vec<Point> {
        path.segments.iter().map(Segment::end).collect()
    }

    #[test]
    fn a_line_closes_the_gap_from_end_to_start() {
        let [path] = closed(vec![u_shape()], &Params::default())
            .unwrap()
            .try_into()
            .unwrap();
        assert!(path.closed);
        assert_eq!(
            line_ends(&path),
            [p(0.0, 10.0), p(10.0, 10.0), p(10.0, 0.0), p(0.0, 0.0)]
        );
    }

    #[test]
    fn extending_runs_the_end_lines_on_until_they_meet() {
        let extend = Params {
            method: Method::Extend,
            ..Params::default()
        };
        let [path] = closed(vec![no_tip()], &extend).unwrap().try_into().unwrap();
        assert!(path.closed);
        assert_eq!(path.start, p(5.0, 10.0));
        assert_eq!(line_ends(&path), [p(0.0, 0.0), p(10.0, 0.0), p(5.0, 10.0)]);
        // the sides of the U are parallel and never meet: closed with a line instead
        let [u] = closed(vec![u_shape()], &extend)
            .unwrap()
            .try_into()
            .unwrap();
        assert!(u.closed);
        assert_eq!(u.segments.len(), 4);
    }

    #[test]
    fn smooth_closes_with_a_curve_carrying_on_from_each_end() {
        let smooth = Params {
            method: Method::Smooth,
            ..Params::default()
        };
        let [path] = closed(vec![u_shape()], &smooth)
            .unwrap()
            .try_into()
            .unwrap();
        assert!(path.closed);
        let Some(&Segment::Cubic { c1, c2, to }) = path.segments.last() else {
            panic!("closed with a curve: {:?}", path.segments.last());
        };
        // down out of the right side, up into the left side, a third of the gap away
        assert_eq!(
            (c1, c2, to),
            (p(10.0, -10.0 / 3.0), p(0.0, -10.0 / 3.0), p(0.0, 0.0))
        );
        assert!(
            (path.bounds().min.y + 2.5).abs() < 1e-9,
            "{:?}",
            path.bounds()
        );
    }

    #[test]
    fn common_point_moves_both_ends_halfway() {
        let meet = Params {
            method: Method::CommonPoint,
            ..Params::default()
        };
        let [path] = closed(vec![u_shape()], &meet).unwrap().try_into().unwrap();
        assert!(path.closed);
        assert_eq!(path.start, p(5.0, 0.0));
        assert_eq!(
            line_ends(&path),
            [p(0.0, 10.0), p(10.0, 10.0), p(5.0, 0.0)],
            "the sides now lean in to meet"
        );
    }

    #[test]
    fn ends_that_already_touch_are_only_marked_closed() {
        let p0 = p(0.0, 0.0);
        let almost = Path::polyline(&[p0, p(10.0, 0.0), p(10.0, 10.0), p0], false);
        let [path] = closed(vec![almost], &Params::default())
            .unwrap()
            .try_into()
            .unwrap();
        assert!(path.closed);
        assert_eq!(path.segments.len(), 3);
    }

    #[test]
    fn max_gap_leaves_vectors_whose_ends_are_further_apart() {
        let tight = Params {
            max_gap: 5.0,
            ..Params::default()
        };
        let err = closed(vec![u_shape()], &tight).unwrap_err();
        assert!(err.contains("max gap"), "{err}");
        let enough = Params {
            max_gap: 10.0,
            ..Params::default()
        };
        assert!(closed(vec![u_shape()], &enough).unwrap()[0].closed);
    }

    #[test]
    fn closed_vectors_are_left_alone_and_are_not_enough() {
        let paths = closed(vec![square(20.0, 20.0, 5.0), u_shape()], &Params::default()).unwrap();
        assert_eq!(paths[0], square(20.0, 20.0, 5.0));
        assert!(paths[1].closed);
        let err = closed(vec![square(0.0, 0.0, 5.0)], &Params::default()).unwrap_err();
        assert!(err.contains("open vectors"), "{err}");
    }
}
