//! How long the machine takes to cut a job: every move, rapids and moves between toolpaths
//! included, speeding up and slowing down as the machine's accelerations allow, rounding
//! corners as its controller does, held to each axis's top speed, with tool changes and
//! drill dwells added.
//!
//! The moves are planned the way a motion controller plans them: each move's top speed is
//! its feed (or the rapid speed) held to what every axis it uses can reach; the speed
//! through each corner comes from the controller's junction deviation; then a pass backward
//! and a pass forward keep every move's start and end speeds within what the machine can
//! slow down from and speed up to over its length. Each move then takes the time of its
//! speed-up, cruise and slow-down.

use crate::toolpath::{Move, MoveKind, Toolpath};
use glam::DVec3;

/// What the estimate goes by: the machine's speeds and accelerations, how it corners, and
/// how long a tool change takes. A 0 is not given: no limit for a speed or acceleration.
#[derive(Clone, Debug, PartialEq)]
pub struct Motion {
    /// The rapid (G0) rate, mm/min, used where the machine gives no top speed for an axis.
    pub rapid: f64,
    /// The fastest each axis moves, X, Y, Z, mm/min: rapids run at it and no feed passes it.
    pub max_rate: [f64; 3],
    /// Each axis's acceleration, X, Y, Z, mm/s².
    pub acceleration: [f64; 3],
    /// How far from the sharp corner the controller lets the path stray to keep its speed
    /// through it (junction deviation), mm; 0 for the usual [`JUNCTION_DEVIATION`].
    pub junction_deviation: f64,
    /// How long a tool change takes, seconds: by hand (stop, swap the bit, zero it, start
    /// again), and by the tool changer.
    pub change_by_hand: f64,
    pub change_by_changer: f64,
    /// The bits loaded in the tool changer, by name: changing to one of them is the
    /// changer's quick change; any other bit is changed by hand.
    pub loaded: Vec<String>,
}

/// The usual junction deviation, mm: the common controller default.
pub const JUNCTION_DEVIATION: f64 = 0.01;
/// The usual time for a tool change, seconds: by hand, and by a tool changer (as Options ▸
/// Toolpaths has them to begin with).
pub const CHANGE_BY_HAND: f64 = 120.0;
pub const CHANGE_BY_CHANGER: f64 = 15.0;

impl Default for Motion {
    fn default() -> Self {
        Self {
            rapid: 5000.0,
            max_rate: [0.0; 3],
            acceleration: [0.0; 3],
            junction_deviation: 0.0,
            change_by_hand: CHANGE_BY_HAND,
            change_by_changer: CHANGE_BY_CHANGER,
            loaded: vec![],
        }
    }
}

impl Motion {
    /// The junction deviation used, mm.
    pub fn junction(&self) -> f64 {
        if self.junction_deviation > 0.0 {
            self.junction_deviation
        } else {
            JUNCTION_DEVIATION
        }
    }

    /// How long changing to the bit called `tool` takes, seconds.
    pub fn tool_change(&self, tool: &str) -> f64 {
        if self.loaded.iter().any(|t| t == tool) {
            self.change_by_changer
        } else {
            self.change_by_hand
        }
        .max(0.0)
    }

    /// Whether any acceleration is given: without one, moves start and stop at once.
    pub fn accelerates(&self) -> bool {
        self.acceleration.iter().any(|a| *a > 0.0)
    }

    /// The fastest the machine may go along unit direction `dir`, mm/s, asked for `rate`
    /// (mm/min; `None` for a rapid): no axis may pass its top speed.
    fn top_speed(&self, dir: DVec3, rate: Option<f64>) -> f64 {
        let axes = (0..3)
            .filter(|&k| dir[k].abs() > 1e-12 && self.max_rate[k] > 0.0)
            .map(|k| self.max_rate[k] / dir[k].abs())
            .fold(f64::INFINITY, f64::min);
        let asked = match rate {
            Some(rate) => rate,
            None if axes.is_finite() => axes,
            None => self.rapid,
        };
        asked.min(axes) / 60.0
    }

