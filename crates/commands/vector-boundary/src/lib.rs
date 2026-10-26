//! Vector Boundary: closed boundaries round the selected vectors (Aspire's help: Vector
//! Boundary). Without a rubber band the boundary follows the selected closed vectors' outsides
//! (overlapping ones as one, holes left out); a rubber band stretches round everything
//! selected, open vectors too. Either can be offset outwards.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Path, Point, winding};
use cut_craft_core::polygon::{Corners, TOLERANCE, offset, union};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct VectorBoundary;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// Grow the boundary outwards by the distance
    #[schemars(title = "Offset Boundary")]
    pub offset: bool,
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub distance: f64,
    #[schemars(extend("x-widget" = "row"))]
    pub corners: Corners,
    /// The boundary a rubber band stretched round the selection would make
    #[schemars(title = "Rubber Band Boundary")]
    pub rubber_band: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            offset: true,
            distance: 5.0,
            corners: Corners::Round,
            rubber_band: false,
        }
    }
}

impl Command for VectorBoundary {
    const ID: &'static str = "vector_boundary";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Vector Boundary",
            group: "Create Vectors",
            icon: "drawing/create_vectors/vector_boundary.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let selected: Vec<&Path> = doc.selection.iter().map(|id| &doc.vectors[id]).collect();
        if selected.is_empty() {
            return Err(CommandError::NothingSelected);
        }
        let shapes: Vec<Path> = if p.rubber_band {
            let points: Vec<Point> = selected.iter().flat_map(|v| v.flatten(TOLERANCE)).collect();
            vec![Path::polyline(&hull(points), true)]
        } else {
            let closed: Vec<Path> = selected
                .iter()
                .filter(|v| v.closed)
                .map(|v| (*v).clone())
                .collect();
            if closed.is_empty() {
                return Err(CommandError::Failed(
                    "select closed vectors, or tick Rubber Band Boundary for open ones".into(),
                ));
            }
            // the outsides only: holes run clockwise
            union(&closed)
                .into_iter()
                .filter(|o| winding(&o.flatten(TOLERANCE)) > 0.0)
                .collect()
        };
        let boundaries = if p.offset && p.distance > 0.0 {
            offset(&shapes, p.distance, p.corners)
        } else {
            shapes
        };
        let mut c = Change::builder(doc);
        let ids: Vec<_> = boundaries.into_iter().map(|b| c.add(b)).collect();
        c.select(ids);
        Ok(c.finish())
    }
}

/// The convex hull of `points`, anticlockwise (Andrew's monotone chain).
fn hull(mut points: Vec<Point>) -> Vec<Point> {
    points.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    points.dedup();
    if points.len() < 3 {
        return points;
    }
    let turn = |o: Point, a: Point, b: Point| (a - o).perp_dot(b - o);
    let mut lower: Vec<Point> = vec![];
    for &p in &points {
        while lower.len() >= 2 && turn(lower[lower.len() - 2], lower[lower.len() - 1], p) <= 0.0 {
            lower.pop();
        }
        lower.push(p);
    }
    let mut upper: Vec<Point> = vec![];
    for &p in points.iter().rev() {
        while upper.len() >= 2 && turn(upper[upper.len() - 2], upper[upper.len() - 1], p) <= 0.0 {
            upper.pop();
        }
        upper.push(p);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{document_with, rejection, run_on, square};

    fn bounds_of_new(doc: &Document, before: usize) -> Vec<cut_craft_core::geometry::Rect> {
        doc.vectors
            .values()
            .skip(before)
            .map(Path::bounds)
            .collect()
    }

    cut_craft_core::contract_tests!(VectorBoundary);

    #[test]
    fn overlapping_shapes_get_one_boundary_offset_outwards() {
        let mut doc = document_with(vec![square(0.0, 0.0, 10.0), square(5.0, 5.0, 10.0)]);
        run_on::<VectorBoundary>(
            &mut doc,
            &Params {
                distance: 2.0,
                ..Params::default()
            },
        );
        let made = bounds_of_new(&doc, 2);
        assert_eq!(made.len(), 1, "one boundary round both");
        let b = made[0];
        assert!(
            (b.min.x + 2.0).abs() < 0.05 && (b.max.x - 17.0).abs() < 0.05,
            "{b:?}"
        );
    }

    #[test]
    fn a_rubber_band_spans_open_vectors_too() {
        let line = Path::polyline(&[Point::new(30.0, 0.0), Point::new(40.0, 20.0)], false);
        let mut doc = document_with(vec![square(0.0, 0.0, 10.0), line]);
        let p = Params {
            offset: false,
            rubber_band: true,
            ..Params::default()
        };
        run_on::<VectorBoundary>(&mut doc, &p);
        let made = bounds_of_new(&doc, 2);
        assert_eq!(made.len(), 1);
        assert_eq!(
            (made[0].min, made[0].max),
            (Point::ZERO, Point::new(40.0, 20.0))
        );
        assert_eq!(
            hull(vec![
                Point::ZERO,
                Point::new(1.0, 0.0),
                Point::new(0.5, 0.2),
                Point::new(0.0, 1.0)
            ])
            .len(),
            3
        );
    }

    #[test]
    fn open_vectors_alone_need_the_rubber_band() {
        let line = Path::polyline(&[Point::ZERO, Point::new(10.0, 0.0)], false);
        let doc = document_with(vec![line]);
        assert!(
            VectorBoundary::run(&doc, &Params::default())
                .unwrap_err()
                .to_string()
                .contains("Rubber Band")
        );
        assert!(
            !rejection::<VectorBoundary>(&Params::default()).is_empty(),
            "nothing selected"
        );
    }
}
