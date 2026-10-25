//! Convert Span to Bezier: the span under the point becomes a bezier.

use cut_craft_core::document::Document;
use cut_craft_core::nodes::{self, NodeAt};
use cut_craft_core::{Change, Command, CommandError, Meta};

pub struct SpanToBezier;

impl Command for SpanToBezier {
    const ID: &'static str = "span_to_bezier";
    type Params = NodeAt;

    fn meta() -> Meta {
        Meta {
            label: "Convert Span to Bezier",
            group: "Node Editing",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &NodeAt) -> Result<Change, CommandError> {
        let path = p.path(doc)?;
        let k = p.span(path)?;
        let mut c = Change::builder(doc);
        c.replace(p.vector, nodes::span_to_bezier(path, k));
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{run_on, sample_document};

    cut_craft_core::contract_tests!(SpanToBezier);

    #[test]
    fn a_circle_half_becomes_beziers() {
        let mut doc = sample_document();
        let id = *doc.vectors.keys().last().unwrap();
        let p = NodeAt {
            vector: id,
            at: cut_craft_core::params::Xy::new(200.0, 125.0),
            ..NodeAt::default()
        };
        run_on::<SpanToBezier>(&mut doc, &p);
        assert!(matches!(
            doc.vectors[&id].segments[0],
            cut_craft_core::geometry::Segment::Cubic { .. }
        ));
    }
}
