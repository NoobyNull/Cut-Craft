//! The material cut by the bit moving exactly as a file moves it.

mod common;

use common::{ball, flat, turned};
use cut_craft_core::glam::{DVec2, DVec3};
use cut_craft_rotary::{Mesh, Pose, Section, Stock, Voxels, angle};
use std::f64::consts::{PI, TAU};

fn round(diameter: f64, length: f64) -> Stock {
    Stock::new(Section::Round { diameter }, length)
}

/// A point `r` from the axis, `along` it, `phi` round from the part's top.
fn polar(along: f64, r: f64, phi: f64) -> DVec3 {
    DVec3::new(along, r * phi.sin(), r * phi.cos())
}

#[test]
fn every_shape_of_material_holds_its_volume() {
    for stock in [
        round(30.0, 40.0),
        Stock::new(Section::Square { side: 25.0 }, 40.0),
        Stock::new(
            Section::Board {
                width: 60.0,
                thickness: 12.0,
            },
            40.0,
        ),
        Stock {
            offset: DVec2::new(4.0, -3.0),
            ..round(20.0, 30.0)
        },
    ] {
        let v = Voxels::of(&stock, 0.25);
        let want = stock.volume();
        assert!(
            (v.volume() - want).abs() / want < 0.01,
            "{stock:?}: {}",
            v.volume()
        );
    }
}

#[test]
fn a_plunge_takes_away_what_the_bit_holds_and_nothing_else() {
    let mut v = Voxels::of(&round(30.0, 40.0), 0.25);
    let bit = flat(6.0);
    let above = Pose::new(20.0, 0.0, 20.0, 0.0);
    let down = Pose::new(20.0, 0.0, 10.0, 0.0);
    assert!(v.cut(&bit, above, down) > 0);
    // under the bit, down to 10 from the axis: gone; below and beside: still there
    assert!(!v.solid(DVec3::new(20.0, 0.0, 12.0)));
    assert!(!v.solid(DVec3::new(22.5, 1.0, 14.9)));
    assert!(v.solid(DVec3::new(20.0, 0.0, 9.5)));
    assert!(v.solid(DVec3::new(24.0, 0.0, 12.0)));
    assert!(v.solid(DVec3::new(20.0, 0.0, -12.0)));
}

#[test]
fn a_ring_cut_while_a_turns_goes_all_the_way_round_at_its_depth() {
    let mut v = Voxels::of(&round(30.0, 40.0), 0.25);
    let bit = ball(4.0);
    // one straight move: A a whole turn, the bit still, 10 from the axis
    v.cut(
        &bit,
        Pose::new(20.0, 0.0, 10.0, 0.0),
        Pose::new(20.0, 0.0, 10.0, TAU),
    );
    for k in 0..360 {
        let phi = TAU * f64::from(k) / 360.0;
        assert!(!v.solid(polar(20.0, 11.0, phi)), "uncut at {phi}");
        assert!(v.solid(polar(20.0, 9.5, phi)), "cut too deep at {phi}");
        assert!(v.solid(polar(23.5, 14.0, phi)), "cut too wide at {phi}");
    }
}

#[test]
fn moving_along_while_a_turns_cuts_a_helix() {
    let mut v = Voxels::of(&round(30.0, 60.0), 0.25);
    let bit = ball(3.0);
    // two turns of A while the bit goes 20 along: a thread 10 to a turn
    v.cut(
        &bit,
        Pose::new(10.0, 0.0, 12.0, 0.0),
        Pose::new(30.0, 0.0, 12.0, 2.0 * TAU),
    );
    for k in 0..40 {
        let t = f64::from(k) / 40.0;
        let along = 10.0 + 20.0 * t;
        let phi = 2.0 * TAU * t;
        assert!(!v.solid(polar(along, 13.0, phi)), "uncut at {along}");
        // half a turn on, the thread is 5 further along: between the turns stays
        assert!(
            v.solid(polar(along, 13.0, phi + PI)),
            "cut between turns at {along}"
        );
    }
}

#[test]
fn a_bit_held_to_one_side_cuts_where_its_tip_really_is() {
    let mut v = Voxels::of(&round(30.0, 40.0), 0.25);
    let bit = ball(2.0);
    let pose = Pose::new(20.0, 6.0, 9.0, 0.7);
    v.cut(
        &bit,
        Pose {
            height: 20.0,
            ..pose
        },
        pose,
    );
    // just above the tip, in the part's frame
    let tip = pose.tip();
    let above = cut_craft_rotary::to_part(DVec3::new(20.0, 6.0, 9.5), 0.7);
    assert!(!v.solid(above));
    assert!(v.solid(tip - (above - tip)), "{tip:?}");
    // the part's angle there is A plus the bit's own angle off the top
    let want = 0.7 + 6f64.atan2(9.0);
    assert!((angle(tip) - want).abs() < 1e-9);
}

