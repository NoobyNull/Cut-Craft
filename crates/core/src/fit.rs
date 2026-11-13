//! Fitting a drawing into a closed shape: the largest size at which the whole drawing lies
//! inside the shape's own outline (not just its box), at least `margin` from its edge.

use crate::geometry::{Point, Rect, inside, polyline_distance};
use glam::{DAffine2, DVec2};

/// How a drawing takes the shape's size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FitMode {
    /// As big as fits, its proportions kept.
    #[default]
    Fit,
    /// Stretched to the shape's proportions (its box's width to height), then as big as
    /// fits: fills a border, but squashes lettering.
    Stretch,
}

/// Why a drawing could not be fitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FitError {
    /// The drawing has no size (nothing, or a single point).
    NoDrawing,
    /// The shape is not a closed outline with an inside.
    NoShape,
    /// The margin leaves no room inside the shape.
    MarginTooBig,
}

impl std::fmt::Display for FitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NoDrawing => "the drawing has nothing in it to fit",
            Self::NoShape => "the chosen shape is not a closed shape: pick a circle, rectangle or other closed vector",
            Self::MarginTooBig => "the margin leaves no room inside the shape: make it smaller",
        })
    }
}

/// How many steps the size is narrowed down in: the last is about a millionth of the size.
const STEPS: usize = 24;
/// Places tried across the shape's box, each way, when the middle is not the best spot.
const GRID: usize = 9;
/// The shape's outline is checked at points no further apart than this share of its size.
const SAMPLES: f64 = 400.0;
/// Times the best spot is nudged about to give the drawing more room.
const NUDGES: usize = 24;

/// The move and scale that put `drawing` (every point of it, flattened) as large as it fits
/// inside the closed ring `shape`, keeping `margin` from the edge. In the middle of the shape
/// when it fits there; otherwise wherever in the shape gives it the most room (the wide
/// bottom of a triangle). A rectangle gives what fitting its box gives.
pub fn fit_into(
    drawing: &[Point],
    shape: &[Point],
    margin: f64,
    mode: FitMode,
) -> Result<DAffine2, FitError> {
    let margin = margin.max(0.0);
    let bounds = drawing
        .iter()
        .map(|&p| Rect::point(p))
        .reduce(Rect::union)
        .ok_or(FitError::NoDrawing)?;
    let size = bounds.size();
    if size.max_element() <= 0.0 || !size.is_finite() {
        return Err(FitError::NoDrawing);
    }
    let ring: Vec<Point> = shape.to_vec();
    if ring.len() < 3 || crate::geometry::winding(&ring).abs() <= 0.0 {
        return Err(FitError::NoShape);
    }
    let area = ring
        .iter()
        .map(|&p| Rect::point(p))
        .reduce(Rect::union)
        .ok_or(FitError::NoShape)?;
    let room = area.size();
    // the drawing's proportions after the mode: unit scale for Fit
    let stretch = match mode {
        FitMode::Stretch if size.x > 0.0 && size.y > 0.0 => {
            (room / size) / (room / size).min_element()
        }
        // a straight line has no proportions to change
        FitMode::Fit | FitMode::Stretch => DVec2::ONE,
    };
    let middle = bounds.center();
    let hull: Vec<Point> = hull(drawing)
        .into_iter()
        .map(|p| (p - middle) * stretch)
        .collect();
    let shaped = size * stretch;
    let space = room - 2.0 * margin;
    if space.min_element() <= 0.0 {
        return Err(FitError::MarginTooBig);
    }
    // the box fit: no size can be bigger (a straight line has no height to limit it)
    let biggest = [space.x / shaped.x, space.y / shaped.y]
        .into_iter()
        .filter(|v| v.is_finite())
        .fold(f64::INFINITY, f64::min);
    let edge = Edge {
        rings: vec![ring.clone()],
        closed: ring.iter().copied().chain(ring.first().copied()).collect(),
        outline: resample(&ring, room.max_element() / SAMPLES),
        margin,
        tol: room.max_element() * 1e-9,
    };
    let fits = |scale: f64, at: Point| edge.holds(&hull, scale, at);
    let place = |scale: f64, at: Point| {
        DAffine2::from_translation(at)
            * DAffine2::from_scale(stretch * scale)
            * DAffine2::from_translation(-middle)
    };
    let centre = area.center();
    if fits(biggest, centre) {
        return Ok(place(biggest, centre));
    }
    let spots = spots(area, centre);
    let found = |scale: f64| spots.iter().copied().find(|&at| fits(scale, at));
    let (mut lo, mut hi) = (0.0, biggest);
    let mut best = None;
    // the smallest size worth trying: a thousandth of the box fit
    let least = biggest * 1e-3;
    if let Some(at) = found(least) {
        best = Some((least, at));
        lo = least;
    }
    if best.is_none() {
        return Err(FitError::MarginTooBig);
    }
    for _ in 0..STEPS {
        let mid = f64::midpoint(lo, hi);
        match found(mid) {
            Some(at) => {
                best = Some((mid, at));
                lo = mid;
            }
            None => hi = mid,
        }
    }
    let (scale, at) = best.ok_or(FitError::MarginTooBig)?;
    // the grid is coarse: nudge the spot about while that lets it grow
    #[expect(clippy::cast_precision_loss, reason = "a small grid")]
    let step = area.size() / (GRID - 1) as f64 / 2.0;
    let (scale, at) = nudged(&fits, (scale, at), step, biggest);
    Ok(place(scale, at))
}

