//! The material as it is held on the turning axis.

use cut_craft_core::glam::{DVec2, DVec3};

/// The material's shape across the axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Section {
    /// A round blank, `diameter` across.
    Round { diameter: f64 },
    /// A square blank, `side` across each face, its faces square to the part's `y` and `z`.
    Square { side: f64 },
    /// A flat board, `width` across the part's `y` and `thickness` up its `z`: lying flat
    /// at A=0.
    Board { width: f64, thickness: f64 },
}

/// The material: its shape across the axis, how long it is along it (from the job's start,
/// the chuck end), and how far its middle is from the axis (in the part's `y`, `z`), for
/// material not held on its centre.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stock {
    pub section: Section,
    pub length: f64,
    pub offset: DVec2,
}

impl Stock {
    #[must_use]
    pub fn new(section: Section, length: f64) -> Self {
        Self {
            section,
            length,
            offset: DVec2::ZERO,
        }
    }

    /// Whether `p` (in the part's frame) is in the material.
    #[must_use]
    pub fn holds(&self, p: DVec3) -> bool {
        if !(0.0..=self.length).contains(&p.x) {
            return false;
        }
        let q = DVec2::new(p.y, p.z) - self.offset;
        match self.section {
            Section::Round { diameter } => q.length() <= diameter / 2.0,
            Section::Square { side } => q.x.abs() <= side / 2.0 && q.y.abs() <= side / 2.0,
            Section::Board { width, thickness } => {
                q.x.abs() <= width / 2.0 && q.y.abs() <= thickness / 2.0
            }
        }
    }

    /// The corners of the section, half across and half up, round its middle.
    fn half(&self) -> DVec2 {
        match self.section {
            Section::Round { diameter } => DVec2::splat(diameter / 2.0),
            Section::Square { side } => DVec2::splat(side / 2.0),
            Section::Board { width, thickness } => DVec2::new(width / 2.0, thickness / 2.0),
        }
    }

    /// How far out from the axis the material reaches as it turns: what must clear the bed
    /// below and the bit above. A board swings by half its diagonal, not half its
    /// thickness.
    #[must_use]
    pub fn swing(&self) -> f64 {
        match self.section {
            Section::Round { diameter } => self.offset.length() + diameter / 2.0,
            Section::Square { .. } | Section::Board { .. } => {
                let h = self.half();
                [
                    DVec2::new(h.x, h.y),
                    DVec2::new(-h.x, h.y),
                    DVec2::new(h.x, -h.y),
                    DVec2::new(-h.x, -h.y),
                ]
                .into_iter()
                .map(|c| (c + self.offset).length())
                .fold(0.0, f64::max)
            }
        }
    }

    /// The corners of a box round the material, in the part's frame.
    #[must_use]
    pub fn bounds(&self) -> (DVec3, DVec3) {
        let h = self.half();
        (
            DVec3::new(0.0, self.offset.x - h.x, self.offset.y - h.y),
            DVec3::new(self.length, self.offset.x + h.x, self.offset.y + h.y),
        )
    }

    /// How much material there is, mm³.
    #[must_use]
    pub fn volume(&self) -> f64 {
        let area = match self.section {
            Section::Round { diameter } => std::f64::consts::PI * diameter * diameter / 4.0,
            Section::Square { side } => side * side,
            Section::Board { width, thickness } => width * thickness,
        };
        area * self.length
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_board_swings_by_half_its_diagonal_not_half_its_thickness() {
        let board = Stock::new(
            Section::Board {
                width: 150.0,
                thickness: 20.0,
            },
            300.0,
        );
        assert!((board.swing() - 75f64.hypot(10.0)).abs() < 1e-9);
        let round = Stock::new(Section::Round { diameter: 50.0 }, 100.0);
        assert!((round.swing() - 25.0).abs() < 1e-12);
        // held 5 off its middle, it swings 5 further
        let off = Stock {
            offset: DVec2::new(3.0, 4.0),
            ..round
        };
        assert!((off.swing() - 30.0).abs() < 1e-12);
    }

    #[test]
    fn a_board_lies_flat_at_a_zero() {
        let board = Stock::new(
            Section::Board {
                width: 100.0,
                thickness: 20.0,
            },
            50.0,
        );
        assert!(board.holds(DVec3::new(10.0, 45.0, 9.0)));
        assert!(!board.holds(DVec3::new(10.0, 9.0, 45.0)));
        assert!(!board.holds(DVec3::new(51.0, 0.0, 0.0)));
    }
}
