//! The fills that are not scrolls: frost, fractal branches, snowflakes, stones, ripples and
//! hammered dimples.

use crate::fill::{Area, Grower, Stroke, Walk};
use cut_craft_core::geometry::{Path, Point, winding};
use cut_craft_core::polygon::{self, Corners};
use std::f64::consts::{FRAC_PI_3, TAU};

/// Points along the area's edge about `spacing` apart, each with the heading straight in.
fn edge_starts(g: &mut Grower, spacing: f64) -> Vec<(Point, f64)> {
    let gap = g.gap();
    let mut starts = Vec::new();
    let mut next = spacing * g.range(0.2, 1.0);
    for ring in &g.area.rings.clone() {
        let n = ring.len();
        let mut run = 0.0;
        for i in 0..n {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            let len = a.distance(b);
            while run + len >= next {
                let at = a.lerp(b, (next - run) / len.max(1e-12));
                let normal = (b - a).normalize_or_zero().perp();
                let into = if g.area.inside(at + normal * gap * 0.5) {
                    normal
                } else {
                    -normal
                };
                starts.push((at + into * gap * 1.15, into.to_angle()));
                next += spacing * g.range(0.7, 1.3);
            }
            run += len;
        }
        next -= run;
    }
    // shuffled, so one side of the shape does not crowd out the rest
    for i in (1..starts.len()).rev() {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss,
            reason = "an index"
        )]
        let j = (g.random.unit() * (i + 1) as f64) as usize;
        starts.swap(i, j.min(i));
    }
    starts
}

/// Somewhere with at least `room` clear round it, or `None` if no try found one.
fn roomy(g: &mut Grower, room: f64) -> Option<Point> {
    (0..20)
        .filter_map(|_| g.area.random_inside(&mut g.random))
        .find(|&p| g.space.room(p, room) >= room)
}

/// Frost: ice ferns growing in from the edge, each a wandering spine with side branches at
/// sixty degrees, shorter towards the tip, which branch again.
pub fn frost(g: &mut Grower, size: f64) -> Vec<Stroke> {
    let mut strokes = Vec::new();
    for (at, heading) in edge_starts(g, size * 0.6) {
        if g.full() {
            break;
        }
        let length = size * g.range(1.0, 2.0);
        fern(g, at, heading, length, size, 0, None, &mut strokes);
    }
    let mut misses = 0;
    while misses < 40 && !g.full() {
        let Some(at) = roomy(g, size * 0.3) else {
            misses += 1;
            continue;
        };
        let heading = g.range(0.0, TAU);
        let length = size * g.range(0.6, 1.2);
        if fern(g, at, heading, length, size, 0, None, &mut strokes) {
            misses = 0;
        } else {
            misses += 1;
        }
    }
    strokes
}

#[expect(clippy::too_many_arguments, reason = "a branch's start and its parent")]
fn fern(
    g: &mut Grower,
    at: Point,
    heading: f64,
    length: f64,
    size: f64,
    depth: u32,
    parent: Option<u32>,
    strokes: &mut Vec<Stroke>,
) -> bool {
    let gap = g.gap();
    let mut drift = 0.0;
    let walk = Walk {
        start: at,
        heading,
        most: length,
        reach: 0.0,
        junction: parent.map(|p| (p, at)),
    };
    let Some((stroke, points, headings)) = g.walk(&walk, gap * 2.0, |g, _, _| {
        drift = drift * 0.9 + g.range(-0.012, 0.012);
        Some(drift)
    }) else {
        return false;
    };
    #[expect(clippy::cast_precision_loss, reason = "points along a spine")]
    let grown = (points.len() - 1) as f64 * g.step;
    strokes.push(Stroke::open(points.clone()));
    if depth >= 2 {
        return true;
    }
    let spacing = (gap * 1.7).max(size * 0.07) * if depth == 0 { 1.0 } else { 1.2 };
    let mut s = spacing * g.range(0.8, 1.2);
    while s < grown - gap {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "an index along the spine"
        )]
        let i = ((s / g.step) as usize).min(points.len() - 1);
        // longest near the base, as a fern's fronds are
        let left = (grown - s) * if depth == 0 { 0.45 } else { 0.4 };
        for side in [1.0, -1.0] {
            let turn = side * (FRAC_PI_3 + g.range(-0.14, 0.14));
            let len = left * g.range(0.7, 1.0) + gap;
            fern(
                g,
                points[i],
                headings[i] + turn,
                len,
                size,
                depth + 1,
                Some(stroke),
                strokes,
            );
        }
        s += spacing * g.range(0.85, 1.25);
    }
    true
}

