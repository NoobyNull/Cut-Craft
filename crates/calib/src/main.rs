//! Blind-test calibration: `cut-craft-calib <tool> <aspire-result.crv3d> <aspire-baseline.crv3d>`.
//!
//! Runs the named Cut Craft tool on `re/extracted/calib/calibration-input.dxf` and compares
//! what it made with the vectors Aspire added to the same job (the result file's vectors that
//! are not in the baseline). Tools: offset-round, offset-sharp, fillet, weld, fit.

use cut_craft_core::document::{Document, EntityId, Job};
use cut_craft_core::geometry::{Path, Point, Rect, polyline_distance};
use cut_craft_core::params::Xy;
use cut_craft_core::{Command, Importer};
use std::path::Path as FsPath;

const INPUT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../re/calib/calibration-input.dxf"
);
const FLAT: f64 = 0.01;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let [_, tool, result, baseline] = args.as_slice() else {
        eprintln!(
            "usage: cut-craft-calib <offset-round|offset-sharp|fillet|weld|fit> RESULT.crv3d BASELINE.crv3d"
        );
        std::process::exit(2);
    };
    let ours = cut_craft(tool);
    let theirs = aspire_added(FsPath::new(result), FsPath::new(baseline));
    println!(
        "{tool}: Cut Craft made {} vectors, Aspire added {}",
        ours.len(),
        theirs.len()
    );
    for (i, mine) in ours.iter().enumerate() {
        let Some(closest) = theirs
            .iter()
            .min_by(|a, b| centre_gap(a, mine).total_cmp(&centre_gap(b, mine)))
        else {
            println!("  #{i}: nothing of Aspire's to compare with");
            continue;
        };
        let (d_ab, d_ba) = (hausdorff(mine, closest), hausdorff(closest, mine));
        println!(
            "  #{i}: nodes {} vs {} | area {:.3} vs {:.3} | bounds {} vs {} | apart {:.4} mm (max either way)",
            mine.segments.len(),
            closest.segments.len(),
            area(mine),
            area(closest),
            show(mine.bounds()),
            show(closest.bounds()),
            d_ab.max(d_ba)
        );
    }
}

/// The calibration job with the DXF imported, every vector selected.
fn job() -> Document {
    let mut doc = Document {
        job: Job {
            width: 200.0,
            height: 150.0,
            thickness: 20.0,
            ..Job::default()
        },
        ..Document::default()
    };
    let bytes = std::fs::read(INPUT).expect("the calibration DXF");
    let change = fmt_import_dxf::ImportDxf::import(&doc, &bytes).expect("the DXF imports");
    doc.apply(&change).expect("applies");
    doc
}

/// Vectors whose box has its lower-left corner near `at`.
fn pick(doc: &Document, at: &[(f64, f64)]) -> Vec<EntityId> {
    doc.vectors
        .iter()
        .filter(|(_, p)| {
            at.iter()
                .any(|&(x, y)| p.bounds().min.distance(Point::new(x, y)) < 1.0)
        })
        .map(|(&id, _)| id)
        .collect()
}

fn cut_craft(tool: &str) -> Vec<Path> {
    let mut doc = job();
    let (square, star, zigzag, pair) = (
        (10.0, 10.0),
        (76.22, 19.77),
        (10.0, 100.0),
        [(120.0, 90.0), (135.0, 105.0)],
    );
    let change = match tool {
        "offset-round" | "offset-sharp" => {
            doc.selection = pick(&doc, &[square, star]).into_iter().collect();
            let p = cmd_offset_vectors::Params {
                distance: 3.0,
                corners: if tool == "offset-round" {
                    cut_craft_core::polygon::Corners::Round
                } else {
                    cut_craft_core::polygon::Corners::Sharp
                },
                keep_original: true,
            };
            cmd_offset_vectors::OffsetVectors::run(&doc, &p)
        }
        "fillet" => {
            // ponytail: the five outer points one after another, each on the path as it is then
            let id = pick(&doc, &[star])[0];
            let mut doc = doc.clone();
            for k in 0..5 {
                let a = (90.0 + 72.0 * f64::from(k)).to_radians();
                let p = cmd_create_fillets::Params {
                    radius: 5.0,
                    fillet_type: cmd_create_fillets::FilletType::Normal,
                    corner: Xy::new(100.0 + 25.0 * a.cos(), 40.0 + 25.0 * a.sin()),
                };
                let c = cmd_create_fillets::CreateFillets::run(&doc, &p).expect("a corner there");
                doc.apply(&c).expect("applies");
            }
            return vec![doc.vectors[&id].clone()];
        }
        "weld" => {
            doc.selection = pick(&doc, &pair).into_iter().collect();
            cmd_weld_vectors::WeldVectors::run(&doc, &cut_craft_core::NoParams {})
        }
        "fit" => {
            doc.selection = pick(&doc, &[zigzag]).into_iter().collect();
            let p = cmd_fit_curves_to_vectors::Params {
                fitting_type: cmd_fit_curves_to_vectors::FittingType::CircularArcs,
                tolerance: 0.5,
                keep_sharp_corners: true,
                max_angle: 20.0,
                replace_selected_vectors: true,
            };
            cmd_fit_curves_to_vectors::FitCurvesToVectors::run(&doc, &p)
        }
        other => {
            eprintln!("unknown tool {other}");
            std::process::exit(2);
        }
    };
    change.expect("the tool runs").added().cloned().collect()
}

/// The result file's vectors that are not in the baseline (by bounds).
fn aspire_added(result: &FsPath, baseline: &FsPath) -> Vec<Path> {
    let load = |p: &FsPath| {
        let mut doc = Document::default();
        let bytes = std::fs::read(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
        let change = fmt_import_crv3d::ImportCrv3d::import(&doc, &bytes).expect("the job imports");
        doc.apply(&change).expect("applies");
        doc.vectors.into_values().collect::<Vec<Path>>()
    };
    let (after, before) = (load(result), load(baseline));
    after
        .into_iter()
        .filter(|p| !before.iter().any(|b| same_box(b.bounds(), p.bounds())))
        .collect()
}

fn same_box(a: Rect, b: Rect) -> bool {
    a.min.distance(b.min) < 1e-3 && a.max.distance(b.max) < 1e-3
}

fn centre_gap(a: &Path, b: &Path) -> f64 {
    a.bounds().center().distance(b.bounds().center())
}

/// How far any point of `a` is from `b`, at most.
fn hausdorff(a: &Path, b: &Path) -> f64 {
    let pb = b.flatten(FLAT);
    a.flatten(FLAT)
        .iter()
        .map(|&q| polyline_distance(&pb, q))
        .fold(0.0, f64::max)
}

fn area(p: &Path) -> f64 {
    if p.closed {
        cut_craft_core::polygon::area(p)
    } else {
        0.0
    }
}

fn show(r: Rect) -> String {
    format!(
        "[{:.2},{:.2}]-[{:.2},{:.2}]",
        r.min.x, r.min.y, r.max.x, r.max.y
    )
}
