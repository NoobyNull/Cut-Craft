//! Dimensions: a length, horizontal or vertical dimension between two points, an angle at a
//! corner or an arc's radius or diameter, as vectors: extension lines, a dimension line with
//! arrow heads, and the measurement in a font, grouped. The group keeps the dimension's
//! settings, so Shift-clicking it with the form open edits it.

use cut_craft_core::document::{Document, Units};
use cut_craft_core::font::{self, Align};
use cut_craft_core::geometry::{Path, Point, Segment, polyline_distance};
use cut_craft_core::glam::DAffine2;
use cut_craft_core::params::Xy;
use cut_craft_core::{Change, Command, CommandError, Drag, Interactive, Meta, ensure_positive};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};

pub struct Dimensions;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// The straight distance, in any direction
    #[default]
    #[schemars(title = "Length Dimension")]
    Length,
    #[schemars(title = "Vertical Dimension")]
    Vertical,
    #[schemars(title = "Horizontal Dimension")]
    Horizontal,
    /// The angle at a corner: drag from the corner to where the arc and its text go
    #[schemars(title = "Angle Dimension")]
    Angle,
    /// An arc's or circle's radius, or its diameter: drag from the arc to where the text goes
    #[schemars(title = "Arc Dimension")]
    Arc,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub kind: Kind,
    /// Drag from one point to the other to set both
    pub from: Xy,
    pub to: Xy,
    #[schemars(schema_with = "font_choices")]
    pub font: String,
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub text_height: f64,
    #[schemars(range(min = 0, max = 6))]
    pub decimal_places: u32,
    /// How far the dimension line sits from the measured points, to their left
    #[schemars(extend("x-unit" = "mm"))]
    pub offset: f64,
    /// Arc dimensions: the diameter instead of the radius
    #[serde(default)]
    #[schemars(title = "Show diameter", extend("x-show-when" = {"kind": "arc"}))]
    pub show_diameter: bool,
    /// Shown instead of the measurement when not empty
    #[schemars(title = "Use custom text")]
    pub custom_text: String,
    /// How close the drag has to start to the corner or arc it measures
    #[serde(default = "default_reach")]
    #[schemars(skip)]
    pub reach: f64,
    /// The dimension being edited (its group): its shapes are replaced. Set when the form
    /// opens on one, or one is Shift-clicked.
    #[schemars(skip)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replaces: Option<u64>,
}

fn default_reach() -> f64 {
    2.0
}

fn font_choices(_: &mut SchemaGenerator) -> Schema {
    json_schema!({ "type": "string", "enum": font::installed(), "x-widget": "drop-down" })
}

impl Default for Params {
    fn default() -> Self {
        Self {
            kind: Kind::Length,
            from: Xy::new(10.0, 10.0),
            to: Xy::new(60.0, 10.0),
            font: font::installed().into_iter().next().unwrap_or_default(),
            text_height: 5.0,
            decimal_places: 2,
            offset: 10.0,
            show_diameter: false,
            custom_text: String::new(),
            reach: default_reach(),
            replaces: None,
        }
    }
}

/// The dimension's marks, its text and where the text goes (centred, turned).
type Drawn = (Vec<Path>, String, DAffine2);

/// The group record's key telling a dimension's settings from text's.
const COMMAND: &str = "command";

impl Params {
    fn text(&self, measured: String) -> String {
        if self.custom_text.is_empty() {
            measured
        } else {
            self.custom_text.clone()
        }
    }

    fn arrow(&self, tip: Point, back: Point, side: Point) -> Path {
        let head = self.text_height * 0.6;
        let wing = side * head * 0.3;
        Path::polyline(
            &[tip, tip + back * head + wing, tip + back * head - wing],
            true,
        )
    }

    /// A length in the job's units, to the decimal places asked.
    fn length_text(&self, doc: &Document, length: f64) -> String {
        let shown = match doc.job.units {
            Units::Mm => length,
            Units::Inch => length / 25.4,
        };
        format!("{shown:.*}", self.decimal_places as usize)
    }

