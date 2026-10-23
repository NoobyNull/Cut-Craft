//! Align Top.

use aspire_core::document::Document;
use aspire_core::geometry::Point;
use aspire_core::{Change, Command, CommandError, Meta, NoParams};

pub struct AlignTop;

impl Command for AlignTop {
    const ID: &'static str = "align_top";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Align Top",
            group: "Align",
            icon: "drawing/align/top.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        Change::align_selected(doc, |path, sel| Point::new(0.0, sel.max.y - path.max.y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    aspire_core::contract_tests!(AlignTop);

    use aspire_core::testing::after;

    #[test]
    fn every_top_edge_meets_the_selection_s() {
        let doc = after::<AlignTop>(&NoParams {});
        assert!(doc.vectors.values().all(|p| {
            let b = p.bounds();
            (b.max.y - 125.0).abs() < 1e-9
        }));
    }
}
