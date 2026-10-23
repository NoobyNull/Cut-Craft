//! Center.

use aspire_core::document::Document;
use aspire_core::{Change, Command, CommandError, Meta, NoParams};

pub struct Center;

impl Command for Center {
    const ID: &'static str = "center";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Center",
            group: "Align",
            icon: "drawing/align/center.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        // cascade: centring is centring horizontally, then vertically
        Change::chain(
            doc,
            &[
                &|d| cmd_center_horizontally::CenterHorizontally::run(d, &NoParams {}),
                &|d| cmd_center_vertically::CenterVertically::run(d, &NoParams {}),
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    aspire_core::contract_tests!(Center);

    use aspire_core::testing::after;

    #[test]
    fn every_centre_meets_the_selection_s() {
        let doc = after::<Center>(&NoParams {});
        assert!(doc.vectors.values().all(|p| {
            let b = p.bounds();
            (b.center() - aspire_core::geometry::Point::new(117.5, 67.5)).length() < 1e-9
        }));
    }
}
