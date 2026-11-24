//! A rotary job's material drawn round its axis: where the flat material's points go, and
//! how far out a round, square or board blank reaches.

use cut_craft_core::glam::{DVec2, DVec3};

/// A rotary job's material drawn round its axis instead of flat (View ▸ Auto-Wrapping View).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wrap {
    /// The axis runs along the material's x, else its y; the other way goes round.
    pub along_x: bool,
    /// The material's radius: the round its faces touch, for a square or a board.
    pub radius: f64,
    /// The height in the material that is its outer surface.
    pub surface: f64,
    /// Drawing the material itself, a square or a board: its half width across and half
    /// thickness up at A=0. Where nothing has cut it, it reaches out to these flat faces
    /// and corners, not the round. `None` for round material, or a model drawn on its own.
    pub outline: Option<DVec2>,
}

/// How far a cell under the surface can sit and still count as uncut: the surface's height
/// as the card's single precision holds it.
const UNCUT: f64 = 1e-3;

impl Wrap {
    /// A point of the flat material (x, y from its corner, z a height) on the cylinder whose
    /// circumference is `round`: the axis along x (or y) through the origin, angle 0 up.
    pub fn place(self, p: DVec3, round: f64) -> DVec3 {
        let (along, across) = if self.along_x { (p.x, p.y) } else { (p.y, p.x) };
        let angle = across / round * std::f64::consts::TAU;
        let out = self.out(p.z, angle);
        let (side, up) = (out * angle.sin(), out * angle.cos());
        if self.along_x {
            DVec3::new(along, side, up)
        } else {
            DVec3::new(side, along, up)
        }
    }

    /// How far from the axis a height `z` sits, `angle` round from the top: below the
    /// surface, cut down from the round; at it, uncut, out to the material's own outline.
    fn out(self, z: f64, angle: f64) -> f64 {
        let rounded = (self.radius + z - self.surface).max(0.0);
        if z < self.surface - UNCUT {
            rounded
        } else {
            rounded.max(self.reach(angle))
        }
    }

    /// How far out the uncut material reaches, `angle` round from the top: the radius for
    /// round material, else to the square's or board's faces.
    fn reach(self, angle: f64) -> f64 {
        self.outline.map_or(self.radius, |half| {
            (half.y / angle.cos().abs()).min(half.x / angle.sin().abs())
        })
    }

    /// How far the material reaches out from the axis as it turns: to the corners of a
    /// square or a board.
    pub fn swing(self) -> f64 {
        self.outline.map_or(self.radius, DVec2::length)
    }
}

/// A square or board rotary job's half width across and half thickness up at A=0, as
/// [`Wrap::outline`] takes them; `None` for round material.
pub fn outline_of(rotary: &cut_craft_core::document::Rotary) -> Option<DVec2> {
    use cut_craft_core::document::Blank;
    let half = rotary.diameter / 2.0;
    match rotary.blank {
        Blank::Round => None,
        Blank::Square => Some(DVec2::splat(half)),
        Blank::Board { width } => Some(DVec2::new(width / 2.0, half)),
    }
}

#[cfg(test)]
mod tests {
    use super::super::{
        Framing, LIGHT, Look, Orbit, cylinder_edges, extent, mesh, stock_edges, wrapped_quads,
    };
    use super::*;
    use cut_craft_core::document::{Along, Job};
    use cut_craft_core::simulation::Material;
    use eframe::egui::pos2;
    use eframe::egui::{Color32, Rect};

    fn wrap() -> Wrap {
        Wrap {
            along_x: true,
            radius: 10.0,
            surface: 0.0,
            outline: None,
        }
    }

    /// A 20 mm square (or, with `width`, a board 20 mm thick) drawn as uncut material.
    fn squared(width: f64) -> Wrap {
        Wrap {
            outline: Some(DVec2::new(width / 2.0, 10.0)),
            ..wrap()
        }
    }

    #[test]
    fn uncut_square_material_reaches_out_to_its_corners() {
        let round = 20.0 * std::f64::consts::PI;
        let corner = squared(20.0).place(DVec3::new(5.0, round / 8.0, 0.0), round);
        let half_diagonal = 10.0 * std::f64::consts::SQRT_2;
        assert!(
            (corner.y.hypot(corner.z) - half_diagonal).abs() < 1e-9,
            "{corner}"
        );
        let face = squared(20.0).place(DVec3::new(5.0, round / 4.0, 0.0), round);
        assert!(face.abs_diff_eq(DVec3::new(5.0, 10.0, 0.0), 1e-9), "{face}");
    }