/// The size and spot `(scale, at)` grown by moving the spot about in ever smaller steps
/// from `step`, never past `biggest`.
fn nudged(
    fits: &dyn Fn(f64, Point) -> bool,
    (mut scale, mut at): (f64, Point),
    mut step: DVec2,
    biggest: f64,
) -> (f64, Point) {
    for _ in 0..NUDGES {
        let around = [-1.0, 0.0, 1.0]
            .into_iter()
            .flat_map(|x| [-1.0, 0.0, 1.0].map(|y| DVec2::new(x, y)))
            .filter(|d| *d != DVec2::ZERO)
            .map(|d| at + d * step);
        let grown = around
            .filter(|&p| fits(scale, p))
            .map(|p| {
                let (mut lo, mut hi) = (scale, biggest);
                for _ in 0..STEPS {
                    let mid = f64::midpoint(lo, hi);
                    if fits(mid, p) {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                (lo, p)
            })
            .max_by(|a, b| a.0.total_cmp(&b.0));
        match grown {
            Some((s, p)) if s > scale * (1.0 + 1e-6) => (scale, at) = (s, p),
            _ => step /= 2.0,
        }
    }
    (scale, at)
}

/// Places to try the drawing at: the middle first, then a grid over the shape's box,
/// nearest the middle first.
fn spots(area: Rect, centre: Point) -> Vec<Point> {
    let mut spots: Vec<Point> = (0..GRID)
        .flat_map(|i| (0..GRID).map(move |j| (i, j)))
        .map(|(i, j)| {
            #[expect(clippy::cast_precision_loss, reason = "a small grid")]
            let t = DVec2::new(i as f64, j as f64) / (GRID - 1) as f64;
            area.min + area.size() * t
        })
        .collect();
    spots.sort_by(|a, b| a.distance(centre).total_cmp(&b.distance(centre)));
    spots.insert(0, centre);
    spots
}

/// The shape's edge, ready for checking a drawing against.
struct Edge {
    rings: Vec<Vec<Point>>,
    /// The ring with its first point again at the end.
    closed: Vec<Point>,
    /// The ring's edge as closely spaced points.
    outline: Vec<Point>,
    margin: f64,
    tol: f64,
}

impl Edge {
    /// Whether the drawing's `hull`, scaled by `scale` and moved to `at`, lies inside the
    /// shape at least the margin from its edge: every hull corner inside and far enough from
    /// the edge, and no point of the edge inside or too near the hull.
    fn holds(&self, hull: &[Point], scale: f64, at: Point) -> bool {
        let (margin, tol) = (self.margin, self.tol);
        let placed: Vec<Point> = hull.iter().map(|&p| at + p * scale).collect();
        let corners_in = placed.iter().all(|&p| {
            let d = polyline_distance(&self.closed, p);
            d >= margin - tol && (d <= tol || inside(&self.rings, p))
        });
        if !corners_in || placed.len() < 3 {
            return corners_in;
        }
        let closed_hull: Vec<Point> = placed
            .iter()
            .copied()
            .chain(placed.first().copied())
            .collect();
        let hull_ring = [placed];
        self.outline.iter().all(|&q| {
            let d = polyline_distance(&closed_hull, q);
            d >= margin - tol && (d <= tol || !inside(&hull_ring, q))
        })
    }
}

/// `ring` with points added along each side so none is further than `step` from the next.
fn resample(ring: &[Point], step: f64) -> Vec<Point> {
    let mut out = vec![];
    for (i, &a) in ring.iter().enumerate() {
        let b = ring[(i + 1) % ring.len()];
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a side's point count, small and positive"
        )]
        let n = (a.distance(b) / step).ceil().max(1.0) as usize;
        #[expect(clippy::cast_precision_loss, reason = "a side's point count")]
        out.extend((0..n).map(|k| a.lerp(b, k as f64 / n as f64)));
    }
    out
}

