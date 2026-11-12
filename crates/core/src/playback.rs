//! Animated toolpath preview (Aspire's help: Preview Toolpaths ▸ Animate preview, Preview
//! Control): the toolpaths' moves played into the material a distance at a time. The
//! position is the distance the tool has travelled; moving it on cuts up to there, part of a
//! move if need be, and moving it back cuts again from the material it started from.

use crate::simulation::Material;
use crate::toolpath::{Tool, Toolpath};
use glam::DVec3;

/// One straight move of the tool.
struct Step {
    tool: usize,
    from: DVec3,
    to: DVec3,
}

pub struct Playback {
    tools: Vec<Tool>,
    /// The id each tool's cells are marked with (`Material::cut_by`), by tool.
    marks: Vec<u32>,
    steps: Vec<Step>,
    /// Distance travelled at the end of each step.
    ends: Vec<f64>,
    start: Material,
    material: Material,
    at: f64,
}

impl Playback {
    /// `toolpaths` to be played, in order, into `start`. Says how far it has got
    /// ([`crate::progress`]), as millions of moves take a while; asked to stop, it leaves
    /// the rest out, for a job that is thrown away.
    pub fn new(start: Material, toolpaths: &[&Toolpath]) -> Self {
        const EVERY: usize = 1 << 16;
        let total: usize = toolpaths.iter().map(|t| t.moves.len()).sum();
        let (mut tools, mut steps) = (vec![], Vec::with_capacity(total));
        'toolpaths: for t in toolpaths {
            tools.push(t.tool.clone());
            for pair in t.moves.windows(2) {
                if steps.len() % EVERY == 0 {
                    if crate::progress::cancelled() {
                        break 'toolpaths;
                    }
                    crate::progress::report_steps(steps.len(), total);
                }
                steps.push(Step {
                    tool: tools.len() - 1,
                    from: pair[0].to,
                    to: pair[1].to,
                });
            }
        }
        let mut total = 0.0;
        let ends = steps
            .iter()
            .map(|s| {
                total += s.from.distance(s.to);
                total
            })
            .collect();
        Self {
            marks: vec![0; tools.len()],
            tools,
            steps,
            ends,
            material: start.clone(),
            start,
            at: 0.0,
        }
    }

    /// The same, each toolpath's cells marked with its id from `marks` (in order).
    #[must_use]
    pub fn marked(mut self, marks: &[u32]) -> Self {
        for (m, &id) in self.marks.iter_mut().zip(marks) {
            *m = id;
        }
        self
    }

    /// A double-sided job's other side, as its toolpaths cut it, kept with the material
    /// played into (and with it when played back again).
    pub fn set_other_side(&mut self, other: Option<std::sync::Arc<Material>>) {
        self.start.other_side.clone_from(&other);
        self.material.other_side = other;
    }

    /// The material as cut so far.
    pub fn material(&self) -> &Material {
        &self.material
    }

    /// How far the tool travels in all.
    pub fn length(&self) -> f64 {
        self.ends.last().copied().unwrap_or(0.0)
    }

    /// How far it has travelled.
    pub fn at(&self) -> f64 {
        self.at
    }

    pub fn finished(&self) -> bool {
        self.at >= self.length()
    }

    /// The step under distance `d`, and how far into it (0 to 1).
    fn locate(&self, d: f64) -> Option<(usize, f64)> {
        let i = self
            .ends
            .partition_point(|&end| end < d)
            .min(self.steps.len().checked_sub(1)?);
        let begin = if i == 0 { 0.0 } else { self.ends[i - 1] };
        let length = self.ends[i] - begin;
        Some((
            i,
            if length > 0.0 {
                ((d - begin) / length).clamp(0.0, 1.0)
            } else {
                1.0
            },
        ))
    }

    /// Where the tool's tip is, and which tool it is.
    pub fn tool(&self) -> Option<(DVec3, &Tool)> {
        let (i, t) = self.locate(self.at)?;
        let s = &self.steps[i];
        Some((s.from.lerp(s.to, t), &self.tools[s.tool]))
    }

    /// Moves the tool to distance `d` (kept within the path), cutting on the way; going back
    /// cuts again from the start.
    pub fn seek(&mut self, d: f64) {
        self.seek_until(d, || false);
    }

    /// [`Self::seek`], a step at a time, stopping at the end of the first step after which
    /// `out_of_time` says so: whether the tool got to `d`. Going back starts again from the
    /// start before it stops. Each call cuts at least one step, so calls in turn get there.
    pub fn seek_until(&mut self, d: f64, mut out_of_time: impl FnMut() -> bool) -> bool {
        let d = d.clamp(0.0, self.length());
        if d < self.at {
            self.material = self.start.clone();
            self.at = 0.0;
        }
        if let (Some((i, ti)), Some((j, tj))) = (self.locate(self.at), self.locate(d)) {
            for k in i..=j {
                let s = &self.steps[k];
                let (a, b) = (if k == i { ti } else { 0.0 }, if k == j { tj } else { 1.0 });
                if b <= a {
                    continue;
                }
                self.material.cutting = self.marks[s.tool];
                self.material.sweep(
                    s.from.lerp(s.to, a),
                    s.from.lerp(s.to, b),
                    &self.tools[s.tool],
                );
                if k < j && out_of_time() {
                    self.at = self.ends[k];
                    return false;
                }
            }
        }
        self.at = d;
        true
    }

    /// The end of the step after the tool's position (Single Step).
    pub fn next_step(&self) -> f64 {
        let i = self.ends.partition_point(|&end| end <= self.at);
        self.ends.get(i).copied().unwrap_or(self.length())
    }

    /// Where the tool next lifts out of the material (Run to Retract): the end of the next
    /// rapid or upward move that leaves the cut, else the end.
    pub fn next_retract(&self) -> f64 {
        let i = self.ends.partition_point(|&end| end <= self.at);
        self.steps
            .iter()
            .enumerate()
            .skip(i)
            .find(|(_, s)| s.to.z > s.from.z && s.to.z >= 0.0 && s.from.z < 0.0)
            .map_or(self.length(), |(k, _)| self.ends[k])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Job;
    use crate::toolpath::{Move, MoveKind};

    /// A 100 × 100 job, and a cut 2 deep along y = 50 from x = 10 to 90 between a plunge and
    /// a retract.
    fn line() -> (Material, Toolpath) {
        let job = Job {
            width: 100.0,
            height: 100.0,
            ..Job::default()
        };
        let m = |kind, x: f64, z: f64| Move {
            kind,
            to: DVec3::new(job.area().min.x + x, job.area().min.y + 50.0, z),
        };
        let toolpath = Toolpath {
            name: "line".into(),
            strategy: String::new(),
            params: serde_json::Value::Null,
            tool: Tool::default(),
            vectors: vec![],
            moves: vec![
                m(MoveKind::Rapid, 10.0, 5.0),
                m(MoveKind::Plunge, 10.0, -2.0),
                m(MoveKind::Cut, 90.0, -2.0),
                m(MoveKind::Rapid, 90.0, 5.0),
            ],
            visible: true,
            power: vec![],
            stale: false,
            group: None,
            drill_bank: None,
        };
        (Material::new(&job, 100), toolpath)
    }

    #[test]
    fn the_cells_a_toolpath_cuts_are_marked_with_its_id() {
        let (start, toolpath) = line();
        let mut p = Playback::new(start, &[&toolpath]).marked(&[7]);
        p.seek(p.length());
        // millimetre cells: the cell at (50, 50) is cut, the one at (50, 10) is not
        let m = p.material();
        assert_eq!(
            m.cut_by[50 * m.columns + 50],
            7,
            "cut by the toolpath marked 7"
        );
        assert_eq!(m.cut_by[10 * m.columns + 50], 0, "uncut");
    }

    fn depth_at(p: &Playback, x: f64) -> f64 {
        let m = p.material();
        m.at(m.origin + glam::DVec2::new(x, 50.0)).unwrap()
    }

    #[test]
    fn cuts_as_far_as_the_tool_has_gone() {
        let (start, toolpath) = line();
        let mut p = Playback::new(start, &[&toolpath]);
        assert_eq!(p.length(), 7.0 + 80.0 + 7.0);
        p.seek(7.0 + 40.0);
        assert!(depth_at(&p, 30.0) < -1.0, "cut where it has been");
        assert!(depth_at(&p, 80.0) > -0.5, "not yet where it has not");
        let (tip, _) = p.tool().unwrap();
        assert!(
            (tip.x - (p.material().origin.x + 50.0)).abs() < 1e-9 && (tip.z + 2.0).abs() < 1e-9
        );
        p.seek(p.length());
        assert!(p.finished() && depth_at(&p, 80.0) < -1.0);
    }

    #[test]
    fn going_back_cuts_again_from_the_start() {
        let (start, toolpath) = line();
        let mut p = Playback::new(start, &[&toolpath]);
        p.seek(p.length());
        p.seek(7.0 + 10.0);
        assert!(depth_at(&p, 80.0) > -0.5, "uncut again");
        assert!(depth_at(&p, 15.0) < -1.0);
    }

    #[test]
    fn steps_and_retracts() {
        let (start, toolpath) = line();
        let mut p = Playback::new(start, &[&toolpath]);
        assert_eq!(p.next_step(), 7.0, "the plunge");
        p.seek(p.next_step());
        assert_eq!(p.next_step(), 87.0, "the cut");
        assert_eq!(p.next_retract(), 94.0, "out at the end");
    }

    #[test]
    fn a_long_seek_stops_when_out_of_time_and_goes_on_from_there() {
        let (start, toolpath) = line();
        let mut p = Playback::new(start, &[&toolpath]);
        // out of time after every step: one step a call, never none
        assert!(!p.seek_until(p.length(), || true), "not there yet");
        assert_eq!(p.at(), 7.0, "the plunge done");
        assert!(!p.seek_until(p.length(), || true));
        assert_eq!(p.at(), 87.0, "then the cut");
        assert!(depth_at(&p, 80.0) < -1.0, "cut as far as it went");
        assert!(
            p.seek_until(p.length(), || true),
            "the last step gets there"
        );
        assert!(p.finished());
    }

    #[test]
    fn a_seek_back_out_of_time_starts_again_from_uncut() {
        let (start, toolpath) = line();
        let mut p = Playback::new(start, &[&toolpath]);
        p.seek(p.length());
        assert!(!p.seek_until(7.0 + 40.0, || true));
        assert_eq!(p.at(), 7.0, "back to the start, the plunge cut again");
        assert!(depth_at(&p, 80.0) > -0.5, "uncut again");
        assert!(p.seek_until(7.0 + 40.0, || true));
        assert_eq!(p.at(), 47.0, "part of the cut, where it was asked to go");
    }
}
