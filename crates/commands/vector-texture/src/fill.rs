//! What every decorative fill is built on: the area being filled, a grid that keeps every
//! stroke a carvable gap from the others and from the edge, and a walker that grows strokes
//! through it.

use cut_craft_core::geometry::{Path, Point, Rect, inside};
use cut_craft_core::random::Random;

/// The most points a fill may lay down before it is judged too fine for the area.
pub const MOST_POINTS: usize = 3_000_000;

/// The edge strokes keep their distance from.
pub const WALL: u32 = 0;

/// The closed vectors filled (or the job's rectangle), as flattened rings.
pub struct Area {
    pub rings: Vec<Vec<Point>>,
    pub bounds: Rect,
}

impl Area {
    pub fn new(rings: Vec<Vec<Point>>) -> Self {
        let bounds = rings
            .iter()
            .flatten()
            .fold(None, |b: Option<Rect>, &p| {
                Some(b.map_or_else(|| Rect::point(p), |b| b.include(p)))
            })
            .unwrap_or_else(|| Rect::point(Point::ZERO));
        Self { rings, bounds }
    }

    pub fn inside(&self, p: Point) -> bool {
        inside(&self.rings, p)
    }

    /// The rings as closed paths, for booleans and offsets.
    pub fn paths(&self) -> Vec<Path> {
        self.rings
            .iter()
            .filter(|r| r.len() > 2)
            .map(|r| Path::polyline(r, true))
            .collect()
    }

    /// A random point in the bounds that is inside; `None` after many misses.
    pub fn random_inside(&self, random: &mut Random) -> Option<Point> {
        let (min, size) = (self.bounds.min, self.bounds.size());
        (0..64)
            .map(|_| min + Point::new(random.unit() * size.x, random.unit() * size.y))
            .find(|&p| self.inside(p))
    }
}

/// One point laid down: where, how far its stroke reaches either side of it, and which
/// stroke it belongs to (and where along it).
#[derive(Clone, Copy)]
struct Mark {
    at: Point,
    reach: f64,
    stroke: u32,
    index: u32,
}

/// What a new point may sit close to: its own stroke's last few points, and its parent
/// stroke near where it branched off.
#[derive(Clone, Copy)]
pub struct Probe {
    pub stroke: u32,
    pub index: u32,
    /// Points of its own stroke this many places back or less are neighbours, not clashes.
    pub skip: u32,
    /// The stroke it grew from, and the junction: that stroke's points within the distance
    /// of the junction are ignored.
    pub junction: Option<(u32, Point, f64)>,
}

/// Every point laid so far, bucketed so a new point can be checked against its neighbours.
pub struct Space {
    pub gap: f64,
    origin: Point,
    cell: f64,
    cols: usize,
    rows: usize,
    cells: Vec<Vec<u32>>,
    marks: Vec<Mark>,
    gone: Vec<bool>,
    widest: f64,
}

