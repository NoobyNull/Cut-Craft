//! The model as triangles in the part's frame, and the bit dropped onto it.

use crate::cutter::Cutter;
use crate::frame::to_machine;
use cut_craft_core::glam::{DVec2, DVec3};

/// A model: triangles in the part's frame, with a tree of boxes round them so the bit only
/// looks at the few near it.
#[derive(Clone, Debug, Default)]
pub struct Mesh {
    triangles: Vec<[DVec3; 3]>,
    nodes: Vec<Node>,
}

#[derive(Clone, Copy, Debug)]
struct Node {
    lo: DVec3,
    hi: DVec3,
    /// A leaf's triangles (`first..first + count`), or an inner node's children (`first`,
    /// `first + 1` is not assumed: `count` is the right child) when `leaf` is false.
    first: usize,
    count: usize,
    leaf: bool,
}

/// Triangles in a leaf of the tree.
const LEAF: usize = 8;

impl Mesh {
    /// `triangles` in the part's frame.
    #[must_use]
    pub fn new(mut triangles: Vec<[DVec3; 3]>) -> Self {
        triangles.retain(|t| t.iter().all(|p| p.is_finite()));
        let mut nodes = Vec::with_capacity(2 * triangles.len() / LEAF + 1);
        if !triangles.is_empty() {
            let n = triangles.len();
            build(&mut triangles, 0, n, &mut nodes);
        }
        Self { triangles, nodes }
    }

    #[must_use]
    pub fn triangles(&self) -> &[[DVec3; 3]] {
        &self.triangles
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.triangles.is_empty()
    }

    /// The corners of a box round the whole model, in the part's frame.
    #[must_use]
    pub fn bounds(&self) -> Option<(DVec3, DVec3)> {
        self.nodes.first().map(|n| (n.lo, n.hi))
    }

    /// How far out from the axis the model reaches.
    #[must_use]
    pub fn swing(&self) -> f64 {
        self.triangles
            .iter()
            .flatten()
            .map(|p| p.y.hypot(p.z))
            .fold(0.0, f64::max)
    }

    /// The highest the tip of `cutter` can sit, straight down from above, held at `along`
    /// and `across` with A turned `turn`, without going into the model: its height above
    /// the axis. `None` when nothing of the model is under the bit.
    #[must_use]
    pub fn drop(&self, cutter: &Cutter, along: f64, across: f64, turn: f64) -> Option<f64> {
        let reach = cutter.reach();
        let centre = DVec2::new(along, across);
        // the machine's across and height directions, in the part's frame
        let (s, c) = turn.sin_cos();
        let across_dir = DVec2::new(c, -s);
        let up_dir = DVec2::new(s, c);
        let mut best = f64::NEG_INFINITY;
        let mut stack = vec![0usize];
        while let Some(i) = stack.pop() {
            let Some(node) = self.nodes.get(i) else {
                continue;
            };
            if node.lo.x > along + reach || node.hi.x < along - reach {
                continue;
            }
            let (mid, half) = ((node.lo + node.hi) / 2.0, (node.hi - node.lo) / 2.0);
            let mid = DVec2::new(mid.y, mid.z);
            let spread = |d: DVec2| half.y * d.x.abs() + half.z * d.y.abs();
            let side = mid.dot(across_dir);
            if side - spread(across_dir) > across + reach
                || side + spread(across_dir) < across - reach
            {
                continue;
            }
            // the tip never sits higher than the highest point under it
            if mid.dot(up_dir) + spread(up_dir) <= best {
                continue;
            }
            if node.leaf {
                for t in &self.triangles[node.first..node.first + node.count] {
                    let seen = t.map(|p| to_machine(p, turn));
                    if let Some(tip) = triangle_drop(cutter, centre, &seen, best) {
                        best = best.max(tip);
                    }
                }
            } else {
                // the higher child first, so more of the lower is passed over
                let top = |i: usize| {
                    let n = &self.nodes[i];
                    let (mid, half) = ((n.lo + n.hi) / 2.0, (n.hi - n.lo) / 2.0);
                    DVec2::new(mid.y, mid.z).dot(up_dir)
                        + half.y * up_dir.x.abs()
                        + half.z * up_dir.y.abs()
                };
                let (a, b) = (node.first, node.count);
                if top(a) > top(b) {
                    stack.push(b);
                    stack.push(a);
                } else {
                    stack.push(a);
                    stack.push(b);
                }
            }
        }
        best.is_finite().then_some(best)
    }

    /// [`Self::drop`] looking at every triangle: what the tree must agree with.
    #[must_use]
    pub fn drop_every(&self, cutter: &Cutter, along: f64, across: f64, turn: f64) -> Option<f64> {
        let centre = DVec2::new(along, across);
        self.triangles
            .iter()
            .filter_map(|t| {
                triangle_drop(
                    cutter,
                    centre,
                    &t.map(|p| to_machine(p, turn)),
                    f64::NEG_INFINITY,
                )
            })
            .reduce(f64::max)
    }
}

