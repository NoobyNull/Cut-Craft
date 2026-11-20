//! Shared makings for the rotary tests: bits, models and a repeatable shuffle.
#![allow(dead_code, reason = "each test file uses some of these")]

use cut_craft_core::glam::DVec3;
use cut_craft_core::toolpath::{Tool, ToolShape};
use cut_craft_rotary::Cutter;

/// A repeatable stream of numbers in 0..1, the same on every run and machine.
pub struct Shuffle(u64);

impl Shuffle {
    pub fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1))
    }

    #[expect(clippy::cast_precision_loss, reason = "53 bits of a random word")]
    pub fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn within(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next()
    }
}

pub fn ball(diameter: f64) -> Cutter {
    Cutter::new(&Tool {
        shape: ToolShape::BallNose,
        diameter,
        flute_length: 30.0,
        ..Tool::default()
    })
}

pub fn flat(diameter: f64) -> Cutter {
    Cutter::new(&Tool {
        shape: ToolShape::EndMill,
        diameter,
        flute_length: 30.0,
        ..Tool::default()
    })
}

pub fn vee(diameter: f64, angle: f64) -> Cutter {
    Cutter::new(&Tool {
        shape: ToolShape::VBit,
        diameter,
        angle,
        flute_length: 30.0,
        ..Tool::default()
    })
}

pub fn bull(diameter: f64, tip: f64) -> Cutter {
    Cutter::new(&Tool {
        shape: ToolShape::RadiusedEndMill,
        diameter,
        tip_radius: tip,
        flute_length: 30.0,
        ..Tool::default()
    })
}

/// Every kind of bit the rotary toolpaths use.
pub fn bits() -> Vec<Cutter> {
    vec![
        ball(6.0),
        flat(4.0),
        vee(8.0, 60.0),
        bull(6.0, 1.0),
        ball(1.5),
    ]
}

/// A surface of revolution round the axis: `radius(along)` from `from` to `to`, in
/// `rings` x `round` quads, with flat ends: a closed solid.
#[expect(clippy::cast_precision_loss, reason = "small counts")]
pub fn turned(
    radius: impl Fn(f64) -> f64,
    from: f64,
    to: f64,
    rings: usize,
    round: usize,
) -> Vec<[DVec3; 3]> {
    let at = |i: usize, j: usize| {
        let x = from + (to - from) * i as f64 / rings as f64;
        let phi = std::f64::consts::TAU * j as f64 / round as f64;
        let r = radius(x);
        DVec3::new(x, r * phi.sin(), r * phi.cos())
    };
    let mut tris = vec![];
    for i in 0..rings {
        for j in 0..round {
            let corners = [at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1)];
            tris.push([corners[0], corners[1], corners[2]]);
            tris.push([corners[0], corners[2], corners[3]]);
        }
    }
    for j in 0..round {
        tris.push([DVec3::new(from, 0.0, 0.0), at(0, j + 1), at(0, j)]);
        tris.push([DVec3::new(to, 0.0, 0.0), at(rings, j), at(rings, j + 1)]);
    }
    tris
}

/// `n` triangles scattered in a box round the axis.
pub fn scattered(n: usize, seed: u64) -> Vec<[DVec3; 3]> {
    let mut s = Shuffle::new(seed);
    (0..n)
        .map(|_| {
            let c = DVec3::new(
                s.within(0.0, 40.0),
                s.within(-15.0, 15.0),
                s.within(-15.0, 15.0),
            );
            [0, 1, 2].map(|_| {
                c + DVec3::new(
                    s.within(-4.0, 4.0),
                    s.within(-4.0, 4.0),
                    s.within(-4.0, 4.0),
                )
            })
        })
        .collect()
}
