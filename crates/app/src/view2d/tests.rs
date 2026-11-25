#[test]
fn the_grid_thins_out_when_its_points_would_crowd() {
    // 10 mm apart at 2 points a mm is 20 points: drawn as it is
    assert_eq!(grid_step(10.0, 2.0, 8.0), Some(10.0));
    // at 0.3 points a mm only every third is 8 or more points apart
    assert_eq!(grid_step(10.0, 0.3, 8.0), Some(30.0));
    assert_eq!(grid_step(0.0, 1.0, 8.0), None);
    let job = cut_craft_core::geometry::Rect::new(Point::new(-5.0, 0.0), Point::new(20.0, 10.0));
    let points = grid_points(job, 10.0);
    assert_eq!(points.len(), 3 * 2, "x 0, 10, 20 by y 0, 10");
    assert!(points.contains(&Point::new(20.0, 10.0)));
}

use super::*;
use eframe::egui::{pos2, vec2};

#[test]
fn a_picture_edited_or_swapped_at_the_same_size_is_drawn_afresh() {
    let ctx = egui::Context::default();
    let picture = |grey: u8| cut_craft_core::bitmap::Bitmap {
        name: "photo".into(),
        columns: 40,
        rows: 30,
        grey: vec![grey; 40 * 30],
        rect: cut_craft_core::geometry::Rect::new(Point::new(0.0, 0.0), Point::new(4.0, 3.0)),
        depth: vec![],
        rgb: vec![],
    };
    let (dark, light) = (picture(20), picture(230));
    let first = bitmap_texture(&ctx, EntityId(1), &dark);
    assert_eq!(bitmap_texture(&ctx, EntityId(1), &dark).id(), first.id());
    // Edit Picture keeps the id and the size: its new pixels are shown
    let edited = bitmap_texture(&ctx, EntityId(1), &light);
    assert_ne!(edited.id(), first.id());
    // and a picture back at its old pixels (Undo) is shown as they were
    assert_ne!(bitmap_texture(&ctx, EntityId(1), &dark).id(), edited.id());
}

/// A colour photo is shown in colour, not as its greys (Matthew, 2026-11-22: "it shows the
/// photo, but in black and white").
#[test]
fn a_colour_picture_is_drawn_in_colour() {
    let mut picture = cut_craft_core::bitmap::Bitmap {
        name: "photo".into(),
        columns: 2,
        rows: 1,
        grey: vec![76, 150],
        rect: cut_craft_core::geometry::Rect::new(Point::new(0.0, 0.0), Point::new(2.0, 1.0)),
        depth: vec![],
        rgb: vec![255, 0, 0, 0, 255, 0],
    };
    let image = bitmap_image(&picture);
    assert_eq!(
        image.pixels,
        [
            egui::Color32::from_rgb(255, 0, 0),
            egui::Color32::from_rgb(0, 255, 0)
        ]
    );
    picture.rgb.clear();
    assert_eq!(
        bitmap_image(&picture).pixels,
        [egui::Color32::from_gray(76), egui::Color32::from_gray(150)],
        "a grey picture stays grey"
    );
}

/// Points of `runs` all told.
fn points(runs: &[Vec<Pos2>]) -> usize {
    runs.iter().map(Vec::len).sum()
}

/// The distance from `p` to the nearest of `runs`' lines.
fn off(runs: &[Vec<Pos2>], p: Pos2) -> f32 {
    runs.iter()
        .flat_map(|run| run.windows(2))
        .map(|w| {
            let (a, b) = (w[0], w[1]);
            let along = (b - a).length_sq();
            let t = if along > 0.0 {
                ((p - a).dot(b - a) / along).clamp(0.0, 1.0)
            } else {
                0.0
            };
            p.distance(a + (b - a) * t)
        })
        .fold(f32::INFINITY, f32::min)
}

/// A 3D finishing raster of millions of tiny cuts (Matthew's: 6.8 million) drew a line
/// for each, and the 2 GB of lines crashed the graphics card's buffer: rows that are
/// straight on screen are drawn as a line each.
#[test]
fn a_dense_raster_is_drawn_with_a_line_a_row() {
    let view = Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 800.0));
    let mut runs = Runs::new(view, usize::MAX);
    let (rows, steps) = (200, 10_000);
    let mut at = pos2(10.0, 10.0);
    for row in 0..rows {
        let dir = if row % 2 == 0 { 1.0 } else { -1.0 };
        for _ in 0..steps {
            let to = at + vec2(dir * 0.09, 0.0);
            runs.line(at, to);
            at = to;
        }
        // the stepover, a cut too
        let to = at + vec2(0.0, 3.5);
        runs.line(at, to);
        at = to;
    }
    let drawn = runs.finish();
    assert_eq!(drawn.len(), 1, "one run: nothing broke it");
    assert!(
        points(&drawn) <= 2 * rows + 2,
        "{} points for {rows} rows",
        points(&drawn)
    );
}

