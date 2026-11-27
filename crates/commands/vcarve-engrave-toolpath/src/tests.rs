use super::*;
use cut_craft_core::document::Job;
use cut_craft_core::geometry::polyline_distance;
use cut_craft_core::simulation::Material;
use cut_craft_core::testing::{made_toolpath, square};
use cut_craft_core::toolpath::Toolpath;

cut_craft_core::contract_tests!(VcarveEngraveToolpath);

fn moved(path: &Path, by: Point) -> Path {
    path.transformed(&cut_craft_core::glam::DAffine2::from_translation(by))
}

#[test]
fn in_selection_order_each_shape_is_carved_before_the_next() {
    let right = moved(&rectangle(10.0, 10.0), Point::new(40.0, 0.0));
    let left = rectangle(10.0, 10.0);
    let p = Params {
        use_vector_selection_order: true,
        ..Params::default()
    };
    let cuts = |t: &Toolpath| -> Vec<bool> {
        t.moves
            .iter()
            .filter(|m| m.kind == MoveKind::Cut)
            .map(|m| m.to.x > 30.0)
            .collect()
    };
    let ordered = cuts(&carve(vec![right.clone(), left.clone()], &p));
    let first_left = ordered.iter().position(|&r| !r).unwrap();
    assert!(
        ordered[..first_left].iter().all(|&r| r),
        "the right one first"
    );
    assert!(
        ordered[first_left..].iter().all(|&r| !r),
        "then the left, once"
    );
    // a letter with a hole stays one shape
    let hole = moved(&rectangle(4.0, 4.0), Point::new(3.0, 3.0));
    assert_eq!(
        shapes(&[left, hole, right])
            .iter()
            .map(Vec::len)
            .collect::<Vec<_>>(),
        [2, 1]
    );
}

#[test]
fn with_vector_start_points_the_first_pass_starts_by_the_vectors_start() {
    // the square starts at its top right corner
    let path = Path::polyline(
        &[
            Point::new(20.0, 20.0),
            Point::new(0.0, 20.0),
            Point::new(0.0, 0.0),
            Point::new(20.0, 0.0),
        ],
        true,
    );
    let start = |style| {
        let t = carve(
            vec![path.clone()],
            &Params {
                use_vector_start_points: true,
                style,
                ..Params::default()
            },
        );
        let m = t.moves.iter().find(|m| m.kind != MoveKind::Rapid).unwrap();
        Point::new(m.to.x, m.to.y)
    };
    let corner = Point::new(20.0, 20.0);
    // light passes start down the centre line, from the corner itself
    let light = start(Style::LightPasses);
    assert!(light.distance(corner) < 1.0, "{light}");
    // staying down starts at the corner of the first pass's floor, the nearest it cuts
    let down = start(Style::StayDown);
    let reach = 6.35 / 2.0;
    assert!(
        (down.x - down.y).abs() < 1e-6 && down.distance(corner) < reach * 2.0,
        "{down}"
    );
}

fn rectangle(width: f64, height: f64) -> Path {
    Path::polyline(
        &[
            Point::new(0.0, 0.0),
            Point::new(width, 0.0),
            Point::new(width, height),
            Point::new(0.0, height),
        ],
        true,
    )
}

fn carve(paths: Vec<Path>, params: &Params) -> Toolpath {
    made_toolpath::<VcarveEngraveToolpath>(paths, params).unwrap()
}

fn deepest(t: &Toolpath) -> f64 {
    t.moves.iter().map(|m| m.to.z).fold(f64::INFINITY, f64::min)
}

/// The material of a 100 mm long job after the toolpath, in quarter-millimetre cells.
fn carved(t: &Toolpath, job: &Job) -> Material {
    let mut m = Material::new(job, 400);
    m.cut(t);
    m
}

