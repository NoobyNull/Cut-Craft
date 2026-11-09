//! Snapping: points the pointer jumps to while drawing or moving nodes, as Aspire's Snapping
//! Options (F4) choose them: geometry (span ends and middles, arc centres, object centres),
//! the job's centre and corners, and a grid. Smart snapping lines the pointer up with those
//! points across or up and down when none is near, and puts it on the tangent of a span
//! where the span ends, or square to that tangent. A line's far end also snaps, as a CAD
//! program's object snaps do, to where the line would meet a span square on (Perpendicular)
//! or just touch a curve (Tangent). Each snap says what kind it is, so the view can mark it.

use crate::constraint::Anchor;
use crate::document::Document;
use crate::geometry::{Point, Segment};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the dialog's own check boxes"
)]
pub struct SnapOptions {
    pub snap_to_grid: bool,
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub grid_spacing: f64,
    #[schemars(title = "Snap to job center and corners")]
    pub job_center_and_corners: bool,
    /// Snap to points on the vectors (the next five)
    pub geometry_snapping: bool,
    /// The middle of each shape's outline box
    pub object_centers: bool,
    /// Where each line or curve starts and ends (a square marker)
    #[schemars(title = "End points")]
    pub span_end_points: bool,
    /// Halfway along each line or curve (a triangle marker)
    #[schemars(title = "Middle points")]
    pub span_mid_points: bool,
    /// The centre of each circle and arc (a plus marker)
    pub arc_centers: bool,
    /// While drawing a line, where it would just touch a circle or curve without crossing it
    /// (Tangent, a circle marker), or meet a line or curve at a right angle (Perpendicular,
    /// a right-angle marker)
    #[serde(default = "on")]
    #[schemars(title = "Tangent and perpendicular")]
    pub tangent_and_perpendicular: bool,
    /// Line the pointer up with the snap points, across or up and down, when none is near
    #[serde(default = "on")]
    pub smart_snapping: bool,
    /// How near, in screen pixels, the pointer must come to a point to snap to it
    #[schemars(range(min = 1, max = 50))]
    pub snapping_radius: f64,
    /// How far Ctrl and an arrow key move the selection
    #[serde(default = "nudge")]
    #[schemars(title = "Fixed nudge distance", extend("x-unit" = "mm"), range(min = 0))]
    pub fixed_nudge: f64,
}

fn nudge() -> f64 {
    1.0
}

fn on() -> bool {
    true
}

impl Default for SnapOptions {
    fn default() -> Self {
        Self {
            snap_to_grid: false,
            grid_spacing: 10.0,
            job_center_and_corners: true,
            geometry_snapping: true,
            object_centers: true,
            span_end_points: true,
            span_mid_points: true,
            arc_centers: true,
            tangent_and_perpendicular: true,
            smart_snapping: true,
            // ponytail: Aspire's slider runs Small to Large; 8 px is our middle [inf]
            snapping_radius: 8.0,
            fixed_nudge: nudge(),
        }
    }
}

/// What the pointer snapped to, for the marker the view draws and the status bar's words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapKind {
    /// Where a span starts or ends
    End,
    /// Halfway along a span
    Mid,
    /// The centre of an arc or circle
    ArcCenter,
    /// The centre of a vector's bounding box
    ObjectCenter,
    /// A corner or the centre of the job
    JobPoint,
    /// Where a line from the previous point just touches a curve
    Tangent,
    /// Where a line from the previous point meets a span at a right angle
    Perpendicular,
    /// On a guide line
    Guide,
    /// Lined up with a snap point, or running on along a span's end
    Aligned,
    /// A point of the snap grid
    Grid,
}

