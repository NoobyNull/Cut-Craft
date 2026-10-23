//! Center Horizontally.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::Point;
use cut_craft_core::{Change, Command, CommandError, Meta, NoParams};

pub struct CenterHorizontally;

impl Command for CenterHorizontally {
    const ID: &'static str = "center_horizontally";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Center Horizontally",
            group: "Align",
            icon: "drawing/align/center_horizontally.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        Change::align_selected(doc, |path, sel| {
            Point::new(sel.center().x - path.center().x, 0.0)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    cut_craft_core::contract_tests!(CenterHorizontally);

    use cut_craft_core::testing::after;

    #[test]
    fn every_centre_lines_up_vertically() {
        let doc = after::<CenterHorizontally>(&NoParams {});
        assert!(doc.vectors.values().all(|p| {
            let b = p.bounds();
            (b.center().x - 117.5).abs() < 1e-9
        }));
    }
}
