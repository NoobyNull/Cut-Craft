//! Bandanna: paisley teardrops (a round belly tapering to a curled tip, an echo of its
//! outline inside and a flower in its belly) laid biggest first, with small flowers and
//! dots in the room between, as on a bandanna print.

use crate::fill::{Grower, Stroke};
use crate::motif::{Curve, Motif, band, fits, lay, spine};
use crate::scroll::{Made, circle, rosette};
use cut_craft_core::geometry::{Path, Point};
use cut_craft_core::polygon::union;
use std::f64::consts::{FRAC_PI_2, TAU};

/// One paisley in its own frame, its belly `radius` round the origin, its tip curling
/// away over the top.
fn paisley(g: &mut Grower, radius: f64) -> Motif {
    let gap = g.gap();
    let mut m = Motif::default();
    let outline = |g: &mut Grower, r: f64| -> Vec<Vec<Point>> {
        let curve = Curve {
            start: 0.6,
            counter: 0.0,
            wave: 0.0,
            period: 1.0,
            phase: 0.0,
            turns: g.range(0.25, 0.45),
        };
        let length = r * 2.2;
        let (points, headings, bends) = spine(length, r * 0.5, gap, g.step, curve);
        // the spine starts at the belly's middle heading up, so the band's broad foot is
        // the belly's width
        let turned = |p: Point| Point::new(-p.y, p.x);
        let points: Vec<Point> = points.into_iter().map(turned).collect();
        let headings: Vec<f64> = headings.iter().map(|h| h + FRAC_PI_2).collect();
        vec![
            circle(Point::ZERO, r),
            band(&points, &headings, &bends, r * 2.0),
        ]
    };
    let shape = outline(g, radius);
    m.shapes.extend(shape);
    // the echo inside, at a set share of the way in
    if radius >= gap * 4.0 {
        let echo = radius * 0.62;
        let rings: Vec<Path> = outline(g, echo)
            .iter()
            .map(|r| Path::polyline(r, true))
            .collect();
        for joined in union(&rings) {
            let mut ring = joined.flatten(0.01);
            if ring.len() > 1 && ring[0] == ring[ring.len() - 1] {
                ring.pop();
            }
            m.details.push(Stroke::shape(ring));
        }
        let bloom = radius * 0.3;
        if bloom >= gap * 1.5 {
            let petals = if g.chance(0.5) { 5.0 } else { 6.0 };
            let turn = g.range(0.0, TAU);
            m.details
                .push(Stroke::shape(rosette(Point::ZERO, bloom, petals, turn)));
        }
    }
    m
}

/// Paisleys over the grower's area, the biggest first down to about `size` across, then
/// small flowers and dots in the room left.
pub fn bandanna(g: &mut Grower, size: f64) -> Made {
    let gap = g.gap();
    let mut made = Made::default();
    let b = g.area.bounds;
    let smallest = (size * 0.35).max(gap * 2.5);
    let mut radius = b.size().x.min(b.size().y) * 0.2;
    // a paisley is built once and tried in a few places and turns before another is
    // built, and its sample points are turned with it rather than taken again
    let mut ready: Option<(Motif, Vec<Point>, Point)>;
    let mut uses = 0;
    while radius >= smallest && !g.full() {
        let mut misses = 0;
        ready = None;
        while misses < 80 && !g.full() {
            let Some(at) = g.area.random_inside(&mut g.random) else {
                misses += 1;
                continue;
            };
            if g.space.room(at, radius * 0.5) < radius * 0.5 {
                misses += 1;
                continue;
            }
            if ready.is_none() || uses >= 12 {
                let r = radius * g.range(0.9, 1.1);
                let motif = paisley(g, r);
                let points = motif.samples(g.step);
                let centre = motif.centre();
                ready = Some((motif, points, centre));
                uses = 0;
            }
            uses += 1;
            let Some((motif, points, centre)) = &ready else {
                continue;
            };
            let turn = Point::from_angle(g.range(0.0, TAU));
            let y = if g.chance(0.5) { -1.0 } else { 1.0 };
            let to = |p: Point| {
                let d = p - *centre;
                at + turn.rotate(Point::new(d.x, d.y * y))
            };
            let moved: Vec<Point> = points.iter().map(|&p| to(p)).collect();
            if fits(g, &moved, None) {
                let placed = motif.map(to);
                lay(g, placed, &moved, &mut made);
                ready = None;
                misses = 0;
            } else {
                misses += 1;
            }
        }
        radius *= 0.8;
    }
    // small four-petal flowers, then dots, where they fit
    for (share, petals) in [(0.25, 4.0), (0.12, 0.0)] {
        let r = (size * share).max(gap * if petals > 0.0 { 2.0 } else { 0.6 });
        let mut misses = 0;
        while misses < 60 && !g.full() {
            let Some(at) = g.area.random_inside(&mut g.random) else {
                misses += 1;
                continue;
            };
            let outline = if petals > 0.0 {
                rosette(at, r, petals, g.range(0.0, TAU))
            } else {
                circle(at, r)
            };
            let bit = Motif {
                shapes: vec![outline],
                details: vec![],
                pieces: vec![],
            };
            let points = bit.samples(g.step);
            if g.space.room(at, r + gap) >= r + gap && fits(g, &points, None) {
                lay(g, bit, &points, &mut made);
                misses = 0;
            } else {
                misses += 1;
            }
        }
    }
    made
}
