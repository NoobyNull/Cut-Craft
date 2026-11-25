use super::*;
use cut_craft_core::geometry::Rect;
use cut_craft_core::testing::{cut_bounds, made_toolpath, square};
use cut_craft_core::toolpath::{MoveKind, Toolpath};

/// The default settings going straight down, so the cut itself is what is tested.
fn plain() -> Params {
    Params {
        entry: Some(EntrySettings {
            mode: cut_craft_core::entry::EntryMode::Plunge,
            ..EntrySettings::default()
        }),
        ..Params::default()
    }
}

cut_craft_core::contract_tests!(ProfileToolpath);

fn profile(side: Side, square_side: f64) -> Result<Toolpath, String> {
    made_toolpath::<ProfileToolpath>(
        vec![square(0.0, 0.0, square_side)],
        &Params { side, ..plain() },
    )
}

fn near(a: Rect, min: (f64, f64), max: (f64, f64)) -> bool {
    a.min.distance(Point::new(min.0, min.1)) < 0.02
        && a.max.distance(Point::new(max.0, max.1)) < 0.02
}

#[test]
fn outside_runs_a_tool_radius_outside_the_shape() {
    let b = cut_bounds(&profile(Side::Outside, 10.0).unwrap());
    assert!(near(b, (-3.0, -3.0), (13.0, 13.0)), "{b:?}");
}

#[test]
fn inside_runs_a_tool_radius_inside_the_shape() {
    let b = cut_bounds(&profile(Side::Inside, 10.0).unwrap());
    assert!(near(b, (3.0, 3.0), (7.0, 7.0)), "{b:?}");
}

#[test]
fn on_follows_the_line_itself() {
    let b = cut_bounds(&profile(Side::On, 10.0).unwrap());
    assert!(near(b, (0.0, 0.0), (10.0, 10.0)), "{b:?}");
}

#[test]
fn cuts_in_passes_down_to_the_depth() {
    let tp = profile(Side::Outside, 10.0).unwrap();
    let plunges: Vec<f64> = tp
        .moves
        .iter()
        .filter(|m| m.kind == MoveKind::Plunge)
        .map(|m| m.to.z)
        .collect();
    assert_eq!(plunges, [-3.0, -6.0], "6 mm deep in 3 mm passes");
}

#[test]
fn cut_right_through_goes_to_the_bottom_of_the_material_whatever_its_thickness() {
    let through = Params {
        through: true,
        ..plain()
    };
    let deepest = |thickness: f64| {
        let mut doc = Document::default();
        doc.job.thickness = thickness;
        let mut c = Change::builder(&doc);
        let id = c.add(square(0.0, 0.0, 10.0));
        c.select([id]);
        doc.apply(&c.finish()).unwrap();
        let change = ProfileToolpath::run(&doc, &through).unwrap();
        doc.apply(&change).unwrap();
        let tp = doc.toolpaths.values().next().unwrap();
        tp.moves.iter().map(|m| -m.to.z).fold(0.0, f64::max)
    };
    assert!((deepest(20.0) - 20.0).abs() < 1e-9, "{}", deepest(20.0));
    assert!((deepest(12.5) - 12.5).abs() < 1e-9, "follows the thickness");
    assert!((through.cut_depth(18.0) - 18.0).abs() < 1e-9);
    assert!((Params::default().cut_depth(18.0) - 6.0).abs() < 1e-9);
}

#[test]
fn the_side_is_on_the_form_under_the_depth_with_a_picture_for_each_choice() {
    let schema = schemars::schema_for!(Params);
    let schema = schema.as_value();
    let side = &schema["properties"]["side"];
    assert_eq!(side["x-group"], SIDE_GROUP);
    assert_eq!(side["x-widget"], "named-icons");
    assert!(
        schema["properties"].get("through").is_none(),
        "a cut past the material is warned of on Calculate instead"
    );
}

#[test]
fn the_tab_settings_are_in_edit_tabs_not_on_the_form() {
    let schema = schemars::schema_for!(Params);
    let schema = schema.as_value();
    assert_eq!(schema["properties"]["tabs"]["x-group"], "Tabs");
    assert!(
        schema["properties"].get("tab").is_none(),
        "the size and spacing are set in the Edit Tabs window"
    );
    // still kept with the toolpath
    let tp = profile(Side::Outside, 10.0).unwrap();
    assert_eq!(tp.params["tab"]["spacing"], "automatic");
}

