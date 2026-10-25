//! Add Draft to Model: a new component holding the visible model with no wall steeper than
//! the draft angle allows (material is added at the foot of steep walls), for moulds.
//! The components it came from stay visible.

use cut_craft_core::component::{Combine, Component, model_at_resolution};
use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct AddDraft;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// The least angle a wall leans from vertical
    #[schemars(extend("x-unit" = "deg"), range(min = 0, max = 45))]
    pub draft_angle: f64,
}

impl Default for Params {
    fn default() -> Self {
        // ponytail: the help's example angle; the dialog's slider runs 0–45°
        Self { draft_angle: 20.0 }
    }
}

impl Command for AddDraft {
    const ID: &'static str = "add_draft";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Add Draft to Model",
            group: "Modeling",
            icon: "modeling/add_draft_to_model.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if !(p.draft_angle > 0.0 && p.draft_angle <= 45.0) {
            return Err(CommandError::InvalidParams(
                "the draft angle is above 0 and at most 45°".into(),
            ));
        }
        if !doc.components.values().any(|c| c.visible) {
            return Err(CommandError::Failed(
                "there is no visible model to add draft to".into(),
            ));
        }
        let mut model = model_at_resolution(doc);
        // the steepest a wall may fall per mm across
        let fall = (90.0 - p.draft_angle).to_radians().tan();
        let (cols, rows) = (model.columns, model.rows);
        let heights = &mut model.heights;
        let (straight, diagonal) = (
            fall * model.cell,
            fall * model.cell * std::f64::consts::SQRT_2,
        );
        // ponytail: two chamfer passes (8 neighbours) give walls within 8 % of the true cone
        let mut lift = |i: usize, j: usize, by: f64| {
            if heights[j] - by > heights[i] {
                heights[i] = heights[j] - by;
            }
        };
        for r in 0..rows {
            for c in 0..cols {
                let i = r * cols + c;
                if c > 0 {
                    lift(i, i - 1, straight);
                }
                if r > 0 {
                    lift(i, i - cols, straight);
                    if c > 0 {
                        lift(i, i - cols - 1, diagonal);
                    }
                    if c + 1 < cols {
                        lift(i, i - cols + 1, diagonal);
                    }
                }
            }
        }
        for r in (0..rows).rev() {
            for c in (0..cols).rev() {
                let i = r * cols + c;
                if c + 1 < cols {
                    lift(i, i + 1, straight);
                }
                if r + 1 < rows {
                    lift(i, i + cols, straight);
                    if c + 1 < cols {
                        lift(i, i + cols + 1, diagonal);
                    }
                    if c > 0 {
                        lift(i, i + cols - 1, diagonal);
                    }
                }
            }
        }
        let name = format!("Draft {}°", p.draft_angle);
        let mut c = Change::builder(doc);
        c.add_component(Component::from_material(&name, Combine::Add, &model));
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::{Point, Rect};
    use cut_craft_core::testing::{domed, run_on};

    cut_craft_core::contract_tests!(AddDraft, domed);

    /// A 10 mm high block with vertical walls, 10 mm square, on a 40 mm job.
    fn block() -> Document {
        let mut doc = domed();
        let k = Component::from_fn(
            "block",
            Combine::Add,
            Rect::new(Point::new(15.0, 15.0), Point::new(25.0, 25.0)),
            0.2,
            |_| Some(10.0),
        );
        doc.components.clear();
        let mut c = Change::builder(&doc);
        c.add_component(k);
        let change = c.finish();
        doc.apply(&change).unwrap();
        doc
    }

    #[test]
    fn vertical_walls_get_a_45_degree_skirt() {
        let mut doc = block();
        run_on::<AddDraft>(&mut doc, &Params { draft_angle: 45.0 });
        assert_eq!(doc.components.len(), 2, "the original stays");
        let made = doc.components.values().last().unwrap();
        assert_eq!(made.name, "Draft 45°");
        // 45°: 2 mm out from the wall it has fallen 2 mm
        let out = made.height_at(Point::new(27.0, 20.0)).unwrap();
        assert!((out - 8.0).abs() < 0.3, "{out}");
        let top = made.height_at(Point::new(20.0, 20.0)).unwrap();
        assert!((top - 10.0).abs() < 1e-9, "the top is untouched");
        let far = made.height_at(Point::new(38.0, 20.0)).unwrap();
        assert!(far.abs() < 1e-9, "the skirt ends 10 mm out");
    }

    #[test]
    fn smaller_angles_make_steeper_skirts() {
        let mut doc = block();
        run_on::<AddDraft>(&mut doc, &Params { draft_angle: 10.0 });
        let made = doc.components.values().last().unwrap();
        // tan 80° ≈ 5.67 mm down per mm out: 1 mm out it is about 4.3 high
        let out = made.height_at(Point::new(26.0, 20.0)).unwrap();
        assert!(
            (out - (10.0 - 80_f64.to_radians().tan())).abs() < 0.8,
            "{out}"
        );
        assert!(AddDraft::run(&block(), &Params { draft_angle: 0.0 }).is_err());
    }
}
