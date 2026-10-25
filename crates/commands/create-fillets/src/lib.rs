//! Create Fillets: rounds the sharp corners of the selected vectors, or cuts dog-bone or
//! T-bone relief into them so a round cutter can clear the corner fully.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Path, Point, Segment, winding};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct CreateFillets;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// A rounded corner: an arc tangent to both sides
    #[default]
    Normal,
    /// A cutter-sized circle into the corner, along the line that halves it
    DogBone,
    /// A cutter-sized notch into the longer side of the corner
    TBone,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Corners {
    /// Every sharp corner
    #[default]
    All,
    /// Corners pointing out of a closed shape: those a cutter inside a pocket cannot reach
    Outer,
    /// Corners pointing into a closed shape: those a cutter outside the shape cannot reach
    Inner,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub kind: Kind,
    /// Of the fillet, or of the cutter for dog-bones and T-bones
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub radius: f64,
    /// Open vectors have no inside, so they get every corner
    pub corners: Corners,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            kind: Kind::Normal,
            radius: 3.0,
            corners: Corners::All,
        }
    }
}

impl Command for CreateFillets {
    const ID: &'static str = "create_fillets";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Create Fillets",
            group: "Create Vectors",
            icon: "drawing/create_vectors/create_fillets.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("radius", p.radius)?;
        let (ids, paths) = doc.selected_paths(|_| true);
        if ids.is_empty() {
            return Err(CommandError::Failed(
                "select vectors whose corners to fillet".into(),
            ));
        }
        let (mut corners, mut done) = (0, 0);
        let mut c = Change::builder(doc);
        for (id, path) in ids.into_iter().zip(&paths) {
            let found = sharp_corners(path, p.corners);
            corners += found.len();
            let plans: Vec<(usize, Corner)> = found
                .into_iter()
                .filter_map(|(at, corner)| plan(corner, p).map(|c| (at, c)))
                .collect();
            if !plans.is_empty() {
                done += plans.len();
                c.replace(id, rebuilt(path, &plans));
            }
        }
        if corners == 0 {
            return Err(CommandError::Failed(
                "the selected vectors have no sharp corners of that kind to fillet".into(),
            ));
        }
        if done == 0 {
            return Err(CommandError::Failed(
                "the radius is too large for every corner: try a smaller one".into(),
            ));
        }
        Ok(c.finish())
    }
}

/// Below this, two sides are taken as one straight line (or doubling straight back).
const STRAIGHT: f64 = 1e-6;

/// A sharp corner `at` between two straight sides, and how a fillet fills it.
#[derive(Clone, Copy, Debug)]
struct Corner {
    at: Point,
    /// Unit vectors from the corner back along the side in, and on along the side out.
    back: Point,
    on: Point,
    /// Lengths of the side in and the side out.
    len_in: f64,
    len_out: f64,
    /// Replaced by the fillet: from `at + back * trim_in` to `at + on * trim_out`.
    trim_in: f64,
    trim_out: f64,
    center: Point,
    /// A point the fillet passes through, which fixes which way round it goes.
    through: Point,
}

