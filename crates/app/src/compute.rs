//! The grid kernels on the GPU: the tool dropped onto a height grid, swept along moves and
//! checked for gouging, as WGSL compute shaders (`compute.wgsl`) on the window's device,
//! registered with core's [`cut_craft_core::compute`] when the window has a card. Every call
//! uploads its grid and items and reads its answers back: the kernels' callers want them on
//! the CPU (toolpath moves for G-code, a preview's heights for the view's picking).
// ponytail: a grid is uploaded per call (4 MB at 1200 cells, under a millisecond); keep it
// on the card between calls if the uploads ever show in a profile. A thread a move: a few
// long moves (a roughing preview, 2.7k of them) give a dispatch little to run in parallel,
// so it comes out behind the CPU's 6 ms; a thread a row of each move if that ever matters.

use cut_craft_core::compute::{Compute, Cpu};
use cut_craft_core::geometry::Point;
use cut_craft_core::glam::DVec3;
use cut_craft_core::simulation::Material;
use cut_craft_core::toolpath::{Move, Tool, ToolShape};
use eframe::egui_wgpu::wgpu;
use wgpu::util::DeviceExt;

/// How far the card's answers (f32) may stray from the CPU's (f64): a micrometre, a
/// thousandth of a preview cell and under what a machine resolves. The tests see a tenth of
/// that at most.
#[cfg(test)]
const TOLERANCE: f64 = 0.001;
/// The `Params` uniform's size, as `compute.wgsl` lays it out.
const PARAMS_BYTES: usize = 80;
/// A move's record: two corners, the reach below the top, padding.
const MOVE_WORDS: usize = 8;
/// Threads a workgroup, as the shaders say.
const WORKGROUP: u32 = 64;
/// The most workgroups a dispatch may have along one axis.
const MOST_GROUPS: u32 = 65_535;
/// The shaders' "no answer" (the overcut of a move over no cell is minus infinity).
const NONE: f32 = 1.0e30;

/// The grid kernels on the window's card.
pub struct GpuCompute {
    device: wgpu::Device,
    queue: wgpu::Queue,
    drop: wgpu::ComputePipeline,
    sweep: wgpu::ComputePipeline,
    overcut: wgpu::ComputePipeline,
}

impl GpuCompute {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("compute"),
            source: wgpu::ShaderSource::Wgsl(include_str!("compute.wgsl").into()),
        });
        let pipeline = |entry: &'static str| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: None,
                module: &module,
                entry_point: Some(entry),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            })
        };
        Self {
            drop: pipeline("drop_main"),
            sweep: pipeline("sweep_main"),
            overcut: pipeline("overcut_main"),
            device,
            queue,
        }
    }

    /// Runs `pipeline` once over `count` items (`input`, at its binding) with the grid's
    /// heights, and reads back `out` f32s the kernel wrote, or the grid itself when `out`
    /// is `None` (the kernel then writes the grid).
    fn run(
        &self,
        pipeline: &wgpu::ComputePipeline,
        params: &[u8],
        grid: &Material,
        (input_binding, input): (u32, &[u8]),
        count: usize,
        out: Option<usize>,
    ) -> Vec<f32> {
        #[expect(clippy::cast_possible_truncation, reason = "the GPU takes f32")]
        let heights: Vec<u8> = grid
            .heights
            .iter()
            .flat_map(|&h| (h as f32).to_ne_bytes())
            .collect();
        let buffer = |label, contents: &[u8], usage| {
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(label),
                    contents,
                    usage,
                })
        };
        let storage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC;
        let params = buffer("compute params", params, wgpu::BufferUsages::UNIFORM);
        let grid = buffer("compute grid", &heights, storage);
        let input = buffer("compute input", input, storage);
        let read = out.map(|n| {
            self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("compute out"),
                size: (n * 4) as u64,
                usage: storage,
                mapped_at_creation: false,
            })
        });
        let mut entries = vec![
            entry(0, &params),
            entry(if read.is_some() { 1 } else { 2 }, &grid),
            entry(input_binding, &input),
        ];
        if let Some(read) = &read {
            entries.push(entry(5, read));
        }
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("compute"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &entries,
        });
        let source = read.as_ref().unwrap_or(&grid);
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("compute staging"),
            size: source.size(),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("compute"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("compute"),
                timestamp_writes: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            let groups = u32::try_from(count).expect("fits u32").div_ceil(WORKGROUP);
            pass.dispatch_workgroups(groups.min(MOST_GROUPS), groups.div_ceil(MOST_GROUPS), 1);
        }
        encoder.copy_buffer_to_buffer(source, 0, &staging, 0, source.size());
        self.queue.submit([encoder.finish()]);
        staging.slice(..).map_async(wgpu::MapMode::Read, |result| {
            result.expect("the readback maps");
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("the card finishes");
        let answers = {
            let view = staging.get_mapped_range(..).expect("mapped");
            view.as_chunks::<4>()
                .0
                .iter()
                .map(|b| f32::from_ne_bytes(*b))
                .collect()
        };
        staging.unmap();
        answers
    }
}

