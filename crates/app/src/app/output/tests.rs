//! Toolpath output through its own interface: a job, a machine and a form in, the post,
//! the plan, the files and the verdict out. No window.

use super::*;
use crate::files::{SaveMode, ToolChanges};
use cut_craft_core::Change;
use cut_craft_core::preflight::{Check, Finding, Severity};
use cut_craft_core::toolpath::{Move, MoveKind, Tool, Toolpath};
use std::collections::BTreeSet;

/// A job with three shown toolpaths (End Mill, End Mill, V-Bit), each with a cut.
fn job() -> (Document, Vec<EntityId>) {
    let mut doc = Document::default();
    let mut c = Change::builder(&doc);
    let ids = [
        ("Pocket", "End Mill", 20.0),
        ("Drill", "End Mill", 30.0),
        ("Text", "V-Bit", 40.0),
    ]
    .map(|(name, tool, end)| {
        c.add_toolpath(Toolpath {
            name: name.into(),
            strategy: "profile_toolpath".into(),
            params: serde_json::json!({}),
            tool: Tool {
                name: tool.into(),
                diameter: 6.0,
                ..Default::default()
            },
            vectors: vec![],
            // down at (10, 10), then a cut across to its own end (Pocket
            // X20, Drill X30, Text X40), so each file shows whose cuts it holds
            moves: [
                (MoveKind::Rapid, 10.0, 5.0),
                (MoveKind::Plunge, 10.0, -1.0),
                (MoveKind::Cut, end, -1.0),
            ]
            .map(|(kind, x, z)| Move {
                kind,
                to: cut_craft_core::glam::DVec3::new(x, 10.0, z),
            })
            .to_vec(),
            visible: true,
            power: vec![],
            stale: false,
            group: None,
            drill_bank: None,
            fallbacks: vec![],
            across: vec![],
        })
    });
    doc.apply(&c.finish()).unwrap();
    (doc, ids.to_vec())
}

fn form(mode: SaveMode, changes: ToolChanges) -> SaveForm {
    SaveForm {
        mode,
        group: false,
        post: None,
        add_side: false,
        changes,
        job_sheet: false,
    }
}

struct Fixture {
    doc: Document,
    machine: Option<Machine>,
    posts: Library,
    tolerances: crate::options::ToolpathSettings,
    selected: Option<EntityId>,
    job_path: Option<PathBuf>,
}

impl Fixture {
    fn new() -> (Self, Vec<EntityId>) {
        let (doc, ids) = job();
        (
            Self {
                doc,
                machine: None,
                posts: Library::new(None),
                tolerances: crate::options::Options::default().toolpaths,
                selected: None,
                job_path: None,
            },
            ids,
        )
    }

    fn output(&self) -> Output<'_> {
        Output {
            doc: &self.doc,
            machine: self.machine.as_ref(),
            posts: &self.posts,
            selected_toolpath: self.selected,
            selected_group: None,
            job_path: self.job_path.as_deref(),
            tolerances: &self.tolerances,
        }
    }
}

fn finding(severity: Severity, message: &str) -> Finding {
    Finding {
        severity,
        message: message.into(),
        check: Check::UpToDate,
    }
}

#[test]
fn without_a_machine_the_job_saves_through_cut_crafts_own_post() {
    let (f, _) = Fixture::new();
    let post = f.output().post(&form(
        SaveMode::VisibleToOneFile,
        ToolChanges::SeparateFiles,
    ));
    assert_eq!(
        post.unwrap().name,
        cut_craft_post::Post::default_for(&f.doc.job).name
    );
}

