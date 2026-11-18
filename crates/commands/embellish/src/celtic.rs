//! Celtic weave: strands plaited over and under one another, as in a carved Celtic band.
//! The strands run diagonally across the band on a lattice of crossings, bounce off its
//! edges in rounded loops, and pass over then under at each crossing in turn. Along an open
//! line the strands turn back at its ends (the band is one finished knot) or are cut off
//! square. Pieces are drawn along the line (`s`) and across it (`v`), then laid onto it.

use crate::motif::{Item, Profile, Track, band, normal, pt};
use cut_craft_core::geometry::Point;

/// What the strands do at an open line's ends.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Ends {
    /// Turn back into the band, so every strand is a closed loop.
    #[default]
    TurnBack,
    /// Stop, cut off square across the band.
    CutOff,
}

/// How the weave is made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Weave {
    /// How many strands are plaited together, 2 or more.
    pub strands: u32,
    /// How many grooves run along each strand, splitting it into cords.
    pub lines: u32,
    pub ends: Ends,
}

impl Default for Weave {
    fn default() -> Self {
        Self {
            strands: 4,
            lines: 2,
            ends: Ends::TurnBack,
        }
    }
}

/// A point of the crossing lattice, in half cells: `x` along, `y` across from 0 to the
/// strand count.
type At = (i32, i32);

/// How the lattice's ends are joined.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lattice {
    /// Round a closed line: along wraps round after `w`.
    Round,
    /// From 0 to `w`, the strands bouncing off both ends.
    TurnBack,
    /// Strands running on past both ends, to be cut off there.
    CutOff,
}

/// One strand: the lattice points it passes, and whether it closes on itself.
struct Strand {
    at: Vec<At>,
    closed: bool,
}

/// Whether odd strand counts can turn back: they cannot, as a strand would run into a
/// corner and double back on itself.
pub fn turns_back(strands: u32) -> bool {
    strands.is_multiple_of(2)
}

/// The strands on a lattice `w` half cells along (even) and `m` across.
fn strands(w: i32, m: i32, how: Lattice) -> Vec<Strand> {
    let zig = |d: i32| m - ((d.rem_euclid(2 * m)) - m).abs();
    match how {
        Lattice::CutOff => (0..m)
            .map(|k| Strand {
                // each strand from well before the start to well past the end
                at: (-2 * m - 2..=w + 2 * m + 2)
                    .map(|x| (x, zig(x - (2 * k + 1))))
                    .collect(),
                closed: false,
            })
            .collect(),
        Lattice::Round => {
            // each strand starts at a crossing just after one of its bottom bounces and goes
            // round until it is back there; it may go round more than once
            let mut seen = vec![false; w as usize];
            let mut out = vec![];
            for b in (1..w).step_by(2) {
                if seen[b as usize] {
                    continue;
                }
                let mut at = vec![(b + 1, 1)];
                let mut x = b + 1;
                loop {
                    x += 1;
                    let y = zig(x - b);
                    if y == 0 {
                        seen[x.rem_euclid(w) as usize] = true;
                    }
                    at.push((x, y));
                    if y == 1 && (x - b - 1).rem_euclid(w) == 0 && zig(x - 1 - b) == 0 {
                        break;
                    }
                }
                out.push(Strand { at, closed: false });
            }
            out
        }
        Lattice::TurnBack => {
            let on = |(x, y): At| (0..=w).contains(&x) && (0..=m).contains(&y);
            let mut used = std::collections::HashSet::new();
            let mut out = vec![];
            for x in 0..=w {
                for y in 0..=m {
                    if (x + y) % 2 == 0 {
                        continue;
                    }
                    for d in [(1, 1), (-1, 1), (1, -1), (-1, -1)] {
                        if used.contains(&((x, y), d)) || !on((x + d.0, y + d.1)) {
                            continue;
                        }
                        let (start, mut at, mut dir) = ((x, y), (x, y), d);
                        let mut ring = vec![];
                        loop {
                            used.insert((at, dir));
                            let next = (at.0 + dir.0, at.1 + dir.1);
                            used.insert((next, (-dir.0, -dir.1)));
                            ring.push(next);
                            if next.0 == 0 || next.0 == w {
                                dir.0 = -dir.0;
                            }
                            if next.1 == 0 || next.1 == m {
                                dir.1 = -dir.1;
                            }
                            at = next;
                            if (at, dir) == (start, d) {
                                break;
                            }
                        }
                        out.push(Strand {
                            at: ring,
                            closed: true,
                        });
                    }
                }
            }
            out
        }
    }
}

/// Whether the strand passing the crossing at `x` going `d` goes over there: the strands
/// rising along go over at even crossings and under at odd ones, so every strand goes over
/// and under in turn, and of the two at a crossing one is over. `flip` swaps them all.
fn over(x: i32, d: At, flip: bool) -> bool {
    ((d.0 * d.1 > 0) == (x.rem_euclid(2) == 0)) != flip
}

