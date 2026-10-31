//! Interactive Vector Trim: click a piece of a vector and the stretch between the nearest
//! crossings with other vectors either side of the click is cut away.
//!
//! Trim Objects: the selected vectors are cut where they cross a boundary (the closed vector
//! picked last) and the pieces inside, or outside, it are cleared away.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Path, Point, inside};
use cut_craft_core::params::Xy;
use cut_craft_core::{Change, Command, CommandError, Drag, Interactive, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct InteractiveVectorTrim;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// Click on the piece to cut away
    pub at: Xy,
    /// How near the click must be to a vector
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub reach: f64,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            at: Xy::new(0.0, 0.0),
            reach: 1.0,
        }
    }
}

impl Command for InteractiveVectorTrim {
    const ID: &'static str = "interactive_vector_trim";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Interactive Vector Trim",
            group: "Edit Objects",
            icon: "drawing/edit_objects/interactive_vector_trim.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let at: Point = p.at.into();
        let Some(id) = doc.pick(at, p.reach).filter(|id| doc.editable(*id)) else {
            return Err(CommandError::Failed(
                "click on the piece of a vector to trim".into(),
            ));
        };
        let tolerance = cut_craft_core::polygon::TOLERANCE;
        let pts = doc.vectors[&id].flatten(tolerance);
        let others: Vec<Vec<Point>> = doc
            .vectors
            .iter()
            .filter(|(other, _)| **other != id && doc.shown(**other))
            .map(|(_, v)| v.flatten(tolerance))
            .collect();
        // the distance along the vector of every crossing, and of the click
        let mut lengths = vec![0.0];
        for pair in pts.windows(2) {
            lengths.push(lengths[lengths.len() - 1] + pair[0].distance(pair[1]));
        }
        let total = lengths[lengths.len() - 1];
        let mut cuts: Vec<f64> = Vec::new();
        for (i, pair) in pts.windows(2).enumerate() {
            for other in &others {
                for q in other.windows(2) {
                    if let Some(t) = crossing(pair[0], pair[1], q[0], q[1]) {
                        cuts.push(lengths[i] + t * pair[0].distance(pair[1]));
                    }
                }
            }
        }
        cuts.sort_by(f64::total_cmp);
        cuts.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        if cuts.is_empty() {
            return Err(CommandError::Failed("nothing crosses that vector".into()));
        }
        let click = along(&pts, &lengths, at);
        let closed = doc.vectors[&id].closed;
        let after = cuts.iter().copied().find(|&c| c > click);
        let before = cuts.iter().copied().rev().find(|&c| c < click);
        // what stays: the vector from `after` round (or on) to `before`
        let keep: Vec<(f64, f64)> = match (closed, before, after) {
            (true, Some(b), Some(a)) => vec![(a, b + total)],
            (true, None, Some(a)) => vec![(a, cuts[cuts.len() - 1])],
            (true, Some(b), None) => vec![(cuts[0] + total, b + total)],
            (false, b, a) => b
                .map(|b| (0.0, b))
                .into_iter()
                .chain(a.map(|a| (a, total)))
                .filter(|(lo, hi)| hi - lo > 1e-9)
                .collect(),
            (true, None, None) => unreachable!("cuts is not empty"),
        };
        let pieces: Vec<Path> = keep
            .into_iter()
            .map(|(lo, hi)| Path::polyline(&stretch(&pts, &lengths, total, lo, hi, closed), false))
            .collect();
        // ponytail: the kept pieces come back as polylines; trimming curves exactly would keep
        // their arcs and Béziers
        Ok(Change::swap(doc, vec![id], pieces))
    }
}

impl Interactive for InteractiveVectorTrim {
    fn from_drag(drag: Drag, current: &Params) -> Params {
        Params {
            at: Xy::new(drag.start.x, drag.start.y),
            ..current.clone()
        }
    }
}
pub struct TrimObjects;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Clear {
    /// Clear what lies inside the boundary
    #[default]
    Inside,
    /// Clear what lies outside the boundary
    Outside,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct TrimParams {
    /// Which side of the boundary loses its pieces
    pub clear: Clear,
}

impl Command for TrimObjects {
    const ID: &'static str = "trim_objects";
    type Params = TrimParams;

