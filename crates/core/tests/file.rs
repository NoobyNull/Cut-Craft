use cut_craft_core::file::{
    JOB_EXTENSION, JOB_EXTENSIONS, LoadError, OLD_JOB_EXTENSION, TEMPLATE_EXTENSIONS, is_template,
    load_job, save_job,
};
use cut_craft_core::testing::sample_document;
use std::path::Path;

#[test]
fn a_saved_job_loads_back_identically() {
    let doc = sample_document();
    assert_eq!(load_job(&save_job(&doc)).unwrap(), doc);
}

#[test]
fn saved_jobs_say_what_they_are() {
    let json: serde_json::Value = serde_json::from_slice(&save_job(&sample_document())).unwrap();
    assert_eq!(json["format"], "cut-craft-job");
    assert_eq!(json["version"], 1);
    assert_eq!(JOB_EXTENSION, "pwjob");
}

#[test]
fn jobs_saved_before_the_rename_to_cut_craft_still_open() {
    let doc = sample_document();
    let mut json: serde_json::Value = serde_json::from_slice(&save_job(&doc)).unwrap();
    json["format"] = "aspire-clone-job".into();
    assert_eq!(load_job(&serde_json::to_vec(&json).unwrap()).unwrap(), doc);
    assert_eq!(OLD_JOB_EXTENSION, "ajob");
}

