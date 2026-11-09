//! Snaps that stay: a point placed on a snap (the end or middle of a line, the centre of a
//! circle, where a line touches a curve or meets it square on, a corner of the material)
//! is kept there. When the thing it snapped to moves or changes, the point follows; when the
//! shape it is part of is moved on its own, the point is put back on its snap. The person
//! frees a point by picking its snap mark and pressing Delete.
//!
//! Every change goes through [`keeping`] before it is applied (see
//! [`History::apply`](crate::History::apply)): it adds the edits that keep each constraint,
//! so undo takes back the move and the points that followed it as one step.

use crate::change::{Change, Edit};
use crate::document::{Document, EntityId};
use crate::geometry::{Path, Point, Rect, Segment};
use crate::nodes::{move_node, nodes};
use crate::snap::{SnapKind, material_points, span_mid, touch_and_square};
use glam::DAffine2;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// What a point is kept on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "on", rename_all = "snake_case")]
pub enum Anchor {
    /// A node (where a span starts or ends) of a vector
    End { vector: EntityId, node: usize },
    /// Halfway along a span
    Mid { vector: EntityId, span: usize },
    /// The centre of an arc span
    Center { vector: EntityId, span: usize },
    /// The centre of a vector's outline box
    ShapeCenter { vector: EntityId },
    /// A corner of the material (0 bottom left, 1 top right, 2 top left, 3 bottom right) or
    /// its centre (4)
    Material { corner: u8 },
    /// Where the line to the point, from the node before it, just touches a span
    Tangent { vector: EntityId, span: usize },
    /// Where the line to the point, from the node before it, meets a span at a right angle
    Perpendicular { vector: EntityId, span: usize },
}

impl Anchor {
    /// The vector it is on, if any.
    pub fn vector(self) -> Option<EntityId> {
        match self {
            Self::End { vector, .. }
            | Self::Mid { vector, .. }
            | Self::Center { vector, .. }
            | Self::ShapeCenter { vector }
            | Self::Tangent { vector, .. }
            | Self::Perpendicular { vector, .. } => Some(vector),
            Self::Material { .. } => None,
        }
    }

    /// The snap it was made from, for its mark and its name.
    pub fn kind(self) -> SnapKind {
        match self {
            Self::End { .. } => SnapKind::End,
            Self::Mid { .. } => SnapKind::Mid,
            Self::Center { .. } => SnapKind::ArcCenter,
            Self::ShapeCenter { .. } => SnapKind::ObjectCenter,
            Self::Material { .. } => SnapKind::JobPoint,
            Self::Tangent { .. } => SnapKind::Tangent,
            Self::Perpendicular { .. } => SnapKind::Perpendicular,
        }
    }

    /// Whether the point is placed by the line that runs to it (tangent, perpendicular).
    fn needs_line(self) -> bool {
        matches!(self, Self::Tangent { .. } | Self::Perpendicular { .. })
    }
}

/// Which point of a vector is kept on its anchor, and how it is kept there.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "pin", rename_all = "snake_case")]
pub enum Pin {
    /// One node: kept by moving that node, so the shape stretches (a line's end)
    Node { node: usize },
    /// A point `offset` from the centre of the vector's outline box: kept by moving the
    /// whole shape, which keeps its size (a circle drawn from a snapped centre)
    Whole { offset: Point },
}

/// A point kept on what it was snapped to.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Constraint {
    /// The vector the point is part of
    pub vector: EntityId,
    pub pin: Pin,
    pub to: Anchor,
}

impl Constraint {
    /// Where the kept point is now, if its vector is here.
    pub fn at(&self, vectors: &BTreeMap<EntityId, Path>) -> Option<Point> {
        pin_at(vectors.get(&self.vector)?, self.pin)
    }

    /// The vectors it ties together.
    fn ties(&self, id: EntityId) -> bool {
        self.vector == id || self.to.vector() == Some(id)
    }
}

fn pin_at(path: &Path, pin: Pin) -> Option<Point> {
    match pin {
        Pin::Node { node } => nodes(path).get(node).copied(),
        Pin::Whole { offset } => Some(path.bounds().center() + offset),
    }
}

/// The node before `node` along `path`, which the line to it runs from.
fn node_before(path: &Path, node: usize) -> Option<usize> {
    match node {
        0 if path.closed => Some(nodes(path).len().checked_sub(1)?),
        0 => None,
        k => Some(k - 1),
    }
}

