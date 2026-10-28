//! Ramps, leads and tabs on a path.

use cut_craft_core::entry::{
    Entry, Lead, Leads, Ramp, RampBy, Ramping, TabSettings, TabSpacing, Tabs, path_moves,
};
use cut_craft_core::geometry::Point;
use cut_craft_core::toolpath::{Move, MoveKind};

const SAFE: f64 = 5.0;

/// A 40 mm square, anticlockwise.
fn square() -> Vec<Point> {
    [(0.0, 0.0), (40.0, 0.0), (40.0, 40.0), (0.0, 40.0)]
        .map(|(x, y)| Point::new(x, y))
        .to_vec()
}

fn ramped(ramp: Ramp, ramping: Ramping) -> Entry {
    Entry {
        ramp,
        ramping,
        ..Entry::default()
    }
}

/// No move goes straight down into the material below `top`.
fn no_straight_plunge_below(moves: &[Move], top: f64) -> bool {
    moves.windows(2).all(|w| {
        let (a, b) = (w[0].to, w[1].to);
        let straight_down = b.z < a.z - 1e-9 && a.truncate().distance(b.truncate()) < 1e-9;
        !(straight_down && b.z < top - 1e-9)
    })
}

fn deepest(moves: &[Move]) -> f64 {
    moves.iter().map(|m| m.to.z).fold(f64::INFINITY, f64::min)
}

#[test]
fn a_smooth_ramp_goes_down_along_the_path_instead_of_straight_down() {
    let r = Ramping {
        distance: 20.0,
        ..Ramping::default()
    };
    let moves = path_moves(
        &square(),
        true,
        &[-3.0, -6.0],
        0.0,
        SAFE,
        &ramped(Ramp::Smooth, r),
    );
    assert!(no_straight_plunge_below(&moves, 0.0));
    assert!((deepest(&moves) + 6.0).abs() < 1e-9);
    // the first ramp ends 20 mm along, 3 mm down
    let first_cut = moves.iter().position(|m| m.kind == MoveKind::Cut).unwrap();
    let bottom = moves[first_cut - 1].to;
    assert!(
        (bottom.z + 3.0).abs() < 1e-9 && bottom.truncate().distance(Point::new(20.0, 0.0)) < 1e-9
    );
}

#[test]
fn a_zig_zag_ramp_comes_back_to_the_start_at_depth() {
    let r = Ramping {
        distance: 10.0,
        ..Ramping::default()
    };
    let moves = path_moves(
        &square(),
        true,
        &[-4.0],
        0.0,
        SAFE,
        &ramped(Ramp::ZigZag, r),
    );
    assert!(no_straight_plunge_below(&moves, 0.0));
    let ramp: Vec<_> = moves
        .iter()
        .filter(|m| m.kind == MoveKind::Plunge && m.to.z < 0.0)
        .collect();
    let end = ramp.last().unwrap().to;
    assert!(end.truncate().distance(Point::ZERO) < 1e-9 && (end.z + 4.0).abs() < 1e-9);
    assert!(
        ramp.iter().any(|m| (m.to.x - 10.0).abs() < 1e-9),
        "out 10 mm"
    );
}

#[test]
fn ramping_by_angle_is_never_steeper_than_the_angle() {
    let r = Ramping {
        by: RampBy::Angle,
        distance: 15.0,
        angle: 5.0,
    };
    for ramp in [Ramp::Smooth, Ramp::ZigZag] {
        let moves = path_moves(&square(), true, &[-6.0], 0.0, SAFE, &ramped(ramp, r));
        let limit = 5.0_f64.to_radians().tan() + 1e-9;
        for w in moves.windows(2) {
            let (a, b) = (w[0].to, w[1].to);
            let run = a.truncate().distance(b.truncate());
            if b.z < a.z && b.z < 0.0 && a.z <= 0.0 {
                assert!((a.z - b.z) <= limit * run, "{ramp:?}: {a} to {b}");
            }
        }
        assert!((deepest(&moves) + 6.0).abs() < 1e-9);
        if ramp == Ramp::ZigZag {
            // no further than 15 mm along
            assert!(
                moves
                    .iter()
                    .filter(|m| m.to.z < 0.0)
                    .all(|m| m.to.y > 0.0 || m.to.x <= 40.0)
            );
        }
    }
}

#[test]
fn a_spiral_goes_down_a_pass_a_loop_then_round_once_more_at_the_bottom() {
    let moves = path_moves(
        &square(),
        true,
        &[-3.0, -6.0],
        0.0,
        SAFE,
        &ramped(Ramp::Spiral, Ramping::default()),
    );
    assert!(no_straight_plunge_below(&moves, 0.0));
    // at each corner of the square on the way round the depth keeps going down
    let cuts: Vec<f64> = moves
        .iter()
        .filter(|m| m.kind == MoveKind::Cut)
        .map(|m| m.to.z)
        .collect();
    assert!(cuts.windows(2).all(|w| w[1] <= w[0] + 1e-9));
    assert!(
        (cuts[3] + 3.0).abs() < 1e-9,
        "a pass in the first loop: {cuts:?}"
    );
    let flat = cuts.iter().filter(|&&z| (z + 6.0).abs() < 1e-9).count();
    assert!(flat >= 5, "a loop at full depth: {cuts:?}");
}

