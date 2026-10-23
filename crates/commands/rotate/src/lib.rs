//! Rotate.

use aspire_core::document::Document;
use aspire_core::glam::DAffine2;
use aspire_core::params::Anchor;
use aspire_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct Rotate;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// Counter-clockwise
    #[schemars(extend("x-unit" = "deg"))]
    pub angle: f64,
    /// Point of the selection to rotate about
    pub anchor: Anchor,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            angle: 90.0,
            anchor: Anchor::Center,
        }
    }
}

impl Command for Rotate {
    const ID: &'static str = "rotate";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Rotate",
            group: "Transform Objects",
            icon: "drawing/edit_objects/rotate.svg",
            shortcut: "R",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let bounds = doc
            .selection_bounds()
            .ok_or(CommandError::NothingSelected)?;
        let a = p.anchor.point(bounds);
        let t = DAffine2::from_translation(a)
            * DAffine2::from_angle(p.angle.to_radians())
            * DAffine2::from_translation(-a);
        Change::transform_selected(doc, |_| t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    aspire_core::contract_tests!(Rotate);

    use aspire_core::geometry::Point;
    use aspire_core::testing::{after, sample_document};

    #[test]
    fn quarter_turn_about_the_centre_swaps_width_and_height() {
        let before = sample_document().selection_bounds().unwrap();
        let after = after::<Rotate>(&Params::default())
            .selection_bounds()
            .unwrap();
        assert!((after.center() - before.center()).length() < 1e-6);
        assert!(
            (after.size() - Point::new(before.size().y, before.size().x)).length() < 1e-6,
            "{after:?}"
        );
    }
}
