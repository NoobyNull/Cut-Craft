//! The carving styles: Light passes, Stay down (round the shape or the shortest path) and
//! Lift between cuts (shallow in or depth out), and how few moves each makes.

use super::*;
use cut_craft_core::testing::made_toolpath;
use cut_craft_core::toolpath::Toolpath;

/// A five-pointed star round (`x`, `y`), its points `r` from the middle.
fn star(x: f64, y: f64, r: f64) -> Path {
    let points: Vec<Point> = (0..10)
        .map(|i| {
            let a = std::f64::consts::FRAC_PI_2 + f64::from(i) * std::f64::consts::PI / 5.0;
            let rr = if i % 2 == 0 { r } else { r * 0.381_966 };
            Point::new(x + rr * a.cos(), y + rr * a.sin())
        })
        .collect();
    Path::polyline(&points, true)
}

fn styled(style: Style) -> Params {
    Params {
        style,
        ..Params::default()
    }
}

fn carve(paths: Vec<Path>, p: &Params) -> Toolpath {
    made_toolpath::<VcarveEngraveToolpath>(paths, p).unwrap()
}

/// The star the samples carve: about 4.8 mm deep with the default 90° bit, so two passes at
/// its 3 mm pass depth.
fn a_star() -> Vec<Path> {
    vec![star(30.0, 30.0, 12.65)]
}

fn cuts(t: &Toolpath) -> usize {
    t.moves.iter().filter(|m| m.kind == MoveKind::Cut).count()
}

fn lifts(t: &Toolpath) -> usize {
    t.moves
        .windows(2)
        .filter(|w| w[1].kind == MoveKind::Rapid && w[1].to.z > w[0].to.z + 1e-9)
        .count()
}

/// The first pass's level: where most cuts are, on its floor.
fn first_pass(t: &Toolpath) -> f64 {
    let mut levels: std::collections::HashMap<i64, usize> = std::collections::HashMap::new();
    for m in t.moves.iter().filter(|m| m.kind == MoveKind::Cut) {
        #[expect(clippy::cast_possible_truncation, reason = "depths of a few mm")]
        let key = (m.to.z * 1e4).round() as i64;
        *levels.entry(key).or_default() += 1;
    }
    let (&level, _) = levels
        .iter()
        .filter(|&(&k, _)| k < -10)
        .max_by_key(|&(_, n)| *n)
        .expect("cuts below the surface");
    #[expect(clippy::cast_precision_loss, reason = "depths of a few mm")]
    let level = level as f64 / 1e4;
    level
}

/// The cut points from the first one below the first pass's level on.
fn below_the_first_pass(t: &Toolpath) -> Vec<DVec3> {
    let level = first_pass(t);
    let cut: Vec<DVec3> = t
        .moves
        .iter()
        .filter(|m| m.kind == MoveKind::Cut)
        .map(|m| m.to)
        .collect();
    let from = cut
        .iter()
        .position(|q| q.z < level - 1e-3)
        .expect("it goes below the first pass");
    cut[from..].to_vec()
}

#[test]
fn jobs_made_before_carving_styles_keep_light_passes_and_new_ones_stay_down() {
    let mut old = serde_json::to_value(Params::default()).unwrap();
    old.as_object_mut().unwrap().remove("style");
    let old: Params = serde_json::from_value(old).unwrap();
    assert_eq!(old.style, Style::LightPasses);
    assert_eq!(Params::default().style, Style::StayDown);
    assert_eq!(Params::default().order, Order::RoundTheShape);
    assert_eq!(Params::default().direction, Direction::ShallowIn);
}

#[test]
fn a_later_light_pass_stops_where_the_pass_before_reached() {
    // the second pass carves only the middle, deeper than the first went: never out to the
    // star's points, which the first pass carved to their full depth
    let t = carve(a_star(), &styled(Style::LightPasses));
    let level = first_pass(&t);
    let shallowest = below_the_first_pass(&t)
        .iter()
        .map(|q| q.z)
        .fold(f64::NEG_INFINITY, f64::max);
    assert!(shallowest <= level + 0.1, "{shallowest} above {level}");
}

#[test]
fn staying_down_cuts_one_ring_on_the_way_down_and_the_centre_lines_once() {
    let t = carve(a_star(), &styled(Style::StayDown));
    let level = first_pass(&t);
    // before the last pass: only the ring round the first pass's floor, all at its level
    let cut: Vec<DVec3> = t
        .moves
        .iter()
        .filter(|m| m.kind == MoveKind::Cut)
        .map(|m| m.to)
        .collect();
    let last = cut.iter().position(|q| q.z < level - 1e-3).unwrap();
    assert!(last > 0, "the first pass cuts something");
    let on_floor = cut[..last]
        .iter()
        .filter(|q| (q.z - level).abs() < 1e-3)
        .count();
    assert!(
        on_floor * 10 >= last * 9,
        "{on_floor} of {last} on the first floor"
    );
    for q in &cut[..last] {
        assert!(
            q.z < level + 0.3,
            "{q}: the first pass carves no centre lines"
        );
    }
    // each point of the star carved once, in the last pass, out and back
    let at_points = below_the_first_pass(&t)
        .windows(2)
        .filter(|w| w[0].z < -0.2 && w[1].z >= -0.2)
        .count();
    assert_eq!(at_points, 5);
    assert_eq!(lifts(&t), 1, "one plunge and one lift a shape");
}