#[test]
#[expect(clippy::cast_precision_loss, reason = "small counts")]
fn a_curve_drawn_thinned_stays_within_half_a_pixel() {
    let view = Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 800.0));
    let mut runs = Runs::new(view, usize::MAX);
    let centre = pos2(400.0, 400.0);
    let on = |i: usize| {
        let a = i as f32 * 0.0005;
        centre + vec2(a.cos(), a.sin()) * (300.0 + 40.0 * (7.0 * a).sin())
    };
    let count = 12_000;
    for i in 0..count {
        runs.line(on(i), on(i + 1));
    }
    let drawn = runs.finish();
    assert!(points(&drawn) < count / 4, "{} points", points(&drawn));
    for i in (0..=count).step_by(37) {
        let gap = off(&drawn, on(i));
        assert!(gap <= 0.5, "point {i} is {gap} px off");
    }
    let last = *drawn.last().unwrap().last().unwrap();
    assert!(
        last.distance(on(count)) < 1e-3,
        "it ends where the cuts end"
    );
}

#[test]
fn cuts_out_of_view_are_left_out_and_break_the_line() {
    let view = Rect::from_min_size(Pos2::ZERO, vec2(100.0, 100.0));
    let mut runs = Runs::new(view, usize::MAX);
    runs.line(pos2(10.0, 10.0), pos2(50.0, 10.0));
    runs.line(pos2(50.0, 10.0), pos2(50.0, 500.0));
    runs.line(pos2(50.0, 500.0), pos2(90.0, 500.0));
    runs.line(pos2(90.0, 500.0), pos2(90.0, 50.0));
    runs.line(pos2(90.0, 50.0), pos2(60.0, 50.0));
    let drawn = runs.finish();
    assert_eq!(
        drawn,
        vec![
            vec![pos2(10.0, 10.0), pos2(50.0, 10.0), pos2(50.0, 500.0)],
            vec![pos2(90.0, 500.0), pos2(90.0, 50.0), pos2(60.0, 50.0)],
        ]
    );
}

#[test]
fn a_gap_between_cuts_starts_a_new_line() {
    let view = Rect::from_min_size(Pos2::ZERO, vec2(100.0, 100.0));
    let mut runs = Runs::new(view, usize::MAX);
    runs.line(pos2(10.0, 10.0), pos2(20.0, 10.0));
    runs.gap();
    runs.line(pos2(20.0, 10.0), pos2(30.0, 10.0));
    // not starting where the last ended: a jump, not a line across
    runs.line(pos2(30.0, 40.0), pos2(40.0, 40.0));
    assert_eq!(
        runs.finish(),
        vec![
            vec![pos2(10.0, 10.0), pos2(20.0, 10.0)],
            vec![pos2(20.0, 10.0), pos2(30.0, 10.0)],
            vec![pos2(30.0, 40.0), pos2(40.0, 40.0)],
        ]
    );
}

/// However the cuts fall, a frame draws no more than its share of points.
#[test]
#[expect(clippy::cast_precision_loss, reason = "small counts")]
fn toolpaths_draw_no_more_than_the_points_they_are_given() {
    let view = Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 800.0));
    let mut runs = Runs::new(view, 1000);
    // a zigzag every point of which turns
    for i in 0..100_000 {
        let x = (i % 900) as f32 + 10.0;
        let y = if i % 2 == 0 { 10.0 } else { 700.0 };
        let (tx, ty) = ((i + 1) % 900, if i % 2 == 0 { 700.0 } else { 10.0 });
        runs.line(pos2(x, y), pos2(tx as f32 + 10.0, ty));
    }
    assert!(runs.used() <= 1000, "{}", runs.used());
    assert!(points(&runs.finish()) <= 1000);
}

/// A picture wider or taller than the graphics card takes is shrunk to fit, not
/// refused: a 16 604 pixel scan crashed the app (wgpu's limit was 8192).
#[test]
fn pictures_fit_the_graphics_card() {
    let wide = egui::ColorImage::new([16_604, 6], vec![egui::Color32::RED; 16_604 * 6]);
    let fitted = within(wide, 8192);
    assert!(
        fitted.size[0] <= 8192 && fitted.size[0] > 4000,
        "{:?}",
        fitted.size
    );
    assert_eq!(fitted.size[1], 2, "both sides shrink alike");
    assert_eq!(fitted.pixels.len(), fitted.size[0] * fitted.size[1]);
    assert_eq!(fitted.pixels[0], egui::Color32::RED);
    let small = egui::ColorImage::new([10, 10], vec![egui::Color32::BLUE; 100]);
    assert_eq!(
        within(small.clone(), 8192),
        small,
        "one that fits is left alone"
    );
}

