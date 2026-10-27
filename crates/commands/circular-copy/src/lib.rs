//! Circular Copy: copies of the selected vectors spaced evenly round a centre.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::Point;
use cut_craft_core::{Change, Command, CommandError, Meta};
use glam::DAffine2;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct CircularCopy;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Around {
    /// The middle of the selection: move the centre to spread the copies out
    #[default]
    SelectionCentre,
    /// The middle of the material
    JobCentre,
    /// The point X, Y
    Point,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AngleMode {
    /// The angle is shared out evenly between the items
    #[default]
    Total,
    /// The angle from one item to the next
    Step,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// How many in all, the original included
    #[schemars(range(min = 2))]
    pub items: u32,
    /// The rotation centre
    pub around: Around,
    /// Of the centre, when copying around a point
    #[schemars(extend("x-unit" = "mm"))]
    pub x: f64,
    #[schemars(extend("x-unit" = "mm"))]
    pub y: f64,
    pub angle_mode: AngleMode,
    /// Positive places the copies clockwise, negative counter-clockwise
    #[schemars(extend("x-unit" = "°"))]
    pub angle: f64,
    /// Turn each copy to match its place on the circle; off, copies keep the original's
    /// orientation
    pub rotate_copies: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            items: 6,
            around: Around::SelectionCentre,
            x: 0.0,
            y: 0.0,
            angle_mode: AngleMode::Total,
            angle: 360.0,
            rotate_copies: true,
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
            icon: "drawing/create_vectors/circular_copy.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if p.items < 2 {
            return Err(CommandError::InvalidParams(
                "items must be at least 2: the original and a copy".into(),
            ));
        }
        if p.angle == 0.0 || !p.angle.is_finite() {
            return Err(CommandError::InvalidParams("angle must not be 0".into()));
        }
        if p.angle_mode == AngleMode::Total && p.angle.abs() > 360.0 {
            return Err(CommandError::InvalidParams(
                "a total angle must be at most 360°".into(),
            ));
        }
        let Some(bounds) = doc.selection_bounds() else {
            return Err(CommandError::Failed("select vectors to copy".into()));
        };
        let middle = bounds.center();
        let centre = match p.around {
            Around::SelectionCentre => middle,
            Around::JobCentre => doc.job.area().center(),
            Around::Point => Point::new(p.x, p.y),
        };
        let step = match p.angle_mode {
            AngleMode::Total => p.angle / f64::from(p.items),
            AngleMode::Step => p.angle,
        };
        // positive is clockwise: the opposite of the maths convention
        let step = -step.to_radians();
        let transforms: Vec<DAffine2> = (1..p.items)
            .map(|i| {
                let turn = DAffine2::from_translation(centre)
                    * DAffine2::from_angle(step * f64::from(i))
                    * DAffine2::from_translation(-centre);
                if p.rotate_copies {
                    turn
                } else {
                    DAffine2::from_translation(turn.transform_point2(middle) - middle)
                }
            })
            .collect();
        Ok(Change::copy_selected(doc, &transforms))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Path;
    use cut_craft_core::testing::{document_with, square};

    cut_craft_core::contract_tests!(CircularCopy);

    /// A 20 wide, 10 high bar to the right of the origin, centred at (50, 0).
    fn bar() -> Path {
        let p = Point::new;
        Path::polyline(
            &[p(40.0, -5.0), p(60.0, -5.0), p(60.0, 5.0), p(40.0, 5.0)],
            true,
        )
    }

    /// (centre, size) of every vector, rounded to µm, in creation order.
    fn placed(p: &Params) -> Vec<((f64, f64), (f64, f64))> {
        let mut doc = document_with(vec![bar()]);
        doc.apply(&CircularCopy::run(&doc, p).unwrap()).unwrap();
        let round = |v: f64| (v * 1000.0).round() / 1000.0 + 0.0;
        doc.vectors
            .values()
            .map(|path| {
                let b = path.bounds();
                let (c, s) = (b.center(), b.size());
                ((round(c.x), round(c.y)), (round(s.x), round(s.y)))
            })
            .collect()
    }

    fn around_origin() -> Params {
        Params {
            items: 4,
            around: Around::Point,
            ..Params::default()
        }
    }

    fn centres(p: &Params) -> Vec<(f64, f64)> {
        placed(p).into_iter().map(|(c, _)| c).collect()
    }

    #[test]
    fn a_total_angle_is_shared_out_clockwise_and_turns_the_copies() {
        assert_eq!(
            placed(&around_origin()),
            [
                ((50.0, 0.0), (20.0, 10.0)),
                ((0.0, -50.0), (10.0, 20.0)),
                ((-50.0, 0.0), (20.0, 10.0)),
                ((0.0, 50.0), (10.0, 20.0)),
            ]
        );
    }

    #[test]
    fn without_rotating_the_copies_only_move() {
        let p = Params {
            rotate_copies: false,
            ..around_origin()
        };
        let sizes: Vec<_> = placed(&p).into_iter().map(|(_, size)| size).collect();
        assert_eq!(sizes, [(20.0, 10.0); 4]);
        assert_eq!(
            centres(&p),
            [(50.0, 0.0), (0.0, -50.0), (-50.0, 0.0), (0.0, 50.0)]
        );
    }

    #[test]
    fn a_part_total_angle_is_divided_by_the_items() {
        let p = Params {
            items: 3,
            angle: 90.0,
            ..around_origin()
        };
        let r = |v: f64| (v * 1000.0).round() / 1000.0;
        let (c30, s30) = (r(50.0 * 30f64.to_radians().cos()), r(50.0 * 0.5));
        assert_eq!(centres(&p), [(50.0, 0.0), (c30, -s30), (s30, -c30)]);
    }

    #[test]
    fn a_step_angle_is_per_item_and_negative_goes_counter_clockwise() {
        let p = Params {
            items: 3,
            angle_mode: AngleMode::Step,
            angle: -90.0,
            ..around_origin()
        };
        assert_eq!(centres(&p), [(50.0, 0.0), (0.0, 50.0), (-50.0, 0.0)]);
    }

    #[test]
    fn the_selection_centre_is_the_default_and_the_job_centre_is_offered() {
        // around its own middle the copies only turn in place, as in Aspire
        let p = Params {
            items: 4,
            ..Params::default()
        };
        assert_eq!(centres(&p), [(50.0, 0.0); 4]);
        let mut doc = document_with(vec![square(0.0, 0.0, 10.0)]);
        let centre = doc.job.area().center();
        let job = Params {
            around: Around::JobCentre,
            ..Params::default()
        };
        doc.apply(&CircularCopy::run(&doc, &job).unwrap()).unwrap();
        assert_eq!(doc.vectors.len(), 6);
        let first = (Point::new(5.0, 5.0) - centre).length();
        for path in doc.vectors.values() {
            let distance = (path.bounds().center() - centre).length();
            assert!((distance - first).abs() < 1e-9, "{distance} vs {first}");
        }
    }

    #[test]
    fn needs_a_selection_two_items_and_an_angle() {
        let doc = document_with(vec![bar()]);
        let one = Params {
            items: 1,
            ..Params::default()
        };
        assert!(CircularCopy::run(&doc, &one).is_err());
        let none = Params {
            angle: 0.0,
            ..Params::default()
        };
        assert!(CircularCopy::run(&doc, &none).is_err());
        let over = Params {
            angle: 400.0,
            ..Params::default()
        };
        assert!(CircularCopy::run(&doc, &over).is_err());
        assert!(CircularCopy::run(&Document::default(), &Params::default()).is_err());
    }
}
