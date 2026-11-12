//! Shaping an estimated depth into relief heights: the photo's backdrop cut away, the
//! subject lifted clear of it, fine detail from the photo's own shading added back (a depth
//! network smooths away hair, scales and faces) and the noise smoothed while keeping edges.
//! Every grid is row by row from the top, `columns` wide.

use cut_craft_core::toolpath::Tool;
/// A grid of values, row by row from the top.
#[derive(Clone, Debug, PartialEq)]
pub struct Grid {
    pub columns: usize,
    pub rows: usize,
    pub values: Vec<f32>,
}

impl Grid {
    pub fn new(columns: usize, rows: usize, values: Vec<f32>) -> Self {
        assert_eq!(values.len(), columns * rows, "grid size");
        Self {
            columns,
            rows,
            values,
        }
    }

    fn at(&self, column: usize, row: usize) -> f32 {
        self.values[row * self.columns + column]
    }
}

/// `grid` shrunk to `columns` by `rows` (no bigger than it is), each new pixel the average
/// of the old ones under it: shaping runs at the relief's own resolution, not the photo's.
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "pixel counts"
)]
pub fn shrunk(grid: &Grid, columns: usize, rows: usize) -> Grid {
    let (columns, rows) = (
        columns.clamp(1, grid.columns.max(1)),
        rows.clamp(1, grid.rows.max(1)),
    );
    if (columns, rows) == (grid.columns, grid.rows) {
        return grid.clone();
    }
    // the source pixels under destination pixel `i` of `n`, along a side `of` long
    let span = |i: usize, n: usize, of: usize| {
        let start = (i as f64 * of as f64 / n as f64).floor() as usize;
        let end = (((i + 1) as f64 * of as f64 / n as f64).ceil() as usize).clamp(start + 1, of);
        start..end
    };
    let mut values = Vec::with_capacity(columns * rows);
    for r in 0..rows {
        let ys = span(r, rows, grid.rows);
        for c in 0..columns {
            let xs = span(c, columns, grid.columns);
            let mut sum = 0.0;
            for y in ys.clone() {
                for x in xs.clone() {
                    sum += grid.at(x, y);
                }
            }
            values.push(sum / (ys.len() * xs.len()) as f32);
        }
    }
    Grid::new(columns, rows, values)
}

/// Separable Gaussian blur, clamped at the edges.
pub fn gaussian(grid: &Grid, sigma: f32) -> Grid {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a kernel a few pixels wide"
    )]
    let reach = (3.0 * sigma).round().max(0.0) as i16;
    let kernel: Vec<f32> = (-reach..=reach)
        .map(|x| (-f32::from(x * x) / (2.0 * sigma * sigma)).exp())
        .collect();
    let reach = isize::from(reach);
    let total: f32 = kernel.iter().sum();
    let kernel: Vec<f32> = kernel.iter().map(|k| k / total).collect();
    let (columns, rows) = (grid.columns, grid.rows);
    #[expect(
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        reason = "clamped to the grid"
    )]
    let pass = |source: &[f32], across: bool| -> Vec<f32> {
        let mut out = vec![0.0; source.len()];
        for row in 0..rows {
            for column in 0..columns {
                let mut sum = 0.0;
                for (k, weight) in kernel.iter().enumerate() {
                    let offset = k as isize - reach;
                    let (c, r) = if across {
                        (
                            (column as isize + offset).clamp(0, columns as isize - 1) as usize,
                            row,
                        )
                    } else {
                        (
                            column,
                            (row as isize + offset).clamp(0, rows as isize - 1) as usize,
                        )
                    };
                    sum += weight * source[r * columns + c];
                }
                out[row * columns + column] = sum;
            }
        }
        out
    };
    let values = pass(&pass(&grid.values, true), false);
    Grid::new(columns, rows, values)
}

/// One pass of edge-preserving (Perona-Malik) smoothing: steps smaller than `edge` (of the
/// full height) melt, larger ones (outlines, the edges of scales) stay sharp.
pub fn diffuse(grid: &Grid, edge: f32) -> Grid {
    const STEP: f32 = 0.2;
    let (columns, rows) = (grid.columns, grid.rows);
    let k2 = edge * edge;
    let mut values = grid.values.clone();
    for row in 0..rows {
        for column in 0..columns {
            let here = grid.at(column, row);
            let neighbours = [
                grid.at(column, row.saturating_sub(1)),
                grid.at(column, (row + 1).min(rows - 1)),
                grid.at(column.saturating_sub(1), row),
                grid.at((column + 1).min(columns - 1), row),
            ];
            let flux: f32 = neighbours
                .iter()
                .map(|n| {
                    let d = n - here;
                    d / (1.0 + d * d / k2)
                })
                .sum();
            values[row * columns + column] = here + STEP * flux;
        }
    }
    Grid::new(columns, rows, values)
}