impl SnapKind {
    /// A word or two for the marker's label.
    pub fn label(self) -> &'static str {
        match self {
            Self::End => "End",
            Self::Mid => "Middle",
            Self::ArcCenter => "Center",
            Self::ObjectCenter => "Shape center",
            Self::JobPoint => "Material",
            Self::Tangent => "Tangent",
            Self::Perpendicular => "Perpendicular",
            Self::Guide => "Guide",
            Self::Aligned => "Lined up",
            Self::Grid => "Grid",
        }
    }

    /// What it means, in plain words, for the status bar.
    pub fn explain(self) -> &'static str {
        match self {
            Self::End => "the end of a line or curve",
            Self::Mid => "halfway along a line or curve",
            Self::ArcCenter => "the center of a circle or arc",
            Self::ObjectCenter => "the middle of a shape's outline box",
            Self::JobPoint => "a corner or the center of the material",
            Self::Tangent => "the line just touches the curve here without crossing it",
            Self::Perpendicular => "the line meets this one at a right angle",
            Self::Guide => "on a guide line",
            Self::Aligned => "in line with a snap point, or running on from a line's end",
            Self::Grid => "a point of the snap grid",
        }
    }
}

/// Where the pointer snapped, and to what.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Snap {
    pub at: Point,
    pub kind: SnapKind,
    /// The thing it snapped to, when a point placed there should stay on it: what a
    /// [`Constraint`](crate::constraint::Constraint) keeps it to.
    pub on: Option<Anchor>,
}

impl Snap {
    /// A snap to nothing in particular (a grid point, a guide, a lined-up point).
    pub fn free(at: Point, kind: SnapKind) -> Self {
        Self { at, kind, on: None }
    }

    /// A snap that a point placed there stays on.
    pub fn on(at: Point, kind: SnapKind, anchor: Anchor) -> Self {
        Self {
            at,
            kind,
            on: Some(anchor),
        }
    }
}

impl SnapOptions {
    /// Where `at` snaps to: the nearest snap point within `reach` (job mm), else the grid
    /// when it is on, else `at` itself.
    pub fn snap(&self, doc: &Document, at: Point, reach: f64) -> Point {
        self.find(doc, None, at, reach).map_or(at, |s| s.at)
    }

    /// Where the end of a line drawn from `from` snaps: as [`Self::snap`], and also to where
    /// the line would meet a span square on, or touch a curve.
    pub fn snap_from(&self, doc: &Document, from: Point, at: Point, reach: f64) -> Point {
        self.find(doc, Some(from), at, reach).map_or(at, |s| s.at)
    }

    /// What `at` snaps to, if anything: the nearest snap point within `reach` (with `from`,
    /// the start of a line being drawn, also its tangent and perpendicular points), else a
    /// guide or a line through a snap point, else the grid.
    pub fn find(&self, doc: &Document, from: Option<Point>, at: Point, reach: f64) -> Option<Snap> {
        let mut points = self.points(doc);
        if let Some(from) = from {
            points.extend(self.square_and_tangent(doc, from));
        }
        let near = points
            .into_iter()
            .map(|s| (s, s.at.distance(at)))
            .filter(|(_, d)| *d <= reach)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((s, _)) = near {
            return Some(s);
        }
        if let Some(s) = self.lined(doc, at, reach) {
            return Some(s);
        }
        (self.snap_to_grid && self.grid_spacing > 0.0).then(|| {
            let g = self.grid_spacing;
            let origin = doc.job.area().min;
            Snap::free(origin + ((at - origin) / g).round() * g, SnapKind::Grid)
        })
    }

