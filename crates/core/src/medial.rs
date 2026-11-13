//! The centre lines of shapes (their medial axis): every point inside a shape that is as far
//! from two or more of its edges, with how far that is. A V-bit whose tip follows them, as
//! deep as that distance takes its cone, touches both edges at once: V-carving's crease.
//!
//! Worked out from the Voronoi diagram of points close together along the edges: its edges
//! inside the shape that lie between points far apart (seen from the edge, at a wide angle)
//! are the centre line; the rest are short spurs to points on the same straight or gently
//! curved edge, which are dropped.

use crate::geometry::{Path, Point};
use crate::toolpath::cut_points;
use spade::handles::{FixedFaceHandle, FixedVertexHandle, InnerTag};
use spade::{
    ConstrainedDelaunayTriangulation, DelaunayTriangulation, Point2, PositionInTriangulation,
    Triangulation,
};

/// A stretch of centre line: its points in order, each with the distance from it to the
/// nearest edge (never more than the true distance, so a cone sized by it never cuts past
/// an edge).
#[derive(Clone, Debug, PartialEq)]
pub struct CentreLine {
    pub points: Vec<Point>,
    pub radii: Vec<f64>,
}

/// Corners sharper than 180° less this keep the stretch of centre line into them; flatter
/// ones (the points of a curve drawn as short lines) do not.
const CORNER: f64 = 15.0;

/// The centre lines of `regions` (closed paths, holes inside outlines: [`crate::polygon::regions`]),
/// the edges looked at every `spacing` or closer: the centre lines are within about that of
/// the true ones, and run to within about that of each sharp corner.
pub fn centre_lines(regions: &[Path], spacing: f64) -> Vec<CentreLine> {
    let mut samples: Vec<Point2<f64>> = vec![];
    let mut edges: Vec<[usize; 2]> = vec![];
    for region in regions.iter().filter(|r| r.closed) {
        let ring = cut_points(region);
        if ring.len() < 3 {
            continue;
        }
        let first = samples.len();
        for (i, &a) in ring.iter().enumerate() {
            let b = ring[(i + 1) % ring.len()];
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a small positive count"
            )]
            let pieces = (a.distance(b) / spacing).ceil().max(1.0) as usize;
            for k in 0..pieces {
                #[expect(clippy::cast_precision_loss, reason = "a small count")]
                let q = a.lerp(b, k as f64 / pieces as f64);
                samples.push(Point2::new(q.x, q.y));
            }
        }
        let last = samples.len();
        edges.extend((first..last).map(|i| [i, if i + 1 == last { first } else { i + 1 }]));
    }
    if samples.len() < 3 {
        return vec![];
    }
    // both keep the points in order (less any doubled), so a vertex of one is the same
    // point in the other: a good place to start looking from
    let Ok(delaunay) = DelaunayTriangulation::<Point2<f64>>::bulk_load_stable(samples.clone())
    else {
        return vec![];
    };
    let Ok(outlines) =
        ConstrainedDelaunayTriangulation::<Point2<f64>>::try_bulk_load_cdt(samples, edges, |_| {})
    else {
        return vec![];
    };
    let inside = Inside::new(&outlines);

    // each triangle's circumcentre (a Voronoi corner), when it is inside the shapes
    let mut corners: Vec<Option<Point>> = vec![None; delaunay.num_all_faces()];
    for face in delaunay.inner_faces() {
        let c = face.circumcenter();
        let c = Point::new(c.x, c.y);
        let mut hint = Some(face.vertices()[0].fix());
        if c.is_finite() && inside.contains(c, &mut hint) {
            corners[face.fix().index()] = Some(c);
        }
    }
    let wide = 2.0 * (CORNER.to_radians() / 2.0).sin();
    // the kept Voronoi edges: between two corners, with one of the two points they lie
    // between (all of the edge is as far from both, and nearer no other)
    let mut kept: Vec<(usize, usize, Point)> = vec![];
    for edge in delaunay.undirected_edges() {
        let directed = edge.as_directed();
        let (Some(left), Some(right)) =
            (directed.face().as_inner(), directed.rev().face().as_inner())
        else {
            continue;
        };
        let (Some(from), Some(to)) = (corners[left.index()], corners[right.index()]) else {
            continue;
        };
        let [site, other] = directed.positions().map(|q| Point::new(q.x, q.y));
        // seen from the centre line, the two points must be far enough apart
        let radius = from.distance(site).max(to.distance(site));
        if site.distance(other) < wide * radius {
            continue;
        }
        let mut hint = Some(directed.from().fix());
        if !inside.contains(from.midpoint(to), &mut hint) {
            continue;
        }
        kept.push((left.index(), right.index(), site));
    }
    chains(&kept, &corners, spacing)
}

