//! Animated toolpath preview (help: Preview Toolpaths ▸ Animate preview, Preview
//! Control): the toolpaths' moves played into the material as the machine would cut them.
//! The position is the machine's time in seconds, each move taking as long as it would at
//! its tool's feed or plunge rate (rapids at the rapid rate); moving it on cuts up to there,
//! part of a move if need be, and moving it back cuts again from the material it started
//! from.

use crate::simulation::Material;
use crate::toolpath::{MoveKind, Tool, Toolpath};
use glam::DVec3;

/// One straight move of the tool.
struct Step {
    tool: usize,
    kind: MoveKind,
    from: DVec3,
    to: DVec3,
}

/// The most saves of the material a playback keeps, so going back starts near where it
/// is going instead of from the start.
const MOST_SAVES: usize = 20;
/// The memory the saves may take in all, bytes.
const SAVES_MEMORY: usize = 400 << 20;

/// How many saves a material of `cells` cells gets: [`MOST_SAVES`], fewer for a large one,
/// so they keep to [`SAVES_MEMORY`]; at least one.
fn saves_for(cells: usize) -> usize {
    let each = cells * (size_of::<f64>() + size_of::<u32>());
    (SAVES_MEMORY / each.max(1)).clamp(1, MOST_SAVES)
}

/// Where each playback's revisions come from, so no two playbacks ever share one.
static REVISIONS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn next_revision() -> u64 {
    REVISIONS.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// The material as cut at the end of a step.
struct Save {
    /// Which stretch of [`Playback::save_every`] it is in (one save each).
    slot: u64,
    time: f64,
    heights: Vec<f64>,
    cut_by: Vec<u32>,
}

pub struct Playback {
    tools: Vec<Tool>,
    /// The id each tool's cells are marked with (`Material::cut_by`), by tool.
    marks: Vec<u32>,
    steps: Vec<Step>,
    /// The machine's time (seconds) at the end of each step.
    ends: Vec<f64>,
    start: Material,
    material: Material,
    at: f64,
    /// The material saved along the way, in time order.
    saves: Vec<Save>,
    /// How far apart the saves are, seconds.
    save_every: f64,
    /// Changes whenever the material does, so what is drawn of it can be kept till then.
    revision: u64,
}

impl Playback {
    /// `toolpaths` to be played, in order, into `start`. Says how far it has got
    /// ([`crate::progress`]), as millions of moves take a while; asked to stop, it leaves
    /// the rest out, for a job that is thrown away. Rapids run at `rapid_rate` (mm/min), as
    /// does a move whose tool has no rate set.
    pub fn new(start: Material, toolpaths: &[&Toolpath], rapid_rate: f64) -> Self {
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
                    kind: pair[1].kind,
                    from: pair[0].to,
                    to: pair[1].to,
                });
            }
        }
        let rapid = if rapid_rate > 0.0 { rapid_rate } else { 5000.0 };
        let mut total = 0.0;
        let ends: Vec<f64> = steps
            .iter()
            .map(|s| {
                let tool: &Tool = &tools[s.tool];
                let rate = match s.kind {
                    MoveKind::Rapid => rapid,
                    MoveKind::Plunge => tool.plunge_rate,
                    MoveKind::Cut => tool.feed_rate,
                };
                let per_second = if rate > 0.0 { rate } else { rapid } / 60.0;
                total += s.from.distance(s.to) / per_second;
                total
            })
            .collect();
        let length: f64 = ends.last().copied().unwrap_or(0.0);
        #[expect(clippy::cast_precision_loss, reason = "a handful of saves")]
        let save_every = length / (saves_for(start.heights.len()) + 1) as f64;
        Self {
            marks: vec![0; tools.len()],
            tools,
            steps,
            ends,
            material: start.clone(),
            start,
            at: 0.0,
            saves: vec![],
            save_every,
            revision: next_revision(),
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
        self.revision = next_revision();
    }

    /// Which state of the material this is: another number once anything about it changes,
    /// and never one another playback has had.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// The material as cut so far.
    pub fn material(&self) -> &Material {
        &self.material
    }

    /// How long the machine takes in all, seconds.
    pub fn length(&self) -> f64 {
        self.ends.last().copied().unwrap_or(0.0)
    }

    /// How long it has been going, seconds.
    pub fn at(&self) -> f64 {
        self.at
    }

    pub fn finished(&self) -> bool {
        self.at >= self.length()
    }

    /// The step under time `d`, and how far into it (0 to 1).
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

    /// Moves the tool to time `d` (seconds) (kept within the path), cutting on the way; going back
    /// cuts again from the start.
    pub fn seek(&mut self, d: f64) {
        self.seek_until(d, || false);
    }

    /// [`Self::seek`], a step at a time, stopping at the end of the first step after which
    /// `out_of_time` says so: whether the tool got to `d`. Going back starts again from the
    /// start before it stops. Each call cuts at least one step, so calls in turn get there.
    pub fn seek_until(&mut self, d: f64, mut out_of_time: impl FnMut() -> bool) -> bool {
        let d = d.clamp(0.0, self.length());
        if d != self.at {
            self.revision = next_revision();
        }
        // from the last save before `d`, when it is ahead or the tool goes back; else back
        // to the start
        let save = (self.saves.iter())
            .rposition(|s| s.time <= d)
            .filter(|&i| d < self.at || self.saves[i].time > self.at);
        if let Some(i) = save {
            let save = &self.saves[i];
            self.material.heights.clone_from(&save.heights);
            self.material.cut_by.clone_from(&save.cut_by);
            self.at = save.time;
        } else if d < self.at {
            self.material.heights.clone_from(&self.start.heights);
            self.material.cut_by.clone_from(&self.start.cut_by);
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
                if b >= 1.0 {
                    self.save_at(k);
                }
                if k < j && out_of_time() {
                    self.at = self.ends[k];
                    return false;
                }
            }
        }
        self.at = d;
        true
    }

    /// The material saved at the end of step `k` when no save has been made in its stretch
    /// of time yet.
    fn save_at(&mut self, k: usize) {
        let time = self.ends[k];
        // none at the very end: nothing goes on from there
        if self.save_every <= 0.0 || time >= self.length() {
            return;
        }
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a small slot number"
        )]
        let slot = (time / self.save_every).floor() as u64;
        if slot == 0 || self.saves.iter().any(|s| s.slot == slot) {
            return;
        }
        let at = self.saves.partition_point(|s| s.time < time);
        self.saves.insert(
            at,
            Save {
                slot,
                time,
                heights: self.material.heights.clone(),
                cut_by: self.material.cut_by.clone(),
            },
        );
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
    use crate::toolpath::Move;

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
            // a millimetre a second, rapids too: seconds along read as millimetres
            tool: Tool {
                feed_rate: 60.0,
                plunge_rate: 60.0,
                ..Tool::default()
            },
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
        let mut p = Playback::new(start, &[&toolpath], 60.0).marked(&[7]);
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
        let mut p = Playback::new(start, &[&toolpath], 60.0);
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
        let mut p = Playback::new(start, &[&toolpath], 60.0);
        p.seek(p.length());
        p.seek(7.0 + 10.0);
        assert!(depth_at(&p, 80.0) > -0.5, "uncut again");
        assert!(depth_at(&p, 15.0) < -1.0);
    }

    #[test]
    fn steps_and_retracts() {
        let (start, toolpath) = line();
        let mut p = Playback::new(start, &[&toolpath], 60.0);
        assert_eq!(p.next_step(), 7.0, "the plunge");
        p.seek(p.next_step());
        assert_eq!(p.next_step(), 87.0, "the cut");
        assert_eq!(p.next_retract(), 94.0, "out at the end");
    }

    #[test]
    fn plays_at_the_tools_feeds_and_the_rapid_rate() {
        let (start, mut toolpath) = line();
        // cut at 1200 mm/min (20 mm/s), plunge at 120 (2 mm/s), rapids at 6000 (100 mm/s)
        toolpath.tool.feed_rate = 1200.0;
        toolpath.tool.plunge_rate = 120.0;
        let p = Playback::new(start, &[&toolpath], 6000.0);
        let seconds = 7.0 / 2.0 + 80.0 / 20.0 + 7.0 / 100.0;
        assert!((p.length() - seconds).abs() < 1e-9, "{} s", p.length());
        assert!((p.next_step() - 3.5).abs() < 1e-9, "the plunge takes 3.5 s");
    }

    #[test]
    fn a_rate_not_set_plays_at_the_rapid_rate() {
        let (start, mut toolpath) = line();
        toolpath.tool.feed_rate = 0.0;
        toolpath.tool.plunge_rate = 0.0;
        let p = Playback::new(start, &[&toolpath], 60.0);
        assert_eq!(p.length(), 94.0);
    }

    /// A zig-zag of 100 cuts, 1 mm a second, across the 100 × 100 job.
    fn zigzag() -> (Material, Toolpath) {
        let (start, mut toolpath) = line();
        let origin = start.origin;
        toolpath.moves = (0..=100)
            .map(|i| Move {
                kind: MoveKind::Cut,
                to: DVec3::new(
                    origin.x + if i % 2 == 0 { 10.0 } else { 90.0 },
                    origin.y + 10.0 + f64::from(i) * 0.8,
                    -2.0,
                ),
            })
            .collect();
        (start, toolpath)
    }

    #[test]
    fn played_through_once_it_goes_back_from_the_nearest_save_not_the_start() {
        let (start, toolpath) = zigzag();
        let mut p = Playback::new(start.clone(), &[&toolpath], 60.0);
        p.seek(p.length());
        let middle = p.length() * 0.55;
        let mut calls = 0;
        while !p.seek_until(middle, || true) {
            calls += 1;
        }
        assert!(calls < 10, "{calls} steps back, not 55 from the start");
        let mut straight = Playback::new(start, &[&toolpath], 60.0);
        straight.seek(middle);
        assert!(
            p.material().heights == straight.material().heights,
            "the same cut as played straight there"
        );
    }

    #[test]
    fn saves_are_few_for_a_large_material() {
        let (start, toolpath) = zigzag();
        let mut p = Playback::new(start, &[&toolpath], 60.0);
        p.seek(p.length());
        assert!(p.saves.len() <= MOST_SAVES, "{}", p.saves.len());
        assert!(
            saves_for(10_000 * 10_000) <= 2,
            "a huge grid gets few saves"
        );
    }

    #[test]
    fn the_revision_changes_only_when_the_material_does() {
        let (start, toolpath) = line();
        let mut p = Playback::new(start.clone(), &[&toolpath], 60.0);
        let other = Playback::new(start, &[&toolpath], 60.0);
        assert_ne!(p.revision(), other.revision(), "no two playbacks share one");
        let first = p.revision();
        p.seek(47.0);
        let cut = p.revision();
        assert_ne!(cut, first, "cut further");
        p.seek(47.0);
        assert_eq!(p.revision(), cut, "nothing moved");
        p.seek(20.0);
        assert_ne!(p.revision(), cut, "back again");
    }

    #[test]
    fn a_long_seek_stops_when_out_of_time_and_goes_on_from_there() {
        let (start, toolpath) = line();
        let mut p = Playback::new(start, &[&toolpath], 60.0);
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
    fn a_seek_back_goes_from_a_save_and_uncuts_what_is_past_it() {
        let (start, toolpath) = line();
        let mut p = Playback::new(start, &[&toolpath], 60.0);
        p.seek(p.length());
        assert!(
            p.seek_until(7.0 + 40.0, || true),
            "from the save after the plunge, in one go"
        );
        assert_eq!(p.at(), 47.0);
        assert!(depth_at(&p, 80.0) > -0.5, "uncut again");
        assert!(depth_at(&p, 15.0) < -1.0);
    }
}
