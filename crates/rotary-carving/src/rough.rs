//! Rotary Roughing's passes: the bulk taken off the blank a level at a time from the outside
//! in, all the way round at each level before the next, down to the allowance above the
//! model. Along the part, the bit runs the whole length with A held still, then A turns a
//! step (like a typewriter); round the part, A turns the bit all the way round, then it steps
//! along. Only what is left of the material is cut, and the collet nut and spindle are kept
//! clear of the material beside the cut and of the chuck and tailstock: where they would not
//! let the bit down it stops short, and that is said.

use cut_craft_core::CommandError;
use cut_craft_core::glam::DVec3;
use cut_craft_core::progress;
use cut_craft_core::work::parallel;
use cut_craft_rotary::{Head, Voxels};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{Check, GRAZE, Pass, Work, reach_along};

/// Which way the bit goes over a carved part.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Way {
    /// The bit runs the whole length of the part with A held still, then A turns a step and
    /// it runs back: best for clearing the bulk, and for long parts
    #[default]
    AlongThePart,
    /// A turns the bit all the way round the part, then it steps along: best for finishing
    /// round shapes, and for short parts
    RoundThePart,
}

/// Which way the bit's edge meets the material, the spindle turning clockwise seen from
/// above.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CutDirection {
    /// Every pass the same way, the edge biting into the material as it comes to it: a
    /// cleaner cut on most woods, but it lifts the bit back to start each pass
    Climb,
    /// Every pass the same way, the edge cutting up out of the material: gentler on a light
    /// machine with play in it, lifting back the same way
    Conventional,
    /// Back and forth, staying down between passes: the quickest
    #[default]
    Both,
}

/// How the levels are to be cut.
pub struct Levels {
    pub way: Way,
    pub direction: CutDirection,
    /// How far down each level goes, and how far apart the passes are at the outside.
    pub depth: f64,
    pub step: f64,
    /// The holder, chuck and tailstock to keep clear of.
    pub head: Head,
}

/// What roughing worked out: the passes, each with whether the bit can come down to it at
/// the rapid rate, and where along the part the holder kept the bit up, by how much.
#[derive(Debug, Default)]
pub struct Roughed {
    pub passes: Vec<(Pass, bool)>,
    pub short: Vec<(f64, f64)>,
}

/// Whether a pass run the way along or round the part the sheet counts up climbs, the next
/// pass stepping on the way the sheet counts up too: the edge to the left of the way the bit
/// moves comes forward, with the spindle turning clockwise seen from above.
#[must_use]
pub fn forward_climbs(way: Way, along_x: bool) -> bool {
    // along X the sheet's X is along and Y round; along Y the other way about
    match way {
        Way::AlongThePart => along_x,
        Way::RoundThePart => !along_x,
    }
}

/// The grid the levels are cut on: lines (along the part at each A, or round it at each step
/// along), each of points; and the lowest the bit may go at each.
struct Grid {
    lines: Vec<Vec<(f64, f64)>>,
    floors: Vec<Vec<f64>>,
}

