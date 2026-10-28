//! Arcs fitted to runs of straight cuts, so a post with arc blocks writes one G2 / G3 where a
//! toolpath has many short lines.

use cut_craft_core::glam::{DVec2, DVec3};
use std::f64::consts::PI;

/// The fewest lines one arc replaces: any two fit some circle.
const FEWEST: usize = 3;

/// An arc in the XY plane from the last position to `to`; when `to` is at another height, Z
/// changes evenly along it (a helix).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Arc {
    pub to: DVec3,
    pub centre: DVec2,
    /// The angle it turns through, in radians, anticlockwise positive.
    pub sweep: f64,
}

impl Arc {
    /// The point halfway along it, for an arc starting at `from`.
    pub fn mid(&self, from: DVec3) -> DVec3 {
        let half = DVec2::from_angle(self.sweep / 2.0).rotate(from.truncate() - self.centre);
        (self.centre + half).extend(f64::midpoint(from.z, self.to.z))
    }
}

/// Which arcs may replace lines.
pub struct Limits {
    /// How far an arc may stray from the lines it replaces.
    pub tolerance: f64,
    pub min_radius: f64,
    pub max_radius: f64,
    /// Arcs may change height.
    pub helical: bool,
}

pub enum Piece {
    Line(DVec3),
    Arc(Arc),
}

/// The cuts from `from` through `points`, as lines and the arcs that replace runs of them.
pub fn fit(from: DVec3, points: &[DVec3], limits: &Limits) -> Vec<Piece> {
    let mut all = Vec::with_capacity(points.len() + 1);
    all.push(from);
    all.extend_from_slice(points);
    let mut pieces = vec![];
    let mut i = 0;
    while i + 1 < all.len() {
        if let Some((lines, arc)) = longest(&all[i..], limits) {
            pieces.push(Piece::Arc(arc));
            i += lines;
        } else {
            pieces.push(Piece::Line(all[i + 1]));
            i += 1;
        }
    }
    pieces
}

/// The most lines from the start of `points` that one arc replaces, and that arc.
fn longest(points: &[DVec3], limits: &Limits) -> Option<(usize, Arc)> {
    let fits = |lines: usize| {
        points
            .get(..=lines)
            .and_then(|run| arc_through(run, limits))
    };
    // double until it misses, then halve the gap between the last fit and the miss
    let mut best = (FEWEST, fits(FEWEST)?);
    let mut miss = FEWEST * 2;
    while let Some(arc) = fits(miss) {
        best = (miss, arc);
        miss *= 2;
    }
    while miss - best.0 > 1 {
        let mid = usize::midpoint(best.0, miss);
        match fits(mid) {
            Some(arc) => best = (mid, arc),
            None => miss = mid,
        }
    }
    Some(best)
}

/// The arc through the first, middle and last of `points`, if every line between them stays
/// within the tolerance of it, they all turn the same way, by at most a half turn in all.
// ponytail: a half turn at most, so posts printing the radius ([Radius]) are never ambiguous
fn arc_through(points: &[DVec3], limits: &Limits) -> Option<Arc> {
    let (first, last) = (points[0], points[points.len() - 1]);
    let centre = circumcentre(
        first.truncate(),
        points[points.len() / 2].truncate(),
        last.truncate(),
    )?;
    let radius = first.truncate().distance(centre);
    if !(limits.min_radius..=limits.max_radius).contains(&radius) {
        return None;
    }
    let turn = |w: &[DVec3]| (w[0].truncate() - centre).angle_to(w[1].truncate() - centre);
    let way = turn(&points[..2]).signum();
    let mut sweep = 0.0;
    for w in points.windows(2) {
        let t = turn(w);
        if t * way <= 0.0
            || stray(w[0].truncate(), w[1].truncate(), centre, radius) > limits.tolerance
        {
            return None;
        }
        sweep += t;
    }
    if sweep.abs() > PI + 1e-9 {
        return None;
    }
    // heights change evenly round the arc, or not at all
    let drop = last.z - first.z;
    if drop.abs() > 1e-9 && !limits.helical {
        return None;
    }
    let mut turned = 0.0;
    for w in points.windows(2) {
        turned += turn(w);
        let z = first.z + drop * turned / sweep;
        if (w[1].z - z).abs() > limits.tolerance {
            return None;
        }
    }
    Some(Arc {
        to: last,
        centre,
        sweep,
    })
}

/// The centre of the circle through `a`, `b` and `c`; `None` when they are in a line.
fn circumcentre(a: DVec2, b: DVec2, c: DVec2) -> Option<DVec2> {
    let (ab, ac) = (b - a, c - a);
    let d = 2.0 * ab.perp_dot(ac);
    (d.abs() > f64::MIN_POSITIVE).then(|| {
        let (lb, lc) = (ab.length_squared(), ac.length_squared());
        a + DVec2::new(ac.y * lb - ab.y * lc, ab.x * lc - ac.x * lb) / d
    })
}

/// How far the line from `a` to `b` strays from the circle: the farthest is at an end or
/// at the point nearest the centre.
fn stray(a: DVec2, b: DVec2, centre: DVec2, radius: f64) -> f64 {
    let d = b - a;
    let t = ((centre - a).dot(d) / d.length_squared()).clamp(0.0, 1.0);
    let nearest = (a + d * t).distance(centre);
    (a.distance(centre) - radius)
        .abs()
        .max((b.distance(centre) - radius).abs())
        .max(radius - nearest)
}
