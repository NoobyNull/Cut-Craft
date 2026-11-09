//! The 3D view drawn by the GPU: the material's height grid (and a model being imported)
//! go to the card once per change, the vertex shader builds the surface from the grid, and
//! a frame costs only a camera uniform and a draw, the depth buffer putting the near faces
//! in front. Lighting and colour are the shader's, so a light that follows the view costs
//! nothing either.
//!
//! The app owns the GPU state ([`Gpu`]) and the grid is a storage buffer, so a compute
//! step over the heights (tool drops, simulation sweeps) can share the device and queue and
//! write what the view reads.

use super::{
    AMBIENT, Camera, DEPTH_SHADE, GRAIN_BUNCHES, GRAIN_DEPTH_SHIFT, GRAIN_DRIFTS, GRAIN_SHARPNESS,
    Look,
};
use cut_craft_core::glam::{DMat4, DVec3};
use cut_craft_core::simulation::Material;
use eframe::egui::{Color32, Rect, Rgba, Shape};
use eframe::egui_wgpu::{self, CallbackResources, CallbackTrait, RenderState, wgpu};
use std::sync::Arc;

/// The window's depth buffer: the view's faces are sorted by it.
pub const DEPTH_BITS: u8 = 32;
/// The camera uniform's size: as `Camera` in `surface.wgsl` lays it out.
const UNIFORM_BYTES: usize = 224;
/// A height meaning no model there (the grid's NaN, which shaders cannot be trusted with).
const EMPTY: f32 = -1.0e30;
/// Numbers a model's face takes in its buffer: three corners and a mark.
const FACE_WORDS: usize = 10;
/// How far the depth range reaches either side of the middle, in bounding-sphere radii:
/// the camera is fitted to a coarser surface than the one drawn, so a peak it never saw
/// must still be inside.
const DEPTH_REACH: f64 = 2.0;

/// A surface's grid: cells across and up, a cell's edge and the block's bottom.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Grid {
    columns: u32,
    rows: u32,
    cell: f64,
    bottom: f64,
    /// The highest height on it: a ray to the light that has risen past it is in the open.
    top: f64,
}

/// What goes to the card: a surface's heights (and the triangles over them), or a model's
/// faces, as bytes ready to upload.
#[derive(Default)]
pub struct Geometry {
    data: Vec<u8>,
    /// The surface's triangles; a model's faces are drawn in order.
    indices: Vec<u8>,
    /// Vertices the shader makes: the grid's cells and its sides', or three a face.
    vertices: u32,
    grid: Option<Grid>,
    /// The surface's heights are followed by each cell's preview paint (`Material::paint`).
    painted: bool,
    /// Then by its underside's heights (`Material::under`), drawn as a grid of its own.
    undersided: bool,
}

impl Geometry {
    fn quad(&mut self, [a, b, c, d]: [u32; 4]) {
        for i in [a, b, c, a, c, d] {
            self.indices.extend_from_slice(&i.to_ne_bytes());
        }
    }

    #[cfg(test)]
    pub fn triangles(&self) -> usize {
        if self.grid.is_some() {
            self.indices.len() / 12
        } else {
            self.vertices as usize / 3
        }
    }

    /// Bytes on the card.
    #[cfg(test)]
    pub fn bytes(&self) -> usize {
        self.data.len() + self.indices.len()
    }

    /// What a draw counts: indices, or vertices in order.
    fn count(&self) -> u32 {
        if self.grid.is_some() {
            u32::try_from(self.indices.len() / 4).expect("fits u32")
        } else {
            self.vertices
        }
    }
}