impl Work<'_> {
    /// Rough what is left of the `material` as `levels` says, down to the allowance above
    /// the model; what it cuts is cut from the `material` as it goes.
    ///
    /// # Errors
    /// When it is stopped.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "line and point counts are small"
    )]
    pub fn roughing(
        &self,
        material: &mut Voxels,
        levels: &Levels,
    ) -> Result<Roughed, CommandError> {
        let radius = self.turned.radius;
        let cell = material.cell();
        let check = Check::new(self.tool, levels.depth + cell, 0.0, 1, cell);
        let swing = material.swing();
        progress::stage("Finding how low the bit may go");
        let grid = self.grid(levels, swing);
        let lowest = grid
            .floors
            .iter()
            .flatten()
            .copied()
            .fold(f64::INFINITY, f64::min);
        // what is left, as far out as it reaches along the part, for the holder
        let bin = (self.tool.diameter / 4.0).max(cell);
        let bins = (self.end / bin).ceil() as usize + 2;
        let mut left = reach_along(material, bin, bins);
        let at = |left: &[f64], along: f64| {
            let i = (along / bin).floor();
            if i < 0.0 {
                0.0
            } else {
                left.get(i as usize).copied().unwrap_or(0.0)
            }
        };
        let room_step = bin.max(1.0);
        let head = &levels.head;
        let forward = forward_climbs(levels.way, self.along_x);
        let one_way = match levels.direction {
            CutDirection::Both => None,
            CutDirection::Climb => Some(forward),
            CutDirection::Conventional => Some(!forward),
        };
        let mut out = Roughed::default();
        let mut level = swing;
        let mut k = 0;
        let steps = ((swing - lowest) / levels.depth).ceil().max(1.0) as usize;
        progress::stage("Clearing the bulk a level at a time");
        while level > lowest + 1e-9 {
            progress::stop_if_cancelled()?;
            k += 1;
            level = (swing - k as f64 * levels.depth).max(lowest);
            // how low the holder lets the bit go at each point along, as the levels before
            // left the material
            let room = |along: f64| head.lowest_over(along, |a| at(&left, a), room_step);
            let mut heights: Vec<Vec<f64>> = Vec::with_capacity(grid.lines.len());
            let mut passes: Vec<Pass> = vec![];
            let mut pass: Pass = vec![];
            for (l, line) in grid.lines.iter().enumerate() {
                let mut hs = Vec::with_capacity(line.len());
                let mut points: Pass = Vec::with_capacity(line.len());
                for (&(along, round), &floor) in line.iter().zip(&grid.floors[l]) {
                    let goal = level.max(floor);
                    let lets = room(along);
                    let h = goal.max(lets).min(self.top);
                    let point = (self.point(along, round, h - radius), 0.0);
                    // outside the machining limit nothing is cut: the material there still
                    // reaches as far out as it did, for the holder, and nothing was cut short
                    if self
                        .limit
                        .is_some_and(|limit| !limit.holds_point(self.along_x, &point))
                    {
                        hs.push(f64::INFINITY);
                    } else {
                        if lets > goal + 1e-6 && goal < at(&left, along) - GRAZE {
                            out.short.push((along, lets - goal));
                        }
                        hs.push(h);
                    }
                    points.push(point);
                }
                heights.push(hs);
                let backwards = match one_way {
                    Some(forward) => !forward,
                    None => l % 2 == 1,
                };
                if backwards {
                    points.reverse();
                }
                if one_way.is_some() {
                    passes.push(points);
                } else {
                    // over to this line from where the last ended, staying down
                    if let (Some(&(from, _)), Some(&(to, _))) = (pass.last(), points.first()) {
                        pass.extend(self.over(from, to, level, &room));
                    }
                    pass.extend(points);
                }
            }
            if !pass.is_empty() {
                passes.push(pass);
            }
            for p in &passes {
                out.passes
                    .extend(self.cutting(p, material, &check, None, &mut vec![]));
            }
            // the material left reaches no further out than this level cut it to
            self.lower(&mut left, bin, &grid, &heights, levels.way);
            progress::report_steps(k.min(steps), steps);
        }
        Ok(out)
    }

    /// The lines of the grid the levels are cut on, and the lowest the bit may go at each
    /// point of them: the model and allowance under the bit, and under it anywhere on the way
    /// to the next point, so the straight moves between never dip into the model.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "line and point counts are small"
    )]
    fn grid(&self, levels: &Levels, swing: f64) -> Grid {
        let bit = self.tool.diameter;
        let radius = self.turned.radius;
        let round_all = self.circumference;
        let length = self.end - self.start;
        let along_at = |count: usize, k: usize| {
            if count <= 1 {
                self.start
            } else {
                self.start + length * k as f64 / (count - 1) as f64
            }
        };
        let lines: Vec<Vec<(f64, f64)>> = match levels.way {
            Way::AlongThePart => {
                // A steps no further apart than the step at the outside of the blank
                let n = ((std::f64::consts::TAU * swing / levels.step).ceil() as usize).max(8);
                let cell = (bit / 4.0).max(self.turned.along_cell);
                let points = ((length / cell).ceil() as usize).max(1) + 1;
                (0..n)
                    .map(|j| {
                        let round = round_all * j as f64 / n as f64;
                        (0..points).map(|k| (along_at(points, k), round)).collect()
                    })
                    .collect()
            }
            Way::RoundThePart => {
                let rings = ((length / levels.step).ceil() as usize).max(1) + 1;
                // points round no further apart than a quarter of the bit at the outside
                let round_points =
                    ((std::f64::consts::TAU * swing / (bit / 4.0)).ceil() as usize).max(16);
                (0..rings)
                    .map(|i| {
                        let along = along_at(rings, i);
                        (0..=round_points)
                            .map(|j| (along, round_all * j as f64 / round_points as f64))
                            .collect()
                    })
                    .collect()
            }
        };
        // the model's own cells, looked at between the points of each line
        let (fine_along, fine_round) = (self.turned.along_cell, self.turned.round_cell);
        let floors = parallel(lines.len(), |l| {
            let line = &lines[l];
            line.iter()
                .enumerate()
                .map(|(k, &(along, round))| {
                    let near = |i: usize| line.get(i).copied().unwrap_or((along, round));
                    let (before, after) = (near(k.saturating_sub(1)), near(k + 1));
                    let mut top = f64::NEG_INFINITY;
                    for (a0, r0) in [before, after] {
                        let far = ((a0 - along).abs() / fine_along)
                            .max((r0 - round).abs() / fine_round)
                            .ceil()
                            .max(1.0);
                        let mut s = 0.0;
                        while s <= far {
                            let t = s / far / 2.0;
                            let a = (along + (a0 - along) * t).clamp(self.start, self.end);
                            top = top.max(self.tip(a, round + (r0 - round) * t, 0.0));
                            s += 1.0;
                        }
                    }
                    top.min(radius.max(self.top))
                })
                .collect()
        });
        Grid { lines, floors }
    }

    /// The points over from `from` to `to` (the end of one line and the start of the next),
    /// no lower than `level`, the model, or what the holder allows.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "point counts are small"
    )]
    fn over(&self, from: DVec3, to: DVec3, level: f64, room: &impl Fn(f64) -> f64) -> Pass {
        let radius = self.turned.radius;
        let split = |p: DVec3| {
            if self.along_x { (p.x, p.y) } else { (p.y, p.x) }
        };
        let ((a0, r0), (a1, r1)) = (split(from), split(to));
        let far = ((a1 - a0).abs() / self.turned.along_cell)
            .max((r1 - r0).abs() / self.turned.round_cell)
            .ceil() as usize;
        (1..far)
            .map(|s| {
                let t = s as f64 / far as f64;
                let (a, r) = (a0 + (a1 - a0) * t, r0 + (r1 - r0) * t);
                let h = level.max(self.tip(a, r, 0.0)).max(room(a)).min(self.top);
                (self.point(a, r, h - radius), 0.0)
            })
            .collect()
    }

    /// `left` (how far out the material reaches in each `bin` along the part) lowered to
    /// what a level, cut at `heights` on the `grid`, left of it; a height that is infinite
    /// (a point the level didn't cut) leaves its bin as it was.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "bin indices are small and positive"
    )]
    fn lower(&self, left: &mut [f64], bin: f64, grid: &Grid, heights: &[Vec<f64>], way: Way) {
        // the highest each bin was cut to, anywhere round
        let mut cut = vec![f64::NEG_INFINITY; left.len()];
        let mut mark = |along: f64, h: f64| {
            let i = (along / bin).floor();
            if i >= 0.0
                && let Some(c) = cut.get_mut(i as usize)
            {
                *c = c.max(h);
            }
        };
        for (line, hs) in grid.lines.iter().zip(heights) {
            for (&(along, _), &h) in line.iter().zip(hs) {
                mark(along, h);
            }
            // a ring covers half the step either side of it
            if way == Way::RoundThePart
                && let Some(&(along, _)) = line.first()
            {
                let h = hs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                let half = self.step / 2.0;
                let mut a = along - half;
                while a <= along + half {
                    mark(a, h);
                    a += bin;
                }
            }
        }
        for (l, c) in left.iter_mut().zip(cut) {
            if c.is_finite() {
                *l = l.min(c);
            }
        }
    }
}
