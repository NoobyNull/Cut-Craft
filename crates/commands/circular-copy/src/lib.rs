//! Circular Copy: copies of the selection round a centre, over a total angle or a step angle
//! apart (positive clockwise), turned with their place on the circle or kept upright.

use cut_craft_core::document::Document;
use cut_craft_core::glam::DAffine2;
use cut_craft_core::params::Xy;
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct CircularCopy;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AngleIs {
    /// The items share this angle
    #[default]
    TotalAngle,
    /// Each item is this far round from the last
    StepAngle,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// The centre the copies go round; the selection's middle to begin with
    #[schemars(title = "Rotation center")]
    pub center: Xy,
    /// Each copy turns with its place on the circle
    pub rotate_copies: bool,
    /// Items round the circle, the original among them
    #[schemars(title = "Number of items", range(min = 2, max = 1000))]
    pub items: u32,
    #[schemars(extend("x-widget" = "row"))]
    pub angle_is: AngleIs,
    /// Positive goes clockwise
    #[schemars(extend("x-unit" = "deg"))]
    pub angle: f64,
    pub group_copies: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            center: Xy::new(0.0, 0.0),
            rotate_copies: true,
            items: 6,
            angle_is: AngleIs::TotalAngle,
            angle: 360.0,
            group_copies: false,
        }
    }
}

impl Command for CircularCopy {
    const ID: &'static str = "circular_copy";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Circular Copy",
            group: "Array Objects",
            icon: "drawing/create_vectors/circular_copy.svg",
            shortcut: "",
        }
    }

    fn initial_params(doc: &Document) -> Params {
        let centre = doc
            .selection_bounds()
            .map(cut_craft_core::geometry::Rect::center)
            .unwrap_or_default();
        Params {
            center: Xy::new(centre.x, centre.y),
            ..Params::default()
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let Some(bounds) = doc.selection_bounds() else {
            return Err(CommandError::NothingSelected);
        };
        if p.items < 2 {
            return Err(CommandError::InvalidParams(
                "make at least two items".into(),
            ));
        }
        let step = match p.angle_is {
            AngleIs::TotalAngle => p.angle / f64::from(p.items),
            AngleIs::StepAngle => p.angle,
        };
        let centre = p.center.into();
        let places: Vec<DAffine2> = (1..p.items)
            .map(|k| {
                // clockwise is a negative turn in the job's y-up coordinates
                let turn = DAffine2::from_translation(centre)
                    * DAffine2::from_angle((-step * f64::from(k)).to_radians())
                    * DAffine2::from_translation(-centre);
                if p.rotate_copies {
                    turn
                } else {
                    let middle = bounds.center();
                    DAffine2::from_translation(turn.transform_point2(middle) - middle)
                }
            })
            .collect();
        Change::copy_selected(doc, &places, p.group_copies)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Point;
    use cut_craft_core::testing::{document_with, run_on, square};

    cut_craft_core::contract_tests!(CircularCopy);

    fn around(p: &Params) -> Vec<Point> {
        let mut doc = document_with(vec![square(18.0, -2.0, 4.0)]);
        run_on::<CircularCopy>(&mut doc, p);
        doc.vectors.values().map(|v| v.bounds().center()).collect()
    }

    #[test]
    fn six_items_round_the_origin_clockwise() {
        let c = around(&Params::default());
        assert_eq!(c.len(), 6);
        // the first copy is 60° clockwise from (20, 0)
        let first = Point::new(
            20.0 * 60_f64.to_radians().cos(),
            -20.0 * 60_f64.to_radians().sin(),
        );
        assert!((c[1] - first).length() < 1e-9, "{}", c[1]);
        for q in &c {
            assert!((q.length() - 20.0).abs() < 1e-9);
        }
    }

    #[test]
    fn step_angles_and_upright_copies() {
        let p = Params {
            angle_is: AngleIs::StepAngle,
            angle: -90.0,
            items: 2,
            rotate_copies: false,
            ..Params::default()
        };
        let mut doc = document_with(vec![square(18.0, -2.0, 4.0)]);
        run_on::<CircularCopy>(&mut doc, &p);
        let copy = doc.vectors.values().last().unwrap().bounds();
        assert!(
            (copy.center() - Point::new(0.0, 20.0)).length() < 1e-9,
            "counter-clockwise"
        );
        assert!(
            (copy.size() - Point::new(4.0, 4.0)).length() < 1e-9,
            "kept upright"
        );
    }
}
