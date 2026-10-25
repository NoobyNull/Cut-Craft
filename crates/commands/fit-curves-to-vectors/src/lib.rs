//! Fit Curves to Vectors: the selected vectors redrawn as circular arcs, Béziers or straight
//! lines within a tolerance, replacing them or added beside them on the current layer.

use cut_craft_core::document::Document;
use cut_craft_core::trace::{self, Fit};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct FitCurvesToVectors;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FittingType {
    #[default]
    #[schemars(title = "Circular Arcs")]
    CircularArcs,
    #[schemars(title = "Bezier Curves")]
    BezierCurves,
    #[schemars(title = "Straight Lines")]
    StraightLines,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub fitting_type: FittingType,
    /// The new vectors stay within this distance of the old
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub tolerance: f64,
    /// Corners turning more than the max angle stay sharp
    pub keep_sharp_corners: bool,
    #[schemars(title = "Max. angle", extend("x-unit" = "deg"), range(min = 0, max = 180))]
    pub max_angle: f64,
    /// Delete the originals; otherwise the fitted vectors are added beside them
    pub replace_selected_vectors: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            fitting_type: FittingType::CircularArcs,
            // ponytail: Aspire's defaults are not in its files [inf]; the help shows 20°
            tolerance: 0.1,
            keep_sharp_corners: true,
            max_angle: 20.0,
            replace_selected_vectors: true,
        }
    }
}

impl Command for FitCurvesToVectors {
    const ID: &'static str = "fit_curves_to_vectors";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Fit Curves to Vectors",
            group: "Edit Objects",
            icon: "drawing/create_vectors/fit_curves_to_vectors.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("tolerance", p.tolerance)?;
        let (ids, paths) = doc.selected_paths(|_| true);
        if ids.is_empty() {
            return Err(CommandError::Failed(
                "select vectors to fit curves to".into(),
            ));
        }
        let fit = match p.fitting_type {
            FittingType::CircularArcs => Fit::Arcs,
            FittingType::BezierCurves => Fit::Beziers,
            FittingType::StraightLines => Fit::Lines,
        };
        let corner = if p.keep_sharp_corners {
            p.max_angle.to_radians()
        } else {
            std::f64::consts::PI
        };
        let fitted = paths
            .iter()
            .map(|path| {
                // ponytail: points a tenth of the tolerance apart feed the fit
                let mut points = path.flatten(p.tolerance / 10.0);
                if path.closed && points.len() > 1 {
                    points.pop();
                }
                trace::fit(&points, path.closed, fit, p.tolerance, corner)
            })
            .collect();
        Ok(Change::swap(
            doc,
            if p.replace_selected_vectors {
                ids
            } else {
                vec![]
            },
            fitted,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::{Path, Point, Segment};
    use cut_craft_core::testing::{document_with, rejection};

    fn fitted(path: Path, p: &Params) -> Document {
        let mut doc = document_with(vec![path]);
        let change = FitCurvesToVectors::run(&doc, p).unwrap();
        doc.apply(&change).unwrap();
        doc
    }

    /// A circle of radius 20 as a 200-sided polygon.
    fn faceted_circle() -> Path {
        let pts: Vec<Point> = (0..200)
            .map(|i| Point::from_angle(f64::from(i) * std::f64::consts::TAU / 200.0) * 20.0)
            .collect();
        Path::polyline(&pts, true)
    }

    cut_craft_core::contract_tests!(FitCurvesToVectors);

    #[test]
    fn a_faceted_circle_becomes_a_few_arcs_within_the_tolerance() {
        let doc = fitted(faceted_circle(), &Params::default());
        assert_eq!(doc.vectors.len(), 1, "replaced");
        let path = doc.vectors.values().next().unwrap();
        assert!(path.closed);
        assert!(path.segments.len() <= 8, "{} segments", path.segments.len());
        assert!(
            path.segments
                .iter()
                .all(|s| matches!(s, Segment::Arc { .. }))
        );
        for q in path.flatten(0.01) {
            assert!((q.length() - 20.0).abs() < 0.1 + 1e-6, "{q} strays");
        }
    }

    #[test]
    fn lines_beziers_and_keeping_the_originals() {
        let lines = fitted(
            faceted_circle(),
            &Params {
                fitting_type: FittingType::StraightLines,
                replace_selected_vectors: false,
                ..Params::default()
            },
        );
        assert_eq!(lines.vectors.len(), 2, "kept beside the original");
        let new = lines.vectors.values().last().unwrap();
        assert!(
            new.segments
                .iter()
                .all(|s| matches!(s, Segment::Line { .. }))
        );
        assert!(new.segments.len() < 200);
        let curves = fitted(
            faceted_circle(),
            &Params {
                fitting_type: FittingType::BezierCurves,
                ..Params::default()
            },
        );
        let path = curves.vectors.values().next().unwrap();
        assert!(
            path.segments
                .iter()
                .any(|s| matches!(s, Segment::Cubic { .. }))
        );
    }

    #[test]
    fn sharp_corners_survive_when_kept() {
        let square = cut_craft_core::testing::square(0.0, 0.0, 10.0);
        let doc = fitted(square, &Params::default());
        let path = doc.vectors.values().next().unwrap();
        let mut corners = vec![path.start];
        corners.extend(path.segments.iter().map(Segment::end));
        for c in [
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
            Point::new(0.0, 10.0),
        ] {
            assert!(
                corners.iter().any(|q| q.distance(c) < 1e-6),
                "corner {c} kept"
            );
        }
    }

    #[test]
    fn needs_a_selection_and_a_tolerance() {
        assert!(rejection::<FitCurvesToVectors>(&Params::default()).contains("select"));
        let p = Params {
            tolerance: 0.0,
            ..Params::default()
        };
        assert!(rejection::<FitCurvesToVectors>(&p).contains("tolerance"));
    }
}
