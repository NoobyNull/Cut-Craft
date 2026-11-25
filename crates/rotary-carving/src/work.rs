//! Part of the rotary carving engine.

use crate::{Limit, MARK_DEPTH, Pass, Solid, Turned, runs};
use cut_craft_core::CommandError;
use cut_craft_core::glam::DVec3;
use cut_craft_core::progress;
use cut_craft_core::toolpath::Tool;
use cut_craft_rotary::{Clearance, Cutter, Head, Pose, Stock, Voxels};

/// The part as the passes are worked out on it.
pub struct Work<'a> {
    pub turned: &'a Turned,
    /// The full 3D models kept as they really are round the axis, the bit to drop on them
    /// (grown by the allowance), and how far out from the axis their heights were moved to
    /// sit in the material as the rest of the model was.
    pub solid: Option<&'a Solid>,
    /// What is left of the material to hold the part ([`supporting`]): kept like a model.
    pub support: Option<&'a Solid>,
    pub tool: &'a Tool,
    pub allowance: f64,
    pub along_x: bool,
    pub circumference: f64,
    /// Where along the part the passes start and end, and the step between them.
    pub start: f64,
    pub end: f64,
    pub step: f64,
    /// The highest the tip ever needs to be above the axis: how far out the blank swings
    /// (a board or a square reaches past its own radius as it turns).
    pub top: f64,
    /// What lengths are said in.
    pub units: cut_craft_core::document::Units,
    /// Where on the unrolled material the bit may cut: none for all of it.
    pub limit: Option<&'a Limit>,
}