    /// A length, horizontal or vertical dimension between the two points.
    fn linear(&self, doc: &Document) -> Result<Drawn, CommandError> {
        let (a, b): (Point, Point) = (self.from.into(), self.to.into());
        let along = match self.kind {
            Kind::Horizontal => Point::new(b.x - a.x, 0.0),
            Kind::Vertical => Point::new(0.0, b.y - a.y),
            _ => b - a,
        };
        let length = along.length();
        if length <= 0.0 {
            return Err(CommandError::InvalidParams(
                "drag between two different points".into(),
            ));
        }
        let dir = along / length;
        let side = dir.perp();
        // the dimension line runs from a's foot to b's foot, `offset` to the left of a
        let start = a + side * self.offset;
        let end = start + dir * length;
        // extension lines leave a small gap at the measured point and run a little past
        let gap = self.text_height * 0.3;
        let extension = |from: Point, to: Point| {
            let out = (to - from).normalize_or_zero() * gap;
            Path::polyline(&[from + out, to + out], false)
        };
        let paths = vec![
            extension(a, start),
            extension(b, end),
            Path::polyline(&[start, end], false),
            self.arrow(start, dir, side),
            self.arrow(end, -dir, side),
        ];
        let text = self.text(self.length_text(doc, length));
        // text centred above the middle of the line, turned with it
        let middle = start.lerp(end, 0.5) + side * gap;
        let place = DAffine2::from_translation(middle) * DAffine2::from_angle(dir.to_angle());
        Ok((paths, text, place))
    }

    /// The angle at the corner nearest the drag's start (two straight sides meeting), its
    /// arc at the drag's radius; with no corner near, the angle from the drag to a quarter
    /// turn on.
    fn angle(&self, doc: &Document) -> Result<Drawn, CommandError> {
        use std::f64::consts::{FRAC_PI_2, PI, TAU};
        let (centre, at): (Point, Point) = (self.from.into(), self.to.into());
        let radius = centre.distance(at);
        if radius <= 0.0 {
            return Err(CommandError::InvalidParams(
                "drag from the corner to where the arc and its text go".into(),
            ));
        }
        let (a, b) = corner_arms(doc, centre, self.reach).unwrap_or_else(|| {
            let d = (at - centre) / radius;
            (d, d.perp())
        });
        // the sweep from one arm round to the other, the short way
        let turn = (b.to_angle() - a.to_angle()).rem_euclid(TAU);
        let (start, sweep) = if turn > PI {
            (b.to_angle(), TAU - turn)
        } else {
            (a.to_angle(), turn)
        };
        let gap = self.text_height * 0.3;
        let along = |t: f64| centre + Point::from_angle(start + sweep * t) * radius;
        let arc: Vec<Point> = (0..=32).map(|i| along(f64::from(i) / 32.0)).collect();
        let tangent = |t: f64| Point::from_angle(start + sweep * t + FRAC_PI_2);
        let mut paths = vec![
            Path::polyline(&arc, false),
            self.arrow(along(0.0), tangent(0.0), -Point::from_angle(start)),
            self.arrow(along(1.0), -tangent(1.0), -Point::from_angle(start + sweep)),
        ];
        // each arm drawn out to the arc, from a little off the corner
        for arm in [Point::from_angle(start), Point::from_angle(start + sweep)] {
            paths.push(Path::polyline(
                &[centre + arm * gap, centre + arm * (radius + gap)],
                false,
            ));
        }
        let text = self.text(format!(
            "{:.*}°",
            self.decimal_places as usize,
            sweep.to_degrees()
        ));
        let out = Point::from_angle(start + sweep / 2.0);
        let place =
            DAffine2::from_translation(centre + out * (radius + gap + self.text_height / 2.0));
        Ok((paths, text, place))
    }

    /// The radius (or diameter) of the arc or circle nearest the drag's start, its text at
    /// the drag's end with a leader to the arc.
    fn arc(&self, doc: &Document) -> Result<Drawn, CommandError> {
        let (near, text_at): (Point, Point) = (self.from.into(), self.to.into());
        let (centre, radius) = nearest_arc(doc, near, self.reach).ok_or_else(|| {
            CommandError::Failed("drag from an arc or circle to where its radius goes".into())
        })?;
        let dir = (text_at - centre).normalize_or(Point::X);
        let tip = centre + dir * radius;
        let paths = vec![
            Path::polyline(&[text_at, tip], false),
            self.arrow(tip, (text_at - tip).normalize_or(dir), dir.perp()),
        ];
        let measured = if self.show_diameter {
            format!("⌀{}", self.length_text(doc, 2.0 * radius))
        } else {
            format!("R{}", self.length_text(doc, radius))
        };
        let text = self.text(measured);
        Ok((paths, text, DAffine2::from_translation(text_at)))
    }
}

