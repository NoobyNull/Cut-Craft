use super::*;
use cut_craft_core::component::{Combine, Component};
use cut_craft_core::document::{Along, Job};
use cut_craft_core::glam::DVec3;
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

/// The model's radius along a bead: 20 mm round at the ends, 12 mm in the middle.
fn bead(x: f64) -> f64 {
    20.0 - 8.0 * (std::f64::consts::PI * x / 60.0).sin()
}

fn beaded() -> Document {
    turned(bead)
}

cut_craft_core::contract_tests!(RotaryFinishingToolpath, beaded);

fn ball(diameter: f64) -> Tool {
    Tool {
        diameter,
        stepover: 0.4,
        finish_stepover: 0.25,
        ..Params::default().tool
    }
}

fn finish(doc: &Document, p: &Params) -> (Toolpath, Vec<String>) {
    made(doc, p).unwrap()
}

fn cuts(t: &Toolpath) -> Vec<DVec3> {
    t.moves
        .iter()
        .filter(|m| m.kind == MoveKind::Cut)
        .map(|m| m.to)
        .collect()
}

fn coarse() -> Params {
    Params {
        tool: ball(6.0),
        ..Params::default()
    }
}

#[test]
fn the_bit_follows_the_model_without_cutting_into_it() {
    let doc = beaded();
    for way in [Way::RoundThePart, Way::AlongThePart] {
        let p = Params { way, ..coarse() };
        let (toolpath, notes) = finish(&doc, &p);
        let cuts = cuts(&toolpath);
        assert!(!cuts.is_empty(), "{way:?} {notes:?}");
        for c in &cuts {
            // the tip of a ball is never below the model under it, as near as the cells see
            let model = bead(c.x.clamp(0.0, 60.0)) - 20.0;
            assert!(c.z >= model - 0.25, "{way:?}: {c:?} under {model}");
        }
        // and it gets right down onto it in the middle
        let deepest = cuts.iter().map(|c| c.z).fold(f64::INFINITY, f64::min);
        assert!(deepest < -7.5, "{way:?} {deepest}");
    }
}

#[test]
fn round_the_part_is_the_usual_and_turns_a_with_the_bit_still_along() {
    assert_eq!(Params::default().way, Way::RoundThePart);
    let (toolpath, _) = finish(&beaded(), &coarse());
    let mut round = 0;
    for w in toolpath.moves.windows(2) {
        if w[1].kind == MoveKind::Cut && w[0].kind == MoveKind::Cut {
            let d = w[1].to - w[0].to;
            if d.y.abs() > 1e-6 && d.x.abs() < 1e-6 {
                round += 1;
            }
        }
    }
    assert!(round > 100, "{round}");
}

#[test]
fn along_the_part_holds_a_still_while_the_bit_runs_along() {
    let p = Params {
        way: Way::AlongThePart,
        cut_direction: CutDirection::Climb,
        ..coarse()
    };
    let (toolpath, _) = finish(&beaded(), &p);
    let mut along = 0;
    for w in toolpath.moves.windows(2) {
        if w[1].kind == MoveKind::Cut && w[0].kind == MoveKind::Cut {
            let d = w[1].to - w[0].to;
            if d.x.abs() > 1e-6 {
                assert!(d.y.abs() < 1e-6, "{w:?}");
                // every pass the same way: along X that climbs
                assert!(d.x > 0.0, "{w:?}");
                along += 1;
            }
        }
    }
    assert!(along > 20, "{along}");
}

#[test]
fn off_centre_holds_the_bit_to_one_side_or_each_in_turn() {
    let doc = beaded();
    let one = Params {
        held: Held::OffCentre,
        offset: 3.0,
        ..coarse()
    };
    let (toolpath, _) = finish(&doc, &one);
    assert_ne!(cuts(&toolpath).len(), 0);
    let sides = |t: &Toolpath| {
        let mut s: Vec<bool> = t
            .across
            .iter()
            .filter(|a| a.abs() > 1e-6)
            .map(|&a| a > 0.0)
            .collect();
        s.dedup();
        s
    };
    assert_eq!(sides(&toolpath), vec![true]);
    let both = Params {
        sides: Sides::BothSides,
        ..one
    };
    let (toolpath, _) = finish(&doc, &both);
    assert_eq!(
        sides(&toolpath),
        vec![true, false],
        "one side, then the other"
    );
}

