//! Array Copy: a grid of copies of the selected vectors, above and to the right of them.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::Point;
use cut_craft_core::{Change, Command, CommandError, Meta};
use glam::DAffine2;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct ArrayCopy;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Spacing {
    /// X and Y are the space between neighbouring copies
    #[default]
    Gap,
    /// X and Y are from one copy's start to the next one's
    Offset,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// Copies up the job, the original's row included
    #[schemars(range(min = 1))]
    pub rows: u32,
    /// Copies across the job, the original's column included
    #[schemars(range(min = 1))]
    pub columns: u32,
    pub spacing: Spacing,
    #[schemars(extend("x-unit" = "mm"))]
    pub x: f64,
    #[schemars(extend("x-unit" = "mm"))]
    pub y: f64,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            rows: 2,
            columns: 2,
            spacing: Spacing::Gap,
            x: 5.0,
            y: 5.0,
        }
    }
}

impl Command for ArrayCopy {
    const ID: &'static str = "array_copy";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Array Copy",
            group: "Array Objects",
            icon: "drawing/create_vectors/array_copy.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if p.rows == 0 || p.columns == 0 {
            return Err(CommandError::InvalidParams(
                "rows and columns must be at least 1".into(),
            ));
        }
        if p.rows * p.columns < 2 {
            return Err(CommandError::InvalidParams(
                "one row of one column makes no copies".into(),
            ));
        }
        let Some(bounds) = doc.selection_bounds() else {
            return Err(CommandError::Failed("select vectors to copy".into()));
        };
        let step = match p.spacing {
            Spacing::Offset => Point::new(p.x, p.y),
            Spacing::Gap => bounds.size() + Point::new(p.x, p.y),
        };
        let mut transforms = vec![];
        for row in 0..p.rows {
            for column in 0..p.columns {
                if row > 0 || column > 0 {
                    let at = Point::new(f64::from(column), f64::from(row)) * step;
                    transforms.push(DAffine2::from_translation(at));
                }
            }
        }
        Ok(Change::copy_selected(doc, &transforms))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{document_with, square, two_layers};

    cut_craft_core::contract_tests!(ArrayCopy);

    fn corners(doc: &Document) -> Vec<(f64, f64)> {
        let mut at: Vec<(f64, f64)> = doc
            .vectors
            .values()
            .map(|p| (p.bounds().min.x, p.bounds().min.y))
            .collect();
        at.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
        at
    }

    fn copied(p: &Params) -> Document {
        let mut doc = document_with(vec![square(0.0, 0.0, 10.0)]);
        doc.apply(&ArrayCopy::run(&doc, p).unwrap()).unwrap();
        doc
    }

    #[test]
    fn offsets_space_copies_start_to_start_up_and_to_the_right() {
        let doc = copied(&Params {
            rows: 2,
            columns: 3,
            spacing: Spacing::Offset,
            x: 20.0,
            y: 15.0,
        });
        assert_eq!(
            corners(&doc),
            [
                (0.0, 0.0),
                (0.0, 15.0),
                (20.0, 0.0),
                (20.0, 15.0),
                (40.0, 0.0),
                (40.0, 15.0)
            ]
        );
        assert_eq!(doc.selection.len(), 6, "the original and every copy");
    }

    #[test]
    fn gaps_space_copies_between_their_edges() {
        let doc = copied(&Params::default());
        assert_eq!(
            corners(&doc),
            [(0.0, 0.0), (0.0, 15.0), (15.0, 0.0), (15.0, 15.0)]
        );
    }

    #[test]
    fn copies_stay_on_their_originals_layers() {
        let mut doc = two_layers();
        let before: Vec<_> = doc.selection.iter().map(|id| doc.layer_of(*id)).collect();
        let change = ArrayCopy::run(
            &doc,
            &Params {
                rows: 1,
                ..Params::default()
            },
        )
        .unwrap();
        doc.apply(&change).unwrap();
        let after: Vec<_> = doc.selection.iter().map(|id| doc.layer_of(*id)).collect();
        assert_eq!(after.len(), before.len() * 2);
        assert_eq!(after[..before.len()], before[..]);
        assert_eq!(after[before.len()..], before[..]);
    }

    #[test]
    fn needs_a_selection_and_more_than_one_place() {
        let doc = document_with(vec![square(0.0, 0.0, 10.0)]);
        let one = Params {
            rows: 1,
            columns: 1,
            ..Params::default()
        };
        assert!(ArrayCopy::run(&doc, &one).is_err());
        assert!(ArrayCopy::run(&Document::default(), &Params::default()).is_err());
    }
}
