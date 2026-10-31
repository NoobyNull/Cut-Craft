//! Smooth Point (node editing's right-click, Aspire's popup 142): the node under the point
//! becomes smooth, its two spans Béziers whose handles there lie on one line.

use cut_craft_core::document::Document;
use cut_craft_core::nodes::{self, NodeAt};
use cut_craft_core::{Change, Command, CommandError, Meta};

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

    fn run(doc: &Document, at: &NodeAt) -> Result<Change, CommandError> {
        let path = at.path(doc)?;
        let k = at.node(path)?;
        let smooth = nodes::smooth_node(path, k).ok_or_else(|| {
            CommandError::Failed("the end of an open vector has nothing to smooth against".into())
        })?;
        let mut c = Change::builder(doc);
        c.replace(at.vector, smooth);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Segment;
    use cut_craft_core::testing::{run_on, sample_document};

    cut_craft_core::contract_tests!(SmoothPoint);

    #[test]
    fn the_corner_under_the_point_becomes_smooth() {
        let mut doc = sample_document();
        let at = NodeAt::default();
        let before = at.path(&doc).unwrap().clone();
        run_on::<SmoothPoint>(&mut doc, &at);
        let after = &doc.vectors[&at.vector];
        assert_ne!(*after, before);
        assert!(
            after
                .segments
                .iter()
                .any(|s| matches!(s, Segment::Cubic { .. })),
            "its spans are Béziers"
        );
    }
}
