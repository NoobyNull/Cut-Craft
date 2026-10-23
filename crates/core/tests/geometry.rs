use aspire_core::geometry::{Path, Point, Rect, Segment};
use glam::DAffine2;
use std::f64::consts::FRAC_PI_2;

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn close(a: Point, b: Point) -> bool {
    a.distance(b) < 1e-9
}

#[test]
fn polyline_bounds_cover_every_vertex() {
    let path = Path::polyline(&[p(1.0, 2.0), p(5.0, -1.0), p(3.0, 7.0)], true);
    assert_eq!(path.bounds(), Rect::new(p(1.0, -1.0), p(5.0, 7.0)));
}

#[test]
fn circle_bounds_include_arc_extremes_not_just_endpoints() {
    let circle = Path::circle(p(10.0, 20.0), 5.0);
    let b = circle.bounds();
    assert!(
        close(b.min, p(5.0, 15.0)) && close(b.max, p(15.0, 25.0)),
        "{b:?}"
    );
}

#[test]
fn quarter_arc_bounds_are_its_endpoints_only() {
    // centre (0,0), from (5,0) counter-clockwise to (0,5): no axis extreme inside the sweep
    let arc = Path {
        start: p(5.0, 0.0),
        segments: vec![Segment::Arc {
            to: p(0.0, 5.0),
            center: p(0.0, 0.0),
            ccw: true,
        }],
        closed: false,
    };
    let b = arc.bounds();
    assert!(
        close(b.min, p(0.0, 0.0)) && close(b.max, p(5.0, 5.0)),
        "{b:?}"
    );
}

#[test]
fn cubic_bounds_are_tight_not_the_control_hull() {
    // symmetric bulge: control points at y=4 but the curve peaks at y=3
    let curve = Path {
        start: p(0.0, 0.0),
        segments: vec![Segment::Cubic {
            c1: p(0.0, 4.0),
            c2: p(4.0, 4.0),
            to: p(4.0, 0.0),
        }],
        closed: false,
    };
    let b = curve.bounds();
    assert!((b.max.y - 3.0).abs() < 1e-9, "{b:?}");
}

#[test]
fn similarity_transform_keeps_arcs_as_arcs() {
    let moved = Path::circle(p(0.0, 0.0), 1.0).transformed(
        &DAffine2::from_scale_angle_translation(glam::DVec2::splat(2.0), FRAC_PI_2, p(3.0, 4.0)),
    );
    assert!(
        moved
            .segments
            .iter()
            .all(|s| matches!(s, Segment::Arc { .. }))
    );
    let b = moved.bounds();
    assert!(
        close(b.min, p(1.0, 2.0)) && close(b.max, p(5.0, 6.0)),
        "{b:?}"
    );
}

#[test]
fn mirror_flips_arc_direction() {
    let mirrored = Path::circle(p(0.0, 0.0), 1.0)
        .transformed(&DAffine2::from_scale(glam::DVec2::new(-1.0, 1.0)));
    assert!(
        mirrored
            .segments
            .iter()
            .all(|s| matches!(s, Segment::Arc { ccw: false, .. }))
    );
}

#[test]
fn non_uniform_scale_turns_arcs_into_cubics() {
    let squashed = Path::circle(p(0.0, 0.0), 1.0)
        .transformed(&DAffine2::from_scale(glam::DVec2::new(2.0, 1.0)));
    assert!(
        squashed
            .segments
            .iter()
            .all(|s| matches!(s, Segment::Cubic { .. }))
    );
    let b = squashed.bounds();
    assert!(
        (b.max.x - 2.0).abs() < 1e-3 && (b.max.y - 1.0).abs() < 1e-3,
        "{b:?}"
    );
}

#[test]
fn rect_union_and_centre() {
    let r = Rect::new(p(0.0, 0.0), p(2.0, 2.0)).union(Rect::new(p(1.0, -1.0), p(4.0, 1.0)));
    assert_eq!(r, Rect::new(p(0.0, -1.0), p(4.0, 2.0)));
    assert_eq!(r.center(), p(2.0, 0.5));
    assert_eq!(r.size(), glam::DVec2::new(4.0, 3.0));
}
