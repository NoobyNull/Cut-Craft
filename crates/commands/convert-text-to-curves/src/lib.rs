//! Convert Text to Curves: the selected text becomes plain vectors that node editing can
//! change, still grouped; after it, the text cannot be edited as text (Aspire's help:
//! Convert Text To Curves).

use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta, NoParams};

pub struct ConvertTextToCurves;

impl Command for ConvertTextToCurves {
    const ID: &'static str = "convert_text_to_curves";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Convert Text to Curves",
            group: "Create Vectors",
            icon: "drawing/create_vectors/convert_text_to_curves.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        let mut groups: Vec<u64> = doc
            .selection
            .iter()
            .filter_map(|id| doc.vector_groups.get(id).copied())
            .filter(|g| doc.texts.contains_key(g))
            .collect();
        groups.sort_unstable();
        groups.dedup();
        if groups.is_empty() {
            return Err(CommandError::Failed("select the text to convert".into()));
        }
        let mut c = Change::builder(doc);
        for group in groups {
            c.set_text(group, None);
        }
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{rejection, run_on};

    fn text() -> Document {
        let mut doc = Document::default();
        let mut c = Change::builder(&doc);
        let id = c.add(cut_craft_core::testing::square(0.0, 0.0, 5.0));
        let group = doc.next_group();
        c.set_group(id, Some(group));
        c.set_text(group, Some(serde_json::json!({ "text": "I" })));
        c.select([id]);
        doc.apply(&c.finish()).unwrap();
        doc
    }

    cut_craft_core::contract_tests!(ConvertTextToCurves, text);

    #[test]
    fn text_becomes_curves_still_grouped() {
        let mut doc = text();
        run_on::<ConvertTextToCurves>(&mut doc, &NoParams {});
        assert!(doc.texts.is_empty(), "no longer text");
        assert_eq!(doc.vector_groups.len(), 1, "still grouped");
        assert_eq!(doc.vectors.len(), 1);
    }

    #[test]
    fn needs_selected_text() {
        assert!(rejection::<ConvertTextToCurves>(&NoParams {}).contains("select the text"));
    }
}
