//! Draw Star: alternating outer and inner vertices, first point straight up (before rotation).

use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Path, Point, polar};
use cut_craft_core::params::Xy;
use cut_craft_core::{Change, Command, CommandError, Drag, Interactive, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct DrawStar;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    #[schemars(title = "Center Point", extend("x-icon" = "drawing_forms/star_center.png"))]
    pub center: Xy,
    /// Centre to point tip
    #[schemars(extend("x-unit" = "mm", "x-icon" = "drawing_forms/star_outer_radius.png"), range(min = 0))]
    pub outer_radius: f64,
    /// Centre to the notch between points
    #[schemars(extend("x-unit" = "mm", "x-icon" = "drawing_forms/star_inner_radius.png"), range(min = 0))]
    pub inner_radius: f64,
    /// How many points the star has
    #[schemars(range(min = 3, max = 100))]
    pub points: u32,
    /// Turns the star counter-clockwise by this angle; at 0 a point faces straight up
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
            icon: "drawing/create_vectors/draw_star.png",
            shortcut: "",
        }
    }

    /// A star, wherever, however big and however turned.
    fn params_from(path: &Path, _: &Params) -> Option<Params> {
        let corners = path.corners()?;
        let points = u32::try_from(corners.len() / 2).ok()?;
        if corners.len() % 2 != 0 || !(3..=100).contains(&points) {
            return None;
        }
        let (sum, n) = corners
            .iter()
            .fold((Point::ZERO, 0.0), |(sum, n), &q| (sum + q, n + 1.0));
        let center = sum / n;
        let turn = (path.start - center)
            .y
            .atan2((path.start - center).x)
            .to_degrees()
            - 90.0;
        let turn = if turn <= -180.0 { turn + 360.0 } else { turn };
        let p = Params {
            center: Xy::new(center.x, center.y),
            outer_radius: path.start.distance(center),
            inner_radius: corners[1].distance(center),
            points,
            // to the millionth of a degree, so 45° reads back as 45, not 44.999…
            rotation: (turn * 1e6).round() / 1e6,
        };
        (p.inner_radius < p.outer_radius && path.matches(&outline(&p), 1e-6)).then_some(p)
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if p.points < 3 {
            return Err(CommandError::InvalidParams(
                "set points to 3 or more".into(),
            ));
        }
        ensure_positive("outer radius", p.outer_radius)?;
        ensure_positive("inner radius", p.inner_radius)?;
        if p.inner_radius >= p.outer_radius {
            return Err(CommandError::InvalidParams(
                "set inner radius smaller than outer radius".into(),
            ));
        }
        Ok(Change::add_and_select(doc, outline(p)))
    }
}

/// The star `p` draws.
fn outline(p: &Params) -> Path {
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
    Path::polyline(&vertices, true)
}

impl Interactive for DrawStar {
    /// Keeps the inner/outer ratio, so the points stay the same depth as the star grows.
    fn from_drag(drag: Drag, current: &Params) -> Params {
        let outer = drag.start.distance(drag.end);
        let ratio = current.inner_radius / current.outer_radius;
        Params {
            center: Xy::new(drag.start.x, drag.start.y),
            outer_radius: outer,
            inner_radius: outer * ratio,
            ..current.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{drawn_path, rejection, vertices};

    cut_craft_core::contract_tests!(DrawStar);

    #[test]
    fn a_drawn_star_reads_back_and_a_polygon_does_not() {
        let p = Params {
            center: Xy::new(-4.0, 9.5),
            outer_radius: 20.0,
            inner_radius: 8.0,
            points: 7,
            rotation: -15.0,
        };
        let back =
            DrawStar::params_from(&drawn_path::<DrawStar>(&p), &Params::default()).expect("a star");
        assert_eq!(back.points, 7);
        assert!((back.outer_radius - 20.0).abs() < 1e-9);
        assert!((back.inner_radius - 8.0).abs() < 1e-9);
        assert!((back.rotation + 15.0).abs() < 1e-9, "{}", back.rotation);
        assert!((back.center.x + 4.0).abs() < 1e-9 && (back.center.y - 9.5).abs() < 1e-9);
        // a hexagon has an even number of corners, all at one radius: no star
        let hexagon = Path::polyline(
            &(0..6)
                .map(|k| polar(Point::ZERO, 10.0, 90.0 + 60.0 * f64::from(k)))
                .collect::<Vec<_>>(),
            true,
        );
        assert!(DrawStar::params_from(&hexagon, &Params::default()).is_none());
    }

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
            "Set inner radius smaller than outer radius."
        );
    }

    #[test]
    fn rejects_fewer_than_three_points() {
        assert_eq!(
            rejection::<DrawStar>(&Params {
                points: 2,
                ..Params::default()
            }),
            "Set points to 3 or more."
        );
    }

    #[test]
    fn a_drag_sets_the_outer_radius_and_keeps_the_point_depth() {
        let drag = cut_craft_core::Drag {
            start: Point::new(0.0, 0.0),
            end: Point::new(0.0, 60.0),
            from_center: false,
            constrain: false,
        };
        let p = DrawStar::from_drag(drag, &Params::default());
        assert_eq!((p.center, p.outer_radius), (Xy::new(0.0, 0.0), 60.0));
        assert!(
            (p.inner_radius - 24.0).abs() < 1e-9,
            "12/30 of the outer radius"
        );
    }
}
