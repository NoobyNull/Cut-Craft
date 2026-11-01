// The grid kernels, each the same sums as its CPU kernel in core's simulation.rs: the tool
// dropped onto the height grid at a point (drop_tool_on), swept along a move (sweep), and
// checked for gouging along a move (overcut_on). A thread an item: a point, or a move.
// Coordinates are millimetres from the grid's origin, heights from the material's top.

struct Params {
    columns: u32,
    rows: u32,
    count: u32,
    // 0 end mill, 1 ball nose, 2 V-bit, 3 radiused end mill, 4 thread mill
    shape: u32,
    cell: f32,
    bottom: f32,
    // the tool's radius, and how far from its axis it reaches (its shank, tapered)
    radius: f32,
    reach: f32,
    // tan of the taper's side angle, 0 for a straight tool; where the taper starts, out
    // from the axis and up from the tip
    taper: f32,
    turn: f32,
    rise: f32,
    // a radiused end mill's corner radius
    tip: f32,
    // tan of half a V-bit's angle; a thread mill's core radius, tooth tip height and flank slope
    vbit_slope: f32,
    tooth_core: f32,
    tooth_offset: f32,
    tooth_slope: f32,
    // drop every cell centre, not the points given; an untapered end mill
    centres: u32,
    flat_end: u32,
    pad0: u32,
    pad1: u32,
}

// A move of the tool's tip, with the radius the tool reaches below the top along it.
struct Move {
    a: vec3<f32>,
    radius: f32,
    b: vec3<f32>,
    pad: f32,
}

@group(0) @binding(0) var<uniform> P: Params;
@group(0) @binding(1) var<storage, read> heights: array<f32>;
// the same grid, written by the sweep (its f32s as bits, for the compare and swap)
@group(0) @binding(2) var<storage, read_write> cut: array<atomic<u32>>;
@group(0) @binding(3) var<storage, read> points: array<vec2<f32>>;
@group(0) @binding(4) var<storage, read> moves: array<Move>;
@group(0) @binding(5) var<storage, read_write> out: array<f32>;

// A height no cell has: no answer.
const NONE: f32 = 1.0e30;
// 1 / golden ratio
const SHRINK: f32 = 0.6180339887498949;

fn item(gid: vec3<u32>, n: vec3<u32>) -> u32 {
    return gid.y * (n.x * 64u) + gid.x;
}

fn centre(column: u32, row: u32) -> vec2<f32> {
    return (vec2<f32>(f32(column), f32(row)) + 0.5) * P.cell;
}

fn height(column: u32, row: u32) -> f32 {
    return heights[row * P.columns + column];
}

// The cells from lo to hi mm along a side count cells long: x up to but not y.
fn span(lo: f32, hi: f32, count: u32) -> vec2<u32> {
    return vec2<u32>(
        u32(max(floor(lo / P.cell), 0.0)),
        min(u32(max(ceil(hi / P.cell), 0.0)), count),
    );
}

// How far up from the tip the cutting edge is, r from the axis (r within reach).
fn profile(r: f32) -> f32 {
    if (P.taper > 0.0 && r > P.turn) {
        return P.rise + (r - P.turn) / P.taper;
    }
    switch (P.shape) {
        case 0u: {
            return 0.0;
        }
        case 1u: {
            return P.radius - sqrt(max(P.radius * P.radius - r * r, 0.0));
        }
        case 2u: {
            return r / P.vbit_slope;
        }
        case 3u: {
            let past = max(r - (P.radius - P.tip), 0.0);
            return P.tip - sqrt(max(P.tip * P.tip - past * past, 0.0));
        }
        default: {
            if (r <= P.tooth_core) {
                return 0.0;
            }
            return P.tooth_offset - (P.radius - r) * P.tooth_slope;
        }
    }
}

fn distance_to_line(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
    let along = b - a;
    var t = 0.0;
    if (any(along != vec2<f32>(0.0))) {
        t = clamp(dot(p - a, along) / dot(along, along), 0.0, 1.0);
    }
    return distance(p, a + along * t);
}

// The span of t in [0, 1] where |start + t along| <= radius; x > y when there is none.
fn covered(start: vec2<f32>, along: vec2<f32>, radius: f32) -> vec2<f32> {
    let a = dot(along, along);
    let half_b = dot(start, along);
    let c = dot(start, start) - radius * radius;
    if (a == 0.0) {
        if (c <= 0.0) {
            return vec2<f32>(0.0, 1.0);
        }
        return vec2<f32>(1.0, 0.0);
    }
    let discriminant = half_b * half_b - a * c;
    if (discriminant < 0.0) {
        return vec2<f32>(1.0, 0.0);
    }
    let root = sqrt(discriminant);
    return vec2<f32>(max((-half_b - root) / a, 0.0), min((-half_b + root) / a, 1.0));
}

// The cutting edge's height over centre with the tip t of the way from a to b.
fn edge(t: f32, a: vec3<f32>, b: vec3<f32>, centre: vec2<f32>) -> f32 {
    let tip = mix(a, b, t);
    return tip.z + profile(min(distance(tip.xy, centre), P.reach));
}

// The least of edge over [lo, hi] by golden-section search; the ends checked exactly.
fn lowest(lo: f32, hi: f32, a: vec3<f32>, b: vec3<f32>, centre: vec2<f32>) -> f32 {
    var start = lo;
    var end = hi;
    var left = end - SHRINK * (end - start);
    var right = start + SHRINK * (end - start);
    var at_left = edge(left, a, b, centre);
    var at_right = edge(right, a, b, centre);
    for (var i = 0u; i < 40u; i++) {
        if (at_left < at_right) {
            end = right;
            right = left;
            at_right = at_left;
            left = end - SHRINK * (end - start);
            at_left = edge(left, a, b, centre);
        } else {
            start = left;
            left = right;
            at_left = at_right;
            right = start + SHRINK * (end - start);
            at_right = edge(right, a, b, centre);
        }
    }
    return min(min(edge(lo, a, b, centre), edge(hi, a, b, centre)), min(at_left, at_right));
}

