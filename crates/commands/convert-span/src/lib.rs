//! Convert Span to Line, Arc or Bezier: the span under the point becomes that kind of
//! segment. Node editing's right-click menu offers the three.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::Path;
use cut_craft_core::nodes::{self, NodeAt};
use cut_craft_core::{Change, Command, CommandError, Meta};

fn meta(label: &'static str) -> Meta {
    Meta {
        label,
        group: "Node Editing",
        icon: "",
        shortcut: "",
    }
}

/// The vector `p` names with its span under `p` replaced by what `convert` makes of it.
fn converted(
    doc: &Document,
    p: &NodeAt,
    convert: impl Fn(&Path, usize) -> Path,
) -> Result<Change, CommandError> {
    let path = p.path(doc)?;
    let k = p.span(path)?;
    let mut c = Change::builder(doc);
    c.replace(p.vector, convert(path, k));
    Ok(c.finish())
}

pub struct SpanToLine;

impl Command for SpanToLine {
    const ID: &'static str = "span_to_line";
    type Params = NodeAt;

    fn meta() -> Meta {
        meta("Convert Span to Line")
    }

    fn run(doc: &Document, p: &NodeAt) -> Result<Change, CommandError> {
        converted(doc, p, nodes::span_to_line)
    }
}

pub struct SpanToArc;

impl Command for SpanToArc {
    const ID: &'static str = "span_to_arc";
    type Params = NodeAt;

    fn meta() -> Meta {
        meta("Convert Span to Arc")
    }

    fn run(doc: &Document, p: &NodeAt) -> Result<Change, CommandError> {
        converted(doc, p, nodes::span_to_arc)
    }
}

pub struct SpanToBezier;

impl Command for SpanToBezier {
    const ID: &'static str = "span_to_bezier";
    type Params = NodeAt;

    fn meta() -> Meta {
        meta("Convert Span to Bezier")
    }

    fn run(doc: &Document, p: &NodeAt) -> Result<Change, CommandError> {
        converted(doc, p, nodes::span_to_bezier)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Segment;
    use cut_craft_core::params::Xy;
    use cut_craft_core::testing::{run_on, sample_document};

    /// The sample's circle, pointed at halfway up its right side.
    fn on_the_circle(doc: &Document) -> NodeAt {
        NodeAt {
            vector: *doc.vectors.keys().last().unwrap(),
            at: Xy::new(200.0, 125.0),
            ..NodeAt::default()
        }
    }

    mod span_to_line {
        use super::*;
        cut_craft_core::contract_tests!(SpanToLine);

        #[test]
        fn a_circle_half_becomes_a_line() {
            let mut doc = sample_document();
            let p = on_the_circle(&doc);
            run_on::<SpanToLine>(&mut doc, &p);
            assert!(matches!(
                doc.vectors[&p.vector].segments[0],
                Segment::Line { .. }
            ));
        }
    }

    mod span_to_arc {
        use super::*;
        cut_craft_core::contract_tests!(SpanToArc);

        #[test]
        fn a_square_side_bows_into_an_arc() {
            let mut doc = sample_document();
            let p = NodeAt {
                at: Xy::new(20.0, 10.0),
                ..NodeAt::default()
            };
            run_on::<SpanToArc>(&mut doc, &p);
            assert!(matches!(
                doc.vectors[&p.vector].segments[0],
                Segment::Arc { .. }
            ));
        }
    }

    mod span_to_bezier {
        use super::*;
        cut_craft_core::contract_tests!(SpanToBezier);

        #[test]
        fn a_circle_half_becomes_beziers() {
            let mut doc = sample_document();
            let p = on_the_circle(&doc);
            run_on::<SpanToBezier>(&mut doc, &p);
            assert!(matches!(
                doc.vectors[&p.vector].segments[0],
                Segment::Cubic { .. }
            ));
        }
    }
}
