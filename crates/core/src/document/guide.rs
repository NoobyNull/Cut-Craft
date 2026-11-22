//! Guide lines in the 2D view.

use super::{Deserialize, Point, Serialize};

/// A guide line in the 2D view: up and down at x = `at` when vertical, else across at
/// y = `at`. A locked guide cannot be dragged.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Guide {
    pub vertical: bool,
    pub at: f64,
    #[serde(default)]
    pub locked: bool,
    /// Turned from its own direction (up for a vertical guide, across for a horizontal one)
    /// in degrees anticlockwise, about where it crosses its axis (added with angled guides).
    #[serde(default)]
    pub tilt: f64,
}

impl Guide {
    pub fn vertical(x: f64) -> Self {
        Self {
            vertical: true,
            at: x,
            locked: false,
            tilt: 0.0,
        }
    }

    pub fn horizontal(y: f64) -> Self {
        Self {
            vertical: false,
            at: y,
            locked: false,
            tilt: 0.0,
        }
    }

    /// Its direction in degrees anticlockwise from the X axis: 0 across, 90 up, plus its
    /// tilt (guide angle).
    #[must_use]
    pub fn angle(&self) -> f64 {
        let own = if self.vertical { 90.0 } else { 0.0 };
        own + self.tilt
    }

    /// Whether it is turned off its axis.
    #[must_use]
    pub fn tilted(&self) -> bool {
        self.tilt.abs() > 1e-9
    }

    /// The guide at `degrees` from the X axis.
    #[must_use]
    pub fn with_angle(self, degrees: f64) -> Self {
        Self {
            tilt: degrees - if self.vertical { 90.0 } else { 0.0 },
            ..self
        }
    }

    /// Where it crosses its axis: (`at`, 0) for a vertical guide, (0, `at`) for a horizontal.
    #[must_use]
    pub fn anchor(&self) -> Point {
        if self.vertical {
            Point::new(self.at, 0.0)
        } else {
            Point::new(0.0, self.at)
        }
    }

    #[must_use]
    pub fn direction(&self) -> Point {
        Point::from_angle(self.angle().to_radians())
    }

    /// The point of the guide nearest `p`.
    #[must_use]
    pub fn nearest(&self, p: Point) -> Point {
        let (anchor, d) = (self.anchor(), self.direction());
        anchor + d * (p - anchor).dot(d)
    }

    #[must_use]
    pub fn distance(&self, p: Point) -> f64 {
        self.nearest(p).distance(p)
    }

    /// The guide moved to pass through `p`, keeping its direction.
    #[must_use]
    pub fn through(self, p: Point) -> Self {
        let d = self.direction();
        let at = if self.vertical {
            if d.y.abs() < 1e-12 {
                p.x
            } else {
                p.x - p.y * d.x / d.y
            }
        } else if d.x.abs() < 1e-12 {
            p.y
        } else {
            p.y - p.x * d.y / d.x
        };
        Self { at, ..self }
    }

    /// A guide parallel to this one, `offset` to its left (across its direction).
    #[must_use]
    pub fn parallel(self, offset: f64) -> Self {
        let left = self.direction().perp();
        let anchor = self.anchor();
        self.through(anchor + left * offset)
    }
}