    /// The most the machine can speed up along unit direction `dir`, mm/s²; infinite when
    /// no axis it uses has an acceleration given.
    fn accel_along(&self, dir: DVec3) -> f64 {
        (0..3)
            .filter(|&k| dir[k].abs() > 1e-12 && self.acceleration[k] > 0.0)
            .map(|k| self.acceleration[k] / dir[k].abs())
            .fold(f64::INFINITY, f64::min)
    }
}

/// Where a job's time goes, seconds.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Breakdown {
    /// Cutting and plunging at the feed and plunge rates.
    pub cutting: f64,
    /// Rapids within the toolpath, and to its start from where the last one ended.
    pub rapids: f64,
    /// Waiting at the bottom of drill pecks.
    pub dwells: f64,
    /// Changing the tool before it.
    pub tool_change: f64,
}

impl Breakdown {
    pub fn total(&self) -> f64 {
        self.cutting + self.rapids + self.dwells + self.tool_change
    }

    fn add(&mut self, other: &Self) {
        self.cutting += other.cutting;
        self.rapids += other.rapids;
        self.dwells += other.dwells;
        self.tool_change += other.tool_change;
    }
}

/// A whole job's time: each toolpath's, in the order cut, and the trip back home after
/// the last one.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct JobTime {
    pub toolpaths: Vec<Breakdown>,
    /// The rapid from the last toolpath's end back to the job's home, seconds.
    pub home_again: f64,
}

impl JobTime {
    /// Every toolpath's time added up, and the trip home.
    pub fn total(&self) -> Breakdown {
        let mut all = Breakdown::default();
        for t in &self.toolpaths {
            all.add(t);
        }
        all.rapids += self.home_again;
        all
    }
}

/// How long `toolpaths` take, cut one after another in the order given, starting from
/// `home` and going back to it at the end, as the saved G-code does.
pub fn job(toolpaths: &[&Toolpath], home: DVec3, motion: &Motion) -> JobTime {
    let mut at = home;
    let mut previous: Option<&str> = None;
    let mut times = Vec::with_capacity(toolpaths.len());
    for toolpath in toolpaths {
        let mut time = toolpath_from(toolpath, Some(at), motion);
        if previous.is_some_and(|name| name != toolpath.tool.name) {
            time.tool_change = motion.tool_change(&toolpath.tool.name);
        }
        previous = Some(&toolpath.tool.name);
        if let Some(last) = toolpath.moves.last() {
            at = last.to;
        }
        times.push(time);
    }
    let back = Leg::new(
        at,
        Move {
            kind: MoveKind::Rapid,
            to: home,
        },
    );
    let home_again = plan(back.as_slice(), motion).rapids;
    JobTime {
        toolpaths: times,
        home_again,
    }
}

/// How long `toolpath` takes on its own, from its first move to its last.
pub fn toolpath(toolpath: &Toolpath, motion: &Motion) -> Breakdown {
    toolpath_from(toolpath, None, motion)
}

/// How long `toolpath` takes, starting with a rapid from `from` to its first move when
/// given.
fn toolpath_from(toolpath: &Toolpath, from: Option<DVec3>, motion: &Motion) -> Breakdown {
    let legs: Vec<Leg> = legs(toolpath, from).into_iter().map(|(_, l)| l).collect();
    plan(&legs, motion)
}

/// How long each of `toolpath`'s moves takes, seconds, planned together from a standstill
/// at its first move: one for each move after the first (the move to it, its dwell
/// included), 0 for a move that goes nowhere. The preview plays the moves at these times.
pub fn each_move(toolpath: &Toolpath, motion: &Motion) -> Vec<f64> {
    let legs = legs(toolpath, None);
    let only: Vec<Leg> = legs.iter().map(|(_, l)| *l).collect();
    let mut seconds = vec![0.0; toolpath.moves.len().saturating_sub(1)];
    for ((i, leg), time) in legs.iter().zip(plan_each(&only, motion)) {
        seconds[i - 1] = time + leg.dwell;
    }
    seconds
}