#[test]
fn it_cuts_only_what_the_toolpaths_above_it_leave() {
    let mut doc = beaded();
    let first = RotaryFinishingToolpath::run(&doc, &coarse()).unwrap();
    doc.apply(&first).unwrap();
    // the same again: the first took it all down to the model
    let (again, notes) = finish(&doc, &coarse());
    assert_eq!(cuts(&again).len(), 0);
    assert!(notes.iter().any(|n| n == NOTHING_LEFT), "{notes:?}");
    // Settings' advanced rotary mode: the whole blank, whatever is above it
    let whole = Params {
        whole_blank: true,
        ..coarse()
    };
    assert_ne!(cuts(&finish(&doc, &whole).0).len(), 0);
}

#[test]
fn the_registration_mark_goes_past_the_end_at_a_zero() {
    let p = Params {
        end: 40.0,
        registration_mark: true,
        ..coarse()
    };
    let (toolpath, notes) = finish(&beaded(), &p);
    let plunges: Vec<DVec3> = toolpath
        .moves
        .iter()
        .filter(|m| m.kind == MoveKind::Plunge)
        .map(|m| m.to)
        .collect();
    let mark = *plunges.last().unwrap();
    assert_eq!(
        (mark.x, mark.y, mark.z),
        (46.0, 0.0, -cut_craft_rotary_carving::MARK_DEPTH),
        "{notes:?}"
    );
}

#[test]
fn a_flat_job_is_refused() {
    assert!(made(&Document::default(), &Params::default()).is_err());
}

#[test]
fn changing_the_model_makes_it_out_of_date() {
    let (toolpath, _) = finish(&beaded(), &coarse());
    assert!(toolpath.reads_model(), "it cuts the model");
}

#[test]
fn after_a_roughing_it_still_takes_the_allowance_left_down_to_the_model() {
    let mut doc = beaded();
    let roughing = cmd_rotary_roughing_toolpath::Params {
        tool: Tool {
            shape: ToolShape::EndMill,
            pass_depth: 3.0,
            ..ball(6.0)
        },
        ..cmd_rotary_roughing_toolpath::Params::default()
    };
    let roughed =
        cmd_rotary_roughing_toolpath::RotaryRoughingToolpath::run(&doc, &roughing).unwrap();
    doc.apply(&roughed).unwrap();
    let (after, notes) = finish(&doc, &coarse());
    let after = cuts(&after);
    assert!(!after.is_empty(), "{notes:?}");
    // still right down onto the model in the middle
    let deepest = after.iter().map(|c| c.z).fold(f64::INFINITY, f64::min);
    assert!(deepest < -7.5, "{deepest}");
}

/// `doc` with a closed rectangle drawn on the job from `x0` to `x1` along and `y0` to `y1`
/// round, selected.
fn with_selected_box(mut doc: Document, (x0, x1): (f64, f64), (y0, y1): (f64, f64)) -> Document {
    let mut c = Change::builder(&doc);
    let at = cut_craft_core::geometry::Point::new;
    let id = c.add(cut_craft_core::geometry::Path::polyline(
        &[at(x0, y0), at(x1, y0), at(x1, y1), at(x0, y1)],
        true,
    ));
    c.select([id]);
    doc.apply(&c.finish()).unwrap();
    doc
}

#[test]
fn a_machining_limit_of_selected_vectors_keeps_the_finish_inside_them_wrapped_round() {
    let doc = with_selected_box(beaded(), (20.0, 40.0), (10.0, 50.0));
    let p = Params {
        machining_boundary: cut_craft_core::component::MachiningBoundary::SelectedVectors,
        ..coarse()
    };
    let (toolpath, notes) = finish(&doc, &p);
    let cuts = cuts(&toolpath);
    assert_ne!(cuts.len(), 0, "{notes:?}");
    for c in &cuts {
        assert!((20.0 - 1e-6..=40.0 + 1e-6).contains(&c.x), "{c:?}");
        assert!((10.0 - 1e-6..=50.0 + 1e-6).contains(&c.y), "{c:?}");
    }
    // kept with the toolpath, so recalculating with nothing selected cuts the same
    assert_eq!(toolpath.vectors.len(), 1);
    // and the whole material is the usual
    assert_eq!(
        Params::default().machining_boundary,
        cut_craft_core::component::MachiningBoundary::Material
    );
}