/// The backdrop's tone: the middle tone along the picture's border.
pub fn backdrop_tone(light: &Grid) -> f32 {
    let (columns, rows) = (light.columns, light.rows);
    let mut border: Vec<f32> = (0..rows)
        .flat_map(|row| (0..columns).map(move |column| (column, row)))
        .filter(|&(c, r)| c == 0 || r == 0 || c + 1 == columns || r + 1 == rows)
        .map(|(c, r)| light.at(c, r))
        .collect();
    border.sort_by(f32::total_cmp);
    border.get(border.len() / 2).copied().unwrap_or(0.0)
}

/// The subject: pixels whose tone differs from the backdrop's by more than `cut_off`, only
/// the largest connected patch of them (specks of the backdrop dropped). A cut-off of 0
/// keeps the whole picture.
pub fn subject(light: &Grid, cut_off: f32) -> Vec<bool> {
    if cut_off <= 0.0 {
        return vec![true; light.values.len()];
    }
    largest_patch(&differs(light, cut_off), light.columns, light.rows)
}

/// Pixels whose tone differs from the backdrop's by more than `cut_off`.
fn differs(light: &Grid, cut_off: f32) -> Vec<bool> {
    let backdrop = backdrop_tone(light);
    light
        .values
        .iter()
        .map(|v| (v - backdrop).abs() > cut_off)
        .collect()
}

/// Only the largest connected patch of `mask` (4 neighbours), found by flood fill.
fn largest_patch(mask: &[bool], columns: usize, rows: usize) -> Vec<bool> {
    let mut patch = vec![usize::MAX; mask.len()];
    let (mut best, mut best_size) = (usize::MAX, 0);
    for start in 0..mask.len() {
        if !mask[start] || patch[start] != usize::MAX {
            continue;
        }
        let mut size = 0;
        let mut stack = vec![start];
        patch[start] = start;
        while let Some(at) = stack.pop() {
            size += 1;
            let (c, r) = (at % columns, at / columns);
            let mut visit = |next: usize| {
                if mask[next] && patch[next] == usize::MAX {
                    patch[next] = start;
                    stack.push(next);
                }
            };
            if c > 0 {
                visit(at - 1);
            }
            if c + 1 < columns {
                visit(at + 1);
            }
            if r > 0 {
                visit(at - columns);
            }
            if r + 1 < rows {
                visit(at + columns);
            }
        }
        if size > best_size {
            (best, best_size) = (start, size);
        }
    }
    patch.iter().map(|&p| p == best).collect()
}

/// `values` from 0 (the farthest, ignoring the farthest half percent) to 1 (the nearest,
/// likewise), clamped.
fn normalised(values: &[f32]) -> Vec<f32> {
    let mut sorted = values.to_vec();
    let (lo, hi) = (
        percentile(&mut sorted, 0.005),
        percentile(&mut sorted, 0.995),
    );
    let span = (hi - lo).max(1e-6);
    values
        .iter()
        .map(|v| ((v - lo) / span).clamp(0.0, 1.0))
        .collect()
}

/// `mask` grown by one pixel (8 neighbours).
fn dilate(mask: &[bool], columns: usize, rows: usize) -> Vec<bool> {
    (0..mask.len())
        .map(|at| {
            let (c, r) = (at % columns, at / columns);
            (r.saturating_sub(1)..=(r + 1).min(rows - 1)).any(|rr| {
                (c.saturating_sub(1)..=(c + 1).min(columns - 1)).any(|cc| mask[rr * columns + cc])
            })
        })
        .collect()
}

/// The `fraction` (0 to 1) percentile of `values`, nearest rank.
fn percentile(values: &mut [f32], fraction: f32) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(f32::total_cmp);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "an index into the slice"
    )]
    let at = (fraction * (values.len() - 1) as f32).round() as usize;
    values[at.min(values.len() - 1)]
}