#[test]
fn the_clearance_tool_goes_round_a_shape_inside_another() {
    use cut_craft_core::testing::{document_with, square};
    let p = Params {
        flat_depth: 3.0,
        clearance_tools: vec![Tool::default()],
        ..Params::default()
    };
    let mut doc = document_with(vec![rectangle(60.0, 40.0), square(25.0, 15.0, 10.0)]);
    doc.apply(&VcarveEngraveToolpath::run(&doc, &p).unwrap())
        .unwrap();
    let mill = doc.toolpaths.values().next().unwrap();
    let island = Point::new(30.0, 20.0);
    let nearest = mill
        .moves
        .windows(2)
        .filter(|w| w[1].kind == MoveKind::Cut)
        .map(|w| {
            cut_craft_core::geometry::polyline_distance(
                &[w[0].to.truncate(), w[1].to.truncate()],
                island,
            )
        })
        .fold(f64::MAX, f64::min);
    // the island's 5 mm half-width and the mill's 3 mm radius
    assert!(
        nearest >= 8.0 - 0.05,
        "the mill cuts {nearest} mm from the island's middle"
    );
}

#[test]
fn a_clearance_tool_clears_the_flat_bottom_and_the_v_bit_cuts_what_it_left() {
    use cut_craft_core::testing::document_with;
    let mill = Tool {
        name: "6 mm End Mill".into(),
        ..Tool::default()
    };
    let p = Params {
        flat_depth: 3.0,
        clearance_tools: vec![mill],
        ..Params::default()
    };
    let mut doc = document_with(vec![rectangle(60.0, 40.0)]);
    doc.apply(&VcarveEngraveToolpath::run(&doc, &p).unwrap())
        .unwrap();
    let made: Vec<&Toolpath> = doc.toolpaths.values().collect();
    assert_eq!(made.len(), 2);
    assert_eq!(made[0].tool.name, "6 mm End Mill");
    assert!(
        (deepest(made[0]) + 3.0).abs() < 1e-9,
        "flat at the flat depth"
    );
    assert_eq!(made[1].tool.shape, ToolShape::VBit);
    let alone = carve(
        vec![rectangle(60.0, 40.0)],
        &Params {
            flat_depth: 3.0,
            ..Params::default()
        },
    );
    assert!(
        made[1].moves.len() < alone.moves.len(),
        "the V-bit leaves the middle to the end mill"
    );
    // together they carve what the V-bit alone would: flat in the middle, sloped walls
    let job = Job {
        width: 60.0,
        height: 40.0,
        ..Job::default()
    };
    let both = {
        let mut m = Material::new(&job, 240);
        m.cut(made[0]);
        m.cut(made[1]);
        m
    };
    let solo = carved(&alone, &job);
    for (x, y) in [(30.0, 20.0), (8.0, 20.0), (1.0, 20.0), (4.5, 4.5)] {
        let (a, b) = (
            both.at(Point::new(x, y)).unwrap(),
            solo.at(Point::new(x, y)).unwrap(),
        );
        assert!(
            (a - b).abs() < 0.3,
            "at ({x}, {y}): {a} with clearance, {b} alone"
        );
    }
    // a clearance tool needs a flat bottom to clear
    let no_flat = Params {
        flat_depth: 0.0,
        ..p
    };
    assert!(VcarveEngraveToolpath::run(&doc, &no_flat).is_err());
}

#[test]
fn a_90_degree_bit_carves_a_slot_as_deep_as_half_its_width() {
    let t = carve(vec![rectangle(100.0, 10.0)], &Params::default());
    let step = t.tool.finishing_stepover() * t.tool.diameter;
    assert!((deepest(&t) + 5.0).abs() <= step, "{}", deepest(&t));
    let job = Job {
        width: 100.0,
        height: 10.0,
        ..Job::default()
    };
    // depth is the distance in from the edge; within a pass spacing plus a cell
    let m = carved(&t, &job);
    for (y, depth) in [(1.0, 1.0), (3.0, 3.0), (5.0, 5.0)] {
        let z = m.at(Point::new(50.0, y)).unwrap();
        assert!((z + depth).abs() <= step + 0.25, "at y {y}: {z}");
    }
}

