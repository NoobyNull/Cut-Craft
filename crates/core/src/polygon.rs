//! Booleans and offsets on closed paths, via `i_overlay`. Each closed path is treated as a
//! filled region; curves are flattened first and results come back as closed polylines.

use crate::geometry::{Path, Point};
use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::single::SingleFloatOverlay;
use i_overlay::mesh::float::outline::offset::OutlineOffset;
use i_overlay::mesh::float::style::{LineJoin, OutlineStyle};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// Chord tolerance (mm) for flattening curves before a boolean or offset.
// ponytail: results are polylines; fit arcs back onto them if curve fidelity matters downstream
pub const TOLERANCE: f64 = 0.01;

type Contour = Vec<[f64; 2]>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Corners {
    #[default]
    Round,
    Sharp,
}

pub fn union(paths: &[Path]) -> Vec<Path> {
    overlay(paths, &[], OverlayRule::Union)
}

/// The regions of `subject` not covered by `clip`.
pub fn difference(subject: &[Path], clip: &[Path]) -> Vec<Path> {
    overlay(subject, clip, OverlayRule::Difference)
}

/// Where `subject` and `clip` overlap.
pub fn intersection(subject: &[Path], clip: &[Path]) -> Vec<Path> {
    overlay(subject, clip, OverlayRule::Intersect)
}

/// Grows (positive `distance`) or shrinks (negative) the closed paths.
pub fn offset(paths: &[Path], distance: f64, corners: Corners) -> Vec<Path> {
    let join = match corners {
        // segment angle that keeps each arc chord within TOLERANCE of the true corner arc
        Corners::Round => {
            LineJoin::Round(2.0 * (1.0 - (TOLERANCE / distance.abs()).min(1.0)).acos())
        }
        Corners::Sharp => LineJoin::Miter(0.01 * PI),
    };
    to_paths(contours(paths).outline(&OutlineStyle::new(distance).line_join(join)))
}

/// Enclosed area of a closed path (curves flattened).
pub fn area(path: &Path) -> f64 {
    signed_area(&contour(path)).abs()
}

fn overlay(subject: &[Path], clip: &[Path], rule: OverlayRule) -> Vec<Path> {
    to_paths(contours(subject).overlay(&contours(clip), rule, FillRule::NonZero))
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
        .flatten(TOLERANCE)
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
