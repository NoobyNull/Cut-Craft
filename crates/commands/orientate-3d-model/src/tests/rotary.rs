use super::*;
use cut_craft_core::document::Job;

/// Binary STL of `faces` in a temporary file of its own.
fn stl(name: &str, faces: &[[[f32; 3]; 3]]) -> String {
    let mut out = vec![0u8; 80];
    out.extend(u32::try_from(faces.len()).unwrap().to_le_bytes());
    for t in faces {
        out.extend([0u8; 12]);
        for x in t.iter().flatten() {
            out.extend(x.to_le_bytes());
        }
        out.extend([0u8; 2]);
    }
    let path = std::env::temp_dir().join(format!(
        "cut-craft-rotary-{}-{name}.stl",
        std::process::id()
    ));
    std::fs::write(&path, out).unwrap();
    path.display().to_string()
}

/// A closed cylinder of `radius` and `length` along X, from 48 flat sides.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "test geometry in f32"
)]
fn cylinder(radius: f64, length: f64) -> Vec<[[f32; 3]; 3]> {
    let sides = 48;
    let at = |i: usize, x: f64| {
        let angle = std::f64::consts::TAU * (i % sides) as f64 / sides as f64;
        [
            x as f32,
            (radius * angle.cos()) as f32,
            (radius * angle.sin()) as f32,
        ]
    };
    let mut faces = vec![];
    for i in 0..sides {
        let (near, next, next_far, far) =
            (at(i, 0.0), at(i + 1, 0.0), at(i + 1, length), at(i, length));
        faces.push([near, next, next_far]);
        faces.push([near, next_far, far]);
        faces.push([[0.0, 0.0, 0.0], next, near]);
        faces.push([[length as f32, 0.0, 0.0], far, next_far]);
    }
    faces
}

fn rotary_job() -> Document {
    Document {
        job: Job::rotary(100.0, 30.0, Along::X),
        ..Document::default()
    }
}

fn import(name: &str, faces: &[[[f32; 3]; 3]], p: &Params) -> (Document, Component) {
    let mut doc = rotary_job();
    let p = Params {
        file: stl(name, faces),
        ..p.clone()
    };
    let change = Orientate3dModel::run(&doc, &p).unwrap();
    doc.apply(&change).unwrap();
    let k = doc.components.values().next().unwrap().clone();
    (doc, k)
}

/// A box from `lo` to `hi`, as twelve triangles facing out.
fn cuboid(lo: [f32; 3], hi: [f32; 3]) -> Vec<[[f32; 3]; 3]> {
    let at = |i: usize| {
        [0, 1, 2].map(|axis| {
            if i >> axis & 1 == 1 {
                hi[axis]
            } else {
                lo[axis]
            }
        })
    };
    // each face's corners counter-clockwise seen from outside, by corner number (bit
    // 0 x, 1 y, 2 z)
    [
        [0, 2, 3, 1],
        [4, 5, 7, 6],
        [0, 1, 5, 4],
        [2, 6, 7, 3],
        [0, 4, 6, 2],
        [1, 3, 7, 5],
    ]
    .iter()
    .flat_map(|&[a, b, c, d]| [[at(a), at(b), at(c)], [at(a), at(c), at(d)]])
    .collect()
}

#[test]
fn a_full_model_previews_turned_round_the_axis_as_it_is_wrapped() {
    let p = Params {
        model_type: ModelType::Full,
        fit_material: true,
        ..Params::default()
    };
    let faces = cylinder(10.0, 50.0);
    let (doc, k) = import("turned", &faces, &p);
    let p = Params {
        file: stl("turned", &faces),
        ..p
    };
    let tris = preview(&rotary_job(), &p).unwrap().expect("shown turned");
    assert!(
        tris.iter().all(|(_, hidden)| !hidden),
        "cut all the way round"
    );
    let corners = || tris.iter().flat_map(|(t, _)| t.iter().copied());
    // fitted to the 30 mm stock: out 15 from the axis, as the wrapped model reaches
    let out = corners().map(|v| v.y.hypot(v.z)).fold(0.0, f64::max);
    assert!((out - 15.0).abs() < 0.1, "{out}");
    let (_, highest) = k.height_range().unwrap();
    assert!((highest - out).abs() < 0.5, "{highest} against {out}");
    // 75 long (1.5 times), centred along the 100 mm job
    let job = doc.job.area();
    let along = |f: fn(f64, f64) -> f64, from| corners().map(|v| v.x).fold(from, f);
    let (first, last) = (
        along(f64::min, f64::INFINITY),
        along(f64::max, f64::NEG_INFINITY),
    );
    // the file is in f32
    assert!((last - first - 75.0).abs() < 1e-4, "{first}..{last}");
    assert!(
        ((first - job.min.x) - (job.max.x - last)).abs() < 1e-6,
        "centred"
    );
}

