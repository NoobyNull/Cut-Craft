//! Arcs: lines that are parts of circles posted as G2/G3, helices, and GRBL's check of their
//! centres.

use super::*;

/// `BASIC` with arc blocks, centres relative to the start.
fn arc_post(helical: bool) -> String {
    let mut blocks = String::from(
        "VAR ARC_CENTRE_I_INC_POSITION = [I|A|I|1.3]\nVAR ARC_CENTRE_J_INC_POSITION = [J|A|J|1.3]\n\
             begin FIRST_CW_ARC_MOVE\n\"G2 [X] [Y] [Z] [I] [J] [F]\"\nbegin CW_ARC_MOVE\n\"G2 [X] [Y] [Z] [I] [J]\"\n\
             begin FIRST_CCW_ARC_MOVE\n\"G3 [X] [Y] [Z] [I] [J] [F]\"\nbegin CCW_ARC_MOVE\n\"G3 [X] [Y] [Z] [I] [J]\"\n",
    );
    if helical {
        blocks.push_str(
            "begin CW_HELICAL_ARC_MOVE\n\"G2 [X] [Y] [Z] [I] [J] (helix)\"\n\
                 begin CCW_HELICAL_ARC_MOVE\n\"G3 [X] [Y] [Z] [I] [J] (helix)\"\n",
        );
    }
    format!("{BASIC}\n{blocks}")
}

/// A toolpath cutting through `points` from the first, 3 mm deep.
fn cutting(points: &[DVec3]) -> Toolpath {
    let start = points[0];
    let mut t = toolpath("Circle", "6 mm");
    t.moves = [
        mv(MoveKind::Rapid, start.x, start.y, 5.0),
        mv(MoveKind::Plunge, start.x, start.y, start.z),
    ]
    .into_iter()
    .chain(points[1..].iter().map(|p| Move {
        kind: MoveKind::Cut,
        to: *p,
    }))
    .chain([mv(MoveKind::Rapid, 0.0, 0.0, 5.0)])
    .collect();
    t
}

/// The outside of a 40 mm circle for a 6 mm tool, as a profile would cut it: the offset
/// outline, round to its start.
fn circle_profile() -> Vec<DVec3> {
    use cut_craft_core::geometry::{Path, Point};
    use cut_craft_core::polygon::{Corners, offset};
    let outline = offset(
        &[Path::circle(Point::new(100.0, 100.0), 20.0)],
        3.0,
        Corners::Round,
    );
    let mut points: Vec<DVec3> = outline[0]
        .flatten(cut_craft_core::polygon::TOLERANCE)
        .iter()
        .map(|p| p.extend(-3.0))
        .collect();
    if points.first() != points.last() {
        points.push(points[0]);
    }
    points
}

/// The XY path a G-code text cuts at Z below 0, arcs expanded into fine steps.
fn expand(lines: &[String]) -> Vec<cut_craft_core::geometry::Point> {
    use cut_craft_core::geometry::Point;
    let word = |line: &str, letter: char| {
        line.split_whitespace()
            .find_map(|w| w.strip_prefix(letter)?.parse::<f64>().ok())
    };
    let mut at = DVec3::ZERO;
    let mut path = vec![];
    for line in lines {
        let Some(code) = line.split_whitespace().next() else {
            continue;
        };
        if !matches!(code, "G0" | "G1" | "G2" | "G3") {
            continue;
        }
        let to = DVec3::new(
            word(line, 'X').unwrap_or(at.x),
            word(line, 'Y').unwrap_or(at.y),
            word(line, 'Z').unwrap_or(at.z),
        );
        if matches!(code, "G2" | "G3") {
            let centre =
                at.truncate() + Point::new(word(line, 'I').unwrap(), word(line, 'J').unwrap());
            let (from, end) = (at.truncate() - centre, to.truncate() - centre);
            let mut sweep = from.angle_to(end);
            if code == "G2" && sweep > 0.0 {
                sweep -= std::f64::consts::TAU;
            } else if code == "G3" && sweep < 0.0 {
                sweep += std::f64::consts::TAU;
            }
            for i in 0..=100 {
                let turn = Point::from_angle(sweep * f64::from(i) / 100.0);
                path.push(centre + turn.rotate(from));
            }
        } else if to.z < 0.0 {
            path.push(to.truncate());
        }
        at = to;
    }
    path
}

#[test]
fn a_circle_posts_as_a_few_arcs_that_retrace_its_lines() {
    use cut_craft_core::geometry::{Point, polyline_distance};
    let circle = circle_profile();
    assert!(circle.len() > 100, "{} lines", circle.len());
    let lines = render(&arc_post(false), vec![cutting(&circle)]);
    let arcs = lines
        .iter()
        .filter(|l| l.starts_with("G2 ") || l.starts_with("G3 "));
    assert!((2..=4).contains(&arcs.count()), "{lines:#?}");
    assert!(
        !lines
            .iter()
            .any(|l| l.starts_with("G1 ") && l.contains("Z-3.000") && !l.contains("F300")),
        "only the plunge is straight: {lines:#?}"
    );
    // within the tolerance both ways, plus the 3 places the post prints to
    let near = cut_craft_core::polygon::TOLERANCE + 0.002;
    let flat: Vec<Point> = circle.iter().map(|p| p.truncate()).collect();
    let traced = expand(&lines);
    for p in &flat {
        assert!(polyline_distance(&traced, *p) < near, "{p} is off the arcs");
    }
    for p in &traced {
        assert!(polyline_distance(&flat, *p) < near, "{p} is off the lines");
    }
}

