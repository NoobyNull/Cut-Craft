//! Import an STL model (binary or ASCII) as a component: seen from above, its top surface
//! becomes a height grid at the job's modeling resolution, centred on the job with its lowest
//! point on the zero plane, shrunk to fit the job if it is bigger.

use cut_craft_core::component::{Combine, Component};
use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Point, Rect};
use cut_craft_core::glam::DVec3;
use cut_craft_core::{Change, CommandError, FileFormat, Importer};

pub struct ImportStl;

impl Importer for ImportStl {
    const ID: &'static str = "import_stl";
    const FORMAT: FileFormat = FileFormat {
        label: "STL model",
        extensions: &["stl"],
    };

    fn import(doc: &Document, bytes: &[u8]) -> Result<Change, CommandError> {
        let triangles = triangles(bytes).map_err(CommandError::Failed)?;
        let (lo, hi) = triangles.iter().flatten().fold(
            (DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY)),
            |(lo, hi), p| (lo.min(*p), hi.max(*p)),
        );
        let size = hi - lo;
        if triangles.is_empty() || size.x <= 0.0 || size.y <= 0.0 {
            return Err(CommandError::Failed(
                "the model has no area seen from above".into(),
            ));
        }
        // ponytail: native size, centred, shrunk to fit; Aspire asks in its Orientate 3D
        // Model step (size, position, depth below top), add that form when it is needed
        let area = doc.job.area();
        let room = area.size();
        let scale = (room.x / size.x).min(room.y / size.y).min(1.0);
        let centre = area.center();
        let footprint = Point::new(size.x, size.y) * scale;
        let place = |p: DVec3| {
            DVec3::new(
                centre.x + (p.x - f64::midpoint(lo.x, hi.x)) * scale,
                centre.y + (p.y - f64::midpoint(lo.y, hi.y)) * scale,
                (p.z - lo.z) * scale,
            )
        };
        let placed: Vec<[DVec3; 3]> = triangles.iter().map(|t| t.map(place)).collect();
        let rect = Rect::new(centre - footprint / 2.0, centre + footprint / 2.0);
        let mut component = Component::from_fn(
            "Imported model",
            Combine::Add,
            rect,
            doc.job.model_cell(),
            |_| None,
        );
        top_down(&mut component, &placed);
        let mut c = Change::builder(doc);
        c.add_component(component);
        Ok(c.finish())
    }
}

/// Undercuts, mm² seen from above: faces turned down that are higher than the model's
/// lowest point, with air under them a cutter coming from above cannot reach.
pub fn undercut(triangles: &[[DVec3; 3]]) -> f64 {
    // ponytail: a tenth of a millimetre clear of the base counts; a bowed base would count too
    const ABOVE: f64 = 0.1;
    let lowest = triangles
        .iter()
        .flatten()
        .map(|p| p.z)
        .fold(f64::INFINITY, f64::min);
    triangles
        .iter()
        .filter(|t| t.iter().map(|p| p.z).fold(f64::INFINITY, f64::min) > lowest + ABOVE)
        .map(|[a, b, c]| (b - a).cross(c - a).z / 2.0)
        .filter(|&up| up < 0.0)
        .map(|up| -up)
        .sum()
}

/// Raises each cell of `k` to the highest triangle above its centre.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "cell indices are small and clamped non-negative"
)]
#[expect(
    clippy::many_single_char_names,
    reason = "the triangle's corners and barycentric u, v"
)]
pub fn top_down(k: &mut Component, triangles: &[[DVec3; 3]]) {
    let cell = k.cell;
    let index = |v: f64, n: usize| (v.max(0.0) as usize).min(n.saturating_sub(1));
    for [a, b, c] in triangles {
        let (ab, ac) = (b - a, c - a);
        let det = ab.x * ac.y - ab.y * ac.x;
        if det.abs() < 1e-12 {
            continue; // edge-on from above
        }
        let lo = a.min(*b).min(*c);
        let hi = a.max(*b).max(*c);
        let c0 = index((lo.x - k.origin.x) / cell - 0.5, k.columns);
        let c1 = index((hi.x - k.origin.x) / cell + 0.5, k.columns);
        let r0 = index((lo.y - k.origin.y) / cell - 0.5, k.rows);
        let r1 = index((hi.y - k.origin.y) / cell + 0.5, k.rows);
        for row in r0..=r1 {
            for col in c0..=c1 {
                let p = k.origin + Point::new(col as f64 + 0.5, row as f64 + 0.5) * cell;
                let (dx, dy) = (p.x - a.x, p.y - a.y);
                let u = (dx * ac.y - dy * ac.x) / det;
                let v = (ab.x * dy - ab.y * dx) / det;
                if u < -1e-9 || v < -1e-9 || u + v > 1.0 + 1e-9 {
                    continue;
                }
                let z = a.z + u * ab.z + v * ac.z;
                let h = &mut k.heights[row * k.columns + col];
                if h.is_nan() || z > *h {
                    *h = z;
                }
            }
        }
    }
}

