//! Add New Sheet: another material sheet, sized as the one it was asked from (the sheet
//! right-clicked) or else as the active one, which stays active.

use cut_craft_core::document::{Document, Sheet};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_named};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct AddSheet;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
    /// The sheet whose size it takes, from 0 (the active one if none)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(skip)]
    pub like: Option<usize>,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "New Sheet".into(),
            like: None,
        }
    }
}

impl Command for AddSheet {
    const ID: &'static str = "add_sheet";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Add New Sheet",
            group: "Sheets",
            icon: "",
            shortcut: "",
        }
    }

    /// The first "Sheet N" not taken.
    fn initial_params(doc: &Document) -> Params {
        // n sheets take at most n of these n + 1 names
        let n = doc.sheets.len();
        let name = (n + 1..=2 * n + 1)
            .map(|n| format!("Sheet {n}"))
            .find(|name| doc.sheets.iter().all(|s| &s.name != name))
            .expect("some number is free");
        Params { name, like: None }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let name = ensure_named(&p.name)?;
        let mut sheets = doc.sheets.clone();
        let job = p.like.and_then(|i| doc.sheet_job(i)).unwrap_or(doc.job);
        sheets.push(Sheet::new(&name, job));
        let mut c = Change::builder(doc);
        c.set_sheets(sheets, doc.active_sheet);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{after, rejection, two_sheets};

    cut_craft_core::contract_tests!(AddSheet);

    #[test]
    fn adds_a_sheet_sized_as_the_active_one() {
        let doc = after::<AddSheet>(&Params {
            name: " Offcuts ".into(),
            like: None,
        });
        assert_eq!(doc.sheets.len(), 2);
        assert_eq!(doc.sheets[1].name, "Offcuts");
        assert_eq!(doc.sheet_job(1), Some(doc.job));
        assert_eq!(doc.active_sheet, 0, "the active sheet stays");
    }

    #[test]
    fn suggests_the_next_free_number_and_needs_a_name() {
        assert_eq!(AddSheet::initial_params(&two_sheets()).name, "Sheet 3");
        assert!(
            rejection::<AddSheet>(&Params {
                name: " ".into(),
                like: None
            })
            .contains("Type a name")
        );
    }

    #[test]
    fn asked_from_a_sheet_it_takes_that_sheets_size() {
        let mut doc = two_sheets();
        let p = Params {
            name: "Like 2".into(),
            like: Some(1),
        };
        doc.apply(&AddSheet::run(&doc, &p).unwrap()).unwrap();
        assert_eq!(doc.sheet_job(2), doc.sheet_job(1));
        assert_ne!(doc.sheet_job(2), Some(doc.job));
    }
}
