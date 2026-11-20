//! From the model to the file and back: a full 3D model on a rotary job, finished by 4-Axis
//! Rotary Machining, written by the post, read back move by move and cut into the material
//! by the simulator. What is left must be the model, never cut into, with the material
//! round it gone.

use cmd_rotary_machining_toolpath::{Params, RotaryMachiningToolpath, Strategy};
use cut_craft_core::component::{Combine, Component};
use cut_craft_core::document::{Along, Document, Job};
use cut_craft_core::glam::DVec3;
use cut_craft_core::rotary_axis::RotaryAxis;
use cut_craft_core::{Change, Command};
use cut_craft_post::{File, Post};
use cut_craft_rotary::gcode::{Reading, read};
use cut_craft_rotary::{Cutter, Pose, Section, Stock, Voxels};

const RADIUS: f64 = 20.0;
/// The box held off to one side of the axis, in the part's frame.
const LO: DVec3 = DVec3::new(10.0, -6.0, -8.0);
const HI: DVec3 = DVec3::new(50.0, 12.0, 10.0);

fn boxed() -> Document {
    let mut doc = Document {
        job: Job::rotary(60.0, 2.0 * RADIUS, Along::X),
        ..Document::default()
    };
    let at = |i: usize| {
        DVec3::new(
            if i & 1 == 1 { HI.x } else { LO.x },
            if i & 2 == 2 { HI.y } else { LO.y },
            if i & 4 == 4 { HI.z } else { LO.z },
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
    let mut model = Component::from_fn("Box", Combine::Add, doc.job.area(), 0.25, |_| Some(RADIUS));
    model.set_solid(&triangles);
    let mut c = Change::builder(&doc);
    c.add_new_component(model);
    doc.apply(&c.finish()).unwrap();
    doc
}

/// How far `p` is outside the box: 0 or less inside.
fn outside(p: DVec3) -> f64 {
    let d = (LO - p).max(p - HI);
    if d.max_element() <= 0.0 {
        d.max_element()
    } else {
        d.max(DVec3::ZERO).length()
    }
}

fn file(doc: &Document, axis: RotaryAxis) -> String {
    let post = Post::default_for(&doc.job).with_rotary(axis);
    post.render(
        doc,
        &File {
            path: std::path::Path::new("/jobs/box.nc"),
            job: None,
            made: chrono::NaiveDateTime::default(),
            slots: &[],
            notes: &[],
        },
    )
    .unwrap()
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "small cell counts"
)]
fn finished(strategy: Strategy, clockwise: bool) {
    let mut doc = boxed();
    let mut p = Params {
        strategy,
        offset: 4.0,
        ..Params::default()
    };
    // flutes long enough to reach the axis where there is no model: the simulator cuts with
    // the flutes alone, as the bit does
    p.tool.flute_length = RADIUS + 5.0;
    let tool = p.tool.clone();
    let change = RotaryMachiningToolpath::run(&doc, &p).unwrap();
    doc.apply(&change).unwrap();
    let axis = RotaryAxis {
        centre_set: true,
        centre_across: 3.0,
        clockwise,
        ..RotaryAxis::default()
    };
    let text = file(&doc, axis);
    let moves = read(&text, &Reading::of(&doc.job, &axis).expect("a rotary job"));
    assert!(moves.len() > 100, "{text}");
    let cutter = Cutter::new(&tool);
    let stock = Stock::new(
        Section::Round {
            diameter: 2.0 * RADIUS,
        },
        60.0,
    );
    let mut material = Voxels::of(&stock, 0.6);
    let mut at: Option<Pose> = None;
    for m in &moves {
        if let Some(from) = at {
            material.cut(&cutter, from, m.to);
        }
        at = Some(m.to);
    }
    let cell = material.cell();
    // nothing inside the box (a cell's middle half a cell in) was cut; everything a little
    // way out from it, along the length the rings cover, was
    let mut kept = 0;
    let steps = (2.0 * RADIUS / cell) as i64;
    for i in 0..(60.0 / cell) as i64 {
        for j in -steps..=steps {
            for k in -steps..=steps {
                let p = DVec3::new((i as f64 + 0.5) * cell, j as f64 * cell, k as f64 * cell);
                if !stock.holds(p) {
                    continue;
                }
                let away = outside(p);
                if away < -cell {
                    assert!(material.solid(p), "{strategy:?}: cut into the box at {p}");
                    kept += 1;
                } else if away > 1.5
                    && (2.0..58.0).contains(&p.x)
                    // held off centre, the bit can't reach right in to the axis
                    && (strategy != Strategy::OffCentre || p.y.hypot(p.z) > 4.0)
                {
                    assert!(!material.solid(p), "{strategy:?}: left standing at {p}");
                }
            }
        }
    }
    assert!(kept > 1000, "{kept}");
}

#[test]
fn rings_round_a_full_model_written_and_read_back_cut_it_and_only_it() {
    finished(Strategy::Radial, true);
}

#[test]
fn off_centre_rings_written_and_read_back_cut_it_and_only_it_either_way_round() {
    finished(Strategy::OffCentre, true);
    finished(Strategy::OffCentre, false);
}
