//! Make Construction Line: turns the selected vectors into construction lines, drawn dashed,
//! that place things (Embellish along them, bend a drawing along them, snap to them) but are
//! never cut and never exported. Make Real Line turns them back.

use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Form, Meta, NoParams};

/// Turns the selected vectors into construction lines.
pub struct MakeConstructionLine;

/// Turns the selected construction lines back into real vectors, that toolpaths can cut.
pub struct MakeRealLine;

impl Command for MakeConstructionLine {
    const ID: &'static str = "make_construction_line";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Make Construction Line",
            group: "Edit Objects",
            icon: "",
            shortcut: "",
            form: Form::Apply,
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        set(doc, true)
    }
}

impl Command for MakeRealLine {
    const ID: &'static str = "make_real_line";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Make Real Line",
            group: "Edit Objects",
            icon: "",
            shortcut: "",
            form: Form::Apply,
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        set(doc, false)
    }
}

/// Every selected vector made a construction line (`construction`) or a real one.
fn set(doc: &Document, construction: bool) -> Result<Change, CommandError> {
    let picked: Vec<_> = doc
        .selection
        .iter()
        .copied()
        .filter(|id| doc.vectors.contains_key(id))
        .collect();
    if picked.is_empty() {
        return Err(CommandError::NothingSelected);
    }
    if picked
        .iter()
        .all(|&id| doc.is_construction(id) == construction)
    {
        return Err(CommandError::Failed(
            if construction {
                "the selected vectors are construction lines already. Select a solid line to \
                 make it one"
            } else {
                "the selected vectors are real lines already. Select a dashed construction \
                 line to make it real"
            }
            .into(),
        ));
    }
    let mut c = Change::builder(doc);
    for id in picked {
        c.set_construction(id, construction);
    }
    Ok(c.finish())
}

