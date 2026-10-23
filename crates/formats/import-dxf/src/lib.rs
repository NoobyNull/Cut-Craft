//! Import vectors from DXF: lines, arcs, circles, polylines (with bulge arcs), ellipses and
//! splines, scaled from the drawing's units to millimetres. Everything else (text, hatches,
//! dimensions, blocks) is skipped.

use aspire_core::document::Document;
use aspire_core::geometry::{Path, Point, Segment, polar};
use aspire_core::glam::{DAffine2, DVec2};
use aspire_core::{Change, CommandError, FileFormat, Importer};
use dxf::entities::{Ellipse, EntityType, Spline};
use dxf::enums::Units;
use std::f64::consts::TAU;

pub struct ImportDxf;

impl Importer for ImportDxf {
    const ID: &'static str = "import_dxf";
    const FORMAT: FileFormat = FileFormat {
        label: "DXF",
        extensions: &["dxf"],
    };

    fn import(doc: &Document, bytes: &[u8]) -> Result<Change, CommandError> {
        let drawing = dxf::Drawing::load(&mut std::io::Cursor::new(bytes))
            .map_err(|e| CommandError::Failed(format!("not a readable DXF: {e}")))?;
        let to_mm = DAffine2::from_scale(DVec2::splat(mm_per_unit(
            drawing.header.default_drawing_units,
        )));
        let paths: Vec<Path> = drawing
            .entities()
            .filter_map(|e| path(&e.specific))
            .map(|p| p.transformed(&to_mm))
            .collect();
        if paths.is_empty() {
            return Err(CommandError::Failed(
                "the DXF has no lines, arcs, circles, polylines, ellipses or splines".into(),
            ));
        }
        Ok(Change::swap(doc, [], paths))
    }
}

/// Millimetres per drawing unit; unitless drawings are taken to be millimetres.
fn mm_per_unit(units: Units) -> f64 {
    match units {
        Units::Inches | Units::USSurveyInch => 25.4,
        Units::Feet | Units::USSurveyFeet => 304.8,
        Units::Yards | Units::USSurveyYard => 914.4,
        Units::Mils => 0.0254,
        Units::Microinches => 0.000_025_4,
        Units::Centimeters => 10.0,
        Units::Decimeters => 100.0,
        Units::Meters => 1000.0,
        Units::Microns => 0.001,
        Units::Nanometers => 0.000_001,
        _ => 1.0,
    }
}

fn path(entity: &EntityType) -> Option<Path> {
    let xy = |p: &dxf::Point| Point::new(p.x, p.y);
    match entity {
        EntityType::Line(l) => Some(Path::polyline(&[xy(&l.p1), xy(&l.p2)], false)),
        EntityType::Circle(c) => Some(ocs(Path::circle(xy(&c.center), c.radius), c.normal.z)),
        EntityType::Arc(a) => Some(ocs(
            arc(xy(&a.center), a.radius, a.start_angle, a.end_angle),
            a.normal.z,
        )),
        EntityType::LwPolyline(p) => bulged(
            &p.vertices
                .iter()
                .map(|v| (Point::new(v.x, v.y), v.bulge))
                .collect::<Vec<_>>(),
            p.flags & 1 != 0,
        )
        .map(|path| ocs(path, p.extrusion_direction.z)),
        // 3D polylines, meshes and polyface meshes are not vectors
        EntityType::Polyline(p) if p.flags & (8 | 16 | 64) == 0 => bulged(
            &p.vertices()
                .map(|v| (xy(&v.location), v.bulge))
                .collect::<Vec<_>>(),
            p.flags & 1 != 0,
        )
        .map(|path| ocs(path, p.normal.z)),
        EntityType::Ellipse(e) => Some(ellipse(e)),
        EntityType::Spline(s) => spline(s),
        _ => None,
    }
}

/// Arcs, circles and 2D polylines are in object coordinates: a downward extrusion mirrors X.
fn ocs(path: Path, normal_z: f64) -> Path {
    if normal_z < 0.0 {
        path.transformed(&DAffine2::from_scale(DVec2::new(-1.0, 1.0)))
    } else {
        path
    }
}

/// Counter-clockwise arc between two angles in degrees; equal angles make a full circle.
fn arc(center: Point, radius: f64, start: f64, end: f64) -> Path {
    if (end - start).rem_euclid(360.0) == 0.0 {
        return Path::circle(center, radius);
    }
    let segments = vec![Segment::Arc {
        to: polar(center, radius, end),
        center,
        ccw: true,
    }];
    Path {
        start: polar(center, radius, start),
        segments,
        closed: false,
    }
}