/// A fractal branch to grow.
struct Branch {
    at: Point,
    heading: f64,
    length: f64,
    depth: u32,
    parent: Option<u32>,
}

/// Fractal branches (Lichtenberg figures): jagged lines from the edge that fork again and
/// again, each fork shorter, until the area is full.
pub fn fractal(g: &mut Grower, size: f64) -> Vec<Stroke> {
    let gap = g.gap();
    let mut strokes = Vec::new();
    let mut queue: Vec<Branch> = Vec::new();
    let starts = edge_starts(g, size * 2.5);
    let mut starts = starts.into_iter();
    let mut misses = 0;
    while misses < 40 && !g.full() {
        if queue.is_empty() {
            let root = starts.next().or_else(|| {
                roomy(g, size * 0.4).map(|p| {
                    let h = g.range(0.0, TAU);
                    (p, h)
                })
            });
            let Some((at, heading)) = root else {
                misses += 1;
                continue;
            };
            let length = size * g.range(3.0, 5.0);
            queue.push(Branch {
                at,
                heading,
                length,
                depth: 0,
                parent: None,
            });
        }
        let b = queue.remove(0);
        // jagged: a new direction every so often, kept near the branch's own
        let mut kink = 0.0;
        let mut offset = 0.0;
        let walk = Walk {
            start: b.at,
            heading: b.heading,
            most: b.length,
            reach: 0.0,
            junction: b.parent.map(|p| (p, b.at)),
        };
        let grown = g.walk(&walk, gap * 2.0, |g, _, s| {
            if s < kink {
                return Some(0.0);
            }
            kink = s + gap * g.range(1.0, 2.2);
            let target = (offset + g.range(-0.55, 0.55)).clamp(-0.6, 0.6);
            let turn = target - offset;
            offset = target;
            Some(turn)
        });
        let Some((stroke, points, headings)) = grown else {
            if b.parent.is_none() {
                misses += 1;
            }
            continue;
        };
        misses = 0;
        #[expect(clippy::cast_precision_loss, reason = "points along a branch")]
        let grown = (points.len() - 1) as f64 * g.step;
        if b.depth < 7 {
            let mut s = gap * g.range(2.0, 4.0);
            while s < grown - gap * 2.0 {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "an index along the branch"
                )]
                let i = ((s / g.step) as usize).min(points.len() - 1);
                if g.chance(0.6) {
                    let turn = g.sign() * g.range(0.35, 0.85);
                    let length = (b.length - s) * g.range(0.45, 0.8) + size * 0.2;
                    queue.push(Branch {
                        at: points[i],
                        heading: headings[i] + turn,
                        length,
                        depth: b.depth + 1,
                        parent: Some(stroke),
                    });
                }
                s += gap * g.range(2.0, 4.0);
            }
        }
        strokes.push(Stroke::open(points));
    }
    strokes
}

