//! Move Node: the node under the point is dragged to a new place. Move Handle: a Bézier
//! span's handle is dragged to bend the span.

use cut_craft_core::document::Document;
use cut_craft_core::nodes::{self, NodeAt};
use cut_craft_core::params::Xy;
use cut_craft_core::{Change, Command, CommandError, Form, Meta};
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
            form: Form::Apply,
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

/// Move Handle's settings: which handle, and where it goes.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct HandleParams {
    /// The handle to move: the one of `vector` nearest `at`, within `reach`
    pub from: NodeAt,
    /// Where it goes
    pub to: Xy,
}

pub struct MoveHandle;

impl Command for MoveHandle {
    const ID: &'static str = "move_handle";
    type Params = HandleParams;

    fn meta() -> Meta {
        Meta {
            label: "Move Handle",
            group: "Node Editing",
            icon: "",
            shortcut: "",
            form: Form::Apply,
        }
    }

    fn run(doc: &Document, p: &HandleParams) -> Result<Change, CommandError> {
        let path = p.from.path(doc)?;
        let h = nodes::handle_near(path, p.from.at.into(), p.from.reach).ok_or_else(|| {
            CommandError::Failed(
                "there is no handle there. Make the span a curve first: right-click it and \
                 pick Convert Span to Bezier"
                    .into(),
            )
        })?;
        let mut c = Change::builder(doc);
        c.replace(
            p.from.vector,
            nodes::move_handle(path, h.span, h.second, p.to.into()),
        );
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

    mod move_handle {
        use super::*;
        use cut_craft_core::geometry::Segment;
        cut_craft_core::contract_tests!(MoveHandle, curved_sample);

        /// The sample with its square's bottom side a Bézier, the first handle pulled in
        /// beside the corner, where the default settings point.
        fn curved_sample() -> Document {
            let mut doc = sample_document();
            let id = NodeAt::default().vector;
            let curved = nodes::span_to_bezier(&doc.vectors[&id], 0);
            let h = nodes::handles(&curved)[0];
            let near = nodes::move_handle(&curved, h.span, h.second, Point::new(10.0, 10.5));
            doc.vectors.insert(id, near);
            doc
        }

        #[test]
        fn a_handle_bends_its_span() {
            let mut doc = curved_sample();
            let id = NodeAt::default().vector;
            let h = nodes::handles(&doc.vectors[&id])[1];
            let p = HandleParams {
                from: NodeAt {
                    vector: id,
                    at: Xy::new(h.at.x, h.at.y),
                    reach: 1.0,
                },
                to: Xy::new(h.at.x, h.at.y - 6.0),
            };
            run_on::<MoveHandle>(&mut doc, &p);
            let Segment::Cubic { c2, .. } = doc.vectors[&id].segments[0] else {
                panic!("still a Bézier");
            };
            assert_eq!(c2, Point::new(h.at.x, h.at.y - 6.0));
        }

        #[test]
        fn no_handle_there_says_how_to_get_one() {
            let doc = sample_document();
            let p = HandleParams::default();
            let Err(CommandError::Failed(why)) = MoveHandle::run(&doc, &p) else {
                panic!("a straight square has no handles");
            };
            assert!(why.contains("Convert Span to Bezier"), "{why}");
        }
    }
}