#[test]
fn remembers_how_it_was_made_so_it_can_be_recalculated() {
    let tp = profile(Side::Inside, 10.0).unwrap();
    assert_eq!(tp.strategy, "profile_toolpath");
    assert_eq!(tp.params["side"], "inside");
    assert_eq!(tp.vectors.len(), 1);
}

#[test]
fn a_shape_smaller_than_the_tool_cannot_be_cut_inside() {
    assert_eq!(
        profile(Side::Inside, 4.0).unwrap_err(),
        "The 6 mm End Mill does not fit inside the selected shapes. Pick a smaller bit, \
             or cut Outside instead."
    );
}

#[test]
fn leads_come_from_outside_for_an_outside_profile_and_inside_for_an_inside_one() {
    for (side, from_outside) in [(Side::Outside, true), (Side::Inside, false)] {
        let tp = made_toolpath::<ProfileToolpath>(
            vec![square(0.0, 0.0, 20.0)],
            &Params {
                side,
                lead: Lead::Straight,
                leads: Leads {
                    angle: 90.0,
                    length: 2.0,
                    ..Leads::default()
                },
                ..plain()
            },
        )
        .unwrap();
        let plunge = tp
            .moves
            .iter()
            .find(|m| m.kind == MoveKind::Plunge)
            .unwrap();
        let p = plunge.to.truncate();
        let inside_square = p.x > 0.0 && p.x < 20.0 && p.y > 0.0 && p.y < 20.0;
        let off_line = !cut_bounds(&tp).contains(p) || inside_square;
        assert!(off_line, "{side:?}: {p}");
        assert_eq!(!inside_square, from_outside, "{side:?}: {p}");
    }
}

#[test]
fn tabs_are_left_at_the_bottom_of_the_cut() {
    let tp = made_toolpath::<ProfileToolpath>(
        vec![square(0.0, 0.0, 40.0)],
        &Params {
            tabs: Tabs::Flat,
            ramp: Ramp::Smooth,
            entry: None,
            ..plain()
        },
    )
    .unwrap();
    let sized = TabSettings::default().sized(Tool::default().pass_depth, 6.0);
    let tab_top = -6.0 + sized.thickness;
    assert!(
        tp.moves
            .iter()
            .any(|m| m.kind == MoveKind::Cut && (m.to.z - tab_top).abs() < 1e-9)
    );
    assert!(tp.moves.iter().any(|m| (m.to.z + 6.0).abs() < 1e-9));
}

/// The way round the first closed pass at full depth runs: true anticlockwise.
/// How far from the 40 mm square's edge each cutting move at `z` ends, nearest and
/// furthest.
fn reach_at(tp: &Toolpath, z: f64) -> (f64, f64) {
    let edge = [
        Point::new(0.0, 0.0),
        Point::new(40.0, 0.0),
        Point::new(40.0, 40.0),
        Point::new(0.0, 40.0),
        Point::new(0.0, 0.0),
    ];
    tp.moves
        .iter()
        .filter(|m| m.kind == MoveKind::Cut && (m.to.z - z).abs() < 1e-9)
        .map(|m| polyline_distance(&edge, m.to.truncate()))
        .fold((f64::MAX, 0.0), |(lo, hi), d| (lo.min(d), hi.max(d)))
}

#[test]
fn loops_keep_turning_anticlockwise_on_a_pass_back_unless_the_job_alternates() {
    use cut_craft_core::geometry::Point;
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
        depth: 9.0,
        motion: Motion::Trochoidal,
        ways: Ways::Both,
        ..plain()
    };
    let made = |way_back| {
        let mut doc = cut_craft_core::testing::document_with(vec![square(0.0, 0.0, 40.0)]);
        doc.job.clearing.way_back = way_back;
        let change = ProfileToolpath::run(&doc, &p).unwrap();
        doc.apply(&change).unwrap();
        doc.toolpaths.into_values().next().unwrap()
    };
    let kept = made(cut_craft_core::entry::WayBack::KeepClimb);
    assert!(bending(&kept, -3.0) > 0.0 && bending(&kept, -6.0) > 0.0);
    let alternate = made(cut_craft_core::entry::WayBack::Alternate);
    assert!(bending(&alternate, -3.0) > 0.0 && bending(&alternate, -6.0) < 0.0);
}

