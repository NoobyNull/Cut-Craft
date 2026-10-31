//! Export the 3D model as a binary STL in millimetres: the model's surface, its detail set
//! by a tolerance or a triangle limit, and its back left open or closed by the material's
//! flat bottom, the front mirrored, or a double-sided job's other side.

use cut_craft_core::component::model;
use cut_craft_core::document::Document;
use cut_craft_core::glam::DVec3;
use cut_craft_core::simulation::Material;
use cut_craft_core::{CommandError, Exporter, FileFormat};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct ExportStl;

/// How the detail of the mesh is limited.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Limit {
    /// No point of the mesh further than the tolerance from the model
    #[default]
    #[schemars(title = "Approximate tolerance")]
    Tolerance,
    /// No more than this many triangles
    #[schemars(title = "Maximum number of triangles")]
    Triangles,
}

/// What closes the back of the mesh.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Back {
    /// None: an open shell, the model's face only
    #[schemars(title = "Leave open (no back face)")]
    Open,
    /// A copy of the front, mirrored onto the back of the material (a solid two-sided part)
    #[schemars(title = "Close with inverted front")]
    InvertedFront,
    /// The material's flat bottom
    #[default]
    #[schemars(title = "Close with a flat plane")]
    FlatPlane,
    /// A double-sided job's other side, turned over as the job turns
    #[schemars(title = "Close with other side (double-sided jobs)")]
    OtherSide,
}

/// Export Model as STL's settings.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Settings {
    #[schemars(title = "Limit triangles by")]
    pub limit: Limit,
    /// The furthest the mesh may stray from the model
    #[schemars(title = "Approximate tolerance", range(min = 0.001))]
    #[schemars(extend("x-unit" = "mm", "x-show-when" = {"limit": "tolerance"}))]
    pub tolerance: f64,
    #[schemars(title = "Maximum number of triangles", range(min = 100))]
    #[schemars(extend("x-show-when" = {"limit": "triangles"}))]
    pub triangles: usize,
    #[schemars(title = "Back face of triangulation")]
    pub back: Back,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            limit: Limit::Tolerance,
            tolerance: 0.05,
            triangles: 1_000_000,
            back: Back::FlatPlane,
        }
    }
}

/// A triangulated model and how far it strays from the model at most (mm).
pub struct Mesh {
    pub triangles: Vec<[DVec3; 3]>,
    pub error: f64,
}

/// Model cells along the longer side of the job tried, coarsest first; the last is also
/// what the error is measured against.
// ponytail: an even grid; an adaptive mesh (fewer triangles where it is flat) is the upgrade
const DETAIL: [usize; 6] = [25, 50, 100, 200, 400, 800];

impl Exporter for ExportStl {
    const ID: &'static str = "export_as_stl";
    const FORMAT: FileFormat = FileFormat {
        label: "STL",
        extensions: &["stl"],
    };

    fn export(doc: &Document) -> Result<Vec<u8>, CommandError> {
        Ok(binary_stl(
            &triangulate(doc, &Settings::default())?.triangles,
        ))
    }
}

/// The model as a mesh by `settings`.
pub fn triangulate(doc: &Document, settings: &Settings) -> Result<Mesh, CommandError> {
    if doc.components.is_empty() {
        return Err(CommandError::Failed(
            "there is no 3D model to export: create a shape first".into(),
        ));
    }
    if settings.back == Back::OtherSide
        && (doc.job.double_sided.is_none() || doc.other_side.components.is_empty())
    {
        return Err(CommandError::Failed(
            "closing with the other side needs a double-sided job with a model on its other side"
                .into(),
        ));
    }
    let finest = model(doc, DETAIL[DETAIL.len() - 1]);
    // tolerance: the coarsest close enough; triangles: the finest few enough (or the coarsest)
    let mut chosen = None;
    for &across in &DETAIL {
        let m = model(doc, across);
        let few_enough = match settings.limit {
            Limit::Tolerance => true,
            Limit::Triangles => triangles_for(&m, settings.back) <= settings.triangles,
        };
        if !few_enough && chosen.is_some() {
            break;
        }
        let error = error(&m, &finest);
        let close_enough = settings.limit == Limit::Tolerance && error <= settings.tolerance;
        chosen = Some((m, error));
        if close_enough || !few_enough {
            break;
        }
    }
    let (m, error) = chosen.expect("one level of detail is always tried");
    if m.columns < 2 || m.rows < 2 {
        return Err(CommandError::Failed(
            "the job is too small to export".into(),
        ));
    }
    let back = back_heights(doc, &m, settings.back);
    Ok(Mesh {
        triangles: mesh(&m, back.as_deref()),
        error,
    })
}

