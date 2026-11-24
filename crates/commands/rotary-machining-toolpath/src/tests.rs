use super::*;
use cut_craft_core::component::{Combine, Component};
use cut_craft_core::document::Job;
use cut_craft_core::toolpath::MoveKind;

/// A rotary job 40 mm across, 60 mm long along X, holding a part `radius(x)` from the
/// axis all the way round (its top at the material's surface).
fn turned(radius: impl Fn(f64) -> f64) -> Document {
    let job = Job::rotary(60.0, 40.0, Along::X);
    let mut doc = Document {
        job,
        ..Document::default()
    };
    let model = Component::from_fn("Turned", Combine::Add, doc.job.area(), 0.25, |p| {
        Some(radius(p.x))
    });
    let mut c = Change::builder(&doc);
    c.add_new_component(model);
    doc.apply(&c.finish()).unwrap();
    doc
}

/// A bead: 20 mm round at the ends, 15 mm in the middle.
fn beaded() -> Document {
    turned(|x| 20.0 - 5.0 * (std::f64::consts::PI * x / 60.0).sin())
}

/// A plain cylinder 12 mm from the axis, with a lip at the material's full 20 mm for
/// its first 2 mm (the model's top sits at the material's surface).
fn lipped() -> Document {
    turned(|x| if x < 2.0 { 20.0 } else { 12.0 })
}

cut_craft_core::contract_tests!(RotaryMachiningToolpath, beaded);

fn ball(diameter: f64) -> Tool {
    Tool {
        diameter,
        stepover: 0.25,
        finish_stepover: 0.25,
        ..Params::default().tool
    }
}

fn cuts_of(doc: &Document, p: &Params) -> Vec<DVec3> {
    machine(doc, p)
        .unwrap()
        .moves
        .iter()
        .filter(|m| m.kind == MoveKind::Cut)
        .map(|m| m.to)
        .collect()
}

#[test]
fn a_plain_cylinder_is_followed_at_its_own_radius_all_the_way_round() {
    let doc = lipped();
    let p = Params {
        tool: ball(6.0),
        ..Params::default()
    };
    // the part is 12 mm from the axis in a 20 mm one: 8 mm below the top all round
    for c in cuts_of(&doc, &p).into_iter().filter(|c| c.x > 6.0) {
        assert!((c.z + 8.0).abs() < 0.05, "{c:?}");
    }
}

#[test]
fn where_the_part_is_the_material_surface_the_bit_cuts_no_air() {
    // the lip is the material's own surface: nothing to cut there, so no ring goes round
    // it; the rest of the part is still followed all the way round
    let doc = lipped();
    let p = Params {
        tool: ball(6.0),
        ..Params::default()
    };
    let cuts = cuts_of(&doc, &p);
    let first = cuts.iter().map(|c| c.x).fold(f64::MAX, f64::min);
    let at: Vec<_> = cuts.iter().filter(|c| c.x < 1.0).take(5).collect();
    assert!(first > 1.0, "no ring on the lip: {first} {at:?}");
    assert!(cuts.iter().any(|c| c.x > 58.0));
    // held to one side, the same
    let off = Params {
        strategy: Strategy::OffCentre,
        ..p.clone()
    };
    assert!(cuts_of(&doc, &off).iter().all(|c| c.x > 1.0));
    // cut side by side, the bit only crosses the lip at the ends of its lines, from one
    // line over to the next: never further than two bit widths through the air
    let sides = Params {
        strategy: Strategy::Indexed,
        ..p
    };
    let toolpath = machine(&doc, &sides).unwrap();
    let (mut through, mut most) = (0.0, 0.0f64);
    for w in toolpath.moves.windows(2) {
        if w[1].kind == MoveKind::Cut && w[1].to.x < 1.0 {
            through += w[0].to.distance(w[1].to);
            most = most.max(through);
        } else {
            through = 0.0;
        }
    }
    assert!(most < 12.0, "{most}");
}

