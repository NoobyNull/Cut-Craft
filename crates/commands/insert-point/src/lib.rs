//! Insert Point: a new node on the span under the point, splitting it there.

use cut_craft_core::document::Document;
use cut_craft_core::nodes::{self, NodeAt};
use cut_craft_core::{Change, Command, CommandError, Meta};

pub struct InsertPoint;

impl Command for InsertPoint {
    const ID: &'static str = "insert_point";
    type Params = NodeAt;

    fn meta() -> Meta {
        Meta {
            label: "Insert Point",
            group: "Node Editing",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &NodeAt) -> Result<Change, CommandError> {
        let path = p.path(doc)?;
        let split = nodes::insert_node(path, p.at.into(), p.reach)
            .ok_or_else(|| CommandError::Failed("there is no span there".into()))?;
        let mut c = Change::builder(doc);
        c.replace(p.vector, split);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Point;
    use cut_craft_core::testing::{run_on, sample_document};

    cut_craft_core::contract_tests!(InsertPoint);

    #[test]
    fn a_point_on_a_side_becomes_a_node() {
        let mut doc = sample_document();
        let p = NodeAt {
            at: cut_craft_core::params::Xy::new(20.0, 10.3),
            ..NodeAt::default()
        };
        run_on::<InsertPoint>(&mut doc, &p);
        let n = nodes::nodes(&doc.vectors[&p.vector]);
        assert_eq!(n.len(), 5);
        assert_eq!(n[1], Point::new(20.0, 10.0));
    }
}
