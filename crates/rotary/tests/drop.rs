//! The bit dropped onto the model: exact, never into it, the same at every turn of A.

mod common;

use common::{Shuffle, ball, bits, flat, scattered, turned};
use cut_craft_core::glam::{DVec2, DVec3};
use cut_craft_rotary::{Cutter, Mesh, to_machine};
use std::f64::consts::{PI, TAU};

/// How far the bit standing with its tip at `tip` is above the model at its nearest:
/// negative when it is in it. Every triangle is looked at finely, corners, edges and faces.
#[expect(clippy::cast_precision_loss, reason = "small counts")]
fn clearance(
    mesh: &Mesh,
    cutter: &Cutter,
    (along, across, turn): (f64, f64, f64),
    tip: f64,
    fine: usize,
) -> f64 {
    let centre = DVec2::new(along, across);
    let mut least = f64::INFINITY;
    for tri in mesh.triangles() {
        let tri = tri.map(|p| to_machine(p, turn));
        let near = tri
            .iter()
            .any(|p| DVec2::new(p.x, p.y).distance(centre) < cutter.reach() + 8.0);
        if !near {
            continue;
        }
        for one in 0..=fine {
            for two in 0..=fine - one {
                let (u, v) = (one as f64 / fine as f64, two as f64 / fine as f64);
                let p = tri[0] + (tri[1] - tri[0]) * u + (tri[2] - tri[0]) * v;
                let r = DVec2::new(p.x, p.y).distance(centre);
                if let Some(edge) = cutter.edge(r) {
                    least = least.min(tip + edge - p.z);
                }
            }
        }
    }
    least
}

#[test]
fn the_tree_finds_exactly_what_looking_at_every_triangle_finds() {
    let mesh = Mesh::new(scattered(400, 7));
    let mut s = Shuffle::new(11);
    for cutter in bits() {
        for _ in 0..200 {
            let (along, across, turn) = (
                s.within(-5.0, 45.0),
                s.within(-20.0, 20.0),
                s.within(-TAU, TAU),
            );
            let tree = mesh.drop(&cutter, along, across, turn);
            let every = mesh.drop_every(&cutter, along, across, turn);
            match (tree, every) {
                (Some(a), Some(b)) => assert!((a - b).abs() < 1e-9, "{a} vs {b}"),
                (a, b) => assert_eq!(a, b),
            }
        }
    }
}

#[test]
fn the_bit_never_goes_into_the_model_and_rests_on_it() {
    let mesh = Mesh::new(scattered(60, 3));
    let mut s = Shuffle::new(5);
    for cutter in bits() {
        let mut checked = 0;
        for _ in 0..30 {
            let (along, across, turn) = (
                s.within(0.0, 40.0),
                s.within(-12.0, 12.0),
                s.within(0.0, TAU),
            );
            let Some(tip) = mesh.drop(&cutter, along, across, turn) else {
                continue;
            };
            checked += 1;
            let at = (along, across, turn);
            let gap = clearance(&mesh, &cutter, at, tip, 24);
            assert!(
                gap > -1e-6,
                "{:?} into the model by {gap}",
                cutter.tool().shape
            );
            // resting: within a very fine sample's spacing of touching
            let gap = clearance(&mesh, &cutter, at, tip, 400);
            assert!(gap < 0.03, "{:?} hangs {gap} above", cutter.tool().shape);
        }
        assert!(checked > 10, "{checked}");
    }
}

#[test]
fn on_a_round_part_the_ball_rests_on_it_wherever_a_is_turned() {
    let mesh = Mesh::new(turned(|_| 10.0, 0.0, 40.0, 8, 720));
    let ball = ball(6.0);
    for k in 0..36 {
        let turn = TAU * f64::from(k) / 36.0;
        let tip = mesh.drop(&ball, 20.0, 0.0, turn).unwrap();
        assert!((tip - 10.0).abs() < 1e-3, "{turn}: {tip}");
    }
}

