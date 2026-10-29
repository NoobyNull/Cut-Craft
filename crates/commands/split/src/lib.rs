//! Split Component: the selected component cut in two by a selected closed vector, as a cookie
//! cutter would: "- A" inside, "- B" outside. Several selected components are baked first.

use cut_craft_core::component::chosen_or_baked;
use cut_craft_core::document::Document;
use cut_craft_core::geometry::inside;
use cut_craft_core::{Change, Command, CommandError, Meta, NoParams};

pub struct Split;

impl Command for Split {
    const ID: &'static str = "split";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Split Component",
            group: "Modeling",
            icon: "modeling/split_selected_component.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        let (_, paths) = doc.selected_paths(|_| true);
        let [cutter] = paths.as_slice() else {
            return Err(CommandError::Failed(
                "select a component and one vector to split it along".into(),
            ));
        };
        // ponytail: closed cutters only; Aspire extends an open vector to the modeling area's
        // edges, add that when it is asked for
        if !cutter.closed {
            return Err(CommandError::Failed("split with a closed vector".into()));
        }
        let Some((ids, part)) = chosen_or_baked(doc) else {
            return Err(CommandError::Failed("select a component to split".into()));
        };
        let ring = [cutter.flatten(cut_craft_core::polygon::TOLERANCE)];
        let (mut a, mut b) = (part.clone(), part.clone());
        for (i, h) in part.heights.iter().enumerate() {
            if !h.is_nan() {
                let into = if inside(&ring, part.centre_of(i)) {
                    &mut b
                } else {
                    &mut a
                };
                into.heights[i] = f64::NAN;
            }
        }
        if a.height_range().is_none() || b.height_range().is_none() {
            return Err(CommandError::Failed(
                "the vector does not cross the component".into(),
            ));
        }
        a.name = format!("{} - A", part.name);
        b.name = format!("{} - B", part.name);
        let mut c = Change::builder(doc);
        for id in ids {
            c.remove_component(id);
        }
        c.add_component(a);
        c.add_component(b);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Point;
    use cut_craft_core::testing::{domed, run_on, square};

    /// The dome with a 10 mm square over its middle selected.
    fn with_square() -> Document {
        let mut doc = domed();
        let mut c = Change::builder(&doc);
        let id = c.add(square(15.0, 15.0, 10.0));
        c.select([id]);
        let change = c.finish();
        doc.apply(&change).unwrap();
        doc
    }

    cut_craft_core::contract_tests!(Split, with_square);

    #[test]
    fn the_square_cuts_the_dome_into_a_middle_and_a_ring() {
        let mut doc = with_square();
        run_on::<Split>(&mut doc, &NoParams {});
        let parts: Vec<_> = doc.components.values().collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].name, "dome - A");
        assert_eq!(parts[1].name, "dome - B");
        let (middle, rim) = (Point::new(20.0, 20.0), Point::new(13.5, 20.0));
        assert!(parts[0].height_at(middle).is_some() && parts[0].height_at(rim).is_none());
        assert!(parts[1].height_at(rim).is_some() && parts[1].height_at(middle).is_none());
    }

    #[test]
    fn a_cutter_missing_the_component_or_open_is_refused() {
        let mut doc = domed();
        let mut c = Change::builder(&doc);
        let id = c.add(square(0.0, 0.0, 5.0));
        c.select([id]);
        let change = c.finish();
        doc.apply(&change).unwrap();
        let err = Split::run(&doc, &NoParams {}).unwrap_err().to_string();
        assert!(err.contains("does not cross"), "{err}");
        assert!(Split::run(&domed(), &NoParams {}).is_err(), "no vector");
    }
}
