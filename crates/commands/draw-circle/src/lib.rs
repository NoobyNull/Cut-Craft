//! Draw Circle.

use aspire_core::document::Document;
use aspire_core::geometry::Path;
use aspire_core::params::Xy;
use aspire_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct DrawCircle;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub center: Xy,
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
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

#[cfg(test)]
mod tests {
    use super::*;
    use aspire_core::geometry::{Point, Rect};
    use aspire_core::testing::{drawn_path, rejection};

    aspire_core::contract_tests!(DrawCircle);

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
}
