//! Draw Polygon: a regular polygon with its first vertex straight up (before rotation).

use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Path, Point, polar};
use cut_craft_core::glam::DVec2;
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
    /// How many straight sides it has: 3 for a triangle, 6 for a hexagon
    #[schemars(range(min = 3, max = 100))]
    pub sides: u32,
    /// Turns the shape counter-clockwise by this angle; at 0 a corner points straight up
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

    /// A regular polygon, wherever, however big and however turned.
    fn params_from(path: &Path, _: &Params) -> Option<Params> {
        let corners = path.corners()?;
        let sides = u32::try_from(corners.len())
            .ok()
            .filter(|n| (3..=100).contains(n))?;
        let center = mean(&corners);
        let p = Params {
            center: Xy::new(center.x, center.y),
            radius: path.start.distance(center),
            sides,
            rotation: turn_of(path.start - center),
        };
        path.matches(&outline(&p), TOLERANCE).then_some(p)
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if p.sides < 3 {
            return Err(CommandError::InvalidParams("set sides to 3 or more".into()));
        }
        ensure_positive("radius", p.radius)?;
        Ok(Change::add_and_select(doc, outline(p)))
    }
}

/// How near a shape must be to the one the settings draw to be read as it, mm.
const TOLERANCE: f64 = 1e-6;

/// The polygon `p` draws.
fn outline(p: &Params) -> Path {
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
    Path::polyline(&points, true)
}

fn mean(points: &[Point]) -> Point {
    let (sum, n) = points
        .iter()
        .fold((Point::ZERO, 0.0), |(sum, n), &q| (sum + q, n + 1.0));
    sum / n
}

/// The turn, degrees counter-clockwise from straight up, of a corner `at` from the centre:
/// to the millionth of a degree, so a polygon turned by 45° reads back as 45, not 44.999….
fn turn_of(at: DVec2) -> f64 {
    let turn = at.y.atan2(at.x).to_degrees() - 90.0;
    let turn = if turn <= -180.0 { turn + 360.0 } else { turn };
    (turn * 1e6).round() / 1e6
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
    fn a_drawn_polygon_reads_back_moved_and_turned() {
        let p = Params {
            center: Xy::new(12.5, -3.0),
            radius: 7.0,
            sides: 5,
            rotation: 33.0,
        };
        let path = drawn_path::<DrawPolygon>(&p);
        let back = DrawPolygon::params_from(&path, &Params::default()).expect("a polygon");
        assert!((back.radius - 7.0).abs() < 1e-9 && back.sides == 5);
        assert!((back.rotation - 33.0).abs() < 1e-9, "{}", back.rotation);
        assert!((back.center.x - 12.5).abs() < 1e-9 && (back.center.y + 3.0).abs() < 1e-9);
        // moved elsewhere, it still reads back
        let moved = path.transformed(&cut_craft_core::glam::DAffine2::from_translation(
            DVec2::new(100.0, 40.0),
        ));
        let back = DrawPolygon::params_from(&moved, &Params::default()).expect("moved");
        assert!((back.center.x - 112.5).abs() < 1e-9);
    }

    #[test]
    fn shapes_that_are_not_regular_polygons_are_not_read_as_one() {
        let squashed = Path::polyline(
            &[
                Point::new(0.0, 0.0),
                Point::new(10.0, 0.0),
                Point::new(10.0, 3.0),
                Point::new(0.0, 3.0),
            ],
            true,
        );
        assert!(DrawPolygon::params_from(&squashed, &Params::default()).is_none());
        let open = Path::polyline(&[Point::new(0.0, 0.0), Point::new(1.0, 0.0)], false);
        assert!(DrawPolygon::params_from(&open, &Params::default()).is_none());
        let circle = Path::circle(Point::new(0.0, 0.0), 5.0);
        assert!(DrawPolygon::params_from(&circle, &Params::default()).is_none());
    }

    #[test]
    fn rejects_fewer_than_three_sides() {
        assert_eq!(
            rejection::<DrawPolygon>(&Params {
                sides: 2,
                ..Params::default()
            }),
            "Set sides to 3 or more."
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