/// Polyline vertices with bulges: each span is a line, or an arc whose bulge is the tangent of
/// a quarter of its included angle (positive = counter-clockwise).
fn bulged(vertices: &[(Point, f64)], closed: bool) -> Option<Path> {
    if vertices.len() < 2 {
        return None;
    }
    let n = vertices.len();
    let spans = if closed { n } else { n - 1 };
    let segments = (0..spans)
        .map(|i| {
            let ((from, bulge), to) = (vertices[i], vertices[(i + 1) % n].0);
            if bulge.abs() < 1e-12 {
                return Segment::Line { to };
            }
            let chord = to - from;
            let sagitta = bulge * chord.length() / 2.0;
            let radius = (chord.length_squared() / 4.0 + sagitta * sagitta) / (2.0 * sagitta.abs());
            let center = (from + to) / 2.0
                + chord.perp().normalize() * (radius - sagitta.abs()) * bulge.signum();
            Segment::Arc {
                to,
                center,
                ccw: bulge > 0.0,
            }
        })
        .collect();
    Some(Path {
        start: vertices[0].0,
        segments,
        closed,
    })
}

/// An ellipse is a unit circle (or arc) stretched, rotated and moved into place.
fn ellipse(e: &Ellipse) -> Path {
    let (from, to) = (e.start_parameter, e.end_parameter);
    let unit = if (to - from).rem_euclid(TAU) < 1e-9 {
        Path::circle(Point::ZERO, 1.0)
    } else {
        let segments = vec![Segment::Arc {
            to: DVec2::from_angle(to),
            center: Point::ZERO,
            ccw: true,
        }];
        Path {
            start: DVec2::from_angle(from),
            segments,
            closed: false,
        }
    };
    let major = DVec2::new(e.major_axis.x, e.major_axis.y);
    let a = major.length();
    let place = DAffine2::from_translation(Point::new(e.center.x, e.center.y))
        * DAffine2::from_angle(major.y.atan2(major.x))
        * DAffine2::from_scale(DVec2::new(a, a * e.minor_axis_ratio));
    unit.transformed(&place)
}

/// Samples per knot span when a spline is turned into a polyline.
// ponytail: sampled; convert to exact Beziers by knot insertion if precision matters
const SPLINE_SAMPLES: u32 = 32;

/// A (possibly rational) B-spline, evaluated with de Boor's algorithm and sampled.
fn spline(s: &Spline) -> Option<Path> {
    let p = usize::try_from(s.degree_of_curve)
        .ok()
        .filter(|&p| p >= 1)?;
    let knots = &s.knot_values;
    let weight = |i: usize| s.weight_values.get(i).copied().unwrap_or(1.0);
    // homogeneous control points: (w*x, w*y, w)
    let ctrl: Vec<[f64; 3]> = s
        .control_points
        .iter()
        .enumerate()
        .map(|(i, c)| [c.x * weight(i), c.y * weight(i), weight(i)])
        .collect();
    let last = ctrl.len().checked_sub(1)?;
    if ctrl.len() <= p || knots.len() != ctrl.len() + p + 1 {
        return None;
    }
    let mut points = vec![];
    for k in p..=last {
        let (a, b) = (knots[k], knots[k + 1]);
        if b > a {
            points.extend((0..SPLINE_SAMPLES).map(|i| {
                de_boor(
                    p,
                    k,
                    knots,
                    &ctrl,
                    a + (b - a) * f64::from(i) / f64::from(SPLINE_SAMPLES),
                )
            }));
        }
    }
    points.push(de_boor(p, last, knots, &ctrl, knots[last + 1]));
    Some(Path::polyline(&points, s.flags & 1 != 0))
}

