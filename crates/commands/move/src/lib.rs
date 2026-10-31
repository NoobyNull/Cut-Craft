//! Move.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::Point;
use cut_craft_core::glam::DAffine2;
use cut_craft_core::params::Anchor;
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct Move;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// Move by X/Y instead of to X/Y
    pub relative: bool,
    #[schemars(extend("x-unit" = "mm"))]
    pub x: f64,
    #[schemars(extend("x-unit" = "mm"))]
    pub y: f64,
    /// Turns the X/Y offset by this angle, anticlockwise (relative moves only)
    #[serde(default)]
    #[schemars(extend("x-unit" = "deg"))]
    pub angle: f64,
    /// Point of the selection placed at X/Y (absolute moves only)
    pub anchor: Anchor,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            relative: true,
            x: 10.0,
            y: 0.0,
            angle: 0.0,
            anchor: Anchor::BottomLeft,
        }
    }
}

impl Command for Move {
    const ID: &'static str = "move";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Move",
            group: "Transform Objects",
            icon: "drawing/edit_objects/move.png",
            shortcut: "M",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let bounds = doc
            .selection_bounds()
            .ok_or(CommandError::NothingSelected)?;
        let to = Point::new(p.x, p.y);
        let delta = if p.relative {
            Point::from_angle(p.angle.to_radians()).rotate(to)
        } else {
            to - p.anchor.point(bounds)
        };
        Change::transform_selected(doc, |_| DAffine2::from_translation(delta))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    cut_craft_core::contract_tests!(Move);

    use cut_craft_core::testing::after;

    #[test]
    fn relative_moves_by_the_offset() {
        let b = after::<Move>(&Params::default())
            .selection_bounds()
            .unwrap();
        assert!((b.min - Point::new(20.0, 10.0)).length() < 1e-9, "{b:?}");
    }

    #[test]
    fn relative_rotated_moves_along_the_angle() {
        let p = Params {
            x: 10.0,
            y: 0.0,
            angle: 90.0,
            ..Params::default()
        };
        let b = after::<Move>(&p).selection_bounds().unwrap();
        assert!((b.min - Point::new(10.0, 20.0)).length() < 1e-9, "{b:?}");
    }

    #[test]
    fn absolute_puts_the_anchor_at_the_point() {
        let p = Params {
            relative: false,
            x: 0.0,
            y: 0.0,
            anchor: Anchor::BottomLeft,
            ..Params::default()
        };
        let b = after::<Move>(&p).selection_bounds().unwrap();
        assert!(b.min.length() < 1e-9, "{b:?}");
    }

    #[test]
    fn moves_a_selected_component() {
        use cut_craft_core::document::EntityId;
        let mut doc = cut_craft_core::testing::domed();
        doc.selected_components = [EntityId(1)].into();
        let p = Params {
            relative: false,
            x: 0.0,
            y: 0.0,
            anchor: Anchor::BottomLeft,
            ..Params::default()
        };
        cut_craft_core::testing::run_on::<Move>(&mut doc, &p);
        let e = doc.components[&EntityId(1)].extent();
        assert!(e.min.length() < 1e-9, "{e:?}");
    }

    #[test]
    fn nothing_selected_is_an_error() {
        assert_eq!(
            Move::run(&Document::default(), &Params::default()),
            Err(CommandError::NothingSelected)
        );
    }
}
