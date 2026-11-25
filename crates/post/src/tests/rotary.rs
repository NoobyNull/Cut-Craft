//! Rotary jobs: Y wrapped round the cylinder as A, how far A may turn, inverse-time feeds and
//! bits held off the axis.

use super::*;

#[test]
fn wrapping_y_prints_it_as_degrees_round_the_cylinder() {
    let post = BASIC.replace(
        "begin HEADER\n",
        "ROTARY_WRAP_Y = \"A\"\nbegin HEADER\n\"([WRAP_DIA] [Z_ORIGIN])\"\n",
    );
    let lines = render_for(&post, hundred_round(), vec![toolpath("P", "T")]);
    assert_eq!(lines[0], "(31.831 Turning Axis)");
    assert!(
        lines.contains(&"G1 X25.400 A91.440 Z12.915".to_owned()),
        "25.4 mm round is 91.44 degrees, 3 mm into a 15.915 radius: {lines:#?}"
    );
}

#[test]
fn a_rotary_job_zeroed_on_its_surface_or_its_axis_gets_the_right_z() {
    use cut_craft_core::document::ZZero;
    let post = BASIC.replace(
        "begin HEADER\n",
        "ROTARY_WRAP_Y = \"A\"\nbegin HEADER\n\"([Z_ORIGIN])\"\n",
    );
    let axis = render_for(&post, hundred_round(), vec![toolpath("P", "T")]);
    let surface = render_for(
        &post,
        cut_craft_core::document::Job {
            z_zero: ZZero::MaterialSurface,
            ..hundred_round()
        },
        vec![toolpath("P", "T")],
    );
    assert_eq!(axis[0], "(Turning Axis)");
    assert_eq!(surface[0], "(Material Top)");
    // 3 mm deep: below the top of the cylinder, or 3 mm short of its 15.915 radius
    assert!(
        surface.contains(&"G1 X25.400 A91.440 Z-3.000".to_owned()),
        "{surface:#?}"
    );
    assert!(
        axis.contains(&"G1 X25.400 A91.440 Z12.915".to_owned()),
        "{axis:#?}"
    );
    // the 5 mm clearance stays 5 mm above the surface either way
    assert!(surface.contains(&"G0 Z5.000".to_owned()), "{surface:#?}");
    assert!(axis.contains(&"G0 Z20.915".to_owned()), "{axis:#?}");
}

#[test]
fn inverse_time_feeds_are_the_rate_over_the_move_length() {
    let post = BASIC
            .replace(
                "begin HEADER\n",
                "ROTARY_WRAP_Y = \"A\"\nINVERSE_TIME_MODE = YES\nVAR INVERSE_TIME = [FI|A|F|1.3]\nbegin HEADER\n",
            )
            .replace("[Z] [F]\"", "[Z] [FI]\"");
    let lines = render_for(&post, hundred_round(), vec![toolpath("P", "T")]);
    assert!(
        lines.contains(&"G1 X0.000 A0.000 Z12.915 F37.500".to_owned()),
        "an 8 mm plunge at 300 mm/min takes 1/37.5 minutes: {lines:#?}"
    );
    assert!(
        lines.contains(&"G1 X25.400 A0.000 Z12.915 F39.370".to_owned()),
        "{lines:#?}"
    );
}

/// A post wrapping Y onto A for `hundred_round`, told the machine's rotary.
fn winding_post(axis: cut_craft_core::rotary_axis::RotaryAxis) -> Post {
    let post = BASIC
            .replace(
                "begin HEADER\n",
                "ROTARY_WRAP_Y = \"A\"\nINVERSE_TIME_MODE = YES\nVAR INVERSE_TIME = [FI|A|F|1.3]\nVAR Y_HOME_POSITION = [YH|A|Y|1.3]\nbegin HEADER\n",
            )
            .replace("[Z] [F]\"", "[Z] [FI]\"")
            .replace("\"M5\"\n\"M30\"", "\"G0 [YH]\"\n\"M5\"\n\"M30\"");
    Post::parse(&post).unwrap().with_rotary(axis)
}

