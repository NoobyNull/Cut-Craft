//! Convert Span to Arc: the span under the point becomes a arc.

use cut_craft_core::document::Document;
use cut_craft_core::nodes::{self, NodeAt};
use cut_craft_core::{Change, Command, CommandError, Meta};

pub struct SpanToArc;

impl Command for SpanToArc {
    const ID: &'static str = "span_to_arc";
    type Params = NodeAt;

    fn meta() -> Meta {
        Meta {
            label: "Convert Span to Arc",
            group: "Node Editing",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &NodeAt) -> Result<Change, CommandError> {
        let path = p.path(doc)?;
        let k = p.span(path)?;
        let mut c = Change::builder(doc);
        c.replace(p.vector, nodes::span_to_arc(path, k));
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{run_on, sample_document};

    cut_craft_core::contract_tests!(SpanToArc);

    #[test]
    fn a_square_side_bows_into_an_arc() {
        let mut doc = sample_document();
        let p = NodeAt {
            at: cut_craft_core::params::Xy::new(20.0, 10.0),
            ..NodeAt::default()
        };
        run_on::<SpanToArc>(&mut doc, &p);
        assert!(matches!(
            doc.vectors[&p.vector].segments[0],
            cut_craft_core::geometry::Segment::Arc { .. }
        ));
    }
}
