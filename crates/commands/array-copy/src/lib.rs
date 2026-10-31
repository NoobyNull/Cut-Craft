//! Array Copy: a grid of copies of the selected vectors, above and to the right of them,
//! each mirrored or turned in a repeating block (symmetry), every other row or column moved
//! along (displacement), and if asked all grouped.

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

/// How the copies in each 2 × 2 block of the grid are turned: the original's corner as it
/// is, the others mirrored or turned about their own centres.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Symmetry {
    /// Every copy as the original
    #[default]
    None,
    /// Every other column mirrored left to right
    MirrorColumns,
    /// Every other row mirrored top to bottom
    MirrorRows,
    /// Both: a four-way mirrored block
    MirrorBoth,
    /// Every other copy, chequered, turned half round
    Rotate,
}

impl Symmetry {
    /// The turn of the copy in `column`, `row` about the origin: (mirror X, mirror Y).
    fn flips(self, column: u32, row: u32) -> (bool, bool) {
        let (odd_column, odd_row) = (column % 2 == 1, row % 2 == 1);
        match self {
            Self::None => (false, false),
            Self::MirrorColumns => (odd_column, false),
            Self::MirrorRows => (false, odd_row),
            Self::MirrorBoth => (odd_column, odd_row),
            Self::Rotate => (odd_column != odd_row, odd_column != odd_row),
        }
    }
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
    #[serde(default)]
    pub symmetry: Symmetry,
    /// Every other row moved along X by this
    #[serde(default)]
    #[schemars(extend("x-unit" = "mm"))]
    pub row_displacement: f64,
    /// Every other column moved along Y by this
    #[serde(default)]
    #[schemars(extend("x-unit" = "mm"))]
    pub column_displacement: f64,
    /// The original and its copies made one group
    #[serde(default)]
    pub group_copies: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            rows: 2,
            columns: 2,
            spacing: Spacing::Gap,
            x: 5.0,
            y: 5.0,
            symmetry: Symmetry::None,
            row_displacement: 0.0,
            column_displacement: 0.0,
            group_copies: false,
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
        let centre = bounds.center();
        let mut transforms = vec![];
        for row in 0..p.rows {
            for column in 0..p.columns {
                if row > 0 || column > 0 {
                    let mut at = Point::new(f64::from(column), f64::from(row)) * step;
                    at.x += f64::from(row % 2) * p.row_displacement;
                    at.y += f64::from(column % 2) * p.column_displacement;
                    let (x, y) = p.symmetry.flips(column, row);
                    let sign = |flip| if flip { -1.0 } else { 1.0 };
                    let turn = DAffine2::from_translation(centre)
                        * DAffine2::from_scale(Point::new(sign(x), sign(y)))
                        * DAffine2::from_translation(-centre);
                    transforms.push(DAffine2::from_translation(at) * turn);
                }
            }
        }
        if !p.group_copies {
            return Ok(Change::copy_selected(doc, &transforms));
        }
        // as copy_selected, with everything put in one new group
        let group = doc.next_group();
        let mut c = Change::builder(doc);
        let originals: Vec<_> = doc.selection.iter().copied().collect();
        let mut all = originals.clone();
        for t in &transforms {
            for &id in &originals {
                let copy = c.add(doc.vectors[&id].transformed(t));
                c.move_to_layer(copy, doc.layer_of(id));
                all.push(copy);
            }
        }
        for &id in &all {
            c.set_group(id, Some(group));
        }
        c.select(all);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{document_with, square, two_layers};

    cut_craft_core::contract_tests!(ArrayCopy);

    /// An L shape, so a mirror shows: its foot points to +X.
    fn ell() -> cut_craft_core::geometry::Path {
        cut_craft_core::geometry::Path::polyline(
            &[
                Point::new(0.0, 0.0),
                Point::new(10.0, 0.0),
                Point::new(10.0, 2.0),
                Point::new(2.0, 2.0),
                Point::new(2.0, 10.0),
                Point::new(0.0, 10.0),
            ],
            true,
        )
    }

    /// Each copy's start point, relative to its bounds' lower left.
    fn starts(p: &Params) -> Vec<(f64, f64)> {
        let mut doc = document_with(vec![ell()]);
        doc.apply(&ArrayCopy::run(&doc, p).unwrap()).unwrap();
        let mut at: Vec<(f64, f64, f64, f64)> = doc
            .vectors
            .values()
            .map(|v| {
                let b = v.bounds();
                (b.min.x, b.min.y, v.start.x - b.min.x, v.start.y - b.min.y)
            })
            .collect();
        at.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.total_cmp(&b.0)));
        at.iter().map(|a| (a.2.round(), a.3.round())).collect()
    }

    #[test]
    fn symmetry_mirrors_or_turns_copies_in_a_repeating_block() {
        let grid = |symmetry| Params {
            symmetry,
            ..Params::default()
        };
        // the L starts in its lower left corner; a mirror moves the corner
        assert_eq!(starts(&grid(Symmetry::None)), [(0.0, 0.0); 4]);
        assert_eq!(
            starts(&grid(Symmetry::MirrorColumns)),
            [(0.0, 0.0), (10.0, 0.0), (0.0, 0.0), (10.0, 0.0)]
        );
        assert_eq!(
            starts(&grid(Symmetry::MirrorBoth)),
            [(0.0, 0.0), (10.0, 0.0), (0.0, 10.0), (10.0, 10.0)]
        );
        assert_eq!(
            starts(&grid(Symmetry::Rotate)),
            [(0.0, 0.0), (10.0, 10.0), (10.0, 10.0), (0.0, 0.0)]
        );
    }

    #[test]
    fn displacement_moves_every_other_row_and_column_and_copies_can_be_grouped() {
        let doc = copied(&Params {
            row_displacement: 3.0,
            column_displacement: 4.0,
            group_copies: true,
            ..Params::default()
        });
        assert_eq!(
            corners(&doc),
            [(0.0, 0.0), (3.0, 15.0), (15.0, 4.0), (18.0, 19.0)]
        );
        let groups: std::collections::BTreeSet<_> = doc
            .vectors
            .keys()
            .map(|id| doc.vector_groups.get(id).copied())
            .collect();
        assert_eq!(groups.len(), 1, "one group: {groups:?}");
        assert!(groups.iter().all(Option::is_some));
    }

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
            ..Params::default()
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
