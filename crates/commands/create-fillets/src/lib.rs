//! Create Fillets: the corner clicked (where two straight spans meet) is rounded with an arc
//! of the radius, or given a dog-bone cut-out a tool of that radius can reach into.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Path, Point, Segment, arc_sweep};
use cut_craft_core::params::Xy;
use cut_craft_core::{Change, Command, CommandError, Drag, Interactive, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct CreateFillets;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FilletType {
    /// A rounded corner
    #[default]
    #[schemars(title = "Normal fillet")]
    Normal,
    /// A circle into the corner, so slotted pieces fit together
    #[schemars(title = "'Dog-Bone' fillet")]
    DogBone,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    #[schemars(title = "Fillet / tool radius", extend("x-unit" = "mm"), range(min = 0))]
    pub radius: f64,
    pub fillet_type: FilletType,
    /// Click on an angled corner to change it into a fillet
    pub corner: Xy,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            radius: 3.0,
            fillet_type: FilletType::Normal,
            corner: Xy::new(0.0, 0.0),
        }
    }
}

/// How near the click must be to a corner, in mm.
// ponytail: fixed reach; Aspire's is a few screen pixels, which a command cannot see
const REACH: f64 = 2.0;

impl Command for CreateFillets {
    const ID: &'static str = "create_fillets";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Create Fillets",
            group: "Create Vectors",
            icon: "drawing/create_vectors/create_fillets.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("radius", p.radius)?;
        let click: Point = p.corner.into();
        let nearest = doc
            .vectors
            .iter()
            .filter(|(id, _)| doc.editable(**id))
            .flat_map(|(id, path)| corners(path).into_iter().map(move |k| (*id, path, k)))
            .min_by(|a, b| {
                vertex(a.1, a.2)
                    .distance(click)
                    .total_cmp(&vertex(b.1, b.2).distance(click))
            });
        let Some((id, path, k)) =
            nearest.filter(|(_, path, k)| vertex(path, *k).distance(click) <= REACH)
        else {
            return Err(CommandError::Failed(
                "click on a corner where two straight spans meet".into(),
            ));
        };
        let rounded = fillet(path, k, p.radius, p.fillet_type)?;
        let mut c = Change::builder(doc);
        c.replace(id, rounded);
        Ok(c.finish())
    }
}

impl Interactive for CreateFillets {
    fn from_drag(drag: Drag, current: &Params) -> Params {
        Params {
            corner: Xy::new(drag.start.x, drag.start.y),
            ..current.clone()
        }
    }
}

/// Point `k` of the path: its start, then each segment's end.
fn vertex(path: &Path, k: usize) -> Point {
    if k == 0 {
        path.start
    } else {
        path.segments[k - 1].end()
    }
}

/// The vertices where two straight spans meet; a closed path's start counts.
fn corners(path: &Path) -> Vec<usize> {
    let line = |i: usize| matches!(path.segments.get(i), Some(Segment::Line { .. }));
    let n = path.segments.len();
    let mut out: Vec<usize> = (1..n).filter(|&k| line(k - 1) && line(k)).collect();
    if path.closed && n > 1 && line(n - 1) && line(0) {
        out.push(0);
    }
    out
}

