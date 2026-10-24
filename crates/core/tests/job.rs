use cut_craft_core::document::{Datum, Document, Job, Resolution, ZZero};
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
