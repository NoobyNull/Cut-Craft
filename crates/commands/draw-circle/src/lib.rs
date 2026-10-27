//! Draw Circle.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::Path;
use cut_craft_core::params::Xy;
use cut_craft_core::{Change, Command, CommandError, Drag, Interactive, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct DrawCircle;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    #[schemars(title = "Center Point", extend("x-icon" = "drawing_forms/circle_center.bmp"))]
    pub center: Xy,
    #[schemars(extend("x-unit" = "mm", "x-icon" = "drawing_forms/circle_radius.bmp"), range(min = 0))]
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
            icon: "drawing/create_vectors/draw_circle.svg",
            shortcut: "",
        }
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
