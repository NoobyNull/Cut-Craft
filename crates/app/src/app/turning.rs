//! A rotary job's preview as the machine runs it (help: Preview Toolpaths ▸ On a rotary
//! job): the toolpaths written by the machine's own post, read back move by move, and played
//! into the material as it really is (round, square or a board) turning with A while the
//! bit moves along, across and up and down. It keeps pace with the Preview Control, so Run,
//! Pause, the steps and dragging the position move it too.

use cut_craft_core::glam::DVec3;
use cut_craft_core::toolpath::Toolpath;
use cut_craft_rotary::gcode::{Reading, read};
use cut_craft_rotary::{Cutter, Replay, Speeds, Stock, Voxels};

/// About how many cells of material the turning preview cuts at the Detail slider's
/// `percent`: eight times the points a flat preview takes there, the material being solid
/// rather than a surface. Fine enough at the middle to show a finish's passes (a third of a
/// millimetre on a 300 mm by 80 mm blank), few enough to cut and draw as it plays.
fn cells(percent: u8) -> usize {
    8 * crate::preview_detail::most_points(percent)
}

/// What is needed to make a turning preview, taken from the window's thread.
pub(super) struct Making {
    post: cut_craft_post::Post,
    job: cut_craft_core::document::Job,
    reading: Reading,
    stock: Stock,
    speeds: Speeds,
    /// Cut first, at once: the toolpaths before these, when the preview carries on from
    /// what they cut.
    earlier: Vec<Toolpath>,
    toolpaths: Vec<Toolpath>,
    /// The Detail slider's percent: how finely the material is cut.
    detail: u8,
}

/// A playing turning preview, and its surface as last drawn.
pub(super) struct Turning {
    pub replay: Replay,
    along_x: bool,
    surface: Option<Drawn>,
}

/// The material's surface in the part's frame, made from the replay's `revision`, and
/// when.
struct Drawn {
    revision: u64,
    when: std::time::Instant,
    points: Vec<DVec3>,
    triangles: Vec<[u32; 3]>,
}

impl Making {
    /// The turning preview of `toolpaths` for a rotary job saved through `post` on a machine
    /// whose rotary is `axis` (the default without one), its rapids at `rapid` mm/min.
    /// `None` for a flat job or a post that does not wrap onto A.
    pub(super) fn new(
        job: &cut_craft_core::document::Job,
        post: cut_craft_post::Post,
        axis: &cut_craft_core::rotary_axis::RotaryAxis,
        rapid: f64,
        earlier: Vec<Toolpath>,
        toolpaths: Vec<Toolpath>,
        detail: u8,
    ) -> Option<Self> {
        post.wraps()?;
        Some(Self {
            post,
            job: *job,
            reading: Reading::of(job, axis)?,
            stock: Stock::of(job)?,
            speeds: Speeds {
                rapid: if rapid > 0.0 { rapid } else { 5000.0 },
                turn: if axis.max_rate > 0.0 {
                    axis.max_rate
                } else {
                    3600.0
                },
            },
            earlier,
            toolpaths,
            detail,
        })
    }

    /// Writes each toolpath's file and reads it back, cuts the earlier ones into the
    /// material, and lays the rest out to play. A toolpath the post can't write is left out.
    pub(super) fn make(self) -> Turning {
        let mut start = Voxels::of(
            &self.stock,
            Voxels::cell_for(&self.stock, cells(self.detail)),
        );
        let read_back = |toolpath: &Toolpath| {
            let mut doc = cut_craft_core::document::Document {
                job: self.job,
                ..cut_craft_core::document::Document::default()
            };
            doc.toolpaths
                .insert(cut_craft_core::document::EntityId(1), toolpath.clone());
            let file = cut_craft_post::File {
                path: std::path::Path::new("preview.nc"),
                job: None,
                made: chrono::NaiveDateTime::default(),
                slots: &[],
                notes: &[],
            };
            let text = self.post.render(&doc, &file).ok()?;
            Some((Cutter::new(&toolpath.tool), read(&text, &self.reading)))
        };
        for (cutter, moves) in self.earlier.iter().filter_map(read_back) {
            for pair in moves.windows(2) {
                start.cut(&cutter, pair[0].to, pair[1].to);
            }
        }
        let parts = self.toolpaths.iter().filter_map(read_back).collect();
        Turning {
            replay: Replay::new(start, parts, self.speeds),
            along_x: self
                .job
                .rotary
                .is_none_or(|r| r.along == cut_craft_core::document::Along::X),
            surface: None,
        }
    }
}

