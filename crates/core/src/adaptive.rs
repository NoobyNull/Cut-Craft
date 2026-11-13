//! Adaptive clearing: an area cleared with the bit's bite held near a target everywhere.
//!
//! The bit starts in a small spiral, then each short step it steers so the part of its
//! leading edge in uncut material stays at or under the angle a side cut `bite` wide would
//! take: in open areas it sweeps round in growing loops, in narrow ones it turns tight, and
//! in corners it takes the material in thin slices instead of cutting full width. When a cut
//! runs out of material it moves through space it has already cleared to the nearest place
//! with material left, and carries on from there. It never goes outside `region`.
//!
//! The material is kept on a fine grid; the toolpath is the bit's centre in the plane, the
//! same for every depth of a straight-walled cut. Our own code: the idea is the general one
//! (hold the engagement near a target), no other program's code was used.

use crate::geometry::{Path, Point};
use crate::polygon::{self, Corners};
use crate::toolpath::cut_points;
use std::f64::consts::{FRAC_PI_2, PI, TAU};

/// What to clear, with which bit.
#[derive(Clone, Copy, Debug)]
pub struct Clearing<'a> {
    /// Where the bit's centre may go: closed regions, a path inside another a hole (an island).
    pub region: &'a [Path],
    /// Material already gone (cleared by an earlier, bigger bit, say): closed regions.
    pub cleared: &'a [Path],
    /// The bit's cutting radius.
    pub radius: f64,
    /// How wide a bite the bit takes, at the most, cutting along a wall: millimetres.
    pub bite: f64,
    /// Climb (the material on the bit's right as it goes) or conventional (on its left).
    pub climb: bool,
}

/// One piece of the clearing, in order.
#[derive(Clone, Debug, PartialEq)]
pub enum Piece {
    /// Go down into uncut material here (the entry the toolpath chose: a plunge, a helix…)
    Enter(Point),
    /// Cut through these points, from where the last piece ended.
    Cut(Vec<Point>),
    /// Move through these points at the cutting depth: the space is already cleared.
    Link(Vec<Point>),
    /// Lift clear of the material, move above it, and go back down here into cleared space.
    Jump(Point),
}

/// The engagement, on the leading half of the bit's edge, a side cut `bite` wide takes, in
/// degrees, for a bit `radius` across its middle.
pub fn engagement_for(bite: f64, radius: f64) -> f64 {
    (1.0 - (bite / radius).clamp(0.0, 2.0)).acos().to_degrees()
}

/// The pieces that clear `c`: empty when there is nothing to clear.
pub fn clear(c: &Clearing) -> Vec<Piece> {
    let Some(mut work) = Work::new(c) else {
        return vec![];
    };
    work.run();
    work.pieces
}

/// Most cells the material grid has: bigger areas get coarser cells.
const MOST_CELLS: f64 = 3.0e6;

/// Points round the bit's edge the engagement is measured at.
const EDGE: usize = 120;

/// A square grid of cells over the area worked on.
struct Grid {
    origin: Point,
    cell: f64,
    nx: usize,
    ny: usize,
}

impl Grid {
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn new(min: Point, max: Point, cell: f64) -> Self {
        let nx = ((max.x - min.x) / cell).ceil() as usize + 1;
        let ny = ((max.y - min.y) / cell).ceil() as usize + 1;
        Self {
            origin: min,
            cell,
            nx,
            ny,
        }
    }

    fn len(&self) -> usize {
        self.nx * self.ny
    }

    /// The column and row of the cell `p` is in, which may be off the grid.
    #[expect(clippy::cast_possible_truncation)]
    fn at(&self, p: Point) -> (i64, i64) {
        let q = (p - self.origin) / self.cell;
        (q.x.round() as i64, q.y.round() as i64)
    }