    /// With no point near: the lines the pointer is pulled onto: guides (with geometry
    /// snapping) and, with smart snapping, lines through every snap point across and up and
    /// down, and along or square to a span where it ends.
    fn lined(&self, doc: &Document, at: Point, reach: f64) -> Option<Snap> {
        // ponytail: Aspire's smart lines go only through the nodes hovered
        let (mut xs, mut ys) = (vec![], vec![]);
        // angled guides: the point on them nearest the pointer
        let mut tilted: Vec<Snap> = vec![];
        if self.geometry_snapping {
            for g in &doc.guides {
                if g.tilted() {
                    let q = g.nearest(at);
                    if q.distance(at) <= reach {
                        tilted.push(Snap::free(q, SnapKind::Guide));
                    }
                } else {
                    if g.vertical { &mut xs } else { &mut ys }.push((g.at, SnapKind::Guide));
                }
            }
        }
        if self.smart_snapping {
            for q in self.points(doc) {
                xs.push((q.at.x, SnapKind::Aligned));
                ys.push((q.at.y, SnapKind::Aligned));
            }
        }
        let nearest = |lines: &[(f64, SnapKind)], v: f64| {
            lines
                .iter()
                .copied()
                .filter(|(l, _)| (l - v).abs() <= reach)
                .min_by(|a, b| (a.0 - v).abs().total_cmp(&(b.0 - v).abs()))
        };
        let (x, y) = (nearest(&xs, at.x), nearest(&ys, at.y));
        let lined = (x.is_some() || y.is_some()).then(|| {
            // a guide's own name wins over lining up
            let guide = [x, y].iter().flatten().any(|l| l.1 == SnapKind::Guide);
            Snap::free(
                Point::new(x.map_or(at.x, |l| l.0), y.map_or(at.y, |l| l.0)),
                if guide {
                    SnapKind::Guide
                } else {
                    SnapKind::Aligned
                },
            )
        });
        let lined = tilted
            .into_iter()
            .chain(lined)
            .min_by(|a, b| a.at.distance(at).total_cmp(&b.at.distance(at)));
        // a span's tangent where it ends, or the line square to it, when nearer
        let along = self
            .smart_snapping
            .then(|| self.tangent_snap(doc, at, reach))
            .flatten()
            .map(|p| Snap::free(p, SnapKind::Aligned));
        match (lined, along) {
            (Some(l), Some(t)) if t.at.distance(at) < l.at.distance(at) => Some(t),
            (Some(l), _) => Some(l),
            (None, t) => t,
        }
    }

    /// Points on the spans where a line from `from` would meet them at a right angle
    /// (Perpendicular), or just touch a curve (Tangent).
    fn square_and_tangent(&self, doc: &Document, from: Point) -> Vec<Snap> {
        let mut out = Vec::new();
        if !(self.tangent_and_perpendicular && self.geometry_snapping) {
            return out;
        }
        for (id, path) in &doc.vectors {
            if !doc.shown(*id) {
                continue;
            }
            let mut start = path.start;
            for (k, span) in path.segments.iter().enumerate() {
                for (at, kind) in touch_and_square(start, *span, from) {
                    let anchor = if kind == SnapKind::Tangent {
                        Anchor::Tangent {
                            vector: *id,
                            span: k,
                        }
                    } else {
                        Anchor::Perpendicular {
                            vector: *id,
                            span: k,
                        }
                    };
                    out.push(Snap::on(at, kind, anchor));
                }
                start = span.end();
            }
        }
        out
    }

    /// The nearest point within `reach` of `at` on a line through a span's end along the
    /// span's direction there (its tangent), or square to it.
    fn tangent_snap(&self, doc: &Document, at: Point, reach: f64) -> Option<Point> {
        let mut best: Option<(Point, f64)> = None;
        for (id, path) in &doc.vectors {
            if !doc.shown(*id) || !self.geometry_snapping {
                continue;
            }
            let mut from = path.start;
            for s in &path.segments {
                let to = s.end();
                // the span's direction leaving its start and arriving at its end
                let (out_dir, in_dir) = match *s {
                    Segment::Line { .. } => (to - from, to - from),
                    Segment::Cubic { c1, c2, .. } => (c1 - from, to - c2),
                    Segment::Arc { center, ccw, .. } => {
                        let turn = |r: Point| if ccw { r.perp() } else { -r.perp() };
                        (turn(from - center), turn(to - center))
                    }
                };
                for (origin, dir) in [(from, out_dir), (to, in_dir)] {
                    let Some(dir) = dir.try_normalize() else {
                        continue;
                    };
                    for d in [dir, dir.perp()] {
                        let foot = origin + d * (at - origin).dot(d);
                        let gap = foot.distance(at);
                        if gap <= reach && best.is_none_or(|b| gap < b.1) {
                            best = Some((foot, gap));
                        }
                    }
                }
                from = to;
            }
        }
        best.map(|b| b.0)
    }