/// The material's surface: its heights, and a quad over each four neighbouring cells that
/// have a model; with `sides` the block's sides and bottom; round the axis when `wrap`, the
/// way round closed on itself. The shader places and lights the vertices.
#[expect(clippy::cast_possible_truncation, reason = "millimetres fit f32")]
pub fn surface(m: &Material, wrap: Option<super::Wrap>, sides: bool) -> Geometry {
    let height = |c: usize, r: usize| m.heights[r * m.columns + c];
    let mut g = Geometry {
        grid: Some(Grid {
            columns: u32::try_from(m.columns).expect("a cell count"),
            rows: u32::try_from(m.rows).expect("a cell count"),
            cell: m.cell,
            bottom: m.bottom,
            top: m
                .heights
                .iter()
                .copied()
                .filter(|h| !h.is_nan())
                .fold(m.bottom, f64::max),
        }),
        ..Geometry::default()
    };
    for &h in &m.heights {
        let h = if h.is_nan() { EMPTY } else { h as f32 };
        g.data.extend_from_slice(&h.to_ne_bytes());
    }
    g.painted = !m.paint.is_empty();
    for &p in &m.paint {
        g.data.extend_from_slice(&p.to_ne_bytes());
    }
    g.undersided = !m.under.is_empty() && sides && wrap.is_none();
    for &h in m.under.iter().filter(|_| g.undersided) {
        let h = if h.is_nan() { EMPTY } else { h as f32 };
        g.data.extend_from_slice(&h.to_ne_bytes());
    }
    let floor = |c: usize, r: usize| m.under.get(r * m.columns + c).copied().unwrap_or(m.bottom);
    let index = |c: usize, r: usize| u32::try_from(r * m.columns + c).expect("fits u32");
    let (columns, rows) = match wrap {
        Some(w) if w.along_x => (m.columns.saturating_sub(1), m.rows),
        Some(_) => (m.columns, m.rows.saturating_sub(1)),
        None => (m.columns.saturating_sub(1), m.rows.saturating_sub(1)),
    };
    for r in 0..rows {
        for c in 0..columns {
            let (nc, nr) = ((c + 1) % m.columns, (r + 1) % m.rows);
            let corners = [(c, r), (nc, r), (nc, nr), (c, nr)];
            if corners.iter().any(|&(c, r)| height(c, r).is_nan()) {
                continue;
            }
            g.quad(corners.map(|(c, r)| index(c, r)));
        }
    }
    let cells = m.columns * m.rows;
    g.vertices = u32::try_from(cells).expect("fits u32");
    if sides && wrap.is_none() {
        // the shader's side vertices: a top and a bottom one for each edge cell going round
        // (front, back, left, right), then the bottom's four corners
        let edges = [
            (0..m.columns).map(|c| (c, 0)).collect::<Vec<_>>(),
            (0..m.columns).map(|c| (c, m.rows - 1)).collect(),
            (0..m.rows).map(|r| (0, r)).collect(),
            (0..m.rows).map(|r| (m.columns - 1, r)).collect(),
        ];
        let mut start = cells;
        for edge in edges {
            for (k, pair) in edge.windows(2).enumerate() {
                if pair
                    .iter()
                    .any(|&(c, r)| height(c, r).is_nan() || floor(c, r).is_nan())
                {
                    continue;
                }
                let top = u32::try_from(start + 2 * k).expect("fits u32");
                g.quad([top, top + 2, top + 3, top + 1]);
            }
            start += 2 * edge.len();
        }
        let bottom = u32::try_from(start).expect("fits u32");
        if g.undersided {
            // the underside's grid, wound the other way round to face down
            for r in 0..m.rows.saturating_sub(1) {
                for c in 0..m.columns.saturating_sub(1) {
                    let corners = [(c, r), (c, r + 1), (c + 1, r + 1), (c + 1, r)];
                    if corners.iter().any(|&(c, r)| floor(c, r).is_nan()) {
                        continue;
                    }
                    g.quad(corners.map(|(c, r)| bottom + index(c, r)));
                }
            }
            g.vertices = bottom + u32::try_from(cells).expect("fits u32");
        } else {
            g.quad([bottom, bottom + 1, bottom + 2, bottom + 3]);
            g.vertices = bottom + 4;
        }
    }
    g
}

/// An imported model's faces: three corners and whether the face is marked, each.
#[expect(clippy::cast_possible_truncation, reason = "millimetres fit f32")]
pub fn model(faces: &[cmd_orientate_3d_model::Seen]) -> Geometry {
    let mut g = Geometry::default();
    g.data.reserve(faces.len() * FACE_WORDS * 4);
    for (t, marked) in faces {
        for v in t.iter().flat_map(DVec3::to_array) {
            g.data.extend_from_slice(&(v as f32).to_ne_bytes());
        }
        g.data
            .extend_from_slice(&f32::from(u8::from(*marked)).to_ne_bytes());
    }
    g.vertices = u32::try_from(faces.len() * 3).expect("fits u32");
    g
}

/// One frame's drawing of a geometry: how the camera sees and colours it.
pub struct Draw {
    geometry: Arc<Geometry>,
    uniform: Vec<u8>,
}

