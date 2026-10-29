//! Overlap.

use cut_craft_core::document::Document;
use cut_craft_core::polygon;
use cut_craft_core::{Change, Command, CommandError, Meta, NoParams};

pub struct OverlapIntersect;

impl Command for OverlapIntersect {
    const ID: &'static str = "overlap_intersect";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Overlap",
            group: "Edit Objects",
            icon: "drawing/weld_and_offset/overlap_intersect.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        let (ids, paths) = doc.selected_paths(|p| p.closed);
        if ids.len() < 2 {
            return Err(CommandError::Failed(
                "select at least two closed vectors to overlap".into(),
            ));
        }
        let common = paths[1..].iter().fold(vec![paths[0].clone()], |acc, p| {
            polygon::intersection(&acc, std::slice::from_ref(p))
        });
        if common.is_empty() {
            return Err(CommandError::Failed(
                "the selected vectors do not overlap".into(),
            ));
        }
        Ok(Change::swap(doc, ids, common))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use cut_craft_core::testing::{document_with, square};

    fn overlapping() -> Document {
        document_with(vec![square(0.0, 0.0, 10.0), square(5.0, 5.0, 10.0)])
    }

    #[test]
    fn keeps_only_the_shared_region() {
        let mut doc = overlapping();
        doc.apply(&OverlapIntersect::run(&doc, &NoParams {}).unwrap())
            .unwrap();
        let area: f64 = doc
            .vectors
            .values()
            .map(cut_craft_core::polygon::area)
            .sum();
        assert!((area - 25.0).abs() < 1e-6, "{area}");
    }

    #[test]
    fn shapes_that_do_not_touch_are_reported() {
        let doc = document_with(vec![square(0.0, 0.0, 10.0), square(50.0, 0.0, 10.0)]);
        assert_eq!(
            OverlapIntersect::run(&doc, &NoParams {})
                .unwrap_err()
                .to_string(),
            "the selected vectors do not overlap"
        );
    }

    cut_craft_core::contract_tests!(OverlapIntersect, overlapping);
}
