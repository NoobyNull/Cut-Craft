//! Segment 3D Model: an STL split by planes square to one axis into segments, so a model
//! with undercuts can be machined in pieces on a 3-axis machine: one cut where asked, or
//! several in equal slices. The segments after the first can be turned over so their far
//! sides are machined; all are laid out side by side across the job, their tops at the
//! material surface, as components, and a segment that still has undercuts says so.

use cut_craft_core::component::{Combine, Component};
use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Point, Rect};
use cut_craft_core::glam::{DMat3, DVec3};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct Segment3dModel;

/// The axis the segmenting plane is square to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Axis {
    X,
    Y,
    /// Across the model's height: a top and a bottom segment
    #[default]
    Z,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FileUnits {
    #[default]
    Mm,
    Inches,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// The STL file (binary or ASCII)
    pub file: String,
    /// The numbers in the file are in these units
    pub units: FileUnits,
    #[schemars(title = "Segmenting plane square to")]
    pub axis: Axis,
    /// How many planes cut the model: one where Position says, or several in equal slices
    #[serde(default = "one")]
    #[schemars(title = "Number of cuts", range(min = 1, max = 20))]
    pub cuts: u32,
    /// Where the one plane cuts, as a share of the model's size along the axis
    #[schemars(extend("x-unit" = "%"), range(min = 0, max = 100))]
    #[schemars(extend("x-show-when" = {"cuts": 1}))]
    pub position: f64,
    /// Turn the segment beyond the plane over, so its far side faces up to be machined
    #[schemars(title = "Turn the second segment over")]
    pub turn_over: bool,
    /// Space between the laid-out segments
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub gap: f64,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            file: String::new(),
            units: FileUnits::Mm,
            axis: Axis::Z,
            cuts: 1,
            position: 50.0,
            turn_over: true,
            // ponytail: 5 mm between segments [inf]
            gap: 5.0,
        }
    }
}

fn one() -> u32 {
    1
}

/// Undercut area past which a segment is said to still have undercuts, mm² seen from above.
const UNDERCUT_NOTE: f64 = 1.0;

/// How close to the segmenting plane counts as on it, mm.
const PLANE_TOLERANCE: f64 = 1e-9;

/// The part of each triangle on the side of the plane `normal · v = at` that `normal`
/// points to, as triangles.
fn clip(tris: &[[DVec3; 3]], normal: DVec3, at: f64) -> Vec<[DVec3; 3]> {
    let mut out = vec![];
    for tri in tris {
        let side = tri.map(|v| normal.dot(v) - at);
        // only what reaches past the plane: faces lying in it belong to neither segment
        if side.iter().all(|&d| d <= PLANE_TOLERANCE) {
            continue;
        }
        let mut kept: Vec<DVec3> = vec![];
        for i in 0..3 {
            let (here, ahead) = (i, (i + 1) % 3);
            if side[here] >= 0.0 {
                kept.push(tri[here]);
            }
            if (side[here] >= 0.0) != (side[ahead] >= 0.0) {
                let t = side[here] / (side[here] - side[ahead]);
                kept.push(tri[here].lerp(tri[ahead], t));
            }
        }
        for i in 1..kept.len().saturating_sub(1) {
            out.push([kept[0], kept[i], kept[i + 1]]);
        }
    }
    out
}

fn bounds(tris: &[[DVec3; 3]]) -> Option<(DVec3, DVec3)> {
    tris.iter().flatten().fold(None, |b, v| {
        Some(b.map_or((*v, *v), |(lo, hi): (DVec3, DVec3)| {
            (lo.min(*v), hi.max(*v))
        }))
    })
}

