//! How a rotary file winds A: which whole turn each pass is cut on, so A goes no further than
//! the machine allows ("A can turn" in Machine Configuration), passes cut back the other way
//! or after a spin back at the limit, backlash taken up on each change of direction, and
//! inverse-time feeds that keep the bit's real speed at the depth it cuts.
//!
//! The wrapped axis is kept flat here (millimetres unrolled round the material's surface): a
//! whole turn of A is one circumference, so a pass moved by whole circumferences cuts the
//! same place on the material with A turned whole turns further.

use crate::Step;
use cut_craft_core::glam::DVec3;
use cut_craft_core::rotary_axis::{AtLimit, RotaryAxis, Turning};
use cut_craft_core::toolpath::MoveKind;

/// A rotary file's winding as it is written: the machine's rotary and where A has got to.
#[derive(Clone, Debug)]
pub(crate) struct Winding {
    axis: RotaryAxis,
    /// The wrapped axis: 0 for X, 1 for Y.
    round: usize,
    /// The material's radius, mm.
    radius: f64,
    /// Flat millimetres a degree.
    per_degree: f64,
    /// The bit's height clear of the material (the job's clearance, from its top).
    clearance: f64,
    /// Whole turns the next passes are moved by, so far.
    turns: f64,
    /// The way A last turned (+1 or -1), for backlash.
    going: f64,
    /// A as last written, before backlash.
    last: Option<f64>,
}

impl Winding {
    /// For a cylinder `diameter` across whose round side is X (`round` 0) or Y (1).
    pub(crate) fn new(axis: RotaryAxis, round: usize, diameter: f64, clearance: f64) -> Self {
        Self {
            axis,
            round,
            radius: diameter / 2.0,
            per_degree: std::f64::consts::PI * diameter / 360.0,
            clearance,
            turns: 0.0,
            going: 1.0,
            last: None,
        }
    }

    fn turn(&self) -> f64 {
        self.per_degree * 360.0
    }

    /// The flat distance round the material that A may reach either way from 0.
    fn limit(&self) -> Option<f64> {
        self.axis.limit().map(|degrees| degrees * self.per_degree)
    }

    /// `steps` as written, starting `at` (as written): each pass moved whole turns, and
    /// turned round where "A can turn" says, with the moves between passes lifting clear
    /// before A spins.
    pub(crate) fn plan(&mut self, steps: Vec<Step>, mut at: DVec3) -> Vec<Step> {
        if steps.iter().any(|s| matches!(s, Step::Arc(..))) {
            // wrapped posts write no arcs; leave anything else as it is
            return steps;
        }
        let lines: Vec<(MoveKind, DVec3, f64)> = steps
            .into_iter()
            .map(|s| match s {
                Step::Line(kind, to, power) => (kind, to, power),
                Step::Arc(..) => unreachable!(),
            })
            .collect();
        let mut out = Vec::with_capacity(lines.len() + 8);
        let mut i = 0;
        while i < lines.len() {
            let rapids_end = i + lines[i..]
                .iter()
                .take_while(|l| l.0 == MoveKind::Rapid)
                .count();
            let pass_end = rapids_end
                + lines[rapids_end..]
                    .iter()
                    .take_while(|l| l.0 != MoveKind::Rapid)
                    .count();
            let rapids = &lines[i..rapids_end];
            let pass = &lines[rapids_end..pass_end];
            if pass.is_empty() || rapids.is_empty() {
                // a last lift, or cutting straight on from where the file starts: as it is
                for l in &lines[i..pass_end] {
                    let to = self.moved(l.1, self.turns);
                    out.push(Step::Line(l.0, to, l.2));
                    at = to;
                }
                i = pass_end;
                continue;
            }
            let approach = rapids[rapids.len() - 1].1;
            let (turns, backwards) = self.choose(at, approach, pass);
            if turns == self.turns && !backwards {
                for l in rapids.iter().chain(pass) {
                    let to = self.moved(l.1, turns);
                    out.push(Step::Line(l.0, to, l.2));
                    at = to;
                }
            } else {
                self.turns = turns;
                let start = if backwards {
                    pass[pass.len() - 1].1
                } else {
                    pass[0].1
                };
                let above = self.moved(DVec3::new(start.x, start.y, approach.z), turns);
                // lift straight up, turn at the clearance, then down as the file would
                let high = self.clearance.max(approach.z);
                if at.z < high - 1e-9 {
                    at.z = high;
                    out.push(Step::Line(MoveKind::Rapid, at, 0.0));
                }
                let over = DVec3::new(above.x, above.y, at.z);
                out.push(Step::Line(MoveKind::Rapid, over, 0.0));
                if (over.z - above.z).abs() > 1e-9 {
                    out.push(Step::Line(MoveKind::Rapid, above, 0.0));
                }
                if backwards {
                    out.push(Step::Line(
                        MoveKind::Plunge,
                        self.moved(start, turns),
                        pass[pass.len() - 1].2,
                    ));
                    for k in (1..pass.len()).rev() {
                        let (kind, _, power) = pass[k];
                        out.push(Step::Line(kind, self.moved(pass[k - 1].1, turns), power));
                    }
                } else {
                    for l in pass {
                        out.push(Step::Line(l.0, self.moved(l.1, turns), l.2));
                    }
                }
                if let Some(Step::Line(_, to, _)) = out.last() {
                    at = *to;
                }
            }
            i = pass_end;
        }
        out
    }

