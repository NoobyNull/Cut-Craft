//! Entry modes: helix, zig-zag, ramp and straight plunge, each fitting itself to the room
//! at every place the tool goes down into the material.

use cut_craft_core::entry::{Entry, EntryMode, EntrySettings, Fallback, Room, path_moves};
use cut_craft_core::geometry::Point;
use cut_craft_core::toolpath::{Move, MoveKind};

const SAFE: f64 = 5.0;
/// A 1/4" bit.
const BIT: f64 = 6.35;

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Point> {
    [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
        .map(|(x, y)| Point::new(x, y))
        .to_vec()
}

fn entering(settings: EntrySettings, room: Option<Room>) -> Entry {
    Entry {
        enter: Some(settings),
        room,
        tool_diameter: BIT,
        ..Entry::default()
    }
}

fn mode(mode: EntryMode) -> EntrySettings {
    EntrySettings {
        mode,
        ..EntrySettings::default()
    }
}

/// The steepest any move goes down below `top`, as an angle from level in degrees; 90 for a
/// move straight down.
fn steepest_below(moves: &[Move], top: f64) -> f64 {
    moves
        .windows(2)
        .filter(|w| w[1].kind != MoveKind::Rapid && w[1].to.z < w[0].to.z - 1e-9)
        .filter(|w| w[1].to.z < top - 1e-9)
        .map(|w| {
            let (a, b) = (w[0].to, w[1].to);
            let across = a.truncate().distance(b.truncate());
            (a.z - b.z).atan2(across).to_degrees()
        })
        .fold(0.0, f64::max)
}

fn deepest(moves: &[Move]) -> f64 {
    moves.iter().map(|m| m.to.z).fold(f64::INFINITY, f64::min)
}

/// The moves from the first one at `top` until the tool first reaches `z`.
fn entry_moves(moves: &[Move], z: f64) -> Vec<Move> {
    let end = moves
        .iter()
        .position(|m| (m.to.z - z).abs() < 1e-9)
        .unwrap();
    moves[..=end].to_vec()
}

fn notes(entry: &Entry) -> Vec<Fallback> {
    entry.fallbacks.lock().unwrap().clone()
}

/// The widest any entry move strays from `centre`.
fn reach_from(moves: &[Move], centre: Point) -> f64 {
    moves
        .iter()
        .map(|m| m.to.truncate().distance(centre))
        .fold(0.0, f64::max)
}

#[test]
fn a_helix_goes_down_in_circles_of_the_size_asked_at_the_angle_asked() {
    // a ring in the middle of a big pocket
    let ring = rect(-2.0, -2.0, 2.0, 2.0);
    let room = Room::inside(vec![rect(-50.0, -50.0, 50.0, 50.0)]);
    let entry = entering(mode(EntryMode::Helix), Some(room));
    let moves = path_moves(&ring, true, &[-6.0], 0.0, SAFE, &entry);
    assert!((deepest(&moves) + 6.0).abs() < 1e-9);
    let down = entry_moves(&moves, -6.0);
    assert!(steepest_below(&down, 0.0) <= 3.0 + 1e-6);
    // every point on the way down is on one circle, 75% of the bit across
    let wanted = 0.75 * BIT / 2.0;
    let pts: Vec<Point> = down
        .iter()
        .filter(|m| m.to.z < -1e-9)
        .map(|m| m.to.truncate())
        .collect();
    assert!(pts.len() > 16);
    let centre = pts.iter().copied().sum::<Point>() / f64::from(u32::try_from(pts.len()).unwrap());
    for p in &pts {
        assert!(
            (p.distance(centre) - wanted).abs() < 0.05,
            "{} from the middle",
            p.distance(centre)
        );
    }
    assert_eq!(notes(&entry), vec![]);
}

#[test]
fn a_helix_shrinks_to_fit_a_small_pocket_and_never_gets_steeper() {
    // the room a 1/4" bit's centre has in a 10 mm square pocket
    let half = 5.0 - BIT / 2.0;
    let edge = rect(-half, -half, half, half);
    let entry = entering(
        mode(EntryMode::Helix),
        Some(Room::inside(vec![edge.clone()])),
    );
    let moves = path_moves(&edge, true, &[-6.0], 0.0, SAFE, &entry);
    let down = entry_moves(&moves, -6.0);
    assert!(steepest_below(&down, 0.0) <= 3.0 + 1e-6);
    // inside the room all the way down
    for m in &down {
        let p = m.to.truncate();
        assert!(p.x.abs() <= half + 1e-6 && p.y.abs() <= half + 1e-6, "{p}");
    }
    // still a helix: it goes round, so it is far from straight down
    assert!(reach_from(&down, down[down.len() - 1].to.truncate()) > 0.5);
    assert_eq!(notes(&entry), vec![]);
}

#[test]
fn a_helix_with_no_room_falls_back_to_a_zig_zag_along_the_path_and_says_so() {
    // a slot 7 mm wide for a 1/4" bit: the centre has 0.65 mm side to side
    let slot = [Point::new(0.0, 0.0), Point::new(40.0, 0.0)];
    let room = Room::inside(vec![rect(0.0, -0.325, 40.0, 0.325)]);
    let entry = entering(mode(EntryMode::Helix), Some(room));
    let moves = path_moves(&slot, false, &[-6.0], 0.0, SAFE, &entry);
    assert!(steepest_below(&moves, 0.0) <= 3.0 + 1e-6);
    for m in &moves {
        assert!(m.to.y.abs() < 1e-9, "stays on the slot: {}", m.to);
    }
    let fell = notes(&entry);
    assert_eq!(fell.len(), 1);
    assert_eq!(fell[0].wanted, EntryMode::Helix);
    assert_eq!(fell[0].used, EntryMode::ZigZag);
    assert!(fell[0].at.distance(Point::ZERO) < 1e-9);
}

#[test]
fn a_helix_with_no_room_given_falls_back_too() {
    let line = [Point::new(0.0, 0.0), Point::new(40.0, 0.0)];
    let entry = entering(mode(EntryMode::Helix), None);
    let moves = path_moves(&line, false, &[-3.0], 0.0, SAFE, &entry);
    assert!(steepest_below(&moves, 0.0) <= 3.0 + 1e-6);
    assert_eq!(notes(&entry)[0].used, EntryMode::ZigZag);
}

#[test]
fn a_zig_zag_goes_back_and_forth_the_passes_asked_at_the_angle_asked() {
    let line = [Point::new(0.0, 0.0), Point::new(100.0, 0.0)];
    let s = EntrySettings {
        mode: EntryMode::ZigZag,
        zigzag_angle: 10.0,
        zigzag_passes: 4,
        ..EntrySettings::default()
    };
    let entry = entering(s, None);
    let moves = path_moves(&line, false, &[-6.0], 0.0, SAFE, &entry);
    let down = entry_moves(&moves, -6.0);
    // four legs, each 1.5 mm down at 10°
    let legs: Vec<_> = down
        .windows(2)
        .filter(|w| w[1].to.z < w[0].to.z - 1e-9 && w[0].to.z < 1e-9)
        .collect();
    assert_eq!(legs.len(), 4);
    let leg = 1.5 / 10f64.to_radians().tan();
    for w in legs {
        assert!((w[0].to.x - w[1].to.x).abs() - leg < 1e-6);
    }
    // it ends where it began, at depth, to cut the line from its start
    assert!(down[down.len() - 1].to.truncate().distance(Point::ZERO) < 1e-9);
    assert_eq!(notes(&entry), vec![]);
}

#[test]
fn a_zig_zag_with_too_little_room_takes_shorter_legs_and_more_of_them() {
    // a short path: 4 legs of 8.5 mm will not fit in 5 mm
    let line = [Point::new(0.0, 0.0), Point::new(5.0, 0.0)];
    let s = EntrySettings {
        mode: EntryMode::ZigZag,
        zigzag_angle: 10.0,
        zigzag_passes: 4,
        ..EntrySettings::default()
    };
    let entry = entering(s, None);
    let moves = path_moves(&line, false, &[-6.0], 0.0, SAFE, &entry);
    assert!(steepest_below(&moves, 0.0) <= 10.0 + 1e-6);
    assert!(moves.iter().all(|m| m.to.x <= 5.0 + 1e-9));
    assert!((deepest(&moves) + 6.0).abs() < 1e-9);
    // still a zig-zag, only with more passes: not a fallback
    assert_eq!(notes(&entry), vec![]);
}

#[test]
fn a_ramp_goes_down_the_depth_over_the_distance() {
    let square = rect(0.0, 0.0, 40.0, 40.0);
    let s = EntrySettings {
        mode: EntryMode::Ramp,
        ramp_distance: 25.0,
        ..EntrySettings::default()
    };
    let entry = entering(s, None);
    let moves = path_moves(&square, true, &[-6.0], 0.0, SAFE, &entry);
    let down = entry_moves(&moves, -6.0);
    // 25 mm along the path, one way
    let bottom = down[down.len() - 1].to.truncate();
    assert!(bottom.distance(Point::new(25.0, 0.0)) < 1e-9, "{bottom}");
    let angle = 6f64.atan2(25.0).to_degrees();
    assert!(steepest_below(&down, 0.0) <= angle + 1e-6);
    assert_eq!(notes(&entry), vec![]);
}

#[test]
fn a_ramp_too_long_for_its_path_zig_zags_at_the_same_slope_instead() {
    let line = [Point::new(0.0, 0.0), Point::new(10.0, 0.0)];
    let s = EntrySettings {
        mode: EntryMode::Ramp,
        ramp_distance: 25.0,
        ..EntrySettings::default()
    };
    let entry = entering(s, None);
    let moves = path_moves(&line, false, &[-6.0], 0.0, SAFE, &entry);
    let angle = 6f64.atan2(25.0).to_degrees();
    assert!(steepest_below(&moves, 0.0) <= angle + 1e-6);
    assert!(moves.iter().all(|m| m.to.x <= 10.0 + 1e-9));
    let fell = notes(&entry);
    assert_eq!(
        (fell[0].wanted, fell[0].used),
        (EntryMode::Ramp, EntryMode::ZigZag)
    );
}

#[test]
fn with_no_room_at_all_it_plunges_straight_down_and_says_so() {
    // a hole only just wider than the bit: its centre goes round a 0.2 mm circle
    let ring: Vec<Point> = (0..24)
        .map(|i| {
            let a = std::f64::consts::TAU * f64::from(i) / 24.0;
            Point::new(0.1 * a.cos(), 0.1 * a.sin())
        })
        .collect();
    let entry = entering(
        mode(EntryMode::Helix),
        Some(Room::inside(vec![ring.clone()])),
    );
    let moves = path_moves(&ring, true, &[-3.0], 0.0, SAFE, &entry);
    assert!((deepest(&moves) + 3.0).abs() < 1e-9);
    let fell = notes(&entry);
    assert_eq!(fell.len(), 1);
    assert_eq!(fell[0].used, EntryMode::Plunge);
}

#[test]
fn a_straight_plunge_asked_for_is_not_a_fallback() {
    let square = rect(0.0, 0.0, 40.0, 40.0);
    let entry = entering(mode(EntryMode::Plunge), None);
    let moves = path_moves(&square, true, &[-3.0, -6.0], 0.0, SAFE, &entry);
    assert!((deepest(&moves) + 6.0).abs() < 1e-9);
    assert_eq!(notes(&entry), vec![]);
}

#[test]
fn each_pass_of_a_profile_enters_again_from_the_depth_above() {
    // an inside profile round a 40 mm square: a helix inside it at each pass
    let square = rect(0.0, 0.0, 40.0, 40.0);
    let entry = Entry {
        side: 1.0, // the waste is left of an anticlockwise path: inside
        ..entering(
            mode(EntryMode::Helix),
            Some(Room::inside(vec![square.clone()])),
        )
    };
    let moves = path_moves(&square, true, &[-3.0, -6.0], 0.0, SAFE, &entry);
    assert!(steepest_below(&moves, 0.0) <= 3.0 + 1e-6);
    // nothing goes outside the square
    for m in &moves {
        let p = m.to.truncate();
        assert!(
            (-1e-6..=40.0 + 1e-6).contains(&p.x) && (-1e-6..=40.0 + 1e-6).contains(&p.y),
            "{p}"
        );
    }
    assert_eq!(notes(&entry), vec![]);
}

#[test]
fn an_outside_profile_helixes_outside_the_part() {
    let square = rect(0.0, 0.0, 40.0, 40.0);
    // anticlockwise round a part: the waste is on the right
    let entry = Entry {
        side: -1.0,
        ..entering(
            mode(EntryMode::Helix),
            Some(Room::outside(vec![square.clone()])),
        )
    };
    let moves = path_moves(&square, true, &[-6.0], 0.0, SAFE, &entry);
    for m in &moves {
        let p = m.to.truncate();
        let inside = p.x > 1e-6 && p.x < 40.0 - 1e-6 && p.y > 1e-6 && p.y < 40.0 - 1e-6;
        assert!(!inside, "{p} is in the part");
    }
    assert!(steepest_below(&moves, 0.0) <= 3.0 + 1e-6);
    assert_eq!(notes(&entry), vec![]);
}

#[test]
fn old_ramp_settings_read_as_the_nearest_entry() {
    use cut_craft_core::entry::{Ramp, RampBy, Ramping};
    let by_distance = Ramping {
        by: RampBy::Distance,
        distance: 20.0,
        angle: 10.0,
    };
    assert_eq!(
        EntrySettings::from_old(Ramp::None, by_distance, 3.0).mode,
        EntryMode::Plunge
    );
    let smooth = EntrySettings::from_old(Ramp::Smooth, by_distance, 3.0);
    assert_eq!(smooth.mode, EntryMode::Ramp);
    assert!((smooth.ramp_distance - 20.0).abs() < 1e-9);
    // a zig-zag over 20 mm, there and back: two passes at the slope a full pass makes
    let zig = EntrySettings::from_old(Ramp::ZigZag, by_distance, 3.0);
    assert_eq!((zig.mode, zig.zigzag_passes), (EntryMode::ZigZag, 2));
    assert!((zig.zigzag_angle - 1.5f64.atan2(20.0).to_degrees()).abs() < 1e-9);
}

#[test]
fn checks_before_cutting_say_where_an_entry_went_down_another_way() {
    use cut_craft_core::checks::problems;
    use cut_craft_core::testing::{document_with, square};
    use cut_craft_core::toolpath::{Tool, Toolpath};
    let doc = document_with(vec![square(0.0, 0.0, 10.0)]);
    let fallback = |used| Fallback {
        at: Point::new(5.0, 5.0),
        wanted: EntryMode::Helix,
        used,
    };
    let carve = |fallbacks| Toolpath {
        name: "V-Carve 1".into(),
        strategy: "vcarve_engrave_toolpath".into(),
        params: serde_json::json!({}),
        tool: Tool::default(),
        vectors: vec![],
        moves: vec![],
        visible: true,
        power: vec![],
        stale: false,
        group: None,
        drill_bank: None,
        fallbacks,
    };
    let toolpath = |fallbacks| Toolpath {
        name: "Pocket 1".into(),
        strategy: "pocket_toolpath".into(),
        params: serde_json::json!({}),
        tool: Tool::default(),
        vectors: vec![],
        moves: vec![
            Move {
                kind: MoveKind::Rapid,
                to: glam::DVec3::new(5.0, 5.0, 5.0),
            },
            Move {
                kind: MoveKind::Plunge,
                to: glam::DVec3::new(5.0, 5.0, -1.0),
            },
        ],
        visible: true,
        power: vec![],
        stale: false,
        group: None,
        drill_bank: None,
        fallbacks,
    };
    let none = problems(&doc, &toolpath(vec![]));
    assert!(
        !none.iter().any(|p| p.contains("straight down")),
        "{none:?}"
    );
    let plunged = problems(&doc, &toolpath(vec![fallback(EntryMode::Plunge)]));
    let said = plunged
        .iter()
        .find(|p| p.contains("straight down"))
        .unwrap_or_else(|| panic!("{plunged:?}"));
    assert!(
        said.contains("1 place") && said.contains("X 5.0 mm Y 5.0 mm"),
        "{said}"
    );
    let zigged = problems(&doc, &toolpath(vec![fallback(EntryMode::ZigZag)]));
    assert!(zigged.iter().any(|p| p.contains("zig-zag")), "{zigged:?}");
    // a bit that must not go straight down ramped in where a plunge was asked for
    let kept_off = Fallback {
        wanted: EntryMode::Plunge,
        used: EntryMode::Ramp,
        ..fallback(EntryMode::Ramp)
    };
    let ramped = problems(&doc, &toolpath(vec![kept_off]));
    let said = ramped
        .iter()
        .find(|p| p.contains("must not go straight down"))
        .unwrap_or_else(|| panic!("{ramped:?}"));
    assert!(said.contains("ramps in instead in 1 place"), "{said}");
    assert!(
        !ramped.iter().any(|p| p.contains("another way than")),
        "{ramped:?}"
    );
    // a V-carve groove too short for its zig-zag plunges: listed, but not a warning
    let short = Fallback {
        wanted: EntryMode::ZigZag,
        ..fallback(EntryMode::Plunge)
    };
    let carved = problems(&doc, &carve(vec![short]));
    let said = carved
        .iter()
        .find(|p| p.contains("straight down"))
        .unwrap_or_else(|| panic!("{carved:?}"));
    assert!(
        said.contains("too short for its zig-zag") && said.contains("nothing to fix"),
        "{said}"
    );
    assert!(!said.contains("hard on the bit"), "{said}");
}

#[test]
fn surfacing_bits_fly_cutters_and_form_tools_cannot_plunge() {
    use cut_craft_core::toolpath::{Tool, ToolShape};
    let named = |name: &str| Tool {
        name: name.into(),
        ..Tool::default()
    };
    for name in [
        "1\" Spoilboard Surfacing Bit",
        "Fly cutter 2in",
        "Flycutter",
        "Slab Planer 50 mm",
        "Surfacing bit",
    ] {
        assert!(named(name).cannot_plunge(), "{name}");
    }
    assert!(!named("1/4\" End Mill").cannot_plunge());
    let form = Tool {
        shape: ToolShape::FormTool,
        ..Tool::default()
    };
    assert!(form.cannot_plunge());
    let surfacing = Tool {
        name: "Wide bit".into(),
        shape: ToolShape::SurfacingBit,
        ..Tool::default()
    };
    assert!(surfacing.cannot_plunge());
}

#[test]
fn a_bit_that_cannot_plunge_ramps_instead_and_says_so() {
    let square = rect(0.0, 0.0, 40.0, 40.0);
    let entry = Entry {
        no_plunge: true,
        ..entering(mode(EntryMode::Plunge), None)
    };
    let moves = path_moves(&square, true, &[-3.0, -6.0], 0.0, SAFE, &entry);
    assert!(steepest_below(&moves, 0.0) < 89.0, "never straight down");
    let fell = notes(&entry);
    assert!(
        fell.iter()
            .all(|f| f.wanted == EntryMode::Plunge && f.used == EntryMode::Ramp),
        "{fell:?}"
    );
    assert_eq!(fell.len(), 2, "one each pass");
}

#[test]
fn a_bit_that_cannot_plunge_zig_zags_where_a_ramp_will_not_fit() {
    let line = [Point::new(0.0, 0.0), Point::new(5.0, 0.0)];
    let entry = Entry {
        no_plunge: true,
        ..entering(mode(EntryMode::Plunge), None)
    };
    let moves = path_moves(&line, false, &[-3.0], 0.0, SAFE, &entry);
    assert!(steepest_below(&moves, 0.0) < 89.0);
    assert_eq!(notes(&entry)[0].used, EntryMode::ZigZag);
}