impl Space {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "grid sizes are small and positive"
    )]
    pub fn new(bounds: Rect, gap: f64) -> Self {
        let cell = gap.max(1e-3);
        let margin = Point::splat(cell * 2.0);
        let origin = bounds.min - margin;
        let size = bounds.size() + margin * 2.0;
        let cols = ((size.x / cell).ceil() as usize).clamp(1, 4096);
        let rows = ((size.y / cell).ceil() as usize).clamp(1, 4096);
        // a huge area with a tiny gap gets bigger cells rather than millions of them
        let cell = cell.max(size.x / cols as f64).max(size.y / rows as f64);
        Self {
            gap,
            origin,
            cell,
            cols,
            rows,
            cells: vec![Vec::new(); cols * rows],
            marks: Vec::new(),
            gone: vec![false],
            widest: 0.0,
        }
    }

    /// A new stroke's number.
    pub fn stroke(&mut self) -> u32 {
        self.gone.push(false);
        u32::try_from(self.gone.len() - 1).expect("fewer strokes than u32")
    }

    pub fn points(&self) -> usize {
        self.marks.len()
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped into the grid"
    )]
    fn cell_of(&self, p: Point) -> (usize, usize) {
        let q = (p - self.origin) / self.cell;
        (
            (q.x.max(0.0) as usize).min(self.cols - 1),
            (q.y.max(0.0) as usize).min(self.rows - 1),
        )
    }

    pub fn add(&mut self, at: Point, reach: f64, stroke: u32, index: u32) {
        let (c, r) = self.cell_of(at);
        let id = u32::try_from(self.marks.len()).expect("fewer points than u32");
        self.marks.push(Mark {
            at,
            reach,
            stroke,
            index,
        });
        self.cells[r * self.cols + c].push(id);
        self.widest = self.widest.max(reach);
    }

    /// Lays the edge of `area` down as the wall, so strokes keep the gap from it.
    pub fn wall(&mut self, ring: &[Point], closed: bool) {
        let step = self.gap / 3.0;
        let n = ring.len();
        let spans = if closed { n } else { n.saturating_sub(1) };
        for i in 0..spans {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            for p in along(a, b, step) {
                self.add(p, 0.0, WALL, 0);
            }
        }
    }

    /// Takes a stroke back out (one that came out too short to keep).
    pub fn remove(&mut self, stroke: u32) {
        self.gone[stroke as usize] = true;
    }

    /// Calls `each` with every live point that could be within `reach` of `p`.
    fn near(&self, p: Point, reach: f64, mut each: impl FnMut(&Mark) -> bool) -> bool {
        let (c0, r0) = self.cell_of(p - Point::splat(reach));
        let (c1, r1) = self.cell_of(p + Point::splat(reach));
        for r in r0..=r1 {
            for c in c0..=c1 {
                for &i in &self.cells[r * self.cols + c] {
                    let m = &self.marks[i as usize];
                    if !self.gone[m.stroke as usize] && !each(m) {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// Whether a point of `probe`'s stroke reaching `reach` either side may go at `p`.
    pub fn clear(&self, p: Point, reach: f64, probe: Probe) -> bool {
        let gap = self.gap;
        self.near(p, gap + reach + self.widest, |m| {
            if m.stroke == probe.stroke && m.index + probe.skip >= probe.index {
                return true;
            }
            if let Some((parent, at, within)) = probe.junction
                && m.stroke == parent
                && m.at.distance(at) < within
            {
                return true;
            }
            m.at.distance(p) >= gap + reach + m.reach - 1e-9
        })
    }

    /// The room around `p`: the distance to the nearest stroke's side, up to `most`.
    pub fn room(&self, p: Point, most: f64) -> f64 {
        let mut room = most;
        self.near(p, most + self.widest, |m| {
            room = room.min(m.at.distance(p) - m.reach);
            true
        });
        room
    }
}

/// Points from `a` (left out) to `b` (included) at most `step` apart.
pub fn along(a: Point, b: Point, step: f64) -> impl Iterator<Item = Point> {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a span's sample count"
    )]
    let n = ((a.distance(b) / step).ceil() as usize).max(1);
    #[expect(clippy::cast_precision_loss, reason = "small counts")]
    (1..=n).map(move |i| a.lerp(b, i as f64 / n as f64))
}

/// A stroke grown or drawn: its points, and whether it closes into a shape.
#[derive(Clone, Debug, Default)]
pub struct Stroke {
    pub points: Vec<Point>,
    pub closed: bool,
}

impl Stroke {
    pub fn open(points: Vec<Point>) -> Self {
        Self {
            points,
            closed: false,
        }
    }

    pub fn shape(points: Vec<Point>) -> Self {
        Self {
            points,
            closed: true,
        }
    }

    pub fn mirrored(&self, axis: f64) -> Self {
        Self {
            points: self
                .points
                .iter()
                .map(|p| Point::new(2.0 * axis - p.x, p.y))
                .collect(),
            closed: self.closed,
        }
    }
}

/// The generator's state: the area, the space it fills, the step strokes grow by and the
/// seeded randomness.
pub struct Grower<'a> {
    pub area: &'a Area,
    pub space: Space,
    pub step: f64,
    pub random: Random,
    /// Fuller, overlapping leaves on the composed scrolls, for a relief, where they are
    /// carved over one another rather than outlined side by side.
    pub lush: bool,
}

/// How a walk turns: each step it is given the step's number and the distance grown, and
/// gives the turn (radians) or `None` to stop.
pub struct Walk {
    pub start: Point,
    pub heading: f64,
    pub most: f64,
    pub reach: f64,
    pub junction: Option<(u32, Point)>,
}

impl<'a> Grower<'a> {
    pub fn new(area: &'a Area, gap: f64, seed: u32) -> Self {
        let mut space = Space::new(area.bounds, gap);
        for ring in &area.rings {
            space.wall(ring, true);
        }
        Self {
            area,
            space,
            step: (gap / 3.0).max(0.02),
            random: Random(u64::from(seed).wrapping_mul(0x9e37_79b9) ^ 0x5eed),
            lush: false,
        }
    }

    pub fn gap(&self) -> f64 {
        self.space.gap
    }

    /// Uniform in `[a, b)`.
    pub fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.random.unit()
    }

    pub fn chance(&mut self, p: f64) -> bool {
        self.random.unit() < p
    }

    /// 1 or -1.
    pub fn sign(&mut self) -> f64 {
        if self.chance(0.5) { 1.0 } else { -1.0 }
    }

    /// Whether the fill has grown too big to carry on.
    pub fn full(&self) -> bool {
        self.space.points() > MOST_POINTS
    }

    /// Grows a stroke from `walk.start` a step at a time, turning as `turn` says, until it
    /// would come within the gap of something else, leave the area or reach `walk.most`;
    /// kept (and laid down) only if at least `shortest` long.
    #[expect(clippy::cast_precision_loss, reason = "step counts are small")]
    pub fn walk(
        &mut self,
        walk: &Walk,
        shortest: f64,
        mut turn: impl FnMut(&mut Self, usize, f64) -> Option<f64>,
    ) -> Option<(u32, Vec<Point>, Vec<f64>)> {
        let stroke = self.space.stroke();
        let gap = self.gap();
        let reach = walk.reach;
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a few steps"
        )]
        let skip = (((gap + 2.0 * reach) * 1.6 / self.step).ceil() as u32).max(2);
        let grace = 2.5 * (gap + 2.0 * reach + self.space.widest);
        let junction = walk.junction.map(|(s, at)| (s, at, grace));
        let probe = |index| Probe {
            stroke,
            index,
            skip,
            junction,
        };
        if !self.area.inside(walk.start) || !self.space.clear(walk.start, reach, probe(0)) {
            self.space.remove(stroke);
            return None;
        }
        let (mut points, mut headings) = (vec![walk.start], vec![walk.heading]);
        self.space.add(walk.start, reach, stroke, 0);
        let mut heading = walk.heading;
        let steps = (walk.most / self.step).ceil();
        let mut i = 1usize;
        while (i as f64) <= steps && !self.full() {
            let Some(bend) = turn(self, i, i as f64 * self.step) else {
                break;
            };
            heading += bend;
            let last = *points.last().expect("started");
            let next = last + Point::from_angle(heading) * self.step;
            let index = u32::try_from(i).expect("short strokes");
            if !self.space.clear(next, reach, probe(index)) {
                break;
            }
            self.space.add(next, reach, stroke, index);
            points.push(next);
            headings.push(heading);
            i += 1;
        }
        if (points.len() - 1) as f64 * self.step < shortest {
            self.space.remove(stroke);
            return None;
        }
        Some((stroke, points, headings))
    }

    /// Lays a closed shape down if all of it is inside, clear of everything but `junction`'s
    /// stroke near it; whether it went down.
    pub fn place(&mut self, outline: &[Point], junction: Option<(u32, Point, f64)>) -> bool {
        let stroke = self.space.stroke();
        let mut samples = Vec::new();
        let n = outline.len();
        for i in 0..n {
            samples.extend(along(outline[i], outline[(i + 1) % n], self.step));
        }
        let probe = Probe {
            stroke,
            index: 0,
            skip: u32::MAX,
            junction,
        };
        let fits = samples
            .iter()
            .all(|&p| self.area.inside(p) && self.space.clear(p, 0.0, probe));
        if !fits {
            self.space.remove(stroke);
            return false;
        }
        for p in samples {
            self.space.add(p, 0.0, stroke, 0);
        }
        true
    }
}

