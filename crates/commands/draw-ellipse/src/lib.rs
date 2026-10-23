//! Draw Ellipse.

use aspire_core::document::Document;
use aspire_core::geometry::Path;
use aspire_core::glam::{DAffine2, DVec2};
use aspire_core::params::Xy;
use aspire_core::{Change, Command, CommandError, Drag, Interactive, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct DrawEllipse;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub center: Xy,
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub width: f64,
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub height: f64,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            center: Xy::new(60.0, 60.0),
            width: 80.0,
            height: 40.0,
        }
    }
}

impl Command for DrawEllipse {
    const ID: &'static str = "draw_ellipse";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Draw Ellipse",
            group: "Create Vectors",
            icon: "drawing/create_vectors/draw_ellipse.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("width", p.width)?;
        ensure_positive("height", p.height)?;
        let fit = DAffine2::from_translation(p.center.into())
            * DAffine2::from_scale(DVec2::new(p.width, p.height) / 2.0);
        Ok(Change::add_and_select(
            doc,
            Path::circle(DVec2::ZERO, 1.0).transformed(&fit),
        ))
    }
}

impl Interactive for DrawEllipse {
    fn from_drag(drag: Drag, _: &Params) -> Params {
        let r = drag.rect();
        Params {
            center: Xy::new(r.center().x, r.center().y),
            width: r.size().x,
            height: r.size().y,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aspire_core::geometry::{Point, Rect};
    use aspire_core::testing::{drawn_path, rejection};

    aspire_core::contract_tests!(DrawEllipse);

    #[test]
    fn spans_exactly_its_width_and_height() {
        let b = drawn_path::<DrawEllipse>(&Params::default()).bounds();
        let want = Rect::new(Point::new(20.0, 40.0), Point::new(100.0, 80.0));
        assert!(
            b.min.distance(want.min) < 1e-9 && b.max.distance(want.max) < 1e-9,
            "{b:?}"
        );
    }

    #[test]
    fn equal_width_and_height_stay_a_true_circle() {
        let path = drawn_path::<DrawEllipse>(&Params {
            width: 50.0,
            height: 50.0,
            ..Params::default()
        });
        assert!(
            path.segments
                .iter()
                .all(|s| matches!(s, aspire_core::geometry::Segment::Arc { .. }))
        );
    }

    #[test]
    fn rejects_a_zero_height() {
        assert_eq!(
            rejection::<DrawEllipse>(&Params {
                height: 0.0,
                ..Params::default()
            }),
            "invalid parameters: height must be greater than 0"
        );
    }

    #[test]
    fn a_drag_box_sets_the_centre_and_size() {
        let drag = aspire_core::Drag {
            start: Point::new(0.0, 0.0),
            end: Point::new(10.0, 4.0),
            from_center: false,
            constrain: false,
        };
        let p = DrawEllipse::from_drag(drag, &Params::default());
        assert_eq!(
            (p.center, p.width, p.height),
            (Xy::new(5.0, 2.0), 10.0, 4.0)
        );
    }
}