fn span_start(path: &Path, span: usize) -> Option<Point> {
    match span {
        0 => Some(path.start),
        k => path.segments.get(k - 1).map(Segment::end),
    }
}

/// Where `c`'s point belongs now, `near` where it is: `None` when it cannot be put there
/// (what it snapped to is gone, or a line can no longer touch the curve).
fn place(
    c: &Constraint,
    vectors: &BTreeMap<EntityId, Path>,
    material: Rect,
    near: Point,
) -> Option<Point> {
    let target = |v: EntityId| vectors.get(&v);
    let span = |v: EntityId, k: usize| {
        let path = target(v)?;
        Some((span_start(path, k)?, *path.segments.get(k)?))
    };
    match c.to {
        Anchor::End { vector, node } => nodes(target(vector)?).get(node).copied(),
        Anchor::Mid { vector, span: k } => span(vector, k).map(|(from, s)| span_mid(from, s)),
        Anchor::Center { vector, span: k } => match span(vector, k)?.1 {
            Segment::Arc { center, .. } => Some(center),
            _ => None,
        },
        Anchor::ShapeCenter { vector } => Some(target(vector)?.bounds().center()),
        Anchor::Material { corner } => material_points(material).get(usize::from(corner)).copied(),
        Anchor::Tangent { vector, span: k } | Anchor::Perpendicular { vector, span: k } => {
            let Pin::Node { node } = c.pin else {
                return None;
            };
            let own = vectors.get(&c.vector)?;
            let from = nodes(own)[node_before(own, node)?];
            let (start, s) = span(vector, k)?;
            let kind = c.to.kind();
            touch_and_square(start, s, from)
                .into_iter()
                .filter(|(_, k)| *k == kind)
                .map(|(at, _)| at)
                .min_by(|a, b| a.distance(near).total_cmp(&b.distance(near)))
        }
    }
}

/// `path` with `pin` moved to `to`.
fn moved(path: &Path, pin: Pin, to: Point) -> Path {
    match pin {
        Pin::Node { node } => move_node(path, node, to),
        Pin::Whole { offset } => {
            let by = to - (path.bounds().center() + offset);
            path.transformed(&DAffine2::from_translation(by))
        }
    }
}

/// What the person is told when a constraint cannot be kept.
fn broken(c: &Constraint) -> String {
    let what = match c.to {
        Anchor::Tangent { .. } => "a line no longer touches its curve (tangent)",
        Anchor::Perpendicular { .. } => "a line can no longer meet its line square on",
        Anchor::Material { .. } => "a point is no longer on the material's corner",
        _ => "a point lost what it was snapped to",
    };
    format!("{what}, so that snap was let go; draw it again to snap it again")
}

/// How far a kept point may be from its place and still count as there.
const TOLERANCE: f64 = 1e-9;
/// How many times round the constraints a change may go: a point that follows one that
/// followed another.
const PASSES: usize = 8;

/// The vectors, material and constraints as a change leaves them, and what it touched.
struct After {
    vectors: BTreeMap<EntityId, Path>,
    material: Rect,
    constraints: Vec<Constraint>,
    /// Vectors added or replaced
    touched: BTreeSet<EntityId>,
    removed: BTreeSet<EntityId>,
    job_moved: bool,
}

impl After {
    fn of(doc: &Document, change: &Change) -> Self {
        let mut after = Self {
            vectors: doc.vectors.clone(),
            material: doc.job.area(),
            constraints: doc.constraints.clone(),
            touched: BTreeSet::new(),
            removed: BTreeSet::new(),
            job_moved: false,
        };
        for e in &change.edits {
            match e {
                Edit::Add { id, path }
                | Edit::Replace {
                    id, after: path, ..
                } => {
                    after.vectors.insert(*id, path.clone());
                    after.touched.insert(*id);
                    after.removed.remove(id);
                }
                Edit::Remove { id, .. } => {
                    after.vectors.remove(id);
                    after.removed.insert(*id);
                }
                Edit::SetJob { after: job, .. } => {
                    after.material = job.area();
                    after.job_moved = true;
                }
                Edit::SetConstraints { after: kept, .. } => {
                    kept.clone_into(&mut after.constraints);
                }
                _ => {}
            }
        }
        after
    }

