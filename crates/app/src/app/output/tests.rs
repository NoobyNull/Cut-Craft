//! Toolpath output through its own interface: a job, a machine and a form in, the post,
//! the plan, the files and the verdict out. No window.

use super::*;
use crate::files::{SaveMode, ToolChanges};
use cut_craft_core::Change;
use cut_craft_core::preflight::{Check, Finding, Severity};
use cut_craft_core::toolpath::{Tool, Toolpath};
use std::collections::BTreeSet;

/// A job with three shown toolpaths (End Mill, End Mill, V-Bit), each with a cut.
fn job() -> (Document, Vec<EntityId>) {
    let mut doc = Document::default();
    let mut c = Change::builder(&doc);
    let ids = [
        ("Pocket", "End Mill"),
        ("Drill", "End Mill"),
        ("Text", "V-Bit"),
    ]
    .map(|(name, tool)| {
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
            moves: vec![],
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
        posts: vec!["Cut Craft FluidNC ATC (mm) (*.nc)".into()],
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
    assert_eq!(names.len(), 2, "{names:?}");
    assert!(
        texts
            .iter()
            .all(|(_, text)| text.contains("M30") || !text.is_empty())
    );
}

#[test]
fn errors_refuse_and_say_each_fix() {
    let found = [
        finding(Severity::Error, "Pocket has no feed rate"),
        finding(Severity::Warning, "Text is out of date"),
        finding(Severity::Error, "Drill would crash"),
    ];
    assert_eq!(
        verdict(&found, &BTreeSet::new(), &[]),
        Verdict::Refused("Not saved: Pocket has no feed rate; Drill would crash".into())
    );
    assert_eq!(refused(&found[1..2]), None);
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
        Verdict::Ask(problems.clone())
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
