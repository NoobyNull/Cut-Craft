//! Emboss Component: the selected component made much lower while its surface detail stays:
//! its broad shape (a blur of it) is scaled to the new height, the detail on top of that is
//! kept, then smoothed.

use cut_craft_core::component::{Component, chosen_or_baked};
use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct Emboss;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// The broad shape's new height, as a share of its height now
    #[schemars(extend("x-unit" = "%"), range(min = 1, max = 50))]
    pub scale_height: f64,
    /// Passes of smoothing over the result: more for a larger scale
    #[schemars(range(min = 0, max = 10))]
    pub detail_smoothness: u32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            // ponytail: the dialog's slider runs 1–50 %; 10 % is the help's example
            scale_height: 10.0,
            detail_smoothness: 2,
        }
    }
}

/// How far the broad shape's blur reaches, as a share of the component's longer side.
// ponytail: our own split of shape and detail [inf]; Aspire's method is not described
const BROAD: f64 = 0.05;

impl Command for Emboss {
    const ID: &'static str = "emboss";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Emboss Component",
            group: "Modeling",
            icon: "modeling/emboss_component.svg",
            shortcut: "",
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "cell counts are small"
    )]
    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if !(1.0..=50.0).contains(&p.scale_height) {
            return Err(CommandError::InvalidParams(
                "scale height is 1 to 50 %".into(),
            ));
        }
        let Some((ids, mut part)) = chosen_or_baked(doc) else {
            return Err(CommandError::Failed("select a component to emboss".into()));
        };
        let reach = ((part.columns.max(part.rows) as f64 * BROAD).ceil() as usize).max(1);
        let broad = blur(&part, reach);
        let s = p.scale_height / 100.0;
        let mut embossed: Vec<Option<f64>> = part
            .heights
            .iter()
            .zip(&broad)
            .map(|(h, b)| h.map(|z| s * b + (z - b)))
            .collect();
        for _ in 0..p.detail_smoothness {
            part.heights = embossed;
            embossed = blur(&part, 1)
                .into_iter()
                .zip(&part.heights)
                .map(|(b, h)| h.map(|_| b))
                .collect();
        }
        part.heights = embossed;
        let mut c = Change::builder(doc);
        for id in ids {
            c.remove_component(id);
        }
        c.add_component(part);
        Ok(c.finish())
    }
}

/// The mean height of each cell's shaped neighbours within `reach` cells (square window,
/// summed-area table so any reach costs the same).
fn blur(part: &Component, reach: usize) -> Vec<f64> {
    let (w, h) = (part.columns, part.rows);
    let mut sum = vec![0.0; (w + 1) * (h + 1)];
    let mut count = vec![0.0; (w + 1) * (h + 1)];
    for r in 0..h {
        for c in 0..w {
            let (v, n) = part.heights[r * w + c].map_or((0.0, 0.0), |z| (z, 1.0));
            let at = (r + 1) * (w + 1) + c + 1;
            sum[at] = v + sum[at - 1] + sum[at - w - 1] - sum[at - w - 2];
            count[at] = n + count[at - 1] + count[at - w - 1] - count[at - w - 2];
        }
    }
    let area = |t: &[f64], r0: usize, c0: usize, r1: usize, c1: usize| {
        t[r1 * (w + 1) + c1] - t[r0 * (w + 1) + c1] - t[r1 * (w + 1) + c0] + t[r0 * (w + 1) + c0]
    };
    (0..h)
        .flat_map(|r| (0..w).map(move |c| (r, c)))
        .map(|(r, c)| {
            let (r0, c0) = (r.saturating_sub(reach), c.saturating_sub(reach));
            let (r1, c1) = ((r + reach + 1).min(h), (c + reach + 1).min(w));
            let n = area(&count, r0, c0, r1, c1);
            if n > 0.0 {
                area(&sum, r0, c0, r1, c1) / n
            } else {
                0.0
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::component::Combine;
    use cut_craft_core::geometry::{Point, Rect};
    use cut_craft_core::testing::{domed, run_on};

    cut_craft_core::contract_tests!(Emboss, domed);

    /// A 20 mm high dome with 0.5 mm ripples on it.
    fn rippled() -> Document {
        let mut doc = Document::default();
        let area = Rect::new(Point::new(0.0, 0.0), Point::new(40.0, 40.0));
        let middle = Point::new(20.0, 20.0);
        let k = Component::from_fn("rippled", Combine::Add, area, 0.2, |p| {
            let r = p.distance(middle);
            (r < 20.0).then(|| (20.0 - r) + 0.5 * (p.x * 3.0).sin())
        });
        let mut c = Change::builder(&doc);
        c.add_component(k);
        let change = c.finish();
        doc.apply(&change).unwrap();
        doc
    }

    #[test]
    fn the_shape_drops_to_the_scale_and_the_ripples_stay() {
        let mut doc = rippled();
        run_on::<Emboss>(
            &mut doc,
            &Params {
                scale_height: 10.0,
                detail_smoothness: 0,
            },
        );
        let k = doc.components.values().next().unwrap();
        let (_, hi) = k.height_range().unwrap();
        assert!(hi < 4.0, "20 mm high at 10 % plus detail: {hi}");
        // ripples a third of a period apart along x still differ clearly
        let at = |x: f64| k.height_at(Point::new(x, 26.0)).unwrap();
        let ripple = (at(26.0) - at(27.05)).abs();
        assert!(ripple > 0.2, "detail kept: {ripple}");
    }

    #[test]
    fn smoothing_softens_and_the_range_is_checked() {
        let smooth = |n| {
            let mut doc = rippled();
            run_on::<Emboss>(
                &mut doc,
                &Params {
                    scale_height: 10.0,
                    detail_smoothness: n,
                },
            );
            let k = doc.components.values().next().unwrap().clone();
            (k.height_at(Point::new(26.0, 26.0)).unwrap()
                - k.height_at(Point::new(27.05, 26.0)).unwrap())
            .abs()
        };
        assert!(smooth(5) < smooth(0));
        let p = Params {
            scale_height: 80.0,
            ..Params::default()
        };
        assert!(Emboss::run(&rippled(), &p).is_err());
    }
}
