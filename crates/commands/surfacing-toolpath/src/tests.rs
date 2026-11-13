use super::*;
use cut_craft_core::testing::{document_with, made_toolpath, square};
use cut_craft_core::toolpath::{ToolShape, Toolpath};

cut_craft_core::contract_tests!(SurfacingToolpath);

/// The toolpath `p` makes on a default job (600 × 400 mm material, 20 mm thick) with
/// `paths` selected.
fn made(paths: Vec<Path>, p: &Params) -> Toolpath {
    made_toolpath::<SurfacingToolpath>(paths, p).unwrap()
}

fn job_made(doc: &Document, p: &Params) -> Toolpath {
    let mut doc = doc.clone();
    let change = SurfacingToolpath::run(&doc, p).unwrap();
    doc.apply(&change).unwrap();
    doc.toolpaths.into_values().next().unwrap()
}

/// Every move that goes down below `top` does it with the bit clear of `area` (by at least
/// its radius): a surfacing bit never plunges into the work.
fn never_plunges_into(t: &Toolpath, area: Rect, top: f64) {
    let r = t.tool.diameter / 2.0;
    for m in &t.moves {
        if m.kind == MoveKind::Plunge && m.to.z < top - 1e-9 {
            let q = m.to.truncate();
            let dx = (area.min.x - q.x).max(q.x - area.max.x).max(0.0);
            let dy = (area.min.y - q.y).max(q.y - area.max.y).max(0.0);
            assert!(
                dx.hypot(dy) >= r - 1e-3,
                "plunged into the work at {q:?} (area {area:?})"
            );
        }
    }
}

fn material() -> Rect {
    Rect::new(Point::ZERO, Point::new(600.0, 400.0))
}

fn cut_ys(t: &Toolpath) -> Vec<f64> {
    let mut ys: Vec<f64> = t
        .moves
        .iter()
        .filter(|m| m.kind == MoveKind::Cut)
        .map(|m| m.to.y)
        .collect();
    ys.sort_by(f64::total_cmp);
    ys.dedup_by(|a, b| (*a - *b).abs() < 1e-3);
    ys
}

#[test]
fn whole_material_rows_run_past_every_edge_a_stepover_apart_at_the_skim_depth() {
    let p = Params::default();
    let t = made(vec![], &p);
    let (d, o) = (p.tool.diameter, p.overhang / 100.0 * p.tool.diameter);
    let cuts: Vec<DVec3> = t
        .moves
        .iter()
        .filter(|m| m.kind != MoveKind::Rapid)
        .map(|m| m.to)
        .collect();
    assert!(
        cuts.iter().all(|c| (c.z + 0.25).abs() < 1e-9),
        "all at -0.25"
    );
    let xs = || cuts.iter().map(|c| c.x);
    assert!(xs().fold(f64::INFINITY, f64::min) <= -o + 1e-3);
    assert!(xs().fold(f64::NEG_INFINITY, f64::max) >= 600.0 + o - 1e-3);
    let ys = cut_ys(&t);
    assert!((ys[0] + o).abs() < 1e-3, "{ys:?}");
    assert!((ys[ys.len() - 1] - (400.0 + o)).abs() < 1e-3, "{ys:?}");
    for w in ys.windows(2) {
        assert!(w[1] - w[0] <= 0.75 * d + 1e-3, "rows {w:?} too far apart");
    }
    never_plunges_into(&t, material(), 0.0);
}

#[test]
fn back_and_forth_goes_down_once_and_stays_down() {
    let t = made(vec![], &Params::default());
    let plunges = t
        .moves
        .iter()
        .filter(|m| m.kind == MoveKind::Plunge)
        .count();
    assert_eq!(plunges, 1, "one way in, beside the material");
    let first = t.moves.iter().find(|m| m.kind == MoveKind::Plunge).unwrap();
    assert!(
        first.to.x > 600.0,
        "climbs: goes down right of the material: {first:?}"
    );
}