#[test]
fn each_ring_starts_at_a_zero_and_goes_all_the_way_round() {
    let doc = beaded();
    let p = Params {
        tool: ball(6.0),
        ..Params::default()
    };
    let toolpath = machine(&doc, &p).unwrap();
    let circumference = doc.job.height;
    let plunges: Vec<DVec3> = toolpath
        .moves
        .iter()
        .filter(|m| m.kind == MoveKind::Plunge)
        .map(|m| m.to)
        .collect();
    // 60 mm in 1.5 mm steps: 41 rings, each plunging at A=0, but for the two at the
    // ends, where the bead is the material's own surface and there is nothing to cut
    assert_eq!(plunges.len(), 39);
    assert!(plunges.iter().all(|p| p.y.abs() < 1e-9));
    let last_cut = |i: usize| {
        toolpath.moves[..i]
            .iter()
            .rev()
            .find(|m| m.kind == MoveKind::Cut)
            .map(|m| m.to.y)
    };
    let lifts: Vec<usize> = (1..toolpath.moves.len())
        .filter(|&i| toolpath.moves[i].kind == MoveKind::Rapid)
        .collect();
    for i in lifts.into_iter().skip(1) {
        assert!(
            (last_cut(i).unwrap() - circumference).abs() < 1e-9,
            "each ring ends back at A=0, a whole turn round"
        );
    }
}

#[test]
#[expect(clippy::cast_precision_loss, reason = "small counts")]
fn the_ball_rests_on_the_part_round_the_axis_and_never_cuts_into_it() {
    // a ridge 1 mm wide all along the part at A=0, the rest 10 mm down: deep in, the
    // ball reaches further round the part than it would on the unrolled sheet
    let mut doc = beaded();
    let circumference = doc.job.height;
    doc.components.clear();
    let model = Component::from_fn("Ridge", Combine::Add, doc.job.area(), 0.1, |p| {
        Some(if p.y.min(circumference - p.y) < 0.5 {
            20.0
        } else {
            10.0
        })
    });
    let mut c = Change::builder(&doc);
    c.add_new_component(model);
    doc.apply(&c.finish()).unwrap();
    let tool = ball(6.0);
    let surface = placed_model(&doc, &tool).unwrap();
    let round = Turned::of(
        Some(&surface),
        20.0,
        true,
        (60.0, circumference),
        0.1,
        Point::ZERO,
    );
    let rb = 3.0;
    for w in [
        0.0,
        1.0,
        2.0,
        3.0,
        4.0,
        6.0,
        10.0,
        30.0,
        circumference - 2.0,
    ] {
        let tip = round.drop(&tool, 30.0, w, 0.0, 0.0);
        let theta = w / 20.0;
        let centre = DVec3::new(30.0, (tip + rb) * theta.cos(), (tip + rb) * theta.sin());
        // the nearest the ball's centre comes to any cell of the part
        let mut nearest = f64::INFINITY;
        for j in 0..round.round {
            for i in 0..round.along {
                let r = round.radii[j * round.along + i];
                let phi = (j as f64 + 0.5) * round.round_cell / 20.0;
                let at = DVec3::new(
                    (i as f64 + 0.5) * round.along_cell,
                    r * phi.cos(),
                    r * phi.sin(),
                );
                nearest = nearest.min(centre.distance(at));
            }
        }
        assert!(nearest >= rb - 1e-6, "cuts in at {w}: {nearest}");
        assert!(
            nearest <= rb + 0.05,
            "hangs above the part at {w}: {nearest}"
        );
    }
}

#[test]
fn an_allowance_leaves_material_all_the_way_round() {
    let doc = lipped();
    let p = Params {
        tool: ball(6.0),
        allowance: 0.5,
        ..Params::default()
    };
    for c in cuts_of(&doc, &p).into_iter().filter(|c| c.x > 6.0) {
        assert!((c.z + 7.5).abs() < 0.05, "{c:?}");
    }
}

#[test]
fn rings_run_from_start_to_end_and_a_registration_mark_comes_after() {
    let doc = lipped();
    let p = Params {
        tool: ball(6.0),
        start: 10.0,
        end: 40.0,
        registration_mark: true,
        ..Params::default()
    };
    let toolpath = machine(&doc, &p).unwrap();
    let plunges: Vec<DVec3> = toolpath
        .moves
        .iter()
        .filter(|m| m.kind == MoveKind::Plunge)
        .map(|m| m.to)
        .collect();
    assert!((plunges[0].x - 10.0).abs() < 1e-9);
    let mark = plunges[plunges.len() - 1];
    assert!((plunges[plunges.len() - 2].x - 40.0).abs() < 1e-9);
    assert_eq!((mark.x, mark.y, mark.z), (46.0, 0.0, -MARK_DEPTH));
}

