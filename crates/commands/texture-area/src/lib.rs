//! Create Texture Area: the selected component tiled over the selected closed vectors (or the
//! whole job) with spacing, alternate-row and -column shifts and reflections in each group of
//! four, as one new component; the tile is hidden. The component remembers its tile and
//! settings: opening the form on it edits it in place, and resizing it tiles its new box.

use cut_craft_core::component::Combine;
use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::texture_area::{Texture, Tiling, tiled};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub use cut_craft_core::texture_area::Reflection;

pub struct TextureArea;

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
    /// The texture area being edited: tiled again over its own box. Set when the form
    /// opens on one.
    #[schemars(skip)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replaces: Option<EntityId>,
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
            replaces: None,
        }
    }
}

impl Params {
    fn tiling(&self) -> Tiling {
        Tiling {
            spacing_x: self.spacing_x,
            spacing_y: self.spacing_y,
            shift_x: self.shift_x,
            shift_y: self.shift_y,
            reflection: self.reflection.clone(),
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
            icon: "modeling/create_texture_area.png",
            shortcut: "",
        }
    }

    /// On a texture area, its own settings, so the form edits it.
    fn initial_params(doc: &Document) -> Params {
        let Some(id) = doc.chosen_component() else {
            return Params::default();
        };
        let k = &doc.components[&id];
        let Some(texture) = &k.texture else {
            return Params::default();
        };
        let t = &texture.tiling;
        Params {
            spacing_x: t.spacing_x,
            spacing_y: t.spacing_y,
            shift_x: t.shift_x,
            shift_y: t.shift_y,
            reflection: t.reflection.clone(),
            name: k.name.clone(),
            combine: k.combine,
            replaces: Some(id),
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let editing = p
            .replaces
            .and_then(|id| doc.components.get(&id).map(|k| (id, k)))
            .filter(|(_, k)| k.texture.is_some());
        let (seed_id, area, rings) = if let Some((_, k)) = editing {
            // an existing texture area: its own box, its tile
            let texture = k.texture.as_ref().expect("filtered above");
            (texture.seed, k.extent(), vec![])
        } else {
            let Some(id) = doc.chosen_component() else {
                return Err(CommandError::Failed("select a component to tile".into()));
            };
            let (_, boundary) = doc.selected_paths(|path| path.closed);
            let rings: Vec<_> = boundary
                .iter()
                .map(|b| b.flatten(cut_craft_core::polygon::TOLERANCE))
                .collect();
            let area = boundary
                .iter()
                .map(cut_craft_core::geometry::Path::bounds)
                .reduce(cut_craft_core::geometry::Rect::union)
                .unwrap_or_else(|| doc.job.area());
            (id, area, rings)
        };
        let seed = doc.components.get(&seed_id).ok_or_else(|| {
            CommandError::Failed(
                "The component used as the tile has been deleted. Select another component \
                     to tile."
                    .into(),
            )
        })?;
        let tiling = p.tiling();
        let mut made = tiled(seed, &p.name, p.combine, area, &rings, &tiling).ok_or_else(|| {
            CommandError::InvalidParams(
                "check the tile settings: the tile needs a shape, the spacing must leave each \
                 tile some room, and there must be four reflections"
                    .into(),
            )
        })?;
        made.texture = Some(Texture {
            seed: seed_id,
            tiling,
        });
        let mut c = Change::builder(doc);
        if let Some((id, old)) = editing {
            made.visible = old.visible;
            made.level = old.level;
            made.group.clone_from(&old.group);
            c.replace_component(id, made);
        } else {
            let mut hidden = seed.clone();
            hidden.visible = false;
            c.replace_component(seed_id, hidden);
            c.add_component(made);
        }
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::component::Component;
    use cut_craft_core::geometry::{Point, Rect};
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

    #[test]
    fn a_texture_area_is_edited_in_place_and_tiled_again_when_scaled() {
        use cut_craft_core::glam::{DAffine2, DVec2};
        let mut doc = ramp();
        let k = textured(&mut doc, &Params::default());
        let id = *doc.components.keys().last().unwrap();
        assert_eq!(
            k.texture.as_ref().map(|t| t.seed),
            doc.components.keys().next().copied()
        );
        // the form opens on it with its settings, and Apply replaces it
        doc.selected_components = [id].into_iter().collect();
        let p = TextureArea::initial_params(&doc);
        assert_eq!(p.replaces, Some(id));
        let gaps = Params {
            spacing_x: 50.0,
            ..p
        };
        run_on::<TextureArea>(&mut doc, &gaps);
        assert_eq!(doc.components.len(), 2, "replaced, not added");
        let k = &doc.components[&id];
        assert!(k.height_at(Point::new(12.0, 2.0)).is_none(), "a gap now");
        assert_eq!(k.texture.as_ref().map(|t| t.tiling.spacing_x), Some(50.0));
        // scaled twice as wide, the tiles stay 10 wide: the box is tiled again
        let change =
            Change::transform_selected(&doc, |_| DAffine2::from_scale(DVec2::new(2.0, 1.0)))
                .unwrap();
        doc.apply(&change).unwrap();
        let k = &doc.components[&id];
        assert!(
            (k.extent().size().x - 80.0).abs() < k.cell,
            "{:?}",
            k.extent()
        );
        let left = k.extent().min.x;
        assert!(k.height_at(Point::new(left + 0.25, 2.0)).unwrap() < 0.6);
        assert!(
            k.height_at(Point::new(left + 9.75, 2.0)).unwrap() > 9.0,
            "the tile is not stretched"
        );
        assert!(
            k.height_at(Point::new(left + 12.0, 2.0)).is_none(),
            "and the gap is kept"
        );
    }
}