/// A 3/4 inch 90° V-bit stepping over 6.2%: 1.18 mm between rings.
fn big_bit() -> Params {
    let mut p = Params::default();
    p.tool.diameter = 19.05;
    p.tool.stepover = 0.062;
    p.tool.finish_stepover = 0.062;
    p
}

fn strip(y: f64, width: f64) -> Path {
    moved(&rectangle(40.0, width), Point::new(0.0, y))
}

#[test]
fn strokes_of_every_width_are_carved_to_a_crease_down_their_middle() {
    // thin strokes beside a wide one used to be missed, or stop a step short of the middle
    let t = carve(
        vec![strip(0.0, 2.0), strip(5.0, 3.6), strip(12.0, 10.0)],
        &big_bit(),
    );
    let job = Job {
        width: 40.0,
        height: 22.0,
        ..Job::default()
    };
    let m = carved(&t, &job);
    for (middle, half) in [(1.0, 1.0), (6.8, 1.8), (17.0, 5.0)] {
        let z = m.at(Point::new(20.0, middle)).unwrap();
        assert!((z + half).abs() < 0.1, "{half} deep at y {middle}: {z}");
        // and sloping up to the edges at the bit's angle
        let z = m.at(Point::new(20.0, middle - half / 2.0)).unwrap();
        assert!(
            (z + half / 2.0).abs() < 0.15,
            "half way out at y {middle}: {z}"
        );
    }
}

#[test]
fn the_narrower_the_bit_the_deeper_it_carves() {
    let deepest_with = |angle: f64| {
        let mut p = Params::default();
        p.tool.angle = angle;
        deepest(&carve(vec![rectangle(40.0, 10.0)], &p))
    };
    // half the width over the tangent of half the angle
    for angle in [60.0, 90.0, 100.0] {
        let want = 5.0 / (f64::to_radians(angle) / 2.0).tan();
        let got = -deepest_with(angle);
        assert!(
            (got - want).abs() < 0.02,
            "{angle}°: {got} deep, not {want}"
        );
    }
}

#[test]
fn corners_are_carved_sharp() {
    // a 20° point: the crease runs right into it, rising to the surface
    let spike = Path::polyline(
        &[
            Point::new(0.0, 0.0),
            Point::new(30.0, 5.0 * (30.0 / 30.0)),
            Point::new(0.0, 10.0),
        ],
        true,
    );
    let t = carve(vec![spike], &Params::default());
    let job = Job {
        width: 30.0,
        height: 10.0,
        ..Job::default()
    };
    let m = carved(&t, &job);
    let half_angle = (5.0f64 / 30.0).atan();
    for from_tip in [0.5, 1.0, 2.0, 3.3, 5.0, 8.0] {
        let at = Point::new(30.0 - from_tip * half_angle.cos(), 5.0);
        let want = from_tip * half_angle.sin();
        let z = m.at(at).unwrap();
        assert!(
            (z + want).abs() < 0.08,
            "{from_tip} from the tip: {z}, not {}",
            -want
        );
    }
}

#[test]
fn no_cut_goes_more_than_a_pass_depth_below_what_is_cut_already() {
    let mut p = Params {
        style: Style::LightPasses,
        ..Params::default()
    };
    p.tool.angle = 60.0;
    p.tool.pass_depth = 2.0;
    let t = carve(vec![rectangle(40.0, 10.0)], &p);
    assert!((deepest(&t) + 8.66).abs() < 0.02, "{}", deepest(&t));
    let job = Job {
        width: 40.0,
        height: 10.0,
        ..Job::default()
    };
    let mut m = Material::new(&job, 200);
    // and next to nothing is cut in air: runs between lifts that cut nothing
    let (mut removed, mut run, mut air, mut all) = (0.0f64, 0.0, 0.0, 0.0);
    for w in t.moves.windows(2) {
        if w[1].kind == MoveKind::Rapid {
            if removed < 0.01 {
                air += run;
            }
            (removed, run) = (0.0, 0.0);
            continue;
        }
        for q in [w[0].to, w[1].to] {
            if let Some(top) = m.at(q.truncate()) {
                assert!(q.z >= top - 2.0 - 0.05, "{q} is {} below {top}", top - q.z);
            }
        }
        removed = removed.max(m.overcut(&t.tool, w[0].to, w[1].to));
        run += w[0].to.distance(w[1].to);
        all += w[0].to.distance(w[1].to);
        m.sweep(w[0].to, w[1].to, &t.tool);
    }
    assert!(air < all * 0.02, "{air} mm of {all} in air");
}

