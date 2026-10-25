//! Export vectors as PDF, EPS or AI (Illustrator reads a PDF saved as `.ai`), one page the
//! size of the material, drawn in points the right way up. Arcs go out as Béziers.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Point, Segment, arc_to_cubics};
use cut_craft_core::{CommandError, Exporter, FileFormat};
use std::fmt::Write as _;

const POINTS_PER_MM: f64 = 72.0 / 25.4;
/// Line width in points.
const LINE: f64 = 0.7;

pub struct ExportPdf;
pub struct ExportEps;
pub struct ExportAi;

impl Exporter for ExportPdf {
    const ID: &'static str = "export_pdf";
    const FORMAT: FileFormat = FileFormat {
        label: "PDF",
        extensions: &["pdf"],
    };

    fn export(doc: &Document) -> Result<Vec<u8>, CommandError> {
        Ok(pdf(doc))
    }
}

impl Exporter for ExportAi {
    const ID: &'static str = "export_ai";
    const FORMAT: FileFormat = FileFormat {
        label: "Adobe Illustrator",
        extensions: &["ai"],
    };

    fn export(doc: &Document) -> Result<Vec<u8>, CommandError> {
        Ok(pdf(doc))
    }
}

impl Exporter for ExportEps {
    const ID: &'static str = "export_eps";
    const FORMAT: FileFormat = FileFormat {
        label: "EPS",
        extensions: &["eps"],
    };

    fn export(doc: &Document) -> Result<Vec<u8>, CommandError> {
        let (w, h) = page(doc);
        let mut out = format!(
            "%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 0 0 {} {}\n%%HiResBoundingBox: 0 0 {w:.3} {h:.3}\n%%Creator: Cut Craft\n%%EndComments\n{LINE} setlinewidth\n",
            w.ceil(),
            h.ceil()
        );
        out += &drawing(doc, ["moveto", "lineto", "curveto", "closepath", "stroke"]);
        out += "showpage\n%%EOF\n";
        Ok(out.into_bytes())
    }
}

/// The page size in points.
fn page(doc: &Document) -> (f64, f64) {
    (
        doc.job.width * POINTS_PER_MM,
        doc.job.height * POINTS_PER_MM,
    )
}

/// Every vector as path operators: `ops` names move, line, curve, close and stroke.
fn drawing(doc: &Document, ops: [&str; 5]) -> String {
    let [mv, ln, cu, close, stroke] = ops;
    let origin = doc.job.area().min;
    let at = |p: Point| (p - origin) * POINTS_PER_MM;
    let mut s = String::new();
    for path in doc.vectors.values() {
        let start = at(path.start);
        let _ = writeln!(s, "{:.3} {:.3} {mv}", start.x, start.y);
        let mut from = path.start;
        for seg in &path.segments {
            let pieces = match *seg {
                Segment::Arc { to, center, ccw } => arc_to_cubics(from, to, center, ccw),
                other => vec![other],
            };
            for piece in pieces {
                match piece {
                    Segment::Line { to } => {
                        let p = at(to);
                        let _ = writeln!(s, "{:.3} {:.3} {ln}", p.x, p.y);
                    }
                    Segment::Cubic { c1, c2, to } => {
                        let (a, b, p) = (at(c1), at(c2), at(to));
                        let _ = writeln!(
                            s,
                            "{:.3} {:.3} {:.3} {:.3} {:.3} {:.3} {cu}",
                            a.x, a.y, b.x, b.y, p.x, p.y
                        );
                    }
                    Segment::Arc { .. } => unreachable!("arcs were turned into cubics"),
                }
            }
            from = seg.end();
        }
        if path.closed {
            s += close;
            s += "\n";
        }
        s += stroke;
        s += "\n";
    }
    s
}

fn pdf(doc: &Document) -> Vec<u8> {
    let (w, h) = page(doc);
    let content = format!("{LINE} w\n{}", drawing(doc, ["m", "l", "c", "h", "S"]));
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {w:.3} {h:.3}] /Contents 4 0 R >>"),
        format!(
            "<< /Length {} >>\nstream\n{content}endstream",
            content.len()
        ),
    ];
    let mut out = String::from("%PDF-1.4\n");
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", i + 1);
    }
    let xref = out.len();
    let _ = write!(out, "xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
    for o in offsets {
        let _ = writeln!(out, "{o:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    );
    out.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Path;
    use cut_craft_core::testing::{document_with, square};

    fn drawing_doc() -> Document {
        document_with(vec![
            square(0.0, 0.0, 25.4),
            Path::circle(Point::new(50.8, 50.8), 25.4),
        ])
    }

    #[test]
    fn pdf_cross_references_point_at_their_objects() {
        let bytes = ExportPdf::export(&drawing_doc()).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        let xref: usize = text
            .lines()
            .skip_while(|l| *l != "startxref")
            .nth(1)
            .unwrap()
            .parse()
            .unwrap();
        assert!(text[xref..].starts_with("xref"));
        for (k, line) in text[xref..].lines().skip(3).take(4).enumerate() {
            let at: usize = line[..10].parse().unwrap();
            assert!(
                text[at..].starts_with(&format!("{} 0 obj", k + 1)),
                "object {}",
                k + 1
            );
        }
        // a 1 inch square is 72 points; the circle is four Béziers
        assert!(text.contains("72.000 0.000 l"));
        assert_eq!(text.matches(" c\n").count(), 4);
        let length: usize = text
            .split("/Length ")
            .nth(1)
            .unwrap()
            .split(' ')
            .next()
            .unwrap()
            .parse()
            .unwrap();
        let begin = text.find(">>\nstream\n").unwrap() + ">>\nstream\n".len();
        assert_eq!(&text[begin + length..begin + length + 9], "endstream");
        assert_eq!(ExportAi::export(&drawing_doc()).unwrap(), text.into_bytes());
    }

    #[test]
    fn eps_has_a_bounding_box_of_the_material() {
        let text = String::from_utf8(ExportEps::export(&drawing_doc()).unwrap()).unwrap();
        assert!(text.starts_with("%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 0 0 1701 1134\n"));
        assert!(text.contains("0.000 0.000 moveto") && text.contains("closepath\nstroke"));
        assert!(text.ends_with("showpage\n%%EOF\n"));
    }
}
