//! Distort Object: the selected vectors, or the selected components, bent into an envelope:
//! their bounding box with its corners moved, the band above an open curve, or the band
//! between two open curves (the last one or two vectors picked).

use cut_craft_core::component::Component;
use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::geometry::{Path, Point, Rect};
use cut_craft_core::params::Xy;
use cut_craft_core::trace::{self, Fit};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct DistortObject;

/// Which envelope the objects are bent into.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Their bounding box, its corners moved to the points below
    #[default]
    #[schemars(title = "Bounding Box Distortion")]
    BoundingBox,
    /// Bent to follow the open vector picked last; they keep their height above it
    #[schemars(title = "Along a Single Curve")]
    AlongCurve,
    /// Stretched between the two open vectors picked last
    #[schemars(title = "Between Two Curves")]
    BetweenCurves,
    /// Adds a rectangle round the objects, picked last, to reshape with Node Edit (N)
    #[schemars(title = "Create Envelope")]
    CreateEnvelope,
    /// Bent into the closed envelope picked last (its four corners and the sides between
    /// them, as Node Edit left them); the envelope goes
    #[schemars(title = "Distort to Envelope")]
    ToEnvelope,
}

impl Mode {
    /// How many of the last vectors picked are curves or the envelope, not objects.
    fn guides(self) -> usize {
        match self {
            Self::BoundingBox | Self::CreateEnvelope => 0,
            Self::AlongCurve | Self::ToEnvelope => 1,
            Self::BetweenCurves => 2,
        }
    }
}

/// Where the bounding box's corners go.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Corners {
    pub bottom_left: Xy,
    pub bottom_right: Xy,
    pub top_right: Xy,
    pub top_left: Xy,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub mode: Mode,
    /// Bounding Box Distortion only
    #[schemars(extend("x-section" = true))]
    pub corners: Corners,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            mode: Mode::BoundingBox,
            corners: corners_of(Rect::new(Point::ZERO, Point::new(20.0, 10.0))),
        }
    }
}

fn xy(p: Point) -> Xy {
    Xy::new(p.x, p.y)
}

fn corners_of(r: Rect) -> Corners {
    Corners {
        bottom_left: xy(r.min),
        bottom_right: xy(Point::new(r.max.x, r.min.y)),
        top_right: xy(r.max),
        top_left: xy(Point::new(r.min.x, r.max.y)),
    }
}

/// Samples along each edge of an envelope.
// ponytail: 256 [inf]; enough that the curves' chords are far finer than the shapes
const EDGE_SAMPLES: usize = 256;
/// Distorted vectors are refitted with Béziers within this, mm.
// ponytail: a fixed 0.01 mm; Aspire keeps the distortion live instead
const FIT_TOLERANCE: f64 = 0.01;

/// The band between a bottom and a top edge, each sampled evenly along its length: unit
/// square (u, v) goes to the point `v` of the way from the bottom edge to the top at `u`.
/// With left and right edges too (bottom to top) it is the Coons patch of all four.
struct Envelope {
    bottom: Vec<Point>,
    top: Vec<Point>,
    sides: Option<(Vec<Point>, Vec<Point>)>,
}

#[expect(clippy::cast_precision_loss, reason = "sample indices are small")]
impl Envelope {
    fn at(&self, across: f64, up: f64) -> Point {
        let (bottom, top) = (sample(&self.bottom, across), sample(&self.top, across));
        let ruled = bottom.lerp(top, up);
        let Some((left, right)) = &self.sides else {
            return ruled;
        };
        // Coons: the ruled surface, corrected toward the side edges
        let (u, v) = (across.clamp(0.0, 1.0), up.clamp(0.0, 1.0));
        let side = sample(left, v).lerp(sample(right, v), u);
        let corners = self.bottom[0]
            .lerp(self.bottom[EDGE_SAMPLES - 1], u)
            .lerp(self.top[0].lerp(self.top[EDGE_SAMPLES - 1], u), v);
        ruled + side - corners
    }