/// A C: a 30 mm square with a 20 x 14 mm notch out of its right side, arms 8 mm wide.
fn c_shape() -> Path {
    Path::polyline(
        &[
            Point::new(0.0, 0.0),
            Point::new(30.0, 0.0),
            Point::new(30.0, 8.0),
            Point::new(10.0, 8.0),
            Point::new(10.0, 22.0),
            Point::new(30.0, 22.0),
            Point::new(30.0, 30.0),
            Point::new(0.0, 30.0),
        ],
        true,
    )
}

#[test]
fn each_shape_is_carved_with_one_plunge_and_one_lift() {
    // the bit stays down between the cuts of a shape, pass after pass, round its bends
    let mut p = Params::default();
    p.tool.pass_depth = 1.0;
    let apart = moved(&rectangle(10.0, 6.0), Point::new(50.0, 0.0));
    let t = carve(vec![c_shape(), apart], &p);
    let plunges = t
        .moves
        .iter()
        .filter(|m| m.kind == MoveKind::Plunge)
        .count();
    let lifts = t
        .moves
        .windows(2)
        .filter(|w| w[1].kind == MoveKind::Rapid && w[1].to.z > w[0].to.z)
        .count();
    assert_eq!((plunges, lifts), (2, 2));
    // the widest circle in the C sits in a back corner, touching the notch's corner
    let widest = 18.0 - 164.0f64.mul_add(-1.0, 18.0 * 18.0).sqrt();
    assert!((deepest(&t) + widest).abs() < 0.02, "{}", deepest(&t));
}

fn with_entry(mode: EntryMode) -> Params {
    let mut p = Params {
        entry: Some(EntrySettings {
            mode,
            ..EntrySettings::default()
        }),
        ..Params::default()
    };
    p.tool.pass_depth = 1.0;
    p
}

/// Times the bit comes down from above the material, and the steepest it then goes
/// down below the surface, in degrees from level.
fn entries_and_steepest(t: &Toolpath) -> (usize, f64) {
    let entries = t
        .moves
        .windows(2)
        .filter(|w| w[0].kind == MoveKind::Rapid && w[1].kind != MoveKind::Rapid)
        .count();
    let steepest = t
        .moves
        .windows(2)
        // the entry's own moves: a groove may fall steeply where it narrows
        .filter(|w| w[1].kind == MoveKind::Plunge && w[1].to.z < w[0].to.z - 1e-9)
        .filter(|w| w[1].to.z < -1e-9)
        .map(|w| {
            let (a, b) = (w[0].to, w[1].to);
            (a.z - b.z)
                .atan2(a.truncate().distance(b.truncate()))
                .to_degrees()
        })
        .fold(0.0, f64::max);
    (entries, steepest)
}

#[test]
fn a_zig_zag_entry_slides_in_along_the_groove_still_once_a_shape() {
    let apart = moved(&rectangle(10.0, 6.0), Point::new(50.0, 0.0));
    let plain = carve(
        vec![c_shape(), apart.clone()],
        &with_entry(EntryMode::Plunge),
    );
    let zig = carve(vec![c_shape(), apart], &with_entry(EntryMode::ZigZag));
    let (entries, steepest) = entries_and_steepest(&zig);
    assert_eq!(entries, 2, "one way down a shape");
    assert!(steepest <= 10.0 + 1e-6, "{steepest}");
    // as deep as the shape's width sets, no deeper
    assert!((deepest(&zig) - deepest(&plain)).abs() < 1e-9);
    assert_eq!(zig.fallbacks, vec![]);
}