#[test]
fn each_tool_gets_its_own_file_unless_the_tool_changer_changes_them() {
    let (mut f, ids) = Fixture::new();
    let separate = form(SaveMode::VisibleToOneFile, ToolChanges::SeparateFiles);
    let post = f.output().post(&separate).unwrap();
    // the two End Mill toolpaths in a row share a file, the V-Bit gets its own
    assert_eq!(
        f.output().plan(&separate, &post).unwrap(),
        vec![vec![ids[0], ids[1]], vec![ids[2]]]
    );
    assert!(split_for(&separate, &post));
    // a tool changer that can change tools puts them all in one file
    f.machine = Some(Machine {
        posts: vec!["PowerWhittle FluidNC ATC (mm) (*.nc)".into()],
        slots: vec![Some("End Mill".into()), Some("V-Bit".into())],
        firmware: Some(cut_craft_machines::Firmware::FluidNc),
        ..Machine::default()
    });
    let changer = form(SaveMode::VisibleToOneFile, ToolChanges::ToolChanger);
    let post = f.output().post(&changer).unwrap();
    assert!(post.changes_tools(), "{}", post.name);
    assert_eq!(f.output().plan(&changer, &post).unwrap(), vec![ids.clone()]);
    assert!(!split_for(&changer, &post));
}

#[test]
fn the_tool_changer_chosen_without_one_set_up_is_warned_about() {
    let (f, _) = Fixture::new();
    assert_eq!(
        f.output().no_tool_changer_for(ToolChanges::ToolChanger),
        Some(no_tool_changer())
    );
    assert_eq!(f.output().no_tool_changer_for(ToolChanges::Pause), None);
}

#[test]
fn files_are_named_after_the_project_or_the_selected_toolpath() {
    let (mut f, ids) = Fixture::new();
    let visible = form(SaveMode::VisibleToOneFile, ToolChanges::SeparateFiles);
    let post = f.output().post(&visible).unwrap();
    assert_eq!(f.output().output_name(&visible), "Untitled");
    f.job_path = Some(PathBuf::from("signs/Front gate.ccjob"));
    assert_eq!(
        f.output().file_name(&visible, &post),
        format!("Front gate.{}", post.extension)
    );
    f.selected = Some(ids[2]);
    let selected = form(SaveMode::Selected, ToolChanges::SeparateFiles);
    assert_eq!(f.output().output_name(&selected), "Text");
}

#[test]
fn the_files_hold_the_g_code_of_each_part_of_the_plan() {
    let (f, _) = Fixture::new();
    let visible = form(SaveMode::VisibleToOneFile, ToolChanges::SeparateFiles);
    let post = f.output().post(&visible).unwrap();
    let made = chrono::NaiveDate::from_ymd_opt(2026, 11, 23)
        .unwrap()
        .and_hms_opt(9, 0, 0)
        .unwrap();
    let path = PathBuf::from("sign.nc");
    let texts = f.output().texts(&visible, &post, &path, made, &[]).unwrap();
    let names: Vec<String> = texts.iter().map(|(p, _)| p.display().to_string()).collect();
    // one file for each tool: the End Mill's and the V-Bit's
    let count = |tool: &str| names.iter().filter(|n| n.contains(tool)).count();
    assert_eq!(
        (names.len(), count("End"), count("V-Bit")),
        (2, 1, 1),
        "{names:?}"
    );
    // each file holds the cuts of its own toolpaths and no others
    let cuts_to = |text: &str, x: &str| {
        text.lines()
            .any(|l| l.starts_with("G1") && l.split_whitespace().any(|w| w == format!("{x}.000")))
    };
    for (path, text) in &texts {
        let shown = path.display().to_string();
        let ends: Vec<&str> = ["X20", "X30", "X40"]
            .into_iter()
            .filter(|x| cuts_to(text, x))
            .collect();
        let expected: &[&str] = if shown.contains("V-Bit") {
            &["X40"]
        } else {
            &["X20", "X30"]
        };
        assert_eq!(ends, expected, "{shown}: {text}");
    }
}