/// Triangles covering `regions` (closed paths, holes inside outlines, as
/// [`crate::polygon::regions`] gives them), for drawing them filled.
pub fn triangles(regions: &[Path]) -> Vec<[Point; 3]> {
    let mut points: Vec<Point2<f64>> = vec![];
    let mut edges: Vec<[usize; 2]> = vec![];
    for region in regions.iter().filter(|r| r.closed) {
        let ring = cut_points(region);
        if ring.len() < 3 {
            continue;
        }
        let first = points.len();
        points.extend(ring.iter().map(|q| Point2::new(q.x, q.y)));
        let last = points.len();
        edges.extend((first..last).map(|i| [i, if i + 1 == last { first } else { i + 1 }]));
    }
    if points.len() < 3 {
        return vec![];
    }
    let Ok(outlines) =
        ConstrainedDelaunayTriangulation::<Point2<f64>>::try_bulk_load_cdt(points, edges, |_| {})
    else {
        return vec![];
    };
    let inside = Inside::new(&outlines);
    outlines
        .inner_faces()
        .filter(|f| inside.within[f.fix().index()])
        .map(|f| f.positions().map(|q| Point::new(q.x, q.y)))
        .collect()
}

/// Whether points are inside the shapes: the triangles of the edges' constrained
/// triangulation, inside or outside by how many edges lie between them and the outside.
struct Inside<'a> {
    triangles: &'a ConstrainedDelaunayTriangulation<Point2<f64>>,
    within: Vec<bool>,
}

impl<'a> Inside<'a> {
    fn new(triangles: &'a ConstrainedDelaunayTriangulation<Point2<f64>>) -> Self {
        let mut within = vec![false; triangles.num_all_faces()];
        let mut seen = vec![false; triangles.num_all_faces()];
        let mut next: Vec<(FixedFaceHandle<InnerTag>, bool)> = triangles
            .convex_hull()
            .filter_map(|e| {
                let face = e.rev().face().as_inner()?;
                Some((face.fix(), e.is_constraint_edge()))
            })
            .collect();
        while let Some((face, is_in)) = next.pop() {
            if seen[face.index()] {
                continue;
            }
            seen[face.index()] = true;
            within[face.index()] = is_in;
            for e in triangles.face(face).adjacent_edges() {
                if let Some(other) = e.rev().face().as_inner()
                    && !seen[other.fix().index()]
                {
                    next.push((other.fix(), is_in != e.is_constraint_edge()));
                }
            }
        }
        Self { triangles, within }
    }

    /// Whether `p` is inside (on an edge is not), starting the search from `hint`.
    fn contains(&self, p: Point, hint: &mut Option<FixedVertexHandle>) -> bool {
        let at = Point2::new(p.x, p.y);
        let found = match *hint {
            Some(h) => self.triangles.locate_with_hint(at, h),
            None => self.triangles.locate(at),
        };
        match found {
            PositionInTriangulation::OnFace(face) => {
                *hint = Some(self.triangles.face(face).vertices()[0].fix());
                self.within[face.index()]
            }
            PositionInTriangulation::OnEdge(edge) => {
                let edge = self.triangles.directed_edge(edge);
                *hint = Some(edge.from().fix());
                !edge.is_constraint_edge()
                    && edge
                        .face()
                        .as_inner()
                        .is_some_and(|f| self.within[f.fix().index()])
            }
            _ => false,
        }
    }
}