/// The point at `param` on a B-spline of `degree`, where `param` lies in knot span `span`.
fn de_boor(degree: usize, span: usize, knots: &[f64], ctrl: &[[f64; 3]], param: f64) -> Point {
    let mut points: Vec<[f64; 3]> = (0..=degree).map(|j| ctrl[j + span - degree]).collect();
    for r in 1..=degree {
        for j in (r..=degree).rev() {
            let (lo, hi) = (knots[j + span - degree], knots[j + 1 + span - r]);
            let alpha = if hi > lo {
                (param - lo) / (hi - lo)
            } else {
                0.0
            };
            let before = points[j - 1];
            for (value, earlier) in points[j].iter_mut().zip(before) {
                *value = (1.0 - alpha) * earlier + alpha * *value;
            }
        }
    }
    let [x, y, w] = points[degree];
    Point::new(x / w, y / w)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aspire_core::geometry::{Path, Point, Rect, Segment};
    use dxf::entities::{Arc, Circle, Ellipse, Entity, EntityType, Line, LwPolyline, Spline};
    use dxf::enums::{AcadVersion, Units};
    use dxf::{Drawing, LwPolylineVertex, Point as DxfPoint, Vector};

    fn dxf(units: Units, entities: Vec<EntityType>) -> Vec<u8> {
        let mut drawing = Drawing::new();
        drawing.header.version = AcadVersion::R2000;
        drawing.header.default_drawing_units = units;
        for e in entities {
            drawing.add_entity(Entity::new(e));
        }
        let mut bytes = vec![];
        drawing.save(&mut bytes).unwrap();
        bytes
    }

    fn import(bytes: &[u8]) -> Vec<Path> {
        let mut doc = Document::default();
        doc.apply(&ImportDxf::import(&doc, bytes).unwrap()).unwrap();
        assert_eq!(
            doc.selection.len(),
            doc.vectors.len(),
            "imported vectors are selected"
        );
        doc.vectors.into_values().collect()
    }

    fn near(r: Rect, min: (f64, f64), max: (f64, f64), tol: f64) -> bool {
        r.min.distance(Point::new(min.0, min.1)) < tol
            && r.max.distance(Point::new(max.0, max.1)) < tol
    }

    fn vertex(x: f64, y: f64, bulge: f64) -> LwPolylineVertex {
        LwPolylineVertex {
            x,
            y,
            bulge,
            ..LwPolylineVertex::default()
        }
    }

    #[test]
    fn lines_circles_and_arcs() {
        let paths = import(&dxf(
            Units::Millimeters,
            vec![
                EntityType::Line(Line::new(
                    DxfPoint::new(0.0, 0.0, 0.0),
                    DxfPoint::new(10.0, 0.0, 0.0),
                )),
                EntityType::Circle(Circle::new(DxfPoint::new(5.0, 5.0, 0.0), 2.0)),
                EntityType::Arc(Arc::new(DxfPoint::new(0.0, 0.0, 0.0), 10.0, 0.0, 90.0)),
            ],
        ));
        assert_eq!(paths.len(), 3);
        assert!(
            paths
                .iter()
                .any(|p| p.closed && near(p.bounds(), (3.0, 3.0), (7.0, 7.0), 1e-9)),
            "the circle"
        );
        let arc = paths
            .iter()
            .find(|p| matches!(p.segments[..], [Segment::Arc { .. }]) && !p.closed)
            .unwrap();
        assert!(
            arc.start.distance(Point::new(10.0, 0.0)) < 1e-9
                && arc.segments[0].end().distance(Point::new(0.0, 10.0)) < 1e-9
        );
    }

    #[test]
    fn polyline_bulges_become_arcs() {
        // a stadium: straight sides joined by two half-circle ends
        let stadium = LwPolyline {
            vertices: vec![
                vertex(0.0, 0.0, 0.0),
                vertex(10.0, 0.0, 1.0),
                vertex(10.0, 4.0, 0.0),
                vertex(0.0, 4.0, 1.0),
            ],
            flags: 1, // closed
            ..LwPolyline::default()
        };
        let paths = import(&dxf(
            Units::Millimeters,
            vec![EntityType::LwPolyline(stadium)],
        ));
        assert!(paths[0].closed);
        assert!(
            near(paths[0].bounds(), (-2.0, 0.0), (12.0, 4.0), 1e-9),
            "{:?}",
            paths[0].bounds()
        );
    }

    #[test]
    fn inch_drawings_are_scaled_to_millimetres() {
        let line = Line::new(DxfPoint::new(0.0, 0.0, 0.0), DxfPoint::new(1.0, 0.0, 0.0));
        let paths = import(&dxf(Units::Inches, vec![EntityType::Line(line)]));
        assert!((paths[0].bounds().size().x - 25.4).abs() < 1e-9);
    }

    #[test]
    fn ellipses() {
        let e = Ellipse {
            center: DxfPoint::new(0.0, 0.0, 0.0),
            major_axis: Vector::new(10.0, 0.0, 0.0),
            minor_axis_ratio: 0.5,
            start_parameter: 0.0,
            end_parameter: std::f64::consts::TAU,
            ..Ellipse::default()
        };
        let paths = import(&dxf(Units::Millimeters, vec![EntityType::Ellipse(e)]));
        assert!(paths[0].closed);
        assert!(
            near(paths[0].bounds(), (-10.0, -5.0), (10.0, 5.0), 1e-6),
            "{:?}",
            paths[0].bounds()
        );
    }

    #[test]
    fn clamped_splines_run_from_first_to_last_control_point() {
        let s = Spline {
            degree_of_curve: 3,
            control_points: [(0.0, 0.0), (0.0, 10.0), (10.0, 10.0), (10.0, 0.0)]
                .iter()
                .map(|&(x, y)| DxfPoint::new(x, y, 0.0))
                .collect(),
            knot_values: vec![0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0],
            ..Spline::default()
        };
        let paths = import(&dxf(Units::Millimeters, vec![EntityType::Spline(s)]));
        let p = &paths[0];
        assert!(p.start.distance(Point::ZERO) < 1e-9);
        assert!(
            p.segments
                .last()
                .unwrap()
                .end()
                .distance(Point::new(10.0, 0.0))
                < 1e-9
        );
        assert!(
            (p.bounds().max.y - 7.5).abs() < 0.05,
            "a cubic with those controls peaks at 7.5: {:?}",
            p.bounds()
        );
    }

    #[test]
    fn nothing_drawable_is_an_error() {
        let err =
            ImportDxf::import(&Document::default(), &dxf(Units::Millimeters, vec![])).unwrap_err();
        assert_eq!(
            err.to_string(),
            "the DXF has no lines, arcs, circles, polylines, ellipses or splines"
        );
        assert!(ImportDxf::import(&Document::default(), b"not a dxf").is_err());
    }
}
