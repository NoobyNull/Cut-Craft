//! The material as triangles to draw: its faces, sides, underside and edges.

use super::{
    Camera, Color32, DVec2, DVec3, Framing, Look, Material, Mesh, Orbit, Pos2, Rect, Wrap,
    brightness, lit, paint_of, shaded, shadow_map, shadowed_brightness, wrapped_quads,
};

/// The material seen from `orbit`, placed by [`screen_of`] and drawn far to near.
#[expect(clippy::cast_precision_loss, reason = "cell indices fit f64")]
pub fn mesh(material: &Material, orbit: Orbit, framing: Framing, look: &Look, area: Rect) -> Mesh {
    let m = material;
    let shade = |brightness: f64| shaded(look.colour, brightness);
    let top = |column: usize, row: usize| {
        DVec3::new(
            (column as f64 + 0.5) * m.cell,
            (row as f64 + 0.5) * m.cell,
            m.heights[row * m.columns + column],
        )
    };
    // quads in job space, a colour per corner
    let mut quads: Vec<([DVec3; 4], [Color32; 4])> = match look.wrap {
        Some(wrap) => wrapped_quads(m, wrap, look.light)
            .into_iter()
            .map(|(corners, brightness)| (corners, [shade(brightness); 4]))
            .collect(),
        None => vec![],
    };
    // with shadows, the surface's highest point, found once
    let shadows = look.shadows.then(|| shadow_map(m, look.light));
    for row in (1..m.rows).filter(|_| look.wrap.is_none()) {
        for column in 1..m.columns {
            let corners = [
                (column - 1, row - 1),
                (column, row - 1),
                (column, row),
                (column - 1, row),
            ];
            // nothing is drawn where there is no model (NaN)
            if corners
                .iter()
                .any(|&(c, r)| m.heights[r * m.columns + c].is_nan())
            {
                continue;
            }
            let grain = |c, r| look.grain.map_or(1.0, |g| g.on_cells(m.cell).at(top(c, r)));
            let colour = |c: usize, r: usize| {
                let b = if let Some(shadows) = &shadows {
                    shadowed_brightness(m, (c, r), look.light, shadows[r * m.columns + c])
                } else {
                    brightness(m, c, r, look.light)
                };
                match paint_of(m, r * m.columns + c) {
                    Some(paint) => shaded(paint, b),
                    None => shade(b * grain(c, r)),
                }
            };
            quads.push((
                corners.map(|(c, r)| top(c, r)),
                corners.map(|(c, r)| colour(c, r)),
            ));
        }
    }
    // painter's algorithm: far quads first
    let mut projected: Vec<(f64, [DVec3; 4], [Color32; 4])> = quads
        .into_iter()
        .map(|(corners, colours)| {
            let depth = corners.iter().map(|&c| orbit.project(c).1).sum::<f64>();
            (depth, corners, colours)
        })
        .collect();
    projected.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut quads: Vec<_> = projected.into_iter().map(|(_, q, c)| (q, c)).collect();
    let camera = Camera::new(m, orbit, framing, area, look.wrap);
    if look.sides && look.wrap.is_none() {
        quads.extend(block_faces(m, &top, &camera, orbit, &shade, look.light));
    }
    let mut mesh = Mesh::default();
    for (corners, colours) in quads {
        let first = u32::try_from(mesh.vertices.len()).expect("mesh fits u32 indices");
        for (corner, colour) in corners.into_iter().zip(colours) {
            mesh.colored_vertex(camera.screen(corner), colour);
        }
        mesh.add_triangle(first, first + 1, first + 2);
        mesh.add_triangle(first, first + 2, first + 3);
    }
    mesh
}

