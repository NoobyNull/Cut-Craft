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
/// Cuts the box with `strategy` and checks what is left; how far the bit went at its
/// cutting feed through nothing but air, as a share of all it went at that feed.
fn finished(strategy: Strategy, clockwise: bool) -> f64 {
    let mut doc = boxed();
    let mut p = Params {
        strategy,
        offset: 4.0,
        faces: 2,
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
    // fine enough to see a thin finishing cut as cutting
    let cell = 0.3;
    let looks = Cutter::new(&cut_craft_core::toolpath::Tool {
        diameter: tool.diameter + 2.0 * cell,
        flute_length: tool.flute_length + cell,
        ..tool.clone()
    });
    let stock = Stock::new(
        Section::Round {
            diameter: 2.0 * RADIUS,
        },
        60.0,
    );
    let mut material = Voxels::of(&stock, cell);
    let mut at: Option<Pose> = None;
    let (mut fed, mut air, mut plunged) = (0.0, 0.0, 0.0);
    for m in &moves {
        if let Some(from) = at {
            // nowhere near the material: nothing within a cell of the bit at either end, before
            // it cuts (a thin finishing cut may find no whole cell, but it is not air)
            let near = |pose: Pose| {
                let lower = Pose {
                    height: pose.height - material.cell(),
                    ..pose
                };
                material.held(&looks, lower, 1) > 0
            };
            let in_air = !m.rapid && !near(from) && !near(m.to);
            let cut = material.cut(&cutter, from, m.to);
            if !m.rapid {
                let far = (m.to.along - from.along)
                    .hypot(m.to.across - from.across)
                    .hypot(m.to.height - from.height)
                    + (m.to.turn - from.turn).abs() * RADIUS;
                fed += far;
                if cut == 0 && in_air {
                    air += far;
                    let down = (m.to.along - from.along).abs() < 1e-6
                        && (m.to.across - from.across).abs() < 1e-6
                        && (m.to.turn - from.turn).abs() < 1e-9;
                    if down {
                        plunged += far;
                    }
                }
            }
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
                    // cut from above and below, the sides each come down to the axis and
                    // no further: where they meet, the ridges between their lines are left
                    && (strategy != Strategy::SidesThenDetails || p.z.abs() > 0.2)
                {
                    assert!(!material.solid(p), "{strategy:?}: left standing at {p}");
                }
            }
        }
    }
    assert!(kept > 1000, "{kept}");
    eprintln!(
        "{strategy:?}: {} moves, fed {fed:.0} mm, air {air:.0} mm of which straight down {plunged:.0} mm",
        moves.len()
    );
    air / fed
}

#[test]
fn rings_round_a_full_model_written_and_read_back_cut_it_and_only_it() {
    finished(Strategy::Radial, true);
}

#[test]
fn bulk_to_fine_detail_written_and_read_back_cuts_it_and_only_it_and_hardly_any_air() {
    // the bulk cleared from each side with A held, each side finished, then rings only
    // where something is left
    let air = finished(Strategy::SidesThenDetails, true);
    assert!(
        air < 0.1,
        "{:.1}% of the feed moves cut only air",
        100.0 * air
    );
    // the same model cut by rings alone goes through air at feed far more
    let rings = finished(Strategy::Radial, true);
    eprintln!(
        "air at feed: bulk to fine {:.1}%, rings {:.1}%",
        100.0 * air,
        100.0 * rings
    );
}

#[test]
fn off_centre_rings_written_and_read_back_cut_it_and_only_it_either_way_round() {
    finished(Strategy::OffCentre, true);
    finished(Strategy::OffCentre, false);
}
