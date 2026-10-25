//! Snapping: points the pointer jumps to while drawing or moving nodes, as Aspire's Snapping
//! Options (F4) choose them: geometry (span ends and middles, arc centres, object centres),
//! the job's centre and corners, and a grid.

use crate::document::Document;
use crate::geometry::{Point, Segment};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the dialog's own check boxes"
)]
pub struct SnapOptions {
    pub snap_to_grid: bool,
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub grid_spacing: f64,
    #[schemars(title = "Snap to job center and corners")]
    pub job_center_and_corners: bool,
    /// Snap to points on the vectors (the next four)
    pub geometry_snapping: bool,
    pub object_centers: bool,
    pub span_end_points: bool,
    pub span_mid_points: bool,
    pub arc_centers: bool,
    /// How near, in screen pixels, the pointer must come to a point to snap to it
    #[schemars(range(min = 1, max = 50))]
    pub snapping_radius: f64,
}

impl Default for SnapOptions {
    fn default() -> Self {
        Self {
            snap_to_grid: false,
            grid_spacing: 10.0,
            job_center_and_corners: true,
            geometry_snapping: true,
            object_centers: true,
            span_end_points: true,
            span_mid_points: true,
            arc_centers: true,
            // ponytail: Aspire's slider runs Small to Large; 8 px is our middle [inf]
            snapping_radius: 8.0,
        }
    }
}

impl SnapOptions {
    /// Where `at` snaps to: the nearest snap point within `reach` (job mm), else the grid
    /// when it is on, else `at` itself.
    pub fn snap(&self, doc: &Document, at: Point, reach: f64) -> Point {
        let near = self
            .points(doc)
            .into_iter()
            .map(|q| (q, q.distance(at)))
            .filter(|(_, d)| *d <= reach)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        match near {
            Some((q, _)) => q,
            None if self.snap_to_grid && self.grid_spacing > 0.0 => {
                let g = self.grid_spacing;
                let origin = doc.job.area().min;
                origin + ((at - origin) / g).round() * g
            }
            None => at,
        }
    }

    fn points(&self, doc: &Document) -> Vec<Point> {
        let mut out = Vec::new();
        if self.job_center_and_corners {
            let a = doc.job.area();
            out.extend([
                a.min,
                a.max,
                Point::new(a.min.x, a.max.y),
                Point::new(a.max.x, a.min.y),
                a.center(),
            ]);
        }
        if !self.geometry_snapping {
            return out;
        }
        for (id, path) in &doc.vectors {
            if !doc.shown(*id) {
                continue;
            }
            if self.object_centers {
                out.push(path.bounds().center());
            }
            let mut from = path.start;
            if self.span_end_points {
                out.push(from);
            }
            for s in &path.segments {
                let to = s.end();
                if self.span_end_points {
                    out.push(to);
                }
                if self.span_mid_points {
                    out.push(match *s {
                        Segment::Arc { center, ccw, .. } => {
                            let half = crate::geometry::arc_sweep(from, to, center, ccw) / 2.0;
                            center + (from - center).rotate(glam::DVec2::from_angle(half))
                        }
                        Segment::Cubic { c1, c2, .. } => (from + 3.0 * c1 + 3.0 * c2 + to) / 8.0,
                        Segment::Line { .. } => from.lerp(to, 0.5),
                    });
                }
                if let (true, Segment::Arc { center, .. }) = (self.arc_centers, s) {
                    out.push(*center);
                }
                from = to;
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{document_with, square};

    #[test]
    fn points_snap_to_corners_middles_centres_and_the_grid() {
        let doc = document_with(vec![square(10.0, 10.0, 20.0)]);
        let s = SnapOptions::default();
        assert_eq!(
            s.snap(&doc, Point::new(29.6, 30.3), 1.0),
            Point::new(30.0, 30.0),
            "a corner"
        );
        assert_eq!(
            s.snap(&doc, Point::new(20.4, 10.2), 1.0),
            Point::new(20.0, 10.0),
            "a middle"
        );
        assert_eq!(
            s.snap(&doc, Point::new(19.5, 20.5), 1.0),
            Point::new(20.0, 20.0),
            "the centre"
        );
        assert_eq!(
            s.snap(&doc, Point::new(57.0, 57.0), 1.0),
            Point::new(57.0, 57.0),
            "nothing near"
        );
        let grid = SnapOptions {
            snap_to_grid: true,
            ..SnapOptions::default()
        };
        assert_eq!(
            grid.snap(&doc, Point::new(57.0, 53.0), 1.0),
            Point::new(60.0, 50.0)
        );
        let none = SnapOptions {
            geometry_snapping: false,
            job_center_and_corners: false,
            ..SnapOptions::default()
        };
        assert_eq!(
            none.snap(&doc, Point::new(29.6, 30.3), 1.0),
            Point::new(29.6, 30.3)
        );
    }

    #[test]
    fn an_arc_snaps_to_its_middle_and_centre() {
        let doc = document_with(vec![crate::geometry::Path::circle(
            Point::new(50.0, 50.0),
            10.0,
        )]);
        let s = SnapOptions::default();
        assert!(
            s.snap(&doc, Point::new(50.3, 59.8), 1.0)
                .distance(Point::new(50.0, 60.0))
                < 1e-9
        );
        assert_eq!(
            s.snap(&doc, Point::new(50.2, 50.2), 1.0),
            Point::new(50.0, 50.0)
        );
    }
}
