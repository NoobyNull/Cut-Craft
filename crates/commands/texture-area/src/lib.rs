//! Create Texture Area: the selected component tiled over the selected closed vectors (or the
//! whole job) with spacing, alternate-row and -column shifts and reflections in each group of
//! four, as one new component; the tile is hidden.

use cut_craft_core::component::{Combine, Component};
use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Point, Rect, inside};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct TextureArea;

/// How one tile of a group of four is flipped.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Reflection {
    #[default]
    None,
    Horizontal,
    Vertical,
    Both,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// Gap between tiles across, as a share of the tile's width; negative overlaps them
    #[schemars(extend("x-unit" = "%"), range(min = -90))]
    pub spacing_x: f64,
    /// Gap between rows, as a share of the tile's height
    #[schemars(extend("x-unit" = "%"), range(min = -90))]
    pub spacing_y: f64,
    /// Every other row moves across by this share of the tile's width
    #[schemars(extend("x-unit" = "%"))]
    pub shift_x: f64,
    /// Every other column moves up by this share of the tile's height
    #[schemars(extend("x-unit" = "%"))]
    pub shift_y: f64,
    /// Lower left, lower right, upper left, upper right tile of each group of four
    #[schemars(length(min = 4, max = 4))]
    pub reflection: Vec<Reflection>,
    pub name: String,
    pub combine: Combine,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            spacing_x: 0.0,
            spacing_y: 0.0,
            shift_x: 0.0,
            shift_y: 0.0,
            reflection: vec![Reflection::None; 4],
            name: "Texture Area".into(),
            combine: Combine::Add,
        }
    }
}

impl Command for TextureArea {
    const ID: &'static str = "texture_area";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Create Texture Area",
            group: "Modeling",
            icon: "modeling/create_texture_area.svg",
            shortcut: "",
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        reason = "tile indices are small; rem_euclid is never negative"
    )]
    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let Some(id) = doc.chosen_component() else {
            return Err(CommandError::Failed("select a component to tile".into()));
        };
        let seed = &doc.components[&id];
        let Some(tile) = shaped_extent(seed) else {
            return Err(CommandError::Failed(
                "the component has no shape to tile".into(),
            ));
        };
        if p.reflection.len() != 4 || p.spacing_x <= -100.0 || p.spacing_y <= -100.0 {
            return Err(CommandError::InvalidParams(
                "spacing must leave each tile some room, and there are four reflections".into(),
            ));
        }
        let (_, boundary) = doc.selected_paths(|path| path.closed);
        let rings: Vec<_> = boundary
            .iter()
            .map(|b| b.flatten(cut_craft_core::polygon::TOLERANCE))
            .collect();
        let area = boundary
            .iter()
            .map(cut_craft_core::geometry::Path::bounds)
            .reduce(Rect::union)
            .unwrap_or_else(|| doc.job.area());
        let size = tile.size();
        let pitch = size * Point::new(1.0 + p.spacing_x / 100.0, 1.0 + p.spacing_y / 100.0);
        let shift = size * Point::new(p.shift_x, p.shift_y) / 100.0;
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
                    let flip = p.reflection[(j.rem_euclid(2) * 2 + i.rem_euclid(2)) as usize];
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
        let made = Component::from_fn(&p.name, p.combine, area, seed.cell, |q| {
            if rings.is_empty() || inside(&rings, q) {
                height(q)
            } else {
                None
            }
        });
        let mut hidden = seed.clone();
        hidden.visible = false;
        let mut c = Change::builder(doc);
        c.replace_component(id, hidden);
        c.add_component(made);
        Ok(c.finish())
    }
}

/// The box round a component's shaped cells.
fn shaped_extent(k: &Component) -> Option<Rect> {
    let half = Point::splat(k.cell / 2.0);
    k.heights
        .iter()
        .enumerate()
        .filter(|(_, h)| h.is_some())
        .map(|(i, _)| {
            let c = k.centre_of(i);
            Rect::new(c - half, c + half)
        })
        .reduce(Rect::union)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{run_on, square};

    /// A 10 x 10 mm tile on a 40 x 40 job, its height rising from 0 on the left to 10 on
    /// the right, so reflections show.
    fn ramp() -> Document {
        let mut doc = cut_craft_core::testing::domed();
        doc.components.clear();
        let k = Component::from_fn(
            "ramp",
            Combine::Add,
            Rect::new(Point::new(5.0, 5.0), Point::new(15.0, 15.0)),
            0.5,
            |q| Some(q.x - 5.0),
        );
        let mut c = Change::builder(&doc);
        c.add_component(k);
        let change = c.finish();
        doc.apply(&change).unwrap();
        doc
    }

    fn textured(doc: &mut Document, p: &Params) -> Component {
        run_on::<TextureArea>(doc, p);
        assert!(
            !doc.components.values().next().unwrap().visible,
            "the tile is hidden"
        );
        doc.components.values().last().unwrap().clone()
    }

    cut_craft_core::contract_tests!(TextureArea, ramp);

    #[test]
    fn tiles_repeat_over_the_job_and_reflect() {
        let mut doc = ramp();
        let p = Params {
            reflection: vec![
                Reflection::None,
                Reflection::Horizontal,
                Reflection::None,
                Reflection::None,
            ],
            ..Params::default()
        };
        let k = textured(&mut doc, &p);
        let at = |x, y| k.height_at(Point::new(x, y)).unwrap();
        // first tile: 0 at its left edge, 9.75 at its right; the second is flipped
        assert!(at(0.25, 2.0) < 0.6 && at(9.75, 2.0) > 9.0);
        assert!(at(10.25, 2.0) > 9.0 && at(19.75, 2.0) < 0.6);
        // the third is plain again, and so is the row above's first
        assert!(at(20.25, 2.0) < 0.6);
        assert!(at(0.25, 12.0) < 0.6);
    }

    #[test]
    fn spacing_leaves_gaps_and_shift_moves_alternate_rows() {
        let mut doc = ramp();
        let p = Params {
            spacing_x: 50.0,
            shift_x: 50.0,
            ..Params::default()
        };
        let k = textured(&mut doc, &p);
        assert!(
            k.height_at(Point::new(12.0, 2.0)).is_none(),
            "a 5 mm gap after each tile"
        );
        assert!(k.height_at(Point::new(16.0, 2.0)).is_some());
        // the second row starts 5 mm in
        assert!(k.height_at(Point::new(2.0, 12.0)).is_none());
        assert!(k.height_at(Point::new(6.0, 12.0)).is_some());
    }

    #[test]
    fn a_selected_vector_bounds_the_area() {
        let mut doc = ramp();
        let mut c = Change::builder(&doc);
        let id = c.add(square(0.0, 0.0, 15.0));
        c.select([id]);
        let change = c.finish();
        doc.apply(&change).unwrap();
        let k = textured(&mut doc, &Params::default());
        assert!(k.height_at(Point::new(14.0, 14.0)).is_some());
        assert!(
            k.height_at(Point::new(16.0, 4.0)).is_none(),
            "outside the square"
        );
    }
}