impl Work<'_> {
    /// The point `along` the part and `round` it at height `z`, in the job's X and Y.
    pub fn point(&self, along: f64, round: f64, z: f64) -> DVec3 {
        if self.along_x {
            DVec3::new(along, round, z)
        } else {
            DVec3::new(round, along, z)
        }
    }

    /// The registration mark `along` the part at A=0, 0.5 mm deep: none where the part is
    /// less than that below the top there, as the notch would cut into it.
    pub fn mark(&self, along: f64) -> Option<Pass> {
        let floor = self.tip(along, 0.0, 0.0) - self.turned.radius;
        (floor <= -MARK_DEPTH).then(|| vec![(self.point(along, 0.0, -MARK_DEPTH), 0.0)])
    }

    /// The bit as low as it goes `along` the part with A at `round`, held `across`.
    pub fn down(&self, along: f64, round: f64, across: f64) -> (DVec3, f64) {
        let tip = self.tip(along, round, across);
        (
            self.point(
                along,
                round,
                (tip - self.turned.radius).min(self.top - self.turned.radius),
            ),
            across,
        )
    }

    /// The lowest the tip of the bit can go without touching the model: its height above the
    /// axis.
    pub fn tip(&self, along: f64, round: f64, across: f64) -> f64 {
        let radius = self.turned.radius;
        let turned = round.rem_euclid(self.circumference);
        let mut tip = self
            .turned
            .drop(self.tool, along, turned, across, self.allowance);
        for solid in self.solid.iter().chain(&self.support) {
            tip = tip.max(solid.drop(along, turned / radius, across));
        }
        tip
    }

    /// Where the bit is at a point of a pass, in the machine's frame.
    pub fn pose(&self, &(point, across): &(DVec3, f64)) -> Pose {
        let (along, round) = if self.along_x {
            (point.x, point.y)
        } else {
            (point.y, point.x)
        };
        Pose::new(
            along,
            across,
            self.turned.radius + point.z,
            round / self.turned.radius,
        )
    }

    /// A ring round the part at each step along it, from A=0 all the way back round, the bit
    /// held `across`.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "ring and point counts are small"
    )]
    pub fn rings(&self, across: f64) -> Result<Vec<Pass>, CommandError> {
        progress::stage("Following the model round");
        let rings = ((self.end - self.start) / self.step).ceil() as usize + 1;
        let mut passes = Vec::with_capacity(rings);
        // a batch at a time over every core, so it can be stopped and show how far it is
        let batch = 4 * cores();
        for first in (0..rings).step_by(batch) {
            progress::stop_if_cancelled()?;
            let count = batch.min(rings - first);
            passes.extend(parallel(count, |i| {
                let along = (self.start + (first + i) as f64 * self.step).min(self.end);
                // the last point closes the ring
                (0..=self.turned.round)
                    .map(|j| self.down(along, j as f64 * self.turned.round_cell, across))
                    .collect::<Pass>()
            }));
            progress::report_steps(first + count, rings);
        }
        Ok(passes)
    }

    /// Each of `faces` sides in turn, from A=0, cut back and forth along the part with A held
    /// still: as far across as its share of the way round, and a bit's width more, so the
    /// sides overlap.
    pub fn sides(&self, faces: u32) -> Result<Vec<Pass>, CommandError> {
        progress::stage("Cutting each side");
        let radius = self.turned.radius;
        let half = if faces == 1 {
            radius
        } else {
            (radius * (std::f64::consts::PI / f64::from(faces)).sin() + self.tool.diameter / 2.0)
                .min(radius)
        };
        let mut passes = Vec::with_capacity(faces as usize);
        for face in 0..faces {
            progress::stop_if_cancelled()?;
            let round = self.circumference * f64::from(face) / f64::from(faces);
            passes.push(
                self.side(half, self.step, self.turned.along_cell, |along, across| {
                    self.down(along, round, across)
                }),
            );
            progress::report_steps(face as usize + 1, faces as usize);
        }
        Ok(passes)
    }

    /// One side: lines `step` apart across it from `-half` to `half`, cut
    /// back and forth along the part at points `cell` apart, staying down from one line
    /// over to the next; `at` says where the bit is at each point.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "line and point counts are small"
    )]
    pub fn side(
        &self,
        half: f64,
        step: f64,
        cell: f64,
        at: impl Fn(f64, f64) -> (DVec3, f64) + Sync,
    ) -> Pass {
        let lines = ((2.0 * half / step).ceil() as usize).max(1) + 1;
        let spacing = 2.0 * half / (lines - 1) as f64;
        let points = ((self.end - self.start) / cell).ceil() as usize + 1;
        let along = |k: usize| (self.start + k as f64 * cell).min(self.end);
        let across = |line: usize| -half + line as f64 * spacing;
        // each line worked out on its own, over every core: back and forth, and over to it
        // from where the line before ended, staying down on the part
        parallel(lines, |line| {
            let mut pass: Pass = vec![];
            if line > 0 {
                let (held, to) = (across(line - 1), across(line));
                let ended = along(if line % 2 == 1 { points - 1 } else { 0 });
                let over = ((to - held) / cell).abs().ceil() as usize;
                for k in 1..over {
                    pass.push(at(ended, held + (to - held) * k as f64 / over as f64));
                }
            }
            if line % 2 == 0 {
                pass.extend((0..points).map(|k| at(along(k), across(line))));
            } else {
                pass.extend((0..points).rev().map(|k| at(along(k), across(line))));
            }
            pass
        })
        .concat()
    }

    /// Bulk to fine detail ([`Strategy::SidesThenDetails`]): each side cleared a level at a
    /// time down to the roughing allowance above the model, A held still; then each side
    /// finished; then A locked at more angles for the details, as `details` says. What is
    /// left of the `stock` is kept as the passes are worked out, and any stretch where the bit
    /// would cut nothing is left out, or where the holder above it would touch the material,
    /// the chuck or the tailstock. Also says where the holder kept the bit from material it
    /// should have cut, and the angles locked at for the details.
    #[expect(
        clippy::too_many_lines,
        reason = "bulk, finish and details worked out in turn"
    )]
    pub fn sides_then_details(
        &self,
        stock: &Stock,
        details: &Details,
    ) -> Result<(Vec<(Pass, bool)>, Outcome), CommandError> {
        let Details {
            faces,
            roughing_allowance,
            head,
            ..
        } = details;
        let faces = *faces;
        let tool = self.tool;
        let d = tool.diameter;
        let radius = self.turned.radius;
        let depth = if tool.pass_depth > 0.0 {
            tool.pass_depth
        } else {
            d / 2.0
        };
        // no finer than a twelfth of the bit: what is left between is not worth a pass
        let mut material = Voxels::of(stock, Voxels::cell_for(stock, CELLS).max(d / 12.0));
        // only the bottom of the bit matters for what a pass cuts: the material above it
        // went with the levels before. The bulk is cleared wherever the bit finds material;
        // a side is finished wherever there is any within a cell of it, however thin; an
        // angle for the details is picked where it finds a few cells' worth
        let cell = material.cell();
        let rough = Check::new(tool, depth + cell, 0.0, 1, cell);
        let fine = Check::new(tool, d, cell, 1, cell);
        let detail = Check::new(tool, d, 0.0, DETAIL, cell);
        // the holder's room, worked out a cell or a millimetre at a time, whichever is coarser
        let room = cell.max(1.0);
        let swing = stock.swing();
        let mut passes = vec![];
        let mut refused = vec![];
        let turn = |round: f64| round / radius;
        let steps = 2 * faces as usize + details.tilts as usize;

        progress::stage("Clearing the bulk from each side");
        let rough_step = tool.stepover.clamp(0.05, 1.0) * d;
        let rough_cell = (d / 4.0).max(self.turned.along_cell);
        for face in 0..faces {
            let round = self.circumference * f64::from(face) / f64::from(faces);
            // the lowest the bit may go anywhere on the side, and the highest a fine drop
            // comes anywhere near each coarse point, so the straight moves between them
            // never dip into the model
            let floor = |along: f64, across: f64| {
                let fine_cell = self.turned.along_cell;
                let n = (rough_cell / fine_cell).ceil();
                let mut top = f64::NEG_INFINITY;
                let mut k = -n;
                while k <= n {
                    let a = (along + k * fine_cell).clamp(self.start, self.end);
                    top = top.max(self.tip(a, round, across));
                    k += 1.0;
                }
                top + roughing_allowance
            };
            let floors = self.side(swing, rough_step, rough_cell, |along, across| {
                (
                    self.point(along, round, floor(along, across) - radius),
                    across,
                )
            });
            let lowest = floors
                .iter()
                .map(|(p, _)| p.z + radius)
                .fold(f64::INFINITY, f64::min);
            let mut level = swing;
            let mut k = 0;
            while level > lowest {
                progress::stop_if_cancelled()?;
                k += 1;
                level = (swing - f64::from(k) * depth).max(lowest);
                let pass: Pass = floors
                    .iter()
                    .map(|&(p, across)| {
                        let mut p = p;
                        p.z = p.z.max(level - radius);
                        (p, across)
                    })
                    .collect();
                // the holder's room as the levels before left it
                let clear = Clearance::of(&material, turn(round), head, room);
                // what the holder keeps the bulk from is left for the finishes after
                passes.extend(self.cutting(
                    &pass,
                    &mut material,
                    &rough,
                    Some(&clear),
                    &mut vec![],
                ));
            }
            progress::report_steps(face as usize + 1, steps);
        }

        progress::stage("Finishing each side");
        for face in 0..faces {
            progress::stop_if_cancelled()?;
            let round = self.circumference * f64::from(face) / f64::from(faces);
            passes.extend(self.locked(round, &mut material, &fine, head, room, &mut refused));
            progress::report_steps((faces + face) as usize + 1, steps);
        }

        progress::stage("Locking A for the details");
        let sides: Vec<f64> = (0..faces)
            .map(|face| 360.0 * f64::from(face) / f64::from(faces))
            .collect();
        let mut tilts: Vec<f64> = vec![];
        for k in 0..details.tilts as usize {
            progress::stop_if_cancelled()?;
            let next = if let Some(angles) = &details.angles {
                angles.get(k).copied()
            } else {
                let tried: Vec<f64> = sides.iter().chain(&tilts).copied().collect();
                self.best_tilt(&material, &detail, head, &tried)
            };
            let Some(angle) = next else { break };
            let round = angle.to_radians() * radius;
            passes.extend(self.locked(round, &mut material, &fine, head, room, &mut refused));
            tilts.push(angle);
            progress::report_steps(2 * faces as usize + k + 1, steps);
        }
        progress::report_steps(steps, steps);
        // where the holder kept the bit off, and something is still left for it there now
        // (not in the chuck or tailstock: that is meant to be left)
        let left: Vec<Pose> = refused
            .into_iter()
            .filter(|p: &Pose| !head.held(p.along))
            .filter(|&p| {
                let looking = Pose {
                    height: p.height - fine.margin,
                    ..p
                };
                material.held(&fine.looks, looking, DETAIL) >= DETAIL
            })
            .collect();
        Ok((passes, Outcome { left, tilts }))
    }

    /// A finish of the part with A locked at `round` (mm round the surface): lines back and
    /// forth along it right across the blank (a board is wider than it is thick, and what is
    /// air is left out anyway), cut where `check` finds material and the holder has room.
    pub fn locked(
        &self,
        round: f64,
        material: &mut Voxels,
        check: &Check,
        head: &Head,
        room: f64,
        refused: &mut Vec<Pose>,
    ) -> Vec<(Pass, bool)> {
        let radius = self.turned.radius;
        let half = material.swing();
        let pass = self.side(half, self.step, self.turned.along_cell, |along, across| {
            (
                self.point(along, round, self.tip(along, round, across) - radius),
                across,
            )
        });
        let clear = Clearance::of(material, round / radius, head, room);
        self.cutting(&pass, material, check, Some(&clear), refused)
    }

    /// The angle (degrees) A locked at reaches the most of what is left of the `material`
    /// for a finish, the holder kept clear, of every 15° but those `tried`: none when no
    /// angle reaches enough to be worth it. Looked at a bit's width apart.
    pub fn best_tilt(
        &self,
        material: &Voxels,
        check: &Check,
        head: &Head,
        tried: &[f64],
    ) -> Option<f64> {
        best_of(&self.tilt_scores(material, check, head, tried), |s| {
            s.reached
        })
    }

    /// The angle (degrees) A locked at reaches the most of what is left of the `material`
    /// where it faces the bit squarely (no steeper than [`FACING`]), as [`Work::best_tilt`]
    /// does: a flat is finished from straight over it, not slantwise with the side of the
    /// ball. Where nothing is left facing any angle worth it, the angle that reaches the
    /// most of it anyhow.
    pub fn best_facing(
        &self,
        material: &Voxels,
        check: &Check,
        head: &Head,
        tried: &[f64],
    ) -> Option<f64> {
        let scores = self.tilt_scores(material, check, head, tried);
        best_of(&scores, |s| s.facing).or_else(|| best_of(&scores, |s| s.reached))
    }

    /// How much of what is left of the `material` A locked at each 15° reaches (none for
    /// those `tried`), and how much of that faces the bit squarely.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "line and point counts are small"
    )]
    fn tilt_scores(
        &self,
        material: &Voxels,
        check: &Check,
        head: &Head,
        tried: &[f64],
    ) -> Vec<Score> {
        let radius = self.turned.radius;
        let step = self.tool.diameter;
        let half = material.swing();
        let lines = ((2.0 * half / step).ceil() as usize).max(1) + 1;
        let spacing = 2.0 * half / (lines - 1) as f64;
        let points = ((self.end - self.start) / step).ceil() as usize + 1;
        let near =
            |a: f64, b: f64| ((a - b).rem_euclid(360.0)).min((b - a).rem_euclid(360.0)) < 1.0;
        let steepest = FACING.to_radians().tan();
        // every angle at once, each worked through on its own thread
        parallel(24, |k| {
            let angle = 15.0 * f64::from(u32::try_from(k).unwrap_or(0));
            let mut score = Score {
                angle,
                reached: 0.0,
                facing: 0.0,
            };
            if tried.iter().any(|&t| near(t, angle)) {
                return score;
            }
            let round = angle.to_radians() * radius;
            // coarser than the finish's own: only to choose by
            let clear = Clearance::of(
                material,
                round / radius,
                head,
                (2.0 * material.cell()).max(2.0),
            );
            let along = |i: usize| (self.start + i as f64 * step).min(self.end);
            let across = |line: usize| -half + line as f64 * spacing;
            let tips: Vec<Vec<f64>> = (0..lines)
                .map(|line| {
                    (0..points)
                        .map(|i| self.tip(along(i), round, across(line)))
                        .collect()
                })
                .collect();
            for line in 0..lines {
                for i in 0..points {
                    if head.held(along(i)) {
                        continue;
                    }
                    let tip = tips[line][i];
                    let pose = Pose::new(along(i), across(line), tip, round / radius);
                    if !clear.allows(pose) {
                        continue;
                    }
                    // looked for with the bit grown by the check's margin all round
                    let looking = Pose {
                        height: pose.height - check.margin,
                        ..pose
                    };
                    let n = material.held(&check.looks, looking, 64);
                    if n < check.enough {
                        continue;
                    }
                    score.reached += n as f64;
                    // how steep what the bit lands on is, across and along: from the tips
                    // either side
                    let slope = |a: f64, b: f64, apart: f64| (a - b).abs() / apart;
                    let (l0, l1) = (line.saturating_sub(1), (line + 1).min(lines - 1));
                    let (i0, i1) = (i.saturating_sub(1), (i + 1).min(points - 1));
                    let sideways = if l1 > l0 {
                        slope(tips[l1][i], tips[l0][i], (l1 - l0) as f64 * spacing)
                    } else {
                        0.0
                    };
                    let lengthways = if i1 > i0 {
                        slope(tips[line][i1], tips[line][i0], along(i1) - along(i0))
                    } else {
                        0.0
                    };
                    let slant = sideways.max(lengthways);
                    if slant < steepest {
                        score.facing += n as f64 * (1.0 - slant / steepest);
                    }
                }
            }
            score
        })
    }

    /// `pass` broken where the bit would cut too little of what is left of the `material`
    /// to be worth it (as `check` says), which loses what the rest cuts; and, with a
    /// `clear`ance, where the holder would touch the material, the chuck or the tailstock
    /// (those points, where the bit would have cut, go in `refused`).
    pub fn cutting(
        &self,
        pass: &[(DVec3, f64)],
        material: &mut Voxels,
        check: &Check,
        clear: Option<&Clearance>,
        refused: &mut Vec<Pose>,
    ) -> Vec<(Pass, bool)> {
        let Check {
            cutter,
            looks,
            above,
            margin,
            enough,
        } = check;
        let mut cuts = Vec::with_capacity(pass.len());
        let mut blocked = Vec::with_capacity(pass.len());
        // where a stretch may start, whether the bit can come straight down to it quickly:
        // nothing of the material anywhere above it, just before it goes there
        let mut quick = vec![false; pass.len()];
        let mut last: Option<Pose> = None;
        for (i, point) in pass.iter().enumerate() {
            let pose = self.pose(point);
            let fits = clear.is_none_or(|c| c.allows(pose));
            blocked.push(!fits);
            // outside the machining limit: not cut, never bridged over, and the material
            // there kept as it is
            if self.limit.is_some_and(|l| !l.holds_point(self.along_x, point)) {
                blocked[i] = true;
                last = None;
                cuts.push(false);
                continue;
            }
            let looking = Pose {
                height: pose.height - margin,
                ..pose
            };
            let mut cutting = material.held(looks, looking, *enough) >= *enough;
            if cutting && !fits {
                refused.push(pose);
                cutting = false;
            }
            if cutting {
                if last.is_none() {
                    let start = if i > 0 && !blocked[i - 1] { i - 1 } else { i };
                    quick[start] = material.held(above, self.pose(&pass[start]), 1) == 0;
                }
                match last {
                    Some(from) => material.cut(cutter, from, pose),
                    None => material.cut_at(cutter, pose),
                };
                last = Some(pose);
            } else {
                last = None;
            }
            cuts.push(cutting);
        }
        let found = runs(pass, &cuts, &blocked, 2.0 * self.tool.diameter);
        found
            .into_iter()
            .map(|run| (pass[run.clone()].to_vec(), quick[run.start]))
            .collect()
    }
}

