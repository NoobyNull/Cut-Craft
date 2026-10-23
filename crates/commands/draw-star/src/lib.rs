//! Draw Star: alternating outer and inner vertices, first point straight up (before rotation).

use aspire_core::document::Document;
use aspire_core::geometry::{Path, polar};
use aspire_core::params::Xy;
use aspire_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct DrawStar;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub center: Xy,
    /// Centre to point tip
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub outer_radius: f64,
    /// Centre to the notch between points
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub inner_radius: f64,
    #[schemars(range(min = 3, max = 100))]
    pub points: u32,
    /// Counter-clockwise
    #[schemars(extend("x-unit" = "deg"))]
    pub rotation: f64,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            center: Xy::new(60.0, 60.0),
            outer_radius: 30.0,
            inner_radius: 12.0,
            points: 5,
            rotation: 0.0,
        }
    }
}

impl Command for DrawStar {
    const ID: &'static str = "draw_star";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Draw Star",
            group: "Create Vectors",
            icon: "drawing/create_vectors/draw_star.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if p.points < 3 {
            return Err(CommandError::InvalidParams(
                "a star needs at least 3 points".into(),
            ));
        }
        ensure_positive("outer radius", p.outer_radius)?;
        ensure_positive("inner radius", p.inner_radius)?;
        if p.inner_radius >= p.outer_radius {
            return Err(CommandError::InvalidParams(
                "inner radius must be smaller than outer radius".into(),
            ));
        }
        let step = 180.0 / f64::from(p.points);
        let vertices: Vec<_> = (0..2 * p.points)
            .map(|k| {
                let r = if k % 2 == 0 {
                    p.outer_radius
                } else {
                    p.inner_radius
                };
                polar(p.center.into(), r, 90.0 + p.rotation + step * f64::from(k))
            })
            .collect();
        Ok(Change::add_and_select(doc, Path::polyline(&vertices, true)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aspire_core::geometry::Point;
    use aspire_core::testing::{drawn_path, rejection, vertices};

    aspire_core::contract_tests!(DrawStar);

    #[test]
    fn vertices_alternate_between_the_two_radii() {
        let path = drawn_path::<DrawStar>(&Params::default());
        let v = vertices(&path);
        assert!(path.closed);
        assert_eq!(
            v.len(),
            11,
            "10 vertices plus the closing return to the start"
        );
        let c = Point::new(60.0, 60.0);
        for (i, q) in v[..10].iter().enumerate() {
            let want = if i % 2 == 0 { 30.0 } else { 12.0 };
            assert!((q.distance(c) - want).abs() < 1e-9, "vertex {i}");
        }
        assert!(v[0].distance(Point::new(60.0, 90.0)) < 1e-9);
    }

    #[test]
    fn rejects_an_inner_radius_not_inside_the_outer() {
        assert_eq!(
            rejection::<DrawStar>(&Params {
                inner_radius: 30.0,
                ..Params::default()
            }),
            "invalid parameters: inner radius must be smaller than outer radius"
        );
    }

    #[test]
    fn rejects_fewer_than_three_points() {
        assert_eq!(
            rejection::<DrawStar>(&Params {
                points: 2,
                ..Params::default()
            }),
            "invalid parameters: a star needs at least 3 points"
        );
    }
}
