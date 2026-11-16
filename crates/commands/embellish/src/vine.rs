//! Folk mix, Scrolls and Flowers as a repeating pattern along a line: a wavy vine down the
//! middle, and in each half-wave a main piece on the side away from the vine's hump (a
//! scroll curling off the vine, or a flower), a smaller piece on the hump's side (a flower
//! or a pair of leaves) and a dot where the vine crosses the middle, each half the mirror
//! of the one before, so they alternate top, bottom, top and scrolls curl left, right,
//! left. Every piece is the same as the last unless random is allowed.
//!
//! Pieces are drawn along the line (`s`) and across it (`v`) from `0` to the line's length,
//! then laid onto it by the border code.

use crate::motif::{
    Ctx, Item, Track, band, circ, flower_ring, pt, rosette, sh, trace, willow_leaf,
};
use crate::rng::Rng;
use cut_craft_core::geometry::Point;
use std::f64::consts::PI;

/// Which arrangement it has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Scrolls off the vine, small flowers on the hump's side, dots.
    FolkMix,
    /// Scrolls off the vine, leaves on the hump's side, dots.
    Scrolls,
    /// Flowers away from the vine, leaves on the hump's side, dots.
    Flowers,
}

/// Which pieces go in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[expect(clippy::struct_excessive_bools, reason = "one tick box each")]
pub struct Parts {
    pub vine: bool,
    pub scrolls: bool,
    pub flowers: bool,
    pub leaves: bool,
    pub dots: bool,
}

impl Parts {
    pub const ALL: Self = Self {
        vine: true,
        scrolls: true,
        flowers: true,
        leaves: true,
        dots: true,
    };
}

/// How the vine pattern is drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Opts {
    pub kind: Kind,
    pub parts: Parts,
    /// 0 to 1: eyes in the scrolls, rosettes rather than plain flowers.
    pub detail: f64,
    /// 0 to 1: how much of its place each piece fills.
    pub fill: f64,
    /// Picks the pieces' shape: the flowers' petals, how tight the scrolls curl, how wide
    /// the leaves fan. Every repeat has the same shape.
    pub seed: u32,
    /// Each repeat's pieces a different size, from the seed.
    pub vary_size: bool,
    /// Each repeat's pieces turned a different way, from the seed.
    pub vary_turn: bool,
    /// Without the main pieces (they do not fit) the pattern is not made at all, so it is
    /// tried again at a smaller gap; off at the smallest gap, so something is always made.
    pub need_main: bool,
    /// How each half-wave is drawn: 1 humps up, 2 down, 3 and 4 the same turned back to
    /// front, 0 a straight stretch with nothing; empty is 1 over and over.
    pub seq: crate::seq::Seq,
}

/// A piece of the pattern and the point it turns about when it is kept upright rather than
/// turned with the line (`None` for the vine, which always follows it).
pub struct Piece {
    pub item: Item,
    pub anchor: Option<Point>,
}

/// The pieces' shape, picked by the Layout number: the same in every repeat.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Shape {
    /// How far a scroll curls, in half turns.
    curl: f64,
    /// How much tighter a scroll gets toward its eye.
    tight: f64,
    /// How wide a pair of leaves fans, radians either side of upright.
    fan: f64,
}

impl Shape {
    fn from_seed(seed: u32) -> Self {
        // the first layout is the one the pattern was designed with
        if seed <= 1 {
            return Self {
                curl: 1.55,
                tight: 0.4,
                fan: 0.55,
            };
        }
        let mut r = Rng::new(seed);
        Self {
            curl: r.range(1.35, 1.75),
            tight: r.range(0.3, 0.5),
            fan: r.range(0.4, 0.7),
        }
    }
}

/// A box along and across the line.
#[derive(Clone, Copy, Debug)]
struct Slot {
    s0: f64,
    s1: f64,
    v0: f64,
    v1: f64,
}

impl Slot {
    fn w(&self) -> f64 {
        self.s1 - self.s0
    }
    fn h(&self) -> f64 {
        self.v1 - self.v0
    }
    fn centre(&self) -> Point {
        pt(
            f64::midpoint(self.s0, self.s1),
            f64::midpoint(self.v0, self.v1),
        )
    }
}

fn bounds(items: &[Item]) -> Option<(Point, Point)> {
    items.iter().flat_map(Item::points).fold(None, |b, q| {
        Some(b.map_or((q, q), |(lo, hi): (Point, Point)| (lo.min(q), hi.max(q))))
    })
}