/// A buffer bound whole at `binding`.
fn entry(binding: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: buffer.as_entire_binding(),
    }
}

/// The `Params` uniform: the grid, `tool`'s profile, and `count` items.
#[expect(clippy::cast_possible_truncation, reason = "the GPU takes f32")]
fn params(grid: &Material, tool: &Tool, count: usize, centres: bool) -> Vec<u8> {
    let radius = tool.diameter / 2.0;
    let (taper, turn, rise) = match tool.tapered() {
        // a ball meets its taper where the ball is as steep as the taper
        Some(a) if tool.shape == ToolShape::BallNose => {
            (a.tan(), radius * a.cos(), radius * (1.0 - a.sin()))
        }
        Some(a) => (a.tan(), radius, 0.0),
        None => (0.0, radius, 0.0),
    };
    let half_angle_slope = (tool.angle.to_radians() / 2.0).tan();
    let shape = match tool.shape {
        ToolShape::EndMill | ToolShape::SurfacingBit => 0,
        ToolShape::BallNose => 1,
        ToolShape::VBit => 2,
        ToolShape::RadiusedEndMill => 3,
        ToolShape::ThreadMill => 4,
        ToolShape::FormTool => unreachable!("a form tool's profile runs on the CPU"),
    };
    let flat_end = tool.shape.is_flat() && tool.reach() <= radius;
    let mut bytes = Vec::with_capacity(PARAMS_BYTES);
    let words = |bytes: &mut Vec<u8>, values: &[u32]| {
        for v in values {
            bytes.extend_from_slice(&v.to_ne_bytes());
        }
    };
    let floats = |bytes: &mut Vec<u8>, values: &[f64]| {
        for v in values {
            bytes.extend_from_slice(&(*v as f32).to_ne_bytes());
        }
    };
    let word = |n: usize| u32::try_from(n).expect("fits u32");
    words(
        &mut bytes,
        &[word(grid.columns), word(grid.rows), word(count), shape],
    );
    floats(&mut bytes, &[grid.cell, grid.bottom, radius, tool.reach()]);
    floats(
        &mut bytes,
        &[taper, turn, rise, tool.tip_radius.clamp(0.0, radius)],
    );
    floats(
        &mut bytes,
        &[
            half_angle_slope,
            radius - tool.tooth_size.clamp(0.0, radius),
            tool.tooth_offset,
            half_angle_slope,
        ],
    );
    words(&mut bytes, &[u32::from(centres), u32::from(flat_end), 0, 0]);
    debug_assert_eq!(bytes.len(), PARAMS_BYTES);
    bytes
}

/// Moves as the shaders read them, from the grid's origin, each with how far `tool`
/// reaches below the top along it.
#[expect(clippy::cast_possible_truncation, reason = "the GPU takes f32")]
fn move_records(grid: &Material, tool: &Tool, moves: &[(DVec3, DVec3)]) -> Vec<u8> {
    let origin = grid.origin.extend(0.0);
    let mut bytes = Vec::with_capacity(moves.len() * MOVE_WORDS * 4);
    for &(a, b) in moves {
        let (a, b) = (a - origin, b - origin);
        let reach = tool.reach_below(-a.z.min(b.z));
        for v in [a.x, a.y, a.z, reach, b.x, b.y, b.z, 0.0] {
            bytes.extend_from_slice(&(v as f32).to_ne_bytes());
        }
    }
    bytes
}