#[test]
fn a_job_zeroed_on_its_centre_cuts_the_same_part_where_the_material_is() {
    use cut_craft_core::document::Datum;
    // a part with a lip at its start and a flat on one side, on a job zeroed at `datum`
    let on = |datum| {
        let job = Job {
            datum,
            ..Job::rotary(60.0, 40.0, Along::X)
        };
        let area = job.area();
        let mut doc = Document {
            job,
            ..Document::default()
        };
        let model = Component::from_fn("Lip and flat", Combine::Add, area, 0.25, |p| {
            let (along, round) = (p.x - area.min.x, p.y - area.min.y);
            Some(if along < 2.0 {
                20.0
            } else if round < 30.0 {
                16.0
            } else {
                12.0
            })
        });
        let mut c = Change::builder(&doc);
        c.add_new_component(model);
        doc.apply(&c.finish()).unwrap();
        doc
    };
    let corner = on(Datum::BottomLeft);
    let centre = on(Datum::Centre);
    let shift = centre.job.area().min;
    assert!(shift.x < 0.0 && shift.y < 0.0, "zeroed on the middle");
    for strategy in [Strategy::Radial, Strategy::Indexed] {
        let p = Params {
            tool: ball(6.0),
            strategy,
            ..Params::default()
        };
        let at_corner = machine(&corner, &p).unwrap();
        let at_centre = machine(&centre, &p).unwrap();
        assert_eq!(at_corner.moves.len(), at_centre.moves.len(), "{strategy:?}");
        for (a, b) in at_corner.moves.iter().zip(&at_centre.moves) {
            assert_eq!(a.kind, b.kind);
            let moved = a.to + DVec3::new(shift.x, shift.y, 0.0);
            assert!(
                (moved - b.to).length() < 1e-6,
                "{strategy:?}: {a:?} vs {b:?}"
            );
        }
        assert_eq!(at_corner.across, at_centre.across);
    }
}

#[test]
fn the_registration_mark_never_cuts_into_the_part_and_says_why_it_is_left_out() {
    // the part fills the material right to its end, and End is the end
    let doc = turned(|_| 20.0);
    let p = Params {
        tool: ball(6.0),
        registration_mark: true,
        ..Params::default()
    };
    let (toolpath, notes) = made(&doc, &p).unwrap();
    assert!(
        toolpath.moves.iter().all(|m| m.to.z > -MARK_DEPTH + GRAZE),
        "nothing goes into the finished surface"
    );
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert!(notes[0].contains("registration mark"), "{notes:?}");
    // with waste past End there is room, and no note
    let (_, notes) = made(&lipped(), &Params { end: 40.0, ..p }).unwrap();
    assert!(notes.is_empty(), "{notes:?}");
}

/// Each cut of `toolpath` as the machine makes it: (along, A as mm round, the bit's
/// height from the top, how far across it is held).
fn held(toolpath: &Toolpath) -> Vec<(f64, f64, f64, f64)> {
    toolpath
        .moves
        .iter()
        .zip(&toolpath.across)
        .filter(|(m, _)| m.kind == MoveKind::Cut)
        .map(|(m, &d)| {
            let (w, z) = rotary_axis::held_at(20.0, m.to.y, m.to.z, d);
            (m.to.x, w, z, d)
        })
        .collect()
}

#[test]
fn held_off_centre_the_ball_still_rests_on_the_part_all_the_way_round() {
    let doc = lipped();
    let p = Params {
        tool: ball(6.0),
        strategy: Strategy::OffCentre,
        offset: 5.0,
        ..Params::default()
    };
    let toolpath = machine(&doc, &p).unwrap();
    assert_eq!(toolpath.across.len(), toolpath.moves.len());
    let cuts = held(&toolpath);
    assert!(cuts.iter().all(|c| c.3 == 5.0));
    // the part is 12 from the axis: the ball's centre 15 from it, wherever A is
    for (a, _, z, d) in cuts.into_iter().filter(|c| c.0 > 6.0) {
        let centre = d.hypot(20.0 + z + 3.0);
        assert!((centre - 15.0).abs() < 0.05, "{a}: {centre}");
    }
    // kept where the tip cuts: 5 across is a little round from A, and not as deep
    let tip = toolpath
        .moves
        .iter()
        .find(|m| m.kind == MoveKind::Cut)
        .unwrap();
    assert!(tip.to.y > 0.0 && tip.to.z > -8.0, "{tip:?}");
}