/// Whether the right-click menu should offer Make Real Line rather than Make Construction
/// Line: every selected vector is a construction line already.
pub fn all_construction(doc: &Document) -> bool {
    !doc.selection.is_empty() && doc.selection.iter().all(|id| doc.is_construction(*id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::document::EntityId;
    use cut_craft_core::geometry::{Path, Point};
    use cut_craft_core::testing::{document_with, run_on, sample_document, square};
    use cut_craft_core::{Change, History};

    fn constructed() -> Document {
        let mut doc = sample_document();
        run_on::<MakeConstructionLine>(&mut doc, &NoParams {});
        doc
    }

    cut_craft_core::contract_tests!(MakeConstructionLine, sample_document);

    mod real {
        use super::*;
        cut_craft_core::contract_tests!(MakeRealLine, constructed);
    }

    fn line() -> Path {
        Path::polyline(&[Point::new(0.0, 0.0), Point::new(100.0, 0.0)], false)
    }

    #[test]
    fn makes_the_selected_vectors_construction_lines_and_back() {
        let mut doc = document_with(vec![line(), square(0.0, 10.0, 10.0)]);
        let ids: Vec<EntityId> = doc.vectors.keys().copied().collect();
        run_on::<MakeConstructionLine>(&mut doc, &NoParams {});
        assert!(ids.iter().all(|&id| doc.is_construction(id)));
        assert!(all_construction(&doc));
        run_on::<MakeRealLine>(&mut doc, &NoParams {});
        assert!(doc.construction.is_empty());
        assert!(!all_construction(&doc));
    }

    #[test]
    fn undo_turns_it_back() {
        let mut doc = document_with(vec![line()]);
        let before = doc.clone();
        let mut history = History::default();
        let change = MakeConstructionLine::run(&doc, &NoParams {}).unwrap();
        history.apply(&mut doc, change).unwrap();
        assert_eq!(doc.construction.len(), 1);
        assert!(history.undo(&mut doc));
        assert_eq!(doc, before);
    }

    #[test]
    fn says_so_when_there_is_nothing_to_change() {
        let doc = document_with(vec![line()]);
        let err = MakeRealLine::run(&doc, &NoParams {}).unwrap_err();
        assert!(err.to_string().contains("real lines already"), "{err}");
        let empty = Document::default();
        assert_eq!(
            MakeConstructionLine::run(&empty, &NoParams {}).unwrap_err(),
            CommandError::NothingSelected
        );
    }

    #[test]
    fn a_deleted_construction_line_leaves_no_mark_for_the_next_vector() {
        let mut doc = document_with(vec![line()]);
        run_on::<MakeConstructionLine>(&mut doc, &NoParams {});
        let id = *doc.vectors.keys().next().unwrap();
        let mut c = Change::builder(&doc);
        c.remove(id);
        doc.apply(&c.finish()).unwrap();
        assert!(doc.construction.is_empty());
        // the next vector drawn may take the same number: it is a real one
        let mut c = Change::builder(&doc);
        let fresh = c.add(line());
        doc.apply(&c.finish()).unwrap();
        assert!(!doc.is_construction(fresh));
    }

    #[test]
    fn copies_of_a_construction_line_are_construction_lines() {
        let mut doc = document_with(vec![line()]);
        run_on::<MakeConstructionLine>(&mut doc, &NoParams {});
        let id = *doc.vectors.keys().next().unwrap();
        let mut c = Change::builder(&doc);
        let copies = c.copies(
            &doc,
            &[id],
            &[glam::DAffine2::from_translation(glam::DVec2::new(
                0.0, 20.0,
            ))],
        );
        doc.apply(&c.finish()).unwrap();
        assert!(copies.iter().all(|&id| doc.is_construction(id)));
    }

    #[test]
    fn an_edited_construction_line_stays_one() {
        let mut doc = document_with(vec![line()]);
        run_on::<MakeConstructionLine>(&mut doc, &NoParams {});
        let id = *doc.vectors.keys().next().unwrap();
        let mut c = Change::builder(&doc);
        c.replace(
            id,
            Path::polyline(&[Point::new(0.0, 0.0), Point::new(50.0, 50.0)], false),
        );
        doc.apply(&c.finish()).unwrap();
        assert!(doc.is_construction(id));
    }

    #[test]
    fn hidden_construction_lines_are_neither_drawn_nor_picked() {
        let mut doc = document_with(vec![line(), square(0.0, 10.0, 10.0)]);
        let ids: Vec<EntityId> = doc.vectors.keys().copied().collect();
        let mut c = Change::builder(&doc);
        c.set_construction(ids[0], true);
        doc.apply(&c.finish()).unwrap();
        assert!(doc.shown(ids[0]) && doc.editable(ids[0]));
        doc.hide_construction = true;
        assert!(!doc.shown(ids[0]) && !doc.editable(ids[0]));
        assert!(doc.shown(ids[1]) && doc.editable(ids[1]), "real ones stay");
    }

    #[test]
    fn construction_lines_are_saved_with_the_job_but_hiding_them_is_not() {
        let mut doc = constructed();
        doc.hide_construction = true;
        let text = serde_json::to_string(&doc).unwrap();
        let back: Document = serde_json::from_str(&text).unwrap();
        assert_eq!(back.construction, doc.construction);
        assert!(!back.hide_construction);
        // a job with none says nothing about them
        let plain = serde_json::to_string(&sample_document()).unwrap();
        assert!(!plain.contains("construction"));
    }

    #[test]
    fn exported_files_leave_construction_lines_out() {
        let mut doc = document_with(vec![line(), square(0.0, 10.0, 10.0)]);
        let ids: Vec<EntityId> = doc.vectors.keys().copied().collect();
        let mut c = Change::builder(&doc);
        c.set_construction(ids[0], true);
        doc.apply(&c.finish()).unwrap();
        let out = doc.without_construction();
        assert_eq!(out.vectors.keys().copied().collect::<Vec<_>>(), [ids[1]]);
        assert!(out.construction.is_empty());
        assert!(!out.selection.contains(&ids[0]));
    }
}
