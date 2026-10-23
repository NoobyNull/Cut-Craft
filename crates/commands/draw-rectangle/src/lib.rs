//! Draw Rectangle: square, rounded or notched corners.

use aspire_core::document::Document;
use aspire_core::geometry::{Path, Point, Segment};
use aspire_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct DrawRectangle;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Corners {
    #[default]
    Square,
    /// Rounded outwards (external radius)
    Rounded,
    /// Cut inwards (internal radius)
    Notched,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// Left edge
    #[schemars(extend("x-unit" = "mm"))]
    pub x: f64,
    /// Bottom edge
    #[schemars(extend("x-unit" = "mm"))]
    pub y: f64,
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub width: f64,
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub height: f64,
    pub corners: Corners,
    /// Corner radius (not used for square corners)
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub radius: f64,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            x: 10.0,
            y: 10.0,
            width: 100.0,
            height: 50.0,
            corners: Corners::Square,
            radius: 5.0,
        }
    }
}

impl Command for DrawRectangle {
    const ID: &'static str = "draw_rectangle";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Draw Rectangle",
            group: "Create Vectors",
            icon: "drawing/create_vectors/draw_rectangle.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("width", p.width)?;
        ensure_positive("height", p.height)?;
        if p.corners != Corners::Square {
            ensure_positive("radius", p.radius)?;
            let max = p.width.min(p.height) / 2.0;
            if p.radius > max {
                return Err(CommandError::InvalidParams(format!(
                    "radius must be at most half the shorter side ({max})"
                )));
            }
        }
        Ok(Change::add_and_select(doc, outline(p)))
    }
}

fn outline(p: &Params) -> Path {
    let (x0, y0, x1, y1) = (p.x, p.y, p.x + p.width, p.y + p.height);
    if p.corners == Corners::Square {
        return Path::polyline(
            &[
                Point::new(x0, y0),
                Point::new(x1, y0),
                Point::new(x1, y1),
                Point::new(x0, y1),
            ],
            true,
        );
    }
    let r = p.radius;
    // Corners counter-clockwise from bottom-right, each with the direction of the edge running into it.
    let corners = [
        (Point::new(x1, y0), Point::X),
        (Point::new(x1, y1), Point::Y),
        (Point::new(x0, y1), -Point::X),
        (Point::new(x0, y0), -Point::Y),
    ];
    let rounded = p.corners == Corners::Rounded;
    let mut segments = Vec::with_capacity(8);
    for (i, &(corner, into)) in corners.iter().enumerate() {
        let out = corners[(i + 1) % 4].1;
        let center = if rounded {
            corner - into * r + out * r
        } else {
            corner
        };
        segments.push(Segment::Line {
            to: corner - into * r,
        });
        segments.push(Segment::Arc {
            to: corner + out * r,
            center,
            ccw: rounded,
        });
    }
    Path {
        start: Point::new(x0 + r, y0),
        segments,
        closed: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aspire_core::geometry::Rect;
    use aspire_core::testing::{drawn_path, rejection};

    aspire_core::contract_tests!(DrawRectangle);

    fn params(corners: Corners) -> Params {
        Params {
            corners,
            ..Params::default()
        }
    }

    #[test]
    fn square_corners_give_a_closed_four_line_outline() {
        let path = drawn_path::<DrawRectangle>(&Params::default());
        assert!(path.closed);
        assert_eq!(path.segments.len(), 4);
        assert!(
            path.segments
                .iter()
                .all(|s| matches!(s, Segment::Line { .. }))
        );
        assert_eq!(
            path.bounds(),
            Rect::new(Point::new(10.0, 10.0), Point::new(110.0, 60.0))
        );
    }

    #[test]
    fn rounded_corners_keep_the_bounds_with_four_outward_arcs() {
        let path = drawn_path::<DrawRectangle>(&params(Corners::Rounded));
        assert_eq!(
            path.bounds(),
            Rect::new(Point::new(10.0, 10.0), Point::new(110.0, 60.0))
        );
        let arcs: Vec<_> = path
            .segments
            .iter()
            .filter(|s| matches!(s, Segment::Arc { ccw: true, .. }))
            .collect();
        assert_eq!(arcs.len(), 4);
    }

    #[test]
    fn notched_corners_are_arcs_centred_on_the_corners() {
        let path = drawn_path::<DrawRectangle>(&params(Corners::Notched));
        let centres: Vec<Point> = path
            .segments
            .iter()
            .filter_map(|s| match *s {
                Segment::Arc {
                    center, ccw: false, ..
                } => Some(center),
                _ => None,
            })
            .collect();
        assert_eq!(
            centres,
            [
                Point::new(110.0, 10.0),
                Point::new(110.0, 60.0),
                Point::new(10.0, 60.0),
                Point::new(10.0, 10.0)
            ]
        );
    }

    #[test]
    fn rejects_a_zero_width() {
        assert_eq!(
            rejection::<DrawRectangle>(&Params {
                width: 0.0,
                ..Params::default()
            }),
            "invalid parameters: width must be greater than 0"
        );
    }

    #[test]
    fn rejects_a_radius_wider_than_half_the_short_side() {
        let p = Params {
            corners: Corners::Rounded,
            radius: 26.0,
            ..Params::default()
        };
        assert_eq!(
            rejection::<DrawRectangle>(&p),
            "invalid parameters: radius must be at most half the shorter side (25)"
        );
    }

    #[test]
    fn square_corners_ignore_the_radius() {
        drawn_path::<DrawRectangle>(&Params {
            radius: 1000.0,
            ..Params::default()
        });
    }
}