#[test]
fn trochoidal_loops_in_the_waste_then_the_line_straight() {
    let p = Params {
        depth: 6.0,
        motion: Motion::Trochoidal,
        ..plain()
    };
    let tp = made_toolpath::<ProfileToolpath>(vec![square(0.0, 0.0, 40.0)], &p).unwrap();
    let r = p.tool.diameter / 2.0;
    let (loop_r, _) = p.loops.sized(p.tool.diameter);
    // above the bottom only loops: never into the part, out to two loop radii past the line
    let (lo, hi) = reach_at(&tp, -3.0);
    assert!(lo >= r - 0.02, "into the part: {lo}");
    assert!(
        (hi - (r + 2.0 * loop_r)).abs() < 0.1,
        "the loops reach {hi}"
    );
    // at the bottom the loops, then the line itself
    let (lo, _) = reach_at(&tp, -6.0);
    assert!((lo - r).abs() < 0.02, "the line: {lo}");
    // with tabs the loops stop at the tabs' tops and the line takes the passes below
    let tabbed = Params {
        tabs: Tabs::Flat,
        ..p.clone()
    };
    let tp = made_toolpath::<ProfileToolpath>(vec![square(0.0, 0.0, 40.0)], &tabbed).unwrap();
    let (_, hi) = reach_at(&tp, -6.0);
    assert!(hi < r + 0.02, "no loops past the tabs' tops: {hi}");
}

#[test]
fn both_directions_take_turns_climb_then_conventional() {
    let p = Params {
        depth: 9.0,
        ways: Ways::Both,
        ..plain()
    };
    let tp = made_toolpath::<ProfileToolpath>(vec![square(0.0, 0.0, 40.0)], &p).unwrap();
    let turning = |z: f64| -> f64 {
        tp.moves
            .windows(2)
            .filter(|w| w[1].kind == MoveKind::Cut)
            .filter(|w| w.iter().all(|m| (m.to.z - z).abs() < 1e-9))
            .map(|w| w[0].to.truncate().perp_dot(w[1].to.truncate()))
            .sum()
    };
    // outside a part, climb is clockwise
    let ways = [turning(-3.0), turning(-6.0), turning(-9.0)];
    assert!(ways[0] < 0.0 && ways[1] > 0.0 && ways[2] < 0.0, "{ways:?}");
}

fn anticlockwise(tp: &Toolpath) -> bool {
    let cuts: Vec<Point> = tp
        .moves
        .iter()
        .filter(|m| m.kind == MoveKind::Cut)
        .map(|m| m.to.truncate())
        .collect();
    winding(&cuts) > 0.0
}

#[test]
fn climb_runs_clockwise_round_a_part_and_anticlockwise_round_a_hole() {
    for (side, direction, expect) in [
        (Side::Outside, Direction::Climb, false),
        (Side::Outside, Direction::Conventional, true),
        (Side::Inside, Direction::Climb, true),
        (Side::Inside, Direction::Conventional, false),
    ] {
        let tp = made_toolpath::<ProfileToolpath>(
            vec![square(0.0, 0.0, 10.0)],
            &Params {
                side,
                direction,
                ..plain()
            },
        )
        .unwrap();
        assert_eq!(anticlockwise(&tp), expect, "{side:?} {direction:?}");
    }
}

#[test]
fn on_the_line_climb_runs_clockwise_round_a_closed_shape_however_it_was_drawn() {
    let drawn = square(0.0, 0.0, 10.0);
    let mut backwards = cut_points(&drawn);
    backwards.reverse();
    let backwards = Path::polyline(&backwards, true);
    for shape in [drawn, backwards] {
        for (direction, expect) in [(Direction::Climb, false), (Direction::Conventional, true)] {
            let tp = made_toolpath::<ProfileToolpath>(
                vec![shape.clone()],
                &Params {
                    side: Side::On,
                    direction,
                    ..plain()
                },
            )
            .unwrap();
            assert_eq!(anticlockwise(&tp), expect, "{direction:?}");
        }
    }
}