impl Compute for GpuCompute {
    #[expect(clippy::cast_possible_truncation, reason = "the GPU takes f32")]
    fn drop_at(&self, grid: &Material, tool: &Tool, points: &[Point]) -> Vec<f64> {
        // ponytail: a form tool's profile is the CPU's; the shader knows the fixed shapes
        if tool.shape == ToolShape::FormTool {
            return Cpu.drop_at(grid, tool, points);
        }
        if points.is_empty() {
            return vec![];
        }
        let input: Vec<u8> = points
            .iter()
            .flat_map(|p| {
                let p = *p - grid.origin;
                [(p.x as f32).to_ne_bytes(), (p.y as f32).to_ne_bytes()]
            })
            .flatten()
            .collect();
        let n = points.len();
        self.run(
            &self.drop,
            &params(grid, tool, n, false),
            grid,
            (3, &input),
            n,
            Some(n),
        )
        .into_iter()
        .map(f64::from)
        .collect()
    }

    fn drop_everywhere(&self, grid: &Material, tool: &Tool) -> Vec<f64> {
        // ponytail: a form tool's profile is the CPU's; the shader knows the fixed shapes
        if tool.shape == ToolShape::FormTool {
            return Cpu.drop_everywhere(grid, tool);
        }
        let n = grid.columns * grid.rows;
        if n == 0 {
            return vec![];
        }
        self.run(
            &self.drop,
            &params(grid, tool, n, true),
            grid,
            (3, &[0; 8]),
            n,
            Some(n),
        )
        .into_iter()
        .map(f64::from)
        .collect()
    }

    fn overcut_along(&self, grid: &Material, tool: &Tool, moves: &[(DVec3, DVec3)]) -> Vec<f64> {
        // ponytail: a form tool's profile is the CPU's; the shader knows the fixed shapes
        if tool.shape == ToolShape::FormTool {
            return Cpu.overcut_along(grid, tool, moves);
        }
        if moves.is_empty() {
            return vec![];
        }
        let n = moves.len();
        self.run(
            &self.overcut,
            &params(grid, tool, n, false),
            grid,
            (4, &move_records(grid, tool, moves)),
            n,
            Some(n),
        )
        .into_iter()
        .map(|worst| {
            if worst <= -NONE {
                f64::NEG_INFINITY
            } else {
                f64::from(worst)
            }
        })
        .collect()
    }

