//! Centerline tracing: one line down the middle of each stroke of a picture (a signature, a
//! line drawing, a map's roads) rather than an outline all round it, for engraving and
//! V-carving. The stroke's pixels are thinned to one pixel wide, the thin lines followed
//! from end to end or to where they meet, and the short spurs thinning leaves at bends and
//! line ends taken off.

use std::collections::HashMap;

/// A line down the middle of a stroke: pixel centres (`x` across from the left, `y` down
/// from the top, in pixels), and whether it closes on itself (an O, a ring).
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub points: Vec<(f64, f64)>,
    pub closed: bool,
}

/// The middle lines of the strokes of a `w` × `h` picture whose pixels `on` says are ink
/// (by index, row by row from the top). Spurs shorter than the strokes are wide, and lines
/// shorter than `shortest` pixels, are left out.
#[must_use]
#[expect(clippy::cast_precision_loss, reason = "pixel counts")]
pub fn lines(w: usize, h: usize, on: impl Fn(usize) -> bool, shortest: f64) -> Vec<Line> {
    // a border of blank pixels round the picture, so no neighbour is off it
    let (gw, gh) = (w + 2, h + 2);
    let mut ink = vec![false; gw * gh];
    let mut inked = 0usize;
    for r in 0..h {
        for c in 0..w {
            if on(r * w + c) {
                ink[(r + 1) * gw + c + 1] = true;
                inked += 1;
            }
        }
    }
    thin(&mut ink, gw, gh);
    let thin_count = ink.iter().filter(|&&v| v).count();
    if thin_count == 0 {
        return vec![];
    }
    // how wide the strokes are, on average: ink pixels for each pixel of middle line
    let width = inked as f64 / thin_count as f64;
    let graph = Graph::of(&ink, gw);
    let mut edges = graph.edges();
    prune(&mut edges, width.max(2.0));
    join_through(&mut edges);
    edges
        .into_iter()
        .filter(|e| length(&e.pixels, gw) >= shortest.max(1.0))
        .map(|e| Line {
            closed: e.closed(),
            points: e
                .pixels
                .iter()
                .map(|&i| ((i % gw) as f64 - 0.5, (i / gw) as f64 - 0.5))
                .collect(),
        })
        .collect()
}

/// Zhang and Suen's thinning: ink pixels taken off the strokes' sides, in two half-steps
/// a pass, until each stroke is one pixel wide and still in one piece.
#[expect(clippy::many_single_char_names, reason = "the method's own names")]
fn thin(ink: &mut [bool], gw: usize, gh: usize) {
    let mut gone = vec![];
    loop {
        let mut changed = false;
        for step in 0..2 {
            gone.clear();
            for r in 1..gh - 1 {
                for c in 1..gw - 1 {
                    let i = r * gw + c;
                    if !ink[i] {
                        continue;
                    }
                    // the neighbours clockwise from above: P2 … P9
                    let p = [
                        ink[i - gw],
                        ink[i - gw + 1],
                        ink[i + 1],
                        ink[i + gw + 1],
                        ink[i + gw],
                        ink[i + gw - 1],
                        ink[i - 1],
                        ink[i - gw - 1],
                    ];
                    let b = p.iter().filter(|&&v| v).count();
                    let a = (0..8).filter(|&k| !p[k] && p[(k + 1) % 8]).count();
                    let (n, e, s, west) = (p[0], p[2], p[4], p[6]);
                    let keep = if step == 0 {
                        s && e && (n || west)
                    } else {
                        n && west && (e || s)
                    };
                    if (2..=6).contains(&b) && a == 1 && !keep {
                        gone.push(i);
                    }
                }
            }
            for &i in &gone {
                ink[i] = false;
            }
            changed |= !gone.is_empty();
        }
        if !changed {
            return;
        }
    }
}

/// The thinned pixels and which touch: side by side, or corner to corner where no pixel
/// beside both joins them already (so a stair step is one line, not a triangle).
struct Graph<'a> {
    ink: &'a [bool],
    gw: usize,
}

