//! Files and jobs: opening, saving, importing, crash reports, clipart and add-ons.

use super::*;

#[test]
fn edit_units_switches_the_job_between_millimetres_and_inches() {
    use cut_craft_core::document::Units;
    let mut h = Harness::new();
    assert_eq!(h.app.units_checked("units_mm"), Some(true));
    assert_eq!(h.app.units_checked("zoom_box"), None);
    h.app.cursor = Some(Point::new(25.4, 50.8));
    assert_eq!(h.app.coordinates().map(|(_, unit)| unit), Some("mm"));
    h.app.invoke(&h.ctx, "units_inch");
    assert_eq!(h.app.doc.job.units, Units::Inch);
    assert_eq!(h.app.units_checked("units_inch"), Some(true));
    assert_eq!(h.app.units_checked("units_mm"), Some(false));
    let (figures, unit) = h.app.coordinates().unwrap();
    assert_eq!((figures.as_str(), unit), ("X    1.000  Y    2.000", "in"));
    h.frame(vec![]);
    assert!(h.text_at("in").is_some(), "the status bar's unit button");
    h.app.invoke(&h.ctx, "undo");
    assert_eq!(h.app.doc.job.units, Units::Mm, "undoable");
}

#[test]
fn a_new_job_is_set_up_over_the_2d_view() {
    let mut h = Harness::at_start();
    h.app.view = View::ThreeD;
    h.app.invoke(&h.ctx, "new");
    assert!(
        h.app.view == View::TwoD,
        "the material shows while it is set up"
    );
}

#[test]
fn it_starts_on_the_start_page_with_the_no_job_menus() {
    let h = Harness::at_start();
    assert_eq!(h.app.screen, Screen::Start);
    assert_eq!(h.app.title, "Cut Craft", "no job, so no file name");
    assert_eq!(
        h.app.menu_names(),
        ["File", "Edit", "Machine", "Toolpaths", "Gadgets", "Help"]
    );
    assert!(
        h.app.view_transform.is_none(),
        "no 2D view before there is a job"
    );
}

#[test]
fn create_a_new_file_goes_through_job_setup_to_the_job() {
    let mut h = Harness::at_classic_start();
    h.app.invoke(&h.ctx, "new");
    h.frame(vec![]);
    assert_eq!(h.app.screen, Screen::NewJob);
    let form = h.app.form.as_mut().expect("the Job Setup form");
    assert_eq!(form.id, "job_setup");
    form.params["size"]["width"] = 300.0.into();
    assert!(
        (h.app.shown_job().width - 300.0).abs() < 1e-9,
        "the 2D view shows the job being set up"
    );
    h.app.job_setup_ok();
    h.frame(vec![]);
    assert_eq!(h.app.screen, Screen::Job);
    assert!(h.app.form.is_none());
    assert!((h.app.doc.job.width - 300.0).abs() < 1e-9);
    assert!(
        !h.app.history.can_undo(),
        "a new job starts with nothing to undo"
    );
    assert_eq!(
        h.app.status.current_tool, "Select",
        "Job Setup is done with"
    );
    assert!(h.app.menu_names().contains(&"Model"));
    assert!(h.app.view_transform.is_some());
}

#[test]
fn cancelling_job_setup_goes_back_to_the_start_page() {
    let mut h = Harness::at_start();
    h.app.invoke(&h.ctx, "new");
    h.app.job_setup_cancel();
    h.frame(vec![]);
    assert_eq!(h.app.screen, Screen::Start);
    assert!(h.app.form.is_none());
}

#[test]
fn closing_the_job_goes_back_to_the_start_page() {
    let mut h = Harness::new();
    h.app.options.job_defaults.classic_startup = true;
    h.app.invoke(&h.ctx, "close");
    h.frame(vec![]);
    assert_eq!(h.app.screen, Screen::Start);
}