/// `mask` shrunk by one pixel (4 neighbours), off the grid counting as outside.
fn erode(mask: &[bool], columns: usize, rows: usize) -> Vec<bool> {
    (0..mask.len())
        .map(|at| {
            let (c, r) = (at % columns, at / columns);
            mask[at]
                && c > 0
                && r > 0
                && c + 1 < columns
                && r + 1 < rows
                && mask[at - 1]
                && mask[at + 1]
                && mask[at - columns]
                && mask[at + columns]
        })
        .collect()
}

/// The fine detail of the photo's shading inside the subject, about -1 to 1: its tone less
/// the tone blurred over `radius` pixels, faded out near the outline so the photo's rim
/// does not carve a groove.
pub fn shading_detail(light: &Grid, subject: &[bool], radius: f32) -> Grid {
    let (columns, rows) = (light.columns, light.rows);
    let inside = Grid::new(
        columns,
        rows,
        subject.iter().map(|&s| if s { 1.0 } else { 0.0 }).collect(),
    );
    let masked = Grid::new(
        columns,
        rows,
        light
            .values
            .iter()
            .zip(&inside.values)
            .map(|(l, f)| l * f)
            .collect(),
    );
    // blurred over the subject only, so the backdrop cannot halo the outline
    let (sum, weight) = (gaussian(&masked, radius), gaussian(&inside, radius));
    let mut detail: Vec<f32> = (0..light.values.len())
        .map(|i| (light.values[i] - sum.values[i] / weight.values[i].max(1e-6)) * inside.values[i])
        .collect();
    let mut sizes: Vec<f32> = detail
        .iter()
        .zip(subject)
        .filter(|&(_, &s)| s)
        .map(|(d, _)| d.abs())
        .collect();
    let scale = percentile(&mut sizes, 0.99).max(1e-6);
    // fade in over `radius` pixels from the outline
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a few pixels"
    )]
    let steps = radius.round().max(1.0) as usize;
    let mut depth_in = vec![0.0_f32; detail.len()];
    let mut mask = subject.to_vec();
    for _ in 0..steps {
        mask = erode(&mask, columns, rows);
        for (d, &m) in depth_in.iter_mut().zip(&mask) {
            if m {
                *d += 1.0;
            }
        }
    }
    #[expect(clippy::cast_precision_loss, reason = "a few pixels")]
    let steps = steps as f32;
    for (d, inner) in detail.iter_mut().zip(&depth_in) {
        *d = *d / scale * inner / steps;
    }
    Grid::new(columns, rows, detail)
}

/// How to shape an estimate.
#[derive(Clone, Copy, Debug, Default)]
pub struct Shaping {
    /// See [`subject`].
    pub cut_off: f32,
    /// Depth cut-off, 0 to 1: everything farther than this share of the way from the
    /// farthest to the nearest point is background (depth2stl's floor). 0 cuts nothing.
    pub far: f32,
    /// Near and far swapped: for a depth map drawn black-nearest.
    pub invert: bool,
    /// How much to sharpen the relief's own detail finer than `radius` (depth2stl's
    /// boost), 0 for none.
    pub sharpen: f32,
    /// Pixels round the subject over which its edge slopes down to the background instead
    /// of dropping as a wall (depth2stl's fade); 0 for a wall.
    pub soft_edge: usize,
    /// Height of the subject's lowest point above the backdrop, 0 to 1.
    pub lift: f32,
    /// How much of the photo's shading detail to add, 0 for none.
    pub detail: f32,
    /// Detail finer than this many pixels comes from the photo.
    pub radius: f32,
    /// Passes of edge-preserving smoothing.
    pub smoothing: usize,
}

