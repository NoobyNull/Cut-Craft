//! Draw Arc: counter-clockwise from the start angle to the end angle.

use aspire_core::document::Document;
use aspire_core::geometry::{Path, Point, Segment, polar};
use aspire_core::params::Xy;
use aspire_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct DrawArc;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub center: Xy,
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub radius: f64,
    /// Measured counter-clockwise from +X
    #[schemars(extend("x-unit" = "deg"))]
    pub start_angle: f64,
    #[schemars(extend("x-unit" = "deg"))]
    pub end_angle: f64,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            center: Xy::new(60.0, 60.0),
            radius: 30.0,
            start_angle: 0.0,
            end_angle: 90.0,
        }
    }
}

impl Command for DrawArc {
    const ID: &'static str = "draw_arc";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Draw Arc",
            group: "Create Vectors",
            icon: "drawing/create_vectors/draw_arc.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("radius", p.radius)?;
        if (p.end_angle - p.start_angle).rem_euclid(360.0) == 0.0 {
            return Err(CommandError::InvalidParams(
                "start and end angle must differ (use Draw Circle for a full circle)".into(),
            ));
        }
        let center: Point = p.center.into();
        let arc = Path {
            start: polar(center, p.radius, p.start_angle),
            segments: vec![Segment::Arc {
                to: polar(center, p.radius, p.end_angle),
                center,
                ccw: true,
            }],
            closed: false,
        };
        Ok(Change::add_and_select(doc, arc))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aspire_core::testing::{drawn_path, rejection};

    aspire_core::contract_tests!(DrawArc);

    #[test]
    fn runs_counter_clockwise_between_the_angles_and_stays_open() {
        let path = drawn_path::<DrawArc>(&Params::default());
        assert!(!path.closed);
        assert!(path.start.distance(Point::new(90.0, 60.0)) < 1e-9);
        match path.segments[..] {
            [
                Segment::Arc {
                    to,
                    center,
                    ccw: true,
                },
            ] => {
                assert!(to.distance(Point::new(60.0, 90.0)) < 1e-9);
                assert_eq!(center, Point::new(60.0, 60.0));
            }
            _ => panic!("expected one counter-clockwise arc: {path:?}"),
        }
    }

    #[test]
    fn rejects_a_full_turn() {
        assert_eq!(
            rejection::<DrawArc>(&Params {
                end_angle: 360.0,
                ..Params::default()
            }),
            "invalid parameters: start and end angle must differ (use Draw Circle for a full circle)"
        );
    }
}