/// How many times the drawn surface's corners are drawn in towards their neighbours, so
/// its cells' steps read as the smooth surface they cut (only as drawn: what is cut stays
/// exactly as the file cuts it).
const SMOOTHING: usize = 2;

/// `points` each moved half way to the middle of those it shares a triangle edge with,
/// `rounds` times over.
fn smoothed(mut points: Vec<DVec3>, triangles: &[[u32; 3]], rounds: usize) -> Vec<DVec3> {
    for _ in 0..rounds {
        let mut sum = vec![DVec3::ZERO; points.len()];
        let mut count = vec![0u32; points.len()];
        for t in triangles {
            for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                let (a, b) = (a as usize, b as usize);
                sum[a] += points[b];
                sum[b] += points[a];
                count[a] += 1;
                count[b] += 1;
            }
        }
        for (i, p) in points.iter_mut().enumerate() {
            if count[i] > 0 {
                *p = (*p + sum[i] / f64::from(count[i])) / 2.0;
            }
        }
    }
    points
}

impl Turning {
    /// A point of the machine's frame (along, across, height above the axis) in the 3D
    /// view's wrapped coordinates (the axis along x, or y, through the origin).
    fn in_view(&self, m: DVec3) -> DVec3 {
        if self.along_x {
            m
        } else {
            DVec3::new(m.y, m.x, m.z)
        }
    }

    /// Keeps up with the Preview Control at `fraction` of the way through, cutting for no
    /// longer than `budget`. Whether it got there.
    pub(super) fn follow(&mut self, fraction: f64, budget: std::time::Duration) -> bool {
        let goal = fraction.clamp(0.0, 1.0) * self.replay.length();
        let until = std::time::Instant::now() + budget;
        self.replay
            .seek_until(goal, || std::time::Instant::now() >= until)
    }

    /// The material's faces as the machine holds it now, A turned, in the view's
    /// coordinates: its surface made again when it has been cut and `redraw` has passed (or
    /// it has stopped, `settled`).
    pub(super) fn faces(
        &mut self,
        redraw: std::time::Duration,
        settled: bool,
    ) -> Vec<cmd_orientate_3d_model::Seen> {
        let revision = self.replay.revision();
        let stale = self
            .surface
            .as_ref()
            .is_none_or(|d| d.revision != revision && (settled || d.when.elapsed() >= redraw));
        if stale {
            let (points, triangles) = self.replay.material().surface();
            let points = smoothed(points, &triangles, SMOOTHING);
            self.surface = Some(Drawn {
                revision,
                when: std::time::Instant::now(),
                points,
                triangles,
            });
        }
        let turn = self.replay.bit().map_or(0.0, |(pose, _)| pose.turn);
        let Some(Drawn {
            points, triangles, ..
        }) = &self.surface
        else {
            return vec![];
        };
        let placed: Vec<DVec3> = points
            .iter()
            .map(|&p| self.in_view(cut_craft_rotary::to_machine(p, turn)))
            .collect();
        triangles
            .iter()
            .map(|t| (t.map(|i| placed[i as usize]), false))
            .collect()
    }

    /// Where the bit's tip is in the view, and the bit.
    pub(super) fn bit(&self) -> Option<(DVec3, &cut_craft_core::toolpath::Tool)> {
        let (pose, cutter) = self.replay.bit()?;
        let tip = self.in_view(DVec3::new(pose.along, pose.across, pose.height));
        Some((tip, cutter.tool()))
    }
}

impl super::CutCraftApp {
    /// What the turning preview of `toolpaths` needs, after `earlier` are cut: on a rotary
    /// job saved through a post that wraps onto A, for the active machine's rotary. `None`
    /// otherwise, or when the job can't be saved for this machine.
    pub(super) fn turning_making(
        &self,
        earlier: Vec<Toolpath>,
        toolpaths: Vec<Toolpath>,
    ) -> Option<Making> {
        let job = self.doc.job;
        job.rotary?;
        let post = self.output_post().ok()?;
        let axis = (self.machines.active())
            .filter(|m| m.has_rotary())
            .map_or_default(|m| m.rotary_axis);
        // thinned as Save Toolpaths thins them, so the preview plays the saved file
        let settings = &self.options.toolpaths;
        let thin = |t: Vec<Toolpath>| -> Vec<Toolpath> {
            t.iter()
                .map(|t| t.thinned(settings.tolerance(cut_craft_core::simplify::family(t))))
                .collect()
        };
        Making::new(
            &job,
            post,
            &axis,
            self.summary.rapid,
            thin(earlier),
            thin(toolpaths),
            self.display.detail,
        )
    }