#[test]
fn saved_and_opened_jobs_are_listed_as_recent_files() {
    let mut h = Harness::new();
    let dir = fresh_dir("recent");
    h.app.recent_path = Some(dir.join("recent.json"));
    let job = dir.join("first.ccjob");
    h.app.save_to(&job);
    assert_eq!(h.app.recent, std::slice::from_ref(&job));
    assert_eq!(
        files::load_recent(&dir.join("recent.json")),
        std::slice::from_ref(&job)
    );
    h.app.invoke(&h.ctx, "close");
    h.app.invoke(&h.ctx, "open_recent:0");
    assert_eq!(h.app.screen, Screen::Job);
    assert_eq!(h.app.session.path.as_deref(), Some(job.as_path()));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_recent_file_that_has_gone_is_dropped() {
    let mut h = Harness::at_start();
    h.app.recent = vec![std::env::temp_dir().join("cut-craft-gone.ccjob")];
    h.app.invoke(&h.ctx, "open_recent:0");
    assert_eq!(h.app.screen, Screen::Start);
    assert_eq!(h.app.recent, Vec::<std::path::PathBuf>::new());
    assert!(h.app.status.message.contains("cut-craft-gone.ccjob"));
}

#[test]
fn the_clipart_library_button_opens_the_library_tab() {
    let mut h = Harness::new();
    assert!(h.app.available("clipart_library"));
    // picked from a flown-out Design page, the page stays out on the Library tab
    h.app.pages.set_pinned(Page::Design, false);
    h.app.pages.open(Page::Design);
    h.app.pick(&h.ctx, Page::Design, "clipart_library");
    assert_eq!(DESIGN_TABS[h.app.design_tab], "Library");
    assert_ne!(h.app.pages.shown(Page::Design), Shown::Hidden);
    assert_eq!(h.app.tool_label("import_cut_list"), "Import Cut List");
}

/// The text drawn by `draw`, alone in a window of its own.
fn drawn_alone(draw: impl FnMut(&mut egui::Ui)) -> Vec<String> {
    let ctx = egui::Context::default();
    let mut draw = draw;
    let out = headless(ctx.run_ui(RawInput::default(), |ui| {
        egui::CentralPanel::default().show(ui, |ui| draw(ui));
    }));
    out.shapes
        .iter()
        .filter_map(|c| match &c.shape {
            egui::Shape::Text(t) => Some(t.galley.text().to_owned()),
            _ => None,
        })
        .collect()
}

#[test]
fn rest_machining_sizes_show_in_inches_in_an_inch_job() {
    use cut_craft_core::document::Units;
    let h = Harness::new();
    let tool = h.app.tools.tools.first().cloned().expect("a starter tool");
    let mut section = serde_json::json!({
        "rest": { "after": [tool], "minimum_detail": 2.54, "overlap": 12.7 }
    });
    let texts = drawn_alone(|ui| {
        rest_section(ui, &mut section, &h.app.tools, &[], Units::Inch);
    });
    assert!(shows_number(&texts, "0.100", " in"), "{texts:?}");
    assert!(shows_number(&texts, "0.500", " in"), "{texts:?}");
    // the bit's line too, though the bit is kept in mm in the Tool Database
    assert!(
        texts
            .iter()
            .any(|t| t == "0.118 in end mill, 0.059 in a pass"),
        "{texts:?}"
    );
}

#[test]
fn a_longer_recovery_interval_in_the_options_waits_longer() {
    let dir = fresh_dir("later");
    let mut h = Harness::new();
    h.app.options.general.autosave_seconds = 120;
    h.app.recovery_path = Some(dir.join("later.ccjob"));
    h.rectangle(10.0, 10.0, 20.0, 20.0);
    h.frame(vec![]);
    h.time += AUTOSAVE_SECONDS;
    h.frame(vec![]);
    assert!(h.app.autosave.is_none(), "not after the default interval");
    h.time += 120.0;
    h.frame(vec![]);
    h.app
        .autosave
        .take()
        .expect("written once the options' interval is up")
        .1
        .join()
        .unwrap()
        .unwrap();
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn unsaved_work_is_kept_for_recovery_and_opens_after_a_crash() {
    let dir = fresh_dir("recovery");
    // each run (window) keeps a copy of its own, named for it
    let copy = |run: &str| dir.join(format!("recovery-{run}.ccjob"));
    let autosaved = |h: &mut Harness, run: &str| {
        h.app.claim_recovery(&dir, run);
        assert_eq!(h.app.recovery_path, Some(copy(run)));
        h.rectangle(10.0, 10.0, 20.0, 20.0);
        h.frame(vec![]);
        assert!(!copy(run).exists(), "not at once");
        h.time += AUTOSAVE_SECONDS;
        h.frame(vec![]);
        let (_, writing) = h.app.autosave.take().expect("a copy is being written");
        writing.join().unwrap().unwrap();
        assert!(copy(run).exists());
    };
    // closing the job on purpose leaves nothing to recover
    let mut h = Harness::new();
    autosaved(&mut h, "1");
    h.app.session.discard_changes(&h.app.history);
    h.app.invoke(&h.ctx, "close");
    assert!(!copy("1").exists());
    drop(h);
    // a second window still running leaves the first one's live work alone
    let mut first = Harness::new();
    autosaved(&mut first, "2");
    let mut second = Harness::at_start();
    second.app.claim_recovery(&dir, "3");
    assert!(
        !second.app.open_recovery(),
        "another window's work is not a crash"
    );
    assert_eq!(second.app.screen, Screen::Start);
    second.app.invoke(&second.ctx, "new");
    second.app.job_setup_ok();
    second.app.invoke(&second.ctx, "close");
    assert!(
        copy("2").exists(),
        "closing the second keeps the first one's copy"
    );
    // a run that ends badly leaves the copy, and the next run opens it
    drop(first);
    drop(second);
    let mut next = Harness::at_start();
    next.app.claim_recovery(&dir, "4");
    assert!(next.app.open_recovery());
    assert_eq!(next.app.screen, Screen::Job);
    assert_eq!(next.app.doc.vectors.len(), 1);
    assert!(
        next.app.session.is_dirty(&next.app.history),
        "unsaved until saved"
    );
    assert!(next.app.status.message.contains("Recovered"));
    // opening it keeps the work, now as this run's copy, so a second crash before any edit
    // still has it
    assert!(!copy("2").exists());
    assert!(copy("4").exists(), "kept until saved or closed");
    let mut other = Harness::at_start();
    other.app.claim_recovery(&dir, "5");
    assert!(
        !other.app.open_recovery(),
        "not while the run that has it is open"
    );
    drop(next);
    assert!(other.app.open_recovery(), "still there for the run after");
    other.app.save_to(&dir.join("kept.ccjob"));
    assert!(
        !copy("4").exists() && !copy("5").exists(),
        "saved: nothing to recover"
    );
    assert!(!other.app.open_recovery());
    // a copy left by a version that kept one for every window is recovered too
    drop(other);
    std::fs::write(
        dir.join("recovery.ccjob"),
        std::fs::read(dir.join("kept.ccjob")).unwrap(),
    )
    .unwrap();
    let mut updated = Harness::at_start();
    updated.app.claim_recovery(&dir, "6");
    assert!(updated.app.open_recovery());
    assert!(!dir.join("recovery.ccjob").exists());
    drop(updated);
    // a crashed run whose number a new run is given still has its copy recovered
    let mut crashed = Harness::new();
    autosaved(&mut crashed, "7");
    drop(crashed);
    let mut same = Harness::at_start();
    same.app.claim_recovery(&dir, "7");
    assert!(same.app.open_recovery());
    assert_eq!(same.app.doc.vectors.len(), 1);
    drop(same);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn options_close_with_changes_not_applied_asks_first() {
    let mut h = options_with_a_change_not_applied();
    click_button(&mut h, "Close");
    h.frame(vec![]);
    assert!(h.app.options_window, "not closed yet");
    assert!(
        h.text_at(super::options_apply::ASK).is_some(),
        "{:?}",
        h.texts()
    );
    // Keep Editing: back to the settings, the change still waiting
    click_button(&mut h, "Keep Editing");
    assert!(h.app.options_window);
    assert!(h.app.options_dirty());
    h.frame(vec![]);
    assert!(h.text_at(super::options_apply::ASK).is_none());
    // Discard Changes: closed, nothing used
    click_button(&mut h, "Close");
    h.frame(vec![]);
    click_button(&mut h, "Discard Changes");
    assert!(!h.app.options_window);
    assert_eq!(h.app.options.general.recent_file_list_size, 9);
}

#[test]
fn options_close_can_apply_the_changes_on_the_way_out() {
    let mut h = options_with_a_change_not_applied();
    click_button(&mut h, "Close");
    h.frame(vec![]);
    click_button(&mut h, "Apply and Close");
    assert!(!h.app.options_window);
    assert_eq!(h.app.options.recent_count(), files::RECENT_COUNT);
}

#[test]
fn options_close_with_nothing_changed_just_closes() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "options");
    h.frame(vec![]);
    h.frame(vec![]);
    let options = h.app.options.clone();
    // Apply does nothing while nothing has changed
    click_button(&mut h, "Apply");
    assert_eq!(h.app.options, options);
    click_button(&mut h, "Close");
    assert!(!h.app.options_window, "closed without asking");
}

#[test]
fn set_as_default_in_job_setup_is_what_new_jobs_start_with() {
    let mut h = Harness::at_classic_start();
    h.app.invoke(&h.ctx, "new");
    h.frame(vec![]);
    let form = h.app.form.as_mut().expect("the Job Setup form");
    form.params["size"]["width"] = 300.0.into();
    form.params["size"]["thickness"] = 12.0.into();
    let button = h
        .text_at("Set as Default")
        .expect("a Set as Default button");
    h.click(button, Modifiers::NONE);
    h.frame(vec![]);
    let saved = h.app.options.default_job.expect("kept in the options");
    assert!((saved.width - 300.0).abs() < 1e-9);
    h.app.job_setup_cancel();
    h.app.invoke(&h.ctx, "new");
    h.frame(vec![]);
    let form = h.app.form.as_ref().expect("Job Setup again");
    assert_eq!(form.params["size"]["width"], 300.0);
    assert_eq!(form.params["size"]["thickness"], 12.0);
}

#[test]
fn a_crash_log_goes_to_the_service_by_itself_when_allowed() {
    use cut_craft_bug_report::relay::Failure;
    type Post = fn(&str, &str) -> Result<(), Failure>;
    #[expect(
        clippy::unnecessary_wraps,
        reason = "a post that can fail, as the real one"
    )]
    fn accepted(url: &str, body: &str) -> Result<(), Failure> {
        assert_eq!(url, "https://crash.example/v1/crash");
        assert!(
            body.contains("\"title\":\"Crash: index out of bounds\""),
            "{body}"
        );
        Ok(())
    }
    fn refused(_: &str, _: &str) -> Result<(), Failure> {
        Err(Failure::of_status(400))
    }
    fn down(_: &str, _: &str) -> Result<(), Failure> {
        Err(Failure::of_status(503))
    }
    // what the status bar says, whether the form opens, and whether the log is kept to try
    // again at the next start
    let cases: [(Post, &str, bool, bool); 3] = [
        (accepted, "crash report was sent", false, false),
        (refused, "", true, false),
        (down, "will try again the next time it starts", false, true),
    ];
    for (post, said, form, retried) in cases {
        let mut h = Harness::new();
        let dir = fresh_dir("retry");
        h.app.options.bug_reports.crash_logs = crate::options::CrashLogs::Send;
        h.app.reporter.crash_service = Some("https://crash.example/v1/crash".into());
        h.app.reporter.post = post;
        h.app.reporter.work_bits = 4;
        h.app.reporter.retry_dir = Some(dir.clone());
        let ctx = h.ctx.clone();
        h.app.offer_crash(&ctx, CRASH.into());
        assert!(
            h.app.options.bug_reports.install_id.is_some(),
            "numbered once"
        );
        for _ in 0..200 {
            h.frame(vec![]);
            if h.app.status.message.contains("crash report") || h.app.reporter.is_open() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        h.frame(vec![]);
        assert!(
            h.app.status.message.contains(said),
            "{}",
            h.app.status.message
        );
        assert_eq!(h.app.reporter.is_open(), form, "{said}");
        let kept = cut_craft_bug_report::logs::take_crash(&dir);
        assert_eq!(
            kept.is_some(),
            retried,
            "only a report that may go later is kept for the next start"
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[test]
fn cancel_takes_back_what_a_tool_did_and_closes_it() {
    let mut h = Harness::new();
    // a tool that has done nothing yet has nothing to cancel
    h.app.start_command("draw_circle");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Cancel").is_some());
    let undoable = h.app.history.undo_depth();
    h.app.form = None;
    // Import 3D Model puts the model in the job at once
    let file = pyramid_stl("cancel");
    h.app.import_file(&file);
    h.frame(vec![]);
    h.frame(vec![]);
    assert_eq!(
        h.app.doc.components.len(),
        1,
        "placed while the form is open"
    );
    let over = h.text_at("Initial Orientation").unwrap();
    let cancel = h.scroll_to(over, "Cancel").expect("a Cancel button");
    h.click(cancel, Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.app.form.is_none(), "closed");
    assert!(h.app.doc.components.is_empty(), "the model taken back");
    assert_eq!(h.app.history.undo_depth(), undoable);
    assert!(h.app.history.can_redo(), "and it can be redone");
    let _ = std::fs::remove_file(file);
}

#[test]
fn job_setup_for_a_new_job_is_as_wide_as_the_design_page_after() {
    let mut h = Harness::at_start();
    h.app.invoke(&h.ctx, "new");
    let setup = page_widths(&mut h, 3);
    h.app.job_setup_ok();
    let job = page_widths(&mut h, 2);
    assert_eq!(setup[2][0], job[1][0], "no jump when the job is made");
}

/// Add to Project with several picked adds them one after another: a model through its
/// Orientate 3D Model form, the next once that form is closed; a file gone meanwhile is
/// reported, not a crash.
#[test]
fn library_adds_several_in_turn() {
    let mut h = Harness::new();
    let dir = fresh_dir("library-add");
    let square = r#"<svg xmlns="http://www.w3.org/2000/svg" width="20mm" height="20mm" viewBox="0 0 20 20"><rect x="0" y="0" width="20" height="20"/></svg>"#;
    let (first, last) = (dir.join("first.svg"), dir.join("last.svg"));
    std::fs::write(&first, square).unwrap();
    std::fs::write(&last, square).unwrap();
    let model = pyramid_stl("library-add");
    h.app
        .add_from_library(vec![first, model, last, dir.join("gone.svg")], None);
    assert_eq!(h.app.doc.vectors.len(), 1, "the first is in");
    assert_eq!(
        h.app.form.as_ref().map(|f| f.id),
        Some("orientate_3d_model"),
        "the model waits on its form"
    );
    assert_eq!(h.app.import_queue.len(), 2, "the rest wait their turn");
    h.app.form = None; // the form closed
    h.frame(vec![]);
    assert_eq!(h.app.doc.vectors.len(), 2, "then the next comes in");
    assert!(h.app.import_queue.is_empty());
    assert!(
        h.app.status.message.contains("isn't there"),
        "{}",
        h.app.status.message
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn what_a_slow_job_made_is_not_used_if_the_job_changed_meanwhile() {
    let mut h = Harness::new();
    h.app.wait_before_showing = std::time::Duration::ZERO;
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    let (go, wait) = std::sync::mpsc::channel::<()>();
    h.app.in_background(
        "Calculating",
        "test:calculating",
        move || {
            wait.recv().ok();
        },
        |app, ()| app.status.set_message("used"),
    );
    h.app.invoke(&h.ctx, "undo");
    go.send(()).unwrap();
    until_not_busy(&mut h);
    assert!(
        h.app.status.message.contains("was not used"),
        "{}",
        h.app.status.message
    );
}

#[test]
fn a_dxf_without_units_asks_which_with_the_guess_picked_then_zooms_to_it() {
    let mut h = Harness::new();
    // a 600 x 400 mm job: 6 mm would be a speck, so inches are picked
    h.app.import_file(&unitless_dxf("units-ask"));
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(
        h.app.doc.vectors.is_empty(),
        "nothing comes in before the answer"
    );
    assert!(
        h.text_at("This drawing doesn't say what units it was made in")
            .is_some()
    );
    assert!(
        h.text_at("Inches: 6.000 in by 6.000 in (fits the material)")
            .is_some()
    );
    assert!(
        h.text_at("Millimetres: 6.0 mm by 6.0 mm (too small to see)")
            .is_some()
    );
    let camera = h.app.camera.map(|c| c.scale);
    h.click(h.text_at("Zoom to It").unwrap(), Modifiers::NONE);
    h.frame(vec![]);
    assert!(
        (drawn_width(&h) - 152.4).abs() < 1e-6,
        "{}",
        drawn_width(&h)
    );
    assert_eq!(h.app.doc.selection.len(), 1, "still selected");
    assert_ne!(h.app.camera.map(|c| c.scale), camera, "zoomed to it");
    assert!(h.app.form.is_none());
    // undone in one step
    h.app.invoke(&h.ctx, "undo");
    assert!(h.app.doc.vectors.is_empty());
}

#[test]
fn a_save_that_fails_after_replace_or_archive_leaves_the_earlier_files_where_they_were() {
    use crate::files::{SaveMode, ToolChanges};
    use crate::options::EarlierGcode;
    use cut_craft_core::keep_out::KeepOutZone;
    let mut h = Harness::new();
    two_tools(&mut h);
    // a clamp put down over the first rectangle after its toolpath was made
    let zone = cut_craft_core::testing::square(55.0, 20.0, 10.0);
    h.app.doc.keep_out = vec![KeepOutZone::new("Clamp 1", zone)];
    let dir = fresh_dir("resave-fail");
    let form = SaveForm {
        mode: SaveMode::VisibleToOneFile,
        group: false,
        post: None,
        add_side: false,
        changes: ToolChanges::ToolChanger,
        job_sheet: false,
    };
    let post = h.app.form_post(&form).unwrap();
    let path = dir.join("sign.nc");
    let numbered = dir.join("sign_3_Old-Vcarve.nc");
    for file in [&path, &numbered] {
        std::fs::write(file, "old").unwrap();
    }
    for choice in [EarlierGcode::Replace, EarlierGcode::Archive] {
        h.app.save_over_earlier(&form, post.clone(), path.clone());
        h.app.answer_earlier(Some(choice), false);
        let message = h.app.status.message.clone();
        assert!(message.contains("keep-out zone"), "{message}");
        assert!(message.contains("earlier files were left"), "{message}");
        for file in [&path, &numbered] {
            assert_eq!(std::fs::read_to_string(file).unwrap(), "old", "{choice:?}");
        }
        let left: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|name| !name.starts_with("Archive"))
            .collect();
        assert_eq!(left.len(), 2, "nothing else left behind: {left:?}");
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn saving_over_an_earlier_save_asks_then_replaces_or_archives_its_files() {
    use crate::files::{SaveMode, ToolChanges};
    use crate::options::EarlierGcode;
    let mut h = Harness::new();
    two_tools(&mut h);
    let dir = fresh_dir("resave");
    let form = SaveForm {
        mode: SaveMode::VisibleToOneFile,
        group: false,
        post: None,
        add_side: false,
        changes: ToolChanges::ToolChanger,
        job_sheet: false,
    };
    let post = h.app.form_post(&form).unwrap();
    let path = dir.join("sign.nc");
    let stale = dir.join("sign_3_Old-Vcarve.nc");
    let mine = dir.join("sign notes.nc");
    for file in [&path, &stale, &mine] {
        std::fs::write(file, "old").unwrap();
    }
    // asked first: nothing is written until it is answered
    h.app.save_over_earlier(&form, post.clone(), path.clone());
    assert!(h.app.earlier_ask.is_some());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "old");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Replace the earlier G-code?").is_some());
    // Cancel keeps them
    h.app.answer_earlier(None, false);
    assert_eq!(std::fs::read_to_string(&stale).unwrap(), "old");
    // Replace: the new file, and no older numbered one left to run by mistake
    h.app.save_over_earlier(&form, post.clone(), path.clone());
    h.app.answer_earlier(Some(EarlierGcode::Replace), false);
    assert!(std::fs::read_to_string(&path).unwrap().contains("G0"));
    assert!(!stale.exists());
    assert!(mine.exists(), "a file of the user's own is left alone");
    assert_eq!(h.app.options.confirmations.earlier_gcode, EarlierGcode::Ask);
    // Archive, remembered: the earlier file moved aside, and not asked next time
    h.app.save_over_earlier(&form, post.clone(), path.clone());
    h.app.answer_earlier(Some(EarlierGcode::Archive), true);
    assert_eq!(
        h.app.options.confirmations.earlier_gcode,
        EarlierGcode::Archive
    );
    let archived =
        |dir: &std::path::Path| std::fs::read_dir(dir.join("Archive")).map_or(0, Iterator::count);
    assert_eq!(archived(&dir), 1, "{}", h.app.status.message);
    assert!(
        h.app.status.message.contains("Archive"),
        "{}",
        h.app.status.message
    );
    // a second archive in the same second gets a folder of its own
    h.app.save_over_earlier(&form, post, path.clone());
    assert!(h.app.earlier_ask.is_none(), "not asked once remembered");
    assert_eq!(archived(&dir), 2);
    assert!(path.exists());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn answering_dont_save_keeps_the_job_unsaved_until_something_replaces_it() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 20.0);
    assert!(h.app.session.is_dirty(&h.app.history));
    // No at "Save changes?", then the file picker cancelled (or the file would not open):
    // the job is still there and still unsaved, so it is asked about again and kept safe
    assert!(h.app.discard_answered(&rfd::MessageDialogResult::No));
    assert!(
        h.app.session.is_dirty(&h.app.history),
        "backing out after No must not mark the work saved"
    );
    assert!(
        h.app
            .session
            .title("Cut Craft", &h.app.history)
            .contains('*')
    );
    assert!(!h.app.discard_answered(&rfd::MessageDialogResult::Cancel));
    assert!(h.app.session.is_dirty(&h.app.history));
    // a file that will not open leaves the job unsaved too
    let missing = std::env::temp_dir().join("cut-craft-no-such-job.ccjob");
    h.app.open_path(&missing);
    assert!(h.app.session.is_dirty(&h.app.history));
}

#[test]
fn save_changes_names_just_the_file_and_its_buttons_say_what_they_do() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 20.0);
    assert_eq!(h.app.save_changes_question(), "Save changes to Untitled?");
    h.app.session.path = Some(std::path::PathBuf::from("/jobs/sign.ccjob"));
    assert_eq!(h.app.save_changes_question(), "Save changes to sign.ccjob?");
    // Don't Save lets the work go; Cancel keeps it
    let custom = |c: &str| rfd::MessageDialogResult::Custom(c.to_owned());
    assert!(h.app.discard_answered(&custom(super::actions::DONT_SAVE)));
    assert!(!h.app.discard_answered(&custom("Cancel")));
}

#[test]
fn save_as_suggests_the_jobs_own_name_in_its_own_folder() {
    let mut h = Harness::new();
    assert_eq!(
        h.app.save_as_suggestion(),
        (None, "untitled.ccjob".to_owned())
    );
    h.app.session.path = Some(std::path::PathBuf::from("/jobs/garden sign.ccjob"));
    assert_eq!(
        h.app.save_as_suggestion(),
        (
            Some(std::path::PathBuf::from("/jobs")),
            "garden sign.ccjob".to_owned()
        )
    );
}

#[test]
fn opening_a_job_from_a_newer_version_warns_which_settings_this_one_will_leave_out() {
    let dir = fresh_dir("newer");
    let path = dir.join("newer.ccjob");
    let mut json: serde_json::Value =
        serde_json::from_slice(&cut_craft_core::file::save_job(&Document::default())).unwrap();
    json["document"]["hologram"] = serde_json::json!(true);
    std::fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();
    let mut h = Harness::at_start();
    assert!(h.app.open_path(&path));
    assert_eq!(h.app.screen, Screen::Job);
    let told = &h.app.status.message;
    assert!(
        told.contains("Warning") && told.contains("newer version") && told.contains("hologram"),
        "{told}"
    );
    // saving says again, as it happens, what was left out
    h.app.save_to(&dir.join("here.ccjob"));
    let told = &h.app.status.message;
    assert!(
        told.starts_with("Saved") && told.contains("Warning") && told.contains("hologram"),
        "{told}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_file_named_on_the_command_line_opens_at_startup() {
    let dir = fresh_dir("argv");
    let path = dir.join("sign.ccjob");
    std::fs::write(&path, cut_craft_core::file::save_job(&Document::default())).unwrap();
    let mut h = Harness::at_start();
    h.app.open_given(Some(&path));
    assert_eq!(h.app.screen, Screen::Job);
    assert_eq!(h.app.session.path.as_deref(), Some(path.as_path()));
    // nothing named: the start page as before
    let mut h = Harness::at_start();
    h.app.open_given(None);
    assert_eq!(h.app.screen, Screen::Start);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn save_anyway_is_asked_in_the_window_not_by_a_dialog_that_can_hide_behind_it() {
    // a 0.6.11 user: the save question sometimes opened behind Cut Craft's window. Linux
    // shows rfd's questions as zenity windows of their own, not kept above Cut Craft's
    let mut h = ready_for_output();
    let form = one_file_form();
    let post = h.app.form_post(&form).unwrap();
    h.app.save_checks = vec![super::SaveChecks {
        plan: h.app.plan(&form, &post).unwrap(),
        changes: form.changes,
        revision: h.app.history.revision(),
        machine: h.app.machines.active,
        found: Some(vec![cut_craft_core::preflight::Finding {
            severity: cut_craft_core::preflight::Severity::Warning,
            message: "Air cutting".to_owned(),
            check: cut_craft_core::preflight::Check::AirCutting,
        }]),
    }];
    // asked over the window rather than by the system (a test opening one panics)
    assert!(
        h.app.ready_to_save(&form, false).is_none(),
        "waits for the answer"
    );
    assert!(h.app.save_anyway.is_some());
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Save the toolpaths anyway?").is_some());
    assert!(h.text_at("• Air cutting").is_some());
    h.click(h.text_at("Don't Save").unwrap(), Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.app.save_anyway.is_none());
    assert_eq!(h.app.status.message, "Toolpaths not saved");
    // answered Save Anyway, the same save goes on without asking again
    assert!(h.app.ready_to_save(&form, false).is_none());
    h.app.checks_accepted = vec!["Air cutting".to_owned()];
    assert!(h.app.ready_to_save(&form, false).is_some(), "agreed to");
    assert!(h.app.checks_accepted.is_empty(), "only for that save");
}

#[test]
fn save_anyway_counts_only_for_the_save_it_answered() {
    // an answer left over from a save that stopped early (refused, or the checks still
    // running) must not let a later save skip its question
    let mut h = ready_for_output();
    let form = one_file_form();
    let post = h.app.form_post(&form).unwrap();
    let (plan, revision, machine) = (
        h.app.plan(&form, &post).unwrap(),
        h.app.history.revision(),
        h.app.machines.active,
    );
    let checks = |severity, message: &str| {
        vec![super::SaveChecks {
            plan: plan.clone(),
            changes: form.changes,
            revision,
            machine,
            found: Some(vec![cut_craft_core::preflight::Finding {
                severity,
                message: message.to_owned(),
                check: cut_craft_core::preflight::Check::AirCutting,
            }]),
        }]
    };
    h.app.save_checks = checks(cut_craft_core::preflight::Severity::Error, "Too deep");
    h.app.checks_accepted = vec!["Too deep".to_owned()];
    assert!(h.app.ready_to_save(&form, false).is_none(), "refused");
    assert!(
        h.app.checks_accepted.is_empty(),
        "used up by the save it was for"
    );
    h.app.save_checks = checks(cut_craft_core::preflight::Severity::MustAsk, "Collet hits");
    assert!(h.app.ready_to_save(&form, false).is_none(), "asks again");
    assert!(h.app.save_anyway.is_some());
}

#[test]
fn new_and_close_give_a_blank_job_and_classic_startup_flow_asks_first() {
    let mut h = Harness::at_start();
    h.app.options.default_job = Some(cut_craft_core::document::Job {
        thickness: 12.0,
        ..Default::default()
    });
    h.app.invoke(&h.ctx, "new");
    assert_eq!(h.app.screen, Screen::Job);
    assert!(h.app.form.is_none());
    assert_eq!(h.app.doc.job.thickness, 12.0, "made with the Job Defaults");
    h.app.invoke(&h.ctx, "close");
    assert_eq!(h.app.screen, Screen::Job, "no start page to go back to");
    // classic: Job Setup first, Close back to the start page
    h.app.options.job_defaults.classic_startup = true;
    h.app.invoke(&h.ctx, "new");
    assert_eq!(h.app.screen, Screen::NewJob);
    assert!(h.app.form.as_ref().is_some_and(|f| f.new_job));
    h.app.job_setup_ok();
    h.app.invoke(&h.ctx, "close");
    assert_eq!(h.app.screen, Screen::Start);
}

#[test]
fn job_defaults_page_changes_new_jobs_once_applied() {
    let mut h = Harness::new();
    h.app.open_job_defaults();
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Classic Startup Flow").is_some());
    assert!(
        h.text_at("XY Datum Position").is_some(),
        "Job Setup's settings too"
    );
    let (_, mut params, _) = h.app.job_defaults_state();
    params["size"]["width"] = 1220.0.into();
    h.app.job_defaults_edited(params.clone());
    assert!(h.app.options_dirty(), "waits for Apply");
    assert_eq!(h.app.options.default_job, None);
    assert_eq!(h.app.doc.job.width, 600.0, "the open job is left as it is");
    // a refused change is kept as typed, with why
    let mut wrong = params.clone();
    wrong["gaps"]["plunge"] = 50.0.into();
    h.app.job_defaults_edited(wrong);
    assert!(h.app.job_defaults_state().2.is_some());
    h.app.job_defaults_edited(params);
    h.app
        .options_button(&h.ctx.clone(), options_apply::Button::Apply);
    assert_eq!(h.app.options.default_job.map(|j| j.width), Some(1220.0));
    assert_eq!(h.app.doc.job.width, 600.0, "still the open job's");
    h.app.invoke(&h.ctx, "new");
    assert_eq!(h.app.doc.job.width, 1220.0, "the next job's");
}

/// An add-ons folder holding the Depth add-on, as Gadgets ▸ Add-ons installs it.
fn depth_installed(name: &str) -> std::path::PathBuf {
    let dir = fresh_dir(name);
    let depth = dir.join("depth");
    std::fs::create_dir_all(&depth).unwrap();
    std::fs::write(
        depth.join("addon.toml"),
        "id = \"depth\"\nname = \"Depth (Relief from Photo)\"\nkind = \"tool\"\nversion = \"0.1.0\"",
    )
    .unwrap();
    std::fs::write(depth.join("installed.json"), "{}").unwrap();
    dir
}

/// Gadgets ▸ My Gadgets in `menus`: each item's name, action and whether it can be picked.
fn my_gadgets_in(menus: &gui::MenuSystem) -> Vec<(String, String, bool)> {
    let gadgets = &menus
        .menus
        .iter()
        .find(|m| m.name == "Gadgets")
        .unwrap()
        .items;
    gadgets
        .iter()
        .find(|i| i.name == gui::MY_GADGETS)
        .expect("a My Gadgets submenu")
        .children
        .iter()
        .map(|i| (i.name.clone(), i.action.clone(), i.is_enabled))
        .collect()
}

#[test]
fn my_gadgets_lists_what_is_installed_and_how_to_add_more() {
    let mut h = Harness::new();
    let nothing = my_gadgets_in(&h.app.menus);
    assert_eq!(
        nothing[0],
        ("Nothing installed yet".to_owned(), String::new(), false)
    );
    let named = |items: &[(String, String, bool)], name: &str| {
        items.iter().any(|(n, _, enabled)| n == name && *enabled)
    };
    assert!(named(&nothing, "Open My Gadgets Folder"));
    let (last, action, _) = nothing.last().unwrap();
    assert_eq!((last.as_str(), action.as_str()), ("Add-ons...", "addons"));
    assert!(h.app.available(gui::OPEN_GADGETS_FOLDER));
    assert_eq!(h.app.gadget_tools(), [""; 0]);

    let dir = depth_installed("my-gadgets");
    h.app.addons = crate::addons::Addons::new(Some(dir.clone()));
    h.app.refresh_installed();
    for menus in [&h.app.menus, &h.app.start_menus] {
        let installed = my_gadgets_in(menus);
        assert_eq!(
            installed[0],
            (
                "Depth (Relief from Photo) \u{26a0} (not official)".to_owned(),
                "relief_from_photo".to_owned(),
                true
            ),
            "clicking it opens the tool it brings; not signed, so marked"
        );
        assert!(!named(&installed, "Nothing installed yet"));
        assert!(named(&installed, "Add-ons..."));
    }
    assert_eq!(h.app.gadget_tools(), ["relief_from_photo"]);

    // removed: gone from the menu at once
    std::fs::remove_dir_all(&dir).unwrap();
    h.app.refresh_installed();
    assert_eq!(my_gadgets_in(&h.app.menus)[0].0, "Nothing installed yet");
}

#[test]
fn my_gadgets_marks_what_is_not_official() {
    let mut h = Harness::new();
    h.app.my_gadgets = vec![
        ("Box Maker".to_owned(), "gadget_box_maker"),
        ("Star Burst".to_owned(), "gadget_star_burst"),
    ];
    let code = crate::trust::Code::gadget("gadget_star_burst", "Star Burst", b"");
    let unsigned = crate::gadgets::Unsigned {
        code,
        waiting: None,
    };
    h.app.unsigned_gadgets = [("gadget_star_burst".to_owned(), unsigned)].into();
    h.app.installed_addons = vec![(
        "Depth (Relief from Photo)".to_owned(),
        Some("relief_from_photo"),
        true,
    )];
    let names: Vec<String> = h
        .app
        .installed_items()
        .into_iter()
        .map(|(n, _)| n)
        .collect();
    assert_eq!(
        names,
        [
            "Box Maker",
            "Star Burst \u{26a0} (not official)",
            "Depth (Relief from Photo)"
        ]
    );
}

#[test]
fn a_gadgets_section_shows_in_every_layout_once_something_is_installed() {
    use crate::options::WindowLayout;
    let dir = depth_installed("gadgets-section");
    let harness = |layout: WindowLayout, installed: bool| {
        let mut h = Harness::new();
        h.app.options.general.layout = layout;
        if installed {
            h.app.addons = crate::addons::Addons::new(Some(dir.clone()));
            h.app.refresh_installed();
        }
        h.frame(vec![]);
        h.frame(vec![]);
        h
    };
    // Classic: a section at the foot of the Design page, under the sub-tabs
    let foot = |installed: bool| {
        let mut h = harness(WindowLayout::Classic, installed);
        let page = h.text_at("Create Vectors").expect("the Design page");
        h.scroll_to(page, "No such text");
        h.texts_at("Gadgets").len()
    };
    assert_eq!(foot(true), foot(false) + 1, "Classic: the section");
    let tabs = |installed: bool| {
        harness(WindowLayout::Ribbon, installed)
            .texts_at("Gadgets")
            .len()
    };
    assert_eq!(tabs(true), tabs(false) + 1, "Ribbon: the tab");
    // the Ribbon's Gadgets tab holds the installed tools
    let mut h = harness(WindowLayout::Ribbon, true);
    h.app.ribbon_tab = RIBBON_TABS.iter().position(|&t| t == "Gadgets").unwrap();
    h.frame(vec![]);
    assert_eq!(
        h.texts_at("Gadgets").len(),
        tabs(false) + 2,
        "the tab, and its section of tools"
    );
    // with nothing installed it is not there to be on
    let mut h = harness(WindowLayout::Ribbon, false);
    h.app.ribbon_tab = RIBBON_TABS.iter().position(|&t| t == "Gadgets").unwrap();
    h.frame(vec![]);
    assert_eq!(h.app.ribbon_tab, 0);
    // the Rail: a last button, after Favourites, opening their list
    let mut h = harness(WindowLayout::Rail, true);
    assert!(h.text_at("Relief from Photo").is_none(), "no list open");
    h.app.rail_gadgets = true;
    h.frame(vec![]);
    assert!(h.text_at("Relief from Photo").is_some());
    let mut h = harness(WindowLayout::Rail, false);
    h.app.rail_gadgets = true;
    h.frame(vec![]);
    assert!(!h.app.rail_gadgets, "nothing installed: no list");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_recovery_copy_that_could_not_be_written_is_tried_again() {
    let dir = fresh_dir("recovery-retry");
    // a file where its folder should be: the first copy cannot be written
    let missing = dir.join("not-yet");
    std::fs::write(&missing, "in the way").unwrap();
    let mut h = Harness::new();
    h.app.recovery_path = Some(missing.join("retry.ccjob"));
    h.rectangle(10.0, 10.0, 20.0, 20.0);
    let written = |h: &mut Harness| {
        for _ in 0..500 {
            h.frame(vec![]);
            if h.app.autosave.is_none() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("the recovery copy never finished");
    };
    h.time += AUTOSAVE_SECONDS;
    h.frame(vec![]);
    assert!(h.app.autosave.is_some(), "a copy is being written");
    written(&mut h);
    assert!(
        h.app.status.message.contains("recovery copy"),
        "{}",
        h.app.status.message
    );
    std::fs::remove_file(&missing).unwrap();
    // not at once, but once the interval is up again
    h.frame(vec![]);
    assert!(h.app.autosave.is_none(), "retries are spaced out");
    h.time += AUTOSAVE_SECONDS;
    h.frame(vec![]);
    assert!(h.app.autosave.is_some(), "tried again with no new change");
    written(&mut h);
    assert!(missing.join("retry.ccjob").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_anyway_agrees_only_to_the_problems_it_showed() {
    // an answer to one list of problems is no answer to a list with more in it
    let mut h = ready_for_output();
    let form = one_file_form();
    let post = h.app.form_post(&form).unwrap();
    let finding = |message: &str| cut_craft_core::preflight::Finding {
        severity: cut_craft_core::preflight::Severity::Warning,
        message: message.to_owned(),
        check: cut_craft_core::preflight::Check::AirCutting,
    };
    h.app.save_checks = vec![super::SaveChecks {
        plan: h.app.plan(&form, &post).unwrap(),
        changes: form.changes,
        revision: h.app.history.revision(),
        machine: h.app.machines.active,
        found: Some(vec![finding("Air cutting"), finding("Slow feed")]),
    }];
    h.app.checks_accepted = vec!["Air cutting".to_owned()];
    assert!(
        h.app.ready_to_save(&form, false).is_none(),
        "Slow feed was never shown"
    );
    assert!(h.app.save_anyway.is_some(), "asked again");
    // answered for both, it saves
    h.app.save_anyway = None;
    h.app.checks_accepted = vec!["Slow feed".to_owned(), "Air cutting".to_owned()];
    assert!(h.app.ready_to_save(&form, false).is_some());
}

#[test]
fn a_new_job_forgets_a_save_anyway_answer() {
    let mut h = ready_for_output();
    h.app.checks_accepted = vec!["Air cutting".to_owned()];
    h.app.reset_job();
    assert!(h.app.checks_accepted.is_empty(), "forgotten");
}
