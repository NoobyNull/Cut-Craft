//! Create Cross Section Vector: an open vector of the visible model's profile along a line
//! dragged across it, drawn from the line's start with distance along the line across and
//! the model's height up (1:1).

use cut_craft_core::component::model_at_resolution;
use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Path, Point};
use cut_craft_core::params::Xy;
use cut_craft_core::{Change, Command, CommandError, Drag, Interactive, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct CrossSection;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// Where the section line starts; drag across the model to set both ends
    pub from: Xy,
    pub to: Xy,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            from: Xy::new(0.0, 20.0),
            to: Xy::new(40.0, 20.0),
        }
    }
}

impl Command for CrossSection {
    const ID: &'static str = "cross_section";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Create Cross Section Vector",
            group: "Modeling",
            icon: "modeling/create_cross_section_vector.svg",
            shortcut: "",
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a small sample count"
    )]
    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let (start, end): (Point, Point) = (p.from.into(), p.to.into());
        let length = start.distance(end);
        if length <= 0.0 {
            return Err(CommandError::InvalidParams(
                "drag a line across the model".into(),
            ));
        }
        if !doc.components.values().any(|c| c.visible) {
            return Err(CommandError::Failed(
                "there is no visible model to cut a section through".into(),
            ));
        }
        let model = model_at_resolution(doc);
        let along = (end - start) / length;
        let up = along.perp();
        // a sample every half cell along the line
        let samples = ((length / (model.cell / 2.0)).ceil() as usize).max(1);
        let points: Vec<Point> = (0..=samples)
            .map(|k| {
                #[expect(clippy::cast_precision_loss, reason = "a small sample count")]
                let t = length * k as f64 / samples as f64;
                let height = model.at(start + along * t).unwrap_or(0.0);
                start + along * t + up * height
            })
            .collect();
        Ok(Change::add_and_select(doc, Path::polyline(&points, false)))
    }
}

impl Interactive for CrossSection {
    fn from_drag(drag: Drag, _: &Params) -> Params {
        Params {
            from: Xy::new(drag.start.x, drag.start.y),
            to: Xy::new(drag.end.x, drag.end.y),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{domed, run_on};

    cut_craft_core::contract_tests!(CrossSection, domed);

    #[test]
    fn a_line_through_the_dome_gives_its_round_profile() {
        let mut doc = domed();
        run_on::<CrossSection>(&mut doc, &Params::default());
        let path = doc.vectors.values().next().unwrap();
        assert!(!path.closed);
        let pts = path.flatten(0.01);
        let top = pts.iter().map(|q| q.y).fold(f64::MIN, f64::max);
        // the line runs at y = 20 to the right, so up is +y; the dome is 8 high
        assert!((top - 28.0).abs() < 0.1, "{top}");
        let ends = (pts[0], *pts.last().unwrap());
        assert!((ends.0 - Point::new(0.0, 20.0)).length() < 1e-9);
        assert!((ends.1 - Point::new(40.0, 20.0)).length() < 1e-9);
    }

    #[test]
    fn a_drag_sets_both_ends() {
        let drag = Drag {
            start: Point::new(1.0, 2.0),
            end: Point::new(3.0, 4.0),
            from_center: false,
            constrain: false,
        };
        let p = CrossSection::from_drag(drag, &Params::default());
        assert_eq!((p.from.x, p.to.y), (1.0, 4.0));
        assert!(
            CrossSection::run(
                &domed(),
                &Params {
                    from: p.from,
                    to: p.from
                }
            )
            .is_err()
        );
    }
}
