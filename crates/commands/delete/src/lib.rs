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
        let components: Vec<_> = doc
            .selected_components
            .iter()
            .filter(|id| doc.components.contains_key(id))
            .copied()
            .collect();
        if doc.selection.is_empty() && doc.selected_bitmaps.is_empty() && components.is_empty() {
            return Err(CommandError::NothingSelected);
        }
        let mut c = Change::builder(doc);
        for id in components {
            c.remove_component(id);
        }
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
            depth: Vec::new(),
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
    fn removes_selected_components_too() {
        use cut_craft_core::component::{Combine, Component};
        use cut_craft_core::geometry::{Point, Rect};
        let mut doc = cut_craft_core::document::Document::default();
        let mut c = Change::builder(&doc);
        let rect = Rect::new(Point::ZERO, Point::splat(10.0));
        let id = c.add_component(Component::from_fn("k", Combine::Add, rect, 1.0, |_| {
            Some(1.0)
        }));
        doc.apply(&c.finish()).unwrap();
        doc.selected_components = [id].into();
        // issue #4: a component alone selected was "nothing is selected"
        let change = Delete::run(&doc, &NoParams {}).unwrap();
        doc.apply(&change).unwrap();
        assert!(doc.components.is_empty());
        doc.apply(&change.inverse()).unwrap();
        assert_eq!(doc.components.len(), 1, "undo brings it back");
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
