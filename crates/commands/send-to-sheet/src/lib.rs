//! Copy / Move to Sheet: the selected vectors onto another sheet, each keeping its layer and
//! group; moving takes them off this one.

use cut_craft_core::document::{Document, EntityId, StoredVector};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct SendToSheet;

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// The sheet's place in the Sheets list, from 0.
    pub sheet: usize,
    /// Take the vectors off this sheet (else copies are sent)
    #[serde(default)]
    #[schemars(title = "Move")]
    pub move_vectors: bool,
}

impl Command for SendToSheet {
    const ID: &'static str = "send_to_sheet";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Copy / Move to Sheet",
            group: "Sheets",
            icon: "",
            shortcut: "",
        }
    }

    fn initial_params(doc: &Document) -> Params {
        Params {
            sheet: usize::from(doc.active_sheet == 0),
            move_vectors: false,
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if p.sheet >= doc.sheets.len() {
            return Err(cut_craft_core::gone("sheet"));
        }
        if p.sheet == doc.active_sheet {
            return Err(CommandError::Failed(
                "the vectors are on that sheet already. Pick another sheet".into(),
            ));
        }
        if doc.selection.is_empty() {
            return Err(CommandError::NothingSelected);
        }
        let mut sheets = doc.sheets.clone();
        // ids are unique over every sheet
        for (next, &id) in (doc.next_id().0..).zip(&doc.selection) {
            sheets[p.sheet].objects.vectors.insert(
                EntityId(next),
                StoredVector {
                    path: doc.vectors[&id].clone(),
                    layer: Some(doc.layer_of(id)),
                    group: doc.vector_groups.get(&id).copied(),
                },
            );
        }
        let mut c = Change::builder(doc);
        if p.move_vectors {
            for &id in &doc.selection {
                c.remove(id);
            }
        }
        c.set_sheets(sheets, doc.active_sheet);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::document::Sheet;

    /// The sample document on its second sheet, the first empty.
    fn two_sheets() -> Document {
        let mut doc = cut_craft_core::testing::sample_document();
        let mut sheets = vec![Sheet::new("Empty", doc.job)];
        sheets.extend(doc.sheets.clone());
        let mut c = Change::builder(&doc);
        c.set_sheets(sheets, 1);
        doc.apply(&c.finish()).unwrap();
        doc
    }

    fn sent(move_vectors: bool) -> (Document, Document) {
        let before = two_sheets();
        let mut after = before.clone();
        let p = Params {
            sheet: 0,
            move_vectors,
        };
        after
            .apply(&SendToSheet::run(&before, &p).unwrap())
            .unwrap();
        (before, after)
    }

    cut_craft_core::contract_tests!(SendToSheet, two_sheets);

    #[test]
    fn copies_arrive_on_the_sheet_and_the_originals_stay() {
        let (before, after) = sent(false);
        let arrived = &after.sheets[0].objects.vectors;
        assert_eq!(arrived.len(), before.selection.len());
        assert_eq!(after.vectors, before.vectors);
        // they come back when that sheet is active, with ids no other object has
        let mut c = Change::builder(&after);
        c.activate_sheet(0);
        let mut there = after.clone();
        there.apply(&c.finish()).unwrap();
        assert_eq!(there.vectors.len(), before.selection.len());
        assert!(
            there
                .vectors
                .keys()
                .all(|id| !before.vectors.contains_key(id))
        );
    }

    #[test]
    fn moving_takes_them_off_this_sheet_and_the_active_sheet_is_refused() {
        let (before, after) = sent(true);
        assert_eq!(
            after.vectors.len(),
            before.vectors.len() - before.selection.len()
        );
        let p = Params {
            sheet: before.active_sheet,
            move_vectors: true,
        };
        assert!(SendToSheet::run(&before, &p).is_err());
    }
}
