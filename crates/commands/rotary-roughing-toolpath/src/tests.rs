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

cut_craft_core::contract_tests!(RotaryRoughingToolpath, beaded);

fn mill(diameter: f64) -> Tool {
    Tool {
        diameter,
        stepover: 0.4,
        pass_depth: 3.0,
        ..Params::default().tool
    }
}

fn rough(doc: &Document, p: &Params) -> (Toolpath, Vec<String>) {
    made(doc, p).unwrap()
}

fn cuts(t: &Toolpath) -> Vec<DVec3> {
    t.moves
        .iter()
        .filter(|m| m.kind == MoveKind::Cut)
        .map(|m| m.to)
        .collect()
}

#[test]
fn the_bulk_comes_off_down_to_the_allowance_and_never_into_the_model() {
    let doc = beaded();
    for way in [Way::AlongThePart, Way::RoundThePart] {
        let p = Params {
            tool: mill(6.0),
            way,
            ..Params::default()
        };
        let (toolpath, notes) = rough(&doc, &p);
        let cuts = cuts(&toolpath);
        assert!(!cuts.is_empty(), "{way:?} {notes:?}");
        // never below the model and its allowance, under any of the bit
        for c in &cuts {
            let mut under = 0.0f64;
            let mut x = c.x - 3.0;
            while x <= c.x + 3.0 {
                under = under.max(bead(x.clamp(0.0, 60.0)));
                x += 0.25;
            }
            // (at most the blank's own surface: the allowance there is above it), as near as
            // the model's cells see it
            assert!(
                20.0 + c.z >= (under + 0.5).min(20.0) - 0.25,
                "{way:?}: {c:?} under {under}"
            );
        }
        // down to the model in the middle, where there is most to take off
        let deepest = cuts.iter().map(|c| c.z).fold(f64::INFINITY, f64::min);
        assert!(deepest < -6.5, "{way:?} {deepest}");
    }
}

#[test]
fn each_level_goes_no_deeper_than_the_pass_depth() {
    let doc = beaded();
    let p = Params {
        tool: mill(6.0),
        ..Params::default()
    };
    let (toolpath, _) = rough(&doc, &p);
    // the first cut is at the first level: one pass depth into the blank
    let first = cuts(&toolpath)[0];
    assert!(first.z >= -3.0 - 1e-6, "{first:?}");
    // and the levels come down in turn: nothing deeper before something shallower is done
    let zs: Vec<f64> = cuts(&toolpath).iter().map(|c| c.z).collect();
    let mut deepest = 0.0f64;
    for z in zs {
        assert!(z >= deepest - 3.0 - 1e-6, "{z} after {deepest}");
        deepest = deepest.min(z);
    }
}

#[test]
fn along_the_part_runs_the_length_with_a_still_then_steps_round() {
    let doc = beaded();
    let p = Params {
        tool: mill(6.0),
        way: Way::AlongThePart,
        cut_direction: CutDirection::Climb,
        ..Params::default()
    };
    let (toolpath, _) = rough(&doc, &p);
    // every cut moves along the part with A held still, and every one the same way: along X
    // the sheet counts up, which climbs with the spindle turning clockwise
    let mut along = 0;
    for w in toolpath.moves.windows(2) {
        if w[1].kind == MoveKind::Cut && w[0].kind == MoveKind::Cut {
            let d = w[1].to - w[0].to;
            if d.x.abs() > 1e-6 {
                assert!(d.y.abs() < 1e-6, "{w:?}");
                assert!(d.x > 0.0, "{w:?}");
                along += 1;
            }
        }
    }
    assert!(along > 20);
}

#[test]
fn round_the_part_turns_a_and_steps_along_between_rings() {
    let doc = beaded();
    let p = Params {
        tool: mill(6.0),
        way: Way::RoundThePart,
        ..Params::default()
    };
    let (toolpath, _) = rough(&doc, &p);
    let mut round = 0;
    for w in toolpath.moves.windows(2) {
        if w[1].kind == MoveKind::Cut && w[0].kind == MoveKind::Cut {
            let d = w[1].to - w[0].to;
            if d.y.abs() > 1e-6 {
                assert!(d.x.abs() < 1e-6, "{w:?}");
                round += 1;
            }
        }
    }
    assert!(round > 20);
}

#[test]
fn it_cuts_only_what_the_toolpaths_above_it_leave() {
    let mut doc = beaded();
    let p = Params {
        tool: mill(6.0),
        ..Params::default()
    };
    let first = RotaryRoughingToolpath::run(&doc, &p).unwrap();
    doc.apply(&first).unwrap();
    // a second the same: the first took it all, so nothing is left for it
    let (again, notes) = rough(&doc, &p);
    assert!(cuts(&again).is_empty(), "{} cuts", cuts(&again).len());
    assert!(notes.iter().any(|n| n == NOTHING_LEFT), "{notes:?}");
    // with less left on the model, only the last of it is cut
    let closer = Params {
        allowance: 0.2,
        ..p.clone()
    };
    let (closer, _) = rough(&doc, &closer);
    let length = |t: &Toolpath| -> f64 {
        t.moves
            .windows(2)
            .filter(|w| w[1].kind == MoveKind::Cut)
            .map(|w| w[0].to.distance(w[1].to))
            .sum()
    };
    let all = length(&rough(&beaded(), &p).0);
    assert!(length(&closer) > 0.0);
    // the steps the levels left above the model, not the bulk again
    assert!(length(&closer) < 0.75 * all, "{} of {all}", length(&closer));
    // Settings' advanced rotary mode: the whole blank, whatever is above it
    let whole = Params {
        whole_blank: true,
        ..p
    };
    assert_ne!(
        cuts(&rough(&doc, &whole).0).len(),
        0,
        "the whole blank again"
    );
}

#[test]
fn a_flat_job_is_refused() {
    let doc = Document::default();
    assert!(made(&doc, &Params::default()).is_err());
}

#[test]
fn changing_the_model_makes_it_out_of_date() {
    let doc = beaded();
    let (toolpath, _) = rough(
        &doc,
        &Params {
            tool: mill(6.0),
            ..Params::default()
        },
    );
    assert!(toolpath.reads_model(), "it cuts the model");
}
