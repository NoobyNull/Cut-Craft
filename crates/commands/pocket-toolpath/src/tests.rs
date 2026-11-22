use super::*;
use cut_craft_core::entry::EntryMode;
use cut_craft_core::geometry::{Point, polyline_distance};
use cut_craft_core::testing::{cut_bounds, made_toolpath, square};
use cut_craft_core::toolpath::Toolpath;

cut_craft_core::contract_tests!(PocketToolpath);

fn pocket(size: f64, p: &Params) -> Result<Toolpath, String> {
    made_toolpath::<PocketToolpath>(vec![square(0.0, 0.0, size)], p)
}

#[test]
fn stays_a_tool_radius_inside_the_shape() {
    let b = cut_bounds(&pocket(20.0, &Params::default()).unwrap());
    assert!(
        b.min.x >= 3.0 - 0.02
            && b.max.x <= 17.0 + 0.02
            && b.min.y >= 3.0 - 0.02
            && b.max.y <= 17.0 + 0.02,
        "{b:?}"
    );
}

#[test]
fn a_pocket_clears_with_the_clearing_stepover_not_the_finishing_one() {
    let with = |stepover: f64, finish_stepover: f64| {
        let mut p = Params::default();
        p.tool.stepover = stepover;
        p.tool.finish_stepover = finish_stepover;
        pocket(30.0, &p).unwrap().moves
    };
    assert_eq!(
        with(0.4, 0.05),
        with(0.4, 0.4),
        "finishing does not change it"
    );
    assert_ne!(with(0.2, 0.4), with(0.4, 0.4), "clearing does");
}

#[test]
fn the_tool_reaches_the_middle() {
    let tp = pocket(20.0, &Params::default()).unwrap();
    let middle = Point::new(10.0, 10.0);
    // distance to the cut segments themselves, not just their end points
    let closest = tp
        .moves
        .windows(2)
        .filter(|w| w[1].kind == MoveKind::Cut)
        .map(|w| polyline_distance(&[w[0].to.truncate(), w[1].to.truncate()], middle))
        .fold(f64::MAX, f64::min);
    assert!(
        closest <= Tool::default().diameter / 2.0,
        "the middle is {closest} mm from the nearest cut"
    );
}

/// The nearest a cutting move comes to `at`.
fn nearest_cut(tp: &Toolpath, at: Point) -> f64 {
    tp.moves
        .windows(2)
        .filter(|w| w[1].kind == MoveKind::Cut)
        .map(|w| polyline_distance(&[w[0].to.truncate(), w[1].to.truncate()], at))
        .fold(f64::MAX, f64::min)
}

/// A 40 mm square with a 10 mm square island in its middle, pocketed with `p`.
fn ring_pocket(p: &Params) -> Toolpath {
    made_toolpath::<PocketToolpath>(vec![square(0.0, 0.0, 40.0), square(15.0, 15.0, 10.0)], p)
        .unwrap()
}

#[test]
fn a_shape_inside_another_is_an_island_the_pocket_goes_round() {
    let radius = Tool::default().diameter / 2.0;
    for (strategy, motion) in [
        (Strategy::Offset, Motion::Straight),
        (Strategy::Raster, Motion::Straight),
        (Strategy::Offset, Motion::Trochoidal),
        (Strategy::Adaptive, Motion::Straight),
    ] {
        let tp = ring_pocket(&Params {
            strategy,
            motion,
            ..Params::default()
        });
        let middle = nearest_cut(&tp, Point::new(20.0, 20.0));
        assert!(
            middle >= 5.0 + radius - 0.05,
            "{strategy:?} {motion:?}: cuts {middle} mm from the island's middle"
        );
        // and the ring round it is still cleared
        for at in [Point::new(7.5, 20.0), Point::new(32.5, 32.5)] {
            let gap = nearest_cut(&tp, at);
            assert!(
                gap <= radius,
                "{strategy:?} {motion:?}: {at:?} {gap} mm from a cut"
            );
        }
    }
}

#[test]
fn the_islands_wall_is_cut_the_chosen_direction_too() {
    for direction in [Direction::Climb, Direction::Conventional] {
        let tp = ring_pocket(&Params {
            direction,
            ..Params::default()
        });
        // the island's wall ring: the cut nearest the island's side
        let wall = nearest_cut(&tp, Point::new(20.0, 20.0));
        let middle = Point::new(20.0, 20.0);
        let mut ring: Vec<Point> = vec![];
        let mut from = None;
        for m in &tp.moves {
            if m.kind == MoveKind::Cut {
                if ring.is_empty() {
                    ring.extend(from);
                }
                ring.push(m.to.truncate());
            } else if !ring.is_empty() {
                if (polyline_distance(&ring, middle) - wall).abs() < 0.01 {
                    break;
                }
                ring.clear();
            }
            from = Some(m.to.truncate());
        }
        assert!(!ring.is_empty(), "{direction:?}: no ring round the island");
        // climb on an island's wall (waste outside it) runs clockwise
        let clockwise = cut_craft_core::geometry::winding(&ring) < 0.0;
        assert_eq!(clockwise, direction == Direction::Climb, "{direction:?}");
    }
}