#[test]
#[expect(clippy::cast_precision_loss, reason = "small counts")]
fn held_off_centre_the_ball_never_cuts_into_a_narrow_ridge() {
    let mut doc = beaded();
    let circumference = doc.job.height;
    doc.components.clear();
    let model = Component::from_fn("Ridge", Combine::Add, doc.job.area(), 0.1, |p| {
        Some(if p.y.min(circumference - p.y) < 0.5 {
            20.0
        } else {
            10.0
        })
    });
    let mut c = Change::builder(&doc);
    c.add_new_component(model);
    doc.apply(&c.finish()).unwrap();
    let tool = ball(6.0);
    let surface = placed_model(&doc, &tool).unwrap();
    let turned = Turned::of(
        Some(&surface),
        20.0,
        true,
        (60.0, circumference),
        0.1,
        Point::ZERO,
    );
    let rb = 3.0;
    for across in [-6.0, -2.0, 4.0, 9.0] {
        for w in [0.0, 1.0, 3.0, 6.0, 10.0, circumference - 2.0] {
            let tip = turned.drop(&tool, 30.0, w, across, 0.0);
            let theta = w / 20.0;
            // the ball's centre, across and up, with A turned to theta
            let centre = DVec3::new(30.0, across, tip + rb);
            let mut nearest = f64::INFINITY;
            for j in 0..turned.round {
                for i in 0..turned.along {
                    let r = turned.radii[j * turned.along + i];
                    let phi = (j as f64 + 0.5) * turned.round_cell / 20.0 - theta;
                    let at = DVec3::new(
                        (i as f64 + 0.5) * turned.along_cell,
                        r * phi.sin(),
                        r * phi.cos(),
                    );
                    nearest = nearest.min(centre.distance(at));
                }
            }
            assert!(nearest >= rb - 1e-6, "cuts in at {across}, {w}: {nearest}");
            assert!(
                nearest <= rb + 0.05 || tip == 0.0,
                "hangs above the part at {across}, {w}: {nearest}"
            );
        }
    }
}

#[test]
fn indexed_cuts_each_side_with_a_held_still_and_the_bit_moving_across() {
    let doc = lipped();
    let p = Params {
        tool: ball(6.0),
        strategy: Strategy::Indexed,
        faces: 4,
        ..Params::default()
    };
    let toolpath = machine(&doc, &p).unwrap();
    let circumference = doc.job.height;
    // four sides, each plunged into once, at A = 0, 90, 180 and 270
    let mut sides: Vec<f64> = vec![];
    for (a, w, z, d) in held(&toolpath) {
        if !sides.iter().any(|s| (s - w).abs() < 1e-6) {
            sides.push(w);
        }
        // never into the part, 12 from the axis
        if a > 6.0 {
            let centre = d.hypot(20.0 + z + 3.0);
            assert!(centre >= 15.0 - 0.05, "{a}, {d}: {centre}");
        }
    }
    let quarter = circumference / 4.0;
    assert_eq!(sides.len(), 4, "{sides:?}");
    for (s, k) in sides.iter().zip([0.0, 1.0, 2.0, 3.0]) {
        assert!((s - quarter * k).abs() < 1e-6, "{sides:?}");
    }
    let plunges = toolpath
        .moves
        .iter()
        .filter(|m| m.kind == MoveKind::Plunge)
        .count();
    assert_eq!(plunges, 4);
    // the bit goes back and forth across each side, a little past its share
    let most = toolpath.across.iter().copied().fold(0.0, f64::max);
    assert!(most > 20.0 * std::f64::consts::FRAC_PI_4.sin(), "{most}");
}

#[test]
fn a_flat_job_is_asked_to_be_rotary() {
    let doc = cut_craft_core::testing::sample_document();
    let why = machine(&doc, &Params::default()).unwrap_err().to_string();
    assert!(why.contains("rotary"), "{why}");
}

/// A rotary job 40 mm across, 60 long along X, holding a box off to one side of the
/// axis as a full 3D model kept as it really is, heights and all.
fn boxed() -> Document {
    boxed_at(0.0)
}

/// [`boxed`] on a job whose X0 Y0 is `offset` in from the material's corner both ways, the
/// box kept where it is on the material.
fn boxed_at(offset: f64) -> Document {
    let mut doc = Document {
        job: Job {
            offset: cut_craft_core::geometry::Point::new(offset, offset),
            ..Job::rotary(60.0, 40.0, Along::X)
        },
        ..Document::default()
    };
    let along = doc.job.area().min.x;
    let (lo, hi) = (
        DVec3::new(along + 10.0, -6.0, -8.0),
        DVec3::new(along + 50.0, 12.0, 10.0),
    );
    let at = |i: usize| {
        DVec3::new(
            if i & 1 == 1 { hi.x } else { lo.x },
            if i & 2 == 2 { hi.y } else { lo.y },
            if i & 4 == 4 { hi.z } else { lo.z },
        )
    };
    let triangles: Vec<[DVec3; 3]> = [
        [0, 2, 3, 1],
        [4, 5, 7, 6],
        [0, 1, 5, 4],
        [2, 6, 7, 3],
        [0, 4, 6, 2],
        [1, 3, 7, 5],
    ]
    .iter()
    .flat_map(|&[a, b, c, d]| [[at(a), at(b), at(c)], [at(a), at(c), at(d)]])
    .collect();
    // its heights as an import wraps them: reaching the material's surface
    let mut model = Component::from_fn("Box", Combine::Add, doc.job.area(), 0.25, |_| Some(20.0));
    model.set_solid(&triangles);
    let mut c = Change::builder(&doc);
    c.add_new_component(model);
    doc.apply(&c.finish()).unwrap();
    doc
}