/// Discs scattered over the area without touching, biggest first: for snowflakes and
/// dimples. Each is `(centre, radius)`.
fn scatter(g: &mut Grower, biggest: f64, smallest: f64) -> Vec<(Point, f64)> {
    const GIVE_UP: u32 = 400;
    let gap = g.gap();
    let mut discs = Vec::new();
    let mut misses = 0;
    while misses < GIVE_UP && discs.len() < 20_000 {
        let t = f64::from(misses) / f64::from(GIVE_UP);
        let r = (biggest + (smallest - biggest) * t) * g.range(0.8, 1.0);
        let r = r.max(smallest);
        let Some(p) = g.area.random_inside(&mut g.random) else {
            misses += 1;
            continue;
        };
        if g.space.room(p, r + gap) < r + gap {
            misses += 1;
            continue;
        }
        let stroke = g.space.stroke();
        g.space.add(p, r, stroke, 0);
        discs.push((p, r));
        misses = misses.saturating_sub(2);
    }
    discs
}

/// Snowflakes of different sizes scattered over the area, each six arms with branches at
/// sixty degrees, some round a hexagon.
pub fn snowflakes(g: &mut Grower, size: f64) -> Vec<Stroke> {
    let gap = g.gap();
    let biggest = size / 2.0;
    let smallest = (size * 0.12).max(gap * 3.0).min(biggest);
    let discs = scatter(g, biggest, smallest);
    discs
        .into_iter()
        .flat_map(|(c, r)| flake(g, c, r))
        .collect()
}

#[expect(clippy::cast_precision_loss, reason = "a few branches")]
fn flake(g: &mut Grower, centre: Point, r: f64) -> Vec<Stroke> {
    let gap = g.gap();
    let turn = g.range(0.0, FRAC_PI_3);
    let hexagon = r >= gap * 6.0 && g.chance(0.4);
    let core = if hexagon {
        (r * 0.2).max(gap * 1.2)
    } else {
        0.0
    };
    let arm = |k: usize| Point::from_angle(turn + FRAC_PI_3 * k as f64);
    let mut strokes = Vec::new();
    if hexagon {
        strokes.push(Stroke::shape(
            (0..6).map(|k| centre + arm(k) * core).collect(),
        ));
        for k in 0..6 {
            strokes.push(Stroke::open(vec![
                centre + arm(k) * core,
                centre + arm(k) * r,
            ]));
        }
    } else {
        for k in 0..3 {
            strokes.push(Stroke::open(vec![centre - arm(k) * r, centre + arm(k) * r]));
        }
    }
    // where the branches leave the arm, the same on every arm
    let first = (core + gap * 1.5).max(gap * 1.6);
    let mut t = first + g.range(0.0, gap);
    let mut branches = Vec::new();
    while t < r * 0.85 && branches.len() < 4 {
        // a branch runs parallel to the next arm, closing on its mirror image there
        let most = f64::midpoint(-t, (4.0 * r * r - 3.0 * t * t).max(0.0).sqrt());
        let length = (r * g.range(0.25, 0.55))
            .min(t - gap * 1.2)
            .min(most - gap * 0.3);
        if length >= gap {
            branches.push((t, length));
        }
        t += gap * g.range(1.7, 2.6);
    }
    for k in 0..6 {
        let a = arm(k);
        for &(t, length) in &branches {
            let base = centre + a * t;
            let left = base + a.rotate(Point::from_angle(FRAC_PI_3)) * length;
            let right = base + a.rotate(Point::from_angle(-FRAC_PI_3)) * length;
            strokes.push(Stroke::open(vec![left, base, right]));
        }
    }
    strokes
}

/// Hammered: round dimples of a few sizes packed over the area.
pub fn hammered(g: &mut Grower, size: f64) -> Vec<Path> {
    let gap = g.gap();
    let biggest = (size / 5.0).max(gap);
    let smallest = (size / 10.0).max(gap * 0.5).min(biggest);
    scatter(g, biggest, smallest)
        .into_iter()
        .map(|(c, r)| Path::circle(c, r))
        .collect()
}

