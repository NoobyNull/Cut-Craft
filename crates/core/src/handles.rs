//! Transform mode's handles on a selection's bounds (Aspire's help: Interactive Selection
//! Mode): the middle moves it; white corners scale it (Alt: each axis on its own, Shift:
//! about the centre); edges scale one axis (Shift: both); with rotation on (a second click
//! on the middle) the corners rotate it about the centre (Alt: in 15° steps).

use crate::geometry::{Point, Rect};
use glam::DAffine2;

/// One of the handles, by where it sits: `x` and `y` are -1, 0 or 1 across and up the
/// bounds (0, 0 the middle).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handle {
    Move,
    Scale { x: i8, y: i8 },
    Rotate { x: i8, y: i8 },
}

/// The keys held while dragging.
#[derive(Clone, Copy, Debug, Default)]
pub struct Keys {
    pub alt: bool,
    pub shift: bool,
}

/// Where `handle` sits on `bounds`.
pub fn position(bounds: Rect, handle: Handle) -> Point {
    let (x, y) = match handle {
        Handle::Move => (0, 0),
        Handle::Scale { x, y } | Handle::Rotate { x, y } => (x, y),
    };
    bounds.center() + bounds.size() / 2.0 * Point::new(f64::from(x), f64::from(y))
}

/// Every handle of a selection with these `bounds`: rotation handles at the corners while
/// rotating, scale handles at the corners and edges otherwise.
pub fn handles(rotating: bool) -> Vec<Handle> {
    let mut out = vec![Handle::Move];
    for x in -1..=1 {
        for y in -1..=1 {
            match (x, y) {
                (0, 0) => {}
                (x, y) if rotating && x != 0 && y != 0 => out.push(Handle::Rotate { x, y }),
                (x, y) if !rotating => out.push(Handle::Scale { x, y }),
                _ => {}
            }
        }
    }
    out
}

/// The handle within `reach` of `at`, the nearest when several are.
pub fn pick(bounds: Rect, rotating: bool, at: Point, reach: f64) -> Option<Handle> {
    handles(rotating)
        .into_iter()
        .map(|h| (h, position(bounds, h).distance(at)))
        .filter(|(_, d)| *d <= reach)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(h, _)| h)
}

