//! Select All Vectors.

use aspire_core::document::Document;
use aspire_core::{Change, Command, CommandError, Meta, NoParams};

pub struct SelectAllVectors;

impl Command for SelectAllVectors {
    const ID: &'static str = "select_all_vectors";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Select All Vectors",
            group: "Edit",
            icon: "",
            shortcut: "Ctrl+A",
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        let mut c = Change::builder(doc);
        c.select(doc.vectors.keys().copied());
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    aspire_core::contract_tests!(SelectAllVectors);

    #[test]
    fn selects_every_vector() {
        let mut doc = aspire_core::testing::sample_document();
        let mut c = Change::builder(&doc);
        c.select([]);
        doc.apply(&c.finish()).unwrap();
        doc.apply(&SelectAllVectors::run(&doc, &NoParams {}).unwrap())
            .unwrap();
        assert_eq!(doc.selection.len(), doc.vectors.len());
    }
}
