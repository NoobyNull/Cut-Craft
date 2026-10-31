// The 3D view: the material's surface built from its height grid, or an imported model's
// faces; each vertex is placed, lit and coloured here from the camera uniform, so a frame
// uploads nothing but that.

struct Camera {
    // material coordinates to clip space
    clip: mat4x4<f32>,
    // where the light comes from, unit length
    light: vec3<f32>,
    // brightness of a face turned from the light
    ambient: f32,
    // towards the viewer, unit length
    towards: vec3<f32>,
    // leave out faces turned away from the viewer (a model's backs)
    cull: u32,
    // the material's or model's colour, and a model's marked faces', linear
    colour: vec4<f32>,
    mark: vec4<f32>,
    // the surface's grid: cells across and up, a cell's edge, the block's bottom
    columns: u32,
    rows: u32,
    cell: f32,
    bottom: f32,
    // drawn round an axis: 0 flat, 1 along x, 2 along y; the radius and the surface height
    wrap: u32,
    radius: f32,
    surface: f32,
    // how much darker the full depth is
    depth_shade: f32,
    // wood grain: two (size, length) drifts along it, two bunchings across it, then its
    // sharpness, how far it shifts going down, its spacing and its depth (0: no grain)
    drifts: vec4<f32>,
    bunches: vec4<f32>,
    grain: vec4<f32>,
    // 0 the surface, 1 a model
    kind: u32,
    // to a whole 16 bytes (scalars: a vec3 would be aligned to 16 and grow the struct)
    pad_a: u32,
    pad_b: u32,
    pad_c: u32,
};

@group(0) @binding(0) var<uniform> camera: Camera;
// the surface's heights row by row (EMPTY where there is no model), or a model's faces:
// three corners and a mark, ten numbers a face
@group(0) @binding(1) var<storage, read> data: array<f32>;

const EMPTY: f32 = -1.0e30;
const TAU: f32 = 6.283185307179586;

struct Placed {
    p: vec3<f32>,
    normal: vec3<f32>,
    // linear
    colour: vec3<f32>,
};

struct Fragment {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) colour: vec3<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> Fragment {
    var placed: Placed;
    if camera.kind == 1u {
        placed = model_vertex(i);
    } else {
        placed = surface_vertex(i);
    }
    var out: Fragment;
    out.clip = camera.clip * vec4<f32>(placed.p, 1.0);
    out.normal = placed.normal;
    out.colour = placed.colour;
    return out;
}

// a model's face corner, the face lit flat by its own normal
fn model_vertex(i: u32) -> Placed {
    let base = (i / 3u) * 10u;
    let corner = i % 3u;
    let p0 = vec3<f32>(data[base], data[base + 1u], data[base + 2u]);
    let p1 = vec3<f32>(data[base + 3u], data[base + 4u], data[base + 5u]);
    let p2 = vec3<f32>(data[base + 6u], data[base + 7u], data[base + 8u]);
    var out: Placed;
    out.p = select(select(p0, p1, corner == 1u), p2, corner == 2u);
    out.normal = cross(p1 - p0, p2 - p0);
    out.colour = select(camera.colour, camera.mark, data[base + 9u] != 0.0).rgb;
    return out;
}

// a cell's centre at height z, flat
fn flat(c: u32, r: u32, z: f32) -> vec3<f32> {
    return vec3<f32>((f32(c) + 0.5) * camera.cell, (f32(r) + 0.5) * camera.cell, z);
}

// a point of the flat material on the cylinder round the axis, angle 0 up
fn place(p: vec3<f32>) -> vec3<f32> {
    if camera.wrap == 0u {
        return p;
    }
    let along_x = camera.wrap == 1u;
    let out = max(camera.radius + p.z - camera.surface, 0.0);
    let round = f32(select(camera.columns, camera.rows, along_x)) * camera.cell;
    let across = select(p.x, p.y, along_x);
    let along = select(p.y, p.x, along_x);
    let angle = across / round * TAU;
    let side = out * sin(angle);
    let up = out * cos(angle);
    if along_x {
        return vec3<f32>(along, side, up);
    }
    return vec3<f32>(side, along, up);
}

// the cell `by` on from `i` of `count`: round again past the end when `wraps`, else the edge
fn step(i: u32, by: i32, count: u32, wraps: bool) -> u32 {
    let n = i32(count);
    let j = i32(i) + by;
    if wraps {
        return u32((j % n + n) % n);
    }
    return u32(clamp(j, 0, n - 1));
}

// where a neighbouring cell's surface is; an empty one counts as level with this cell
fn at(c: u32, r: u32, own: f32) -> vec3<f32> {
    let h = data[r * camera.columns + c];
    return place(flat(c, r, select(h, own, h <= EMPTY)));
}

