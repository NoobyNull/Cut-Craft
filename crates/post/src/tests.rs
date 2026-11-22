#[test]
fn every_old_rotary_name_is_renamed_to_a_post_cut_craft_has() {
    use super::*;
    let names: Vec<String> = BUILT_IN
        .iter()
        .map(|(_, text)| Post::parse(text).unwrap().name)
        .collect();
    let mut renamed_count = 0;
    for controller in ["GRBL", "Mach3", "FluidNC ATC"] {
        for wrapped in ["Y", "X"] {
            for g93 in ["", " G93"] {
                for units in ["mm", "inch"] {
                    let ext = if controller == "Mach3" { "tap" } else { "nc" };
                    let old = format!(
                        "Cut Craft {controller} Wrap {wrapped} to A{g93} ({units}) (*.{ext})"
                    );
                    let new = renamed(&old).unwrap();
                    assert!(names.contains(&new), "{old} -> {new}");
                    renamed_count += 1;
                }
            }
        }
    }
    assert_eq!(renamed_count, 24);
    let new = renamed("Cut Craft FluidNC ATC Wrap A G93 (mm) (*.nc)").unwrap();
    assert!(names.contains(&new), "{new}");
    assert_eq!(
        renamed("Cut Craft GRBL Wrap Y to A G93 (mm) (*.nc)").as_deref(),
        Some("Cut Craft grblHAL Rotary A, cylinder along X, G93 (mm) (*.nc)")
    );
    for kept in [
        "Cut Craft GRBL Arcs (mm) (*.nc)",
        "My Wrap Y to A (mm)",
        "Cut Craft GRBL Wrap Y to Anything",
        "Cut Craft GRBL Wrap Y to A (mm) (*.nc) - Custom",
        "Cut Craft GRBL Wrap Y to A (mm) (*.tap)",
    ] {
        assert_eq!(renamed(kept), None, "{kept}");
    }
}

use super::*;
use cut_craft_core::Change;
use cut_craft_core::glam::DVec3;
use cut_craft_core::toolpath::{Move, MoveKind, Tool, Toolpath};

/// A small post written for these tests, in the documented format.
const BASIC: &str = r#"
+ test post
POST_NAME = "Test G-code (mm) (*.nc)"
FILE_EXTENSION = "nc"
UNITS = "MM"
LINE_ENDING = "[13][10]"

VAR SPINDLE_SPEED = [S|A|S|1.0]
VAR FEED_RATE = [F|C|F|1.1]
VAR X_POSITION = [X|A|X|1.3]
VAR Y_POSITION = [Y|A|Y|1.3]
VAR Z_POSITION = [Z|A|Z|1.3]
VAR SAFE_Z_HEIGHT = [SAFEZ|A||1.3]

begin HEADER
"([TOOLPATH_NAME] with [TOOLNAME])"
"G21 G90"
"G0 Z[SAFEZ]"
"[S] M3"

begin RAPID_MOVE
"G0 [X] [Y] [Z]"

begin FIRST_FEED_MOVE
"G1 [X] [Y] [Z] [F]"

begin FEED_MOVE
"G1 [X] [Y] [Z] [F]"

begin TOOLCHANGE
"M5"
"(change to [TOOLNAME])"
"M0"
"[S] M3"

begin FOOTER
"G0 Z[SAFEZ]"
"M5"
"M30"
"#;

fn mv(kind: MoveKind, x: f64, y: f64, z: f64) -> Move {
    Move {
        kind,
        to: DVec3::new(x, y, z),
    }
}

#[test]
fn the_tools_used_are_listed_once_each_with_their_number() {
    let (first, v) = (
        toolpath("Profile", "6 mm End Mill"),
        toolpath("Carve", "V-Bit"),
    );
    let toolpaths = [&first, &v, &first];
    let names = |numbers: [u32; 2]| {
        vec![
            ("6 mm End Mill".to_owned(), numbers[0]),
            ("V-Bit".to_owned(), numbers[1]),
        ]
    };
    assert_eq!(tools_and_numbers(&toolpaths, &[]), names([1, 2]));
    let slots = [None, None, Some("V-Bit".to_owned())];
    assert_eq!(tools_and_numbers(&toolpaths, &slots), names([0, 3]));
}

fn toolpath(name: &str, tool: &str) -> Toolpath {
    Toolpath {
        name: name.into(),
        strategy: "profile_toolpath".into(),
        params: serde_json::json!({}),
        tool: Tool {
            name: tool.into(),
            ..Tool::default()
        },
        power: vec![],
        stale: false,
        group: None,
        vectors: vec![],
        moves: vec![
            mv(MoveKind::Rapid, 0.0, 0.0, 5.0),
            mv(MoveKind::Plunge, 0.0, 0.0, -3.0),
            mv(MoveKind::Cut, 25.4, 0.0, -3.0),
            mv(MoveKind::Cut, 25.4, 25.4, -3.0),
            mv(MoveKind::Rapid, 25.4, 25.4, 5.0),
        ],
        visible: true,
        drill_bank: None,
        fallbacks: vec![],
        across: vec![],
    }
}

fn render(post: &str, toolpaths: Vec<Toolpath>) -> Vec<String> {
    render_for(post, cut_craft_core::document::Job::default(), toolpaths)
}

fn render_for(
    post: &str,
    job: cut_craft_core::document::Job,
    toolpaths: Vec<Toolpath>,
) -> Vec<String> {
    let post = Post::parse(post).unwrap();
    assert_eq!(post.line_ending, "\r\n", "LINE_ENDING is honoured");
    render_with(&post, job, toolpaths)
}

fn render_with(
    post: &Post,
    job: cut_craft_core::document::Job,
    toolpaths: Vec<Toolpath>,
) -> Vec<String> {
    let mut doc = Document {
        job,
        ..Document::default()
    };
    let mut c = Change::builder(&doc);
    for t in toolpaths {
        c.add_toolpath(t);
    }
    doc.apply(&c.finish()).unwrap();
    let made = chrono::NaiveDate::from_ymd_opt(2026, 10, 24)
        .and_then(|d| d.and_hms_opt(9, 37, 0))
        .unwrap();
    let file = File {
        path: std::path::Path::new("/jobs/sign/front.nc"),
        job: Some(std::path::Path::new("/jobs/sign.ccjob")),
        made,
        slots: &[],
        notes: &[],
    };
    let text = post.render(&doc, &file).unwrap();
    text.lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect()
}

#[test]
fn laser_toolpaths_turn_the_jet_on_after_moving_and_off_before_rapids() {
    let post = BASIC.replace(
        "begin HEADER",
        "LASER_SUPPORT = \"YES\"\nVAR POWER = [P|C|S|1.0|10.0]\n\
             begin JET_TOOL_ON\n\"M4 [P]\"\nbegin JET_TOOL_POWER\n\"[P]\"\n\
             begin JET_TOOL_OFF\n\"M5\"\nbegin HEADER",
    );
    let mut laser = toolpath("Burn", "Laser");
    laser.power = vec![0.0, 50.0, 50.0, 80.0, 0.0];
    let lines = render(&post, vec![laser]);
    let at = |text: &str| {
        lines
            .iter()
            .position(|l| l == text)
            .unwrap_or_else(|| panic!("{text} in {lines:?}"))
    };
    let on = at("M4 S500");
    assert!(on > at("G0 X0.000 Y0.000 Z5.000"), "on once it is in place");
    assert!(at("S800") > on, "a new power prints on its own");
    assert!(lines[at("S800") + 1].starts_with("G1 X25.400 Y25.400"));
    let off = lines.iter().rposition(|l| l == "M5").unwrap();
    assert!(
        lines.iter().position(|l| l == "M5").unwrap() < at("G0 X25.400 Y25.400 Z5.000"),
        "off before the rapid"
    );
    assert!(off > on);
}

#[test]
fn a_post_without_laser_support_refuses_laser_toolpaths() {
    let mut laser = toolpath("Burn", "Laser");
    laser.power = vec![0.0, 50.0, 50.0, 50.0, 0.0];
    let mut doc = Document::default();
    let mut c = Change::builder(&doc);
    c.add_toolpath(laser);
    let change = c.finish();
    doc.apply(&change).unwrap();
    let file = File {
        path: Path::new("x.nc"),
        job: None,
        made: chrono::NaiveDateTime::default(),
        slots: &[],
        notes: &[],
    };
    let why = Post::parse(BASIC).unwrap().render(&doc, &file).unwrap_err();
    assert!(why.contains("LASER_SUPPORT"), "{why}");
}

#[test]
fn z_zero_on_the_machine_bed_raises_every_z_by_the_thickness() {
    use cut_craft_core::document::{Job, ZZero};
    let job = Job {
        z_zero: ZZero::MachineBed,
        thickness: 20.0,
        ..Job::default()
    };
    let surface = render(BASIC, vec![toolpath("P", "T")]);
    let bed = render_for(BASIC, job, vec![toolpath("P", "T")]);
    assert!(surface.contains(&"G0 Z5.000".to_owned()), "{surface:?}");
    assert!(bed.contains(&"G0 Z25.000".to_owned()), "{bed:?}");
}

#[test]
fn reads_the_global_settings() {
    let post = Post::parse(BASIC).unwrap();
    assert_eq!(
        (post.name.as_str(), post.extension.as_str(), post.inches),
        ("Test G-code (mm) (*.nc)", "nc", false)
    );
}

