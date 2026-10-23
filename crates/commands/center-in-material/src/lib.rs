//! Center In Material.

use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta, NoParams};

pub struct CenterInMaterial;

impl Command for CenterInMaterial {
    const ID: &'static str = "center_in_material";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Center In Material",
            group: "Align",
            icon: "drawing/align/center_in_material.svg",
            shortcut: "F9",
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        // cascade: centre horizontally in the material, then vertically
        Change::chain(
            doc,
            &[
                &|d| {
                    cmd_center_horizontally_in_material::CenterHorizontallyInMaterial::run(
                        d,
                        &NoParams {},
                    )
                },
                &|d| {
                    cmd_center_vertically_in_material::CenterVerticallyInMaterial::run(
                        d,
                        &NoParams {},
                    )
                },
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    cut_craft_core::contract_tests!(CenterInMaterial);

    use cut_craft_core::testing::after;

    #[test]
    fn selection_centre_is_the_job_centre() {
        let c = after::<CenterInMaterial>(&NoParams {})
            .selection_bounds()
            .unwrap()
            .center();
        assert!(
            (c - cut_craft_core::geometry::Point::new(300.0, 200.0)).length() < 1e-9,
            "{c:?}"
        );
    }
}