/// The triangles of a binary or ASCII STL file.
pub fn triangles(bytes: &[u8]) -> Result<Vec<[DVec3; 3]>, String> {
    let binary_count = bytes
        .get(80..84)
        .map(|c| u32::from_le_bytes(c.try_into().expect("4 bytes")) as usize);
    // ASCII files start "solid", but so do some binary ones: the size decides
    if let Some(n) = binary_count.filter(|&n| bytes.len() == 84 + 50 * n) {
        return Ok((0..n)
            .map(|i| {
                let at = 84 + 50 * i + 12;
                let f = |k: usize| {
                    let o = at + 4 * k;
                    f64::from(f32::from_le_bytes(
                        bytes[o..o + 4].try_into().expect("4 bytes"),
                    ))
                };
                [0, 1, 2].map(|v| DVec3::new(f(3 * v), f(3 * v + 1), f(3 * v + 2)))
            })
            .collect());
    }
    let text = std::str::from_utf8(bytes).map_err(|_| "not an STL file".to_string())?;
    if !text.trim_start().starts_with("solid") {
        return Err("not an STL file".into());
    }
    let mut vertices = Vec::new();
    let mut words = text.split_whitespace();
    while let Some(w) = words.next() {
        if w == "vertex" {
            let mut xyz = [0.0; 3];
            for v in &mut xyz {
                *v = words
                    .next()
                    .and_then(|s| s.parse().ok())
                    .ok_or("a vertex in the STL file is not three numbers")?;
            }
            vertices.push(DVec3::from(xyz));
        }
    }
    if vertices.len() % 3 != 0 {
        return Err("the STL file has a broken triangle".into());
    }
    Ok(vertices.as_chunks::<3>().0.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faces_turned_down_above_the_base_are_undercuts() {
        let v = DVec3::new;
        // the base faces down at the bottom: not an undercut
        let base = [v(0.0, 0.0, 0.0), v(0.0, 10.0, 0.0), v(10.0, 0.0, 0.0)];
        let top = [v(0.0, 0.0, 5.0), v(10.0, 0.0, 5.0), v(0.0, 10.0, 5.0)];
        assert!(undercut(&[base, top]).abs() < 1e-12);
        // a ledge's underside 3 mm up is
        let ledge = [v(0.0, 0.0, 3.0), v(0.0, 4.0, 3.0), v(4.0, 0.0, 3.0)];
        assert!((undercut(&[base, top, ledge]) - 8.0).abs() < 1e-12);
    }

    /// A square pyramid 20 mm across and 10 mm high, as four sloping faces and a base.
    fn pyramid() -> Vec<[[f32; 3]; 3]> {
        let (a, b, c, d) = (
            [0.0, 0.0, 0.0],
            [20.0, 0.0, 0.0],
            [20.0, 20.0, 0.0],
            [0.0, 20.0, 0.0],
        );
        let top = [10.0, 10.0, 10.0];
        vec![
            [a, b, top],
            [b, c, top],
            [c, d, top],
            [d, a, top],
            [a, c, b],
            [a, d, c],
        ]
    }

    fn binary(t: &[[[f32; 3]; 3]]) -> Vec<u8> {
        let mut out = b"solid but binary".to_vec();
        out.resize(80, 0);
        out.extend(u32::try_from(t.len()).unwrap().to_le_bytes());
        for tri in t {
            out.extend([0u8; 12]);
            for v in tri {
                for x in v {
                    out.extend(x.to_le_bytes());
                }
            }
            out.extend([0u8; 2]);
        }
        out
    }

    fn ascii(t: &[[[f32; 3]; 3]]) -> Vec<u8> {
        use std::fmt::Write;
        let mut s = String::from("solid pyramid\n");
        for tri in t {
            s += "facet normal 0 0 0\n outer loop\n";
            for v in tri {
                writeln!(s, "  vertex {} {} {}", v[0], v[1], v[2]).unwrap();
            }
            s += " endloop\nendfacet\n";
        }
        s += "endsolid pyramid\n";
        s.into_bytes()
    }

    fn imported(bytes: &[u8]) -> Component {
        let mut doc = Document::default();
        let change = ImportStl::import(&doc, bytes).unwrap();
        doc.apply(&change).unwrap();
        doc.components.values().next().unwrap().clone()
    }

    #[test]
    fn a_pyramid_becomes_a_centred_height_grid_in_both_encodings() {
        for bytes in [binary(&pyramid()), ascii(&pyramid())] {
            let k = imported(&bytes);
            let centre = Document::default().job.area().center();
            let peak = k.height_at(centre).unwrap();
            assert!((peak - 10.0).abs() < 0.2, "{peak}");
            let halfway = k.height_at(centre + Point::new(5.0, 0.0)).unwrap();
            assert!((halfway - 5.0).abs() < 0.2, "{halfway}");
            assert!(
                k.height_at(centre + Point::new(11.0, 0.0)).is_none(),
                "nothing past its edge"
            );
            let e = k.extent();
            assert!((e.size().x - 20.0).abs() < 2.0 * k.cell);
        }
    }

    #[test]
    fn too_big_models_shrink_to_the_job_and_junk_is_refused() {
        let big: Vec<_> = pyramid()
            .into_iter()
            .map(|t| t.map(|v| v.map(|x| x * 100.0)))
            .collect();
        let k = imported(&binary(&big));
        let job = Document::default().job;
        assert!(k.extent().size().y <= job.height + 2.0 * k.cell);
        assert!(ImportStl::import(&Document::default(), b"not a model").is_err());
        assert!(ImportStl::import(&Document::default(), b"solid x\n vertex 1 2\n").is_err());
    }
}