#[test]
fn the_allowance_leaves_the_part_bigger() {
    let tp = made_toolpath::<ProfileToolpath>(
        vec![square(0.0, 0.0, 10.0)],
        &Params {
            allowance: 1.0,
            ..plain()
        },
    )
    .unwrap();
    let b = cut_bounds(&tp);
    assert!(near(b, (-4.0, -4.0), (14.0, 14.0)), "{b:?}");
}

#[test]
fn clicked_tabs_sit_where_clicked_on_the_path() {
    let tp = made_toolpath::<ProfileToolpath>(
        vec![square(0.0, 0.0, 40.0)],
        &Params {
            tabs: Tabs::Flat,
            tab: TabSettings {
                spacing: TabSpacing::Placed,
                size_from_tool: false,
                length: 10.0,
                thickness: 2.0,
                ..TabSettings::default()
            },
            // on the vector's bottom edge, 10 mm in
            tab_points: vec![Point::new(10.0, 0.0)],
            ..plain()
        },
    )
    .unwrap();
    let top = -6.0 + 2.0;
    let lifted: Vec<Point> = tp
        .moves
        .iter()
        .filter(|m| (m.to.z - top).abs() < 1e-9)
        .map(|m| m.to.truncate())
        .collect();
    // one tab, lifted over its length and the tool, round x = 10 on the line 3 mm out
    assert!(lifted.len() > 1, "lifted over the tab");
    assert!(
        lifted
            .iter()
            .all(|p| (p.y + 3.0).abs() < 1e-6 && (p.x - 10.0).abs() <= 8.0 + 1e-6),
        "{lifted:?}"
    );
    let none = made_toolpath::<ProfileToolpath>(
        vec![square(0.0, 0.0, 40.0)],
        &Params {
            tabs: Tabs::Flat,
            tab: TabSettings {
                spacing: TabSpacing::Placed,
                ..TabSettings::default()
            },
            ..plain()
        },
    );
    assert!(none.unwrap_err().contains("Click"));
}

/// The tabs a profile round a `w` x `h` rectangle leaves, cut `depth` deep with a 1/4"
/// bit in 1/4" passes: (how many, their thickness, lifted length on the centre line).
fn tabs_on(w: f64, h: f64, depth: f64) -> (usize, f64, f64) {
    tabs_with(w, h, depth, cut_craft_core::entry::TabSettings::default())
}

fn tabs_with(
    w: f64,
    h: f64,
    depth: f64,
    tab: cut_craft_core::entry::TabSettings,
) -> (usize, f64, f64) {
    let quarter = 6.35;
    let tool = Tool {
        diameter: quarter,
        pass_depth: quarter,
        ..Tool::default()
    };
    let tp = made_toolpath::<ProfileToolpath>(
        vec![cut_craft_core::geometry::Path::polyline(
            &[(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)].map(|(x, y)| Point::new(x, y)),
            true,
        )],
        &Params {
            depth,
            tool,
            tabs: Tabs::Flat,
            tab,
            ..plain()
        },
    )
    .unwrap();
    // the last pass: straight up onto each tab and straight down off it
    let ups: Vec<(Point, f64)> = tp
        .moves
        .windows(2)
        .filter(|w| {
            w[1].kind == MoveKind::Cut
                && w[0].to.truncate() == w[1].to.truncate()
                && w[1].to.z > w[0].to.z
                && (w[0].to.z + depth).abs() < 1e-9
        })
        .map(|w| (w[1].to.truncate(), w[1].to.z))
        .collect();
    let downs: Vec<Point> = tp
        .moves
        .windows(2)
        .filter(|w| {
            w[1].kind == MoveKind::Cut
                && w[0].to.truncate() == w[1].to.truncate()
                && w[1].to.z < w[0].to.z
                && (w[1].to.z + depth).abs() < 1e-9
        })
        .map(|w| w[1].to.truncate())
        .collect();
    let thickness = ups.first().map_or(0.0, |&(_, z)| z + depth);
    let length = ups
        .first()
        .zip(downs.first())
        .map_or(0.0, |((a, _), b)| a.distance(*b));
    (ups.len(), thickness, length)
}

