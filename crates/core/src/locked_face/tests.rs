use super::*;
use crate::document::Rotary;
use crate::rotary_axis::held_at;
use crate::toolpath::{Move, Tool};
use glam::DVec3;
use serde_json::json;

/// A rotary job 200 long along X, 40 across, of `blank`.
fn job(blank: Blank) -> Job {
    let mut job = Job::rotary(200.0, 40.0, Along::X);
    job.rotary = Some(Rotary {
        blank,
        ..job.rotary.unwrap()
    });
    job
}

/// A groove `depth` deep at 50 along, from `from` to `to` mm round the sheet, down from and
/// back up to 5 above.
fn groove(job: &Job, from: f64, to: f64, depth: f64) -> Toolpath {
    let c = job.area().min;
    let at = |round: f64, z: f64| DVec3::new(c.x + 50.0, c.y + round, z);
    Toolpath {
        name: "Profile 1".into(),
        strategy: "profile_toolpath".into(),
        params: json!({}),
        tool: Tool::default(),
        vectors: vec![],
        moves: vec![
            Move {
                kind: MoveKind::Rapid,
                to: at(from, 5.0),
            },
            Move {
                kind: MoveKind::Plunge,
                to: at(from, -depth),
            },
            Move {
                kind: MoveKind::Cut,
                to: at(to, -depth),
            },
            Move {
                kind: MoveKind::Rapid,
                to: at(to, 5.0),
            },
        ],
        visible: true,
        power: vec![],
        stale: false,
        group: None,
        drill_bank: None,
        fallbacks: vec![],
        across: vec![],
    }
}

/// Where the post puts A (as mm round the surface) and how far above the axis the tip of
/// the bit is, for each move.
fn written(job: &Job, t: &Toolpath) -> Vec<(f64, f64)> {
    let radius = job.rotary.unwrap().diameter / 2.0;
    let c = job.area().min;
    t.moves
        .iter()
        .zip(&t.across)
        .map(|(m, &across)| {
            let (round, z) = held_at(radius, m.to.y - c.y, m.to.z, across);
            (round, z + radius)
        })
        .collect()
}

fn locked(angle: Option<f64>) -> serde_json::Value {
    json!({ KEY: { "angle": angle } })
}

#[test]
fn a_groove_on_a_round_bar_is_cut_flat_with_a_locked_where_it_is_drawn() {
    let job = job(Blank::Round);
    let mut t = groove(&job, 20.0, 40.0, 2.0);
    apply(&job, &mut t, &locked(None)).unwrap();
    assert_eq!(t.across.len(), t.moves.len());
    // across the face as far as it was drawn round the sheet, either side of its middle
    assert!((t.across[1] + 10.0).abs() < 1e-9, "{:?}", t.across);
    assert!((t.across[2] - 10.0).abs() < 1e-9, "{:?}", t.across);
    for (i, (round, height)) in written(&job, &t).into_iter().enumerate() {
        assert!(
            (round - 30.0).abs() < 1e-9,
            "A stays at the groove's middle: {round}"
        );
        let want = if t.moves[i].kind == MoveKind::Rapid {
            25.0
        } else {
            18.0
        };
        assert!(
            (height - want).abs() < 1e-9,
            "depth from the flat, not the curve: {height}"
        );
    }
    assert_eq!(LockedFace::of(&t.params), Some(LockedFace { angle: None }));
}

#[test]
fn a_square_bar_turns_to_the_nearest_face_and_cuts_down_from_it() {
    let job = job(Blank::Square);
    // a quarter turn round is 10π mm round the sheet: a cut drawn a little past it
    let quarter = 20.0 * FRAC_PI_2;
    let mut t = groove(&job, quarter - 4.0, quarter + 8.0, 3.0);
    apply(&job, &mut t, &locked(None)).unwrap();
    for (round, height) in written(&job, &t).into_iter().skip(1).take(2) {
        assert!((round - quarter).abs() < 1e-9, "A at the face: {round}");
        assert!(
            (height - 17.0).abs() < 1e-9,
            "3 down from the face: {height}"
        );
    }
    assert!((t.across[1] + 4.0).abs() < 1e-9, "{:?}", t.across);
    assert!((t.across[2] - 8.0).abs() < 1e-9, "{:?}", t.across);
}

#[test]
fn a_board_on_its_edge_cuts_down_from_the_edge() {
    let job = job(Blank::Board { width: 100.0 });
    let quarter = 20.0 * FRAC_PI_2;
    let mut t = groove(&job, quarter - 2.0, quarter + 2.0, 1.0);
    apply(&job, &mut t, &locked(None)).unwrap();
    let (_, height) = written(&job, &t)[2];
    assert!(
        (height - 49.0).abs() < 1e-9,
        "half the board's width less 1: {height}"
    );
}

#[test]
fn an_angle_typed_in_is_where_a_locks() {
    let job = job(Blank::Round);
    let mut t = groove(&job, 0.0, 10.0, 1.0);
    apply(&job, &mut t, &locked(Some(45.0))).unwrap();
    let want = 20.0 * 45f64.to_radians();
    for (round, _) in written(&job, &t) {
        assert!((round - want).abs() < 1e-9, "{round}");
    }
    assert_eq!(
        LockedFace::of(&t.params),
        Some(LockedFace { angle: Some(45.0) })
    );
}

#[test]
fn nothing_changes_on_a_flat_job_or_when_not_asked() {
    let rotary = job(Blank::Round);
    let before = groove(&rotary, 20.0, 40.0, 2.0);
    let mut t = before.clone();
    apply(&rotary, &mut t, &json!({})).unwrap();
    assert_eq!(t, before);
    let flat = Job::default();
    let mut t = before.clone();
    apply(&flat, &mut t, &locked(None)).unwrap();
    assert_eq!(t, before);
}

#[test]
fn a_toolpath_that_turns_a_itself_is_refused() {
    let job = job(Blank::Round);
    let mut t = groove(&job, 20.0, 40.0, 2.0);
    t.across = vec![0.0; t.moves.len()];
    let err = apply(&job, &mut t, &locked(None)).unwrap_err();
    assert!(err.to_string().contains("only for flat toolpaths"), "{err}");
}

#[test]
fn a_square_locked_part_way_round_cuts_down_from_its_corner() {
    let job = job(Blank::Square);
    let mut t = groove(&job, 0.0, 10.0, 1.0);
    apply(&job, &mut t, &locked(Some(45.0))).unwrap();
    // a 40 mm square's corner is 20√2 from the axis
    let (_, height) = written(&job, &t)[2];
    assert!(
        (height - (20.0 * 2f64.sqrt() - 1.0)).abs() < 1e-9,
        "1 down from the corner: {height}"
    );
}

#[test]
fn a_drill_bank_or_a_cut_past_the_axis_is_refused() {
    let job = job(Blank::Round);
    let mut t = groove(&job, 20.0, 40.0, 2.0);
    t.drill_bank = Some(crate::drill_bank::BankToolpath::default());
    let err = apply(&job, &mut t, &locked(None)).unwrap_err();
    assert!(err.to_string().contains("drill bank"), "{err}");
    // 25 down from a 20 mm radius goes 5 past the axis
    let mut t = groove(&job, 20.0, 40.0, 25.0);
    let err = apply(&job, &mut t, &locked(None)).unwrap_err();
    assert!(err.to_string().contains("past the turning axis"), "{err}");
}