#[test]
fn lines_that_are_not_arcs_stay_straight() {
    let lines = render(&arc_post(true), vec![toolpath("P", "T")]);
    assert!(
        lines.contains(&"G1 X25.400 Y0.000 Z-3.000 F1000.0".to_owned())
            && lines.contains(&"G1 X25.400 Y25.400 Z-3.000".to_owned()),
        "{lines:#?}"
    );
    assert!(
        !lines
            .iter()
            .any(|l| l.starts_with("G2 ") || l.starts_with("G3 ")),
        "{lines:#?}"
    );
}

#[test]
fn a_post_without_arc_blocks_writes_every_line() {
    let circle = circle_profile();
    let lines = render(BASIC, vec![cutting(&circle)]);
    let cuts = lines.iter().filter(|l| l.starts_with("G1 ")).count();
    assert_eq!(cuts, circle.len(), "the plunge and each line: {lines:#?}");
}

#[test]
fn a_ramp_round_a_circle_is_a_helix_when_the_post_has_helical_arcs() {
    // a quarter turn of radius 10 going down 2 mm, in 30 steps
    let helix: Vec<DVec3> = (0..=30)
        .map(|i| {
            let turn = f64::from(i) / 30.0 * std::f64::consts::FRAC_PI_2;
            DVec3::new(
                10.0 * turn.cos(),
                10.0 * turn.sin(),
                -1.0 - 2.0 * f64::from(i) / 30.0,
            )
        })
        .collect();
    let lines = render(&arc_post(true), vec![cutting(&helix)]);
    assert!(
        lines.contains(&"G3 X0.000 Y10.000 Z-3.000 I-10.000 J0.000 (helix)".to_owned()),
        "{lines:#?}"
    );
    let flat = render(&arc_post(false), vec![cutting(&helix)]);
    assert!(!flat.iter().any(|l| l.starts_with("G3 ")), "{flat:#?}");
}

#[test]
fn a_helix_entry_goes_down_in_helical_arcs_at_the_plunge_rate() {
    // an entry helix: a quarter turn of radius 10 down 2 mm, gone down at the plunge rate
    let mut t = cutting(&[DVec3::new(10.0, 0.0, 0.0), DVec3::new(10.0, 0.0, -3.0)]);
    let helix = (1..=30).map(|i| {
        let turn = f64::from(i) / 30.0 * std::f64::consts::FRAC_PI_2;
        mv(
            MoveKind::Plunge,
            10.0 * turn.cos(),
            10.0 * turn.sin(),
            -1.0 - 2.0 * f64::from(i) / 30.0,
        )
    });
    t.moves = [
        mv(MoveKind::Rapid, 10.0, 0.0, 5.0),
        mv(MoveKind::Plunge, 10.0, 0.0, -1.0),
    ]
    .into_iter()
    .chain(helix)
    .chain([mv(MoveKind::Rapid, 0.0, 10.0, 5.0)])
    .collect();
    let plunge = t.tool.plunge_rate;
    let lines = render(&arc_post(true), vec![t]);
    let arc = lines
        .iter()
        .find(|l| l.starts_with("G3 X0.000 Y10.000 Z-3.000"))
        .unwrap_or_else(|| panic!("{lines:#?}"));
    assert!(arc.ends_with("(helix)"), "{arc}");
    // the feed is set to the plunge rate before it, not the cutting feed
    let at = lines.iter().position(|l| l == arc).unwrap();
    let fed = lines[..at]
        .iter()
        .rev()
        .find_map(|l| l.split_whitespace().find(|w| w.starts_with('F')))
        .unwrap();
    assert_eq!(fed, format!("F{plunge:.1}"), "{lines:#?}");
}

/// How far, in mm, the end of each arc in `lines` (in `post`'s units) is from its centre
/// further than its start is, with the arc's radius: what GRBL compares (error 33).
fn arc_mismatches(lines: &[String], inches: bool) -> Vec<(f64, f64)> {
    let unit = if inches { MM_PER_INCH } else { 1.0 };
    let word = |line: &str, letter: char| {
        line.split_whitespace()
            .find_map(|w| w.strip_prefix(letter)?.parse::<f64>().ok())
            .map(|v| v * unit)
    };
    let (mut x, mut y) = (0.0, 0.0);
    let mut out = vec![];
    for line in lines {
        let to_x = word(line, 'X').unwrap_or(x);
        let to_y = word(line, 'Y').unwrap_or(y);
        if line.starts_with("G2 ") || line.starts_with("G3 ") {
            let c = (
                x + word(line, 'I').unwrap_or(0.0),
                y + word(line, 'J').unwrap_or(0.0),
            );
            let r0 = f64::hypot(x - c.0, y - c.1);
            let r1 = f64::hypot(to_x - c.0, to_y - c.1);
            out.push(((r1 - r0).abs(), r0));
        }
        if line.starts_with('G') {
            (x, y) = (to_x, to_y);
        }
    }
    out
}

