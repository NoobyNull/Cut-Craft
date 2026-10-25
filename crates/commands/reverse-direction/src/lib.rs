//! Reverse Direction: the vector under the point runs the other way.

use cut_craft_core::document::Document;
use cut_craft_core::nodes::NodeAt;
use cut_craft_core::{Change, Command, CommandError, Meta};

pub struct ReverseDirection;

impl Command for ReverseDirection {
    const ID: &'static str = "reverse_direction";
    type Params = NodeAt;

    fn meta() -> Meta {
        Meta {
            label: "Reverse Direction",
            group: "Node Editing",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &NodeAt) -> Result<Change, CommandError> {
        let path = p.path(doc)?;
        let mut c = Change::builder(doc);
        c.replace(p.vector, path.reversed());
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Point;
    use cut_craft_core::testing::{run_on, sample_document};

    cut_craft_core::contract_tests!(ReverseDirection);

    #[test]
    fn the_open_polyline_now_starts_at_its_end() {
        let mut doc = sample_document();
        let id = cut_craft_core::document::EntityId(3);
        let p = NodeAt {
            vector: id,
            ..NodeAt::default()
        };
        run_on::<ReverseDirection>(&mut doc, &p);
        assert_eq!(doc.vectors[&id].start, Point::new(140.0, 10.0));
    }
}
