//! Space Vertically: evenly spaces the selected objects along y, the outermost staying put.

use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta, NoParams};

pub struct SpaceVertically;

impl Command for SpaceVertically {
    const ID: &'static str = "space_vertically";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Space Vertically",
            group: "Align",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        Change::space_selected(doc, true)
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
        let change = SpaceVertically::run(&doc, &NoParams {}).unwrap();
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
        assert!(SpaceVertically::run(&doc, &NoParams {}).is_err());
    }
}