#[test]
fn shapes_side_by_side_are_each_pocketed() {
    let tp = made_toolpath::<PocketToolpath>(
        vec![square(0.0, 0.0, 20.0), square(30.0, 0.0, 20.0)],
        &Params::default(),
    )
    .unwrap();
    for at in [Point::new(10.0, 10.0), Point::new(40.0, 10.0)] {
        assert!(
            nearest_cut(&tp, at) <= Tool::default().diameter / 2.0,
            "{at:?}"
        );
    }
}

/// The length of the cutting moves (cuts and plunges) in `tp`.
fn cut_length(tp: &Toolpath) -> f64 {
    tp.moves
        .windows(2)
        .filter(|w| w[1].kind != MoveKind::Rapid)
        .map(|w| w[0].to.distance(w[1].to))
        .sum()
}

#[test]
fn adaptive_clears_the_whole_pocket_and_stays_inside_it() {
    let p = Params {
        strategy: Strategy::Adaptive,
        depth: 6.0,
        ..Params::default()
    };
    let tp = pocket(40.0, &p).unwrap();
    let r = Tool::default().diameter / 2.0;
    let b = cut_bounds(&tp);
    assert!(
        b.min.x >= r - 0.02 && b.max.x <= 40.0 - r + 0.02,
        "inside the walls: {b:?}"
    );
    assert!(b.min.y >= r - 0.02 && b.max.y <= 40.0 - r + 0.02, "{b:?}");
    for i in 0..=20 {
        for j in 0..=20 {
            let at = Point::new(
                r + (40.0 - 2.0 * r) * f64::from(i) / 20.0,
                r + (40.0 - 2.0 * r) * f64::from(j) / 20.0,
            );
            let gap = nearest_cut(&tp, at);
            assert!(gap <= r + 0.05, "{at:?} is {gap} mm from the nearest cut");
        }
    }
    // the wall gets its own pass: cuts run along it, a radius in, at the bottom
    let along_wall = tp
        .moves
        .iter()
        .filter(|m| m.kind == MoveKind::Cut && (m.to.z + 6.0).abs() < 1e-9)
        .filter(|m| (m.to.x - r).abs() < 0.02)
        .count();
    assert!(along_wall >= 2, "no wall pass at the bottom");
}

#[test]
fn adaptive_goes_down_once_per_area_per_pass() {
    let p = Params {
        strategy: Strategy::Adaptive,
        depth: 6.0,
        ..Params::default()
    };
    let tp = pocket(40.0, &p).unwrap();
    // each time the tool comes down from the clearance height into uncut material: once
    // per pass depth (the wall pass and any jumps go down into space already cleared)
    let entries = tp
        .moves
        .windows(2)
        .filter(|w| w[0].kind == MoveKind::Rapid && w[1].kind == MoveKind::Plunge)
        .count();
    let lifts = tp
        .moves
        .windows(2)
        .filter(|w| w[1].kind == MoveKind::Rapid && w[1].to.z > w[0].to.z)
        .count();
    assert!(entries >= 2, "{entries} entries for two passes");
    assert!(lifts <= 6, "it lifts {lifts} times");
}

#[test]
fn adaptive_cuts_less_than_trochoidal_loops() {
    let adaptive = pocket(
        40.0,
        &Params {
            strategy: Strategy::Adaptive,
            ..Params::default()
        },
    )
    .unwrap();
    let loops = pocket(
        40.0,
        &Params {
            motion: Motion::Trochoidal,
            ..Params::default()
        },
    )
    .unwrap();
    let (a, l) = (cut_length(&adaptive), cut_length(&loops));
    assert!(a < l, "adaptive {a:.0} mm, loops {l:.0} mm");
}

#[test]
fn adaptive_sweeps_a_narrow_channel_and_only_crosses_at_safe_height() {
    // two 15 mm wide channels off a 40 mm room: each swept, then the room cleared
    let p = Point::new;
    let shape = cut_craft_core::geometry::Path::polyline(
        &[
            p(0.0, 0.0),
            p(40.0, 0.0),
            p(40.0, 12.0),
            p(120.0, 12.0),
            p(120.0, 27.0),
            p(40.0, 27.0),
            p(40.0, 40.0),
            p(27.0, 40.0),
            p(27.0, 120.0),
            p(12.0, 120.0),
            p(12.0, 40.0),
            p(0.0, 40.0),
        ],
        true,
    );
    let params = Params {
        strategy: Strategy::Adaptive,
        depth: 3.0,
        ..Params::default()
    };
    let tp = made_toolpath::<PocketToolpath>(vec![shape], &params).unwrap();
    let safe = tp.moves.iter().map(|m| m.to.z).fold(f64::MIN, f64::max);
    for w in tp.moves.windows(2) {
        let across = w[0].to.truncate().distance(w[1].to.truncate()) > 1e-9;
        if w[1].kind == MoveKind::Rapid && across {
            assert!(
                (w[0].to.z - safe).abs() < 1e-9 && (w[1].to.z - safe).abs() < 1e-9,
                "a rapid across below safe height: {:?} -> {:?}",
                w[0].to,
                w[1].to
            );
        }
    }
    // the channels are cleared: their far ends are reached
    let r = Tool::default().diameter / 2.0;
    assert!(nearest_cut(&tp, p(120.0 - r - 1.0, 19.5)) <= r);
    assert!(nearest_cut(&tp, p(19.5, 120.0 - r - 1.0)) <= r);
}