/// The block's sides that face the viewer, from the surface down to its underside, and the
/// underside seen from below: flat, or (a double-sided preview) cut from below. The block
/// is all behind them, so they go after the top.
pub(super) fn block_faces(
    m: &Material,
    top: &dyn Fn(usize, usize) -> DVec3,
    camera: &Camera,
    orbit: Orbit,
    shade: &dyn Fn(f64) -> Color32,
    light: DVec3,
) -> Vec<([DVec3; 4], [Color32; 4])> {
    let mut quads = vec![];
    let (last_column, last_row) = (m.columns - 1, m.rows - 1);
    let sides: [(DVec3, Vec<(usize, usize)>); 4] = [
        (DVec3::NEG_Y, (0..m.columns).map(|c| (c, 0)).collect()),
        (DVec3::Y, (0..m.columns).map(|c| (c, last_row)).collect()),
        (DVec3::NEG_X, (0..m.rows).map(|r| (0, r)).collect()),
        (DVec3::X, (0..m.rows).map(|r| (last_column, r)).collect()),
    ];
    // the underside: flat at the bottom, or (a double-sided preview) cut from below
    let floor = |c: usize, r: usize| {
        let p = top(c, r);
        let z = m.under.get(r * m.columns + c).copied().unwrap_or(m.bottom);
        DVec3::new(p.x, p.y, z)
    };
    for (outward, edge) in sides {
        if !camera.faces(floor(edge[0].0, edge[0].1), outward) {
            continue;
        }
        let colour = shade(lit(outward, light));
        for pair in edge.windows(2) {
            let (a, b) = (top(pair[0].0, pair[0].1), top(pair[1].0, pair[1].1));
            let (fa, fb) = (floor(pair[0].0, pair[0].1), floor(pair[1].0, pair[1].1));
            if a.z.is_nan() || b.z.is_nan() || fa.z.is_nan() || fb.z.is_nan() {
                continue;
            }
            quads.push(([a, b, fb, fa], [colour; 4]));
        }
    }
    if m.under.is_empty() {
        let bottom = [
            (0, 0),
            (last_column, 0),
            (last_column, last_row),
            (0, last_row),
        ]
        .map(|(c, r)| floor(c, r));
        if camera.faces(bottom[0], DVec3::NEG_Z) {
            quads.push((bottom, [shade(lit(DVec3::NEG_Z, light)); 4]));
        }
    } else if camera.faces(floor(0, 0), DVec3::NEG_Z) {
        quads.extend(underside(m, &floor, camera, orbit, shade, light));
    }
    quads
}

/// The underside's quads cut from below, seen from below (it faces the viewer), lit as
/// they face and far to near.
pub(super) fn underside(
    m: &Material,
    floor: &dyn Fn(usize, usize) -> DVec3,
    camera: &Camera,
    orbit: Orbit,
    shade: &dyn Fn(f64) -> Color32,
    light: DVec3,
) -> Vec<([DVec3; 4], [Color32; 4])> {
    let mut quads = vec![];
    for row in 1..m.rows {
        for column in 1..m.columns {
            // wound the other way round from the top, as seen from below
            let corners = [
                (column - 1, row - 1),
                (column - 1, row),
                (column, row),
                (column, row - 1),
            ]
            .map(|(c, r)| floor(c, r));
            if corners.iter().any(|p| p.z.is_nan()) {
                continue;
            }
            let normal = (corners[1] - corners[0]).cross(corners[3] - corners[0]);
            let normal = if normal.z > 0.0 { -normal } else { normal };
            if !camera.faces(corners[0], normal) {
                continue;
            }
            quads.push((corners, [shade(lit(normal, light)); 4]));
        }
    }
    // far first
    quads.sort_by(|a, b| {
        let depth = |q: &[DVec3; 4]| q.iter().map(|&c| orbit.project(c).1).sum::<f64>();
        depth(&a.0).total_cmp(&depth(&b.0))
    });
    quads
}

