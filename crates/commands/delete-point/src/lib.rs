//! Delete Point: the node under the point goes; its spans become one line.

use cut_craft_core::document::Document;
use cut_craft_core::nodes::{self, NodeAt};
use cut_craft_core::{Change, Command, CommandError, Meta};

pub struct DeletePoint;

impl Command for DeletePoint {
    const ID: &'static str = "delete_point";
    type Params = NodeAt;

    fn meta() -> Meta {
        Meta {
            label: "Delete Point",
            group: "Node Editing",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &NodeAt) -> Result<Change, CommandError> {
        let path = p.path(doc)?;
        let fewer = nodes::delete_node(path, p.node(path)?)
            .ok_or_else(|| CommandError::Failed("the vector needs the nodes it has left".into()))?;
        let mut c = Change::builder(doc);
        c.replace(p.vector, fewer);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Point;
    use cut_craft_core::testing::{run_on, sample_document};

    cut_craft_core::contract_tests!(DeletePoint);

    #[test]
    fn the_corner_goes() {
        let mut doc = sample_document();
        run_on::<DeletePoint>(&mut doc, &NodeAt::default());
        let n = nodes::nodes(&doc.vectors[&NodeAt::default().vector]);
        assert_eq!(n.len(), 3);
        assert!(!n.contains(&Point::new(10.0, 10.0)));
    }
}
