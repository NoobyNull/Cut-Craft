//! Create Vector Texture: rows of wavy lines at an angle over the selected closed vectors
//! (or the job), their spacing, waves and noise varied by a seeded pattern.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Path, Point, Rect, inside};
use cut_craft_core::random::Random;
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct VectorTexture;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// The lines' angle from the X axis
    #[schemars(extend("x-unit" = "deg"), range(min = -90, max = 90))]
    pub angle: f64,
    #[schemars(title = "Max. spacing", extend("x-unit" = "mm"), range(min = 0))]
    pub max_spacing: f64,
    /// 0 keeps the lines evenly spaced; 100 lets the gaps run from nothing to the maximum
    #[schemars(title = "Spacing variation", extend("x-unit" = "%"), range(min = 0, max = 100))]
    pub variation: f64,
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub wavelength: f64,
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub amplitude: f64,
    /// Randomness in the waves
    #[schemars(extend("x-unit" = "%"), range(min = 0, max = 100))]
    pub noise: f64,
    /// Another seed gives another pattern with the same settings
    pub seed: u32,
    /// Put the texture on a new layer of its own
    pub create_on_new_layer: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            angle: 0.0,
            max_spacing: 5.0,
            variation: 0.0,
            wavelength: 40.0,
            amplitude: 2.0,
            noise: 0.0,
            seed: 1,
            create_on_new_layer: false,
        }
    }
}

impl Command for VectorTexture {
    const ID: &'static str = "vector_texture";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Create Vector Texture",
            group: "Create Vectors",
            icon: "drawing/create_vectors/vector_texture.png",
            shortcut: "",
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "sample counts are small"
    )]
    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("max. spacing", p.max_spacing)?;
        let (_, boundary) = doc.selected_paths(|path| path.closed);
        let rings: Vec<_> = boundary
            .iter()
            .map(|b| b.flatten(cut_craft_core::polygon::TOLERANCE))
            .collect();
        let area = boundary
            .iter()
            .map(Path::bounds)
            .reduce(Rect::union)
            .unwrap_or_else(|| doc.job.area());
        let inside_area = |q: Point| {
            if rings.is_empty() {
                area.contains(q)
            } else {
                inside(&rings, q)
            }
        };
        let (along, across) = {
            let a = p.angle.to_radians();
            (Point::new(a.cos(), a.sin()), Point::new(-a.sin(), a.cos()))
        };
        // cover the area's circle so every angle fills it
        let (centre, radius) = (area.center(), area.size().length() / 2.0);
        let step = (p.max_spacing / 4.0).min(p.wavelength.max(p.max_spacing) / 16.0);
        let samples = ((2.0 * radius / step).ceil() as usize).max(2);
        let mut random = Random(u64::from(p.seed));
        let mut lines = Vec::new();
        let mut offset = -radius;
        while offset <= radius {
            let phase = random.unit() * std::f64::consts::TAU * p.noise / 100.0;
            let wobble = 1.0 + (random.unit() - 0.5) * p.noise / 100.0;
            let mut run: Vec<Point> = Vec::new();
            for k in 0..=samples {
                let s = -radius + 2.0 * radius * k as f64 / samples as f64;
                let wave = if p.wavelength > 0.0 {
                    p.amplitude * wobble * (s / p.wavelength * std::f64::consts::TAU + phase).sin()
                } else {
                    0.0
                };
                let q = centre + along * s + across * (offset + wave);
                if inside_area(q) {
                    run.push(q);
                } else if run.len() > 1 {
                    lines.push(Path::polyline(&std::mem::take(&mut run), false));
                } else {
                    run.clear();
                }
            }
            if run.len() > 1 {
                lines.push(Path::polyline(&run, false));
            }
            let gap = p.max_spacing * (1.0 - p.variation / 100.0 * random.unit());
            offset += gap.max(p.max_spacing / 100.0);
        }
        if lines.is_empty() {
            return Err(CommandError::Failed(
                "the area is too small for the texture".into(),
            ));
        }
        if !p.create_on_new_layer {
            return Ok(Change::swap(doc, vec![], lines));
        }
        let mut c = Change::builder(doc);
        let layer = c.add_layer("Vector Texture");
        let mut added = Vec::new();
        for line in lines {
            let id = c.add(line);
            c.move_to_layer(id, layer);
            added.push(id);
        }
        c.select(added);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{document_with, square};

    cut_craft_core::contract_tests!(VectorTexture);

    fn textured(p: &Params) -> Vec<Path> {
        let mut doc = document_with(vec![square(0.0, 0.0, 50.0)]);
        let change = VectorTexture::run(&doc, p).unwrap();
        doc.apply(&change).unwrap();
        doc.vectors.values().skip(1).cloned().collect()
    }

    #[test]
    fn straight_lines_fill_the_square_at_the_spacing() {
        let lines = textured(&Params {
            amplitude: 0.0,
            ..Params::default()
        });
        assert_eq!(lines.len(), 10, "50 mm at 5 mm spacing");
        for l in &lines {
            let b = l.bounds();
            assert!(b.size().y < 1e-9 && b.min.x >= -1e-9 && b.max.x <= 50.0 + 1e-9);
        }
    }

    #[test]
    fn waves_stay_inside_and_seeds_vary_noisy_patterns() {
        let wavy = textured(&Params::default());
        assert!(
            wavy.iter().any(|l| l.bounds().size().y > 3.0),
            "the waves rise and fall"
        );
        for l in &wavy {
            for q in l.flatten(0.01) {
                assert!(Rect::new(Point::ZERO, Point::splat(50.0)).contains(q));
            }
        }
        let noisy = |seed| {
            textured(&Params {
                noise: 80.0,
                variation: 50.0,
                seed,
                ..Params::default()
            })
        };
        assert_eq!(noisy(3), noisy(3));
        assert_ne!(noisy(3), noisy(4));
    }

    #[test]
    fn the_texture_can_go_on_a_new_layer() {
        let mut doc = document_with(vec![square(0.0, 0.0, 50.0)]);
        let layers = doc.layers.len();
        let p = Params {
            create_on_new_layer: true,
            ..Params::default()
        };
        doc.apply(&VectorTexture::run(&doc, &p).unwrap()).unwrap();
        assert_eq!(doc.layers.len(), layers + 1);
        let new_layer = doc.layers.last().unwrap().id;
        let lines: Vec<_> = doc.vectors.keys().skip(1).copied().collect();
        assert!(!lines.is_empty());
        assert!(lines.iter().all(|id| doc.layer_of(*id) == new_layer));
        assert!(
            doc.selection.iter().eq(lines.iter()),
            "the texture stays selected"
        );
    }
}