#[test]
fn adaptive_is_saved_by_name() {
    let p = Params {
        strategy: Strategy::Adaptive,
        ..Params::default()
    };
    let json = serde_json::to_value(&p).unwrap();
    assert_eq!(json["strategy"], "adaptive");
    let back: Params = serde_json::from_value(json).unwrap();
    assert_eq!(back.strategy, Strategy::Adaptive);
}

/// The pocket `p` makes in a `size` square, with the job's Clearing Settings `clearing`.
fn pocket_with(
    clearing: cut_craft_core::entry::ClearingSettings,
    size: f64,
    p: &Params,
) -> Toolpath {
    let mut doc = cut_craft_core::testing::document_with(vec![square(0.0, 0.0, size)]);
    doc.job.clearing = clearing;
    let change = PocketToolpath::run(&doc, p).unwrap();
    doc.apply(&change).unwrap();
    doc.toolpaths.into_values().next().unwrap()
}

fn stepping(step: f64) -> cut_craft_core::entry::ClearingSettings {
    let mut clearing = cut_craft_core::entry::ClearingSettings::default();
    clearing.loops.step = step;
    clearing
}

#[test]
fn adaptive_bites_the_jobs_light_step_of_the_bit() {
    let tool = Tool::default();
    let ten = stepping(10.0);
    assert!((adaptive_bite(&tool, &ten) - 0.1 * tool.diameter).abs() < 1e-12);
    // a lighter step is a longer path, whatever the tool's own clearing stepover
    let p = Params {
        strategy: Strategy::Adaptive,
        ..Params::default()
    };
    let light = cut_length(&pocket_with(stepping(8.0), 30.0, &p));
    let heavy = cut_length(&pocket_with(stepping(20.0), 30.0, &p));
    assert!(light > 1.3 * heavy, "8%: {light:.0} mm, 20%: {heavy:.0} mm");
}

/// The long cuts across a raster pocket at angle 0: true for each that runs towards -x.
fn rows_heading_left(tp: &Toolpath) -> Vec<bool> {
    tp.moves
        .windows(2)
        .filter(|w| w[1].kind == MoveKind::Cut && (w[1].to.z - w[0].to.z).abs() < 1e-9)
        .filter(|w| (w[1].to.y - w[0].to.y).abs() < 1e-9 && (w[1].to.x - w[0].to.x).abs() > 20.0)
        .map(|w| w[1].to.x < w[0].to.x)
        .collect()
}

#[test]
fn keep_climb_raster_rows_all_cut_climb_and_come_back_through_the_row_just_cut() {
    let p = Params {
        strategy: Strategy::Raster,
        profile_pass: ProfilePass::None,
        ..Params::default()
    };
    let tp = pocket_with(cut_craft_core::entry::ClearingSettings::default(), 40.0, &p);
    // rows step up the pocket (+y), the material above: climb keeps it on the bit's
    // right, so a row cuts right to left, and comes back left to right in it
    let rows = rows_heading_left(&tp);
    let (cutting, returning): (Vec<bool>, Vec<bool>) = rows.iter().partition(|&&left| left);
    assert!(cutting.len() >= 10, "{rows:?}");
    assert_eq!(
        returning.len(),
        cutting.len() - 1,
        "a return after every row but the last"
    );
    // and still without lifting between rows
    let lifts = tp
        .moves
        .windows(2)
        .filter(|w| w[1].kind == MoveKind::Rapid && w[1].to.z > w[0].to.z)
        .count();
    assert_eq!(lifts, 1, "one pass, one lift at its end");
    // conventional: the other way
    let conventional = Params {
        direction: Direction::Conventional,
        ..p
    };
    let tp = pocket_with(
        cut_craft_core::entry::ClearingSettings::default(),
        40.0,
        &conventional,
    );
    let rows = rows_heading_left(&tp);
    assert!(rows.iter().filter(|&&l| !l).count() > rows.iter().filter(|&&l| l).count());
}

#[test]
fn alternate_raster_rows_go_back_and_forth() {
    let p = Params {
        strategy: Strategy::Raster,
        profile_pass: ProfilePass::None,
        ..Params::default()
    };
    let clearing = cut_craft_core::entry::ClearingSettings {
        way_back: cut_craft_core::entry::WayBack::Alternate,
        ..Default::default()
    };
    let rows = rows_heading_left(&pocket_with(clearing, 40.0, &p));
    assert!(rows.len() >= 10);
    assert!(rows.windows(2).all(|w| w[0] != w[1]), "{rows:?}");
}

#[test]
fn changing_the_clearing_settings_marks_adaptive_and_raster_pockets_stale() {
    let mut doc = cut_craft_core::testing::document_with(vec![square(0.0, 0.0, 30.0)]);
    let mut history = cut_craft_core::History::default();
    for strategy in [Strategy::Adaptive, Strategy::Raster, Strategy::Offset] {
        let p = Params {
            strategy,
            ..Params::default()
        };
        let change = PocketToolpath::run(&doc, &p).unwrap();
        history.apply(&mut doc, change).unwrap();
    }
    let mut c = cut_craft_core::Change::builder(&doc);
    let mut job = doc.job;
    job.clearing.loops.step = 15.0;
    c.set_job(job);
    let change = c.finish();
    history.apply(&mut doc, change).unwrap();
    let stale: Vec<(String, bool)> = doc
        .toolpaths
        .values()
        .map(|t| (t.params["strategy"].as_str().unwrap().to_owned(), t.stale))
        .collect();
    for (strategy, is_stale) in &stale {
        assert_eq!(*is_stale, strategy != "offset", "{stale:?}");
    }
}