/// The convex hull of `points`, anticlockwise (monotone chain).
fn hull(points: &[Point]) -> Vec<Point> {
    let mut points = points.to_vec();
    points.sort_by(|p, q| p.x.total_cmp(&q.x).then(p.y.total_cmp(&q.y)));
    points.dedup();
    if points.len() < 3 {
        return points;
    }
    let turn = |o: Point, a: Point, b: Point| (a - o).perp_dot(b - o);
    let half = |points: &mut dyn Iterator<Item = Point>| {
        let mut h: Vec<Point> = vec![];
        for p in points {
            while h.len() >= 2 && turn(h[h.len() - 2], h[h.len() - 1], p) <= 0.0 {
                h.pop();
            }
            h.push(p);
        }
        h.pop();
        h
    };
    let mut h = half(&mut points.iter().copied());
    h.extend(half(&mut points.iter().rev().copied()));
    h
}

/// Directions round the shape's middle its edge is measured in, for Morph.
const RAYS: usize = 1440;

/// Bending a drawing to follow a shape's outline: each point keeps its direction from the
/// drawing's middle, and its share of the way out to the drawing's box becomes the same share
/// of the way out to the shape's edge (less the margin). The box's edge lands on the shape's.
#[derive(Clone, Debug)]
pub struct Morph {
    from: Rect,
    centre: Point,
    /// How far the shape's edge (less the margin) is from `centre`, at each of [`RAYS`]
    /// directions anticlockwise from +X.
    reach: Vec<f64>,
}

impl Morph {
    /// The bend taking the box `drawing` onto the closed ring `shape`, `margin` inside it.
    pub fn new(drawing: Rect, shape: &[Point], margin: f64) -> Result<Self, FitError> {
        let size = drawing.size();
        if size.min_element() <= 0.0 || !size.is_finite() {
            return Err(FitError::NoDrawing);
        }
        if shape.len() < 3 || crate::geometry::winding(shape).abs() <= 0.0 {
            return Err(FitError::NoShape);
        }
        // the balance point, or for a shape it lies outside of (a C or U) the point deepest
        // inside it
        let rings = [shape.to_vec()];
        let centre = Some(centroid(shape))
            .filter(|&c| inside(&rings, c))
            .unwrap_or_else(|| deepest(shape));
        let margin = margin.max(0.0);
        let mut reach = Vec::with_capacity(RAYS);
        for k in 0..RAYS {
            #[expect(clippy::cast_precision_loss, reason = "a ray's number")]
            let dir = DVec2::from_angle(std::f64::consts::TAU * k as f64 / RAYS as f64);
            // the nearest edge that way: where a shape curls back on itself (a C or U), the
            // part hidden behind it is left out, and the user sees how it bends
            let hit = ray_hits(shape, centre, dir)
                .first()
                .copied()
                .ok_or(FitError::NoShape)?;
            let r = hit - margin;
            if r <= 0.0 {
                return Err(FitError::MarginTooBig);
            }
            reach.push(r);
        }
        Ok(Self {
            from: drawing,
            centre,
            reach,
        })
    }