#[test]
fn a_full_model_on_a_rotary_job_is_kept_as_it_really_is_round_the_axis() {
    let p = Params {
        model_type: ModelType::Full,
        fit_material: true,
        ..Params::default()
    };
    let faces = cylinder(10.0, 50.0);
    let (doc, k) = import("kept", &faces, &p);
    let solid = k.solid_triangles();
    assert_eq!(solid.len(), faces.len(), "every triangle kept");
    // what the 3D view shows turned round the axis, to the micrometre
    let shown = preview(
        &rotary_job(),
        &Params {
            file: stl("kept", &faces),
            ..p
        },
    )
    .unwrap()
    .unwrap();
    for (kept, (seen, _)) in solid.iter().zip(&shown) {
        for (a, b) in kept.iter().zip(seen) {
            assert!(a.distance(*b) < 1e-3, "{a} vs {b}");
        }
    }
    // and saved and opened again with the job
    let json = serde_json::to_string(&doc).unwrap();
    let opened: Document = serde_json::from_str(&json).unwrap();
    let again = opened.components.values().next().unwrap();
    assert_eq!(again.solid, k.solid);
    // a flat job's model keeps none
    let (_, flat) = {
        let mut doc = Document::default();
        let p = Params {
            file: stl("flat", &faces),
            ..Params::default()
        };
        doc.apply(&Orientate3dModel::run(&doc, &p).unwrap())
            .unwrap();
        let k = doc.components.values().next().unwrap().clone();
        (doc, k)
    };
    assert_eq!(flat.solid, Vec::<f32>::new());
}

#[test]
fn a_rotary_job_starts_at_the_file_s_size_with_its_size_locked() {
    // scaled to fit only when asked, so a turn or a small change never resizes it unseen
    let p = Orientate3dModel::initial_params(&rotary_job());
    assert!(!p.fit_material);
    assert!(
        !p.size.change_size,
        "the size is locked until Change size is ticked"
    );
    assert_eq!(p.size.scale, [1.0; 3]);
}

#[test]
fn scale_to_fit_is_worked_out_once_and_kept_while_the_model_turns() {
    // a 50 long, 20 across cylinder on a 100 long, 30 across job: as it lies, it fits at
    // 1.5 (its 10 mm reach to the 15 mm radius)
    let faces = cylinder(10.0, 50.0);
    let p = Params {
        file: stl("fit-once", &faces),
        model_type: ModelType::Full,
        ..Params::default()
    };
    let fits = fit_scale(&rotary_job(), &p).unwrap();
    assert!((fits - 1.5).abs() < 1e-3, "{fits}");
    let p = Params {
        size: ModelSize {
            scale: [fits; 3],
            ..p.size.clone()
        },
        ..p
    };
    let reach = |p: &Params| {
        let tris = preview(&rotary_job(), p).unwrap().unwrap();
        tris.iter()
            .flat_map(|(t, _)| t.iter().map(|v| v.y.hypot(v.z)))
            .fold(0.0, f64::max)
    };
    assert!((reach(&p) - 15.0).abs() < 0.1, "fitted: {}", reach(&p));
    // turned 30° about Y: the same size, not fitted again (its reach out grows past the
    // radius instead of the model shrinking every step of the turn)
    let turned = Params {
        turn: Turn {
            y: 30.0,
            ..Turn::default()
        },
        ..p.clone()
    };
    let tris = preview(&rotary_job(), &turned).unwrap().unwrap();
    let first = tris[0].0;
    let straight = preview(&rotary_job(), &p).unwrap().unwrap()[0].0;
    for (a, b) in [(0, 1), (1, 2), (0, 2)] {
        let (now, was) = (
            first[a].distance(first[b]),
            straight[a].distance(straight[b]),
        );
        assert!((now - was).abs() < 1e-6, "edge {a}-{b}: {now} vs {was}");
    }
    let out = reach(&turned);
    assert!(
        out > 15.5,
        "kept its size, so it reaches past the radius: {out}"
    );
}

