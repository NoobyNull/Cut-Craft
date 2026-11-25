use super::*;
use crate::document::Job;
use crate::toolpath::{Move, Tool};

fn job() -> Document {
    Document {
        job: Job {
            width: 200.0,
            height: 200.0,
            thickness: 18.0,
            ..Job::default()
        },
        // its zero looked at in Material Setup: no Zeroing warning in every test
        material_checked: true,
        ..Document::default()
    }
}

#[test]
fn a_zero_never_looked_at_is_said_where_to_set() {
    let mut doc = job();
    let path = slot("Profile", 150.0, 20.0, 180.0, 3.0);
    let zeroing = |doc: &Document| {
        check(doc, &[&path], &Limits::default())
            .into_iter()
            .filter(|f| f.check == Check::Zeroing)
            .collect::<Vec<_>>()
    };
    assert!(zeroing(&doc).is_empty(), "looked at in Material Setup");
    doc.material_checked = false;
    let found = zeroing(&doc);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].severity, Severity::Warning, "to read and tick");
    assert!(
            found[0]
                .message
                .contains("zero the bit's Z on top of the material, and X0 Y0 at the material's bottom left corner. "),
            "{}",
            found[0].message
        );
    doc.job.datum = crate::document::Datum::Centre;
    doc.job.offset = crate::geometry::Point::new(10.0, 5.0);
    assert!(
        zeroing(&doc)[0]
            .message
            .contains("in the middle of the material, moved 10 mm across and 5 mm up."),
    );
}

fn tool() -> Tool {
    Tool {
        name: "6 mm End Mill".into(),
        diameter: 6.0,
        feed_rate: 1500.0,
        plunge_rate: 500.0,
        spindle_speed: 18000.0,
        flute_length: 25.0,
        ..Tool::default()
    }
}

fn mv(kind: MoveKind, x: f64, y: f64, z: f64) -> Move {
    Move {
        kind,
        to: DVec3::new(x, y, z),
    }
}

