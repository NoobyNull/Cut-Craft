//! Delete Sheet: removes a sheet; a job keeps at least one. Deleting the active sheet
//! activates the one that takes its place.

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
        let mut sheets = doc.sheets.clone();
        // the active sheet's setup is the job's
        sheets[doc.active_sheet].job = doc.job;
        sheets.remove(p.sheet);
        let active = match doc.active_sheet {
            a if a > p.sheet => a - 1,
            a if a == p.sheet => a.min(sheets.len() - 1),
            a => a,
        };
        let mut c = Change::builder(doc);
        c.set_job(sheets[active].job);
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
    fn the_last_sheet_stays() {
        assert!(rejection::<DeleteSheet>(&Params { sheet: 0 }).contains("at least one sheet"));
    }
}
