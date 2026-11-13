//! Create Fillets: rounds the sharp corners of the selected vectors, cuts dog-bone or T-bone
//! relief into them so a round cutter can clear the corner fully, or loops a plasma or drag
//! knife round the outside of them.

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::geometry::{Path, Point, Segment, polyline_distance, winding};
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
    /// The sides run on past the corner into a loop outside it, so a plasma torch or drag
    /// knife turns there instead of on the corner
    PlasmaDragKnife,
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
    /// What is cut into each corner
    pub kind: Kind,
    /// Of the fillet, or of the cutter for dog-bones and T-bones
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub radius: f64,
    /// Open vectors have no inside, so they get every corner
    pub corners: Corners,
    /// A click in the 2D view instead of the selection's corners: the nearest sharp corner
    /// within `within` of it is filleted, or the fillet there is taken out
    #[serde(default)]
    #[schemars(skip)]
    pub at: Option<Point>,
    #[serde(default)]
    #[schemars(skip)]
    pub within: f64,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            kind: Kind::Normal,
            radius: 3.0,
            corners: Corners::All,
            at: None,
            within: 0.0,
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
            icon: "drawing/create_vectors/create_fillets.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("radius", p.radius)?;
        if let Some(at) = p.at {
            let (id, path, note) = at_click(doc, p, at)?;
            let mut c = Change::builder(doc);
            c.replace(id, path);
            if let Some(note) = note {
                c.note(note);
            }
            return Ok(c.finish());
        }
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
            // corners are filleted in turn, as if clicked one after another: each may use
            // what is left of its sides after the corners before it
            let n = path.segments.len();
            let mut used = vec![0.0; n];
            let mut plans: Vec<(usize, Corner)> = vec![];
            for (k, corner) in found {
                let next = (k + 1) % n;
                let left = (corner.len_in - used[k], corner.len_out - used[next]);
                if let Some(planned) = plan(corner, p, left) {
                    used[k] += planned.trim_in.max(0.0);
                    used[next] += planned.trim_out.max(0.0);
                    plans.push((k, planned));
                }
            }
            if !plans.is_empty() {
                done += plans.len();
                c.replace(id, rebuilt(path, &plans));
            }
        }
        if corners == 0 {
            return Err(CommandError::Failed(
                "the selected vectors have no sharp corners of that kind to fillet. Pick another corner type, or select other vectors".into(),
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
    /// Clicked: whether the click was nearer the side in than the side out, which is the
    /// side a T-bone notches; `None` notches the longer side.
    click_side: Option<bool>,
}

impl Corner {
    /// Whether a T-bone notches the side in (else the side out).
    fn notches_side_in(&self) -> bool {
        self.click_side.unwrap_or(self.len_in >= self.len_out)
    }
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
                click_side: None,
            };
            found.push((k, corner));
        }
    }
    found
}

