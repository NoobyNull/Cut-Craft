//! Bending a drawing along an open line, like lettering on an arc: the drawing is sized so
//! its width runs the line's length (less a gap at each end), proportions kept, and each
//! point goes as far along the line as it was across the drawing, and as far out from the
//! line (square to it) as it was up from the drawing's bottom.

use crate::fit::FitError;
use crate::geometry::{Point, Rect};
use glam::DVec2;

/// Which side of the line the drawing sits on, looking at the line running left to right.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Side {
    /// The drawing stands on the line, as lettering stands on a ruled line.
    #[default]
    Above,
    /// The line runs through the middle of the drawing.
    On,
    /// The drawing hangs below the line.
    Below,
}

/// A drawing's box bent along an open line. See the module's description.
#[derive(Clone, Debug)]
pub struct Bend {
    from: Rect,
    /// The line's points, read left to right.
    line: Vec<Point>,
    /// How far along the line each point is.
    run: Vec<f64>,
    /// The way out from the line at each point (to its left, so up for a line read left
    /// to right): the sides' own directions averaged, so the drawing bends without gaps.
    out: Vec<DVec2>,
    scale: f64,
    /// How far along the line the drawing starts.
    start: f64,
    /// How far out from the line the drawing's bottom is (below it is negative).
    base: f64,
}

impl Bend {
    /// The bend taking the box `drawing` along the open line `line` (its points in order),
    /// `gap` from each end, on the `side` chosen.
    pub fn new(drawing: Rect, line: &[Point], gap: f64, side: Side) -> Result<Self, FitError> {
        let extent = drawing.size();
        if extent.x <= 0.0 || !extent.is_finite() {
            return Err(FitError::NoDrawing);
        }
        let mut line: Vec<Point> = line.to_vec();
        line.dedup_by(|a, b| a.distance(*b) <= 1e-9);
        if line.len() < 2 {
            return Err(FitError::NoLine);
        }
        // read left to right, so the drawing comes out upright rather than upside down
        if line[0].x > line[line.len() - 1].x {
            line.reverse();
        }
        let run: Vec<f64> = std::iter::once(0.0)
            .chain(line.windows(2).scan(0.0, |total, w| {
                *total += w[0].distance(w[1]);
                Some(*total)
            }))
            .collect();
        let length = run[run.len() - 1];
        if length <= 0.0 || !length.is_finite() {
            return Err(FitError::NoLine);
        }
        let gap = gap.max(0.0);
        let room = length - 2.0 * gap;
        if room <= 0.0 {
            return Err(FitError::GapTooBig);
        }
        let sides: Vec<DVec2> = line
            .windows(2)
            .map(|w| (w[1] - w[0]).normalize_or_zero().perp())
            .collect();
        let out = (0..line.len())
            .map(|i| {
                let before = sides[i.saturating_sub(1)];
                let after = sides[i.min(sides.len() - 1)];
                // averaged, then lengthened so the bent drawing keeps its height at a corner
                let mean = (before + after).normalize_or(after);
                mean / mean.dot(after).max(0.25)
            })
            .collect();
        let scale = room / extent.x;
        let height = extent.y * scale;
        let base = match side {
            Side::Above => 0.0,
            Side::On => -height / 2.0,
            Side::Below => -height,
        };
        Ok(Self {
            from: drawing,
            line,
            run,
            out,
            scale,
            start: gap,
            base,
        })
    }

    /// How much bigger the drawing is made (its width runs the line's length).
    pub fn scale(&self) -> f64 {
        self.scale
    }

    /// Where the drawing's point `p` goes.
    pub fn point(&self, p: Point) -> Point {
        let along = self.start + (p.x - self.from.min.x) * self.scale;
        let up = self.base + (p.y - self.from.min.y) * self.scale;
        let (at, out) = self.at(along);
        at + out * up
    }