impl Draw {
    /// `geometry` seen by `camera` over `area` (the callback's rect), moved by `offset` in
    /// the material's coordinates, coloured and lit as `look` says; a model's marked faces
    /// in `mark`, and its backs left out.
    #[expect(clippy::cast_possible_truncation, reason = "the GPU takes f32")]
    pub fn new(
        geometry: Arc<Geometry>,
        camera: &Camera,
        area: Rect,
        offset: DVec3,
        look: &Look,
        mark: Color32,
    ) -> Self {
        let clip = camera.clip(area, DEPTH_REACH) * DMat4::from_translation(offset);
        let grid = geometry.grid;
        let mut uniform = Vec::with_capacity(UNIFORM_BYTES);
        let floats = |uniform: &mut Vec<u8>, values: &[f64]| {
            for v in values {
                uniform.extend_from_slice(&(*v as f32).to_ne_bytes());
            }
        };
        let words = |uniform: &mut Vec<u8>, values: &[u32]| {
            for v in values {
                uniform.extend_from_slice(&v.to_ne_bytes());
            }
        };
        // linear, and not premultiplied: the shader lights the colour, then blends by its
        // alpha (see-through only for a ghost)
        let linear = |c: Color32| {
            let [r, g, b, a] = c.to_srgba_unmultiplied();
            let [r, g, b, _] = Rgba::from(Color32::from_rgb(r, g, b)).to_array();
            [r, g, b, f32::from(a) / 255.0].map(f64::from)
        };
        floats(&mut uniform, &clip.to_cols_array());
        floats(&mut uniform, &light_unit(look.light).to_array());
        floats(&mut uniform, &[AMBIENT]);
        floats(&mut uniform, &camera.orbit.towards_viewer().to_array());
        words(&mut uniform, &[u32::from(grid.is_none())]);
        floats(&mut uniform, &linear(look.colour));
        floats(&mut uniform, &linear(mark));
        let g = grid.unwrap_or(Grid {
            columns: 0,
            rows: 0,
            cell: 0.0,
            bottom: 0.0,
            top: 0.0,
        });
        words(&mut uniform, &[g.columns, g.rows]);
        floats(&mut uniform, &[g.cell, g.bottom]);
        let wrap = look.wrap.filter(|_| grid.is_some());
        // a flat surface's shadows are traced in the shader, up to its highest point
        let shadows = look.shadows && wrap.is_none() && grid.is_some();
        words(
            &mut uniform,
            &[wrap.map_or(0, |w| if w.along_x { 1 } else { 2 })],
        );
        floats(
            &mut uniform,
            &[
                wrap.map_or(0.0, |w| w.radius),
                wrap.map_or(if shadows { g.top } else { 0.0 }, |w| w.surface),
                DEPTH_SHADE,
            ],
        );
        let grain = look
            .grain
            .filter(|_| look.wrap.is_none())
            .map(|grain| grain.on_cells(g.cell));
        for [first, second] in [GRAIN_DRIFTS, GRAIN_BUNCHES] {
            floats(&mut uniform, &[first.0, first.1, second.0, second.1]);
        }
        floats(
            &mut uniform,
            &[
                f64::from(GRAIN_SHARPNESS),
                GRAIN_DEPTH_SHIFT,
                grain.map_or(1.0, |grain| grain.spacing),
                grain.map_or(0.0, |grain| f64::from(grain.depth)),
            ],
        );
        words(
            &mut uniform,
            &[
                u32::from(grid.is_none()),
                u32::from(geometry.painted),
                u32::from(geometry.undersided),
                u32::from(shadows),
            ],
        );
        debug_assert_eq!(uniform.len(), UNIFORM_BYTES);
        Self { geometry, uniform }
    }
}

/// `light` at unit length.
fn light_unit(light: DVec3) -> DVec3 {
    light.normalize_or(super::LIGHT.normalize())
}

/// The card's copy of a geometry, with its camera. The heights (or the faces) are a storage
/// buffer, held by the bind group, that a compute step could bind and write too.
struct Piece {
    geometry: Arc<Geometry>,
    indices: Option<wgpu::Buffer>,
    uniform: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl Piece {
    fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, geometry: Arc<Geometry>) -> Self {
        use wgpu::util::DeviceExt;
        let buffer = |label, contents: &[u8], usage| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents,
                usage,
            })
        };
        let data = buffer(
            "view3d data",
            &geometry.data,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let indices = (!geometry.indices.is_empty()).then(|| {
            buffer(
                "view3d indices",
                &geometry.indices,
                wgpu::BufferUsages::INDEX,
            )
        });
        let uniform = buffer(
            "view3d camera",
            &[0; UNIFORM_BYTES],
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("view3d"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: data.as_entire_binding(),
                },
            ],
        });
        Self {
            geometry,
            indices,
            uniform,
            bind_group,
        }
    }
}

/// The window's device: eframe's, but with buffers as big as the card allows rather than
/// wgpu's conservative defaults (128 MB a storage binding), so a model's size is bounded by
/// the card's memory alone.
pub fn device(adapter: &wgpu::Adapter) -> wgpu::DeviceDescriptor<'static> {
    let base = if adapter.get_info().backend == wgpu::Backend::Gl {
        wgpu::Limits::downlevel_webgl2_defaults()
    } else {
        wgpu::Limits::default()
    };
    let own = adapter.limits();
    wgpu::DeviceDescriptor {
        label: Some("cut craft"),
        required_limits: wgpu::Limits {
            max_buffer_size: own.max_buffer_size,
            max_storage_buffer_binding_size: own.max_storage_buffer_binding_size,
            // eframe's: the depth texture must cover a 4k+ window
            max_texture_dimension_2d: 8192,
            ..base
        },
        ..wgpu::DeviceDescriptor::default()
    }
}