#[test]
fn tabs_are_sized_from_the_tool_and_spread_about_every_150_mm() {
    // 3/4" deep: 1/4" thick, 1/2" long (lifted over that and the bit's 1/4")
    let (count, thickness, length) = tabs_on(400.0, 800.0, 19.05);
    assert!((thickness - 6.35).abs() < 1e-6, "{thickness}");
    assert!((length - 3.0 * 6.35).abs() < 1e-6, "{length}");
    // round 2,440 mm of centre line: about one every 150 mm
    assert_eq!(count, 16);
    // a small part still gets three, and a shallow cut thinner tabs
    let (count, thickness, _) = tabs_on(60.0, 60.0, 6.0);
    assert_eq!(count, 3);
    assert!((thickness - 3.0).abs() < 1e-6, "half the cut: {thickness}");
    // a tiny one is not all tab
    assert_eq!(tabs_on(15.0, 15.0, 19.05).0, 1);
}

#[test]
fn tabs_by_distance_keep_between_the_minimum_and_maximum_number() {
    use cut_craft_core::entry::{TabSettings, TabSpacing};
    let by = |distance: f64, minimum: u32, maximum: u32| TabSettings {
        spacing: TabSpacing::ConstantDistance,
        distance,
        minimum,
        maximum,
        ..TabSettings::default()
    };
    // 2,440 mm round at one every 100 mm wants 24
    assert_eq!(tabs_with(400.0, 800.0, 19.05, by(100.0, 1, 100)).0, 24);
    assert_eq!(tabs_with(400.0, 800.0, 19.05, by(100.0, 2, 8)).0, 8);
    // a small part wanting one gets the minimum
    assert_eq!(tabs_with(60.0, 60.0, 19.05, by(500.0, 2, 8)).0, 2);
}

/// The cutting moves of each path, in the order the paths are cut: each starts where the
/// tool comes down from a rapid.
fn cuts(tp: &Toolpath) -> Vec<Vec<Point>> {
    let mut out: Vec<Vec<Point>> = vec![];
    for w in tp.moves.windows(2) {
        if w[0].kind == MoveKind::Rapid && w[1].kind == MoveKind::Plunge {
            out.push(vec![w[1].to.truncate()]);
        } else if w[1].kind == MoveKind::Cut {
            out.last_mut().unwrap().push(w[1].to.truncate());
        }
    }
    out
}

fn box_of(points: &[Point]) -> Rect {
    points
        .iter()
        .map(|&q| Rect::point(q))
        .reduce(Rect::union)
        .unwrap()
}

fn profiled(paths: Vec<Path>, p: &Params) -> Toolpath {
    made_toolpath::<ProfileToolpath>(paths, p).unwrap()
}

/// A 30 mm square with a 10 mm square hole in its middle, like the letter "O".
fn o_shape() -> Vec<Path> {
    vec![square(0.0, 0.0, 30.0), square(10.0, 10.0, 10.0)]
}

#[test]
fn an_o_is_cut_on_the_right_side_of_both_rings_the_inner_one_first() {
    for (side, inner, outer) in [
        (Side::Outside, (13.0, 17.0), (-3.0, 33.0)),
        (Side::Inside, (7.0, 23.0), (3.0, 27.0)),
    ] {
        let tp = profiled(o_shape(), &Params { side, ..plain() });
        let cuts = cuts(&tp);
        assert_eq!(cuts.len(), 2, "{side:?}");
        let (a, b) = (box_of(&cuts[0]), box_of(&cuts[1]));
        assert!(
            near(a, (inner.0, inner.0), (inner.1, inner.1)),
            "{side:?} {a:?}"
        );
        assert!(
            near(b, (outer.0, outer.0), (outer.1, outer.1)),
            "{side:?} {b:?}"
        );
        // climb: anticlockwise round the waste inside a ring, clockwise round a part
        let waste_inside = side == Side::Outside;
        assert_eq!(winding(&cuts[0]) > 0.0, waste_inside, "{side:?}");
        assert_eq!(winding(&cuts[1]) > 0.0, !waste_inside, "{side:?}");
    }
}

/// The middle x (or y, `across` false) of each path cut, in order, with these orderings.
fn cut_order(paths: Vec<Path>, order: Order, across: bool) -> Vec<f64> {
    let tp = profiled(paths, &Params { order, ..plain() });
    cuts(&tp)
        .iter()
        .map(|c| {
            let b = box_of(c);
            let mid = (b.min + b.max) / 2.0;
            (if across { mid.x } else { mid.y }).round()
        })
        .collect()
}