    /// The point `along` the line and the way out from it there: past either end the line
    /// carries on straight.
    fn at(&self, along: f64) -> (Point, DVec2) {
        let last = self.run.len() - 1;
        let k = self.run.partition_point(|&r| r <= along).clamp(1, last);
        let (a, b) = (self.run[k - 1], self.run[k]);
        let t = (along - a) / (b - a).max(f64::EPSILON);
        let at = self.line[k - 1] + (self.line[k] - self.line[k - 1]) * t;
        let t = t.clamp(0.0, 1.0);
        (at, self.out[k - 1] * (1.0 - t) + self.out[k] * t)
    }

    /// Whether the line bends tighter than the drawing is tall, somewhere along it, so the
    /// drawing's edge on the inside of the bend folds back over itself (it crumples).
    pub fn folds(&self) -> bool {
        let height = self.from.size().y * self.scale;
        let (from, to) = (self.start, self.start + self.from.size().x * self.scale);
        let steps = 2000;
        let edge = |up: f64| -> Vec<Point> {
            (0..=steps)
                .map(|i| {
                    let along = from + (to - from) * f64::from(i) / f64::from(steps);
                    let (at, out) = self.at(along);
                    at + out * up
                })
                .collect()
        };
        let line = edge(0.0);
        [self.base, self.base + height].into_iter().any(|up| {
            up.abs() > 1e-9
                && edge(up)
                    .windows(2)
                    .zip(line.windows(2))
                    .any(|(e, l)| (e[1] - e[0]).dot(l[1] - l[0]) < 0.0)
        })
    }