#[test]
fn trochoidal_loops_clear_the_whole_pocket_and_stay_inside_it() {
    let p = Params {
        motion: Motion::Trochoidal,
        ..Params::default()
    };
    let tp = pocket(40.0, &p).unwrap();
    let r = Tool::default().diameter / 2.0;
    let b = cut_bounds(&tp);
    assert!(
        b.min.x >= r - 0.02 && b.max.x <= 40.0 - r + 0.02,
        "inside the walls: {b:?}"
    );
    assert!(b.min.y >= r - 0.02 && b.max.y <= 40.0 - r + 0.02, "{b:?}");
    // every place the tool's middle may go is cut
    for i in 0..=20 {
        for j in 0..=20 {
            let at = Point::new(
                r + (40.0 - 2.0 * r) * f64::from(i) / 20.0,
                r + (40.0 - 2.0 * r) * f64::from(j) / 20.0,
            );
            let gap = nearest_cut(&tp, at);
            assert!(gap <= r + 0.05, "{at:?} is {gap} mm from the nearest cut");
        }
    }
    // it loops: within a dozen moves the tool heads back the way it came, again and again
    let offset = pocket(40.0, &Params::default()).unwrap();
    let turns = |tp: &Toolpath| {
        tp.moves
            .windows(13)
            .filter(|w| w.iter().all(|m| m.kind == MoveKind::Cut))
            .filter(|w| (w[1].to - w[0].to).dot(w[12].to - w[11].to) < 0.0)
            .count()
    };
    assert!(
        turns(&tp) > 10 * turns(&offset).max(1),
        "{} turns",
        turns(&tp)
    );
}

#[test]
fn keep_climb_loops_still_turn_anticlockwise_on_the_pass_back() {
    // how the path bends, move to move, at `z`: positive where it turns anticlockwise
    let bending = |tp: &Toolpath, z: f64| -> f64 {
        let at: Vec<Point> = tp
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Cut && (m.to.z - z).abs() < 1e-9)
            .map(|m| m.to.truncate())
            .collect();
        at.windows(3)
            .map(|w| (w[1] - w[0]).perp_dot(w[2] - w[1]).signum())
            .sum()
    };
    let p = Params {
        depth: 6.0,
        motion: Motion::Trochoidal,
        ways: Ways::Both,
        ..Params::default()
    };
    let tp = pocket_with(cut_craft_core::entry::ClearingSettings::default(), 30.0, &p);
    assert!(bending(&tp, -3.0) > 0.0 && bending(&tp, -6.0) > 0.0);
    // and the second pass does run back the other way round the pocket
    let first = tp
        .moves
        .iter()
        .find(|m| m.kind == MoveKind::Cut && (m.to.z + 3.0).abs() < 1e-9);
    let second = tp
        .moves
        .iter()
        .find(|m| m.kind == MoveKind::Cut && (m.to.z + 6.0).abs() < 1e-9);
    assert!(first.is_some() && second.is_some());
}

#[test]
fn both_directions_take_turns_climb_then_conventional() {
    // twice the area the cuts at `z` go round: positive anticlockwise (climb in a pocket)
    let turning = |tp: &Toolpath, z: f64| -> f64 {
        tp.moves
            .windows(2)
            .filter(|w| w[1].kind == MoveKind::Cut)
            .filter(|w| w.iter().all(|m| (m.to.z - z).abs() < 1e-9))
            .map(|w| w[0].to.truncate().perp_dot(w[1].to.truncate()))
            .sum()
    };
    // each pass goes back round the other way, rings and loops alike, when the job's
    // Clearing Settings alternate
    let alternate = cut_craft_core::entry::ClearingSettings {
        way_back: cut_craft_core::entry::WayBack::Alternate,
        ..Default::default()
    };
    for motion in [Motion::Straight, Motion::Trochoidal] {
        let p = Params {
            depth: 9.0,
            motion,
            ways: Ways::Both,
            ..Params::default()
        };
        let tp = pocket_with(alternate, 30.0, &p);
        let ways: Vec<f64> = [-3.0, -6.0, -9.0].map(|z| turning(&tp, z)).to_vec();
        assert!(
            ways[0] > 0.0 && ways[1] < 0.0 && ways[2] > 0.0,
            "{motion:?}: climb, conventional, climb: {ways:?}"
        );
    }
}