/// What dragging `handle` from `start` to `end` does to a selection with these `bounds`.
pub fn transform(bounds: Rect, handle: Handle, start: Point, end: Point, keys: Keys) -> DAffine2 {
    let centre = bounds.center();
    match handle {
        Handle::Move => {
            let mut by = end - start;
            if keys.alt {
                // along one axis only
                if by.x.abs() > by.y.abs() {
                    by.y = 0.0;
                } else {
                    by.x = 0.0;
                }
            }
            DAffine2::from_translation(by)
        }
        Handle::Scale { x, y } => {
            let (x, y) = (f64::from(x), f64::from(y));
            // about the opposite corner or edge, or with Shift the centre
            let anchor = if keys.shift {
                centre
            } else {
                centre - bounds.size() / 2.0 * Point::new(x, y)
            };
            let ratio = |from: f64, to: f64| if from.abs() > 1e-12 { to / from } else { 1.0 };
            let (from, to) = (start - anchor, end - anchor);
            let (mut sx, mut sy) = (ratio(from.x, to.x), ratio(from.y, to.y));
            let corner = x != 0.0 && y != 0.0;
            if x == 0.0 {
                sx = 1.0;
            }
            if y == 0.0 {
                sy = 1.0;
            }
            // corners keep the shape unless Alt; edges change one axis unless Shift
            let proportional = if corner { !keys.alt } else { keys.shift };
            if proportional {
                let s = if from.length_squared() > 1e-12 {
                    to.dot(from) / from.length_squared()
                } else {
                    1.0
                };
                (sx, sy) = (s, s);
            }
            DAffine2::from_translation(anchor)
                * DAffine2::from_scale(Point::new(sx, sy))
                * DAffine2::from_translation(-anchor)
        }
        Handle::Rotate { .. } => {
            let mut angle = (start - centre).angle_to(end - centre);
            if keys.alt {
                let step = 15_f64.to_radians();
                angle = (angle / step).round() * step;
            }
            DAffine2::from_translation(centre)
                * DAffine2::from_angle(angle)
                * DAffine2::from_translation(-centre)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOX: Rect = Rect {
        min: Point::new(0.0, 0.0),
        max: Point::new(20.0, 10.0),
    };

    fn moved(h: Handle, start: Point, end: Point, keys: Keys) -> (Point, Point) {
        let t = transform(BOX, h, start, end, keys);
        (t.transform_point2(BOX.min), t.transform_point2(BOX.max))
    }

    #[test]
    fn handles_sit_on_the_bounds_and_are_picked_nearby() {
        assert_eq!(handles(false).len(), 9);
        assert_eq!(handles(true).len(), 5);
        let top_right = Handle::Scale { x: 1, y: 1 };
        assert_eq!(position(BOX, top_right), Point::new(20.0, 10.0));
        assert_eq!(
            pick(BOX, false, Point::new(19.5, 9.6), 1.0),
            Some(top_right)
        );
        assert_eq!(
            pick(BOX, false, Point::new(10.2, 5.0), 1.0),
            Some(Handle::Move)
        );
        assert_eq!(
            pick(BOX, true, Point::new(20.0, 5.0), 1.0),
            None,
            "no edges while rotating"
        );
        assert_eq!(pick(BOX, false, Point::new(30.0, 30.0), 1.0), None);
    }

    #[test]
    fn the_middle_moves_and_alt_keeps_to_one_axis() {
        let (a, b) = (Point::new(10.0, 5.0), Point::new(15.0, 7.0));
        assert_eq!(
            moved(Handle::Move, a, b, Keys::default()).0,
            Point::new(5.0, 2.0)
        );
        let keys = Keys {
            alt: true,
            shift: false,
        };
        assert_eq!(moved(Handle::Move, a, b, keys).0, Point::new(5.0, 0.0));
    }

    #[test]
    fn corners_scale_about_the_opposite_corner_keeping_the_shape() {
        let h = Handle::Scale { x: 1, y: 1 };
        let (lo, hi) = moved(
            h,
            Point::new(20.0, 10.0),
            Point::new(40.0, 20.0),
            Keys::default(),
        );
        assert_eq!((lo, hi), (Point::ZERO, Point::new(40.0, 20.0)));
        let alt = Keys {
            alt: true,
            shift: false,
        };
        let (_, hi) = moved(h, Point::new(20.0, 10.0), Point::new(30.0, 30.0), alt);
        assert_eq!(hi, Point::new(30.0, 30.0), "each axis on its own");
        let shift = Keys {
            alt: false,
            shift: true,
        };
        let (lo, hi) = moved(h, Point::new(20.0, 10.0), Point::new(30.0, 15.0), shift);
        assert_eq!(
            (lo, hi),
            (Point::new(-10.0, -5.0), Point::new(30.0, 15.0)),
            "about the centre"
        );
    }

    #[test]
    fn edges_scale_one_axis() {
        let right = Handle::Scale { x: 1, y: 0 };
        let (lo, hi) = moved(
            right,
            Point::new(20.0, 5.0),
            Point::new(30.0, 9.0),
            Keys::default(),
        );
        assert_eq!((lo, hi), (Point::ZERO, Point::new(30.0, 10.0)));
    }

    #[test]
    fn rotation_turns_about_the_centre_in_steps_with_alt() {
        let h = Handle::Rotate { x: 1, y: 1 };
        let centre = BOX.center();
        let t = transform(
            BOX,
            h,
            centre + Point::new(10.0, 0.0),
            centre + Point::new(0.0, 10.0),
            Keys::default(),
        );
        let turned = t.transform_point2(centre + Point::new(1.0, 0.0));
        assert!(
            (turned - (centre + Point::new(0.0, 1.0))).length() < 1e-9,
            "a quarter turn"
        );
        let alt = Keys {
            alt: true,
            shift: false,
        };
        let near_20 = centre + Point::new(10.0, 0.0).rotate(Point::from_angle(20_f64.to_radians()));
        let t = transform(BOX, h, centre + Point::new(10.0, 0.0), near_20, alt);
        let turned = t.transform_point2(centre + Point::new(1.0, 0.0)) - centre;
        assert!((turned.to_angle().to_degrees() - 15.0).abs() < 1e-9);
    }
}
