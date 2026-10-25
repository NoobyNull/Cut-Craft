//! Add Zero Plane: a flat component at height 0 over the whole material, a base for others.

use cut_craft_core::component::{Combine, Component};
use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta, NoParams};

pub struct AddZeroPlane;

impl Command for AddZeroPlane {
    const ID: &'static str = "add_zero_plane";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Add Zero Plane",
            group: "Modeling",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        let plane = Component::from_fn(
            "Zero Plane",
            Combine::Add,
            doc.job.area(),
            doc.job.model_cell(),
            |_| Some(0.0),
        );
        let mut c = Change::builder(doc);
        c.add_component(plane);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    cut_craft_core::contract_tests!(AddZeroPlane);

    #[test]
    fn covers_the_material_at_zero() {
        let doc = cut_craft_core::testing::after::<AddZeroPlane>(&NoParams {});
        let plane = doc.components.values().next().unwrap();
        assert_eq!(plane.height_range(), Some((0.0, 0.0)));
        assert!(
            plane
                .height_at(doc.job.area().min + cut_craft_core::geometry::Point::splat(1.0))
                .is_some()
        );
    }
}