    fn meta() -> Meta {
        Meta {
            label: "Trim Objects",
            group: "Edit Objects",
            icon: "drawing/edit_objects/interactive_vector_trim.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &TrimParams) -> Result<Change, CommandError> {
        let boundary = doc
            .selection
            .last()
            .filter(|id| doc.selection.len() > 1 && doc.vectors[id].closed)
            .ok_or_else(|| {
                CommandError::Failed(
                    "select the vectors to trim, then the closed boundary last".into(),
                )
            })?;
        let tolerance = cut_craft_core::polygon::TOLERANCE;
        let ring = vec![doc.vectors[&boundary].flatten(tolerance)];
        let keep = |at: Point| inside(&ring, at) == (p.clear == Clear::Outside);
        let mut removed = vec![];
        let mut pieces = vec![];
        for &id in doc.selection.iter().filter(|id| **id != boundary) {
            let path = &doc.vectors[&id];
            let pts = path.flatten(tolerance);
            let mut lengths = vec![0.0];
            for pair in pts.windows(2) {
                lengths.push(lengths[lengths.len() - 1] + pair[0].distance(pair[1]));
            }
            let total = lengths[lengths.len() - 1];
            let mut cuts: Vec<f64> = Vec::new();
            for (i, pair) in pts.windows(2).enumerate() {
                // a closed path's flattening ends back at its start, so this goes all the way round
                for q in ring[0].windows(2) {
                    if let Some(t) = crossing(pair[0], pair[1], q[0], q[1]) {
                        cuts.push(lengths[i] + t * pair[0].distance(pair[1]));
                    }
                }
            }
            cuts.sort_by(f64::total_cmp);
            cuts.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
            if cuts.is_empty() {
                if !keep(pts[0]) {
                    removed.push(id);
                }
                continue;
            }
            // the stretches between neighbouring cuts; a closed vector's last one wraps round
            let spans: Vec<(f64, f64)> = if path.closed {
                cuts.windows(2)
                    .map(|w| (w[0], w[1]))
                    .chain([(cuts[cuts.len() - 1], cuts[0] + total)])
                    .collect()
            } else {
                std::iter::once(0.0)
                    .chain(cuts.iter().copied())
                    .zip(cuts.iter().copied().chain([total]))
                    .collect()
            };
            removed.push(id);
            for (lo, hi) in spans.into_iter().filter(|(lo, hi)| hi - lo > 1e-9) {
                let piece = stretch(&pts, &lengths, total, lo, hi, path.closed);
                let middle = stretch(
                    &pts,
                    &lengths,
                    total,
                    f64::midpoint(lo, hi),
                    f64::midpoint(lo, hi),
                    path.closed,
                )[0];
                if keep(middle) {
                    pieces.push(Path::polyline(&piece, false));
                }
            }
        }
        if removed.is_empty() {
            return Err(CommandError::Failed("nothing to clear".into()));
        }
        // ponytail: cut pieces come back as polylines, as Interactive Vector Trim's do
        Ok(Change::swap(doc, removed, pieces))
    }
}

/// Where segments ab and cd cross, as a share of ab; `None` when they do not.
#[expect(
    clippy::many_single_char_names,
    reason = "the usual names for two segments' ends"
)]
/// Where segments ab and cd cross, as a share of ab; `None` when they do not.
fn crossing(a: Point, b: Point, c: Point, d: Point) -> Option<f64> {
    let (r, s) = (b - a, d - c);
    let den = r.perp_dot(s);
    if den.abs() < 1e-12 {
        return None;
    }
    let t = (c - a).perp_dot(s) / den;
    let u = (c - a).perp_dot(r) / den;
    ((0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)).then_some(t)
}

/// The distance along the polyline of its point nearest `at`.
fn along(pts: &[Point], lengths: &[f64], at: Point) -> f64 {
    pts.windows(2)
        .enumerate()
        .map(|(i, w)| {
            let d = w[1] - w[0];
            let t = ((at - w[0]).dot(d) / d.length_squared().max(1e-24)).clamp(0.0, 1.0);
            ((w[0] + d * t).distance(at), lengths[i] + t * d.length())
        })
        .min_by(|x, y| x.0.total_cmp(&y.0))
        .map_or(0.0, |(_, s)| s)
}

