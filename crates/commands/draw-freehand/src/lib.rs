//! Draw Freehand: the pointer's path while dragging, smoothed into Béziers.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::Point;
use cut_craft_core::params::Xy;
use cut_craft_core::trace::{self, Fit};
use cut_craft_core::{Change, Command, CommandError, Drag, Interactive, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct DrawFreehand;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// How far the smoothed curve may stray from the drawn path
    #[schemars(title = "Smoothing", extend("x-unit" = "mm"), range(min = 0))]
    pub tolerance: f64,
    /// The drawn path; drag in the 2D view to draw it
    #[schemars(skip)]
    #[serde(default)]
    pub points: Vec<Xy>,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            // ponytail: 0.5 mm [inf]; Aspire's freehand smoothing is not described
            tolerance: 0.5,
            points: vec![
                Xy::new(10.0, 10.0),
                Xy::new(20.0, 15.0),
                Xy::new(30.0, 10.0),
            ],
        }
    }
}

impl Command for DrawFreehand {
    const ID: &'static str = "draw_freehand";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Draw Freehand",
            group: "Create Vectors",
            icon: "drawing/create_vectors/draw_freehand.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("smoothing", p.tolerance)?;
        let mut pts: Vec<Point> = p.points.iter().map(|&q| q.into()).collect();
        pts.dedup_by(|a, b| a.distance(*b) < 1e-9);
        if pts.len() < 2 {
            return Err(CommandError::InvalidParams("drag to draw".into()));
        }
        let path = trace::fit(&pts, false, Fit::Beziers, p.tolerance, 60_f64.to_radians());
        Ok(Change::add_and_select(doc, path))
    }
}

impl Interactive for DrawFreehand {
    /// The app fills in the drawn points; a drag with none is a straight stroke.
    fn from_drag(drag: Drag, current: &Params) -> Params {
        let mut p = current.clone();
        if p.points.len() < 2 {
            p.points = vec![
                Xy::new(drag.start.x, drag.start.y),
                Xy::new(drag.end.x, drag.end.y),
            ];
        }
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{drawn_path, rejection};

    cut_craft_core::contract_tests!(DrawFreehand);

    #[test]
    fn a_wobbly_stroke_becomes_a_smooth_curve_near_it() {
        let points: Vec<Xy> = (0..=100)
            .map(|i| {
                let x = f64::from(i);
                Xy::new(
                    x,
                    10.0 * (x / 20.0).sin() + if i % 2 == 0 { 0.1 } else { -0.1 },
                )
            })
            .collect();
        let p = Params {
            points: points.clone(),
            ..Params::default()
        };
        let path = drawn_path::<DrawFreehand>(&p);
        assert!(path.segments.len() < 20, "{} spans", path.segments.len());
        for q in path.flatten(0.01) {
            let near = points
                .iter()
                .map(|&s| Point::from(s).distance(q))
                .fold(f64::MAX, f64::min);
            assert!(near < 1.5, "{q} strays {near}");
        }
        let one = Params {
            points: vec![Xy::new(1.0, 1.0)],
            ..Params::default()
        };
        assert!(rejection::<DrawFreehand>(&one).contains("drag"));
    }
}