#[test]
fn renders_header_moves_and_footer() {
    let lines = render(BASIC, vec![toolpath("Profile 1", "6 mm End Mill")]);
    assert_eq!(
        lines,
        [
            "(Profile 1 with 6 mm End Mill)",
            "G21 G90",
            "G0 Z5.000",
            "S18000 M3",
            "G0 X0.000 Y0.000 Z5.000",
            "G1 X0.000 Y0.000 Z-3.000 F300.0",
            "G1 X25.400 Y0.000 Z-3.000 F1000.0",
            "G1 X25.400 Y25.400 Z-3.000",
            "G0 X25.400 Y25.400 Z5.000",
            "G0 Z5.000",
            "M5",
            "M30",
        ]
    );
}

#[test]
fn the_same_tool_at_a_new_spindle_speed_sets_the_new_speed() {
    let mut fast = toolpath("A", "6 mm");
    fast.tool.spindle_speed = 18000.0;
    let mut slow = toolpath("B", "6 mm");
    slow.tool.spindle_speed = 12000.0;
    let lines = render(BASIC, vec![fast, slow]);
    let header = lines.iter().position(|l| l == "S18000 M3");
    let second = lines.iter().position(|l| l == "S12000 M3");
    assert!(header.is_some() && second > header, "{lines:#?}");
    assert!(
        !lines.contains(&"M0".to_owned()),
        "no tool change: {lines:#?}"
    );
    // the same speed again writes nothing new
    let lines = render(BASIC, vec![toolpath("A", "6 mm"), toolpath("B", "6 mm")]);
    assert_eq!(lines.iter().filter(|l| l.ends_with("M3")).count(), 1);
}

#[test]
fn a_new_tool_uses_the_toolchange_block() {
    let lines = render(BASIC, vec![toolpath("A", "6 mm"), toolpath("B", "3 mm")]);
    assert!(lines.contains(&"(change to 3 mm)".to_owned()), "{lines:#?}");
    assert_eq!(lines.iter().filter(|l| *l == "M0").count(), 1);
}

#[test]
fn the_first_rapid_after_a_tool_change_always_says_its_height() {
    // the bit is set to a new Z zero during the change, so the machine must not be
    // left to move sideways at whatever height it was zeroed at
    let job = cut_craft_core::document::Job::default();
    let two = || vec![toolpath("A", "6 mm"), toolpath("B", "3 mm")];
    let posts: Vec<Post> = BUILT_IN
        .iter()
        .filter(|(name, _)| !name.contains("wrap"))
        .map(|(_, text)| Post::parse(text).unwrap())
        .flat_map(|p| [p.with_pauses(), p])
        .collect();
    for post in &posts {
        let lines = render_with(post, job, two());
        let change = lines
            .iter()
            .rposition(|l| l.contains("M0") || l.contains("M6"))
            .unwrap_or_else(|| panic!("a tool change: {lines:#?}"));
        let rapid = lines[change..]
            .iter()
            .find(|l| l.starts_with("G0") && (l.contains('X') || l.contains('Y')))
            .unwrap_or_else(|| panic!("a rapid after the change: {lines:#?}"));
        let lifted = lines[change..]
            .iter()
            .take_while(|l| *l != rapid)
            .any(|l| l.starts_with("G0") && l.contains('Z'));
        assert!(
            lifted,
            "{}: no straight lift before the first sideways rapid after the change: {lines:#?}",
            post.name
        );
    }
}

#[test]
fn pausing_for_tools_swaps_a_tool_changer_for_a_stop_to_change_by_hand() {
    let job = cut_craft_core::document::Job::default();
    let two = || vec![toolpath("A", "6 mm"), toolpath("B", "3 mm")];
    let atc = Post::parse(include_str!("../posts/fluidnc_atc_mm.pp")).unwrap();
    assert!(!atc.pauses_for_tools(), "an ATC post changes tools itself");
    let paused = atc.with_pauses();
    assert!(paused.pauses_for_tools());
    let lines = render_with(&paused, job, two());
    assert!(!lines.iter().any(|l| l.contains("M6")), "{lines:#?}");
    assert!(
        lines.iter().any(|l| l.starts_with("(First tool T1: 6 mm")),
        "{lines:#?}"
    );
    let at = lines.iter().position(|l| l == "M0").expect("a stop");
    assert!(lines[at - 1].contains("3 mm"), "names the tool: {lines:#?}");
    // a post that already pauses keeps its own tool change
    let grbl = Post::parse(include_str!("../posts/grbl_straight_mm.pp")).unwrap();
    assert!(grbl.pauses_for_tools());
    assert_eq!(grbl.with_pauses().blocks, grbl.blocks);
    // one without any tool change gets one
    let plain = Post::parse(
        "POST_NAME = \"Plain (*.nc)\"\nFILE_EXTENSION = \"nc\"\nUNITS = \"MM\"\n\
             begin HEADER\n\"G21\"\nbegin RAPID_MOVE\n\"G0 [X] [Y] [Z]\"\n\
             begin FEED_MOVE\n\"G1 [X] [Y] [Z]\"\n",
    )
    .unwrap();
    assert!(!plain.changes_tools());
    let paused = plain.with_pauses();
    assert!(paused.changes_tools() && paused.pauses_for_tools());
    let lines = render_with(&paused, job, two());
    assert_eq!(lines.iter().filter(|l| *l == "M0").count(), 1, "{lines:#?}");
}