/// `make(size)` made as big as fits in `slot` (no bigger than `most`), its box centred in
/// it; nothing when even a small one does not fit.
fn fitted(slot: Slot, most: f64, make: impl Fn(f64) -> Vec<Item>) -> Vec<Item> {
    fitted_at(slot, most, make).0
}

/// [`fitted`], and how far it was moved (where its (0, 0) went).
fn fitted_at(slot: Slot, most: f64, make: impl Fn(f64) -> Vec<Item>) -> (Vec<Item>, Option<Point>) {
    let mut size = most.min(slot.w().min(slot.h()));
    for _ in 0..14 {
        let items = make(size);
        if let Some((lo, hi)) = bounds(&items) {
            let span = hi - lo;
            if span.x <= slot.w() && span.y <= slot.h() {
                let shift = slot.centre() - (lo + hi) / 2.0;
                let moved = items.iter().map(|it| it.map(&|q| q + shift)).collect();
                return (moved, Some(shift));
            }
        }
        size *= 0.85;
    }
    (vec![], None)
}

/// The pattern along a line `len` long, `hw` either side of it, its half-waves about `p`
/// long, the pieces `g` apart. A closed line has an even number of half-waves, so the
/// alternation meets itself.
pub fn pieces(len: f64, closed: bool, hw: f64, p: f64, g: f64, o: &Opts) -> Vec<Piece> {
    let seq = o.seq;
    let mut n = (len / p).round().max(1.0);
    if closed {
        // a whole number of turns of the sequence, so it meets itself
        let l = seq.len().max(1) as f64;
        n = ((n / l).round() * l).max(l);
    }
    let h = len / n;
    // which way the vine humps in each half-wave: up, down, or not at all for a space
    let hump = |k: usize| match seq.turn(k) {
        None => 0.0,
        Some(t) if t.across => -1.0,
        Some(_) => 1.0,
    };
    let half = |s: f64| ((s / h).floor().max(0.0) as usize).min(n as usize - 1);
    let parts = o.parts;
    let (amp, wv) = if parts.vine {
        (hw * 0.28, (g * 1.1).max(hw * 0.12))
    } else {
        (0.0, 0.0)
    };
    let vine_v = |s: f64| amp * hump(half(s)) * (PI * s / h).sin().abs();
    let mut out = vec![];
    if parts.vine {
        let steps = ((len / 0.5).ceil() as usize).max(8);
        let ss: Vec<f64> = (0..=steps).map(|i| len * i as f64 / steps as f64).collect();
        let t = Track {
            p: ss.iter().map(|&s| pt(s, vine_v(s))).collect(),
            h: ss
                .iter()
                .map(|&s| {
                    let k = half(s);
                    let local = s - k as f64 * h;
                    (amp * hump(k) * PI / h * (PI * local / h).cos()).atan()
                })
                .collect(),
        };
        out.push(Piece {
            item: sh(band(&t, &vec![wv; t.p.len()], !closed, !closed), 1.0),
            anchor: None,
        });
    }
    // the room either side of the vine, a gap off it
    let off = if parts.vine { wv / 2.0 + g } else { g / 2.0 };
    let edge = hw - g * 0.25;
    let mut rng = Rng::new(o.seed.wrapping_mul(2_654_435_761).wrapping_add(1));
    let shape = Shape::from_seed(o.seed);
    for k in 0..n as usize {
        let (s0, s1) = (k as f64 * h, (k as f64 + 1.0) * h);
        // the vine humps toward `side` in this half-wave; nothing in a space
        let Some(turn) = seq.turn(k) else { continue };
        let side = if turn.across { -1.0 } else { 1.0 };
        // turned back to front: mirrored along about the half-wave's middle
        let mid = f64::midpoint(s0, s1);
        let back = |q: Point| {
            if turn.back {
                pt(2.0 * mid - q.x, q.y)
            } else {
                q
            }
        };
        let margin = h * 0.08 + g / 2.0;
        let (a, b) = (s0 + margin, s1 - margin);
        // away from the hump: down to the far edge, up to the vine where it is lowest
        let low = amp * (PI * margin / h).sin() - off;
        let main = Slot {
            s0: a,
            s1: b,
            v0: -edge,
            v1: low,
        };
        let hump = Slot {
            s0: a + h * 0.1,
            s1: b - h * 0.1,
            v0: amp + off,
            v1: edge,
        };
        // each repeat's own size and turn, when they vary
        let jitter = if o.vary_size || turn.size {
            rng.range(0.7, 1.0)
        } else {
            1.0
        };
        let lean = if o.vary_turn || turn.lean {
            rng.range(-PI / 4.0, PI / 4.0)
        } else {
            0.0
        };
        let fill = (0.7 + 0.3 * o.fill) * jitter;
        // a piece drawn above the middle, mirrored below it in every other half-wave
        let mut place = |items: Vec<Item>, slot: Slot, side: f64| {
            let anchor = slot.centre();
            let m = |q: Point| back(pt(q.x, q.y * side));
            // turned about the middle of its place, smaller where it would reach out of it
            let angle = turn.angle * side + lean;
            let items = if angle == 0.0 {
                items
            } else {
                let spin = crate::seq::Turn {
                    across: false,
                    back: false,
                    angle,
                    size: false,
                    lean: false,
                };
                crate::border::oriented(
                    &items,
                    pt(slot.s0, slot.v0),
                    pt(slot.s1, slot.v1),
                    spin,
                    1.0,
                )
            };
            for it in items {
                out.push(Piece {
                    item: it.map(&m),
                    anchor: Some(m(anchor)),
                });
            }
        };
        // the first layout's flowers are the ones the pattern was designed with
        let seed = if o.seed <= 1 { 7 } else { o.seed };
        let main_items = match o.kind {
            Kind::FolkMix | Kind::Scrolls if parts.scrolls => scroll(
                main,
                fill,
                g,
                o.detail,
                &shape,
                parts
                    .vine
                    .then_some(|s: f64| amp * (PI * (s - s0) / h).sin()),
            ),
            Kind::Flowers if parts.flowers => flower(main, fill, g, o.detail, seed),
            _ => vec![],
        };
        let wanted = match o.kind {
            Kind::FolkMix | Kind::Scrolls => parts.scrolls,
            Kind::Flowers => parts.flowers,
        };
        // the main piece asked for and too big for its place at this gap: none of the
        // pattern, so it is made again at a smaller gap (and the form says so)
        if o.need_main && wanted && main_items.is_empty() {
            return vec![];
        }
        // the main piece is drawn on the side away from the hump: below when it humps up
        place(main_items, main, side);
        // on the hump's side a small flower for Folk mix, else (or where none fits) leaves
        let mut hump_items = match o.kind {
            Kind::FolkMix if parts.flowers => flower(hump, fill, g, 0.0, seed),
            _ => vec![],
        };
        if hump_items.is_empty() && parts.leaves {
            hump_items = leaves(hump, fill, g, &shape);
        }
        place(hump_items, hump, side);
        // a dot where the vine crosses the middle, on alternate sides
        if parts.dots && (closed || k > 0) {
            let r = (h * 0.07).min(hw * 0.2).max(g * 0.5);
            let dot = Slot {
                s0: s0 - r,
                s1: s0 + r,
                v0: edge - 2.0 * r,
                v1: edge,
            };
            if 2.0 * r < edge - off {
                let c = dot.centre();
                let dside = -side;
                out.push(Piece {
                    item: sh(circ(pt(c.x, c.y * dside), r), 0.75),
                    anchor: Some(pt(c.x, c.y * dside)),
                });
            }
        }
    }
    out
}