#[test]
fn a_flat_job_s_scale_to_fit_is_worked_out_once_too() {
    // a 10 tall box 40 by 20 on the 600 by 400, 20 thick default job: 2 times fills
    // its thickness first
    let faces = cuboid([0.0, 0.0, 0.0], [40.0, 20.0, 10.0]);
    let p = Params {
        file: stl("fit-flat", &faces),
        ..Params::default()
    };
    let doc = Document::default();
    let fits = fit_scale(&doc, &p).unwrap();
    assert!((fits - 2.0).abs() < 1e-9, "{fits}");
    let p = Params {
        size: ModelSize {
            scale: [fits; 3],
            ..p.size.clone()
        },
        turn: Turn {
            z: 90.0,
            ..Turn::default()
        },
        ..p
    };
    let tris = preview(&doc, &p).unwrap().unwrap();
    let (lo, hi) = bounds(&tris.iter().map(|(t, _)| *t).collect::<Vec<_>>()).unwrap();
    let size = hi - lo;
    // turned flat on the spot, still 80 by 40 by 20: not fitted again
    assert!(
        (size.x - 40.0).abs() < 1e-4 && (size.y - 80.0).abs() < 1e-4,
        "{size}"
    );
    assert!((size.z - 20.0).abs() < 1e-4, "{size}");
}

#[test]
fn a_steep_slope_is_within_reach() {
    // a wedge rising at 70° across the job: every face is cut from above, so none is red
    // (the old rule marked a steep face by its own cell, which is higher than its middle)
    let (w, h) = (10.0_f32, 27.5_f32);
    let faces = vec![
        [[0.0, 0.0, 0.0], [w, 0.0, h], [w, 10.0, h]],
        [[0.0, 0.0, 0.0], [w, 10.0, h], [0.0, 10.0, 0.0]],
        [[0.0, 0.0, 0.0], [0.0, 10.0, 0.0], [w, 10.0, 0.0]],
        [[0.0, 0.0, 0.0], [w, 10.0, 0.0], [w, 0.0, 0.0]],
        [[w, 0.0, 0.0], [w, 10.0, 0.0], [w, 10.0, h]],
        [[w, 0.0, 0.0], [w, 10.0, h], [w, 0.0, h]],
        [[0.0, 0.0, 0.0], [w, 0.0, 0.0], [w, 0.0, h]],
        [[0.0, 10.0, 0.0], [w, 10.0, h], [w, 10.0, 0.0]],
    ];
    let p = Params {
        file: stl("wedge", &faces),
        ..Params::default()
    };
    let tris = preview(&Document::default(), &p).unwrap().unwrap();
    assert!(tris.iter().all(|(_, red)| !red), "nothing is out of reach");
    let p = Params {
        show_undercuts: false,
        ..p
    };
    let tris = preview(&Document::default(), &p).unwrap().unwrap();
    assert!(
        tris.iter().all(|(_, red)| !red),
        "nothing marked when not shown"
    );
}

/// Dates a file's last change `seconds` back, as one saved a while ago.
fn saved_ago(path: &str, seconds: u64) -> std::time::SystemTime {
    let when = std::time::SystemTime::now() - std::time::Duration::from_secs(seconds);
    std::fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(when)
        .unwrap();
    when
}

