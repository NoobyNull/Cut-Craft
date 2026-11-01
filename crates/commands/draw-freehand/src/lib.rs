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
    /// Small movements this close to the path's end do not move it: the pointer has to
    /// reach the edge of the circle drawn there to pull the path on
    #[serde(default)]
    #[schemars(title = "Stabilize", extend("x-unit" = "mm"), range(min = 0))]
    pub stabilize: f64,
    /// Stabilized, the path ends where the pointer was let go, not at the circle's centre
    #[serde(default)]
    #[schemars(title = "End vector at cursor")]
    pub end_at_cursor: bool,
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
            stabilize: 0.0,
            end_at_cursor: false,
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
        let drawn: Vec<Point> = p.points.iter().map(|&q| q.into()).collect();
        let mut pts = stabilized(&drawn, p.stabilize, p.end_at_cursor);
        pts.dedup_by(|a, b| a.distance(*b) < 1e-9);
        if pts.len() < 2 {
            return Err(CommandError::InvalidParams("drag to draw".into()));
        }
        let path = trace::fit(&pts, false, Fit::Beziers, p.tolerance, 60_f64.to_radians());
        Ok(Change::add_and_select(doc, path))
    }
}

/// The path's points as the pointer's `points` pull it along: its end moves only when the
/// pointer is more than `radius` away, and then only to `radius` short of the pointer, so the
/// jitters inside that circle are lost (`radius` 0: the points as they are). `end_at_cursor`
/// ends the path where the pointer stopped.
#[must_use]
pub fn stabilized(points: &[Point], radius: f64, end_at_cursor: bool) -> Vec<Point> {
    let Some((&first, rest)) = points.split_first() else {
        return vec![];
    };
    if radius <= 0.0 {
        return points.to_vec();
    }
    let mut end = first;
    let mut out = vec![first];
    for &pointer in rest {
        let pull = pointer - end;
        let dist = pull.length();
        if dist > radius {
            end += pull * ((dist - radius) / dist);
            out.push(end);
        }
    }
    if end_at_cursor
        && let Some(&last) = rest.last()
        && last != end
    {
        out.push(last);
    }
    out
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

    #[test]
    fn stabilizing_loses_the_jitters_inside_the_circle_and_the_end_can_stay_at_the_cursor() {
        let p = Point::new;
        let jitter = [
            p(0.0, 0.0),
            p(0.3, 0.4),
            p(0.0, 0.0),
            p(5.0, 0.0),
            p(5.0, 0.5),
            p(10.0, 0.0),
        ];
        assert_eq!(stabilized(&jitter, 0.0, false), jitter);
        let steady = stabilized(&jitter, 1.0, false);
        // the wiggle inside the circle leaves nothing; each pull stops a radius short
        assert_eq!(steady.len(), 4, "{steady:?}");
        assert!((steady[1] - p(4.0, 0.0)).length() < 1e-9, "{steady:?}");
        assert!((steady[3] - p(9.0, 0.0)).length() < 0.05, "{steady:?}");
        let ended = stabilized(&jitter, 1.0, true);
        assert_eq!(ended.last(), Some(&p(10.0, 0.0)), "ends at the cursor");
        assert_eq!(ended.len(), 5);
        // a stabilized stroke draws a path along the steadied points
        let path = drawn_path::<DrawFreehand>(&Params {
            stabilize: 1.0,
            points: jitter.iter().map(|q| Xy::new(q.x, q.y)).collect(),
            ..Params::default()
        });
        let far = path
            .flatten(0.01)
            .iter()
            .map(|q| q.y.abs())
            .fold(0.0, f64::max);
        assert!(far < 0.3, "no wiggle survives: {far}");
    }
}
