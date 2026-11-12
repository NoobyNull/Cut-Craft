//! The pieces raised into heights: each rounded over from its outline to its own height,
//! the higher piece winning where two meet, lines carved in as V grooves.

use crate::field::{edt2, fill_rings};
use crate::motif::{Item, Profile};
use cut_craft_core::geometry::Point;

/// A grid of heights: `columns` by `rows` cells of `cell` mm from `origin` (in the pieces'
/// frame), NaN outside the shape.
pub struct Grid {
    pub origin: Point,
    pub cell: f64,
    pub columns: usize,
    pub rows: usize,
    pub heights: Vec<f64>,
}

impl Grid {
    fn centre(&self, i: usize, j: usize) -> Point {
        let (x, y) = (i as f64 + 0.5, j as f64 + 0.5);
        self.origin + Point::new(x, y) * self.cell
    }
}

/// The box of cells (columns `i0..i1`, rows `j0..j1`) round `pts` grown by `pad` cells,
/// clipped to the grid; `None` when off it.
fn cells_round(
    grid: &Grid,
    pts: impl Iterator<Item = Point>,
    pad: f64,
) -> Option<(usize, usize, usize, usize)> {
    let (mut lo, mut hi) = (Point::splat(f64::INFINITY), Point::splat(f64::NEG_INFINITY));
    for p in pts {
        lo = lo.min(p);
        hi = hi.max(p);
    }
    let a = ((lo - grid.origin) / grid.cell - pad)
        .floor()
        .max(Point::ZERO);
    let b = ((hi - grid.origin) / grid.cell + pad).ceil();
    let (i0, j0) = (a.x as usize, a.y as usize);
    let (i1, j1) = (
        (b.x.max(0.0) as usize).min(grid.columns),
        (b.y.max(0.0) as usize).min(grid.rows),
    );
    (i1 > i0 + 1 && j1 > j0 + 1).then_some((i0, j0, i1, j1))
}

/// Raises `items` on `grid` (whose heights start at 0 inside, NaN outside), the tallest
/// piece standing `up` mm; the pieces marked in `sunk` are hollowed in instead, the deepest
/// going `down` mm.
pub fn raise(grid: &mut Grid, items: &[Item], sunk: &[bool], up: f64, down: f64) {
    let tallest = |hollow: bool| {
        items
            .iter()
            .zip(sunk)
            .filter_map(|(it, &s)| match it {
                Item::Shape { h, sink: false, .. } if s == hollow => Some(*h),
                _ => None,
            })
            .fold(0.0, f64::max)
    };
    let (high, deep) = (tallest(false), tallest(true));
    let rise = if high > 0.0 { up / high } else { 0.0 };
    let fall = if deep > 0.0 { down / deep } else { 0.0 };
    if rise <= 0.0 && fall <= 0.0 {
        return;
    }
    // the ground's stipple goes in as deep as the pieces' scale says
    let punch = if rise > 0.0 { rise } else { fall };
    let cell = grid.cell;
    for (it, &hollow) in items.iter().zip(sunk) {
        match it {
            Item::Shape {
                rings,
                h,
                profile,
                sink,
            } => {
                let Some((i0, j0, i1, j1)) = cells_round(grid, it.points(), 2.0) else {
                    continue;
                };
                let (bw, bh) = (i1 - i0, j1 - j0);
                // ring points in this box's cells, a cell's centre at a whole number
                let base = grid.origin + Point::new(i0 as f64 + 0.5, j0 as f64 + 0.5) * cell;
                let local: Vec<Vec<Point>> = rings
                    .iter()
                    .map(|r| r.iter().map(|&p| (p - base) / cell).collect())
                    .collect();
                let mut f = vec![0.0; bw * bh];
                fill_rings(&mut f, bw, bh, 1.0, &local, 1e20);
                if !f.iter().any(|&v| v > 0.0) {
                    continue;
                }
                edt2(&mut f, bw, bh);
                let mut most = 0.0_f64;
                for v in &mut f {
                    if *v > 0.0 {
                        *v = v.sqrt();
                        most = most.max(*v);
                    }
                }
                let top = h * if *sink {
                    punch
                } else if hollow {
                    fall
                } else {
                    rise
                };
                for j in 0..bh {
                    for i in 0..bw {
                        let d = f[j * bw + i];
                        if d <= 0.0 {
                            continue;
                        }
                        let v = match profile {
                            Profile::Pillow => {
                                let u = (d / most).min(1.0);
                                top * (1.0 - (1.0 - u) * (1.0 - u)).sqrt()
                            }
                            Profile::Round(r) => {
                                let u = (d / (r / cell).max(1.0)).min(1.0);
                                top * (1.0 - (1.0 - u) * (1.0 - u)).sqrt()
                            }
                            Profile::Ball => {
                                let rho = (most - d).max(0.0) / most;
                                top * (1.0 - rho * rho).max(0.0).sqrt()
                            }
                        };
                        let o = (j0 + j) * grid.columns + i0 + i;
                        let cur = grid.heights[o];
                        if cur.is_nan() {
                            continue;
                        }
                        grid.heights[o] = if *sink || hollow {
                            cur.min(-v)
                        } else {
                            cur.max(v)
                        };
                    }
                }
            }
            Item::Line { points, width } => {
                let hw = width / 2.0 / cell;
                let Some((i0, j0, i1, j1)) = cells_round(grid, points.iter().copied(), hw + 2.0)
                else {
                    continue;
                };
                let (bw, bh) = (i1 - i0, j1 - j0);
                let base = grid.origin + Point::new(i0 as f64 + 0.5, j0 as f64 + 0.5) * cell;
                let mut f = vec![1e20; bw * bh];
                for w in points.windows(2) {
                    let (a, b) = ((w[0] - base) / cell, (w[1] - base) / cell);
                    let n = (a.distance(b) * 2.0).ceil().max(1.0) as usize;
                    for t in 0..=n {
                        let q = a.lerp(b, t as f64 / n as f64).round();
                        if q.x >= 0.0 && q.y >= 0.0 && q.x < bw as f64 && q.y < bh as f64 {
                            let k = q.y as usize * bw + q.x as usize;
                            f[k] = 0.0;
                        }
                    }
                }
                edt2(&mut f, bw, bh);
                let depth = width / 2.0 * 3.0_f64.sqrt();
                for j in 0..bh {
                    for i in 0..bw {
                        let d = f[j * bw + i].sqrt();
                        if d >= hw {
                            continue;
                        }
                        let o = (j0 + j) * grid.columns + i0 + i;
                        if !grid.heights[o].is_nan() {
                            grid.heights[o] -= depth * (1.0 - d / hw);
                        }
                    }
                }
            }
        }
    }
}