/// The model's body under its surface, so it shows solid rather than as a skin: walls
/// hanging from the edge of the surface the grid draws (its quads between four cells that
/// have a model) down to the modeling plane (z = 0), and a bottom there; wrapped round the
/// axis (`wrap`), the walls go down to the axis and need no bottom. Triangles in the
/// surface's coordinates, wound to face out.
#[expect(clippy::cast_precision_loss, reason = "cell counts")]
pub fn solid(material: &Material, wrap: Option<Wrap>) -> Vec<cmd_orientate_3d_model::Seen> {
    let (cols, rows, cell) = (material.columns, material.rows, material.cell);
    if cols < 2 || rows < 2 {
        return vec![];
    }
    let height = |col: usize, row: usize| material.heights[row * cols + col];
    // the quads the surface draws, round again past the end the way the job wraps
    let (wraps_c, wraps_r) = (
        wrap.is_some_and(|w| !w.along_x),
        wrap.is_some_and(|w| w.along_x),
    );
    let quads = |n: usize, wraps: bool| if wraps { n } else { n - 1 };
    let quad = |col: usize, row: usize| {
        col < quads(cols, wraps_c)
            && row < quads(rows, wraps_r)
            && [(0, 0), (1, 0), (1, 1), (0, 1)]
                .iter()
                .all(|&(dc, dr)| !height((col + dc) % cols, (row + dr) % rows).is_nan())
    };
    // the quad before, if there is one
    let before = |i: usize, n: usize, wraps: bool| match i {
        0 => wraps.then(|| n - 1),
        i => Some(i - 1),
    };
    let floor = wrap.map_or(0.0, |w| w.surface - w.radius);
    let round = |along_x: bool| if along_x { rows } else { cols } as f64 * cell;
    let flat = |col: usize, row: usize, z: f64| {
        DVec3::new((col as f64 + 0.5) * cell, (row as f64 + 0.5) * cell, z)
    };
    let at = |col: usize, row: usize, z: f64| {
        let p = flat(col, row, z);
        wrap.map_or(p, |w| w.place(p, round(w.along_x)))
    };
    let mut out = vec![];
    // a wall below the edge between two cells, facing `outward` before wrapping
    let mut wall = |from: (usize, usize), to: (usize, usize), outward: DVec3| {
        let along = flat(to.0, to.1, 0.0) - flat(from.0, from.1, 0.0);
        let (from, to) = if along.cross(DVec3::Z).dot(outward) < 0.0 {
            (to, from)
        } else {
            (from, to)
        };
        let top = (
            at(from.0, from.1, height(from.0, from.1)),
            at(to.0, to.1, height(to.0, to.1)),
        );
        let low = (at(from.0, from.1, floor), at(to.0, to.1, floor));
        out.push(([low.0, low.1, top.1], false));
        out.push(([low.0, top.1, top.0], false));
    };
    for row in 0..rows {
        for col in 0..cols {
            let (right, up) = ((col + 1) % cols, (row + 1) % rows);
            let here = quad(col, row);
            // the edge along the bottom of this quad, drawn where only one side has a quad
            let below = before(row, rows, wraps_r).is_some_and(|r| quad(col, r));
            match (here, below) {
                (true, false) => wall((col, row), (right, row), DVec3::NEG_Y),
                (false, true) => wall((col, row), (right, row), DVec3::Y),
                _ => {}
            }
            // the edge along its left side
            let left = before(col, cols, wraps_c).is_some_and(|c| quad(c, row));
            match (here, left) {
                (true, false) => wall((col, row), (col, up), DVec3::NEG_X),
                (false, true) => wall((col, row), (col, up), DVec3::X),
                _ => {}
            }
        }
    }
    if wrap.is_none() {
        // the bottom, a strip for each run of quads along a row, facing down
        for row in 0..rows - 1 {
            let mut col = 0;
            while col < cols - 1 {
                if !quad(col, row) {
                    col += 1;
                    continue;
                }
                let first = col;
                while col < cols - 1 && quad(col, row) {
                    col += 1;
                }
                let corners = [
                    flat(first, row, floor),
                    flat(first, row + 1, floor),
                    flat(col, row + 1, floor),
                    flat(col, row, floor),
                ];
                out.push(([corners[0], corners[1], corners[2]], false));
                out.push(([corners[0], corners[2], corners[3]], false));
            }
        }
    }
    out
}

/// Triangles in the material's coordinates (as [`screen_of`] takes them), lit as the
/// surface is and drawn far to near with their backs left out; the `true` ones in `mark`.
// ponytail: sorted every frame; cache by view as fine_mesh does if big models drag
pub fn triangles(
    tris: &[cmd_orientate_3d_model::Seen],
    screen: impl Fn(DVec3) -> Pos2,
    orbit: Orbit,
    look: &Look,
    mark: Color32,
) -> Mesh {
    let towards = orbit.towards_viewer();
    let mut seen: Vec<(f64, &[DVec3; 3], Color32, DVec3)> = tris
        .iter()
        .filter_map(|(t, marked)| {
            let normal = (t[1] - t[0]).cross(t[2] - t[0]).normalize_or_zero();
            let depth = t.iter().map(|&c| orbit.project(c).1).sum::<f64>();
            let colour = if *marked { mark } else { look.colour };
            (normal.dot(towards) > 0.0).then_some((depth, t, colour, normal))
        })
        .collect();
    seen.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut mesh = Mesh::default();
    for (_, t, colour, normal) in seen {
        let shade = shaded(colour, lit(normal, look.light));
        let first = u32::try_from(mesh.vertices.len()).expect("mesh fits u32 indices");
        for &corner in t {
            mesh.colored_vertex(screen(corner), shade);
        }
        mesh.add_triangle(first, first + 1, first + 2);
    }
    mesh
}

/// A model's faces split at the material's edges (`area`, in plan): the parts over it,
/// each keeping whether a cutter can reach it, and the parts past its edges, which the 3D
/// view shows as a faint red ghost rather than leaving out. A face across an edge is cut
/// along it, its pieces keeping its winding.
pub fn split_at_edges(
    faces: &[cmd_orientate_3d_model::Seen],
    area: cut_craft_core::geometry::Rect,
) -> (Vec<cmd_orientate_3d_model::Seen>, Vec<[DVec3; 3]>) {
    let (mut over, mut past) = (Vec::with_capacity(faces.len()), vec![]);
    // each edge as an axis (0 x, 1 y), where it is, and which side is the material's
    let edges = [
        (0, area.min.x, 1.0),
        (0, area.max.x, -1.0),
        (1, area.min.y, 1.0),
        (1, area.max.y, -1.0),
    ];
    for &(t, marked) in faces {
        let inside = |v: &DVec3| {
            edges
                .iter()
                .all(|&(axis, at, side)| (v[axis] - at) * side >= 0.0)
        };
        if t.iter().all(inside) {
            over.push((t, marked));
            continue;
        }
        let mut kept = t.to_vec();
        for &(axis, at, side) in &edges {
            let (keep, cut) = halves(&kept, |v| (v[axis] - at) * side);
            past.extend(fan(&cut));
            kept = keep;
            if kept.len() < 3 {
                break;
            }
        }
        over.extend(fan(&kept).into_iter().map(|t| (t, marked)));
    }
    (over, past)
}