impl Graph<'_> {
    fn of(ink: &[bool], gw: usize) -> Graph<'_> {
        Graph { ink, gw }
    }

    #[expect(
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        reason = "inside the bordered grid"
    )]
    fn neighbours(&self, i: usize) -> Vec<usize> {
        let gw = self.gw as isize;
        let at = |dx: isize, dy: isize| (i as isize + dy * gw + dx) as usize;
        let mut out = vec![];
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            if self.ink[at(dx, dy)] {
                out.push(at(dx, dy));
            }
        }
        for (dx, dy) in [(1, 1), (1, -1), (-1, 1), (-1, -1)] {
            if self.ink[at(dx, dy)] && !self.ink[at(dx, 0)] && !self.ink[at(0, dy)] {
                out.push(at(dx, dy));
            }
        }
        out
    }

    /// The lines between ends and meeting points, and the rings that have neither.
    fn edges(&self) -> Vec<Edge> {
        let pixels: Vec<usize> = (0..self.ink.len()).filter(|&i| self.ink[i]).collect();
        let around: HashMap<usize, Vec<usize>> =
            pixels.iter().map(|&i| (i, self.neighbours(i))).collect();
        let node = |i: usize| around[&i].len() != 2;
        let mut walked: std::collections::HashSet<(usize, usize)> =
            std::collections::HashSet::default();
        let mut edges = vec![];
        let walk = |from: usize, first: usize, walked: &mut std::collections::HashSet<_>| {
            let mut line = vec![from, first];
            walked.insert((from, first));
            walked.insert((first, from));
            let (mut prev, mut at) = (from, first);
            while !node(at) && at != from {
                let Some(&next) = around[&at].iter().find(|&&n| n != prev) else {
                    break;
                };
                walked.insert((at, next));
                walked.insert((next, at));
                line.push(next);
                (prev, at) = (at, next);
            }
            line
        };
        for &i in pixels.iter().filter(|&&i| node(i)) {
            if around[&i].is_empty() {
                // a dot on its own
                edges.push(Edge { pixels: vec![i] });
                continue;
            }
            for &n in &around[&i] {
                if !walked.contains(&(i, n)) {
                    edges.push(Edge {
                        pixels: walk(i, n, &mut walked),
                    });
                }
            }
        }
        // rings: every pixel of them has two neighbours
        for &i in &pixels {
            if node(i) || walked.contains(&(i, around[&i][0])) {
                continue;
            }
            let ring = walk(i, around[&i][0], &mut walked);
            edges.push(Edge { pixels: ring });
        }
        edges
    }
}

/// A run of thinned pixels from one end or meeting point to the next (a ring starts and
/// ends on the same pixel).
#[derive(Clone, Debug)]
struct Edge {
    pixels: Vec<usize>,
}

impl Edge {
    fn ends(&self) -> (usize, usize) {
        (self.pixels[0], self.pixels[self.pixels.len() - 1])
    }

    fn closed(&self) -> bool {
        self.pixels.len() > 2 && self.ends().0 == self.ends().1
    }
}

/// How many lines meet at each end pixel.
fn degrees(edges: &[Edge]) -> HashMap<usize, usize> {
    let mut d = HashMap::new();
    for e in edges {
        let (a, b) = e.ends();
        *d.entry(a).or_insert(0) += 1;
        *d.entry(b).or_insert(0) += 1;
    }
    d
}

/// Takes off spurs: lines from a loose end to a meeting point shorter than `spur` pixels.
#[expect(clippy::cast_precision_loss, reason = "pixel counts")]
fn prune(edges: &mut Vec<Edge>, spur: f64) {
    let d = degrees(edges);
    edges.retain(|e| {
        let (a, b) = e.ends();
        let loose = d[&a] == 1 || d[&b] == 1;
        let meets = d[&a] >= 3 || d[&b] >= 3;
        !(loose && meets && e.pixels.len() as f64 <= spur)
    });
}

