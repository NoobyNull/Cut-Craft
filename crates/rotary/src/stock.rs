//! The material as it is held on the turning axis.

use crate::Cutter;
use cut_craft_core::document::{Along, Blank, Job};
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

    /// The material a rotary `job` is set up with (Job Setup's diameter, shape and length),
    /// held on its centre. `None` for a flat job.
    #[must_use]
    pub fn of(job: &Job) -> Option<Self> {
        let rotary = job.rotary?;
        let d = rotary.diameter;
        let section = match rotary.blank {
            Blank::Round => Section::Round { diameter: d },
            Blank::Square => Section::Square { side: d },
            Blank::Board { width } => Section::Board {
                width,
                thickness: d,
            },
        };
        let length = match rotary.along {
            Along::X => job.width,
            Along::Y => job.height,
        };
        Some(Self::new(section, length))
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

    /// The highest the tip of `cutter` can sit, straight down from above, held at `along`
    /// and `across` with A turned `turn`, and still touch the material: its height above
    /// the axis. `None` when the bit is clear of the material wherever it goes down. A bit
    /// held higher than this cuts only air.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "a fixed, small number of samples across the bit"
    )]
    pub fn drop(&self, cutter: &Cutter, along: f64, across: f64, turn: f64) -> Option<f64> {
        /// Samples across the bit, besides the section's own corners.
        const ACROSS: usize = 64;
        let reach = cutter.reach();
        // how far past the material's ends the bit's axis is
        let past = (-along).max(along - self.length).max(0.0);
        if past > reach {
            return None;
        }
        let (sin, cos) = turn.sin_cos();
        let machine = |q: DVec2| DVec2::new(q.x * cos - q.y * sin, q.y * cos + q.x * sin);
        // the section's top at `x` across, in the machine's frame
        let corners: Vec<DVec2> = match self.section {
            Section::Round { .. } => vec![],
            Section::Square { .. } | Section::Board { .. } => {
                let h = self.half();
                [(1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)]
                    .into_iter()
                    .map(|(a, b)| machine(DVec2::new(a * h.x, b * h.y) + self.offset))
                    .collect()
            }
        };
        let top = |x: f64| -> Option<f64> {
            if let Section::Round { diameter } = self.section {
                let middle = machine(self.offset);
                let r = diameter / 2.0;
                let off = x - middle.x;
                return (off.abs() <= r).then(|| middle.y + (r * r - off * off).max(0.0).sqrt());
            }
            let mut best: Option<f64> = None;
            for i in 0..4 {
                let (one, two) = (corners[i], corners[(i + 1) % 4]);
                let (lo, hi) = (one.x.min(two.x), one.x.max(two.x));
                if x < lo - 1e-12 || x > hi + 1e-12 {
                    continue;
                }
                let z = if hi - lo < 1e-12 {
                    one.y.max(two.y)
                } else {
                    one.y + (two.y - one.y) * ((x - one.x) / (two.x - one.x)).clamp(0.0, 1.0)
                };
                best = Some(best.map_or(z, |m: f64| m.max(z)));
            }
            best
        };
        let mut best: Option<f64> = None;
        let mut at = |x: f64| {
            let r = past.hypot(x - across);
            if let (Some(z), Some(edge)) = (top(x), cutter.edge(r)) {
                let tip = z - edge;
                best = Some(best.map_or(tip, |m: f64| m.max(tip)));
            }
        };
        for k in 0..=ACROSS {
            at(across - reach + 2.0 * reach * k as f64 / ACROSS as f64);
        }
        for corner in &corners {
            if (corner.x - across).abs() <= reach {
                at(corner.x);
            }
        }
        best
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
    fn a_rotary_job_is_set_up_with_the_material_job_setup_says() {
        use cut_craft_core::document::Rotary;
        let mut job = Job::rotary(300.0, 40.0, Along::X);
        assert_eq!(
            Stock::of(&job),
            Some(Stock::new(Section::Round { diameter: 40.0 }, 300.0))
        );
        job.rotary = Some(Rotary {
            blank: Blank::Board { width: 150.0 },
            ..job.rotary.unwrap()
        });
        assert_eq!(
            Stock::of(&job).map(|s| s.section),
            Some(Section::Board {
                width: 150.0,
                thickness: 40.0
            })
        );
        let along_y = Job::rotary(250.0, 30.0, Along::Y);
        assert_eq!(Stock::of(&along_y).map(|s| s.length), Some(250.0));
        assert_eq!(Stock::of(&Job::default()), None);
    }

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

    fn bit(diameter: f64, shape: cut_craft_core::toolpath::ToolShape) -> Cutter {
        Cutter::new(&cut_craft_core::toolpath::Tool {
            diameter,
            shape,
            ..cut_craft_core::toolpath::Tool::default()
        })
    }

    #[test]
    fn a_bit_comes_down_onto_the_material_as_it_is_turned() {
        use cut_craft_core::toolpath::ToolShape::{BallNose, EndMill};
        use std::f64::consts::FRAC_PI_4;
        let ball = bit(6.0, BallNose);
        let flat = bit(6.0, EndMill);
        let round = Stock::new(Section::Round { diameter: 20.0 }, 100.0);
        let near = |a: Option<f64>, b: f64| a.is_some_and(|a| (a - b).abs() < 0.01);
        // straight over the axis it rests on the top, whatever the turn
        assert!(near(round.drop(&ball, 50.0, 0.0, 0.0), 10.0));
        assert!(near(round.drop(&ball, 50.0, 0.0, 1.0), 10.0));
        // to one side the ball's centre stays its radius from the circle
        assert!(near(
            round.drop(&ball, 50.0, 8.0, 0.0),
            (13.0f64.powi(2) - 64.0).sqrt() - 3.0
        ));
        // past the side, and past the ends, it never touches
        assert_eq!(round.drop(&ball, 50.0, 13.5, 0.0), None);
        assert_eq!(round.drop(&ball, -3.5, 0.0, 0.0), None);
        assert!(round.drop(&ball, -2.0, 0.0, 0.0).is_some_and(|z| z < 10.0));
        // a board lies flat at A=0 and stands up at a quarter turn
        let board = Stock::new(
            Section::Board {
                width: 40.0,
                thickness: 10.0,
            },
            100.0,
        );
        assert!(near(board.drop(&flat, 50.0, 18.0, 0.0), 5.0));
        assert!(near(
            board.drop(&flat, 50.0, 0.0, std::f64::consts::FRAC_PI_2),
            20.0
        ));
        assert_eq!(
            board.drop(&flat, 50.0, 30.0, std::f64::consts::FRAC_PI_2),
            None
        );
        // a square turned an eighth: a flat bit sits on its corner
        let square = Stock::new(Section::Square { side: 20.0 }, 100.0);
        assert!(near(
            square.drop(&flat, 50.0, 2.0, FRAC_PI_4),
            10.0 * 2f64.sqrt()
        ));
    }

    #[test]
    fn the_drop_on_the_material_agrees_with_looking_at_every_point_of_it() {
        use cut_craft_core::toolpath::ToolShape::{BallNose, EndMill, VBit};
        let stocks = [
            Stock::new(Section::Round { diameter: 30.0 }, 60.0),
            Stock::new(Section::Square { side: 24.0 }, 60.0),
            Stock {
                offset: DVec2::new(4.0, -3.0),
                ..Stock::new(
                    Section::Board {
                        width: 50.0,
                        thickness: 12.0,
                    },
                    60.0,
                )
            },
        ];
        let bits = [bit(6.0, BallNose), bit(8.0, EndMill), {
            let mut v = cut_craft_core::toolpath::Tool {
                diameter: 10.0,
                shape: VBit,
                ..cut_craft_core::toolpath::Tool::default()
            };
            v.angle = 90.0;
            Cutter::new(&v)
        }];
        for stock in &stocks {
            for cutter in &bits {
                for i in 0..40 {
                    let turn = f64::from(i) * 0.37;
                    let across = f64::from(i % 13) * 3.1 - 20.0;
                    let along: f64 = if i % 5 == 0 { -1.5 } else { 30.0 };
                    // every point of the section's edge, as finely as it matters
                    let mut brute: Option<f64> = None;
                    let (lo, hi) = stock.bounds();
                    let n = 600;
                    for a in 0..=n {
                        for b in 0..=n {
                            let (fa, fb) =
                                (f64::from(a) / f64::from(n), f64::from(b) / f64::from(n));
                            let p = DVec3::new(
                                along.clamp(0.0, stock.length),
                                lo.y + (hi.y - lo.y) * fa,
                                lo.z + (hi.z - lo.z) * fb,
                            );
                            if !stock.holds(p) {
                                continue;
                            }
                            let m = crate::to_machine(p, turn);
                            let r = (m.x - along).hypot(m.y - across);
                            if let Some(edge) = cutter.edge(r) {
                                let tip = m.z - edge;
                                brute = Some(brute.map_or(tip, |x: f64| x.max(tip)));
                            }
                        }
                    }
                    let got = stock.drop(cutter, along, across, turn);
                    match (got, brute) {
                        (Some(g), Some(b)) => assert!(
                            (g - b).abs() < 0.15,
                            "{stock:?} {:?} turn {turn} across {across}: {g} vs {b}",
                            cutter.tool().shape
                        ),
                        (None, None) => {}
                        // only where the bit just grazes it, which the grid misses
                        (Some(_), None)
                            if stock
                                .drop(
                                    cutter,
                                    along - 0.2 * f64::from(u8::from(along < 0.0)),
                                    across + 0.2 * across.signum(),
                                    turn,
                                )
                                .is_none() => {}
                        (g, b) => panic!("{stock:?} turn {turn} across {across}: {g:?} vs {b:?}"),
                    }
                }
            }
        }
    }
}