    /// Lets go of the constraints on vectors the change deleted, and on vectors given
    /// more or fewer spans (their nodes are numbered again).
    fn drop_lost(&mut self, doc: &Document, notes: &mut Vec<String>) {
        let removed = &self.removed;
        self.constraints
            .retain(|c| !removed.iter().any(|&id| c.ties(id)));
        let vectors = &self.vectors;
        let renumbered = |id: EntityId| {
            let count = |v: &BTreeMap<EntityId, Path>| v.get(&id).map(|p| p.segments.len());
            count(&doc.vectors).is_some() && count(&doc.vectors) != count(vectors)
        };
        let before = self.constraints.len();
        self.constraints
            .retain(|c| !renumbered(c.vector) && c.to.vector().is_none_or(|v| !renumbered(v)));
        if self.constraints.len() < before {
            notes.push(
                "A snapped shape's nodes were added or taken away, so its snaps were let go"
                    .to_owned(),
            );
        }
    }

    /// Puts every constraint stirred by the change (or new in it) back on its anchor, over
    /// a few passes for points that follow points; lets go, with a note, of those that
    /// cannot be kept.
    fn solve(&mut self, made: &[Constraint], notes: &mut Vec<String>) {
        let mut touched = self.touched.clone();
        for _ in 0..PASSES {
            let mut moved_now = BTreeSet::new();
            let mut keep = Vec::with_capacity(self.constraints.len());
            for c in &self.constraints {
                let stirred = touched.iter().any(|&id| c.ties(id))
                    || (self.job_moved && matches!(c.to, Anchor::Material { .. }))
                    || made.contains(c);
                // a vector on another sheet: left as it is until it is back
                let here = self.vectors.contains_key(&c.vector)
                    && c.to.vector().is_none_or(|v| self.vectors.contains_key(&v));
                if !stirred || !here {
                    keep.push(*c);
                    continue;
                }
                let placed = c
                    .at(&self.vectors)
                    .and_then(|now| Some((now, place(c, &self.vectors, self.material, now)?)));
                let Some((now, to)) = placed else {
                    notes.push(broken(c));
                    continue;
                };
                if to.distance(now) > TOLERANCE {
                    let path = moved(&self.vectors[&c.vector], c.pin, to);
                    self.vectors.insert(c.vector, path);
                    moved_now.insert(c.vector);
                }
                keep.push(*c);
            }
            self.constraints = keep;
            if moved_now.is_empty() {
                break;
            }
            touched = moved_now;
        }
    }
}

/// `change` with the edits that keep every constraint added: points whose snaps moved (or
/// whose own shape moved) put back on them, and constraints on removed vectors, or that
/// can no longer be kept, dropped with a note.
#[must_use]
pub fn keeping(doc: &Document, change: Change) -> Change {
    let made_some = change
        .edits
        .iter()
        .any(|e| matches!(e, Edit::SetConstraints { .. }));
    if doc.constraints.is_empty() && !made_some {
        return change;
    }
    let mut after = After::of(doc, &change);
    let made: Vec<Constraint> = after
        .constraints
        .iter()
        .filter(|c| !doc.constraints.contains(c))
        .copied()
        .collect();
    let before = after.constraints.clone();
    let start = after.vectors.clone();
    let mut notes = vec![];
    after.drop_lost(doc, &mut notes);
    after.solve(&made, &mut notes);
    let mut out = change;
    let mut put_back = false;
    for (id, now) in &after.vectors {
        if let Some(was) = start.get(id).filter(|was| *was != now) {
            put_back |= after.touched.contains(id);
            out.edits.push(Edit::Replace {
                id: *id,
                before: was.clone(),
                after: now.clone(),
            });
        }
    }
    if put_back {
        notes.push(
            "Snapped points were kept on their snaps; to free one, click its snap mark and \
             press Delete"
                .to_owned(),
        );
    }
    if after.constraints != before {
        out.edits.push(Edit::SetConstraints {
            before,
            after: after.constraints,
        });
    }
    notes.dedup();
    out.notes.extend(notes);
    out
}

