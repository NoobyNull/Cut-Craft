use cut_craft_core::document::Job;
use cut_craft_core::geometry::Point;
use cut_craft_core::glam::DVec3;
use cut_craft_core::simulation::Material;
use cut_craft_core::toolpath::{Move, MoveKind, Tool, ToolShape, Toolpath};

/// A 100 x 100 x 20 mm job in half-millimetre cells.
fn material() -> Material {
    let job = Job {
        width: 100.0,
        height: 100.0,
        ..Job::default()
    };
    Material::new(&job, 200)
}

/// A 6 mm end mill following `points` (x, y, z) in order.
fn toolpath(points: &[(f64, f64, f64)]) -> Toolpath {
    with_tool(Tool::default(), points)
}

fn with_tool(tool: Tool, points: &[(f64, f64, f64)]) -> Toolpath {
    Toolpath {
        name: "test".into(),
        strategy: "test".into(),
        params: serde_json::Value::Null,
        tool,
        vectors: vec![],
        moves: points
            .iter()
            .map(|&(x, y, z)| Move {
                kind: MoveKind::Cut,
                to: DVec3::new(x, y, z),
            })
            .collect(),
        visible: true,
        power: vec![],
        stale: false,
    }
}

fn depth_at(m: &Material, x: f64, y: f64) -> f64 {
    m.at(Point::new(x, y)).expect("inside the material")
}

#[test]
fn new_material_is_uncut_and_covers_the_job() {
    let m = material();
    assert_eq!((m.columns, m.rows), (200, 200));
    assert!((m.cell - 0.5).abs() < 1e-12);
    assert!(m.heights.iter().all(|&z| z == 0.0));
    assert_eq!(m.at(Point::new(-1.0, 50.0)), None);
    assert_eq!(m.at(Point::new(50.0, 100.5)), None);
}

#[test]
fn the_longer_side_gets_the_cells_and_the_shorter_side_keeps_them_square() {
    let m = Material::new(&Job::default(), 300); // 600 x 400
    assert_eq!((m.columns, m.rows), (300, 200));
}

#[test]
fn a_plunge_leaves_a_hole_the_size_of_the_tool() {
    let mut m = material();
    m.cut(&toolpath(&[
        (50.0, 50.0, 5.0),
        (50.0, 50.0, -3.0),
        (50.0, 50.0, 5.0),
    ]));
    assert_eq!(depth_at(&m, 50.0, 50.0), -3.0);
    assert_eq!(depth_at(&m, 52.0, 50.0), -3.0, "inside the 3 mm radius");
    assert_eq!(depth_at(&m, 54.0, 50.0), 0.0, "outside it");
}

#[test]
fn a_cut_leaves_a_slot_with_round_ends() {
    let mut m = material();
    m.cut(&toolpath(&[(10.0, 50.0, -2.0), (90.0, 50.0, -2.0)]));
    assert_eq!(depth_at(&m, 50.0, 52.0), -2.0);
    assert_eq!(depth_at(&m, 50.0, 54.0), 0.0);
    assert_eq!(depth_at(&m, 92.0, 50.0), -2.0, "the round end");
    assert_eq!(
        depth_at(&m, 92.5, 52.5),
        0.0,
        "beyond the round end's corner"
    );
}

#[test]
fn a_ramp_cuts_to_the_lowest_point_of_the_tool_over_each_spot() {
    let mut m = material();
    m.cut(&toolpath(&[(10.0, 50.0, 0.0), (90.0, 50.0, -8.0)]));
    // over x = 50 the tool is there from x = 47 to 53 along the ramp, deepest at 53
    let z = depth_at(&m, 50.0, 50.0);
    assert!((-4.5..=-4.2).contains(&z), "{z}");
}

#[test]
fn nothing_cuts_below_the_bottom_of_the_material() {
    let mut m = material();
    m.cut(&toolpath(&[(50.0, 50.0, 5.0), (50.0, 50.0, -30.0)]));
    assert_eq!(depth_at(&m, 50.0, 50.0), -20.0);
}

