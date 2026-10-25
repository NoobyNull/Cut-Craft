//! Offset Model: a new component holding the visible model grown (or shrunk) by a ball of the
//! offset's radius in 3D; the components it came from are hidden.

use cut_craft_core::component::{Combine, Component, model_at_resolution};
use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct OffsetModel;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// Outwards when positive, inwards when negative
    #[schemars(title = "Offset distance", extend("x-unit" = "mm"))]
    pub distance: f64,
    /// Nothing ends up below the zero plane
    #[schemars(title = "Clip Z values to zero plane")]
    pub clip_to_zero_plane: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            distance: 1.0,
            clip_to_zero_plane: true,
        }
    }
}

impl Command for OffsetModel {
    const ID: &'static str = "offset_model";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Offset Model",
            group: "Modeling",
            icon: "modeling/offset_model.svg",
            shortcut: "",
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_possible_wrap,
        clippy::cast_precision_loss,
        reason = "cell counts and reaches are small"
    )]
    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if p.distance == 0.0 {
            return Err(CommandError::InvalidParams("distance must not be 0".into()));
        }
        let visible: Vec<_> = doc
            .components
            .iter()
            .filter(|(_, c)| c.visible)
            .map(|(&id, c)| (id, c.clone()))
            .collect();
        if visible.is_empty() {
            return Err(CommandError::Failed(
                "there is no visible model to offset".into(),
            ));
        }
        let model = model_at_resolution(doc);
        let (cols, rows, cell) = (model.columns as i64, model.rows as i64, model.cell);
        let r = p.distance.abs();
        let reach = (r / cell).floor() as i64;
        // the ball's height over its centre at each offset within reach
        let ball: Vec<(i64, i64, f64)> = (-reach..=reach)
            .flat_map(|dy| (-reach..=reach).map(move |dx| (dx, dy)))
            .filter_map(|(dx, dy)| {
                let d2 = ((dx * dx + dy * dy) as f64) * cell * cell;
                (d2 <= r * r).then(|| (dx, dy, (r * r - d2).sqrt()))
            })
            .collect();
        let grow = p.distance > 0.0;
        let mut out = model.clone();
        for row in 0..rows {
            for col in 0..cols {
                let mut best = if grow {
                    f64::NEG_INFINITY
                } else {
                    f64::INFINITY
                };
                for &(dx, dy, lift) in &ball {
                    let (c, rr) = (col + dx, row + dy);
                    // off the job the model is its flat base at 0
                    let h = if (0..cols).contains(&c) && (0..rows).contains(&rr) {
                        model.heights[(rr * cols + c) as usize]
                    } else {
                        0.0
                    };
                    best = if grow {
                        best.max(h + lift)
                    } else {
                        best.min(h - lift)
                    };
                }
                let z = if p.clip_to_zero_plane {
                    best.max(0.0)
                } else {
                    best
                };
                out.heights[(row * cols + col) as usize] = z;
            }
        }
        let mut c = Change::builder(doc);
        for (id, mut k) in visible {
            k.visible = false;
            c.replace_component(id, k);
        }
        c.add_component(Component::from_material("Offset", Combine::Add, &out));
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Point;
    use cut_craft_core::testing::{domed, run_on};

    cut_craft_core::contract_tests!(OffsetModel, domed);

    fn offset(distance: f64, clip: bool) -> Document {
        let mut doc = domed();
        run_on::<OffsetModel>(
            &mut doc,
            &Params {
                distance,
                clip_to_zero_plane: clip,
            },
        );
        doc
    }

    #[test]
    fn growing_raises_the_dome_and_hides_the_original() {
        let doc = offset(1.0, true);
        assert_eq!(doc.components.len(), 2);
        assert!(!doc.components.values().next().unwrap().visible);
        let made = doc.components.values().last().unwrap();
        assert!(made.visible);
        let top = made.height_at(Point::new(20.0, 20.0)).unwrap();
        assert!((top - 9.0).abs() < 0.05, "8 high dome + 1: {top}");
        // the flat base grows by the distance too
        let base = made.height_at(Point::new(2.0, 2.0)).unwrap();
        assert!((base - 1.0).abs() < 1e-9);
        // a sphere of radius 8 grown by 1 is a sphere of radius 9
        let side = made.height_at(Point::new(28.5, 20.0)).unwrap();
        // within a model cell of the true sphere (the cells are about 0.1 mm)
        assert!((side - (81.0_f64 - 72.25).sqrt()).abs() < 0.3, "{side}");
    }

    #[test]
    fn shrinking_lowers_it_and_clipping_keeps_it_above_zero() {
        let clipped = offset(-1.0, true);
        let made = clipped.components.values().last().unwrap();
        let top = made.height_at(Point::new(20.0, 20.0)).unwrap();
        assert!((top - 7.0).abs() < 0.05, "{top}");
        assert!(made.height_range().unwrap().0 >= 0.0);
        let free = offset(-1.0, false);
        let low = free
            .components
            .values()
            .last()
            .unwrap()
            .height_range()
            .unwrap()
            .0;
        assert!(
            (low + 1.0).abs() < 1e-9,
            "unclipped base sinks to -1: {low}"
        );
    }

    #[test]
    fn needs_a_distance_and_a_model() {
        assert!(
            cut_craft_core::testing::rejection::<OffsetModel>(&Params::default()).contains("model")
        );
        let p = Params {
            distance: 0.0,
            ..Params::default()
        };
        assert!(OffsetModel::run(&domed(), &p).is_err());
    }
}