/// Constraints for a vector just made from a drawing, from where its points snapped:
/// `snaps` are the snapped points with what they snapped to. A drawn line's nodes stretch
/// to stay on their snaps (`stretch`); any other shape is moved whole, by the first of its
/// points that snapped.
pub fn pins(
    path: &Path,
    vector: EntityId,
    snaps: &[(Point, Anchor)],
    stretch: bool,
) -> Vec<Constraint> {
    const NEAR: f64 = 1e-6;
    let mut out: Vec<Constraint> = vec![];
    let points = nodes(path);
    for &(at, to) in snaps {
        // never onto itself
        if to.vector() == Some(vector) {
            continue;
        }
        if stretch {
            let Some(node) = points.iter().position(|p| p.distance(at) <= NEAR) else {
                continue;
            };
            if to.needs_line() && node_before(path, node).is_none() {
                continue;
            }
            let pin = Pin::Node { node };
            // one snap for each node: the last placed there wins
            out.retain(|c| c.pin != pin);
            out.push(Constraint { vector, pin, to });
        } else if out.is_empty() && !to.needs_line() {
            let offset = at - path.bounds().center();
            out.push(Constraint {
                vector,
                pin: Pin::Whole { offset },
                to,
            });
        }
    }
    out
}

/// `existing` with `made` added: a point snapped again is kept on its new snap only, and
/// a shape moved whole by one snap only.
pub fn merged(existing: &[Constraint], made: Vec<Constraint>) -> Vec<Constraint> {
    let same = |a: &Constraint, b: &Constraint| {
        a.vector == b.vector
            && match (a.pin, b.pin) {
                (Pin::Node { node: x }, Pin::Node { node: y }) => x == y,
                (Pin::Whole { .. }, Pin::Whole { .. }) => true,
                _ => false,
            }
    };
    let mut out: Vec<Constraint> = existing
        .iter()
        .filter(|c| !made.iter().any(|m| same(c, m)))
        .copied()
        .collect();
    out.extend(made);
    out
}

