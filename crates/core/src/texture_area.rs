//! Texture areas: a seed component repeated over an area (Create Texture Area), kept on the
//! component made so it can be edited again, and tiled again when its box changes.

use crate::component::{Combine, Component};
use crate::document::EntityId;
use crate::geometry::{Point, Rect, inside};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// How one tile of a group of four is flipped.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Reflection {
    /// The tile as it is
    #[default]
    None,
    /// The tile flipped left to right
    Horizontal,
    /// The tile flipped top to bottom
    Vertical,
    /// The tile flipped both ways
    Both,
}

/// Create Texture Area's settings: the gaps between tiles across and up and the shifts of
/// alternate rows and columns, as shares (percent) of the tile's size, and how the lower
/// left, lower right, upper left and upper right tile of each group of four is flipped.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Tiling {
    pub spacing_x: f64,
    pub spacing_y: f64,
    pub shift_x: f64,
    pub shift_y: f64,
    pub reflection: Vec<Reflection>,
}

impl Default for Tiling {
    fn default() -> Self {
        Self {
            spacing_x: 0.0,
            spacing_y: 0.0,
            shift_x: 0.0,
            shift_y: 0.0,
            reflection: vec![Reflection::None; 4],
        }
    }
}

/// What a texture area component repeats: its hidden seed, and how.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Texture {
    pub seed: EntityId,
    pub tiling: Tiling,
}

/// The box round a component's shaped cells.
#[must_use]
pub fn shaped_extent(k: &Component) -> Option<Rect> {
    let half = Point::splat(k.cell / 2.0);
    k.heights
        .iter()
        .enumerate()
        .filter(|(_, h)| !h.is_nan())
        .map(|(i, _)| {
            let c = k.centre_of(i);
            Rect::new(c - half, c + half)
        })
        .reduce(Rect::union)
}

/// `seed` repeated over `area` (inside `rings` where there are any) as `tiling` says, on
/// the seed's cells, named and combined as asked. None when the seed has no shape, the
/// spacing leaves a tile no room, or there are not four reflections.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "tile indices are small; rem_euclid is never negative"
)]
pub fn tiled(
    seed: &Component,
    name: &str,
    combine: Combine,
    area: Rect,
    rings: &[Vec<Point>],
    tiling: &Tiling,
) -> Option<Component> {
    let tile = shaped_extent(seed)?;
    if tiling.reflection.len() != 4 || tiling.spacing_x <= -100.0 || tiling.spacing_y <= -100.0 {
        return None;
    }
    let size = tile.size();
    let pitch = size
        * Point::new(
            1.0 + tiling.spacing_x / 100.0,
            1.0 + tiling.spacing_y / 100.0,
        );
    let shift = size * Point::new(tiling.shift_x, tiling.shift_y) / 100.0;
    let height = |q: Point| -> Option<f64> {
        let rel = q - area.min;
        // every tile that could cover q, overlapping ones included
        let reach = |v: f64, s: f64, pitch: f64, size: f64| {
            let lo = ((v - size - s.abs()) / pitch).floor() as i64;
            let hi = ((v + s.abs()) / pitch).floor() as i64;
            lo..=hi
        };
        let mut best: Option<f64> = None;
        for j in reach(rel.y, shift.y, pitch.y, size.y) {
            for i in reach(rel.x, shift.x, pitch.x, size.x) {
                let corner = Point::new(
                    i as f64 * pitch.x + if j.rem_euclid(2) == 1 { shift.x } else { 0.0 },
                    j as f64 * pitch.y + if i.rem_euclid(2) == 1 { shift.y } else { 0.0 },
                );
                let mut local = rel - corner;
                if !(0.0..size.x).contains(&local.x) || !(0.0..size.y).contains(&local.y) {
                    continue;
                }
                let flip = tiling.reflection[(j.rem_euclid(2) * 2 + i.rem_euclid(2)) as usize];
                if matches!(flip, Reflection::Horizontal | Reflection::Both) {
                    local.x = size.x - local.x;
                }
                if matches!(flip, Reflection::Vertical | Reflection::Both) {
                    local.y = size.y - local.y;
                }
                if let Some(z) = seed.height_at(tile.min + local) {
                    best = Some(best.map_or(z, |b: f64| b.max(z)));
                }
            }
        }
        best
    };
    Some(Component::from_fn(name, combine, area, seed.cell, |q| {
        if rings.is_empty() || inside(rings, q) {
            height(q)
        } else {
            None
        }
    }))
}

/// `area` (a texture area component) tiled again over its own box, its tiles the same size,
/// as Aspire's texture areas keep their tiles when resized. None when it is no texture area
/// or its seed is gone; the result keeps its name, combine, level, group and visibility.
#[must_use]
pub fn retiled(
    area: &Component,
    seed_of: impl Fn(EntityId) -> Option<Component>,
) -> Option<Component> {
    let texture = area.texture.as_ref()?;
    let seed = seed_of(texture.seed)?;
    let mut again = tiled(
        &seed,
        &area.name,
        area.combine,
        area.extent(),
        &[],
        &texture.tiling,
    )?;
    again.visible = area.visible;
    again.level = area.level;
    again.group.clone_from(&area.group);
    again.texture.clone_from(&area.texture);
    Some(again)
}
