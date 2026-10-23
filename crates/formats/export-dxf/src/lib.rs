//! Export vectors as DXF (R2000, millimetres): line-and-arc paths as polylines with exact
//! bulges, paths with curves as LINE / ARC / SPLINE entities.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Path, Point, Segment, arc_sweep};
use cut_craft_core::{CommandError, Exporter, FileFormat};
use dxf::entities::{Arc, Entity, EntityType, Line, LwPolyline, Spline};
use dxf::enums::{AcadVersion, Units};
use dxf::{Drawing, LwPolylineVertex};
use std::f64::consts::TAU;

pub struct ExportDxf;

impl Exporter for ExportDxf {
    const ID: &'static str = "export_dxf";
    const FORMAT: FileFormat = FileFormat {
        label: "DXF",
        extensions: &["dxf"],
    };

    fn export(doc: &Document) -> Result<Vec<u8>, CommandError> {
        if doc.vectors.is_empty() {
            return Err(CommandError::Failed(
                "there are no vectors to export".into(),
            ));
        }
        let mut drawing = Drawing::new();
        drawing.header.version = AcadVersion::R2000;
        drawing.header.default_drawing_units = Units::Millimeters;
        for path in doc.vectors.values() {
            if path
                .segments
                .iter()
                .any(|s| matches!(s, Segment::Cubic { .. }))
            {
                for entity in segment_entities(path) {
                    drawing.add_entity(Entity::new(entity));
                }
            } else {
                drawing.add_entity(Entity::new(EntityType::LwPolyline(polyline(path))));
            }
        }
        let mut bytes = vec![];
        drawing
            .save(&mut bytes)
            .map_err(|e| CommandError::Failed(format!("could not write the DXF: {e}")))?;
        Ok(bytes)
    }
}

fn dxf_point(p: Point) -> dxf::Point {
    dxf::Point::new(p.x, p.y, 0.0)
}

fn vertex(p: Point, bulge: f64) -> LwPolylineVertex {
    LwPolylineVertex {
        x: p.x,
        y: p.y,
        bulge,
        ..LwPolylineVertex::default()
    }
}

/// A line-and-arc path as one polyline: each vertex carries the bulge (tan of a quarter of the
/// included angle) of the span that starts there. A full-circle arc is split in two.
fn polyline(path: &Path) -> LwPolyline {
    let mut vertices = vec![];
    let mut from = path.start;
    for segment in &path.segments {
        match *segment {
            Segment::Line { .. } => vertices.push(vertex(from, 0.0)),
            Segment::Arc { to, center, ccw } => {
                let sweep = arc_sweep(from, to, center, ccw);
                if (sweep.abs() - TAU).abs() < 1e-12 {
                    let half = (sweep / 2.0 / 4.0).tan();
                    vertices.push(vertex(from, half));
                    vertices.push(vertex(center * 2.0 - from, half));
                } else {
                    vertices.push(vertex(from, (sweep / 4.0).tan()));
                }
            }
            Segment::Cubic { .. } => {
                unreachable!("paths with curves are written as separate entities")
            }
        }
        from = segment.end();
    }
    if !path.closed {
        vertices.push(vertex(from, 0.0));
    }
    LwPolyline {
        vertices,
        flags: i32::from(path.closed),
        ..LwPolyline::default()
    }
}

/// A path with curves as LINE, ARC and (exact, Bezier-equivalent) SPLINE entities.
fn segment_entities(path: &Path) -> Vec<EntityType> {
    let mut from = path.start;
    let mut entities = vec![];
    for segment in &path.segments {
        entities.push(match *segment {
            Segment::Line { to } => EntityType::Line(Line::new(dxf_point(from), dxf_point(to))),
            Segment::Arc { to, center, ccw } => {
                // DXF arcs always run counter-clockwise
                let (a, b) = if ccw { (from, to) } else { (to, from) };
                let angle = |p: Point| (p - center).y.atan2((p - center).x).to_degrees();
                EntityType::Arc(Arc::new(
                    dxf_point(center),
                    from.distance(center),
                    angle(a),
                    angle(b),
                ))
            }
            Segment::Cubic { c1, c2, to } => EntityType::Spline(Spline {
                degree_of_curve: 3,
                control_points: [from, c1, c2, to].into_iter().map(dxf_point).collect(),
                knot_values: vec![0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0],
                ..Spline::default()
            }),
        });
        from = segment.end();
    }
    entities
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::Importer;
    use cut_craft_core::geometry::{Path, Point, Rect, Segment};
    use cut_craft_core::testing::{document_with, sample_document};
    use dxf::entities::EntityType;

    fn drawing(doc: &Document) -> dxf::Drawing {
        dxf::Drawing::load(&mut std::io::Cursor::new(ExportDxf::export(doc).unwrap())).unwrap()
    }

    fn bounds(doc: &Document) -> Rect {
        doc.vectors
            .values()
            .map(Path::bounds)
            .reduce(Rect::union)
            .unwrap()
    }

    #[test]
    fn round_trips_through_the_importer() {
        let original = sample_document();
        let mut back = Document::default();
        let change =
            fmt_import_dxf::ImportDxf::import(&back, &ExportDxf::export(&original).unwrap())
                .unwrap();
        back.apply(&change).unwrap();
        assert_eq!(back.vectors.len(), 4);
        let (a, b) = (bounds(&back), bounds(&original));
        assert!(
            a.min.distance(b.min) < 1e-9 && a.max.distance(b.max) < 1e-9,
            "{a:?} vs {b:?}"
        );
    }

    #[test]
    fn written_in_millimetres() {
        assert_eq!(
            drawing(&sample_document()).header.default_drawing_units,
            dxf::enums::Units::Millimeters
        );
    }

    #[test]
    fn circles_become_closed_polylines_with_half_circle_bulges() {
        let d = drawing(&document_with(vec![Path::circle(Point::ZERO, 5.0)]));
        let EntityType::LwPolyline(p) = &d.entities().next().unwrap().specific else {
            panic!("expected a polyline")
        };
        assert_eq!(p.flags & 1, 1);
        let bulges: Vec<f64> = p.vertices.iter().map(|v| v.bulge).collect();
        assert!(
            bulges.len() == 2 && bulges.iter().all(|b| (b - 1.0).abs() < 1e-12),
            "{bulges:?}"
        );
    }

    #[test]
    fn curves_are_written_as_splines() {
        let curve = Path {
            start: Point::ZERO,
            segments: vec![Segment::Cubic {
                c1: Point::new(0.0, 10.0),
                c2: Point::new(10.0, 10.0),
                to: Point::new(10.0, 0.0),
            }],
            closed: false,
        };
        let d = drawing(&document_with(vec![curve]));
        assert!(
            d.entities().any(
                |e| matches!(&e.specific, EntityType::Spline(s) if s.control_points.len() == 4)
            )
        );
    }

    #[test]
    fn nothing_to_export_is_an_error() {
        assert_eq!(
            ExportDxf::export(&Document::default())
                .unwrap_err()
                .to_string(),
            "there are no vectors to export"
        );
    }
}
