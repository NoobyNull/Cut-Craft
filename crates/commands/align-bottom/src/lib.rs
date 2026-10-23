//! Align Bottom.

use aspire_core::document::Document;
use aspire_core::geometry::Point;
use aspire_core::{Change, Command, CommandError, Meta, NoParams};

pub struct AlignBottom;

impl Command for AlignBottom {
    const ID: &'static str = "align_bottom";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Align Bottom",
            group: "Align",
            icon: "drawing/align/bottom.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        Change::align_selected(doc, |path, sel| Point::new(0.0, sel.min.y - path.min.y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    aspire_core::contract_tests!(AlignBottom);

    use aspire_core::testing::after;

    #[test]
    fn every_bottom_edge_meets_the_selection_s() {
        let doc = after::<AlignBottom>(&NoParams {});
        assert!(doc.vectors.values().all(|p| {
            let b = p.bounds();
            (b.min.y - 10.0).abs() < 1e-9
        }));
    }
}