fn build(triangles: &mut [[DVec3; 3]], first: usize, count: usize, nodes: &mut Vec<Node>) -> usize {
    let slice = &mut triangles[first..first + count];
    let (lo, hi) = slice.iter().flatten().fold(
        (DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY)),
        |(lo, hi), p| (lo.min(*p), hi.max(*p)),
    );
    let me = nodes.len();
    nodes.push(Node {
        lo,
        hi,
        first,
        count,
        leaf: true,
    });
    if count <= LEAF {
        return me;
    }
    let size = hi - lo;
    let axis = if size.x >= size.y && size.x >= size.z {
        0
    } else if size.y >= size.z {
        1
    } else {
        2
    };
    let centre = |t: &[DVec3; 3]| (t[0][axis] + t[1][axis] + t[2][axis]) / 3.0;
    let half = count / 2;
    slice.select_nth_unstable_by(half, |a, b| centre(a).total_cmp(&centre(b)));
    let left = build(triangles, first, half, nodes);
    let right = build(triangles, first + half, count - half, nodes);
    nodes[me] = Node {
        lo,
        hi,
        first: left,
        count: right,
        leaf: false,
    };
    me
}

/// The highest `f` reaches on `lo..=hi`, and where, for `f` rising then falling (or only one
/// of them): narrowing in on it by thirds.
fn highest(lo: f64, hi: f64, height: impl Fn(f64) -> f64) -> (f64, f64) {
    // the golden section: one new look each step, the span shrinking by 0.618
    const G: f64 = 0.618_033_988_749_894_9;
    let (mut low, mut high) = (lo, hi);
    let mut left = (high - G * (high - low), 0.0);
    let mut right = (low + G * (high - low), 0.0);
    left.1 = height(left.0);
    right.1 = height(right.0);
    for _ in 0..90 {
        if high - low < 1e-10 * (1.0 + hi - lo) {
            break;
        }
        if left.1 < right.1 {
            low = left.0;
            left = right;
            right.0 = low + G * (high - low);
            right.1 = height(right.0);
        } else {
            high = right.0;
            right = left;
            left.0 = high - G * (high - low);
            left.1 = height(left.0);
        }
    }
    // the ends too: the highest may be at one of them
    [(lo, height(lo)), (hi, height(hi)), left, right]
        .into_iter()
        .fold(
            (lo, f64::NEG_INFINITY),
            |best, x| if x.1 > best.1 { x } else { best },
        )
}

/// The highest `f` reaches on `lo..=hi` for any `f`: sampled finely, then narrowed in round
/// the best sample. For bits whose edge is not convex.
#[expect(clippy::cast_precision_loss, reason = "a fixed small count")]
fn highest_anyhow(lo: f64, hi: f64, f: impl Fn(f64) -> f64) -> (f64, f64) {
    const SAMPLES: usize = 256;
    let step = (hi - lo) / SAMPLES as f64;
    let (mut at, mut best) = (lo, f(lo));
    for k in 1..=SAMPLES {
        let t = lo + k as f64 * step;
        let v = f(t);
        if v > best {
            (at, best) = (t, v);
        }
    }
    let (t, v) = highest((at - step).max(lo), (at + step).min(hi), &f);
    if v > best { (t, v) } else { (at, best) }
}