#[test]
fn errors_are_asked_about_every_time_each_ticked_on_its_own() {
    let found = [
        finding(Severity::Error, "Pocket has no feed rate"),
        finding(Severity::Warning, "Text is out of date"),
        finding(Severity::Error, "Drill would crash"),
    ];
    let problems: Vec<String> = found.iter().map(|f| f.message.clone()).collect();
    let errors = vec![
        "Pocket has no feed rate".to_owned(),
        "Drill would crash".to_owned(),
    ];
    let asking = Asking {
        problems: problems.clone(),
        errors: errors.clone(),
        wont_run: vec![],
    };
    // nothing is refused: asked, the errors among the problems
    assert_eq!(
        verdict(&found, &BTreeSet::new(), &[]),
        Verdict::Ask(asking.clone())
    );
    // ticked as read is not enough for an error: every save asks again
    let ticked: BTreeSet<String> = problems.iter().cloned().collect();
    assert_eq!(verdict(&found, &ticked, &[]), Verdict::Ask(asking));
    // Save Anyway answered for each one shown
    assert_eq!(verdict(&found, &ticked, &problems), Verdict::Go);
    assert_eq!(super::errors(&found), errors);
}

#[test]
fn a_job_that_will_not_run_says_so() {
    let found = [
        Finding {
            severity: Severity::Error,
            message: "Pocket has no feed rate".into(),
            check: Check::CanCut,
        },
        finding(Severity::Error, "Drill would crash"),
    ];
    let Verdict::Ask(asking) = verdict(&found, &BTreeSet::new(), &[]) else {
        panic!("errors are asked about");
    };
    assert_eq!(asking.wont_run, ["Pocket has no feed rate"]);
}

#[test]
fn files_saved_past_errors_say_so_at_the_top() {
    let errors = ["Drill would crash into Clamp 1".to_owned()];
    let semicolons = vec![(
        PathBuf::from("a.nc"),
        "; Made by PowerWhittle\n; Job: sign\nG21\nG0 X0 Y0\nM30\n".to_owned(),
    )];
    let marked_text = &marked(semicolons.clone(), &errors, None)[0].1;
    let lines: Vec<&str> = marked_text.lines().collect();
    assert_eq!(lines[..2], ["; Made by PowerWhittle", "; Job: sign"]);
    assert!(lines[2].starts_with("; SAVED WITH ERRORS"), "{marked_text}");
    assert_eq!(lines[3], "; Drill would crash into Clamp 1");
    assert_eq!(lines[4], "G21", "before the first command");
    // brackets where the post comments in brackets
    let brackets = vec![(
        PathBuf::from("a.nc"),
        "(Made by PowerWhittle)\nG21\n".to_owned(),
    )];
    let text = &marked(brackets, &errors, None)[0].1;
    assert!(
        text.lines()
            .nth(1)
            .unwrap()
            .starts_with("(SAVED WITH ERRORS")
    );
    // a post ending its lines with CR alone keeps them so
    let cr = vec![(PathBuf::from("a.nc"), "; Header\rG21\rM30\r".to_owned())];
    let text = &marked(cr, &errors, None)[0].1;
    assert!(!text.contains('\n'), "{text:?}");
    let lines: Vec<&str> = text.split('\r').collect();
    assert!(lines[1].starts_with("; SAVED WITH ERRORS"), "{text:?}");
    assert_eq!(lines[3], "G21", "{text:?}");
    // nothing said when saved without errors
    assert_eq!(marked(semicolons.clone(), &[], None), semicolons);
}

#[test]
fn a_toolpath_with_no_moves_is_left_out_and_said() {
    let (mut f, ids) = Fixture::new();
    f.doc.toolpaths.get_mut(&ids[2]).unwrap().moves.clear();
    let visible = form(SaveMode::VisibleToOneFile, ToolChanges::SeparateFiles);
    let post = f.output().post(&visible).unwrap();
    assert_eq!(f.output().left_out(&visible, &post), ["Text"]);
    let made = chrono::NaiveDate::from_ymd_opt(2026, 11, 23)
        .unwrap()
        .and_hms_opt(9, 0, 0)
        .unwrap();
    let texts = (f.output())
        .texts(&visible, &post, Path::new("sign.nc"), made, &[])
        .unwrap();
    assert_eq!(texts.len(), 1, "the V-Bit's file had nothing in it");
    assert!(!texts[0].1.contains("Text"));
    // nothing left at all: no files, and why, so an earlier save is kept
    for id in &ids[..2] {
        f.doc.toolpaths.get_mut(id).unwrap().moves.clear();
    }
    let why = (f.output())
        .texts(&visible, &post, Path::new("sign.nc"), made, &[])
        .unwrap_err();
    assert!(
        why.starts_with("Nothing to save: Pocket, Drill, Text have no moves"),
        "{why}"
    );
    assert_eq!(
        with_left_out("Saved sign.nc".into(), &["Text".to_owned()]),
        "Saved sign.nc. Left out, as it has no moves to cut: Text"
    );
}

