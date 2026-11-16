//! Import vectors from SVG: every subpath becomes a vector, in millimetres, y up. An SVG
//! sized in pixels (or not sized) is read at 96 pixels per inch, and a note says the size it
//! came in at, since programs that use 72 per inch make it come in 25% small.

use cut_craft_core::document::{Document, Units};
use cut_craft_core::geometry::{Path, Point, Segment};
use cut_craft_core::{Change, CommandError, FileFormat, Importer};
use usvg::tiny_skia_path::{self as skia, PathSegment};
use usvg::{NodeKind, TreeParsing};

/// usvg resolves every unit to CSS pixels at 96 per inch.
const MM_PER_PX: f64 = 25.4 / 96.0;

pub struct ImportSvg;

impl Importer for ImportSvg {
    const ID: &'static str = "import_svg";
    const FORMAT: FileFormat = FileFormat {
        label: "SVG",
        extensions: &["svg"],
    };

    fn import(doc: &Document, bytes: &[u8]) -> Result<Change, CommandError> {
        let mut tree = usvg::Tree::from_data(bytes, &usvg::Options::default())
            .map_err(|e| CommandError::Failed(format!("not a readable SVG: {e}")))?;
        tree.calculate_abs_transforms();
        let to_px =
            usvg::utils::view_box_to_transform(tree.view_box.rect, tree.view_box.aspect, tree.size);
        let height_mm = f64::from(tree.size.height()) * MM_PER_PX;

        let mut c = Change::builder(doc);
        let mut ids = vec![];
        for node in tree.root.descendants() {
            let NodeKind::Path(ref path) = *node.borrow() else {
                continue;
            };
            // a path's points are in its parent group's space
            let group = node.parent().and_then(|g| match *g.borrow() {
                NodeKind::Group(ref g) => Some(g.abs_transform),
                _ => None,
            });
            let t = to_px.pre_concat(group.unwrap_or_default());
            let to_mm = |p: skia::Point| {
                let mut q = p;
                t.map_point(&mut q);
                Point::new(
                    f64::from(q.x) * MM_PER_PX,
                    height_mm - f64::from(q.y) * MM_PER_PX,
                )
            };
            ids.extend(subpaths(&path.data, to_mm).into_iter().map(|p| c.add(p)));
        }
        if ids.is_empty() {
            return Err(CommandError::Failed(
                "the SVG has no vector paths. Export it again with its shapes as paths".into(),
            ));
        }
        if !real_size(bytes) {
            let (w, h) = (
                f64::from(tree.size.width()) * MM_PER_PX,
                f64::from(tree.size.height()) * MM_PER_PX,
            );
            let size = match doc.job.units {
                Units::Mm => format!("{w:.1} x {h:.1} mm"),
                Units::Inch => format!("{:.2} x {:.2} in", w / 25.4, h / 25.4),
            };
            c.note(format!(
                "The SVG gives its size in pixels, so it was read at 96 pixels per inch and came \
                 in at {size}. If it came from a program that uses 72 per inch, it is 25% too \
                 small: put it right with Set Size."
            ));
        }
        c.select(ids);
        Ok(c.finish())
    }
}

/// Whether the SVG's own `width` and `height` are in real-world units (mm, cm, in, pt, pc),
/// so its size does not depend on how many pixels make an inch.
fn real_size(bytes: &[u8]) -> bool {
    let text = String::from_utf8_lossy(bytes);
    let Some(start) = text.find("<svg") else {
        return false;
    };
    let tag = &text[start..];
    let tag = &tag[..tag.find('>').unwrap_or(tag.len())];
    let real = |name: &str| {
        attribute(tag, name).is_some_and(|v| {
            let v = v.trim().to_ascii_lowercase();
            ["mm", "cm", "in", "pt", "pc"]
                .iter()
                .any(|u| v.ends_with(u))
        })
    };
    real("width") && real("height")
}

/// The value of attribute `name` in an element's start `tag`.
fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let mut from = 0;
    while let Some(at) = tag[from..].find(name).map(|i| i + from) {
        from = at + name.len();
        let before = tag[..at].chars().next_back();
        if !before.is_some_and(char::is_whitespace) {
            continue;
        }
        let rest = tag[from..].trim_start();
        let Some(rest) = rest.strip_prefix('=') else {
            continue;
        };
        let rest = rest.trim_start();
        let quote = rest.chars().next().filter(|q| *q == '"' || *q == '\'')?;
        let value = &rest[1..];
        return value.find(quote).map(|end| &value[..end]);
    }
    None
}