    /// Where the drawing's point `p` goes: its box's four sides go onto the four stretches of
    /// the shape's edge between where the box's corners land, and the inside is blended
    /// smoothly between them (a Coons patch), so grid lines curve evenly with no kinks.
    pub fn point(&self, p: Point) -> Point {
        let (min, size) = (self.from.min, self.from.size());
        let t = ((p - min) / size).clamp(DVec2::ZERO, DVec2::ONE);
        let (u, v) = (t.x, t.y);
        let on = |x: f64, y: f64| self.edge(min + size * DVec2::new(x, y));
        let (bottom, top) = (on(u, 0.0), on(u, 1.0));
        let (left, right) = (on(0.0, v), on(1.0, v));
        let corners = (1.0 - u) * (1.0 - v) * on(0.0, 0.0)
            + u * (1.0 - v) * on(1.0, 0.0)
            + (1.0 - u) * v * on(0.0, 1.0)
            + u * v * on(1.0, 1.0);
        (1.0 - v) * bottom + v * top + (1.0 - u) * left + u * right - corners
    }

    /// Where the point `p` on the drawing's box edge lands on the shape's edge: the same
    /// way out from the middle.
    fn edge(&self, p: Point) -> Point {
        let dir = (p - self.from.center()).normalize_or_zero();
        self.centre + dir * self.reach_at(dir)
    }

    fn reach_at(&self, dir: DVec2) -> f64 {
        #[expect(clippy::cast_precision_loss, reason = "the ray count")]
        let at = dir.y.atan2(dir.x).rem_euclid(std::f64::consts::TAU) / std::f64::consts::TAU
            * RAYS as f64;
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a ray's number, within the ray count"
        )]
        let k = at.floor() as usize % RAYS;
        let t = at - at.floor();
        self.reach[k] * (1.0 - t) + self.reach[(k + 1) % RAYS] * t
    }

    /// The drawing's polyline `points` bent: added points along each side first, so straight
    /// lines curve with the shape, no further apart than `step`.
    pub fn polyline(&self, points: &[Point], closed: bool, step: f64) -> Vec<Point> {
        let mut out = vec![];
        let sides = if closed {
            points.len()
        } else {
            points.len().saturating_sub(1)
        };
        for i in 0..sides {
            let (a, b) = (points[i], points[(i + 1) % points.len()]);
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a side's point count, small and positive"
            )]
            let n = (a.distance(b) / step).ceil().clamp(1.0, 10_000.0) as usize;
            #[expect(clippy::cast_precision_loss, reason = "a side's point count")]
            out.extend((0..n).map(|k| self.point(a.lerp(b, k as f64 / n as f64))));
        }
        if !closed && let Some(&last) = points.last() {
            out.push(self.point(last));
        }
        if points.len() == 1 {
            out.push(self.point(points[0]));
        }
        out
    }
}

/// Places tried across a shape's box, each way, for the point deepest inside it.
const DEEPEST_GRID: usize = 40;