// The lowest the cutting edge reaches over centre while the tip goes from a to b; NONE if
// the tool never covers it.
fn lowest_edge(a: vec3<f32>, b: vec3<f32>, centre: vec2<f32>) -> f32 {
    let start = a.xy;
    let along = (b - a).xy;
    let span = covered(start - centre, along, P.reach);
    if (span.x > span.y) {
        return NONE;
    }
    // z is linear in t, so a flat end is lowest at one end of the covered span
    if (P.flat_end != 0u) {
        return min(mix(a, b, span.x).z, mix(a, b, span.y).z);
    }
    // a level move is lowest where it passes closest
    if (abs(b.z - a.z) < 1e-9 && any(along != vec2<f32>(0.0))) {
        let nearest = -dot(start - centre, along) / dot(along, along);
        return edge(clamp(nearest, span.x, span.y), a, b, centre);
    }
    return lowest(span.x, span.y, a, b, centre);
}

// How low the tool's tip can go over each point (or each cell centre) before its edge
// touches the surface at any cell centre it covers.
@compute @workgroup_size(64)
fn drop_main(@builtin(global_invocation_id) gid: vec3<u32>, @builtin(num_workgroups) n: vec3<u32>) {
    let i = item(gid, n);
    if (i >= P.count) {
        return;
    }
    var at = vec2<f32>(0.0);
    if (P.centres != 0u) {
        at = centre(i % P.columns, i / P.columns);
    } else {
        at = points[i];
    }
    let columns = span(at.x - P.reach, at.x + P.reach, P.columns);
    let rows = span(at.y - P.reach, at.y + P.reach, P.rows);
    var tip = -NONE;
    for (var r = rows.x; r < rows.y; r++) {
        for (var c = columns.x; c < columns.y; c++) {
            let d = distance(centre(c, r), at);
            if (d > P.reach) {
                continue;
            }
            tip = max(tip, height(c, r) - profile(d));
        }
    }
    if (tip <= -NONE) {
        // a tool narrower than a cell may cover no centre: rest on the cell under it
        tip = 0.0;
        let cell = at / P.cell;
        if (cell.x >= 0.0 && cell.y >= 0.0 && u32(cell.x) < P.columns && u32(cell.y) < P.rows) {
            tip = height(u32(cell.x), u32(cell.y));
        }
    }
    out[i] = tip;
}

// cut[i] = min(cut[i], z), whichever threads get there first.
fn lower(i: u32, z: f32) {
    var old = atomicLoad(&cut[i]);
    loop {
        if (bitcast<f32>(old) <= z) {
            return;
        }
        let swap = atomicCompareExchangeWeak(&cut[i], old, bitcast<u32>(z));
        if (swap.exchanged) {
            return;
        }
        old = swap.old_value;
    }
}

// Lowers each cell a move passes over to the lowest the cutting edge reaches there.
@compute @workgroup_size(64)
fn sweep_main(@builtin(global_invocation_id) gid: vec3<u32>, @builtin(num_workgroups) n: vec3<u32>) {
    let i = item(gid, n);
    if (i >= P.count) {
        return;
    }
    let m = moves[i];
    // the material is nowhere above its top, 0
    if (m.a.z >= 0.0 && m.b.z >= 0.0) {
        return;
    }
    let lo = min(m.a.xy, m.b.xy) - m.radius;
    let hi = max(m.a.xy, m.b.xy) + m.radius;
    let columns = span(lo.x, hi.x, P.columns);
    let rows = span(lo.y, hi.y, P.rows);
    for (var r = rows.x; r < rows.y; r++) {
        for (var c = columns.x; c < columns.y; c++) {
            let z = lowest_edge(m.a, m.b, centre(c, r));
            if (z < NONE) {
                lower(r * P.columns + c, max(z, P.bottom));
            }
        }
    }
}

// How far below the surface the tool would cut going straight along each move, at the
// deepest cell centre it passes over; -NONE when it covers none.
@compute @workgroup_size(64)
fn overcut_main(@builtin(global_invocation_id) gid: vec3<u32>, @builtin(num_workgroups) n: vec3<u32>) {
    let i = item(gid, n);
    if (i >= P.count) {
        return;
    }
    let m = moves[i];
    let low = min(m.a.z, m.b.z);
    let lo = min(m.a.xy, m.b.xy) - P.reach;
    let hi = max(m.a.xy, m.b.xy) + P.reach;
    let columns = span(lo.x, hi.x, P.columns);
    let rows = span(lo.y, hi.y, P.rows);
    var worst = -NONE;
    for (var r = rows.x; r < rows.y; r++) {
        for (var c = columns.x; c < columns.y; c++) {
            let at = centre(c, r);
            let h = height(c, r);
            // the edge comes no lower than the lowest end at the nearest the line passes:
            // past that, the exact search cannot find worse
            let nearest = distance_to_line(at, m.a.xy, m.b.xy);
            if (nearest > P.reach || h - (low + profile(nearest)) <= worst) {
                continue;
            }
            let z = lowest_edge(m.a, m.b, at);
            if (z < NONE) {
                worst = max(worst, h - z);
            }
        }
    }
    out[i] = worst;
}