/// `toolpath`'s moves as legs, each with the index of the move it goes to, starting with a
/// rapid from `from` to its first move when given.
fn legs(toolpath: &Toolpath, from: Option<DVec3>) -> Vec<(usize, Leg)> {
    let dwell = toolpath
        .params
        .get("dwell")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0)
        .max(0.0);
    // a bit with no rate set moves at the rapid speed
    let rate = |kind| {
        match kind {
            MoveKind::Rapid => None,
            MoveKind::Plunge => Some(toolpath.tool.plunge_rate),
            MoveKind::Cut => Some(toolpath.tool.feed_rate),
        }
        .filter(|r| *r > 0.0)
    };
    let mut legs = Vec::with_capacity(toolpath.moves.len());
    let mut previous = from;
    for (i, m) in toolpath.moves.iter().enumerate() {
        // the first move is where the toolpath starts: reached by a rapid from `from`
        let m = if i == 0 {
            Move {
                kind: MoveKind::Rapid,
                ..*m
            }
        } else {
            *m
        };
        if let Some(start) = previous
            && let Some(mut leg) = Leg::new(start, m)
        {
            leg.rate = rate(m.kind);
            // a drill waits at the bottom of each plunge: it stops there
            leg.dwell = if m.kind == MoveKind::Plunge {
                dwell
            } else {
                0.0
            };
            legs.push((i, leg));
        }
        previous = Some(m.to);
    }
    legs
}

/// One straight move: its direction (unit), length, mm, the rate asked for (mm/min;
/// `None` for a rapid) and the wait at its end, seconds.
#[derive(Clone, Copy, Debug)]
struct Leg {
    dir: DVec3,
    length: f64,
    rate: Option<f64>,
    dwell: f64,
}

impl Leg {
    /// The move from `from` to `m`, a rapid until told its rate; `None` when it goes
    /// nowhere.
    fn new(from: DVec3, m: Move) -> Option<Self> {
        let d = m.to - from;
        let length = d.length();
        (length > 1e-9).then(|| Self {
            dir: d / length,
            length,
            rate: None,
            dwell: 0.0,
        })
    }
}

/// The time `legs` take one after another, from a standstill to a standstill.
fn plan(legs: &[Leg], motion: &Motion) -> Breakdown {
    let mut time = Breakdown::default();
    for (leg, seconds) in legs.iter().zip(plan_each(legs, motion)) {
        if leg.rate.is_some() {
            time.cutting += seconds;
        } else {
            time.rapids += seconds;
        }
        time.dwells += leg.dwell;
    }
    time
}