#[test]
fn cutting_never_puts_material_back_and_cuts_nothing_twice() {
    let mut v = Voxels::of(&round(30.0, 40.0), 0.5);
    let bit = ball(6.0);
    let (a, b) = (
        Pose::new(5.0, 0.0, 11.0, 0.0),
        Pose::new(35.0, 3.0, 9.0, 2.0),
    );
    let before = v.count();
    let cut = v.cut(&bit, a, b);
    assert!(cut > 0);
    assert_eq!(v.count(), before - cut);
    assert_eq!(v.cut(&bit, a, b), 0);
    assert_eq!(v.cut(&bit, b, a), 0);
}

#[test]
fn a_fast_turn_in_one_move_still_cuts_a_whole_groove() {
    // a big blank, A spun a whole turn in one move: steps are small enough everywhere
    let mut v = Voxels::of(&round(120.0, 20.0), 0.5);
    v.cut(
        &ball(2.0),
        Pose::new(10.0, 0.0, 59.0, 0.0),
        Pose::new(10.0, 0.0, 59.0, TAU),
    );
    for k in 0..720 {
        let phi = TAU * f64::from(k) / 720.0;
        assert!(
            !v.solid(polar(10.0, 59.7, phi)),
            "gap in the groove at {phi}"
        );
    }
}

#[test]
fn a_board_flipped_by_a_is_cut_on_both_faces() {
    let board = Stock::new(
        Section::Board {
            width: 60.0,
            thickness: 12.0,
        },
        40.0,
    );
    let mut v = Voxels::of(&board, 0.25);
    let bit = flat(6.0);
    // the top face 2 deep at A = 0, then A turned half round and the other face 2 deep
    v.cut(
        &bit,
        Pose::new(20.0, 0.0, 20.0, 0.0),
        Pose::new(20.0, 0.0, 4.0, 0.0),
    );
    v.cut(
        &bit,
        Pose::new(20.0, 0.0, 20.0, PI),
        Pose::new(20.0, 0.0, 4.0, PI),
    );
    assert!(!v.solid(DVec3::new(20.0, 0.0, 5.0)));
    assert!(!v.solid(DVec3::new(20.0, 0.0, -5.0)));
    assert!(v.solid(DVec3::new(20.0, 0.0, 3.5)));
    assert!(v.solid(DVec3::new(20.0, 0.0, -3.5)));
    // the far ends of the board were never near the bit
    assert!(v.solid(DVec3::new(20.0, 25.0, 5.0)));
}

#[test]
fn the_surface_drawn_holds_the_material_inside_it() {
    let mut v = Voxels::of(&round(20.0, 20.0), 0.5);
    v.cut(
        &ball(4.0),
        Pose::new(10.0, 0.0, 7.0, 0.0),
        Pose::new(10.0, 0.0, 7.0, TAU),
    );
    let (points, triangles) = v.surface();
    assert!(triangles.len() > 100, "{}", triangles.len());
    // facing out, the triangles enclose about what the cells hold
    let enclosed: f64 = triangles
        .iter()
        .map(|t| {
            let [a, b, c] = t.map(|i| points[i as usize]);
            a.dot(b.cross(c)) / 6.0
        })
        .sum();
    let held = v.volume();
    assert!(
        (enclosed - held).abs() / held < 0.03,
        "{enclosed} vs {held}"
    );
}

#[test]
fn rings_from_the_drop_finish_a_part_without_ever_cutting_into_it() {
    // a bead 6 to 8 from the axis in a 20 mm blank, finished ring by ring
    let profile = |x: f64| 6.0 + 2.0 * (PI * (x - 5.0) / 20.0).sin().max(0.0);
    let mesh = Mesh::new(turned(profile, 5.0, 25.0, 40, 360));
    let mut v = Voxels::of(&round(20.0, 30.0), 0.25);
    let bit = ball(3.0);
    let step = 0.5;
    let mut along = 6.0;
    while along <= 24.0 {
        let mut last: Option<Pose> = None;
        for k in 0..=180 {
            let turn = TAU * f64::from(k) / 180.0;
            let tip = mesh.drop(&bit, along, 0.0, turn).unwrap();
            let pose = Pose::new(along, 0.0, tip, turn);
            if let Some(from) = last {
                v.cut(&bit, from, pose);
            }
            last = Some(pose);
        }
        along += step;
    }
    // inside the bead: never cut
    for k in 0..90 {
        let phi = TAU * f64::from(k) / 90.0;
        for x in [8.0, 12.0, 15.0, 18.0, 22.0] {
            let inside = profile(x) - 0.4;
            assert!(v.solid(polar(x, inside, phi)), "gouged at {x}, {phi}");
            // well above it, under the rings: cut away
            assert!(
                !v.solid(polar(x, profile(x) + 1.0, phi)),
                "left at {x}, {phi}"
            );
        }
    }
}