#[test]
fn one_way_climb_runs_every_row_right_to_left_going_down_beside_the_material_each_time() {
    let p = Params {
        rows: Rows::OneWay,
        ..Params::default()
    };
    let t = made(vec![], &p);
    let rows = cut_ys(&t).len();
    let plunges: Vec<&Move> = t
        .moves
        .iter()
        .filter(|m| m.kind == MoveKind::Plunge)
        .collect();
    assert_eq!(plunges.len(), rows);
    assert!(
        plunges.iter().all(|m| m.to.x > 600.0),
        "each from the right"
    );
    for w in t.moves.windows(2) {
        if w[0].kind == MoveKind::Cut && w[1].kind == MoveKind::Cut {
            assert!(w[1].to.x <= w[0].to.x + 1e-9, "a row ran left to right");
        }
    }
    never_plunges_into(&t, material(), 0.0);
}

#[test]
fn rows_along_y_step_across_x() {
    let p = Params {
        pattern: Pattern::AlongY,
        ..Params::default()
    };
    let t = made(vec![], &p);
    let long = t
        .moves
        .windows(2)
        .filter(|w| w[1].kind == MoveKind::Cut)
        .any(|w| (w[1].to.y - w[0].to.y).abs() > 400.0 && (w[1].to.x - w[0].to.x).abs() < 1e-9);
    assert!(long, "rows run the length of Y");
    never_plunges_into(&t, material(), 0.0);
}

#[test]
fn spiral_goes_round_from_the_outside_in_clockwise_to_climb_with_one_way_in() {
    let p = Params {
        pattern: Pattern::Spiral,
        ..Params::default()
    };
    let t = made(vec![], &p);
    let plunges = t
        .moves
        .iter()
        .filter(|m| m.kind == MoveKind::Plunge)
        .count();
    assert_eq!(plunges, 1);
    never_plunges_into(&t, material(), 0.0);
    let cuts: Vec<Point> = t
        .moves
        .iter()
        .filter(|m| m.kind == MoveKind::Cut)
        .map(|m| m.to.truncate())
        .collect();
    // the first ring, outside the material, then ever further in
    let o = p.overhang / 100.0 * p.tool.diameter;
    assert!(cuts.iter().take(5).any(|q| q.x <= -o + 1e-3));
    let last = cuts[cuts.len() - 1];
    assert!(
        (last.y - 200.0).abs() < 1.0,
        "ends along the middle, at {last:?}"
    );
    // clockwise: the first ring's turn
    let ring: Vec<Point> = cuts.iter().take(6).copied().collect();
    assert!(winding(&ring) < 0.0, "{ring:?}");
}

#[test]
fn more_passes_take_it_off_in_steps() {
    let mut p = Params {
        depth: 1.0,
        ..Params::default()
    };
    p.tool.pass_depth = 0.5;
    let t = made(vec![], &p);
    let mut zs: Vec<f64> = t
        .moves
        .iter()
        .filter(|m| m.kind == MoveKind::Cut)
        .map(|m| m.to.z)
        .collect();
    zs.sort_by(f64::total_cmp);
    zs.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    assert_eq!(zs, [-1.0, -0.5]);
    never_plunges_into(&t, material(), 0.0);
}

#[test]
fn spoilboard_is_flattened_from_x0_y0_and_its_top_is_the_bed_when_zeroed_there() {
    let mut doc = document_with(vec![]);
    doc.job.z_zero = ZZero::MachineBed;
    let p = Params {
        area: Area::Spoilboard,
        spoilboard_width: 800.0,
        spoilboard_height: 500.0,
        ..Params::default()
    };
    let t = job_made(&doc, &p);
    let o = p.overhang / 100.0 * p.tool.diameter;
    let z = -doc.job.thickness - 0.25;
    assert!(
        t.moves
            .iter()
            .filter(|m| m.kind == MoveKind::Cut)
            .all(|m| (m.to.z - z).abs() < 1e-9)
    );
    let ys = cut_ys(&t);
    assert!((ys[ys.len() - 1] - (500.0 + o)).abs() < 1e-3);
    never_plunges_into(
        &t,
        Rect::new(Point::ZERO, Point::new(800.0, 500.0)),
        -doc.job.thickness,
    );
}

