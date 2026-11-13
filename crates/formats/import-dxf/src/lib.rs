//! Import vectors from DXF: lines, arcs, circles, polylines (with bulge arcs), ellipses and
//! splines, scaled from the drawing's units to millimetres. Everything else (text, hatches,
//! dimensions, blocks) is skipped.

use cut_craft_core::document::{Document, Job, Units as JobUnits};
use cut_craft_core::geometry::{Path, Point, Rect, Segment, polar};
use cut_craft_core::glam::{DAffine2, DVec2};
use cut_craft_core::{Change, CommandError, FileFormat, Importer, UnitGuess};
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
        let mut paths = vec![];
        let mut left_out = LeftOut::default();
        for e in drawing.entities() {
            match path(&e.specific) {
                Some(p) => paths.push(p),
                None => left_out.add(kind(&e.specific)),
            }
        }
        if paths.is_empty() {
            return Err(CommandError::Failed(
                "the DXF has no lines, arcs, circles, polylines, ellipses or splines. Export it again from the program that made it".into(),
            ));
        }
        // what the DXF reader passes over without a word (hatches...), from the file itself
        for name in entity_names(bytes) {
            left_out.add(plain(&name));
        }
        let size = paths
            .iter()
            .map(Path::bounds)
            .reduce(Rect::union)
            .map_or(DVec2::ZERO, Rect::size);
        let (scale, unit_guess) = scale(drawing.header.default_drawing_units, size, &doc.job);
        let to_mm = DAffine2::from_scale(DVec2::splat(scale));
        let paths = paths
            .into_iter()
            .map(|p| p.transformed(&to_mm).closed_where_ends_meet())
            .collect();
        let mut change = Change::swap(doc, [], paths);
        change.unit_guess = unit_guess;
        change.notes.extend(left_out.note());
        Ok(change)
    }
}

/// Millimetres per drawing unit, and the guess made if the DXF does not say its units (an
/// R12 file never does): the job's units, unless the drawing would then be too small to see
/// or too big for the material and the other units would fit, so a 6 inch drawing does not
/// come into a millimetre job as a 6 mm speck (re/SPEC.md, Import). The person is asked
/// which units it is in, the guess picked.
fn scale(units: Units, size: DVec2, job: &Job) -> (f64, Option<UnitGuess>) {
    if units != Units::Unitless {
        return (mm_per_unit(units), None);
    }
    let theirs = job.units;
    let other = other_units(theirs);
    let taken = match (fit(size, theirs, job), fit(size, other, job)) {
        (Fit::TooSmall | Fit::TooBig, Fit::Fits) => other,
        _ => theirs,
    };
    (taken.per(), Some(UnitGuess { taken, size }))
}

/// Inches for millimetres and millimetres for inches.
pub fn other_units(units: JobUnits) -> JobUnits {
    match units {
        JobUnits::Mm => JobUnits::Inch,
        JobUnits::Inch => JobUnits::Mm,
    }
}

/// How a drawing `size` units across sits on the job's material if its units are `units`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fit {
    Fits,
    /// Its longest side under a fiftieth of the material's shorter side.
    TooSmall,
    TooBig,
}

