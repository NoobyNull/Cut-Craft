//! Smooth Selected Components: softens a component's shape by averaging each height with
//! its neighbours, once per pass.

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct SmoothSelectedComponents;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// The component to smooth
    pub component: EntityId,
    /// Passes of smoothing: more is smoother
    #[schemars(range(min = 1, max = 50))]
    pub passes: u32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            component: EntityId(0),
            passes: 3,
        }
    }
}

impl Command for SmoothSelectedComponents {
    const ID: &'static str = "smooth_selected_components";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Smooth Selected Components",
            group: "Modeling",
            icon: "",
            shortcut: "",
        }
    }

    // ponytail: components cannot be selected yet, so the form starts on the newest
    fn initial_params(doc: &Document) -> Params {
        Params {
            component: doc
                .components
                .keys()
                .next_back()
                .copied()
                .unwrap_or(EntityId(0)),
            ..Params::default()
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let id = if doc.components.contains_key(&p.component) {
            p.component
        } else {
            *doc.components
                .keys()
                .next_back()
                .ok_or_else(|| CommandError::Failed("there are no components to smooth".into()))?
        };
        if p.passes == 0 {
            return Err(CommandError::InvalidParams(
                "it needs at least 1 pass".into(),
            ));
        }
        let mut smooth = doc.components[&id].clone();
        let (w, h) = (smooth.columns, smooth.rows);
        for _ in 0..p.passes {
            let before = smooth.heights.clone();
            for row in 0..h {
                for column in 0..w {
                    if before[row * w + column].is_none() {
                        continue;
                    }
                    let (mut sum, mut count) = (0.0, 0.0);
                    for r in row.saturating_sub(1)..(row + 2).min(h) {
                        for c in column.saturating_sub(1)..(column + 2).min(w) {
                            if let Some(v) = before[r * w + c] {
                                sum += v;
                                count += 1.0;
                            }
                        }
                    }
                    smooth.heights[row * w + column] = Some(sum / count);
                }
            }
        }
        let mut c = Change::builder(doc);
        c.replace_component(id, smooth);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::domed;

    cut_craft_core::contract_tests!(SmoothSelectedComponents, domed);

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
            after.heights.iter().filter(|h| h.is_some()).count(),
            before.heights.iter().filter(|h| h.is_some()).count()
        );
    }
}
