#[test]
fn an_earlier_save_is_the_file_and_the_numbered_tiled_and_sided_ones_beside_it() {
    let dir = std::env::temp_dir().join(format!("cut-craft-earlier-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let ours = [
        "sign.nc",
        "sign_1_Profile.nc",
        "sign_3_Old-Vcarve.nc",
        "sign_tile2.nc",
        "sign_Top.nc",
        "sign_2_Pocket_Bottom_tile1.nc",
    ];
    let others = [
        "sign_2024.nc",
        "signs.nc",
        "sign_1_Profile.tap",
        "sign v2.nc",
        "sign_notes.nc",
    ];
    for name in ours.iter().chain(&others) {
        std::fs::write(dir.join(name), "G0").unwrap();
    }
    let mut found: Vec<String> = super::earlier_save(&dir.join("sign.nc"))
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    found.sort();
    let mut want: Vec<String> = ours.iter().map(|s| (*s).to_owned()).collect();
    want.sort();
    assert_eq!(found, want);
    // archived: moved into a dated folder under Archive, the others left
    let when = chrono::NaiveDate::from_ymd_opt(2026, 11, 15)
        .and_then(|d| d.and_hms_opt(7, 54, 22))
        .unwrap();
    let path = dir.join("sign.nc");
    let folder = super::archive(&super::earlier_save(&path), &path, when).unwrap();
    assert_eq!(folder, dir.join("Archive").join("sign 2026-11-15 075422"));
    for name in ours {
        assert!(
            !dir.join(name).exists() && folder.join(name).exists(),
            "{name}"
        );
    }
    for name in others {
        assert!(dir.join(name).exists(), "{name}");
    }
    assert_eq!(
        super::earlier_save(&dir.join("sign.nc")),
        Vec::<std::path::PathBuf>::new()
    );
    // archived again in the same second: a folder of its own, the first one kept
    std::fs::write(&path, "second").unwrap();
    let again = super::archive(&super::earlier_save(&path), &path, when).unwrap();
    assert_eq!(
        again,
        dir.join("Archive").join("sign 2026-11-15 075422 (2)")
    );
    assert_eq!(
        std::fs::read_to_string(folder.join("sign.nc")).unwrap(),
        "G0"
    );
    assert_eq!(
        std::fs::read_to_string(again.join("sign.nc")).unwrap(),
        "second"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_extension_goes_on_the_end_of_a_typed_name_with_a_dot() {
    let added = |typed: &str| super::with_extension_added(std::path::Path::new(typed), "nc");
    assert_eq!(added("sign v1.5"), std::path::Path::new("sign v1.5.nc"));
    assert_eq!(added("sign.nc"), std::path::Path::new("sign.nc"));
    assert_eq!(added("sign.NC"), std::path::Path::new("sign.NC"));
    assert_eq!(added("sign"), std::path::Path::new("sign.nc"));
}

#[test]
fn a_saved_name_keeps_its_dots_and_asks_before_replacing_a_file_it_was_not_given() {
    let named =
        |typed: &str| super::named_for_saving(std::path::Path::new(typed), &["ccjob", "ajob"]);
    assert_eq!(
        named("Sign 2.5in walnut"),
        std::path::Path::new("Sign 2.5in walnut.ccjob")
    );
    assert_eq!(
        named("plaque v1.2"),
        std::path::Path::new("plaque v1.2.ccjob")
    );
    assert_eq!(named("old.ajob"), std::path::Path::new("old.ajob"));
    assert_eq!(named("sign.CCJOB"), std::path::Path::new("sign.CCJOB"));
    let dir = std::env::temp_dir().join(format!("cut-craft-named-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("sign.ccjob"), "kept").unwrap();
    // the dialog asked about the name typed, not the one the extension made
    assert!(super::replaces_unasked(
        &dir.join("sign"),
        &dir.join("sign.ccjob")
    ));
    assert!(!super::replaces_unasked(
        &dir.join("sign.ccjob"),
        &dir.join("sign.ccjob")
    ));
    assert!(!super::replaces_unasked(
        &dir.join("new"),
        &dir.join("new.ccjob")
    ));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn form_settings_load_back_into_the_same_form_only() {
    let saved = serde_json::json!({ "max_cut_depth": 2.5, "gone": 1 });
    let bytes = super::form_settings("texture_toolpath", "Texture Toolpath", &saved);
    let mut params = serde_json::json!({ "max_cut_depth": 1.5, "seed": 3 });
    super::load_form_settings("texture_toolpath", "Texture Toolpath", &mut params, &bytes).unwrap();
    // only the settings the form has, so a file from an older version still loads
    assert_eq!(
        params,
        serde_json::json!({ "max_cut_depth": 2.5, "seed": 3 })
    );
    let wrong =
        super::load_form_settings("profile_toolpath", "Profile Toolpath", &mut params, &bytes)
            .unwrap_err();
    assert!(wrong.contains("Texture Toolpath"), "{wrong}");
    assert!(wrong.contains("Profile Toolpath"), "{wrong}");
    assert!(
        super::load_form_settings("texture_toolpath", "Texture Toolpath", &mut params, b"{")
            .is_err()
    );
}

#[test]
fn settings_added_inside_a_group_since_the_file_was_saved_keep_their_defaults() {
    let saved = serde_json::json!({ "tab": { "number": 6 } });
    let bytes = super::form_settings("profile_toolpath", "Profile Toolpath", &saved);
    let mut params = serde_json::json!({ "tab": { "number": 4, "minimum": 2, "maximum": 9 } });
    super::load_form_settings("profile_toolpath", "Profile Toolpath", &mut params, &bytes).unwrap();
    assert_eq!(
        params,
        serde_json::json!({ "tab": { "number": 6, "minimum": 2, "maximum": 9 } })
    );
}

#[test]
fn toolpath_templates_keep_settings_and_drop_moves() {
    let mut doc = cut_craft_core::testing::with_toolpath();
    let id = *doc.toolpaths.keys().next().unwrap();
    doc.toolpaths
        .get_mut(&id)
        .unwrap()
        .moves
        .push(cut_craft_core::toolpath::Move {
            kind: cut_craft_core::toolpath::MoveKind::Cut,
            to: cut_craft_core::glam::DVec3::ZERO,
        });
    let bytes = super::toolpath_template(&doc, &[id]);
    let change = super::load_toolpath_template(&doc, &bytes).unwrap();
    doc.apply(&change).unwrap();
    assert_eq!(doc.toolpaths.len(), 2);
    let loaded = doc.toolpaths.values().last().unwrap();
    assert_eq!(loaded.name, "Profile");
    assert_eq!(loaded.moves, vec![]);
    assert!(super::load_toolpath_template(&doc, b"[]").is_err());
}

#[test]
fn a_template_s_merged_toolpath_merges_the_template_s_own_toolpaths() {
    use cut_craft_core::document::EntityId;
    let last = |doc: &Document, n: usize| -> Vec<EntityId> {
        let ids: Vec<EntityId> = doc.toolpaths.keys().copied().collect();
        ids[ids.len() - n..].to_vec()
    };
    let mut doc = cut_craft_core::testing::three_toolpaths();
    for t in doc.toolpaths.values_mut() {
        t.params = serde_json::json!({ "name": t.name });
    }
    let [a, b, m] = last(&doc, 3)[..] else {
        unreachable!()
    };
    doc.toolpaths.get_mut(&m).unwrap().params["sources"] = serde_json::json!([a.0, b.0]);
    // all three: the merged one names the two saved with it, at their new ids
    let bytes = super::toolpath_template(&doc, &[a, b, m]);
    let change = super::load_toolpath_template(&doc, &bytes).unwrap();
    doc.apply(&change).unwrap();
    let [a2, b2, m2] = last(&doc, 3)[..] else {
        unreachable!()
    };
    assert_eq!(
        doc.toolpaths[&m2].params["sources"],
        serde_json::json!([a2.0, b2.0])
    );
    assert!(doc.toolpaths[&m2].params.get("template_id").is_none());
    // the merged one alone: what it merged is not in the template, so it names nothing
    // of this job, and says so
    let bytes = super::toolpath_template(&doc, &[m]);
    let change = super::load_toolpath_template(&doc, &bytes).unwrap();
    assert!(
        change.notes.iter().any(|n| n.contains("tick")),
        "{:?}",
        change.notes
    );
    doc.apply(&change).unwrap();
    let loaded = doc.toolpaths.values().last().unwrap();
    assert_eq!(loaded.params["sources"], serde_json::json!([]));
    // an Array Copy's one source likewise
    let mut doc = cut_craft_core::testing::three_toolpaths();
    for t in doc.toolpaths.values_mut() {
        t.params = serde_json::json!({ "name": t.name });
    }
    let [a, _, m] = last(&doc, 3)[..] else {
        unreachable!()
    };
    doc.toolpaths.get_mut(&m).unwrap().params["only"] = serde_json::json!(a.0);
    let bytes = super::toolpath_template(&doc, &[a, m]);
    doc.apply(&super::load_toolpath_template(&doc, &bytes).unwrap())
        .unwrap();
    let [a2, m2] = last(&doc, 2)[..] else {
        unreachable!()
    };
    assert_eq!(doc.toolpaths[&m2].params["only"], serde_json::json!(a2.0));
}

#[test]
fn templates_and_saved_settings_leave_out_the_picture_a_toolpath_was_made_from() {
    // a Photo V-Carve made from picture 7 of its job
    let mut doc = cut_craft_core::testing::with_toolpath();
    let id = *doc.toolpaths.keys().next().unwrap();
    doc.toolpaths.get_mut(&id).unwrap().params["bitmap"] = serde_json::json!(7);
    let bytes = super::toolpath_template(&doc, &[id]);
    let change = super::load_toolpath_template(&doc, &bytes).unwrap();
    doc.apply(&change).unwrap();
    let loaded = doc.toolpaths.values().last().unwrap();
    assert_eq!(
        loaded.params.get("bitmap"),
        None,
        "another job carves its own picture"
    );
    // settings saved from its Edit form, loaded into a new form
    let saved = super::form_settings(
        "photo_vcarve_toolpath",
        "Photo V-Carve",
        &serde_json::json!({ "name": "Face", "bitmap": 7 }),
    );
    let mut params = serde_json::json!({ "name": "Photo V-Carve", "bitmap": null });
    super::load_form_settings(
        "photo_vcarve_toolpath",
        "Photo V-Carve",
        &mut params,
        &saved,
    )
    .unwrap();
    assert_eq!(params["name"], "Face");
    assert_eq!(
        params["bitmap"],
        serde_json::Value::Null,
        "the form's own picture"
    );
}

/// Four shown toolpaths (End Mill, End Mill, V-Bit, End Mill) and a hidden one.
fn four_toolpaths() -> (Document, Vec<EntityId>) {
    let mut doc = Document::default();
    let mut c = Change::builder(&doc);
    let ids = [
        ("Pocket", "End Mill"),
        ("Drill", "End Mill"),
        ("Text", "V-Bit"),
        ("Cut Out", "End Mill"),
        ("Hidden", "V-Bit"),
    ]
    .map(|(name, tool)| {
        c.add_toolpath(cut_craft_core::toolpath::Toolpath {
            name: name.into(),
            strategy: "profile_toolpath".into(),
            params: serde_json::json!({}),
            tool: cut_craft_core::toolpath::Tool {
                name: tool.into(),
                ..Default::default()
            },
            vectors: vec![],
            moves: vec![],
            visible: name != "Hidden",
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

#[test]
fn save_plans_take_the_selected_or_shown_toolpaths_in_list_order() {
    let (doc, ids) = four_toolpaths();
    let plan = |mode, group| save_plan(&doc, mode, Some(ids[2]), None, group).unwrap();
    assert_eq!(plan(SaveMode::Selected, false), vec![vec![ids[2]]]);
    assert_eq!(
        plan(SaveMode::VisibleToOneFile, false),
        vec![ids[..4].to_vec()]
    );
    assert_eq!(
        plan(SaveMode::VisibleToMultipleFiles, false),
        ids[..4].iter().map(|&id| vec![id]).collect::<Vec<_>>()
    );
    // runs of one tool share a file; the order is kept, so the last end mill is alone
    assert_eq!(
        plan(SaveMode::VisibleToMultipleFiles, true),
        vec![vec![ids[0], ids[1]], vec![ids[2]], vec![ids[3]]]
    );
    assert!(save_plan(&doc, SaveMode::Selected, None, None, false).is_err());
    let mut hidden = doc.clone();
    hidden
        .toolpaths
        .values_mut()
        .for_each(|t| t.visible = false);
    assert!(save_plan(&hidden, SaveMode::VisibleToOneFile, None, None, false).is_err());
}

#[test]
fn a_post_without_tool_changes_gets_a_file_for_each_tool_in_turn() {
    let (doc, ids) = four_toolpaths();
    let one = save_plan(&doc, SaveMode::VisibleToOneFile, None, None, false).unwrap();
    // the order is kept: the last end mill runs after the V-bit, in a file of its own
    assert_eq!(
        by_tool(&doc, one),
        vec![vec![ids[0], ids[1]], vec![ids[2]], vec![ids[3]]]
    );
    let path = Path::new("/out/sign.nc");
    let job = cut_craft_core::document::Job::default();
    let plan = vec![vec![ids[0], ids[1]], vec![ids[2]]];
    let files = save_files(&Post::default_for(&job), &doc, &plan, path, None, true).unwrap();
    let names: Vec<_> = files.iter().map(|(p, _)| p.clone()).collect();
    assert_eq!(
        names,
        [
            path.with_file_name("sign_1_End_Mill.nc"),
            path.with_file_name("sign_2_V-Bit.nc")
        ]
    );
}

#[test]
fn saved_files_are_named_per_toolpath_and_need_a_tool_changing_post_for_two_tools() {
    let (doc, ids) = four_toolpaths();
    let job = cut_craft_core::document::Job::default();
    let changer = Post::default_for(&job);
    let path = Path::new("/out/widget.nc");
    let plan = vec![vec![ids[0], ids[1]], vec![ids[2]]];
    let files = save_files(&changer, &doc, &plan, path, Some("Top"), false).unwrap();
    // compared as paths: Windows joins the folder and name with a backslash
    let names: Vec<_> = files.iter().map(|(p, _)| p.clone()).collect();
    assert_eq!(
        names,
        [
            path.with_file_name("widget_1_Pocket_Top.nc"),
            path.with_file_name("widget_2_Text_Top.nc")
        ]
    );
    assert_eq!(files[0].1.toolpaths.len(), 2);
    assert!(files[1].1.toolpaths.contains_key(&ids[2]));
    let one = save_files(&changer, &doc, &[ids[..3].to_vec()], path, None, false).unwrap();
    assert_eq!(one[0].0, path);
    // a post without tool changes refuses a file mixing end mill and V-bit
    let no_changes = Post::parse(
        "POST_NAME = \"Plain (*.nc)\"\nFILE_EXTENSION = \"nc\"\nUNITS = \"MM\"\n\
             begin HEADER\n\"G21\"\nbegin RAPID_MOVE\n\"G0 [X] [Y] [Z]\"\n\
             begin FEED_MOVE\n\"G1 [X] [Y] [Z]\"\n",
    )
    .unwrap();
    let why = save_files(&no_changes, &doc, &[ids[..3].to_vec()], path, None, false).unwrap_err();
    assert!(why.contains("different tools"), "{why}");
    assert!(save_files(&no_changes, &doc, &[ids[..2].to_vec()], path, None, false).is_ok());
}

use super::*;
use cut_craft_core::testing::sample_document;
use cut_craft_core::toolpath::ToolDatabase;
use std::fs;

/// `name` in a temporary folder of its own, removed when this is dropped.
struct Temp {
    root: PathBuf,
    path: PathBuf,
}

impl std::ops::Deref for Temp {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.path
    }
}

impl AsRef<Path> for Temp {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn temp(name: &str) -> Temp {
    let folder = format!(
        "cut-craft-files-{}-{}",
        std::process::id(),
        name.replace('/', "-")
    );
    let root = std::env::temp_dir().join(folder);
    fs::create_dir_all(&root).unwrap();
    Temp {
        path: root.join(name),
        root,
    }
}

fn registry() -> Registry {
    crate::commands::registry()
}

#[test]
fn saving_then_opening_restores_the_job_with_a_clean_history() {
    let path = temp("round-trip.ccjob");
    let doc = sample_document();
    let mut session = Session::default();
    session.save(&doc, &History::default(), &path).unwrap();
    let (opened, history) = Session::default().open(&path).unwrap();
    assert_eq!(opened, doc);
    assert!(!history.can_undo());
}

#[test]
fn changes_make_the_session_dirty_until_saved() {
    let path = temp("dirty.ccjob");
    let mut doc = Document::default();
    let mut history = History::default();
    let mut session = Session::default();
    assert!(!session.is_dirty(&history));
    let change = Change::add_and_select(
        &doc,
        cut_craft_core::geometry::Path::circle(cut_craft_core::geometry::Point::ZERO, 1.0),
    );
    history.apply(&mut doc, change).unwrap();
    assert!(session.is_dirty(&history));
    assert_eq!(
        session.title("Cut Craft", &history),
        "Untitled* - Cut Craft"
    );
    let mut discarded = Session::default();
    discarded.discard_changes(&history);
    assert!(
        !discarded.is_dirty(&history),
        "changes thrown away are not asked about again"
    );
    session.save(&doc, &history, &path).unwrap();
    assert!(!session.is_dirty(&history));
    assert_eq!(
        session.title("Cut Craft", &history),
        "dirty.ccjob - Cut Craft"
    );
}

#[test]
fn opening_something_that_is_not_a_job_explains_why() {
    let path = temp("not-a-job.ccjob");
    fs::write(&path, b"{}").unwrap();
    let err = Session::default().open(&path).unwrap_err();
    assert!(err.contains("not a Cut Craft job"), "{err}");
    assert!(
        Session::default()
            .open(&temp("missing.ccjob"))
            .unwrap_err()
            .contains("missing.ccjob")
    );
}

#[test]
fn importing_an_svg_is_an_undoable_change() {
    let path = temp("shape.svg");
    fs::write(&path, r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><circle cx="5" cy="5" r="4"/></svg>"#).unwrap();
    let mut doc = Document::default();
    let mut history = History::default();
    let change = import(&registry(), &doc, &path).unwrap();
    history.apply(&mut doc, change).unwrap();
    assert_eq!(doc.vectors.len(), 1);
    history.undo(&mut doc);
    assert!(doc.vectors.is_empty());
}

#[test]
fn unknown_import_types_are_named_in_the_error() {
    let path = temp("drawing.xyz");
    fs::write(&path, b"x").unwrap();
    assert_eq!(
        import(&registry(), &Document::default(), &path).unwrap_err(),
        "no importer for .xyz files"
    );
}

#[test]
fn export_writes_the_format() {
    let path = temp("out.svg");
    export(&registry(), "export_svg", &sample_document(), &path).unwrap();
    assert!(fs::read_to_string(&path).unwrap().contains("<svg"));
}

const POST: &str = "POST_NAME = \"Test post\"\nFILE_EXTENSION = \"tap\"\nVAR X_POSITION = [X|A|X|1.1]\nbegin RAPID_MOVE\n\"G0 [X]\"\nbegin FEED_MOVE\n\"G1 [X]\"\n";

#[test]
fn recent_files_put_the_latest_first_once_and_keep_four() {
    let mut recent = vec![];
    for name in ["a", "b", "c", "a", "d", "e"] {
        remember(&mut recent, Path::new(name), RECENT_COUNT);
    }
    assert_eq!(recent, ["e", "d", "a", "c"].map(PathBuf::from));
}

#[test]
fn recent_files_survive_a_restart_and_a_broken_list_is_empty() {
    let path = temp("recent/recent.json");
    let list = [PathBuf::from("/jobs/a.ccjob")];
    save_recent(&list, &path).unwrap();
    assert_eq!(load_recent(&path), list);
    fs::write(&path, "{").unwrap();
    assert_eq!(load_recent(&path), Vec::<PathBuf>::new());
    assert_eq!(load_recent(&temp("no-recent.json")), Vec::<PathBuf>::new());
    let config = recent_files_path().expect("a config folder");
    assert!(
        config.ends_with("cut-craft/recent.json"),
        "{}",
        config.display()
    );
}

#[test]
fn the_tool_database_lives_in_the_cut_craft_config_folder() {
    let path = tool_database_path().expect("a config folder");
    assert!(path.ends_with("cut-craft/tools.json"), "{}", path.display());
}

#[test]
fn a_missing_tool_database_starts_with_the_starter_tools() {
    assert_eq!(
        load_tools(&temp("none/tools.json")).unwrap(),
        ToolDatabase::starter()
    );
}

#[test]
fn saved_tools_load_back_into_a_new_folder() {
    let path = temp("new-folder/tools.json");
    let mut db = ToolDatabase::starter();
    db.tools.truncate(1);
    save_tools(&db, &path).unwrap();
    assert_eq!(load_tools(&path).unwrap(), db);
}

#[test]
fn a_bit_listed_per_material_is_joined_on_loading_after_a_backup() {
    use cut_craft_core::toolpath::{Tool, ToolShape};
    let path = temp("joined/tools.json");
    let copy = |name: &str, folder: &str, shank: f64| Tool {
        name: name.into(),
        group: format!("SpeTool/V-BIT/{folder}/1/4\" SHANK"),
        shape: ToolShape::VBit,
        diameter: 19.05,
        angle: 90.0,
        shank_diameter: shank,
        ..Tool::default()
    };
    let db = ToolDatabase {
        tools: vec![
            copy("V 90", "MDF", 6.35),
            copy("V 90 2", "SOFTWOOD", 0.0),
            copy("V 90 3", "HARDWOOD", 12.7),
        ],
        ..ToolDatabase::default()
    };
    save_tools(&db, &path).unwrap();
    let (joined, report) = load_tools_joining(&path).unwrap();
    assert_eq!(joined.tools.len(), 1);
    assert_eq!(joined.get("V 90 3").unwrap().shank_diameter, 6.35);
    assert_eq!(report.len(), 1, "the 12.7 shank is said: {report:?}");
    assert_eq!(
        load_tools(&tools_backup_path(&path)).unwrap(),
        db,
        "the file as it was is kept"
    );
    assert_eq!(
        load_tools(&path).unwrap(),
        joined,
        "and the joined one saved"
    );
    // loading again changes nothing and keeps the first backup
    let (again, report) = load_tools_joining(&path).unwrap();
    assert_eq!((again, report), (joined, vec![]));
    assert_eq!(load_tools(&tools_backup_path(&path)).unwrap(), db);
}

#[test]
fn a_broken_tool_database_names_the_file() {
    let path = temp("broken-tools.json");
    fs::write(&path, "{").unwrap();
    let err = load_tools(&path).unwrap_err();
    assert!(err.contains("broken-tools.json"), "{err}");
}

#[test]
fn toolpaths_save_through_the_post() {
    let post = Post::parse(POST).unwrap();
    let mut doc = sample_document();
    let registry = crate::commands::registry();
    let made = registry.get("drilling_toolpath").unwrap();
    let params = made.default_params();
    let change = made.run_json(&doc, &params).unwrap();
    doc.apply(&change).unwrap();
    let out = temp("out.tap");
    save_through(&post, &[], &doc, &out, None).unwrap();
    assert!(
        fs::read_to_string(&out)
            .unwrap()
            .lines()
            .any(|l| l.starts_with("G0 X"))
    );
}

/// Locate hands the file manager a URI: spaces and other bytes escaped, a Windows path
/// with forward slashes.
#[test]
fn file_uris() {
    assert_eq!(
        super::file_uri(Path::new("/home/m/0325. Welcome #2.stl")),
        "file:///home/m/0325.%20Welcome%20%232.stl"
    );
    assert_eq!(
        super::file_uri(Path::new(r"C:\Clip art\a.stl")),
        "file:///C:/Clip%20art/a.stl"
    );
}