    /// The drawing's polyline `points` bent: points added along each side first, no further
    /// apart than `step` (in the drawing's own size), so straight lines curve with the line.
    pub fn polyline(&self, points: &[Point], closed: bool, step: f64) -> Vec<Point> {
        let sides = if closed {
            points.len()
        } else {
            points.len().saturating_sub(1)
        };
        let mut out = vec![];
        for i in 0..sides {
            let (a, b) = (points[i], points[(i + 1) % points.len()]);
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a side's point count, small and positive"
            )]
            let n = (a.distance(b) / step).ceil().clamp(1.0, 10_000.0) as usize;
            #[expect(clippy::cast_precision_loss, reason = "a side's point count")]
            out.extend((0..n).map(|k| self.point(a.lerp(b, k as f64 / n as f64))));
        }
        if (!closed || points.len() == 1)
            && let Some(&last) = points.last()
        {
            out.push(self.point(last));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Point, b: Point) {
        assert!(a.distance(b) < 1e-6, "{a} is not {b}");
    }

    fn box_of(w: f64, h: f64) -> Rect {
        Rect::new(Point::new(3.0, 7.0), Point::new(3.0 + w, 7.0 + h))
    }

    /// A ∩ arch: half a circle of `radius` round the origin, from its left end over the top.
    fn arch(radius: f64) -> Vec<Point> {
        (0..=180)
            .map(|i| {
                let a = std::f64::consts::PI * (1.0 - f64::from(i) / 180.0);
                Point::new(a.cos(), a.sin()) * radius
            })
            .collect()
    }

    #[test]
    fn on_a_straight_line_the_drawing_runs_its_length_proportions_kept() {
        let line = [Point::ZERO, Point::new(100.0, 0.0)];
        let bend = Bend::new(box_of(10.0, 5.0), &line, 0.0, Side::Above).unwrap();
        assert!((bend.scale() - 10.0).abs() < 1e-9);
        close(bend.point(Point::new(3.0, 7.0)), Point::ZERO);
        close(bend.point(Point::new(13.0, 7.0)), Point::new(100.0, 0.0));
        close(bend.point(Point::new(3.0, 12.0)), Point::new(0.0, 50.0));
    }

    #[test]
    fn the_gap_keeps_the_drawing_off_both_ends() {
        let line = [Point::ZERO, Point::new(100.0, 0.0)];
        let bend = Bend::new(box_of(10.0, 5.0), &line, 10.0, Side::Above).unwrap();
        close(bend.point(Point::new(3.0, 7.0)), Point::new(10.0, 0.0));
        close(bend.point(Point::new(13.0, 7.0)), Point::new(90.0, 0.0));
    }

    #[test]
    fn a_line_drawn_right_to_left_still_reads_left_to_right_upright() {
        let line = [Point::new(100.0, 0.0), Point::ZERO];
        let bend = Bend::new(box_of(10.0, 5.0), &line, 0.0, Side::Above).unwrap();
        close(bend.point(Point::new(3.0, 7.0)), Point::ZERO);
        close(bend.point(Point::new(3.0, 12.0)), Point::new(0.0, 50.0));
    }

    #[test]
    fn on_an_arch_the_bottom_follows_it_and_the_top_stands_outside() {
        let line = arch(50.0);
        let bend = Bend::new(box_of(20.0, 2.0), &line, 0.0, Side::Above).unwrap();
        let tall = 2.0 * bend.scale();
        for i in 0..=20 {
            let x = 3.0 + f64::from(i);
            let bottom = bend.point(Point::new(x, 7.0)).length();
            let top = bend.point(Point::new(x, 9.0)).length();
            assert!((bottom - 50.0).abs() < 0.05, "bottom at {bottom}");
            assert!((top - (50.0 + tall)).abs() < 0.05, "top at {top}");
        }
        // the middle of the drawing is at the top of the arch
        let mid = bend.point(Point::new(13.0, 7.0));
        assert!(mid.x.abs() < 0.01 && mid.y > 49.9, "{mid}");
        assert!(!bend.folds());
    }

    #[test]
    fn centred_and_below_the_line() {
        let line = arch(50.0);
        let on = Bend::new(box_of(20.0, 2.0), &line, 0.0, Side::On).unwrap();
        let below = Bend::new(box_of(20.0, 2.0), &line, 0.0, Side::Below).unwrap();
        let tall = 2.0 * on.scale();
        let r = |b: &Bend, y: f64| b.point(Point::new(13.0, y)).length();
        assert!((r(&on, 8.0) - 50.0).abs() < 0.05, "the middle on the line");
        assert!((r(&on, 7.0) - (50.0 - tall / 2.0)).abs() < 0.05);
        assert!((r(&below, 9.0) - 50.0).abs() < 0.05, "the top on the line");
        assert!((r(&below, 7.0) - (50.0 - tall)).abs() < 0.05);
    }

    #[test]
    fn a_drawing_taller_than_the_bend_inside_it_folds() {
        // a small arch, a square drawing: below it, the drawing is 157 tall in a 50 radius
        let line = arch(50.0);
        assert!(
            Bend::new(box_of(1.0, 1.0), &line, 0.0, Side::Below)
                .unwrap()
                .folds()
        );
        assert!(
            !Bend::new(box_of(1.0, 1.0), &line, 0.0, Side::Above)
                .unwrap()
                .folds()
        );
    }

    #[test]
    fn straight_sides_curve_with_the_line() {
        let line = arch(50.0);
        let bend = Bend::new(box_of(20.0, 2.0), &line, 0.0, Side::Above).unwrap();
        let bottom = [Point::new(3.0, 7.0), Point::new(23.0, 7.0)];
        let curved = bend.polyline(&bottom, false, 0.1);
        assert!(curved.len() > 100);
        for p in curved {
            assert!((p.length() - 50.0).abs() < 0.05, "{p} is on the arch");
        }
    }

    #[test]
    fn what_cannot_be_bent_says_why() {
        let line = [Point::ZERO, Point::new(100.0, 0.0)];
        assert_eq!(
            Bend::new(box_of(0.0, 5.0), &line, 0.0, Side::Above).unwrap_err(),
            FitError::NoDrawing
        );
        assert_eq!(
            Bend::new(
                box_of(10.0, 5.0),
                &[Point::ZERO, Point::ZERO],
                0.0,
                Side::Above
            )
            .unwrap_err(),
            FitError::NoLine
        );
        assert_eq!(
            Bend::new(box_of(10.0, 5.0), &line, 50.0, Side::Above).unwrap_err(),
            FitError::GapTooBig
        );
        // a flat drawing (a single straight line) bends fine
        assert!(Bend::new(box_of(10.0, 0.0), &line, 0.0, Side::Above).is_ok());
    }
}