impl Document {
    /// The kept points and what they are kept on, for their marks: index into
    /// `constraints`, where the point is, and the snap it was made from.
    pub fn constraint_marks(&self) -> Vec<(usize, Point, SnapKind)> {
        self.constraints
            .iter()
            .enumerate()
            .filter_map(|(i, c)| Some((i, c.at(&self.vectors)?, c.to.kind())))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::History;
    use crate::testing::{document_with, square};

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn ids(doc: &Document) -> Vec<EntityId> {
        doc.vectors.keys().copied().collect()
    }

    fn constrain(doc: &mut Document, history: &mut History, c: Vec<Constraint>) {
        let mut b = Change::builder(doc);
        b.set_constraints(c);
        history.apply(doc, b.finish()).unwrap();
    }

    fn move_by(doc: &mut Document, history: &mut History, id: EntityId, by: Point) {
        let mut b = Change::builder(doc);
        let path = doc.vectors[&id].transformed(&DAffine2::from_translation(by));
        b.replace(id, path);
        history.apply(doc, b.finish()).unwrap();
    }

    #[test]
    fn a_line_end_snapped_to_a_corner_follows_the_square_when_it_moves() {
        let mut doc = document_with(vec![
            square(10.0, 10.0, 20.0),
            Path::polyline(&[p(0.0, 0.0), p(30.0, 30.0)], false),
        ]);
        let [sq, line] = ids(&doc)[..] else { panic!() };
        let mut h = History::default();
        let to = Anchor::End {
            vector: sq,
            node: 2,
        };
        constrain(
            &mut doc,
            &mut h,
            vec![Constraint {
                vector: line,
                pin: Pin::Node { node: 1 },
                to,
            }],
        );
        move_by(&mut doc, &mut h, sq, p(5.0, -3.0));
        assert_eq!(nodes(&doc.vectors[&line]), [p(0.0, 0.0), p(35.0, 27.0)]);
        assert_eq!(doc.constraints.len(), 1, "still kept");
        // undo takes the square and the line back together
        h.undo(&mut doc);
        assert_eq!(nodes(&doc.vectors[&line]), [p(0.0, 0.0), p(30.0, 30.0)]);
    }

    #[test]
    fn a_line_moved_on_its_own_keeps_its_snapped_end_where_it_was() {
        let mut doc = document_with(vec![
            square(10.0, 10.0, 20.0),
            Path::polyline(&[p(0.0, 0.0), p(30.0, 30.0)], false),
        ]);
        let [sq, line] = ids(&doc)[..] else { panic!() };
        let mut h = History::default();
        let to = Anchor::End {
            vector: sq,
            node: 2,
        };
        constrain(
            &mut doc,
            &mut h,
            vec![Constraint {
                vector: line,
                pin: Pin::Node { node: 1 },
                to,
            }],
        );
        let mut b = Change::builder(&doc);
        b.replace(
            line,
            doc.vectors[&line].transformed(&DAffine2::from_translation(p(-5.0, 0.0))),
        );
        let change = keeping(&doc, b.finish());
        assert!(
            change.notes.iter().any(|n| n.contains("Delete")),
            "{:?}",
            change.notes
        );
        h.apply(&mut doc, change).unwrap();
        assert_eq!(nodes(&doc.vectors[&line]), [p(-5.0, 0.0), p(30.0, 30.0)]);
    }

    #[test]
    fn a_tangent_line_stays_tangent_when_the_circle_moves() {
        let mut doc = document_with(vec![
            Path::circle(p(50.0, 70.0), 10.0),
            Path::polyline(&[p(70.0, 70.0), p(55.0, 70.0 - 75.0_f64.sqrt())], false),
        ]);
        let [circle, line] = ids(&doc)[..] else {
            panic!()
        };
        let mut h = History::default();
        let to = Anchor::Tangent {
            vector: circle,
            span: 0,
        };
        constrain(
            &mut doc,
            &mut h,
            vec![Constraint {
                vector: line,
                pin: Pin::Node { node: 1 },
                to,
            }],
        );
        move_by(&mut doc, &mut h, circle, p(0.0, 5.0));
        let [from, end] = nodes(&doc.vectors[&line])[..] else {
            panic!()
        };
        assert_eq!(from, p(70.0, 70.0), "the free end stays");
        let c = p(50.0, 75.0);
        assert!(
            (end.distance(c) - 10.0).abs() < 1e-9,
            "on the circle: {end}"
        );
        assert!((end - c).dot(end - from).abs() < 1e-6, "touching it: {end}");
    }

    #[test]
    fn a_snap_that_can_no_longer_be_kept_is_let_go_with_a_note() {
        let mut doc = document_with(vec![
            Path::circle(p(50.0, 70.0), 10.0),
            Path::polyline(&[p(70.0, 70.0), p(55.0, 70.0 - 75.0_f64.sqrt())], false),
        ]);
        let [circle, line] = ids(&doc)[..] else {
            panic!()
        };
        let mut h = History::default();
        let to = Anchor::Tangent {
            vector: circle,
            span: 0,
        };
        constrain(
            &mut doc,
            &mut h,
            vec![Constraint {
                vector: line,
                pin: Pin::Node { node: 1 },
                to,
            }],
        );
        // the circle moved over the line's free end: no line from inside can touch it
        let mut b = Change::builder(&doc);
        b.replace(circle, Path::circle(p(68.0, 70.0), 10.0));
        let change = keeping(&doc, b.finish());
        assert!(
            change.notes.iter().any(|n| n.contains("tangent")),
            "{:?}",
            change.notes
        );
        h.apply(&mut doc, change).unwrap();
        assert_eq!(doc.constraints, []);
    }

    #[test]
    fn a_circle_drawn_on_a_corner_moves_whole_with_it_and_deleting_lets_go() {
        let mut doc = document_with(vec![
            square(10.0, 10.0, 20.0),
            Path::circle(p(30.0, 30.0), 4.0),
        ]);
        let [sq, circle] = ids(&doc)[..] else {
            panic!()
        };
        let mut h = History::default();
        let made = pins(
            &doc.vectors[&circle],
            circle,
            &[(
                p(30.0, 30.0),
                Anchor::End {
                    vector: sq,
                    node: 2,
                },
            )],
            false,
        );
        assert_eq!(made.len(), 1);
        constrain(&mut doc, &mut h, made);
        move_by(&mut doc, &mut h, sq, p(-10.0, 0.0));
        let b = doc.vectors[&circle].bounds();
        assert!(
            b.center().distance(p(20.0, 30.0)) < 1e-9,
            "same size, moved: {b:?}"
        );
        assert!((b.size().x - 8.0).abs() < 1e-9);
        // the square deleted: the snap goes with it
        let mut c = Change::builder(&doc);
        c.remove(sq);
        let c = c.finish();
        h.apply(&mut doc, c).unwrap();
        assert_eq!(doc.constraints, []);
        h.undo(&mut doc);
        assert_eq!(doc.constraints.len(), 1, "undo brings it back");
    }

    #[test]
    fn a_chain_of_snaps_follows_through() {
        // line a ends on the square's corner, line b ends on a's middle
        let mut doc = document_with(vec![
            square(10.0, 10.0, 20.0),
            Path::polyline(&[p(0.0, 0.0), p(30.0, 30.0)], false),
            Path::polyline(&[p(0.0, 30.0), p(15.0, 15.0)], false),
        ]);
        let [sq, a, b] = ids(&doc)[..] else { panic!() };
        let mut h = History::default();
        constrain(
            &mut doc,
            &mut h,
            vec![
                Constraint {
                    vector: a,
                    pin: Pin::Node { node: 1 },
                    to: Anchor::End {
                        vector: sq,
                        node: 2,
                    },
                },
                Constraint {
                    vector: b,
                    pin: Pin::Node { node: 1 },
                    to: Anchor::Mid { vector: a, span: 0 },
                },
            ],
        );
        move_by(&mut doc, &mut h, sq, p(10.0, 10.0));
        assert_eq!(nodes(&doc.vectors[&a])[1], p(40.0, 40.0));
        assert_eq!(nodes(&doc.vectors[&b])[1], p(20.0, 20.0));
    }

    #[test]
    fn a_line_end_on_a_material_corner_follows_a_bigger_material() {
        let mut doc = document_with(vec![Path::polyline(&[p(10.0, 10.0), p(0.0, 0.0)], false)]);
        let [line] = ids(&doc)[..] else { panic!() };
        let mut h = History::default();
        let top_right = material_points(doc.job.area())[1];
        let snapped = pins(
            &doc.vectors[&line],
            line,
            &[(p(0.0, 0.0), Anchor::Material { corner: 0 })],
            true,
        );
        constrain(&mut doc, &mut h, snapped);
        let mut job = doc.job;
        job.width += 100.0;
        let mut c = Change::builder(&doc);
        c.set_job(job);
        let c = c.finish();
        h.apply(&mut doc, c).unwrap();
        assert_eq!(nodes(&doc.vectors[&line])[1], doc.job.area().min);
        assert_ne!(top_right, material_points(doc.job.area())[1]);
    }

    #[test]
    fn constraints_are_saved_with_the_job() {
        let mut doc = document_with(vec![
            square(10.0, 10.0, 20.0),
            Path::circle(p(30.0, 30.0), 4.0),
        ]);
        let [sq, circle] = ids(&doc)[..] else {
            panic!()
        };
        doc.constraints = pins(
            &doc.vectors[&circle],
            circle,
            &[(
                p(30.0, 30.0),
                Anchor::End {
                    vector: sq,
                    node: 2,
                },
            )],
            false,
        );
        let back = crate::file::load_job(&crate::file::save_job(&doc)).unwrap();
        assert_eq!(back.constraints, doc.constraints);
        assert_eq!(back.constraint_marks(), [(0, p(30.0, 30.0), SnapKind::End)]);
    }

    #[test]
    fn snapping_a_point_again_replaces_its_old_snap() {
        let pin = Pin::Node { node: 1 };
        let old = Constraint {
            vector: EntityId(1),
            pin,
            to: Anchor::Material { corner: 0 },
        };
        let other = Constraint {
            pin: Pin::Node { node: 0 },
            ..old
        };
        let new = Constraint {
            to: Anchor::Material { corner: 1 },
            ..old
        };
        assert_eq!(merged(&[old, other], vec![new]), [other, new]);
    }

    #[test]
    fn tangent_and_perpendicular_need_the_line_before_the_point() {
        let line = Path::polyline(&[p(0.0, 0.0), p(10.0, 0.0)], false);
        let to = Anchor::Tangent {
            vector: EntityId(99),
            span: 0,
        };
        let made = pins(
            &line,
            EntityId(1),
            &[(p(0.0, 0.0), to), (p(10.0, 0.0), to)],
            true,
        );
        assert_eq!(made.len(), 1, "only the end the line runs to");
        assert_eq!(made[0].pin, Pin::Node { node: 1 });
        assert_eq!(pins(&line, EntityId(1), &[(p(0.0, 0.0), to)], false), []);
    }
}