    #[expect(
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation
    )]
    fn index(&self, (i, j): (i64, i64)) -> Option<usize> {
        (i >= 0 && j >= 0 && i < self.nx as i64 && j < self.ny as i64)
            .then(|| j as usize * self.nx + i as usize)
    }

    #[expect(clippy::cast_precision_loss)]
    fn centre(&self, k: usize) -> Point {
        self.origin + self.cell * Point::new((k % self.nx) as f64, (k / self.nx) as f64)
    }

    /// The cells whose centres are inside `rings` (a ring inside another is a hole).
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    fn fill(&self, rings: &[Vec<Point>]) -> Vec<bool> {
        let mut cells = vec![false; self.len()];
        let mut xs = vec![];
        for j in 0..self.ny {
            let y = self.origin.y + self.cell * j as f64;
            xs.clear();
            for ring in rings {
                for (n, &a) in ring.iter().enumerate() {
                    let b = ring[(n + 1) % ring.len()];
                    if (a.y > y) != (b.y > y) {
                        xs.push(a.x + (y - a.y) * (b.x - a.x) / (b.y - a.y));
                    }
                }
            }
            xs.sort_by(f64::total_cmp);
            for pair in xs.as_chunks::<2>().0 {
                let from = ((pair[0] - self.origin.x) / self.cell).ceil().max(0.0) as usize;
                let to = ((pair[1] - self.origin.x) / self.cell).floor();
                if to < 0.0 {
                    continue;
                }
                let to = (to as usize).min(self.nx - 1);
                for i in from..=to {
                    cells[j * self.nx + i] = true;
                }
            }
        }
        cells
    }

    /// For each cell, the offset in cells to the nearest cell where `site` is set, or `None`
    /// when no cell is (two sweeps passing nearest offsets on: within a fraction of a cell).
    #[expect(
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation
    )]
    fn nearest(&self, site: &[bool]) -> Vec<Option<(i32, i32)>> {
        let (nx, ny) = (self.nx as i64, self.ny as i64);
        let mut near: Vec<Option<(i32, i32)>> = site.iter().map(|&s| s.then_some((0, 0))).collect();
        let d2 = |o: (i32, i32)| i64::from(o.0).pow(2) + i64::from(o.1).pow(2);
        let look = |near: &mut Vec<Option<(i32, i32)>>, i: i64, j: i64, di: i64, dj: i64| {
            let (a, b) = (i + di, j + dj);
            if a < 0 || b < 0 || a >= nx || b >= ny {
                return;
            }
            let Some(o) = near[(b * nx + a) as usize] else {
                return;
            };
            #[expect(clippy::cast_possible_truncation)]
            let o = (o.0 + di as i32, o.1 + dj as i32);
            let here = &mut near[(j * nx + i) as usize];
            if here.is_none_or(|h| d2(o) < d2(h)) {
                *here = Some(o);
            }
        };
        for j in 0..ny {
            for i in 0..nx {
                for (di, dj) in [(-1, 0), (-1, -1), (0, -1), (1, -1)] {
                    look(&mut near, i, j, di, dj);
                }
            }
            for i in (0..nx).rev() {
                look(&mut near, i, j, 1, 0);
            }
        }
        for j in (0..ny).rev() {
            for i in (0..nx).rev() {
                for (di, dj) in [(1, 0), (1, 1), (0, 1), (-1, 1)] {
                    look(&mut near, i, j, di, dj);
                }
            }
            for i in 0..nx {
                look(&mut near, i, j, -1, 0);
            }
        }
        near
    }
}

/// The clearing as it goes.
struct Work {
    grid: Grid,
    /// Uncut material.
    material: Vec<bool>,
    /// Where the bit's centre may go.
    allowed: Vec<bool>,
    /// The cells within the bit's radius of its centre, as offsets.
    disk: Vec<(i64, i64)>,
    /// The cells past the bit's edge and within a bite of it, as offsets: material there
    /// is what a cut from a place can take.
    ring: Vec<(i64, i64)>,
    /// Points round the bit's edge, as unit vectors.
    edge: Vec<Point>,
    radius: f64,
    bite: f64,
    /// The engagement held to, in degrees.
    target: f64,
    step: f64,
    climb: bool,
    /// Places a cut was tried from and went nowhere.
    tried: Vec<bool>,
    pieces: Vec<Piece>,
}