/// The sharp line-to-line corners of `path` that `which` asks for: (index of the segment
/// ending at the corner, the corner).
fn sharp_corners(path: &Path, which: Corners) -> Vec<(usize, Corner)> {
    let n = path.segments.len();
    let start_of = |k: usize| {
        if k == 0 {
            path.start
        } else {
            path.segments[k - 1].end()
        }
    };
    // which way a closed path turns overall (true: counter-clockwise); open ones have no inside
    let ccw = path.closed.then(|| winding(&path.flatten(0.01)) > 0.0);
    let last = if path.closed { n } else { n.saturating_sub(1) };
    let mut found = vec![];
    for k in 0..last {
        let next = (k + 1) % n;
        let (Segment::Line { to: at }, Segment::Line { to: end }) =
            (path.segments[k], path.segments[next])
        else {
            continue;
        };
        let (into, out) = (at - start_of(k), end - at);
        let (len_in, len_out) = (into.length(), out.length());
        if len_in < STRAIGHT || len_out < STRAIGHT {
            continue;
        }
        let turn = into.perp_dot(out) / (len_in * len_out);
        if turn.abs() < STRAIGHT {
            continue;
        }
        let wanted = match (which, ccw) {
            (Corners::All, _) | (_, None) => true,
            (Corners::Outer, Some(ccw)) => (turn > 0.0) == ccw,
            (Corners::Inner, Some(ccw)) => (turn > 0.0) != ccw,
        };
        if wanted {
            let corner = Corner {
                at,
                back: -into / len_in,
                on: out / len_out,
                len_in,
                len_out,
                trim_in: 0.0,
                trim_out: 0.0,
                center: at,
                through: at,
            };
            found.push((k, corner));
        }
    }
    found
}

/// Where the fillet goes in `corner`. A fillet may use at most half of each side, so
/// neighbouring corners never overlap; a radius needing more is cut to the largest that
/// fits, as Aspire does. `None` when not even a tiny one fits.
fn plan(mut corner: Corner, p: &Params) -> Option<Corner> {
    let (back, on) = (corner.back, corner.on);
    let cos = back.dot(on).clamp(-1.0, 1.0);
    // halves the angle between the sides, pointing into it
    let bisector = (back + on).normalize();
    let half = cos.acos() / 2.0;
    let (len_in, len_out) = (corner.len_in / 2.0, corner.len_out / 2.0);
    let largest = match p.kind {
        Kind::Normal => len_in.min(len_out) * half.tan(),
        Kind::DogBone => len_in.min(len_out) / (2.0 * half.cos()),
        Kind::TBone => {
            let (notched, other) = if corner.len_in >= corner.len_out {
                (len_in, len_out)
            } else {
                (len_out, len_in)
            };
            let across = if cos > 0.0 {
                other / (2.0 * cos)
            } else {
                f64::INFINITY
            };
            (notched / 2.0).min(across)
        }
    };
    let r = p.radius.min(largest);
    if r <= STRAIGHT {
        return None;
    }
    let (trim_in, trim_out, center) = match p.kind {
        Kind::Normal => {
            let t = r / half.tan();
            (t, t, corner.at + bisector * (r / half.sin()))
        }
        Kind::DogBone => {
            let t = 2.0 * r * half.cos();
            (t, t, corner.at + bisector * r)
        }
        Kind::TBone => {
            let other = (2.0 * r * cos).max(0.0);
            if corner.len_in >= corner.len_out {
                (2.0 * r, other, corner.at + back * r)
            } else {
                (other, 2.0 * r, corner.at + on * r)
            }
        }
    };
    corner.trim_in = trim_in;
    corner.trim_out = trim_out;
    corner.center = center;
    corner.through = center - bisector * r;
    Some(corner)
}

/// `path` with each planned corner (keyed by the segment ending there) cut back and joined
/// by its arc.
fn rebuilt(path: &Path, plans: &[(usize, Corner)]) -> Path {
    let n = path.segments.len();
    let at = |k: usize| plans.iter().find(|(i, _)| *i == k).map(|(_, c)| c);
    let start = match at(n - 1) {
        Some(c) if path.closed => c.at + c.on * c.trim_out,
        _ => path.start,
    };
    let mut segments = Vec::with_capacity(n + plans.len());
    let mut from = start;
    for (k, segment) in path.segments.iter().enumerate() {
        let Some(c) = at(k) else {
            segments.push(*segment);
            from = segment.end();
            continue;
        };
        // a planned corner sits between two lines: this one ends where the fillet starts
        let begin = c.at + c.back * c.trim_in;
        let end = c.at + c.on * c.trim_out;
        if (begin - from).length() > STRAIGHT {
            segments.push(Segment::Line { to: begin });
        }
        segments.push(Segment::Arc {
            to: end,
            center: c.center,
            ccw: ccw_through(begin, c.through, end, c.center),
        });
        from = end;
        // the next line starts at the fillet's end, so its own start needs no change
    }
    Path {
        start,
        segments,
        closed: path.closed,
    }
}