/// Down at (x0, y), along to x1 at `depth`, and back up.
fn slot(name: &str, y: f64, x0: f64, x1: f64, depth: f64) -> Toolpath {
    Toolpath {
        name: name.into(),
        strategy: "profile_toolpath".into(),
        params: serde_json::Value::Null,
        tool: tool(),
        vectors: vec![],
        moves: vec![
            mv(MoveKind::Rapid, x0, y, 5.0),
            mv(MoveKind::Plunge, x0, y, -depth),
            mv(MoveKind::Cut, x1, y, -depth),
            mv(MoveKind::Rapid, x1, y, 5.0),
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

fn checks_of(found: &[Finding]) -> Vec<Check> {
    found.iter().map(|f| f.check).collect()
}

#[test]
fn a_rotary_job_asks_for_the_rotarys_centre_line_and_its_direction() {
    use crate::document::Along;
    use crate::rotary_axis::RotaryAxis;
    let doc = Document {
        job: Job::rotary(200.0, 50.0, Along::Y),
        ..Document::default()
    };
    let cut = slot("Wrap", 20.0, 10.0, 60.0, 2.0);
    let rotary_findings = |limits: &Limits, doc: &Document| -> Vec<String> {
        check(doc, &[&cut], limits)
            .into_iter()
            .filter(|f| f.check == Check::RotarySetup)
            .map(|f| f.message)
            .collect()
    };
    let mut limits = Limits {
        rotary: Some(RotaryAxis::default()),
        ..Limits::default()
    };
    let said = rotary_findings(&limits, &doc);
    assert_eq!(said.len(), 2, "{said:?}");
    assert!(said[0].contains("centre line is not set"), "{said:?}");
    assert!(
        said[1].contains("lies along Y") && said[1].contains("along X"),
        "{said:?}"
    );
    // a bit held to one side of the axis cuts in the wrong place until it is found
    let held = Toolpath {
        across: vec![4.0; cut.moves.len()],
        ..cut.clone()
    };
    let said: Vec<String> = check(&doc, &[&held], &limits)
        .into_iter()
        .filter(|f| f.check == Check::RotarySetup)
        .map(|f| f.message)
        .collect();
    assert!(said[0].contains("holds the bit to one side"), "{said:?}");
    limits.rotary = Some(RotaryAxis {
        centre_set: true,
        along: Along::Y,
        ..RotaryAxis::default()
    });
    assert_eq!(rotary_findings(&limits, &doc), Vec::<String>::new());
    assert!(
        !not_checked(&doc, &[&cut], &limits)
            .iter()
            .any(|(c, _)| *c == Check::RotarySetup)
    );
    let flat = job();
    assert_eq!(rotary_findings(&limits, &flat), Vec::<String>::new());
    assert!(
        not_checked(&flat, &[&cut], &limits)
            .iter()
            .any(|(c, why)| *c == Check::RotarySetup && why == "Not a rotary job")
    );
    limits.rotary = None;
    assert!(
        not_checked(&doc, &[&cut], &limits)
            .iter()
            .any(|(c, why)| *c == Check::RotarySetup && why.contains("4th axis"))
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one board, checked against each measure of room in turn"
)]
fn a_board_on_the_rotary_must_clear_the_spoilboard_the_bit_and_the_rapids_as_it_turns() {
    use crate::document::{Along, Blank, Rotary};
    use crate::rotary_axis::RotaryAxis;
    use crate::toolpath::Move;
    let mut job = Job::rotary(200.0, 20.0, Along::X);
    job.rotary = Some(Rotary {
        blank: Blank::Board { width: 150.0 },
        ..Rotary::round(20.0, Along::X)
    });
    let doc = Document {
        job,
        ..Document::default()
    };
    let swing = 75f64.hypot(10.0);
    // one pass, then A turned half a turn at 5 above the board's face: into its edge
    let mut flip = slot("Flip", 20.0, 10.0, 60.0, 2.0);
    flip.params = serde_json::json!({"registration_mark": true});
    let at = |kind, x: f64, round: f64, z: f64| Move {
        kind,
        to: DVec3::new(x, round, z),
    };
    flip.moves = vec![
        at(MoveKind::Rapid, 10.0, 0.0, 5.0),
        at(MoveKind::Plunge, 10.0, 0.0, -1.0),
        at(MoveKind::Cut, 50.0, 0.0, -1.0),
        at(MoveKind::Rapid, 50.0, 0.0, 5.0),
        at(MoveKind::Rapid, 50.0, 31.4, 5.0),
    ];
    let said = |axis: RotaryAxis, t: &Toolpath| -> Vec<String> {
        let limits = Limits {
            rotary: Some(RotaryAxis {
                centre_set: true,
                ..axis
            }),
            ..Limits::default()
        };
        check(&doc, &[t], &limits)
            .into_iter()
            .filter(|f| f.check == Check::RotarySetup)
            .map(|f| f.message)
            .collect()
    };
    // not measured: asked to measure; the low turn is lifted clear by the file itself
    let found = said(RotaryAxis::default(), &flip);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("Measure the axis height"), "{found:?}");
    // 60 above an 18 mm spoilboard: taking the spoilboard out makes room
    let low = RotaryAxis {
        axis_height: 60.0,
        spoilboard: 18.0,
        ..RotaryAxis::default()
    };
    let found = said(low, &flip);
    assert!(
        found[0].contains("hits it as it turns") && found[0].contains("Take the spoilboard out"),
        "{found:?}"
    );
    let out = RotaryAxis {
        spoilboard_off: true,
        ..low
    };
    assert!(!said(out, &flip).iter().any(|m| m.contains("hits it")));
    // too little: smaller material
    let lower = RotaryAxis {
        axis_height: 40.0,
        spoilboard: 18.0,
        spoilboard_off: true,
        ..RotaryAxis::default()
    };
    assert!(said(lower, &flip)[0].contains("Use smaller material"));
    // the bit lifted all the way up is below the edges: A can't turn the board under it
    let short = RotaryAxis {
        room_above: 70.0,
        ..out
    };
    assert!(
        said(short, &flip)
            .iter()
            .any(|m| m.contains("can't turn the material"))
    );
    // lifted clear before turning: only what the machine's room says
    let mut clear = flip.clone();
    let lift = swing - 10.0 + 2.0;
    clear.moves[3].to.z = lift;
    clear.moves[4].to.z = lift;
    let roomy = RotaryAxis {
        room_above: 120.0,
        ..out
    };
    assert_eq!(said(roomy, &clear), Vec::<String>::new());
    // left low, the file lifts it: room for that is all that matters
    assert_eq!(said(roomy, &flip), Vec::<String>::new());
    let tight = RotaryAxis {
        room_above: 78.0,
        ..out
    };
    assert!(
        said(tight, &flip)
            .iter()
            .any(|m| m.contains("the file lifts the bit") && m.contains("higher than Z")),
        "{:?}",
        said(tight, &flip)
    );
    // held off the axis, a rapid that only changes how far across turns A too
    let mut sideways = flip.clone();
    sideways.moves[4].to.y = 0.0;
    sideways.across = vec![0.0, 0.0, 0.0, 0.0, 3.0];
    assert!(
        said(tight, &sideways)
            .iter()
            .any(|m| m.contains("the file lifts the bit")),
        "{:?}",
        said(tight, &sideways)
    );
    // and lifted higher than Z can go
    let cramped = RotaryAxis {
        room_above: 77.0,
        ..out
    };
    assert!(
        said(cramped, &clear)
            .iter()
            .any(|m| m.contains("higher than Z"))
    );
    // a round blank swings no further than its top: nothing to measure or lift
    let round = Document {
        job: Job::rotary(200.0, 20.0, Along::X),
        ..Document::default()
    };
    let limits = Limits {
        rotary: Some(RotaryAxis {
            centre_set: true,
            ..RotaryAxis::default()
        }),
        ..Limits::default()
    };
    let found: Vec<_> = check(&round, &[&flip], &limits)
        .into_iter()
        .filter(|f| f.check == Check::RotarySetup)
        .collect();
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_free_rotary_wound_hundreds_of_turns_and_a_job_with_no_registration_mark_are_warned() {
    use crate::document::Along;
    use crate::rotary_axis::{RotaryAxis, Turning};
    use crate::toolpath::Move;
    let doc = Document {
        job: Job::rotary(200.0, 50.0, Along::X),
        ..Document::default()
    };
    let circumference = std::f64::consts::PI * 50.0;
    // 600 rings, each one whole turn round
    let mut rings = slot("Rings", 20.0, 10.0, 60.0, 2.0);
    rings.strategy = "rotary_machining_toolpath".into();
    rings.moves = (0..600)
        .flat_map(|k| {
            let x = f64::from(k) * 0.3;
            [
                Move {
                    kind: MoveKind::Rapid,
                    to: DVec3::new(x, 0.0, 5.0),
                },
                Move {
                    kind: MoveKind::Plunge,
                    to: DVec3::new(x, 0.0, -1.0),
                },
                Move {
                    kind: MoveKind::Cut,
                    to: DVec3::new(x, circumference, -1.0),
                },
                Move {
                    kind: MoveKind::Rapid,
                    to: DVec3::new(x, circumference, 5.0),
                },
            ]
        })
        .collect();
    let said = |limits: &Limits, t: &Toolpath| -> Vec<String> {
        check(&doc, &[t], limits)
            .into_iter()
            .filter(|f| f.check == Check::RotarySetup)
            .map(|f| f.message)
            .collect()
    };
    let free = Limits {
        rotary: Some(RotaryAxis {
            centre_set: true,
            ..RotaryAxis::default()
        }),
        ..Limits::default()
    };
    let found = said(&free, &rings);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found[0].contains("about 600 turns"), "{found:?}");
    assert!(found[1].contains("registration mark"), "{found:?}");
    // limited turns, and a mark: nothing to say
    let limited = Limits {
        rotary: Some(RotaryAxis {
            centre_set: true,
            turning: Turning::Unwind,
            ..RotaryAxis::default()
        }),
        ..Limits::default()
    };
    rings.params = serde_json::json!({"registration_mark": true});
    assert_eq!(said(&limited, &rings), Vec::<String>::new());
    // half a turn each way, but every ring goes a whole turn without lifting: asked about
    // before saving, naming the toolpath, but never refused
    let half = Limits {
        rotary: Some(RotaryAxis {
            centre_set: true,
            turning: Turning::Unwind,
            turns: 0.5,
            ..RotaryAxis::default()
        }),
        ..Limits::default()
    };
    let found = said(&half, &rings);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("\"Rings\" turns A 360°"), "{found:?}");
    assert!(found[0].contains("(180° each way)"), "{found:?}");
    assert!(
        check(&doc, &[&rings], &half)
            .iter()
            .any(|f| f.check == Check::RotarySetup && f.severity == Severity::MustAsk),
        "asked about on every save, never refused"
    );
}

#[test]
fn each_finding_names_the_check_that_found_it() {
    let limits = Limits {
        name: "Shapeoko".into(),
        bed: [100.0, 100.0],
        slots: vec![Some("3 mm Ball Nose".into())],
        ..Limits::default()
    };
    let crash = Toolpath {
        moves: vec![
            mv(MoveKind::Rapid, 10.0, 10.0, 5.0),
            mv(MoveKind::Rapid, 10.0, 10.0, -5.0),
        ],
        ..slot("Crash", 10.0, 10.0, 10.0, 0.0)
    };
    let found = check(
        &job(),
        &[&slot("Slot", 50.0, 20.0, 180.0, 3.0), &crash],
        &limits,
    );
    let checks = checks_of(&found);
    assert!(checks.contains(&Check::Bed), "{found:?}");
    assert!(checks.contains(&Check::ToolChanger), "{found:?}");
    assert!(checks.contains(&Check::Rapids), "{found:?}");
}

#[test]
fn cutting_into_a_keep_out_zone_must_be_fixed() {
    let mut doc = job();
    let clamp = crate::testing::square(90.0, 40.0, 20.0);
    doc.keep_out = vec![crate::keep_out::KeepOutZone::new("Clamp 1", clamp)];
    let found = check(
        &doc,
        &[&slot("Slot", 50.0, 20.0, 180.0, 3.0)],
        &Limits::default(),
    );
    let zone: Vec<&Finding> = found.iter().filter(|f| f.check == Check::KeepOut).collect();
    assert_eq!(zone.len(), 1, "{found:?}");
    assert_eq!(zone[0].severity, Severity::Error);
    assert!(
        zone[0].message.contains("\"Clamp 1\""),
        "{}",
        zone[0].message
    );
    // clear of the zone: nothing to say
    let found = check(
        &doc,
        &[&slot("Slot", 150.0, 20.0, 180.0, 3.0)],
        &Limits::default(),
    );
    assert!(!checks_of(&found).contains(&Check::KeepOut), "{found:?}");
}

#[test]
fn two_bits_with_one_tool_number_are_called_out_but_hand_changed_ones_are_not() {
    let mut first = slot("Pocket", 50.0, 20.0, 180.0, 3.0);
    let mut second = slot("V-Carve", 100.0, 20.0, 180.0, 1.0);
    second.tool.name = "60° V-Bit".into();
    first.tool.number = 3;
    second.tool.number = 3;
    let found = check(&job(), &[&first, &second], &Limits::default());
    let numbers: Vec<&Finding> = found
        .iter()
        .filter(|f| f.check == Check::ToolNumbers)
        .collect();
    assert_eq!(numbers.len(), 1, "{found:?}");
    assert!(
        numbers[0]
            .message
            .starts_with("Bit number 3 is given to both 6 mm End Mill and 60° V-Bit"),
        "{}",
        numbers[0].message
    );
    // the same bit twice, or bits changed by hand (0), share a number safely
    let mut again = slot("Profile", 150.0, 20.0, 180.0, 3.0);
    again.tool.number = 3;
    second.tool.number = 0;
    first.tool.number = 3;
    let mut hand = second.clone();
    hand.tool.name = "1/4\" Ball Nose".into();
    let found = check(
        &job(),
        &[&first, &again, &second, &hand],
        &Limits::default(),
    );
    assert!(
        !checks_of(&found).contains(&Check::ToolNumbers),
        "{found:?}"
    );
}

fn with_library(tools: Vec<Tool>) -> Limits {
    Limits {
        library: Some(tools.into_iter().map(|t| (t.name.clone(), t)).collect()),
        ..Limits::default()
    }
}

#[test]
fn a_generic_bit_or_one_not_in_the_tool_database_is_called_out() {
    let slot = slot("Slot", 50.0, 20.0, 180.0, 3.0);
    let found = check(&job(), &[&slot], &with_library(vec![tool()]));
    let generic: Vec<&str> = found
        .iter()
        .filter(|f| f.check == Check::GenericTool)
        .map(|f| f.message.as_str())
        .collect();
    assert_eq!(generic.len(), 1, "{found:?}");
    assert!(generic[0].contains("is a generic bit"), "{}", generic[0]);
    let found = check(&job(), &[&slot], &with_library(vec![]));
    assert!(
        found
            .iter()
            .any(|f| f.check == Check::GenericTool && f.message.contains("not in the Bit Library")),
        "{found:?}"
    );
    // not compared at all: nothing said, and the list says why
    let found = check(&job(), &[&slot], &Limits::default());
    assert!(
        !checks_of(&found).contains(&Check::GenericTool),
        "{found:?}"
    );
    let skipped = not_checked(&job(), &[&slot], &Limits::default());
    assert!(skipped.iter().any(|(c, _)| *c == Check::GenericTool));
}

#[test]
fn feeds_and_speeds_changed_from_the_makers_figures_are_called_out() {
    let makers = Tool {
        manufacturer: "Amana".into(),
        feed_rate: 1000.0,
        plunge_rate: 480.0,
        spindle_speed: 18000.0,
        ..tool()
    };
    // feed 1500 is 50% above 1000; plunge 500 within a tenth of 480; spindle the same
    let found = check(
        &job(),
        &[&slot("Slot", 50.0, 20.0, 180.0, 3.0)],
        &with_library(vec![makers.clone()]),
    );
    let changed: Vec<&str> = found
        .iter()
        .filter(|f| f.check == Check::MakersFigures)
        .map(|f| f.message.as_str())
        .collect();
    assert_eq!(changed.len(), 1, "{found:?}");
    assert!(
        changed[0].contains("feed rate of 1500 mm/min is 50% above the maker's 1000 mm/min"),
        "{}",
        changed[0]
    );
    assert!(!changed[0].contains("plunge"), "{}", changed[0]);
    assert!(changed[0].contains("Amana's figures"), "{}", changed[0]);
    assert!(
        !checks_of(&found).contains(&Check::GenericTool),
        "{found:?}"
    );
    // as the maker gives: nothing to say
    let same = Tool {
        feed_rate: 1500.0,
        ..makers
    };
    let found = check(
        &job(),
        &[&slot("Slot", 50.0, 20.0, 180.0, 3.0)],
        &with_library(vec![same]),
    );
    assert!(
        !checks_of(&found).contains(&Check::MakersFigures),
        "{found:?}"
    );
}

#[test]
fn checks_that_cannot_be_made_say_why() {
    let slot = slot("Slot", 50.0, 20.0, 180.0, 3.0);
    let skipped = not_checked(&job(), &[&slot], &Limits::default());
    let checks: Vec<Check> = skipped.iter().map(|(c, _)| *c).collect();
    for check in [
        Check::Bed,
        Check::ZTravel,
        Check::MachineSpeeds,
        Check::SpindleRange,
        Check::ToolChanger,
        Check::KeepOut,
        Check::ToolNumbers,
    ] {
        assert!(checks.contains(&check), "{check:?} in {skipped:?}");
    }
    assert!(
        skipped.iter().any(|(c, why)| *c == Check::Bed
            && why == "the machine has no bed size in Machine Configuration"),
        "{skipped:?}"
    );
    let limits = Limits {
        name: "Shapeoko".into(),
        bed: [800.0, 800.0],
        z_travel: 100.0,
        max_rate: [5000.0, 5000.0, 1000.0],
        spindle: [10000.0, 24000.0],
        slots: vec![Some("6 mm End Mill".into())],
        left: None,
        library: Some(std::collections::BTreeMap::new()),
        holding: crate::collision::Holding::default(),
        rotary: None,
    };
    let skipped = not_checked(&job(), &[&slot], &limits);
    assert!(
        skipped
            .iter()
            .all(|(c, _)| matches!(c, Check::KeepOut | Check::ToolNumbers | Check::RotarySetup)),
        "{skipped:?}"
    );
}

#[test]
fn the_collet_hitting_the_material_asks_first() {
    let mut deep = slot("Deep slot", 100.0, 50.0, 150.0, 30.0);
    deep.tool.flute_length = 32.0;
    deep.tool.stickout = 25.0;
    let doc = Document {
        job: Job {
            thickness: 40.0,
            ..job().job
        },
        ..Document::default()
    };
    let found = check(&doc, &[&deep], &Limits::default());
    let warned = messages(&found, Severity::MustAsk);
    assert!(
        warned
            .iter()
            .any(|m| m.starts_with("Deep slot: the collet nut would hit the material")),
        "{warned:?}"
    );
    assert!(
        found
            .iter()
            .filter(|f| f.severity == Severity::MustAsk)
            .all(|f| f.check == Check::Collisions),
        "{found:?}"
    );
}

fn messages(found: &[Finding], severity: Severity) -> Vec<&str> {
    found
        .iter()
        .filter(|f| f.severity == severity)
        .map(|f| f.message.as_str())
        .collect()
}

/// A 3D finish's raster as a 1/64" tapered ball nose cuts it: rows `stepover` apart
/// in steps of a fiftieth of a millimetre over a gently waving surface.
#[expect(clippy::cast_precision_loss, reason = "a row number")]
fn fine_finish(rows: usize, stepover: f64) -> Toolpath {
    let mut finish = slot("3D Finishing", 0.0, 0.0, 0.0, 0.0);
    finish.strategy = "3d_finishing_toolpath".into();
    finish.tool = Tool {
        name: "TBN 1/64\"".into(),
        shape: ToolShape::BallNose,
        diameter: 0.396_875,
        taper: 6.3,
        shank_diameter: 6.35,
        flute_length: 25.0,
        ..tool()
    };
    let mut moves = vec![mv(MoveKind::Rapid, 0.0, 0.0, 5.0)];
    for row in 0..rows {
        let y = 10.0 + stepover * row as f64;
        moves.push(mv(MoveKind::Rapid, 0.0, y, 5.0));
        moves.push(mv(MoveKind::Plunge, 0.0, y, -1.0));
        for step in 1..=10_000 {
            let x = 0.02 * f64::from(step);
            moves.push(mv(MoveKind::Cut, x, y, -1.0 - 0.5 * (x / 5.0).sin()));
        }
        moves.push(mv(MoveKind::Rapid, 200.0, y, 5.0));
    }
    finish.moves = moves;
    finish
}

#[test]
fn a_3d_finish_of_millions_of_tiny_steps_is_checked_in_moments() {
    // Matthew's lock-up (v0.6.10): Analyse on a 3D finish of 12.7 million moves ran the
    // simulation a move at a time on one core, about a minute per million moves. A
    // million here took over a minute before; simplified to the simulation's cells and
    // measured a batch at a time across the cores, a few seconds at most.
    let finish = fine_finish(100, 0.16);
    assert!(finish.moves.len() > 1_000_000);
    let started = std::time::Instant::now();
    let found = check(&job(), &[&finish], &Limits::default());
    let took = started.elapsed();
    assert!(took < std::time::Duration::from_secs(20), "took {took:?}");
    // the first rows cut uncut material: no crash and no air
    assert!(messages(&found, Severity::Error).is_empty(), "{found:?}");
    assert!(
        !messages(&found, Severity::Warning)
            .iter()
            .any(|m| m.contains("cuts air")),
        "{found:?}"
    );
}

#[test]
fn a_fine_finish_run_twice_still_cuts_air_the_second_time() {
    // simplified and batched, the second pass over what the first cut is still air
    let first = fine_finish(20, 0.16);
    let mut again = first.clone();
    again.name = "Again".into();
    let found = check(&job(), &[&first, &again], &Limits::default());
    let warnings = messages(&found, Severity::Warning);
    assert!(
        warnings.iter().any(|m| m.starts_with("Again cuts air")),
        "{warnings:?}"
    );
    assert!(
        !warnings
            .iter()
            .any(|m| m.starts_with("3D Finishing cuts air")),
        "{warnings:?}"
    );
}

#[test]
fn a_drill_cutting_sideways_is_warned_about_and_drilling_with_it_is_not() {
    let mut sideways = slot("Profile", 50.0, 20.0, 80.0, 3.0);
    sideways.tool.drill_only = true;
    sideways.tool.name = "Drill 6mm".into();
    let mut drilling = sideways.clone();
    drilling.strategy = "drilling_toolpath".into();
    drilling.name = "Holes".into();
    let warned = |t: &Toolpath| {
        check(&job(), &[t], &Limits::default())
            .iter()
            .any(|f| f.severity == Severity::Warning && f.message.contains("is a drill"))
    };
    assert!(warned(&sideways));
    assert!(!warned(&drilling));
    assert!(!warned(&slot("Profile", 50.0, 20.0, 80.0, 3.0)));
}

#[test]
fn a_good_toolpath_has_nothing_to_say() {
    let found = check(
        &job(),
        &[&slot("Slot", 50.0, 20.0, 180.0, 3.0)],
        &Limits::default(),
    );
    assert_eq!(found, vec![]);
}

#[test]
fn a_rapid_through_the_material_is_a_crash() {
    let mut path = slot("Slot", 50.0, 20.0, 180.0, 3.0);
    // straight across uncut material at full speed, 3 mm down
    path.moves.insert(1, mv(MoveKind::Rapid, 20.0, 100.0, -3.0));
    path.moves.insert(2, mv(MoveKind::Rapid, 20.0, 50.0, -3.0));
    let found = check(&job(), &[&path], &Limits::default());
    let errors = messages(&found, Severity::Error);
    assert_eq!(errors.len(), 1, "{found:?}");
    assert!(
        errors[0].contains("Slot would crash") && errors[0].contains("clearance (Z1)"),
        "{errors:?}"
    );
}

#[test]
fn a_rapid_inside_what_was_cut_is_no_crash() {
    let mut path = slot("Slot", 50.0, 20.0, 180.0, 3.0);
    // back along the slot just cut, at its depth
    path.moves.insert(3, mv(MoveKind::Rapid, 20.0, 50.0, -3.0));
    let found = check(&job(), &[&path], &Limits::default());
    assert_eq!(messages(&found, Severity::Error), Vec::<&str>::new());
}

#[test]
fn cutting_again_where_an_earlier_toolpath_cut_is_air_cutting() {
    let first = slot("Roughing", 50.0, 20.0, 180.0, 3.0);
    let again = slot("Again", 50.0, 20.0, 180.0, 3.0);
    let deeper = slot("Deeper", 50.0, 20.0, 180.0, 6.0);
    let found = check(&job(), &[&first, &again, &deeper], &Limits::default());
    let warnings = messages(&found, Severity::Warning);
    assert_eq!(warnings.len(), 1, "{found:?}");
    assert!(
        warnings[0].starts_with("Again cuts air for 100%"),
        "{warnings:?}"
    );
}

#[test]
fn fine_v_carving_on_a_big_job_is_not_air() {
    // a 3/4 in V-bit carving grooves 1 mm deep, 2 mm wide, into fresh stock on a
    // 1.2 m job: cells sized by the bit's width (9.5 mm) missed the thin grooves and
    // called 97% of a fresh V-Carve air
    let mut doc = job();
    doc.job.width = 1200.0;
    doc.job.height = 1200.0;
    let mut carve = slot("V-Carve", 50.0, 20.0, 180.0, 1.0);
    carve.tool = Tool {
        name: "3/4 in V-Bit".into(),
        shape: crate::toolpath::ToolShape::VBit,
        diameter: 19.05,
        angle: 90.0,
        ..tool()
    };
    carve.moves.clear();
    for k in 0..20 {
        let y = 50.0 + 3.1 * f64::from(k);
        carve.moves.extend([
            mv(MoveKind::Rapid, 20.0, y, 5.0),
            mv(MoveKind::Plunge, 20.0, y, -1.0),
            mv(MoveKind::Cut, 180.0, y, -1.0),
            mv(MoveKind::Rapid, 180.0, y, 5.0),
        ]);
    }
    let found = check(&doc, &[&carve], &Limits::default());
    assert!(
        !messages(&found, Severity::Warning)
            .iter()
            .any(|m| m.contains("cuts air")),
        "{found:?}"
    );
}

#[test]
fn toolpaths_with_the_same_name_are_told_apart() {
    // calculating twice leaves two toolpaths of one name: which one cuts air?
    let first = slot("V-Carve", 50.0, 20.0, 180.0, 3.0);
    let again = slot("V-Carve", 50.0, 20.0, 180.0, 3.0);
    let found = check(&job(), &[&first, &again], &Limits::default());
    let warnings = messages(&found, Severity::Warning);
    assert_eq!(warnings.len(), 1, "{found:?}");
    assert!(
        warnings[0].starts_with("V-Carve (2nd of 2) cuts air for 100%"),
        "{warnings:?}"
    );
}

#[test]
fn cutting_above_the_material_is_air_cutting() {
    let above = slot("Above", 50.0, 20.0, 180.0, -1.0);
    let found = check(&job(), &[&above], &Limits::default());
    assert!(
        messages(&found, Severity::Warning)
            .iter()
            .any(|m| m.contains("cuts air")),
        "{found:?}"
    );
}

#[test]
fn toolpaths_that_cannot_cut_are_errors() {
    let mut empty = slot("Empty", 50.0, 20.0, 180.0, 3.0);
    empty.moves.clear();
    let mut still = slot("Still", 80.0, 20.0, 180.0, 3.0);
    still.tool.feed_rate = 0.0;
    still.tool.plunge_rate = 0.0;
    let found = check(&job(), &[&empty, &still], &Limits::default());
    let errors = messages(&found, Severity::Error);
    assert_eq!(errors.len(), 3, "{found:?}");
    assert!(errors[0].contains("Empty has no moves"), "{errors:?}");
    assert!(errors[1].contains("no feed rate"), "{errors:?}");
    assert!(errors[2].contains("no plunge rate"), "{errors:?}");
}

#[test]
fn errors_come_before_warnings() {
    let mut stale = slot("Stale", 50.0, 20.0, 180.0, 3.0);
    stale.stale = true;
    let mut empty = slot("Empty", 80.0, 20.0, 180.0, 3.0);
    empty.moves.clear();
    let found = check(&job(), &[&stale, &empty], &Limits::default());
    assert_eq!(
        found.iter().map(|f| f.severity).collect::<Vec<_>>(),
        vec![Severity::Error, Severity::Warning],
        "{found:?}"
    );
    assert!(found[1].message.contains("out of date"));
}

#[test]
fn the_machine_limits_are_checked() {
    let limits = Limits {
        name: "Shapeoko".into(),
        bed: [100.0, 100.0],
        z_travel: 6.0,
        max_rate: [1000.0, 1200.0, 300.0],
        spindle: [10000.0, 16000.0],
        slots: vec![Some("3 mm Ball Nose".into()), None],
        left: None,
        library: None,
        holding: crate::collision::Holding::default(),
        rotary: None,
    };
    let found = check(&job(), &[&slot("Slot", 50.0, 20.0, 180.0, 3.0)], &limits);
    let errors = messages(&found, Severity::Error);
    assert_eq!(errors.len(), 2, "{found:?}");
    assert!(
        errors
            .iter()
            .any(|m| m.contains("more than the bed of Shapeoko"))
    );
    assert!(errors.iter().any(|m| m.contains("the Z axis of Shapeoko")));
    let warnings = messages(&found, Severity::Warning);
    assert_eq!(warnings.len(), 4, "{found:?}");
    assert!(
        warnings
            .iter()
            .any(|m| m.contains("no faster than 16000 rpm"))
    );
    assert!(
        warnings
            .iter()
            .any(|m| m.contains("faster than Shapeoko moves (1000"))
    );
    assert!(warnings.iter().any(|m| m.contains("moves in Z (300")));
    assert!(
        warnings
            .iter()
            .any(|m| m.contains("not loaded in the tool changer"))
    );
    // a tool deleted from the Bit Library cannot be loaded: the advice says what to do
    assert!(
        warnings
            .iter()
            .any(|m| m.contains("pick a tool again with the toolpath's Select"))
    );
}

#[test]
fn an_inch_job_hears_about_its_machine_in_inches() {
    let limits = Limits {
        name: "Shapeoko".into(),
        bed: [101.6, 101.6],
        z_travel: 6.0,
        max_rate: [508.0, 635.0, 127.0],
        ..Limits::default()
    };
    let mut doc = job();
    doc.job.units = crate::document::Units::Inch;
    let found = check(&doc, &[&slot("Slot", 50.0, 20.0, 180.0, 3.0)], &limits);
    let all: Vec<&str> = found.iter().map(|f| f.message.as_str()).collect();
    let says = |text: &str| all.iter().any(|m| m.contains(text));
    assert!(says("(4.00 in × 4.00 in)"), "{all:?}");
    assert!(says("faster than Shapeoko moves (20.0 in/min)"), "{all:?}");
    assert!(says("moves in Z (5.0 in/min)"), "{all:?}");
    assert!(says("the 0.24 in the Z axis"), "{all:?}");
    // only the bit's own name says mm
    assert!(
        !all.iter()
            .any(|m| m.replace("6 mm End Mill", "").contains(" mm")),
        "{all:?}"
    );
    let deep = slot("Cut out", 50.0, 20.0, 180.0, 21.0);
    let found = check(&doc, &[&deep], &Limits::default());
    assert!(
        found
            .iter()
            .any(|f| f.message.contains("0.118 in below the material")),
        "{found:?}"
    );
}

#[test]
fn a_tiled_job_fits_the_bed_when_each_tile_does() {
    let limits = Limits {
        name: "Desktop".into(),
        bed: [100.0, 100.0],
        ..Limits::default()
    };
    let wide = slot("Slot", 50.0, 20.0, 180.0, 3.0);
    let mut doc = job();
    assert_eq!(check(&doc, &[&wide], &limits).len(), 1, "too wide untiled");
    doc.tiling = Some(crate::tiling::Tiling {
        width: 80.0,
        height: 80.0,
        overlap: 0.0,
        ..crate::tiling::Tiling::default()
    });
    assert_eq!(check(&doc, &[&wide], &limits), vec![]);
    // tiles bigger than the bed still do not fit
    doc.tiling = Some(crate::tiling::Tiling {
        width: 150.0,
        height: 150.0,
        overlap: 0.0,
        ..crate::tiling::Tiling::default()
    });
    assert_eq!(check(&doc, &[&wide], &limits).len(), 1);
}

#[test]
fn a_cut_well_into_the_spoilboard_is_flagged() {
    let through = slot("Cut out", 50.0, 20.0, 180.0, 18.5);
    let found = check(&job(), &[&through], &Limits::default());
    assert_eq!(found, vec![], "half a millimetre through is on purpose");
    let deep = slot("Cut out", 50.0, 20.0, 180.0, 21.0);
    let found = check(&job(), &[&deep], &Limits::default());
    assert!(
        messages(&found, Severity::Warning)
            .iter()
            .any(|m| m.contains("3.0 mm below the material")),
        "{found:?}"
    );
}

#[test]
fn flattening_the_spoilboard_is_a_reminder_not_a_cut_too_deep() {
    // zeroed on the bed, a skim off the spoilboard is under the 20 mm material
    let mut skim = slot("Surfacing", 50.0, 20.0, 180.0, 20.25);
    skim.strategy = "surfacing_toolpath".into();
    skim.params = serde_json::json!({"area": "spoilboard"});
    let found = check(&job(), &[&skim], &Limits::default());
    let warnings = messages(&found, Severity::Warning);
    assert!(
        !warnings.iter().any(|m| m.contains("below the material")),
        "{found:?}"
    );
    assert!(
        warnings
            .iter()
            .any(|m| m.contains("Surfacing flattens the spoilboard")),
        "{found:?}"
    );
}

#[test]
fn a_fine_stepover_is_not_air_cutting() {
    // 0.3 mm between rows is finer than the simulation's cells
    let mut raster = slot("Finishing", 20.0, 20.0, 180.0, 1.0);
    raster.moves.truncate(2);
    for row in 0..100 {
        let (y, x) = (
            20.0 + f64::from(row) * 0.3,
            if row % 2 == 0 { 180.0 } else { 20.0 },
        );
        raster.moves.push(mv(
            MoveKind::Cut,
            raster.moves.last().unwrap().to.x,
            y,
            -1.0,
        ));
        raster.moves.push(mv(MoveKind::Cut, x, y, -1.0));
    }
    let found = check(&job(), &[&raster], &Limits::default());
    assert_eq!(found, vec![]);
}

#[test]
fn what_a_toolpath_said_it_could_not_cut_when_made_is_said_again_before_cutting() {
    let mut rough = slot("Rotary", 150.0, 20.0, 180.0, 3.0);
    rough.params = serde_json::json!({
        crate::toolpath::MADE_NOTES: ["Some of the model is left rough between 40 mm and 260 mm."]
    });
    let found: Vec<_> = check(&job(), &[&rough], &Limits::default())
        .into_iter()
        .filter(|f| f.check == Check::MissedSpots)
        .collect();
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].severity, Severity::Warning, "to read and tick");
    assert_eq!(
        found[0].message,
        "Rotary: Some of the model is left rough between 40 mm and 260 mm."
    );
}

#[test]
fn cutting_after_the_centre_stock_is_cut_away_is_warned() {
    use crate::document::Along;
    let doc = Document {
        job: Job::rotary(200.0, 40.0, Along::X),
        material_checked: true,
        ..Document::default()
    };
    let mut away = slot("Free the part", 10.0, 20.0, 180.0, 3.0);
    away.strategy = "rotary_machining_toolpath".into();
    away.params = serde_json::json!({ "cut_support": true });
    let after = slot("Details", 10.0, 20.0, 180.0, 3.0);
    let said = |toolpaths: &[&Toolpath]| -> Vec<String> {
        check(&doc, toolpaths, &Limits::default())
            .into_iter()
            .filter(|f| f.message.contains("Cut Away the Support"))
            .map(|f| f.message)
            .collect()
    };
    let found = said(&[&away, &after]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("Details"), "{}", found[0]);
    assert!(said(&[&after, &away]).is_empty(), "last: nothing to say");
}
