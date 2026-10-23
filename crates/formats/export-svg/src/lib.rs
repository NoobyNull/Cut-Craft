//! Export vectors as SVG, in millimetres, the right way up.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Path, Segment, arc_sweep};
use cut_craft_core::{CommandError, Exporter, FileFormat};
use std::f64::consts::{PI, TAU};
use std::fmt::Write as _;

pub struct ExportSvg;

impl Exporter for ExportSvg {
    const ID: &'static str = "export_svg";
    const FORMAT: FileFormat = FileFormat {
        label: "SVG",
        extensions: &["svg"],
    };

    fn export(doc: &Document) -> Result<Vec<u8>, CommandError> {
        let (w, h) = (doc.job.width, doc.job.height);
        // Coordinates stay in job millimetres; the group flips y so the drawing reads upright.
        let mut svg = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}mm\" height=\"{h}mm\" viewBox=\"0 0 {w} {h}\">\n\
             <g transform=\"matrix(1 0 0 -1 0 {h})\" fill=\"none\" stroke=\"black\" stroke-width=\"0.25\">\n"
        );
        for path in doc.vectors.values() {
            let _ = writeln!(svg, "<path d=\"{}\"/>", path_data(path));
        }
        svg.push_str("</g>\n</svg>\n");
        Ok(svg.into_bytes())
    }
}

/// SVG path data. In the flipped group, SVG's positive sweep is our counter-clockwise.
fn path_data(path: &Path) -> String {
    let mut d = format!("M {} {}", path.start.x, path.start.y);
    let mut from = path.start;
    for segment in &path.segments {
        let _ = match *segment {
            Segment::Line { to } => write!(d, " L {} {}", to.x, to.y),
            Segment::Cubic { c1, c2, to } => write!(
                d,
                " C {} {} {} {} {} {}",
                c1.x, c1.y, c2.x, c2.y, to.x, to.y
            ),
            Segment::Arc { to, center, ccw } => {
                let r = from.distance(center);
                let sweep = arc_sweep(from, to, center, ccw);
                let flag = u8::from(ccw);
                if (sweep.abs() - TAU).abs() < 1e-12 {
                    // SVG cannot draw a full circle as one arc: go via the opposite point
                    let opposite = center * 2.0 - from;
                    write!(
                        d,
                        " A {r} {r} 0 0 {flag} {} {} A {r} {r} 0 0 {flag} {} {}",
                        opposite.x, opposite.y, to.x, to.y
                    )
                } else {
                    write!(
                        d,
                        " A {r} {r} 0 {} {flag} {} {}",
                        u8::from(sweep.abs() > PI),
                        to.x,
                        to.y
                    )
                }
            }
        };
        from = segment.end();
    }
    if path.closed {
        d.push_str(" Z");
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::sample_document;

    fn svg() -> String {
        String::from_utf8(ExportSvg::export(&sample_document()).unwrap()).unwrap()
    }

    #[test]
    fn declares_the_job_size_in_millimetres() {
        assert!(svg().contains(r#"width="600mm" height="400mm" viewBox="0 0 600 400""#));
    }

    #[test]
    fn flips_y_so_the_job_reads_upright() {
        assert!(svg().contains(r#"transform="matrix(1 0 0 -1 0 400)""#));
    }

    #[test]
    fn one_path_element_per_vector() {
        assert_eq!(svg().matches("<path ").count(), 4);
    }

    #[test]
    fn arcs_keep_their_radius_and_direction() {
        assert!(svg().contains("A 25 25 0 0 1 175 100"), "{}", svg());
    }

    #[test]
    fn closed_paths_end_with_z_and_open_ones_do_not() {
        let s = svg();
        assert_eq!(
            s.matches(" Z\"").count(),
            3,
            "two squares and the circle are closed"
        );
    }
}