    fn cut_along(&self, material: &mut Material, tool: &Tool, moves: &[Move]) {
        // ponytail: a form tool's profile is the CPU's; the shader knows the fixed shapes
        if tool.shape == ToolShape::FormTool {
            return Cpu.cut_along(material, tool, moves);
        }
        let pairs: Vec<(DVec3, DVec3)> = moves.windows(2).map(|w| (w[0].to, w[1].to)).collect();
        if pairs.is_empty() || material.heights.is_empty() {
            return;
        }
        let heights = self.run(
            &self.sweep,
            &params(material, tool, pairs.len(), false),
            material,
            (4, &move_records(material, tool, &pairs)),
            pairs.len(),
            None,
        );
        material.heights = heights.into_iter().map(f64::from).collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::compute::Compute;
    use cut_craft_core::geometry::Point;
    use cut_craft_core::glam::DVec3;
    use cut_craft_core::simulation::Material;
    use cut_craft_core::toolpath::{Move, MoveKind, Tool, ToolShape};
    use eframe::egui_wgpu::wgpu;

    /// Waits for a wgpu future: on native they are ready as soon as they are polled.
    fn block_on<F: Future>(f: F) -> F::Output {
        let mut f = std::pin::pin!(f);
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        loop {
            if let std::task::Poll::Ready(out) = f.as_mut().poll(&mut cx) {
                return out;
            }
            std::thread::yield_now();
        }
    }

    /// The card to test on, or none: headless, the tests then pass without testing.
    fn gpu() -> Option<GpuCompute> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter =
            block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).ok()?;
        let (device, queue) =
            block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;
        eprintln!("testing on {}", adapter.get_info().name);
        Some(GpuCompute::new(device, queue))
    }

    /// 90 x 60 mm in 0.37 mm cells (no tool's reach is a whole number of them): a dome, a
    /// 6 mm deep slot with square walls, a slope, and a bump standing above the top.
    #[expect(clippy::cast_precision_loss, reason = "a small test grid")]
    fn relief() -> Material {
        let (columns, rows, cell) = (243, 162, 0.37);
        let mut m = Material {
            origin: Point::new(10.0, -5.0),
            columns,
            rows,
            cell,
            bottom: -12.0,
            heights: vec![0.0; columns * rows],
            cut_by: vec![0; columns * rows],
            cutting: 0,
            paint: vec![],
            other_side: None,
            under: vec![],
        };
        for row in 0..rows {
            for column in 0..columns {
                let p = m.centre(column, row) - m.origin;
                let dome = {
                    let r = p.distance(Point::new(25.0, 30.0));
                    (r < 12.0).then(|| (144.0 - r * r).sqrt() - 12.0)
                };
                let slot = (p.y > 8.0 && p.y < 14.0 && p.x > 40.0 && p.x < 80.0).then_some(-6.0);
                let slope = (p.x > 55.0 && p.y > 30.0).then(|| -(p.x - 55.0) * 0.2);
                let bump = {
                    let r = p.distance(Point::new(70.0, 50.0));
                    (r < 5.0).then_some(2.0 - r * 0.4)
                };
                let h = dome.or(slot).or(slope).or(bump).unwrap_or(0.0);
                m.heights[row * columns + column] = h + (row as f64 * 0.013).sin() * 0.05;
            }
        }
        m
    }

    fn tools() -> Vec<Tool> {
        let shaped = |name: &str, shape, diameter| Tool {
            name: name.into(),
            shape,
            diameter,
            ..Tool::default()
        };
        vec![
            shaped("end mill", ToolShape::EndMill, 6.0),
            shaped("ball nose", ToolShape::BallNose, 6.0),
            Tool {
                angle: 90.0,
                ..shaped("v-bit", ToolShape::VBit, 12.7)
            },
            Tool {
                tip_radius: 1.5,
                ..shaped("radiused", ToolShape::RadiusedEndMill, 8.0)
            },
            Tool {
                taper: 5.0,
                shank_diameter: 8.0,
                ..shaped("tapered ball", ToolShape::BallNose, 3.0)
            },
            Tool {
                angle: 60.0,
                tooth_size: 1.5,
                tooth_offset: 2.0,
                ..shaped("thread mill", ToolShape::ThreadMill, 8.0)
            },
        ]
    }

    /// `n` points over the relief and a little beyond its edges, the same every run.
    #[expect(clippy::cast_precision_loss, reason = "a small test count")]
    fn points(n: usize) -> Vec<Point> {
        let mut seed: u64 = 0x2545_F491_4F6C_DD1D;
        let mut next = || {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (seed >> 11) as f64 / (1u64 << 53) as f64
        };
        (0..n)
            .map(|_| Point::new(6.0 + 98.0 * next(), -9.0 + 68.0 * next()))
            .collect()
    }

    /// A path over the relief: a wave rising and falling, a plunge, a pass above the top, a
    /// long steep diagonal and a level move.
    fn path() -> Vec<Move> {
        let origin = DVec3::new(10.0, -5.0, 0.0);
        let mut points: Vec<DVec3> = (0..60)
            .map(|i| {
                let t = f64::from(i) / 59.0;
                DVec3::new(
                    4.0 + 82.0 * t,
                    30.0 + 24.0 * (t * 9.0).sin(),
                    -1.0 - 3.0 * (t * 5.0).cos(),
                )
            })
            .collect();
        points.extend([
            DVec3::new(45.0, 30.0, 5.0),
            DVec3::new(45.0, 30.0, -4.0),
            DVec3::new(10.0, 10.0, 5.0),
            DVec3::new(80.0, 50.0, 5.0),
            DVec3::new(5.0, 55.0, 0.0),
            DVec3::new(85.0, 5.0, -6.0),
            DVec3::new(5.0, 20.0, -2.0),
            DVec3::new(85.0, 20.0, -2.0),
        ]);
        points
            .into_iter()
            .map(|to| Move {
                kind: MoveKind::Cut,
                to: to + origin,
            })
            .collect()
    }

    /// The most `b` strays from `a`, cell by cell (infinities count as equal).
    fn worst(a: &[f64], b: &[f64]) -> f64 {
        assert_eq!(a.len(), b.len());
        a.iter()
            .zip(b)
            .map(|(&x, &y)| {
                if x.total_cmp(&y).is_eq() {
                    0.0
                } else {
                    (x - y).abs()
                }
            })
            .fold(0.0, f64::max)
    }

    /// Windows' DirectX shader compiler (FXC) refuses a function whose value comes back only
    /// from inside a switch or an if/else ("not all control paths return a value"), even
    /// when every branch returns, and the kernels then fail to build on the card. Every
    /// function that gives a value ends in a plain return, so FXC sees one on every path.
    #[test]
    fn every_shader_function_ends_in_a_return_so_directx_builds_it() {
        let module = naga::front::wgsl::parse_str(include_str!("compute.wgsl"))
            .unwrap_or_else(|e| panic!("{e}"));
        let unended: Vec<&str> = module
            .functions
            .iter()
            .map(|(_, f)| f)
            .filter(|f| f.result.is_some())
            .filter(|f| !matches!(f.body.last(), Some(naga::Statement::Return { .. })))
            .map(|f| f.name.as_deref().unwrap_or("?"))
            .collect();
        assert!(
            unended.is_empty(),
            "these end without a plain return: {unended:?}"
        );
    }

    #[test]
    fn the_gpu_drops_the_tool_where_the_cpu_does() {
        let Some(gpu) = gpu() else {
            eprintln!("no GPU: skipped");
            return;
        };
        let relief = relief();
        let points = points(3000);
        for tool in tools() {
            let (cpu_at, gpu_at) = (
                Cpu.drop_at(&relief, &tool, &points),
                gpu.drop_at(&relief, &tool, &points),
            );
            let strays = worst(&cpu_at, &gpu_at);
            eprintln!("{}: at points strays {strays:.2e} mm", tool.name);
            assert!(strays <= TOLERANCE, "{}: {strays} mm at points", tool.name);
            let (cpu_all, gpu_all) = (
                Cpu.drop_everywhere(&relief, &tool),
                gpu.drop_everywhere(&relief, &tool),
            );
            let strays = worst(&cpu_all, &gpu_all);
            eprintln!("{}: everywhere strays {strays:.2e} mm", tool.name);
            assert!(strays <= TOLERANCE, "{}: {strays} mm everywhere", tool.name);
            assert!(cpu_all.iter().any(|&z| z < -1.0), "something to drop into");
        }
    }

    #[test]
    fn the_gpu_cuts_what_the_cpu_cuts() {
        let Some(gpu) = gpu() else {
            eprintln!("no GPU: skipped");
            return;
        };
        let path = path();
        for tool in tools() {
            let (mut cpu_cut, mut gpu_cut) = (relief(), relief());
            Cpu.cut_along(&mut cpu_cut, &tool, &path);
            gpu.cut_along(&mut gpu_cut, &tool, &path);
            let strays = worst(&cpu_cut.heights, &gpu_cut.heights);
            eprintln!("{}: the cut strays {strays:.2e} mm", tool.name);
            assert!(strays <= TOLERANCE, "{}: {strays} mm", tool.name);
            let cut = cpu_cut
                .heights
                .iter()
                .zip(&relief().heights)
                .filter(|(a, b)| a < b)
                .count();
            assert!(cut > 1000, "{}: only {cut} cells cut", tool.name);
        }
    }

    #[test]
    fn the_gpu_finds_the_overcut_the_cpu_finds() {
        let Some(gpu) = gpu() else {
            eprintln!("no GPU: skipped");
            return;
        };
        let relief = relief();
        let path = path();
        let moves: Vec<(DVec3, DVec3)> = path.windows(2).map(|w| (w[0].to, w[1].to)).collect();
        for tool in tools() {
            let (cpu_over, gpu_over) = (
                Cpu.overcut_along(&relief, &tool, &moves),
                gpu.overcut_along(&relief, &tool, &moves),
            );
            let strays = worst(&cpu_over, &gpu_over);
            eprintln!("{}: the overcut strays {strays:.2e} mm", tool.name);
            assert!(strays <= TOLERANCE, "{}: {strays} mm", tool.name);
            assert!(
                cpu_over.iter().any(|&o| o > 1.0) && cpu_over.iter().any(|&o| o < 0.0),
                "{}: moves both gouging and clear",
                tool.name
            );
        }
    }

    /// `cargo test --release -p cut-craft -- --ignored compute_timing --nocapture`: a 6 mm
    /// ball nose finishing pass, a roughing pass and their previews over a 360 x 240 x 20 mm
    /// relief with three domes and a slot, 1200 cells across, on the CPU and on the card.
    #[test]
    #[ignore = "timing, by hand"]
    fn compute_timing() {
        use cut_craft_core::Command;
        use cut_craft_core::component::{Combine, Component};
        use cut_craft_core::compute::use_compute;
        use cut_craft_core::document::{Document, Job};
        use cut_craft_core::geometry::Rect;
        use cut_craft_core::toolpath::ToolDatabase;
        use std::sync::Arc;
        use std::time::Instant;
        let Some(gpu) = gpu() else {
            eprintln!("no GPU");
            return;
        };
        let gpu: Arc<dyn Compute> = Arc::new(gpu);
        let mut doc = Document {
            job: Job {
                width: 360.0,
                height: 240.0,
                thickness: 20.0,
                ..Job::default()
            },
            ..Document::default()
        };
        let origin = doc.job.area().min;
        let cell = doc.job.model_cell();
        let mut change = cut_craft_core::Change::builder(&doc);
        for x in [70.0, 180.0, 290.0] {
            let centre = origin + Point::new(x, 120.0);
            let area = Rect::new(centre - Point::splat(55.0), centre + Point::splat(55.0));
            change.add_component(Component::from_fn(
                "dome",
                Combine::Add,
                area,
                cell,
                move |p| {
                    let r = p.distance(centre);
                    (r < 55.0).then(|| (3025.0 - r * r).sqrt() * 0.25)
                },
            ));
        }
        let slot = Rect::new(
            origin + Point::new(20.0, 114.0),
            origin + Point::new(340.0, 126.0),
        );
        change.add_component(Component::from_fn(
            "slot",
            Combine::Subtract,
            slot,
            cell,
            |_| Some(6.0),
        ));
        doc.apply(&change.finish()).unwrap();
        let ball = ToolDatabase::starter()
            .get("6 mm Ball Nose")
            .unwrap()
            .clone();
        let finishing = cmd_3d_finishing_toolpath::Params {
            tools: vec![ball],
            ..Default::default()
        };
        let roughing = cmd_3d_roughing_toolpath::Params::default();
        for (name, on) in [("CPU", None), ("GPU", Some(gpu))] {
            use_compute(on);
            let time = |label: &str, make: &dyn Fn() -> cut_craft_core::Change| {
                let started = Instant::now();
                let change = make();
                let took = started.elapsed();
                let mut cut = doc.clone();
                cut.apply(&change).unwrap();
                let toolpath = cut.toolpaths.into_values().next().unwrap();
                let started = Instant::now();
                let mut material = Material::new(&doc.job, 1200);
                material.cut(&toolpath);
                eprintln!(
                    "{name}: {label} ({} moves) {took:?}; its preview at 1200 cells {:?}",
                    toolpath.moves.len(),
                    started.elapsed()
                );
            };
            time("3D finishing, 6 mm ball nose", &|| {
                cmd_3d_finishing_toolpath::FinishingToolpath3d::run(&doc, &finishing).unwrap()
            });
            time("3D roughing", &|| {
                cmd_3d_roughing_toolpath::RoughingToolpath3d::run(&doc, &roughing).unwrap()
            });
        }
        use_compute(None);
    }
}
