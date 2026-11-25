//! Rotary Finishing's passes: the bit kept on the model all the way round, as low as it goes
//! without touching it, round the part ring by ring or along it line by line; over the
//! turning axis, or held to one side of it (Off centre), one side or both. Only where some of
//! the material is still left is cut, and where the collet nut and spindle can't get down
//! beside what is left the bit is kept off, and that is said.

use cut_craft_core::CommandError;
use cut_craft_core::glam::DVec3;
use cut_craft_core::progress;
use cut_craft_core::work::parallel;
use cut_craft_rotary::{Head, Voxels};

use crate::{Check, CutDirection, Pass, Posed, Way, Work, forward_climbs, reach_along};

/// How a finish goes over the part.
pub struct Finish {
    pub way: Way,
    pub direction: CutDirection,
    /// How far across the axis the bit is held for each run over the whole part: `[0.0]`
    /// over the axis, `[d]` to one side, `[d, -d]` one side then the other.
    pub held: Vec<f64>,
    /// The holder, chuck and tailstock to keep clear of.
    pub head: Head,
    /// Follows the surface: `held` aside, A and the bit's place across the axis change
    /// together at every point so the bit meets the model square on (see
    /// [`Work::following`]).
    pub follow: bool,
}

/// What a finish worked out: the passes, and the places along the part where the holder
/// kept the bit off material it should have cut.
#[derive(Debug, Default)]
pub struct Finished {
    pub passes: Vec<(Pass, bool)>,
    pub kept_off: Vec<f64>,
}

impl Work<'_> {
    /// The finish `finish` asks for over what is left of the `material`, which it cuts as
    /// it goes.
    ///
    /// # Errors
    /// When it is stopped.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "line and point counts are small"
    )]
    pub fn finishing(
        &self,
        material: &mut Voxels,
        finish: &Finish,
    ) -> Result<Finished, CommandError> {
        let radius = self.turned.radius;
        let cell = material.cell();
        // a pass is cut wherever the bit there meets any of what is left: half a cell's worth
        // is found, and a finish over a finish finds nothing (looking any lower would find
        // the cells the model's surface passes through, and cut it all again)
        let check = Check::new(self.tool, self.tool.diameter, 0.0, 1, cell);
        // how far out what is left reaches along the part, for the holder: it only gets
        // lower as the finish goes, so this keeps it clear
        let bin = (self.tool.diameter / 4.0).max(cell);
        let bins = (self.end / bin).ceil() as usize + 2;
        let left = reach_along(material, bin, bins);
        let at = |along: f64| {
            let i = (along / bin).floor();
            if i < 0.0 {
                0.0
            } else {
                left.get(i as usize).copied().unwrap_or(0.0)
            }
        };
        let room_step = bin.max(1.0);
        let head = &finish.head;
        let forward = forward_climbs(finish.way, self.along_x);
        let one_way = match finish.direction {
            CutDirection::Both => None,
            CutDirection::Climb => Some(forward),
            CutDirection::Conventional => Some(!forward),
        };
        let lines = self.finish_lines(finish.way);
        let mut out = Finished::default();
        let runs_held = if finish.follow {
            vec![0.0]
        } else {
            finish.held.clone()
        };
        let runs = runs_held.len();
        for (r, &across) in runs_held.iter().enumerate() {
            progress::stage("Following the model");
            progress::stop_if_cancelled()?;
            let poses = self.posed(&lines, finish, across);
            let room: Vec<f64> = {
                let mut alongs: Vec<f64> = lines.iter().flatten().map(|p| p.0).collect();
                alongs.sort_by(f64::total_cmp);
                alongs.dedup();
                alongs
            };
            let lowest: Vec<f64> = room
                .iter()
                .map(|&a| head.lowest_over(a, at, room_step))
                .collect();
            let lets = |along: f64| {
                let i = room.partition_point(|&a| a < along - 1e-9);
                lowest.get(i).copied().unwrap_or(f64::NEG_INFINITY)
            };
            let mut passes: Vec<Pass> = vec![];
            let mut pass: Pass = vec![];
            for (l, line) in poses.iter().enumerate() {
                let mut points: Pass = Vec::with_capacity(line.len());
                for &Posed {
                    along,
                    round,
                    across,
                    tip,
                } in line
                {
                    let allowed = lets(along);
                    let h = if allowed > tip + 1e-6 {
                        // kept off, where there is still something to cut there
                        if tip < at(along) - crate::GRAZE {
                            out.kept_off.push(along);
                        }
                        allowed.min(self.top).max(tip)
                    } else {
                        tip
                    };
                    points.push((self.point(along, round, h - radius), across));
                }
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
                    if let (Some(&(from, held)), Some(&(to, _))) = (pass.last(), points.first()) {
                        pass.extend(self.across_to(from, to, held, &lets));
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
            progress::report_steps(r + 1, runs);
        }
        Ok(out)
    }

    /// Where A turns to, how far across the bit is held and how low it goes at every point
    /// of `lines`, over every core: held `across` the axis, or following the surface.
    fn posed(&self, lines: &[Vec<(f64, f64)>], finish: &Finish, across: f64) -> Vec<Vec<Posed>> {
        parallel(lines.len(), |l| {
            if finish.follow {
                self.following(&lines[l], finish.way)
            } else {
                lines[l]
                    .iter()
                    .map(|&(along, round)| Posed {
                        along,
                        round,
                        across,
                        tip: self.tip(along, round, across).min(self.top),
                    })
                    .collect()
            }
        })
    }

    /// The finish's lines: rings round the part a step apart along it, or lines along the
    /// part a step apart round it (at the surface), each with points a model's cell apart.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "line and point counts are small"
    )]
    fn finish_lines(&self, way: Way) -> Vec<Vec<(f64, f64)>> {
        let length = self.end - self.start;
        let spaced = |count: usize, k: usize| {
            if count <= 1 {
                self.start
            } else {
                self.start + length * k as f64 / (count - 1) as f64
            }
        };
        let c = self.circumference;
        match way {
            Way::RoundThePart => {
                let rings = ((length / self.step).ceil() as usize).max(1) + 1;
                let round = self.turned.round;
                (0..rings)
                    .map(|i| {
                        let along = spaced(rings, i);
                        (0..=round)
                            .map(|j| (along, j as f64 * self.turned.round_cell))
                            .collect()
                    })
                    .collect()
            }
            Way::AlongThePart => {
                let lines = ((c / self.step).ceil() as usize).max(8);
                let points = ((length / self.turned.along_cell).ceil() as usize).max(1) + 1;
                (0..lines)
                    .map(|j| {
                        let round = c * j as f64 / lines as f64;
                        (0..points).map(|k| (spaced(points, k), round)).collect()
                    })
                    .collect()
            }
        }
    }

    /// The points over from `from` to `to` (the end of one line and the start of the next),
    /// the bit held `across` and kept on the model, no lower than the holder `lets` it.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "point counts are small"
    )]
    fn across_to(&self, from: DVec3, to: DVec3, across: f64, lets: &impl Fn(f64) -> f64) -> Pass {
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
                let h = self.tip(a, r, across).max(lets(a)).min(self.top);
                (self.point(a, r, h - radius), across)
            })
            .collect()
    }
}