#[test]
fn held_to_one_side_the_ball_rests_on_the_round_part_lower_down() {
    let (rho, rb) = (10.0, 3.0);
    let mesh = Mesh::new(turned(|_| rho, 0.0, 40.0, 8, 1440));
    let ball = ball(2.0 * rb);
    for across in [-8.0, -4.0, 0.0, 2.5, 6.0, 12.0] {
        let tip = mesh.drop(&ball, 20.0, across, 0.3).unwrap();
        // the ball's centre stays rho + rb from the axis
        let want = ((rho + rb) * (rho + rb) - across * across).sqrt() - rb;
        assert!((tip - want).abs() < 2e-3, "{across}: {tip} vs {want}");
    }
}

#[test]
fn a_flat_bottom_rests_on_the_top_of_a_round_part() {
    let mesh = Mesh::new(turned(|_| 10.0, 0.0, 40.0, 8, 1440));
    let tip = mesh.drop(&flat(4.0), 20.0, 0.0, 1.0).unwrap();
    assert!((tip - 10.0).abs() < 1e-3, "{tip}");
    // off to one side its edge catches the part where it is higher
    let tip = mesh.drop(&flat(4.0), 20.0, 5.0, 1.0).unwrap();
    assert!((tip - (100f64 - 9.0).sqrt()).abs() < 1e-3, "{tip}");
}

#[test]
fn a_bump_comes_under_the_bit_when_a_turns_to_it() {
    // a ridge on a round part, a quarter turn round from the top
    let ridge = |x: f64| {
        if (18.0..22.0).contains(&x) {
            12.0
        } else {
            10.0
        }
    };
    let mut tris = turned(|_| 10.0, 0.0, 40.0, 8, 360);
    // a small block standing out at angle 90° (along +y)
    let (lo, hi) = (
        DVec3::new(18.0, 9.0, -1.0),
        DVec3::new(22.0, ridge(20.0), 1.0),
    );
    tris.extend(block(lo, hi));
    let mesh = Mesh::new(tris);
    let ball = ball(2.0);
    let at_top = mesh.drop(&ball, 20.0, 0.0, 0.0).unwrap();
    let turned_to = mesh.drop(&ball, 20.0, 0.0, PI / 2.0).unwrap();
    assert!((at_top - 10.0).abs() < 1e-2, "{at_top}");
    assert!((turned_to - 12.0).abs() < 1e-6, "{turned_to}");
}

/// The twelve triangles of a box.
fn block(lo: DVec3, hi: DVec3) -> Vec<[DVec3; 3]> {
    let c = |x: bool, y: bool, z: bool| {
        DVec3::new(
            if x { hi.x } else { lo.x },
            if y { hi.y } else { lo.y },
            if z { hi.z } else { lo.z },
        )
    };
    let quads = [
        [
            c(false, false, false),
            c(false, true, false),
            c(true, true, false),
            c(true, false, false),
        ],
        [
            c(false, false, true),
            c(true, false, true),
            c(true, true, true),
            c(false, true, true),
        ],
        [
            c(false, false, false),
            c(true, false, false),
            c(true, false, true),
            c(false, false, true),
        ],
        [
            c(false, true, false),
            c(false, true, true),
            c(true, true, true),
            c(true, true, false),
        ],
        [
            c(false, false, false),
            c(false, false, true),
            c(false, true, true),
            c(false, true, false),
        ],
        [
            c(true, false, false),
            c(true, true, false),
            c(true, true, true),
            c(true, false, true),
        ],
    ];
    quads
        .iter()
        .flat_map(|q| [[q[0], q[1], q[2]], [q[0], q[2], q[3]]])
        .collect()
}

#[test]
fn nothing_under_the_bit_is_said_so() {
    let mesh = Mesh::new(turned(|_| 10.0, 0.0, 40.0, 4, 90));
    assert_eq!(mesh.drop(&ball(6.0), 60.0, 0.0, 0.0), None);
    assert_eq!(mesh.drop(&ball(6.0), 20.0, 30.0, 0.0), None);
    assert_eq!(Mesh::new(vec![]).drop(&ball(6.0), 0.0, 0.0, 0.0), None);
}

#[test]
fn a_swing_is_the_furthest_the_model_reaches() {
    let mesh = Mesh::new(turned(|x| 5.0 + x / 10.0, 0.0, 40.0, 8, 64));
    assert!((mesh.swing() - 9.0).abs() < 1e-9, "{}", mesh.swing());
}