#[test]
fn every_arc_passes_grbls_check_that_its_ends_are_the_same_distance_from_its_centre() {
    // small arcs at awkward places, where rounding to the printed places matters most:
    // GRBL stops a job with error 33 when the ends differ by over 0.005 mm and 0.1%
    let mut seed = 12_345_u64;
    let mut next = || {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        #[expect(clippy::cast_precision_loss)]
        let r = (seed >> 11) as f64 / (1_u64 << 53) as f64;
        r
    };
    let mut toolpaths = vec![];
    for k in 0..3000 {
        let (cx, cy) = (10.0 + 80.0 * next(), 10.0 + 80.0 * next());
        let radius = 0.2 + 2.0 * next();
        let (start, sweep) = (std::f64::consts::TAU * next(), 2.4 + 0.7 * next());
        let way = if k % 2 == 0 { 1.0 } else { -1.0 };
        let points: Vec<DVec3> = (0..=24)
            .map(|i| {
                let a = start + way * sweep * f64::from(i) / 24.0;
                DVec3::new(cx + radius * a.cos(), cy + radius * a.sin(), -1.0)
            })
            .collect();
        toolpaths.push(cutting(&points));
    }
    for (name, text) in BUILT_IN {
        let post = Post::parse(text).unwrap();
        if post.arc_limits(false).is_none() {
            continue;
        }
        let lines = render_with(
            &post,
            cut_craft_core::document::Job::default(),
            toolpaths.clone(),
        );
        let arcs = arc_mismatches(&lines, post.inches);
        assert!(arcs.len() >= 3000, "{name}: {} arcs", arcs.len());
        for (off, radius) in arcs {
            assert!(
                off <= 0.005 || (off <= 0.5 && off <= 0.001 * radius),
                "{name}: an arc of radius {radius:.4} mm ends {off:.4} mm off its circle"
            );
        }
    }
}

#[test]
fn arcs_turning_clockwise_are_g2_with_the_centre_from_their_start() {
    // a half turn of radius 10 round 50, 50 from its right, clockwise
    let half: Vec<DVec3> = (0..=40)
        .map(|i| {
            let turn = -f64::from(i) / 40.0 * std::f64::consts::PI;
            DVec3::new(50.0 + 10.0 * turn.cos(), 50.0 + 10.0 * turn.sin(), -3.0)
        })
        .collect();
    let lines = render(&arc_post(false), vec![cutting(&half)]);
    assert!(
        lines.contains(&"G2 X40.000 Y50.000 Z-3.000 I-10.000 J0.000 F1000.0".to_owned()),
        "{lines:#?}"
    );
}

#[test]
fn plunges_rapid_down_to_the_plunge_gap_first() {
    use cut_craft_core::document::Job;
    let job = Job {
        plunge: Some(1.0),
        ..Job::default()
    };
    let lines = render_for(BASIC, job, vec![toolpath("P", "T")]);
    let at = lines
        .iter()
        .position(|l| l == "G0 X0.000 Y0.000 Z5.000")
        .unwrap();
    assert_eq!(
        lines[at + 1..at + 3],
        ["G0 X0.000 Y0.000 Z1.000", "G1 X0.000 Y0.000 Z-3.000 F300.0"]
    );
    // with the gap at the clearance (older jobs) nothing changes
    let lines = render(BASIC, vec![toolpath("P", "T")]);
    assert_eq!(lines[5], "G1 X0.000 Y0.000 Z-3.000 F300.0");
}

#[test]
fn the_tool_starts_and_ends_at_home() {
    use cut_craft_core::document::{Home, Job, ZZero};
    let job = Job {
        home: Some(Home {
            x: 10.0,
            y: 20.0,
            z: 30.0,
        }),
        z_zero: ZZero::MachineBed,
        ..Job::default()
    };
    let g = render_default(job, vec![toolpath("P", "T")]);
    let lines: Vec<&str> = g.lines().collect();
    let header = lines.iter().position(|l| *l == "G0 Z50.000").unwrap();
    assert_eq!(lines[header + 1], "G0 X10.000 Y20.000", "{g}");
    assert_eq!(
        lines[lines.len() - 4..],
        ["G0 Z50.000", "M5", "G0 X10.000 Y20.000", "M30"],
        "30 above the 20 mm material, zeroed on the bed: {g}"
    );
}

#[test]
fn the_default_post_writes_arcs() {
    let g = render_default(
        cut_craft_core::document::Job::default(),
        vec![cutting(&circle_profile())],
    );
    assert!(g.contains("\nG2 ") || g.contains("\nG3 "), "{g}");
    assert!(g.lines().count() < 40, "{g}");
}