    fn straight(from: Point, to: Point) -> Vec<Point> {
        (0..EDGE_SAMPLES)
            .map(|i| from.lerp(to, i as f64 / (EDGE_SAMPLES - 1) as f64))
            .collect()
    }

    /// `points` walked left to right [inf] and resampled evenly by length.
    fn along(points: &[Point]) -> Vec<Point> {
        let mut points = points.to_vec();
        if points.first().map(|p| p.x) > points.last().map(|p| p.x) {
            points.reverse();
        }
        Self::resampled(&points)
    }

    /// `points` resampled evenly by length, in their own order.
    fn resampled(points: &[Point]) -> Vec<Point> {
        let lengths: Vec<f64> = std::iter::once(0.0)
            .chain(points.windows(2).scan(0.0, |run, w| {
                *run += w[0].distance(w[1]);
                Some(*run)
            }))
            .collect();
        let total = *lengths.last().unwrap_or(&0.0);
        let mut k = 0;
        (0..EDGE_SAMPLES)
            .map(|i| {
                let want = total * i as f64 / (EDGE_SAMPLES - 1) as f64;
                while k + 2 < lengths.len() && lengths[k + 1] < want {
                    k += 1;
                }
                let span = (lengths[k + 1] - lengths[k]).max(f64::EPSILON);
                points[k].lerp(points[k + 1], ((want - lengths[k]) / span).clamp(0.0, 1.0))
            })
            .collect()
    }

    /// Where `target` came from: the (u, v) that maps onto it, by Newton steps from the
    /// nearest point of a coarse lattice; `None` when it is outside the envelope.
    fn inverse(&self, target: Point, lattice: &[(f64, f64, Point)]) -> Option<(f64, f64)> {
        let &(mut across, mut up, _) = lattice.iter().min_by(|a, b| {
            a.2.distance_squared(target)
                .total_cmp(&b.2.distance_squared(target))
        })?;
        let step = 1e-4;
        for _ in 0..12 {
            let d_across = (self.at(across + step, up) - self.at(across - step, up)) / (2.0 * step);
            let d_up = (self.at(across, up + step) - self.at(across, up - step)) / (2.0 * step);
            let det = d_across.perp_dot(d_up);
            if det.abs() < 1e-12 {
                break;
            }
            let miss = target - self.at(across, up);
            across += miss.perp_dot(d_up) / det;
            up += d_across.perp_dot(miss) / det;
        }
        let unit = -1e-6..=1.0 + 1e-6;
        (unit.contains(&across)
            && unit.contains(&up)
            && self.at(across, up).distance(target) < 1e-3)
            .then_some((across, up))
    }
}

/// Edge `points` (evenly spaced) at `t` of the way along.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "sample indices are small"
)]
fn sample(points: &[Point], t: f64) -> Point {
    let along = t.clamp(0.0, 1.0) * (EDGE_SAMPLES - 1) as f64;
    let index = (along.floor() as usize).min(EDGE_SAMPLES - 2);
    points[index].lerp(points[index + 1], along - index as f64)
}

