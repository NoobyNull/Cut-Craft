//! Mirror (Mirror Selected Objects): flips the selection in place, about the job's centre,
//! or beside itself (to the left, right, top or bottom), or about the line between the ends
//! of the vector picked last; or leaves the selection and adds mirrored copies.

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::geometry::Point;
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
    /// Left to right and top to bottom at once
    Both,
}

/// What the selection is mirrored about.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum About {
    /// Flipped in place, about the selection's own centre
    #[default]
    Selection,
    /// About the job's centre lines
    JobCentre,
    /// Beside the selection, to the left: about its left edge
    Left,
    /// Beside the selection, to the right: about its right edge
    Right,
    /// Beside the selection, above: about its top edge
    Top,
    /// Beside the selection, below: about its bottom edge
    Bottom,
    /// About the line between the two ends of the vector picked last (which stays put)
    Line,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// Which way the selection is flipped
    #[schemars(extend("x-show-when" = {"about": ["selection", "job_centre"]}))]
    pub flip: Flip,
    #[serde(default)]
    pub about: About,
    /// Keep the originals and add the mirrored ones as copies
    #[serde(default)]
    pub create_mirrored_copy: bool,
}

/// The reflection in the line through `a` and `b`.
fn reflection(from: Point, to: Point) -> Option<DAffine2> {
    let normal = (to - from).try_normalize()?.perp();
    let flip = DMat2::IDENTITY - 2.0 * DMat2::from_cols(normal * normal.x, normal * normal.y);
    Some(DAffine2::from_mat2_translation(flip, from - flip * from))
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
        // about a line: the vector picked last is the mirror, the rest are mirrored
        let (mirrored, t): (Vec<EntityId>, DAffine2) = if p.about == About::Line {
            let line = doc.selection.last().ok_or(CommandError::NothingSelected)?;
            let path = &doc.vectors[&line];
            let end = path
                .segments
                .last()
                .map_or(path.start, cut_craft_core::geometry::Segment::end);
            let t = reflection(path.start, end).ok_or_else(|| {
                CommandError::Failed("the vector picked last has its two ends together. Pick a straight line to mirror about".into())
            })?;
            let rest: Vec<EntityId> = doc
                .selection
                .iter()
                .copied()
                .filter(|&id| id != line)
                .collect();
            if rest.is_empty() {
                return Err(CommandError::Failed(
                    "pick the objects, then last the line to mirror them about".into(),
                ));
            }
            (rest, t)
        } else {
            let bounds = doc
                .selection_bounds()
                .ok_or(CommandError::NothingSelected)?;
            let (c, flip) = match p.about {
                About::Selection => (bounds.center(), p.flip),
                About::JobCentre => (doc.job.area().center(), p.flip),
                About::Left => (
                    Point::new(bounds.min.x, bounds.center().y),
                    Flip::Horizontal,
                ),
                About::Right => (
                    Point::new(bounds.max.x, bounds.center().y),
                    Flip::Horizontal,
                ),
                About::Top => (Point::new(bounds.center().x, bounds.max.y), Flip::Vertical),
                About::Bottom => (Point::new(bounds.center().x, bounds.min.y), Flip::Vertical),
                About::Line => unreachable!("handled above"),
            };
            let scale = match flip {
                Flip::Horizontal => Point::new(-1.0, 1.0),
                Flip::Vertical => Point::new(1.0, -1.0),
                Flip::Both => Point::splat(-1.0),
            };
            let t = DAffine2::from_translation(c)
                * DAffine2::from_scale(scale)
                * DAffine2::from_translation(-c);
            if !p.create_mirrored_copy {
                return Change::transform_selected(doc, |_| t);
            }
            (doc.selection.iter().copied().collect(), t)
        };
        if p.create_mirrored_copy {
            return Ok(Change::copy_vectors(doc, &mirrored, &[t]));
        }
        let mut c = Change::builder(doc);
        for id in mirrored {
            c.replace(id, doc.vectors[&id].transformed(&t));
        }
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    cut_craft_core::contract_tests!(Mirror);

    use cut_craft_core::geometry::Path;
    use cut_craft_core::testing::{after, sample_document};

    #[test]
    fn horizontal_flip_keeps_the_bounds_and_swaps_sides() {
        let before = sample_document().selection_bounds().unwrap();
        let doc = after::<Mirror>(&Params::default());
        let b = doc.selection_bounds().unwrap();
        assert!((b.min - before.min).length() < 1e-9 && (b.max - before.max).length() < 1e-9);
        // the square at x 10..30 ends up at the far right, 205..225
        assert!((doc.vectors[&EntityId(1)].bounds().min.x - 205.0).abs() < 1e-9);
    }

    fn bounds_of(doc: &Document, id: u64) -> cut_craft_core::geometry::Rect {
        doc.vectors[&EntityId(id)].bounds()
    }

    #[test]
    fn mirroring_beside_with_a_copy_adds_the_flipped_copy_next_to_the_original() {
        let doc = sample_document();
        let before = doc.selection_bounds().unwrap();
        let doc = after::<Mirror>(&Params {
            about: About::Right,
            create_mirrored_copy: true,
            ..Params::default()
        });
        let all = doc.selection_bounds().unwrap();
        assert!(
            (all.min.x - before.min.x).abs() < 1e-9,
            "the originals stay"
        );
        let width = before.max.x - before.min.x;
        assert!(
            (all.max.x - (before.max.x + width)).abs() < 1e-9,
            "the copy lies beside"
        );
        assert_eq!(doc.vectors.len(), sample_document().vectors.len() * 2);
    }

    #[test]
    fn mirroring_about_the_job_centre_and_about_a_line() {
        let job_mid = sample_document().job.area().center();
        let doc = after::<Mirror>(&Params {
            about: About::JobCentre,
            flip: Flip::Vertical,
            ..Params::default()
        });
        let (a, b) = (bounds_of(&sample_document(), 1), bounds_of(&doc, 1));
        assert!((a.center().y + b.center().y - 2.0 * job_mid.y).abs() < 1e-9);
        // a diagonal line picked last: the square is reflected across it, the line stays
        let mut doc = sample_document();
        let diagonal = Path::polyline(&[Point::new(0.0, 0.0), Point::new(100.0, 100.0)], false);
        let mut c = Change::builder(&doc);
        let line = c.add(diagonal.clone());
        doc.apply(&c.finish()).unwrap();
        doc.selection.insert(line);
        let square = bounds_of(&doc, 1);
        doc.apply(
            &Mirror::run(
                &doc,
                &Params {
                    about: About::Line,
                    ..Params::default()
                },
            )
            .unwrap(),
        )
        .unwrap();
        let flipped = bounds_of(&doc, 1);
        assert!(
            (flipped.min.x - square.min.y).abs() < 1e-9
                && (flipped.min.y - square.min.x).abs() < 1e-9
        );
        assert_eq!(doc.vectors[&line], diagonal);
    }
}