/// Heights from 0 (backdrop) to 1 (nearest), from the photo's tones (`light`, 0 black to
/// 1 white) and the network's estimate (`depth`, bigger nearer, any scale).
pub fn heights(light: &Grid, depth: &Grid, shaping: &Shaping) -> Grid {
    const EDGE: f32 = 0.05;
    assert_eq!(
        (light.columns, light.rows),
        (depth.columns, depth.rows),
        "photo and depth the same size"
    );
    let (columns, rows) = (depth.columns, depth.rows);
    let depth: Vec<f32> = if shaping.invert {
        depth.values.iter().map(|d| -d).collect()
    } else {
        depth.values.clone()
    };
    // the background: what differs too little from the border's tone, or lies too far
    let mut keep = if shaping.cut_off > 0.0 {
        differs(light, shaping.cut_off)
    } else {
        vec![true; depth.len()]
    };
    if shaping.far > 0.0 {
        for (k, d) in keep.iter_mut().zip(normalised(&depth)) {
            *k &= d > shaping.far;
        }
    }
    let mut fig = largest_patch(&keep, columns, rows);
    if !fig.contains(&true) {
        fig = vec![true; fig.len()];
    }
    let mut inside: Vec<f32> = depth
        .iter()
        .zip(&fig)
        .filter(|&(_, &f)| f)
        .map(|(d, _)| *d)
        .collect();
    // the extremes ignored, so a stray pixel cannot squash the rest
    let (lo, hi) = (
        percentile(&mut inside, 0.005),
        percentile(&mut inside, 0.995),
    );
    let span = (hi - lo).max(1e-6);
    let mut h: Vec<f32> = depth
        .iter()
        .map(|d| ((d - lo) / span).clamp(0.0, 1.0))
        .collect();
    if shaping.detail > 0.0 {
        let detail = shading_detail(light, &fig, shaping.radius);
        for (h, d) in h.iter_mut().zip(&detail.values) {
            *h = (*h + shaping.detail * 0.5 * d).clamp(0.0, 1.0);
        }
    }
    if shaping.sharpen > 0.0 {
        let blurred = gaussian(&Grid::new(columns, rows, h.clone()), shaping.radius);
        for ((h, b), &f) in h.iter_mut().zip(&blurred.values).zip(&fig) {
            if f {
                *h = (*h + shaping.sharpen * (*h - b)).clamp(0.0, 1.0);
            }
        }
    }
    let lift = shaping.lift.clamp(0.0, 1.0);
    for (h, &f) in h.iter_mut().zip(&fig) {
        *h = if f { lift + (1.0 - lift) * *h } else { 0.0 };
    }
    let mut grid = Grid::new(columns, rows, h);
    for _ in 0..shaping.smoothing {
        grid = diffuse(&grid, EDGE);
    }
    for (h, &f) in grid.values.iter_mut().zip(&fig) {
        if !f {
            *h = 0.0;
        }
    }
    if shaping.soft_edge > 0 {
        soften(&mut grid, &fig, lift, shaping.soft_edge);
    }
    grid
}

/// Round the subject, a slope down to 0 over `width` pixels (smoothstep) from the height of
/// the outline it leaves, but never from below `floor`, so the relief has no wall there.
#[expect(clippy::cast_precision_loss, reason = "a few pixels")]
fn soften(grid: &mut Grid, fig: &[bool], floor: f32, width: usize) {
    let (columns, rows) = (grid.columns, grid.rows);
    let mut reached = fig.to_vec();
    // the height each reached pixel slopes down from: the outline's, carried outward
    let mut from: Vec<f32> = grid
        .values
        .iter()
        .zip(fig)
        .map(|(&h, &f)| if f { h.max(floor) } else { 0.0 })
        .collect();
    for step in 1..=width {
        let grown = dilate(&reached, columns, rows);
        let r = 1.0 - step as f32 / (width + 1) as f32;
        let fall = r * r * (3.0 - 2.0 * r);
        let mut next = from.clone();
        for at in (0..reached.len()).filter(|&at| grown[at] && !reached[at]) {
            let (c, row) = (at % columns, at / columns);
            let top = (row.saturating_sub(1)..=(row + 1).min(rows - 1))
                .flat_map(|rr| {
                    (c.saturating_sub(1)..=(c + 1).min(columns - 1))
                        .map(move |cc| rr * columns + cc)
                })
                .filter(|&n| reached[n])
                .map(|n| from[n])
                .fold(0.0, f32::max);
            next[at] = top;
            grid.values[at] = top * fall;
        }
        from = next;
        reached = grown;
    }
}

