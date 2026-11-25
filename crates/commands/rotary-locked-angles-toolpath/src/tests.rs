use super::*;
use cut_craft_core::component::{Combine, Component};
use cut_craft_core::document::{Along, Job};
use cut_craft_core::glam::DVec3;
use cut_craft_core::toolpath::MoveKind;

/// A rotary job 40 mm across, 60 mm long along X, holding a part `radius(x, angle)` from the
/// axis (its top at the material's surface; angle in radians round from A=0).
fn shaped(radius: impl Fn(f64, f64) -> f64) -> Document {
    let job = Job::rotary(60.0, 40.0, Along::X);
    let mut doc = Document {
        job,
        ..Document::default()
    };
    let model = Component::from_fn("Shaped", Combine::Add, doc.job.area(), 0.25, |p| {
        Some(radius(p.x, p.y / 20.0).min(20.0))
    });
    let mut c = Change::builder(&doc);
    c.add_new_component(model);
    doc.apply(&c.finish()).unwrap();
    doc
}

/// How far a square bar's flats are from the axis: its corners just touch the material's
/// surface (the model's top is put at the material's top).
const FLAT: f64 = 20.0 * std::f64::consts::FRAC_1_SQRT_2;

/// A square bar, its faces square to A=0, 90°, 180° and 270°.
fn square(_: f64, angle: f64) -> f64 {
    FLAT / angle.cos().abs().max(angle.sin().abs())
}

fn squared() -> Document {
    shaped(square)
}

/// The model's radius along a bead: 20 mm round at the ends, 12 mm in the middle.
fn bead(x: f64, _: f64) -> f64 {
    20.0 - 8.0 * (std::f64::consts::PI * x / 60.0).sin()
}

cut_craft_core::contract_tests!(RotaryLockedAnglesToolpath, squared);

fn ball(diameter: f64) -> Tool {
    Tool {
        diameter,
        stepover: 0.4,
        finish_stepover: 0.25,
        ..Params::default().tool
    }
}

fn coarse() -> Params {
    Params {
        tool: ball(6.0),
        ..Params::default()
    }
}

fn cuts(t: &Toolpath) -> Vec<DVec3> {
    t.moves
        .iter()
        .filter(|m| m.kind == MoveKind::Cut)
        .map(|m| m.to)
        .collect()
}

/// Where A is (degrees) at each cut: the moves keep where the tip cuts, with how far across
/// the bit is held beside them.
fn turns(t: &Toolpath) -> Vec<f64> {
    t.moves
        .iter()
        .zip(&t.across)
        .filter(|(m, _)| m.kind == MoveKind::Cut)
        .map(|(m, &across)| {
            let (round, _) = cut_craft_core::rotary_axis::held_at(20.0, m.to.y, m.to.z, across);
            (round / 20.0).to_degrees().rem_euclid(360.0)
        })
        .collect()
}

/// The angles (degrees) A locked at, in order.
fn angles(doc: &Document, p: &Params) -> Vec<f64> {
    let (toolpath, _) = made(doc, p).unwrap();
    let mut seen: Vec<f64> = vec![];
    for a in turns(&toolpath) {
        let a = (a / 15.0).round() * 15.0 % 360.0;
        if seen.last() != Some(&a) {
            seen.push(a);
        }
    }
    seen
}

#[test]
fn a_square_bar_takes_four_angles_however_many_are_allowed() {
    let doc = squared();
    let p = Params {
        most_angles: 12,
        ..coarse()
    };
    let (toolpath, notes) = made(&doc, &p).unwrap();
    assert_ne!(cuts(&toolpath).len(), 0, "{notes:?}");
    let mut locked = angles(&doc, &p);
    locked.sort_by(f64::total_cmp);
    assert_eq!(locked, vec![0.0, 90.0, 180.0, 270.0], "{notes:?}");
    // and nothing more to say: every face is done
    assert!(
        notes.iter().all(|n| !n.contains("Raise Most angles")),
        "{notes:?}"
    );
}

#[test]
fn each_side_is_cut_with_a_held_still_and_never_into_the_model() {
    let doc = squared();
    let (toolpath, _) = made(&doc, &coarse()).unwrap();
    // A does not move while it cuts: it is only ever at the angles it locked at
    for a in turns(&toolpath) {
        assert!((a / 90.0 - (a / 90.0).round()).abs() < 1e-6, "{a}");
    }
    // the tip never below a face, and right down onto them
    let lowest = cuts(&toolpath)
        .iter()
        .map(|c| c.z)
        .fold(f64::INFINITY, f64::min);
    let flat = FLAT - 20.0;
    assert!(lowest >= flat - 0.25, "{lowest}");
    assert!(lowest < flat + 0.25, "down onto the flats: {lowest}");
}

#[test]
fn too_few_angles_says_more_is_left() {
    let doc = squared();
    let p = Params {
        most_angles: 2,
        ..coarse()
    };
    let (_, notes) = made(&doc, &p).unwrap();
    assert!(
        notes.iter().any(|n| n.contains("Raise Most angles")),
        "{notes:?}"
    );
}

#[test]
fn it_cuts_only_what_the_toolpaths_above_it_leave() {
    let mut doc = squared();
    let first = RotaryLockedAnglesToolpath::run(&doc, &coarse()).unwrap();
    doc.apply(&first).unwrap();
    let (again, notes) = made(&doc, &coarse()).unwrap();
    assert_eq!(cuts(&again).len(), 0);
    assert!(notes.iter().any(|n| n == NOTHING_LEFT), "{notes:?}");
    // Settings' advanced rotary mode: the whole blank, whatever is above it
    let whole = Params {
        whole_blank: true,
        ..coarse()
    };
    assert_ne!(cuts(&made(&doc, &whole).unwrap().0).len(), 0);
}

#[test]
fn after_a_roughing_a_round_part_still_gets_its_finish() {
    let mut doc = shaped(bead);
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
    let (toolpath, notes) = made(&doc, &coarse()).unwrap();
    assert_ne!(cuts(&toolpath).len(), 0, "{notes:?}");
}

#[test]
fn a_flat_job_is_refused() {
    assert!(made(&Document::default(), &Params::default()).is_err());
}

#[test]
fn changing_the_model_makes_it_out_of_date() {
    let (toolpath, _) = made(&squared(), &coarse()).unwrap();
    assert!(toolpath.reads_model(), "it cuts the model");
}

#[test]
fn where_the_holder_keeps_every_angle_off_it_says_so() {
    // a groove 20 mm wide down to a rod 14 mm under the material's top, roughed out above;
    // then a bit sticking out only 3 mm: the collet nut, wider than the groove, can't get
    // down to what the roughing left at any angle
    let mut doc = shaped(|x, _| if (20.0..40.0).contains(&x) { 6.0 } else { 20.0 });
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
    let p = Params {
        tool: Tool {
            flute_length: 3.0,
            stickout: 3.0,
            ..ball(6.0)
        },
        ..coarse()
    };
    let (_, notes) = made(&doc, &p).unwrap();
    assert!(
        notes.iter().any(|n| n.contains("collet nut")),
        "the material it couldn't reach is said: {notes:?}"
    );
    assert!(
        !notes.iter().any(|n| n.starts_with("Nothing is left")),
        "not that nothing is left: {notes:?}"
    );
}
