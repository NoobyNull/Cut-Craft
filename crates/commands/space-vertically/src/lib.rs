//! Space Vertically: evenly spaces the selected objects along y, the outermost staying put.

use cut_craft_core::Sequence;
use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct SpaceVertically;

/// Which object goes where along the span.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Order {
    /// Keep the order they are in
    #[default]
    AsPlaced,
    /// The first picked at the bottom
    BottomToTop,
    /// The first picked at the top
    TopToBottom,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub order: Order,
}

impl Command for SpaceVertically {
    const ID: &'static str = "space_vertically";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Space Vertically",
            group: "Align",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let sequence = match p.order {
            Order::AsPlaced => Sequence::ByPosition,
            Order::BottomToTop => Sequence::PickedFromLow,
            Order::TopToBottom => Sequence::PickedFromHigh,
        };
        Change::space_selected(doc, true, sequence)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Point;
    use cut_craft_core::testing::{document_with, square};

    cut_craft_core::contract_tests!(SpaceVertically);

    fn at(x: f64) -> cut_craft_core::geometry::Path {
        let p = if true {
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
        let change = SpaceVertically::run(&doc, &Params::default()).unwrap();
        doc.apply(&change).unwrap();
        let mut starts: Vec<f64> = doc
            .vectors
            .values()
            .map(|p| {
                if true {
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
        assert!(SpaceVertically::run(&doc, &Params::default()).is_err());
    }

    fn picked_starts(order: Order, picked: [f64; 3]) -> Vec<f64> {
        let mut doc = document_with(picked.map(at).to_vec());
        let change = SpaceVertically::run(&doc, &Params { order }).unwrap();
        doc.apply(&change).unwrap();
        doc.selection
            .iter()
            .map(|id| doc.vectors[id].bounds().min.y)
            .collect()
    }

    #[test]
    fn the_first_picked_goes_at_the_start_of_the_direction() {
        // picked at 100, 0, 15: the same span, laid out in pick order
        assert_eq!(
            picked_starts(Order::BottomToTop, [100.0, 0.0, 15.0]),
            [0.0, 50.0, 100.0]
        );
        assert_eq!(
            picked_starts(Order::TopToBottom, [100.0, 0.0, 15.0]),
            [100.0, 50.0, 0.0]
        );
    }
}
