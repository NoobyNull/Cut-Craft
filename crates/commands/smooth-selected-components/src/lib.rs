//! Smooth Selected Components: softens the shape of each component picked in the component
//! tree (the newest when none is) by averaging each height with its neighbours, once per
//! pass.

use cut_craft_core::component::Component;
use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct SmoothSelectedComponents;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// Passes of smoothing: more is smoother
    #[schemars(range(min = 1, max = 50))]
    pub passes: u32,
}

impl Default for Params {
    fn default() -> Self {
        Self { passes: 3 }
    }
}

impl Command for SmoothSelectedComponents {
    const ID: &'static str = "smooth_selected_components";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Smooth Selected Components",
            group: "Modeling",
            icon: "modeling/texture.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let mut ids: Vec<EntityId> = doc
            .selected_components
            .iter()
            .copied()
            .filter(|id| doc.components.contains_key(id))
            .collect();
        if ids.is_empty() {
            ids.extend(doc.chosen_component());
        }
        if ids.is_empty() {
            return Err(CommandError::Failed(
                "there are no components to smooth. Create one with a modelling tool first".into(),
            ));
        }
        if p.passes == 0 {
            return Err(CommandError::InvalidParams(
                "set passes to 1 or more".into(),
            ));
        }
        let mut c = Change::builder(doc);
        for id in ids {
            let smooth = smoothed(&doc.components[&id], p.passes);
            c.replace_component(id, smooth.hand_edited());
        }
        Ok(c.finish())
    }
}

/// `k` with each shaped height averaged with its shaped neighbours, `passes` times.
fn smoothed(k: &Component, passes: u32) -> Component {
    let mut smooth = k.clone();
    let (w, h) = (smooth.columns, smooth.rows);
    for _ in 0..passes {
        let before = smooth.heights.clone();
        for row in 0..h {
            for column in 0..w {
                if before[row * w + column].is_nan() {
                    continue;
                }
                let (mut sum, mut count) = (0.0, 0.0);
                for r in row.saturating_sub(1)..(row + 2).min(h) {
                    for c in column.saturating_sub(1)..(column + 2).min(w) {
                        let v = before[r * w + c];
                        if !v.is_nan() {
                            sum += v;
                            count += 1.0;
                        }
                    }
                }
                smooth.heights[row * w + column] = sum / count;
            }
        }
    }
    smooth
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{domed, two_components};

    cut_craft_core::contract_tests!(SmoothSelectedComponents, domed);

    fn smoothed(picked: &[u64]) -> Vec<bool> {
        let mut doc = two_components();
        let before = doc.clone();
        doc.selected_components = picked.iter().map(|&i| EntityId(i)).collect();
        let p = SmoothSelectedComponents::initial_params(&doc);
        let change = SmoothSelectedComponents::run(&doc, &p).unwrap();
        doc.apply(&change).unwrap();
        doc.components
            .iter()
            .map(|(id, k)| {
                let was = &before.components[id];
                !cut_craft_core::component::same_heights(&k.heights, &was.heights)
            })
            .collect()
    }

    #[test]
    fn the_picked_components_are_smoothed() {
        let ids: Vec<u64> = two_components().components.keys().map(|k| k.0).collect();
        assert_eq!(
            smoothed(&ids[..1]),
            [true, false],
            "the older one, as picked"
        );
        assert_eq!(smoothed(&ids), [true, true], "every one picked");
    }

    #[test]
    fn the_peak_comes_down_and_the_shape_keeps_its_extent() {
        let mut doc = domed();
        let before = doc.components[&EntityId(1)].clone();
        let change =
            SmoothSelectedComponents::run(&doc, &SmoothSelectedComponents::initial_params(&doc))
                .unwrap();
        doc.apply(&change).unwrap();
        let after = &doc.components[&EntityId(1)];
        assert!(after.height_range().unwrap().1 < before.height_range().unwrap().1);
        assert_eq!(
            after.heights.iter().filter(|h| !h.is_nan()).count(),
            before.heights.iter().filter(|h| !h.is_nan()).count()
        );
    }
}
