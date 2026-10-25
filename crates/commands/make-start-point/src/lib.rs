//! Make Start Point: the closed vector starts at the node under the point.

use cut_craft_core::document::Document;
use cut_craft_core::nodes::{self, NodeAt};
use cut_craft_core::{Change, Command, CommandError, Meta};

pub struct MakeStartPoint;

impl Command for MakeStartPoint {
    const ID: &'static str = "make_start_point";
    type Params = NodeAt;

    fn meta() -> Meta {
        Meta {
            label: "Make Start Point",
            group: "Node Editing",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &NodeAt) -> Result<Change, CommandError> {
        let path = p.path(doc)?;
        let restarted = nodes::make_start(path, p.node(path)?)
            .ok_or_else(|| CommandError::Failed("only a closed vector's start can move".into()))?;
        let mut c = Change::builder(doc);
        c.replace(p.vector, restarted);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Point;
    use cut_craft_core::testing::{run_on, sample_document};

    cut_craft_core::contract_tests!(MakeStartPoint);

    #[test]
    fn the_square_starts_at_its_second_corner() {
        let mut doc = sample_document();
        let p = NodeAt {
            at: cut_craft_core::params::Xy::new(30.0, 10.0),
            ..NodeAt::default()
        };
        run_on::<MakeStartPoint>(&mut doc, &p);
        assert_eq!(doc.vectors[&p.vector].start, Point::new(30.0, 10.0));
    }
}
