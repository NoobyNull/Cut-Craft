use super::*;
use cut_craft_core::component::{Combine, Component};
use cut_craft_core::document::{Along, Job};
use cut_craft_core::glam::DVec3;
use cut_craft_core::toolpath::MoveKind;

/// A rotary job 40 mm across, 60 mm long along X: a plain round part the blank's own size
/// with a groove round it at 30 mm along, 3 mm wide and 4 mm deep.
fn grooved() -> Document {
    let job = Job::rotary(60.0, 40.0, Along::X);
    let mut doc = Document {
        job,
        ..Document::default()
    };
    let model = Component::from_fn("Grooved", Combine::Add, doc.job.area(), 0.25, |p| {
        Some(if (p.x - 30.0).abs() < 1.5 { 16.0 } else { 20.0 })
    });
    let mut c = Change::builder(&doc);
    c.add_new_component(model);
    doc.apply(&c.finish()).unwrap();
    doc
}

cut_craft_core::contract_tests!(RotaryDetailsToolpath, grooved);

fn ball(diameter: f64) -> Tool {
    Tool {
        name: format!("{diameter} mm ball"),
        diameter,
        stepover: 0.4,
        finish_stepover: 0.25,
        ..Params::default().tool
    }
}

fn cuts(t: &Toolpath) -> Vec<DVec3> {
    t.moves
        .iter()
        .filter(|m| m.kind == MoveKind::Cut)
        .map(|m| m.to)
        .collect()
}

fn after_a_big_bit() -> Params {
    Params {
        tool: ball(2.0),
        bits_before: vec![ball(10.0)],
        whole_blank: true,
        ..Params::default()
    }
}

#[test]
fn the_detail_bit_cuts_only_the_groove_the_big_bit_could_not_get_into() {
    let (toolpath, notes) = made(&grooved(), &after_a_big_bit()).unwrap();
    let cuts = cuts(&toolpath);
    assert_ne!(cuts.len(), 0, "{notes:?}");
    // only in and beside the groove: the big bit took the rest
    for c in &cuts {
        assert!((c.x - 30.0).abs() < 3.0, "{c:?}");
    }
    // right down to the groove's floor, and never below it
    let deepest = cuts.iter().map(|c| c.z).fold(f64::INFINITY, f64::min);
    assert!(deepest < -3.7, "{deepest}");
    assert!(deepest > -4.0 - 0.25, "{deepest}");
}

#[test]
fn a_finish_above_it_in_the_list_counts_as_a_bit_before() {
    let mut doc = grooved();
    let finish = cmd_rotary_finishing_toolpath::Params {
        tool: ball(10.0),
        ..cmd_rotary_finishing_toolpath::Params::default()
    };
    let finished =
        cmd_rotary_finishing_toolpath::RotaryFinishingToolpath::run(&doc, &finish).unwrap();
    doc.apply(&finished).unwrap();
    let p = Params {
        tool: ball(2.0),
        ..Params::default()
    };
    let (toolpath, notes) = made(&doc, &p).unwrap();
    let cuts = cuts(&toolpath);
    assert_ne!(cuts.len(), 0, "{notes:?}");
    for c in &cuts {
        assert!((c.x - 30.0).abs() < 3.0, "{c:?}");
    }
}

#[test]
fn a_detail_bit_no_smaller_than_the_bit_before_has_nothing_to_cut() {
    let p = Params {
        tool: ball(10.0),
        ..after_a_big_bit()
    };
    let (toolpath, notes) = made(&grooved(), &p).unwrap();
    assert_eq!(cuts(&toolpath).len(), 0);
    assert!(notes.iter().any(|n| n == NOTHING_LEFT), "{notes:?}");
}

#[test]
fn a_flat_job_is_refused() {
    assert!(made(&Document::default(), &Params::default()).is_err());
}

#[test]
fn changing_the_model_makes_it_out_of_date() {
    let (toolpath, _) = made(&grooved(), &after_a_big_bit()).unwrap();
    assert!(toolpath.reads_model(), "it cuts the model");
}
