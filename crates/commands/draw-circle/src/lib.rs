//! Draw Circle.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Path, Segment};
use cut_craft_core::params::Xy;
use cut_craft_core::{Change, Command, CommandError, Drag, Interactive, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct DrawCircle;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    #[schemars(title = "Center Point", extend("x-icon" = "drawing_forms/circle_center.png"))]
    pub center: Xy,
    #[schemars(extend("x-unit" = "mm", "x-icon" = "drawing_forms/circle_radius.png"), range(min = 0))]
    pub radius: f64,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            center: Xy::new(60.0, 60.0),
            radius: 25.0,
        }
    }
}

impl Command for DrawCircle {
    const ID: &'static str = "draw_circle";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Draw Circle",
            group: "Create Vectors",
            icon: "drawing/create_vectors/draw_circle.png",
            shortcut: "",
        }
    }

    /// A circle, wherever and however big.
    fn params_from(path: &Path, _: &Params) -> Option<Params> {
        let Some(Segment::Arc { center, .. }) = path.segments.first() else {
            return None;
        };
        let p = Params {
            center: Xy::new(center.x, center.y),
            radius: path.start.distance(*center),
        };
        path.matches(&Path::circle(*center, p.radius), 1e-6)
            .then_some(p)
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("radius", p.radius)?;
        Ok(Change::add_and_select(
            doc,
            Path::circle(p.center.into(), p.radius),
        ))
    }
}

impl Interactive for DrawCircle {
    fn from_drag(drag: Drag, _: &Params) -> Params {
        Params {
            center: Xy::new(drag.start.x, drag.start.y),
            radius: drag.start.distance(drag.end),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_circle_is_read_back_and_other_shapes_are_not() {
        let back =
            DrawCircle::params_from(&Path::circle(Point::new(3.0, 4.0), 7.5), &Params::default())
                .unwrap();
        assert_eq!(back.center, Xy::new(3.0, 4.0));
        assert!((back.radius - 7.5).abs() < 1e-9);
        let square = Path::polyline(
            &[Point::ZERO, Point::new(1.0, 0.0), Point::new(1.0, 1.0)],
            true,
        );
        assert!(DrawCircle::params_from(&square, &Params::default()).is_none());
    }
    use cut_craft_core::geometry::{Point, Rect};
    use cut_craft_core::testing::{drawn_path, rejection};

    cut_craft_core::contract_tests!(DrawCircle);

    #[test]
    fn draws_a_closed_circle_of_the_given_radius() {
        let path = drawn_path::<DrawCircle>(&Params::default());
        assert!(path.closed);
        assert_eq!(
            path.bounds(),
            Rect::new(Point::new(35.0, 35.0), Point::new(85.0, 85.0))
        );
    }

    #[test]
    fn rejects_a_zero_radius() {
        assert_eq!(
            rejection::<DrawCircle>(&Params {
                radius: 0.0,
                ..Params::default()
            }),
            "invalid parameters: radius must be greater than 0"
        );
    }

    #[test]
    fn a_drag_goes_from_the_centre_out_to_the_radius() {
        let drag = cut_craft_core::Drag {
            start: Point::new(5.0, 5.0),
            end: Point::new(8.0, 9.0),
            from_center: false,
            constrain: false,
        };
        let p = DrawCircle::from_drag(drag, &Params::default());
        assert_eq!((p.center, p.radius), (Xy::new(5.0, 5.0), 5.0));
    }
}
