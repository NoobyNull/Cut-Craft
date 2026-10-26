//! Activate Sheet: work on another sheet; the job takes its setup.

use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct ActivateSheet;

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// The sheet's place in the Sheets list, from 0.
    pub sheet: usize,
}

impl Command for ActivateSheet {
    const ID: &'static str = "activate_sheet";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Activate Sheet",
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
        let mut c = Change::builder(doc);
        c.activate_sheet(p.sheet);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{rejection, run_on, two_sheets};

    cut_craft_core::contract_tests!(ActivateSheet, two_sheets);

    #[test]
    fn the_job_takes_the_sheets_setup() {
        let mut doc = two_sheets();
        run_on::<ActivateSheet>(&mut doc, &Params { sheet: 1 });
        assert_eq!(doc.active_sheet, 1);
        assert_eq!((doc.job.width, doc.job.thickness), (300.0, 12.0));
    }

    #[test]
    fn needs_a_sheet_that_exists() {
        assert!(rejection::<ActivateSheet>(&Params { sheet: 5 }).contains("no sheet 6"));
    }
}
