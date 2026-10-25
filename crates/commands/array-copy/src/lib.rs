//! Array Copy: copies of the selection in rows and columns, spaced by a gap between them or
//! an offset from one to the next, every other row or column displaced if asked.

use cut_craft_core::document::Document;
use cut_craft_core::glam::{DAffine2, DVec2};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct ArrayCopy;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Spacing {
    /// X and Y are the gaps between the copies' edges
    #[default]
    Gap,
    /// X and Y are how far each copy is from the one before
    Offset,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    #[schemars(title = "Rows (Y)", range(min = 1, max = 1000))]
    pub rows: u32,
    #[schemars(title = "Columns (X)", range(min = 1, max = 1000))]
    pub columns: u32,
    #[schemars(extend("x-widget" = "row"))]
    pub spacing: Spacing,
    #[schemars(extend("x-unit" = "mm"))]
    pub x: f64,
    #[schemars(extend("x-unit" = "mm"))]
    pub y: f64,
    /// Every other row moves across by this much
    #[schemars(title = "Row displacement", extend("x-unit" = "mm"))]
    pub row_displacement: f64,
    /// Every other column moves up by this much
    #[schemars(title = "Column displacement", extend("x-unit" = "mm"))]
    pub column_displacement: f64,
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
            icon: "drawing/create_vectors/array_copy.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let Some(bounds) = doc.selection_bounds() else {
            return Err(CommandError::NothingSelected);
        };
        if p.rows * p.columns < 2 {
            return Err(CommandError::InvalidParams(
                "make at least two rows or columns".into(),
            ));
        }
        let step = match p.spacing {
            Spacing::Gap => bounds.size() + DVec2::new(p.x, p.y),
            Spacing::Offset => DVec2::new(p.x, p.y),
        };
        let places: Vec<DAffine2> = (0..p.rows)
            .flat_map(|r| (0..p.columns).map(move |c| (r, c)))
            .filter(|&rc| rc != (0, 0))
            .map(|(r, c)| {
                let odd = |k: u32| f64::from(k % 2);
                DAffine2::from_translation(DVec2::new(
                    f64::from(c) * step.x + odd(r) * p.row_displacement,
                    f64::from(r) * step.y + odd(c) * p.column_displacement,
                ))
            })
            .collect();
        Change::copy_selected(doc, &places, p.group_copies)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Point;
    use cut_craft_core::testing::{document_with, run_on, square};

    cut_craft_core::contract_tests!(ArrayCopy);

    fn corners(doc: &Document) -> Vec<Point> {
        let mut v: Vec<Point> = doc.vectors.values().map(|p| p.bounds().min).collect();
        v.sort_by(|a, b| (a.y, a.x).partial_cmp(&(b.y, b.x)).unwrap());
        v
    }

    #[test]
    fn gaps_lay_a_grid_of_copies_and_group_them() {
        let mut doc = document_with(vec![square(0.0, 0.0, 10.0)]);
        run_on::<ArrayCopy>(
            &mut doc,
            &Params {
                rows: 2,
                columns: 3,
                group_copies: true,
                ..Params::default()
            },
        );
        assert_eq!(doc.vectors.len(), 6);
        let want = [
            (0.0, 0.0),
            (15.0, 0.0),
            (30.0, 0.0),
            (0.0, 15.0),
            (15.0, 15.0),
            (30.0, 15.0),
        ];
        for (got, (x, y)) in corners(&doc).iter().zip(want) {
            assert!((*got - Point::new(x, y)).length() < 1e-9, "{got}");
        }
        assert_eq!(doc.selection.len(), 5, "the copies are selected");
        let copy = *doc.selection.iter().next().unwrap();
        assert_eq!(doc.group_members(copy).len(), 5);
    }

    #[test]
    fn offsets_and_displacement() {
        let mut doc = document_with(vec![square(0.0, 0.0, 10.0)]);
        run_on::<ArrayCopy>(
            &mut doc,
            &Params {
                rows: 2,
                columns: 1,
                spacing: Spacing::Offset,
                x: 0.0,
                y: 20.0,
                row_displacement: 4.0,
                ..Params::default()
            },
        );
        let c = corners(&doc);
        assert!((c[1] - Point::new(4.0, 20.0)).length() < 1e-9, "{c:?}");
        let one = Params {
            rows: 1,
            columns: 1,
            ..Params::default()
        };
        assert!(ArrayCopy::run(&document_with(vec![square(0.0, 0.0, 1.0)]), &one).is_err());
    }
}
