//! Mirror.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Path, Point, Segment};
use cut_craft_core::glam::{DAffine2, DMat2};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct Mirror;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Flip {
    /// Left to right
    #[default]
    Horizontal,
    /// Top to bottom
    Vertical,
    Both,
}

/// What the selection is mirrored about.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum About {
    /// The centre of the selection, so it flips in place
    #[default]
    Selection,
    /// The centre of the job
    Job,
    /// The selection's left edge: with a copy, the copy lands to its left
    Left,
    /// The selection's right edge
    Right,
    /// The selection's top edge
    Top,
    /// The selection's bottom edge
    Bottom,
    /// The straight line picked last, which stays where it is
    Line,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub flip: Flip,
    #[serde(default)]
    pub about: About,
    /// Keep the originals and add the mirrored copies
    #[serde(default)]
    pub copy: bool,
}

/// The two ends of `path` if it is one straight line.
fn straight_line(path: &Path) -> Option<(Point, Point)> {
    match path.segments.as_slice() {
        [Segment::Line { to }] if !path.closed && path.start.distance(*to) > 1e-9 => {
            Some((path.start, *to))
        }
        _ => None,
    }
}

impl Command for Mirror {
    const ID: &'static str = "mirror";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Mirror",
            group: "Transform Objects",
            icon: "drawing/edit_objects/mirror.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let bounds = doc
            .selection_bounds()
            .ok_or(CommandError::NothingSelected)?;
        let mut vectors: Vec<_> = doc.selection.iter().copied().collect();
        let t = if p.about == About::Line {
            let line = doc.selection.last().filter(|_| vectors.len() > 1);
            let Some((from, to)) = line.and_then(|id| straight_line(&doc.vectors[&id])) else {
                return Err(CommandError::Failed(
                    "pick the line to mirror about last, after what it mirrors".into(),
                ));
            };
            vectors.pop();
            let d = (to - from).normalize();
            // reflection across the direction d: 2 d d^T - I
            let r = DMat2::from_cols(
                Point::new(2.0 * d.x * d.x - 1.0, 2.0 * d.x * d.y),
                Point::new(2.0 * d.x * d.y, 2.0 * d.y * d.y - 1.0),
            );
            DAffine2::from_translation(from)
                * DAffine2::from_mat2(r)
                * DAffine2::from_translation(-from)
        } else {
            let mut at = bounds.center();
            match p.about {
                About::Job => at = doc.job.area().center(),
                About::Left => at.x = bounds.min.x,
                About::Right => at.x = bounds.max.x,
                About::Top => at.y = bounds.max.y,
                About::Bottom => at.y = bounds.min.y,
                About::Selection | About::Line => {}
            }
            let scale = match p.flip {
                Flip::Horizontal => Point::new(-1.0, 1.0),
                Flip::Vertical => Point::new(1.0, -1.0),
                Flip::Both => Point::splat(-1.0),
            };
            DAffine2::from_translation(at)
                * DAffine2::from_scale(scale)
                * DAffine2::from_translation(-at)
        };
        let components = doc
            .selected_components
            .iter()
            .filter(|id| doc.components.contains_key(id));
        let mut change = Change::builder(doc);
        if p.copy {
            let mut selected = vectors.clone();
            for &id in &vectors {
                let copy = change.add(doc.vectors[&id].transformed(&t));
                change.move_to_layer(copy, doc.layer_of(id));
                selected.push(copy);
            }
            for id in components {
                change.add_component(doc.components[id].transformed(&t));
            }
            change.select(selected);
        } else {
            for &id in &vectors {
                change.replace(id, doc.vectors[&id].transformed(&t));
            }
            for id in components {
                change.replace_component(*id, doc.components[id].transformed(&t));
            }
        }
        Ok(change.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    cut_craft_core::contract_tests!(Mirror);

    use cut_craft_core::document::EntityId;
    use cut_craft_core::testing::{after, document_with, sample_document, square};

    #[test]
    fn horizontal_flip_keeps_the_bounds_and_swaps_sides() {
        let before = sample_document().selection_bounds().unwrap();
        let doc = after::<Mirror>(&Params::default());
        let b = doc.selection_bounds().unwrap();
        assert!((b.min - before.min).length() < 1e-9 && (b.max - before.max).length() < 1e-9);
        // the square at x 10..30 ends up at the far right, 205..225
        assert!((doc.vectors[&EntityId(1)].bounds().min.x - 205.0).abs() < 1e-9);
    }

    #[test]
    fn a_copy_about_the_right_edge_lands_beside_the_original() {
        let mut doc = document_with(vec![square(0.0, 0.0, 10.0)]);
        let p = Params {
            about: About::Right,
            copy: true,
            ..Params::default()
        };
        doc.apply(&Mirror::run(&doc, &p).unwrap()).unwrap();
        let mut xs: Vec<f64> = doc.vectors.values().map(|v| v.bounds().min.x).collect();
        xs.sort_by(f64::total_cmp);
        assert_eq!(xs.len(), 2);
        assert!(xs[0].abs() < 1e-9 && (xs[1] - 10.0).abs() < 1e-9, "{xs:?}");
        assert_eq!(doc.selection.len(), 2);
    }

    #[test]
    fn flips_about_the_job_centre() {
        let mut doc = document_with(vec![square(0.0, 0.0, 10.0)]);
        let centre = doc.job.area().center();
        let p = Params {
            about: About::Job,
            ..Params::default()
        };
        doc.apply(&Mirror::run(&doc, &p).unwrap()).unwrap();
        let b = doc.vectors.values().next().unwrap().bounds();
        assert!((b.max.x - 2.0 * centre.x).abs() < 1e-9, "{b:?}");
    }

    #[test]
    fn flips_about_the_line_picked_last_and_leaves_the_line() {
        let line = Path::polyline(&[Point::new(0.0, 0.0), Point::new(10.0, 10.0)], false);
        let mut doc = document_with(vec![square(5.0, 0.0, 2.0), line.clone()]);
        let p = Params {
            about: About::Line,
            ..Params::default()
        };
        doc.apply(&Mirror::run(&doc, &p).unwrap()).unwrap();
        let ids: Vec<_> = doc.selection.iter().copied().collect();
        assert_eq!(doc.vectors[&ids[1]], line);
        // across y = x the square at x 5..7, y 0..2 goes to x 0..2, y 5..7
        let b = doc.vectors[&ids[0]].bounds();
        assert!(b.min.distance(Point::new(0.0, 5.0)) < 1e-9, "{b:?}");
    }

    #[test]
    fn about_a_line_needs_a_straight_line_picked_last() {
        let doc = document_with(vec![square(0.0, 0.0, 10.0)]);
        let p = Params {
            about: About::Line,
            ..Params::default()
        };
        assert!(Mirror::run(&doc, &p).is_err());
    }
}