/// The kept Voronoi edges joined end to end into centre lines, each edge split into pieces
/// no longer than `spacing` with the distance at each point.
fn chains(
    kept: &[(usize, usize, Point)],
    corners: &[Option<Point>],
    spacing: f64,
) -> Vec<CentreLine> {
    let mut at: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
    for (i, &(f, g, _)) in kept.iter().enumerate() {
        at.entry(f).or_default().push(i);
        at.entry(g).or_default().push(i);
    }
    let mut used = vec![false; kept.len()];
    let mut lines = vec![];
    // from ends and forks first, so each line runs between them; then the loops left
    let mut starts: Vec<usize> = at
        .iter()
        .filter(|(_, e)| e.len() != 2)
        .map(|(&n, _)| n)
        .collect();
    starts.sort_unstable();
    let mut loops: Vec<usize> = at
        .iter()
        .filter(|(_, e)| e.len() == 2)
        .map(|(&n, _)| n)
        .collect();
    loops.sort_unstable();
    for start in starts.into_iter().chain(loops) {
        while let Some(&first) = at[&start].iter().find(|&&e| !used[e]) {
            let mut line = CentreLine {
                points: vec![],
                radii: vec![],
            };
            let (mut node, mut edge) = (start, first);
            loop {
                used[edge] = true;
                let (f, g, site) = kept[edge];
                let other = if f == node { g } else { f };
                let (from, to) = (
                    corners[node].expect("kept edges join corners"),
                    corners[other].expect("kept edges join corners"),
                );
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "a small positive count"
                )]
                let pieces = (from.distance(to) / spacing).ceil().max(1.0) as usize;
                let first_piece = usize::from(!line.points.is_empty());
                for k in first_piece..=pieces {
                    #[expect(clippy::cast_precision_loss, reason = "a small count")]
                    let p = from.lerp(to, k as f64 / pieces as f64);
                    line.points.push(p);
                    line.radii.push(safe_radius(p.distance(site), spacing));
                }
                node = other;
                let edges = &at[&node];
                if edges.len() != 2 {
                    break;
                }
                match edges.iter().find(|&&e| !used[e]) {
                    Some(&e) => edge = e,
                    None => break,
                }
            }
            // a stub shorter than the spacing close to an edge (where points along a
            // straight edge come out a hair off) adds nothing the cone does not cut anyway;
            // a short one further in is the middle of a dot, and is kept
            let length: f64 = line.points.windows(2).map(|w| w[0].distance(w[1])).sum();
            let deepest = line.radii.iter().copied().fold(0.0, f64::max);
            if length > 2.0 * spacing || deepest > 2.0 * spacing {
                lines.push(simplified(line, spacing / 25.0));
            }
        }
    }
    lines
}

