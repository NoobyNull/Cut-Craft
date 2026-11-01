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

/// The keys held while dragging. Shift or Ctrl constrains any handle (scale in proportion,
/// rotate in steps of 5.625° with Shift or 1° with Ctrl, move along one axis); without them a handle goes freely. Alt scales
/// about the centre.
#[derive(Clone, Copy, Debug, Default)]
pub struct Keys {
    pub alt: bool,
    pub shift: bool,
    pub ctrl: bool,
}

/// The angle rotation snaps to with Shift, degrees: a sixty-fourth of a turn.
pub const ROTATE_STEP: f64 = 360.0 / 64.0;

/// The angle rotation snaps to with Ctrl, degrees.
pub const FINE_ROTATE_STEP: f64 = 1.0;

impl Keys {
    /// Whether the drag is constrained: Shift or Ctrl held.
    pub fn constrained(self) -> bool {
        self.shift || self.ctrl
    }

    /// The step rotation snaps to, degrees: 1° with Ctrl, 5.625° with Shift, none freely.
    pub fn rotate_step(self) -> Option<f64> {
        if self.ctrl {
            Some(FINE_ROTATE_STEP)
        } else if self.shift {
            Some(ROTATE_STEP)
        } else {
            None
        }
    }
}

impl Handle {
    /// What it does and the keys that change it, for the status bar.
    pub fn hint(self) -> &'static str {
        match self {
            Self::Move => "Move: drag. Hold Shift or Ctrl to keep to one axis",
            Self::Scale { x, y } if x != 0 && y != 0 => {
                "Scale: drag. Hold Shift or Ctrl to keep the proportions, Alt to scale about the centre"
            }
            Self::Scale { .. } => {
                "Scale one way: drag. Hold Shift or Ctrl to scale both ways in proportion, Alt to scale about the centre"
            }
            Self::Rotate { .. } => "Rotate: drag. Hold Shift to snap to 5.625°, or Ctrl to 1°",
        }
    }
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
    pick_among(&handles(rotating), bounds, at, reach, 0.0)
}

/// Every handle at once: the scale handles on the box and the rotation handles `out`
/// beyond its corners, diagonally (a plain selection's, so neither needs a click to show).
pub fn all() -> Vec<Handle> {
    let mut out = handles(false);
    out.extend(handles(true).into_iter().filter(|h| *h != Handle::Move));
    out
}

/// Where `handle` sits, a rotation handle `out` further from each side than its corner.
pub fn placed(bounds: Rect, handle: Handle, out: f64) -> Point {
    let at = position(bounds, handle);
    match handle {
        Handle::Rotate { x, y } => at + Point::new(f64::from(x), f64::from(y)) * out,
        _ => at,
    }
}

