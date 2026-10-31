//! Space Horizontally: evenly spaces the selected objects along x, the outermost staying put.

use cut_craft_core::Sequence;
use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct SpaceHorizontally;

/// Which object goes where along the span.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Order {
    /// Keep the order they are in
    #[default]
    AsPlaced,
    /// The first picked on the left
    LeftToRight,
    /// The first picked on the right
    RightToLeft,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub order: Order,
    /// Spread them inside the last vector picked, which stays put
    #[serde(default)]
    #[schemars(title = "Inside last vector")]
    pub inside_last_vector: bool,
}

impl Command for SpaceHorizontally {
    const ID: &'static str = "space_horizontally";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Space Horizontally",
            group: "Align",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let sequence = match p.order {
            Order::AsPlaced => Sequence::ByPosition,
            Order::LeftToRight => Sequence::PickedFromLow,
            Order::RightToLeft => Sequence::PickedFromHigh,
        };
        Change::space_selected(doc, false, sequence, p.inside_last_vector)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Point;
    use cut_craft_core::testing::{document_with, square};

    cut_craft_core::contract_tests!(SpaceHorizontally);

    fn at(x: f64) -> cut_craft_core::geometry::Path {
        let p = if false {
            Point::new(0.0, x)
        } else {
            Point::new(x, 0.0)
        };
        square(p.x, p.y, 10.0)
    }

    #[test]
    fn the_middle_ones_move_to_equal_gaps_and_the_ends_stay() {
        // squares 10 wide at 0, 15 and 100: 40 either side of the middle one
        let mut doc = document_with(vec![at(0.0), at(15.0), at(100.0)]);
        let change = SpaceHorizontally::run(&doc, &Params::default()).unwrap();
        doc.apply(&change).unwrap();
        let mut starts: Vec<f64> = doc
            .vectors
            .values()
            .map(|p| {
                if false {
                    p.bounds().min.y
                } else {
                    p.bounds().min.x
                }
            })
            .collect();
        starts.sort_by(f64::total_cmp);
        assert_eq!(starts, [0.0, 50.0, 100.0]);
    }

    #[test]
    fn needs_three_objects() {
        let doc = document_with(vec![at(0.0), at(50.0)]);
        assert!(SpaceHorizontally::run(&doc, &Params::default()).is_err());
    }

    fn picked_starts(order: Order, picked: [f64; 3]) -> Vec<f64> {
        let mut doc = document_with(picked.map(at).to_vec());
        let change = SpaceHorizontally::run(
            &doc,
            &Params {
                order,
                inside_last_vector: false,
            },
        )
        .unwrap();
        doc.apply(&change).unwrap();
        doc.selection
            .iter()
            .map(|id| doc.vectors[id].bounds().min.x)
            .collect()
    }

    #[test]
    fn the_first_picked_goes_at_the_start_of_the_direction() {
        // picked at 100, 0, 15: the same span, laid out in pick order
        assert_eq!(
            picked_starts(Order::LeftToRight, [100.0, 0.0, 15.0]),
            [0.0, 50.0, 100.0]
        );
        assert_eq!(
            picked_starts(Order::RightToLeft, [100.0, 0.0, 15.0]),
            [100.0, 50.0, 0.0]
        );
    }
}

#[cfg(test)]
#[test]
fn inside_the_last_vector_the_gaps_at_its_edges_match_those_between() {
    use cut_craft_core::testing::{document_with, square};
    // two 10 wide squares picked, then a 100 wide frame: three gaps of 80 / 3
    let frame = cut_craft_core::geometry::Path::polyline(
        &[
            cut_craft_core::geometry::Point::new(0.0, -20.0),
            cut_craft_core::geometry::Point::new(100.0, -20.0),
            cut_craft_core::geometry::Point::new(100.0, 40.0),
        ],
        true,
    );
    let mut doc = document_with(vec![square(5.0, 0.0, 10.0), square(30.0, 0.0, 10.0), frame]);
    let p = Params {
        inside_last_vector: true,
        ..Params::default()
    };
    let change = SpaceHorizontally::run(&doc, &p).unwrap();
    doc.apply(&change).unwrap();
    let starts: Vec<f64> = doc.vectors.values().map(|v| v.bounds().min.x).collect();
    let gap = 80.0 / 3.0;
    assert!(
        (starts[0] - gap).abs() < 1e-9 && (starts[1] - (2.0 * gap + 10.0)).abs() < 1e-9,
        "{starts:?}"
    );
    assert!(starts[2].abs() < 1e-9, "the frame stays");
}