impl Work {
    #[expect(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    fn new(c: &Clearing) -> Option<Self> {
        let rings = |paths: &[Path]| -> Vec<Vec<Point>> {
            paths
                .iter()
                .filter(|p| p.closed)
                .map(cut_points)
                .filter(|r| r.len() > 2)
                .collect()
        };
        let reach = polygon::offset_regions(c.region, c.radius, Corners::Round);
        let reach_rings = rings(&reach);
        let points = reach_rings.iter().flatten();
        let (min, max) = points.fold(
            (Point::splat(f64::MAX), Point::splat(f64::MIN)),
            |(lo, hi), &p| (lo.min(p), hi.max(p)),
        );
        if min.x > max.x || c.radius <= 0.0 || c.bite <= 0.0 {
            return None;
        }
        let size = max - min;
        let cell = (c.radius / 20.0).max((size.x * size.y / MOST_CELLS).sqrt());
        let grid = Grid::new(
            min - Point::splat(2.0 * cell),
            max + Point::splat(2.0 * cell),
            cell,
        );
        // a cell short of the region's edge, so a centre in an allowed cell is in the region;
        // the material is what the bit reaches from there (the wall's own pass takes the rest)
        let allowed_region = polygon::offset_regions(c.region, -cell, Corners::Round);
        let allowed = grid.fill(&rings(&allowed_region));
        let reachable = polygon::offset_regions(&allowed_region, c.radius, Corners::Round);
        let mut material = grid.fill(&rings(&reachable));
        for (m, gone) in material.iter_mut().zip(grid.fill(&rings(c.cleared))) {
            *m &= !gone;
        }
        // the cells that may be under the bit, wherever in its cell its centre is
        let k = (c.radius / cell).ceil() as i64 + 1;
        let disk = (-k..=k)
            .flat_map(|j| (-k..=k).map(move |i| (i, j)))
            .filter(|&(i, j)| ((i * i + j * j) as f64).sqrt() * cell <= c.radius + cell)
            .collect();
        let bite = c.bite.min(c.radius);
        let out = ((c.radius + bite) / cell).ceil() as i64;
        let ring = (-out..=out)
            .flat_map(|j| (-out..=out).map(move |i| (i, j)))
            .filter(|&(i, j)| {
                let d = ((i * i + j * j) as f64).sqrt() * cell;
                d > c.radius - cell && d <= c.radius + bite
            })
            .collect();
        let edge = (0..EDGE)
            .map(|n| Point::from_angle(TAU * n as f64 / EDGE as f64))
            .collect();
        Some(Self {
            tried: vec![false; grid.len()],
            grid,
            material,
            allowed,
            disk,
            ring,
            edge,
            radius: c.radius,
            bite,
            // a little under, for the grid's coarseness: the cut never goes over
            target: 0.95 * engagement_for(bite, c.radius),
            step: (c.radius / 10.0).max(cell),
            climb: c.climb,
            pieces: vec![],
        })
    }

    fn is(&self, cells: &[bool], p: Point) -> bool {
        self.grid.index(self.grid.at(p)).is_some_and(|k| cells[k])
    }

    fn may_go(&self, p: Point) -> bool {
        self.is(&self.allowed, p)
    }

    /// Degrees of the bit's edge, on the half leading towards `heading`, in uncut material
    /// with the bit's centre at `p`; more than any engagement when `p` is out of bounds.
    #[expect(clippy::cast_precision_loss)]
    fn engagement(&self, p: Point, heading: Point) -> f64 {
        if !self.may_go(p) {
            return f64::INFINITY;
        }
        // just inside the edge: material there is cut, not rubbed past
        // just inside its edge: a cell there is under the bit, so it is cut, not rubbed past
        let r = self.radius - 0.5 * self.grid.cell;
        let r2 = self.radius * self.radius;
        let touching = self
            .edge
            .iter()
            .filter(|e| e.dot(heading) >= 0.0)
            .filter_map(|e| self.grid.index(self.grid.at(p + r * *e)))
            .filter(|&k| self.material[k] && self.grid.centre(k).distance_squared(p) <= r2)
            .count();
        touching as f64 * 360.0 / EDGE as f64
    }

    /// The engagement moving to `p` heading `heading`, but no more than the target when the
    /// material taken is no more than a side cut's: head on into a thin sliver (in a
    /// corner) the edge touches a lot of material but takes very little.
    #[expect(clippy::cast_precision_loss)]
    fn load(&self, p: Point, heading: Point) -> f64 {
        let angle = self.engagement(p, heading);
        if angle <= self.target || angle.is_infinite() {
            return angle;
        }
        let taken =
            self.under(p).filter(|&k| self.material[k]).count() as f64 * self.grid.cell.powi(2);
        if taken <= self.bite * self.step {
            self.target
        } else {
            angle
        }
    }

    /// How many cells of material a cut from `p` could take.
    fn in_reach(&self, p: Point) -> usize {
        let (ci, cj) = self.grid.at(p);
        self.ring
            .iter()
            .filter(|&&(i, j)| {
                self.grid
                    .index((ci + i, cj + j))
                    .is_some_and(|k| self.material[k])
            })
            .count()
    }

    /// The bit's centre moves to `p`: the material under it is gone. How many cells were.
    fn cut_at(&mut self, p: Point) -> usize {
        let under: Vec<usize> = self.under(p).collect();
        let mut gone = 0;
        for k in under {
            if self.material[k] {
                self.material[k] = false;
                gone += 1;
            }
        }
        gone
    }

    /// The cells whose middles are under the bit with its centre at `p`.
    fn under(&self, p: Point) -> impl Iterator<Item = usize> + '_ {
        let (ci, cj) = self.grid.at(p);
        let r2 = self.radius * self.radius;
        self.disk.iter().filter_map(move |&(i, j)| {
            let k = self.grid.index((ci + i, cj + j))?;
            (self.grid.centre(k).distance_squared(p) <= r2).then_some(k)
        })
    }

    /// Fewest cells of material a cut from a place must take for it to be worth going
    /// there: more than a scallop's worth.
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn enough(&self) -> usize {
        (0.5 * self.bite * self.step / self.grid.cell.powi(2)).max(2.0) as usize
    }

    /// `heading` turned `by` radians away from the material side.
    fn turned(&self, heading: Point, by: f64) -> Point {
        let by = if self.climb { by } else { -by };
        Point::from_angle(by).rotate(heading)
    }

    fn run(&mut self) {
        // ponytail: a cap on the work, should the clearing ever fail to make progress
        let most = self.grid.len() * 4;
        let mut at: Option<Point> = None;
        for _ in 0..most {
            let start = match at.and_then(|p| self.next_from(p)) {
                Some(start) => Some(start),
                None => self.enter(),
            };
            let Some((from, heading)) = start else {
                break;
            };
            let (cut, taken) = self.cut_from(from, heading);
            if cut.len() < 2 || taken < self.enough() {
                let (ci, cj) = self.grid.at(from);
                for &(i, j) in &self.disk {
                    if let Some(k) = self.grid.index((ci + i / 2, cj + j / 2)) {
                        self.tried[k] = true;
                    }
                }
                if cut.len() >= 2 {
                    at = cut.last().copied();
                    self.pieces.push(Piece::Cut(cut));
                } else {
                    at = Some(from);
                }
                continue;
            }
            at = cut.last().copied();
            self.pieces.push(Piece::Cut(cut));
        }
        // drop moves that end the clearing going nowhere
        while matches!(self.pieces.last(), Some(Piece::Link(_) | Piece::Jump(_))) {
            self.pieces.pop();
        }
    }

    /// A new start in uncut material: the place in it furthest from the region's edge, with a
    /// spiral out from it, and where the spiral ends and which way it is going.
    fn enter(&mut self) -> Option<(Point, Point)> {
        // the place deepest in uncut material: furthest from anything cut or out of reach
        let open: Vec<bool> = self.material.iter().map(|&m| !m).collect();
        let open = self.grid.nearest(&open);
        let d2 = |o: Option<(i32, i32)>| {
            o.map_or(i64::MAX, |(a, b)| i64::from(a).pow(2) + i64::from(b).pow(2))
        };
        let (k, depth) = (0..self.grid.len())
            .filter(|&k| self.allowed[k] && self.material[k] && !self.tried[k])
            .map(|k| (k, d2(open[k])))
            .max_by_key(|&(_, d)| d)?;
        // nothing left but slivers thinner than a bite
        #[expect(clippy::cast_precision_loss)]
        if (depth as f64).sqrt() * self.grid.cell < self.bite {
            return None;
        }
        let centre = self.grid.centre(k);
        self.pieces.push(Piece::Enter(centre));
        let _ = self.cut_at(centre);
        // out in a spiral, a bite further each turn, until the bit has room to steer
        let mut spiral = vec![centre];
        let ccw = self.climb; // material outside, on the right of an anticlockwise turn
        let mut angle: f64 = 0.0;
        loop {
            let reach = self.bite * angle / TAU;
            if reach >= self.radius {
                break;
            }
            angle += (self.step / reach.max(self.step)).min(PI / 8.0);
            let reach = self.bite * angle / TAU;
            let a = if ccw { angle } else { -angle };
            let p = centre + reach * Point::from_angle(a);
            if !self.may_go(p) {
                break;
            }
            let _ = self.cut_at(p);
            spiral.push(p);
        }
        let n = spiral.len();
        let heading = if n > 1 {
            (spiral[n - 1] - spiral[n - 2]).normalize_or(Point::X)
        } else {
            Point::X
        };
        let end = spiral[n - 1];
        if n > 1 {
            self.pieces.push(Piece::Cut(spiral));
        }
        Some((end, heading))
    }

    /// The bit's path from `from`, heading `heading`, steering to hold the engagement at the
    /// target, until it has gone a while without cutting or cannot go on.
    #[expect(clippy::cast_precision_loss)]
    fn cut_from(&mut self, from: Point, mut heading: Point) -> (Vec<Point>, usize) {
        let mut path = vec![from];
        let mut taken = 0;
        let mut idle = 0usize;
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let most_idle = (PI * self.radius / self.step).ceil() as usize;
        let mut at = from;
        let mut last_cut = 1;
        loop {
            let try_turn = |w: &Self, a: f64| {
                let d = w.turned(heading, a);
                (w.load(at + w.step * d, d), d)
            };
            // from hard towards the material side round to back the way it came: the first
            // turn that holds the engagement, refined against the one before it
            // (turning towards the material no tighter than half the bit's radius, so with
            // nothing to cut it curves round to find some instead of spinning on the spot)
            let turns = 24;
            let tightest = (self.step / (0.5 * self.radius)).min(FRAC_PI_2);
            let turn = |n: usize| -tightest + (PI + tightest) * n as f64 / turns as f64;
            let Some(n) = (0..=turns).find(|&n| try_turn(self, turn(n)).0 <= self.target) else {
                break;
            };
            let a = if n == 0 {
                turn(0)
            } else {
                let (mut ok, mut over) = (turn(n), turn(n - 1));
                for _ in 0..8 {
                    let mid = f64::midpoint(ok, over);
                    if try_turn(self, mid).0 <= self.target {
                        ok = mid;
                    } else {
                        over = mid;
                    }
                }
                ok
            };
            let d = self.turned(heading, a);
            at += self.step * d;
            heading = d;
            let gone = self.cut_at(at);
            taken += gone;
            path.push(at);
            // cutting only when material actually goes: a step that just grazes is idle
            if gone > 0 {
                idle = 0;
                last_cut = path.len();
            } else {
                idle += 1;
                if idle > most_idle {
                    break;
                }
            }
            if path.len() > self.grid.len() {
                break;
            }
        }
        path.truncate(last_cut);
        (path, taken)
    }

    /// The next place to cut from after a cut ending at `at`: the nearest place, going
    /// through cleared space, where the bit fits with material in reach, and the move there
    /// at depth; with no way through, the nearest place at all and a jump to it.
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::many_single_char_names
    )]
    fn next_from(&mut self, at: Point) -> Option<(Point, Point)> {
        let near = self.grid.nearest(&self.material);
        let cell = self.grid.cell;
        let gap = |k: usize| near[k].map(|(a, b)| f64::from(a).hypot(f64::from(b)) * cell);
        let fits = self.radius - 0.5 * cell;
        let free: Vec<bool> = (0..self.grid.len())
            .map(|k| self.allowed[k] && gap(k).is_none_or(|g| g >= fits))
            .collect();
        let enough = self.enough();
        let place = |w: &mut Self, k: usize| -> bool {
            if w.tried[k] || !gap(k).is_some_and(|g| g <= w.radius + w.bite) {
                return false;
            }
            // enough material in reach to make a cut worth it: not just a scallop
            let worth = w.in_reach(w.grid.centre(k)) >= enough;
            if !worth {
                w.tried[k] = true;
            }
            worth
        };
        // out through the cleared space, nearest first
        let route = self.grid.index(self.grid.at(at)).and_then(|start| {
            let mut from = vec![usize::MAX; self.grid.len()];
            from[start] = start;
            let mut queue = std::collections::VecDeque::from([start]);
            let (nx, ny) = (self.grid.nx, self.grid.ny);
            while let Some(k) = queue.pop_front() {
                if k != start && place(self, k) {
                    let mut cells = vec![k];
                    while *cells.last().expect("a cell") != start {
                        cells.push(from[*cells.last().expect("a cell")]);
                    }
                    cells.reverse();
                    return Some(cells);
                }
                let (i, j) = (k % nx, k / nx);
                for (di, dj) in [
                    (1, 0),
                    (0, 1),
                    (-1, 0),
                    (0, -1),
                    (1, 1),
                    (-1, 1),
                    (1, -1),
                    (-1, -1),
                ] {
                    let (a, b) = (i.wrapping_add_signed(di), j.wrapping_add_signed(dj));
                    if a < nx && b < ny {
                        let n = b * nx + a;
                        if free[n] && from[n] == usize::MAX {
                            from[n] = k;
                            queue.push_back(n);
                        }
                    }
                }
            }
            None
        });
        let clear = |a: Point, b: Point| {
            let steps = (a.distance(b) / cell).ceil().max(1.0);
            (1..steps as usize).all(|n| {
                let p = a.lerp(b, n as f64 / steps);
                self.grid.index(self.grid.at(p)).is_some_and(|k| free[k])
            })
        };
        let (to, piece) = if let Some(cells) = route {
            // the route as few straight moves as stay in the clear
            let points: Vec<Point> = std::iter::once(at)
                .chain(cells[1..].iter().map(|&k| self.grid.centre(k)))
                .collect();
            let mut way = vec![at];
            let mut i = 0;
            while i + 1 < points.len() {
                let mut j = points.len() - 1;
                while j > i + 1 && !clear(points[i], points[j]) {
                    j -= 1;
                }
                way.push(points[j]);
                i = j;
            }
            (*way.last().expect("a place"), Piece::Link(way))
        } else {
            // nowhere reachable at depth: the nearest place at all
            let mut spots: Vec<(usize, f64)> = (0..self.grid.len())
                .filter(|&k| free[k])
                .map(|k| (k, self.grid.centre(k).distance_squared(at)))
                .collect();
            spots.sort_by(|a, b| a.1.total_cmp(&b.1));
            let k = spots
                .into_iter()
                .map(|(k, _)| k)
                .find(|&k| place(self, k))?;
            let to = self.grid.centre(k);
            (to, Piece::Jump(to))
        };
        let k = self.grid.index(self.grid.at(to)).expect("on the grid");
        let (a, b) = near[k].expect("material in reach");
        let towards = Point::new(f64::from(a), f64::from(b)).normalize_or(Point::X);
        // material on the right going climb: heading a quarter turn left of it
        let heading = self.turned(towards, FRAC_PI_2);
        self.pieces.push(piece);
        Some((to, heading))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::square;

    const R: f64 = 3.0;
    const BITE: f64 = 0.9;

    fn clearing(region: &[Path]) -> Clearing<'_> {
        Clearing {
            region,
            cleared: &[],
            radius: R,
            bite: BITE,
            climb: true,
        }
    }

    /// Where the bit's centre may go for a pocket of `shapes` (a shape inside another an island).
    fn centre_region(shapes: &[Path]) -> Vec<Path> {
        polygon::offset_regions(&polygon::regions(shapes), -R, Corners::Round)
    }

    /// The tool centre's path through `pieces`, a polyline per piece that moves at depth,
    /// with whether it is a cut.
    fn runs(pieces: &[Piece]) -> Vec<(Vec<Point>, bool)> {
        let mut out = vec![];
        let mut at: Option<Point> = None;
        for piece in pieces {
            match piece {
                Piece::Enter(p) | Piece::Jump(p) => at = Some(*p),
                Piece::Cut(points) | Piece::Link(points) => {
                    let mut run: Vec<Point> = at.into_iter().collect();
                    run.extend(points);
                    at = run.last().copied();
                    out.push((run, matches!(piece, Piece::Cut(_))));
                }
            }
        }
        out
    }

    /// Cutting simulated on a fine grid of its own: the most engagement anywhere (in degrees,
    /// the leading half of the edge) where the cut is wider than the bite, leaving out the
    /// spirals in from each entry; the most material one link takes (mm²); and how thick
    /// the material left within the bit's reach of `region` is.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        clippy::many_single_char_names
    )]
    fn simulate(region: &[Path], pieces: &[Piece], bite: f64) -> (f64, f64, f64) {
        let cell = 0.08;
        let reach = polygon::offset_regions(region, R, Corners::Round);
        let rings: Vec<Vec<Point>> = reach.iter().map(cut_points).collect();
        let (lo, hi) = rings.iter().flatten().fold(
            (Point::splat(f64::MAX), Point::splat(f64::MIN)),
            |(lo, hi), &p| (lo.min(p), hi.max(p)),
        );
        let grid = Grid::new(lo - Point::splat(1.0), hi + Point::splat(1.0), cell);
        let mut material = grid.fill(&rings);
        let k = (R / cell).ceil() as i64 + 1;
        let disk: Vec<(i64, i64)> = (-k..=k)
            .flat_map(|j| (-k..=k).map(move |i| (i, j)))
            .collect();
        let under = |p: Point| -> Vec<usize> {
            let (ci, cj) = grid.at(p);
            disk.iter()
                .filter_map(|&(i, j)| grid.index((ci + i, cj + j)))
                .filter(|&n| grid.centre(n).distance(p) <= R)
                .collect()
        };
        let edge: Vec<Point> = (0..360)
            .map(|a| Point::from_angle(f64::from(a).to_radians()))
            .collect();
        // degrees of the leading half of the edge in material that this step cuts
        let engaged = |material: &Vec<bool>, p: Point, d: Point| {
            edge.iter()
                .filter(|e| e.dot(d) >= 0.0)
                .filter_map(|e| grid.index(grid.at(p + (R - 0.5 * cell) * *e)))
                .filter(|&n| material[n] && grid.centre(n).distance(p) <= R)
                .count() as f64
        };
        let enters: Vec<Point> = pieces
            .iter()
            .filter_map(|p| match p {
                Piece::Enter(p) => Some(*p),
                _ => None,
            })
            .collect();
        let target = engagement_for(bite, R);
        let (mut worst, mut worst_link) = (0.0_f64, 0.0_f64);
        for (run, is_cut) in runs(pieces) {
            for n in under(run[0]) {
                material[n] = false;
            }
            // what a link takes, all told: it may graze, never cut
            let mut link_took = 0.0;
            // the material taken over the last few steps, for how wide the cut is
            let mut recent = std::collections::VecDeque::<(f64, f64)>::default();
            for w in run.windows(2) {
                let d = (w[1] - w[0]).normalize_or_zero();
                let n = (w[0].distance(w[1]) / cell).ceil().max(1.0) as usize;
                let len = w[0].distance(w[1]) / n as f64;
                for s in 1..=n {
                    let p = w[0].lerp(w[1], s as f64 / n as f64);
                    let e = engaged(&material, p, d);
                    let mut gone = 0.0;
                    for k in under(p) {
                        if material[k] {
                            material[k] = false;
                            gone += cell * cell;
                        }
                    }
                    recent.push_back((gone, len));
                    if recent.len() > 6 {
                        recent.pop_front();
                    }
                    let (area, along) = recent
                        .iter()
                        .fold((0.0, 0.0), |(a, l), &(g, s)| (a + g, l + s));
                    let width = area / along.max(1e-9);
                    if !is_cut {
                        link_took += gone;
                        worst_link = worst_link.max(link_took);
                    } else if enters.iter().all(|c| c.distance(p) > 1.5 * R) {
                        // overloaded: the edge deep in material and the cut wide too
                        let load = if width > 1.2 * bite { e } else { e.min(target) };
                        worst = worst.max(load);
                    }
                }
            }
        }
        // material the bit could reach, by a clear margin, that is still there
        let inside_reach: Vec<Vec<Point>> =
            polygon::offset_regions(region, R - 0.2, Corners::Round)
                .iter()
                .map(cut_points)
                .collect();
        let reachable = grid.fill(&inside_reach);
        // how thick the thickest of what is left is: from each cell left to the nearest cut
        let cut: Vec<bool> = material.iter().map(|&m| !m).collect();
        let near_cut = grid.nearest(&cut);
        let left = (0..grid.len())
            .filter(|&n| material[n] && reachable[n])
            .filter_map(|n| near_cut[n])
            .map(|(a, b)| f64::from(a).hypot(f64::from(b)) * cell)
            .fold(0.0, f64::max);
        (worst, worst_link, left)
    }

    fn check(name: &str, region: &[Path], pieces: &[Piece]) {
        check_bite(name, region, pieces, BITE);
    }

    fn check_bite(name: &str, region: &[Path], pieces: &[Piece], bite: f64) {
        assert!(!pieces.is_empty(), "{name}: nothing cut");
        let (worst, worst_link, left) = simulate(region, pieces, bite);
        let target = engagement_for(bite, R);
        assert!(
            worst <= target + 12.0,
            "{name}: a bite of {worst}° (target {target}°)"
        );
        assert!(
            worst_link <= 0.5,
            "{name}: a link cuts {worst_link} mm² away"
        );
        // what is left (in corners the bite can't reach) is thinner than a bite: the wall's
        // own pass takes it
        assert!(left <= bite, "{name}: material {left} mm thick left");
        // never outside where the centre may go
        let rings: Vec<Vec<Point>> = polygon::offset_regions(region, 0.01, Corners::Round)
            .iter()
            .map(cut_points)
            .collect();
        for (run, _) in runs(pieces) {
            for p in run {
                assert!(crate::geometry::inside(&rings, p), "{name}: {p} outside");
            }
        }
    }

    #[test]
    fn the_check_sees_a_full_width_slot() {
        let region = centre_region(&[square(0.0, 0.0, 40.0)]);
        let p = Point::new;
        let slot = [
            Piece::Enter(p(5.0, 20.0)),
            Piece::Cut(vec![p(5.0, 20.0), p(35.0, 20.0)]),
        ];
        let (worst, _, left) = simulate(&region, &slot, BITE);
        assert!(worst > 100.0, "a slot reads as {worst}°");
        assert!(left > 5.0, "{left} mm left");
    }

    #[test]
    fn a_square_pocket_is_cleared_with_the_bite_held() {
        let region = centre_region(&[square(0.0, 0.0, 40.0)]);
        check("square", &region, &clear(&clearing(&region)));
    }

    #[test]
    fn a_wider_bite_is_held_too() {
        // a quarter of the bit: what Pocket takes with a usual 40 % stepover
        let region = centre_region(&[square(0.0, 0.0, 40.0)]);
        let c = Clearing {
            bite: 1.5,
            ..clearing(&region)
        };
        check_bite("quarter", &region, &clear(&c), 1.5);
    }

    #[test]
    fn an_l_shaped_pocket_with_an_inside_corner() {
        let p = Point::new;
        let l = Path::polyline(
            &[
                p(0.0, 0.0),
                p(60.0, 0.0),
                p(60.0, 20.0),
                p(25.0, 20.0),
                p(25.0, 45.0),
                p(0.0, 45.0),
            ],
            true,
        );
        let region = centre_region(&[l]);
        check("L", &region, &clear(&clearing(&region)));
    }

    #[test]
    fn an_island_is_gone_round() {
        let region = centre_region(&[square(0.0, 0.0, 40.0), square(15.0, 15.0, 10.0)]);
        let pieces = clear(&clearing(&region));
        check("island", &region, &pieces);
        // the bit's centre stays a radius from the island (15..25 square)
        for (run, _) in runs(&pieces) {
            for q in run {
                let off = (q - Point::splat(20.0)).abs() - Point::splat(5.0);
                let gap = off.max(Point::ZERO).length();
                assert!(gap >= R - 0.05, "{q} is {gap} mm from the island");
            }
        }
    }

    #[test]
    fn a_narrow_neck_between_two_rooms() {
        let p = Point::new;
        // two 20 mm rooms joined by a neck 8 mm wide (the bit is 6 mm)
        let shape = Path::polyline(
            &[
                p(0.0, 0.0),
                p(20.0, 0.0),
                p(20.0, 6.0),
                p(30.0, 6.0),
                p(30.0, 0.0),
                p(50.0, 0.0),
                p(50.0, 20.0),
                p(30.0, 20.0),
                p(30.0, 14.0),
                p(20.0, 14.0),
                p(20.0, 20.0),
                p(0.0, 20.0),
            ],
            true,
        );
        let region = centre_region(&[shape]);
        check("neck", &region, &clear(&clearing(&region)));
    }

    #[test]
    fn a_slot_two_bits_wide() {
        let p = Point::new;
        let slot = Path::polyline(
            &[p(0.0, 0.0), p(60.0, 0.0), p(60.0, 12.0), p(0.0, 12.0)],
            true,
        );
        let region = centre_region(&[slot]);
        check("slot", &region, &clear(&clearing(&region)));
    }

    #[test]
    fn separate_pockets_each_get_an_entry() {
        let region = centre_region(&[square(0.0, 0.0, 20.0), square(30.0, 0.0, 20.0)]);
        let pieces = clear(&clearing(&region));
        check("two", &region, &pieces);
        let enters = pieces
            .iter()
            .filter(|p| matches!(p, Piece::Enter(_)))
            .count();
        assert_eq!(enters, 2);
    }

    #[test]
    fn material_already_cleared_is_not_cut_again() {
        let region = centre_region(&[square(0.0, 0.0, 30.0)]);
        let all = [square(-5.0, -5.0, 40.0)];
        let c = Clearing {
            cleared: &all,
            ..clearing(&region)
        };
        assert_eq!(clear(&c), vec![]);
    }

    #[test]
    fn nothing_to_clear_when_the_bit_does_not_fit() {
        assert_eq!(clear(&clearing(&[])), vec![]);
    }

    #[test]
    fn the_engagement_for_a_bite() {
        assert!((engagement_for(R, R) - 90.0).abs() < 1e-9);
        assert!((engagement_for(0.0, R)).abs() < 1e-9);
        assert!((engagement_for(2.0 * R, R) - 180.0).abs() < 1e-9);
    }
}
