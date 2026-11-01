//! Draw Line / Polyline drawn in the 2D view, as Aspire's tool is: a click adds a point, a
//! click and drag adds one with a smooth span (the next span smooth too), S switches between
//! curves and straight lines, Backspace takes the last point back, and typed values place
//! the next point (a length along the pointer's direction, `x,y`, or `angle A length L`).
//! The app finishes it (Space, Tab to close, Esc, or a click on the first point) into the
//! command's params.

use cut_craft_core::geometry::{Path, Point, Segment};

/// A polyline being drawn.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Draft {
    pub points: Vec<Point>,
    /// Each span's control points (span `i` runs from point `i` to the next); `None`: straight.
    pub curves: Vec<Option<[Point; 2]>>,
    /// The handle a drag left on the last point, which the next span starts along.
    out: Option<Point>,
    /// S's curve mode: clicked points are joined by smooth curves.
    pub curved: bool,
    /// Values typed so far, placed with Enter.
    pub typed: String,
    /// Tab closed the last one: egui moved its keyboard focus with it, to be taken back.
    pub tabbed: bool,
    /// Edit Polyline: the open vector drawn on from (by id, turned to end where the drawing
    /// starts), which the drawing extends rather than making a new one.
    pub extends: Option<(cut_craft_core::document::EntityId, Path)>,
}

impl Draft {
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// The span from the last point to `to`, curved as the mode or a dragged handle says.
    pub fn span_to(&self, to: Point) -> Option<[Point; 2]> {
        let &prev = self.points.last()?;
        let into = to.lerp(prev, 1.0 / 3.0);
        if let Some(out) = self.out {
            return Some([out, into]);
        }
        self.curved.then(|| {
            // along the line through the point before, so the curve runs on smoothly
            let before = self.points.iter().rev().nth(1).copied();
            let tangent = before.map_or(to - prev, |b| (to - b) / 2.0);
            [prev + tangent / 3.0, into]
        })
    }

    /// A click: the next point.
    pub fn click(&mut self, at: Point) {
        if !self.points.is_empty() {
            self.curves.push(self.span_to(at));
        }
        self.points.push(at);
        self.out = None;
    }

    /// A drag from `at` to `handle`: the next point, its span curving into it along the drag,
    /// and the next span leaving towards `handle`.
    pub fn drag(&mut self, at: Point, handle: Point) {
        if let Some(&prev) = self.points.last() {
            let out = self.out.unwrap_or_else(|| prev.lerp(at, 1.0 / 3.0));
            self.curves.push(Some([out, at - (handle - at)]));
        }
        self.points.push(at);
        self.out = Some(handle);
    }

    /// Backspace: the last point taken back.
    pub fn back(&mut self) {
        self.points.pop();
        self.curves.truncate(self.points.len().saturating_sub(1));
        self.out = None;
    }

    /// S: a dragged handle's smoothing dropped (the next span straight), else curves and
    /// straight lines swapped.
    pub fn toggle_curves(&mut self) {
        if self.out.take().is_none() {
            self.curved = !self.curved;
        }
    }

    /// The polyline so far, closed or not, and (drawing) its span on to the pointer.
    pub fn path(&self, closed: bool, pointer: Option<Point>) -> Option<Path> {
        let &start = self.points.first()?;
        let mut segments: Vec<Segment> = self.points[1..]
            .iter()
            .zip(&self.curves)
            .map(|(&to, curve)| segment(to, *curve))
            .collect();
        if let Some(to) = pointer {
            segments.push(segment(to, self.span_to(to)));
        }
        if closed {
            segments.push(segment(start, self.span_to(start)));
        }
        (!segments.is_empty()).then_some(Path {
            start,
            segments,
            closed,
        })
    }

    /// Edit Polyline's result: the vector it extends with its spans added (closed back to
    /// the vector's start when `closed`), if it has a span.
    pub fn extended(&self, closed: bool) -> Option<(cut_craft_core::document::EntityId, Path)> {
        let (id, base) = self.extends.as_ref()?;
        let tail = self.path(false, None)?;
        let mut path = base.clone();
        path.segments.extend(tail.segments);
        if closed {
            path.segments
                .push(segment(base.start, self.span_to(base.start)));
            path.closed = true;
        }
        Some((*id, path))
    }