/// How bulk to fine detail is to go: how many sides, what the bulk leaves, how many angles
/// A locks at for the details (and which, when they are given), and the holder, chuck and
/// tailstock to keep clear of.
pub struct Details {
    pub faces: u32,
    pub roughing_allowance: f64,
    pub tilts: u32,
    pub angles: Option<Vec<f64>>,
    pub head: Head,
}

/// What bulk to fine detail found as it went: where the holder kept the bit from material
/// still left for it, and the angles A locked at for the details (degrees).
#[derive(Debug, Default)]
pub struct Outcome {
    pub left: Vec<Pose>,
    pub tilts: Vec<f64>,
}

use cut_craft_core::work::{cores, parallel};

/// How a pass of bulk to fine detail decides whether a point cuts anything.
pub struct Check {
    /// The bit, as far up as matters.
    pub cutter: Cutter,
    /// The bit grown by `margin` all round (and its tip lowered by it), to look for
    /// material with: the material is kept in cells, and a layer thinner than a cell must
    /// still be found.
    pub looks: Cutter,
    /// The whole bit and holder, all the way up, a cell wider all round: what must find
    /// nothing above a point for the bit to come straight down to it at the rapid rate.
    pub above: Cutter,
    pub margin: f64,
    /// How many cells it must find.
    pub enough: usize,
}