/// The highest the tip of `cutter`, its axis straight up through `centre` (along, across),
/// can sit without going into triangle `t` (in the machine's frame: along, across,
/// height). `None` when the triangle is not under the bit, or cannot hold it higher than
/// `floor`.
pub(crate) fn triangle_drop(
    cutter: &Cutter,
    centre: DVec2,
    tri: &[DVec3; 3],
    floor: f64,
) -> Option<f64> {
    let reach = cutter.reach();
    // the tip never sits above the highest point under it
    if tri.iter().map(|p| p.z).fold(f64::NEG_INFINITY, f64::max) <= floor {
        return None;
    }
    let (lo, hi) = (
        tri[0]
            .truncate()
            .min(tri[1].truncate())
            .min(tri[2].truncate()),
        tri[0]
            .truncate()
            .max(tri[1].truncate())
            .max(tri[2].truncate()),
    );
    let near = centre.clamp(lo, hi);
    if near.distance_squared(centre) > reach * reach {
        return None;
    }
    let flat = |p: DVec3| DVec2::new(p.x, p.y);
    let search = |lo: f64, hi: f64, f: &dyn Fn(f64) -> f64| {
        if cutter.is_convex() {
            highest(lo, hi, f)
        } else {
            highest_anyhow(lo, hi, f)
        }
    };
    let under = |r: f64| cutter.edge(r.min(reach));
    let mut best = f64::NEG_INFINITY;
    // corners
    for p in tri {
        if let Some(edge) =
            under(flat(*p).distance(centre)).filter(|_| flat(*p).distance(centre) <= reach)
        {
            best = best.max(p.z - edge);
        }
    }
    // edges: where each is within reach of the axis
    for k in 0..3 {
        let (start, end) = (tri[k], tri[(k + 1) % 3]);
        if start.z.max(end.z) <= best.max(floor) {
            continue;
        }
        let (from, dir) = (flat(start) - centre, flat(end) - flat(start));
        let dd = dir.length_squared();
        if dd < 1e-24 {
            continue;
        }
        // |from + s dir|² = reach²
        let half_b = from.dot(dir);
        let disc = half_b * half_b - dd * (from.length_squared() - reach * reach);
        if disc < 0.0 {
            continue;
        }
        let root = disc.sqrt();
        let (s0, s1) = (
            ((-half_b - root) / dd).max(0.0),
            ((-half_b + root) / dd).min(1.0),
        );
        if s0 > s1 {
            continue;
        }
        let along = |s: f64| {
            let p = start.lerp(end, s);
            let r = flat(p).distance(centre);
            under(r).map_or(f64::NEG_INFINITY, |edge| p.z - edge)
        };
        let (_, v) = search(s0, s1, &along);
        best = best.max(v);
    }
    // the face: the bit touches it where the face rises as steeply as the bit's edge,
    // straight uphill from the axis
    let normal = (tri[1] - tri[0]).cross(tri[2] - tri[0]);
    if normal.z.abs() > 1e-12 * normal.length().max(1e-300) {
        let slope = DVec2::new(-normal.x / normal.z, -normal.y / normal.z);
        let plane = |q: DVec2| tri[0].z + slope.dot(q - flat(tri[0]));
        let uphill = slope.try_normalize().unwrap_or(DVec2::X);
        let ray = |r: f64| plane(centre + uphill * r) - under(r).unwrap_or(f64::INFINITY);
        let (r, _) = if slope.length_squared() < 1e-24 {
            (0.0, ray(0.0))
        } else {
            search(0.0, reach, &ray)
        };
        let touch = centre + uphill * r;
        if inside(touch, flat(tri[0]), flat(tri[1]), flat(tri[2])) {
            best = best.max(ray(r));
        }
    }
    best.is_finite().then_some(best)
}

/// Whether `q` is inside (or on the edge of) the triangle `a`, `b`, `c`.
fn inside(q: DVec2, a: DVec2, b: DVec2, c: DVec2) -> bool {
    let side = |p: DVec2, r: DVec2| (r - p).perp_dot(q - p);
    let (d1, d2, d3) = (side(a, b), side(b, c), side(c, a));
    let scale = (b - a).perp_dot(c - a).abs().max(1e-300);
    let eps = 1e-12 * scale;
    let neg = d1 < -eps || d2 < -eps || d3 < -eps;
    let pos = d1 > eps || d2 > eps || d3 > eps;
    !(neg && pos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::toolpath::{Tool, ToolShape};

    fn ball(diameter: f64) -> Cutter {
        Cutter::new(&Tool {
            shape: ToolShape::BallNose,
            diameter,
            ..Tool::default()
        })
    }

    #[test]
    fn a_ball_rests_on_a_flat_face_a_corner_and_an_edge() {
        let cutter = ball(6.0);
        // a level face at height 5 under the axis: the ball's tip sits on it
        let face = [
            DVec3::new(-10.0, -10.0, 5.0),
            DVec3::new(10.0, -10.0, 5.0),
            DVec3::new(0.0, 10.0, 5.0),
        ];
        let tip = triangle_drop(&cutter, DVec2::ZERO, &face, f64::NEG_INFINITY).unwrap();
        assert!((tip - 5.0).abs() < 1e-9, "{tip}");
        // a corner 2 out from the axis: the ball rests on it, sqrt(9-4) up its side
        let corner = [
            DVec3::new(2.0, 0.0, 5.0),
            DVec3::new(20.0, 0.0, -50.0),
            DVec3::new(20.0, 1.0, -50.0),
        ];
        let tip = triangle_drop(&cutter, DVec2::ZERO, &corner, f64::NEG_INFINITY).unwrap();
        assert!((tip - (5.0 - (3.0 - 5f64.sqrt()))).abs() < 1e-6, "{tip}");
        // a level edge 1 out passing beside the axis
        let edge = [
            DVec3::new(-20.0, 1.0, 4.0),
            DVec3::new(20.0, 1.0, 4.0),
            DVec3::new(0.0, 30.0, -60.0),
        ];
        let tip = triangle_drop(&cutter, DVec2::ZERO, &edge, f64::NEG_INFINITY).unwrap();
        assert!((tip - (4.0 - (3.0 - 8f64.sqrt()))).abs() < 1e-6, "{tip}");
    }

    #[test]
    fn nothing_out_of_reach_is_touched() {
        let cutter = ball(6.0);
        let far = [
            DVec3::new(10.0, 10.0, 5.0),
            DVec3::new(12.0, 10.0, 5.0),
            DVec3::new(10.0, 12.0, 5.0),
        ];
        assert_eq!(
            triangle_drop(&cutter, DVec2::ZERO, &far, f64::NEG_INFINITY),
            None
        );
    }
}