#[test]
fn a_file_is_read_once_until_it_changes() {
    let file = stl("read-once", &cuboid([0.0, 0.0, 0.0], [5.0, 5.0, 5.0]));
    saved_ago(&file, 60);
    let (first, md5) = read_file(&file).unwrap();
    let (again, _) = read_file(&file).unwrap();
    assert!(
        Arc::ptr_eq(&first, &again),
        "the same triangles, not read again"
    );
    // written again with another model: read again
    std::fs::write(
        &file,
        std::fs::read(stl("read-twice", &cuboid([0.0, 0.0, 0.0], [5.0, 5.0, 9.0]))).unwrap(),
    )
    .unwrap();
    let (changed, md5_changed) = read_file(&file).unwrap();
    assert!(!Arc::ptr_eq(&first, &changed));
    assert_ne!(md5, md5_changed);
    assert!(read_file("/nonexistent/model.stl").is_err());
}

/// Windows dates a change to the clock's tick (about 16 ms), so a model saved again at
/// once, the same size, can keep its date: a file changed that recently is read again.
#[test]
fn a_file_saved_again_within_the_clock_tick_is_read_again() {
    let file = stl("same-tick", &cuboid([0.0, 0.0, 0.0], [5.0, 5.0, 5.0]));
    let (first, md5) = read_file(&file).unwrap();
    let date = std::fs::metadata(&file).unwrap().modified().unwrap();
    let taller = stl(
        "same-tick-taller",
        &cuboid([0.0, 0.0, 0.0], [5.0, 5.0, 9.0]),
    );
    std::fs::copy(&taller, &file).unwrap();
    std::fs::File::options()
        .write(true)
        .open(&file)
        .unwrap()
        .set_modified(date)
        .unwrap();
    let (again, md5_again) = read_file(&file).unwrap();
    assert!(!Arc::ptr_eq(&first, &again), "read again");
    assert_ne!(md5, md5_again);
}

/// `cargo test -p cmd-orientate-3d-model -- --ignored --nocapture big_model`: how long
/// a two-million-face model takes to place, rasterise and judge.
#[test]
#[ignore = "timing, by hand"]
#[expect(clippy::cast_precision_loss, reason = "a test grid")]
fn big_model_timing() {
    let n = 1000;
    let mut faces = Vec::with_capacity(2 * n * n);
    let z = |x: usize, y: usize| ((x as f32 / 37.0).sin() + (y as f32 / 23.0).cos()) * 5.0 + 10.0;
    let at = |x: usize, y: usize| [x as f32 / 10.0, y as f32 / 10.0, z(x, y)];
    for x in 0..n {
        for y in 0..n {
            faces.push([at(x, y), at(x + 1, y), at(x + 1, y + 1)]);
            faces.push([at(x, y), at(x + 1, y + 1), at(x, y + 1)]);
        }
    }
    let p = Params {
        file: stl("big", &faces),
        ..Params::default()
    };
    let doc = Document::default();
    for pass in 0..3 {
        let started = Instant::now();
        let change = Orientate3dModel::run(&doc, &p).unwrap();
        let ran = started.elapsed();
        let mut made = doc.clone();
        made.apply(&change).unwrap();
        let top = made.components.values().next().unwrap();
        let started = Instant::now();
        let seen = preview_over(&doc, &p, Some(top)).unwrap().unwrap();
        println!(
            "pass {pass}: run {} ms, preview {} ms, {} faces",
            ran.as_millis(),
            started.elapsed().as_millis(),
            seen.len()
        );
    }
}

#[test]
fn the_preview_marks_an_overhang_and_what_is_under_it() {
    // a post 10 high with a shelf off its top, over a low step on the bed
    let post = cuboid([0.0, 0.0, 0.0], [10.0, 10.0, 10.0]);
    let shelf = cuboid([10.0, 0.0, 8.0], [30.0, 10.0, 10.0]);
    let step = cuboid([10.0, 0.0, 0.0], [30.0, 10.0, 2.0]);
    let faces: Vec<_> = [post, shelf, step].concat();
    let file = stl("shelf", &faces);
    let p = Params {
        file: file.clone(),
        ..Params::default()
    };
    let doc = Document::default();
    let tris = preview(&doc, &p).unwrap().unwrap();
    let red = |pick: &dyn Fn(&[DVec3; 3]) -> bool| {
        tris.iter().filter(|(t, _)| pick(t)).all(|(_, red)| *red)
    };
    let on = |t: &[DVec3; 3], z: f64| t.iter().all(|v| (v.z - z).abs() < 1e-9);
    assert!(red(&|t| on(t, 8.0)), "the shelf's underside");
    assert!(red(&|t| on(t, 2.0)), "the step's top, under the shelf");
    assert!(
        tris.iter()
            .filter(|(t, _)| on(t, 10.0))
            .all(|(_, red)| !red),
        "the tops are reached"
    );
    assert!(
        tris.iter().filter(|(t, _)| on(t, 0.0)).all(|(_, red)| !red),
        "the base rests on the bed"
    );
    let _ = std::fs::remove_file(file);
}