/// Two rings round `hundred_round`, each one whole turn (100 mm round), 1 mm deep,
/// at X0 then X10.
fn two_rings() -> Toolpath {
    Toolpath {
        moves: vec![
            mv(MoveKind::Rapid, 0.0, 0.0, 5.0),
            mv(MoveKind::Plunge, 0.0, 0.0, -1.0),
            mv(MoveKind::Cut, 0.0, 50.0, -1.0),
            mv(MoveKind::Cut, 0.0, 100.0, -1.0),
            mv(MoveKind::Rapid, 0.0, 100.0, 5.0),
            mv(MoveKind::Rapid, 10.0, 0.0, 5.0),
            mv(MoveKind::Plunge, 10.0, 0.0, -1.0),
            mv(MoveKind::Cut, 10.0, 50.0, -1.0),
            mv(MoveKind::Cut, 10.0, 100.0, -1.0),
            mv(MoveKind::Rapid, 10.0, 100.0, 5.0),
        ],
        ..toolpath("Rings", "T")
    }
}

/// A line of `lines` starts with `start`.
fn has(lines: &[String], start: &str) -> bool {
    lines.iter().any(|l| l.starts_with(start))
}

/// Every A a file writes.
fn a_words(lines: &[String]) -> Vec<f64> {
    lines
        .iter()
        .flat_map(|l| l.split_whitespace())
        .filter_map(|w| w.strip_prefix('A')?.parse().ok())
        .collect()
}

#[test]
fn a_free_rotary_keeps_turning_the_same_way_ring_after_ring() {
    let axis = cut_craft_core::rotary_axis::RotaryAxis::default();
    let lines = render_with(&winding_post(axis), hundred_round(), vec![two_rings()]);
    // the second ring starts where the first ended (360) and carries on to 720
    assert!(has(&lines, "G1 X10.000 A720.000 Z14.915"), "{lines:#?}");
    // its A never ran back between the rings
    let a = a_words(&lines);
    let cutting = &a[..a.len() - 1];
    assert!(cutting.windows(2).all(|w| w[1] >= w[0] - 1e-9), "{a:?}");
    // and at the end it goes home to the start's place nearest where it is: 720
    assert_eq!(a.last(), Some(&720.0), "{lines:#?}");
    // told nothing of the machine, the post writes A as the job unrolls, as before
    let post = Post {
        rotary: None,
        ..winding_post(axis)
    };
    let lines = render_with(&post, hundred_round(), vec![two_rings()]);
    assert!(
        a_words(&lines).iter().all(|a| (0.0..=360.0).contains(a)),
        "{lines:#?}"
    );
}

#[test]
fn at_its_limit_a_rotary_cuts_the_next_pass_backwards_or_spins_back() {
    use cut_craft_core::rotary_axis::{AtLimit, RotaryAxis, Turning};
    let limited = |at_limit| RotaryAxis {
        turning: Turning::Unwind,
        turns: 1.0,
        at_limit,
        ..RotaryAxis::default()
    };
    let both = render_with(
        &winding_post(limited(AtLimit::CutBothWays)),
        hundred_round(),
        vec![two_rings()],
    );
    let a = a_words(&both);
    assert!(a.iter().all(|a| (-360.0..=360.0).contains(a)), "{both:#?}");
    // the second ring goes straight down at 360 and cuts back round to 0
    let at = both
        .iter()
        .position(|l| l.starts_with("G1 X10.000 A360.000 Z14.915"))
        .unwrap();
    assert!(
        both[at + 1].starts_with("G1 X10.000 A180.000 Z14.915"),
        "{both:#?}"
    );
    assert!(
        both[at + 2].starts_with("G1 X10.000 A0.000 Z14.915"),
        "{both:#?}"
    );

    let back = render_with(
        &winding_post(limited(AtLimit::SpinBack)),
        hundred_round(),
        vec![two_rings()],
    );
    assert!(
        a_words(&back).iter().all(|a| (-360.0..=360.0).contains(a)),
        "{back:#?}"
    );
    // lifted clear before A spins back to 0, then the second ring cut forwards again
    let spin = back
        .iter()
        .position(|l| l == "G0 X10.000 A0.000 Z20.915")
        .unwrap();
    assert!(back[spin - 1].ends_with("A360.000 Z20.915"), "{back:#?}");
    assert!(has(&back, "G1 X10.000 A360.000 Z14.915"), "{back:#?}");
    // and home at the very start, unwound
    assert_eq!(a_words(&back).last(), Some(&0.0));
}

