use cut_craft_core::file::{JOB_EXTENSION, LoadError, OLD_JOB_EXTENSION, load_job, save_job};
use cut_craft_core::testing::sample_document;

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
    assert_eq!(JOB_EXTENSION, "ccjob");
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
        "this is not a Cut Craft job file"
    );
}

#[test]
fn jobs_from_a_newer_version_are_refused_with_a_reason() {
    let newer = br#"{"format": "cut-craft-job", "version": 99, "document": {}}"#;
    let err = load_job(newer).unwrap_err();
    assert_eq!(err, LoadError::NewerVersion(99));
    assert_eq!(
        err.to_string(),
        "this job was saved by a newer version (format 99; this build reads up to 1)"
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