/// Lines that now meet only each other at a pixel (their spur taken off) made one.
fn join_through(edges: &mut Vec<Edge>) {
    loop {
        let d = degrees(edges);
        // a pixel two different lines end at (not a dot, nor a ring ending where it starts)
        let pair = d.iter().filter(|&(_, &n)| n == 2).find_map(|(&at, _)| {
            let both: Vec<usize> = (0..edges.len())
                .filter(|&k| {
                    let (a, b) = edges[k].ends();
                    a == at || b == at
                })
                .collect();
            match both[..] {
                [k1, k2] => Some((at, k1, k2)),
                _ => None,
            }
        });
        let Some((at, k1, k2)) = pair else { return };
        let (mut first, mut second) = (edges[k1].clone(), edges[k2].clone());
        if first.ends().1 != at {
            first.pixels.reverse();
        }
        if second.ends().0 != at {
            second.pixels.reverse();
        }
        first.pixels.extend_from_slice(&second.pixels[1..]);
        let (hi, lo) = (k1.max(k2), k1.min(k2));
        edges.swap_remove(hi);
        edges.swap_remove(lo);
        edges.push(first);
    }
}

/// A run of pixels' length, in pixels.
#[expect(clippy::cast_precision_loss, reason = "pixel positions")]
fn length(pixels: &[usize], gw: usize) -> f64 {
    pixels
        .windows(2)
        .map(|p| {
            let (a, b) = (p[0], p[1]);
            let dx = (a % gw) as f64 - (b % gw) as f64;
            let dy = (a / gw) as f64 - (b / gw) as f64;
            dx.hypot(dy)
        })
        .sum()
}

#[cfg(test)]
#[expect(clippy::cast_precision_loss, reason = "small test pictures")]
mod tests {
    use super::*;

    fn picture(w: usize, h: usize, ink: impl Fn(f64, f64) -> bool) -> Vec<bool> {
        (0..w * h)
            .map(|i| ink((i % w) as f64 + 0.5, (i / w) as f64 + 0.5))
            .collect()
    }

    #[test]
    fn a_thick_bar_becomes_one_line_down_its_middle() {
        // 60 x 20, a bar 5 pixels thick across the middle from x = 10 to 50
        let pic = picture(60, 20, |x, y| {
            (10.0..50.0).contains(&x) && (8.0..13.0).contains(&y)
        });
        let found = lines(60, 20, |i| pic[i], 2.0);
        assert_eq!(found.len(), 1, "{found:?}");
        let l = &found[0];
        assert!(!l.closed);
        assert!(
            l.points.iter().all(|&(_, y)| (y - 10.0).abs() <= 1.0),
            "{l:?}"
        );
        let xs: Vec<f64> = l.points.iter().map(|p| p.0).collect();
        let span = xs.iter().copied().fold(f64::MIN, f64::max)
            - xs.iter().copied().fold(f64::MAX, f64::min);
        assert!(span > 30.0, "nearly the bar's length: {span}");
    }

    #[test]
    fn a_ring_is_one_closed_line() {
        let pic = picture(40, 40, |x, y| {
            let r = (x - 20.0).hypot(y - 20.0);
            (10.0..14.0).contains(&r)
        });
        let found = lines(40, 40, |i| pic[i], 2.0);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].closed);
        let mean = found[0]
            .points
            .iter()
            .map(|&(x, y)| (x - 20.0).hypot(y - 20.0))
            .sum::<f64>()
            / found[0].points.len() as f64;
        assert!((mean - 12.0).abs() < 1.0, "down the ring's middle: {mean}");
    }

    #[test]
    fn a_cross_is_four_arms_meeting_with_no_spurs() {
        // a plus sign, arms 4 pixels thick
        let pic = picture(50, 50, |x, y| {
            ((23.0..27.0).contains(&x) && (5.0..45.0).contains(&y))
                || ((23.0..27.0).contains(&y) && (5.0..45.0).contains(&x))
        });
        let found = lines(50, 50, |i| pic[i], 2.0);
        assert_eq!(found.len(), 4, "{found:?}");
        assert!(found.iter().all(|l| l.points.len() > 10));
    }

    #[test]
    fn specks_shorter_than_asked_are_left_out() {
        let pic = picture(30, 30, |x, y| {
            (5.0..8.0).contains(&x) && (5.0..8.0).contains(&y)
        });
        assert_eq!(lines(30, 30, |i| pic[i], 4.0), vec![]);
        assert_eq!(lines(30, 30, |_| false, 1.0), vec![]);
    }
}
