use super::*;

/// A 20 × 10 × 5 box (binary STL, all six faces) in a temporary file of its own.
fn box_file(name: &str) -> std::path::PathBuf {
    let (w, d, h) = (20.0_f32, 10.0_f32, 5.0_f32);
    let v = |x: f32, y: f32, z: f32| [x, y, z];
    let quad = |a: [f32; 3], b: [f32; 3], c: [f32; 3], e: [f32; 3]| [[a, b, c], [a, c, e]];
    let faces: Vec<[[f32; 3]; 3]> = [
        quad(v(0.0, 0.0, h), v(w, 0.0, h), v(w, d, h), v(0.0, d, h)),
        quad(
            v(0.0, 0.0, 0.0),
            v(0.0, d, 0.0),
            v(w, d, 0.0),
            v(w, 0.0, 0.0),
        ),
        quad(
            v(0.0, 0.0, 0.0),
            v(w, 0.0, 0.0),
            v(w, 0.0, h),
            v(0.0, 0.0, h),
        ),
        quad(v(0.0, d, 0.0), v(0.0, d, h), v(w, d, h), v(w, d, 0.0)),
        quad(
            v(0.0, 0.0, 0.0),
            v(0.0, 0.0, h),
            v(0.0, d, h),
            v(0.0, d, 0.0),
        ),
        quad(v(w, 0.0, 0.0), v(w, d, 0.0), v(w, d, h), v(w, 0.0, h)),
    ]
    .concat();
    let mut out = vec![0u8; 80];
    out.extend(u32::try_from(faces.len()).unwrap().to_le_bytes());
    for t in faces {
        out.extend([0u8; 12]);
        for p in t {
            for x in p {
                out.extend(x.to_le_bytes());
            }
        }
        out.extend([0u8; 2]);
    }
    let path =
        std::env::temp_dir().join(format!("cut-craft-box-{}-{name}.stl", std::process::id()));
    std::fs::write(&path, out).unwrap();
    path
}

fn with_file(name: &str, p: &Params) -> Params {
    Params {
        file: box_file(name).display().to_string(),
        ..p.clone()
    }
}

#[test]
fn contract() {
    cut_craft_core::testing::id_is_a_slug_and_meta_is_labelled::<Orientate3dModel>();
    cut_craft_core::testing::schema_describes_an_object::<Orientate3dModel>();
    cut_craft_core::testing::default_params_round_trip_through_json::<Orientate3dModel>();
    let doc = Document::default();
    let change = Orientate3dModel::run(&doc, &with_file("contract", &Params::default())).unwrap();
    let mut done = doc.clone();
    done.apply(&change).unwrap();
    done.apply(&change.inverse()).unwrap();
    assert_eq!(done, doc);
    assert!(
        Orientate3dModel::run(&doc, &Params::default()).is_err(),
        "no file"
    );
}

fn imported(name: &str, p: &Params) -> (Document, Component) {
    let mut doc = Document::default();
    let change = Orientate3dModel::run(&doc, &with_file(name, p)).unwrap();
    doc.apply(&change).unwrap();
    let k = doc.components.values().next().unwrap().clone();
    (doc, k)
}