/// Each cut as the bit is held: along, A turned (radians), height above the axis, across.
fn poses(toolpath: &Toolpath) -> Vec<(f64, f64, f64, f64)> {
    toolpath
        .moves
        .iter()
        .enumerate()
        .filter(|(_, m)| m.kind == MoveKind::Cut)
        .map(|(i, m)| {
            let d = toolpath.across.get(i).copied().unwrap_or(0.0);
            let (w, z) = rotary_axis::held_at(20.0, m.to.y, m.to.z, d);
            (m.to.x, w / 20.0, 20.0 + z, d)
        })
        .collect()
}

#[test]
fn a_full_model_on_a_rotary_job_is_cut_round_as_it_really_is() {
    let doc = boxed();
    let mesh = Mesh::new(doc.components.values().next().unwrap().solid_triangles());
    for (strategy, offset) in [(Strategy::Radial, 0.0), (Strategy::OffCentre, 5.0)] {
        let p = Params {
            tool: ball(6.0),
            strategy,
            offset,
            ..Params::default()
        };
        let cutter = Cutter::new(&p.tool);
        let toolpath = machine(&doc, &p).unwrap();
        let mut on = 0;
        for (along, turn, height, across) in poses(&toolpath) {
            // exactly where the bit rests on the box, looked for on every triangle
            let rests = mesh
                .drop_every(&cutter, along, across, turn)
                .unwrap_or(0.0)
                .clamp(0.0, 20.0);
            assert!(
                (height - rests).abs() < 1e-6,
                "{strategy:?} at {along}, {turn}: {height} against {rests}"
            );
            on += usize::from(rests > 1.0);
        }
        assert!(on > 100, "{strategy:?}: the box is cut round ({on})");
    }
}

#[test]
fn a_full_model_is_cut_where_it_really_is_whatever_material_setup_says() {
    // the box's unrolled heights short of the surface, and Material Setup's flat-job
    // gap: neither moves the real model, which is cut exactly where it sits in space
    for (heights, gap) in [(12.0, 0.0), (20.0, 3.0), (12.0, 4.0)] {
        let mut doc = boxed();
        for k in doc.components.values_mut() {
            k.heights.fill(heights);
        }
        doc.job.model_position.gap = gap;
        let mesh = Mesh::new(doc.components.values().next().unwrap().solid_triangles());
        let p = Params {
            tool: ball(6.0),
            ..Params::default()
        };
        let cutter = Cutter::new(&p.tool);
        let toolpath = machine(&doc, &p).unwrap();
        let mut on = 0;
        for (along, turn, height, across) in poses(&toolpath) {
            let rests = mesh
                .drop_every(&cutter, along, across, turn)
                .unwrap_or(0.0)
                .clamp(0.0, 20.0);
            assert!(
                (height - rests).abs() < 1e-6,
                "heights {heights}, gap {gap}: at {along}, {turn}: {height} against {rests}"
            );
            on += usize::from(rests > 1.0);
        }
        assert!(on > 100, "the box is cut round ({on})");
    }
}

#[test]
fn an_allowance_leaves_at_least_that_much_on_a_full_model() {
    let doc = boxed();
    let mesh = Mesh::new(doc.components.values().next().unwrap().solid_triangles());
    let p = Params {
        tool: ball(6.0),
        allowance: 0.5,
        ..Params::default()
    };
    let cutter = Cutter::new(&p.tool);
    for (along, turn, height, across) in poses(&machine(&doc, &p).unwrap()) {
        if let Some(rests) = mesh.drop_every(&cutter, along, across, turn)
            && rests + 0.5 < 20.0
        {
            assert!(
                height >= rests + 0.5 - 1e-6,
                "{along}, {turn}: {height} vs {rests}"
            );
        }
    }
}