#[test]
fn the_loops_turn_their_own_way_whichever_the_cut_direction() {
    use cut_craft_core::entry::Turn;
    // twice the area the first pass's loops go round: positive anticlockwise
    let turning = |direction, turn| -> f64 {
        let p = Params {
            motion: Motion::Trochoidal,
            direction,
            ..Params::default()
        };
        let mut clearing = cut_craft_core::entry::ClearingSettings::default();
        clearing.loops.turn = turn;
        let tp = pocket_with(clearing, 30.0, &p);
        tp.moves
            .windows(2)
            .filter(|w| w.iter().all(|m| m.kind == MoveKind::Cut))
            .map(|w| w[0].to.truncate().perp_dot(w[1].to.truncate()))
            .sum()
    };
    for direction in [Direction::Climb, Direction::Conventional] {
        assert!(
            turning(direction, Turn::Anticlockwise) > 0.0,
            "{direction:?}"
        );
        assert!(turning(direction, Turn::Clockwise) < 0.0, "{direction:?}");
    }
}

#[test]
fn rings_step_from_the_middle_out_the_outline_in_or_by_turns() {
    // how far from the middle each pass's first ring is
    let first_rings = |rings: RingOrder| -> Vec<f64> {
        let p = Params {
            depth: 9.0,
            rings,
            ..Params::default()
        };
        let tp = pocket(30.0, &p).unwrap();
        [-3.0, -6.0, -9.0]
            .map(|z| {
                let m = tp
                    .moves
                    .iter()
                    .find(|m| m.kind == MoveKind::Cut && (m.to.z - z).abs() < 1e-9)
                    .unwrap();
                m.to.truncate().distance(Point::new(15.0, 15.0))
            })
            .to_vec()
    };
    let (middle, outline) = (|d: &f64| *d < 5.0, |d: &f64| *d > 9.0);
    let out = first_rings(RingOrder::MiddleOut);
    assert!(out.iter().all(middle), "{out:?}");
    let inward = first_rings(RingOrder::OutlineIn);
    assert!(inward.iter().all(outline), "{inward:?}");
    let turns = first_rings(RingOrder::Alternate);
    assert!(
        middle(&turns[0]) && outline(&turns[1]) && middle(&turns[2]),
        "middle, outline, middle: {turns:?}"
    );
}

/// The default settings going straight down.
fn plain() -> Params {
    Params {
        entry: Some(EntrySettings {
            mode: EntryMode::Plunge,
            ..EntrySettings::default()
        }),
        ..Params::default()
    }
}

/// How many times the tool goes down from above the material into it.
fn entries(tp: &Toolpath) -> usize {
    tp.moves
        .windows(2)
        .filter(|w| w[0].to.z >= 0.0 && w[1].to.z < -1e-9 && w[1].kind != MoveKind::Rapid)
        .count()
}

/// The steepest any move goes down into the material, in degrees from level.
fn steepest(tp: &Toolpath) -> f64 {
    tp.moves
        .windows(2)
        .filter(|w| w[1].kind != MoveKind::Rapid && w[1].to.z < w[0].to.z - 1e-9)
        .filter(|w| w[1].to.z < -1e-9 && w[0].to.z < 1e-9)
        .map(|w| {
            let (a, b) = (w[0].to, w[1].to);
            (a.z - b.z)
                .atan2(a.truncate().distance(b.truncate()))
                .to_degrees()
        })
        .fold(0.0, f64::max)
}

/// The default settings going down in a helix.
fn helix() -> Params {
    Params {
        entry: Some(EntrySettings {
            mode: EntryMode::Helix,
            ..EntrySettings::default()
        }),
        ..Params::default()
    }
}

#[test]
fn a_surfacing_bit_never_goes_straight_down_into_the_material() {
    let tp = pocket(
        60.0,
        &Params {
            tool: Tool {
                name: "Spoilboard Surfacing Bit".into(),
                diameter: 25.4,
                ..Tool::default()
            },
            depth: 1.0,
            ..Params::default()
        },
    )
    .unwrap();
    let straight_in = tp.moves.windows(2).any(|w| {
        let (a, b) = (w[0].to, w[1].to);
        b.z < -1e-9 && b.z < a.z - 1e-9 && a.truncate().distance(b.truncate()) < 1e-9
    });
    assert!(!straight_in, "{:?}", tp.moves);
    assert_ne!(tp.fallbacks, vec![], "and Checks before cutting says so");
}

#[test]
fn a_new_pocket_goes_straight_down_until_asked_otherwise() {
    assert_eq!(
        Params::default().entry.map(|e| e.mode),
        Some(EntryMode::Plunge)
    );
}

#[test]
fn a_pocket_helixes_down_once_a_level_and_links_its_rings_at_depth() {
    let tp = pocket(
        40.0,
        &Params {
            depth: 6.0,
            ..helix()
        },
    )
    .unwrap();
    // two passes of 3 mm: two entries, each a helix no steeper than 3°
    assert_eq!(entries(&tp), 2);
    assert!(steepest(&tp) <= 3.0 + 1e-6, "{}", steepest(&tp));
    assert_eq!(tp.fallbacks, vec![]);
    // and it still clears the pocket to the edge
    let b = cut_bounds(&tp);
    assert!(b.min.x < 3.1 && b.max.x > 36.9, "{b:?}");
}

#[test]
fn each_area_of_a_pocket_is_entered_on_its_own() {
    let tp = made_toolpath::<PocketToolpath>(
        vec![square(0.0, 0.0, 30.0), square(50.0, 0.0, 30.0)],
        &helix(),
    )
    .unwrap();
    assert_eq!(entries(&tp), 2);
    assert!(steepest(&tp) <= 3.0 + 1e-6);
}