#[test]
fn inch_posts_convert_positions_and_feeds() {
    let post = BASIC
        .replace(r#"UNITS = "MM""#, r#"UNITS = "INCHES""#)
        .replace("[X|A|X|1.3]", "[X|A|X|1.4]");
    let lines = render(&post, vec![toolpath("P", "T")]);
    assert!(
        lines.iter().any(|l| l.starts_with("G1 X1.0000 Y0.000")),
        "{lines:#?}"
    );
    assert!(
        lines.iter().any(|l| l.ends_with("F39.4")),
        "1000 mm/min is 39.4 in/min: {lines:#?}"
    );
}

#[test]
fn format_flags_multipliers_and_line_numbers() {
    let post = format!(
        "{}\nLINE_NUMBER = 10\nLINE_NUMBER_INCREMENT = 10\nVAR LINE_NUMBER = [N|A|N|1.0]\nVAR Z_POSITION = [Z|A|Z|+08.2]\nVAR FEED_RATE = [F|A|F|1.0|0.5]\n",
        BASIC.replace("\"G0 [X] [Y] [Z]\"", "\"[N] G0 [X] [Y] [Z]\"")
    );
    let lines = render(&post, vec![toolpath("P", "T")]);
    assert!(
        lines.contains(&"N10 G0 X0.000 Y0.000 Z+0005.00".to_owned()),
        "{lines:#?}"
    );
    assert!(
        lines.contains(&"N20 G0 X25.400 Y25.400 Z+0005.00".to_owned()),
        "{lines:#?}"
    );
    assert!(
        lines.iter().any(|l| l.ends_with("F150")),
        "plunge rate 300 x 0.5, as an integer: {lines:#?}"
    );
}

#[test]
fn substitute_replaces_characters_in_names() {
    let post = format!("{BASIC}\nSUBSTITUTE = \"({{)}}\"\n");
    let lines = render(&post, vec![toolpath("Profile (outside)", "6 mm")]);
    assert_eq!(lines[0], "(Profile {outside} with 6 mm)");
}

#[test]
fn unsupported_variables_are_named_not_ignored() {
    let post = BASIC.replace(
        "\"G1 [X] [Y] [Z] [F]\"",
        "\"G1 [X] [Y] [Z] [REVISION] [F]\"",
    );
    assert_eq!(
        Post::parse(&post).unwrap_err(),
        "the post uses [REVISION], which is not supported yet"
    );
}

/// A rotary job 100 mm round (so 1 mm is 3.6 degrees), zeroed on its axis.
fn hundred_round() -> cut_craft_core::document::Job {
    use cut_craft_core::document::{Along, Job, ZZero};
    Job {
        z_zero: ZZero::MachineBed,
        ..Job::rotary(300.0, 100.0 / std::f64::consts::PI, Along::X)
    }
}

#[test]
fn wrapping_y_prints_it_as_degrees_round_the_cylinder() {
    let post = BASIC.replace(
        "begin HEADER\n",
        "ROTARY_WRAP_Y = \"A\"\nbegin HEADER\n\"([WRAP_DIA] [Z_ORIGIN])\"\n",
    );
    let lines = render_for(&post, hundred_round(), vec![toolpath("P", "T")]);
    assert_eq!(lines[0], "(31.831 Cylinder Axis)");
    assert!(
        lines.contains(&"G1 X25.400 A91.440 Z12.915".to_owned()),
        "25.4 mm round is 91.44 degrees, 3 mm into a 15.915 radius: {lines:#?}"
    );
}

#[test]
fn a_rotary_job_zeroed_on_its_surface_or_its_axis_gets_the_right_z() {
    use cut_craft_core::document::ZZero;
    let post = BASIC.replace(
        "begin HEADER\n",
        "ROTARY_WRAP_Y = \"A\"\nbegin HEADER\n\"([Z_ORIGIN])\"\n",
    );
    let axis = render_for(&post, hundred_round(), vec![toolpath("P", "T")]);
    let surface = render_for(
        &post,
        cut_craft_core::document::Job {
            z_zero: ZZero::MaterialSurface,
            ..hundred_round()
        },
        vec![toolpath("P", "T")],
    );
    assert_eq!(axis[0], "(Cylinder Axis)");
    assert_eq!(surface[0], "(Cylinder Surface)");
    // 3 mm deep: below the top of the cylinder, or 3 mm short of its 15.915 radius
    assert!(
        surface.contains(&"G1 X25.400 A91.440 Z-3.000".to_owned()),
        "{surface:#?}"
    );
    assert!(
        axis.contains(&"G1 X25.400 A91.440 Z12.915".to_owned()),
        "{axis:#?}"
    );
    // the 5 mm clearance stays 5 mm above the surface either way
    assert!(surface.contains(&"G0 Z5.000".to_owned()), "{surface:#?}");
    assert!(axis.contains(&"G0 Z20.915".to_owned()), "{axis:#?}");
}

#[test]
fn inverse_time_feeds_are_the_rate_over_the_move_length() {
    let post = BASIC
            .replace(
                "begin HEADER\n",
                "ROTARY_WRAP_Y = \"A\"\nINVERSE_TIME_MODE = YES\nVAR INVERSE_TIME = [FI|A|F|1.3]\nbegin HEADER\n",
            )
            .replace("[Z] [F]\"", "[Z] [FI]\"");
    let lines = render_for(&post, hundred_round(), vec![toolpath("P", "T")]);
    assert!(
        lines.contains(&"G1 X0.000 A0.000 Z12.915 F37.500".to_owned()),
        "an 8 mm plunge at 300 mm/min takes 1/37.5 minutes: {lines:#?}"
    );
    assert!(
        lines.contains(&"G1 X25.400 A0.000 Z12.915 F39.370".to_owned()),
        "{lines:#?}"
    );
}

/// A post wrapping Y onto A for `hundred_round`, told the machine's rotary.
fn winding_post(axis: cut_craft_core::rotary_axis::RotaryAxis) -> Post {
    let post = BASIC
            .replace(
                "begin HEADER\n",
                "ROTARY_WRAP_Y = \"A\"\nINVERSE_TIME_MODE = YES\nVAR INVERSE_TIME = [FI|A|F|1.3]\nVAR Y_HOME_POSITION = [YH|A|Y|1.3]\nbegin HEADER\n",
            )
            .replace("[Z] [F]\"", "[Z] [FI]\"")
            .replace("\"M5\"\n\"M30\"", "\"G0 [YH]\"\n\"M5\"\n\"M30\"");
    Post::parse(&post).unwrap().with_rotary(axis)
}

/// Two rings round `hundred_round`, each one whole turn (100 mm round), 1 mm deep,
/// at X0 then X10.
fn two_rings() -> Toolpath {
    Toolpath {
        moves: vec![
            mv(MoveKind::Rapid, 0.0, 0.0, 5.0),
            mv(MoveKind::Plunge, 0.0, 0.0, -1.0),
            mv(MoveKind::Cut, 0.0, 50.0, -1.0),
            mv(MoveKind::Cut, 0.0, 100.0, -1.0),
            mv(MoveKind::Rapid, 0.0, 100.0, 5.0),
            mv(MoveKind::Rapid, 10.0, 0.0, 5.0),
            mv(MoveKind::Plunge, 10.0, 0.0, -1.0),
            mv(MoveKind::Cut, 10.0, 50.0, -1.0),
            mv(MoveKind::Cut, 10.0, 100.0, -1.0),
            mv(MoveKind::Rapid, 10.0, 100.0, 5.0),
        ],
        ..toolpath("Rings", "T")
    }
}

/// A line of `lines` starts with `start`.
fn has(lines: &[String], start: &str) -> bool {
    lines.iter().any(|l| l.starts_with(start))
}

/// Every A a file writes.
fn a_words(lines: &[String]) -> Vec<f64> {
    lines
        .iter()
        .flat_map(|l| l.split_whitespace())
        .filter_map(|w| w.strip_prefix('A')?.parse().ok())
        .collect()
}

#[test]
fn a_free_rotary_keeps_turning_the_same_way_ring_after_ring() {
    let axis = cut_craft_core::rotary_axis::RotaryAxis::default();
    let lines = render_with(&winding_post(axis), hundred_round(), vec![two_rings()]);
    // the second ring starts where the first ended (360) and carries on to 720
    assert!(has(&lines, "G1 X10.000 A720.000 Z14.915"), "{lines:#?}");
    // its A never ran back between the rings
    let a = a_words(&lines);
    let cutting = &a[..a.len() - 1];
    assert!(cutting.windows(2).all(|w| w[1] >= w[0] - 1e-9), "{a:?}");
    // and at the end it goes home to the start's place nearest where it is: 720
    assert_eq!(a.last(), Some(&720.0), "{lines:#?}");
    // told nothing of the machine, the post writes A as the job unrolls, as before
    let post = Post {
        rotary: None,
        ..winding_post(axis)
    };
    let lines = render_with(&post, hundred_round(), vec![two_rings()]);
    assert!(
        a_words(&lines).iter().all(|a| (0.0..=360.0).contains(a)),
        "{lines:#?}"
    );
}

#[test]
fn at_its_limit_a_rotary_cuts_the_next_pass_backwards_or_spins_back() {
    use cut_craft_core::rotary_axis::{AtLimit, RotaryAxis, Turning};
    let limited = |at_limit| RotaryAxis {
        turning: Turning::Unwind,
        turns: 1.0,
        at_limit,
        ..RotaryAxis::default()
    };
    let both = render_with(
        &winding_post(limited(AtLimit::CutBothWays)),
        hundred_round(),
        vec![two_rings()],
    );
    let a = a_words(&both);
    assert!(a.iter().all(|a| (-360.0..=360.0).contains(a)), "{both:#?}");
    // the second ring goes straight down at 360 and cuts back round to 0
    let at = both
        .iter()
        .position(|l| l.starts_with("G1 X10.000 A360.000 Z14.915"))
        .unwrap();
    assert!(
        both[at + 1].starts_with("G1 X10.000 A180.000 Z14.915"),
        "{both:#?}"
    );
    assert!(
        both[at + 2].starts_with("G1 X10.000 A0.000 Z14.915"),
        "{both:#?}"
    );

    let back = render_with(
        &winding_post(limited(AtLimit::SpinBack)),
        hundred_round(),
        vec![two_rings()],
    );
    assert!(
        a_words(&back).iter().all(|a| (-360.0..=360.0).contains(a)),
        "{back:#?}"
    );
    // lifted clear before A spins back to 0, then the second ring cut forwards again
    let spin = back
        .iter()
        .position(|l| l == "G0 X10.000 A0.000 Z20.915")
        .unwrap();
    assert!(back[spin - 1].ends_with("A360.000 Z20.915"), "{back:#?}");
    assert!(has(&back, "G1 X10.000 A360.000 Z14.915"), "{back:#?}");
    // and home at the very start, unwound
    assert_eq!(a_words(&back).last(), Some(&0.0));
}

#[test]
fn a_machine_saved_before_a_can_turn_writes_a_within_one_turn_as_before() {
    let axis = cut_craft_core::rotary_axis::RotaryAxis::saved_before();
    let lines = render_with(&winding_post(axis), hundred_round(), vec![two_rings()]);
    assert!(
        a_words(&lines).iter().all(|a| (0.0..=360.0).contains(a)),
        "{lines:#?}"
    );
    assert_eq!(a_words(&lines).last(), Some(&0.0), "{lines:#?}");
}

#[test]
fn a_turns_only_as_high_as_the_file_lifts_even_when_a_pass_is_reached_quickly() {
    use cut_craft_core::rotary_axis::RotaryAxis;
    // a ring, then lifted 30 above the top (clear of a board's edges as it swings), over to
    // half way round, and down at the rapid rate to just above the next pass before plunging
    let quick = Toolpath {
        moves: vec![
            mv(MoveKind::Rapid, 0.0, 0.0, 30.0),
            mv(MoveKind::Plunge, 0.0, 0.0, -1.0),
            mv(MoveKind::Cut, 0.0, 50.0, -1.0),
            mv(MoveKind::Cut, 0.0, 100.0, -1.0),
            mv(MoveKind::Rapid, 0.0, 100.0, 30.0),
            mv(MoveKind::Rapid, 10.0, 50.0, 30.0),
            mv(MoveKind::Rapid, 10.0, 50.0, 0.0),
            mv(MoveKind::Plunge, 10.0, 50.0, -1.0),
            mv(MoveKind::Cut, 10.0, 100.0, -1.0),
            mv(MoveKind::Rapid, 10.0, 100.0, 30.0),
        ],
        ..toolpath("Rings", "T")
    };
    let lines = render_with(
        &winding_post(RotaryAxis::default()),
        hundred_round(),
        vec![quick],
    );
    // A goes on round to 540 at the file's 30 above the top, not the job's 5 mm clearance
    let turn = lines.iter().position(|l| l.contains("A540.000")).unwrap();
    assert_eq!(lines[turn], "G0 X10.000 A540.000 Z45.915", "{lines:#?}");
}

#[test]
fn backlash_is_taken_up_each_time_a_turns_the_other_way() {
    use cut_craft_core::rotary_axis::{RotaryAxis, Turning};
    let axis = RotaryAxis {
        turning: Turning::Unwind,
        turns: 1.0,
        backlash: 0.5,
        ..RotaryAxis::default()
    };
    let lines = render_with(&winding_post(axis), hundred_round(), vec![two_rings()]);
    // going back round, every A is half a degree further back
    assert!(has(&lines, "G1 X10.000 A179.500 Z14.915"), "{lines:#?}");
    assert!(has(&lines, "G1 X10.000 A-0.500 Z14.915"), "{lines:#?}");
}

/// One side of `hundred_round` cut with A held at 90° (25 mm round): back and forth along
/// X, 2 mm down, with the bit held 4 mm to one side of the axis and then 4 mm to the other.
/// Its moves are where the tip cuts, as a toolpath keeps them.
#[test]
fn a_rotary_toolpath_whose_across_list_does_not_match_its_moves_is_refused() {
    use cut_craft_core::rotary_axis::RotaryAxis;
    // something shortened the moves and left the list of how far the bit is held off the
    // axis as it was: every move after the first dropped one would be written with
    // another move's place, so the post must not write it at all
    let mut face = one_face();
    face.across.pop();
    let mut doc = Document {
        job: hundred_round(),
        ..Document::default()
    };
    let mut c = Change::builder(&doc);
    c.add_toolpath(face);
    doc.apply(&c.finish()).unwrap();
    let post = Post::default_for(&doc.job).with_rotary(RotaryAxis::default());
    let file = File {
        path: std::path::Path::new("/jobs/face.nc"),
        job: None,
        made: chrono::NaiveDateTime::default(),
        slots: &[],
        notes: &[],
    };
    let refused = post.render(&doc, &file).unwrap_err();
    assert!(refused.contains("Face"), "{refused}");
}

fn one_face() -> Toolpath {
    use cut_craft_core::rotary_axis::tip_of;
    let radius = 50.0 / std::f64::consts::PI;
    let held = [
        (MoveKind::Rapid, 0.0, 5.0, -4.0),
        (MoveKind::Plunge, 0.0, -2.0, -4.0),
        (MoveKind::Cut, 10.0, -2.0, -4.0),
        (MoveKind::Cut, 10.0, -2.0, 4.0),
        (MoveKind::Cut, 0.0, -2.0, 4.0),
        (MoveKind::Rapid, 0.0, 5.0, 4.0),
    ];
    let moves = held
        .iter()
        .map(|&(kind, x, z, across)| {
            let (round, z) = tip_of(radius, 25.0, z, across);
            mv(kind, x, round, z)
        })
        .collect();
    Toolpath {
        moves,
        across: held.iter().map(|h| h.3).collect(),
        ..toolpath("Face", "T")
    }
}

#[test]
fn a_bit_held_off_the_axis_is_written_as_a_held_still_and_the_across_axis_moving() {
    use cut_craft_core::rotary_axis::RotaryAxis;
    let job = hundred_round();
    let axis = RotaryAxis {
        centre_across: 5.0,
        ..RotaryAxis::default()
    };
    let post = Post::default_for(&job).with_rotary(axis);
    let lines = render_with(&post, job, vec![one_face()]);
    let words = |letter: char| -> Vec<f64> {
        lines
            .iter()
            .filter(|l| l.starts_with("G1"))
            .flat_map(|l| l.split_whitespace())
            .filter_map(|w| w.strip_prefix(letter)?.parse().ok())
            .collect()
    };
    // A turns to the side and holds still while the bit cuts it
    assert!(
        words('A').iter().all(|a| (a - 90.0).abs() < 1e-9),
        "{lines:#?}"
    );
    // the bit moves across, 4 mm each side of the axis (which is 5 mm across from zero)
    assert!(has(&lines, "G0 X0.000 A90.000 Y1.000"), "{lines:#?}");
    assert!(words('Y').contains(&9.0), "{lines:#?}");
    // the cut straight across at X10 (Y1 to Y9) is written on its own, before the cut
    // back along X, not folded into one diagonal with it
    let across = lines
        .iter()
        .position(|l| l.starts_with("G1") && l.contains("Y9.000") && !l.contains("X0.000"))
        .unwrap_or_else(|| panic!("no cut across: {lines:#?}"));
    let back = lines
        .iter()
        .position(|l| l.starts_with("G1") && l.contains("X0.000"))
        .unwrap();
    assert!(across < back, "{lines:#?}");
    // and 2 mm below the top straight over the axis (Z zero on the axis)
    assert!(
        words('Z').iter().all(|z| (z - 13.915).abs() < 1e-3),
        "{lines:#?}"
    );
    // turning the other way, positive across is the other way along Y
    let job = hundred_round();
    let post = Post::default_for(&job).with_rotary(RotaryAxis {
        clockwise: false,
        ..axis
    });
    let lines = render_with(&post, job, vec![one_face()]);
    assert!(has(&lines, "G0 X0.000 A90.000 Y9.000"), "{lines:#?}");
    // a post that can't move across says so rather than cut in the wrong place
    let job = hundred_round();
    let flat = BASIC.replace("begin HEADER\n", "ROTARY_WRAP_Y = \"A\"\nbegin HEADER\n");
    let mut doc = Document {
        job,
        ..Document::default()
    };
    let mut c = Change::builder(&doc);
    c.add_toolpath(one_face());
    doc.apply(&c.finish()).unwrap();
    let why = Post::parse(&flat)
        .unwrap()
        .render(
            &doc,
            &File {
                path: std::path::Path::new("/jobs/face.nc"),
                job: None,
                made: chrono::NaiveDateTime::default(),
                slots: &[],
                notes: &[],
            },
        )
        .unwrap_err();
    assert!(why.contains("off the turning axis"), "{why}");
}

#[test]
fn inverse_time_on_a_rotary_is_timed_where_the_bit_cuts_and_within_a_top_speed() {
    use cut_craft_core::rotary_axis::RotaryAxis;
    let radius = 50.0 / std::f64::consts::PI;
    let half_way = Toolpath {
        moves: vec![
            mv(MoveKind::Rapid, 0.0, 0.0, 5.0),
            mv(MoveKind::Plunge, 0.0, 0.0, -radius / 2.0),
            mv(MoveKind::Cut, 0.0, 10.0, -radius / 2.0),
        ],
        ..toolpath("Half way in", "T")
    };
    let fi = |max_rate| {
        let axis = RotaryAxis {
            max_rate,
            ..RotaryAxis::default()
        };
        let lines = render_with(&winding_post(axis), hundred_round(), vec![half_way.clone()]);
        lines
            .iter()
            .find(|l| l.contains("A36.000"))
            .and_then(|l| l.rsplit_once('F'))
            .map(|(_, f)| f.to_owned())
            .unwrap()
    };
    // 10 mm round the surface is 5 mm half way to the axis: 1/200 minute at 1000 mm/min
    assert_eq!(fi(0.0), "200.000");
    // but 36 degrees at no more than 360 a minute takes a tenth of a minute
    assert_eq!(fi(360.0), "10.000");
}

#[test]
fn a_pass_at_several_depths_is_never_cut_backwards_deepest_first() {
    use cut_craft_core::rotary_axis::{AtLimit, RotaryAxis, Turning};
    // a ring at X0, then at X10 a pass 1 mm down one turn round and 2 mm down the next,
    // with no lift between
    let mut moves = two_rings().moves[..5].to_vec();
    moves.extend([
        mv(MoveKind::Rapid, 10.0, 0.0, 5.0),
        mv(MoveKind::Plunge, 10.0, 0.0, -1.0),
        mv(MoveKind::Cut, 10.0, 50.0, -1.0),
        mv(MoveKind::Cut, 10.0, 100.0, -1.0),
        mv(MoveKind::Plunge, 10.0, 100.0, -2.0),
        mv(MoveKind::Cut, 10.0, 150.0, -2.0),
        mv(MoveKind::Cut, 10.0, 200.0, -2.0),
        mv(MoveKind::Rapid, 10.0, 200.0, 5.0),
    ]);
    let deeper = Toolpath {
        moves,
        ..toolpath("Two depths", "T")
    };
    let axis = RotaryAxis {
        turning: Turning::Unwind,
        turns: 1.0,
        at_limit: AtLimit::CutBothWays,
        ..RotaryAxis::default()
    };
    let lines = render_with(&winding_post(axis), hundred_round(), vec![deeper]);
    assert!(
        a_words(&lines).iter().all(|a| (-360.0..=360.0).contains(a)),
        "{lines:#?}"
    );
    // it spins back clear of the material, then cuts 1 mm down before 2 mm down
    assert!(has(&lines, "G0 X10.000 A-360.000 Z20.915"), "{lines:#?}");
    let first_cut = lines.iter().find(|l| l.starts_with("G1 X10.000")).unwrap();
    assert!(first_cut.contains("Z14.915"), "{lines:#?}");
}

#[test]
fn a_pass_longer_than_a_can_turn_is_still_written_as_near_the_middle_as_it_goes() {
    use cut_craft_core::rotary_axis::{RotaryAxis, Turning};
    // half a turn each way, but each ring is a whole turn: the checks before cutting warn
    // and ask; the file cuts each ring as little past the limit as it can
    let half = RotaryAxis {
        turning: Turning::Unwind,
        turns: 0.5,
        ..RotaryAxis::default()
    };
    let lines = render_with(&winding_post(half), hundred_round(), vec![two_rings()]);
    let a = a_words(&lines);
    assert!(a.iter().all(|a| (-360.0..=360.0).contains(a)), "{lines:#?}");
    assert!(has(&lines, "G1 X10.000"), "both rings are cut: {lines:#?}");
}

#[test]
fn a_rotary_file_cutting_only_over_the_axis_never_moves_the_across_axis() {
    // zeroed on the axis by hand along Y, a plain wrapped file must leave Y where it is
    let post = BASIC
        .replace(
            "begin HEADER\n",
            "ROTARY_WRAP_Y = \"A\"\nVAR ACROSS_POSITION = [ACROSS|C|Y|1.3]\nbegin HEADER\n",
        )
        .replace("[Z] [F]\"", "[Z] [ACROSS] [F]\"");
    let lines = render_for(&post, hundred_round(), vec![two_rings()]);
    assert!(
        lines
            .iter()
            .all(|l| !l.split_whitespace().any(|w| w.starts_with('Y'))),
        "{lines:#?}"
    );
    // the default rotary post too
    let job = hundred_round();
    let lines = render_with(&Post::default_for(&job), job, vec![two_rings()]);
    assert!(
        lines
            .iter()
            .all(|l| !l.split_whitespace().any(|w| w.starts_with('Y'))),
        "{lines:#?}"
    );
}

#[test]
fn a_wrapping_post_needs_a_rotary_job() {
    let post = BASIC.replace("begin HEADER\n", "ROTARY_WRAP_Y = \"A\"\nbegin HEADER\n");
    let mut doc = Document::default();
    let mut c = Change::builder(&doc);
    c.add_toolpath(toolpath("P", "T"));
    let change = c.finish();
    doc.apply(&change).unwrap();
    let file = File {
        path: std::path::Path::new("/out.nc"),
        job: None,
        made: chrono::NaiveDateTime::default(),
        slots: &[],
        notes: &[],
    };
    let why = Post::parse(&post).unwrap().render(&doc, &file).unwrap_err();
    assert!(why.contains("rotary"), "{why}");
}

#[test]
fn inverse_time_needs_wrapping() {
    let post = BASIC.replace("begin HEADER\n", "INVERSE_TIME_MODE = YES\nbegin HEADER\n");
    let why = Post::parse(&post).unwrap_err();
    assert!(why.contains("INVERSE_TIME_MODE"), "{why}");
}

/// Renders through the default post for `job`.
fn render_default(job: cut_craft_core::document::Job, toolpaths: Vec<Toolpath>) -> String {
    let mut doc = Document {
        job,
        ..Document::default()
    };
    let mut c = Change::builder(&doc);
    for t in toolpaths {
        c.add_toolpath(t);
    }
    let change = c.finish();
    doc.apply(&change).unwrap();
    let file = File {
        path: std::path::Path::new("/out.nc"),
        job: None,
        made: chrono::NaiveDateTime::default(),
        slots: &[],
        notes: &[],
    };
    Post::default_for(&doc.job).render(&doc, &file).unwrap()
}

#[test]
fn the_default_post_is_fluidnc_with_a_tool_changer() {
    let g = render_default(
        cut_craft_core::document::Job::default(),
        vec![toolpath("A", "6 mm"), toolpath("B", "3 mm")],
    );
    assert!(g.contains("M6 T1") && g.contains("M6 T2"), "{g}");
    assert!(g.contains("F1000.0") && !g.contains("G93"), "{g}");
}

#[test]
fn a_post_changes_tools_only_with_a_toolchange_block() {
    assert!(Post::parse(BASIC).unwrap().changes_tools());
    let without = BASIC.replace("begin TOOLCHANGE", "begin UNUSED_BLOCK");
    assert!(!Post::parse(&without).unwrap().changes_tools());
    let job = cut_craft_core::document::Job::default();
    assert!(Post::default_for(&job).changes_tools());
}

#[test]
fn a_dwelling_drill_waits_after_each_plunge_where_the_post_can() {
    let mut drill = toolpath("Holes", "6 mm");
    drill.params = serde_json::json!({ "dwell": 0.5 });
    let mut doc = Document::default();
    let mut c = Change::builder(&doc);
    c.add_toolpath(drill);
    doc.apply(&c.finish()).unwrap();
    let file = File {
        path: std::path::Path::new("/out.nc"),
        job: None,
        made: chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap(),
        slots: &[],
        notes: &[],
    };
    let plain = Post::parse(BASIC).unwrap();
    let why = plain.render(&doc, &file).unwrap_err();
    assert!(why.contains("DWELL_MOVE"), "{why}");
    let dwelling = Post::parse(&format!(
        "{BASIC}\nVAR DWELL_TIME = [DWELL|A|P|1.1]\nbegin DWELL_MOVE\n\"G4 [DWELL]\"\n"
    ))
    .unwrap();
    let g = dwelling.render(&doc, &file).unwrap();
    assert!(g.contains("G4 P0.5"), "{g}");
    // Cut Craft's own posts dwell
    let own = Post::default_for(&doc.job).render(&doc, &file).unwrap();
    assert!(own.contains("G4 P0.5"), "{own}");
}

#[test]
fn rotary_jobs_get_the_default_post_wrapped_onto_a() {
    use cut_craft_core::document::Along;
    let round = |along| {
        let job = cut_craft_core::document::Job::rotary(300.0, 50.0, along);
        render_default(job, vec![toolpath("P", "T")])
    };
    // 25.4 mm round a 50 mm cylinder is 58.213 degrees
    let along_x = round(Along::X);
    assert!(
        along_x.contains("G93") && along_x.contains("G94"),
        "{along_x}"
    );
    assert!(
        along_x.contains("X25.400") && along_x.contains("A58.213"),
        "Y goes round a cylinder along X: {along_x}"
    );
    let along_y = round(Along::Y);
    assert!(
        along_y.contains("Y25.400") && along_y.contains("A58.213") && !along_y.contains("X25.400"),
        "X goes round a cylinder along Y: {along_y}"
    );
}

#[test]
fn a_new_feed_rate_starts_with_the_first_feed_move_block() {
    // like most real posts, only FIRST_FEED_MOVE prints the rate
    let post = BASIC.replace(
        "begin FEED_MOVE\n\"G1 [X] [Y] [Z] [F]\"",
        "begin FEED_MOVE\n\"G1 [X] [Y] [Z]\"",
    );
    let lines = render(&post, vec![toolpath("P", "T")]);
    assert!(
        lines.contains(&"G1 X25.400 Y0.000 Z-3.000 F1000.0".to_owned()),
        "after plunging at 300 the cut prints its own 1000: {lines:#?}"
    );
}

#[test]
fn posts_may_define_blocks_cut_craft_never_writes() {
    let post = format!(
        "{BASIC}\nbegin CW_HELICAL_ARC_PLUNGE_MOVE\n\"G2 [X] [Y] [Z] [I] [J] [FP]\"\nbegin REVISION_COMMENT\n\"(revision [REVISION])\"\n"
    );
    let lines = render(&post, vec![toolpath("P", "T")]);
    assert!(
        lines
            .iter()
            .all(|l| !l.starts_with("G2 ") && !l.starts_with("(revision")),
        "{lines:#?}"
    );
}

/// `BASIC` with arc blocks, centres relative to the start.
fn arc_post(helical: bool) -> String {
    let mut blocks = String::from(
        "VAR ARC_CENTRE_I_INC_POSITION = [I|A|I|1.3]\nVAR ARC_CENTRE_J_INC_POSITION = [J|A|J|1.3]\n\
             begin FIRST_CW_ARC_MOVE\n\"G2 [X] [Y] [Z] [I] [J] [F]\"\nbegin CW_ARC_MOVE\n\"G2 [X] [Y] [Z] [I] [J]\"\n\
             begin FIRST_CCW_ARC_MOVE\n\"G3 [X] [Y] [Z] [I] [J] [F]\"\nbegin CCW_ARC_MOVE\n\"G3 [X] [Y] [Z] [I] [J]\"\n",
    );
    if helical {
        blocks.push_str(
            "begin CW_HELICAL_ARC_MOVE\n\"G2 [X] [Y] [Z] [I] [J] (helix)\"\n\
                 begin CCW_HELICAL_ARC_MOVE\n\"G3 [X] [Y] [Z] [I] [J] (helix)\"\n",
        );
    }
    format!("{BASIC}\n{blocks}")
}

/// A toolpath cutting through `points` from the first, 3 mm deep.
fn cutting(points: &[DVec3]) -> Toolpath {
    let start = points[0];
    let mut t = toolpath("Circle", "6 mm");
    t.moves = [
        mv(MoveKind::Rapid, start.x, start.y, 5.0),
        mv(MoveKind::Plunge, start.x, start.y, start.z),
    ]
    .into_iter()
    .chain(points[1..].iter().map(|p| Move {
        kind: MoveKind::Cut,
        to: *p,
    }))
    .chain([mv(MoveKind::Rapid, 0.0, 0.0, 5.0)])
    .collect();
    t
}

/// The outside of a 40 mm circle for a 6 mm tool, as a profile would cut it: the offset
/// outline, round to its start.
fn circle_profile() -> Vec<DVec3> {
    use cut_craft_core::geometry::{Path, Point};
    use cut_craft_core::polygon::{Corners, offset};
    let outline = offset(
        &[Path::circle(Point::new(100.0, 100.0), 20.0)],
        3.0,
        Corners::Round,
    );
    let mut points: Vec<DVec3> = outline[0]
        .flatten(cut_craft_core::polygon::TOLERANCE)
        .iter()
        .map(|p| p.extend(-3.0))
        .collect();
    if points.first() != points.last() {
        points.push(points[0]);
    }
    points
}

/// The XY path a G-code text cuts at Z below 0, arcs expanded into fine steps.
fn expand(lines: &[String]) -> Vec<cut_craft_core::geometry::Point> {
    use cut_craft_core::geometry::Point;
    let word = |line: &str, letter: char| {
        line.split_whitespace()
            .find_map(|w| w.strip_prefix(letter)?.parse::<f64>().ok())
    };
    let mut at = DVec3::ZERO;
    let mut path = vec![];
    for line in lines {
        let Some(code) = line.split_whitespace().next() else {
            continue;
        };
        if !matches!(code, "G0" | "G1" | "G2" | "G3") {
            continue;
        }
        let to = DVec3::new(
            word(line, 'X').unwrap_or(at.x),
            word(line, 'Y').unwrap_or(at.y),
            word(line, 'Z').unwrap_or(at.z),
        );
        if matches!(code, "G2" | "G3") {
            let centre =
                at.truncate() + Point::new(word(line, 'I').unwrap(), word(line, 'J').unwrap());
            let (from, end) = (at.truncate() - centre, to.truncate() - centre);
            let mut sweep = from.angle_to(end);
            if code == "G2" && sweep > 0.0 {
                sweep -= std::f64::consts::TAU;
            } else if code == "G3" && sweep < 0.0 {
                sweep += std::f64::consts::TAU;
            }
            for i in 0..=100 {
                let turn = Point::from_angle(sweep * f64::from(i) / 100.0);
                path.push(centre + turn.rotate(from));
            }
        } else if to.z < 0.0 {
            path.push(to.truncate());
        }
        at = to;
    }
    path
}

#[test]
fn a_circle_posts_as_a_few_arcs_that_retrace_its_lines() {
    use cut_craft_core::geometry::{Point, polyline_distance};
    let circle = circle_profile();
    assert!(circle.len() > 100, "{} lines", circle.len());
    let lines = render(&arc_post(false), vec![cutting(&circle)]);
    let arcs = lines
        .iter()
        .filter(|l| l.starts_with("G2 ") || l.starts_with("G3 "));
    assert!((2..=4).contains(&arcs.count()), "{lines:#?}");
    assert!(
        !lines
            .iter()
            .any(|l| l.starts_with("G1 ") && l.contains("Z-3.000") && !l.contains("F300")),
        "only the plunge is straight: {lines:#?}"
    );
    // within the tolerance both ways, plus the 3 places the post prints to
    let near = cut_craft_core::polygon::TOLERANCE + 0.002;
    let flat: Vec<Point> = circle.iter().map(|p| p.truncate()).collect();
    let traced = expand(&lines);
    for p in &flat {
        assert!(polyline_distance(&traced, *p) < near, "{p} is off the arcs");
    }
    for p in &traced {
        assert!(polyline_distance(&flat, *p) < near, "{p} is off the lines");
    }
}

#[test]
fn lines_that_are_not_arcs_stay_straight() {
    let lines = render(&arc_post(true), vec![toolpath("P", "T")]);
    assert!(
        lines.contains(&"G1 X25.400 Y0.000 Z-3.000 F1000.0".to_owned())
            && lines.contains(&"G1 X25.400 Y25.400 Z-3.000".to_owned()),
        "{lines:#?}"
    );
    assert!(
        !lines
            .iter()
            .any(|l| l.starts_with("G2 ") || l.starts_with("G3 ")),
        "{lines:#?}"
    );
}

#[test]
fn a_post_without_arc_blocks_writes_every_line() {
    let circle = circle_profile();
    let lines = render(BASIC, vec![cutting(&circle)]);
    let cuts = lines.iter().filter(|l| l.starts_with("G1 ")).count();
    assert_eq!(cuts, circle.len(), "the plunge and each line: {lines:#?}");
}

#[test]
fn a_ramp_round_a_circle_is_a_helix_when_the_post_has_helical_arcs() {
    // a quarter turn of radius 10 going down 2 mm, in 30 steps
    let helix: Vec<DVec3> = (0..=30)
        .map(|i| {
            let turn = f64::from(i) / 30.0 * std::f64::consts::FRAC_PI_2;
            DVec3::new(
                10.0 * turn.cos(),
                10.0 * turn.sin(),
                -1.0 - 2.0 * f64::from(i) / 30.0,
            )
        })
        .collect();
    let lines = render(&arc_post(true), vec![cutting(&helix)]);
    assert!(
        lines.contains(&"G3 X0.000 Y10.000 Z-3.000 I-10.000 J0.000 (helix)".to_owned()),
        "{lines:#?}"
    );
    let flat = render(&arc_post(false), vec![cutting(&helix)]);
    assert!(!flat.iter().any(|l| l.starts_with("G3 ")), "{flat:#?}");
}

#[test]
fn a_helix_entry_goes_down_in_helical_arcs_at_the_plunge_rate() {
    // an entry helix: a quarter turn of radius 10 down 2 mm, gone down at the plunge rate
    let mut t = cutting(&[DVec3::new(10.0, 0.0, 0.0), DVec3::new(10.0, 0.0, -3.0)]);
    let helix = (1..=30).map(|i| {
        let turn = f64::from(i) / 30.0 * std::f64::consts::FRAC_PI_2;
        mv(
            MoveKind::Plunge,
            10.0 * turn.cos(),
            10.0 * turn.sin(),
            -1.0 - 2.0 * f64::from(i) / 30.0,
        )
    });
    t.moves = [
        mv(MoveKind::Rapid, 10.0, 0.0, 5.0),
        mv(MoveKind::Plunge, 10.0, 0.0, -1.0),
    ]
    .into_iter()
    .chain(helix)
    .chain([mv(MoveKind::Rapid, 0.0, 10.0, 5.0)])
    .collect();
    let plunge = t.tool.plunge_rate;
    let lines = render(&arc_post(true), vec![t]);
    let arc = lines
        .iter()
        .find(|l| l.starts_with("G3 X0.000 Y10.000 Z-3.000"))
        .unwrap_or_else(|| panic!("{lines:#?}"));
    assert!(arc.ends_with("(helix)"), "{arc}");
    // the feed is set to the plunge rate before it, not the cutting feed
    let at = lines.iter().position(|l| l == arc).unwrap();
    let fed = lines[..at]
        .iter()
        .rev()
        .find_map(|l| l.split_whitespace().find(|w| w.starts_with('F')))
        .unwrap();
    assert_eq!(fed, format!("F{plunge:.1}"), "{lines:#?}");
}

/// How far, in mm, the end of each arc in `lines` (in `post`'s units) is from its centre
/// further than its start is, with the arc's radius: what GRBL compares (error 33).
fn arc_mismatches(lines: &[String], inches: bool) -> Vec<(f64, f64)> {
    let unit = if inches { MM_PER_INCH } else { 1.0 };
    let word = |line: &str, letter: char| {
        line.split_whitespace()
            .find_map(|w| w.strip_prefix(letter)?.parse::<f64>().ok())
            .map(|v| v * unit)
    };
    let (mut x, mut y) = (0.0, 0.0);
    let mut out = vec![];
    for line in lines {
        let to_x = word(line, 'X').unwrap_or(x);
        let to_y = word(line, 'Y').unwrap_or(y);
        if line.starts_with("G2 ") || line.starts_with("G3 ") {
            let c = (
                x + word(line, 'I').unwrap_or(0.0),
                y + word(line, 'J').unwrap_or(0.0),
            );
            let r0 = f64::hypot(x - c.0, y - c.1);
            let r1 = f64::hypot(to_x - c.0, to_y - c.1);
            out.push(((r1 - r0).abs(), r0));
        }
        if line.starts_with('G') {
            (x, y) = (to_x, to_y);
        }
    }
    out
}

#[test]
fn every_arc_passes_grbls_check_that_its_ends_are_the_same_distance_from_its_centre() {
    // small arcs at awkward places, where rounding to the printed places matters most:
    // GRBL stops a job with error 33 when the ends differ by over 0.005 mm and 0.1%
    let mut seed = 12_345_u64;
    let mut next = || {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        #[expect(clippy::cast_precision_loss)]
        let r = (seed >> 11) as f64 / (1_u64 << 53) as f64;
        r
    };
    let mut toolpaths = vec![];
    for k in 0..3000 {
        let (cx, cy) = (10.0 + 80.0 * next(), 10.0 + 80.0 * next());
        let radius = 0.2 + 2.0 * next();
        let (start, sweep) = (std::f64::consts::TAU * next(), 2.4 + 0.7 * next());
        let way = if k % 2 == 0 { 1.0 } else { -1.0 };
        let points: Vec<DVec3> = (0..=24)
            .map(|i| {
                let a = start + way * sweep * f64::from(i) / 24.0;
                DVec3::new(cx + radius * a.cos(), cy + radius * a.sin(), -1.0)
            })
            .collect();
        toolpaths.push(cutting(&points));
    }
    for (name, text) in BUILT_IN {
        let post = Post::parse(text).unwrap();
        if post.arc_limits(false).is_none() {
            continue;
        }
        let lines = render_with(
            &post,
            cut_craft_core::document::Job::default(),
            toolpaths.clone(),
        );
        let arcs = arc_mismatches(&lines, post.inches);
        assert!(arcs.len() >= 3000, "{name}: {} arcs", arcs.len());
        for (off, radius) in arcs {
            assert!(
                off <= 0.005 || (off <= 0.5 && off <= 0.001 * radius),
                "{name}: an arc of radius {radius:.4} mm ends {off:.4} mm off its circle"
            );
        }
    }
}

#[test]
fn arcs_turning_clockwise_are_g2_with_the_centre_from_their_start() {
    // a half turn of radius 10 round 50, 50 from its right, clockwise
    let half: Vec<DVec3> = (0..=40)
        .map(|i| {
            let turn = -f64::from(i) / 40.0 * std::f64::consts::PI;
            DVec3::new(50.0 + 10.0 * turn.cos(), 50.0 + 10.0 * turn.sin(), -3.0)
        })
        .collect();
    let lines = render(&arc_post(false), vec![cutting(&half)]);
    assert!(
        lines.contains(&"G2 X40.000 Y50.000 Z-3.000 I-10.000 J0.000 F1000.0".to_owned()),
        "{lines:#?}"
    );
}

#[test]
fn plunges_rapid_down_to_the_plunge_gap_first() {
    use cut_craft_core::document::Job;
    let job = Job {
        plunge: Some(1.0),
        ..Job::default()
    };
    let lines = render_for(BASIC, job, vec![toolpath("P", "T")]);
    let at = lines
        .iter()
        .position(|l| l == "G0 X0.000 Y0.000 Z5.000")
        .unwrap();
    assert_eq!(
        lines[at + 1..at + 3],
        ["G0 X0.000 Y0.000 Z1.000", "G1 X0.000 Y0.000 Z-3.000 F300.0"]
    );
    // with the gap at the clearance (older jobs) nothing changes
    let lines = render(BASIC, vec![toolpath("P", "T")]);
    assert_eq!(lines[5], "G1 X0.000 Y0.000 Z-3.000 F300.0");
}

#[test]
fn the_tool_starts_and_ends_at_home() {
    use cut_craft_core::document::{Home, Job, ZZero};
    let job = Job {
        home: Some(Home {
            x: 10.0,
            y: 20.0,
            z: 30.0,
        }),
        z_zero: ZZero::MachineBed,
        ..Job::default()
    };
    let g = render_default(job, vec![toolpath("P", "T")]);
    let lines: Vec<&str> = g.lines().collect();
    let header = lines.iter().position(|l| *l == "G0 Z50.000").unwrap();
    assert_eq!(lines[header + 1], "G0 X10.000 Y20.000", "{g}");
    assert_eq!(
        lines[lines.len() - 4..],
        ["G0 Z50.000", "M5", "G0 X10.000 Y20.000", "M30"],
        "30 above the 20 mm material, zeroed on the bed: {g}"
    );
}

#[test]
fn the_default_post_writes_arcs() {
    let g = render_default(
        cut_craft_core::document::Job::default(),
        vec![cutting(&circle_profile())],
    );
    assert!(g.contains("\nG2 ") || g.contains("\nG3 "), "{g}");
    assert!(g.lines().count() < 40, "{g}");
}

#[test]
fn variables_without_a_var_line_print_in_default_formats() {
    let post = "POST_NAME = \"Bare\"\nFILE_EXTENSION = \"nc\"\nUNITS = \"MM\"\nLINE_ENDING = \"[13][10]\"\nbegin HEADER\n\"[S] M3 [XLENGTH]\"\nbegin RAPID_MOVE\n\"G0 [X] [Y] [Z]\"\nbegin FEED_MOVE\n\"G1 [X] [Y] [Z] [F]\"\n";
    let lines = render(post, vec![toolpath("P", "T")]);
    assert_eq!(
        lines[0], "18000 M3 600.000",
        "rates whole, lengths to 3 places"
    );
    assert!(
        lines.contains(&"G1 25.400 0.000 -3.000 1000".to_owned()),
        "{lines:#?}"
    );
}

#[test]
fn header_variables_describe_the_job_the_file_and_the_time() {
    let post = BASIC.replace(
            "begin HEADER\n",
            "begin HEADER\n\"([TP_FILENAME]|[TP_DIR]|[TP_EXT]|[PATHNAME]|[FILE_NAME]|[FILE_PATHNAME])\"\n\"([DATE] - [TIME])\"\n\"([XLENGTH] [YLENGTH] [ZLENGTH] [XMIN] [XMAX] [YMIN] [YMAX] [ZMIN] [ZMAX])\"\n\"([Z_ORIGIN] / [XY_ORIGIN] / [UNITS] / [TDIA])\"\n\"([PRODUCT] / [POST_NAME] / [FILE_NOTES][TOOLPATH_NOTES][TOOL_NOTES])\"\n",
        );
    let lines = render(&post, vec![toolpath("P", "T")]);
    assert_eq!(
        lines[..5],
        [
            "(front|/jobs/sign|nc|/jobs/sign/front.nc|sign|/jobs/sign.ccjob)",
            "(Saturday October 24 2026 - 09:37 AM)",
            "(600.000 400.000 20.000 0.000 600.000 0.000 400.000 -20.000 0.000)",
            "(Material Surface / Bottom Left Corner / MM / 6.000)",
            &format!(
                "(Cut Craft {} / Test G-code (mm) (*.nc) / )",
                env!("CARGO_PKG_VERSION")
            ),
        ]
    );
}

#[test]
fn notes_go_under_the_header_as_comments_only_when_asked() {
    let mut doc = Document::default();
    let mut c = Change::builder(&doc);
    c.add_toolpath(toolpath("A", "6 mm"));
    doc.apply(&c.finish()).unwrap();
    doc.notes = "# Setup\n- clamp (corners)\nG0 Z-50".into();
    let lines = vec!["Setup".to_owned(), "- clamp [corners]".to_owned()];
    let render = |post: &str, doc: &Document, notes: &[String]| {
        let file = File {
            path: std::path::Path::new("/out.nc"),
            job: None,
            made: chrono::NaiveDateTime::default(),
            slots: &[],
            notes,
        };
        Post::parse(post).unwrap().render(doc, &file).unwrap()
    };
    let post = BASIC.replace(
        "begin HEADER\n",
        "begin HEADER\n\"(top)\"\n\"([FILE_NOTES])\"\n",
    );
    let text = render(&post, &doc, &lines);
    assert!(!text.contains("Job notes"), "off unless asked: {text}");
    // [FILE_NOTES] is one line, and the job's own notes are made safe when none come
    assert!(text.contains("(Setup / - clamp [corners])"), "{text}");
    let text = render(&post, &doc, &[]);
    assert!(
        text.contains("(Setup / - clamp [corners] / G0 Z-50)"),
        "{text}"
    );
    doc.notes_in_gcode = true;
    let text = render(&post, &doc, &lines);
    let at: Vec<&str> = text.lines().collect();
    let top = at.iter().position(|l| *l == "(top)").unwrap();
    let notes = at.iter().position(|l| *l == "(Job notes:)").unwrap();
    assert!(notes > top, "under the header's comments: {text}");
    assert!(
        at[..notes].iter().all(|l| l.starts_with('(')),
        "before any command: {text}"
    );
    assert_eq!(at[notes + 1..notes + 3], ["(Setup)", "(- clamp [corners])"]);
    // a post whose header comments with ; gets ; comments
    let post = "POST_NAME = \"Semi\"\nFILE_EXTENSION = \"gcode\"\nUNITS = \"MM\"\nLINE_ENDING = \"[13][10]\"\nbegin HEADER\n\"; top\"\nbegin RAPID_MOVE\n\"G0 [X] [Y] [Z]\"\nbegin FEED_MOVE\n\"G1 [X] [Y] [Z] [F]\"\n";
    let text = render(post, &doc, &lines);
    assert!(text.contains("; Job notes:\r\n; Setup\r\n"), "{text}");
}

#[test]
fn a_tool_changer_numbers_tools_by_their_slot_and_a_manual_bit_0() {
    let post = BASIC.replace("begin TOOLCHANGE\n", "begin TOOLCHANGE\n\"M6 T[T]\"\n");
    let mut doc = Document::default();
    let mut c = Change::builder(&doc);
    for t in [toolpath("A", "6 mm"), toolpath("B", "3 mm")] {
        c.add_toolpath(t);
    }
    doc.apply(&c.finish()).unwrap();
    let slots = [None, Some("3 mm".to_owned()), None, Some("6 mm".to_owned())];
    let file = |slots| File {
        path: std::path::Path::new("/out.nc"),
        job: None,
        made: chrono::NaiveDateTime::default(),
        slots,
        notes: &[],
    };
    let post = Post::parse(&post).unwrap();
    let text = post.render(&doc, &file(&slots)).unwrap();
    assert!(text.contains("M6 T2"), "3 mm is in slot 2: {text}");
    let unloaded = [Some("6 mm".to_owned())];
    let text = post.render(&doc, &file(&unloaded)).unwrap();
    assert!(text.contains("M6 T0"), "3 mm is changed by hand: {text}");
    // no tool changer: numbered in the order used
    assert!(post.render(&doc, &file(&[])).unwrap().contains("M6 T2"));
}

#[test]
fn every_made_variant_writes_its_units_tool_changes_arcs_and_wrapping() {
    use cut_craft_core::document::{Along, Job};
    let mut names: Vec<String> = BUILT_IN
        .iter()
        .map(|(_, text)| Post::parse(text).unwrap().name)
        .collect();
    // the two hand-written FluidNC defaults have tests of their own
    let made = BUILT_IN
        .iter()
        .filter(|(f, _)| !["fluidnc_atc_mm.pp", "fluidnc_atc_wrap_a_mm.pp"].contains(f));
    for (file, text) in made {
        // the test renderer splits on the CR LF line ending
        let post = text.replace(r#"LINE_ENDING = "[10]""#, r#"LINE_ENDING = "[13][10]""#);
        let along = if file.contains("wrap_y") {
            Some(Along::X)
        } else if file.contains("wrap_x") {
            Some(Along::Y)
        } else {
            None
        };
        let two = vec![toolpath("A", "6 mm"), toolpath("B", "3 mm")];
        let job = along.map_or_else(Job::default, |a| Job::rotary(300.0, 50.0, a));
        let lines = render_for(&post, job, two);
        let any = |s: &str| lines.iter().any(|l| l.contains(s));
        let has = |s: &str| lines.iter().any(|l| l == s);
        assert_eq!(any("G20"), file.contains("inch"), "{file}");
        if file.starts_with("grbl") {
            assert!(has("M0") && !any("M6"), "{file}: {lines:?}");
        } else if file.starts_with("mach3") {
            assert!(has("T2 M6") && !any("TT"), "{file}: {lines:?}");
        } else {
            assert!(has("M6 T2"), "{file}: {lines:?}");
        }
        assert_eq!(has("G91.1"), file.starts_with("mach3_arcs"), "{file}");
        if along.is_some() {
            assert!(any(" A"), "{file} turns A: {lines:?}");
        }
        if file.contains("g93") {
            let feeds = lines.iter().filter(|l| l.starts_with("G1"));
            assert!(feeds.clone().all(|l| l.contains(" F")), "{file}: {lines:?}");
            assert!(has("G93") && has("G94"), "{file}");
        }
    }
    names.sort();
    names.dedup();
    assert_eq!(names.len(), BUILT_IN.len(), "each has its own name");
}

#[test]
fn tools_used_and_toolpaths_output_list_one_line_each_in_order_of_use() {
    let post = BASIC.replace(
        "begin HEADER\n",
        "begin HEADER\n\"(tool [TOOLS_USED])\"\n\"(path [TOOLPATHS_OUTPUT])\"\n",
    );
    let toolpaths = vec![
        toolpath("A", "6 mm"),
        toolpath("B", "3 mm"),
        toolpath("C", "6 mm"),
    ];
    let lines = render(&post, toolpaths);
    assert_eq!(
        lines[..5],
        [
            "(tool 6 mm)",
            "(tool 3 mm)",
            "(path A)",
            "(path B)",
            "(path C)"
        ]
    );
}

#[test]
fn posts_without_moves_are_rejected() {
    let post = BASIC.replace("begin FEED_MOVE", "begin SOMETHING_ELSE");
    assert_eq!(
        Post::parse(&post).unwrap_err(),
        "the post has no FEED_MOVE block"
    );
}

/// A five-drill bank toolpath of two plunges: holes 0..5 at x = 0, 32... 128, then
/// holes 5 and 6 at x = 160 and 192, each drilled straight down 10 mm.
fn bank_toolpath() -> Toolpath {
    use cut_craft_core::drill_bank::{Arm, BankHit, BankToolpath, DrillBank, DrillId};
    let bank = DrillBank::new(5, 0, 32.0, 5.0);
    let mut moves = vec![];
    for i in 0..7_i32 {
        let x = 32.0 * f64::from(i);
        moves.push(mv(MoveKind::Rapid, x, 0.0, 5.0));
        moves.push(mv(MoveKind::Plunge, x, 0.0, -10.0));
        moves.push(mv(MoveKind::Rapid, x, 0.0, 5.0));
    }
    let h = |index| DrillId {
        arm: Arm::Horizontal,
        index,
    };
    let hits = vec![
        BankHit {
            start: 0,
            end: 15,
            head: cut_craft_core::geometry::Point::new(64.0, 0.0),
            drills: (0..5).map(h).collect(),
        },
        BankHit {
            start: 15,
            end: 21,
            // the reference (middle) drill two pitches before the first hole
            head: cut_craft_core::geometry::Point::new(224.0, 0.0),
            drills: vec![h(0), h(1)],
        },
    ];
    Toolpath {
        strategy: "drill_bank_toolpath".into(),
        moves,
        drill_bank: Some(BankToolpath { bank, hits }),
        fallbacks: vec![],
        across: vec![],
        ..toolpath("Shelf Pins", "5 mm Drill")
    }
}

const PATTERN: &str = "VAR DRILL_LIST = [DL|D|,| ]\n\
         begin SETUP_DRILL_BANK\n\"(bank on)\"\n\
         begin DRILL_BANK_PATTERN\n\"M100 P[DB] (X[DBX] Y[DBY]) [DL]\"\n\
         begin FINAL_DRILL_BANK_ACTION\n\"(bank off)\"\n\
         begin HEADER";

#[test]
fn a_drill_bank_plunges_once_for_each_pattern_of_drills() {
    let post = BASIC.replace("begin HEADER", PATTERN);
    let lines = render(&post, vec![bank_toolpath()]);
    let at = |text: &str| {
        lines
            .iter()
            .position(|l| l == text)
            .unwrap_or_else(|| panic!("{text} in {lines:?}"))
    };
    let on = at("(bank on)");
    let first = at("M100 P31 (X31 Y0) D1 D2,D4,D8,D16");
    let second = at("M100 P3 (X3 Y0) D1 D2");
    assert!(on < first && first < second && second < at("(bank off)"));
    // the head goes where its reference drill is: one plunge a hit, not a hole
    let plunges: Vec<&String> = lines.iter().filter(|l| l.contains("Z-10.000")).collect();
    assert_eq!(plunges.len(), 2, "{lines:?}");
    assert!(
        lines[first + 1].starts_with("G0 X64.000 Y0.000"),
        "{lines:?}"
    );
    assert!(
        lines[second + 1].starts_with("G0 X224.000 Y0.000"),
        "{lines:?}"
    );
}

#[test]
fn a_way_round_a_keep_out_zone_before_a_plunge_still_plunges_once() {
    let mut t = bank_toolpath();
    // a waypoint leading into the second hit, as keep-out routing puts there
    t.moves.insert(15, mv(MoveKind::Rapid, 150.0, 40.0, 5.0));
    let bank = t.drill_bank.as_mut().unwrap();
    bank.hits[1].end += 1;
    let post = BASIC.replace("begin HEADER", PATTERN);
    let lines = render(&post, vec![t]);
    let plunges = lines.iter().filter(|l| l.contains("Z-10.000")).count();
    assert_eq!(plunges, 2, "{lines:?}");
}

#[test]
fn a_per_drill_post_extends_and_retracts_only_the_drills_that_change() {
    let post = BASIC.replace(
        "begin HEADER",
        "begin SETUP_DRILL_BANK\n\"(bank on)\"\n\
             begin DRILL_BANK_EXTEND_DRILL\n\"M61 P[CD]\"\n\
             begin DRILL_BANK_RETRACT_DRILL\n\"M62 P[CD]\"\n\
             begin FINAL_DRILL_BANK_ACTION\n\"(bank off)\"\n\
             begin HEADER",
    );
    let lines = render(&post, vec![bank_toolpath()]);
    let bank: Vec<&str> = lines
        .iter()
        .map(String::as_str)
        .filter(|l| l.starts_with("M6") || l.starts_with("(bank"))
        .collect();
    assert_eq!(
        bank,
        [
            "(bank on)",
            "M61 P1",
            "M61 P2",
            "M61 P4",
            "M61 P8",
            "M61 P16",
            // the second plunge keeps 1 and 2 down
            "M62 P4",
            "M62 P8",
            "M62 P16",
            // all up again before leaving
            "M62 P1",
            "M62 P2",
            "(bank off)",
        ]
    );
}

#[test]
fn a_post_without_drill_bank_blocks_refuses_drill_bank_toolpaths() {
    let mut doc = Document::default();
    let mut c = Change::builder(&doc);
    c.add_toolpath(bank_toolpath());
    doc.apply(&c.finish()).unwrap();
    let file = File {
        path: Path::new("x.nc"),
        job: None,
        made: chrono::NaiveDateTime::default(),
        slots: &[],
        notes: &[],
    };
    let why = Post::parse(BASIC).unwrap().render(&doc, &file).unwrap_err();
    assert!(
        why.starts_with("Shelf Pins uses a drill bank") && why.contains("SETUP_DRILL_BANK"),
        "{why}"
    );
    // both modes at once cannot be told apart
    let both = BASIC.replace(
        "begin HEADER",
        &format!(
            "{PATTERN}\nbegin DRILL_BANK_EXTEND_DRILL\n\"M61\"\n\
                 begin DRILL_BANK_RETRACT_DRILL\n\"M62\"\nbegin HEADER"
        ),
    );
    let why = Post::parse(&both).unwrap().render(&doc, &file).unwrap_err();
    assert!(why.contains("not both"), "{why}");
}
