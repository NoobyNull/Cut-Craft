//! Updating Cut Craft: the version's right-click menu, and Install Now keeping the work that
//! is open for the new version to open again.

use super::*;

#[test]
fn right_clicking_the_version_offers_check_for_updates() {
    let mut h = Harness::new();
    h.frame(vec![]);
    let version = format!("v{}", env!("CARGO_PKG_VERSION"));
    assert!(h.text_at(&version).is_some(), "at the foot of the window");
    // the version is the status bar's last word, at the far right
    let at = h.ctx.content_rect().right_bottom() + egui::vec2(-25.0, -12.0);
    h.right_click(at);
    // a new menu is drawn unseen for a frame while egui sizes it
    h.frame(vec![]);
    assert!(h.text_at("About Cut Craft").is_some(), "{version}'s menu");
    let item = h
        .text_at("Check for Updates...")
        .expect("in the version's menu");
    h.click(item, Modifiers::NONE);
    assert!(h.app.updates.is_open(), "the Updates window opens and asks");
}

/// A job saved as `sign.ccjob` in `dir`, with this run's recovery copy claimed in
/// `dir/settings` as run `run`; the saved file's bytes.
fn saved_job(h: &mut Harness, dir: &std::path::Path, run: &str) -> Vec<u8> {
    h.app.claim_recovery(&dir.join("settings"), run);
    let job = dir.join("sign.ccjob");
    h.app.save_to(&job);
    std::fs::read(&job).expect("saved")
}

/// The next run, started in `dir/settings` as run `run`: whether it opened work left for it.
fn next_run(dir: &std::path::Path, run: &str) -> (Harness, bool) {
    let mut next = Harness::at_start();
    next.app.claim_recovery(&dir.join("settings"), run);
    let opened = next.app.open_recovery();
    (next, opened)
}

#[test]
fn updating_keeps_unsaved_changes_and_the_new_version_opens_them_under_the_same_name() {
    let dir = fresh_dir("update-keeps-changes");
    let mut h = Harness::new();
    let saved = saved_job(&mut h, &dir, "1");
    h.rectangle(10.0, 10.0, 20.0, 20.0);
    assert!(h.app.hand_off_for_update(), "kept without asking");
    let job = dir.join("sign.ccjob");
    assert_eq!(
        std::fs::read(&job).unwrap(),
        saved,
        "the job's own file untouched"
    );
    assert_eq!(
        h.app.job_to_reopen(),
        None,
        "the kept copy opens, not the file"
    );
    // the old version is still closing as the new one starts
    let (next, opened) = next_run(&dir, "2");
    assert!(opened, "the new version opens the kept work");
    assert_eq!(next.app.session.path, Some(job), "under the job's own name");
    assert!(
        next.app.session.is_dirty(&next.app.history),
        "still unsaved"
    );
    assert_eq!(next.app.doc.vectors.len(), 1);
    assert!(
        next.app.status.message.contains("update"),
        "{}",
        next.app.status.message
    );
    drop(h);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn updating_keeps_a_job_that_was_never_saved() {
    let dir = fresh_dir("update-keeps-untitled");
    let mut h = Harness::new();
    h.app.claim_recovery(&dir.join("settings"), "1");
    h.rectangle(10.0, 10.0, 20.0, 20.0);
    assert!(h.app.hand_off_for_update());
    let (next, opened) = next_run(&dir, "2");
    assert!(opened);
    assert_eq!(next.app.session.path, None, "still Untitled");
    assert!(next.app.session.is_dirty(&next.app.history));
    assert_eq!(next.app.doc.vectors.len(), 1);
    drop(h);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn updating_with_nothing_unsaved_reopens_the_job_from_its_file() {
    let dir = fresh_dir("update-nothing-unsaved");
    let mut h = Harness::new();
    saved_job(&mut h, &dir, "1");
    assert!(h.app.hand_off_for_update());
    assert_eq!(h.app.job_to_reopen(), Some(dir.join("sign.ccjob")));
    let (_, opened) = next_run(&dir, "2");
    assert!(!opened, "no copy was needed");
    drop(h);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn updating_asks_to_save_when_the_work_cannot_be_kept() {
    let mut h = Harness::new();
    h.app.recovery_path = None;
    h.rectangle(10.0, 10.0, 20.0, 20.0);
    assert!(
        !h.app.hand_off_for_update(),
        "not kept: closing asks Save changes? as before"
    );
}

#[test]
fn work_recovered_after_a_crash_keeps_its_file_name() {
    let dir = fresh_dir("crash-keeps-name");
    let mut h = Harness::new();
    saved_job(&mut h, &dir, "1");
    h.rectangle(10.0, 10.0, 20.0, 20.0);
    h.frame(vec![]);
    h.time += AUTOSAVE_SECONDS;
    h.frame(vec![]);
    let (_, writing) = h.app.autosave.take().expect("a copy is being written");
    writing.join().unwrap().unwrap();
    // the run ends badly: its lock goes with it, its copy stays
    drop(h);
    let (next, opened) = next_run(&dir, "2");
    assert!(opened);
    assert_eq!(next.app.session.path, Some(dir.join("sign.ccjob")));
    assert!(next.app.session.is_dirty(&next.app.history));
    let _ = std::fs::remove_dir_all(dir);
}