#[test]
fn moves_above_the_material_cut_nothing() {
    let mut m = material();
    m.cut(&toolpath(&[(10.0, 10.0, 5.0), (90.0, 90.0, 5.0)]));
    assert!(m.heights.iter().all(|&z| z == 0.0));
}

fn shaped(shape: ToolShape) -> Tool {
    Tool {
        shape,
        angle: 90.0,
        ..Tool::default()
    }
}

#[test]
fn a_ball_nose_plunge_leaves_a_round_dimple() {
    let mut m = material();
    let plunge = [(50.0, 50.0, 5.0), (50.0, 50.0, -3.0), (50.0, 50.0, 5.0)];
    m.cut(&with_tool(shaped(ToolShape::BallNose), &plunge));
    assert!((depth_at(&m, 50.0, 50.0) + 3.0).abs() < 0.05);
    // the cell centred 2.25 mm out (and 0.25 up): 3 - sqrt(9 - r^2) above the tip
    let r = (2.25f64.powi(2) + 0.25f64.powi(2)).sqrt();
    let expected = -3.0 + 3.0 - (9.0 - r * r).sqrt();
    assert!((depth_at(&m, 52.0, 50.0) - expected).abs() < 1e-9);
    assert_eq!(depth_at(&m, 54.0, 50.0), 0.0);
}

#[test]
fn a_v_bit_groove_is_as_wide_as_it_is_deep_at_90_degrees() {
    let mut m = material();
    m.cut(&with_tool(
        shaped(ToolShape::VBit),
        &[(10.0, 50.0, -2.0), (90.0, 50.0, -2.0)],
    ));
    // cell centres 0.25 mm above y: the 90 degree cone rises 1 mm per mm out
    assert!((depth_at(&m, 50.0, 50.0) + 1.75).abs() < 1e-9);
    assert!((depth_at(&m, 50.0, 51.0) + 0.75).abs() < 1e-9);
    assert_eq!(
        depth_at(&m, 50.0, 53.0),
        0.0,
        "past where the cone leaves the surface"
    );
}

#[test]
fn a_tapered_tool_is_held_off_a_peak_by_its_taper() {
    let mut m = material(); // half-millimetre cells
    let peak = Point::new(50.25, 50.25);
    let i = 100 * m.columns + 100;
    m.heights[i] = 5.0;
    let tapered = Tool {
        shape: ToolShape::BallNose,
        diameter: 1.0,
        taper: 30.0,
        shank_diameter: 8.0,
        ..Tool::default()
    };
    // 3 mm away the ball is clear of the peak, but the taper is not
    let beside = peak + Point::new(3.0, 0.0);
    let held = m.drop_tool(&tapered, beside);
    assert!(
        (held - (5.0 - tapered.profile(3.0).unwrap())).abs() < 1e-12,
        "{held}"
    );
    assert!(held > 0.0);
    // as far as the whole tool reaches, the cut shows the taper too
    let mut cut = material();
    cut.cut(&with_tool(
        tapered.clone(),
        &[
            (50.25, 50.25, 5.0),
            (50.25, 50.25, -8.0),
            (50.25, 50.25, 5.0),
        ],
    ));
    let edge = depth_at(&cut, 52.75, 50.25);
    assert!(
        (edge - (-8.0 + tapered.profile(2.5).unwrap())).abs() < 1e-9,
        "{edge}"
    );
}

#[test]
fn a_dropped_tool_rests_on_the_highest_thing_under_it() {
    let mut m = material(); // half-millimetre cells
    let peak = Point::new(50.25, 50.25); // a cell centre
    let i = 100 * m.columns + 100;
    m.heights[i] = 5.0;
    let ball = shaped(ToolShape::BallNose); // 6 mm
    let end_mill = Tool::default();
    // straight over the peak every tool sits on it
    assert_eq!(m.drop_tool(&end_mill, peak), 5.0);
    assert_eq!(m.drop_tool(&ball, peak), 5.0);
    // 2 mm to the side the flat end still sits on it; the ball's curve is lower there
    let beside = peak + Point::new(2.0, 0.0);
    assert_eq!(m.drop_tool(&end_mill, beside), 5.0);
    assert!((m.drop_tool(&ball, beside) - (5.0 - (3.0 - 5f64.sqrt()))).abs() < 1e-12);
    // out of reach, the tool sits on the flat surface
    assert_eq!(m.drop_tool(&ball, peak + Point::new(4.0, 0.0)), 0.0);
}