#[test]
fn a_generated_border_from_the_library_comes_in_at_its_recipes_size() {
    let dir = std::env::temp_dir().join(format!("cut-craft-border-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (wide, high) = (60, 20);
    // a ridge along the middle, 0 at the top and bottom edges
    let depth: Vec<f32> = (0..wide * high)
        .map(|at| {
            let y = f32::from(u16::try_from(at / wide).unwrap());
            1.0 - (y - 9.5).abs() / 9.5
        })
        .collect();
    cut_craft_border_tile::save(
        &dir,
        "Rope",
        &depth,
        wide,
        high,
        &one_pixel_bmp(),
        &cut_craft_border_tile::Recipe::new("rope", "3:1", 90.0, 30.0, 4.0),
    )
    .unwrap();
    let params = Params {
        file: dir.join("Rope.png").display().to_string(),
        ..Params::default()
    };
    let mut doc = Document::default();
    let change = Orientate3dModel::run(&doc, &params).unwrap();
    doc.apply(&change).unwrap();
    let model = doc.components.values().next().unwrap();
    let size = |n: usize| f64::from(u32::try_from(n).unwrap()) * model.cell;
    let (x, y) = (size(model.columns), size(model.rows));
    assert!(
        (x - 90.0).abs() < 2.0 * model.cell && (y - 30.0).abs() < 2.0 * model.cell,
        "{x} {y}"
    );
    let top = model
        .heights
        .iter()
        .copied()
        .filter(|v| v.is_finite())
        .fold(0.0, f64::max);
    assert!((top - 4.0).abs() < 0.3, "{top}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A picture one pixel big, as a BMP.
fn one_pixel_bmp() -> Vec<u8> {
    let mut b = vec![b'B', b'M'];
    b.extend(58u32.to_le_bytes());
    b.extend([0u8; 4]);
    b.extend(54u32.to_le_bytes());
    b.extend(40u32.to_le_bytes());
    b.extend(1i32.to_le_bytes());
    b.extend(1i32.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend(24u16.to_le_bytes());
    b.extend([0u8; 24]);
    b.extend([90, 150, 200, 0]);
    b
}

#[test]
fn undercuts_are_noted_and_a_plain_box_has_none() {
    let doc = Document::default();
    let plain = Orientate3dModel::run(&doc, &with_file("plain", &Params::default())).unwrap();
    assert_eq!(plain.notes, Vec::<String>::new());
    // the box on a stalk: its underside is 5 mm clear of the stalk's foot
    let (s, b) = (2.0_f32, 5.0_f32);
    let v = |x: f32, y: f32, z: f32| [x, y, z];
    let mut faces = vec![
        [v(0.0, 0.0, 0.0), v(0.0, s, 0.0), v(s, 0.0, 0.0)],
        [v(0.0, 0.0, b), v(0.0, 20.0, b), v(20.0, 0.0, b)],
        [
            v(0.0, 0.0, b + 3.0),
            v(20.0, 0.0, b + 3.0),
            v(0.0, 20.0, b + 3.0),
        ],
    ];
    faces.push([v(0.0, 0.0, 0.0), v(s, 0.0, 0.0), v(0.0, 0.0, b)]);
    let mut out = vec![0u8; 80];
    out.extend(u32::try_from(faces.len()).unwrap().to_le_bytes());
    for t in &faces {
        out.extend([0u8; 12]);
        out.extend(t.iter().flatten().flat_map(|x| x.to_le_bytes()));
        out.extend([0u8; 2]);
    }
    let path = std::env::temp_dir().join(format!("cut-craft-stalk-{}.stl", std::process::id()));
    std::fs::write(&path, out).unwrap();
    let p = Params {
        file: path.display().to_string(),
        ..Params::default()
    };
    let change = Orientate3dModel::run(&doc, &p).unwrap();
    assert_eq!(change.notes.len(), 1, "{:?}", change.notes);
    assert!(change.notes[0].contains("2.0 cm²"), "{:?}", change.notes);
}

#[test]
fn the_model_position_in_material_chosen_on_import_is_the_jobs() {
    use cut_craft_core::document::{GapFrom, ModelPosition};
    let at = |from, gap| Params {
        in_material: ModelPosition { from, gap },
        ..Params::default()
    };
    let (doc, _) = imported("bottom", &at(GapFrom::Below, 0.0));
    assert_eq!(
        doc.job.model_position,
        ModelPosition {
            from: GapFrom::Below,
            gap: 0.0
        },
        "the 5 high box sits on the material's bottom"
    );
    let (doc, _) = imported("under", &at(GapFrom::Above, 2.5));
    assert_eq!(doc.job.model_position.gap, 2.5);
    // a gap past the room left is kept as typed, never refused (the form says so in red)
    let (doc, _) = imported("past", &at(GapFrom::Below, 18.0));
    assert_eq!(doc.job.model_position.gap, 18.0);
    // the form opens on what the job has
    let mut job = Document::default();
    job.job.model_position = ModelPosition {
        from: GapFrom::Below,
        gap: 3.0,
    };
    assert_eq!(
        Orientate3dModel::initial_params(&job).in_material,
        job.job.model_position
    );
}

#[test]
fn scaled_to_fit_the_material_it_stays_where_the_alignment_put_it() {
    // the 20 by 10 by 5 box aligned at the file's size against the material's right,
    // top edge and top, then scaled to fit: still there, measured from the material's
    // corner, not grown out from the model's own corner
    let doc = Document::default();
    let (area, thickness) = (doc.job.area(), doc.job.thickness);
    let aligned = |align: Align, fit_material| Params {
        fit_material,
        placing: Placing {
            center: false,
            position: Xy::new(area.max.x - 20.0, area.max.y - 10.0),
            depth_below_top: thickness,
            align,
            ..Params::default().placing
        },
        ..Params::default()
    };
    let far = Align {
        across: Some(1.0),
        up: Some(1.0),
        height: Some(1.0),
    };
    let (_, k) = imported("aligned-fit", &aligned(far, true));
    let e = k.extent();
    assert!(e.size().x > 40.0, "scaled up: {e:?}");
    assert!(
        (e.max.x - area.max.x).abs() < 2.0 * k.cell,
        "{e:?} in {area:?}"
    );
    assert!(
        (e.max.y - area.max.y).abs() < 2.0 * k.cell,
        "{e:?} in {area:?}"
    );
    let (_, hi) = k.height_range().unwrap();
    assert!(
        (hi - thickness).abs() < 1e-6,
        "top at the material's top: {hi}"
    );
    // centred across, on the front edge, half way up
    let middle = Align {
        across: Some(0.5),
        up: Some(0.0),
        height: Some(0.5),
    };
    let (_, k) = imported("centred-fit", &aligned(middle, true));
    let e = k.extent();
    assert!(
        (e.center().x - area.center().x).abs() < 2.0 * k.cell,
        "{e:?}"
    );
    assert!((e.min.y - area.min.y).abs() < 2.0 * k.cell, "{e:?}");
    // a position typed, with no alignment kept, stays where it was typed
    let (_, k) = imported("typed-fit", &aligned(Align::default(), true));
    let e = k.extent();
    assert!(
        (e.min.x - (area.max.x - 20.0)).abs() < 2.0 * k.cell,
        "{e:?}"
    );
}

#[test]
fn centred_at_the_file_size_resting_on_the_bed() {
    let (doc, k) = imported("centred", &Params::default());
    let middle = doc.job.area().center();
    let top = k.height_at(middle).unwrap();
    assert!(
        (top - 5.0).abs() < 1e-6,
        "the 5 high box from the bed up: {top}"
    );
    let e = k.extent();
    assert!((e.size().x - 20.0).abs() < 2.0 * k.cell && (e.size().y - 10.0).abs() < 2.0 * k.cell);
    assert!((e.center() - middle).length() < 2.0 * k.cell);
}

#[test]
fn front_face_up_turns_depth_into_height_and_size_rescales_all_three() {
    let (_, k) = imported(
        "front",
        &Params {
            orientation: Orientation {
                face: Face::Front,
                rotation: 0.0,
            },
            size: ModelSize {
                x: 40.0,
                ..Params::default().size
            },
            placing: Placing {
                depth_below_top: 8.0,
                discard_below_zero: false,
                ..Params::default().placing
            },
            ..Params::default()
        },
    );
    let e = k.extent();
    // 20 wide becomes 40 (ratio locked: the 5 deep front face is 10 tall now)
    assert!((e.size().x - 40.0).abs() < 2.0 * k.cell, "{e:?}");
    assert!((e.size().y - 10.0).abs() < 2.0 * k.cell, "{e:?}");
    // a box seen from above is its flat top: 8 up, the plane 8 below the doubled face
    let (_, hi) = k.height_range().unwrap();
    assert!((hi - 8.0).abs() < 1e-6, "{hi}");
}

#[test]
fn z_sets_the_height_alone_or_with_the_ratio_locked_all_three() {
    let size = |lock_ratio| ModelSize {
        lock_ratio,
        z: 10.0,
        ..Params::default().size
    };
    let (_, k) = imported(
        "z_alone",
        &Params {
            size: size(false),
            ..Params::default()
        },
    );
    assert!((k.height_range().unwrap().1 - 10.0).abs() < 1e-6);
    assert!(
        (k.extent().size().x - 20.0).abs() < 2.0 * k.cell,
        "footprint kept"
    );
    let (_, k) = imported(
        "z_locked",
        &Params {
            size: size(true),
            ..Params::default()
        },
    );
    assert!((k.height_range().unwrap().1 - 10.0).abs() < 1e-6);
    assert!(
        (k.extent().size().x - 40.0).abs() < 2.0 * k.cell,
        "doubled with it"
    );
}

#[test]
fn a_plain_box_previews_whole_and_reachable() {
    // the box's bottom faces down but is its base, not an overhang
    let file = box_file("preview");
    let p = Params {
        file: file.display().to_string(),
        ..Params::default()
    };
    let doc = Document::default();
    let overhangs = |p: &Params| {
        preview(&doc, p)
            .unwrap()
            .unwrap()
            .iter()
            .filter(|(_, over)| *over)
            .count()
    };
    assert_eq!(overhangs(&p), 0, "a box's bottom rests on the bed");
    let tris = preview(&doc, &p).unwrap().unwrap();
    assert_eq!(tris.len(), 12);
    assert!(
        tris.iter()
            .flat_map(|(t, _)| t)
            .all(|v| v.z >= 0.0 && v.z <= 5.0)
    );
    let _ = std::fs::remove_file(file);
}

#[test]
fn inches_scale_and_discarding_below_zero() {
    let (_, k) = imported(
        "inches",
        &Params {
            size: ModelSize {
                units: FileUnits::Inches,
                ..Params::default().size
            },
            ..Params::default()
        },
    );
    assert!((k.extent().size().x - 20.0 * 25.4).abs() < 2.0 * k.cell);
    // the zero plane 2 below the top of the 5 high box: 2 of it is kept above the plane,
    // the rest discarded
    let (_, k) = imported(
        "deep",
        &Params {
            placing: Placing {
                depth_below_top: 2.0,
                ..Params::default().placing
            },
            ..Params::default()
        },
    );
    let (lo, hi) = k.height_range().unwrap();
    assert!((hi - 2.0).abs() < 1e-6 && lo >= 0.0, "{lo} to {hi}");
}

#[test]
fn a_depth_past_the_models_height_lifts_it_up_to_the_materials_top() {
    let lifted = |depth: f64| {
        let p = Params {
            placing: Placing {
                depth_below_top: depth,
                ..Params::default().placing
            },
            ..Params::default()
        };
        imported(&format!("lifted-{depth}"), &p)
            .1
            .height_range()
            .unwrap()
            .1
    };
    let thickness = Document::default().job.thickness;
    assert!(thickness > 8.0, "room above the 5 high box: {thickness}");
    assert!(
        (lifted(8.0) - 8.0).abs() < 1e-6,
        "3 off the plane, its top at 8"
    );
    assert!(
        (lifted(thickness + 100.0) - thickness).abs() < 1e-6,
        "no higher than the material's top"
    );
}