/// The model cut by `p`'s planes square to `normal` (the model spanning `lo` to `hi`):
/// the slices, the far one first, the ones after it turned over when asked; empty ones left
/// out.
fn slices(
    tris: &[[DVec3; 3]],
    p: &Params,
    normal: DVec3,
    (lo, hi): (DVec3, DVec3),
) -> Vec<Vec<[DVec3; 3]>> {
    // the planes, as shares along the axis
    let shares: Vec<f64> = if p.cuts <= 1 {
        vec![p.position / 100.0]
    } else {
        (1..=p.cuts)
            .map(|k| f64::from(k) / f64::from(p.cuts + 1))
            .collect()
    };
    let at = |share: f64| normal.dot(lo) + normal.dot(hi - lo) * share;
    // the slices between the planes, the far one first (as one cut's segment beyond the
    // plane leads)
    let mut edges: Vec<Option<f64>> = vec![None];
    edges.extend(shares.iter().map(|&s| Some(at(s))));
    edges.push(None);
    let slices: Vec<Vec<[DVec3; 3]>> = edges
        .windows(2)
        .rev()
        .map(|w| {
            let above = w[0].map_or_else(|| tris.to_vec(), |a| clip(tris, normal, a));
            w[1].map_or(above.clone(), |b| clip(&above, -normal, -b))
        })
        .collect();
    // turned over about the X axis (Y for an X plane, so the turn keeps the cut square)
    let turn = if p.axis == Axis::X {
        DMat3::from_rotation_y(std::f64::consts::PI)
    } else {
        DMat3::from_rotation_x(std::f64::consts::PI)
    };
    slices
        .into_iter()
        .enumerate()
        .map(|(i, s)| {
            if i > 0 && p.turn_over {
                s.iter().map(|t| t.map(|v| turn * v)).collect()
            } else {
                s
            }
        })
        .filter(|s: &Vec<[DVec3; 3]>| !s.is_empty())
        .collect()
}

impl Command for Segment3dModel {
    const ID: &'static str = "segment_3d_model";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Segment 3D Model",
            group: "Modeling",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if p.file.is_empty() {
            return Err(CommandError::InvalidParams("choose an STL file".into()));
        }
        if !(0.0..=100.0).contains(&p.position) {
            return Err(CommandError::InvalidParams(
                "the plane must cut between 0 and 100 %".into(),
            ));
        }
        let bytes =
            std::fs::read(&p.file).map_err(|e| CommandError::Failed(format!("{}: {e}", p.file)))?;
        let unit = match p.units {
            FileUnits::Mm => 1.0,
            FileUnits::Inches => 25.4,
        };
        let tris: Vec<[DVec3; 3]> = fmt_import_stl::triangles(&bytes)
            .map_err(CommandError::Failed)?
            .iter()
            .map(|t| t.map(|v| v * unit))
            .collect();
        let (lo, hi) =
            bounds(&tris).ok_or_else(|| CommandError::Failed("the model is empty".into()))?;
        let normal = match p.axis {
            Axis::X => DVec3::X,
            Axis::Y => DVec3::Y,
            Axis::Z => DVec3::Z,
        };
        let segments = slices(&tris, p, normal, (lo, hi));
        if segments.len() < 2 {
            return Err(CommandError::Failed(
                "the plane misses the model: move it inside".into(),
            ));
        }
        let boxes: Vec<(DVec3, DVec3)> = segments.iter().filter_map(|s| bounds(s)).collect();
        let width: f64 = boxes.iter().map(|(l, h)| h.x - l.x).sum::<f64>() + p.gap;
        let job = doc.job.area();
        let mut left = job.center().x - width / 2.0;
        let cell = doc.job.model_cell();
        let mut c = Change::builder(doc);
        for (n, (tris, (l, h))) in segments.iter().zip(&boxes).enumerate() {
            ensure_positive("segment size", (h.x - l.x).min(h.y - l.y))?;
            let shift = DVec3::new(
                left - l.x,
                job.center().y - f64::midpoint(l.y, h.y),
                doc.job.thickness - h.z,
            );
            let placed: Vec<[DVec3; 3]> = tris.iter().map(|t| t.map(|v| v + shift)).collect();
            let corner = Point::new(left, job.center().y - (h.y - l.y) / 2.0);
            let area = Rect::new(corner, corner + Point::new(h.x - l.x, h.y - l.y));
            let mut k = Component::from_fn(
                &format!("Segment {}", n + 1),
                Combine::Add,
                area,
                cell,
                |_| None,
            );
            fmt_import_stl::top_down(&mut k, &placed);
            let under = fmt_import_stl::undercut(&placed);
            if under > UNDERCUT_NOTE {
                c.note(format!(
                    "Segment {} still has undercuts ({under:.0} mm² seen from above)",
                    n + 1
                ));
            }
            for z in &mut k.heights {
                if *z < 0.0 {
                    *z = f64::NAN;
                }
            }
            c.add_component(k);
            left += h.x - l.x + p.gap;
        }
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::document::EntityId;