/// A grid over `columns` by `rows` cells from `origin`, 0 where `inside`, NaN elsewhere.
pub fn grid(
    origin: Point,
    cell: f64,
    columns: usize,
    rows: usize,
    inside: impl Fn(Point) -> bool,
) -> Grid {
    let mut g = Grid {
        origin,
        cell,
        columns,
        rows,
        heights: vec![],
    };
    g.heights = (0..rows)
        .flat_map(|j| (0..columns).map(move |i| (i, j)))
        .map(|(i, j)| {
            if inside(g.centre(i, j)) {
                0.0
            } else {
                f64::NAN
            }
        })
        .collect();
    g
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::motif::{circ, sh};

    #[test]
    fn a_bead_rises_to_the_height_in_its_middle_and_falls_to_its_edge() {
        let mut g = grid(Point::ZERO, 0.25, 80, 80, |_| true);
        raise(
            &mut g,
            &[sh(circ(Point::new(10.0, 10.0), 5.0), 1.0)],
            &[false],
            4.0,
            2.0,
        );
        let at = |x: f64, y: f64| g.heights[(y / 0.25) as usize * 80 + (x / 0.25) as usize];
        assert!((at(10.0, 10.0) - 4.0).abs() < 0.2, "{}", at(10.0, 10.0));
        assert!(at(14.9, 10.0) < 2.0);
        assert_eq!(at(1.0, 1.0), 0.0);
    }

    #[test]
    fn a_sunk_bead_is_hollowed_to_the_depth_beside_a_raised_one() {
        let mut g = grid(Point::ZERO, 0.25, 160, 80, |_| true);
        let beads = [
            sh(circ(Point::new(10.0, 10.0), 5.0), 1.0),
            sh(circ(Point::new(30.0, 10.0), 5.0), 1.0),
        ];
        raise(&mut g, &beads, &[false, true], 4.0, 1.5);
        let at = |x: f64, y: f64| g.heights[(y / 0.25) as usize * 160 + (x / 0.25) as usize];
        assert!((at(10.0, 10.0) - 4.0).abs() < 0.2, "{}", at(10.0, 10.0));
        assert!((at(30.0, 10.0) + 1.5).abs() < 0.1, "{}", at(30.0, 10.0));
        assert!(at(34.9, 10.0) > -0.8 && at(34.9, 10.0) < 0.0);
        assert_eq!(at(20.0, 10.0), 0.0, "the ground between stays put");
    }

    #[test]
    fn outside_the_shape_stays_empty() {
        let mut g = grid(Point::ZERO, 0.5, 40, 40, |p| p.x < 10.0);
        raise(
            &mut g,
            &[sh(circ(Point::new(10.0, 10.0), 5.0), 1.0)],
            &[false],
            4.0,
            2.0,
        );
        assert!(g.heights[20 * 40 + 30].is_nan());
        assert!(g.heights[20 * 40 + 19] > 0.0);
    }
}
