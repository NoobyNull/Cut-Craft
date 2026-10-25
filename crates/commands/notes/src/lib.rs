//! Notes: free text kept with the job. Posts print it as `[FILE_NOTES]`; notes starting with
//! "." open by themselves when the job is opened.

use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct Notes;

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// To have notes open when the file is loaded, start them with a "."
    #[schemars(extend("x-widget" = "multiline"))]
    pub notes: String,
}

impl Command for Notes {
    const ID: &'static str = "notes";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Notes",
            group: "Edit",
            icon: "",
            shortcut: "",
        }
    }

    fn initial_params(doc: &Document) -> Params {
        Params {
            notes: doc.notes.clone(),
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let mut c = Change::builder(doc);
        c.set_notes(&p.notes);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{after, sample_document};

    cut_craft_core::contract_tests!(Notes);

    #[test]
    fn notes_are_kept_with_the_job_and_shown_again() {
        let doc = after::<Notes>(&Params {
            notes: ".Glue up first".into(),
        });
        assert_eq!(doc.notes, ".Glue up first");
        assert_eq!(Notes::initial_params(&doc).notes, ".Glue up first");
        assert_eq!(Notes::initial_params(&sample_document()).notes, "");
    }
}