#[test]
fn zooming_keeps_the_point_under_the_cursor_still() {
    let t = ViewTransform {
        origin: pos2(100.0, 400.0),
        scale: 2.0,
    };
    let cursor = pos2(250.0, 180.0);
    let zoomed = t.zoomed_about(cursor, 1.5);
    assert!((zoomed.scale - 3.0).abs() < 1e-6);
    assert!(zoomed.to_job(cursor).distance(t.to_job(cursor)) < 1e-4);
}

#[test]
fn panning_moves_everything_with_the_drag() {
    let t = ViewTransform {
        origin: pos2(100.0, 400.0),
        scale: 2.0,
    };
    let p = Point::new(10.0, 20.0);
    let moved = t.panned(vec2(30.0, -5.0));
    assert!((moved.to_screen(p) - t.to_screen(p) - vec2(30.0, -5.0)).length() < 1e-4);
}

#[test]
fn ruler_steps_go_one_two_five_and_never_crowd_their_labels() {
    let step = |pixels_per_unit| ruler_step(pixels_per_unit, 50.0);
    for (ppu, expected) in [
        (0.5, 100.0),
        (3.0, 20.0),
        (12.0, 5.0),
        (60.0, 1.0),
        (600.0, 0.1),
    ] {
        assert!(
            (step(ppu) - expected).abs() < 1e-9,
            "{ppu} px per unit: {}",
            step(ppu)
        );
    }
    assert_eq!(
        ruler_ticks(-35.0, 120.0, 20.0),
        [-20.0, 0.0, 20.0, 40.0, 60.0, 80.0, 100.0, 120.0]
    );
}

#[test]
fn a_job_centred_on_its_datum_is_still_fitted_in_the_middle() {
    let area = Rect::from_min_size(pos2(0.0, 0.0), vec2(500.0, 300.0));
    let job = Job {
        datum: cut_craft_core::document::Datum::Centre,
        ..Job::default()
    };
    let t = ViewTransform::fit(area, 10.0, job.area());
    let middle = t.to_screen(Point::ZERO);
    assert!(
        (middle.x - 250.0).abs() < 1e-3 && (middle.y - 150.0).abs() < 1e-3,
        "{middle:?}"
    );
}

#[test]
fn fit_keeps_the_aspect_ratio_and_centres_the_job() {
    let area = Rect::from_min_size(pos2(0.0, 0.0), vec2(500.0, 300.0));
    let t = ViewTransform::fit(area, 10.0, Job::default().area());
    assert!(
        (t.scale - 0.7).abs() < 1e-6,
        "limited by height: (300-20)/400"
    );
    let (bl, tr) = (
        t.to_screen(Point::new(0.0, 0.0)),
        t.to_screen(Point::new(600.0, 400.0)),
    );
    assert!(
        (bl.y - 290.0).abs() < 1e-3 && (tr.y - 10.0).abs() < 1e-3,
        "{bl:?} {tr:?}"
    );
    assert!(
        (f32::midpoint(bl.x, tr.x) - 250.0).abs() < 1e-3,
        "centred horizontally"
    );
}

#[test]
fn y_points_up_in_the_job_and_down_on_screen() {
    let t = ViewTransform {
        origin: pos2(100.0, 200.0),
        scale: 2.0,
    };
    assert_eq!(t.to_screen(Point::new(10.0, 5.0)), pos2(120.0, 190.0));
}

#[test]
fn screen_to_job_inverts_job_to_screen() {
    let t = ViewTransform {
        origin: pos2(37.0, 410.0),
        scale: 1.7,
    };
    let p = Point::new(123.4, -56.7);
    assert!(t.to_job(t.to_screen(p)).distance(p) < 1e-3);
}

#[test]
fn a_lasers_marks_at_the_surface_are_cuts_not_joins() {
    assert!(is_join("profile_toolpath", MoveKind::Cut, 0.0, 0.0));
    assert!(!is_join("profile_toolpath", MoveKind::Cut, 0.0, -1.0));
    assert!(!is_join("laser_cut_fill_toolpath", MoveKind::Cut, 0.0, 0.0));
    assert!(!is_join("laser_picture_toolpath", MoveKind::Cut, 0.0, 0.0));
    assert!(is_join("laser_cut_fill_toolpath", MoveKind::Cut, 2.0, 2.0));
    assert!(!is_join("profile_toolpath", MoveKind::Rapid, 5.0, 5.0));
}

#[test]
fn a_view_too_small_for_its_margin_still_fits_the_job_the_right_way_round() {
    for size in [vec2(400.0, 22.0), vec2(22.0, 22.0), vec2(0.0, 0.0)] {
        let area = Rect::from_min_size(pos2(0.0, 0.0), size);
        let t = ViewTransform::fit(area, 18.0, Job::default().area());
        assert!(t.scale > 0.0, "{size:?}: {}", t.scale);
    }
}