    /// A 20 x 10 x 10 box with a 10 x 10 x 10 cube on top at its left end: an L from the
    /// front. Binary STL in a temporary file of its own.
    fn l_shape(name: &str) -> String {
        let cube = |lo: [f32; 3], hi: [f32; 3]| {
            let v = |x: usize, y: usize, z: usize| {
                [[lo[0], hi[0]][x], [lo[1], hi[1]][y], [lo[2], hi[2]][z]]
            };
            let quad = |a, b, c, d| [[a, b, c], [a, c, d]];
            [
                quad(v(0, 0, 1), v(1, 0, 1), v(1, 1, 1), v(0, 1, 1)),
                quad(v(0, 0, 0), v(0, 1, 0), v(1, 1, 0), v(1, 0, 0)),
                quad(v(0, 0, 0), v(1, 0, 0), v(1, 0, 1), v(0, 0, 1)),
                quad(v(0, 1, 0), v(0, 1, 1), v(1, 1, 1), v(1, 1, 0)),
                quad(v(0, 0, 0), v(0, 0, 1), v(0, 1, 1), v(0, 1, 0)),
                quad(v(1, 0, 0), v(1, 1, 0), v(1, 1, 1), v(1, 0, 1)),
            ]
            .concat()
        };
        let faces = [
            cube([0.0, 0.0, 0.0], [20.0, 10.0, 10.0]),
            cube([0.0, 0.0, 10.0], [10.0, 10.0, 20.0]),
        ]
        .concat();
        let mut out = vec![0u8; 80];
        out.extend(u32::try_from(faces.len()).unwrap().to_le_bytes());
        for t in faces {
            out.extend([0u8; 12]);
            for x in t.iter().flatten() {
                out.extend(x.to_le_bytes());
            }
            out.extend([0u8; 2]);
        }
        let path =
            std::env::temp_dir().join(format!("cut-craft-seg-{}-{name}.stl", std::process::id()));
        std::fs::write(&path, out).unwrap();
        path.display().to_string()
    }

    fn segmented(name: &str, p: &Params) -> Document {
        let mut doc = Document::default();
        let p = Params {
            file: l_shape(name),
            ..p.clone()
        };
        let change = Segment3dModel::run(&doc, &p).unwrap();
        doc.apply(&change).unwrap();
        doc
    }

    #[test]
    fn contract() {
        cut_craft_core::testing::id_is_a_slug_and_meta_is_labelled::<Segment3dModel>();
        cut_craft_core::testing::schema_describes_an_object::<Segment3dModel>();
        cut_craft_core::testing::default_params_round_trip_through_json::<Segment3dModel>();
        assert!(Segment3dModel::run(&Document::default(), &Params::default()).is_err());
    }

    #[test]
    fn a_plane_across_the_height_gives_two_side_by_side_segments_tops_at_the_surface() {
        let doc = segmented("halves", &Params::default());
        assert_eq!(doc.components.len(), 2);
        let (top, bottom) = (&doc.components[&EntityId(1)], &doc.components[&EntityId(2)]);
        assert!(
            top.extent().max.x < bottom.extent().min.x,
            "laid out left to right"
        );
        // the top segment: the 10 mm cube; the bottom: the 20 mm slab turned over
        assert!((top.extent().size().x - 10.0).abs() < 2.0 * top.cell);
        assert!((bottom.extent().size().x - 20.0).abs() < 2.0 * bottom.cell);
        for k in [top, bottom] {
            let (_, high) = k.height_range().unwrap();
            assert!((high - doc.job.thickness).abs() < 1e-6, "{high}");
        }
        let (low, high) = bottom.height_range().unwrap();
        assert!((high - low).abs() < 1e-6, "the slab's underside is flat");
    }

    #[test]
    fn several_cuts_make_equal_slices_laid_out_in_a_row() {
        // three planes across the L's 20 mm length: four 5 mm slices
        let doc = segmented(
            "slices",
            &Params {
                axis: Axis::X,
                cuts: 3,
                ..Params::default()
            },
        );
        assert_eq!(doc.components.len(), 4);
        let mut lefts: Vec<f64> = doc.components.values().map(|k| k.extent().min.x).collect();
        let sorted = {
            let mut s = lefts.clone();
            s.sort_by(f64::total_cmp);
            s
        };
        assert_eq!(lefts, sorted, "left to right, in order");
        lefts.dedup();
        assert_eq!(lefts.len(), 4);
    }

    #[test]
    fn without_turning_over_the_second_segment_shows_its_cut_top() {
        let doc = segmented(
            "kept",
            &Params {
                turn_over: false,
                ..Params::default()
            },
        );
        let bottom = &doc.components[&EntityId(2)];
        let (low, high) = bottom.height_range().unwrap();
        assert!((high - low).abs() < 1e-6, "the cut is flat");
    }

    #[test]
    fn a_plane_outside_the_model_is_refused() {
        let p = Params {
            file: l_shape("outside"),
            position: 100.0,
            ..Params::default()
        };
        assert!(Segment3dModel::run(&Document::default(), &p).is_err());
    }
}