#[test]
fn a_tiled_job_with_nothing_in_any_tile_saves_nothing() {
    let (mut f, ids) = Fixture::new();
    f.doc.tiling = Some(cut_craft_core::tiling::Tiling::default());
    // one move each: something in the plan, but no cut for any tile to hold
    for id in &ids {
        f.doc.toolpaths.get_mut(id).unwrap().moves.truncate(1);
    }
    let visible = form(SaveMode::VisibleToOneFile, ToolChanges::SeparateFiles);
    let post = f.output().post(&visible).unwrap();
    let made = chrono::NaiveDate::from_ymd_opt(2026, 11, 23)
        .unwrap()
        .and_hms_opt(9, 0, 0)
        .unwrap();
    let why = (f.output())
        .texts(&visible, &post, Path::new("sign.nc"), made, &[])
        .unwrap_err();
    assert!(why.starts_with("Nothing to save: no tile"), "{why}");
}

#[test]
fn warnings_are_asked_about_unless_ticked_or_answered() {
    let found = [
        finding(Severity::Warning, "Text is out of date"),
        finding(Severity::Warning, "Air cutting"),
    ];
    let problems: Vec<String> = found.iter().map(|f| f.message.clone()).collect();
    assert_eq!(
        verdict(&found, &BTreeSet::new(), &[]),
        Verdict::Ask(Asking {
            problems: problems.clone(),
            ..Asking::default()
        })
    );
    // ticked as read in Save Toolpaths: that was the asking
    let ticked: BTreeSet<String> = problems.iter().cloned().collect();
    assert_eq!(verdict(&found, &ticked, &[]), Verdict::Go);
    // one ticked is not enough
    let one: BTreeSet<String> = problems[..1].iter().cloned().collect();
    assert!(matches!(verdict(&found, &one, &[]), Verdict::Ask(_)));
    // Save Anyway answered for every problem shown
    assert_eq!(verdict(&found, &BTreeSet::new(), &problems), Verdict::Go);
    assert!(matches!(
        verdict(&found, &BTreeSet::new(), &problems[..1]),
        Verdict::Ask(_)
    ));
    assert_eq!(verdict(&[], &BTreeSet::new(), &[]), Verdict::Go);
}

#[test]
fn the_collet_hitting_the_material_is_asked_about_even_when_ticked() {
    let found = [finding(Severity::MustAsk, "The collet hits the material")];
    let ticked: BTreeSet<String> = ["The collet hits the material".to_owned()].into();
    assert!(matches!(verdict(&found, &ticked, &[]), Verdict::Ask(_)));
    let answered = ["The collet hits the material".to_owned()];
    assert_eq!(verdict(&found, &ticked, &answered), Verdict::Go);
}

#[test]
fn saving_while_tiling_is_on_says_so() {
    let path = Path::new("sign.nc");
    let tiles = [
        PathBuf::from("sign_tile1.nc"),
        PathBuf::from("sign_tile2.nc"),
    ];
    let said = saved_message(&tiles, path, true);
    assert!(said.contains("Toolpath Tiling is on"), "{said}");
    let one = saved_message(&tiles[..1], path, true);
    assert!(one.contains("Toolpath Tiling is on"), "{one}");
    let plain = saved_message(&[path.to_path_buf()], path, false);
    assert_eq!(plain, "Saved toolpaths to sign.nc");
}

/// A fresh folder of its own for one test.
fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("cut-craft-output-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn when() -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 11, 23)
        .unwrap()
        .and_hms_opt(9, 30, 0)
        .unwrap()
}