#[test]
fn every_job_starts_on_a_full_model_none_fitted_unasked() {
    let rotary = Orientate3dModel::initial_params(&rotary_job());
    let flat = Orientate3dModel::initial_params(&Document::default());
    assert_eq!(
        (rotary.model_type, flat.model_type),
        (ModelType::Full, ModelType::Full)
    );
    assert!(!rotary.fit_material && !flat.fit_material);
}

#[test]
#[expect(clippy::cast_precision_loss, reason = "a small grid")]
fn scale_to_fit_grows_every_axis_alike_until_one_meets_the_material() {
    // a 20 x 10 x 5 box in 100 x 50 x 12 mm: 5 times would fit the length and width,
    // but the thickness stops it at 2.4 times; nothing is stretched
    let p = Params {
        file: stl("fit", &cuboid([0.0, 0.0, 0.0], [20.0, 10.0, 5.0])),
        fit_material: true,
        ..Params::default()
    };
    let mut doc = Document::default();
    doc.job = cut_craft_core::document::Job {
        width: 100.0,
        height: 50.0,
        thickness: 12.0,
        ..doc.job
    };
    let change = Orientate3dModel::run(&doc, &p).unwrap();
    doc.apply(&change).unwrap();
    let k = doc.components.values().next().unwrap();
    let (_, top) = k.height_range().unwrap();
    assert!((top - 12.0).abs() < 0.05, "{top}");
    let size = Point::new(k.cell * k.columns as f64, k.cell * k.rows as f64);
    assert!(
        (size.x - 48.0).abs() < k.cell && (size.y - 24.0).abs() < k.cell,
        "{size}"
    );
    let _ = std::fs::remove_file(p.file);
}

#[test]
fn a_full_model_in_a_double_sided_job_puts_its_underside_on_the_other_side() {
    // a slab 3 above the bed on a post under its middle, in 10 mm material
    let slab = cuboid([0.0, 0.0, 3.0], [10.0, 10.0, 6.0]);
    let post = cuboid([4.0, 4.0, 0.0], [6.0, 6.0, 3.0]);
    let faces: Vec<_> = [slab, post].concat();
    let file = stl("two-sided", &faces);
    let mut doc = Document::default();
    doc.job = cut_craft_core::document::Job {
        width: 100.0,
        height: 50.0,
        thickness: 10.0,
        double_sided: Some(cut_craft_core::document::DoubleSided {
            flip: cut_craft_core::document::Flip::Horizontal,
            zero_same_side: false,
        }),
        ..doc.job
    };
    for (model_type, other_has_one) in [(ModelType::Full, true), (ModelType::Flat, false)] {
        let p = Params {
            file: file.clone(),
            model_type,
            placing: Placing {
                both_sides: true,
                ..Params::default().placing
            },
            ..Params::default()
        };
        let mut made = doc.clone();
        made.apply(&Orientate3dModel::run(&doc, &p).unwrap())
            .unwrap();
        let top = made.components.values().next().unwrap();
        assert!((top.height_at(Point::new(50.0, 25.0)).unwrap() - 6.0).abs() < 1e-6);
        assert_eq!(made.other_side.components.len(), usize::from(other_has_one));
        if other_has_one {
            let under = made.other_side.components.values().next().unwrap();
            // the post reaches the bed: 10 up from the other face; the slab's underside
            // 3 above the bed is 7 up from it, mirrored left to right
            assert!((under.height_at(Point::new(50.0, 25.0)).unwrap() - 10.0).abs() < 1e-6);
            assert!((under.height_at(Point::new(54.0, 21.0)).unwrap() - 7.0).abs() < 1e-6);
            assert_eq!(
                under.height_at(Point::new(44.0, 21.0)),
                None,
                "the post's left is the slab's right"
            );
        }
    }
    let _ = std::fs::remove_file(file);
}

