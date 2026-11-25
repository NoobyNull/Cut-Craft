//! Rotary jobs in the 2D view: the sheet is one turn of the material unrolled, so what runs
//! off one seam comes back in at the other. Bands past each seam show that: the drawing near
//! the far seam is drawn again there, faded (a ghost), and a click or a snap on a ghost
//! reaches the real vector it copies.

use crate::options::WrapBands;
use cut_craft_core::document::{Along, Job};
use cut_craft_core::geometry::{Point, Rect};
use cut_craft_core::snap::{Snap, SnapKind};

/// How a rotary job's sheet wraps: where it is, which way is round, one turn's length and
/// how far the bands reach past each seam, all in job millimetres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wrap {
    sheet: Rect,
    /// 0 when X goes round (the material lies along Y), 1 when Y does.
    round: usize,
    turn: f64,
    band: f64,
}

impl Wrap {
    /// The wrap of `job` with bands `bands` reach: none for a flat job, or with the bands
    /// switched off.
    pub fn of(job: &Job, bands: WrapBands) -> Option<Self> {
        let rotary = job.rotary?;
        let sheet = job.area();
        let round = match rotary.along {
            Along::X => 1,
            Along::Y => 0,
        };
        let turn = sheet.size()[round];
        let band = bands.turns() * turn;
        (band > 0.0 && turn > 0.0).then_some(Self {
            sheet,
            round,
            turn,
            band,
        })
    }

    /// Whether the seams run across the view (the material along X), so the bands are
    /// above and below; else they run up and down, with the bands left and right.
    pub fn seams_across(&self) -> bool {
        self.round == 1
    }

    /// `d` millimetres the way round.
    fn round_by(&self, d: f64) -> Point {
        let mut p = Point::ZERO;
        p[self.round] = d;
        p
    }

    /// Where the ghost copies sit from the real drawing: one turn on, and one turn back.
    pub fn shifts(&self) -> [Point; 2] {
        [self.round_by(self.turn), self.round_by(-self.turn)]
    }

    /// The sheet with its two bands: what zoom to fit shows, and where ghosts are drawn.
    pub fn strip(&self) -> Rect {
        let grow = self.round_by(self.band);
        Rect::new(self.sheet.min - grow, self.sheet.max + grow)
    }

    /// The two bands: past the far seam, then past the near one.
    pub fn bands(&self) -> [Rect; 2] {
        let strip = self.strip();
        let mut far_min = self.sheet.min;
        far_min[self.round] = self.sheet.max[self.round];
        let mut near_max = self.sheet.max;
        near_max[self.round] = self.sheet.min[self.round];
        [
            Rect::new(far_min, strip.max),
            Rect::new(strip.min, near_max),
        ]
    }

    /// The lines the view marks across the strip, the way round from the near edge: the
    /// near band's edge, the near seam, the far seam, the far band's edge. Each with the A
    /// angle there (A is 0 where the round coordinate is, as the post winds it).
    pub fn marks(&self) -> [([Point; 2], f64); 4] {
        let (lo, hi) = (self.sheet.min[self.round], self.sheet.max[self.round]);
        let across = 1 - self.round;
        let (a, b) = (self.sheet.min[across], self.sheet.max[across]);
        [lo - self.band, lo, hi, hi + self.band].map(|w| {
            let mut from = Point::ZERO;
            from[self.round] = w;
            from[across] = a;
            let mut to = from;
            to[across] = b;
            ([from, to], w / self.turn * 360.0)
        })
    }

    /// Whether a ghost copy of something with `bounds` lies, shifted by `by`, in the strip.
    pub fn ghost_shows(&self, bounds: Rect, by: Point) -> bool {
        let (lo, hi) = (bounds.min + by, bounds.max + by);
        let strip = self.strip();
        lo.cmple(strip.max).all() && hi.cmpge(strip.min).all()
    }

    /// The real points a click at `at` may stand for: `at` itself, then, inside the strip,
    /// the points the ghosts drawn there copy.
    fn reaches(&self, at: Point) -> Vec<Point> {
        let mut points = vec![at];
        if self.strip().contains(at) {
            points.extend(self.shifts().map(|by| at - by));
        }
        points
    }

    /// What `hit` finds at `at`, or else on the real drawing a ghost at `at` copies.
    pub fn pick<T>(&self, at: Point, hit: impl FnMut(Point) -> Option<T>) -> Option<T> {
        self.reaches(at).into_iter().find_map(hit)
    }

    /// What a point at `at` snaps to, as `find` says (from `from`, the point before): a
    /// point on the drawing, real or a ghost's (the nearest), else whatever `find` gives at
    /// `at` itself (a guide, the grid). A ghost's snap is free: it keeps no hold on the real
    /// vector, which lies a turn away.
    pub fn snap(
        &self,
        at: Point,
        from: Option<Point>,
        find: impl Fn(Option<Point>, Point) -> Option<Snap>,
    ) -> Option<Snap> {
        let own = find(from, at);
        let ghosts = if self.strip().contains(at) {
            self.shifts().to_vec()
        } else {
            vec![]
        };
        let near = ghosts
            .into_iter()
            .filter_map(|by| {
                let s = find(from.map(|f| f - by), at - by)?;
                on_drawing(s.kind).then(|| Snap::free(s.at + by, s.kind))
            })
            .chain(own.filter(|s| on_drawing(s.kind)))
            .min_by(|a, b| a.at.distance(at).total_cmp(&b.at.distance(at)));
        near.or(own)
    }
}