#[test]
fn a_machining_limit_with_nothing_selected_says_what_to_do() {
    let p = Params {
        machining_boundary: cut_craft_core::component::MachiningBoundary::SelectedVectors,
        ..coarse()
    };
    // never refused: an empty toolpath, and in red what to select
    let change = RotaryFinishingToolpath::run(&beaded(), &p).unwrap();
    let mut doc = beaded();
    doc.apply(&change).unwrap();
    let toolpath = doc.toolpaths.values().next().unwrap();
    assert_eq!(toolpath.moves, vec![]);
    assert!(
        change.notes.iter().any(|n| n.contains("closed vectors")),
        "{:?}",
        change.notes
    );
}

/// How far a square bar's flats are from the axis: its corners just touch the material's
/// surface.
const FLAT: f64 = 20.0 * std::f64::consts::FRAC_1_SQRT_2;

/// A rotary job 40 mm across holding a square bar, its faces square to A=0, 90°, 180° and
/// 270°.
fn squared() -> Document {
    let job = Job::rotary(60.0, 40.0, Along::X);
    let mut doc = Document {
        job,
        ..Document::default()
    };
    let model = Component::from_fn("Square", Combine::Add, doc.job.area(), 0.25, |p| {
        let angle = p.y / 20.0;
        Some((FLAT / angle.cos().abs().max(angle.sin().abs())).min(20.0))
    });
    let mut c = Change::builder(&doc);
    c.add_new_component(model);
    doc.apply(&c.finish()).unwrap();
    doc
}

/// Where A is (degrees, 0 to 360) at each cut of `t`.
fn turns(t: &Toolpath) -> Vec<f64> {
    let across = if t.across.is_empty() {
        vec![0.0; t.moves.len()]
    } else {
        t.across.clone()
    };
    t.moves
        .iter()
        .zip(&across)
        .filter(|(m, _)| m.kind == MoveKind::Cut)
        .map(|(m, &across)| {
            let (round, _) = cut_craft_core::rotary_axis::held_at(20.0, m.to.y, m.to.z, across);
            (round / 20.0).to_degrees().rem_euclid(360.0)
        })
        .collect()
}

/// How many of `turns` are within 5° of a square bar's faces facing straight up.
fn facing_up(turns: &[f64]) -> f64 {
    #[expect(clippy::cast_precision_loss, reason = "a share of the cuts")]
    let share = turns
        .iter()
        .filter(|a| {
            let off = a.rem_euclid(90.0);
            off.min(90.0 - off) < 5.0
        })
        .count() as f64
        / turns.len().max(1) as f64;
    share
}

#[test]
fn following_the_surface_cuts_flat_faces_with_a_still_and_never_into_the_model() {
    let doc = squared();
    let over = finish(&doc, &coarse()).0;
    let p = Params {
        held: Held::FollowsTheSurface,
        ..coarse()
    };
    let (followed, notes) = finish(&doc, &p);
    assert!(
        !followed.across.is_empty(),
        "held across the axis: {notes:?}"
    );
    // over the axis A turns steadily, so few cuts are with a face straight up; following,
    // most are: the bit moves across each face with A held
    let (before, after) = (facing_up(&turns(&over)), facing_up(&turns(&followed)));
    assert!(before < 0.3, "over the axis: {before}");
    assert!(after > 0.6, "following: {after}");
    for (m, &across) in followed.moves.iter().zip(&followed.across) {
        if m.kind != MoveKind::Cut {
            continue;
        }
        let angle = m.to.y / 20.0;
        let model = (FLAT / angle.cos().abs().max(angle.sin().abs())).min(20.0) - 20.0;
        assert!(
            m.to.z >= model - 0.3,
            "{:?} held {across} under {model}",
            m.to
        );
    }
    // and it gets right down onto the faces
    let deepest = cuts(&followed)
        .iter()
        .map(|c| c.z)
        .fold(f64::INFINITY, f64::min);
    assert!(deepest < FLAT - 20.0 + 0.3, "{deepest}");
}
