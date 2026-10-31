//! Move: the selection to a point (an anchor of its box there), or by an offset, which
//! (relative rotated) can be turned by an angle to follow work laid out at that angle.

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
    /// Point of the selection placed at X/Y (absolute moves only)
    pub anchor: Anchor,
    /// Relative moves: the X/Y offset turned by the angle below, so X runs along it
    #[serde(default)]
    #[schemars(title = "Relative rotated", extend("x-show-when" = {"relative": true}))]
    pub rotated: bool,
    /// The angle the offset is turned by, anticlockwise from the X axis
    #[serde(default)]
    #[schemars(extend("x-unit" = "deg", "x-show-when" = {"rotated": true}))]
    pub angle: f64,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            relative: true,
            x: 10.0,
            y: 0.0,
            anchor: Anchor::BottomLeft,
            rotated: false,
            angle: 0.0,
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
        let delta = if p.relative && p.rotated {
            let (sin, cos) = p.angle.to_radians().sin_cos();
            Point::new(to.x * cos - to.y * sin, to.x * sin + to.y * cos)
        } else if p.relative {
            to
        } else {
            to - p.anchor.point(bounds)
        };
        Change::transform_selected(doc, |_| DAffine2::from_translation(delta))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rotated_relative_move_runs_along_the_angle() {
        let before = cut_craft_core::testing::sample_document()
            .selection_bounds()
            .unwrap();
        let after = after::<Move>(&Params {
            relative: true,
            rotated: true,
            angle: 90.0,
            x: 10.0,
            y: 0.0,
            ..Params::default()
        })
        .selection_bounds()
        .unwrap();
        let moved = after.min - before.min;
        assert!(
            moved.distance(Point::new(0.0, 10.0)) < 1e-9,
            "10 along 90° is up: {moved}"
        );
    }

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
