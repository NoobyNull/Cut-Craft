use cut_craft_core::document::{Along, Datum, Document, Home, Job, Resolution, Rotary, ZZero};
use cut_craft_core::file::{load_job, save_job};
use cut_craft_core::geometry::{Point, Rect};
use cut_craft_core::simulation::Material;

fn sized(datum: Datum, offset: Point) -> Job {
    Job {
        width: 100.0,
        height: 50.0,
        datum,
        offset,
        ..Job::default()
    }
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Rect {
    Rect::new(Point::new(x0, y0), Point::new(x1, y1))
}

#[test]
fn the_datum_is_where_x0_y0_is_on_the_material() {
    assert_eq!(
        sized(Datum::BottomLeft, Point::ZERO).area(),
        rect(0.0, 0.0, 100.0, 50.0)
    );
    assert_eq!(
        sized(Datum::Centre, Point::ZERO).area(),
        rect(-50.0, -25.0, 50.0, 25.0)
    );
    assert_eq!(
        sized(Datum::TopRight, Point::ZERO).area(),
        rect(-100.0, -50.0, 0.0, 0.0)
    );
    assert_eq!(
        sized(Datum::BottomRight, Point::ZERO).area(),
        rect(-100.0, 0.0, 0.0, 50.0)
    );
    // with an offset the datum corner is at the offset instead of 0, 0
    assert_eq!(
        sized(Datum::TopLeft, Point::new(10.0, 20.0)).area(),
        rect(10.0, -30.0, 110.0, 20.0)
    );
}

#[test]
fn z_zero_on_the_machine_bed_lifts_every_height_by_the_thickness() {
    let surface = Job::default();
    assert!((surface.machine_z(-3.0) + 3.0).abs() < 1e-12);
    let bed = Job {
        z_zero: ZZero::MachineBed,
        ..Job::default()
    };
    assert!((bed.machine_z(-3.0) - (surface.thickness - 3.0)).abs() < 1e-12);
}

#[test]
fn the_z_zero_sits_on_top_or_a_thickness_under_it_on_the_bed() {
    let surface = Job::default();
    assert!(
        surface.zero_height().abs() < 1e-12,
        "zeroed on top: the surface"
    );
    let bed = Job {
        z_zero: ZZero::MachineBed,
        ..Job::default()
    };
    assert!(
        (bed.zero_height() + bed.thickness).abs() < 1e-12,
        "zeroed on the bed: the material's bottom, a thickness under the top"
    );
    let axis = Job {
        z_zero: ZZero::MachineBed,
        ..Job::rotary(100.0, 40.0, Along::X)
    };
    assert!(
        (axis.zero_height() + 20.0).abs() < 1e-12,
        "a rotary job zeroed on the axis: a radius under the top"
    );
}

#[test]
fn a_rotary_job_is_its_cylinder_unrolled_with_the_axis_under_it() {
    use std::f64::consts::PI;
    let along_x = Job::rotary(100.0, 40.0, Along::X);
    assert_eq!(along_x.rotary, Some(Rotary::round(40.0, Along::X)));
    assert!((along_x.width - 100.0).abs() < 1e-12 && (along_x.height - 40.0 * PI).abs() < 1e-9);
    assert!(
        (along_x.thickness - 20.0).abs() < 1e-12,
        "the depth to the axis is the radius"
    );
    let along_y = Job::rotary(100.0, 40.0, Along::Y);
    assert!((along_y.width - 40.0 * PI).abs() < 1e-9 && (along_y.height - 100.0).abs() < 1e-12);
    let axis = Job {
        z_zero: ZZero::MachineBed,
        ..along_x
    };
    assert!(
        (axis.machine_z(-2.0) - 18.0).abs() < 1e-12,
        "zeroed on the axis, 2 mm into a 20 mm radius is 18 up"
    );
    assert_eq!(
        Job::default().rotary,
        None,
        "jobs are flat unless set up rotary"
    );
}

#[test]
fn the_modeling_resolution_spreads_its_points_over_the_job() {
    let job = Job {
        width: 1000.0,
        height: 1000.0,
        ..Job::default()
    };
    // standard is a million points: here one per square millimetre
    assert!((job.model_cell() - 1.0).abs() < 1e-12);
    for (resolution, slower) in [(Resolution::High, 3.0), (Resolution::Highest, 7.0)] {
        let finer = Job { resolution, ..job };
        assert!((finer.model_cell() - 1.0 / f64::sqrt(slower)).abs() < 1e-12);
    }
}

#[test]
fn jobs_saved_before_datums_load_at_the_bottom_left_on_the_surface() {
    let mut json: serde_json::Value =
        serde_json::from_slice(&save_job(&Document::default())).unwrap();
    let job = json["document"]["job"].as_object_mut().unwrap();
    for key in ["z_zero", "datum", "offset", "resolution", "colour"] {
        assert!(job.remove(key).is_some(), "{key} is saved");
    }
    let old = load_job(&serde_json::to_vec(&json).unwrap()).unwrap();
    assert_eq!(old.job, Job::default());
}

#[test]
fn the_simulated_material_lies_where_the_job_is() {
    let job = Job {
        width: 100.0,
        height: 100.0,
        datum: Datum::Centre,
        ..Job::default()
    };
    let m = Material::new(&job, 100);
    assert_eq!(m.centre(0, 0), Point::new(-49.5, -49.5));
    assert_eq!(m.at(Point::new(-49.0, 49.0)), Some(0.0));
    assert_eq!(m.at(Point::new(60.0, 0.0)), None);
}

#[test]
fn older_jobs_plunge_from_the_clearance_and_start_at_x0_y0_at_it() {
    let doc = Document {
        job: Job {
            clearance: 12.0,
            ..Job::default()
        },
        ..Document::default()
    };
    let mut json: serde_json::Value = serde_json::from_slice(&save_job(&doc)).unwrap();
    let job = json["document"]["job"].as_object_mut().unwrap();
    assert!(!job.contains_key("plunge") && !job.contains_key("home"));
    let old = load_job(&serde_json::to_vec(&json).unwrap()).unwrap().job;
    assert!((old.plunge_gap() - 12.0).abs() < 1e-12, "no rapid plunges");
    assert_eq!(
        old.home(),
        Home {
            x: 0.0,
            y: 0.0,
            z: 12.0
        }
    );
    let set = Job {
        plunge: Some(2.0),
        home: Some(Home {
            x: 10.0,
            y: 20.0,
            z: 30.0,
        }),
        ..old
    };
    assert!((set.plunge_gap() - 2.0).abs() < 1e-12);
    assert!((set.home().x - 10.0).abs() < 1e-12);
}

#[test]
fn a_square_or_a_board_on_the_rotary_swings_further_than_its_top() {
    use cut_craft_core::document::Blank;
    let round = Rotary::round(40.0, Along::X);
    assert!((round.swing() - 20.0).abs() < 1e-12);
    let square = Rotary {
        blank: Blank::Square,
        ..round
    };
    assert!((square.swing() - 20.0 * std::f64::consts::SQRT_2).abs() < 1e-9);
    let board = Rotary {
        blank: Blank::Board { width: 150.0 },
        ..Rotary::round(20.0, Along::X)
    };
    assert!((board.swing() - 75f64.hypot(10.0)).abs() < 1e-9);
    let mut job = Job::rotary(100.0, 20.0, Along::X);
    assert!(
        (job.turning_gap() - job.clearance).abs() < 1e-12,
        "round: just the gap"
    );
    job.rotary = Some(board);
    assert!((job.turning_gap() - (job.clearance + board.swing() - 10.0)).abs() < 1e-9);
    // a job saved before shapes is round
    let old: Rotary = serde_json::from_str(r#"{"diameter":40.0,"along":"x"}"#).unwrap();
    assert_eq!(old, round);
    let saved = serde_json::to_string(&board).unwrap();
    assert_eq!(serde_json::from_str::<Rotary>(&saved).unwrap(), board);
}