/// A leaf-like closed outline along a bending centre line from `base`: `length` long,
/// `width` at its widest, its centre line turning by `bend` radians over its length, and
/// `lobes` scallops along each side (0 for a smooth leaf).
#[expect(clippy::cast_precision_loss, reason = "a few dozen samples")]
pub fn leaf(
    base: Point,
    heading: f64,
    length: f64,
    width: f64,
    bend: f64,
    lobes: u32,
) -> Vec<Point> {
    const N: usize = 40;
    let mut centre = Vec::with_capacity(N + 1);
    let (mut p, mut h) = (base, heading);
    for i in 0..=N {
        centre.push((p, h));
        h += bend / N as f64;
        p += Point::from_angle(h) * (length / N as f64);
        let _ = i;
    }
    let half = |t: f64| {
        let body = (std::f64::consts::PI * t).sin().powf(0.8) * (1.0 - 0.3 * t);
        let scallop = if lobes == 0 {
            1.0
        } else {
            0.8 + 0.2 * (std::f64::consts::TAU * f64::from(lobes) * t).cos().abs()
        };
        width / 2.0 * body * scallop
    };
    let mut left = Vec::with_capacity(N + 1);
    let mut right = Vec::with_capacity(N + 1);
    for (i, &(c, h)) in centre.iter().enumerate() {
        let t = i as f64 / N as f64;
        let n = Point::from_angle(h).perp() * half(t);
        left.push(c + n);
        right.push(c - n);
    }
    // the base and tip are single points
    left.pop();
    right.reverse();
    right.pop();
    left.extend(right);
    left
}