/// Splits a path at each move-to; quadratics become cubics (exactly).
fn subpaths(data: &skia::Path, to_mm: impl Fn(skia::Point) -> Point) -> Vec<Path> {
    // ponytail: a drawing segment straight after Close (no MoveTo) continues the closed path;
    // usvg always emits a MoveTo there, so this has not been needed
    let mut done = vec![];
    let mut current: Option<Path> = None;
    let flush = |current: &mut Option<Path>, done: &mut Vec<Path>| {
        if let Some(p) = current.take().filter(|p| !p.segments.is_empty()) {
            done.push(p.closed_where_ends_meet());
        }
    };
    for segment in data.segments() {
        if let PathSegment::MoveTo(p) = segment {
            flush(&mut current, &mut done);
            current = Some(Path {
                start: to_mm(p),
                segments: vec![],
                closed: false,
            });
            continue;
        }
        let Some(path) = current.as_mut() else {
            continue;
        };
        let from = path.segments.last().map_or(path.start, Segment::end);
        match segment {
            PathSegment::LineTo(p) => path.segments.push(Segment::Line { to: to_mm(p) }),
            PathSegment::QuadTo(c, p) => {
                let (c, to) = (to_mm(c), to_mm(p));
                path.segments.push(Segment::Cubic {
                    c1: from + (c - from) * (2.0 / 3.0),
                    c2: to + (c - to) * (2.0 / 3.0),
                    to,
                });
            }
            PathSegment::CubicTo(c1, c2, p) => path.segments.push(Segment::Cubic {
                c1: to_mm(c1),
                c2: to_mm(c2),
                to: to_mm(p),
            }),
            PathSegment::Close => {
                if from.distance(path.start) > 0.0 {
                    path.segments.push(Segment::Line { to: path.start });
                }
                path.closed = true;
            }
            PathSegment::MoveTo(_) => unreachable!("handled above"),
        }
    }
    flush(&mut current, &mut done);
    done
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::Exporter;
    use cut_craft_core::geometry::{Point, Rect};
    use cut_craft_core::testing::sample_document;

    fn import(svg: &str) -> Document {
        let mut doc = Document::default();
        let change = ImportSvg::import(&doc, svg.as_bytes()).unwrap();
        doc.apply(&change).unwrap();
        doc
    }

    fn all_bounds(doc: &Document) -> Rect {
        doc.vectors
            .values()
            .map(cut_craft_core::geometry::Path::bounds)
            .reduce(Rect::union)
            .unwrap()
    }

    #[test]
    fn our_own_export_comes_back_the_same_shape_and_place() {
        let original = sample_document();
        let svg = String::from_utf8(fmt_export_svg::ExportSvg::export(&original).unwrap()).unwrap();
        let doc = import(&svg);
        assert_eq!(doc.vectors.len(), 4);
        let (a, b) = (all_bounds(&doc), original.selection_bounds().unwrap());
        assert!(
            a.min.distance(b.min) < 1e-3 && a.max.distance(b.max) < 1e-3,
            "{a:?} vs {b:?}"
        );
    }

    #[test]
    fn pixel_units_are_converted_to_millimetres_with_y_up() {
        // 96 px = 1 inch; the rect is 30 x 40 px, 20 px below the top of a 100 px tall canvas
        let doc = import(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><rect x="10" y="20" width="30" height="40"/></svg>"#,
        );
        let b = all_bounds(&doc);
        let mm = 25.4 / 96.0;
        let want = Rect::new(
            Point::new(10.0 * mm, 40.0 * mm),
            Point::new(40.0 * mm, 80.0 * mm),
        );
        assert!(
            b.min.distance(want.min) < 1e-3 && b.max.distance(want.max) < 1e-3,
            "{b:?} vs {want:?}"
        );
    }

    #[test]
    fn a_size_in_pixels_or_none_says_what_it_was_read_at() {
        let notes = |svg: &str| {
            ImportSvg::import(&Document::default(), svg.as_bytes())
                .unwrap()
                .notes
                .join(" ")
        };
        let rect = r#"<rect x="0" y="0" width="72" height="36"/>"#;
        for size in [
            r#"width="288" height="144""#,
            r#"width="288px" height="144px""#,
            "",
        ] {
            let told = notes(&format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" {size} viewBox="0 0 288 144">{rect}</svg>"#
            ));
            assert!(
                told.contains("96 pixels per inch") && told.contains("Set Size"),
                "{size}: {told}"
            );
            assert!(told.contains("72"), "names the other common size: {told}");
        }
        // a size in real units needs no note
        let told = notes(&format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="50mm" viewBox="0 0 100 50">{rect}</svg>"#
        ));
        assert!(told.is_empty(), "{told}");
    }

    #[test]
    fn each_subpath_becomes_its_own_vector() {
        let doc = import(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><path d="M0 0 L10 0 M20 0 L30 0" stroke="black"/></svg>"#,
        );
        assert_eq!(doc.vectors.len(), 2);
    }

    #[test]
    fn imported_vectors_are_selected() {
        let doc = import(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><circle cx="5" cy="5" r="4"/></svg>"#,
        );
        assert_eq!(doc.selection.len(), 1);
        assert!(doc.vectors.values().next().unwrap().closed);
    }

    #[test]
    fn a_path_that_ends_where_it_starts_is_closed_without_a_z() {
        // as Vectric exports them: back to the first point with L, no Z
        let doc = import(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><path d="M 10,10 L 40,10 L 40,40 L 10,10" stroke="black"/><path d="M 50,10 L 80,10 L 80,40" stroke="black"/></svg>"#,
        );
        let mut closed: Vec<bool> = doc.vectors.values().map(|p| p.closed).collect();
        closed.sort_unstable();
        assert_eq!(
            closed,
            [false, true],
            "the triangle closed, the open line not"
        );
    }

    #[test]
    fn garbage_is_an_error_not_a_panic() {
        assert!(ImportSvg::import(&Document::default(), b"definitely not svg").is_err());
        let empty = r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"/>"#;
        assert_eq!(
            ImportSvg::import(&Document::default(), empty.as_bytes())
                .unwrap_err()
                .to_string(),
            "The SVG has no vector paths. Export it again with its shapes as paths."
        );
    }
}