/// Stones: the area cut into rounded cells, a gap between them, as cobbles or crazy paving.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "grid counts are small"
)]
/// `None` when the stones would be too many to make.
pub fn stones(g: &mut Grower, size: f64) -> Option<Vec<Path>> {
    let gap = g.gap();
    let spacing = size * 0.5;
    let b = g.area.bounds;
    let (min, extent) = (
        b.min - Point::splat(spacing),
        b.size() + Point::splat(spacing * 2.0),
    );
    let cols = ((extent.x / spacing).ceil() as usize).max(1);
    let rows = ((extent.y / spacing).ceil() as usize).max(1);
    if cols * rows > 100_000 {
        return None;
    }
    let mut seeds = Vec::with_capacity(cols * rows);
    for r in 0..rows {
        for c in 0..cols {
            let jitter = Point::new(g.range(0.1, 0.9), g.range(0.1, 0.9));
            seeds.push(min + (Point::new(c as f64, r as f64) + jitter) * spacing);
        }
    }
    let mut cells = Vec::new();
    for r in 0..rows {
        for c in 0..cols {
            let p = seeds[r * cols + c];
            let mut cell = vec![
                p + Point::new(-2.0, -2.0) * spacing,
                p + Point::new(2.0, -2.0) * spacing,
                p + Point::new(2.0, 2.0) * spacing,
                p + Point::new(-2.0, 2.0) * spacing,
            ];
            for rr in r.saturating_sub(2)..(r + 3).min(rows) {
                for cc in c.saturating_sub(2)..(c + 3).min(cols) {
                    if (rr, cc) != (r, c) {
                        cell = half_plane(&cell, p, seeds[rr * cols + cc]);
                    }
                }
            }
            if cell.len() > 2 {
                cells.push(Path::polyline(&cell, true));
            }
        }
    }
    let round = spacing * 0.15;
    let shrunk = polygon::offset(&cells, -(gap / 2.0 + round), Corners::Round);
    let stones = polygon::offset(&shrunk, round, Corners::Round);
    // the area pulled in by the gap: its outlines run anticlockwise, its holes clockwise,
    // and a boolean counts every closed path as filled, so holes are taken away after
    let inside = polygon::offset_regions(&polygon::regions(&g.area.paths()), -gap, Corners::Round);
    let (outlines, holes): (Vec<Path>, Vec<Path>) = inside
        .into_iter()
        .partition(|p| winding(&p.flatten(polygon::TOLERANCE)) > 0.0);
    polygon::difference(&polygon::intersection(&stones, &outlines), &holes)
        .into_iter()
        .filter(|p| polygon::area(p).abs() >= (gap * 1.5).powi(2))
        .collect::<Vec<_>>()
        .into()
}

/// The part of convex polygon `cell` nearer `p` than `q`.
fn half_plane(cell: &[Point], p: Point, q: Point) -> Vec<Point> {
    let (mid, dir) = ((p + q) / 2.0, q - p);
    let side = |x: Point| (x - mid).dot(dir);
    let mut out = Vec::with_capacity(cell.len() + 1);
    for i in 0..cell.len() {
        let (a, b) = (cell[i], cell[(i + 1) % cell.len()]);
        let (sa, sb) = (side(a), side(b));
        if sa <= 0.0 {
            out.push(a);
        }
        if (sa < 0.0) != (sb < 0.0) && (sa - sb).abs() > 1e-12 {
            out.push(a.lerp(b, sa / (sa - sb)));
        }
    }
    out
}

/// Ripples: lines following the edge, each a step further in, as contour lines.
pub fn ripples(area: &Area, size: f64, gap: f64) -> Vec<Path> {
    let spacing = (size / 5.0).max(gap);
    let regions = polygon::regions(&area.paths());
    let mut out = Vec::new();
    for k in 1..2000 {
        let ring = polygon::offset_regions(&regions, -spacing * f64::from(k), Corners::Round);
        if ring.is_empty() {
            break;
        }
        out.extend(ring);
    }
    out
}