#[test]
fn a_ramp_entry_never_goes_below_the_groove_it_follows() {
    let p = with_entry(EntryMode::Ramp);
    let t = carve(vec![rectangle(10.0, 6.0)], &p);
    let (_, steepest) = entries_and_steepest(&t);
    assert!(steepest < 89.0, "never straight down: {steepest}");
    let plain = carve(vec![rectangle(10.0, 6.0)], &with_entry(EntryMode::Plunge));
    assert!((deepest(&t) - deepest(&plain)).abs() < 1e-9);
}

#[test]
fn a_groove_too_short_for_the_ramp_is_entered_straight_down() {
    let mut p = with_entry(EntryMode::Ramp);
    p.entry = p.entry.map(|e| EntrySettings {
        ramp_distance: 500.0,
        ..e
    });
    p.place = Place::OnTheLine;
    let t = carve(vec![rectangle(10.0, 6.0)], &p);
    assert_eq!(t.fallbacks.len(), 1);
    assert_eq!(t.fallbacks[0].used, EntryMode::Plunge);
}

#[test]
fn a_v_carve_starts_by_going_straight_down() {
    assert_eq!(
        Params::default().entry.map(|e| e.mode),
        Some(EntryMode::Plunge)
    );
}

#[test]
fn on_the_line_ramps_in_along_the_line() {
    let mut p = with_entry(EntryMode::Ramp);
    p.place = Place::OnTheLine;
    let line = Path::polyline(&[Point::new(0.0, 0.0), Point::new(60.0, 0.0)], false);
    let t = carve(vec![line], &p);
    let (_, steepest) = entries_and_steepest(&t);
    assert!(steepest < 89.0, "{steepest}");
}

#[test]
fn staying_down_between_cuts_never_cuts_outside_the_shape() {
    let mut p = Params::default();
    p.tool.pass_depth = 1.0;
    let shape = c_shape();
    let t = carve(vec![shape.clone()], &p);
    let job = Job {
        width: 40.0,
        height: 40.0,
        ..Job::default()
    };
    let m = carved(&t, &job);
    let ring = [cut_points(&shape)];
    let mut edges = ring[0].clone();
    edges.push(edges[0]);
    for column in 0..160 {
        for row in 0..160 {
            let at = Point::new(f64::from(column) * 0.25, f64::from(row) * 0.25);
            let edge = polyline_distance(&edges, at);
            let z = m.at(at).unwrap();
            if inside(&ring, at) {
                // and no deeper than the bit's sides touching the edges
                assert!(z >= -edge - 0.1, "{at} cut to {z}, {edge} from the edge");
            } else if edge > 0.3 {
                assert!(z > -0.05, "{at} outside the shape cut to {z}");
            }
        }
    }
}

#[test]
fn inside_and_outside_carve_a_groove_beside_the_edge_with_sharp_corners() {
    let job = Job {
        width: 30.0,
        height: 30.0,
        ..Job::default()
    };
    let at = |place: Place, x: f64, y: f64| {
        let p = Params {
            place,
            depth: 1.0,
            ..Params::default()
        };
        carved(&carve(vec![square(10.0, 10.0, 10.0)], &p), &job)
            .at(Point::new(x, y))
            .unwrap()
    };
    // a 90° bit 1 mm deep: a groove 2 mm wide, one side on the edge at x 10
    for (x, outside, inside) in [
        (8.0, 0.0, 0.0),
        (8.5, -0.5, 0.0),
        (9.0, -1.0, 0.0),
        (9.5, -0.5, 0.0),
        (10.5, 0.0, -0.5),
        (11.0, 0.0, -1.0),
        (11.5, 0.0, -0.5),
        (12.5, 0.0, 0.0),
        (15.0, 0.0, 0.0),
    ] {
        let (o, i) = (at(Place::Outside, x, 15.0), at(Place::Inside, x, 15.0));
        assert!((o - outside).abs() < 0.12, "outside at x {x}: {o}");
        assert!((i - inside).abs() < 0.12, "inside at x {x}: {i}");
    }
    // inside, the groove runs right into the corner: half a millimetre in from both edges
    // it is half a millimetre deep
    let z = at(Place::Inside, 10.5, 10.5);
    assert!((z + 0.5).abs() < 0.12, "{z}");
}