pub fn fit(size: DVec2, units: JobUnits, job: &Job) -> Fit {
    let (long, short) = (
        size.max_element() * units.per(),
        size.min_element() * units.per(),
    );
    let (wide, narrow) = (job.width.max(job.height), job.width.min(job.height));
    if long < narrow / 50.0 {
        Fit::TooSmall
    } else if long > wide + 1e-9 || short > narrow + 1e-9 {
        Fit::TooBig
    } else {
        Fit::Fits
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

/// What was not brought in, by its plain name, counted.
#[derive(Default)]
struct LeftOut(std::collections::BTreeMap<&'static str, usize>);

impl LeftOut {
    fn add(&mut self, kind: &'static str) {
        *self.0.entry(kind).or_default() += 1;
    }

    /// "Left out 1 hatch fill and 2 texts: ...", or nothing if all came in.
    fn note(&self) -> Option<String> {
        let mut items: Vec<String> = self
            .0
            .iter()
            .map(|(kind, &n)| {
                if n == 1 {
                    format!("1 {kind}")
                } else {
                    format!("{n} {kind}s")
                }
            })
            .collect();
        let final_item = items.pop()?;
        let list = if items.is_empty() {
            final_item
        } else {
            format!("{} and {final_item}", items.join(", "))
        };
        let text = if self.0.contains_key("text") {
            " To bring text in, turn it into curves in the program that made the DXF."
        } else {
            ""
        };
        Some(format!(
            "Left out {list}: only the lines and curves of a DXF come in.{text}"
        ))
    }
}

/// A plain name for an entity the importer does not draw.
fn kind(entity: &EntityType) -> &'static str {
    match entity {
        EntityType::Text(_)
        | EntityType::MText(_)
        | EntityType::RText(_)
        | EntityType::Attribute(_)
        | EntityType::AttributeDefinition(_) => "text",
        EntityType::Insert(_) => "block",
        EntityType::Polyline(_) => "3D polyline or mesh",
        EntityType::Face3D(_) => "3D face",
        EntityType::Solid(_) | EntityType::Trace(_) => "solid fill",
        EntityType::ModelPoint(_) => "point",
        EntityType::Image(_) => "picture",
        EntityType::RotatedDimension(_)
        | EntityType::RadialDimension(_)
        | EntityType::DiameterDimension(_)
        | EntityType::AngularThreePointDimension(_)
        | EntityType::OrdinateDimension(_) => "dimension",
        EntityType::Leader(_) => "leader",
        _ => "other item",
    }
}

/// A plain name for an entity the DXF reader does not know at all.
fn plain(name: &str) -> &'static str {
    match name {
        "HATCH" => "hatch fill",
        "MPOLYGON" => "filled shape",
        "MLEADER" | "MULTILEADER" => "leader",
        "ACAD_TABLE" => "table",
        _ => "other item",
    }
}

/// The names of the entities in a text DXF's ENTITIES section that the DXF reader skips
/// without a word: the ones it has no type for (HATCH...). Nothing for a binary DXF.
fn entity_names(bytes: &[u8]) -> Vec<String> {
    let mut lines = bytes.split(|&b| b == b'\n').map(<[u8]>::trim_ascii);
    let mut names = vec![];
    let mut in_entities = false;
    while let (Some(code), Some(value)) = (lines.next(), lines.next()) {
        match (code, value) {
            (b"2", b"ENTITIES") => in_entities = true,
            (b"0", b"ENDSEC") if in_entities => break,
            (b"0", name) if in_entities && !READ.contains(&name) => {
                names.push(String::from_utf8_lossy(name).into_owned());
            }
            _ => {}
        }
    }
    names
}

/// Entities the DXF reader has a type for: [`kind`] names those it brings but are not drawn.
const READ: [&[u8]; 46] = [
    b"3DFACE",
    b"3DSOLID",
    b"ACAD_PROXY_ENTITY",
    b"ARC",
    b"ARCALIGNEDTEXT",
    b"ATTDEF",
    b"ATTRIB",
    b"BODY",
    b"CIRCLE",
    b"DIMENSION",
    b"ELLIPSE",
    b"HELIX",
    b"IMAGE",
    b"INSERT",
    b"LEADER",
    b"LIGHT",
    b"LINE",
    b"3DLINE",
    b"LWPOLYLINE",
    b"MLINE",
    b"MTEXT",
    b"OLEFRAME",
    b"OLE2FRAME",
    b"POINT",
    b"POLYLINE",
    b"RAY",
    b"REGION",
    b"RTEXT",
    b"SECTION",
    b"SEQEND",
    b"SHAPE",
    b"SOLID",
    b"SPLINE",
    b"TEXT",
    b"TOLERANCE",
    b"TRACE",
    b"DGNUNDERLAY",
    b"DWFUNDERLAY",
    b"PDFUNDERLAY",
    b"VERTEX",
    b"WIPEOUT",
    b"XLINE",
    b"ENDSEC",
    b"EOF",
    b"ENDBLK",
    b"BLOCK",
];

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

/// A (possibly rational) B-spline, evaluated with de Boor's algorithm and sampled at the
/// points per knot span Options ▸ Advanced sets (32 unless changed).
// ponytail: sampled; convert to exact Beziers by knot insertion if precision matters
fn spline(s: &Spline) -> Option<Path> {
    spline_sampled(s, cut_craft_core::import_settings::spline_samples())
}

/// [`spline`] with `samples` points per knot span.
fn spline_sampled(s: &Spline, samples: u32) -> Option<Path> {
    let samples = cut_craft_core::import_settings::samples_in_range(samples);
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
            points.extend((0..samples).map(|i| {
                de_boor(
                    p,
                    k,
                    knots,
                    &ctrl,
                    a + (b - a) * f64::from(i) / f64::from(samples),
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
    use cut_craft_core::document::Units as JobUnits;
    use cut_craft_core::geometry::{Path, Point, Rect, Segment};
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
    fn a_polyline_that_ends_where_it_starts_is_closed_without_the_flag() {
        let around = LwPolyline {
            vertices: vec![
                vertex(0.0, 0.0, 0.0),
                vertex(10.0, 0.0, 0.0),
                vertex(10.0, 10.0, 0.0),
                vertex(0.0, 0.0, 0.0),
            ],
            ..LwPolyline::default()
        };
        let paths = import(&dxf(
            Units::Millimeters,
            vec![EntityType::LwPolyline(around)],
        ));
        assert!(paths[0].closed);
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
    fn a_spline_span_becomes_as_many_lines_as_samples_asked_for() {
        let s = Spline {
            degree_of_curve: 3,
            control_points: [(0.0, 0.0), (0.0, 10.0), (10.0, 10.0), (10.0, 0.0)]
                .iter()
                .map(|&(x, y)| DxfPoint::new(x, y, 0.0))
                .collect(),
            knot_values: vec![0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0],
            ..Spline::default()
        };
        // one knot span: the samples along it, then the last point
        assert_eq!(spline_sampled(&s, 8).unwrap().segments.len(), 8);
        assert_eq!(spline_sampled(&s, 64).unwrap().segments.len(), 64);
        // kept to Options' range
        assert_eq!(spline_sampled(&s, 1).unwrap().segments.len(), 4);
    }

    /// A drawing in `version` that says nothing about its units, as R12 files never do.
    fn unitless(version: AcadVersion, entities: Vec<EntityType>) -> Vec<u8> {
        let mut drawing = Drawing::new();
        drawing.header.version = version;
        drawing.header.default_drawing_units = Units::Unitless;
        for e in entities {
            drawing.add_entity(Entity::new(e));
        }
        let mut bytes = vec![];
        drawing.save(&mut bytes).unwrap();
        bytes
    }

    /// A `size` square from lines, starting at the origin.
    fn square(size: f64) -> Vec<EntityType> {
        let c = [
            (0.0, 0.0),
            (size, 0.0),
            (size, size),
            (0.0, size),
            (0.0, 0.0),
        ];
        c.windows(2)
            .map(|w| {
                EntityType::Line(Line::new(
                    DxfPoint::new(w[0].0, w[0].1, 0.0),
                    DxfPoint::new(w[1].0, w[1].1, 0.0),
                ))
            })
            .collect()
    }

    fn job(units: JobUnits, width: f64, height: f64) -> Document {
        let mut doc = Document::default();
        doc.job.units = units;
        doc.job.width = width;
        doc.job.height = height;
        doc
    }

    /// The size of everything the import brings into `doc`, its notes, and the units it
    /// guessed the drawing is in if the file doesn't say.
    fn imported(doc: &Document, bytes: &[u8]) -> (DVec2, Vec<String>, Option<UnitGuess>) {
        let change = ImportDxf::import(doc, bytes).unwrap();
        let (notes, guess) = (change.notes.clone(), change.unit_guess);
        let mut doc = doc.clone();
        doc.apply(&change).unwrap();
        let bounds = doc
            .vectors
            .values()
            .map(Path::bounds)
            .reduce(Rect::union)
            .unwrap();
        (bounds.size(), notes, guess)
    }

    fn guessed(taken: JobUnits, size: f64) -> UnitGuess {
        UnitGuess {
            taken,
            size: DVec2::splat(size),
        }
    }

    #[test]
    fn a_drawing_without_units_is_read_in_the_job_s_units_and_the_guess_kept_to_ask() {
        let bytes = unitless(AcadVersion::R12, square(6.0));
        let (size, notes, guess) = imported(&job(JobUnits::Inch, 610.0, 610.0), &bytes);
        assert!((size.x - 152.4).abs() < 1e-9, "{size:?}");
        assert_eq!(guess, Some(guessed(JobUnits::Inch, 6.0)));
        assert!(notes.is_empty(), "the person is asked instead: {notes:?}");
        let bytes = unitless(AcadVersion::R12, square(100.0));
        let (size, _, guess) = imported(&job(JobUnits::Mm, 600.0, 400.0), &bytes);
        assert!((size.x - 100.0).abs() < 1e-9, "{size:?}");
        assert_eq!(guess, Some(guessed(JobUnits::Mm, 100.0)));
    }

    #[test]
    fn a_drawing_without_units_too_small_to_see_in_millimetres_is_read_in_inches() {
        // the 6 inch Aztec calendar saved as an R12 DXF, brought into a millimetre job:
        // 6 mm would be a speck on the material
        let bytes = unitless(AcadVersion::R12, square(6.0));
        let (size, _, guess) = imported(&job(JobUnits::Mm, 600.0, 400.0), &bytes);
        assert!((size.x - 152.4).abs() < 1e-9, "{size:?}");
        assert_eq!(guess, Some(guessed(JobUnits::Inch, 6.0)));
        // a small part that is still a sensible size in millimetres stays in millimetres
        let bytes = unitless(AcadVersion::R12, square(20.0));
        let (size, _, _) = imported(&job(JobUnits::Mm, 600.0, 400.0), &bytes);
        assert!((size.x - 20.0).abs() < 1e-9, "{size:?}");
    }

    #[test]
    fn a_drawing_without_units_too_big_for_the_material_in_inches_is_read_in_millimetres() {
        let bytes = unitless(AcadVersion::R12, square(300.0));
        let (size, _, guess) = imported(&job(JobUnits::Inch, 610.0, 610.0), &bytes);
        assert!((size.x - 300.0).abs() < 1e-9, "{size:?}");
        assert_eq!(guess, Some(guessed(JobUnits::Mm, 300.0)));
        // too big either way: the job's units
        let bytes = unitless(AcadVersion::R12, square(3000.0));
        let (size, _, _) = imported(&job(JobUnits::Inch, 610.0, 610.0), &bytes);
        assert!((size.x - 3000.0 * 25.4).abs() < 1e-6, "{size:?}");
    }

    #[test]
    fn how_a_size_fits_the_material() {
        let doc = job(JobUnits::Mm, 600.0, 400.0);
        let six = DVec2::splat(6.0);
        assert_eq!(fit(six, JobUnits::Mm, &doc.job), Fit::TooSmall);
        assert_eq!(fit(six, JobUnits::Inch, &doc.job), Fit::Fits);
        assert_eq!(
            fit(DVec2::new(500.0, 100.0), JobUnits::Mm, &doc.job),
            Fit::Fits
        );
        assert_eq!(
            fit(DVec2::new(500.0, 450.0), JobUnits::Mm, &doc.job),
            Fit::TooBig
        );
    }

    #[test]
    fn a_drawing_with_units_is_never_guessed_at() {
        let (size, notes, guess) = imported(
            &job(JobUnits::Inch, 610.0, 610.0),
            &dxf(Units::Millimeters, square(6.0)),
        );
        assert!((size.x - 6.0).abs() < 1e-9, "{size:?}");
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(guess, None);
    }

    /// An R12 file as CAD programs write them: CRLF lines, a POLYLINE's points as VERTEXes ended by SEQEND, no
    /// units in the header; with a text and a hatch fill, which are not vectors.
    const PLAIN_R12: &str = "  0\r\nSECTION\r\n  2\r\nHEADER\r\n  9\r\n$ACADVER\r\n  1\r\nAC1009\r\n  0\r\nENDSEC\r\n  0\r\nSECTION\r\n  2\r\nENTITIES\r\n  0\r\nPOLYLINE\r\n  8\r\nLAYER_0\r\n 66\r\n     1\r\n 10\r\n0.0\r\n 20\r\n0.0\r\n 30\r\n0.0\r\n 70\r\n     1\r\n  0\r\nVERTEX\r\n  8\r\nLAYER_0\r\n 10\r\n0.0\r\n 20\r\n0.0\r\n 30\r\n0.0\r\n  0\r\nVERTEX\r\n  8\r\nLAYER_0\r\n 10\r\n6.0\r\n 20\r\n0.0\r\n 30\r\n0.0\r\n  0\r\nVERTEX\r\n  8\r\nLAYER_0\r\n 10\r\n6.0\r\n 20\r\n6.0\r\n 30\r\n0.0\r\n  0\r\nVERTEX\r\n  8\r\nLAYER_0\r\n 10\r\n0.0\r\n 20\r\n6.0\r\n 30\r\n0.0\r\n  0\r\nSEQEND\r\n  8\r\nLAYER_0\r\n  0\r\nTEXT\r\n  8\r\nLAYER_0\r\n 10\r\n1.0\r\n 20\r\n1.0\r\n 30\r\n0.0\r\n 40\r\n0.5\r\n  1\r\nAztec\r\n  0\r\nTEXT\r\n  8\r\nLAYER_0\r\n 10\r\n1.0\r\n 20\r\n2.0\r\n 30\r\n0.0\r\n 40\r\n0.5\r\n  1\r\nSun\r\n  0\r\nHATCH\r\n  8\r\nLAYER_0\r\n  2\r\nSOLID\r\n  0\r\nENDSEC\r\n  0\r\nEOF\r\n";

    #[test]
    fn r12_polylines_come_in_closed_at_their_size() {
        let (size, notes, _) = imported(&job(JobUnits::Mm, 600.0, 400.0), PLAIN_R12.as_bytes());
        assert!((size.x - 152.4).abs() < 1e-9, "{size:?}");
        let change =
            ImportDxf::import(&job(JobUnits::Mm, 600.0, 400.0), PLAIN_R12.as_bytes()).unwrap();
        let mut doc = Document::default();
        doc.apply(&change).unwrap();
        assert_eq!(doc.vectors.len(), 1);
        assert!(doc.vectors.values().all(|p| p.closed));
        assert!(notes.len() == 1, "{notes:?}");
    }

    #[test]
    fn what_is_left_out_is_said_in_plain_words() {
        let (_, notes, _) = imported(&job(JobUnits::Mm, 600.0, 400.0), PLAIN_R12.as_bytes());
        assert_eq!(
            notes[0],
            "Left out 1 hatch fill and 2 texts: only the lines and curves of a DXF come in. \
             To bring text in, turn it into curves in the program that made the DXF."
        );
    }

    #[test]
    fn the_aztec_calendar_imports_like_its_svg() {
        // aztec6inch v2.dxf (11 MB, not kept here) when it was looked into: 2980 closed
        // POLYLINEs of 114,840 VERTEXes in all, nothing else, 6 x 6 with no units; the SVG
        // saved with it imports as 2980 closed vectors, 152.4 mm square
        let Ok(path) = std::env::var("CUT_CRAFT_AZTEC_DXF") else {
            return;
        };
        let bytes = std::fs::read(path).unwrap();
        let doc = job(JobUnits::Mm, 600.0, 400.0);
        let started = std::time::Instant::now();
        let change = ImportDxf::import(&doc, &bytes).unwrap();
        eprintln!("imported in {:?}", started.elapsed());
        let mut doc = doc.clone();
        doc.apply(&change).unwrap();
        assert_eq!(doc.vectors.len(), 2980);
        assert!(doc.vectors.values().all(|p| p.closed));
        let (size, _, _) = imported(&job(JobUnits::Mm, 600.0, 400.0), &bytes);
        assert!((size - DVec2::splat(152.4)).length() < 1e-6, "{size:?}");
    }

    #[test]
    fn nothing_drawable_is_an_error() {
        let err =
            ImportDxf::import(&Document::default(), &dxf(Units::Millimeters, vec![])).unwrap_err();
        assert_eq!(
            err.to_string(),
            "The DXF has no lines, arcs, circles, polylines, ellipses or splines. Export it again from the program that made it."
        );
        assert!(ImportDxf::import(&Document::default(), b"not a dxf").is_err());
    }
}