/// The polyline's points from distance `lo` to `hi` (wrapping round a closed one).
fn stretch(
    pts: &[Point],
    lengths: &[f64],
    total: f64,
    lo: f64,
    hi: f64,
    closed: bool,
) -> Vec<Point> {
    let at = |s: f64| {
        let s = if closed { s.rem_euclid(total) } else { s };
        let i = lengths.partition_point(|&l| l < s).clamp(1, pts.len() - 1);
        let span = lengths[i] - lengths[i - 1];
        let part = if span > 0.0 {
            (s - lengths[i - 1]) / span
        } else {
            0.0
        };
        pts[i - 1].lerp(pts[i], part)
    };
    let laps: &[f64] = if closed { &[0.0, total] } else { &[0.0] };
    let mut out = vec![at(lo)];
    for lap in laps {
        out.extend(
            lengths
                .iter()
                .zip(pts)
                .filter(|(l, _)| (lo..hi).contains(&(*l + lap)) && **l + lap > lo)
                .map(|(_, q)| *q),
        );
    }
    out.push(at(hi));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{document_with, run_on};

    /// A horizontal line crossed by two vertical ones at x = 30 and x = 70.
    fn crossed() -> Document {
        let line = |a: (f64, f64), b: (f64, f64)| {
            Path::polyline(&[Point::new(a.0, a.1), Point::new(b.0, b.1)], false)
        };
        document_with(vec![
            line((0.0, 50.0), (100.0, 50.0)),
            line((30.0, 0.0), (30.0, 100.0)),
            line((70.0, 0.0), (70.0, 100.0)),
        ])
    }

    fn click(x: f64, y: f64) -> Params {
        Params {
            at: Xy::new(x, y),
            reach: 1.0,
        }
    }

    #[test]
    fn contract() {
        cut_craft_core::testing::id_is_a_slug_and_meta_is_labelled::<InteractiveVectorTrim>();
        cut_craft_core::testing::schema_describes_an_object::<InteractiveVectorTrim>();
        cut_craft_core::testing::default_params_round_trip_through_json::<InteractiveVectorTrim>();
        let doc = crossed();
        let change = InteractiveVectorTrim::run(&doc, &click(50.0, 50.0)).unwrap();
        let mut done = doc.clone();
        done.apply(&change).unwrap();
        done.apply(&change.inverse()).unwrap();
        assert_eq!(done, doc);
    }

    #[test]
    fn the_middle_piece_goes_and_the_ends_stay() {
        let mut doc = crossed();
        run_on::<InteractiveVectorTrim>(&mut doc, &click(50.0, 50.0));
        let pieces: Vec<_> = doc
            .vectors
            .values()
            .filter(|v| v.bounds().size().y < 1.0)
            .collect();
        assert_eq!(pieces.len(), 2);
        let xs: Vec<f64> = pieces.iter().map(|v| v.bounds().size().x).collect();
        assert!(xs.iter().all(|x| (x - 30.0).abs() < 1e-9), "{xs:?}");
    }

    #[test]
    fn trim_objects_clears_inside_or_outside_the_boundary() {
        let boundary = cut_craft_core::testing::square(20.0, 20.0, 60.0);
        let line = Path::polyline(&[Point::new(0.0, 50.0), Point::new(100.0, 50.0)], false);
        let far = cut_craft_core::testing::square(200.0, 200.0, 5.0);
        let doc = document_with(vec![line, far, boundary.clone()]);
        let mut inner = doc.clone();
        run_on::<TrimObjects>(&mut inner, &TrimParams::default());
        // the line's middle and nothing else goes; the far square and the boundary stay
        let mut widths: Vec<f64> = inner
            .vectors
            .values()
            .map(|v| v.bounds().size().x)
            .collect();
        widths.sort_by(f64::total_cmp);
        assert_eq!(widths.len(), 4, "{widths:?}");
        assert!((widths[1] - 20.0).abs() < 1e-9 && (widths[2] - 20.0).abs() < 1e-9);
        assert!(inner.vectors.values().any(|v| *v == boundary));
        let mut outer = doc.clone();
        run_on::<TrimObjects>(
            &mut outer,
            &TrimParams {
                clear: Clear::Outside,
            },
        );
        let mut widths: Vec<f64> = outer
            .vectors
            .values()
            .map(|v| v.bounds().size().x)
            .collect();
        widths.sort_by(f64::total_cmp);
        assert_eq!(widths.len(), 2, "{widths:?}");
        assert!((widths[0] - 60.0).abs() < 1e-9 && (widths[1] - 60.0).abs() < 1e-9);
        let single = document_with(vec![Path::polyline(
            &[Point::new(0.0, 0.0), Point::new(1.0, 1.0)],
            false,
        )]);
        assert!(TrimObjects::run(&single, &TrimParams::default()).is_err());
    }

    #[test]
    fn an_end_piece_goes_and_a_circle_loses_an_arc() {
        let mut doc = crossed();
        run_on::<InteractiveVectorTrim>(&mut doc, &click(10.0, 50.0));
        let line = doc
            .vectors
            .values()
            .find(|v| v.bounds().size().y < 1.0)
            .unwrap()
            .bounds();
        assert!((line.min.x - 30.0).abs() < 1e-9 && (line.max.x - 100.0).abs() < 1e-9);
        let mut ring = document_with(vec![
            Path::circle(Point::new(50.0, 50.0), 20.0),
            Path::polyline(&[Point::new(0.0, 50.0), Point::new(100.0, 50.0)], false),
        ]);
        run_on::<InteractiveVectorTrim>(&mut ring, &click(50.0, 70.0));
        let left = ring
            .vectors
            .values()
            .find(|v| !v.closed && v.bounds().size().y > 1.0)
            .unwrap();
        assert!(
            left.bounds().max.y < 50.0 + 1e-6,
            "only the lower half stays"
        );
        assert!(InteractiveVectorTrim::run(&ring, &click(500.0, 500.0)).is_err());
    }
}