#[test]
fn a_pocket_too_small_for_a_helix_says_where_it_went_down_another_way() {
    // a 7 mm square for a 6.35 mm bit: no room to go round
    let tp = pocket(7.0, &helix()).unwrap();
    assert_ne!(tp.fallbacks, vec![]);
    assert!(tp.fallbacks.iter().all(|f| f.wanted == EntryMode::Helix));
}

#[test]
fn clears_each_depth_before_going_deeper() {
    let tp = pocket(
        20.0,
        &Params {
            depth: 6.0,
            ..plain()
        },
    )
    .unwrap();
    let depths: Vec<f64> = tp
        .moves
        .iter()
        .filter(|m| m.kind == MoveKind::Plunge)
        .map(|m| m.to.z)
        .collect();
    assert!(
        depths.windows(2).all(|w| w[1] <= w[0]),
        "never climbs back to a shallower pass: {depths:?}"
    );
    assert_eq!(depths.first(), Some(&-3.0));
    assert_eq!(depths.last(), Some(&-6.0));
}

#[test]
fn clearance_tools_clear_first_and_the_pocket_tool_cuts_only_what_they_left() {
    use cut_craft_core::testing::document_with;
    let mill = |d: f64| Tool {
        name: format!("{d} mm End Mill"),
        diameter: d,
        ..Tool::default()
    };
    let p = Params {
        tool: mill(2.0),
        clearance_tools: vec![mill(12.0)],
        ..Params::default()
    };
    let mut doc = document_with(vec![square(0.0, 0.0, 40.0)]);
    doc.apply(&PocketToolpath::run(&doc, &p).unwrap()).unwrap();
    let made: Vec<&Toolpath> = doc.toolpaths.values().collect();
    assert_eq!(made.len(), 2);
    assert_eq!(made[0].name, "Pocket [1]");
    assert_eq!(made[0].tool.diameter, 12.0);
    assert_eq!(made[1].name, "Pocket [2]");
    assert_eq!(made[1].tool.diameter, 2.0);
    // the small tool only goes where the big one could not: far less than a full pocket
    let alone = pocket(
        40.0,
        &Params {
            tool: mill(2.0),
            ..Params::default()
        },
    )
    .unwrap();
    assert!(
        made[1].moves.len() * 2 < alone.moves.len(),
        "{} vs {}",
        made[1].moves.len(),
        alone.moves.len()
    );
    // and it keeps its rest settings, so recalculating it does the same
    assert!(cut_craft_core::rest::Rest::of(&made[1].params).is_some());
}

#[test]
fn a_clearing_bit_not_bigger_than_the_finishing_bit_is_refused() {
    use cut_craft_core::testing::document_with;
    let mill = |d: f64| Tool {
        name: format!("{d} mm End Mill"),
        diameter: d,
        ..Tool::default()
    };
    let p = Params {
        tool: mill(6.0),
        clearance_tools: vec![mill(3.0)],
        ..Params::default()
    };
    let doc = document_with(vec![square(0.0, 0.0, 40.0)]);
    let Err(CommandError::InvalidParams(why)) = PocketToolpath::run(&doc, &p) else {
        panic!("a 3 mm clearing bit before a 6 mm finishing bit");
    };
    assert!(
        why.contains("must be bigger than the finishing bit"),
        "{why}"
    );
}

#[test]
fn clearing_bits_cut_largest_first_whatever_order_they_were_picked_in() {
    use cut_craft_core::testing::document_with;
    let mill = |d: f64| Tool {
        name: format!("{d} mm End Mill"),
        diameter: d,
        ..Tool::default()
    };
    let p = Params {
        tool: mill(2.0),
        clearance_tools: vec![mill(8.0), mill(20.0)],
        ..Params::default()
    };
    let mut doc = document_with(vec![square(0.0, 0.0, 60.0)]);
    doc.apply(&PocketToolpath::run(&doc, &p).unwrap()).unwrap();
    let sizes: Vec<f64> = doc.toolpaths.values().map(|t| t.tool.diameter).collect();
    assert_eq!(sizes, vec![20.0, 8.0, 2.0]);
}

#[test]
fn a_shape_smaller_than_the_tool_cannot_be_pocketed() {
    assert_eq!(
        pocket(4.0, &Params::default()).unwrap_err(),
        "The 6 mm End Mill does not fit inside the selected shapes. Pick a smaller tool, \
             or make the shapes bigger."
    );
}

#[test]
fn stepover_must_be_a_fraction_of_the_tool() {
    let p = Params {
        tool: Tool {
            stepover: 0.0,
            ..Tool::default()
        },
        ..Params::default()
    };
    assert_eq!(
        pocket(20.0, &p).unwrap_err(),
        "Set the tool's clearing stepover to more than 0% and at most 100% in the Tool Database."
    );
}

#[test]
fn raster_lines_link_along_the_wall_without_lifting() {
    // like the reference zig-zag: down once per pass, each line stepping over to the next
    // at depth along the wall, never back down into uncut material
    let p = Params {
        strategy: Strategy::Raster,
        depth: 6.0,
        ..Params::default()
    };
    let tp = pocket(40.0, &p).unwrap();
    let lifts = tp
        .moves
        .windows(2)
        .filter(|w| w[1].kind == MoveKind::Rapid && w[1].to.z > w[0].to.z)
        .count();
    // a lift at the end of each pass (two passes), none between lines
    assert!(lifts <= 2, "it lifts {lifts} times");
}