/// Whether the arc around `center` from `from` to `to` that passes `through` runs
/// counter-clockwise.
fn ccw_through(from: Point, through: Point, to: Point, center: Point) -> bool {
    let angle = |p: Point| (p - center).to_angle();
    let ccw_from = |p: Point| (angle(p) - angle(from)).rem_euclid(std::f64::consts::TAU);
    ccw_from(through) < ccw_from(to)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::{Path, Point, Segment, polyline_distance, winding};
    use cut_craft_core::testing::{document_with, square};

    cut_craft_core::contract_tests!(CreateFillets);

    const EPS: f64 = 1e-9;

    fn filleted(paths: Vec<Path>, p: &Params) -> Result<Vec<Path>, String> {
        let mut doc = document_with(paths);
        let change = CreateFillets::run(&doc, p).map_err(|e| e.to_string())?;
        doc.apply(&change).unwrap();
        Ok(doc.vectors.values().cloned().collect())
    }

    fn params(kind: Kind, radius: f64, corners: Corners) -> Params {
        Params {
            kind,
            radius,
            corners,
        }
    }

    fn arcs(path: &Path) -> Vec<Point> {
        path.segments
            .iter()
            .filter_map(|s| match *s {
                Segment::Arc { center, .. } => Some(center),
                _ => None,
            })
            .collect()
    }

    fn area(path: &Path) -> f64 {
        winding(&path.flatten(1e-4)).abs() / 2.0
    }

    #[test]
    fn round_fillets_are_tangent_arcs_inside_each_corner() {
        let [path] = filleted(vec![square(0.0, 0.0, 10.0)], &Params::default())
            .unwrap()
            .try_into()
            .unwrap();
        assert!(path.closed);
        assert_eq!(path.segments.len(), 8, "four lines and four arcs");
        let mut centers = arcs(&path);
        centers.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
        let p = Point::new;
        assert_eq!(
            centers,
            [p(3.0, 3.0), p(3.0, 7.0), p(7.0, 3.0), p(7.0, 7.0)]
        );
        let b = path.bounds();
        assert!((b.min - p(0.0, 0.0)).length() < EPS && (b.max - p(10.0, 10.0)).length() < EPS);
        // each corner loses a 3×3 square less a quarter circle
        let expected = 100.0 - 4.0 * (9.0 - std::f64::consts::PI * 9.0 / 4.0);
        assert!((area(&path) - expected).abs() < 1e-2, "{}", area(&path));
    }

    #[test]
    fn dog_bones_pass_through_the_corner_so_the_cutter_reaches_it() {
        let r = 1.0;
        let [path] = filleted(
            vec![square(0.0, 0.0, 10.0)],
            &params(Kind::DogBone, r, Corners::All),
        )
        .unwrap()
        .try_into()
        .unwrap();
        let points = path.flatten(1e-4);
        for corner in [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)] {
            let corner = Point::new(corner.0, corner.1);
            assert!(polyline_distance(&points, corner) < 1e-3, "{corner}");
        }
        // the circles sit inside the corners, centre r from the corner along the bisector
        let reach = r * (1.0 - std::f64::consts::FRAC_1_SQRT_2);
        let b = path.bounds();
        assert!((b.max.x - (10.0 + reach)).abs() < 1e-6, "{}", b.max.x);
        assert!((b.min.y + reach).abs() < 1e-6, "{}", b.min.y);
    }

    #[test]
    fn t_bones_notch_into_the_longer_side() {
        let r = 1.0;
        let p = Point::new;
        let wide = Path::polyline(
            &[p(0.0, 0.0), p(20.0, 0.0), p(20.0, 10.0), p(0.0, 10.0)],
            true,
        );
        let [path] = filleted(vec![wide], &params(Kind::TBone, r, Corners::All))
            .unwrap()
            .try_into()
            .unwrap();
        // the long sides are top and bottom: the notches go up and down, not sideways
        let b = path.bounds();
        assert!((b.min.y + r).abs() < 1e-6, "{}", b.min.y);
        assert!((b.max.y - (10.0 + r)).abs() < 1e-6, "{}", b.max.y);
        assert!(
            b.min.x.abs() < 1e-6 && (b.max.x - 20.0).abs() < 1e-6,
            "{b:?}"
        );
        assert_eq!(arcs(&path).len(), 4);
    }

    #[test]
    fn corners_picks_outer_or_inner_corners_of_a_closed_shape() {
        let p = Point::new;
        // an L: five outer corners and one inner one at (10, 10)
        let ell = Path::polyline(
            &[
                p(0.0, 0.0),
                p(20.0, 0.0),
                p(20.0, 10.0),
                p(10.0, 10.0),
                p(10.0, 20.0),
                p(0.0, 20.0),
            ],
            true,
        );
        let count = |corners| {
            let [path] = filleted(vec![ell.clone()], &params(Kind::Normal, 2.0, corners))
                .unwrap()
                .try_into()
                .unwrap();
            arcs(&path).len()
        };
        assert_eq!(count(Corners::All), 6);
        assert_eq!(count(Corners::Outer), 5);
        assert_eq!(count(Corners::Inner), 1);
        // the same holds when the L is drawn the other way round
        let [path] = filleted(
            vec![ell.reversed()],
            &params(Kind::Normal, 2.0, Corners::Inner),
        )
        .unwrap()
        .try_into()
        .unwrap();
        assert_eq!(arcs(&path), [p(12.0, 12.0)]);
    }

    #[test]
    fn open_vectors_keep_their_ends_and_round_the_corners_between() {
        let p = Point::new;
        let vee = Path::polyline(&[p(0.0, 10.0), p(10.0, 0.0), p(20.0, 10.0)], false);
        let [path] = filleted(vec![vee], &params(Kind::Normal, 2.0, Corners::Outer))
            .unwrap()
            .try_into()
            .unwrap();
        assert!(!path.closed);
        assert_eq!(path.start, p(0.0, 10.0));
        assert_eq!(path.segments.last().unwrap().end(), p(20.0, 10.0));
        assert_eq!(arcs(&path).len(), 1);
    }

    #[test]
    fn a_radius_too_large_is_cut_to_the_largest_that_fits() {
        // half of each 10 mm side is the most a corner may use: radius 5 rounds it fully
        let [path] = filleted(
            vec![square(0.0, 0.0, 10.0)],
            &params(Kind::Normal, 50.0, Corners::All),
        )
        .unwrap()
        .try_into()
        .unwrap();
        let centres = arcs(&path);
        assert_eq!(centres.len(), 4);
        assert!(
            centres
                .iter()
                .all(|c| (*c - Point::new(5.0, 5.0)).length() < EPS)
        );
        assert!(
            path.segments
                .iter()
                .all(|s| matches!(s, Segment::Arc { .. }))
        );
    }

    #[test]
    fn curves_have_no_sharp_corners() {
        let err = filleted(
            vec![Path::circle(Point::new(0.0, 0.0), 5.0)],
            &Params::default(),
        )
        .unwrap_err();
        assert!(err.contains("no sharp corners"), "{err}");
    }

    #[test]
    fn needs_a_positive_radius_and_a_selection() {
        assert!(
            filleted(
                vec![square(0.0, 0.0, 10.0)],
                &params(Kind::Normal, 0.0, Corners::All)
            )
            .is_err()
        );
        let doc = Document::default();
        assert!(CreateFillets::run(&doc, &Params::default()).is_err());
    }
}