/// A closed envelope's four sides, each resampled: bottom and top left to right, left and
/// right bottom to top. Its corners are the nodes nearest its bounding box's corners.
fn sides_of(path: &Path) -> Result<Envelope, CommandError> {
    let nodes: Vec<Point> = std::iter::once(path.start)
        .chain(
            path.segments
                .iter()
                .map(cut_craft_core::geometry::Segment::end),
        )
        .collect();
    let count = path.segments.len();
    let b = path.bounds();
    let near = |corner: Point| {
        (0..count)
            .min_by(|&i, &j| {
                nodes[i]
                    .distance(corner)
                    .total_cmp(&nodes[j].distance(corner))
            })
            .unwrap_or(0)
    };
    let [bl, br, tr, tl] = [
        b.min,
        Point::new(b.max.x, b.min.y),
        b.max,
        Point::new(b.min.x, b.max.y),
    ]
    .map(near);
    let mut seen = [bl, br, tr, tl];
    seen.sort_unstable();
    if !path.closed || count < 4 || seen.windows(2).any(|w| w[0] == w[1]) {
        return Err(CommandError::Failed(
            "pick the envelope last: a closed vector with a node at each corner".into(),
        ));
    }
    // the spans from node `from` forward to node `to`, as points
    let forward = |from: usize, to: usize| {
        let mut points = vec![nodes[from]];
        let mut i = from;
        while i != to {
            let span = Path {
                start: nodes[i],
                segments: vec![path.segments[i]],
                closed: false,
            };
            points.extend(span.flatten(FIT_TOLERANCE).into_iter().skip(1));
            i = (i + 1) % count;
        }
        points
    };
    // a side between two corners, the way round that passes no other corner
    let side = |from: usize, to: usize| {
        let passes = |a: usize, b: usize| {
            let mut i = a;
            while i != b {
                if i != a && seen.contains(&i) {
                    return true;
                }
                i = (i + 1) % count;
            }
            false
        };
        let points = if passes(from, to) {
            let mut back = forward(to, from);
            back.reverse();
            back
        } else {
            forward(from, to)
        };
        Envelope::resampled(&points)
    };
    Ok(Envelope {
        bottom: side(bl, br),
        top: side(tl, tr),
        sides: Some((side(bl, tl), side(br, tr))),
    })
}

/// The objects to distort and the curves to distort them along, from the selection.
enum Objects {
    Vectors(Vec<EntityId>),
    Components(Vec<EntityId>),
}

fn open_curve(doc: &Document, id: EntityId) -> Result<Vec<Point>, CommandError> {
    let path = &doc.vectors[&id];
    if path.closed {
        return Err(CommandError::Failed(
            "pick the curve to distort along last: an open vector".into(),
        ));
    }
    Ok(path.flatten(FIT_TOLERANCE))
}

fn envelope(
    doc: &Document,
    p: &Params,
    area: Rect,
) -> Result<(Envelope, Vec<EntityId>), CommandError> {
    let picked: Vec<EntityId> = doc.selection.iter().copied().collect();
    let curves = p.mode.guides();
    if picked.len() < curves {
        return Err(CommandError::Failed(format!(
            "pick {curves} open vector(s) to distort along last"
        )));
    }
    let used = picked[picked.len() - curves..].to_vec();
    let c = p.corners;
    let made = match p.mode {
        Mode::BoundingBox => Envelope {
            bottom: Envelope::straight(c.bottom_left.into(), c.bottom_right.into()),
            top: Envelope::straight(c.top_left.into(), c.top_right.into()),
            sides: None,
        },
        Mode::AlongCurve => {
            let bottom = Envelope::along(&open_curve(doc, used[0])?);
            let height = area.size().y;
            let top = (0..EDGE_SAMPLES)
                .map(|i| {
                    let (a, b) = (
                        bottom[i.saturating_sub(1)],
                        bottom[(i + 1).min(EDGE_SAMPLES - 1)],
                    );
                    bottom[i] + (b - a).normalize_or_zero().perp() * height
                })
                .collect();
            Envelope {
                bottom,
                top,
                sides: None,
            }
        }
        Mode::BetweenCurves => {
            let (mut bottom, mut top) = (
                Envelope::along(&open_curve(doc, used[0])?),
                Envelope::along(&open_curve(doc, used[1])?),
            );
            let mean = |e: &[Point]| e.iter().map(|p| p.y).sum::<f64>();
            if mean(&bottom) > mean(&top) {
                std::mem::swap(&mut bottom, &mut top);
            }
            Envelope {
                bottom,
                top,
                sides: None,
            }
        }
        Mode::ToEnvelope => sides_of(&doc.vectors[&used[0]])?,
        Mode::CreateEnvelope => unreachable!("Create Envelope distorts nothing"),
    };
    Ok((made, used))
}

