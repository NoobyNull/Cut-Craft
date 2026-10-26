//! Sheet Properties: a sheet's name and whether it shows. Its size and setup are the
//! job's while it is active (Job Setup).

use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_named};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct SheetProperties;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// The sheet's place in the Sheets list, from 0.
    pub sheet: usize,
    pub name: String,
    /// Shown in the 2D view
    pub visible: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            sheet: 0,
            name: "Sheet 1".into(),
            visible: true,
        }
    }
}

impl Command for SheetProperties {
    const ID: &'static str = "sheet_properties";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Sheet Properties",
            group: "Sheets",
            icon: "",
            shortcut: "",
        }
    }

    fn initial_params(doc: &Document) -> Params {
        let sheet = &doc.sheets[doc.active_sheet.min(doc.sheets.len() - 1)];
        Params {
            sheet: doc.active_sheet,
            name: sheet.name.clone(),
            visible: sheet.visible,
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let name = ensure_named(&p.name)?;
        let mut sheets = doc.sheets.clone();
        let Some(sheet) = sheets.get_mut(p.sheet) else {
            return Err(CommandError::InvalidParams(format!(
                "there is no sheet {}",
                p.sheet + 1
            )));
        };
        (sheet.name, sheet.visible) = (name, p.visible);
        let mut c = Change::builder(doc);
        c.set_sheets(sheets, doc.active_sheet);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{rejection, run_on, two_sheets};

    cut_craft_core::contract_tests!(SheetProperties, two_sheets);

    #[test]
    fn renames_and_hides_a_sheet() {
        let mut doc = two_sheets();
        run_on::<SheetProperties>(
            &mut doc,
            &Params {
                sheet: 1,
                name: "Offcuts".into(),
                visible: false,
            },
        );
        assert_eq!(doc.sheets[1].name, "Offcuts");
        assert!(!doc.sheets[1].visible);
        assert_eq!(doc.sheets[0].name, "Sheet 1");
    }

    #[test]
    fn needs_a_name_and_a_sheet() {
        assert!(
            rejection::<SheetProperties>(&Params {
                name: String::new(),
                ..Params::default()
            })
            .contains("name is empty")
        );
        assert!(
            rejection::<SheetProperties>(&Params {
                sheet: 3,
                ..Params::default()
            })
            .contains("no sheet 4")
        );
    }
}