/// Where the fillet goes in `corner`, with `left` of the side in and the side out still
/// straight. A radius needing more is cut to the largest that fits.
/// `None` when not even a tiny one fits.
fn plan(mut corner: Corner, p: &Params, left: (f64, f64)) -> Option<Corner> {
    let (back, on) = (corner.back, corner.on);
    let cos = back.dot(on).clamp(-1.0, 1.0);
    // halves the angle between the sides, pointing into it
    let bisector = (back + on).normalize();
    let half = cos.acos() / 2.0;
    let (len_in, len_out) = (left.0.max(0.0), left.1.max(0.0));
    let largest = match p.kind {
        // past the corner, on the sides' extensions: nothing of the sides is used
        Kind::PlasmaDragKnife => f64::INFINITY,
        Kind::Normal => len_in.min(len_out) * half.tan(),
        Kind::DogBone => len_in.min(len_out) / (2.0 * half.cos()),
        Kind::TBone => {
            let (notched, other) = if corner.notches_side_in() {
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
        Kind::PlasmaDragKnife => {
            // the normal fillet mirrored through the corner, round the long way
            let t = r / half.tan();
            (-t, -t, corner.at - bisector * (r / half.sin()))
        }
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
            if corner.notches_side_in() {
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
            // a side used up by the fillets at both its ends is left out
            let used_up = matches!(segment, Segment::Line { .. })
                && (segment.end() - from).length() <= STRAIGHT;
            if !used_up {
                segments.push(*segment);
            }
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

/// What a click at `at` does with Create Fillets open: the nearest sharp corner within
/// `p.within` of it gets a fillet (a T-bone notching the side the click is nearer); else the
/// nearest fillet arc within reach is taken out, its sides meeting sharp again. The vector
/// as it becomes, and a note when the radius had to be cut down to fit.
pub fn at_click(
    doc: &Document,
    p: &Params,
    at: Point,
) -> Result<(EntityId, Path, Option<String>), CommandError> {
    let mut corner: Option<(f64, EntityId, usize, Corner)> = None;
    let mut arc: Option<(f64, EntityId, usize)> = None;
    for (&id, path) in &doc.vectors {
        for (seg, sharp) in sharp_corners(path, Corners::All) {
            let dist = sharp.at.distance(at);
            if dist <= p.within && corner.is_none_or(|best| dist < best.0) {
                corner = Some((dist, id, seg, sharp));
            }
        }
        for (seg, segment) in path.segments.iter().enumerate() {
            if !matches!(segment, Segment::Arc { .. }) {
                continue;
            }
            let dist = polyline_distance(&segment_points(path, seg), at);
            if dist <= p.within && arc.is_none_or(|best| dist < best.0) {
                arc = Some((dist, id, seg));
            }
        }
    }
    match (corner, arc) {
        (Some((dist, id, seg, mut sharp)), arc) if arc.is_none_or(|nearest| dist <= nearest.0) => {
            let off = at - sharp.at;
            sharp.click_side = Some(off.dot(sharp.back) >= off.dot(sharp.on));
            let planned = plan(sharp, p, (sharp.len_in, sharp.len_out)).ok_or_else(|| {
                CommandError::Failed("no fillet fits that corner. Try a smaller radius".into())
            })?;
            let radius = (planned.center - planned.through).length();
            let note = (radius < p.radius - STRAIGHT).then(|| {
                format!(
                    "the radius was cut to {}, the largest that fits",
                    doc.job.units.length(radius, 2)
                )
            });
            Ok((id, rebuilt(&doc.vectors[&id], &[(seg, planned)]), note))
        }
        (_, Some((_, id, seg))) => {
            let path = unfilleted(&doc.vectors[&id], seg).ok_or_else(|| {
                CommandError::Failed(
                    "only a fillet between two straight sides can be taken out. Click a fillet between two straight sides".into(),
                )
            })?;
            Ok((id, path, None))
        }
        _ => Err(CommandError::Failed(
            "click on a sharp corner to fillet it, or on a fillet to take it out".into(),
        )),
    }
}

/// The points along segment `k` of `path`.
fn segment_points(path: &Path, k: usize) -> Vec<Point> {
    let start = if k == 0 {
        path.start
    } else {
        path.segments[k - 1].end()
    };
    Path {
        start,
        segments: vec![path.segments[k]],
        closed: false,
    }
    .flatten(0.01)
}

/// `path` with the fillet at segment `k` taken out: the straight sides either side of it run
/// on to meet at the sharp corner again. `None` when a side is not straight (or missing).
fn unfilleted(path: &Path, k: usize) -> Option<Path> {
    let n = path.segments.len();
    let (prev, next) = if path.closed {
        (n >= 3).then_some(((k + n - 1) % n, (k + 1) % n))?
    } else {
        (k.checked_sub(1)?, (k + 1 < n).then_some(k + 1)?)
    };
    let start_of = |i: usize| {
        if i == 0 {
            path.start
        } else {
            path.segments[i - 1].end()
        }
    };
    let (Segment::Line { to: a1 }, Segment::Line { to: b1 }) =
        (path.segments[prev], path.segments[next])
    else {
        return None;
    };
    let x = meet(start_of(prev), a1, path.segments[k].end(), b1)?;
    let mut segments = path.segments.clone();
    segments[prev] = Segment::Line { to: x };
    segments.remove(k);
    // a path that began at the fillet now begins at the corner
    let start = if k == 0 || (path.closed && k == n - 1) {
        x
    } else {
        path.start
    };
    Some(Path {
        start,
        segments,
        closed: path.closed,
    })
}

/// Where the line through `a0` and `a1` meets the line through `b0` and `b1`.
fn meet(a0: Point, a1: Point, b0: Point, b1: Point) -> Option<Point> {
    let (da, db) = (a1 - a0, b1 - b0);
    let denominator = da.perp_dot(db);
    if denominator.abs() < STRAIGHT {
        return None;
    }
    Some(a0 + da * ((b0 - a0).perp_dot(db) / denominator))
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
            ..Params::default()
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
        // within EPS: the centres come from trigonometry, whose last bits differ by platform
        let expected = [p(3.0, 3.0), p(3.0, 7.0), p(7.0, 3.0), p(7.0, 7.0)];
        assert_eq!(centers.len(), expected.len());
        for (c, e) in centers.iter().zip(expected) {
            assert!((*c - e).length() < EPS, "{centers:?}");
        }
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
    fn a_fillet_may_use_a_whole_side_and_later_corners_get_what_is_left() {
        // in turn: the first corner takes radius 6 (6 of each side), the next only 4 of its
        // side is left, the third 6 again, the last 4
        let [path] = filleted(
            vec![square(0.0, 0.0, 10.0)],
            &params(Kind::Normal, 6.0, Corners::All),
        )
        .unwrap()
        .try_into()
        .unwrap();
        let p = Point::new;
        let expected = [p(4.0, 6.0), p(6.0, 6.0), p(6.0, 4.0), p(4.0, 4.0)];
        let centres = arcs(&path);
        assert_eq!(centres.len(), 4);
        for (got, want) in centres.iter().zip(expected) {
            assert!((*got - want).length() < EPS, "{got} vs {want}");
        }
    }

    #[test]
    fn a_radius_too_large_is_cut_to_the_largest_that_fits() {
        // radius 50 on a 10 mm square: the first corner takes both its whole sides, which
        // leaves the next corner nothing; the third takes the other two sides
        let [path] = filleted(
            vec![square(0.0, 0.0, 10.0)],
            &params(Kind::Normal, 50.0, Corners::All),
        )
        .unwrap()
        .try_into()
        .unwrap();
        assert!(path.closed);
        assert_eq!(path.segments.len(), 2, "two quarter circles: a lens");
        assert!(
            path.segments
                .iter()
                .all(|s| matches!(s, Segment::Arc { .. }))
        );
    }

    #[test]
    fn plasma_fillets_loop_outside_each_corner() {
        let r = 1.0;
        let [path] = filleted(
            vec![square(0.0, 0.0, 10.0)],
            &params(Kind::PlasmaDragKnife, r, Corners::All),
        )
        .unwrap()
        .try_into()
        .unwrap();
        // every corner is still on the path, and each loop sits diagonally outside its
        // corner: centre (11, -1) for the corner at (10, 0)
        let points = path.flatten(1e-4);
        for corner in [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)] {
            let corner = Point::new(corner.0, corner.1);
            assert!(polyline_distance(&points, corner) < 1e-3, "{corner}");
        }
        assert!(
            arcs(&path)
                .iter()
                .any(|c| (*c - Point::new(11.0, -1.0)).length() < EPS)
        );
        let b = path.bounds();
        assert!(
            (b.max.x - 12.0).abs() < 1e-6 && (b.min.y + 2.0).abs() < 1e-6,
            "{b:?}"
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

    #[test]
    fn a_clicked_corner_alone_is_filleted_and_a_click_on_the_fillet_takes_it_out() {
        let mut doc = document_with(vec![square(0.0, 0.0, 10.0)]);
        let id = *doc.vectors.keys().next().unwrap();
        let click = |at: Point| Params {
            at: Some(at),
            within: 1.0,
            ..Params::default()
        };
        let change = CreateFillets::run(&doc, &click(Point::new(0.3, -0.2))).unwrap();
        doc.apply(&change).unwrap();
        let path = &doc.vectors[&id];
        assert_eq!((path.segments.len(), arcs(path).len()), (5, 1), "{path:?}");
        assert!((arcs(path)[0] - Point::new(3.0, 3.0)).length() < EPS);
        // the fillet's middle, where a click takes it out
        let mid = Point::splat(3.0 - 3.0 * std::f64::consts::FRAC_1_SQRT_2);
        let change = CreateFillets::run(&doc, &click(mid)).unwrap();
        doc.apply(&change).unwrap();
        let path = &doc.vectors[&id];
        assert_eq!((path.segments.len(), arcs(path).len()), (4, 0), "{path:?}");
        assert!(
            polyline_distance(&path.flatten(0.01), Point::ZERO) < EPS,
            "the corner is back"
        );
        let far = CreateFillets::run(&doc, &click(Point::new(5.0, 5.0))).unwrap_err();
        assert!(far.to_string().contains("Click on a sharp corner"), "{far}");
    }

    #[test]
    fn a_clicked_t_bone_notches_the_side_the_click_is_nearer_and_an_oversize_radius_is_noted() {
        let p = Point::new;
        let wide = Path::polyline(
            &[p(0.0, 0.0), p(20.0, 0.0), p(20.0, 10.0), p(0.0, 10.0)],
            true,
        );
        let doc = document_with(vec![wide]);
        // just up the short left side from the corner at the origin
        let at = p(0.1, 0.6);
        let params = Params {
            kind: Kind::TBone,
            radius: 1.0,
            at: Some(at),
            within: 1.0,
            ..Params::default()
        };
        let (_, path, note) = at_click(&doc, &params, at).unwrap();
        let b = path.bounds();
        assert!(
            b.min.x < -0.5 && b.min.y > -EPS,
            "notched into the left side: {b:?}"
        );
        assert!(note.is_none());
        let big = Params {
            radius: 50.0,
            at: Some(Point::ZERO),
            within: 1.0,
            ..Params::default()
        };
        let (_, _, note) = at_click(&doc, &big, Point::ZERO).unwrap();
        assert!(
            note.as_ref().is_some_and(|n| n.contains("cut to")),
            "{note:?}"
        );
    }
}
