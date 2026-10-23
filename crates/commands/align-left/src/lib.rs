//! Align Left.

use aspire_core::document::Document;
use aspire_core::geometry::Point;
use aspire_core::{Change, Command, CommandError, Meta, NoParams};

pub struct AlignLeft;

impl Command for AlignLeft {
    const ID: &'static str = "align_left";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Align Left",
            group: "Align",
            icon: "drawing/align/left.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        Change::align_selected(doc, |path, sel| Point::new(sel.min.x - path.min.x, 0.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    aspire_core::contract_tests!(AlignLeft);

    use aspire_core::testing::after;

    #[test]
    fn every_left_edge_meets_the_selection_s() {
        let doc = after::<AlignLeft>(&NoParams {});
        assert!(doc.vectors.values().all(|p| {
            let b = p.bounds();
            (b.min.x - 10.0).abs() < 1e-9
        }));
    }
}
