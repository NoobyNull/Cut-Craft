//! Mirror.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::Point;
use cut_craft_core::glam::DAffine2;
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct Mirror;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Flip {
    /// Left to right
    #[default]
    Horizontal,
    /// Top to bottom
    Vertical,
    Both,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub flip: Flip,
}

impl Command for Mirror {
    const ID: &'static str = "mirror";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Mirror",
            group: "Transform Objects",
            icon: "drawing/edit_objects/mirror.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let bounds = doc
            .selection_bounds()
            .ok_or(CommandError::NothingSelected)?;
        let c = bounds.center();
        let scale = match p.flip {
            Flip::Horizontal => Point::new(-1.0, 1.0),
            Flip::Vertical => Point::new(1.0, -1.0),
            Flip::Both => Point::splat(-1.0),
        };
        let t = DAffine2::from_translation(c)
            * DAffine2::from_scale(scale)
            * DAffine2::from_translation(-c);
        Change::transform_selected(doc, |_| t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    cut_craft_core::contract_tests!(Mirror);

    use cut_craft_core::document::EntityId;
    use cut_craft_core::testing::{after, sample_document};

    #[test]
    fn horizontal_flip_keeps_the_bounds_and_swaps_sides() {
        let before = sample_document().selection_bounds().unwrap();
        let doc = after::<Mirror>(&Params::default());
        let b = doc.selection_bounds().unwrap();
        assert!((b.min - before.min).length() < 1e-9 && (b.max - before.max).length() < 1e-9);
        // the square at x 10..30 ends up at the far right, 205..225
        assert!((doc.vectors[&EntityId(1)].bounds().min.x - 205.0).abs() < 1e-9);
    }
}
