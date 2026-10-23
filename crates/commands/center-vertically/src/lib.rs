//! Center Vertically.

use aspire_core::document::Document;
use aspire_core::geometry::Point;
use aspire_core::{Change, Command, CommandError, Meta, NoParams};

pub struct CenterVertically;

impl Command for CenterVertically {
    const ID: &'static str = "center_vertically";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Center Vertically",
            group: "Align",
            icon: "drawing/align/center_vertically.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        Change::align_selected(doc, |path, sel| {
            Point::new(0.0, sel.center().y - path.center().y)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    aspire_core::contract_tests!(CenterVertically);

    use aspire_core::testing::after;

    #[test]
    fn every_centre_lines_up_horizontally() {
        let doc = after::<CenterVertically>(&NoParams {});
        assert!(doc.vectors.values().all(|p| {
            let b = p.bounds();
            (b.center().y - 67.5).abs() < 1e-9
        }));
    }
}