/// The nearest any cut at depth `z` comes to the left wall (x = 0) of a square pocket.
fn nearest_left(tp: &Toolpath, z: f64) -> f64 {
    tp.moves
        .iter()
        .filter(|m| m.kind == MoveKind::Cut && (m.to.z - z).abs() < 1e-9)
        .map(|m| m.to.x)
        .fold(f64::INFINITY, f64::min)
}

#[test]
fn the_clearing_leaves_a_skim_for_the_edge_pass_whatever_the_style() {
    let r = Tool::default().diameter / 2.0;
    for strategy in [Strategy::Offset, Strategy::Raster, Strategy::Adaptive] {
        let p = Params {
            strategy,
            edge_allowance: 0.5,
            ..Params::default()
        };
        let tp = pocket(40.0, &p).unwrap();
        // the walls are cut, by the edge pass
        assert!((nearest_left(&tp, -3.0) - r).abs() < 0.02, "{strategy:?}");
        // and before it the clearing kept the skim's width off them
        let last_wall = tp
            .moves
            .iter()
            .rposition(|m| m.kind == MoveKind::Cut && (m.to.x - r).abs() < 0.02)
            .unwrap();
        let first_wall = tp
            .moves
            .iter()
            .position(|m| m.kind == MoveKind::Cut && (m.to.x - r).abs() < 0.02)
            .unwrap();
        let before: Vec<f64> = tp.moves[..first_wall]
            .iter()
            .filter(|m| m.kind == MoveKind::Cut)
            .map(|m| m.to.x)
            .collect();
        assert!(
            before.iter().all(|&x| x >= r + 0.5 - 0.02),
            "{strategy:?}: clearing came within {:?} of the wall",
            before.iter().copied().fold(f64::INFINITY, f64::min)
        );
        assert!(last_wall > first_wall, "{strategy:?}");
    }
}

#[test]
fn the_edge_pass_goes_round_once_at_full_depth_when_the_flutes_reach() {
    let r = Tool::default().diameter / 2.0;
    for strategy in [Strategy::Offset, Strategy::Raster, Strategy::Adaptive] {
        let p = Params {
            strategy,
            depth: 9.0,
            edge_allowance: 0.5,
            edge_full_depth: true,
            tool: Tool {
                flute_length: 12.0,
                ..Tool::default()
            },
            ..Params::default()
        };
        let tp = pocket(40.0, &p).unwrap();
        for z in [-3.0, -6.0] {
            assert!(
                nearest_left(&tp, z) >= r + 0.5 - 0.02,
                "{strategy:?} at {z}"
            );
        }
        assert!((nearest_left(&tp, -9.0) - r).abs() < 0.02, "{strategy:?}");
        // short flutes: round the edge at each pass instead
        let short = Params {
            tool: Tool {
                flute_length: 5.0,
                ..Tool::default()
            },
            ..p.clone()
        };
        let tp = pocket(40.0, &short).unwrap();
        assert!((nearest_left(&tp, -3.0) - r).abs() < 0.02, "{strategy:?}");
    }
}

#[test]
fn adaptive_can_leave_out_the_pass_round_the_edge() {
    let with = pocket(
        40.0,
        &Params {
            strategy: Strategy::Adaptive,
            ..Params::default()
        },
    )
    .unwrap();
    let without = pocket(
        40.0,
        &Params {
            strategy: Strategy::Adaptive,
            profile_pass: ProfilePass::None,
            ..Params::default()
        },
    )
    .unwrap();
    let wall = 4.0 * (40.0 - Tool::default().diameter);
    assert!(cut_length(&with) - cut_length(&without) > 0.9 * wall);
}

#[test]
fn raster_lines_run_at_the_angle_with_a_profile_pass_round_the_edge() {
    let p = Params {
        strategy: Strategy::Raster,
        raster_angle: 90.0,
        ..Params::default()
    };
    let tp = pocket(20.0, &p).unwrap();
    let b = cut_bounds(&tp);
    assert!(b.min.x >= 3.0 - 0.02 && b.max.x <= 17.0 + 0.02, "{b:?}");
    // the lines run along Y: most cuts keep x and change y
    let along_y = tp
        .moves
        .windows(2)
        .filter(|w| w[1].kind == MoveKind::Cut)
        .filter(|w| (w[0].to.x - w[1].to.x).abs() < 1e-9 && (w[0].to.y - w[1].to.y).abs() > 5.0)
        .count();
    assert!(along_y >= 4, "{along_y}");
    // the last cuts go round the edge, a tool radius in
    let last = tp
        .moves
        .iter()
        .rev()
        .find(|m| m.kind == MoveKind::Cut)
        .unwrap();
    let p = last.to.truncate();
    let on_edge = [p.x - 3.0, 17.0 - p.x, p.y - 3.0, 17.0 - p.y]
        .iter()
        .any(|d| d.abs() < 0.05);
    assert!(on_edge, "{p}");
}

