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
    /// The middle of the material
    #[default]
    JobCentre,
    /// The point X, Y
    Point,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// How many in all, the original included
    #[schemars(range(min = 2))]
    pub items: u32,
    pub around: Around,
    /// Of the centre, when copying around a point
    #[schemars(extend("x-unit" = "mm"))]
    pub x: f64,
    #[schemars(extend("x-unit" = "mm"))]
    pub y: f64,
    /// Spread the items over this angle, counter-clockwise; 360 spaces them evenly all round
    #[schemars(extend("x-unit" = "°"), range(min = 0, max = 360))]
    pub angle: f64,
    /// Turn each copy to face the centre as the original does; off, copies only move
    pub rotate_copies: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            items: 6,
            around: Around::JobCentre,
            x: 0.0,
            y: 0.0,
            angle: 360.0,
            rotate_copies: true,
        }
    }
}

/// Spreads at least this close to a full turn space the items as a full circle.
const FULL_TURN: f64 = 360.0 - 1e-9;

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

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if p.items < 2 {
            return Err(CommandError::InvalidParams(
                "items must be at least 2: the original and a copy".into(),
            ));
        }
        if !(p.angle > 0.0 && p.angle <= 360.0) {
            return Err(CommandError::InvalidParams(
                "angle must be more than 0° and at most 360°".into(),
            ));
        }
        let Some(bounds) = doc.selection_bounds() else {
            return Err(CommandError::Failed("select vectors to copy".into()));
        };
        let centre = match p.around {
            Around::JobCentre => doc.job.area().center(),
            Around::Point => Point::new(p.x, p.y),
        };
        // a full circle puts the last item a step before the first; a part one ends on it
        let gaps = if p.angle >= FULL_TURN {
            p.items
        } else {
            p.items - 1
        };
        let step = (p.angle / f64::from(gaps)).to_radians();
        let middle = bounds.center();
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

    #[test]
    fn a_full_circle_spaces_the_items_evenly_and_turns_them() {
        assert_eq!(
            placed(&around_origin()),
            [
                ((50.0, 0.0), (20.0, 10.0)),
                ((0.0, 50.0), (10.0, 20.0)),
                ((-50.0, 0.0), (20.0, 10.0)),
                ((0.0, -50.0), (10.0, 20.0)),
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
        let centres: Vec<_> = placed(&p).into_iter().map(|(c, _)| c).collect();
        assert_eq!(
            centres,
            [(50.0, 0.0), (0.0, 50.0), (-50.0, 0.0), (0.0, -50.0)]
        );
    }

    #[test]
    fn a_part_circle_puts_the_last_item_at_the_end_of_the_angle() {
        let p = Params {
            items: 3,
            angle: 90.0,
            ..around_origin()
        };
        let centres: Vec<_> = placed(&p).into_iter().map(|(c, _)| c).collect();
        let d = (50.0 * std::f64::consts::FRAC_1_SQRT_2 * 1000.0).round() / 1000.0;
        assert_eq!(centres, [(50.0, 0.0), (d, d), (0.0, 50.0)]);
    }

    #[test]
    fn the_job_centre_is_the_default_centre() {
        let mut doc = document_with(vec![square(0.0, 0.0, 10.0)]);
        let centre = doc.job.area().center();
        doc.apply(&CircularCopy::run(&doc, &Params::default()).unwrap())
            .unwrap();
        assert_eq!(doc.vectors.len(), 6);
        for path in doc.vectors.values() {
            let distance = (path.bounds().center() - centre).length();
            let first = (Point::new(5.0, 5.0) - centre).length();
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
        assert!(CircularCopy::run(&Document::default(), &Params::default()).is_err());
    }
}