/// Whether a snap is to a point on the drawing, which a ghost copies; guides, the grid and
/// the material's own points are not copied.
fn on_drawing(kind: SnapKind) -> bool {
    matches!(
        kind,
        SnapKind::End
            | SnapKind::Mid
            | SnapKind::ArcCenter
            | SnapKind::ObjectCenter
            | SnapKind::Tangent
            | SnapKind::Perpendicular
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::document::Datum;

    /// A 50 mm round blank 300 mm long along X: the sheet is 300 by one turn (157.08 mm).
    fn along_x() -> Job {
        Job::rotary(300.0, 50.0, Along::X)
    }

    fn turn() -> f64 {
        std::f64::consts::PI * 50.0
    }

    fn close(a: Point, b: Point) -> bool {
        a.distance(b) < 1e-9
    }

    #[test]
    fn flat_jobs_and_bands_switched_off_have_no_wrap() {
        assert_eq!(Wrap::of(&Job::default(), WrapBands::QuarterTurn), None);
        assert_eq!(Wrap::of(&along_x(), WrapBands::Off), None);
    }

    #[test]
    fn a_quarter_turn_band_lies_above_and_below_a_job_along_x() {
        let w = Wrap::of(&along_x(), WrapBands::QuarterTurn).unwrap();
        assert!(w.seams_across());
        let strip = w.strip();
        assert!(close(strip.min, Point::new(0.0, -turn() / 4.0)));
        assert!(close(strip.max, Point::new(300.0, turn() * 1.25)));
        let [far, near] = w.bands();
        assert!(close(far.min, Point::new(0.0, turn())));
        assert!(close(far.max, strip.max));
        assert!(close(near.min, strip.min));
        assert!(close(near.max, Point::new(300.0, 0.0)));
        assert!(close(w.shifts()[0], Point::new(0.0, turn())));
        assert!(close(w.shifts()[1], Point::new(0.0, -turn())));
    }

    #[test]
    fn a_half_turn_band_lies_left_and_right_of_a_job_along_y() {
        let job = Job::rotary(300.0, 50.0, Along::Y);
        let w = Wrap::of(&job, WrapBands::HalfTurn).unwrap();
        assert!(!w.seams_across());
        let strip = w.strip();
        assert!(close(strip.min, Point::new(-turn() / 2.0, 0.0)));
        assert!(close(strip.max, Point::new(turn() * 1.5, 300.0)));
        assert!(close(w.shifts()[0], Point::new(turn(), 0.0)));
    }

    #[test]
    fn the_marks_name_the_a_angle_at_each_seam_and_band_edge() {
        let w = Wrap::of(&along_x(), WrapBands::QuarterTurn).unwrap();
        let angles = w.marks().map(|(_, a)| a.round());
        assert_eq!(angles, [-90.0, 0.0, 360.0, 450.0]);
        let ([from, to], _) = w.marks()[2];
        assert!(close(from, Point::new(0.0, turn())));
        assert!(close(to, Point::new(300.0, turn())));
        // with the datum in the middle, A is 0 across the middle of the sheet
        let mut job = along_x();
        job.datum = Datum::Centre;
        let w = Wrap::of(&job, WrapBands::QuarterTurn).unwrap();
        let angles = w.marks().map(|(_, a)| a.round());
        assert_eq!(angles, [-270.0, -180.0, 180.0, 270.0]);
    }

    #[test]
    fn a_ghost_shows_only_where_its_copy_reaches_the_strip() {
        let w = Wrap::of(&along_x(), WrapBands::QuarterTurn).unwrap();
        let [up, down] = w.shifts();
        // a small circle near the bottom seam: its copy a turn up shows in the top band
        let low = Rect::new(Point::new(10.0, 5.0), Point::new(30.0, 25.0));
        assert!(w.ghost_shows(low, up));
        // its copy a turn down falls far below the bottom band
        assert!(!w.ghost_shows(low, down));
        // something in the middle of the sheet has no ghost at all
        let middle = Rect::new(Point::new(10.0, 60.0), Point::new(30.0, 90.0));
        assert!(!w.ghost_shows(middle, up));
        assert!(!w.ghost_shows(middle, down));
    }

    #[test]
    fn a_click_on_a_ghost_reaches_the_real_drawing_a_turn_away() {
        let w = Wrap::of(&along_x(), WrapBands::QuarterTurn).unwrap();
        let real = Point::new(50.0, 10.0);
        let hit = |p: Point| (p.distance(real) < 1.0).then_some("line");
        assert_eq!(w.pick(real, hit), Some("line"), "the real one, as ever");
        let ghost = real + w.shifts()[0];
        assert_eq!(
            w.pick(ghost, hit),
            Some("line"),
            "its ghost in the top band"
        );
        // outside the strip nothing is drawn, so nothing is reached
        let beyond = real + w.shifts()[0] * 2.0;
        assert_eq!(w.pick(beyond, hit), None);
    }

    #[test]
    fn a_ghost_end_snaps_where_the_ghost_is_and_keeps_no_hold() {
        let w = Wrap::of(&along_x(), WrapBands::QuarterTurn).unwrap();
        let end = Point::new(80.0, 3.0);
        let find = |_: Option<Point>, at: Point| {
            if at.distance(end) < 2.0 {
                Some(Snap {
                    at: end,
                    kind: SnapKind::End,
                    on: None,
                })
            } else {
                Some(Snap::free(at.round(), SnapKind::Grid))
            }
        };
        let ghost = end + w.shifts()[0];
        let s = w.snap(ghost + Point::new(0.5, 0.5), None, find).unwrap();
        assert_eq!(s.kind, SnapKind::End);
        assert!(close(s.at, ghost));
        assert!(s.on.is_none());
        // away from any ghost, the grid as before
        let s = w.snap(Point::new(150.4, 80.2), None, find).unwrap();
        assert_eq!(s.kind, SnapKind::Grid);
    }
}