/// A smooth curve through the line `p` (a cubic B-spline, subdivided `times` over): straight
/// runs stay straight and each corner is rounded over a step either side, drawn in to a
/// third of the way to its neighbours. An open line keeps its ends. Each point keeps how
/// far along the line it is, counted in the points it started with (going on past the last
/// round a closed line).
fn smoothed(p: &[Point], closed: bool, times: usize) -> Vec<(Point, f64)> {
    let count = p.len() as f64;
    let mut p: Vec<(Point, f64)> = p.iter().enumerate().map(|(i, &q)| (q, i as f64)).collect();
    for _ in 0..times {
        let n = p.len();
        if n < 3 {
            break;
        }
        // the neighbour after `i` (round a closed line, a lap further on)
        let next = |i: usize| {
            let (q, t) = p[(i + 1) % n];
            (q, if i + 1 == n { t + count } else { t })
        };
        let prev = |i: usize| {
            let (q, t) = p[(i + n - 1) % n];
            (q, if i == 0 { t - count } else { t })
        };
        let mut q = Vec::with_capacity(2 * n);
        for i in 0..n {
            let (c, tc) = p[i];
            if !closed && (i == 0 || i + 1 == n) {
                q.push((c, tc));
            } else {
                let ((a, ta), (b, tb)) = (prev(i), next(i));
                q.push(((a + c * 6.0 + b) / 8.0, (ta + tc * 6.0 + tb) / 8.0));
            }
            if closed || i + 1 < n {
                let (b, tb) = next(i);
                q.push(((c + b) / 2.0, (tc + tb) / 2.0));
            }
        }
        p = q;
    }
    p
}

/// The headings along a line, from its neighbours.
fn headings(p: &[Point]) -> Vec<f64> {
    let n = p.len();
    (0..n)
        .map(|i| {
            let d = p[(i + 1).min(n - 1)] - p[i.saturating_sub(1)];
            d.y.atan2(d.x)
        })
        .collect()
}

/// The sizes of the weave in a band `hw` either side of its middle, the crossings `r` times
/// as far apart along as across, with gap `g`: the lattice's half cell across and along,
/// and the strands' width.
fn sizes(m: f64, hw: f64, r: f64, g: f64) -> (f64, f64, f64) {
    // parallel strands sit two half cells apart along; their width is most of that,
    // measured square to them
    let square = 2.0 * r / r.hypot(1.0);
    let share = 0.86;
    // the loops at the edges are drawn in a third of a half cell (see [`smoothed`])
    let uy = 2.0 * hw / (m - 0.6 + share * square);
    let spacing = square * uy;
    let ws = (share * spacing).min(spacing - g);
    (uy, uy * r, ws)
}