#[test]
fn ramp_plunge_moves_zig_zag_down_from_the_level_above() {
    let p = Params {
        depth: 6.0,
        ramp_plunge_moves: true,
        ramp_distance: 4.0,
        entry: None,
        ..Params::default()
    };
    let tp = pocket(20.0, &p).unwrap();
    // straight down only to the surface or the level already cleared, never into
    // material
    let straight: Vec<f64> = tp
        .moves
        .windows(2)
        .filter(|w| {
            let (a, b) = (w[0].to, w[1].to);
            b.z < a.z - 1e-9 && a.truncate().distance(b.truncate()) < 1e-9
        })
        .map(|w| w[1].to.z)
        .collect();
    assert!(
        straight
            .iter()
            .all(|&z| z.abs() < 1e-9 || (z + 3.0).abs() < 1e-9),
        "{straight:?}"
    );
}

#[test]
fn the_allowance_leaves_a_wall_and_rings_run_the_chosen_way() {
    let p = Params {
        allowance: 1.0,
        ..Params::default()
    };
    let b = cut_bounds(&pocket(20.0, &p).unwrap());
    assert!(
        (b.min.x - 4.0).abs() < 0.02 && (b.max.x - 16.0).abs() < 0.02,
        "{b:?}"
    );
    for (direction, anticlockwise) in [(Direction::Climb, true), (Direction::Conventional, false)] {
        let tp = pocket(
            20.0,
            &Params {
                direction,
                ..Params::default()
            },
        )
        .unwrap();
        // the outer ring is the last one cut
        let start = tp
            .moves
            .iter()
            .rposition(|m| m.kind == MoveKind::Plunge)
            .unwrap();
        let ring: Vec<Point> = tp.moves[start..]
            .iter()
            .filter(|m| m.kind == MoveKind::Cut)
            .map(|m| m.to.truncate())
            .collect();
        let turn = cut_craft_core::geometry::winding(&ring);
        assert_eq!(turn > 0.0, anticlockwise, "{direction:?}");
    }
}

mod suggesting {
    use super::*;
    use cut_craft_core::testing::document_with;

    fn mill(d: f64) -> Tool {
        Tool {
            name: format!("{d} mm End Mill"),
            diameter: d,
            feed_rate: 1000.0 + 100.0 * d,
            ..Tool::default()
        }
    }

    fn by_hand(tool: Tool) -> Candidate {
        Candidate {
            tool,
            loaded: false,
        }
    }

    const CHANGES: Changes = Changes {
        by_hand: 2.0,
        by_changer: 0.25,
    };

    fn big_pocket() -> (Document, Params) {
        let doc = document_with(vec![square(100.0, 100.0, 120.0)]);
        let p = Params {
            tool: mill(2.0),
            depth: 6.0,
            ..Params::default()
        };
        (doc, p)
    }

    #[test]
    fn a_big_pocket_is_quicker_cleared_with_a_bigger_bit_first() {
        let (doc, p) = big_pocket();
        let s = suggest(
            &doc,
            &p,
            &[by_hand(mill(12.0)), by_hand(mill(6.0))],
            false,
            5000.0,
            CHANGES,
        )
        .unwrap();
        assert!(!s.clearing.is_empty(), "clears first");
        assert!(
            s.minutes < s.alone / 2.0,
            "{} vs {} alone",
            s.minutes,
            s.alone
        );
        assert!(
            s.clearing.windows(2).all(|w| w[0].diameter > w[1].diameter),
            "biggest first"
        );
    }

    #[test]
    fn smaller_bits_and_v_bits_are_never_suggested() {
        let (doc, p) = big_pocket();
        let v = Tool {
            name: "V".into(),
            shape: cut_craft_core::toolpath::ToolShape::VBit,
            diameter: 12.0,
            ..Tool::default()
        };
        let s = suggest(
            &doc,
            &p,
            &[by_hand(mill(1.0)), by_hand(v)],
            false,
            5000.0,
            CHANGES,
        )
        .unwrap();
        assert!(s.clearing.is_empty(), "{:?}", s.clearing);
        assert!((s.minutes - s.alone).abs() < 1e-9);
    }

    #[test]
    fn a_small_pocket_is_quickest_with_the_finishing_bit_alone() {
        // the 12 mm bit barely fits: a bit change costs more than it saves
        let doc = document_with(vec![square(100.0, 100.0, 14.0)]);
        let p = Params {
            tool: mill(6.0),
            depth: 3.0,
            ..Params::default()
        };
        let s = suggest(&doc, &p, &[by_hand(mill(12.0))], false, 5000.0, CHANGES).unwrap();
        assert!(s.clearing.is_empty(), "{:?}", s.clearing);
    }

    #[test]
    fn of_two_bits_the_same_size_the_one_in_the_tool_changer_is_picked() {
        let (doc, p) = big_pocket();
        let loaded = Candidate {
            tool: Tool {
                name: "12 mm in holder 3".into(),
                ..mill(12.0)
            },
            loaded: true,
        };
        let s = suggest(
            &doc,
            &p,
            &[by_hand(mill(12.0)), loaded],
            true,
            5000.0,
            CHANGES,
        )
        .unwrap();
        assert_eq!(s.clearing[0].name, "12 mm in holder 3");
    }
}