/// The GPU as the 3D view uses it: the window's device and queue (at hand here for work on
/// the card beyond drawing), the view's pipeline, and what is on the card.
pub struct Gpu {
    state: RenderState,
    pipeline: wgpu::RenderPipeline,
    /// The see-through faces' pipeline: blended over what is drawn, and leaving the depth
    /// buffer as it is, so they hide nothing behind them.
    see_through: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    surface: Option<Piece>,
    model: Option<Piece>,
    /// The model's body under its surface (walls down to the modeling plane).
    body: Option<Piece>,
    /// A model's parts past the material's edges, drawn see-through after the rest.
    ghost: Option<Piece>,
    /// Pieces drawn lately and put aside, so going back to one (the 3D view's detail
    /// following the frame rate) uploads nothing; one whose geometry nothing else holds
    /// leaves the card.
    spare: Vec<Piece>,
}

/// The most pieces put aside: a surface and body at each of the four details.
const SPARE: usize = 8;

/// What a spare piece draws.
trait Draws {
    fn geometry(&self) -> &Arc<Geometry>;
}

impl Draws for Piece {
    fn geometry(&self) -> &Arc<Geometry> {
        &self.geometry
    }
}

/// The one in `spare` drawing `geometry`, taken out of it.
fn take_spare<T: Draws>(spare: &mut Vec<T>, geometry: &Arc<Geometry>) -> Option<T> {
    let at = spare
        .iter()
        .position(|p| Arc::ptr_eq(p.geometry(), geometry))?;
    Some(spare.remove(at))
}

/// Lets go of the spares whose geometry is held by nothing else, then of the oldest past
/// [`SPARE`].
fn tidy_spare<T: Draws>(spare: &mut Vec<T>) {
    spare.retain(|p| Arc::strong_count(p.geometry()) > 1);
    let over = spare.len().saturating_sub(SPARE);
    spare.drain(..over);
}

/// One draw of the frame: how it is drawn, what the shader reads, and how many it draws.
struct Call {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    indices: Option<wgpu::Buffer>,
    count: u32,
}

/// What the 3D view draws this frame, for egui's render pass: the surface and, importing,
/// the model, sharing the depth buffer so the model sits in the material.
struct Paint {
    calls: Vec<Call>,
}