/// `line` with each point within `tolerance` of the straight stretch past it (in place and
/// in distance to the edge) left out, its radii made 1.5 `tolerance` smaller.
fn simplified(line: CentreLine, tolerance: f64) -> CentreLine {
    let n = line.points.len();
    if n < 3 {
        return line;
    }
    let at = |i: usize| glam::DVec3::new(line.points[i].x, line.points[i].y, line.radii[i]);
    let mut keep = vec![false; n];
    keep[0] = true;
    keep[n - 1] = true;
    let mut stack = vec![(0, n - 1)];
    while let Some((a, b)) = stack.pop() {
        let (pa, pb) = (at(a), at(b));
        let along = pb - pa;
        let length = along.length_squared();
        let off = |i: usize| {
            let q = at(i) - pa;
            let t = if length > 0.0 {
                (q.dot(along) / length).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (q - along * t).length()
        };
        if let Some((far, d)) = (a + 1..b)
            .map(|i| (i, off(i)))
            .max_by(|x, y| x.1.total_cmp(&y.1))
            && d > tolerance
        {
            keep[far] = true;
            stack.push((a, far));
            stack.push((far, b));
        }
    }
    // the distance along a stretch bows below the straight line between its ends, by up to
    // the tolerance (half as much again where it climbs as fast as it goes along, as it does
    // at most): each radius is lowered by that, so none is ever too big
    CentreLine {
        points: (0..n)
            .filter(|&i| keep[i])
            .map(|i| line.points[i])
            .collect(),
        radii: (0..n)
            .filter(|&i| keep[i])
            .map(|i| (line.radii[i] - 1.5 * tolerance).max(0.0))
            .collect(),
    }
}

/// The distance to the nearest edge from a point `nearest` from the nearest of the points
/// along the edges `spacing` apart: an edge between two of them can come a little closer.
fn safe_radius(nearest: f64, spacing: f64) -> f64 {
    (nearest * nearest - spacing * spacing / 2.0)
        .max(0.0)
        .sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polygon;

    fn rectangle(x: f64, y: f64, w: f64, h: f64) -> Path {
        Path::polyline(
            &[
                Point::new(x, y),
                Point::new(x + w, y),
                Point::new(x + w, y + h),
                Point::new(x, y + h),
            ],
            true,
        )
    }

    /// Every point of the lines, and more along their straight stretches every 0.05.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "small counts"
    )]
    fn all(lines: &[CentreLine]) -> Vec<(Point, f64)> {
        let mut points = vec![];
        for l in lines {
            points.push((l.points[0], l.radii[0]));
            for i in 1..l.points.len() {
                let (a, b) = (l.points[i - 1], l.points[i]);
                let steps = (a.distance(b) / 0.05).ceil().max(1.0);
                for k in 1..=steps as usize {
                    let t = k as f64 / steps;
                    points.push((
                        a.lerp(b, t),
                        l.radii[i - 1] + (l.radii[i] - l.radii[i - 1]) * t,
                    ));
                }
            }
        }
        points
    }

    #[test]
    fn a_strips_centre_line_runs_down_its_middle_half_its_width_from_the_edges() {
        let lines = centre_lines(&polygon::regions(&[rectangle(0.0, 0.0, 40.0, 4.0)]), 0.05);
        let points = all(&lines);
        // away from the ends, on the middle line at 2 from both edges
        let middle: Vec<_> = points
            .iter()
            .filter(|(p, _)| p.x > 3.0 && p.x < 37.0)
            .collect();
        assert_ne!(middle.len(), 0);
        for (p, r) in &middle {
            assert!((p.y - 2.0).abs() < 0.01, "{p}");
            assert!((r - 2.0).abs() < 0.01, "{r}");
        }
        // all along it
        for x in [4.0, 10.0, 20.0, 30.0, 36.0] {
            assert!(
                middle.iter().any(|(p, _)| (p.x - x).abs() < 0.1),
                "nothing near x {x}"
            );
        }
        // and out into each corner, coming to the edges there
        for corner in [
            Point::new(0.0, 0.0),
            Point::new(40.0, 0.0),
            Point::new(40.0, 4.0),
            Point::new(0.0, 4.0),
        ] {
            let (p, r) = points
                .iter()
                .min_by(|a, b| a.0.distance(corner).total_cmp(&b.0.distance(corner)))
                .unwrap();
            assert!(p.distance(corner) < 0.15, "{p} for {corner}");
            assert!(*r < 0.1, "{r}");
        }
    }

    #[test]
    fn every_point_is_no_further_from_an_edge_than_its_radius() {
        // a ring with a hole, and a star
        let star: Vec<Point> = (0..10)
            .map(|k| {
                let r = if k % 2 == 0 { 12.0 } else { 5.0 };
                Point::new(40.0, 0.0)
                    + Point::from_angle(f64::from(k) * std::f64::consts::PI / 5.0) * r
            })
            .collect();
        let paths = [
            rectangle(0.0, 0.0, 20.0, 20.0),
            rectangle(5.0, 5.0, 10.0, 10.0),
            Path::polyline(&star, true),
        ];
        let regions = polygon::regions(&paths);
        let rings: Vec<Vec<Point>> = paths
            .iter()
            .map(|p| {
                let mut r = cut_points(p);
                r.push(r[0]);
                r
            })
            .collect();
        let points = all(&centre_lines(&regions, 0.05));
        assert!(points.len() > 100);
        for (p, r) in points {
            let d = rings
                .iter()
                .map(|ring| crate::geometry::polyline_distance(ring, p))
                .fold(f64::INFINITY, f64::min);
            assert!(r <= d + 1e-9, "{p}: {r} but {d} from an edge");
            assert!(r >= d - 0.05, "{p}: {r} but {d} from an edge");
            assert!(crate::geometry::inside(&rings, p), "{p} outside");
        }
    }

    #[test]
    fn a_ring_around_a_hole_has_its_centre_line_all_the_way_round() {
        let paths = [
            rectangle(0.0, 0.0, 20.0, 20.0),
            rectangle(4.0, 4.0, 12.0, 12.0),
        ];
        let points = all(&centre_lines(&polygon::regions(&paths), 0.05));
        // the middle of the 4 wide band, on all four sides
        for at in [
            Point::new(10.0, 2.0),
            Point::new(18.0, 10.0),
            Point::new(10.0, 18.0),
            Point::new(2.0, 10.0),
        ] {
            let (p, r) = points
                .iter()
                .min_by(|a, b| a.0.distance(at).total_cmp(&b.0.distance(at)))
                .unwrap();
            assert!(p.distance(at) < 0.05, "{p} for {at}");
            assert!((r - 2.0).abs() < 0.01);
        }
    }

    #[test]
    fn a_round_dot_has_its_middle() {
        // a 0.8 mm dot drawn with 32 points, inside a bigger ring
        let dot: Vec<Point> = (0..32)
            .map(|k| {
                Point::new(10.0, 10.0)
                    + Point::from_angle(f64::from(k) * std::f64::consts::TAU / 32.0) * 0.4
            })
            .collect();
        let paths = [
            rectangle(0.0, 0.0, 20.0, 20.0),
            rectangle(5.0, 5.0, 10.0, 10.0),
            Path::polyline(&dot, true),
        ];
        let points = all(&centre_lines(&polygon::regions(&paths), 0.05));
        let (p, r) = points
            .iter()
            .min_by(|a, b| {
                a.0.distance(Point::new(10.0, 10.0))
                    .total_cmp(&b.0.distance(Point::new(10.0, 10.0)))
            })
            .unwrap();
        assert!(p.distance(Point::new(10.0, 10.0)) < 0.05, "{p}");
        assert!((r - 0.4).abs() < 0.03, "{r}");
    }

    #[test]
    fn triangles_cover_the_shapes_and_not_their_holes() {
        let paths = [
            rectangle(0.0, 0.0, 20.0, 10.0),
            rectangle(5.0, 2.0, 4.0, 4.0),
        ];
        let triangles = triangles(&polygon::regions(&paths));
        let area: f64 = triangles
            .iter()
            .map(|[a, b, c]| (*b - *a).perp_dot(*c - *a).abs() / 2.0)
            .sum();
        assert!((area - (200.0 - 16.0)).abs() < 1e-6, "{area}");
        let middle = |[a, b, c]: &[Point; 3]| (*a + *b + *c) / 3.0;
        assert!(triangles.iter().all(|t| {
            let m = middle(t);
            !(m.x > 5.0 && m.x < 9.0 && m.y > 2.0 && m.y < 6.0)
        }));
    }

    #[test]
    fn nothing_for_nothing() {
        assert_eq!(centre_lines(&[], 0.05).len(), 0);
    }
}
