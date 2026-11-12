//! Segment 3D Model: an STL split by planes square to one axis into segments, so a model
//! with undercuts can be machined in pieces on a 3-axis machine: one cut where asked, or
//! several in equal slices. The segments after the first can be turned over so their far
//! sides are machined; all are laid out side by side across the job, their tops at the
//! material surface, as components, and a segment that still has undercuts says so.

use cut_craft_core::component::{Combine, Component};
use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Point, Rect};
use cut_craft_core::glam::{DMat3, DVec3};
use cut_craft_core::{Change, Command, CommandError, EMPTY_MODEL, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct Segment3dModel;

/// The axis the segmenting plane is square to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Axis {
    /// Across the model's length: a left and a right segment
    X,
    /// Across the model's width: a front and a back segment
    Y,
    /// Across the model's height: a top and a bottom segment
    #[default]
    Z,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FileUnits {
    /// The file was drawn in millimetres
    #[default]
    Mm,
    /// The file was drawn in inches
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
    /// The segment tree: a segment cut again by a plane of its own gives two segments
    /// named after it (cutting segment 2 gives 2.1 beyond the plane and 2.2 before it),
    /// which can be cut again in turn
    #[serde(default)]
    #[schemars(title = "Cut segments again")]
    pub further_cuts: Vec<FurtherCut>,
    /// Which face of a segment is turned up in the layout; segments not listed follow
    /// Turn the second segment over
    #[serde(default)]
    #[schemars(title = "Segment orientations")]
    pub orientations: Vec<Placement>,
}

/// One more segmenting plane, through one segment.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct FurtherCut {
    /// The segment's name, as its component is named: 2, or 2.1 for a part of segment 2
    pub segment: String,
    #[schemars(title = "Plane square to")]
    pub axis: Axis,
    /// Where the plane cuts, as a share of the segment's size along the axis
    #[schemars(extend("x-unit" = "%"), range(min = 0, max = 100))]
    pub position: f64,
}

/// The side of a segment that faces up when it is laid out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Face {
    /// As in the model
    #[default]
    Top,
    /// Turned over, its underside up
    Bottom,
    /// The side facing -Y turned up
    Front,
    /// The side facing +Y turned up
    Back,
    /// The side facing -X turned up
    Left,
    /// The side facing +X turned up
    Right,
}

/// A segment's orientation in the layout.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Placement {
    /// The segment's name, as its component is named
    pub segment: String,
    #[schemars(title = "Face up")]
    pub face: Face,
}

impl Face {
    /// The turn that brings this face up.
    fn turn(self) -> DMat3 {
        use std::f64::consts::{FRAC_PI_2, PI};
        match self {
            Face::Top => DMat3::IDENTITY,
            Face::Bottom => DMat3::from_rotation_x(PI),
            Face::Front => DMat3::from_rotation_x(-FRAC_PI_2),
            Face::Back => DMat3::from_rotation_x(FRAC_PI_2),
            Face::Left => DMat3::from_rotation_y(FRAC_PI_2),
            Face::Right => DMat3::from_rotation_y(-FRAC_PI_2),
        }
    }
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
            further_cuts: vec![],
            orientations: vec![],
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
/// the slices, the far one first, empty ones too.
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
    edges
        .windows(2)
        .rev()
        .map(|w| {
            let above = w[0].map_or_else(|| tris.to_vec(), |a| clip(tris, normal, a));
            w[1].map_or(above.clone(), |b| clip(&above, -normal, -b))
        })
        .collect()
}

fn axis_normal(axis: Axis) -> DVec3 {
    match axis {
        Axis::X => DVec3::X,
        Axis::Y => DVec3::Y,
        Axis::Z => DVec3::Z,
    }
}

/// A segment of the tree: its name and its triangles, where they are in the model.
struct Segment {
    name: String,
    tris: Vec<[DVec3; 3]>,
    /// From the first cut's slices after the first: turned over when asked
    beyond_first: bool,
}