#[test]
fn typed_tilt_angles_are_read_in_degrees_and_anything_else_is_left_out_and_said() {
    assert_eq!(angles(""), (None, None));
    assert_eq!(angles("  "), (None, None));
    assert_eq!(
        angles("15, 165;45° -15"),
        (Some(vec![15.0, 165.0, 45.0, 345.0]), None)
    );
    // a word that is not an angle is left out, and said in red; never refused
    let (kept, why) = angles("15, up");
    assert_eq!(kept, Some(vec![15.0]));
    let why = why.expect("a note");
    assert!(why.contains("\"up\" is not an angle"), "{why}");
    let (kept, why) = angles("up down");
    assert_eq!(kept, None);
    assert!(why.expect("a note").contains("the angles are picked"));
}

/// The box cut bulk to fine detail with `tool`, the holder, chuck and tailstock as
/// `around` says: the toolpath and what it said in red.
fn detailed(tool: Tool, around: Surroundings, tilts: u32, typed: &str) -> (Toolpath, Vec<String>) {
    let doc = boxed();
    let p = Params {
        tool,
        strategy: Strategy::SidesThenDetails,
        faces: 2,
        tilts,
        tilt_angles: typed.into(),
        surroundings: Some(around),
        ..Params::default()
    };
    made(&doc, &p).unwrap()
}

/// The angles (degrees) A is at as the toolpath cuts.
fn cutting_angles(toolpath: &Toolpath) -> Vec<f64> {
    let mut seen: Vec<f64> = vec![];
    for (_, turn, _, _) in poses(toolpath) {
        let a = turn.to_degrees().rem_euclid(360.0);
        if !seen
            .iter()
            .any(|s| (s - a).abs() < 0.5 || (s - a).abs() > 359.5)
        {
            seen.push(a);
        }
    }
    seen.sort_by(f64::total_cmp);
    seen
}

#[test]
fn after_the_sides_a_locks_only_at_the_angles_given_or_none() {
    let long = || Tool {
        stickout: 40.0,
        flute_length: 25.0,
        ..ball(6.0)
    };
    let (none, _) = detailed(long(), Surroundings::default(), 0, "");
    assert_eq!(cutting_angles(&none), vec![0.0, 180.0]);
    let (given, _) = detailed(long(), Surroundings::default(), 6, "90");
    let seen = cutting_angles(&given);
    assert!(seen.contains(&90.0), "{seen:?}");
    assert!(
        seen.iter().all(|a| [0.0, 90.0, 180.0].contains(a)),
        "{seen:?}"
    );
}

#[test]
fn the_collet_nut_never_comes_down_onto_the_box_and_what_it_keeps_from_is_said() {
    // a bit sticking out only 8 mm under a 35 mm nut: beside the box's walls the nut sits
    // on the box's top before the bit gets down to the axis
    let stubby = Tool {
        stickout: 8.0,
        flute_length: 6.0,
        ..ball(6.0)
    };
    let around = Surroundings::default();
    let (toolpath, notes) = detailed(stubby, around, 0, "");
    let nut = around.holder.nut_diameter / 2.0;
    let mut checked = 0;
    for (along, turn, height, across) in poses(&toolpath) {
        // the box seen from above with A at 0 or 180: across and up as y and z are, or
        // turned over
        let flip = if turn.cos() > 0.999 {
            1.0
        } else if turn.cos() < -0.999 {
            -1.0
        } else {
            continue;
        };
        let (lo, hi) = if flip > 0.0 {
            (-6.0, 12.0)
        } else {
            (-12.0, 6.0)
        };
        let top = if flip > 0.0 { 10.0 } else { 8.0 };
        // the nut's disc over the box's top, seen from above
        let dx = (10.0 - along).max(along - 50.0).max(0.0);
        let dy = (lo - across).max(across - hi).max(0.0);
        if dx.hypot(dy) < nut {
            assert!(
                height + 8.0 >= top - 0.01,
                "the nut is down on the box at {along}, {across} (A {turn}): tip {height}"
            );
            checked += 1;
        }
    }
    assert!(checked > 100, "{checked}");
    assert!(
        notes
            .iter()
            .any(|n| n.contains("collet nut (35 mm across)")),
        "{notes:?}"
    );
}