/// A scroll in `slot` (below the vine), its foot heading down from the vine at its left and
/// curling round to the right, as `shape` says. With the vine, a stem joins
/// its foot to the vine.
fn scroll(
    slot: Slot,
    fill: f64,
    g: f64,
    detail: f64,
    shape: &Shape,
    vine: Option<impl Fn(f64) -> f64>,
) -> Vec<Item> {
    let most = slot.w().min(slot.h()) * fill;
    // heading down and rolling to the right, the foot at (0, 0)
    let turn = move |q: Point| pt(q.y, -q.x);
    let (items, shift) = fitted_at(slot, most, |size| {
        hook(size, g, detail, shape)
            .iter()
            .map(|i| i.map(&turn))
            .collect()
    });
    let mut items = items;
    if let (Some(vine), Some(shift), false) = (vine, shift, items.is_empty()) {
        // the stem, as wide as the scroll's band at its foot
        let foot = shift;
        let w = (g * 1.1).max(slot.h() * 0.06);
        let top = vine(foot.x);
        if top > foot.y {
            let steps = ((top - foot.y) / 0.4).ceil().max(2.0) as usize;
            let t = Track {
                p: (0..=steps)
                    .map(|i| pt(foot.x, top - (top - foot.y) * i as f64 / steps as f64))
                    .collect(),
                h: vec![-PI / 2.0; steps + 1],
            };
            items.push(sh(band(&t, &vec![w; steps + 1], false, false), 0.95));
        }
    }
    items
}