/// `segments` with each further cut made in turn: the segment named is replaced by its
/// two parts, beyond the plane first, named after it.
fn cut_again(
    mut segments: Vec<Segment>,
    cuts: &[FurtherCut],
) -> Result<Vec<Segment>, CommandError> {
    for cut in cuts {
        let name = cut.segment.trim();
        let Some(i) = segments.iter().position(|s| s.name == name) else {
            return Err(CommandError::InvalidParams(format!(
                "there is no segment {name} to cut again"
            )));
        };
        let whole = segments.remove(i);
        let normal = axis_normal(cut.axis);
        let parts = bounds(&whole.tris).map(|(lo, hi)| {
            let at = normal.dot(lo) + normal.dot(hi - lo) * cut.position / 100.0;
            [
                clip(&whole.tris, normal, at),
                clip(&whole.tris, -normal, -at),
            ]
        });
        let Some([beyond, before]) = parts.filter(|[a, b]| !a.is_empty() && !b.is_empty()) else {
            return Err(CommandError::Failed(format!(
                "the plane misses segment {name}: move it inside"
            )));
        };
        for (k, tris) in [(2, before), (1, beyond)] {
            segments.insert(
                i,
                Segment {
                    name: format!("{name}.{k}"),
                    tris,
                    beyond_first: whole.beyond_first,
                },
            );
        }
    }
    Ok(segments)
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
                "set the plane between 0 and 100 %".into(),
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
        let (lo, hi) = bounds(&tris).ok_or_else(|| CommandError::Failed(EMPTY_MODEL.into()))?;
        let first: Vec<(usize, Vec<[DVec3; 3]>)> = slices(&tris, p, axis_normal(p.axis), (lo, hi))
            .into_iter()
            .enumerate()
            .filter(|(_, s)| !s.is_empty())
            .collect();
        if first.len() < 2 {
            return Err(CommandError::Failed(
                "the plane misses the model: move it inside".into(),
            ));
        }
        let tree = first
            .into_iter()
            .enumerate()
            .map(|(n, (i, tris))| Segment {
                name: (n + 1).to_string(),
                tris,
                beyond_first: i > 0,
            })
            .collect();
        let tree = cut_again(tree, &p.further_cuts)?;
        // turned over about the X axis (Y for an X plane, so the turn keeps the cut square),
        // or with the face asked for up
        let turn_over = if p.axis == Axis::X {
            DMat3::from_rotation_y(std::f64::consts::PI)
        } else {
            DMat3::from_rotation_x(std::f64::consts::PI)
        };
        let names: Vec<String> = tree.iter().map(|s| s.name.clone()).collect();
        let segments: Vec<Vec<[DVec3; 3]>> = tree
            .into_iter()
            .map(|s| {
                let asked = p.orientations.iter().find(|o| o.segment.trim() == s.name);
                let turn = match asked {
                    Some(o) => o.face.turn(),
                    None if s.beyond_first && p.turn_over => turn_over,
                    None => DMat3::IDENTITY,
                };
                s.tris.iter().map(|t| t.map(|v| turn * v)).collect()
            })
            .collect();
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
                &format!("Segment {}", names[n]),
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
                    names[n]
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

    fn named<'a>(doc: &'a Document, name: &str) -> &'a cut_craft_core::component::Component {
        doc.components
            .values()
            .find(|k| k.name == format!("Segment {name}"))
            .unwrap_or_else(|| panic!("no segment {name}"))
    }

    #[test]
    fn a_segment_cut_again_gives_two_named_after_it() {
        // the L cut across its height, then the bottom slab cut across its length, then
        // one half of that across its depth
        let cut = |segment: &str, axis| FurtherCut {
            segment: segment.into(),
            axis,
            position: 50.0,
        };
        let doc = segmented(
            "tree",
            &Params {
                further_cuts: vec![cut("2", Axis::X), cut("2.1", Axis::Y)],
                ..Params::default()
            },
        );
        let mut names: Vec<&str> = doc.components.values().map(|k| k.name.as_str()).collect();
        names.sort_unstable();
        assert_eq!(
            names,
            ["Segment 1", "Segment 2.1.1", "Segment 2.1.2", "Segment 2.2"]
        );
        // 2.2 is the slab's half nearest X = 0: 10 long; 2.1.1 a quarter, 10 by 5
        let size = |name| named(&doc, name).extent().size();
        assert!((size("2.2").x - 10.0).abs() < 1.0, "{:?}", size("2.2"));
        assert!((size("2.1.1").y - 5.0).abs() < 1.0, "{:?}", size("2.1.1"));
        let p = Params {
            file: l_shape("tree-missing"),
            further_cuts: vec![cut("3", Axis::X)],
            ..Params::default()
        };
        let why = Segment3dModel::run(&Document::default(), &p)
            .unwrap_err()
            .to_string();
        assert!(why.contains("no segment 3"), "{why}");
    }

    #[test]
    fn each_segment_can_have_its_own_face_up() {
        // across the length at 3/4: segment 2 is the L's 15 mm with the cube on it
        let p = |orientations| Params {
            axis: Axis::X,
            position: 75.0,
            orientations,
            ..Params::default()
        };
        let plain = segmented("faces", &p(vec![]));
        let size = named(&plain, "2").extent().size();
        assert!((size.y - 10.0).abs() < 1.0, "lying as modelled: {size:?}");
        let front = segmented(
            "faces-front",
            &p(vec![Placement {
                segment: "2".into(),
                face: Face::Front,
            }]),
        );
        let size = named(&front, "2").extent().size();
        assert!(
            (size.y - 20.0).abs() < 1.0,
            "its 20 mm height now across Y: {size:?}"
        );
        let (low, high) = named(&front, "2").height_range().unwrap();
        assert!(
            (high - low).abs() < 1e-6,
            "its flat front up, all of it 10 deep"
        );
    }
}
