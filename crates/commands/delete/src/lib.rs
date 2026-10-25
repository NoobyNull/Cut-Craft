//! Delete.

use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta, NoParams};

pub struct Delete;

impl Command for Delete {
    const ID: &'static str = "delete";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Delete",
            group: "Edit",
            icon: "",
            shortcut: "Del",
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        if doc.selection.is_empty() && doc.selected_bitmaps.is_empty() {
            return Err(CommandError::NothingSelected);
        }
        let mut c = Change::builder(doc);
        for &id in &doc.selection {
            c.remove(id);
        }
        for &id in doc
            .selected_bitmaps
            .iter()
            .filter(|id| doc.bitmaps.contains_key(id))
        {
            c.remove_bitmap(id);
        }
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    cut_craft_core::contract_tests!(Delete);

    #[test]
    fn removes_selected_bitmaps_too() {
        use cut_craft_core::bitmap::Bitmap;
        use cut_craft_core::geometry::{Point, Rect};
        let mut doc = cut_craft_core::document::Document::default();
        let mut c = Change::builder(&doc);
        let id = c.add_bitmap(Bitmap {
            name: "b".into(),
            columns: 1,
            rows: 1,
            grey: vec![0],
            rect: Rect::new(Point::ZERO, Point::splat(10.0)),
        });
        let change = c.finish();
        doc.apply(&change).unwrap();
        doc.selected_bitmaps = [id].into();
        let change = Delete::run(&doc, &NoParams {}).unwrap();
        doc.apply(&change).unwrap();
        assert!(doc.bitmaps.is_empty() && doc.selected_bitmaps.is_empty());
        doc.apply(&change.inverse()).unwrap();
        assert_eq!(doc.bitmaps.len(), 1, "undo brings it back");
    }

    #[test]
    fn removes_the_selected_vectors() {
        assert!(
            cut_craft_core::testing::after::<Delete>(&NoParams {})
                .vectors
                .is_empty()
        );
    }

    #[test]
    fn nothing_selected_is_an_error() {
        assert_eq!(
            Delete::run(&Document::default(), &NoParams {}),
            Err(CommandError::NothingSelected)
        );
    }
}