impl Command for DistortObject {
    const ID: &'static str = "distort_object";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Distort Object",
            group: "Transform Objects",
            icon: "drawing/create_vectors/distort_object.png",
            shortcut: "",
        }
    }

    fn initial_params(doc: &Document) -> Params {
        Params {
            corners: doc
                .selection_bounds()
                .map_or(Params::default().corners, corners_of),
            ..Params::default()
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let picked: Vec<EntityId> = doc.selection.iter().copied().collect();
        let curves = p.mode.guides();
        let vectors = picked[..picked.len().saturating_sub(curves)].to_vec();
        let components: Vec<EntityId> = doc
            .selected_components
            .iter()
            .copied()
            .filter(|id| doc.components.contains_key(id))
            .collect();
        let objects = match (vectors.is_empty(), components.is_empty()) {
            (false, false) => {
                return Err(CommandError::Failed(
                    "vectors and components cannot be distorted together".into(),
                ));
            }
            (false, true) => Objects::Vectors(vectors),
            (true, false) => Objects::Components(components),
            (true, true) => return Err(CommandError::NothingSelected),
        };
        let area = match &objects {
            Objects::Vectors(ids) => ids
                .iter()
                .map(|id| doc.vectors[id].bounds())
                .reduce(Rect::union),
            Objects::Components(ids) => ids
                .iter()
                .map(|id| doc.components[id].extent())
                .reduce(Rect::union),
        }
        .ok_or(CommandError::NothingSelected)?;
        if area.size().x <= f64::EPSILON || area.size().y <= f64::EPSILON {
            return Err(CommandError::Failed(
                "the objects have no width or height".into(),
            ));
        }
        if p.mode == Mode::CreateEnvelope {
            let corners = [
                area.min,
                Point::new(area.max.x, area.min.y),
                area.max,
                Point::new(area.min.x, area.max.y),
            ];
            let mut c = Change::builder(doc);
            let made = c.add(Path::polyline(&corners, true));
            let objects = match &objects {
                Objects::Vectors(ids) => ids.clone(),
                Objects::Components(_) => vec![],
            };
            c.select(objects.into_iter().chain([made]));
            return Ok(c.finish());
        }
        let (env, used) = envelope(doc, p, area)?;
        let unit = |q: Point| {
            (
                (q.x - area.min.x) / area.size().x,
                (q.y - area.min.y) / area.size().y,
            )
        };
        let mut c = Change::builder(doc);
        match objects {
            Objects::Vectors(ids) => {
                for id in ids {
                    c.replace(id, distort_path(&doc.vectors[&id], &env, &unit, area));
                }
            }
            Objects::Components(ids) => {
                for id in ids {
                    c.replace_component(id, distort_component(&doc.components[&id], &env, area));
                }
            }
        }
        if p.mode == Mode::ToEnvelope {
            c.remove(used[0]);
        }
        Ok(c.finish())
    }
}

/// The path's points, chords no longer than a hundredth of the area so straight spans bend,
/// moved into the envelope and refitted.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "a small subdivision count"
)]
fn distort_path(
    path: &Path,
    env: &Envelope,
    unit: &impl Fn(Point) -> (f64, f64),
    area: Rect,
) -> Path {
    let step = area.size().max_element() / 100.0;
    let flat = path.flatten(FIT_TOLERANCE);
    let mut dense = vec![flat[0]];
    for w in flat.windows(2) {
        let n = (w[0].distance(w[1]) / step).ceil().max(1.0) as usize;
        dense.extend((1..=n).map(|i| w[0].lerp(w[1], i as f64 / n as f64)));
    }
    if path.closed && dense.len() > 1 && dense[0].distance(dense[dense.len() - 1]) < 1e-9 {
        dense.pop();
    }
    let moved: Vec<Point> = dense
        .into_iter()
        .map(|q| {
            let (u, v) = unit(q);
            env.at(u, v)
        })
        .collect();
    trace::fit(
        &moved,
        path.closed,
        Fit::Beziers,
        FIT_TOLERANCE,
        30_f64.to_radians(),
    )
}

