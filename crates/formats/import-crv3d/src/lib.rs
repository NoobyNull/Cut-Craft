//! Import the vectors of a Vectric `.crv3d` job: each contour onto a layer of the job's
//! name (see `re/SPEC.md`, Job files). Text objects, the model and toolpaths are left out.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Path, Point, Segment};
use cut_craft_core::vfont::{arc, compound_streams};
use cut_craft_core::{Change, CommandError, FileFormat, Importer};

pub struct ImportCrv3d;

/// Every record in `VectorData/2dDataV2` sits behind this wrapper: u32 16, the name, u32 1,
/// u32 36, a GUID string, u32 wrapper version; the ones followed by a `vdContour` are vectors.
const CONTOUR_KEY: &[u8] = b"Vectric__Version";

impl Importer for ImportCrv3d {
    const ID: &'static str = "import_crv3d";
    const FORMAT: FileFormat = FileFormat {
        label: "Vectric job (vectors)",
        extensions: &["crv3d", "crv"],
    };

    fn import(doc: &Document, bytes: &[u8]) -> Result<Change, CommandError> {
        let streams = compound_streams(bytes).map_err(CommandError::Failed)?;
        let data = streams
            .iter()
            .find(|(n, _)| n == "2dDataV2")
            .map(|(_, d)| d.as_slice())
            .ok_or_else(|| CommandError::Failed("not a Vectric job: no 2D data".into()))?;
        // ponytail: byte 0x11 of DocumentData was 0 in an inch job and 1 in a mm job [inf]
        let inches = streams
            .iter()
            .find(|(n, _)| n == "DocumentData")
            .is_some_and(|(_, d)| d.get(0x11) == Some(&0));
        let scale = if inches { 25.4 } else { 1.0 };
        let layers = layer_names(data);
        let mut c = Change::builder(doc);
        let mut made = Vec::new();
        let mut ids = Vec::new();
        for (at, path) in contours(data, scale) {
            let name = layers
                .iter()
                .rev()
                .find(|(o, _)| *o < at)
                .map(|(_, n)| n.as_str());
            let id = c.add(path);
            if let Some(name) = name {
                let layer = if let Some((_, l)) = made.iter().find(|(n, _)| n == name) {
                    *l
                } else {
                    let existing = doc.layers.iter().find(|l| l.name == name).map(|l| l.id);
                    let l = existing.unwrap_or_else(|| c.add_layer(name));
                    made.push((name.to_owned(), l));
                    l
                };
                c.move_to_layer(id, layer);
            }
            ids.push(id);
        }
        if ids.is_empty() {
            return Err(CommandError::Failed("the job holds no vectors".into()));
        }
        c.select(ids);
        Ok(c.finish())
    }
}

fn u16_at(b: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(o..o + 2)?.try_into().ok()?))
}

fn u32_at(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(o..o + 4)?.try_into().ok()?))
}

fn f64_at(b: &[u8], o: usize) -> Option<f64> {
    Some(f64::from_le_bytes(b.get(o..o + 8)?.try_into().ok()?))
}

/// (offset, name) of each UTF-16 string: `FF FE FF`, a u8 (or `FF` + u16) length, the text.
fn layer_names(b: &[u8]) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(k) = b[i..].windows(3).position(|w| w == [0xFF, 0xFE, 0xFF]) {
        let at = i + k;
        let mut o = at + 3;
        let mut n = usize::from(*b.get(o).unwrap_or(&0));
        o += 1;
        if n == 0xFF {
            n = usize::from(u16_at(b, o).unwrap_or(0));
            o += 2;
        }
        if (1..64).contains(&n)
            && let Some(units) = b.get(o..o + 2 * n)
        {
            let units: Vec<u16> = units
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_le_bytes(*c))
                .collect();
            if let Ok(s) = String::from_utf16(&units)
                && s.chars().all(|c| !c.is_control())
            {
                out.push((at, s));
            }
        }
        i = at + 3;
    }
    out
}

/// (offset, path) of each contour, scaled into millimetres.
fn contours(b: &[u8], scale: f64) -> Vec<(usize, Path)> {
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(k) = b[i..]
        .windows(CONTOUR_KEY.len())
        .position(|w| w == CONTOUR_KEY)
    {
        let at = i + k;
        i = at + CONTOUR_KEY.len();
        if let Some(path) = contour_at(b, i, scale) {
            out.push((at, path));
        }
    }
    out
}

