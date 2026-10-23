//! Align Right.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::Point;
use cut_craft_core::{Change, Command, CommandError, Meta, NoParams};

pub struct AlignRight;

impl Command for AlignRight {
    const ID: &'static str = "align_right";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Align Right",
            group: "Align",
            icon: "drawing/align/right.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        Change::align_selected(doc, |path, sel| Point::new(sel.max.x - path.max.x, 0.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    cut_craft_core::contract_tests!(AlignRight);

    use cut_craft_core::testing::after;

    #[test]
    fn every_right_edge_meets_the_selection_s() {
        let doc = after::<AlignRight>(&NoParams {});
        assert!(doc.vectors.values().all(|p| {
            let b = p.bounds();
            (b.max.x - 225.0).abs() < 1e-9
        }));
    }
}
