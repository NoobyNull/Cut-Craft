//! Booleans and offsets on closed paths, via `i_overlay`. Each closed path is treated as a
//! filled region; curves are flattened first and results come back as closed polylines.

use crate::geometry::{Path, Point, Segment};
use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::single::SingleFloatOverlay;
use i_overlay::mesh::float::outline::offset::OutlineOffset;
use i_overlay::mesh::float::stroke::offset::StrokeOffset;
use i_overlay::mesh::float::style::{LineCap, LineJoin, OutlineStyle, StrokeStyle};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// Chord tolerance (mm) for flattening curves before a boolean or offset unless Options ▸
/// Advanced says otherwise ([`crate::accuracy::curve`]); drawing keeps to this one.
// ponytail: results are polylines; fit arcs back onto them if curve fidelity matters downstream
pub const TOLERANCE: f64 = 0.01;

type Contour = Vec<[f64; 2]>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Corners {
    /// Corners curve round, like the path of a round bit
    #[default]
    Round,
    /// Corners stay pointed, the sides carried on until they meet
    Sharp,
}

pub fn union(paths: &[Path]) -> Vec<Path> {
    overlay(paths, &[], OverlayRule::Union, FillRule::NonZero)
}

/// The regions of `subject` not covered by `clip`.
pub fn difference(subject: &[Path], clip: &[Path]) -> Vec<Path> {
    overlay(subject, clip, OverlayRule::Difference, FillRule::NonZero)
}

/// Where `subject` and `clip` overlap.
pub fn intersection(subject: &[Path], clip: &[Path]) -> Vec<Path> {
    overlay(subject, clip, OverlayRule::Intersect, FillRule::NonZero)
}

/// Grows (positive `distance`) or shrinks (negative) the closed paths.
pub fn offset(paths: &[Path], distance: f64, corners: Corners) -> Vec<Path> {
    outline(&contours(paths), distance, corners)
}

/// The closed paths as regions where a path inside another is a hole (even-odd), as outlines
/// running anticlockwise and holes clockwise, for [`offset_regions`].
pub fn regions(paths: &[Path]) -> Vec<Path> {
    overlay(paths, &[], OverlayRule::Union, FillRule::EvenOdd)
}

/// [`regions`] (or paths as `regions` gives them) as separate pieces: each outline with the
/// holes in it, so each can be worked on by itself.
pub fn pieces(regions: &[Path]) -> Vec<Vec<Path>> {
    contours(regions)
        .overlay(
            &Vec::<Contour>::new(),
            OverlayRule::Union,
            FillRule::EvenOdd,
        )
        .into_iter()
        .map(|shape| to_paths(vec![shape]))
        .filter(|piece| !piece.is_empty())
        .collect()
}

/// Grows or shrinks [`regions`]: shrinking pulls outlines in and pushes holes out.
pub fn offset_regions(regions: &[Path], distance: f64, corners: Corners) -> Vec<Path> {
    let oriented: Vec<Contour> = regions.iter().filter(|p| p.closed).map(contour).collect();
    outline(&oriented, distance, corners)
}

/// [`offset`] with round corners, for offsetting the result again and again (rings stepping
/// in from the edge): points that lie within a tenth of [`TOLERANCE`] of the line past them
/// are dropped. Without that each step's round corners got a little round corner at every
/// point of the last step's, doubling their points each step, so a big pocket or carving
/// took minutes.
pub fn offset_again(paths: &[Path], distance: f64) -> Vec<Path> {
    thinned(offset(paths, distance, Corners::Round))
}

/// [`offset_regions`] with round corners, thinned as [`offset_again`] is.
pub fn offset_regions_again(regions: &[Path], distance: f64) -> Vec<Path> {
    thinned(offset_regions(regions, distance, Corners::Round))
}

fn thinned(rings: Vec<Path>) -> Vec<Path> {
    rings
        .into_iter()
        .map(|ring| without_spare_points(ring, TOLERANCE / 10.0))
        .collect()
}

