//! Create Shape: a 3D component raised over the selected closed vectors, rising from their
//! edges as a round dome, an angled slope or a flat top.

use cut_craft_core::component::{Combine, Component};
use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Point, Rect, inside, polyline_distance};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_named};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct CreateShape;

/// Component cells along the longer side of the selection.
// ponytail: fixed detail until job setup has a model resolution
const CELLS_ACROSS: f64 = 200.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Profile {
    /// Rises at the angle from the edge and rounds over the top
    #[default]
    Round,
    /// Rises at the angle all the way
    Angular,
    /// One height all over
    Flat,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
    pub profile: Profile,
    /// How steeply round and angular sides rise from the edge
    #[schemars(extend("x-unit" = "°"), range(min = 1, max = 89))]
    pub angle: f64,
    /// The flat top's height; round and angular shapes stop rising here (0: no limit)
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub height: f64,
    /// Added everywhere inside the shape
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub base: f64,
    pub combine: Combine,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Shape".into(),
            profile: Profile::Round,
            angle: 45.0,
            height: 0.0,
            base: 0.0,
            combine: Combine::Add,
        }
    }
}

impl Command for CreateShape {
    const ID: &'static str = "create_shape";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Create Shape",
            group: "Modeling",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let name = ensure_named(&p.name)?;
        if !(1.0..=89.0).contains(&p.angle) {
            return Err(CommandError::InvalidParams(
                "the angle must be between 1 and 89 degrees".into(),
            ));
        }
        if p.profile == Profile::Flat && p.height + p.base <= 0.0 {
            return Err(CommandError::InvalidParams(
                "a flat shape needs a height".into(),
            ));
        }
        let (_, paths) = doc.selected_paths(|path| path.closed);
        let Some(bounds) = paths
            .iter()
            .map(cut_craft_core::geometry::Path::bounds)
            .reduce(Rect::union)
        else {
            return Err(CommandError::Failed(
                "select the closed vectors to shape".into(),
            ));
        };
        let cell = bounds.size().max_element() / CELLS_ACROSS;
        let rings: Vec<Vec<Point>> = paths.iter().map(|path| path.flatten(cell / 4.0)).collect();
        // first each cell's distance in from the edge, then the profile over it
        let mut component = Component::from_fn(&name, p.combine, bounds, cell, |at| {
            inside(&rings, at).then(|| {
                rings
                    .iter()
                    .map(|ring| polyline_distance(ring, at))
                    .fold(f64::INFINITY, f64::min)
            })
        });
        let deepest = component
            .heights
            .iter()
            .flatten()
            .fold(0.0, |a: f64, &d| a.max(d));
        let (sin, tan) = (p.angle.to_radians().sin(), p.angle.to_radians().tan());
        let rise = |d: f64| match p.profile {
            Profile::Angular => d * tan,
            // a circular arc meeting the edge at the angle, its top over the deepest point
            Profile::Round => {
                let radius = deepest / sin;
                (radius * radius - (deepest - d).powi(2)).max(0.0).sqrt() - deepest / tan
            }
            Profile::Flat => p.height,
        };
        let limit = if p.height > 0.0 {
            p.height
        } else {
            f64::INFINITY
        };
        for height in component.heights.iter_mut().flatten() {
            *height = rise(*height).min(limit) + p.base;
        }
        let mut c = Change::builder(doc);
        c.add_component(component);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::{Path, Point};
    use cut_craft_core::testing::{document_with, square};

    cut_craft_core::contract_tests!(CreateShape);

    /// The component made over `paths` with `params`, or the error.
    fn shape(paths: Vec<Path>, params: &Params) -> Result<Component, String> {
        let mut doc = document_with(paths);
        let change = CreateShape::run(&doc, params).map_err(|e| e.to_string())?;
        doc.apply(&change).unwrap();
        assert_eq!(doc.components.len(), 1);
        Ok(doc.components.into_values().next().unwrap())
    }

    fn height(c: &Component, x: f64, y: f64) -> f64 {
        c.height_at(Point::new(x, y)).expect("inside the shape")
    }

    fn near(a: f64, b: f64, within: f64) -> bool {
        (a - b).abs() <= within
    }

    #[test]
    fn angular_shapes_rise_at_their_angle_from_the_edge() {
        let params = Params {
            profile: Profile::Angular,
            ..Params::default()
        };
        let c = shape(vec![square(0.0, 0.0, 100.0)], &params).unwrap();
        // half a cell of distance error either way
        assert!(
            near(height(&c, 50.0, 50.0), 50.0, 0.5),
            "{}",
            height(&c, 50.0, 50.0)
        );
        assert!(near(height(&c, 10.0, 50.0), 10.0, 0.5));
        assert_eq!(c.name, "Shape");
    }

    #[test]
    fn round_shapes_rise_at_the_edge_angle_and_round_over_the_top() {
        let params = Params {
            angle: 60.0,
            ..Params::default()
        };
        let c = shape(vec![Path::circle(Point::new(50.0, 50.0), 50.0)], &params).unwrap();
        // a circular arc meeting the edge at 60 degrees peaks at radius x tan(30 degrees)
        let peak = 50.0 * 30f64.to_radians().tan();
        assert!(
            near(height(&c, 50.0, 50.0), peak, 0.5),
            "{}",
            height(&c, 50.0, 50.0)
        );
        assert!(height(&c, 50.0, 50.0) > height(&c, 80.0, 50.0));
    }

    #[test]
    fn the_height_limit_flattens_the_top_and_the_base_raises_it_all() {
        let params = Params {
            profile: Profile::Angular,
            height: 20.0,
            base: 5.0,
            ..Params::default()
        };
        let c = shape(vec![square(0.0, 0.0, 100.0)], &params).unwrap();
        assert!(near(height(&c, 50.0, 50.0), 25.0, 1e-9));
        assert!(near(height(&c, 10.0, 50.0), 15.0, 0.5));
    }

    #[test]
    fn flat_shapes_need_a_height() {
        let flat = Params {
            profile: Profile::Flat,
            ..Params::default()
        };
        let why = shape(vec![square(0.0, 0.0, 100.0)], &flat).unwrap_err();
        assert!(why.contains("height"), "{why}");
        let c = shape(
            vec![square(0.0, 0.0, 100.0)],
            &Params {
                height: 8.0,
                ..flat
            },
        )
        .unwrap();
        assert!(near(height(&c, 50.0, 50.0), 8.0, 1e-9));
    }

    #[test]
    fn holes_stay_empty_and_the_combine_mode_is_kept() {
        let params = Params {
            combine: Combine::Subtract,
            ..Params::default()
        };
        let c = shape(
            vec![square(0.0, 0.0, 100.0), square(40.0, 40.0, 20.0)],
            &params,
        )
        .unwrap();
        assert_eq!(c.height_at(Point::new(50.0, 50.0)), None);
        assert_eq!(c.combine, Combine::Subtract);
    }

    #[test]
    fn needs_closed_vectors() {
        let open = Path::polyline(&[Point::new(0.0, 0.0), Point::new(10.0, 0.0)], false);
        let why = shape(vec![open], &Params::default()).unwrap_err();
        assert!(why.contains("closed"), "{why}");
    }
}