    fn points(&self, doc: &Document) -> Vec<Snap> {
        let mut out = Vec::new();
        let mut put = |at, kind, on| out.push(Snap { at, kind, on });
        if self.job_center_and_corners {
            for (corner, at) in material_points(doc.job.area()).into_iter().enumerate() {
                let corner = u8::try_from(corner).expect("five points");
                put(at, SnapKind::JobPoint, Some(Anchor::Material { corner }));
            }
        }
        if !self.geometry_snapping {
            return out;
        }
        for (&vector, path) in &doc.vectors {
            if !doc.shown(vector) {
                continue;
            }
            let n = path.segments.len();
            // a closed path's last node is its first
            let node = |k: usize| if path.closed && k == n { 0 } else { k };
            let mut from = path.start;
            if self.span_end_points {
                put(from, SnapKind::End, Some(Anchor::End { vector, node: 0 }));
            }
            for (span, s) in path.segments.iter().enumerate() {
                let to = s.end();
                if self.span_end_points {
                    let node = node(span + 1);
                    put(to, SnapKind::End, Some(Anchor::End { vector, node }));
                }
                if self.span_mid_points {
                    let mid = span_mid(from, *s);
                    put(mid, SnapKind::Mid, Some(Anchor::Mid { vector, span }));
                }
                if let (true, Segment::Arc { center, .. }) = (self.arc_centers, s) {
                    let on = Some(Anchor::Center { vector, span });
                    put(*center, SnapKind::ArcCenter, on);
                }
                from = to;
            }
            // after the spans, so a circle's own centre names it where the two meet
            if self.object_centers {
                let on = Some(Anchor::ShapeCenter { vector });
                put(path.bounds().center(), SnapKind::ObjectCenter, on);
            }
        }
        out
    }
}

/// The material's corners and centre, in the order [`Anchor::Material`] numbers them.
pub(crate) fn material_points(a: crate::geometry::Rect) -> [Point; 5] {
    [
        a.min,
        a.max,
        Point::new(a.min.x, a.max.y),
        Point::new(a.max.x, a.min.y),
        a.center(),
    ]
}

/// Halfway along `span`, which starts at `from`.
pub(crate) fn span_mid(from: Point, span: Segment) -> Point {
    let to = span.end();
    match span {
        Segment::Arc { center, ccw, .. } => {
            let half = crate::geometry::arc_sweep(from, to, center, ccw) / 2.0;
            center + (from - center).rotate(glam::DVec2::from_angle(half))
        }
        Segment::Cubic { c1, c2, .. } => (from + 3.0 * c1 + 3.0 * c2 + to) / 8.0,
        Segment::Line { .. } => from.lerp(to, 0.5),
    }
}

/// Where a line from `from` meets `span` (starting at `start`) at a right angle
/// ([`SnapKind::Perpendicular`]) or just touches it ([`SnapKind::Tangent`]).
pub(crate) fn touch_and_square(start: Point, span: Segment, from: Point) -> Vec<(Point, SnapKind)> {
    use crate::geometry::{angle, in_sweep};
    let (square, touch) = (SnapKind::Perpendicular, SnapKind::Tangent);
    let end = span.end();
    let mut out = Vec::new();
    match span {
        Segment::Line { .. } => {
            let d = end - start;
            if d.length_squared() > 1e-18 {
                let t = (from - start).dot(d) / d.length_squared();
                if (0.0..=1.0).contains(&t) {
                    out.push((start + d * t, square));
                }
            }
        }
        Segment::Arc { center, ccw, .. } => {
            let (r, v) = ((start - center).length(), from - center);
            let dist = v.length();
            if dist > 1e-9 {
                let u = v / dist;
                // square on: the nearest and farthest points; touching: the two tangent
                // points, when `from` is outside the circle
                let mut on = vec![(center + u * r, square), (center - u * r, square)];
                if dist > r {
                    let turn = (r / dist).acos();
                    for t in [turn, -turn] {
                        on.push((center + u.rotate(glam::DVec2::from_angle(t)) * r, touch));
                    }
                }
                let (a0, a1) = (angle(start - center), angle(end - center));
                out.extend(
                    on.into_iter()
                        .filter(|q| in_sweep(a0, a1, ccw, angle(q.0 - center))),
                );
            }
        }
        Segment::Cubic { c1, c2, .. } => {
            let b = [start, c1, c2, end];
            let (at, dir) = (|t| cubic_at(b, t), |t| cubic_dir(b, t));
            // touching: the line from `from` runs along the curve; square on: it crosses
            // the curve's direction at a right angle
            let along = |t: f64| (at(t) - from).perp_dot(dir(t));
            let across = |t: f64| (at(t) - from).dot(dir(t));
            out.extend(roots(along).into_iter().map(|t| (at(t), touch)));
            out.extend(roots(across).into_iter().map(|t| (at(t), square)));
        }
    }
    out
}

