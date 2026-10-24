use cut_craft_core::Change;
use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::geometry::{Path, Point, Segment};
use cut_craft_core::polygon::{self, Corners};
use cut_craft_core::testing::square;
use std::f64::consts::PI;

fn total_area(paths: &[Path]) -> f64 {
    paths.iter().map(polygon::area).sum()
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() < tol
}

#[test]
fn area_of_a_square_and_a_circle() {
    assert!(close(polygon::area(&square(0.0, 0.0, 10.0)), 100.0, 1e-9));
    // flattening to TOLERANCE can lose at most perimeter x tolerance
    let bound = 2.0 * PI * 10.0 * polygon::TOLERANCE;
    assert!(close(
        polygon::area(&Path::circle(Point::ZERO, 10.0)),
        PI * 100.0,
        bound
    ));
}

#[test]
fn union_of_overlapping_squares_is_one_outline() {
    let out = polygon::union(&[square(0.0, 0.0, 10.0), square(5.0, 5.0, 10.0)]);
    assert_eq!(out.len(), 1);
    assert!(close(total_area(&out), 175.0, 1e-6));
}

#[test]
fn union_of_separate_squares_keeps_both() {
    assert_eq!(
        polygon::union(&[square(0.0, 0.0, 10.0), square(20.0, 0.0, 10.0)]).len(),
        2
    );
}

#[test]
fn difference_and_intersection() {
    let (a, b) = (square(0.0, 0.0, 10.0), square(5.0, 5.0, 10.0));
    assert!(close(
        total_area(&polygon::difference(
            std::slice::from_ref(&a),
            std::slice::from_ref(&b)
        )),
        75.0,
        1e-6
    ));
    assert!(close(
        total_area(&polygon::intersection(&[a], &[b])),
        25.0,
        1e-6
    ));
}

#[test]
fn a_hole_comes_back_as_its_own_closed_path() {
    let out = polygon::difference(&[square(0.0, 0.0, 10.0)], &[square(3.0, 3.0, 4.0)]);
    assert_eq!(out.len(), 2, "outline plus hole");
    assert!(out.iter().all(|p| p.closed));
}

#[test]
fn offsetting_outwards_with_round_and_sharp_corners() {
    let s = [square(0.0, 0.0, 10.0)];
    assert!(close(
        total_area(&polygon::offset(&s, 1.0, Corners::Sharp)),
        144.0,
        1e-3
    ));
    assert!(close(
        total_area(&polygon::offset(&s, 1.0, Corners::Round)),
        140.0 + PI,
        0.05
    ));
}

#[test]
fn offsetting_inwards_shrinks() {
    assert!(close(
        total_area(&polygon::offset(
            &[square(0.0, 0.0, 10.0)],
            -1.0,
            Corners::Sharp
        )),
        64.0,
        1e-3
    ));
}

#[test]
fn reversing_a_path_walks_it_backwards_exactly() {
    let path = Path {
        start: Point::new(0.0, 0.0),
        segments: vec![
            Segment::Line {
                to: Point::new(10.0, 0.0),
            },
            Segment::Arc {
                to: Point::new(10.0, 10.0),
                center: Point::new(10.0, 5.0),
                ccw: true,
            },
            Segment::Cubic {
                c1: Point::new(5.0, 15.0),
                c2: Point::new(2.0, 12.0),
                to: Point::new(0.0, 10.0),
            },
        ],
        closed: false,
    };
    let back = path.reversed();
    assert_eq!(back.start, Point::new(0.0, 10.0));
    assert_eq!(
        back.segments[0],
        Segment::Cubic {
            c1: Point::new(2.0, 12.0),
            c2: Point::new(5.0, 15.0),
            to: Point::new(10.0, 10.0)
        }
    );
    assert_eq!(
        back.segments[1],
        Segment::Arc {
            to: Point::new(10.0, 0.0),
            center: Point::new(10.0, 5.0),
            ccw: false
        }
    );
    assert_eq!(
        back.segments[2],
        Segment::Line {
            to: Point::new(0.0, 0.0)
        }
    );
    assert_eq!(back.reversed(), path);
}

#[test]
fn swap_replaces_vectors_with_new_selected_ones() {
    let mut doc = Document::default();
    let setup = Change::add_and_select(&doc, square(0.0, 0.0, 1.0));
    doc.apply(&setup).unwrap();
    let change = Change::swap(
        &doc,
        [EntityId(1)],
        vec![square(5.0, 5.0, 1.0), square(9.0, 9.0, 1.0)],
    );
    doc.apply(&change).unwrap();
    assert!(!doc.vectors.contains_key(&EntityId(1)));
    assert_eq!(doc.vectors.len(), 2);
    assert_eq!(doc.selection.len(), 2);
}

#[test]
fn regions_make_inner_paths_holes_and_shrinking_grows_the_holes() {
    use cut_craft_core::geometry::inside;
    let regions = polygon::regions(&[square(0.0, 0.0, 100.0), square(40.0, 40.0, 20.0)]);
    assert_eq!(regions.len(), 2, "outline and hole");
    let shrunk = polygon::offset_regions(&regions, -5.0, Corners::Sharp);
    let mut areas: Vec<f64> = shrunk.iter().map(polygon::area).collect();
    areas.sort_by(f64::total_cmp);
    assert!(
        close(areas[0], 900.0, 1e-3) && close(areas[1], 8100.0, 1e-3),
        "{areas:?}"
    );
    let rings: Vec<Vec<Point>> = shrunk.iter().map(|p| p.flatten(0.01)).collect();
    assert!(inside(&rings, Point::new(20.0, 50.0)));
    assert!(
        !inside(&rings, Point::new(37.0, 50.0)),
        "the hole grew to 35"
    );
    assert!(
        !inside(&rings, Point::new(3.0, 50.0)),
        "the outline shrank to 5"
    );
}