/// What `bit` (a tool from the Tool Database, the one Calculate's reach check uses) can
/// carve of `heights` (cells `cell` apart, a height of 1 standing `relief` high): the bit is lowered everywhere as far as it goes without cutting into the
/// relief, and what its shape leaves behind is the answer. Pits and grooves too narrow for
/// it fill in and the foot of a wall slopes at its taper; nothing is ever cut lower than
/// asked. (depth2stl's carve simulation.)
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "cell offsets of a kernel a few dozen cells wide"
)]
pub fn carvable(heights: &Grid, cell: f64, relief: f64, bit: &Tool) -> Grid {
    if cell <= 0.0 || relief <= 0.0 || bit.diameter <= 0.0 || heights.values.is_empty() {
        return heights.clone();
    }
    // no part of the bit higher than the relief's whole range can touch it
    let (lo, hi) = heights
        .values
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), &v| {
            (lo.min(v), hi.max(v))
        });
    let range = f64::from(hi - lo);
    let reach = bit.reach_below(range * relief);
    let most = (reach / cell).ceil().max(1.0) as isize;
    // each cell offset under the bit, with the bit's height there in relief heights
    let mut under: Vec<(isize, isize, f32)> = Vec::new();
    for dy in -most..=most {
        for dx in -most..=most {
            let r = ((dx * dx + dy * dy) as f64).sqrt() * cell;
            if let Some(t) = bit.profile(r).map(|t| t / relief)
                && t <= range
            {
                under.push((dx, dy, t as f32));
            }
        }
    }
    // lowest part of the bit first, so a cell can stop looking once the rest of the bit
    // is too high to matter there
    under.sort_by(|a, b| a.2.total_cmp(&b.2));
    let (columns, rows) = (heights.columns, heights.rows);
    let at = |values: &[f32], column: isize, row: isize| {
        let column = column.clamp(0, columns as isize - 1) as usize;
        let row = row.clamp(0, rows as isize - 1) as usize;
        values[row * columns + column]
    };
    let most = most as usize;
    // how low the tip can go over each cell: resting on the highest thing under the bit.
    // Nothing under the bit is higher than the highest cell within its reach, so on
    // smooth ground only the bit's tip is ever looked at.
    let highest = within(&heights.values, columns, most, |a, b| a >= b);
    let mut tip = vec![0.0_f32; heights.values.len()];
    cut_craft_core::simulation::by_rows(&mut tip, columns, |first, band| {
        for (i, out) in band.iter_mut().enumerate() {
            let at_cell = first * columns + i;
            let (column, row) = ((i % columns) as isize, (first + i / columns) as isize);
            let mut best = heights.values[at_cell];
            for &(dx, dy, t) in &under {
                if highest[at_cell] - t <= best {
                    break;
                }
                best = best.max(at(&heights.values, column + dx, row + dy) - t);
            }
            *out = best;
        }
    });
    // what is left: the lowest the bit's surface came over each cell
    let lowest = within(&tip, columns, most, |a, b| a <= b);
    let mut left = vec![0.0_f32; tip.len()];
    cut_craft_core::simulation::by_rows(&mut left, columns, |first, band| {
        for (i, out) in band.iter_mut().enumerate() {
            let at_cell = first * columns + i;
            let (column, row) = ((i % columns) as isize, (first + i / columns) as isize);
            let mut best = tip[at_cell];
            for &(dx, dy, t) in &under {
                if lowest[at_cell] + t >= best {
                    break;
                }
                best = best.min(at(&tip, column - dx, row - dy) + t);
            }
            *out = best.clamp(0.0, 1.0);
        }
    });
    Grid::new(columns, rows, left)
}

/// For each cell, the value `wins` picks (the highest, or the lowest) of the square of
/// cells within `radius` of it, in time that does not grow with `radius`.
fn within(
    values: &[f32],
    columns: usize,
    radius: usize,
    wins: impl Fn(f32, f32) -> bool + Copy,
) -> Vec<f32> {
    let rows = values.len() / columns;
    let mut along = vec![0.0_f32; values.len()];
    for (line, out) in values.chunks(columns).zip(along.chunks_mut(columns)) {
        window(line, radius, wins, out);
    }
    let mut out = vec![0.0_f32; values.len()];
    let (mut line, mut swept) = (vec![0.0_f32; rows], vec![0.0_f32; rows]);
    for column in 0..columns {
        for (row, v) in line.iter_mut().enumerate() {
            *v = along[row * columns + column];
        }
        window(&line, radius, wins, &mut swept);
        for (row, v) in swept.iter().enumerate() {
            out[row * columns + column] = *v;
        }
    }
    out
}