/// A closed polyline without the points within `tolerance` of the line from the last point
/// kept to the next; other paths as they are.
fn without_spare_points(ring: Path, tolerance: f64) -> Path {
    let mut points = vec![ring.start];
    for s in &ring.segments {
        match s {
            Segment::Line { to } => points.push(*to),
            _ => return ring,
        }
    }
    if points.len() > 1 && points.first() == points.last() {
        points.pop();
    }
    if !ring.closed || points.len() < 4 {
        return ring;
    }
    let off_line = |a: Point, b: Point, q: Point| {
        let ab = b - a;
        let length = ab.length();
        if length == 0.0 {
            q.distance(a)
        } else {
            (ab.perp_dot(q - a) / length).abs()
        }
    };
    let n = points.len();
    let mut kept = vec![points[0]];
    for i in 1..n {
        let next = points[(i + 1) % n];
        if off_line(*kept.last().expect("starts with one"), next, points[i]) > tolerance {
            kept.push(points[i]);
        }
    }
    if kept.len() < 3 {
        return ring;
    }
    Path::polyline(&kept, true)
}

/// [`offset_regions`] with sharp corners, each mitred out no further than `reach` times the
/// distance from its corner; a corner too sharp for that is squared off at the limit.
pub fn offset_regions_mitred(regions: &[Path], distance: f64, reach: f64) -> Vec<Path> {
    let oriented: Vec<Contour> = regions.iter().filter(|p| p.closed).map(contour).collect();
    // a mitre reaches the distance over the sine of half the corner's inside angle
    let sharpest = 2.0 * (1.0 / reach.max(1.0)).asin();
    joined(&oriented, distance, LineJoin::Miter(sharpest))
}

fn outline(contours: &[Contour], distance: f64, corners: Corners) -> Vec<Path> {
    let join = match corners {
        // segment angle that keeps each arc chord within TOLERANCE of the true corner arc
        Corners::Round => LineJoin::Round(
            2.0 * (1.0 - (crate::accuracy::curve() / distance.abs()).min(1.0)).acos(),
        ),
        Corners::Sharp => LineJoin::Miter(0.01 * PI),
    };
    joined(contours, distance, join)
}

fn joined(contours: &[Contour], distance: f64, join: LineJoin<f64>) -> Vec<Path> {
    to_paths(contours.outline(&OutlineStyle::new(distance).line_join(join)))
}

/// The area within `half_width` of the lines of `paths`, open or closed, with round ends and
/// corners: the band a bit that wide cuts following them.
pub fn stroke(paths: &[Path], half_width: f64) -> Vec<Path> {
    if half_width <= 0.0 {
        return vec![];
    }
    let round = 2.0 * (1.0 - (crate::accuracy::curve() / half_width).min(1.0)).acos();
    let style = StrokeStyle::new(2.0 * half_width)
        .line_join(LineJoin::Round(round))
        .start_cap(LineCap::Round(round))
        .end_cap(LineCap::Round(round));
    let bands: Vec<Path> = paths
        .iter()
        .flat_map(|p| to_paths(contour(p).stroke(style.clone(), p.closed)))
        .collect();
    union(&bands)
}

/// Enclosed area of a closed path (curves flattened).
pub fn area(path: &Path) -> f64 {
    signed_area(&contour(path)).abs()
}

fn overlay(subject: &[Path], clip: &[Path], rule: OverlayRule, fill: FillRule) -> Vec<Path> {
    to_paths(contours(subject).overlay(&contours(clip), rule, fill))
}

/// Closed paths as counter-clockwise contours, so each is a filled region whatever its direction.
fn contours(paths: &[Path]) -> Vec<Contour> {
    paths
        .iter()
        .filter(|p| p.closed)
        .map(|p| {
            let mut c = contour(p);
            if signed_area(&c) < 0.0 {
                c.reverse();
            }
            c
        })
        .collect()
}

fn contour(path: &Path) -> Contour {
    let mut points: Contour = path
        .flatten(crate::accuracy::curve())
        .into_iter()
        .map(|p| [p.x, p.y])
        .collect();
    if points.len() > 1 && points.first() == points.last() {
        points.pop();
    }
    points
}

fn signed_area(c: &[[f64; 2]]) -> f64 {
    let n = c.len();
    (0..n)
        .map(|i| c[i][0] * c[(i + 1) % n][1] - c[(i + 1) % n][0] * c[i][1])
        .sum::<f64>()
        / 2.0
}

fn to_paths(shapes: Vec<Vec<Contour>>) -> Vec<Path> {
    shapes
        .into_iter()
        .flatten()
        .filter(|c| c.len() >= 3)
        .map(|c| {
            Path::polyline(
                &c.iter().map(|&[x, y]| Point::new(x, y)).collect::<Vec<_>>(),
                true,
            )
        })
        .collect()
}
