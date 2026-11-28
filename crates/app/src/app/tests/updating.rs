//! Updating PowerWhittle: the version's right-click menu, and Install Now keeping the work that
//! is open for the new version to open again.

use super::*;
use crate::app::job_files::HandOff;

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
    assert!(
        h.text_at("About PowerWhittle").is_some(),
        "{version}'s menu"
    );
    // About is the Help menu's own item
    assert!(
        gui::version_menu()
            .iter()
            .any(|i| i.action == "about_cut_craft")
    );
    let item = h
        .text_at("Check for Updates...")
        .expect("in the version's menu");
    h.click(item, Modifiers::NONE);
    assert!(h.updates_window_open(), "the Updates window opens and asks");
}

/// A job saved as `sign.ccjob` in `dir`, with this run's recovery copy claimed in
/// `dir/settings` as run `run`; the saved file's bytes.
fn saved_job(h: &mut Harness, dir: &std::path::Path, run: &str) -> Vec<u8> {
    h.given_settings_folder(&dir.join("settings"), run);
    let job = dir.join("sign.ccjob");
    h.save_to(&job);
    std::fs::read(&job).expect("saved")
}

/// The next run, started in `dir/settings` as run `run`: whether it opened work left for it.
fn next_run(dir: &std::path::Path, run: &str) -> (Harness, bool) {
    let mut next = Harness::at_start();
    next.given_settings_folder(&dir.join("settings"), run);
    let opened = next.open_recovered_work();
    (next, opened)
}

#[test]
fn updating_keeps_unsaved_changes_and_the_new_version_opens_them_under_the_same_name() {
    let dir = fresh_dir("update-keeps-changes");
    let mut h = Harness::new();
    let saved = saved_job(&mut h, &dir, "1");
    h.rectangle(10.0, 10.0, 20.0, 20.0);
    assert_eq!(
        h.hand_off_for_update(),
        HandOff::Kept,
        "kept without asking"
    );
    let job = dir.join("sign.ccjob");
    assert_eq!(
        std::fs::read(&job).unwrap(),
        saved,
        "the job's own file untouched"
    );
    assert_eq!(h.job_to_reopen(), None, "the kept copy opens, not the file");
    // the old version is still closing as the new one starts
    let (next, opened) = next_run(&dir, "2");
    assert!(opened, "the new version opens the kept work");
    assert_eq!(
        next.job_path(),
        Some(job.as_path()),
        "under the job's own name"
    );
    assert!(next.has_unsaved_changes(), "still unsaved");
    assert_eq!(next.doc().vectors.len(), 1);
    assert!(
        next.status_message().contains("update"),
        "{}",
        next.status_message()
    );
    drop(h);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn updating_keeps_a_job_that_was_never_saved() {
    let dir = fresh_dir("update-keeps-untitled");
    let mut h = Harness::new();
    h.given_settings_folder(&dir.join("settings"), "1");
    h.rectangle(10.0, 10.0, 20.0, 20.0);
    assert_eq!(h.hand_off_for_update(), HandOff::Kept);
    let (next, opened) = next_run(&dir, "2");
    assert!(opened);
    assert_eq!(next.job_path(), None, "still Untitled");
    assert!(next.has_unsaved_changes());
    assert_eq!(next.doc().vectors.len(), 1);
    drop(h);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn updating_with_nothing_unsaved_reopens_the_job_from_its_file() {
    let dir = fresh_dir("update-nothing-unsaved");
    let mut h = Harness::new();
    saved_job(&mut h, &dir, "1");
    assert_eq!(h.hand_off_for_update(), HandOff::NothingUnsaved);
    assert_eq!(h.job_to_reopen(), Some(dir.join("sign.ccjob")));
    let (_, opened) = next_run(&dir, "2");
    assert!(!opened, "no copy was needed");
    drop(h);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn updating_asks_to_save_when_the_work_cannot_be_kept() {
    let mut h = Harness::new();
    h.given_no_recovery_folder();
    h.rectangle(10.0, 10.0, 20.0, 20.0);
    assert_eq!(
        h.hand_off_for_update(),
        HandOff::NotKept,
        "closing asks Save changes? as before"
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
    h.finish_recovery_copy();
    // the run ends badly: its lock goes with it, its copy stays
    drop(h);
    let (next, opened) = next_run(&dir, "2");
    assert!(opened);
    assert_eq!(next.job_path(), Some(dir.join("sign.ccjob").as_path()));
    assert!(next.has_unsaved_changes());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_kept_work_is_not_written_over_while_cut_craft_closes() {
    let dir = fresh_dir("update-no-autosave-after");
    let mut h = Harness::new();
    saved_job(&mut h, &dir, "1");
    h.rectangle(10.0, 10.0, 20.0, 20.0);
    assert_eq!(h.hand_off_for_update(), HandOff::Kept);
    // the recovery interval passes before the window has quite gone
    h.time += AUTOSAVE_SECONDS;
    h.frame(vec![]);
    assert!(
        !h.writing_recovery_copy(),
        "no ordinary copy written over it"
    );
    let copy = dir.join("settings").join("recovery-1.ccjob");
    assert!(crate::files::recovery_note(&copy).after_update);
    drop(h);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn closing_again_after_the_work_is_kept_neither_asks_nor_throws_it_away() {
    let dir = fresh_dir("update-close-twice");
    let mut h = Harness::new();
    saved_job(&mut h, &dir, "1");
    h.rectangle(10.0, 10.0, 20.0, 20.0);
    assert_eq!(h.hand_off_for_update(), HandOff::Kept);
    // a second close request (no Save changes? question, which a test would hang on)
    assert!(h.close_window(), "closes");
    assert!(
        dir.join("settings").join("recovery-1.ccjob").exists(),
        "the work is still kept"
    );
    drop(h);
    let _ = std::fs::remove_dir_all(dir);
}
