//! Draw Curve: a smooth curve through a list of points, as Béziers whose tangent at each
//! point runs parallel to its neighbours (a Catmull-Rom spline).

use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Path, Point, Segment};
use cut_craft_core::params::Xy;
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct DrawCurve;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub points: Vec<Xy>,
    /// Join the last point back to the first, smoothly
    pub closed: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            points: vec![
                Xy::new(10.0, 10.0),
                Xy::new(60.0, 60.0),
                Xy::new(110.0, 10.0),
            ],
            closed: false,
        }
    }
}

impl Command for DrawCurve {
    const ID: &'static str = "draw_curve";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Draw Curve",
            group: "Create Vectors",
            icon: "drawing/create_vectors/draw_curve.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let pts: Vec<Point> = p.points.iter().map(|&q| q.into()).collect();
        if pts.len() < 2 || (p.closed && pts.len() < 3) {
            return Err(CommandError::InvalidParams(
                "a curve needs at least 2 points (3 to close)".into(),
            ));
        }
        let n = pts.len();
        let at = |i: isize| {
            if p.closed {
                pts[i.rem_euclid(n.cast_signed()).cast_unsigned()]
            } else {
                pts[i.clamp(0, n.cast_signed() - 1).cast_unsigned()]
            }
        };
        let spans = if p.closed { n } else { n - 1 };
        let segments = (0..spans.cast_signed())
            .map(|i| {
                let (a, b) = (at(i), at(i + 1));
                Segment::Cubic {
                    c1: a + (b - at(i - 1)) / 6.0,
                    c2: b - (at(i + 2) - a) / 6.0,
                    to: b,
                }
            })
            .collect();
        Ok(Change::add_and_select(
            doc,
            Path {
                start: pts[0],
                segments,
                closed: p.closed,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{drawn_path, rejection};

    cut_craft_core::contract_tests!(DrawCurve);

    #[test]
    fn the_curve_passes_through_every_point() {
        let path = drawn_path::<DrawCurve>(&Params::default());
        assert_eq!(path.segments.len(), 2);
        assert_eq!(path.segments[0].end(), Point::new(60.0, 60.0));
        let closed = drawn_path::<DrawCurve>(&Params {
            closed: true,
            ..Params::default()
        });
        assert_eq!(closed.segments.len(), 3);
        assert_eq!(closed.segments[2].end(), Point::new(10.0, 10.0));
        let one = Params {
            points: vec![Xy::new(0.0, 0.0)],
            closed: false,
        };
        assert!(rejection::<DrawCurve>(&one).contains("2 points"));
    }
}
