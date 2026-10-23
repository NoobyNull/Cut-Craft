use aspire_core::document::Job;
use aspire_core::geometry::Point;
use aspire_core::glam::DVec3;
use aspire_core::simulation::Material;
use aspire_core::toolpath::{Move, MoveKind, Tool, ToolShape, Toolpath};

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