/// A scroll `size` across: from (0, 0) heading along +x it rolls up to the left, tighter as
/// it goes, through most of a turn, ending in a round eye. One turn only, so it keeps the
/// gap `g` from itself at any size a bit can carve.
fn hook(size: f64, g: f64, detail: f64, shape: &Shape) -> Vec<Item> {
    let w = (g * 1.1).max(size * 0.13);
    let r = (size - w) / 2.0;
    let eye = w * if detail > 0.5 { 1.0 } else { 0.8 };
    // the eye at the end of the turn keeps a gap from the band where it started
    if r < w + eye + g {
        return vec![];
    }
    let total = PI * shape.curl;
    let tight = shape.tight;
    let k = |t: f64, _| 1.0 / (r * (1.0 - tight * t));
    // the length that turns through `total` as the radius shrinks
    let len = r * (1.0 - tight / 2.0) * total;
    let t = trace(Point::ZERO, 0.0, len, k, (len / 160.0).max(0.1));
    let n = t.p.len();
    let widths: Vec<f64> = (0..n)
        .map(|i| w * (1.0 - 0.3 * i as f64 / (n - 1) as f64))
        .collect();
    let end = t.p[n - 1];
    let mut out = vec![sh(band(&t, &widths, true, false), 0.95)];
    out.push(sh(circ(end, eye), 1.0));
    out
}

/// A flower in `slot`: a rosette when there is detail, else a plain ring of petals.
fn flower(slot: Slot, fill: f64, g: f64, detail: f64, seed: u32) -> Vec<Item> {
    let most = slot.w().min(slot.h()) * fill;
    fitted(slot, most, |size| {
        let mut c = Ctx {
            g,
            detail,
            rng: Rng::new(seed),
        };
        let made = if detail > 0.5 {
            rosette(size, &mut c)
        } else {
            flower_ring(size, &mut c)
        };
        made.unwrap_or_default()
    })
}