#[test]
fn a_machine_saved_before_a_can_turn_writes_a_within_one_turn_as_before() {
    let axis = cut_craft_core::rotary_axis::RotaryAxis::saved_before();
    let lines = render_with(&winding_post(axis), hundred_round(), vec![two_rings()]);
    assert!(
        a_words(&lines).iter().all(|a| (0.0..=360.0).contains(a)),
        "{lines:#?}"
    );
    assert_eq!(a_words(&lines).last(), Some(&0.0), "{lines:#?}");
}

#[test]
fn a_turns_only_as_high_as_the_file_lifts_even_when_a_pass_is_reached_quickly() {
    use cut_craft_core::rotary_axis::RotaryAxis;
    // a ring, then lifted 30 above the top (clear of a board's edges as it swings), over to
    // half way round, and down at the rapid rate to just above the next pass before plunging
    let quick = Toolpath {
        moves: vec![
            mv(MoveKind::Rapid, 0.0, 0.0, 30.0),
            mv(MoveKind::Plunge, 0.0, 0.0, -1.0),
            mv(MoveKind::Cut, 0.0, 50.0, -1.0),
            mv(MoveKind::Cut, 0.0, 100.0, -1.0),
            mv(MoveKind::Rapid, 0.0, 100.0, 30.0),
            mv(MoveKind::Rapid, 10.0, 50.0, 30.0),
            mv(MoveKind::Rapid, 10.0, 50.0, 0.0),
            mv(MoveKind::Plunge, 10.0, 50.0, -1.0),
            mv(MoveKind::Cut, 10.0, 100.0, -1.0),
            mv(MoveKind::Rapid, 10.0, 100.0, 30.0),
        ],
        ..toolpath("Rings", "T")
    };
    let lines = render_with(
        &winding_post(RotaryAxis::default()),
        hundred_round(),
        vec![quick],
    );
    // A goes on round to 540 at the file's 30 above the top, not the job's 5 mm clearance
    let turn = lines.iter().position(|l| l.contains("A540.000")).unwrap();
    assert_eq!(lines[turn], "G0 X10.000 A540.000 Z45.915", "{lines:#?}");
}

#[test]
fn backlash_is_taken_up_each_time_a_turns_the_other_way() {
    use cut_craft_core::rotary_axis::{RotaryAxis, Turning};
    let axis = RotaryAxis {
        turning: Turning::Unwind,
        turns: 1.0,
        backlash: 0.5,
        ..RotaryAxis::default()
    };
    let lines = render_with(&winding_post(axis), hundred_round(), vec![two_rings()]);
    // going back round, every A is half a degree further back
    assert!(has(&lines, "G1 X10.000 A179.500 Z14.915"), "{lines:#?}");
    assert!(has(&lines, "G1 X10.000 A-0.500 Z14.915"), "{lines:#?}");
}

/// One side of `hundred_round` cut with A held at 90° (25 mm round): back and forth along
/// X, 2 mm down, with the bit held 4 mm to one side of the axis and then 4 mm to the other.
/// Its moves are where the tip cuts, as a toolpath keeps them.
#[test]
fn a_rotary_toolpath_whose_across_list_does_not_match_its_moves_is_refused() {
    use cut_craft_core::rotary_axis::RotaryAxis;
    // something shortened the moves and left the list of how far the bit is held off the
    // axis as it was: every move after the first dropped one would be written with
    // another move's place, so the post must not write it at all
    let mut face = one_face();
    face.across.pop();
    let mut doc = Document {
        job: hundred_round(),
        ..Document::default()
    };
    let mut c = Change::builder(&doc);
    c.add_toolpath(face);
    doc.apply(&c.finish()).unwrap();
    let post = Post::default_for(&doc.job).with_rotary(RotaryAxis::default());
    let file = File {
        path: std::path::Path::new("/jobs/face.nc"),
        job: None,
        made: chrono::NaiveDateTime::default(),
        slots: &[],
        notes: &[],
    };
    let refused = post.render(&doc, &file).unwrap_err();
    assert!(refused.contains("Face"), "{refused}");
}

