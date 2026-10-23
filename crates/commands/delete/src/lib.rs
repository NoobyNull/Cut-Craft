//! Delete.

use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta, NoParams};

pub struct Delete;

impl Command for Delete {
    const ID: &'static str = "delete";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Delete",
            group: "Edit",
            icon: "",
            shortcut: "Del",
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        if doc.selection.is_empty() {
            return Err(CommandError::NothingSelected);
        }
        let mut c = Change::builder(doc);
        for &id in &doc.selection {
            c.remove(id);
        }
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    cut_craft_core::contract_tests!(Delete);

    #[test]
    fn removes_the_selected_vectors() {
        assert!(
            cut_craft_core::testing::after::<Delete>(&NoParams {})
                .vectors
                .is_empty()
        );
    }

    #[test]
    fn nothing_selected_is_an_error() {
        assert_eq!(
            Delete::run(&Document::default(), &NoParams {}),
            Err(CommandError::NothingSelected)
        );
    }
}