/// Each of `legs`' time, seconds, moving (dwells left out), one after another from a
/// standstill to a standstill.
fn plan_each(legs: &[Leg], motion: &Motion) -> Vec<f64> {
    let n = legs.len();
    if n == 0 {
        return vec![];
    }
    // each leg's top speed (mm/s) and acceleration (mm/s²)
    let top: Vec<f64> = legs
        .iter()
        .map(|l| motion.top_speed(l.dir, l.rate))
        .collect();
    let accel: Vec<f64> = legs.iter().map(|l| motion.accel_along(l.dir)).collect();
    // the fastest each leg may start at: the speed through the corner before it
    let mut start = vec![0.0; n + 1];
    for i in 1..n {
        start[i] = if legs[i - 1].dwell > 0.0 {
            0.0
        } else {
            corner(legs[i - 1].dir, legs[i].dir, motion)
                .min(top[i - 1])
                .min(top[i])
        };
    }
    // backward: each leg must be able to slow to the next one's start
    for i in (0..n).rev() {
        let reach = (start[i + 1] * start[i + 1] + 2.0 * accel[i] * legs[i].length).sqrt();
        start[i] = start[i].min(reach);
    }
    // forward: each leg can only speed up so much over its length
    for i in 0..n {
        let reach = (start[i] * start[i] + 2.0 * accel[i] * legs[i].length).sqrt();
        start[i + 1] = start[i + 1].min(reach);
    }
    legs.iter()
        .enumerate()
        .map(|(i, leg)| {
            let (v, a, l) = (top[i], accel[i], leg.length);
            if v <= 0.0 {
                0.0
            } else if !a.is_finite() {
                l / v
            } else {
                let (vs, ve) = (start[i].min(v), start[i + 1].min(v));
                let (up, down) = ((v * v - vs * vs) / (2.0 * a), (v * v - ve * ve) / (2.0 * a));
                if up + down <= l {
                    (v - vs) / a + (v - ve) / a + (l - up - down) / v
                } else {
                    // too short to reach its speed: up to a peak and down again
                    let peak = ((2.0 * a * l + vs * vs + ve * ve) / 2.0).sqrt();
                    (peak - vs) / a + (peak - ve) / a
                }
            }
        })
        .collect()
}

