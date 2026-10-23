//! The preview: simulated material drawn as a lit relief seen from above.

use aspire_core::glam::DVec3;
use aspire_core::simulation::Material;
use eframe::egui::{Color32, ColorImage, Rgba};

// ponytail: fixed lighting until the 3D view can orbit and has light settings
/// Where the light comes from: upper left (-x, +y) and above.
const LIGHT: DVec3 = DVec3::new(-1.0, 1.0, 1.0);
/// Brightness of surfaces facing away from the light.
const AMBIENT: f64 = 0.35;
/// How much darker the full material thickness down is than the top.
const DEPTH_SHADE: f64 = 0.5;

/// The material lit from the upper left, darker the deeper it is cut, in shades of `colour`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "brightness is between 0 and 1; cell steps are 1 or 2"
)]
pub fn shaded(material: &Material, colour: Color32) -> ColorImage {
    let (columns, rows) = (material.columns, material.rows);
    let light = LIGHT.normalize();
    let base = Rgba::from(colour);
    let height = |column: usize, row: usize| material.heights[row * columns + column];
    // slope by central differences, one-sided at the edges
    let slope = |before: usize, after: usize, rise: f64| {
        rise / (material.cell * (after - before).max(1) as f64)
    };
    let pixels = (0..rows)
        .rev() // the image starts at the top, the material at y = 0
        .flat_map(|row| (0..columns).map(move |column| (column, row)))
        .map(|(column, row)| {
            let (left, right) = (column.saturating_sub(1), (column + 1).min(columns - 1));
            let (below, above) = (row.saturating_sub(1), (row + 1).min(rows - 1));
            let normal = DVec3::new(
                -slope(left, right, height(right, row) - height(left, row)),
                -slope(below, above, height(column, above) - height(column, below)),
                1.0,
            )
            .normalize();
            let depth = height(column, row) / material.bottom;
            let lit = (AMBIENT + (1.0 - AMBIENT) * normal.dot(light).max(0.0))
                * (1.0 - DEPTH_SHADE * depth);
            let lit = lit as f32;
            Rgba::from_rgb(base.r() * lit, base.g() * lit, base.b() * lit).into()
        })
        .collect();
    ColorImage {
        size: [columns, rows],
        pixels,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aspire_core::document::Job;

    /// 100 x 100 mm in 1 mm cells, with a 5 mm deep slot from x = 40 to 60 running along y.
    fn slotted() -> Material {
        let job = Job {
            width: 100.0,
            height: 100.0,
            ..Job::default()
        };
        let mut m = Material::new(&job, 100);
        for row in 0..m.rows {
            for column in 40..60 {
                m.heights[row * m.columns + column] = -5.0;
            }
        }
        m
    }

    fn brightness(image: &ColorImage, x: usize, y: usize) -> u8 {
        image.pixels[y * image.width() + x].r()
    }

    #[test]
    fn walls_facing_the_light_are_brighter_than_flat_and_the_far_ones_darker() {
        let image = shaded(&slotted(), Color32::WHITE);
        assert_eq!(image.size, [100, 100]);
        let flat = brightness(&image, 10, 50);
        assert!(
            brightness(&image, 60, 50) > flat,
            "the wall rising towards +x faces the light"
        );
        assert!(
            brightness(&image, 39, 50) < flat,
            "the wall dropping towards +x faces away"
        );
    }

    #[test]
    fn deeper_is_darker_and_the_top_row_is_the_far_edge() {
        let mut m = slotted();
        for column in 0..m.columns {
            m.heights[(m.rows - 1) * m.columns + column] = -10.0;
        }
        let image = shaded(&m, Color32::WHITE);
        assert!(
            brightness(&image, 50, 50) < brightness(&image, 10, 50),
            "the slot floor is darker"
        );
        assert!(
            brightness(&image, 10, 0) < brightness(&image, 10, 50),
            "y = top is drawn at the top"
        );
    }
}