/// The one of `among` within `reach` of `at`, the nearest, rotation handles `out` beyond the
/// corners.
pub fn pick_among(
    among: &[Handle],
    bounds: Rect,
    at: Point,
    reach: f64,
    out: f64,
) -> Option<Handle> {
    among
        .iter()
        .map(|&h| (h, placed(bounds, h, out).distance(at)))
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
            if keys.constrained() {
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
            // about the opposite corner or edge, or with Alt the centre
            let anchor = if keys.alt {
                centre
            } else {
                centre - bounds.size() / 2.0 * Point::new(x, y)
            };
            let ratio = |from: f64, to: f64| if from.abs() > 1e-12 { to / from } else { 1.0 };
            let (from, to) = (start - anchor, end - anchor);
            let (mut sx, mut sy) = (ratio(from.x, to.x), ratio(from.y, to.y));
            if x == 0.0 {
                sx = 1.0;
            }
            if y == 0.0 {
                sy = 1.0;
            }
            // freely (a corner each axis on its own, an edge its own axis) unless constrained,
            // then in proportion
            if keys.constrained() {
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
            if let Some(step) = keys.rotate_step() {
                let step = step.to_radians();
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

    const SHIFT: Keys = Keys {
        alt: false,
        shift: true,
        ctrl: false,
    };
    const CTRL: Keys = Keys {
        alt: false,
        shift: false,
        ctrl: true,
    };
    const ALT: Keys = Keys {
        alt: true,
        shift: false,
        ctrl: false,
    };

    #[test]
    fn the_middle_moves_freely_and_constrained_to_one_axis() {
        let (a, b) = (Point::new(10.0, 5.0), Point::new(15.0, 7.0));
        assert_eq!(
            moved(Handle::Move, a, b, Keys::default()).0,
            Point::new(5.0, 2.0)
        );
        for keys in [SHIFT, CTRL] {
            assert_eq!(moved(Handle::Move, a, b, keys).0, Point::new(5.0, 0.0));
        }
    }

    #[test]
    fn corners_scale_freely_or_with_shift_or_ctrl_in_proportion() {
        let h = Handle::Scale { x: 1, y: 1 };
        let (from, to) = (Point::new(20.0, 10.0), Point::new(30.0, 30.0));
        let (lo, hi) = moved(h, from, to, Keys::default());
        assert_eq!(
            (lo, hi),
            (Point::ZERO, Point::new(30.0, 30.0)),
            "each axis on its own"
        );
        for keys in [SHIFT, CTRL] {
            let (lo, hi) = moved(h, from, Point::new(40.0, 20.0), keys);
            assert_eq!(
                (lo, hi),
                (Point::ZERO, Point::new(40.0, 20.0)),
                "in proportion"
            );
            let (_, hi) = moved(h, from, to, keys);
            assert!((hi.x / hi.y - 2.0).abs() < 1e-9, "the shape kept: {hi:?}");
        }
        let (lo, hi) = moved(h, from, Point::new(30.0, 15.0), ALT);
        assert_eq!(
            (lo, hi),
            (Point::new(-10.0, -5.0), Point::new(30.0, 15.0)),
            "about the centre"
        );
    }

    #[test]
    fn edges_scale_their_own_axis_or_constrained_in_proportion() {
        let right = Handle::Scale { x: 1, y: 0 };
        let (from, to) = (Point::new(20.0, 5.0), Point::new(30.0, 9.0));
        let (lo, hi) = moved(right, from, to, Keys::default());
        assert_eq!((lo, hi), (Point::ZERO, Point::new(30.0, 10.0)));
        // in proportion about the opposite edge's middle: twice as long, twice as tall
        let (lo, hi) = moved(right, from, Point::new(40.0, 5.0), SHIFT);
        assert_eq!(hi - lo, Point::new(40.0, 20.0), "{lo:?} {hi:?}");
    }

    #[test]
    fn rotation_turns_freely_or_constrained_in_steps_of_a_64th_turn() {
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
        // with Shift 7° snaps to 5.625° and 10° to 11.25°; with Ctrl to whole degrees
        for (keys, dragged, snapped) in [
            (SHIFT, 7.0, 5.625),
            (SHIFT, 10.0, 11.25),
            (CTRL, 7.4, 7.0),
            (CTRL, 10.6, 11.0),
        ] {
            let to =
                centre + Point::new(10.0, 0.0).rotate(Point::from_angle(f64::to_radians(dragged)));
            let t = transform(BOX, h, centre + Point::new(10.0, 0.0), to, keys);
            let turned = t.transform_point2(centre + Point::new(1.0, 0.0)) - centre;
            assert!(
                (turned.to_angle().to_degrees() - snapped).abs() < 1e-9,
                "{dragged}°"
            );
        }
    }

    #[test]
    fn every_handle_at_once_the_rotation_ones_beyond_the_corners() {
        let every = all();
        assert_eq!(every.len(), 13, "the middle, eight scale, four rotation");
        let turn = Handle::Rotate { x: 1, y: 1 };
        assert_eq!(placed(BOX, turn, 2.0), Point::new(22.0, 12.0));
        // the corner itself scales; just beyond it, rotates
        let corner = Point::new(20.0, 10.0);
        assert_eq!(
            pick_among(&every, BOX, corner, 1.0, 2.0),
            Some(Handle::Scale { x: 1, y: 1 })
        );
        assert_eq!(
            pick_among(&every, BOX, Point::new(22.3, 11.8), 1.0, 2.0),
            Some(turn)
        );
    }
}