/// A cubic Bézier's point at `t`.
fn cubic_at([p0, p1, p2, p3]: [Point; 4], t: f64) -> Point {
    let u = 1.0 - t;
    p0 * (u * u * u) + p1 * (3.0 * u * u * t) + p2 * (3.0 * u * t * t) + p3 * (t * t * t)
}

/// A cubic Bézier's direction (its derivative) at `t`.
fn cubic_dir([p0, p1, p2, p3]: [Point; 4], t: f64) -> Point {
    let u = 1.0 - t;
    (p1 - p0) * (3.0 * u * u) + (p2 - p1) * (6.0 * u * t) + (p3 - p2) * (3.0 * t * t)
}

/// Where `f` crosses zero on 0..=1: a sign change between samples, closed in on by halving.
fn roots(f: impl Fn(f64) -> f64) -> Vec<f64> {
    const SAMPLES: u32 = 64;
    const HALVINGS: u32 = 50;
    let mut out = Vec::new();
    let mut prev = (0.0, f(0.0));
    for i in 1..=SAMPLES {
        let t = f64::from(i) / f64::from(SAMPLES);
        let ft = f(t);
        if prev.1 == 0.0 {
            out.push(prev.0);
        } else if prev.1.signum() != ft.signum() && ft != 0.0 {
            let (mut lo, mut hi, mut f_lo) = (prev.0, t, prev.1);
            for _ in 0..HALVINGS {
                let mid = f64::midpoint(lo, hi);
                let fm = f(mid);
                if fm.signum() == f_lo.signum() {
                    (lo, f_lo) = (mid, fm);
                } else {
                    hi = mid;
                }
            }
            out.push(f64::midpoint(lo, hi));
        }
        prev = (t, ft);
    }
    if prev.1 == 0.0 {
        out.push(prev.0);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{document_with, square};

    #[test]
    fn a_line_end_snaps_square_onto_a_span_and_onto_a_tangent() {
        let doc = document_with(vec![
            square(10.0, 10.0, 20.0),
            crate::geometry::Path::circle(Point::new(50.0, 70.0), 10.0),
        ]);
        let s = SnapOptions::default();
        let (from, at) = (Point::new(50.0, 14.0), Point::new(30.4, 14.5));
        assert_eq!(
            s.snap_from(&doc, from, at, 1.0),
            Point::new(30.0, 14.0),
            "square onto the right side"
        );
        assert_eq!(s.snap(&doc, at, 1.0), Point::new(30.0, 14.5), "no start");
        let tangent = Point::new(55.0, 70.0 - 75.0_f64.sqrt());
        let got = s.snap_from(&doc, Point::new(70.0, 70.0), Point::new(55.3, 61.6), 1.0);
        assert!(got.distance(tangent) < 1e-9, "{got:?}");
        let off = SnapOptions {
            smart_snapping: false,
            tangent_and_perpendicular: false,
            ..SnapOptions::default()
        };
        assert_eq!(off.snap_from(&doc, from, at, 1.0), at);
    }

    #[test]
    fn each_snap_says_what_it_is() {
        let doc = document_with(vec![
            square(10.0, 10.0, 20.0),
            crate::geometry::Path::circle(Point::new(50.0, 70.0), 10.0),
        ]);
        let s = SnapOptions::default();
        let kind = |from, at| s.find(&doc, from, at, 1.0).map(|k| k.kind);
        assert_eq!(kind(None, Point::new(29.6, 30.3)), Some(SnapKind::End));
        assert_eq!(kind(None, Point::new(20.4, 10.2)), Some(SnapKind::Mid));
        assert_eq!(
            kind(None, Point::new(50.2, 70.2)),
            Some(SnapKind::ArcCenter)
        );
        assert_eq!(
            kind(None, Point::new(19.5, 20.5)),
            Some(SnapKind::ObjectCenter)
        );
        assert_eq!(kind(None, Point::new(0.3, 0.2)), Some(SnapKind::JobPoint));
        assert_eq!(kind(None, Point::new(29.7, 45.0)), Some(SnapKind::Aligned));
        assert_eq!(kind(None, Point::new(57.0, 57.0)), None, "nothing near");
        let from = Point::new(50.0, 14.0);
        assert_eq!(
            kind(Some(from), Point::new(30.4, 14.5)),
            Some(SnapKind::Perpendicular)
        );
        assert_eq!(
            kind(Some(Point::new(70.0, 70.0)), Point::new(55.3, 61.6)),
            Some(SnapKind::Tangent)
        );
        for k in [SnapKind::End, SnapKind::Mid, SnapKind::Tangent] {
            assert!(!k.label().is_empty() && !k.explain().is_empty());
        }
    }

    #[test]
    fn tangent_snapping_works_with_smart_snapping_off() {
        let doc = document_with(vec![crate::geometry::Path::circle(
            Point::new(50.0, 70.0),
            10.0,
        )]);
        let s = SnapOptions {
            smart_snapping: false,
            ..SnapOptions::default()
        };
        let tangent = Point::new(55.0, 70.0 - 75.0_f64.sqrt());
        let got = s.snap_from(&doc, Point::new(70.0, 70.0), Point::new(55.3, 61.6), 1.0);
        assert!(got.distance(tangent) < 1e-9, "{got:?}");
        let off = SnapOptions {
            tangent_and_perpendicular: false,
            ..s
        };
        assert_eq!(
            off.snap_from(&doc, Point::new(70.0, 70.0), Point::new(55.3, 61.6), 1.0),
            Point::new(55.3, 61.6)
        );
    }

    #[test]
    fn a_line_snaps_tangent_and_square_onto_a_bezier_curve() {
        use crate::geometry::Path;
        // a hump from (0, 0) up to its top at (20, 15) and down to (40, 0)
        let hump = Path {
            start: Point::new(0.0, 0.0),
            segments: vec![Segment::Cubic {
                c1: Point::new(0.0, 20.0),
                c2: Point::new(40.0, 20.0),
                to: Point::new(40.0, 0.0),
            }],
            closed: false,
        };
        let doc = document_with(vec![hump]);
        // the top is the curve's middle too: leave that and the centre out
        let s = SnapOptions {
            job_center_and_corners: false,
            smart_snapping: false,
            span_mid_points: false,
            object_centers: false,
            ..SnapOptions::default()
        };
        // from straight above the top, the line meets it square on at the top
        let hit = s.find(
            &doc,
            Some(Point::new(20.0, 40.0)),
            Point::new(20.4, 15.3),
            1.0,
        );
        let hit = hit.expect("square on");
        assert_eq!(hit.kind, SnapKind::Perpendicular);
        assert!(hit.at.distance(Point::new(20.0, 15.0)) < 1e-6, "{hit:?}");
        // from level with the top, far to the right, the line just touches it at the top
        let hit = s.find(
            &doc,
            Some(Point::new(100.0, 15.0)),
            Point::new(19.6, 14.8),
            1.0,
        );
        let hit = hit.expect("touching");
        assert_eq!(hit.kind, SnapKind::Tangent);
        assert!(hit.at.distance(Point::new(20.0, 15.0)) < 1e-6, "{hit:?}");
    }

    #[test]
    fn points_snap_to_corners_middles_centres_and_the_grid() {
        let doc = document_with(vec![square(10.0, 10.0, 20.0)]);
        let s = SnapOptions::default();
        assert_eq!(
            s.snap(&doc, Point::new(29.6, 30.3), 1.0),
            Point::new(30.0, 30.0),
            "a corner"
        );
        assert_eq!(
            s.snap(&doc, Point::new(20.4, 10.2), 1.0),
            Point::new(20.0, 10.0),
            "a middle"
        );
        assert_eq!(
            s.snap(&doc, Point::new(19.5, 20.5), 1.0),
            Point::new(20.0, 20.0),
            "the centre"
        );
        assert_eq!(
            s.snap(&doc, Point::new(57.0, 57.0), 1.0),
            Point::new(57.0, 57.0),
            "nothing near"
        );
        let grid = SnapOptions {
            snap_to_grid: true,
            ..SnapOptions::default()
        };
        assert_eq!(
            grid.snap(&doc, Point::new(57.0, 53.0), 1.0),
            Point::new(60.0, 50.0)
        );
        let none = SnapOptions {
            geometry_snapping: false,
            job_center_and_corners: false,
            ..SnapOptions::default()
        };
        assert_eq!(
            none.snap(&doc, Point::new(29.6, 30.3), 1.0),
            Point::new(29.6, 30.3)
        );
    }

    #[test]
    fn smart_snapping_runs_on_along_a_span_and_square_to_it() {
        let doc = document_with(vec![crate::geometry::Path::polyline(
            &[Point::new(0.0, 0.0), Point::new(10.0, 10.0)],
            false,
        )]);
        let s = SnapOptions {
            job_center_and_corners: false,
            ..SnapOptions::default()
        };
        let on = s.snap(&doc, Point::new(20.3, 19.8), 1.0);
        assert!(
            on.distance(Point::new(20.05, 20.05)) < 1e-9,
            "on along it: {on}"
        );
        let square = s.snap(&doc, Point::new(16.1, 3.8), 1.0);
        assert!(
            square.distance(Point::new(16.15, 3.85)) < 1e-9,
            "square to it: {square}"
        );
        let off = SnapOptions {
            smart_snapping: false,
            ..s
        };
        assert_eq!(
            off.snap(&doc, Point::new(20.3, 19.8), 1.0),
            Point::new(20.3, 19.8)
        );
    }

    #[test]
    fn smart_snapping_lines_the_pointer_up_with_a_point() {
        let doc = document_with(vec![square(10.0, 10.0, 20.0)]);
        let s = SnapOptions::default();
        assert_eq!(
            s.snap(&doc, Point::new(29.7, 45.0), 1.0),
            Point::new(30.0, 45.0),
            "straight above the right side"
        );
        assert_eq!(
            s.snap(&doc, Point::new(45.0, 19.4), 1.0),
            Point::new(45.0, 20.0),
            "level with the centre"
        );
        let off = SnapOptions {
            smart_snapping: false,
            ..SnapOptions::default()
        };
        assert_eq!(
            off.snap(&doc, Point::new(29.7, 45.0), 1.0),
            Point::new(29.7, 45.0)
        );
    }

    #[test]
    fn guides_pull_the_pointer_onto_them() {
        use crate::document::Guide;
        let mut doc = document_with(vec![]);
        doc.guides = vec![Guide::vertical(42.0), Guide::horizontal(17.0)];
        let s = SnapOptions {
            job_center_and_corners: false,
            ..SnapOptions::default()
        };
        assert_eq!(
            s.snap(&doc, Point::new(42.6, 70.0), 1.0),
            Point::new(42.0, 70.0)
        );
        assert_eq!(
            s.snap(&doc, Point::new(41.5, 16.5), 1.0),
            Point::new(42.0, 17.0),
            "where they cross"
        );
        let off = SnapOptions {
            geometry_snapping: false,
            ..s
        };
        assert_eq!(
            off.snap(&doc, Point::new(42.6, 70.0), 1.0),
            Point::new(42.6, 70.0)
        );
    }

    #[test]
    fn an_arc_snaps_to_its_middle_and_centre() {
        let doc = document_with(vec![crate::geometry::Path::circle(
            Point::new(50.0, 50.0),
            10.0,
        )]);
        let s = SnapOptions::default();
        assert!(
            s.snap(&doc, Point::new(50.3, 59.8), 1.0)
                .distance(Point::new(50.0, 60.0))
                < 1e-9
        );
        assert_eq!(
            s.snap(&doc, Point::new(50.2, 50.2), 1.0),
            Point::new(50.0, 50.0)
        );
    }
}