impl Check {
    pub fn new(tool: &Tool, length: f64, margin: f64, enough: usize, cell: f64) -> Self {
        let bit = |grow: f64, length: f64| {
            Cutter::new(&Tool {
                diameter: tool.diameter + 2.0 * grow,
                flute_length: length + grow,
                ..tool.clone()
            })
        };
        Self {
            cutter: bit(0.0, length),
            looks: bit(margin, length),
            // past the material's swing however long the bit is: as high as matters
            above: bit(cell, f64::MAX / 4.0),
            margin,
            enough,
        }
    }
}

/// How many cells the material is kept in as bulk to fine detail works it out: about 16 MB.
pub const CELLS: usize = 120_000_000;

/// How many cells of material a ring must find to be worth cutting for detail.
pub const DETAIL: usize = 3;

/// The steepest (degrees from square to the bit) a surface may be and still face A locked
/// at an angle, for [`Work::best_facing`].
pub const FACING: f64 = 30.0;

/// What A locked at an `angle` reaches of what is left: all of it, and what of it faces the
/// bit squarely.
#[derive(Debug)]
struct Score {
    angle: f64,
    reached: f64,
    /// Weighted by how squarely: full straight under the bit, none at [`FACING`].
    facing: f64,
}

/// The angle with the most of what `by` counts, when that is enough to be worth it: a few
/// points' worth at least, as less is slivers, not detail.
#[expect(clippy::cast_precision_loss, reason = "a small count")]
fn best_of(scores: &[Score], by: impl Fn(&Score) -> f64) -> Option<f64> {
    let mut best: Option<(f64, f64)> = None;
    for s in scores {
        if by(s) > best.map_or(0.0, |b| b.1) {
            best = Some((s.angle, by(s)));
        }
    }
    best.filter(|b| b.1 >= (3 * DETAIL * 8) as f64).map(|b| b.0)
}