/// The weave's pieces along a line `len` long (round a closed one when `closed`), in a band
/// `hw` either side of its middle, its crossings `r` times as far apart along as across,
/// with gap `g` round the strands. `flip` swaps over and under.
pub fn weave(len: f64, closed: bool, hw: f64, r: f64, g: f64, w: Weave, flip: bool) -> Vec<Item> {
    let m = w.strands.max(2) as i32;
    let (uy, ux0, ws) = sizes(f64::from(m), hw, r, g);
    if ws < g.max(0.05) || len <= ws {
        return vec![];
    }
    let how = if closed {
        Lattice::Round
    } else if w.ends == Ends::TurnBack && turns_back(w.strands) {
        Lattice::TurnBack
    } else {
        Lattice::CutOff
    };
    // the room the lattice has along, and where it starts: an open line's ends keep the
    // strands' half width clear (where they loop back, the loops are drawn in a third of a
    // half cell)
    let inset = |ux: f64| match how {
        Lattice::TurnBack => ws / 2.0 - ux * 0.3,
        _ => ws / 2.0,
    };
    let (mut room, mut s0, mut ux) = (len, 0.0, ux0);
    let mut wl = 2;
    for _ in 0..3 {
        if !closed {
            s0 = inset(ux);
            room = len - 2.0 * s0;
        }
        wl = 2 * ((room / ux0 / 2.0).round() as i32).max(1);
        ux = room / f64::from(wl);
    }
    if !closed {
        s0 = inset(ux);
        room = len - 2.0 * s0;
        ux = room / f64::from(wl);
    }
    let place = |(x, y): At| {
        pt(
            s0 + f64::from(x) * ux,
            (f64::from(y) - f64::from(m) / 2.0) * uy,
        )
    };
    // where a strand under another is cut back, either side of the crossing: clear of the
    // one over by the gap
    let sin = 2.0 * ux * uy / (ux * ux + uy * uy);
    let cover = (ws / 2.0 + g) / sin.max(0.2);
    let (lo, hi) = (ws / 2.0, len - ws / 2.0);
    // a strand passing under falls from its full height where it goes over to this share
    // of it where it is cut back
    let step = ux.hypot(uy);
    let low = 0.25;
    let mut shapes = vec![];
    let mut grooves = vec![];
    for strand in strands(wl, m, how) {
        let n = strand.at.len();
        let interior =
            |(x, y): At| (1..m).contains(&y) && (how != Lattice::TurnBack || (1..wl).contains(&x));
        // the crossings where it goes under
        // (the way it goes through a crossing: from the one before, or to the one after at
        // an open strand's start)
        let heading = |i: usize| {
            let (a, b) = if strand.closed || i > 0 {
                (strand.at[(i + n - 1) % n], strand.at[i])
            } else {
                (strand.at[0], strand.at[1])
            };
            (b.0 - a.0, b.1 - a.1)
        };
        let under: Vec<f64> = (0..n)
            .filter(|&i| interior(strand.at[i]))
            .filter(|&i| !over(strand.at[i].0, heading(i), flip))
            .map(|i| i as f64)
            .collect();
        // how far along the strand it is cut back either side of each of them, in its
        // own steps (a strand can cross itself: only where it passes under is cut)
        let back = cover / step;
        let nf = n as f64;
        let line: Vec<Point> = strand.at.iter().map(|&a| place(a)).collect();
        let smooth = smoothed(&line, strand.closed, 4);
        let line: Vec<Point> = smooth.iter().map(|q| q.0).collect();
        // each point: cut back where it passes under, cut off past an open line's ends
        let gap: Vec<bool> = smooth
            .iter()
            .map(|&(_, t)| {
                under.iter().any(|&u| {
                    let d = if strand.closed {
                        let d = (t - u).rem_euclid(nf);
                        d.min(nf - d)
                    } else {
                        (t - u).abs()
                    };
                    d <= back
                })
            })
            .collect();
        let keep: Vec<bool> = smooth
            .iter()
            .zip(&gap)
            .map(|(&(q, _), &gap)| !gap && (how != Lattice::CutOff || (lo..=hi).contains(&q.x)))
            .collect();
        // runs start at a gap, so none wraps round the end
        let k = line.len();
        let start = if strand.closed {
            keep.iter().position(|&x| !x).unwrap_or(0)
        } else {
            0
        };
        let mut run: Vec<Point> = vec![];
        // whether the run began where it came out from under another strand
        let mut from_under = false;
        let mut flush = |run: &mut Vec<Point>, from_under: bool, to_under: bool| {
            if run.len() > 3 {
                let t = Track {
                    h: headings(run),
                    p: std::mem::take(run),
                };
                shapes.push(Item::Shape {
                    rings: vec![band(&t, &vec![ws; t.p.len()], false, false)],
                    h: 1.0,
                    profile: Profile::Strand {
                        dips: [from_under, to_under],
                        len: (step - cover - ws / 2.0).max(ws / 3.0),
                        low,
                    },
                    sink: false,
                });
                for j in 1..=w.lines {
                    let off = ws * (f64::from(j) / f64::from(w.lines + 1) - 0.5);
                    grooves.push(Item::Line {
                        points: t
                            .p
                            .iter()
                            .zip(&t.h)
                            .map(|(&q, &h)| q + normal(h) * off)
                            .collect(),
                        width: (ws * 0.09).max(0.2),
                    });
                }
            }
            run.clear();
        };
        for c in 0..k {
            let i = (start + c) % k;
            if keep[i] {
                if run.is_empty() {
                    from_under =
                        c > 0 && gap[(i + k - 1) % k] || strand.closed && gap[(i + k - 1) % k];
                }
                run.push(line[i]);
            } else {
                flush(&mut run, from_under, gap[i]);
            }
        }
        if strand.closed && keep.iter().all(|&x| x) {
            run.push(line[start]);
        }
        flush(&mut run, from_under, strand.closed && gap[start]);
    }
    shapes.extend(grooves);
    shapes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn crossings(s: &[Strand], w: i32, m: i32, how: Lattice) -> Vec<(At, bool)> {
        let mut out = vec![];
        for st in s {
            let n = st.at.len();
            for i in 0..n {
                // a strand round a closed line ends where it started
                let last = how != Lattice::Round && i + 1 == n;
                if !st.closed && (i == 0 || last) {
                    continue;
                }
                let (a, b) = (st.at[(i + n - 1) % n], st.at[i]);
                let inside = (1..m).contains(&b.1)
                    && match how {
                        Lattice::TurnBack => (1..w).contains(&b.0),
                        Lattice::CutOff => (0..=w).contains(&b.0),
                        Lattice::Round => true,
                    };
                if inside {
                    let x = if how == Lattice::Round {
                        b.0.rem_euclid(w)
                    } else {
                        b.0
                    };
                    out.push(((x, b.1), over(b.0, (b.0 - a.0, b.1 - a.1), false)));
                }
            }
        }
        out
    }

    #[test]
    fn at_every_crossing_one_strand_goes_over_and_one_under() {
        for (how, m) in [
            (Lattice::Round, 3),
            (Lattice::Round, 4),
            (Lattice::TurnBack, 4),
            (Lattice::TurnBack, 6),
            (Lattice::CutOff, 3),
            (Lattice::CutOff, 5),
        ] {
            let w = 12;
            let all = crossings(&strands(w, m, how), w, m, how);
            let mut by: std::collections::HashMap<At, Vec<bool>> = Default::default();
            for (at, o) in all {
                by.entry(at).or_default().push(o);
            }
            assert!(!by.is_empty());
            for (at, v) in by {
                assert_eq!(v.len(), 2, "{how:?} {m}: {at:?} passed {v:?}");
                assert!(v[0] != v[1], "{how:?} {m}: {at:?} {v:?}");
            }
        }
    }

    #[test]
    fn each_strand_goes_over_then_under_in_turn() {
        for (how, m) in [
            (Lattice::Round, 4),
            (Lattice::TurnBack, 4),
            (Lattice::CutOff, 3),
        ] {
            let w = 10;
            for st in strands(w, m, how) {
                let n = st.at.len();
                let mut seq = vec![];
                for i in 0..n {
                    if !st.closed && (i == 0 || i + 1 == n) {
                        continue;
                    }
                    let (a, b) = (st.at[(i + n - 1) % n], st.at[i]);
                    let inside = (1..m).contains(&b.1)
                        && (how != Lattice::TurnBack || (1..w).contains(&b.0));
                    if inside {
                        seq.push(over(b.0, (b.0 - a.0, b.1 - a.1), false));
                    }
                }
                for p in seq.windows(2) {
                    assert!(p[0] != p[1], "{how:?}: {seq:?}");
                }
            }
        }
    }

    #[test]
    fn a_round_weave_has_as_many_strands_as_asked_or_fewer_longer_ones() {
        // round a closed line strands may join up into fewer, longer ones, but every
        // crossing is still passed twice
        let w = 12;
        for m in 2..=6 {
            let s = strands(w, m, Lattice::Round);
            assert!(!s.is_empty() && s.len() <= m as usize, "{m}: {}", s.len());
            let passed: usize = s.iter().map(|st| st.at.len() - 1).sum();
            assert_eq!(passed as i32, w * m, "{m}");
        }
    }

    #[test]
    fn turning_back_makes_closed_loops_and_cutting_off_open_strands() {
        assert!(strands(8, 4, Lattice::TurnBack).iter().all(|s| s.closed));
        let cut = strands(8, 3, Lattice::CutOff);
        assert_eq!(cut.len(), 3);
        assert!(cut.iter().all(|s| !s.closed));
        assert!(turns_back(4) && !turns_back(3));
    }

    #[test]
    fn the_pieces_keep_inside_the_band() {
        let hw = 15.0;
        for (closed, ends, strands) in [
            (true, Ends::TurnBack, 4),
            (false, Ends::TurnBack, 4),
            (false, Ends::CutOff, 5),
        ] {
            let items = weave(
                200.0,
                closed,
                hw,
                1.0,
                0.8,
                Weave {
                    strands,
                    lines: 2,
                    ends,
                },
                false,
            );
            let shapes = items
                .iter()
                .filter(|i| matches!(i, Item::Shape { .. }))
                .count();
            assert!(shapes > 10, "{closed} {ends:?}: {shapes}");
            for q in items.iter().flat_map(Item::points) {
                assert!(q.y.abs() <= hw + 1e-6, "{closed} {ends:?}: {q:?}");
                if !closed {
                    assert!((-0.5..=200.5).contains(&q.x), "{ends:?}: {q:?}");
                }
            }
        }
    }

    #[test]
    fn each_strand_has_its_grooves() {
        let items = |lines| {
            weave(
                100.0,
                false,
                10.0,
                1.0,
                0.5,
                Weave {
                    lines,
                    ..Weave::default()
                },
                false,
            )
        };
        let count = |v: &[Item]| v.iter().filter(|i| matches!(i, Item::Line { .. })).count();
        let shapes = |v: &[Item]| v.iter().filter(|i| matches!(i, Item::Shape { .. })).count();
        assert_eq!(count(&items(0)), 0);
        let two = items(2);
        assert_eq!(count(&two), 2 * shapes(&two));
    }
}
