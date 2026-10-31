//! Move Nodes: the nodes picked with a box while node editing move together by one offset
//! (Aspire's drag of several selected nodes).

use cut_craft_core::document::Document;
use cut_craft_core::geometry::Point;
use cut_craft_core::nodes::{self, NodeAt};
use cut_craft_core::params::Xy;
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct MoveNodes;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// The nodes to move, each on its vector
    pub nodes: Vec<NodeAt>,
    /// How far they all move
    pub by: Xy,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            nodes: vec![NodeAt::default()],
            by: Xy::new(1.0, 0.0),
        }
    }
}

impl Command for MoveNodes {
    const ID: &'static str = "move_nodes";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Move Nodes",
            group: "Node Editing",
            icon: "",
            shortcut: "",
        }
    }

    /// The first node of the first selected vector, not moved yet.
    fn initial_params(doc: &Document) -> Params {
        let first = doc.selection.iter().next().and_then(|&vector| {
            let p = *nodes::nodes(doc.vectors.get(&vector)?).first()?;
            Some(NodeAt {
                vector,
                at: Xy::new(p.x, p.y),
                reach: 0.5,
            })
        });
        Params {
            nodes: first.into_iter().collect(),
            by: Xy::default(),
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if p.nodes.is_empty() {
            return Err(CommandError::InvalidParams("pick the nodes to move".into()));
        }
        let by: Point = p.by.into();
        let mut moved = std::collections::BTreeMap::new();
        for at in &p.nodes {
            let path = moved
                .get(&at.vector)
                .cloned()
                .map_or_else(|| at.path(doc).cloned(), Ok)?;
            // found where it was before anything moved: the node count never changes here
            let k = at.node(at.path(doc)?)?;
            let to = nodes::nodes(&path)[k] + by;
            moved.insert(at.vector, nodes::move_node(&path, k, to));
        }
        let mut c = Change::builder(doc);
        for (id, path) in moved {
            c.replace(id, path);
        }
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{run_on, sample_document};

    fn two_corners() -> Params {
        let doc = sample_document();
        let vector = NodeAt::default().vector;
        let corners = nodes::nodes(&doc.vectors[&vector]);
        Params {
            nodes: corners[..2]
                .iter()
                .map(|p| NodeAt {
                    vector,
                    at: Xy::new(p.x, p.y),
                    reach: 0.5,
                })
                .collect(),
            by: Xy::new(3.0, -2.0),
        }
    }

    cut_craft_core::contract_tests!(MoveNodes);

    #[test]
    fn every_picked_node_moves_by_the_same_offset_and_the_rest_stay() {
        let mut doc = sample_document();
        let p = two_corners();
        let vector = p.nodes[0].vector;
        let before = nodes::nodes(&doc.vectors[&vector]);
        run_on::<MoveNodes>(&mut doc, &p);
        let after = nodes::nodes(&doc.vectors[&vector]);
        let by = Point::new(3.0, -2.0);
        assert_eq!(&after[..2], &[before[0] + by, before[1] + by]);
        assert_eq!(&after[2..], &before[2..]);
    }

    #[test]
    fn nothing_picked_is_refused() {
        let none = Params {
            nodes: vec![],
            ..Params::default()
        };
        assert!(MoveNodes::run(&sample_document(), &none).is_err());
    }
}