fn one_face() -> Toolpath {
    use cut_craft_core::rotary_axis::tip_of;
    let radius = 50.0 / std::f64::consts::PI;
    let held = [
        (MoveKind::Rapid, 0.0, 5.0, -4.0),
        (MoveKind::Plunge, 0.0, -2.0, -4.0),
        (MoveKind::Cut, 10.0, -2.0, -4.0),
        (MoveKind::Cut, 10.0, -2.0, 4.0),
        (MoveKind::Cut, 0.0, -2.0, 4.0),
        (MoveKind::Rapid, 0.0, 5.0, 4.0),
    ];
    let moves = held
        .iter()
        .map(|&(kind, x, z, across)| {
            let (round, z) = tip_of(radius, 25.0, z, across);
            mv(kind, x, round, z)
        })
        .collect();
    Toolpath {
        moves,
        across: held.iter().map(|h| h.3).collect(),
        ..toolpath("Face", "T")
    }
}

#[test]
fn a_bit_held_off_the_axis_is_written_as_a_held_still_and_the_across_axis_moving() {
    use cut_craft_core::rotary_axis::RotaryAxis;
    let job = hundred_round();
    let axis = RotaryAxis {
        centre_across: 5.0,
        ..RotaryAxis::default()
    };
    let post = Post::default_for(&job).with_rotary(axis);
    let lines = render_with(&post, job, vec![one_face()]);
    let words = |letter: char| -> Vec<f64> {
        lines
            .iter()
            .filter(|l| l.starts_with("G1"))
            .flat_map(|l| l.split_whitespace())
            .filter_map(|w| w.strip_prefix(letter)?.parse().ok())
            .collect()
    };
    // A turns to the side and holds still while the bit cuts it
    assert!(
        words('A').iter().all(|a| (a - 90.0).abs() < 1e-9),
        "{lines:#?}"
    );
    // the bit moves across, 4 mm each side of the axis (which is 5 mm across from zero)
    assert!(has(&lines, "G0 X0.000 A90.000 Y1.000"), "{lines:#?}");
    assert!(words('Y').contains(&9.0), "{lines:#?}");
    // the cut straight across at X10 (Y1 to Y9) is written on its own, before the cut
    // back along X, not folded into one diagonal with it
    let across = lines
        .iter()
        .position(|l| l.starts_with("G1") && l.contains("Y9.000") && !l.contains("X0.000"))
        .unwrap_or_else(|| panic!("no cut across: {lines:#?}"));
    let back = lines
        .iter()
        .position(|l| l.starts_with("G1") && l.contains("X0.000"))
        .unwrap();
    assert!(across < back, "{lines:#?}");
    // and 2 mm below the top straight over the axis (Z zero on the axis)
    assert!(
        words('Z').iter().all(|z| (z - 13.915).abs() < 1e-3),
        "{lines:#?}"
    );
    // turning the other way, positive across is the other way along Y
    let job = hundred_round();
    let post = Post::default_for(&job).with_rotary(RotaryAxis {
        clockwise: false,
        ..axis
    });
    let lines = render_with(&post, job, vec![one_face()]);
    assert!(has(&lines, "G0 X0.000 A90.000 Y9.000"), "{lines:#?}");
    // a post that can't move across says so rather than cut in the wrong place
    let job = hundred_round();
    let flat = BASIC.replace("begin HEADER\n", "ROTARY_WRAP_Y = \"A\"\nbegin HEADER\n");
    let mut doc = Document {
        job,
        ..Document::default()
    };
    let mut c = Change::builder(&doc);
    c.add_toolpath(one_face());
    doc.apply(&c.finish()).unwrap();
    let why = Post::parse(&flat)
        .unwrap()
        .render(
            &doc,
            &File {
                path: std::path::Path::new("/jobs/face.nc"),
                job: None,
                made: chrono::NaiveDateTime::default(),
                slots: &[],
                notes: &[],
            },
        )
        .unwrap_err();
    assert!(why.contains("off the turning axis"), "{why}");
}

#[test]
fn inverse_time_on_a_rotary_is_timed_where_the_bit_cuts_and_within_a_top_speed() {
    use cut_craft_core::rotary_axis::RotaryAxis;
    let radius = 50.0 / std::f64::consts::PI;
    let half_way = Toolpath {
        moves: vec![
            mv(MoveKind::Rapid, 0.0, 0.0, 5.0),
            mv(MoveKind::Plunge, 0.0, 0.0, -radius / 2.0),
            mv(MoveKind::Cut, 0.0, 10.0, -radius / 2.0),
        ],
        ..toolpath("Half way in", "T")
    };
    let fi = |max_rate| {
        let axis = RotaryAxis {
            max_rate,
            ..RotaryAxis::default()
        };
        let lines = render_with(&winding_post(axis), hundred_round(), vec![half_way.clone()]);
        lines
            .iter()
            .find(|l| l.contains("A36.000"))
            .and_then(|l| l.rsplit_once('F'))
            .map(|(_, f)| f.to_owned())
            .unwrap()
    };
    // 10 mm round the surface is 5 mm half way to the axis: 1/200 minute at 1000 mm/min
    assert_eq!(fi(0.0), "200.000");
    // but 36 degrees at no more than 360 a minute takes a tenth of a minute
    assert_eq!(fi(360.0), "10.000");
}

