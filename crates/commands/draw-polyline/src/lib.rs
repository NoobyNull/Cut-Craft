//! Draw Line / Polyline: straight lines through a list of points.

use aspire_core::document::Document;
use aspire_core::geometry::{Path, Point};
use aspire_core::params::Xy;
use aspire_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct DrawPolyline;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub points: Vec<Xy>,
    /// Join the last point back to the first
    pub closed: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            points: vec![Xy::new(10.0, 10.0), Xy::new(110.0, 60.0)],
            closed: false,
        }
    }
}

impl Command for DrawPolyline {
    const ID: &'static str = "draw_polyline";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Draw Line / Polyline",
            group: "Create Vectors",
            icon: "drawing/create_vectors/draw_line_polyline.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if p.points.len() < 2 {
            return Err(CommandError::InvalidParams(
                "a line needs at least 2 points".into(),
            ));
        }
        if p.closed && p.points.len() < 3 {
            return Err(CommandError::InvalidParams(
                "a closed shape needs at least 3 points".into(),
            ));
        }
        let points: Vec<Point> = p.points.iter().map(|&q| q.into()).collect();
        Ok(Change::add_and_select(
            doc,
            Path::polyline(&points, p.closed),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aspire_core::testing::{drawn_path, rejection, vertices};

    aspire_core::contract_tests!(DrawPolyline);

    fn points(xy: &[(f64, f64)]) -> Vec<Xy> {
        xy.iter().map(|&(x, y)| Xy::new(x, y)).collect()
    }

    #[test]
    fn passes_through_every_point_in_order() {
        let p = Params {
            points: points(&[(0.0, 0.0), (10.0, 0.0), (10.0, 5.0)]),
            closed: false,
        };
        let v = vertices(&drawn_path::<DrawPolyline>(&p));
        assert_eq!(
            v,
            [
                Point::new(0.0, 0.0),
                Point::new(10.0, 0.0),
                Point::new(10.0, 5.0)
            ]
        );
    }

    #[test]
    fn closed_returns_to_the_first_point() {
        let p = Params {
            points: points(&[(0.0, 0.0), (10.0, 0.0), (10.0, 5.0)]),
            closed: true,
        };
        let path = drawn_path::<DrawPolyline>(&p);
        assert!(path.closed);
        assert_eq!(vertices(&path).last(), Some(&Point::new(0.0, 0.0)));
    }

    #[test]
    fn rejects_a_single_point() {
        let p = Params {
            points: points(&[(1.0, 1.0)]),
            closed: false,
        };
        assert_eq!(
            rejection::<DrawPolyline>(&p),
            "invalid parameters: a line needs at least 2 points"
        );
    }

    #[test]
    fn rejects_closing_fewer_than_three_points() {
        let p = Params {
            points: points(&[(0.0, 0.0), (1.0, 1.0)]),
            closed: true,
        };
        assert_eq!(
            rejection::<DrawPolyline>(&p),
            "invalid parameters: a closed shape needs at least 3 points"
        );
    }
}