/// The two sides meeting at the straight-line corner of any vector nearest `at`, within
/// `reach`, as unit directions away from the corner.
fn corner_arms(doc: &Document, at: Point, reach: f64) -> Option<(Point, Point)> {
    let mut best: Option<(f64, Point, Point)> = None;
    for path in doc.vectors.values() {
        let mut points: Vec<Point> = std::iter::once(path.start)
            .chain(path.segments.iter().map(Segment::end))
            .collect();
        let straight: Vec<bool> = path
            .segments
            .iter()
            .map(|s| matches!(s, Segment::Line { .. }))
            .collect();
        if path.closed && points.len() > 1 && points[0].distance(points[points.len() - 1]) < 1e-9 {
            points.pop();
        }
        let n = points.len();
        if n < 3 {
            continue;
        }
        for i in 0..n {
            let (before, after) = ((i + n - 1) % n, (i + 1) % n);
            if !path.closed && (i == 0 || i == n - 1) {
                continue;
            }
            // the side in ends at this point, the side out starts here
            if !straight[(i + n - 1) % n] || !straight[i % straight.len()] {
                continue;
            }
            let corner = points[i];
            let d = corner.distance(at);
            if d <= reach && best.is_none_or(|b| d < b.0) {
                let arm = |q: Point| (q - corner).normalize_or_zero();
                best = Some((d, arm(points[before]), arm(points[after])));
            }
        }
    }
    best.map(|(_, a, b)| (a, b))
}

/// The centre and radius of the arc span of any vector nearest `at`, within `reach`.
fn nearest_arc(doc: &Document, at: Point, reach: f64) -> Option<(Point, f64)> {
    let mut best: Option<(f64, Point, f64)> = None;
    for path in doc.vectors.values() {
        let mut from = path.start;
        for segment in &path.segments {
            if let Segment::Arc { center, .. } = *segment {
                let one = Path {
                    start: from,
                    segments: vec![*segment],
                    closed: false,
                };
                let d = polyline_distance(&one.flatten(0.01), at);
                if d <= reach && best.is_none_or(|b| d < b.0) {
                    best = Some((d, center, from.distance(center)));
                }
            }
            from = segment.end();
        }
    }
    best.map(|(_, centre, radius)| (centre, radius))
}

impl Command for Dimensions {
    const ID: &'static str = "dimensions";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Dimensions",
            group: "Create Vectors",
            icon: "drawing/create_vectors/dimensions.png",
            shortcut: "",
        }
    }

    /// On a selected dimension, its own settings, so the form edits it.
    fn initial_params(doc: &Document) -> Params {
        doc.selection
            .iter()
            .filter_map(|id| doc.vector_groups.get(id))
            .find_map(|group| {
                let record = doc.texts.get(group)?;
                if record[COMMAND] != Self::ID {
                    return None;
                }
                let p: Params = serde_json::from_value(record.clone()).ok()?;
                Some(Params {
                    replaces: Some(*group),
                    ..p
                })
            })
            .unwrap_or_default()
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("text height", p.text_height)?;
        let (mut paths, text, place) = match p.kind {
            Kind::Angle => p.angle(doc)?,
            Kind::Arc => p.arc(doc)?,
            _ => p.linear(doc)?,
        };
        let face = font::load(&p.font, false, false, &text).map_err(CommandError::Failed)?;
        paths.extend(
            face.layout(&text, p.text_height, Align::Center, Point::ZERO)
                .iter()
                .map(|q| q.transformed(&place)),
        );
        let group = p
            .replaces
            .filter(|g| doc.texts.contains_key(g))
            .unwrap_or_else(|| doc.next_group());
        let mut c = Change::builder(doc);
        for id in doc.group_members_of(group) {
            c.remove(id);
        }
        let ids: Vec<_> = paths.into_iter().map(|q| c.add(q)).collect();
        for &id in &ids {
            c.set_group(id, Some(group));
        }
        // the settings stay with the group, so the dimension can be edited again
        let mut record = serde_json::to_value(Params {
            replaces: None,
            ..p.clone()
        })
        .expect("params serialise");
        record[COMMAND] = Self::ID.into();
        c.set_text(group, Some(record));
        c.select(ids);
        Ok(c.finish())
    }
}