/// The point of a grid over the ring's box that is inside it and furthest from its edge.
fn deepest(ring: &[Point]) -> Point {
    let area = ring
        .iter()
        .map(|&p| Rect::point(p))
        .reduce(Rect::union)
        .unwrap_or(Rect::point(Point::ZERO));
    let rings = [ring.to_vec()];
    let closed: Vec<Point> = ring.iter().copied().chain(ring.first().copied()).collect();
    let middle = centroid(ring);
    #[expect(clippy::cast_precision_loss, reason = "a small grid")]
    let step = area.size().max_element() / DEEPEST_GRID as f64 / 4.0;
    (0..=DEEPEST_GRID)
        .flat_map(|i| (0..=DEEPEST_GRID).map(move |j| (i, j)))
        .map(|(i, j)| {
            #[expect(clippy::cast_precision_loss, reason = "a small grid")]
            let t = DVec2::new(i as f64, j as f64) / DEEPEST_GRID as f64;
            area.min + area.size() * t
        })
        .filter(|&p| inside(&rings, p))
        .map(|p| (polyline_distance(&closed, p), p))
        // deepest first; of equally deep points, the one nearest the balance point
        .max_by(|(da, a), (db, b)| {
            let near = |p: &Point| -p.distance(middle);
            ((da / step).round())
                .total_cmp(&(db / step).round())
                .then(near(a).total_cmp(&near(b)))
        })
        .map_or(ring[0], |(_, p)| p)
}

/// The ring's area centre (its balance point), or its first point if it has no area.
fn centroid(ring: &[Point]) -> Point {
    let (mut area, mut sum) = (0.0, DVec2::ZERO);
    for (i, &a) in ring.iter().enumerate() {
        let b = ring[(i + 1) % ring.len()];
        let cross = a.perp_dot(b);
        area += cross;
        sum += (a + b) * cross;
    }
    if area.abs() <= 0.0 {
        return ring[0];
    }
    sum / (3.0 * area)
}

