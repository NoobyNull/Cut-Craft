use cut_craft_core::Drag;
use cut_craft_core::geometry::{Point, Rect};

fn drag(sx: f64, sy: f64, ex: f64, ey: f64) -> Drag {
    Drag {
        start: Point::new(sx, sy),
        end: Point::new(ex, ey),
        from_center: false,
        constrain: false,
    }
}

#[test]
fn corner_to_corner_in_any_direction() {
    assert_eq!(
        drag(10.0, 20.0, 4.0, 2.0).rect(),
        Rect::new(Point::new(4.0, 2.0), Point::new(10.0, 20.0))
    );
}

#[test]
fn from_centre_mirrors_the_drag_around_the_start() {
    let d = Drag {
        from_center: true,
        ..drag(10.0, 10.0, 13.0, 12.0)
    };
    assert_eq!(
        d.rect(),
        Rect::new(Point::new(7.0, 8.0), Point::new(13.0, 12.0))
    );
}

#[test]
fn constrain_makes_a_square_in_the_drag_direction() {
    let d = Drag {
        constrain: true,
        ..drag(10.0, 10.0, 4.0, 12.0)
    };
    assert_eq!(
        d.rect(),
        Rect::new(Point::new(4.0, 10.0), Point::new(10.0, 16.0))
    );
}

#[test]
fn constrain_and_from_centre_together() {
    let d = Drag {
        constrain: true,
        from_center: true,
        ..drag(0.0, 0.0, 3.0, -1.0)
    };
    assert_eq!(
        d.rect(),
        Rect::new(Point::new(-3.0, -3.0), Point::new(3.0, 3.0))
    );
}