impl Interactive for Dimensions {
    fn from_drag(drag: Drag, current: &Params) -> Params {
        Params {
            from: Xy::new(drag.start.x, drag.start.y),
            to: Xy::new(drag.end.x, drag.end.y),
            ..current.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Rect;
    use cut_craft_core::testing::{after, document_with, rejection, run_on, square};

    cut_craft_core::contract_tests!(Dimensions);

    fn bounds(doc: &Document) -> Rect {
        doc.selection
            .iter()
            .map(|id| doc.vectors[id].bounds())
            .reduce(Rect::union)
            .unwrap()
    }

    #[test]
    fn a_length_dimension_sits_offset_above_its_points_with_its_text() {
        let doc = after::<Dimensions>(&Params::default());
        let b = bounds(&doc);
        // from (10, 10) to (60, 10), the line 10 above; text 5 tall above that
        assert!(
            (b.min.x - 10.0).abs() < 1e-6 && (b.max.x - 60.0).abs() < 1e-6,
            "{b:?}"
        );
        assert!(b.max.y > 25.0 && b.max.y < 27.0, "{b:?}");
        let first = *doc.selection.iter().next().unwrap();
        assert_eq!(
            doc.group_members(first).len(),
            doc.selection.len(),
            "one group"
        );
    }

    #[test]
    fn vertical_dimensions_measure_only_height() {
        let p = Params {
            kind: Kind::Vertical,
            from: Xy::new(0.0, 0.0),
            to: Xy::new(30.0, 20.0),
            custom_text: "H".into(),
            ..Params::default()
        };
        let doc = after::<Dimensions>(&p);
        // the dimension line runs up 20 mm, 10 mm to the left of x = 0
        let line = doc
            .vectors
            .values()
            .find(|v| v.segments.len() == 1 && v.bounds().size().y > 19.0)
            .unwrap();
        let lb = line.bounds();
        assert!(
            (lb.min.x + 10.0).abs() < 1e-6 && (lb.size().y - 20.0).abs() < 1e-6,
            "{lb:?}"
        );
        let same = Params {
            to: Xy::new(30.0, 0.0),
            ..p
        };
        assert!(rejection::<Dimensions>(&same).contains("two different points"));
    }

    #[test]
    fn an_angle_dimension_measures_the_corner_it_starts_at_and_an_arc_one_a_radius() {
        let mut doc = document_with(vec![
            square(0.0, 0.0, 20.0),
            Path::circle(Point::new(60.0, 60.0), 15.0),
        ]);
        let mut c = Change::builder(&doc);
        c.select([]);
        doc.apply(&c.finish()).unwrap();
        // the square's corner at (20, 0): a right angle, its arc 10 out
        let angle = Params {
            kind: Kind::Angle,
            from: Xy::new(20.5, 0.0),
            to: Xy::new(10.0, 0.0),
            font: "Test Boxes".into(),
            ..Params::default()
        };
        run_on::<Dimensions>(&mut doc, &angle);
        let record = doc.texts.values().last().unwrap();
        assert_eq!(record["command"], "dimensions");
        assert_eq!(record["kind"], "angle");
        let b = bounds(&doc);
        assert!(
            b.min.x < 10.6 && b.max.y > 10.0,
            "the arc round the corner: {b:?}"
        );
        // no corner near: the drag and a quarter turn on
        let free = Params {
            from: Xy::new(100.0, 100.0),
            custom_text: "a".into(),
            ..angle.clone()
        };
        assert!(Dimensions::run(&doc, &free).is_ok());
        // the circle's radius, then its diameter
        let arc = Params {
            kind: Kind::Arc,
            from: Xy::new(75.0, 60.0),
            to: Xy::new(90.0, 60.0),
            font: "Test Boxes".into(),
            ..Params::default()
        };
        let before = doc.vectors.len();
        run_on::<Dimensions>(&mut doc, &arc);
        assert!(
            doc.vectors.len() > before + 2,
            "leader, arrow and the text's boxes"
        );
        let diameter = Params {
            show_diameter: true,
            ..arc.clone()
        };
        assert!(Dimensions::run(&doc, &diameter).is_ok());
        let nowhere = Params {
            from: Xy::new(100.0, 100.0),
            ..arc
        };
        assert!(rejection::<Dimensions>(&nowhere).contains("arc"));
    }

    #[test]
    fn a_dimension_opens_in_the_form_with_its_settings_and_is_replaced_when_edited() {
        let mut doc = after::<Dimensions>(&Params {
            custom_text: "W".into(),
            ..Params::default()
        });
        let p = Dimensions::initial_params(&doc);
        let group = p.replaces.expect("the dimension's group");
        assert_eq!(p.custom_text, "W");
        let members = doc.group_members_of(group).len();
        let vectors = doc.vectors.len();
        run_on::<Dimensions>(
            &mut doc,
            &Params {
                custom_text: "WW".into(),
                ..p
            },
        );
        assert_eq!(
            doc.vectors.len(),
            vectors + 1,
            "one more letter, the rest replaced"
        );
        assert_eq!(doc.group_members_of(group).len(), members + 1);
    }
}