/// The component resampled over where the envelope lands: each new cell takes the height
/// from where it came from, heights unchanged.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "small lattice and cell counts"
)]
fn distort_component(k: &Component, env: &Envelope, area: Rect) -> Component {
    let k = k.baked();
    let n = ((area.size().max_element() / k.cell / 2.0).ceil() as usize).clamp(8, 128);
    let lattice: Vec<(f64, f64, Point)> = (0..=n)
        .flat_map(|i| (0..=n).map(move |j| (i as f64 / n as f64, j as f64 / n as f64)))
        .map(|(u, v)| (u, v, env.at(u, v)))
        .collect();
    let landed = lattice
        .iter()
        .fold(Rect::point(lattice[0].2), |r, l| r.include(l.2));
    let mut out = Component::from_fn(&k.name, k.combine, landed, k.cell, |q| {
        let (u, v) = env.inverse(q, &lattice)?;
        k.height_at(area.min + Point::new(u, v) * area.size())
    });
    out.visible = k.visible;
    out.level = k.level;
    out.group.clone_from(&k.group);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{domed, rejection, run_on};

    fn rectangle(doc: &mut Document, a: Point, b: Point) -> EntityId {
        let mut c = Change::builder(doc);
        let id = c.add(Path::polyline(
            &[a, Point::new(b.x, a.y), b, Point::new(a.x, b.y)],
            true,
        ));
        doc.apply(&c.finish()).unwrap();
        id
    }

    fn line(doc: &mut Document, points: &[Point]) -> EntityId {
        let mut c = Change::builder(doc);
        let id = c.add(Path::polyline(points, false));
        doc.apply(&c.finish()).unwrap();
        id
    }

    fn square() -> (Document, EntityId) {
        let mut doc = Document::default();
        let id = rectangle(&mut doc, Point::ZERO, Point::new(10.0, 10.0));
        doc.selection = [id].into();
        (doc, id)
    }

    fn near(a: Point, b: Point) -> bool {
        a.distance(b) < 0.05
    }

    cut_craft_core::contract_tests!(DistortObject, || square().0);

    #[test]
    fn moving_a_corner_pulls_the_shape_with_it() {
        let (mut doc, id) = square();
        let mut p = DistortObject::initial_params(&doc);
        p.corners.top_right = Xy::new(20.0, 15.0);
        run_on::<DistortObject>(&mut doc, &p);
        let points = doc.vectors[&id].flatten(0.01);
        assert!(points.iter().any(|&q| near(q, Point::new(20.0, 15.0))));
        assert!(points.iter().any(|&q| near(q, Point::ZERO)));
        let b = doc.vectors[&id].bounds();
        assert!(
            (b.max.x - 20.0).abs() < 0.05 && (b.max.y - 15.0).abs() < 0.05,
            "{b:?}"
        );
    }

    #[test]
    fn along_a_curve_the_shape_sits_on_it_and_keeps_its_height() {
        let (mut doc, id) = square();
        let arch: Vec<Point> = (0..=40)
            .map(|i| {
                let x = f64::from(i);
                Point::new(x, 10.0 * (x / 40.0 * std::f64::consts::PI).sin())
            })
            .collect();
        let curve = line(&mut doc, &arch);
        doc.selection = [id, curve].into();
        run_on::<DistortObject>(
            &mut doc,
            &Params {
                mode: Mode::AlongCurve,
                ..Params::default()
            },
        );
        let points = doc.vectors[&id].flatten(0.01);
        let on_curve = |q: Point| arch.iter().any(|&a| a.distance(q) < 0.3);
        assert!(
            points.iter().any(|&q| near(q, Point::ZERO)),
            "starts at the curve's start"
        );
        assert!(
            points.iter().filter(|&&q| on_curve(q)).count() > 5,
            "its bottom lies on the curve"
        );
        assert!(
            doc.vectors.contains_key(&curve),
            "the curve is left as it was"
        );
        assert_eq!(doc.vectors[&curve].flatten(0.01).len(), arch.len());
    }

    #[test]
    fn between_two_curves_the_shape_fills_the_band() {
        let (mut doc, id) = square();
        let low = line(&mut doc, &[Point::new(0.0, 0.0), Point::new(30.0, 0.0)]);
        let high = line(&mut doc, &[Point::new(0.0, 20.0), Point::new(30.0, 40.0)]);
        doc.selection = [id, high, low].into();
        run_on::<DistortObject>(
            &mut doc,
            &Params {
                mode: Mode::BetweenCurves,
                ..Params::default()
            },
        );
        let b = doc.vectors[&id].bounds();
        assert!(
            (b.max.x - 30.0).abs() < 0.05 && (b.max.y - 40.0).abs() < 0.05,
            "{b:?}"
        );
        assert!(b.min.length() < 0.05);
    }

    #[test]
    fn a_component_is_distorted_with_its_heights() {
        let mut doc = domed();
        doc.selected_components = [EntityId(1)].into();
        let top = doc.components[&EntityId(1)].height_range().unwrap().1;
        let mut p = DistortObject::initial_params(&doc);
        p.corners.bottom_right = Xy::new(36.0, 12.0);
        p.corners.top_right = Xy::new(36.0, 28.0);
        run_on::<DistortObject>(&mut doc, &p);
        let k = &doc.components[&EntityId(1)];
        assert!((k.extent().max.x - 36.0).abs() < 0.5, "{:?}", k.extent());
        assert!((k.height_range().unwrap().1 - top).abs() < 0.3);
        // the dome's middle went from x 20 to x 24: half-way across the stretched box
        assert!(k.height_at(Point::new(24.0, 20.0)).unwrap() > top - 0.5);
    }

    #[test]
    fn vectors_and_components_together_are_refused() {
        let (mut doc, _) = square();
        let dome = domed().components[&EntityId(1)].clone();
        let mut c = Change::builder(&doc);
        let k = c.add_component(dome);
        doc.apply(&c.finish()).unwrap();
        doc.selected_components = [k].into();
        assert!(DistortObject::run(&doc, &DistortObject::initial_params(&doc)).is_err());
    }

    #[test]
    fn a_closed_curve_is_refused() {
        let (mut doc, id) = square();
        let other = rectangle(&mut doc, Point::new(20.0, 0.0), Point::new(30.0, 5.0));
        doc.selection = [id, other].into();
        let p = Params {
            mode: Mode::AlongCurve,
            ..Params::default()
        };
        assert!(DistortObject::run(&doc, &p).is_err());
    }

    #[test]
    fn nothing_selected_is_an_error() {
        assert!(rejection::<DistortObject>(&Params::default()).contains("nothing"));
    }
}