/// `path` with corner `k` filleted.
fn fillet(path: &Path, k: usize, radius: f64, kind: FilletType) -> Result<Path, CommandError> {
    // a closed path's start corner moves inside by starting one vertex on
    let (path, k) = if k == 0 {
        let mut segments = path.segments[1..].to_vec();
        segments.push(path.segments[0]);
        let moved = Path {
            start: path.segments[0].end(),
            segments,
            closed: true,
        };
        let last = moved.segments.len() - 1;
        (moved, last)
    } else {
        (path.clone(), k)
    };
    let (before, at, after) = (vertex(&path, k - 1), vertex(&path, k), vertex(&path, k + 1));
    let (u, v) = ((at - before).normalize(), (after - at).normalize());
    let turn = u.perp_dot(v);
    let half = (-u).angle_to(v).abs() / 2.0; // half the corner's inside angle
    if turn.abs() < 1e-9 || half < 1e-9 {
        return Err(CommandError::Failed("that corner is straight".into()));
    }
    let bisector = (v - u).normalize();
    let (enter, leave, centre, ccw) = match kind {
        FilletType::Normal => {
            let reach = radius / half.tan();
            (
                at - u * reach,
                at + v * reach,
                at + bisector * (radius / half.sin()),
                turn > 0.0,
            )
        }
        FilletType::DogBone => {
            // a circle through the corner, centred on the bisector inside it
            let centre = at + bisector * radius;
            let enter = at - u * (2.0 * (centre - at).dot(-u));
            let leave = at + v * (2.0 * (centre - at).dot(v));
            // the way round that passes through the corner
            let ccw = arc_sweep(enter, at, centre, true) < arc_sweep(enter, leave, centre, true);
            (enter, leave, centre, ccw)
        }
    };
    if enter.distance(at) > at.distance(before) + 1e-9
        || leave.distance(at) > at.distance(after) + 1e-9
    {
        return Err(CommandError::Failed(
            "the radius is too big for that corner".into(),
        ));
    }
    let mut out = path;
    out.segments[k - 1] = Segment::Line { to: enter };
    out.segments.insert(
        k,
        Segment::Arc {
            to: leave,
            center: centre,
            ccw,
        },
    );
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{document_with, run_on, square};

    fn at(x: f64, y: f64, kind: FilletType) -> Params {
        Params {
            radius: 2.0,
            fillet_type: kind,
            corner: Xy::new(x, y),
        }
    }

    fn one_square() -> Document {
        document_with(vec![square(0.0, 0.0, 10.0)])
    }

    #[test]
    fn contract() {
        // the contract document has no corner at the default click; check the rest of it
        cut_craft_core::testing::id_is_a_slug_and_meta_is_labelled::<CreateFillets>();
        cut_craft_core::testing::default_params_round_trip_through_json::<CreateFillets>();
        cut_craft_core::testing::schema_describes_an_object::<CreateFillets>();
        let mut doc = one_square();
        let p = at(10.0, 10.0, FilletType::Normal);
        let change = CreateFillets::run(&doc, &p).unwrap();
        let before = doc.clone();
        doc.apply(&change).unwrap();
        doc.apply(&change.inverse()).unwrap();
        assert_eq!(doc, before);
    }

    #[test]
    fn a_normal_fillet_rounds_the_clicked_corner() {
        let mut doc = one_square();
        run_on::<CreateFillets>(&mut doc, &at(10.2, 9.9, FilletType::Normal));
        let path = doc.vectors.values().next().unwrap();
        assert_eq!(path.segments.len(), 5);
        let b = path.bounds();
        assert!((b.max - Point::new(10.0, 10.0)).length() < 1e-9);
        let pts = path.flatten(0.001);
        // the corner is cut: nothing nearer (10, 10) than the arc's 2(√2 − 1)·… gap
        let nearest = pts
            .iter()
            .map(|q| q.distance(Point::new(10.0, 10.0)))
            .fold(f64::MAX, f64::min);
        assert!(
            (nearest - 2.0 * (2_f64.sqrt() - 1.0)).abs() < 0.01,
            "{nearest}"
        );
    }

    #[test]
    fn the_start_corner_and_dog_bones() {
        let mut doc = one_square();
        run_on::<CreateFillets>(&mut doc, &at(0.0, 0.0, FilletType::DogBone));
        let path = doc.vectors.values().next().unwrap();
        assert!(path.closed);
        let pts = path.flatten(0.001);
        assert!(
            pts.iter().any(|q| q.distance(Point::ZERO) < 0.01),
            "the circle passes through the corner"
        );
        let b = path.bounds();
        assert!(
            b.min.x < -0.5 && b.min.y < -0.5,
            "it bulges past the corner: {b:?}"
        );
    }

    #[test]
    fn misses_and_big_radii_are_refused() {
        assert!(CreateFillets::run(&one_square(), &at(5.0, 5.0, FilletType::Normal)).is_err());
        let p = Params {
            radius: 20.0,
            ..at(10.0, 10.0, FilletType::Normal)
        };
        assert!(CreateFillets::run(&one_square(), &p).is_err());
    }
}