#[test]
fn a_pass_at_several_depths_is_never_cut_backwards_deepest_first() {
    use cut_craft_core::rotary_axis::{AtLimit, RotaryAxis, Turning};
    // a ring at X0, then at X10 a pass 1 mm down one turn round and 2 mm down the next,
    // with no lift between
    let mut moves = two_rings().moves[..5].to_vec();
    moves.extend([
        mv(MoveKind::Rapid, 10.0, 0.0, 5.0),
        mv(MoveKind::Plunge, 10.0, 0.0, -1.0),
        mv(MoveKind::Cut, 10.0, 50.0, -1.0),
        mv(MoveKind::Cut, 10.0, 100.0, -1.0),
        mv(MoveKind::Plunge, 10.0, 100.0, -2.0),
        mv(MoveKind::Cut, 10.0, 150.0, -2.0),
        mv(MoveKind::Cut, 10.0, 200.0, -2.0),
        mv(MoveKind::Rapid, 10.0, 200.0, 5.0),
    ]);
    let deeper = Toolpath {
        moves,
        ..toolpath("Two depths", "T")
    };
    let axis = RotaryAxis {
        turning: Turning::Unwind,
        turns: 1.0,
        at_limit: AtLimit::CutBothWays,
        ..RotaryAxis::default()
    };
    let lines = render_with(&winding_post(axis), hundred_round(), vec![deeper]);
    assert!(
        a_words(&lines).iter().all(|a| (-360.0..=360.0).contains(a)),
        "{lines:#?}"
    );
    // it spins back clear of the material, then cuts 1 mm down before 2 mm down
    assert!(has(&lines, "G0 X10.000 A-360.000 Z20.915"), "{lines:#?}");
    let first_cut = lines.iter().find(|l| l.starts_with("G1 X10.000")).unwrap();
    assert!(first_cut.contains("Z14.915"), "{lines:#?}");
}

#[test]
fn a_pass_longer_than_a_can_turn_is_still_written_as_near_the_middle_as_it_goes() {
    use cut_craft_core::rotary_axis::{RotaryAxis, Turning};
    // half a turn each way, but each ring is a whole turn: the checks before cutting warn
    // and ask; the file cuts each ring as little past the limit as it can
    let half = RotaryAxis {
        turning: Turning::Unwind,
        turns: 0.5,
        ..RotaryAxis::default()
    };
    let lines = render_with(&winding_post(half), hundred_round(), vec![two_rings()]);
    let a = a_words(&lines);
    assert!(a.iter().all(|a| (-360.0..=360.0).contains(a)), "{lines:#?}");
    assert!(has(&lines, "G1 X10.000"), "both rings are cut: {lines:#?}");
}

#[test]
fn a_rotary_file_cutting_only_over_the_axis_never_moves_the_across_axis() {
    // zeroed on the axis by hand along Y, a plain wrapped file must leave Y where it is
    let post = BASIC
        .replace(
            "begin HEADER\n",
            "ROTARY_WRAP_Y = \"A\"\nVAR ACROSS_POSITION = [ACROSS|C|Y|1.3]\nbegin HEADER\n",
        )
        .replace("[Z] [F]\"", "[Z] [ACROSS] [F]\"");
    let lines = render_for(&post, hundred_round(), vec![two_rings()]);
    assert!(
        lines
            .iter()
            .all(|l| !l.split_whitespace().any(|w| w.starts_with('Y'))),
        "{lines:#?}"
    );
    // the default rotary post too
    let job = hundred_round();
    let lines = render_with(&Post::default_for(&job), job, vec![two_rings()]);
    assert!(
        lines
            .iter()
            .all(|l| !l.split_whitespace().any(|w| w.starts_with('Y'))),
        "{lines:#?}"
    );
}