/// The fastest the machine can go through the corner from unit direction `a` into `b`,
/// mm/s, by the junction deviation: the speed at which turning along a circle that strays
/// the junction deviation from the corner takes no more than the machine's acceleration.
fn corner(a: DVec3, b: DVec3, motion: &Motion) -> f64 {
    let cos = -a.dot(b);
    if cos > 0.999_999 {
        // straight back the way it came: a full stop
        return 0.0;
    }
    if cos < -0.999_999 {
        // straight on
        return f64::INFINITY;
    }
    let across = b - a;
    let accel = motion.accel_along(across.normalize());
    if !accel.is_finite() {
        return f64::INFINITY;
    }
    let sin_half = (0.5 * (1.0 - cos)).sqrt();
    (accel * motion.junction() * sin_half / (1.0 - sin_half)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toolpath::Tool;

    fn toolpath(moves: &[(MoveKind, [f64; 3])]) -> Toolpath {
        Toolpath {
            name: "t".into(),
            strategy: String::new(),
            params: serde_json::Value::Null,
            tool: Tool {
                name: "6 mm".into(),
                feed_rate: 1200.0,
                plunge_rate: 300.0,
                ..Tool::default()
            },
            vectors: vec![],
            moves: moves
                .iter()
                .map(|&(kind, [x, y, z])| Move {
                    kind,
                    to: DVec3::new(x, y, z),
                })
                .collect(),
            visible: true,
            power: vec![],
            stale: false,
            group: None,
            drill_bank: None,
            fallbacks: vec![],
        }
    }

    fn router() -> Motion {
        Motion {
            rapid: 5000.0,
            max_rate: [6000.0, 6000.0, 1200.0],
            acceleration: [500.0, 500.0, 100.0],
            ..Motion::default()
        }
    }

    const C: MoveKind = MoveKind::Cut;
    const R: MoveKind = MoveKind::Rapid;
    const P: MoveKind = MoveKind::Plunge;

    #[test]
    fn without_accelerations_each_move_is_its_length_at_its_rate() {
        let t = toolpath(&[
            (R, [0.0, 0.0, 5.0]),
            (P, [0.0, 0.0, -3.0]),
            (C, [120.0, 0.0, -3.0]),
        ]);
        let flat = Motion::default();
        let time = super::toolpath(&t, &flat);
        // 8 mm at 300 mm/min and 120 mm at 1200 mm/min
        assert!((time.cutting - (8.0 / 5.0 + 6.0)).abs() < 1e-9, "{time:?}");
        assert!(time.rapids.abs() < 1e-12);
    }

    #[test]
    fn speeding_up_and_slowing_down_add_time_to_a_move() {
        let t = toolpath(&[(C, [0.0, 0.0, 0.0]), (C, [100.0, 0.0, 0.0])]);
        let flat = super::toolpath(&t, &Motion::default()).total();
        let real = super::toolpath(&t, &router()).total();
        // 20 mm/s, 500 mm/s²: 0.04 s up and 0.04 s down, losing 0.04 s against the ideal
        assert!((real - (flat + 0.04)).abs() < 1e-9, "{real} against {flat}");
    }

    #[test]
    fn a_short_move_never_reaches_its_feed() {
        let t = toolpath(&[(C, [0.0, 0.0, 0.0]), (C, [0.1, 0.0, 0.0])]);
        let time = super::toolpath(&t, &router()).total();
        // up for 0.05 mm and down again: 2 * sqrt(0.05 / 250)
        assert!(
            (time - 2.0 * (0.05_f64 / 250.0).sqrt()).abs() < 1e-9,
            "{time}"
        );
    }

    #[test]
    fn many_small_moves_round_a_curve_without_stopping_at_each() {
        // a circle of 50 mm radius in 360 one-degree pieces
        let mut moves = vec![(C, [50.0, 0.0, 0.0])];
        for i in 1..=360 {
            let a = f64::from(i).to_radians();
            moves.push((C, [50.0 * a.cos(), 50.0 * a.sin(), 0.0]));
        }
        let t = toolpath(&moves);
        let time = super::toolpath(&t, &router()).total();
        let ideal = 2.0 * std::f64::consts::PI * 50.0 / 20.0;
        assert!(time < ideal * 1.05, "{time} s against {ideal} s");
        assert!(time > ideal, "{time} s against {ideal} s");
    }

    #[test]
    fn a_sharp_corner_slows_the_machine_more_than_a_gentle_one() {
        let sharp = toolpath(&[
            (C, [0.0, 0.0, 0.0]),
            (C, [50.0, 0.0, 0.0]),
            (C, [0.0, 1.0, 0.0]),
        ]);
        let gentle = toolpath(&[
            (C, [0.0, 0.0, 0.0]),
            (C, [50.0, 0.0, 0.0]),
            (C, [100.0, 1.0, 0.0]),
        ]);
        let m = router();
        let (s, g) = (
            super::toolpath(&sharp, &m).total(),
            super::toolpath(&gentle, &m).total(),
        );
        let (ls, lg) = (50.0 + 50.01_f64, 50.0 + 50.01_f64);
        assert!(s - ls / 20.0 > g - lg / 20.0, "sharp {s}, gentle {g}");
        // a looser corner keeps more speed
        let loose = Motion {
            junction_deviation: 0.5,
            ..m
        };
        assert!(super::toolpath(&sharp, &loose).total() < s);
    }

    #[test]
    fn rapids_run_at_each_axis_top_speed() {
        let t = toolpath(&[
            (R, [0.0, 0.0, 0.0]),
            (R, [600.0, 0.0, 0.0]),
            (R, [600.0, 0.0, 60.0]),
        ]);
        let fast = Motion {
            max_rate: [6000.0, 6000.0, 1200.0],
            ..Motion::default()
        };
        let time = super::toolpath(&t, &fast);
        // 600 mm at 100 mm/s, then 60 mm at 20 mm/s
        assert!((time.rapids - 9.0).abs() < 1e-9, "{time:?}");
        // without top speeds, the rapid rate
        let rapid = super::toolpath(&t, &Motion::default());
        assert!(
            (rapid.rapids - 660.0 / (5000.0 / 60.0)).abs() < 1e-9,
            "{rapid:?}"
        );
    }

    #[test]
    fn no_feed_passes_the_axis_top_speed() {
        let t = toolpath(&[(P, [0.0, 0.0, 0.0]), (P, [0.0, 0.0, -10.0])]);
        let slow_z = Motion {
            max_rate: [6000.0, 6000.0, 150.0],
            ..Motion::default()
        };
        // plunging at 300 mm/min asked, but Z goes no faster than 150
        assert!((super::toolpath(&t, &slow_z).cutting - 4.0).abs() < 1e-9);
    }

    #[test]
    fn the_job_goes_from_home_between_toolpaths_and_back_home() {
        let a = toolpath(&[(R, [100.0, 0.0, 5.0]), (C, [200.0, 0.0, 5.0])]);
        let b = toolpath(&[(R, [200.0, 100.0, 5.0]), (C, [300.0, 100.0, 5.0])]);
        let m = Motion::default();
        let job = super::job(&[&a, &b], DVec3::new(0.0, 0.0, 5.0), &m);
        let per_s = 5000.0 / 60.0;
        assert!(
            (job.toolpaths[0].rapids - 100.0 / per_s).abs() < 1e-9,
            "{job:?}"
        );
        assert!(
            (job.toolpaths[1].rapids - 100.0 / per_s).abs() < 1e-9,
            "{job:?}"
        );
        let back = (300.0_f64 * 300.0 + 100.0 * 100.0).sqrt() / per_s;
        assert!((job.home_again - back).abs() < 1e-9, "{job:?}");
        assert!(
            job.toolpaths.iter().all(|t| t.tool_change == 0.0),
            "the same bit: no change"
        );
    }

    #[test]
    fn a_new_bit_adds_a_tool_change() {
        let a = toolpath(&[(C, [0.0, 0.0, 0.0])]);
        let mut b = toolpath(&[(C, [0.0, 0.0, 0.0])]);
        b.tool.name = "V-bit".into();
        let by_hand = super::job(&[&a, &b, &b], DVec3::ZERO, &Motion::default());
        let changes: Vec<f64> = by_hand.toolpaths.iter().map(|t| t.tool_change).collect();
        assert_eq!(changes, [0.0, CHANGE_BY_HAND, 0.0]);
        // a bit in the tool changer is its quick change
        let with_changer = Motion {
            loaded: vec!["V-bit".into()],
            ..Motion::default()
        };
        let quick = super::job(&[&a, &b, &a], DVec3::ZERO, &with_changer);
        let changes: Vec<f64> = quick.toolpaths.iter().map(|t| t.tool_change).collect();
        assert_eq!(changes, [0.0, CHANGE_BY_CHANGER, CHANGE_BY_HAND]);
    }

    #[test]
    fn each_move_s_time_adds_up_to_the_toolpath_s() {
        let mut moves = vec![
            (R, [0.0, 0.0, 5.0]),
            (P, [0.0, 0.0, -2.0]),
            (P, [0.0, 0.0, -2.0]),
        ];
        for i in 1..=90 {
            let a = f64::from(i).to_radians();
            moves.push((C, [30.0 * a.cos() - 30.0, 30.0 * a.sin(), -2.0]));
        }
        let t = toolpath(&moves);
        let each = each_move(&t, &router());
        assert_eq!(each.len(), t.moves.len() - 1);
        assert_eq!(each[1], 0.0, "a move that goes nowhere takes no time");
        let sum: f64 = each.iter().sum();
        assert!((sum - super::toolpath(&t, &router()).total()).abs() < 1e-9);
    }

    #[test]
    fn a_dwelling_drill_waits_and_stops_at_the_bottom_of_each_peck() {
        let mut t = toolpath(&[
            (R, [0.0, 0.0, 5.0]),
            (P, [0.0, 0.0, -5.0]),
            (R, [0.0, 0.0, 5.0]),
            (P, [0.0, 0.0, -5.0]),
        ]);
        t.params = serde_json::json!({ "dwell": 0.5 });
        let time = super::toolpath(&t, &router());
        assert!((time.dwells - 1.0).abs() < 1e-12, "{time:?}");
    }
}
