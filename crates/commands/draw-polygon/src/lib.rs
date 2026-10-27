//! Draw Polygon: a regular polygon with its first vertex straight up (before rotation).

use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Path, polar};
use cut_craft_core::params::Xy;
use cut_craft_core::{Change, Command, CommandError, Drag, Interactive, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct DrawPolygon;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    #[schemars(title = "Center Point", extend("x-icon" = "drawing_forms/polygon_center.png"))]
    pub center: Xy,
    /// Centre to vertex
    #[schemars(extend("x-unit" = "mm", "x-icon" = "drawing_forms/polygon_radius.png"), range(min = 0))]
    pub radius: f64,
    #[schemars(range(min = 3, max = 100))]
    pub sides: u32,
    /// Counter-clockwise
    #[schemars(extend("x-unit" = "deg"))]
    pub rotation: f64,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            center: Xy::new(60.0, 60.0),
            radius: 30.0,
            sides: 6,
            rotation: 0.0,
        }
    }
}

impl Command for DrawPolygon {
    const ID: &'static str = "draw_polygon";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Draw Polygon",
            group: "Create Vectors",
            icon: "drawing/create_vectors/draw_polygon.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if p.sides < 3 {
            return Err(CommandError::InvalidParams(
                "a polygon needs at least 3 sides".into(),
            ));
        }
        ensure_positive("radius", p.radius)?;
        let step = 360.0 / f64::from(p.sides);
        let points: Vec<_> = (0..p.sides)
            .map(|k| {
                polar(
                    p.center.into(),
                    p.radius,
                    90.0 + p.rotation + step * f64::from(k),
                )
            })
            .collect();
        Ok(Change::add_and_select(doc, Path::polyline(&points, true)))
    }
}

impl Interactive for DrawPolygon {
    fn from_drag(drag: Drag, current: &Params) -> Params {
        Params {
            center: Xy::new(drag.start.x, drag.start.y),
            radius: drag.start.distance(drag.end),
            ..current.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Point;
    use cut_craft_core::testing::{drawn_path, rejection, vertices};

    cut_craft_core::contract_tests!(DrawPolygon);

    #[test]
    fn every_vertex_is_on_the_radius_and_the_first_points_up() {
        let path = drawn_path::<DrawPolygon>(&Params::default());
        let v = vertices(&path);
        assert!(path.closed);
        assert_eq!(
            v.len(),
            7,
            "6 vertices plus the closing return to the start"
        );
        assert!(
            v.iter()
                .all(|q| (q.distance(Point::new(60.0, 60.0)) - 30.0).abs() < 1e-9)
        );
        assert!(v[0].distance(Point::new(60.0, 90.0)) < 1e-9);
    }

    #[test]
    fn rotation_turns_the_first_vertex_counter_clockwise() {
        let v = vertices(&drawn_path::<DrawPolygon>(&Params {
            rotation: 90.0,
            ..Params::default()
        }));
        assert!(v[0].distance(Point::new(30.0, 60.0)) < 1e-9, "{:?}", v[0]);
    }

    #[test]
    fn rejects_fewer_than_three_sides() {
        assert_eq!(
            rejection::<DrawPolygon>(&Params {
                sides: 2,
                ..Params::default()
            }),
            "invalid parameters: a polygon needs at least 3 sides"
        );
    }

    #[test]
    fn a_drag_sets_the_centre_and_radius_and_keeps_the_sides() {
        let drag = cut_craft_core::Drag {
            start: Point::new(5.0, 5.0),
            end: Point::new(8.0, 9.0),
            from_center: false,
            constrain: false,
        };
        let p = DrawPolygon::from_drag(
            drag,
            &Params {
                sides: 8,
                ..Params::default()
            },
        );
        assert_eq!((p.center, p.radius, p.sides), (Xy::new(5.0, 5.0), 5.0, 8));
    }
}