/// A convex polygon cut by a line: the part where `side` is not negative, and the rest.
pub(super) fn halves(polygon: &[DVec3], side: impl Fn(&DVec3) -> f64) -> (Vec<DVec3>, Vec<DVec3>) {
    let (mut keep, mut cut) = (vec![], vec![]);
    for (i, &a) in polygon.iter().enumerate() {
        let b = polygon[(i + 1) % polygon.len()];
        let (sa, sb) = (side(&a), side(&b));
        // a corner on the line belongs to both
        if sa >= 0.0 {
            keep.push(a);
        }
        if sa <= 0.0 {
            cut.push(a);
        }
        if (sa > 0.0 && sb < 0.0) || (sa < 0.0 && sb > 0.0) {
            let crossing = a + (b - a) * (sa / (sa - sb));
            keep.push(crossing);
            cut.push(crossing);
        }
    }
    (keep, cut)
}

/// A convex polygon as triangles fanned from its first corner, leaving out slivers with
/// no area (a polygon only touching the line, or under three corners, makes none).
pub(super) fn fan(polygon: &[DVec3]) -> Vec<[DVec3; 3]> {
    (1..polygon.len().saturating_sub(1))
        .map(|i| [polygon[0], polygon[i], polygon[i + 1]])
        .filter(|t| (t[1] - t[0]).cross(t[2] - t[0]).length_squared() > 0.0)
        .collect()
}

/// The lines along a rotary stock's wireframe, and the sides of each end's ring.
pub(super) const CYLINDER_SIDES: usize = 24;

/// A wireframe of the cylinder of `radius` round the axis (y = z = 0 along x, else x = z = 0
/// along y) from `from` to `to` along it: a ring at each end and lines along between.
#[expect(clippy::cast_precision_loss, reason = "a handful of sides")]
pub fn cylinder_edges(along_x: bool, radius: f64, from: f64, to: f64) -> Vec<[DVec3; 2]> {
    let at = |side: usize, along: f64| {
        let angle = std::f64::consts::TAU * side as f64 / CYLINDER_SIDES as f64;
        let (across, up) = (radius * angle.sin(), radius * angle.cos());
        if along_x {
            DVec3::new(along, across, up)
        } else {
            DVec3::new(across, along, up)
        }
    };
    (0..CYLINDER_SIDES)
        .flat_map(|i| {
            [
                [at(i, from), at(i + 1, from)],
                [at(i, to), at(i + 1, to)],
                [at(i, from), at(i, to)],
            ]
        })
        .collect()
}

/// A wireframe of a rotary job's material round the axis from `from` to `to` along it: the
/// cylinder of `radius`, or with an `outline` (half width across, half thickness up, as
/// [`Wrap::outline`]) the square's or board's box.
pub fn stock_edges(
    along_x: bool,
    radius: f64,
    outline: Option<DVec2>,
    from: f64,
    to: f64,
) -> Vec<[DVec3; 2]> {
    let Some(half) = outline else {
        return cylinder_edges(along_x, radius, from, to);
    };
    if along_x {
        box_edges(
            DVec3::new(from, -half.x, -half.y),
            DVec3::new(to, half.x, half.y),
        )
    } else {
        box_edges(
            DVec3::new(-half.x, from, -half.y),
            DVec3::new(half.x, to, half.y),
        )
    }
}

/// The twelve edges of the box from `lo` to `hi`.
pub fn box_edges(lo: DVec3, hi: DVec3) -> Vec<[DVec3; 2]> {
    let corner = |i: usize| {
        DVec3::new(
            if i & 1 == 1 { hi.x } else { lo.x },
            if i & 2 == 2 { hi.y } else { lo.y },
            if i & 4 == 4 { hi.z } else { lo.z },
        )
    };
    // corners one bit apart share an edge
    (0..8)
        .flat_map(|i| [1, 2, 4].map(move |bit| (i, i | bit)))
        .filter(|(i, j)| i != j)
        .map(|(i, j)| [corner(i), corner(j)])
        .collect()
}