#[test]
fn a_wrapping_post_needs_a_rotary_job() {
    let post = BASIC.replace("begin HEADER\n", "ROTARY_WRAP_Y = \"A\"\nbegin HEADER\n");
    let mut doc = Document::default();
    let mut c = Change::builder(&doc);
    c.add_toolpath(toolpath("P", "T"));
    let change = c.finish();
    doc.apply(&change).unwrap();
    let file = File {
        path: std::path::Path::new("/out.nc"),
        job: None,
        made: chrono::NaiveDateTime::default(),
        slots: &[],
        notes: &[],
    };
    let why = Post::parse(&post).unwrap().render(&doc, &file).unwrap_err();
    assert!(why.contains("rotary"), "{why}");
}

#[test]
fn inverse_time_needs_wrapping() {
    let post = BASIC.replace("begin HEADER\n", "INVERSE_TIME_MODE = YES\nbegin HEADER\n");
    let why = Post::parse(&post).unwrap_err();
    assert!(why.contains("INVERSE_TIME_MODE"), "{why}");
}

#[test]
fn a_rapid_turning_a_under_the_bit_lifts_clear_of_square_corners_first() {
    use cut_craft_core::document::Blank;
    // a 31.8 mm square on its axis: its corners swing 22.5 mm out, 6.6 past its top, so the
    // job's 5 mm clearance isn't enough for A to turn under the bit
    let mut job = hundred_round();
    if let Some(r) = job.rotary.as_mut() {
        r.blank = Blank::Square;
    }
    let gap = job.turning_gap();
    assert!(gap > job.clearance + 6.0, "{gap}");
    let hop = Toolpath {
        moves: vec![
            mv(MoveKind::Rapid, 0.0, 0.0, 5.0),
            mv(MoveKind::Plunge, 0.0, 0.0, -1.0),
            mv(MoveKind::Cut, 0.0, 10.0, -1.0),
            mv(MoveKind::Rapid, 0.0, 10.0, 5.0),
            // over to 216 degrees at the clearance: under a corner
            mv(MoveKind::Rapid, 0.0, 60.0, 5.0),
            mv(MoveKind::Plunge, 0.0, 60.0, -1.0),
            mv(MoveKind::Cut, 0.0, 70.0, -1.0),
            mv(MoveKind::Rapid, 0.0, 70.0, 5.0),
        ],
        ..toolpath("Hop", "T")
    };
    let post =
        Post::parse(&BASIC.replace("begin HEADER\n", "ROTARY_WRAP_Y = \"A\"\nbegin HEADER\n"))
            .unwrap();
    let lines = render_with(&post, job, vec![hop]);
    let z = |l: &str| -> Option<f64> {
        l.split_whitespace()
            .find_map(|w| w.strip_prefix('Z')?.parse().ok())
    };
    // the axis is the Z zero here: the corners need the bit above radius + gap
    let clear = 100.0 / std::f64::consts::PI / 2.0 + gap - 1e-3;
    let turn = lines
        .iter()
        .position(|l| l.contains("A216.000"))
        .expect("A turns to 216");
    assert!(
        z(&lines[turn]).is_some_and(|z| z >= clear),
        "A turns with the bit clear of the corners: {lines:#?}"
    );
    // and comes back down to the clearance before plunging, as the toolpath asked
    assert!(
        lines[turn + 1..]
            .iter()
            .any(|l| l.contains("A216.000") && z(l).is_some_and(|z| (z - 20.915).abs() < 1e-3)),
        "{lines:#?}"
    );
}

#[test]
fn a_round_blank_turns_a_at_the_clearance_as_before() {
    let hop = Toolpath {
        moves: vec![
            mv(MoveKind::Rapid, 0.0, 0.0, 5.0),
            mv(MoveKind::Rapid, 0.0, 60.0, 5.0),
        ],
        ..toolpath("Hop", "T")
    };
    let post =
        Post::parse(&BASIC.replace("begin HEADER\n", "ROTARY_WRAP_Y = \"A\"\nbegin HEADER\n"))
            .unwrap();
    let lines = render_with(&post, hundred_round(), vec![hop]);
    let turns: Vec<&String> = lines.iter().filter(|l| l.contains("A216.000")).collect();
    assert_eq!(turns.len(), 1, "no lift: {lines:#?}");
    assert!(turns[0].contains("Z20.915"), "{lines:#?}");
}