#[test]
fn on_the_line_the_tip_follows_open_vectors_as_deep_as_set() {
    let line = Path::polyline(&[Point::new(0.0, 5.0), Point::new(20.0, 5.0)], false);
    let p = Params {
        place: Place::OnTheLine,
        depth: 1.5,
        ..Params::default()
    };
    let t = carve(vec![line], &p);
    assert!((deepest(&t) + 1.5).abs() < 1e-9);
    assert!(
        t.moves
            .iter()
            .filter(|m| m.kind == MoveKind::Cut)
            .all(|m| (m.to.y - 5.0).abs() < 1e-9)
    );
    // but a groove deeper than the bit's cone is wide is refused, saying how deep it can go
    let too_deep = Params { depth: 7.0, ..p };
    let why =
        made_toolpath::<VcarveEngraveToolpath>(vec![rectangle(10.0, 10.0)], &too_deep).unwrap_err();
    assert!(why.contains("6.35 mm deep"), "{why}");
}

#[test]
fn nested_loops_carve_every_other_band() {
    // three loops one inside another: the band between the outer two is carved, the
    // band between the inner two is left standing, the middle of the inner one is carved
    let loops = || {
        vec![
            square(0.0, 0.0, 40.0),
            square(5.0, 5.0, 30.0),
            square(12.0, 12.0, 16.0),
        ]
    };
    let job = Job {
        width: 40.0,
        height: 40.0,
        ..Job::default()
    };
    let depth_at = |paths: Vec<Path>, x: f64| {
        carved(&carve(paths, &Params::default()), &job)
            .at(Point::new(x, 20.0))
            .unwrap()
    };
    assert!(
        (depth_at(loops(), 2.5) + 2.5).abs() < 0.1,
        "outer band carved"
    );
    assert!(
        depth_at(loops(), 8.5).abs() < 1e-9,
        "middle band left standing"
    );
    assert!(
        (depth_at(loops(), 20.0) + 6.35).abs() < 0.1,
        "inner middle carved"
    );
    // the same loop twice cancels out: with the middle loop doubled, the outer and
    // inner loops make one 12 mm band, carved as one, and the inner middle is left
    let mut doubled = loops();
    doubled.push(square(5.0, 5.0, 30.0));
    assert!(
        (depth_at(doubled.clone(), 8.5) + 3.5).abs() < 0.1,
        "one wide band"
    );
    assert!(depth_at(doubled, 20.0).abs() < 1e-9, "inner middle left");
}

#[test]
fn the_area_to_carve_is_what_the_form_will_carve() {
    let area = |place: Place, paths: Vec<Path>| -> f64 {
        let p = Params {
            place,
            depth: 1.0,
            ..Params::default()
        };
        // outlines run anticlockwise and holes clockwise: their signed areas add up
        carved_area(&paths, &p)
            .iter()
            .map(|r| {
                let q = cut_points(r);
                (0..q.len())
                    .map(|i| q[i].perp_dot(q[(i + 1) % q.len()]) / 2.0)
                    .sum::<f64>()
            })
            .sum()
    };
    let loops = || {
        vec![
            square(0.0, 0.0, 40.0),
            square(5.0, 5.0, 30.0),
            square(12.0, 12.0, 16.0),
        ]
    };
    // every other band: 40² - 30² + 16²
    assert!((area(Place::Fill, loops()) - (1600.0 - 900.0 + 256.0)).abs() < 1e-6);
    // a 90° bit 1 mm deep: a groove 2 mm wide inside a 10 mm square
    let inside = area(Place::Inside, vec![square(0.0, 0.0, 10.0)]);
    assert!((inside - (100.0 - 36.0)).abs() < 1e-6, "{inside}");
    // and along an open line, 2 mm wide with round ends
    let line = Path::polyline(&[Point::new(0.0, 0.0), Point::new(10.0, 0.0)], false);
    let along = area(Place::OnTheLine, vec![line]);
    assert!(
        (along - (20.0 + std::f64::consts::PI)).abs() < 0.05,
        "{along}"
    );
    // Fill leaves open vectors out, as the carving does
    assert!(
        area(
            Place::Fill,
            vec![Path::polyline(
                &[Point::new(0.0, 0.0), Point::new(10.0, 0.0)],
                false
            )]
        )
        .abs()
            < 1e-9
    );
}