const NONE: Order = Order {
    vector_selection_order: false,
    left_to_right: false,
    bottom_to_top: false,
    grid: false,
    shortest_path: false,
};

#[test]
fn cuts_in_selection_order_left_to_right_or_bottom_to_top() {
    let row = || {
        vec![
            square(80.0, 0.0, 10.0),
            square(0.0, 0.0, 10.0),
            square(40.0, 0.0, 10.0),
        ]
    };
    let selection = Order {
        vector_selection_order: true,
        ..NONE
    };
    assert_eq!(cut_order(row(), selection, true), [85.0, 5.0, 45.0]);
    let left_to_right = Order {
        left_to_right: true,
        ..NONE
    };
    assert_eq!(cut_order(row(), left_to_right, true), [5.0, 45.0, 85.0]);
    let column = vec![
        square(0.0, 40.0, 10.0),
        square(0.0, 80.0, 10.0),
        square(0.0, 0.0, 10.0),
    ];
    let bottom_to_top = Order {
        bottom_to_top: true,
        ..NONE
    };
    assert_eq!(cut_order(column, bottom_to_top, false), [5.0, 45.0, 85.0]);
    // with both ticked the shorter wins, and so does the default
    let both = Order {
        vector_selection_order: true,
        left_to_right: true,
        ..NONE
    };
    assert_eq!(cut_order(row(), both, true), [5.0, 45.0, 85.0]);
    assert_eq!(cut_order(row(), Order::default(), true), [5.0, 45.0, 85.0]);
    // none ticked: as selected
    assert_eq!(cut_order(row(), NONE, true), [85.0, 5.0, 45.0]);
}

#[test]
fn a_grid_finishes_one_row_of_squares_before_the_next_turning_back_along_it() {
    let parts = vec![
        square(0.0, 40.0, 10.0),
        square(40.0, 0.0, 10.0),
        square(40.0, 40.0, 10.0),
        square(0.0, 0.0, 10.0),
    ];
    let grid = Order { grid: true, ..NONE };
    let tp = profiled(
        parts,
        &Params {
            order: grid,
            ..plain()
        },
    );
    let middles: Vec<(f64, f64)> = cuts(&tp)
        .iter()
        .map(|c| {
            let b = box_of(c);
            let mid = (b.min + b.max) / 2.0;
            (mid.x.round(), mid.y.round())
        })
        .collect();
    assert_eq!(
        middles,
        [(5.0, 5.0), (45.0, 5.0), (45.0, 45.0), (5.0, 45.0)]
    );
}

#[test]
fn inner_shapes_are_cut_first_whatever_the_order() {
    for order in [
        Order {
            vector_selection_order: true,
            ..NONE
        },
        Order {
            left_to_right: true,
            ..NONE
        },
        Order {
            shortest_path: true,
            ..NONE
        },
    ] {
        let mut paths = o_shape();
        // a separate part off to the left, selected last
        paths.push(square(-40.0, 0.0, 10.0));
        let tp = profiled(paths, &Params { order, ..plain() });
        let boxes: Vec<Rect> = cuts(&tp).iter().map(|c| box_of(c)).collect();
        let hole = boxes.iter().position(|b| b.min.x > 10.0).unwrap();
        let ring = boxes
            .iter()
            .position(|b| b.min.x < 0.0 && b.max.x > 30.0)
            .unwrap();
        assert!(hole < ring, "{order:?}");
    }
}

fn first_start(paths: Vec<Path>, start_at: StartAt, start_near: Anchor) -> Point {
    let tp = profiled(
        paths,
        &Params {
            start_at,
            start_near,
            ..plain()
        },
    );
    cuts(&tp)[0][0]
}

#[test]
fn keeps_the_vectors_own_start_point_moved_out_to_the_tool() {
    let p = Point::new;
    // a square starting half way along its bottom edge
    let drawn = Path::polyline(
        &[
            p(5.0, 0.0),
            p(10.0, 0.0),
            p(10.0, 10.0),
            p(0.0, 10.0),
            p(0.0, 0.0),
        ],
        true,
    );
    let at = first_start(vec![drawn], StartAt::KeepCurrent, Anchor::TopLeft);
    assert!(at.distance(p(5.0, -3.0)) < 1e-6, "{at}");
}