/// How many triangles `m` makes closed by `back`.
fn triangles_for(m: &Material, back: Back) -> usize {
    let surface = 2 * m.columns.saturating_sub(1) * m.rows.saturating_sub(1);
    let ring = 2 * (m.columns + m.rows).saturating_sub(2);
    match back {
        Back::Open => surface,
        Back::FlatPlane => surface + 3 * ring,
        Back::InvertedFront | Back::OtherSide => 2 * surface + 2 * ring,
    }
}

/// The furthest the surface through `coarse`'s cell centres is from `fine`'s heights.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "cell positions"
)]
fn error(coarse: &Material, fine: &Material) -> f64 {
    let height = |column: usize, row: usize| coarse.heights[row * coarse.columns + column];
    let mut worst: f64 = 0.0;
    for row in 0..fine.rows {
        for column in 0..fine.columns {
            // where the fine cell's centre falls among the coarse cell centres
            let at = (fine.centre(column, row) - coarse.origin) / coarse.cell - 0.5;
            let across = at.x.clamp(0.0, (coarse.columns - 1) as f64);
            let up = at.y.clamp(0.0, (coarse.rows - 1) as f64);
            let (left, below) = (across as usize, up as usize);
            let right = (left + 1).min(coarse.columns - 1);
            let above = (below + 1).min(coarse.rows - 1);
            let (sideways, upwards) = (across - left as f64, up - below as f64);
            let near = height(left, below) * (1.0 - sideways) + height(right, below) * sideways;
            let far = height(left, above) * (1.0 - sideways) + height(right, above) * sideways;
            let between = near * (1.0 - upwards) + far * upwards;
            worst = worst.max((between - fine.heights[row * fine.columns + column]).abs());
        }
    }
    worst
}

/// The back's height under each of `m`'s cells: the bottom less the relief on that side;
/// `None` for an open back.
fn back_heights(doc: &Document, m: &Material, back: Back) -> Option<Vec<f64>> {
    let relief: Vec<f64> = match back {
        Back::Open => return None,
        Back::FlatPlane => vec![0.0; m.heights.len()],
        Back::InvertedFront => m.heights.clone(),
        Back::OtherSide => {
            let flip = doc.job.double_sided.map(|d| d.flip).unwrap_or_default();
            let turn = flip.mirror(doc.job.area());
            let mut other = doc.clone();
            other.components = doc.other_side.components.clone();
            let o = model(&other, DETAIL[DETAIL.len() - 1]);
            (0..m.rows)
                .flat_map(|r| (0..m.columns).map(move |c| (c, r)))
                .map(|(c, r)| o.at(turn.transform_point2(m.centre(c, r))).unwrap_or(0.0))
                .collect()
        }
    };
    Some(relief.iter().map(|h| m.bottom - h).collect())
}