#[test]
fn it_never_carves_below_the_material() {
    use cut_craft_core::testing::document_with;
    let mut doc = document_with(vec![rectangle(40.0, 10.0)]);
    doc.job.thickness = 3.0;
    let t = VcarveEngraveToolpath::run(&doc, &Params::default()).unwrap();
    doc.apply(&t).unwrap();
    let t = doc.toolpaths.values().next().unwrap();
    assert!((deepest(t) + 3.0).abs() < 1e-6, "{}", deepest(t));
}

#[test]
fn each_floor_ring_is_the_shapes_shrunk_by_its_offset() {
    // a star with a hole, and a thin sliver that vanishes between steps
    let star: Vec<Point> = (0..10)
        .map(|k| {
            let r = if k % 2 == 0 { 12.0 } else { 5.0 };
            Point::from_angle(f64::from(k) * std::f64::consts::PI / 5.0) * r
        })
        .collect();
    let paths = [
        Path::polyline(&star, true),
        moved(&rectangle(2.0, 2.0), Point::new(-1.0, -1.0)),
        moved(&rectangle(20.0, 1.1), Point::new(20.0, 0.0)),
    ];
    let regions = polygon::regions(&paths);
    let offsets = stepped_in(&regions, 0.5);
    assert!(offsets.len() >= 4, "{}", offsets.len());
    for (offset, rings) in &offsets {
        let direct = polygon::offset_regions(&regions, -offset, Corners::Round);
        assert_eq!(rings.len(), direct.len(), "at {offset}");
        // each ring all the way round, back to its first point
        let direct: Vec<Vec<Point>> = direct
            .iter()
            .map(|d| {
                let mut points = cut_points(d);
                points.push(points[0]);
                points
            })
            .collect();
        for ring in rings {
            for q in cut_points(ring) {
                let off = direct
                    .iter()
                    .map(|d| polyline_distance(d, q))
                    .fold(f64::INFINITY, f64::min);
                assert!(off < 0.03, "at {offset}: {q} is {off} off");
            }
        }
    }
}

#[test]
fn holes_are_left_standing() {
    let t = carve(
        vec![square(0.0, 0.0, 30.0), square(10.0, 10.0, 10.0)],
        &Params::default(),
    );
    let hole = [square(10.0, 10.0, 10.0).flatten(0.01)];
    assert!(
        t.moves
            .iter()
            .filter(|m| m.kind == MoveKind::Cut)
            .all(|m| !inside(&hole, m.to.truncate())),
        "nothing is cut inside the hole"
    );
}

#[test]
fn the_flat_depth_limits_how_deep_it_goes() {
    let params = Params {
        flat_depth: 2.0,
        ..Params::default()
    };
    let t = carve(vec![rectangle(100.0, 10.0)], &params);
    assert!(deepest(&t) >= -2.0 - 1e-9);
    assert!(deepest(&t) <= -2.0 + 1e-9, "and it does reach it");
}