#[test]
fn staying_down_makes_far_fewer_moves_than_light_passes() {
    let light = cuts(&carve(a_star(), &styled(Style::LightPasses)));
    let down = cuts(&carve(a_star(), &styled(Style::StayDown)));
    let lift = cuts(&carve(a_star(), &styled(Style::Lift)));
    assert!(
        down * 5 < light * 4,
        "stay down {down}, light passes {light}"
    );
    assert!(lift < down, "lift {lift}, stay down {down}");
    // a star is about ten straight stretches and the curves where they meet: the centre
    // lines' small wobble is not followed point by point
    assert!(down < 140, "{down} cuts");
}

#[test]
fn round_the_shape_visits_the_points_in_turn_anticlockwise() {
    let p = Params {
        order: Order::RoundTheShape,
        ..styled(Style::StayDown)
    };
    let t = carve(a_star(), &p);
    // the angle round the middle of each point the bit reaches
    let tips: Vec<f64> = below_the_first_pass(&t)
        .windows(2)
        .filter(|w| w[0].z < -0.2 && w[1].z >= -0.2)
        .map(|w| (w[1].y - 30.0).atan2(w[1].x - 30.0))
        .collect();
    assert_eq!(tips.len(), 5);
    let turns: Vec<f64> = tips
        .windows(2)
        .map(|w| (w[1] - w[0]).rem_euclid(std::f64::consts::TAU))
        .collect();
    for turn in &turns {
        assert!(
            (turn - std::f64::consts::TAU / 5.0).abs() < 0.2,
            "points visited at {tips:?}"
        );
    }
}

#[test]
fn lifting_cuts_each_stretch_once_from_the_end_asked_for() {
    for (direction, down) in [(Direction::ShallowIn, true), (Direction::DepthOut, false)] {
        let p = Params {
            direction,
            ..styled(Style::Lift)
        };
        let t = carve(a_star(), &p);
        // the stretches of the last pass, each between a plunge and a lift
        let mut stretches: Vec<Vec<DVec3>> = vec![];
        for m in &t.moves {
            match m.kind {
                MoveKind::Plunge => stretches.push(vec![m.to]),
                MoveKind::Cut => stretches.last_mut().unwrap().push(m.to),
                MoveKind::Rapid => {}
            }
        }
        let level = first_pass(&t);
        let spokes: Vec<&Vec<DVec3>> = stretches
            .iter()
            .filter(|s| s.iter().any(|q| q.z < level - 1e-3))
            .collect();
        assert!(
            spokes.len() >= 5,
            "{direction:?}: {} stretches",
            spokes.len()
        );
        for s in spokes {
            let (first, last) = (s[0].z, s[s.len() - 1].z);
            if down {
                assert!(first >= last, "{direction:?} from {first} to {last}");
            } else {
                assert!(first <= last, "{direction:?} from {first} to {last}");
            }
        }
    }
}

/// How deep the cone of `toolpath`'s bit (90°) has cut at `at`: the lowest its sides come over
/// it, from every cut and entry, followed every twentieth of a millimetre.
fn carved_at(toolpath: &Toolpath, at: Point) -> f64 {
    let spread = (toolpath.tool.angle.to_radians() / 2.0).tan();
    let mut lowest = 0.0_f64;
    for w in toolpath.moves.windows(2) {
        if w[1].kind == MoveKind::Rapid {
            continue;
        }
        let (from, to) = (w[0].to, w[1].to);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a short cut"
        )]
        let steps = (from.distance(to) / 0.05).ceil().max(1.0) as usize;
        for k in 0..=steps {
            #[expect(clippy::cast_precision_loss, reason = "a short cut")]
            let tip = from.lerp(to, k as f64 / steps as f64);
            lowest = lowest.min(tip.z + tip.truncate().distance(at) / spread);
        }
    }
    lowest
}

#[test]
fn every_style_carves_the_same_star() {
    // the same groove whichever way it is cut: as one pass would carve it
    let carve_with = |style: Style, pass_depth: f64| {
        let mut p = styled(style);
        p.tool.pass_depth = pass_depth;
        carve(a_star(), &p)
    };
    let one = carve_with(Style::LightPasses, 20.0);
    for style in [Style::LightPasses, Style::StayDown, Style::Lift] {
        let other = carve_with(style, 3.0);
        for i in 0..30 {
            for j in 0..30 {
                let q = Point::new(15.0 + f64::from(i), 15.0 + f64::from(j));
                let (a, b) = (carved_at(&one, q), carved_at(&other, q));
                assert!((a - b).abs() < 0.03, "{style:?} at {q}: {b}, one pass {a}");
            }
        }
    }
}