/// The surface through `m`'s cell centres, closed by `back` (heights under each cell)
/// with sides round the edge cells: a flat back as a fan, any other as a second surface.
fn mesh(m: &Material, back: Option<&[f64]>) -> Vec<[DVec3; 3]> {
    let top = |(column, row): (usize, usize)| {
        m.centre(column, row)
            .extend(m.heights[row * m.columns + column])
    };
    let mut triangles = vec![];
    // the surface, joining cell centres; anticlockwise from above faces up
    for row in 1..m.rows {
        for column in 1..m.columns {
            let [corner, right, opposite, up] = [
                (column - 1, row - 1),
                (column, row - 1),
                (column, row),
                (column - 1, row),
            ]
            .map(top);
            triangles.extend([[corner, right, opposite], [corner, opposite, up]]);
        }
    }
    let Some(back) = back else {
        return triangles;
    };
    let low = |(column, row): (usize, usize)| {
        m.centre(column, row).extend(back[row * m.columns + column])
    };
    // the edge cells, anticlockwise from above: outside is to the right going round
    let (last_column, last_row) = (m.columns - 1, m.rows - 1);
    let ring: Vec<(usize, usize)> = (0..last_column)
        .map(|c| (c, 0))
        .chain((0..last_row).map(|r| (last_column, r)))
        .chain((1..m.columns).rev().map(|c| (c, last_row)))
        .chain((1..m.rows).rev().map(|r| (0, r)))
        .collect();
    let flat = back.iter().all(|&z| (z - back[0]).abs() < 1e-9);
    let middle = ((low((0, 0)) + low((last_column, last_row))) / 2.0)
        .truncate()
        .extend(back[0]);
    for (i, &a) in ring.iter().enumerate() {
        let b = ring[(i + 1) % ring.len()];
        triangles.extend([[low(a), low(b), top(b)], [low(a), top(b), top(a)]]);
        if flat {
            // the bottom faces down: clockwise from above
            triangles.push([middle, low(b), low(a)]);
        }
    }
    if !flat {
        for row in 1..m.rows {
            for column in 1..m.columns {
                let [corner, right, opposite, up] = [
                    (column - 1, row - 1),
                    (column, row - 1),
                    (column, row),
                    (column - 1, row),
                ]
                .map(low);
                triangles.extend([[corner, opposite, right], [corner, up, opposite]]);
            }
        }
    }
    triangles
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "STL stores single-precision floats"
)]
pub fn binary_stl(triangles: &[[DVec3; 3]]) -> Vec<u8> {
    let mut out = Vec::with_capacity(84 + 50 * triangles.len());
    // the header must not start with "solid", which marks text STL
    let mut header = [0u8; 80];
    let title = b"Cut Craft model, mm";
    header[..title.len()].copy_from_slice(title);
    out.extend(header);
    let count = u32::try_from(triangles.len()).expect("fewer than 4 billion triangles");
    out.extend(count.to_le_bytes());
    for &[a, b, c] in triangles {
        let normal = (b - a).cross(c - a).normalize_or_zero();
        for v in [normal, a, b, c] {
            for k in v.to_array() {
                out.extend((k as f32).to_le_bytes());
            }
        }
        out.extend([0, 0]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::Change;
    use cut_craft_core::component::{Combine, Component};
    use cut_craft_core::document::Job;
    use cut_craft_core::geometry::{Point, Rect};
    use std::collections::HashMap;

    type Triangle = [[f32; 3]; 3];

    fn triangles(stl: &[u8]) -> Vec<Triangle> {
        let count = u32::from_le_bytes(stl[80..84].try_into().unwrap()) as usize;
        assert_eq!(
            stl.len(),
            84 + 50 * count,
            "header, count, 50 bytes a triangle"
        );
        let float = |at: usize| f32::from_le_bytes(stl[at..at + 4].try_into().unwrap());
        (0..count)
            .map(|t| {
                let at = 84 + 50 * t + 12; // after the normal
                [0, 1, 2].map(|v| [0, 1, 2].map(|k| float(at + 12 * v + 4 * k)))
            })
            .collect()
    }

    /// A 40 x 30 x 10 mm job with a 5 mm high block component over x 10..20, y 10..20.
    fn doc() -> Document {
        let mut doc = Document {
            job: Job {
                width: 40.0,
                height: 30.0,
                thickness: 10.0,
                ..Job::default()
            },
            ..Document::default()
        };
        let area = Rect::new(Point::new(10.0, 10.0), Point::new(20.0, 20.0));
        let mut c = Change::builder(&doc);
        c.add_component(Component::from_fn("block", Combine::Add, area, 0.5, |_| {
            Some(5.0)
        }));
        let change = c.finish();
        doc.apply(&change).unwrap();
        doc
    }

    fn by(doc: &Document, settings: Settings) -> (Vec<Triangle>, f64) {
        let mesh = triangulate(doc, &settings).unwrap();
        (triangles(&binary_stl(&mesh.triangles)), mesh.error)
    }

    #[test]
    fn the_back_is_open_or_closed_by_the_front_mirrored_or_the_other_side() {
        let doc = doc();
        let (open, _) = by(
            &doc,
            Settings {
                back: Back::Open,
                ..Settings::default()
            },
        );
        assert!(!closed(&open), "a shell");
        let (mirrored, _) = by(
            &doc,
            Settings {
                back: Back::InvertedFront,
                ..Settings::default()
            },
        );
        assert!(closed(&mirrored));
        let lowest = |mesh: &[Triangle]| {
            mesh.iter()
                .flatten()
                .map(|v| v[2])
                .fold(f32::INFINITY, f32::min)
        };
        assert!(
            (lowest(&mirrored) + 15.0).abs() < 1e-3,
            "the block's 5 mm below the bottom"
        );
        // the other side: refused on a single-sided job, then a 3 mm block under there
        let refused = triangulate(
            &doc,
            &Settings {
                back: Back::OtherSide,
                ..Settings::default()
            },
        );
        assert!(refused.is_err());
        let mut two = doc.clone();
        two.job.double_sided = Some(cut_craft_core::document::DoubleSided::default());
        let area = Rect::new(Point::new(25.0, 5.0), Point::new(35.0, 15.0));
        two.other_side.components.insert(
            cut_craft_core::document::EntityId(99),
            Component::from_fn("under", Combine::Add, area, 0.5, |_| Some(3.0)),
        );
        let (other, _) = by(
            &two,
            Settings {
                back: Back::OtherSide,
                ..Settings::default()
            },
        );
        assert!(closed(&other));
        assert!((lowest(&other) + 13.0).abs() < 1e-3, "{}", lowest(&other));
    }

    #[test]
    fn detail_follows_the_tolerance_or_the_triangle_limit() {
        let doc = doc();
        let rough = Settings {
            tolerance: 10.0,
            ..Settings::default()
        };
        let (coarse, error) = by(&doc, rough);
        assert!(error <= 10.0);
        let (fine, error) = by(
            &doc,
            Settings {
                tolerance: 0.001,
                ..Settings::default()
            },
        );
        assert!(fine.len() > coarse.len() && error < 10.0);
        let limit = Settings {
            limit: Limit::Triangles,
            triangles: 20_000,
            ..Settings::default()
        };
        let (few, _) = by(&doc, limit);
        assert!(few.len() <= 20_000 && few.len() > 2_000, "{}", few.len());
    }

    /// Each directed edge once, and its reverse once: closed and all facing the same way.
    fn closed(mesh: &[Triangle]) -> bool {
        let mut edges: HashMap<([u32; 3], [u32; 3]), i32> = HashMap::new();
        let key = |v: [f32; 3]| v.map(f32::to_bits);
        for t in mesh {
            for i in 0..3 {
                *edges.entry((key(t[i]), key(t[(i + 1) % 3]))).or_default() += 1;
            }
        }
        edges
            .iter()
            .all(|(&(a, b), &n)| n == 1 && edges.get(&(b, a)) == Some(&1))
    }

    #[test]
    fn the_solid_is_closed_and_consistently_turned() {
        let mesh = triangles(&ExportStl::export(&doc()).unwrap());
        assert!(closed(&mesh), "an open or flipped edge");
    }

    #[test]
    fn its_volume_is_the_block_plus_the_model() {
        let mesh = triangles(&ExportStl::export(&doc()).unwrap());
        // sum of signed tetrahedra to the origin; outward-facing triangles make it positive
        let volume: f64 = mesh
            .iter()
            .map(|t| {
                let [a, b, c] = t.map(|v| v.map(f64::from));
                (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
                    + a[2] * (b[0] * c[1] - b[1] * c[0]))
                    / 6.0
            })
            .sum();
        // the surface joins cell centres, so the block is half a cell short on each side
        let block = 10.0 * 40.0 * 30.0;
        let model = 5.0 * 10.0 * 10.0;
        assert!(
            (volume - block - model).abs() < 0.03 * block,
            "{volume} vs {}",
            block + model
        );
    }

    #[test]
    fn the_header_says_what_made_it() {
        let stl = ExportStl::export(&doc()).unwrap();
        assert!(stl.starts_with(b"Cut Craft model, mm"));
    }

    #[test]
    fn there_must_be_a_model() {
        let why = ExportStl::export(&Document::default())
            .unwrap_err()
            .to_string();
        assert!(why.contains("no 3D model"), "{why}");
    }
}
