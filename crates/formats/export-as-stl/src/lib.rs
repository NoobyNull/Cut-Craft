//! Export the 3D model as a binary STL: the model's surface on its material block, one closed
//! solid in millimetres.

use cut_craft_core::component::model;
use cut_craft_core::document::Document;
use cut_craft_core::glam::DVec3;
use cut_craft_core::{CommandError, Exporter, FileFormat};

pub struct ExportStl;

/// Model cells along the longer side of the job.
// ponytail: fixed detail; following the job's modeling resolution (`Job::model_cell`) is the
// upgrade, at about 6x the file size for a Standard 600 x 400 job
const CELLS_ACROSS: usize = 500;

impl Exporter for ExportStl {
    const ID: &'static str = "export_as_stl";
    const FORMAT: FileFormat = FileFormat {
        label: "STL",
        extensions: &["stl"],
    };

    fn export(doc: &Document) -> Result<Vec<u8>, CommandError> {
        if doc.components.is_empty() {
            return Err(CommandError::Failed(
                "there is no 3D model to export: create a shape first".into(),
            ));
        }
        let surface = model(doc, CELLS_ACROSS);
        let m = &surface;
        if m.columns < 2 || m.rows < 2 {
            return Err(CommandError::Failed(
                "the job is too small to export".into(),
            ));
        }
        let top = |(column, row): (usize, usize)| {
            m.centre(column, row)
                .extend(m.heights[row * m.columns + column])
        };
        let low = |(column, row): (usize, usize)| m.centre(column, row).extend(m.bottom);
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
        // the edge cells, anticlockwise from above: outside is to the right going round
        let (last_column, last_row) = (m.columns - 1, m.rows - 1);
        let ring: Vec<(usize, usize)> = (0..last_column)
            .map(|c| (c, 0))
            .chain((0..last_row).map(|r| (last_column, r)))
            .chain((1..m.columns).rev().map(|c| (c, last_row)))
            .chain((1..m.rows).rev().map(|r| (0, r)))
            .collect();
        let middle = ((low((0, 0)) + low((last_column, last_row))) / 2.0)
            .truncate()
            .extend(m.bottom);
        for (i, &a) in ring.iter().enumerate() {
            let b = ring[(i + 1) % ring.len()];
            triangles.extend([
                [low(a), low(b), top(b)],
                [low(a), top(b), top(a)],
                // the bottom faces down: clockwise from above
                [middle, low(b), low(a)],
            ]);
        }
        Ok(binary_stl(&triangles))
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "STL stores single-precision floats"
)]
fn binary_stl(triangles: &[[DVec3; 3]]) -> Vec<u8> {
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

    #[test]
    fn the_solid_is_closed_and_consistently_turned() {
        let mesh = triangles(&ExportStl::export(&doc()).unwrap());
        // each directed edge once, and its reverse once: closed and all facing the same way
        let mut edges: HashMap<([u32; 3], [u32; 3]), i32> = HashMap::new();
        let key = |v: [f32; 3]| v.map(f32::to_bits);
        for t in &mesh {
            for i in 0..3 {
                *edges.entry((key(t[i]), key(t[(i + 1) % 3]))).or_default() += 1;
            }
        }
        assert!(
            edges
                .iter()
                .all(|(&(a, b), &n)| n == 1 && edges.get(&(b, a)) == Some(&1)),
            "an open or flipped edge"
        );
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