/// [`within`] along one line: a sliding window that keeps only the cells that could
/// still win.
fn window(line: &[f32], radius: usize, wins: impl Fn(f32, f32) -> bool, out: &mut [f32]) {
    let mut held: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
    let mut next = 0;
    for (i, out) in out.iter_mut().enumerate() {
        while next < line.len() && next <= i + radius {
            while held.back().is_some_and(|&b| wins(line[next], line[b])) {
                held.pop_back();
            }
            held.push_back(next);
            next += 1;
        }
        while held.front().is_some_and(|&f| f + radius < i) {
            held.pop_front();
        }
        *out = held.front().map_or(line[i], |&f| line[f]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::toolpath::ToolShape;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    /// `size` x `size`, `back` everywhere but a centred square of `fill`, `width` wide.
    fn square(size: usize, width: usize, back: f32, fill: f32) -> Grid {
        let first = (size - width) / 2;
        let hit = |i: usize| (first..first + width).contains(&i);
        Grid::new(
            size,
            size,
            (0..size * size)
                .map(|i| {
                    if hit(i % size) && hit(i / size) {
                        fill
                    } else {
                        back
                    }
                })
                .collect(),
        )
    }

    #[test]
    fn shrinking_averages_the_pixels_under_each_new_one() {
        let grid = Grid::new(4, 2, vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]);
        assert_eq!(shrunk(&grid, 2, 1).values, [2.5, 4.5]);
        assert_eq!(shrunk(&grid, 8, 8), grid, "never grown");
    }

    #[test]
    fn gaussian_keeps_a_flat_grid_flat_and_spreads_a_spike() {
        let flat = Grid::new(5, 4, vec![0.3; 20]);
        assert!(gaussian(&flat, 2.0).values.iter().all(|&v| close(v, 0.3)));
        let mut spike = Grid::new(9, 9, vec![0.0; 81]);
        spike.values[40] = 1.0;
        let blurred = gaussian(&spike, 1.0);
        assert!(blurred.at(4, 4) < 1.0 && blurred.at(5, 4) > 0.0);
        assert!(close(blurred.at(5, 4), blurred.at(3, 4)), "symmetric");
        assert!(
            close(blurred.values.iter().sum::<f32>(), 1.0),
            "nothing lost"
        );
    }

    #[test]
    fn diffusion_melts_small_steps_and_keeps_big_ones() {
        let small = square(12, 4, 0.0, 0.01);
        let big = square(12, 4, 0.0, 0.8);
        let (mut s, mut b) = (small.clone(), big.clone());
        for _ in 0..20 {
            s = diffuse(&s, 0.05);
            b = diffuse(&b, 0.05);
        }
        let step = |g: &Grid| g.at(5, 5) - g.at(0, 0);
        assert!(
            step(&s) < 0.5 * step(&small),
            "small step melts: {}",
            step(&s)
        );
        assert!(step(&b) > 0.9 * step(&big), "big step kept: {}", step(&b));
    }

    #[test]
    fn the_backdrop_is_whatever_runs_round_the_border() {
        assert!(close(backdrop_tone(&square(10, 4, 0.9, 0.2)), 0.9), "light");
        assert!(
            close(backdrop_tone(&square(10, 4, 0.05, 0.6)), 0.05),
            "dark"
        );
    }

    #[test]
    fn the_subject_differs_from_the_backdrop_and_specks_are_dropped() {
        // a light backdrop (an ordinary photo), a dark subject and one stray dark pixel
        let mut light = square(10, 4, 0.9, 0.2);
        light.values[1] = 0.2;
        let fig = subject(&light, 0.1);
        assert!(fig[5 * 10 + 5], "subject");
        assert!(!fig[0] && !fig[1], "backdrop and speck");
        assert_eq!(fig.iter().filter(|&&f| f).count(), 16);
        assert!(
            subject(&light, 0.0).iter().all(|&f| f),
            "no cut-off keeps it all"
        );
    }

    #[test]
    fn shading_detail_is_zero_on_flat_tone_and_fades_at_the_outline() {
        let flat = square(20, 12, 0.0, 0.5);
        let fig = subject(&flat, 0.1);
        assert!(
            shading_detail(&flat, &fig, 3.0)
                .values
                .iter()
                .all(|v| close(*v, 0.0))
        );
        // a light dot in the middle of the subject stands out
        let mut dotted = flat.clone();
        dotted.values[10 * 20 + 10] = 0.9;
        let detail = shading_detail(&dotted, &fig, 3.0);
        assert!(detail.at(10, 10) > 0.5);
        assert!(close(detail.at(4, 10), 0.0), "the outline gets none");
    }

    /// A busy backdrop (tones all over) round a centred square that is near; the
    /// backdrop's depth wanders but stays far.
    fn busy_photo() -> (Grid, Grid) {
        let light = Grid::new(
            12,
            12,
            (0..144_u16)
                .map(|i| f32::from(i * 37 % 100) / 100.0)
                .collect(),
        );
        let near = square(12, 6, 0.0, 1.0);
        let depth = Grid::new(
            12,
            12,
            near.values
                .iter()
                .zip(0_u16..)
                .map(|(n, i)| n * 10.0 + 5.0 + f32::from(i % 3))
                .collect(),
        );
        (light, depth)
    }

    #[test]
    fn the_depth_cut_off_drops_what_is_far_whatever_its_tone() {
        let (light, depth) = busy_photo();
        let shaping = Shaping {
            far: 0.3,
            lift: 0.2,
            radius: 3.0,
            ..Shaping::default()
        };
        let h = heights(&light, &depth, &shaping);
        assert!(
            close(h.at(0, 0), 0.0) && close(h.at(1, 10), 0.0),
            "far backdrop cut"
        );
        assert!(
            h.at(5, 5) >= 0.2 - 1e-4,
            "near subject kept: {}",
            h.at(5, 5)
        );
        let kept = h.values.iter().filter(|&&v| v > 0.0).count();
        assert_eq!(kept, 36, "only the near square");
    }

    #[test]
    fn invert_swaps_near_and_far() {
        let light = Grid::new(3, 1, vec![0.5; 3]);
        let depth = Grid::new(3, 1, vec![0.0, 1.0, 2.0]);
        let plain = heights(&light, &depth, &Shaping::default());
        let swapped = heights(
            &light,
            &depth,
            &Shaping {
                invert: true,
                ..Shaping::default()
            },
        );
        assert!(plain.at(2, 0) > plain.at(0, 0));
        assert!(swapped.at(0, 0) > swapped.at(2, 0));
    }

    #[test]
    fn sharpen_makes_a_small_bump_stand_out_more() {
        let light = Grid::new(15, 15, vec![0.5; 225]);
        let mut depth = Grid::new(
            15,
            15,
            (0..225_u16).map(|i| f32::from(i % 15) / 14.0).collect(),
        );
        depth.values[7 * 15 + 7] += 0.1;
        let bump = |g: &Grid| g.at(7, 7) - f32::midpoint(g.at(6, 7), g.at(8, 7));
        let base = Shaping {
            radius: 3.0,
            ..Shaping::default()
        };
        let plain = heights(&light, &depth, &base);
        let sharp = heights(
            &light,
            &depth,
            &Shaping {
                sharpen: 1.0,
                ..base
            },
        );
        assert!(
            bump(&sharp) > bump(&plain) * 1.5,
            "{} {}",
            bump(&sharp),
            bump(&plain)
        );
    }

    #[test]
    fn a_soft_edge_slopes_down_round_the_subject() {
        let light = square(16, 6, 0.95, 0.3);
        let depth = Grid::new(16, 16, vec![1.0; 256]);
        let shaping = Shaping {
            cut_off: 0.1,
            lift: 0.5,
            radius: 3.0,
            soft_edge: 3,
            ..Shaping::default()
        };
        let h = heights(&light, &depth, &shaping);
        // the subject spans columns 5 to 10; row 8 runs out to the right
        let out: Vec<f32> = (10..16).map(|c| h.at(c, 8)).collect();
        assert!(out[0] >= 0.5 - 1e-4, "inside, lifted: {out:?}");
        assert!(out[1] > 0.0 && out[1] < 0.5, "sloping: {out:?}");
        assert!(out.windows(2).all(|w| w[1] <= w[0]), "downhill: {out:?}");
        assert!(
            close(out[4], 0.0) && close(out[5], 0.0),
            "flat beyond: {out:?}"
        );
    }

    #[test]
    fn a_soft_edge_starts_from_the_rim_s_own_height() {
        // the subject's right side stands at full height: the slope begins up there
        let light = square(16, 6, 0.95, 0.3);
        let depth = Grid::new(16, 16, (0..256_u16).map(|i| f32::from(i % 16)).collect());
        let shaping = Shaping {
            cut_off: 0.1,
            lift: 0.2,
            radius: 3.0,
            soft_edge: 3,
            ..Shaping::default()
        };
        let h = heights(&light, &depth, &shaping);
        let (rim, first) = (h.at(10, 8), h.at(11, 8));
        assert!(rim > 0.9, "{rim}");
        assert!(first > 0.5 * rim && first < rim, "no wall: {rim} {first}");
    }

    #[test]
    fn heights_put_the_backdrop_at_zero_and_the_subject_from_the_lift_up() {
        let light = square(12, 6, 0.95, 0.4);
        // the estimate rises left to right across the subject, at any scale
        let depth = Grid::new(
            12,
            12,
            (0..144_u16)
                .map(|i| 50.0 + f32::from(i % 12) * 10.0)
                .collect(),
        );
        let shaping = Shaping {
            cut_off: 0.1,
            lift: 0.2,
            radius: 3.0,
            ..Shaping::default()
        };
        let h = heights(&light, &depth, &shaping);
        assert!(
            close(h.at(0, 0), 0.0) && close(h.at(11, 6), 0.0),
            "backdrop"
        );
        assert!(close(h.at(3, 6), 0.2), "nearest-but-least lifted clear");
        assert!(close(h.at(8, 6), 1.0), "the nearest is the top");
        assert!(h.at(5, 6) > h.at(4, 6), "order kept");
    }

    /// A tapered ball nose `tip` across, `taper` degrees a side, up to a 1/4 inch shank.
    fn tapered_ball(tip: f64, taper: f64) -> Tool {
        Tool {
            shape: ToolShape::BallNose,
            diameter: tip,
            taper,
            shank_diameter: 6.35,
            ..Tool::default()
        }
    }

    /// 40 x 40 cells 0.05 mm apart, 8 mm of relief: a flat at half height with a one-cell
    /// pit, a wide plateau and a lone peak.
    fn carved_test() -> Grid {
        let mut values = vec![0.5; 1600];
        values[20 * 40 + 20] = 0.0;
        for row in 5..15 {
            for column in 5..15 {
                values[row * 40 + column] = 0.9;
            }
        }
        values[30 * 40 + 30] = 1.0;
        Grid::new(40, 40, values)
    }

    #[test]
    fn a_pit_narrower_than_the_bit_fills_in_and_tops_survive() {
        let flat = carved_test();
        let bit = tapered_ball(0.397, 10.0);
        let cut = carvable(&flat, 0.05, 8.0, &bit);
        assert!(
            cut.values
                .iter()
                .zip(&flat.values)
                .all(|(c, f)| *c >= f - 1e-6),
            "the bit only leaves material, never cuts lower than asked"
        );
        assert!(cut.at(20, 20) > 0.49, "the pit: {}", cut.at(20, 20));
        assert!((cut.at(10, 10) - 0.9).abs() < 1e-6, "the plateau's top");
        assert!((cut.at(30, 30) - 1.0).abs() < 1e-6, "the peak's top");
        // beside the plateau's wall the bit's side holds it off the floor
        assert!(cut.at(16, 10) > 0.6, "{}", cut.at(16, 10));
        assert!((cut.at(39, 0) - 0.5).abs() < 1e-6, "open floor is reached");
        // the same heights depth2stl's simulation gives for this map
        for ((column, row), want) in [
            ((20, 20), 0.665_735),
            ((16, 10), 0.829_109),
            ((0, 0), 0.767_439),
        ] {
            let got = cut.at(column, row);
            assert!((got - want).abs() < 1e-4, "({column}, {row}): {got}");
        }
    }

    #[test]
    fn a_pit_wider_than_the_bit_is_cut_to_its_floor() {
        let mut values = vec![0.5; 1600];
        for row in 10..30 {
            for column in 10..30 {
                values[row * 40 + column] = 0.0;
            }
        }
        let wide = Grid::new(40, 40, values);
        let straight = tapered_ball(0.397, 0.0);
        assert!(carvable(&wide, 0.05, 8.0, &straight).at(20, 20) < 1e-6);
        // no bit, no relief: nothing to do
        let none = tapered_ball(0.0, 10.0);
        assert_eq!(carvable(&wide, 0.05, 8.0, &none), wide);
    }
}
