//! Move Node: the node under the point is dragged to a new place. Smooth Point: the
//! Bézier handles either side of it lined up.

use cut_craft_core::document::Document;
use cut_craft_core::nodes::{self, NodeAt};
use cut_craft_core::params::Xy;
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// The node to move
    pub from: NodeAt,
    /// Where it goes
    pub to: Xy,
}

pub struct MoveNode;

impl Command for MoveNode {
    const ID: &'static str = "move_node";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Move Node",
            group: "Node Editing",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let path = p.from.path(doc)?;
        let k = p.from.node(path)?;
        let mut c = Change::builder(doc);
        c.replace(p.from.vector, nodes::move_node(path, k, p.to.into()));
        Ok(c.finish())
    }
}

pub struct SmoothPoint;

impl Command for SmoothPoint {
    const ID: &'static str = "smooth_point";
    type Params = NodeAt;

    fn meta() -> Meta {
        Meta {
            label: "Smooth Point",
            group: "Node Editing",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &NodeAt) -> Result<Change, CommandError> {
        let path = p.path(doc)?;
        let k = p.node(path)?;
        let smooth = nodes::smooth_node(path, k).ok_or_else(|| {
            CommandError::Failed("there is no Bézier handle at this point to smooth".into())
        })?;
        let mut c = Change::builder(doc);
        c.replace(p.vector, smooth);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Point;
    use cut_craft_core::testing::{run_on, sample_document};

    cut_craft_core::contract_tests!(MoveNode);

    #[test]
    fn the_corner_moves() {
        let mut doc = sample_document();
        let p = Params {
            to: Xy::new(5.0, 5.0),
            ..Params::default()
        };
        run_on::<MoveNode>(&mut doc, &p);
        assert_eq!(doc.vectors[&p.from.vector].start, Point::new(5.0, 5.0));
    }
}
