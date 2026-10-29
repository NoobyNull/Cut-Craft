//! Delete Sheet: removes a sheet and what is on it; a job keeps at least one. Deleting the
//! active sheet activates the one that takes its place.

use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct DeleteSheet;

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// The sheet's place in the Sheets list, from 0.
    pub sheet: usize,
}

impl Command for DeleteSheet {
    const ID: &'static str = "delete_sheet";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Delete Sheet",
            group: "Sheets",
            icon: "",
            shortcut: "",
        }
    }

    fn initial_params(doc: &Document) -> Params {
        Params {
            sheet: doc.active_sheet,
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if p.sheet >= doc.sheets.len() {
            return Err(CommandError::InvalidParams(format!(
                "there is no sheet {}",
                p.sheet + 1
            )));
        }
        if doc.sheets.len() == 1 {
            return Err(CommandError::Failed(
                "a job needs at least one sheet".into(),
            ));
        }
        let mut c = Change::builder(doc);
        let mut active = doc.active_sheet;
        if p.sheet == active {
            // the next sheet takes its place, else the one before
            active = if p.sheet + 1 < doc.sheets.len() {
                p.sheet + 1
            } else {
                p.sheet - 1
            };
            c.activate_sheet(active);
        }
        let mut sheets = c.sheets().to_vec();
        sheets.remove(p.sheet);
        if active > p.sheet {
            active -= 1;
        }
        c.set_sheets(sheets, active);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{rejection, run_on, two_sheets};

    cut_craft_core::contract_tests!(DeleteSheet, two_sheets);

    #[test]
    fn deleting_the_active_sheet_activates_the_next() {
        let mut doc = two_sheets();
        run_on::<DeleteSheet>(&mut doc, &Params { sheet: 0 });
        assert_eq!(doc.sheets.len(), 1);
        assert_eq!(doc.sheets[0].name, "Sheet 2");
        assert_eq!((doc.active_sheet, doc.job.width), (0, 300.0));
    }

    #[test]
    fn deleting_another_keeps_the_active_one_and_its_setup() {
        let mut doc = two_sheets();
        let job = doc.job;
        run_on::<DeleteSheet>(&mut doc, &Params { sheet: 1 });
        assert_eq!((doc.sheets.len(), doc.active_sheet), (1, 0));
        assert_eq!(doc.job, job);
    }

    #[test]
    fn the_sheets_objects_go_with_it() {
        let mut doc = two_sheets();
        let drawn = doc.vectors.len();
        assert!(drawn > 0, "the sample document has vectors");
        run_on::<DeleteSheet>(&mut doc, &Params { sheet: 0 });
        assert!(doc.vectors.is_empty(), "sheet 2's (none) are in");
        assert!(doc.sheets.iter().all(|s| s.objects.is_empty()));
    }

    #[test]
    fn the_last_sheet_stays() {
        assert!(rejection::<DeleteSheet>(&Params { sheet: 0 }).contains("at least one sheet"));
    }
}