fn contour_at(b: &[u8], mut o: usize, scale: f64) -> Option<Path> {
    if u32_at(b, o + 4)? != 36 {
        return None;
    }
    // a nil record GUID marks a component's 2D preview outline, not a drawn vector
    if b.get(o + 8..o + 8 + 36)? == b"00000000-0000-0000-0000-000000000000" {
        return None;
    }
    o += 8 + 36;
    if u32_at(b, o)? != 2 {
        return None;
    }
    o += 4;
    let tag = u16_at(b, o)?;
    o += 2;
    if tag == 0xFFFF {
        o += 2;
        o += 2 + usize::from(u16_at(b, o)?);
    }
    if u32_at(b, o)? != 7 {
        return None;
    }
    o += 4 + 4 + 8 + 8 + 2;
    let n = u32_at(b, o)?;
    o += 4;
    let pt = |o: usize| Some(Point::new(f64_at(b, o)?, f64_at(b, o + 8)?) * scale);
    let mut start = None;
    let mut segments = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let kind = *b.get(o)?;
        o += 1 + 8;
        let (from, to) = (pt(o)?, pt(o + 24)?);
        o += 48 + 4;
        start.get_or_insert(from);
        segments.push(match kind {
            1 => {
                let bulge = f64_at(b, o)?;
                o += 8;
                arc(from, to, bulge)
            }
            2 => {
                let (c1, c2) = (pt(o)?, pt(o + 16)?);
                o += 32;
                Segment::Cubic { c1, c2, to }
            }
            _ => Segment::Line { to },
        });
    }
    let start = start?;
    let end = segments.last().map_or(start, Segment::end);
    let closed = !segments.is_empty() && end.distance(start) < 1e-9 * (1.0 + start.length());
    Some(Path {
        start,
        segments,
        closed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reads the sample saved from Aspire when it is there (it is gitignored).
    #[test]
    fn the_shapes_sample_imports_onto_its_layers() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../re/extracted/crv3d/shapes.crv3d"
        );
        let Ok(bytes) = std::fs::read(path) else {
            eprintln!("no sample at {path}; skipped");
            return;
        };
        let mut doc = Document::default();
        let change = ImportCrv3d::import(&doc, &bytes).unwrap();
        doc.apply(&change).unwrap();
        assert_eq!(doc.vectors.len(), 4);
        let names: Vec<&str> = doc.layers.iter().map(|l| l.name.as_str()).collect();
        assert!(
            names.contains(&"Square") && names.contains(&"Circle") && names.contains(&"Shape"),
            "{names:?}"
        );
        let circle = doc
            .vectors
            .values()
            .find(|p| matches!(p.segments[0], Segment::Arc { .. }))
            .unwrap();
        let b = circle.bounds();
        assert!((b.size().x - 8.0 * 25.4).abs() < 1e-6, "radius 4 in: {b:?}");
        assert!(circle.closed);
    }

    #[test]
    fn the_snowman_sample_imports_its_layers_in_millimetres() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../re/extracted/crv3d/shapes2.crv3d"
        );
        let Ok(bytes) = std::fs::read(path) else {
            return;
        };
        let mut doc = Document::default();
        let change = ImportCrv3d::import(&doc, &bytes).unwrap();
        doc.apply(&change).unwrap();
        assert_eq!(
            doc.vectors.len(),
            3,
            "component preview outlines are not vectors"
        );
        let names: Vec<&str> = doc.layers.iter().map(|l| l.name.as_str()).collect();
        assert!(
            names.contains(&"Bounding Box") && names.contains(&"2Rail Sweep Rails"),
            "{names:?}"
        );
        let b = doc
            .vectors
            .values()
            .find(|p| p.closed && p.bounds().size().x > 290.0)
            .unwrap()
            .bounds();
        assert!((b.size().x - 300.0).abs() < 1e-6, "a 300 mm job box: {b:?}");
    }

    #[test]
    fn junk_is_refused() {
        assert!(ImportCrv3d::import(&Document::default(), b"nope").is_err());
    }
}
