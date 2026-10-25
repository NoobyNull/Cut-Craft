//! Close With a Line: the open vector under the point is closed by a straight line from its
//! end back to its start.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::Segment;
use cut_craft_core::nodes::NodeAt;
use cut_craft_core::{Change, Command, CommandError, Meta};

pub struct CloseWithLine;

impl Command for CloseWithLine {
    const ID: &'static str = "close_with_line";
    type Params = NodeAt;

    fn meta() -> Meta {
        Meta {
            label: "Close With a Line",
            group: "Node Editing",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &NodeAt) -> Result<Change, CommandError> {
        let path = p.path(doc)?;
        if path.closed {
            return Err(CommandError::Failed("that vector is already closed".into()));
        }
        let mut closed = path.clone();
        let end = closed.segments.last().map_or(closed.start, Segment::end);
        if end.distance(closed.start) > 0.0 {
            closed.segments.push(Segment::Line { to: closed.start });
        }
        closed.closed = true;
        let mut c = Change::builder(doc);
        c.replace(p.vector, closed);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{run_on, sample_document};

    fn on_polyline() -> NodeAt {
        NodeAt {
            vector: cut_craft_core::document::EntityId(3),
            ..NodeAt::default()
        }
    }

    #[test]
    fn contract() {
        cut_craft_core::testing::id_is_a_slug_and_meta_is_labelled::<CloseWithLine>();
        cut_craft_core::testing::schema_describes_an_object::<CloseWithLine>();
        cut_craft_core::testing::default_params_round_trip_through_json::<CloseWithLine>();
    }

    #[test]
    fn the_polyline_closes_and_a_closed_square_is_refused() {
        let mut doc = sample_document();
        run_on::<CloseWithLine>(&mut doc, &on_polyline());
        let v = &doc.vectors[&on_polyline().vector];
        assert!(v.closed && v.segments.len() == 3);
        assert!(CloseWithLine::run(&sample_document(), &NodeAt::default()).is_err());
    }
}