/// A pair of leaves fanning up and out from the slot's foot.
fn leaves(slot: Slot, fill: f64, g: f64, shape: &Shape) -> Vec<Item> {
    let most = slot.w().min(slot.h()) * fill;
    fitted(slot, most, |size| {
        let (l, wl) = (size * 0.9, size * 0.32);
        if wl < g * 1.2 {
            return vec![];
        }
        let mut out = vec![];
        for (a, k) in [
            (PI * 0.5 - shape.fan, 0.012),
            (PI * 0.5 + shape.fan, -0.012),
        ] {
            let foot = pt(a.cos(), a.sin()) * (g * 0.8);
            out.extend(willow_leaf(foot, a, l, wl, k * 30.0 / size, g, false));
        }
        out
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(kind: Kind) -> Opts {
        Opts {
            kind,
            parts: Parts::ALL,
            detail: 0.5,
            fill: 0.7,
            seed: 1,
            vary_size: false,
            vary_turn: false,
            need_main: true,
            seq: crate::seq::Seq::EMPTY,
        }
    }

    #[test]
    fn every_piece_stays_in_the_band_and_the_line() {
        for kind in [Kind::FolkMix, Kind::Scrolls, Kind::Flowers] {
            let all = pieces(300.0, false, 15.0, 30.0, 1.0, &opts(kind));
            assert!(all.len() > 20, "{kind:?}: {}", all.len());
            for piece in &all {
                for q in piece.item.points() {
                    assert!(q.y.abs() <= 15.0 + 1e-6, "{kind:?}: {q:?} out of the band");
                    assert!(
                        (-1.0..=301.0).contains(&q.x),
                        "{kind:?}: {q:?} off the line"
                    );
                }
            }
        }
    }

    #[test]
    fn the_main_pieces_alternate_sides_and_repeat_exactly() {
        let o = Opts {
            seq: crate::seq::Seq::parse("12"),
            ..opts(Kind::Flowers)
        };
        let all = pieces(300.0, false, 15.0, 30.0, 1.0, &o);
        // the flowers' middles (their anchors), half-wave by half-wave
        let mut mains: Vec<Point> = all
            .iter()
            .filter_map(|p| p.anchor)
            // the half-waves are 30 long: their pieces sit at the middle of each, the dots
            // at the ends
            .filter(|a| a.y.abs() > 4.0 && (a.x % 30.0 - 15.0).abs() < 0.5)
            .collect();
        mains.sort_by(|a, b| a.x.total_cmp(&b.x));
        mains.dedup_by(|a, b| a.distance(*b) < 1e-6);
        assert!(mains.len() >= 10);
        let below: Vec<&Point> = mains.iter().filter(|a| a.y < 0.0).collect();
        let above: Vec<&Point> = mains.iter().filter(|a| a.y > 0.0).collect();
        assert!(below.len() >= 5 && above.len() >= 5, "on both sides");
        // the same piece every other half-wave: the same shape, moved along (below the
        // middle, main pieces and the hump's pieces alternate)
        let shape = |at: &Point| -> Vec<Point> {
            let mut pts: Vec<Point> = all
                .iter()
                .filter(|p| p.anchor == Some(*at))
                .flat_map(|p| p.item.points().map(|q| q - *at).collect::<Vec<_>>())
                .collect();
            pts.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
            pts
        };
        let (first, second) = (shape(below[0]), shape(below[2]));
        assert_eq!(first.len(), second.len());
        for (a, b) in first.iter().zip(&second) {
            assert!(a.distance(*b) < 1e-6);
        }
    }

    #[test]
    fn unticked_pieces_are_left_out_and_random_varies_them() {
        let only_vine = Opts {
            parts: Parts {
                vine: true,
                scrolls: false,
                flowers: false,
                leaves: false,
                dots: false,
            },
            ..opts(Kind::FolkMix)
        };
        assert_eq!(pieces(300.0, false, 15.0, 30.0, 1.0, &only_vine).len(), 1);
        let count = |o: &Opts| {
            pieces(300.0, false, 15.0, 30.0, 1.0, o)
                .iter()
                .map(|p| p.item.points().count())
                .sum::<usize>()
        };
        let plain = opts(Kind::Scrolls);
        let random = Opts {
            vary_size: true,
            seed: 3,
            ..plain
        };
        assert_eq!(count(&plain), count(&plain), "the same every time");
        assert_ne!(count(&plain), count(&random), "random changes it");
    }

    /// The main pieces' middles, half-wave by half-wave (30 long, the pieces at the middle).
    fn mains(all: &[Piece]) -> Vec<Point> {
        let mut m: Vec<Point> = all
            .iter()
            .filter_map(|p| p.anchor)
            .filter(|a| a.y.abs() > 4.0 && (a.x % 30.0 - 15.0).abs() < 0.5)
            .collect();
        m.sort_by(|a, b| a.x.total_cmp(&b.x));
        m.dedup_by(|a, b| a.distance(*b) < 1e-6);
        m
    }

    #[test]
    fn the_sequence_says_which_way_each_half_wave_goes() {
        let with = |text: &str| Opts {
            seq: crate::seq::Seq::parse(text),
            ..opts(Kind::Flowers)
        };
        // nothing typed is 1 over and over: every main piece on the same side
        let same = mains(&pieces(300.0, false, 15.0, 30.0, 1.0, &with("")));
        assert!(same.len() >= 10);
        let first: Vec<f64> = same.iter().filter(|a| a.x < 30.0).map(|a| a.y).collect();
        for k in 1..10 {
            let here: Vec<f64> = same
                .iter()
                .filter(|a| (a.x / 30.0).floor() as usize == k)
                .map(|a| a.y)
                .collect();
            assert_eq!(here, first, "half-wave {k} as the first");
        }
        // a 0 leaves its half-wave empty
        let gaps = pieces(300.0, false, 15.0, 30.0, 1.0, &with("10"));
        let halves: std::collections::BTreeSet<i64> = gaps
            .iter()
            .filter_map(|p| p.anchor)
            .map(|a| (a.x / 30.0).floor() as i64)
            .collect();
        assert!(
            halves.iter().all(|k| k % 2 == 0),
            "only every other: {halves:?}"
        );
        // turned ones stay in the band
        for p in pieces(300.0, false, 15.0, 30.0, 1.0, &with("1/4,R1/4")) {
            assert!(p.item.points().all(|q| q.y.abs() <= 15.0 + 1e-6));
        }
    }
}