    /// `p` moved `turns` whole turns round.
    fn moved(&self, mut p: DVec3, turns: f64) -> DVec3 {
        p[self.round] += turns * self.turn();
        p
    }

    /// Which whole turn a `pass` reached from `approach` is cut on, from `at`, and whether it
    /// is cut backwards.
    fn choose(&self, at: DVec3, approach: DVec3, pass: &[(MoveKind, DVec3, f64)]) -> (f64, bool) {
        let round = self.round;
        // the whole turns that bring `start` nearest to where A is
        let nearest = |start: f64| ((at[round] - start) / self.turn()).round();
        let (low, high) = pass.iter().fold((f64::MAX, f64::MIN), |(lo, hi), l| {
            (lo.min(l.1[round]), hi.max(l.1[round]))
        });
        let fits = |turns: f64| {
            self.limit().is_none_or(|limit| {
                let shift = turns * self.turn();
                low + shift >= -limit - 1e-6 && high + shift <= limit + 1e-6
            })
        };
        let forward = nearest(pass[0].1[round]);
        if self.axis.turning == Turning::Free || fits(forward) {
            return (forward, false);
        }
        if self.axis.at_limit == AtLimit::CutBothWays && reversible(approach, pass) {
            let back = nearest(pass[pass.len() - 1].1[round]);
            if fits(back) {
                return (back, true);
            }
        }
        // spin back: the whole turns that bring the pass nearest the middle, and of two
        // as near, the one A spins back least to reach
        let middle = -(f64::midpoint(low, high) / self.turn()).round();
        let off_centre = |turns: f64| (f64::midpoint(low, high) + turns * self.turn()).abs();
        let spin = |turns: f64| (pass[0].1[round] + turns * self.turn() - at[round]).abs();
        [middle - 1.0, middle, middle + 1.0]
            .into_iter()
            .filter(|&t| fits(t))
            .min_by(|&a, &b| {
                if (off_centre(a) - off_centre(b)).abs() > 1e-6 {
                    off_centre(a).total_cmp(&off_centre(b))
                } else {
                    spin(a).total_cmp(&spin(b))
                }
            })
            .map_or((middle, false), |t| (t, false))
    }

    /// A for the wrapped axis at flat `w`, in degrees, with the backlash taken up when A
    /// turns the other way from last time.
    pub(crate) fn degrees(&mut self, w: f64) -> f64 {
        let a = w / self.per_degree;
        if let Some(last) = self.last {
            let d = a - last;
            if d.abs() > 1e-9 {
                self.going = d.signum();
            }
        }
        self.last = Some(a);
        if self.going < 0.0 {
            a - self.axis.backlash
        } else {
            a
        }
    }

    /// Inverse-time feed (one over the minutes it takes) for a move `from`→`to` at `rate`:
    /// its length measured where the bit is, nearer the axis than the surface when it cuts
    /// deep, and no faster than A's top speed.
    pub(crate) fn inverse_time(&self, from: DVec3, to: DVec3, rate: f64) -> f64 {
        let d = to - from;
        let depth = f64::midpoint(from.z, to.z);
        let r = (self.radius + depth).max(self.radius * 0.01);
        let round = d[self.round] * r / self.radius;
        let along = d[1 - self.round];
        let length = (round * round + along * along + d.z * d.z).sqrt();
        let mut minutes = length / rate;
        let degrees = (d[self.round] / self.per_degree).abs();
        if self.axis.max_rate > 0.0 {
            minutes = minutes.max(degrees / self.axis.max_rate);
        }
        if minutes > 0.0 { 1.0 / minutes } else { 0.0 }
    }

    /// Where A goes home at the end of the file, flat: the very start after a spin back to
    /// it, otherwise the start's place on the material nearest where A is.
    pub(crate) fn home(&self, home: f64, at: f64) -> f64 {
        if self.axis.turning == Turning::Unwind && self.axis.at_limit == AtLimit::SpinBack {
            home
        } else {
            home + ((at - home) / self.turn()).round() * self.turn()
        }
    }
}

/// A pass can be cut backwards when it goes straight down into the material and never cuts
/// upwards after: backwards it then goes straight down at its far end, and never cuts down
/// sideways.
fn reversible(approach: DVec3, pass: &[(MoveKind, DVec3, f64)]) -> bool {
    let (kind, first, _) = pass[0];
    kind == MoveKind::Plunge
        && (first.x - approach.x).abs() < 1e-6
        && (first.y - approach.y).abs() < 1e-6
        && pass.windows(2).all(|w| w[1].1.z <= w[0].1.z + 1e-6)
}