    /// The Draw Line / Polyline params that make it, if it has a span.
    pub fn params(&self, closed: bool) -> Option<serde_json::Value> {
        let closed = closed && self.points.len() >= 3;
        if self.points.len() < 2 {
            return None;
        }
        let xy = |p: Point| serde_json::json!({ "x": p.x, "y": p.y });
        let mut curves = self.curves.clone();
        if closed {
            curves.push(self.span_to(self.points[0]));
        }
        let curves: Vec<serde_json::Value> = curves
            .iter()
            .map(|c| {
                c.map_or(serde_json::Value::Null, |[a, b]| {
                    serde_json::json!([xy(a), xy(b)])
                })
            })
            .collect();
        Some(serde_json::json!({
            "points": self.points.iter().map(|&p| xy(p)).collect::<Vec<_>>(),
            "closed": closed,
            "curves": curves,
        }))
    }

    /// Where the values typed place the next point, `unit` millimetres to a typed unit and
    /// `toward` the pointer: `length` along the pointer's direction from the last point,
    /// `x,y`, or `angle A length L` (degrees from the last point).
    pub fn typed_point(&self, unit: f64, toward: Point) -> Option<Point> {
        let text = self.typed.trim().to_ascii_lowercase();
        let number = |s: &str| s.trim().parse::<f64>().ok();
        if let Some((x, y)) = text.split_once(',') {
            return Some(Point::new(number(x)?, number(y)?) * unit);
        }
        let last = *self.points.last()?;
        if let Some((angle, rest)) = text.split_once('a') {
            let length = number(rest.trim().trim_end_matches('l'))? * unit;
            let (sin, cos) = number(angle)?.to_radians().sin_cos();
            return Some(last + Point::new(cos, sin) * length);
        }
        let length = number(&text)? * unit;
        let along = (toward - last).try_normalize()?;
        Some(last + along * length)
    }
}

fn segment(to: Point, curve: Option<[Point; 2]>) -> Segment {
    match curve {
        Some([c1, c2]) => Segment::Cubic { c1, c2, to },
        None => Segment::Line { to },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn clicks_make_lines_and_a_drag_a_curve_the_next_span_follows() {
        let mut d = Draft::default();
        d.click(p(0.0, 0.0));
        d.click(p(10.0, 0.0));
        assert_eq!(d.curves, [None], "a click: a straight span");
        // a drag at (20, 0) towards (20, 5): the span curves in, the next leaves upwards
        d.drag(p(20.0, 0.0), p(20.0, 5.0));
        assert_eq!(
            d.curves[1].unwrap()[1],
            p(20.0, -5.0),
            "into it along the drag"
        );
        d.click(p(30.0, 10.0));
        assert_eq!(
            d.curves[2].unwrap()[0],
            p(20.0, 5.0),
            "the next span is smooth too"
        );
        // S drops the smoothing: the next span is straight
        d.drag(p(40.0, 10.0), p(45.0, 10.0));
        d.toggle_curves();
        d.click(p(50.0, 0.0));
        assert_eq!(d.curves[4], None);
        assert!(!d.curved, "S dropped the handle, not the mode");
        // S again: curve mode, where clicks are joined smoothly
        d.toggle_curves();
        d.click(p(60.0, 10.0));
        assert!(d.curves[5].is_some() && d.curved);
        // Backspace takes points back
        d.back();
        d.back();
        assert_eq!((d.points.len(), d.curves.len()), (5, 4));
    }

    #[test]
    fn the_params_close_and_draw_the_same_path_as_the_preview() {
        let mut d = Draft::default();
        assert!(d.params(false).is_none(), "nothing to draw");
        d.click(p(0.0, 0.0));
        d.drag(p(10.0, 0.0), p(10.0, 5.0));
        d.click(p(10.0, 10.0));
        let v = d.params(true).unwrap();
        assert_eq!(v["closed"], true);
        assert_eq!(
            v["curves"].as_array().unwrap().len(),
            3,
            "the closing span too"
        );
        let path = d.path(true, None).unwrap();
        assert_eq!(path.segments.len(), 3);
        assert!(matches!(path.segments[1], Segment::Cubic { .. }));
        assert_eq!(path.segments[2].end(), p(0.0, 0.0));
    }

    #[test]
    fn typed_values_place_the_next_point() {
        let mut d = Draft {
            typed: "1,2.5".into(),
            ..Draft::default()
        };
        assert_eq!(d.typed_point(1.0, p(0.0, 0.0)), Some(p(1.0, 2.5)), "x,y");
        d.click(p(10.0, 10.0));
        d.typed = "3.5".into();
        assert_eq!(
            d.typed_point(1.0, p(20.0, 10.0)),
            Some(p(13.5, 10.0)),
            "along the pointer"
        );
        d.typed = "90A 2L".into();
        let up = d.typed_point(25.4, p(0.0, 0.0)).unwrap();
        assert!(
            up.distance(p(10.0, 60.8)) < 1e-9,
            "angle and length, in inches: {up}"
        );
        d.typed = "x".into();
        assert_eq!(d.typed_point(1.0, p(0.0, 0.0)), None);
    }
}