    /// Once a frame: the turning preview kept at the same point through the job as the
    /// Preview Control, cutting for no longer than the frame's budget; gone once the
    /// preview's controls close.
    pub(super) fn follow_turning(&mut self, ctx: &egui::Context) {
        let Some(playback) = self.playback.as_ref().or(self.kept_playback.as_ref()) else {
            self.turning = None;
            return;
        };
        let fraction = if playback.length() > 0.0 {
            playback.at() / playback.length()
        } else {
            1.0
        };
        if let Some(turning) = self.turning.as_mut()
            && !turning.follow(fraction, self.playback_budget)
        {
            ctx.request_repaint();
        }
    }

    /// The turning preview in the 3D view: the material as the machine holds it now, given
    /// to the GPU when the view has one (else painted), and the bit standing over it where
    /// the machine has it.
    pub(super) fn paint_turning(
        &mut self,
        ui: &egui::Ui,
        area: egui::Rect,
        camera: &super::view3d::Camera,
        look: &super::view3d::Look,
        on_gpu: bool,
    ) -> Option<super::view3d::gpu::Draw> {
        let settled = !self.playing && self.playback_goal.is_none();
        let redraw = self.playback_redraw;
        let turning = self.turning.as_mut()?;
        let faces = turning.faces(redraw, settled);
        let mark = ui.visuals().error_fg_color;
        let painter = ui.painter_at(area);
        let draw = if on_gpu {
            let geometry = std::sync::Arc::new(super::view3d::gpu::model(&faces));
            let at = DVec3::ZERO;
            Some(super::view3d::gpu::Draw::new(
                geometry, camera, area, at, look, mark,
            ))
        } else {
            let screen = |p| camera.screen(p);
            painter.add(super::view3d::triangles(
                &faces, screen, self.orbit, look, mark,
            ));
            None
        };
        if let Some((tip, tool)) = turning.bit() {
            let parts = super::view3d::tool_parts(tool);
            let selection = ui.visuals().selection;
            for (i, rings) in parts.iter().enumerate() {
                let fill = if i == 0 {
                    selection.bg_fill
                } else {
                    super::TOOL_SHANK
                };
                painter.add(egui::Shape::convex_polygon(
                    super::view3d::turned_outline(|p| camera.screen(p), tip, rings),
                    fill,
                    selection.stroke,
                ));
            }
        }
        draw
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::document::{Along, Job};
    use cut_craft_core::toolpath::{Move, MoveKind, Tool, ToolShape};

    /// A ring cut round a 40 mm dowel, 5 mm deep, from 20 to 30 along.
    fn ring(along: Along) -> (Job, Toolpath) {
        let job = Job::rotary(60.0, 40.0, along);
        let round = std::f64::consts::PI * 40.0;
        let at = |a: f64, r: f64, z: f64, kind| {
            let (x, y) = match along {
                Along::X => (a, r),
                Along::Y => (r, a),
            };
            Move {
                to: DVec3::new(x, y, z),
                kind,
            }
        };
        let toolpath = Toolpath {
            name: "Ring".into(),
            strategy: String::new(),
            params: serde_json::Value::Null,
            vectors: vec![],
            visible: true,
            power: vec![],
            stale: false,
            group: None,
            drill_bank: None,
            fallbacks: vec![],
            across: vec![],
            tool: Tool {
                shape: ToolShape::EndMill,
                diameter: 6.0,
                flute_length: 20.0,
                feed_rate: 1000.0,
                plunge_rate: 300.0,
                spindle_speed: 18000.0,
                ..Tool::default()
            },
            moves: vec![
                at(25.0, 0.0, 5.0, MoveKind::Rapid),
                at(25.0, 0.0, -5.0, MoveKind::Plunge),
                at(25.0, round, -5.0, MoveKind::Cut),
                at(25.0, round, 5.0, MoveKind::Rapid),
            ],
        };
        (job, toolpath)
    }

    #[test]
    fn the_material_is_cut_as_finely_as_the_detail_slider_says() {
        let (job, toolpath) = ring(Along::X);
        let cell = |detail| {
            let post = cut_craft_post::Post::default_for(&job);
            let axis = cut_craft_core::rotary_axis::RotaryAxis::default();
            Making::new(
                &job,
                post,
                &axis,
                5000.0,
                vec![],
                vec![toolpath.clone()],
                detail,
            )
            .unwrap()
            .make()
            .replay
            .material()
            .cell()
        };
        let (coarse, middle, fine) = (cell(0), cell(50), cell(100));
        assert!(coarse > middle && middle > fine, "{coarse} {middle} {fine}");
        // a 300 mm by 80 mm blank at the middle: finer than a finish's passes are apart
        let blank = 300.0 * 80.0 * 80.0;
        let cells = u32::try_from(cells(50)).unwrap();
        assert!((blank / f64::from(cells)).cbrt() < 0.4);
    }

    fn turning(along: Along) -> Turning {
        let (job, toolpath) = ring(along);
        let post = cut_craft_post::Post::default_for(&job);
        let axis = cut_craft_core::rotary_axis::RotaryAxis::default();
        Making::new(&job, post, &axis, 5000.0, vec![], vec![toolpath], 50)
            .expect("a rotary job and a wrapping post")
            .make()
    }

    #[test]
    fn a_ring_written_and_read_back_turns_the_dowel_a_whole_turn_under_the_bit() {
        for along in [Along::X, Along::Y] {
            let mut t = turning(along);
            assert!(t.replay.length() > 0.0);
            assert!(t.follow(1.0, std::time::Duration::from_secs(60)));
            let material = t.replay.material();
            // round the ring 5 mm deep is gone, all the way round; either side it stands
            for degrees in (0..360).step_by(30) {
                let a = f64::from(degrees).to_radians();
                let p = |along: f64, r: f64| DVec3::new(along, r * a.sin(), r * a.cos());
                assert!(!material.solid(p(25.0, 17.0)), "{along:?} {degrees}°");
                assert!(material.solid(p(25.0, 13.0)), "{along:?} {degrees}°");
                assert!(material.solid(p(10.0, 19.0)), "{along:?} {degrees}°");
            }
            // at the end the file sends the bit home, clear of the material
            let (tip, tool) = t.bit().unwrap();
            assert!((tool.diameter - 6.0).abs() < 1e-12);
            let along_view = if along == Along::X { tip.x } else { tip.y };
            assert!(along_view.abs() < 1e-6 && tip.z > 20.0, "{along:?} {tip}");
            // part way round, it is over the ring, 5 deep, along the job's own axis
            t.follow(0.5, std::time::Duration::from_secs(60));
            let (tip, _) = t.bit().unwrap();
            let along_view = if along == Along::X { tip.x } else { tip.y };
            assert!((along_view - 25.0).abs() < 1e-6, "{along:?} {tip}");
            assert!((tip.z - 15.0).abs() < 1e-6, "{along:?} {tip}");
        }
    }

    #[test]
    fn half_way_the_material_has_turned_half_way_with_a() {
        let mut t = turning(Along::X);
        // part way round the ring, the bit is still cutting and A has turned
        t.follow(0.6, std::time::Duration::from_secs(60));
        let (pose, _) = t.replay.bit().unwrap();
        assert!(
            pose.turn > 0.5 && pose.turn < std::f64::consts::TAU,
            "{pose:?}"
        );
        let faces = t.faces(std::time::Duration::ZERO, true);
        assert_ne!(faces, Vec::new());
        // the faces are the material turned: none reaches further out than the dowel
        for (tri, _) in &faces {
            for p in tri {
                assert!(p.y.hypot(p.z) < 20.0 + 2.0 * t.replay.material().cell());
            }
        }
    }

    #[test]
    fn a_flat_job_or_a_flat_post_has_no_turning_preview() {
        let (job, toolpath) = ring(Along::X);
        let axis = cut_craft_core::rotary_axis::RotaryAxis::default();
        let flat = cut_craft_post::Post::default_for(&Job::default());
        assert!(
            Making::new(
                &job,
                flat,
                &axis,
                5000.0,
                vec![],
                vec![toolpath.clone()],
                50
            )
            .is_none()
        );
        let post = cut_craft_post::Post::default_for(&job);
        assert!(
            Making::new(
                &Job::default(),
                post,
                &axis,
                5000.0,
                vec![],
                vec![toolpath],
                50
            )
            .is_none()
        );
    }
}