    #[test]
    fn a_cut_in_square_material_goes_down_from_the_round_its_faces_touch() {
        let round = 20.0 * std::f64::consts::PI;
        // 2 mm deep, where a corner was: cut down to 8 mm from the axis, the corner gone
        let cut = squared(20.0).place(DVec3::new(5.0, round / 8.0, -2.0), round);
        assert!((cut.y.hypot(cut.z) - 8.0).abs() < 1e-9, "{cut}");
    }

    #[test]
    fn an_uncut_board_lies_flat_with_its_edges_out_to_the_sides() {
        let round = 20.0 * std::f64::consts::PI;
        let board = squared(60.0);
        let top = board.place(DVec3::new(5.0, 0.0, 0.0), round);
        assert!(top.abs_diff_eq(DVec3::new(5.0, 0.0, 10.0), 1e-9), "{top}");
        let edge = board.place(DVec3::new(5.0, round / 4.0, 0.0), round);
        assert!(edge.abs_diff_eq(DVec3::new(5.0, 30.0, 0.0), 1e-9), "{edge}");
        assert!((board.swing() - 30f64.hypot(10.0)).abs() < 1e-9);
        assert!((wrap().swing() - 10.0).abs() < 1e-12);
    }

    #[test]
    fn a_square_or_board_job_has_its_outline_and_a_round_one_none() {
        use cut_craft_core::document::{Blank, Rotary};
        let rotary = |blank| Rotary {
            blank,
            ..Rotary::round(20.0, Along::X)
        };
        assert_eq!(outline_of(&rotary(Blank::Round)), None);
        assert_eq!(
            outline_of(&rotary(Blank::Square)),
            Some(DVec2::new(10.0, 10.0))
        );
        assert_eq!(
            outline_of(&rotary(Blank::Board { width: 60.0 })),
            Some(DVec2::new(30.0, 10.0))
        );
    }

    #[test]
    fn square_material_s_wireframe_is_a_box_round_the_axis() {
        let edges = stock_edges(true, 10.0, Some(DVec2::new(30.0, 10.0)), 0.0, 100.0);
        assert_eq!(edges.len(), 12);
        let far = edges
            .iter()
            .flatten()
            .fold(DVec3::ZERO, |m, p| m.max(p.abs()));
        assert!(
            far.abs_diff_eq(DVec3::new(100.0, 30.0, 10.0), 1e-9),
            "{far}"
        );
        let round = stock_edges(true, 10.0, None, 0.0, 100.0);
        assert_eq!(round, cylinder_edges(true, 10.0, 0.0, 100.0));
    }

    #[test]
    fn the_view_fits_round_a_board_s_edges() {
        let job = Job::rotary(60.0, 20.0, Along::X);
        let m = Material::new(&job, 60);
        let (lo, hi) = extent(&m, Some(squared(60.0)));
        let swing = 30f64.hypot(10.0);
        assert!((hi.y - swing).abs() < 1e-9 && (lo.z + swing).abs() < 1e-9);
    }

    #[test]
    fn the_surface_goes_round_the_axis_from_the_top() {
        let round = 20.0 * std::f64::consts::PI;
        let top = wrap().place(DVec3::new(5.0, 0.0, 0.0), round);
        assert!(top.abs_diff_eq(DVec3::new(5.0, 0.0, 10.0), 1e-9));
        let quarter = wrap().place(DVec3::new(5.0, round / 4.0, -2.0), round);
        assert!(
            quarter.abs_diff_eq(DVec3::new(5.0, 8.0, 0.0), 1e-9),
            "{quarter}"
        );
    }

    #[test]
    fn a_rotary_material_is_drawn_closed_round_and_inside_the_view() {
        let job = Job::rotary(60.0, 20.0, Along::X);
        let m = Material::new(&job, 60);
        let area = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 300.0));
        let look = Look {
            colour: Color32::WHITE,
            sides: true,
            light: LIGHT,
            wrap: Some(wrap()),
            grain: None,
            shadows: false,
        };
        let quads = wrapped_quads(&m, wrap(), LIGHT);
        assert_eq!(
            quads.len(),
            (m.columns - 1) * m.rows,
            "every row, the seam too"
        );
        let drawn = mesh(&m, Orbit::default(), Framing::default(), &look, area);
        assert_eq!(drawn.vertices.len(), 4 * quads.len());
        assert!(
            drawn
                .vertices
                .iter()
                .all(|v| area.expand(1.0).contains(v.pos))
        );
    }
}