#[test]
fn overcut_is_how_far_a_move_would_cut_below_the_surface() {
    let mut m = material();
    m.heights[100 * m.columns + 100] = 5.0; // one tall cell at (50.25, 50.25)
    let tool = Tool::default(); // 6 mm end mill
    let at = |x: f64, z: f64| DVec3::new(x, 50.25, z);
    assert!((m.overcut(&tool, at(40.0, 1.0), at(60.0, 1.0)) - 4.0).abs() < 1e-12);
    assert!(m.overcut(&tool, at(40.0, 6.0), at(60.0, 6.0)) <= -1.0 + 1e-12);
    // climbing from 0 to 6 over 20 mm, the end mill reaches the cell at x = 47.25: z 2.175
    let climb = m.overcut(&tool, at(40.0, 0.0), at(60.0, 6.0));
    assert!((climb - (5.0 - 2.175)).abs() < 1e-9, "{climb}");
}

#[test]
fn zigzag_lines_alternate_merge_level_runs_and_never_overcut() {
    use cut_craft_core::toolpath::zigzag;
    let tool = Tool::default();
    let flat = zigzag(&material(), &tool, 10.0, 0.01, f64::NEG_INFINITY);
    // 10 lines, each reduced to its two ends
    assert_eq!(flat.len(), 20);
    assert!(
        flat[0].x < flat[1].x && flat[2].x > flat[3].x,
        "there and back"
    );
    assert!((flat[2].y - flat[0].y - 10.0).abs() < 1e-12);
    assert!(flat.iter().all(|p| p.z.abs() < 1e-12));
    let floored = zigzag(&material(), &tool, 10.0, 0.01, 2.0);
    assert!(
        floored.iter().all(|p| (p.z - 2.0).abs() < 1e-12),
        "nothing below the floor"
    );
    // a 5 mm block with vertical sides from x = 50 to 60
    let mut m = material();
    for row in 0..m.rows {
        for column in 100..120 {
            m.heights[row * m.columns + column] = 5.0;
        }
    }
    let points = zigzag(&m, &tool, 10.0, 0.01, f64::NEG_INFINITY);
    assert!(
        points
            .windows(2)
            .all(|w| m.overcut(&tool, w[0], w[1]) <= 0.01)
    );
    assert!(
        points.iter().any(|p| (p.z - 5.0).abs() < 1e-12),
        "it goes over the block"
    );
}

#[test]
fn a_ramping_radiused_end_mill_cuts_where_its_edge_reaches_lowest() {
    let mut m = material(); // half-millimetre cells
    let tool = Tool {
        shape: ToolShape::RadiusedEndMill,
        diameter: 6.0,
        tip_radius: 1.0,
        ..Tool::default()
    };
    // ramping down along y = 50.25, from z -1 at x = 20 to z -5 at x = 80
    let ramp = |x: f64| -1.0 - 4.0 * (x - 20.0) / 60.0;
    m.cut(&with_tool(
        tool.clone(),
        &[(20.0, 50.25, -1.0), (80.0, 50.25, -5.0)],
    ));
    // by brute force: the lowest the tool's edge reaches over a spot, tip sampled every micron
    let reference = |x: f64, y: f64| {
        (20_000..=80_000)
            .map(|i| f64::from(i) / 1000.0)
            .filter_map(|tip| {
                let d = Point::new(tip, 50.25).distance(Point::new(x, y));
                tool.profile(d).map(|rise| ramp(tip) + rise)
            })
            .fold(f64::INFINITY, f64::min)
    };
    for y in [50.25, 51.75, 52.75, 53.25] {
        let (cut, expected) = (depth_at(&m, 50.25, y), reference(50.25, y));
        assert!(
            (cut - expected).abs() < 1e-3,
            "y {y}: cut {cut}, expected {expected}"
        );
    }
}