fn surface_vertex(i: u32) -> Placed {
    let cells = camera.columns * camera.rows;
    if i >= cells {
        return side_vertex(i - cells);
    }
    let c = i % camera.columns;
    let r = i / camera.columns;
    let own = data[i];
    let wrap_c = camera.wrap == 2u;
    let wrap_r = camera.wrap == 1u;
    // the slope, from the neighbours either side
    let across = at(step(c, 1, camera.columns, wrap_c), r, own)
        - at(step(c, -1, camera.columns, wrap_c), r, own);
    let along = at(c, step(r, 1, camera.rows, wrap_r), own)
        - at(c, step(r, -1, camera.rows, wrap_r), own);
    var out: Placed;
    out.p = place(flat(c, r, own));
    // outward: up, or away from the axis
    var outward = vec3<f32>(0.0, 0.0, 1.0);
    if camera.wrap == 1u {
        outward = vec3<f32>(0.0, out.p.y, out.p.z);
    } else if camera.wrap == 2u {
        outward = vec3<f32>(out.p.x, 0.0, out.p.z);
    }
    var normal = cross(across, along);
    if dot(normal, outward) < 0.0 {
        normal = -normal;
    }
    out.normal = normal;
    var depth = 0.0;
    if camera.bottom < 0.0 {
        if camera.wrap == 0u {
            depth = own / camera.bottom;
        } else {
            depth = clamp((own - camera.surface) / camera.bottom, 0.0, 1.0);
        }
    }
    var shade = 1.0 - camera.depth_shade * depth;
    if camera.wrap == 0u && camera.grain.w > 0.0 {
        shade *= grain(flat(c, r, own));
    }
    out.colour = camera.colour.rgb * shade;
    return out;
}

// the block's sides: a top and a bottom vertex for each edge cell, the edges in turn (front,
// back, left, right), then the bottom's four corners
fn side_vertex(j: u32) -> Placed {
    let columns = camera.columns;
    let rows = camera.rows;
    let perimeter = 2u * (columns + rows);
    var out: Placed;
    out.colour = camera.colour.rgb;
    if j < 2u * perimeter {
        let k = j / 2u;
        let down = (j & 1u) == 1u;
        var c = 0u;
        var r = 0u;
        if k < columns {
            c = k;
            out.normal = vec3<f32>(0.0, -1.0, 0.0);
        } else if k < 2u * columns {
            c = k - columns;
            r = rows - 1u;
            out.normal = vec3<f32>(0.0, 1.0, 0.0);
        } else if k < 2u * columns + rows {
            r = k - 2u * columns;
            out.normal = vec3<f32>(-1.0, 0.0, 0.0);
        } else {
            c = columns - 1u;
            r = k - 2u * columns - rows;
            out.normal = vec3<f32>(1.0, 0.0, 0.0);
        }
        let h = data[r * columns + c];
        out.p = flat(c, r, select(h, camera.bottom, down));
    } else {
        let k = j - 2u * perimeter;
        let c = select(0u, columns - 1u, k == 1u || k == 2u);
        let r = select(0u, rows - 1u, k >= 2u);
        out.p = flat(c, r, camera.bottom);
        out.normal = vec3<f32>(0.0, 0.0, -1.0);
    }
    return out;
}

// two waves of (size, length) in line spacings, at `along`
fn waves(along: f32, pair: vec4<f32>, spacing: f32) -> f32 {
    return pair.x * spacing * sin(TAU * along / (pair.y * spacing))
        + pair.z * spacing * sin(TAU * along / (pair.w * spacing));
}

// how lit the wood is at `p`: 1 between grain lines, 1 - depth on one
fn grain(p: vec3<f32>) -> f32 {
    let spacing = camera.grain.z;
    let across = p.y + waves(p.x, camera.drifts, spacing) + waves(p.y, camera.bunches, spacing)
        + camera.grain.y * p.z;
    let line = pow(0.5 + 0.5 * cos(TAU * across / spacing), camera.grain.x);
    return 1.0 - camera.grain.w * line;
}

fn turned_away(normal: vec3<f32>) -> bool {
    return camera.cull != 0u && dot(normal, camera.towards) <= 0.0;
}

// the fragment's colour, linear
fn lit(f: Fragment, normal: vec3<f32>) -> vec3<f32> {
    let facing = max(dot(normal, camera.light), 0.0);
    return f.colour * (camera.ambient + (1.0 - camera.ambient) * facing);
}

// a framebuffer that takes linear colours (sRGB aware)
@fragment
fn fs_linear(f: Fragment) -> @location(0) vec4<f32> {
    let normal = normalize(f.normal);
    if turned_away(normal) {
        discard;
    }
    return vec4<f32>(lit(f, normal), 1.0);
}

// a framebuffer that takes sRGB (egui's preference)
@fragment
fn fs_gamma(f: Fragment) -> @location(0) vec4<f32> {
    let normal = normalize(f.normal);
    if turned_away(normal) {
        discard;
    }
    return vec4<f32>(gamma_from_linear(lit(f, normal)), 1.0);
}

// the sRGB transfer curve
fn gamma_from_linear(rgb: vec3<f32>) -> vec3<f32> {
    let low = rgb * 12.92;
    let high = 1.055 * pow(rgb, vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(high, low, rgb < vec3<f32>(0.0031308));
}