#[test]
fn a_start_depth_lowers_the_whole_carving() {
    let params = Params {
        start_depth: 3.0,
        flat_depth: 2.0,
        ..Params::default()
    };
    let t = carve(vec![rectangle(100.0, 10.0)], &params);
    assert!((deepest(&t) + 5.0).abs() < 1e-9, "{}", deepest(&t));
    let job = Job {
        width: 100.0,
        height: 10.0,
        ..Job::default()
    };
    // the edge is cut at the start depth, sloping down from there
    let step = t.tool.finishing_stepover() * t.tool.diameter;
    let z = carved(&t, &job).at(Point::new(50.0, 1.0)).unwrap();
    assert!((z + 4.0).abs() <= step + 0.25, "{z}");
}

#[test]
fn projected_onto_the_model_it_carves_below_its_surface() {
    // an 8 mm square on the top of the domed job's 8 mm dome
    let mut doc = cut_craft_core::testing::domed();
    let change = cut_craft_core::Change::swap(&doc, [], vec![square(16.0, 16.0, 8.0)]);
    doc.apply(&change).unwrap();
    let params = Params {
        project_onto_model: true,
        ..Params::default()
    };
    let change = VcarveEngraveToolpath::run(&doc, &params).unwrap();
    doc.apply(&change).unwrap();
    let t = doc.toolpaths.into_values().next().unwrap();
    let surface = |p: Point| {
        let r = p.distance(Point::new(20.0, 20.0));
        (64.0 - r * r).sqrt() - 8.0
    };
    // up to the 4 mm the bit reaches in the middle below the dome (and a cell's slope)
    let cuts: Vec<_> = t
        .moves
        .iter()
        .filter(|m| m.kind == MoveKind::Cut)
        .map(|m| m.to)
        .collect();
    let off = cuts.iter().find(|q| {
        let below = surface(q.truncate()) - q.z;
        !(-0.3..=4.3).contains(&below)
    });
    assert!(off.is_none(), "{off:?}");
    // the corners are cut where the dome has fallen away, not near the material's top
    let from_corner =
        |q: &cut_craft_core::glam::DVec3| q.truncate().distance(Point::new(16.0, 16.0));
    let corner = cuts
        .iter()
        .min_by(|a, b| from_corner(a).total_cmp(&from_corner(b)))
        .unwrap();
    assert!(corner.z < -2.0, "{corner:?}");
}

#[test]
fn it_needs_a_v_bit_and_closed_vectors() {
    let end_mill = Params {
        tool: Tool::default(),
        ..Params::default()
    };
    let why =
        made_toolpath::<VcarveEngraveToolpath>(vec![rectangle(10.0, 10.0)], &end_mill).unwrap_err();
    assert!(why.contains("V-bit"), "{why}");
    let open = Path::polyline(&[Point::new(0.0, 0.0), Point::new(10.0, 0.0)], false);
    let why = made_toolpath::<VcarveEngraveToolpath>(vec![open], &Params::default()).unwrap_err();
    assert!(why.contains("closed"), "{why}");
}

/// The notes Fill leaves on the form for `paths`.
fn notes_for(paths: Vec<Path>, p: &Params) -> Vec<String> {
    let doc = cut_craft_core::testing::document_with(paths);
    VcarveEngraveToolpath::run(&doc, p).unwrap().notes
}

#[test]
fn a_shape_wider_than_the_bit_can_carve_to_a_point_says_so() {
    // the default 1/2" 90° bit is 6.35 mm tall: a 30 mm wide strip needs 15
    let p = Params::default();
    let notes = notes_for(vec![rectangle(60.0, 30.0)], &p);
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert!(notes[0].contains("6.35"), "its V's height: {}", notes[0]);
    assert!(notes[0].contains("Clearance Bits"), "{}", notes[0]);
    // narrow enough to carve to a point, a flat bottom asked for, or cleared by end mills
    assert_eq!(
        notes_for(vec![rectangle(60.0, 10.0)], &p),
        Vec::<String>::new()
    );
    let flat = Params {
        flat_depth: 2.0,
        ..Params::default()
    };
    assert_eq!(
        notes_for(vec![rectangle(60.0, 30.0)], &flat),
        Vec::<String>::new()
    );
}
