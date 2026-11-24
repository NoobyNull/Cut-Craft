//! Notes: rich text kept with the job (headings, lists, check boxes, links, bold and italic,
//! never HTML) with live variables (see [`cut_craft_core::notes`]). Posts print it as
//! `[FILE_NOTES]`; notes starting with "." open by themselves when the job is opened, and
//! `in_gcode` puts them at the top of the G-code.

use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Form, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct Notes;

/// Notes kept with the job: rich text with headings, lists, check boxes and links, and live
/// values from the job, the machine and the toolpaths, shown on the job sheet and, if you
/// like, at the top of the G-code.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// The notes, written like Markdown: `# heading`, `- bullet`, `1. numbered`,
    /// `- [ ] check box`, `**bold**`, `*italic*`, `[words](https://…)` and `{{variable}}`.
    /// To have notes open when the file is loaded, start them with a "."
    #[schemars(extend("x-widget" = "multiline"))]
    pub notes: String,
    /// Put the notes, variables filled in, at the top of the G-code as comments
    #[serde(default)]
    #[schemars(title = "Put these notes at the top of the G-code")]
    pub in_gcode: bool,
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
            form: Form::Apply,
        }
    }

    fn initial_params(doc: &Document) -> Params {
        Params {
            notes: doc.notes.clone(),
            in_gcode: doc.notes_in_gcode,
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let mut c = Change::builder(doc);
        c.set_notes(&p.notes);
        c.set_notes_in_gcode(p.in_gcode);
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
            in_gcode: true,
        });
        assert_eq!(doc.notes, ".Glue up first");
        assert!(doc.notes_in_gcode);
        assert_eq!(Notes::initial_params(&doc).notes, ".Glue up first");
        assert!(Notes::initial_params(&doc).in_gcode);
        assert_eq!(Notes::initial_params(&sample_document()).notes, "");
    }

    #[test]
    fn notes_sent_without_the_g_code_setting_leave_it_off() {
        // a script or the Web API sending only the text, as before rich notes
        let p: Params = serde_json::from_value(serde_json::json!({ "notes": "x" })).unwrap();
        assert!(!p.in_gcode);
    }

    #[test]
    fn old_jobs_load_their_notes_and_save_them_as_before() {
        let mut doc = sample_document();
        doc.notes = "Plain old notes".into();
        let saved = cut_craft_core::file::save_job(&doc);
        let text = String::from_utf8_lossy(&saved);
        assert!(!text.contains("notes_in_gcode"), "left out while off");
        let mut c = Change::builder(&doc);
        c.set_notes_in_gcode(true);
        let change = c.finish();
        let undo = change.inverse();
        doc.apply(&change).unwrap();
        assert!(doc.notes_in_gcode);
        doc.apply(&undo).unwrap();
        assert!(!doc.notes_in_gcode);
    }
}
