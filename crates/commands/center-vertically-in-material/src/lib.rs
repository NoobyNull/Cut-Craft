//! Center Vertically In Material.

use aspire_core::document::Document;
use aspire_core::geometry::Point;
use aspire_core::{Change, Command, CommandError, Meta, NoParams};

pub struct CenterVerticallyInMaterial;

impl Command for CenterVerticallyInMaterial {
    const ID: &'static str = "center_vertically_in_material";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Center Vertically In Material",
            group: "Align",
            icon: "drawing/align/center_vertically_in_material.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        let job = doc.job;
        Change::align_selected(doc, |_, sel| {
            Point::new(0.0, job.height / 2.0 - sel.center().y)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    aspire_core::contract_tests!(CenterVerticallyInMaterial);

    use aspire_core::testing::after;

    #[test]
    fn selection_centre_is_on_the_job_s_horizontal_centre_line() {
        let doc = after::<CenterVerticallyInMaterial>(&NoParams {});
        let c = doc.selection_bounds().unwrap().center();
        assert!((c.y - 200.0).abs() < 1e-9, "{c:?}");
    }

    #[test]
    fn keeps_the_selection_s_layout() {
        let before = aspire_core::testing::sample_document();
        let doc = after::<CenterVerticallyInMaterial>(&NoParams {});
        let size = |d: &Document| d.selection_bounds().unwrap().size();
        assert!((size(&doc) - size(&before)).length() < 1e-9);
    }
}