#[test]
fn optimized_starts_are_the_nearest_points_to_where_the_tool_is() {
    let p = Point::new;
    let corner = 20.0 - 3.0 * std::f64::consts::FRAC_1_SQRT_2;
    let at = first_start(
        vec![square(20.0, 20.0, 10.0)],
        StartAt::Optimize,
        Anchor::TopLeft,
    );
    assert!(
        at.distance(p(corner, corner)) < 0.05,
        "from the origin: {at}"
    );
    // the next part starts on its corner nearest the first's start, not its own start
    let tp = profiled(
        vec![square(20.0, 20.0, 10.0), square(60.0, 20.0, 10.0)],
        &plain(),
    );
    let cuts = cuts(&tp);
    let (first, second) = (cuts[0][0], cuts[1][0]);
    assert!(first.distance(p(corner, corner)) < 0.05, "{first}");
    assert!(
        second.x < 57.1 && (second.y - 20.0).abs() < 0.5,
        "{first} then {second}"
    );
}

#[test]
fn starts_at_the_span_end_nearest_the_chosen_point_of_the_bounding_box() {
    let at = first_start(
        vec![square(0.0, 0.0, 10.0)],
        StartAt::BoundingBox,
        Anchor::TopRight,
    );
    let corner = 10.0 + 3.0 * std::f64::consts::FRAC_1_SQRT_2;
    assert!(at.distance(Point::new(corner, corner)) < 0.3, "{at}");
    let at = first_start(
        vec![square(0.0, 0.0, 10.0)],
        StartAt::BoundingBox,
        Anchor::BottomLeft,
    );
    assert!(at.x < 0.0 && at.y < 0.0, "{at}");
}

#[test]
fn sharp_corners_follow_the_vectors_angle_out_to_a_point_within_the_limit() {
    let sharp = Params {
        sharp_corners: true,
        ..plain()
    };
    let tp = profiled(vec![square(0.0, 0.0, 10.0)], &sharp);
    let points: Vec<Point> = cuts(&tp).concat();
    assert!(
        points
            .iter()
            .any(|q| q.distance(Point::new(-3.0, -3.0)) < 1e-6)
    );
    assert!(near(cut_bounds(&tp), (-3.0, -3.0), (13.0, 13.0)));
    // an 11° tip would mitre out 30 mm; it stops at the limit
    let p = Point::new;
    let thin = Path::polyline(&[p(0.0, 0.0), p(50.0, 0.0), p(50.0, 10.0)], true);
    let tp = profiled(vec![thin], &sharp);
    let furthest = cuts(&tp)
        .concat()
        .iter()
        .filter(|q| q.x < 0.0)
        .map(|q| q.length())
        .fold(0.0, f64::max);
    assert!(
        furthest > 3.0 && furthest <= MITRE_REACH * 3.0 + 1e-3,
        "{furthest}"
    );
}

#[test]
fn projected_onto_the_model_it_follows_the_dome() {
    let mut doc = cut_craft_core::testing::domed();
    let mut c = Change::builder(&doc);
    let id = c.add(square(14.0, 14.0, 12.0));
    c.select(vec![id]);
    doc.apply(&c.finish()).unwrap();
    let p = Params {
        side: Side::On,
        depth: 1.0,
        project_onto_model: true,
        ..plain()
    };
    doc.apply(&ProfileToolpath::run(&doc, &p).unwrap()).unwrap();
    let tp = doc.toolpaths.values().next().unwrap();
    // the square's corners lie on the dome's slope: lower than its middles
    let at = |x: f64, y: f64| {
        tp.moves
            .iter()
            .filter(|m| m.kind == MoveKind::Cut)
            .find(|m| m.to.truncate().distance(Point::new(x, y)) < 0.2)
            .map(|m| m.to.z)
            .unwrap()
    };
    assert!(at(14.0, 14.0) < at(20.0, 14.0) - 0.5);
    assert!(
        at(20.0, 14.0) < -1.0,
        "a millimetre below the dome's surface"
    );
}