#[test]
fn other_json_is_not_a_job() {
    assert_eq!(load_job(br#"{"hello": 1}"#), Err(LoadError::NotAJob));
    assert_eq!(load_job(b"not json at all"), Err(LoadError::NotAJob));
    assert_eq!(
        LoadError::NotAJob.to_string(),
        "this is not a PowerWhittle job file"
    );
}

#[test]
fn jobs_from_a_newer_version_are_refused_with_a_reason() {
    let newer = br#"{"format": "cut-craft-job", "version": 99, "document": {}}"#;
    let err = load_job(newer).unwrap_err();
    assert_eq!(err, LoadError::NewerVersion(99));
    assert_eq!(
        err.to_string(),
        "this job was saved by a newer version of PowerWhittle (format 99; this build reads up \
         to 1). Update PowerWhittle (Help ▸ Check for Updates) to open it"
    );
}

#[test]
fn a_job_cut_short_is_damaged_not_another_kind_of_file() {
    let whole = save_job(&sample_document());
    let cut_short = &whole[..whole.len() / 2];
    let err = load_job(cut_short).unwrap_err();
    assert!(matches!(err, LoadError::Damaged(_)), "{err:?}");
    let told = err.to_string();
    assert!(
        told.contains("damaged") && told.contains("backup"),
        "says what to do: {told}"
    );
}

#[test]
fn a_damaged_document_reports_why() {
    let damaged = br#"{"format": "cut-craft-job", "version": 1, "document": {"job": 5}}"#;
    assert!(matches!(load_job(damaged), Err(LoadError::Damaged(_))));
}

#[test]
fn an_atomic_write_replaces_the_file_whole_and_leaves_no_temp_behind() {
    use cut_craft_core::file::write_atomic;
    let dir = std::env::temp_dir().join(format!("cut-craft-atomic-{}", std::process::id()));
    let path = dir.join("deep/er/job.ccjob");
    write_atomic(&path, b"first").unwrap();
    assert_eq!(
        std::fs::read(&path).unwrap(),
        b"first",
        "the folder is made"
    );
    write_atomic(&path, b"second").unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"second");
    let left: Vec<_> = std::fs::read_dir(path.parent().unwrap())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(left, ["job.ccjob"], "no .tmp file stays");
    // a path that cannot be written fails and keeps the old file
    assert!(write_atomic(&path.join("under-a-file"), b"x").is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"second");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn settings_a_newer_version_added_are_named_rather_than_dropped_silently() {
    use cut_craft_core::file::load_job_noting;
    let mut json: serde_json::Value =
        serde_json::from_slice(&save_job(&sample_document())).unwrap();
    json["document"]["hologram"] = serde_json::json!(true);
    json["document"]["job"]["glitter"] = serde_json::json!(3);
    let (doc, unknown) = load_job_noting(&serde_json::to_vec(&json).unwrap()).unwrap();
    assert_eq!(doc, sample_document());
    assert_eq!(unknown, ["job.glitter", "hologram"]);
    let (_, unknown) = load_job_noting(&save_job(&sample_document())).unwrap();
    assert!(
        unknown.is_empty(),
        "its own files know every setting: {unknown:?}"
    );
}

#[test]
fn jobs_save_as_pwjob_and_cut_craft_s_ccjob_open_as_the_very_same_file() {
    // the format inside does not change with the name
    assert_eq!(JOB_EXTENSIONS, ["pwjob", "ccjob", "ajob"]);
    assert_eq!(TEMPLATE_EXTENSIONS, ["pwjobt", "ccjobt"]);
    assert!(is_template(Path::new("/jobs/Plate.pwjobt")));
    assert!(is_template(Path::new("/jobs/Plate.CCJOBT")));
    assert!(!is_template(Path::new("/jobs/Plate.pwjob")));
    assert!(!is_template(Path::new("/jobs/pwjobt")));
}

#[test]
fn settings_from_before_the_rename_come_over_once_and_stay_where_they_were() {
    use cut_craft_core::file::{OLD_SETTINGS_FOLDER, SETTINGS_FOLDER, bring_over_settings};
    assert_eq!(
        (OLD_SETTINGS_FOLDER, SETTINGS_FOLDER),
        ("cut-craft", "powerwhittle")
    );
    let base = std::env::temp_dir().join(format!("pw-settings-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let old = base.join("cut-craft");
    std::fs::create_dir_all(old.join("posts")).unwrap();
    std::fs::write(old.join("options.json"), "{}").unwrap();
    std::fs::write(old.join("posts/mine.pp"), "post").unwrap();
    // the first start of the renamed app copies everything across
    assert!(bring_over_settings(&base).unwrap());
    let new = base.join("powerwhittle");
    assert_eq!(
        std::fs::read_to_string(new.join("options.json")).unwrap(),
        "{}"
    );
    assert_eq!(
        std::fs::read_to_string(new.join("posts/mine.pp")).unwrap(),
        "post"
    );
    assert!(
        old.join("options.json").exists(),
        "the old folder stays for an older copy"
    );
    // later starts leave the new folder alone, changes and all
    std::fs::write(new.join("options.json"), "{\"changed\": true}").unwrap();
    assert!(!bring_over_settings(&base).unwrap());
    assert_eq!(
        std::fs::read_to_string(new.join("options.json")).unwrap(),
        "{\"changed\": true}"
    );
    // a fresh install has nothing to bring
    let fresh = base.join("fresh");
    std::fs::create_dir_all(&fresh).unwrap();
    assert!(!bring_over_settings(&fresh).unwrap());
    assert!(!fresh.join("powerwhittle").exists());
    std::fs::remove_dir_all(&base).unwrap();
}

#[test]
fn a_bring_over_cut_short_is_finished_next_start_keeping_what_the_new_folder_has() {
    use cut_craft_core::file::{BROUGHT_OVER, bring_over_settings};
    let base = std::env::temp_dir().join(format!("pw-settings-retry-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let (old, new) = (base.join("cut-craft"), base.join("powerwhittle"));
    std::fs::create_dir_all(old.join("addons/ru")).unwrap();
    std::fs::write(old.join("options.json"), "{\"old\": true}").unwrap();
    std::fs::write(old.join("addons/ru/main.ftl"), "old pack").unwrap();
    // a start that failed part way, after which the log made the new folder, and a pack
    // copied in by hand before the first start
    std::fs::create_dir_all(new.join("addons/ru")).unwrap();
    std::fs::write(new.join("cut-craft.log"), "log").unwrap();
    std::fs::write(new.join("addons/ru/main.ftl"), "new pack").unwrap();
    assert!(bring_over_settings(&base).unwrap());
    assert_eq!(
        std::fs::read_to_string(new.join("options.json")).unwrap(),
        "{\"old\": true}",
        "what was missing comes over"
    );
    assert_eq!(
        std::fs::read_to_string(new.join("addons/ru/main.ftl")).unwrap(),
        "new pack",
        "what the new folder had is kept"
    );
    assert!(new.join(BROUGHT_OVER).exists());
    let left: Vec<_> = std::fs::read_dir(&new)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.contains(".copying"))
        .collect();
    assert!(left.is_empty(), "no part copies left: {left:?}");
    // once done, it is not done again
    std::fs::remove_file(new.join("options.json")).unwrap();
    assert!(!bring_over_settings(&base).unwrap());
    assert!(!new.join("options.json").exists());
    std::fs::remove_dir_all(&base).unwrap();
}

#[test]
fn after_a_failed_bring_over_the_app_saves_in_the_old_folder_and_the_next_start_brings_it() {
    use cut_craft_core::file::{BROUGHT_OVER, COPIED_SO_FAR, bring_over_settings, settings_folder};
    let base = std::env::temp_dir().join(format!("pw-settings-failed-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let (old, new) = (base.join("cut-craft"), base.join("powerwhittle"));
    std::fs::create_dir_all(&old).unwrap();
    std::fs::write(old.join("options.json"), "{\"mine\": true}").unwrap();
    // a start that copied options.json, then failed on another file
    std::fs::create_dir_all(&new).unwrap();
    std::fs::write(new.join("options.json"), "{\"mine\": true}").unwrap();
    std::fs::write(new.join(COPIED_SO_FAR), "options.json\n").unwrap();
    assert_eq!(
        settings_folder(&base),
        old,
        "the app keeps to the old folder"
    );
    // that session saves its options where it reads them, with the very same time as the
    // part copy, as a folder that keeps times to the second or two can give
    let copy_time = std::fs::metadata(new.join("options.json"))
        .unwrap()
        .modified()
        .unwrap();
    std::fs::write(old.join("options.json"), "{\"changed\": true}").unwrap();
    std::fs::File::options()
        .write(true)
        .open(old.join("options.json"))
        .unwrap()
        .set_modified(copy_time)
        .unwrap();
    // the next start brings the saved options over, not the stale copy
    assert!(bring_over_settings(&base).unwrap());
    assert_eq!(
        std::fs::read_to_string(new.join("options.json")).unwrap(),
        "{\"changed\": true}"
    );
    assert!(new.join(BROUGHT_OVER).exists());
    assert!(!new.join(COPIED_SO_FAR).exists(), "the list goes once done");
    assert_eq!(
        settings_folder(&base),
        new,
        "and from then on the new folder"
    );
    std::fs::remove_dir_all(&base).unwrap();
}

#[test]
fn starts_at_the_same_time_take_turns_and_bring_the_settings_over_once() {
    use cut_craft_core::file::bring_over_settings;
    let base = std::env::temp_dir().join(format!("pw-settings-together-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let old = base.join("cut-craft");
    std::fs::create_dir_all(old.join("posts")).unwrap();
    for n in 0..50 {
        std::fs::write(old.join(format!("posts/{n}.pp")), format!("post {n}")).unwrap();
    }
    let starts: Vec<_> = (0..4)
        .map(|_| {
            let base = base.clone();
            std::thread::spawn(move || bring_over_settings(&base).unwrap())
        })
        .collect();
    let brought = starts
        .into_iter()
        .map(|s| s.join().unwrap())
        .filter(|&b| b)
        .count();
    assert_eq!(brought, 1, "one start brings them, the others find it done");
    for n in 0..50 {
        assert_eq!(
            std::fs::read_to_string(base.join(format!("powerwhittle/posts/{n}.pp"))).unwrap(),
            format!("post {n}")
        );
    }
    std::fs::remove_dir_all(&base).unwrap();
}