#[test]
fn a_spoilboard_filling_the_bed_stays_on_it_and_ramps_in_instead_of_plunging() {
    let p = Params {
        area: Area::Spoilboard,
        spoilboard_width: 800.0,
        spoilboard_height: 500.0,
        bed: [800.0, 500.0],
        ..Params::default()
    };
    let t = made(vec![], &p);
    for m in &t.moves {
        let q = m.to;
        assert!(
            (0.0..=800.0).contains(&q.x) && (0.0..=500.0).contains(&q.y),
            "off the bed at {q:?}"
        );
        if m.kind == MoveKind::Plunge {
            assert!(q.z >= -1e-9, "plunged into the board at {q:?}");
        }
    }
    // the ramp: down to the cut depth while moving along
    let ramp = t.moves.windows(2).any(|w| {
        w[1].kind == MoveKind::Cut
            && w[0].to.z > w[1].to.z + 0.1
            && w[0].to.truncate() != w[1].to.truncate()
    });
    assert!(ramp, "ramps in");
}

#[test]
fn spoilboard_needs_a_size() {
    let p = Params {
        area: Area::Spoilboard,
        ..Params::default()
    };
    let e = SurfacingToolpath::run(&document_with(vec![]), &p).unwrap_err();
    assert!(e.to_string().contains("width and height"), "{e}");
}

#[test]
fn selected_vectors_flatten_inside_the_outline_and_need_one() {
    let p = Params {
        area: Area::SelectedVectors,
        ..Params::default()
    };
    let mut doc = document_with(vec![square(100.0, 100.0, 200.0)]);
    doc.selection = doc.vectors.keys().copied().collect();
    let t = job_made(&doc, &p);
    let o = p.overhang / 100.0 * p.tool.diameter;
    let ys = cut_ys(&t);
    assert!((ys[0] - (100.0 - o)).abs() < 1e-3, "{ys:?}");
    assert!((ys[ys.len() - 1] - (300.0 + o)).abs() < 1e-3, "{ys:?}");
    never_plunges_into(
        &t,
        Rect::new(Point::new(100.0, 100.0), Point::new(300.0, 300.0)),
        0.0,
    );
    assert_eq!(t.vectors.len(), 1, "made from the outline");
    let e = SurfacingToolpath::run(&document_with(vec![]), &p).unwrap_err();
    assert!(e.to_string().contains("closed outline"), "{e}");
}

#[test]
fn a_tiny_bit_stepover_or_no_depth_says_what_to_fix() {
    let mut p = Params {
        depth: 0.0,
        ..Params::default()
    };
    assert!(SurfacingToolpath::run(&document_with(vec![]), &p).is_err());
    p.depth = 0.25;
    p.tool.stepover = 0.0;
    let e = SurfacingToolpath::run(&document_with(vec![]), &p).unwrap_err();
    assert!(e.to_string().contains("stepover"), "{e}");
}

#[test]
fn the_form_starts_with_a_surfacing_bit_and_a_light_skim() {
    let p = Params::default();
    assert_eq!(p.tool.shape, ToolShape::SurfacingBit);
    assert!((p.tool.stepover - 0.75).abs() < 1e-9);
    assert!((p.depth - 0.25).abs() < 1e-9);
    assert!(cuts_spoilboard(&serde_json::json!({"area": "spoilboard"})));
    assert!(!cuts_spoilboard(
        &serde_json::json!({"area": "whole_material"})
    ));
}

#[test]
fn an_l_shaped_slab_is_never_plunged_into_whatever_the_pattern() {
    let at = Point::new;
    let slab = Path::polyline(
        &[
            at(0.0, 0.0),
            at(300.0, 0.0),
            at(300.0, 100.0),
            at(100.0, 100.0),
            at(100.0, 300.0),
            at(0.0, 300.0),
        ],
        true,
    );
    let rings = [cut_points(&slab)];
    let mut doc = document_with(vec![slab]);
    doc.selection = doc.vectors.keys().copied().collect();
    for pattern in [Pattern::AlongX, Pattern::AlongY, Pattern::Spiral] {
        for rows in [Rows::BackAndForth, Rows::OneWay] {
            let p = Params {
                area: Area::SelectedVectors,
                pattern,
                rows,
                ..Params::default()
            };
            let t = job_made(&doc, &p);
            let radius = p.tool.diameter / 2.0;
            for m in &t.moves {
                if m.kind == MoveKind::Plunge && m.to.z < -1e-9 {
                    let q = m.to.truncate();
                    assert!(
                        !cut_craft_core::geometry::inside(&rings, q)
                            && polyline_distance(&closed(&rings[0]), q) >= radius - 1e-3,
                        "{pattern:?} {rows:?} plunged into the slab at {q:?}"
                    );
                }
            }
        }
    }
}