impl Gpu {
    /// Readies the window's GPU for the 3D view, with the window's multisampling.
    pub fn new(state: &RenderState, samples: u16) -> Self {
        let device = &state.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("view3d"),
            source: wgpu::ShaderSource::Wgsl(include_str!("surface.wgsl").into()),
        });
        let buffer = |binding, visibility, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("view3d"),
            entries: &[
                buffer(
                    0,
                    wgpu::ShaderStages::VERTEX_FRAGMENT,
                    wgpu::BufferBindingType::Uniform,
                ),
                buffer(
                    1,
                    wgpu::ShaderStages::VERTEX,
                    wgpu::BufferBindingType::Storage { read_only: true },
                ),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("view3d"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = |see_through: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("view3d"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: egui_wgpu::depth_format_from_bits(DEPTH_BITS, 0)
                        .expect("a depth format"),
                    depth_write_enabled: Some(!see_through),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: u32::from(samples.max(1)),
                    ..wgpu::MultisampleState::default()
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    // egui prefers a framebuffer that takes sRGB; a linear one takes linear
                    entry_point: Some(if state.target_format.is_srgb() {
                        "fs_linear"
                    } else {
                        "fs_gamma"
                    }),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        blend: see_through.then_some(wgpu::BlendState::ALPHA_BLENDING),
                        ..state.target_format.into()
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        Self {
            state: state.clone(),
            pipeline: pipeline(false),
            see_through: pipeline(true),
            layout,
            surface: None,
            model: None,
            body: None,
            ghost: None,
            spare: Vec::new(),
        }
    }

    /// This frame's picture over `rect`: uploads what changed, sets the cameras, and gives
    /// the shape that draws it; the see-through `ghost` goes last, over the rest. A
    /// geometry not drawn leaves the card.
    pub fn paint(
        &mut self,
        rect: Rect,
        surface: Option<Draw>,
        model: Option<Draw>,
        body: Option<Draw>,
        ghost: Option<Draw>,
    ) -> Shape {
        let mut calls = vec![];
        let spare = &mut self.spare;
        for (slot, draw, pipeline) in [
            (&mut self.surface, surface, &self.pipeline),
            (&mut self.model, model, &self.pipeline),
            (&mut self.body, body, &self.pipeline),
            (&mut self.ghost, ghost, &self.see_through),
        ] {
            let Some(draw) = draw.filter(|d| d.geometry.count() > 0) else {
                spare.extend(slot.take());
                continue;
            };
            if slot
                .as_ref()
                .is_none_or(|p| !Arc::ptr_eq(&p.geometry, &draw.geometry))
            {
                let piece = take_spare(spare, &draw.geometry).unwrap_or_else(|| {
                    Piece::new(&self.state.device, &self.layout, Arc::clone(&draw.geometry))
                });
                spare.extend(slot.replace(piece));
            }
            let piece = slot.as_mut().expect("put above");
            self.state
                .queue
                .write_buffer(&piece.uniform, 0, &draw.uniform);
            calls.push(Call {
                pipeline: pipeline.clone(),
                bind_group: piece.bind_group.clone(),
                indices: piece.indices.clone(),
                count: piece.geometry.count(),
            });
        }
        tidy_spare(spare);
        let paint = Paint { calls };
        Shape::Callback(egui_wgpu::Callback::new_paint_callback(rect, paint))
    }
}

impl CallbackTrait for Paint {
    fn paint(
        &self,
        _info: eframe::egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        _resources: &CallbackResources,
    ) {
        for call in &self.calls {
            pass.set_pipeline(&call.pipeline);
            pass.set_bind_group(0, &call.bind_group, &[]);
            match &call.indices {
                Some(indices) => {
                    pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..call.count, 0, 0..1);
                }
                None => pass.draw(0..call.count, 0..1),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Framing, LIGHT, Orbit, Wrap};
    use super::*;

    impl Draws for Arc<Geometry> {
        fn geometry(&self) -> &Arc<Geometry> {
            self
        }
    }

    #[test]
    fn spare_pieces_are_found_again_and_let_go_once_nothing_holds_them() {
        let (fine, coarse) = (Arc::new(Geometry::default()), Arc::new(Geometry::default()));
        let mut spare = vec![Arc::clone(&fine), Arc::clone(&coarse)];
        tidy_spare(&mut spare);
        assert_eq!(spare.len(), 2, "both still held");
        let found = take_spare(&mut spare, &coarse).expect("kept");
        assert!(Arc::ptr_eq(&found, &coarse));
        assert!(take_spare(&mut spare, &coarse).is_none(), "taken out");
        drop(fine);
        tidy_spare(&mut spare);
        assert!(spare.is_empty(), "nothing else held it");
        let held: Vec<_> = (0..SPARE + 3)
            .map(|_| Arc::new(Geometry::default()))
            .collect();
        let mut spare: Vec<_> = held.iter().map(Arc::clone).collect();
        tidy_spare(&mut spare);
        assert_eq!(spare.len(), SPARE);
        assert!(Arc::ptr_eq(&spare[0], &held[3]), "the oldest let go first");
    }
    use cut_craft_core::document::Job;
    use cut_craft_core::glam::DVec4;
    use eframe::egui::pos2;

    fn look() -> Look {
        Look {
            colour: Color32::WHITE,
            sides: false,
            light: LIGHT,
            wrap: None,
            grain: None,
            shadows: false,
        }
    }

    #[test]
    fn the_shader_parses_and_validates() {
        use wgpu::naga::valid::{Capabilities, ValidationFlags, Validator};
        let module = wgpu::naga::front::wgsl::parse_str(include_str!("surface.wgsl"))
            .unwrap_or_else(|e| panic!("{e}"));
        Validator::new(ValidationFlags::all(), Capabilities::empty())
            .validate(&module)
            .unwrap_or_else(|e| panic!("{e}"));
    }

    /// 100 x 100 mm in 1 mm cells, with a 5 mm deep slot from x = 40 to 60 running along y.
    fn slotted() -> Material {
        let job = Job {
            width: 100.0,
            height: 100.0,
            ..Job::default()
        };
        let mut m = Material::new(&job, 100);
        for row in 0..m.rows {
            for column in 40..60 {
                m.heights[row * m.columns + column] = -5.0;
            }
        }
        m
    }

    fn f32s(bytes: &[u8]) -> Vec<f32> {
        bytes
            .chunks(4)
            .map(|b| f32::from_ne_bytes(b.try_into().unwrap()))
            .collect()
    }

    fn u32s(bytes: &[u8]) -> Vec<u32> {
        bytes
            .chunks(4)
            .map(|b| u32::from_ne_bytes(b.try_into().unwrap()))
            .collect()
    }

    #[test]
    fn the_shader_is_valid_wgsl() {
        let module = naga::front::wgsl::parse_str(include_str!("surface.wgsl"))
            .unwrap_or_else(|e| panic!("{e}"));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap_or_else(|e| panic!("{e:?}"));
    }

    #[test]
    fn a_painted_surface_uploads_its_paint_after_its_heights() {
        let mut m = slotted();
        let cells = m.columns * m.rows;
        assert!(!surface(&m, None, false).painted);
        m.paint = vec![0x0100_00ff; cells];
        let g = surface(&m, None, false);
        assert!(g.painted);
        assert_eq!(g.data.len(), cells * 8, "a height and a paint a cell");
        assert_eq!(
            u32::from_ne_bytes(g.data[cells * 4..cells * 4 + 4].try_into().unwrap()),
            0x0100_00ff
        );
    }

    #[test]
    fn the_surface_uploads_its_heights_and_a_quad_over_each_cell() {
        let m = slotted();
        let g = surface(&m, None, false);
        let heights = f32s(&g.data);
        assert_eq!(heights.len(), m.columns * m.rows);
        assert_eq!(heights[50 * m.columns + 50], -5.0);
        assert_eq!(g.triangles(), 2 * (m.columns - 1) * (m.rows - 1));
        assert_eq!(g.vertices as usize, m.columns * m.rows);
        let indices = u32s(&g.indices);
        assert_eq!(&indices[..6], &[0, 1, 101, 0, 101, 100], "the first quad");
        assert_eq!(g.grid.unwrap().cell, 1.0);
    }

    #[test]
    fn a_cut_underside_is_a_grid_of_its_own_after_the_sides() {
        let mut m = slotted();
        let bottom = m.bottom;
        m.under = vec![bottom; m.heights.len()];
        m.under[5_050] = bottom + 3.0;
        let flat = surface(&slotted(), None, true);
        let g = surface(&m, None, true);
        assert!(g.undersided);
        assert_eq!(g.vertices as usize, 100 * 100 + 2 * 400 + 100 * 100);
        assert_eq!(
            g.data.len(),
            2 * 100 * 100 * 4,
            "heights, then the underside"
        );
        // the flat bottom's one quad becomes 99 x 99
        assert_eq!(g.triangles(), flat.triangles() - 2 + 2 * 99 * 99);
        assert!(
            !surface(&m, None, false).undersided,
            "no sides, no underside"
        );
    }

    #[test]
    fn empty_cells_are_marked_and_left_out_and_sides_add_their_vertices() {
        let mut half = slotted();
        let columns = half.columns;
        for (i, h) in half.heights.iter_mut().enumerate() {
            if i % columns < columns / 2 {
                *h = f64::NAN;
            }
        }
        let full = surface(&slotted(), None, false).triangles();
        let shown = surface(&half, None, false);
        assert!(f32s(&shown.data)[..50].iter().all(|&h| h == EMPTY));
        assert!(shown.triangles() < full * 2 / 3 && shown.triangles() > 0);
        let sided = surface(&slotted(), None, true);
        // four sides of 99 quads each and the bottom
        assert_eq!(sided.triangles(), full + 2 * (4 * 99 + 1));
        // a top and bottom vertex for each of the 400 edge cells, and the bottom's corners
        assert_eq!(sided.vertices as usize, 100 * 100 + 2 * 400 + 4);
        let indices = u32s(&sided.indices);
        let first_side = &indices[3 * full..][..6];
        assert_eq!(
            first_side,
            &[10_000, 10_002, 10_003, 10_000, 10_003, 10_001]
        );
        assert_eq!(indices.last(), Some(&10_803));
    }

    #[test]
    fn a_wrapped_surface_closes_the_way_round() {
        let m = slotted();
        let wrap = Wrap {
            along_x: true,
            radius: 20.0,
            surface: 0.0,
        };
        let g = surface(&m, Some(wrap), true);
        // along the axis the quads stop; round it they meet themselves; no sides
        assert_eq!(g.triangles(), 2 * (m.columns - 1) * m.rows);
        let indices = u32s(&g.indices);
        let last_quad = &indices[indices.len() - 6..];
        let first_row = u32::try_from(m.columns).unwrap();
        assert!(
            last_quad.iter().any(|&i| i < first_row),
            "the last row's quads reach the first: {last_quad:?}"
        );
    }

    #[test]
    fn a_model_packs_ten_numbers_a_face() {
        let faces = [
            ([DVec3::ZERO, DVec3::X, DVec3::Y], false),
            ([DVec3::ZERO, DVec3::Y, DVec3::X], true),
        ];
        let g = model(&faces);
        assert_eq!((g.triangles(), g.vertices, g.count()), (2, 6, 6));
        let words = f32s(&g.data);
        assert_eq!(words.len(), 2 * FACE_WORDS);
        assert_eq!(&words[3..6], &[1.0, 0.0, 0.0]);
        assert_eq!(
            (words[9], words[19]),
            (0.0, 1.0),
            "the second face is marked"
        );
    }

    #[test]
    #[expect(clippy::cast_possible_truncation, reason = "small test coordinates")]
    fn the_clip_matrix_places_points_where_the_camera_does() {
        let m = slotted();
        let area = Rect::from_min_max(pos2(100.0, 50.0), pos2(500.0, 350.0));
        let framing = Framing {
            zoom: 1.5,
            pan: eframe::egui::vec2(20.0, -10.0),
            perspective: false,
            ..Framing::default()
        };
        for perspective in [false, true] {
            let framing = Framing {
                perspective,
                ..framing
            };
            let camera = Camera::new(&m, Orbit::default(), framing, area, None);
            let clip = camera.clip(area, DEPTH_REACH);
            let mut last_z = f64::NEG_INFINITY;
            // far to near across the block
            for p in [
                DVec3::new(90.0, 90.0, -5.0),
                DVec3::new(50.0, 50.0, 0.0),
                DVec3::new(10.0, 10.0, 0.0),
            ] {
                let c = clip * DVec4::new(p.x, p.y, p.z, 1.0);
                let ndc = c / c.w;
                let (centre, half) = (area.center(), area.size() / 2.0);
                let on = pos2(
                    (f64::from(centre.x) + ndc.x * f64::from(half.x)) as f32,
                    (f64::from(centre.y) - ndc.y * f64::from(half.y)) as f32,
                );
                let expected = camera.screen(p);
                assert!(
                    on.distance(expected) < 1e-3,
                    "perspective {perspective}: {p} at {on:?}, not {expected:?}"
                );
                assert!(ndc.z > 0.0 && ndc.z < 1.0, "depth in range: {}", ndc.z);
                assert!(
                    ndc.z < last_z || last_z == f64::NEG_INFINITY,
                    "nearer is less deep"
                );
                last_z = ndc.z;
            }
        }
    }

    #[test]
    fn from_any_way_of_facing_the_block_stays_in_the_clip_box() {
        let m = slotted();
        let area = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 300.0));
        let corners = (0..8).map(|i| {
            DVec3::new(
                if i & 1 == 1 { 100.0 } else { 0.0 },
                if i & 2 == 2 { 100.0 } else { 0.0 },
                if i & 4 == 4 { 0.0 } else { m.bottom },
            )
        });
        for yaw in [-3.0, -1.0, 0.0, 0.7, 2.5] {
            for pitch in [
                0.0,
                0.4,
                std::f64::consts::FRAC_PI_2,
                2.8,
                std::f64::consts::PI,
            ] {
                for roll in [-2.0, 0.0, 1.3] {
                    for perspective in [false, true] {
                        let orbit = Orbit { yaw, pitch, roll };
                        let framing = Framing {
                            perspective,
                            ..Framing::default()
                        };
                        let camera = Camera::new(&m, orbit, framing, area, None);
                        let clip = camera.clip(area, DEPTH_REACH);
                        for p in corners.clone() {
                            let c = clip * DVec4::new(p.x, p.y, p.z, 1.0);
                            assert!(c.w > 0.0, "{orbit:?} {perspective}: behind the eye");
                            let ndc = c / c.w;
                            assert!(
                                ndc.x.abs() <= 1.0
                                    && ndc.y.abs() <= 1.0
                                    && ndc.z > 0.0
                                    && ndc.z < 1.0,
                                "{orbit:?} {perspective}: {p} at {ndc}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn a_see_through_colour_packs_its_alpha_without_darkening_the_red() {
        let m = slotted();
        let area = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 300.0));
        let camera = Camera::new(&m, Orbit::default(), Framing::default(), area, None);
        let see_through = Look {
            colour: Color32::from_rgba_unmultiplied(255, 0, 0, 51),
            ..look()
        };
        let packed = |look: &Look| {
            let draw = Draw::new(
                Arc::new(model(&[])),
                &camera,
                area,
                DVec3::ZERO,
                look,
                Color32::RED,
            );
            f32s(&draw.uniform)
        };
        let floats = packed(&see_through);
        assert_eq!(
            &floats[24..27],
            &[1.0, 0.0, 0.0],
            "full red, not premultiplied"
        );
        assert!(
            (floats[27] - 0.2).abs() < 1e-6,
            "a fifth solid: {}",
            floats[27]
        );
        // an opaque colour is wholly solid, as it always was
        assert_eq!(packed(&look())[27], 1.0);
    }

    #[test]
    #[expect(clippy::cast_possible_truncation, reason = "the GPU takes f32")]
    fn a_draw_packs_the_camera_uniform_as_the_shader_reads_it() {
        let m = slotted();
        let area = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 300.0));
        let camera = Camera::new(&m, Orbit::default(), Framing::default(), area, None);
        let look = Look {
            colour: Color32::from_rgb(255, 0, 0),
            grain: Some(super::super::Grain {
                spacing: 4.0,
                depth: 0.3,
            }),
            ..look()
        };
        let g = Arc::new(surface(&m, None, true));
        let draw = Draw::new(g, &camera, area, DVec3::ZERO, &look, Color32::RED);
        assert_eq!(draw.uniform.len(), UNIFORM_BYTES);
        let words = u32s(&draw.uniform);
        let floats = f32s(&draw.uniform);
        assert_eq!(words[23], 0, "the surface is not culled");
        assert!((f64::from(floats[19]) - AMBIENT).abs() < 1e-6);
        assert_eq!(&floats[24..27], &[1.0, 0.0, 0.0], "red, linear");
        assert_eq!(&words[32..34], &[100, 100], "the grid");
        assert_eq!(&floats[34..36], &[1.0, m.bottom as f32]);
        assert_eq!(words[36], 0, "flat");
        assert_eq!(floats[39], DEPTH_SHADE as f32);
        assert_eq!(floats[40], GRAIN_DRIFTS[0].0 as f32);
        // a spacing of 4 cells is too coarse for full-strength grain: 4 of the 6 it needs
        assert_eq!(floats[50], 4.0, "the grain's spacing");
        assert!(
            (floats[51] - 0.3 * 4.0 / 6.0).abs() < 1e-6,
            "its depth, faded"
        );
        assert_eq!(words[52], 0, "a surface");
        assert_eq!(words[55], 0, "no shadows unless asked");
        let shadowed = Draw::new(
            Arc::new(surface(&m, None, true)),
            &camera,
            area,
            DVec3::ZERO,
            &Look {
                shadows: true,
                ..look
            },
            Color32::RED,
        );
        let (words, floats) = (u32s(&shadowed.uniform), f32s(&shadowed.uniform));
        assert_eq!(words[55], 1, "shadows traced");
        assert_eq!(
            floats[38], 0.0,
            "up to the surface's highest point, its top"
        );
        let model_draw = Draw::new(
            Arc::new(model(&[([DVec3::ZERO, DVec3::X, DVec3::Y], false)])),
            &camera,
            area,
            DVec3::ZERO,
            &look,
            Color32::RED,
        );
        let words = u32s(&model_draw.uniform);
        assert_eq!((words[23], words[52]), (1, 1), "a model, its backs culled");
    }

    /// `cargo test --release -p cut-craft -- --ignored gpu_timing --nocapture`: what a frame
    /// of the CPU meshes costs against building the GPU's geometry once and its uniform each
    /// frame, for a job at 1200 cells and a two-million-face model.
    #[test]
    #[ignore = "timing, by hand"]
    #[expect(clippy::cast_precision_loss, reason = "a test grid")]
    fn gpu_timing() {
        use cut_craft_core::component::{Combine, Component};
        use std::time::Instant;
        let mut doc = cut_craft_core::document::Document::default();
        let origin = doc.job.area().min;
        let rect = cut_craft_core::geometry::Rect::new(
            origin + cut_craft_core::geometry::Point::new(150.0, 100.0),
            origin + cut_craft_core::geometry::Point::new(450.0, 250.0),
        );
        let mut change = cut_craft_core::Change::builder(&doc);
        change.add_component(Component::from_fn(
            "m",
            Combine::Add,
            rect,
            doc.job.model_cell(),
            |p| Some((p.x * 0.3).sin() + (p.y * 0.2).cos() + 3.0),
        ));
        doc.apply(&change.finish()).unwrap();
        let area = Rect::from_min_size(eframe::egui::Pos2::ZERO, eframe::egui::vec2(1400.0, 900.0));
        let look = Look {
            colour: Color32::GRAY,
            ..look()
        };
        let shape = cut_craft_core::component::model_shape(&doc, 1200);
        let camera = Camera::new(&shape, Orbit::default(), Framing::default(), area, None);
        let started = Instant::now();
        let mesh = super::super::mesh(&shape, Orbit::default(), Framing::default(), &look, area);
        eprintln!(
            "surface, 1200 cells: CPU mesh a frame {:?} ({} triangles)",
            started.elapsed(),
            mesh.indices.len() / 3
        );
        let started = Instant::now();
        let geometry = Arc::new(surface(&shape, None, false));
        let built = started.elapsed();
        let started = Instant::now();
        let draw = Draw::new(
            Arc::clone(&geometry),
            &camera,
            area,
            DVec3::ZERO,
            &look,
            Color32::RED,
        );
        eprintln!(
            "surface, 1200 cells: GPU geometry once {built:?} ({} triangles, {} MB), a frame {:?}",
            geometry.triangles(),
            geometry.bytes() / 1_000_000,
            started.elapsed()
        );
        drop(draw);
        // the model big_model_timing builds, as faces
        let side = 1000;
        let height =
            |x: usize, y: usize| ((x as f64 / 37.0).sin() + (y as f64 / 23.0).cos()) * 5.0 + 10.0;
        let at = |x: usize, y: usize| DVec3::new(x as f64 / 10.0, y as f64 / 10.0, height(x, y));
        let faces: Vec<cmd_orientate_3d_model::Seen> = (0..side)
            .flat_map(|x| (0..side).map(move |y| (x, y)))
            .flat_map(|(x, y)| {
                [
                    ([at(x, y), at(x + 1, y), at(x + 1, y + 1)], false),
                    ([at(x, y), at(x + 1, y + 1), at(x, y + 1)], y % 7 == 0),
                ]
            })
            .collect();
        let started = Instant::now();
        let mesh = super::super::triangles(
            &faces,
            |p| camera.screen(p),
            Orbit::default(),
            &look,
            Color32::RED,
        );
        eprintln!(
            "model, {} faces: CPU triangles a frame {:?} ({} drawn)",
            faces.len(),
            started.elapsed(),
            mesh.indices.len() / 3
        );
        let started = Instant::now();
        let geometry = Arc::new(model(&faces));
        let built = started.elapsed();
        let started = Instant::now();
        let draw = Draw::new(
            Arc::clone(&geometry),
            &camera,
            area,
            DVec3::ZERO,
            &look,
            Color32::RED,
        );
        eprintln!(
            "model, {} faces: GPU geometry once {built:?} ({} MB), a frame {:?}",
            faces.len(),
            geometry.bytes() / 1_000_000,
            started.elapsed()
        );
        drop(draw);
    }
}
