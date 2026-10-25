//! Slice Model: the visible model divided into Z bands, each a new component starting at the
//! zero plane, for parts thicker than the material or the machine allows. The components it
//! came from stay.

use cut_craft_core::component::{Combine, Component, model_at_resolution, silhouette};
use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct Slice;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Slicing {
    /// Slices of the given thickness; the last takes what is left
    #[default]
    SliceThickness,
    /// The model's thickness shared between this many slices
    NumberOfSlices,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    #[schemars(title = "Model slicing")]
    pub slicing: Slicing,
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub slice_thickness: f64,
    /// Slice from the top down, so the thinner last slice is at the bottom
    pub from_top: bool,
    #[schemars(range(min = 1, max = 100))]
    pub number_of_slices: u32,
    /// A closed vector round each slice
    pub create_boundary_vectors: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            slicing: Slicing::SliceThickness,
            // ponytail: Aspire offers the material thickness here [inf]
            slice_thickness: 20.0,
            from_top: true,
            number_of_slices: 2,
            create_boundary_vectors: false,
        }
    }
}

impl Command for Slice {
    const ID: &'static str = "slice";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Slice Model",
            group: "Modeling",
            icon: "modeling/slice_model.svg",
            shortcut: "",
        }
    }

    fn initial_params(doc: &Document) -> Params {
        Params {
            slice_thickness: doc.job.thickness,
            ..Params::default()
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if !doc.components.values().any(|c| c.visible) {
            return Err(CommandError::Failed(
                "there is no visible model to slice".into(),
            ));
        }
        let model = model_at_resolution(doc);
        let top = model.heights.iter().copied().fold(0.0, f64::max);
        if top <= 0.0 {
            return Err(CommandError::Failed(
                "the model has no thickness above the zero plane".into(),
            ));
        }
        let cuts = match p.slicing {
            Slicing::NumberOfSlices => {
                if p.number_of_slices == 0 {
                    return Err(CommandError::InvalidParams(
                        "make at least one slice".into(),
                    ));
                }
                let n = f64::from(p.number_of_slices);
                (0..=p.number_of_slices)
                    .map(|k| top * f64::from(k) / n)
                    .collect()
            }
            Slicing::SliceThickness => {
                ensure_positive("slice thickness", p.slice_thickness)?;
                bands(top, p.slice_thickness, p.from_top)
            }
        };
        let mut c = Change::builder(doc);
        for (k, band) in cuts.windows(2).enumerate() {
            let (lo, hi) = (band[0], band[1]);
            let mut slice =
                Component::from_material(&format!("Slice {}", k + 1), Combine::Add, &model);
            for h in &mut slice.heights {
                *h = h.filter(|&z| z > lo).map(|z| z.min(hi) - lo);
            }
            if p.create_boundary_vectors {
                for path in silhouette(&[&slice]) {
                    c.add(path);
                }
            }
            c.add_component(slice);
        }
        Ok(c.finish())
    }
}

/// Cut heights from 0 to `top`, `step` apart, the odd slice at the bottom when `from_top`.
fn bands(top: f64, step: f64, from_top: bool) -> Vec<f64> {
    let mut cuts = vec![];
    let mut z = 0.0;
    while z < top - 1e-9 {
        cuts.push(z);
        z += step;
    }
    cuts.push(top);
    if from_top {
        cuts = cuts.iter().rev().map(|z| top - z).collect();
    }
    cuts
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Point;
    use cut_craft_core::testing::{domed, run_on};

    cut_craft_core::contract_tests!(Slice, domed);

    #[test]
    fn bands_put_the_thin_slice_at_the_far_end() {
        let close = |a: &[f64], b: &[f64]| {
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-9)
        };
        assert!(close(&bands(10.0, 4.0, true), &[0.0, 2.0, 6.0, 10.0]));
        assert!(close(&bands(10.0, 4.0, false), &[0.0, 4.0, 8.0, 10.0]));
        assert!(close(&bands(8.0, 4.0, false), &[0.0, 4.0, 8.0]));
        assert!(close(&bands(3.0, 4.0, true), &[0.0, 3.0]));
    }

    #[test]
    fn the_dome_in_two_slices_each_from_zero() {
        let mut doc = domed();
        let p = Params {
            slicing: Slicing::NumberOfSlices,
            number_of_slices: 2,
            create_boundary_vectors: true,
            ..Params::default()
        };
        run_on::<Slice>(&mut doc, &p);
        assert_eq!(doc.components.len(), 3, "the dome stays, two slices added");
        let slices: Vec<_> = doc.components.values().skip(1).collect();
        let top = model_at_resolution(&domed())
            .heights
            .iter()
            .copied()
            .fold(0.0, f64::max);
        let middle = Point::new(20.0, 20.0);
        // the bottom slice is flat-topped at half the height, the top one the dome's cap
        let bottom = slices[0].height_at(middle).unwrap();
        assert!((bottom - top / 2.0).abs() < 1e-9);
        let cap = slices[1].height_at(middle).unwrap();
        assert!((cap - top / 2.0).abs() < 0.05);
        let lowest = slices[1].height_range().unwrap().0;
        assert!(
            (0.0..0.5).contains(&lowest),
            "the cap is shifted down to the zero plane: {lowest}"
        );
        // the cap is narrower than the base
        let cap_edge = Point::new(27.0, 20.0);
        assert!(slices[1].height_at(cap_edge).is_none());
        assert!(slices[0].height_at(cap_edge).is_some());
        assert_eq!(doc.vectors.len(), 2, "a boundary round each slice");
    }

    #[test]
    fn needs_a_model() {
        assert!(cut_craft_core::testing::rejection::<Slice>(&Params::default()).contains("model"));
    }
}