#[cfg(test)]
mod envelope_tests {
    use super::*;
    use cut_craft_core::nodes::{insert_node, move_node, span_to_bezier};
    use cut_craft_core::testing::{domed, run_on};

    /// A 10 x 10 square at the origin, selected, and an envelope made round it.
    fn enveloped() -> (Document, EntityId, EntityId) {
        let mut doc = Document::default();
        let mut c = Change::builder(&doc);
        let square = c.add(Path::polyline(
            &[
                Point::ZERO,
                Point::new(10.0, 0.0),
                Point::new(10.0, 10.0),
                Point::new(0.0, 10.0),
            ],
            true,
        ));
        c.select([square]);
        doc.apply(&c.finish()).unwrap();
        let create = Params {
            mode: Mode::CreateEnvelope,
            ..Params::default()
        };
        run_on::<DistortObject>(&mut doc, &create);
        let picked: Vec<EntityId> = doc.selection.iter().copied().collect();
        (doc, square, picked[1])
    }

    fn edit(doc: &mut Document, id: EntityId, f: impl Fn(&Path) -> Path) {
        let after = f(&doc.vectors[&id]);
        let mut c = Change::builder(doc);
        c.replace(id, after);
        doc.apply(&c.finish()).unwrap();
    }

    fn to_envelope() -> Params {
        Params {
            mode: Mode::ToEnvelope,
            ..Params::default()
        }
    }

