//! Unselect All.

use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta, NoParams};

pub struct UnselectAll;

impl Command for UnselectAll {
    const ID: &'static str = "unselect_all";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Unselect All",
            group: "Edit",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        let mut c = Change::builder(doc);
        c.select([]);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    cut_craft_core::contract_tests!(UnselectAll);

    #[test]
    fn clears_the_selection() {
        assert!(
            cut_craft_core::testing::after::<UnselectAll>(&NoParams {})
                .selection
                .is_empty()
        );
    }
}