/// `material` with a groove along y from x = 50 to `50 + width`, 2 mm deep.
fn grooved(width: f64) -> Material {
    let mut m = material();
    for row in 0..m.rows {
        for column in 0..m.columns {
            let x = m.centre(column, row).x;
            if (50.0..50.0 + width).contains(&x) {
                m.heights[row * m.columns + column] = -2.0;
            }
        }
    }
    m
}

#[test]
fn a_tool_leaves_what_its_cutting_edge_cannot_reach() {
    let model = grooved(1.0);
    let end_mill = Tool::default(); // 6 mm
    let left = model.left_by(&end_mill);
    assert_eq!(depth_at(&left, 20.0, 20.0), 0.0, "the flat is finished");
    assert_eq!(
        depth_at(&left, 50.25, 20.0),
        0.0,
        "a 1 mm groove is out of reach"
    );
    let wide = grooved(10.0);
    let left = wide.left_by(&end_mill);
    assert_eq!(
        depth_at(&left, 55.25, 20.0),
        -2.0,
        "a 10 mm groove's middle is reached"
    );
}

#[test]
fn rest_areas_ignore_small_spots_and_grow_by_the_boundary_offset() {
    use cut_craft_core::simulation::rest_areas;
    let model = material(); // half-millimetre cells
    let mut left = material();
    // material left over a 10 mm square and a 1.5 mm spot
    for row in 0..left.rows {
        for column in 0..left.columns {
            let p = left.centre(column, row);
            let square = (20.0..30.0).contains(&p.x) && (20.0..30.0).contains(&p.y);
            let spot = (70.0..71.5).contains(&p.x) && (70.0..71.5).contains(&p.y);
            if square || spot {
                left.heights[row * left.columns + column] = 1.0;
            }
        }
    }
    let rest = rest_areas(&model, &left, 2.0, 1.0);
    let inside = |x: f64, y: f64| rest(Point::new(x, y));
    assert!(inside(25.0, 25.0), "the square");
    assert!(inside(19.25, 25.0) && inside(30.75, 25.0), "grown by 1 mm");
    assert!(!inside(17.75, 25.0), "but not 2 mm");
    assert!(
        !inside(70.75, 70.75),
        "the spot is under the 2 mm minimum detail"
    );
}

#[test]
fn a_zigzag_within_areas_cuts_only_inside_them() {
    use cut_craft_core::toolpath::zigzag_within;
    let flat = material();
    let tool = Tool::default();
    // two strips: x under 30, and x from 60 to 70
    let inside = |p: Point| p.x < 30.0 || (60.0..70.0).contains(&p.x);
    let runs = zigzag_within(&flat, &tool, 3.0, 0.002, f64::NEG_INFINITY, &inside);
    assert_ne!(runs.len(), 0);
    for run in &runs {
        assert!(run.iter().all(|p| inside(p.truncate())), "{run:?}");
    }
    let lines = runs
        .iter()
        .filter(|run| run.iter().all(|p| p.x < 30.0))
        .count();
    assert_eq!(
        lines * 2,
        runs.len(),
        "each line is two runs, one per strip"
    );
}

#[test]
fn hatch_lines_cross_the_regions_leaving_holes_out() {
    use cut_craft_core::geometry::{Path, Point};
    use cut_craft_core::toolpath::hatch_lines;
    let square = |x: f64, s: f64| {
        Path::polyline(
            &[
                Point::new(x, x),
                Point::new(x + s, x),
                Point::new(x + s, x + s),
                Point::new(x, x + s),
            ],
            true,
        )
    };
    let regions = cut_craft_core::polygon::regions(&[square(0.0, 10.0), square(4.0, 2.0)]);
    let rows = hatch_lines(&regions, 1.0, 0.0);
    assert_eq!(rows.len(), 10);
    assert_eq!(rows[0].len(), 1, "below the hole: one piece");
    let through = &rows[5]; // y = 5.5
    assert_eq!(through.len(), 2, "either side of the hole: {through:?}");
    assert!((through[0].1.x - 4.0).abs() < 1e-9 && (through[1].0.x - 6.0).abs() < 1e-9);
    let upright = hatch_lines(&regions, 1.0, 90.0);
    let (a, b) = upright[0][0];
    assert!(
        (a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() > 9.0,
        "{:?}",
        upright[0]
    );
}
