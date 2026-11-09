//! A [`Shape`] as a closed triangle mesh: sampled on a grid, each square cut to the
//! outline (where `inside` crosses 0, the height there being 0), the top at the shape's
//! height and the bottom either flat (a half, to glue to another) or its mirror (the whole
//! piece).

use crate::shape::{Sample, Shape};
use glam::DVec3;

/// What is made of a shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cut {
    /// The top half, flat underneath: two glued face to face make the piece.
    Half,
    /// The whole piece, both halves: cut from both sides of the wood.
    Whole,
}

/// `shape` as triangles, sampled every `step` mm, lying on its side: its foot at `x = 0`,
/// its middle at `z = 0`.
pub fn mesh(shape: &Shape, cut: Cut, step: f64) -> Vec<[DVec3; 3]> {
    let (length, half_width) = shape.extent();
    let pad = 2.0 * step;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a piece is a few hundred steps long"
    )]
    let count = |span: f64| ((span + 2.0 * pad) / step).ceil() as usize + 1;
    let (nx, ny) = (count(length), count(2.0 * half_width));
    let at = |i: usize, j: usize| {
        #[expect(clippy::cast_precision_loss, reason = "small counts")]
        let (x, y) = (-pad + i as f64 * step, -half_width - pad + j as f64 * step);
        (x, y)
    };
    let grid: Vec<Sample> = (0..nx * ny)
        .map(|k| {
            let (x, y) = at(k % nx, k / nx);
            shape.sample(x, y)
        })
        .collect();
    let mut triangles = Vec::new();
    for j in 0..ny - 1 {
        for i in 0..nx - 1 {
            let corners = [(i, j), (i + 1, j), (i + 1, j + 1), (i, j + 1)];
            let samples = corners.map(|(a, b)| grid[b * nx + a]);
            if samples.iter().all(|s| s.inside <= 0.0) {
                continue;
            }
            // the square cut to the outline, anticlockwise from above
            let mut polygon: Vec<(f64, f64, f64)> = Vec::with_capacity(6);
            for k in 0..4 {
                let (a, b) = (samples[k], samples[(k + 1) % 4]);
                let (pa, pb) = (
                    at(corners[k].0, corners[k].1),
                    at(corners[(k + 1) % 4].0, corners[(k + 1) % 4].1),
                );
                if a.inside > 0.0 {
                    polygon.push((pa.0, pa.1, a.height));
                }
                if (a.inside > 0.0) != (b.inside > 0.0) {
                    let t = a.inside / (a.inside - b.inside);
                    polygon.push((pa.0 + t * (pb.0 - pa.0), pa.1 + t * (pb.1 - pa.1), 0.0));
                }
            }
            for k in 1..polygon.len() - 1 {
                let tri = [polygon[0], polygon[k], polygon[k + 1]];
                triangles.push(tri.map(|(x, y, z)| DVec3::new(x, y, z)));
                let under = |(x, y, z): (f64, f64, f64)| match cut {
                    Cut::Half => DVec3::new(x, y, 0.0),
                    Cut::Whole => DVec3::new(x, y, 0.0 - z),
                };
                triangles.push([under(tri[0]), under(tri[2]), under(tri[1])]);
            }
        }
    }
    triangles
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::Piece;
    use std::collections::HashMap;

    /// Each edge of a closed mesh is used once each way.
    fn open_edges(triangles: &[[DVec3; 3]]) -> usize {
        let key = |v: DVec3| (v * 1e6).round().to_array().map(|c| (c + 0.0).to_bits());
        let mut edges: HashMap<_, i32> = HashMap::new();
        for t in triangles {
            for k in 0..3 {
                let (a, b) = (key(t[k]), key(t[(k + 1) % 3]));
                *edges.entry((a, b)).or_default() += 1;
                *edges.entry((b, a)).or_default() -= 1;
            }
        }
        edges.values().filter(|&&n| n != 0).count()
    }

    fn volume(triangles: &[[DVec3; 3]]) -> f64 {
        triangles
            .iter()
            .map(|[a, b, c]| a.dot(b.cross(*c)) / 6.0)
            .sum()
    }

    #[test]
    fn every_piece_is_closed_both_ways() {
        for piece in Piece::ALL {
            let shape = Shape::new(piece);
            for cut in [Cut::Half, Cut::Whole] {
                let t = mesh(&shape, cut, 0.5);
                assert!(!t.is_empty());
                assert_eq!(open_edges(&t), 0, "{piece:?} {cut:?}");
                assert!(volume(&t) > 0.0, "{piece:?} {cut:?} faces inwards");
            }
        }
    }

    #[test]
    fn a_whole_piece_is_two_halves() {
        let shape = Shape::new(Piece::Bishop);
        let half = volume(&mesh(&shape, Cut::Half, 0.5));
        let whole = volume(&mesh(&shape, Cut::Whole, 0.5));
        assert!((whole - 2.0 * half).abs() < 1e-6 * whole, "{half} {whole}");
    }

    #[test]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "a few thousand slices"
    )]
    fn a_turned_piece_has_about_the_volume_of_its_profile_turned() {
        let piece = Piece::Pawn;
        let profile = piece.profile();
        let dx = 0.01;
        let turned: f64 = (0..(profile.height() / dx) as usize)
            .map(|i| std::f64::consts::PI * profile.radius_at((i as f64 + 0.5) * dx).powi(2) * dx)
            .sum();
        let meshed = volume(&mesh(&Shape::new(piece), Cut::Whole, 0.3));
        assert!(
            (meshed - turned).abs() < 0.03 * turned,
            "{meshed} vs {turned}"
        );
    }

    #[test]
    fn a_half_lies_flat_and_fits_its_wood() {
        for piece in Piece::ALL {
            let t = mesh(&Shape::new(piece), Cut::Half, 0.5);
            let (low, high) = t
                .iter()
                .flatten()
                .fold((f64::MAX, f64::MIN), |(l, h), v| (l.min(v.z), h.max(v.z)));
            assert_eq!(low, 0.0);
            assert!(high <= piece.size().0 + 1e-9, "{piece:?} {high}");
        }
    }
}