#[test]
fn replacing_an_earlier_save_leaves_only_the_new_files() {
    let dir = folder("replace");
    let path = dir.join("sign.nc");
    let old = dir.join("sign_tile2.nc");
    std::fs::write(&path, "old").unwrap();
    std::fs::write(&old, "old tile").unwrap();
    let earlier = files::earlier_save(&path);
    let texts = vec![(path.clone(), "new".to_owned())];
    let over = write_over(|| Ok(texts), &path, &earlier, false, when()).unwrap();
    assert_eq!(over.written, vec![path.clone()]);
    assert_eq!(over.archived, None);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
    assert!(!old.exists());
    let left: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().collect();
    assert_eq!(left.len(), 1, "the set-aside folder is gone too: {left:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn archiving_an_earlier_save_keeps_it_in_a_dated_folder() {
    let dir = folder("archive");
    let path = dir.join("sign.nc");
    std::fs::write(&path, "old").unwrap();
    let earlier = files::earlier_save(&path);
    let texts = vec![(path.clone(), "new".to_owned())];
    let over = write_over(|| Ok(texts), &path, &earlier, true, when()).unwrap();
    let archived = over.archived.expect("archived");
    assert!(
        archived.starts_with(dir.join("Archive")),
        "{}",
        archived.display()
    );
    assert_eq!(
        std::fs::read_to_string(archived.join("sign.nc")).unwrap(),
        "old"
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_failed_save_puts_the_earlier_files_back() {
    let dir = folder("put-back");
    let path = dir.join("sign.nc");
    std::fs::write(&path, "old").unwrap();
    let earlier = files::earlier_save(&path);
    let failed = write_over(
        || Err("Not saved: Pocket would cut into a keep-out zone".to_owned()),
        &path,
        &earlier,
        false,
        when(),
    )
    .unwrap_err();
    assert_eq!(
        failed,
        "Not saved: Pocket would cut into a keep-out zone; the earlier files were left as they were"
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "old");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_risk_of_damage_to_the_machine_is_refused_whatever_is_ticked_or_answered() {
    let damage = Finding {
        severity: Severity::Damage,
        message: "The toolpaths cover 800 mm × 600 mm, more than the bed".into(),
        check: Check::Bed,
    };
    let found = [
        damage.clone(),
        finding(Severity::Warning, "Text is out of date"),
    ];
    let everything: Vec<String> = found.iter().map(|f| f.message.clone()).collect();
    let ticked: BTreeSet<String> = everything.iter().cloned().collect();
    assert_eq!(
        verdict(&found, &ticked, &everything),
        Verdict::Refused(vec![damage.message.clone()]),
        "neither ticks nor Save Anyway get past it"
    );
    // it has no tick of its own: the rest ticked is all ticked
    let warning_ticked: BTreeSet<String> = ["Text is out of date".to_owned()].into();
    assert!(all_acknowledged(&found, &warning_ticked));
    assert_eq!(refused(&found), std::slice::from_ref(&damage.message));
    let said = refusal(&refused(&found));
    assert!(
        said.starts_with("Nothing saved: this could damage the machine. The toolpaths cover")
            && said
                .ends_with("switch the check off in Machine Configuration's Protect the Machine"),
        "{said}"
    );
}

#[test]
fn files_made_with_the_machines_protection_off_say_so_at_the_top() {
    let (mut f, _) = Fixture::new();
    assert_eq!(
        f.output().protection_off(),
        None,
        "no machine: nothing to say"
    );
    let mut machine = Machine {
        name: "Shapeoko".into(),
        ..Machine::default()
    };
    f.machine = Some(machine.clone());
    assert_eq!(f.output().protection_off(), None, "every check on");
    machine.protect.bed = false;
    machine.protect.spoilboard = false;
    f.machine = Some(machine);
    let off = f.output().protection_off().unwrap();
    assert_eq!(
        off,
        "PROTECTION OFF: the checks for bed size, spoilboard depth were switched off for \
         Shapeoko in Machine Configuration as this was made"
    );
    let texts = vec![(PathBuf::from("a.nc"), "(Header)\nG21\n".to_owned())];
    let text = &marked(texts, &[], Some(&off))[0].1;
    assert_eq!(text, &format!("(Header)\n({off})\nG21\n"));
}