#[test]
fn nothing_is_cut_where_the_chuck_holds_the_material_and_a_model_too_near_it_is_said() {
    let long = Tool {
        stickout: 40.0,
        flute_length: 25.0,
        ..ball(6.0)
    };
    let around = Surroundings {
        chuck: 15.0,
        ..Surroundings::default()
    };
    let (toolpath, notes) = detailed(long, around, 0, "");
    for (along, _, _, _) in poses(&toolpath) {
        assert!(along - 3.0 >= 15.0 - 1e-6, "cut in the chuck at {along}");
    }
    assert!(
        notes
            .iter()
            .any(|n| n.contains("the chuck holds the first 15.0 mm")),
        "{notes:?}"
    );
    // well clear of it: nothing said about it
    let (_, notes) = detailed(
        Tool {
            stickout: 40.0,
            flute_length: 25.0,
            ..ball(6.0)
        },
        Surroundings::default(),
        0,
        "",
    );
    assert!(!notes.iter().any(|n| n.contains("chuck")), "{notes:?}");
}

#[test]
fn nothing_is_cut_where_the_tailstock_holds_the_material_however_far_out() {
    let long = Tool {
        stickout: 40.0,
        flute_length: 25.0,
        ..ball(6.0)
    };
    let around = Surroundings {
        tailstock: 8.0,
        ..Surroundings::default()
    };
    // the job is 60 long: the tailstock holds from 52 on, all the way out to the swing
    let (toolpath, _) = detailed(long, around, 2, "");
    let mut cut = 0;
    for (along, _, _, _) in poses(&toolpath) {
        assert!(
            along + 3.0 <= 52.0 + 1e-6,
            "cut in the tailstock at {along}"
        );
        cut += 1;
    }
    assert!(cut > 0);
    // the registration mark would go at the job's end, in the tailstock: left out, and said
    let p = Params {
        tool: Tool {
            stickout: 40.0,
            flute_length: 25.0,
            ..ball(6.0)
        },
        strategy: Strategy::SidesThenDetails,
        faces: 2,
        tilts: 0,
        registration_mark: true,
        surroundings: Some(around),
        ..Params::default()
    };
    let (toolpath, notes) = made(&boxed(), &p).unwrap();
    for (along, _, _, _) in poses(&toolpath) {
        assert!(
            along + 3.0 <= 52.0 + 1e-6,
            "cut in the tailstock at {along}"
        );
    }
    assert!(
        notes
            .iter()
            .any(|n| n.contains("No room for the registration mark")),
        "{notes:?}"
    );
}

#[test]
fn how_near_the_tailstock_the_model_ends_is_said_the_same_wherever_x0_y0_is() {
    let p = Params {
        tool: Tool {
            stickout: 40.0,
            flute_length: 25.0,
            ..ball(6.0)
        },
        strategy: Strategy::SidesThenDetails,
        faces: 2,
        tilts: 0,
        surroundings: Some(Surroundings {
            tailstock: 15.0,
            ..Surroundings::default()
        }),
        ..Params::default()
    };
    for offset in [0.0, 30.0, -12.5] {
        let (_, notes) = made(&boxed_at(offset), &p).unwrap();
        assert!(
            notes
                .iter()
                .any(|n| n.contains("The model ends 10.0 mm from the job's end")),
            "X0 Y0 {offset} in: {notes:?}"
        );
    }
}

#[test]
fn what_is_left_rough_is_kept_with_the_toolpath_and_a_guessed_stick_out_is_said() {
    // a bit whose stick-out the Bit Library doesn't know: 25 mm is taken, too short to
    // get the nut down beside the box
    let unknown = Tool {
        stickout: 0.0,
        flute_length: 0.0,
        ..ball(6.0)
    };
    let around = Surroundings {
        tailstock: 15.0,
        ..Surroundings::default()
    };
    let (toolpath, notes) = detailed(unknown, around, 0, "");
    let kept = toolpath.made_notes();
    assert!(kept.iter().any(|n| n.contains("left rough")), "{kept:?}");
    assert!(kept.iter().any(|n| n.contains("tailstock")), "{kept:?}");
    assert!(
        kept.iter().all(|n| notes.contains(n)),
        "said in the form too"
    );
    assert!(
        kept.iter().any(|n| n.contains(
            "The Bit Library doesn't say how far this bit sticks out of the collet, so 25 mm \
             was taken"
        )),
        "{kept:?}"
    );
    // a stick-out given is not called a guess
    let (toolpath, _) = detailed(
        Tool {
            stickout: 8.0,
            flute_length: 6.0,
            ..ball(6.0)
        },
        Surroundings::default(),
        0,
        "",
    );
    let kept = toolpath.made_notes();
    assert!(kept.iter().any(|n| n.contains("left rough")), "{kept:?}");
    assert!(!kept.iter().any(|n| n.contains("was taken")), "{kept:?}");
}