/// Drops points that add nothing (within `tolerance` of the line past them).
pub fn simplify(points: &[Point], tolerance: f64) -> Vec<Point> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;
    let mut stack = vec![(0, points.len() - 1)];
    while let Some((a, b)) = stack.pop() {
        let (pa, pb) = (points[a], points[b]);
        let d = pb - pa;
        let len = d.length();
        let far = (a + 1..b)
            .map(|i| {
                let q = points[i] - pa;
                let off = if len < 1e-12 {
                    q.length()
                } else {
                    (q.perp_dot(d) / len).abs()
                };
                (i, off)
            })
            .max_by(|x, y| x.1.total_cmp(&y.1));
        if let Some((i, off)) = far
            && off > tolerance
        {
            keep[i] = true;
            stack.push((a, i));
            stack.push((i, b));
        }
    }
    points
        .iter()
        .zip(keep)
        .filter_map(|(&p, k)| k.then_some(p))
        .collect()
}

/// The strokes as vectors, with points that add nothing dropped.
pub fn paths(strokes: &[Stroke], gap: f64) -> Vec<Path> {
    let tolerance = (gap / 40.0).clamp(0.005, 0.05);
    strokes
        .iter()
        .filter(|s| s.points.len() > 1)
        .map(|s| {
            let mut points = s.points.clone();
            if s.closed {
                // close at the start so simplifying keeps it whole
                points.push(points[0]);
                let mut kept = simplify(&points, tolerance);
                kept.pop();
                Path::polyline(&kept, true)
            } else {
                Path::polyline(&simplify(&points, tolerance), false)
            }
        })
        .filter(|p| p.closed || !p.segments.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Area {
        Area::new(vec![vec![
            Point::new(0.0, 0.0),
            Point::new(50.0, 0.0),
            Point::new(50.0, 50.0),
            Point::new(0.0, 50.0),
        ]])
    }

    #[test]
    fn a_walk_stops_short_of_the_wall_by_the_gap() {
        let area = square();
        let mut g = Grower::new(&area, 3.0, 1);
        let walk = Walk {
            start: Point::new(25.0, 25.0),
            heading: 0.0,
            most: 100.0,
            reach: 0.0,
            junction: None,
        };
        let (_, points, _) = g.walk(&walk, 1.0, |_, _, _| Some(0.0)).unwrap();
        let end = points.last().unwrap().x;
        assert!((47.0 - g.step..=47.0).contains(&end), "{end}");
    }

    #[test]
    fn a_curling_walk_stops_before_it_crosses_itself() {
        let area = square();
        let mut g = Grower::new(&area, 3.0, 1);
        let walk = Walk {
            start: Point::new(25.0, 10.0),
            heading: 0.0,
            most: 400.0,
            reach: 0.0,
            junction: None,
        };
        // tightening all the time: a spiral inwards
        let (_, points, _) = g
            .walk(&walk, 1.0, |_, _, s| Some(0.02 + s * 0.0004))
            .unwrap();
        let n = points.len();
        for i in 0..n {
            for j in i + 15..n {
                assert!(points[i].distance(points[j]) >= 3.0 - 1e-6 || j - i < 15);
            }
        }
    }

    #[test]
    fn a_short_walk_is_taken_back() {
        let area = square();
        let mut g = Grower::new(&area, 3.0, 1);
        let walk = Walk {
            start: Point::new(46.0, 25.0),
            heading: 0.0,
            most: 100.0,
            reach: 0.0,
            junction: None,
        };
        assert!(g.walk(&walk, 5.0, |_, _, _| Some(0.0)).is_none());
        // and leaves nothing behind
        assert!(g.space.clear(Point::new(44.0, 25.0), 0.0, probe(99)));
    }

    fn probe(stroke: u32) -> Probe {
        Probe {
            stroke,
            index: 0,
            skip: 0,
            junction: None,
        }
    }

    #[test]
    fn simplifying_keeps_corners_and_drops_straight_runs() {
        let pts: Vec<Point> = (0..=10)
            .map(|i| Point::new(f64::from(i), 0.0))
            .chain((1..=10).map(|i| Point::new(10.0, f64::from(i))))
            .collect();
        assert_eq!(
            simplify(&pts, 0.01),
            [Point::ZERO, Point::new(10.0, 0.0), Point::new(10.0, 10.0)]
        );
    }

    #[test]
    fn a_leaf_is_closed_pointed_and_as_long_as_asked() {
        let l = leaf(Point::ZERO, 0.0, 20.0, 6.0, 0.0, 0);
        let b = l.iter().fold(Rect::point(l[0]), |b, &p| b.include(p));
        assert!((b.size().x - 20.0).abs() < 0.6, "{b:?}");
        assert!(b.size().y > 4.0 && b.size().y <= 6.0 + 1e-9);
    }
}
