//! Shaping an estimated depth into relief heights: the photo's backdrop cut away, the
//! subject lifted clear of it, fine detail from the photo's own shading added back (a depth
//! network smooths away hair, scales and faces) and the noise smoothed while keeping edges.
//! Every grid is row by row from the top, `columns` wide.

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
    let backdrop = backdrop_tone(light);
    let differs: Vec<bool> = light
        .values
        .iter()
        .map(|v| (v - backdrop).abs() > cut_off)
        .collect();
    // the largest patch, by flood fill
    let (columns, rows) = (light.columns, light.rows);
    let mut patch = vec![usize::MAX; differs.len()];
    let (mut best, mut best_size) = (usize::MAX, 0);
    for start in 0..differs.len() {
        if !differs[start] || patch[start] != usize::MAX {
            continue;
        }
        let mut size = 0;
        let mut stack = vec![start];
        patch[start] = start;
        while let Some(at) = stack.pop() {
            size += 1;
            let (c, r) = (at % columns, at / columns);
            let mut visit = |next: usize| {
                if differs[next] && patch[next] == usize::MAX {
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
#[derive(Clone, Copy, Debug)]
pub struct Shaping {
    /// See [`subject`].
    pub cut_off: f32,
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
    let mut fig = subject(light, shaping.cut_off);
    if !fig.contains(&true) {
        fig = vec![true; fig.len()];
    }
    let mut inside: Vec<f32> = depth
        .values
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
        .values
        .iter()
        .map(|d| ((d - lo) / span).clamp(0.0, 1.0))
        .collect();
    if shaping.detail > 0.0 {
        let detail = shading_detail(light, &fig, shaping.radius);
        for (h, d) in h.iter_mut().zip(&detail.values) {
            *h = (*h + shaping.detail * 0.5 * d).clamp(0.0, 1.0);
        }
    }
    let lift = shaping.lift.clamp(0.0, 1.0);
    for (h, &f) in h.iter_mut().zip(&fig) {
        *h = if f { lift + (1.0 - lift) * *h } else { 0.0 };
    }
    let mut grid = Grid::new(depth.columns, depth.rows, h);
    for _ in 0..shaping.smoothing {
        grid = diffuse(&grid, EDGE);
    }
    for (h, &f) in grid.values.iter_mut().zip(&fig) {
        if !f {
            *h = 0.0;
        }
    }
    grid
}

#[cfg(test)]
mod tests {
    use super::*;

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
            detail: 0.0,
            radius: 3.0,
            smoothing: 0,
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
}