/// [`boxed`] held as `support` says.
fn supported(joined: cut_craft_core::document::Joined, beam: f64) -> Document {
    let mut doc = boxed();
    doc.job.rotary = doc.job.rotary.map(|r| cut_craft_core::document::Rotary {
        support: cut_craft_core::document::Support { joined, beam },
        ..r
    });
    doc
}

/// How far the ball's centre is from the turning axis at each cut, and where along.
fn centres(toolpath: &Toolpath, ball: f64) -> Vec<(f64, f64)> {
    poses(toolpath)
        .into_iter()
        .map(|(along, _, height, across)| (along, across.hypot(height + ball / 2.0)))
        .collect()
}

#[test]
fn centre_stock_is_left_standing_along_the_axis_through_the_model_and_past_it() {
    use cut_craft_core::document::Joined;
    let long = || Tool {
        stickout: 40.0,
        flute_length: 25.0,
        ..ball(6.0)
    };
    for strategy in [
        Strategy::Radial,
        Strategy::Indexed,
        Strategy::SidesThenDetails,
    ] {
        let p = Params {
            tool: long(),
            strategy,
            faces: 2,
            tilts: 2,
            ..Params::default()
        };
        // a beam 10 across: the ball's centre never comes within its radius and the ball's
        let beamed = machine(&supported(Joined::Neither, 10.0), &p).unwrap();
        let near = centres(&beamed, 6.0);
        assert!(
            near.iter().all(|&(_, d)| d >= 5.0 + 3.0 - 1e-3),
            "{strategy:?}: into the beam at {:?}",
            near.iter().find(|&&(_, d)| d < 8.0 - 1e-3)
        );
        // the beam is cut round, past the box's ends too
        assert!(
            near.iter().any(|&(a, d)| a < 7.0 && d < 8.0 + 0.5),
            "{strategy:?}: the beam followed past the model"
        );
        // without it, or with Cut Away the Support ticked, the bit goes nearer the axis
        let plain = machine(&supported(Joined::Neither, 0.0), &p).unwrap();
        let cut_away = machine(
            &supported(Joined::Neither, 10.0),
            &Params {
                cut_support: true,
                ..p.clone()
            },
        )
        .unwrap();
        for open in [plain, cut_away] {
            assert!(
                centres(&open, 6.0).iter().any(|&(_, d)| d < 8.0 - 1.0),
                "{strategy:?}: down past where the beam would be"
            );
        }
    }
}

#[test]
fn past_the_model_at_an_end_it_is_kept_joined_at_the_material_is_left_whole() {
    use cut_craft_core::document::Joined;
    let p = Params {
        tool: Tool {
            stickout: 40.0,
            flute_length: 25.0,
            ..ball(6.0)
        },
        strategy: Strategy::SidesThenDetails,
        faces: 2,
        tilts: 0,
        ..Params::default()
    };
    // the box runs from 10 to 50 along: the ball's side reaches 3 past its centre
    let cut_at = |joined| -> Vec<f64> {
        let toolpath = machine(&supported(joined, 0.0), &p).unwrap();
        poses(&toolpath).into_iter().map(|(a, ..)| a).collect()
    };
    let chuck = cut_at(Joined::Chuck);
    assert!(
        chuck.iter().all(|&a| a + 3.0 >= 10.0 - 1e-6),
        "into the chuck end"
    );
    assert!(
        chuck.iter().any(|&a| a - 3.0 > 50.0),
        "the tailstock end is cut away"
    );
    let both = cut_at(Joined::Both);
    assert!(
        both.iter()
            .all(|&a| a + 3.0 >= 10.0 - 1e-6 && a - 3.0 <= 50.0 + 1e-6),
        "into an end"
    );
    let neither = cut_at(Joined::Neither);
    assert!(
        neither.iter().any(|&a| a + 3.0 < 10.0),
        "the chuck end is cut away"
    );
    // joined at the chuck, the model needs no room beside it
    let near_chuck = Params {
        surroundings: Some(Surroundings {
            chuck: 2.0,
            ..Surroundings::default()
        }),
        ..p.clone()
    };
    for joined in [Joined::Neither, Joined::Chuck] {
        let (_, notes) = made(&supported(joined, 0.0), &near_chuck).unwrap();
        assert_eq!(
            notes.iter().any(|n| n.contains("chuck holds")),
            joined == Joined::Neither,
            "{joined:?}: {notes:?}"
        );
    }
}