/// How far along the ray from `from` towards `dir` (a unit vector) it crosses each side of
/// `ring`, near crossings at a corner counted once.
fn ray_hits(ring: &[Point], from: Point, dir: DVec2) -> Vec<f64> {
    let mut hits: Vec<f64> = vec![];
    for (i, &a) in ring.iter().enumerate() {
        let b = ring[(i + 1) % ring.len()];
        let side = b - a;
        let denom = dir.perp_dot(side);
        if denom.abs() <= f64::EPSILON {
            continue;
        }
        let w = a - from;
        let t = w.perp_dot(side) / denom;
        let u = w.perp_dot(dir) / denom;
        if t > 0.0 && (0.0..=1.0).contains(&u) {
            hits.push(t);
        }
    }
    hits.sort_by(f64::total_cmp);
    let scale = hits.last().copied().unwrap_or(1.0);
    hits.dedup_by(|a, b| (*a - *b).abs() <= scale * 1e-9);
    hits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Path;

    fn square(min: Point, side: f64) -> Vec<Point> {
        vec![
            min,
            min + DVec2::new(side, 0.0),
            min + DVec2::new(side, side),
            min + DVec2::new(0.0, side),
        ]
    }

    fn rect(min: Point, w: f64, h: f64) -> Vec<Point> {
        vec![
            min,
            min + DVec2::new(w, 0.0),
            min + DVec2::new(w, h),
            min + DVec2::new(0.0, h),
        ]
    }

    fn circle(centre: Point, r: f64) -> Vec<Point> {
        Path::circle(centre, r).flatten(0.001)
    }

    fn placed(t: &DAffine2, points: &[Point]) -> Rect {
        points
            .iter()
            .map(|&p| Rect::point(t.transform_point2(p)))
            .reduce(Rect::union)
            .unwrap()
    }

    fn near(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() <= tol, "{a} is not {b} (±{tol})");
    }

    #[test]
    fn a_drawing_in_a_rectangle_fits_its_box_centred_and_keeps_its_proportions() {
        // 20 x 10 drawing into a 100 x 80 rectangle: 100 wide, 50 tall, centred
        let drawing = rect(Point::new(5.0, 5.0), 20.0, 10.0);
        let shape = rect(Point::new(0.0, 0.0), 100.0, 80.0);
        let t = fit_into(&drawing, &shape, 0.0, FitMode::Fit).unwrap();
        let b = placed(&t, &drawing);
        near(b.size().x, 100.0, 1e-6);
        near(b.size().y, 50.0, 1e-6);
        near(b.center().x, 50.0, 1e-6);
        near(b.center().y, 40.0, 1e-6);
    }

    #[test]
    fn the_margin_keeps_the_drawing_off_the_edge() {
        let drawing = rect(Point::ZERO, 20.0, 10.0);
        let shape = rect(Point::ZERO, 100.0, 80.0);
        let t = fit_into(&drawing, &shape, 5.0, FitMode::Fit).unwrap();
        let b = placed(&t, &drawing);
        near(b.size().x, 90.0, 1e-6);
        near(b.min.x, 5.0, 1e-6);
    }

    #[test]
    fn a_square_drawing_in_a_circle_keeps_its_corners_inside_the_circle() {
        // a box fit would make it 100 wide with corners 70.7 from the middle: outside
        let drawing = square(Point::ZERO, 10.0);
        let shape = circle(Point::new(50.0, 50.0), 50.0);
        let t = fit_into(&drawing, &shape, 0.0, FitMode::Fit).unwrap();
        let b = placed(&t, &drawing);
        // the square's diagonal is the circle's diameter: side 100 / √2
        near(b.size().x, 100.0 / 2f64.sqrt(), 0.05);
        near(b.center().x, 50.0, 1e-6);
        near(b.center().y, 50.0, 1e-6);
        for p in &drawing {
            assert!(t.transform_point2(*p).distance(Point::new(50.0, 50.0)) <= 50.0 + 1e-6);
        }
    }

    #[test]
    fn a_margin_inside_a_circle_shrinks_the_fit_by_it() {
        let drawing = square(Point::ZERO, 10.0);
        let shape = circle(Point::new(50.0, 50.0), 50.0);
        let t = fit_into(&drawing, &shape, 10.0, FitMode::Fit).unwrap();
        let b = placed(&t, &drawing);
        near(b.size().x, 80.0 / 2f64.sqrt(), 0.05);
    }

    #[test]
    fn a_drawing_moves_to_where_a_triangle_has_most_room() {
        // a wide flat drawing in a triangle point up: it sits low, where the triangle is wide
        let drawing = rect(Point::ZERO, 40.0, 10.0);
        let shape = vec![
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            Point::new(50.0, 100.0),
        ];
        let t = fit_into(&drawing, &shape, 0.0, FitMode::Fit).unwrap();
        let b = placed(&t, &drawing);
        assert!(b.center().y < 50.0, "it sits low: {:?}", b.center());
        // and it is bigger than it would be at the box's middle
        assert!(b.size().x > 50.0, "{:?}", b.size());
        let rings = vec![shape];
        for p in &drawing {
            let q = t.transform_point2(*p);
            assert!(
                inside(&rings, q) || polyline_distance(&rings[0], q) < 1e-3,
                "{q} is outside"
            );
        }
    }

    #[test]
    fn a_drawing_does_not_bridge_a_notch_in_the_shape() {
        // a U: two arms joined at the bottom; the drawing must not span the gap between arms
        let shape = vec![
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            Point::new(100.0, 100.0),
            Point::new(70.0, 100.0),
            Point::new(70.0, 30.0),
            Point::new(30.0, 30.0),
            Point::new(30.0, 100.0),
            Point::new(0.0, 100.0),
        ];
        let drawing = square(Point::ZERO, 10.0);
        let t = fit_into(&drawing, &shape, 0.0, FitMode::Fit).unwrap();
        let b = placed(&t, &drawing);
        // the biggest square without crossing the notch is 30 (an arm, or the base strip)
        assert!(b.size().x <= 30.0 + 1e-3, "{b:?}");
        assert!(b.size().x > 29.5, "{b:?}");
    }

    #[test]
    fn stretch_takes_the_shapes_proportions() {
        let drawing = square(Point::ZERO, 10.0);
        let shape = rect(Point::ZERO, 100.0, 40.0);
        let t = fit_into(&drawing, &shape, 0.0, FitMode::Stretch).unwrap();
        let b = placed(&t, &drawing);
        near(b.size().x, 100.0, 1e-6);
        near(b.size().y, 40.0, 1e-6);
    }

    #[test]
    fn a_drawing_with_no_size_or_a_margin_too_big_says_why() {
        let shape = rect(Point::ZERO, 100.0, 40.0);
        assert_eq!(
            fit_into(&[Point::ZERO], &shape, 0.0, FitMode::Fit),
            Err(FitError::NoDrawing)
        );
        assert_eq!(
            fit_into(&square(Point::ZERO, 1.0), &shape, 25.0, FitMode::Fit),
            Err(FitError::MarginTooBig)
        );
        assert_eq!(
            fit_into(
                &square(Point::ZERO, 1.0),
                &[Point::ZERO, Point::X],
                0.0,
                FitMode::Fit
            ),
            Err(FitError::NoShape)
        );
    }

    #[test]
    fn a_line_drawing_fits_by_its_length() {
        // a straight line has no height: it is as long as fits
        let drawing = vec![Point::ZERO, Point::new(10.0, 0.0)];
        let shape = rect(Point::ZERO, 100.0, 40.0);
        let t = fit_into(&drawing, &shape, 0.0, FitMode::Fit).unwrap();
        near(placed(&t, &drawing).size().x, 100.0, 1e-6);
    }

    #[test]
    fn morph_puts_the_drawings_box_on_the_shapes_edge() {
        // a 10 x 10 box bent into a circle of radius 50: its corners and side middles all
        // land on the circle, its middle on the circle's middle
        let box_ = Rect::new(Point::ZERO, Point::new(10.0, 10.0));
        let shape = circle(Point::new(50.0, 50.0), 50.0);
        let m = Morph::new(box_, &shape, 0.0).unwrap();
        let c = Point::new(50.0, 50.0);
        for p in [
            Point::ZERO,
            Point::new(10.0, 0.0),
            Point::new(5.0, 10.0),
            Point::new(10.0, 10.0),
        ] {
            near(m.point(p).distance(c), 50.0, 0.05);
        }
        assert!(m.point(Point::new(5.0, 5.0)).distance(c) < 1e-6);
        // the middle line across stays straight through the middle, and inside the circle
        let across = m.polyline(&[Point::new(0.0, 5.0), Point::new(10.0, 5.0)], false, 0.5);
        for q in &across {
            near(q.y, 50.0, 1e-6);
        }
        for p in [Point::new(2.5, 2.5), Point::new(7.5, 8.0)] {
            assert!(m.point(p).distance(c) < 50.0);
        }
    }

    #[test]
    fn morph_bends_straight_sides_round_the_shape_and_keeps_the_margin() {
        let box_ = Rect::new(Point::ZERO, Point::new(20.0, 10.0));
        let shape = circle(Point::new(0.0, 0.0), 40.0);
        let m = Morph::new(box_, &shape, 5.0).unwrap();
        let side = m.polyline(&[Point::ZERO, Point::new(20.0, 0.0)], false, 0.5);
        assert!(side.len() > 20, "the side gets points along it to bend");
        for p in &side {
            near(p.length(), 35.0, 0.05);
        }
    }

    #[test]
    fn a_shape_that_curls_back_is_still_morphed_into_from_inside_it() {
        // a C: its balance point is in the gap, so the bend starts from inside the C's back,
        // and goes as far as the nearest edge each way
        let shape = vec![
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            Point::new(100.0, 20.0),
            Point::new(20.0, 20.0),
            Point::new(20.0, 80.0),
            Point::new(100.0, 80.0),
            Point::new(100.0, 100.0),
            Point::new(0.0, 100.0),
        ];
        let box_ = Rect::new(Point::ZERO, Point::new(10.0, 10.0));
        let m = Morph::new(box_, &shape, 0.0).expect("morphed, not refused");
        // the drawing's box edge lands on the C's edge; how the inside bends, the user sees
        let mut closed = shape.clone();
        closed.push(shape[0]);
        for p in [
            Point::new(0.0, 5.0),
            Point::new(10.0, 3.0),
            Point::new(4.0, 10.0),
        ] {
            let q = m.point(p);
            assert!(
                polyline_distance(&closed, q) < 0.01,
                "{q} is on the C's edge"
            );
        }
    }
}
