//! Cut Vector: the vector is cut open at the node under the point (an open one in two).

use cut_craft_core::document::Document;
use cut_craft_core::nodes::{self, NodeAt};
use cut_craft_core::{Change, Command, CommandError, Meta};

pub struct CutVector;

impl Command for CutVector {
    const ID: &'static str = "cut_vector";
    type Params = NodeAt;

    fn meta() -> Meta {
        Meta {
            label: "Cut Vector",
            group: "Node Editing",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &NodeAt) -> Result<Change, CommandError> {
        let path = p.path(doc)?;
        let parts = nodes::cut_at(path, p.node(path)?);
        if parts.len() == 1 && !path.closed {
            return Err(CommandError::Failed(
                "cut between the ends of an open vector".into(),
            ));
        }
        Ok(Change::swap(doc, vec![p.vector], parts))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Point;
    use cut_craft_core::testing::{run_on, sample_document};

    cut_craft_core::contract_tests!(CutVector);

    #[test]
    fn a_closed_square_opens_at_the_corner() {
        let mut doc = sample_document();
        run_on::<CutVector>(&mut doc, &NodeAt::default());
        let opened = doc.vectors.values().last().unwrap();
        assert!(!opened.closed && opened.start == Point::new(10.0, 10.0));
    }
}