#[test]
fn a_straight_lead_comes_in_from_the_side_asked_at_the_angle() {
    let entry = Entry {
        lead: Lead::Straight,
        leads: Leads {
            angle: 90.0,
            length: 5.0,
            lead_out: true,
            overcut: 2.0,
            ..Leads::default()
        },
        side: -1.0, // right of travel: outside an anticlockwise square
        ..Entry::default()
    };
    let moves = path_moves(&square(), true, &[-3.0], 0.0, SAFE, &entry);
    let plunge = moves.iter().find(|m| m.kind == MoveKind::Plunge).unwrap();
    assert!(
        plunge.to.truncate().distance(Point::new(0.0, -5.0)) < 1e-9,
        "{}",
        plunge.to
    );
    let last_cut = moves
        .iter()
        .rev()
        .find(|m| m.kind == MoveKind::Cut)
        .unwrap();
    // round, 2 mm past the start, then 5 mm out
    assert!(
        last_cut.to.truncate().distance(Point::new(2.0, -5.0)) < 1e-9,
        "{}",
        last_cut.to
    );
}

#[test]
fn a_circular_lead_meets_the_path_along_it() {
    let entry = Entry {
        lead: Lead::Circular,
        leads: Leads {
            radius: 5.0,
            length: 5.0 * std::f64::consts::FRAC_PI_2,
            ..Leads::default()
        },
        side: -1.0,
        ..Entry::default()
    };
    let moves = path_moves(&square(), true, &[-3.0], 0.0, SAFE, &entry);
    let plunge = moves
        .iter()
        .position(|m| m.kind == MoveKind::Plunge)
        .unwrap();
    // a quarter circle centred 5 mm outside the start: it starts 5 mm behind and out
    assert!(moves[plunge].to.truncate().distance(Point::new(-5.0, -5.0)) < 1e-6);
    // every point of the arc is 5 mm from its centre, and the last step runs along +x
    let centre = Point::new(0.0, -5.0);
    let arc: Vec<Point> = moves[plunge..=plunge + 16]
        .iter()
        .map(|m| m.to.truncate())
        .collect();
    assert!(arc.iter().all(|p| (p.distance(centre) - 5.0).abs() < 1e-6));
    let last = arc[16] - arc[15];
    assert!(last.y.abs() < 0.1 * last.x && last.x > 0.0, "{last}");
}

fn tabbed(tabs: Tabs) -> Vec<Move> {
    let entry = Entry {
        tabs,
        tab: TabSettings {
            length: 4.0,
            thickness: 2.0,
            spacing: TabSpacing::ConstantNumber,
            number: 4,
            distance: 0.0,
        },
        tab_top: -6.0 + 2.0,
        tool_diameter: 6.0,
        ..Entry::default()
    };
    path_moves(&square(), true, &[-3.0, -6.0], 0.0, SAFE, &entry)
}

#[test]
fn flat_tabs_lift_over_length_and_diameter_only_on_passes_below_them() {
    let moves = tabbed(Tabs::Flat);
    // the first pass (3 mm) is above the tabs' top (4 mm down): no lifts
    let at_top: Vec<&Move> = moves
        .iter()
        .filter(|m| (m.to.z + 4.0).abs() < 1e-9)
        .collect();
    assert!(at_top.len() > 1, "the bottom pass reaches the tabs' top");
    // one tab in the middle of the bottom side: lifted from x = 15 to 25 (4 + 6 long)
    let lifted: Vec<f64> = at_top
        .iter()
        .filter(|m| m.to.y.abs() < 1e-9)
        .map(|m| m.to.x)
        .collect();
    let (lo, hi) = lifted
        .iter()
        .fold((f64::MAX, f64::MIN), |(l, h), &x| (l.min(x), h.max(x)));
    assert!(
        (lo - 15.0).abs() < 1e-9 && (hi - 25.0).abs() < 1e-9,
        "{lifted:?}"
    );
    // straight up and down at the ends
    let ups = moves
        .windows(2)
        .filter(|w| {
            w[1].to.z > w[0].to.z
                && w[0].to.truncate() == w[1].to.truncate()
                && w[1].kind == MoveKind::Cut
        })
        .count();
    assert_eq!(ups, 4);
}

#[test]
fn triangular_tabs_peak_at_the_middle() {
    let moves = tabbed(Tabs::Triangular);
    let peak = moves.iter().find(|m| (m.to.z + 4.0).abs() < 1e-9).unwrap();
    assert!(
        peak.to.truncate().distance(Point::new(20.0, 0.0)) < 1e-9,
        "{}",
        peak.to
    );
    let ups = moves
        .windows(2)
        .filter(|w| {
            w[1].to.z > w[0].to.z
                && w[0].to.truncate() == w[1].to.truncate()
                && w[1].kind == MoveKind::Cut
        })
        .count();
    assert_eq!(ups, 0, "no straight lifts");
}