    #[test]
    fn create_envelope_adds_the_objects_box_picked_last() {
        let (doc, square, envelope) = enveloped();
        assert_eq!(
            doc.vectors[&envelope].bounds(),
            doc.vectors[&square].bounds()
        );
        assert_eq!(doc.vectors[&envelope].segments.len(), 4);
    }

    #[test]
    fn a_node_dragged_on_the_envelope_pulls_the_shape_and_the_envelope_goes() {
        let (mut doc, square, envelope) = enveloped();
        // the top right corner (node 2) out to (15, 15)
        edit(&mut doc, envelope, |p| {
            move_node(p, 2, Point::new(15.0, 15.0))
        });
        run_on::<DistortObject>(&mut doc, &to_envelope());
        assert!(!doc.vectors.contains_key(&envelope));
        let points = doc.vectors[&square].flatten(0.01);
        assert!(
            points
                .iter()
                .any(|q| q.distance(Point::new(15.0, 15.0)) < 0.05)
        );
        assert!(points.iter().any(|q| q.distance(Point::ZERO) < 0.05));
    }

    #[test]
    fn a_side_bent_with_a_node_in_its_middle_bends_the_shape() {
        let (mut doc, square, envelope) = enveloped();
        // a node in the middle of the top side, raised by 4: the top bulges, corners stay
        edit(&mut doc, envelope, |p| {
            insert_node(p, Point::new(5.0, 10.0), 0.5).unwrap()
        });
        let middle =
            cut_craft_core::nodes::node_near(&doc.vectors[&envelope], Point::new(5.0, 10.0), 0.5)
                .unwrap();
        edit(&mut doc, envelope, |p| {
            move_node(p, middle, Point::new(5.0, 14.0))
        });
        run_on::<DistortObject>(&mut doc, &to_envelope());
        let b = doc.vectors[&square].bounds();
        assert!((b.max.y - 14.0).abs() < 0.05, "{b:?}");
        assert!((b.min.y).abs() < 0.05 && (b.max.x - 10.0).abs() < 0.05);
    }

    #[test]
    fn a_curved_side_works_and_components_follow_too() {
        let mut doc = domed();
        doc.selected_components = [EntityId(1)].into();
        let create = Params {
            mode: Mode::CreateEnvelope,
            ..Params::default()
        };
        run_on::<DistortObject>(&mut doc, &create);
        let envelope = doc.selection.last().unwrap();
        edit(&mut doc, envelope, |p| span_to_bezier(p, 1));
        edit(&mut doc, envelope, |p| {
            move_node(p, 1, Point::new(34.0, 12.0))
        });
        edit(&mut doc, envelope, |p| {
            move_node(p, 2, Point::new(34.0, 28.0))
        });
        let top = doc.components[&EntityId(1)].height_range().unwrap().1;
        run_on::<DistortObject>(&mut doc, &to_envelope());
        let k = &doc.components[&EntityId(1)];
        assert!((k.extent().max.x - 34.0).abs() < 1.0, "{:?}", k.extent());
        assert!((k.height_range().unwrap().1 - top).abs() < 0.3);
    }

    #[test]
    fn an_open_or_too_simple_envelope_is_refused() {
        let (mut doc, _, envelope) = enveloped();
        edit(&mut doc, envelope, |p| Path {
            closed: false,
            ..p.clone()
        });
        assert!(DistortObject::run(&doc, &to_envelope()).is_err());
    }
}