#[test]
fn a_cylinder_wraps_to_its_radius_all_round_and_its_length_along() {
    let p = Params {
        model_type: ModelType::Full,
        fit_material: false,
        ..Params::default()
    };
    let (doc, k) = import("cylinder", &cylinder(10.0, 50.0), &p);
    let job = doc.job.area();
    for turn in [0.05, 0.3, 0.55, 0.8, 0.97] {
        let at = Point::new(50.0, job.min.y + job.size().y * turn);
        let z = k.height_at(at).unwrap();
        assert!((z - 10.0).abs() < 0.15, "{turn}: {z}");
    }
    assert_eq!(
        k.height_at(Point::new(20.0, 40.0)),
        None,
        "the cylinder spans x 25..75"
    );
    assert!(k.height_at(Point::new(27.0, 40.0)).is_some());
}

#[test]
fn fitting_the_material_scales_it_to_the_job_diameter() {
    let p = Params {
        model_type: ModelType::Full,
        fit_material: true,
        ..Params::default()
    };
    let (doc, k) = import("fit-cylinder", &cylinder(10.0, 50.0), &p);
    let (_, hi) = k.height_range().unwrap();
    assert!((hi - 15.0).abs() < 0.05, "{hi}");
    // 1.5 times as long: 75 of the 100
    assert!(k.height_at(Point::new(14.0, 40.0)).is_some());
    assert_eq!(k.height_at(Point::new(11.0, 40.0)), None);
    assert!(doc.job.rotary.is_some());
}

#[test]
fn a_flat_model_in_a_rotary_job_is_placed_like_clipart() {
    let p = Params {
        model_type: ModelType::Flat,
        ..Params::default()
    };
    let (doc, k) = import("flat", &cylinder(10.0, 50.0), &p);
    assert!(k.extent().size().y < doc.job.area().size().y / 2.0);
}

#[test]
#[expect(clippy::cast_precision_loss, reason = "a few vertices")]
fn a_fine_turn_about_x_stands_the_model_up_and_the_axis_offset_lifts_it_off_the_axis() {
    // a 20 x 10 x 5 box: turned 90 about X its 10 runs up
    let p = Params {
        file: stl("fine_turn", &cuboid([0.0, 0.0, 0.0], [20.0, 10.0, 5.0])),
        show_undercuts: false,
        turn: Turn {
            x: 90.0,
            ..Turn::default()
        },
        ..Params::default()
    };
    let seen = preview(&Document::default(), &p).unwrap().unwrap();
    let z = |pick: fn(f64, f64) -> f64| {
        seen.iter().flat_map(|(t, _)| t.iter().map(|v| v.z)).fold(
            if pick(0.0, 1.0) > 0.5 {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            },
            pick,
        )
    };
    assert!(
        (z(f64::max) - z(f64::min) - 10.0).abs() < 1e-6,
        "{}",
        z(f64::max) - z(f64::min)
    );
    // in a rotary job, Up 5 puts the model's middle 5 mm above the axis
    let doc = rotary_job();
    let centred = Params {
        fit_material: false,
        ..p.clone()
    };
    let lifted = Params {
        axis: AxisMove {
            drag: false,
            across: 0.0,
            up: 5.0,
        },
        ..centred.clone()
    };
    let middle_z = |p: &Params| {
        let seen = preview(&doc, p).unwrap().unwrap();
        let zs: Vec<f64> = seen
            .iter()
            .flat_map(|(t, _)| t.iter().map(|v| v.z))
            .collect();
        zs.iter().sum::<f64>() / zs.len() as f64
    };
    assert!(middle_z(&centred).abs() < 1e-6);
    assert!(
        (middle_z(&lifted) - 5.0).abs() < 1e-6,
        "{}",
        middle_z(&lifted)
    );
    let _ = std::fs::remove_file(p.file);
}
