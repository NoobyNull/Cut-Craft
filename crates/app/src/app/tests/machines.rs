//! Machines: Machine Configuration, post processors, First Time Setup and sending files to the
//! machine.

use super::*;

#[test]
fn a_drill_bank_toolpath_drills_with_the_machines_bank_and_says_when_there_is_none() {
    use cut_craft_core::drill_bank::DrillBank;
    use cut_craft_core::geometry::{Path, Point};
    let mut h = Harness::new();
    let mut c = cut_craft_core::Change::builder(&h.app.doc);
    let holes: Vec<_> = (0..3)
        .map(|i| {
            c.add(Path::circle(
                Point::new(20.0 + 32.0 * f64::from(i), 20.0),
                2.5,
            ))
        })
        .collect();
    c.select(holes);
    h.app.doc.apply(&c.finish()).unwrap();
    // no machine: the form opens and the toolpath says where the bank is set up
    h.app.invoke(&h.ctx, "drill_bank_toolpath");
    let params = h.app.form.as_ref().expect("its form").params.clone();
    let err = h.app.apply("drill_bank_toolpath", &params).unwrap_err();
    assert!(err.contains("Machine Configuration"), "{err}");
    // a machine with a bank of three: one plunge drills all three
    h.app.machines.add();
    h.app.machines.active_mut().unwrap().drill_bank = Some(DrillBank::new(3, 0, 32.0, 5.0));
    h.app.invoke(&h.ctx, "drill_bank_toolpath");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("drill_bank_toolpath", &params).unwrap();
    h.app.form = None;
    let (&id, toolpath) = h.app.doc.toolpaths.iter().next().unwrap();
    assert_eq!(toolpath.drill_bank.as_ref().unwrap().hits.len(), 1);
    // editing it again picks up the bank as the machine has it now
    h.app.machines.active_mut().unwrap().drill_bank = Some(DrillBank::new(2, 0, 32.0, 5.0));
    h.app.selected_toolpath = Some(id);
    h.app.invoke(&h.ctx, "edit_toolpath");
    let form = h.app.form.as_ref().expect("the toolpath's form");
    assert_eq!(
        form.params["bank"]["horizontal"].as_array().unwrap().len(),
        2
    );
    // Cut Craft's own posts have no drill bank blocks: saving says so
    let post = h.app.output_post().unwrap();
    let out = std::env::temp_dir().join(format!("cut-craft-bank-{}.nc", std::process::id()));
    let err = files::save_through(&post, &h.app.slots(), &h.app.doc, &out, None).unwrap_err();
    assert!(err.contains("SETUP_DRILL_BANK"), "{err}");
    let _ = std::fs::remove_file(out);
}

#[test]
fn any_toolpath_rest_machines_after_the_tools_named_in_its_form() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 40.0, 40.0);
    h.app.invoke(&h.ctx, "pocket_toolpath");
    // the section, drawn on its own (it is below the fold of the pocket form)
    let ctx = egui::Context::default();
    let mut section = serde_json::json!({});
    let out = headless(ctx.run_ui(RawInput::default(), |ui| {
        egui::CentralPanel::default().show(ui, |ui| {
            rest_section(
                ui,
                &mut section,
                &h.app.tools,
                &[],
                cut_craft_core::document::Units::Mm,
            )
        });
    }));
    let drawn = |text: &str| {
        out.shapes
            .iter()
            .any(|c| matches!(&c.shape, egui::Shape::Text(t) if t.galley.text() == text))
    };
    assert!(drawn("Rest Machining") && drawn("Add a tool…"));
    let mut params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("pocket_toolpath", &params).unwrap();
    // how far it cuts: a pocket links its rings at depth, so moves alone undercount it
    let cut_length = |t: &cut_craft_core::toolpath::Toolpath| -> f64 {
        t.moves
            .windows(2)
            .filter(|w| w[1].kind == cut_craft_core::toolpath::MoveKind::Cut)
            .map(|w| w[0].to.distance(w[1].to))
            .sum()
    };
    let whole = cut_length(h.app.doc.toolpaths.values().last().unwrap());
    let mut big = h.app.form.as_ref().unwrap().params["tool"].clone();
    big["diameter"] = Value::from(params["tool"]["diameter"].as_f64().unwrap() * 4.0);
    params["rest"] = serde_json::json!({ "after": [big], "minimum_detail": 0.0, "overlap": 0.0 });
    h.app.apply("pocket_toolpath", &params).unwrap();
    let rest = h.app.doc.toolpaths.values().last().unwrap();
    assert!(
        cut_length(rest) < whole,
        "{} of {whole} mm cut",
        cut_length(rest)
    );
    assert!(
        cut_craft_core::rest::Rest::of(&rest.params).is_some(),
        "kept to recalculate"
    );
}

#[test]
fn f11_designs_f12_machines_and_switch_layout_swaps() {
    let mut h = Harness::new();
    assert_eq!(h.app.pages, Pages::design_layout());
    h.frame(vec![key(Key::F12)]);
    assert_eq!(h.app.pages, Pages::machining_layout());
    h.frame(vec![key(Key::F11)]);
    assert_eq!(h.app.pages, Pages::design_layout());
    h.app.invoke(&h.ctx, "interface_layout");
    assert_eq!(
        h.app.pages,
        Pages::machining_layout(),
        "the Switch Layout button"
    );
}

#[test]
fn an_installed_post_is_kept_and_the_machine_saves_through_it() {
    let mut h = Harness::new();
    let dir = std::env::temp_dir().join(format!("cut-craft-app-posts-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    h.app.posts = cut_craft_machines::Library::new(Some(dir.join("posts")));
    let file = dir.join("mine.pp");
    std::fs::create_dir_all(&dir).unwrap();
    let text = cut_craft_post::BUILT_IN[0]
        .1
        .replace("Cut Craft FluidNC ATC (mm)", "My Router (mm)");
    std::fs::write(&file, text).unwrap();
    h.app.install_post(&file);
    assert_eq!(h.app.machines.machines.len(), 1, "a machine to use it");
    assert_eq!(h.app.output_post().unwrap().name, "My Router (mm) (*.nc)");
    // installed again: kept beside the first as a dated copy, the machine left as it was
    h.app.install_post(&file);
    let said = h.app.status.message.clone();
    assert!(
        said.starts_with("Installed as My Router (mm) (copy 1) ("),
        "{said}"
    );
    assert_eq!(h.app.output_post().unwrap().name, "My Router (mm) (*.nc)");
    std::fs::remove_file(&file).unwrap();
    assert!(
        h.app.output_post().is_ok(),
        "kept in the custom posts, not read from where it was"
    );
    // the Machine menu opens its windows; the online ones stay greyed
    for id in [
        "machine_configuration",
        "post_processor_management",
        "add_machine",
    ] {
        assert!(h.app.available(id), "{id}");
        h.app.invoke(&h.ctx, id);
        h.frame(vec![]);
    }
    assert!(h.app.machine_window.is_some() && h.app.post_window.is_some());
    assert!(h.text_at("Associated Post-Processors").is_some());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn toolpaths_save_through_the_default_fluidnc_post_until_one_is_installed() {
    use cut_craft_core::document::{Along, Job};
    let mut h = Harness::new();
    assert_eq!(
        h.app.output_post().unwrap().name,
        "Cut Craft FluidNC ATC (mm) (*.nc)"
    );
    h.app.doc.job = Job::rotary(300.0, 50.0, Along::X);
    assert_eq!(
        h.app.output_post().unwrap().name,
        "Cut Craft FluidNC ATC Rotary A, G93 (mm) (*.nc)"
    );
}

#[test]
fn machine_configuration_says_when_changes_wait_and_apply_keeps_them() {
    let mut h = Harness::new();
    // tall enough for the whole window, buttons and all
    h.window = vec2(1280.0, 2000.0);
    h.app.invoke(&h.ctx, "machine_configuration");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at(super::options_apply::ALL_APPLIED).is_some());
    let before = h.app.machines.machines.len();
    h.app.machine_window.as_mut().unwrap().draft.add();
    h.frame(vec![]);
    assert!(
        h.text_at(super::options_apply::NOT_APPLIED).is_some(),
        "{:?}",
        h.texts()
    );
    assert_eq!(h.app.machines.machines.len(), before, "not kept yet");
    click_button(&mut h, "Apply");
    assert_eq!(h.app.machines.machines.len(), before + 1, "kept");
    assert!(h.app.machine_window.is_some(), "still open");
    h.frame(vec![]);
    assert!(h.text_at(super::options_apply::ALL_APPLIED).is_some());
}

#[test]
fn an_offline_package_installs_its_machines_once() {
    let mut h = Harness::new();
    let package =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../machines/genmitsu.json");
    h.app.install_package(&package);
    let installed = h.app.machines.machines.len();
    assert!(installed > 0, "the package's machines are added");
    assert!(
        h.app
            .machines
            .machines
            .iter()
            .all(|m| m.name.starts_with("Genmitsu"))
    );
    h.app.install_package(&package);
    assert_eq!(
        h.app.machines.machines.len(),
        installed,
        "installing again adds none"
    );
    h.app.install_package(&package.with_file_name("README.md"));
    assert_eq!(
        h.app.machines.machines.len(),
        installed,
        "not a package: nothing added"
    );
    for id in ["install_offline_package", "search_for_machine_online"] {
        assert!(h.app.available(id), "{id}");
    }
}

#[test]
fn rotary_jobs_need_a_4th_axis_and_save_through_the_post_wrapping_their_side() {
    use cut_craft_core::document::{Along, Job};
    let mut h = Harness::new();
    let package =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../machines/genmitsu.json");
    h.app.install_package(&package);
    let wraps = |app: &CutCraftApp| app.output_post().map(|p| p.wraps().map(str::to_owned));
    assert_eq!(wraps(&h.app), Ok(None), "a flat job: the flat default");
    h.app.doc.job = Job::rotary(300.0, 50.0, Along::X);
    let why = wraps(&h.app).unwrap_err();
    assert!(why.contains("Standard GRBL supports only 3 axes"), "{why}");
    h.app.machines.active_mut().unwrap().firmware = Some(cut_craft_machines::Firmware::GrblHal);
    let why = wraps(&h.app).unwrap_err();
    assert!(why.contains("tick its 4th axis"), "{why}");
    let names: Vec<String> = h.app.posts.entries().into_iter().map(|e| e.name).collect();
    let machine = h.app.machines.active_mut().unwrap();
    machine.rotary = true;
    machine.add_rotary_posts(&names);
    let firmware = machine.firmware();
    machine.firmware = Some(cut_craft_machines::Firmware::Grbl);
    let why = wraps(&h.app).unwrap_err();
    assert!(why.contains("Standard GRBL supports only 3 axes"), "{why}");
    // the firmware is said first, with the tick off or no posts at all
    let machine = h.app.machines.active_mut().unwrap();
    let (posts, ticked) = (std::mem::take(&mut machine.posts), machine.rotary);
    machine.rotary = false;
    let why = wraps(&h.app).unwrap_err();
    assert!(why.contains("Standard GRBL supports only 3 axes"), "{why}");
    let machine = h.app.machines.active_mut().unwrap();
    (machine.posts, machine.rotary) = (posts, ticked);
    h.app.machines.active_mut().unwrap().firmware = Some(firmware)
        .filter(|f| f.has_a_axis())
        .or(Some(cut_craft_machines::Firmware::GrblHal));
    assert_eq!(
        wraps(&h.app),
        Ok(Some("Y".into())),
        "Y goes round a cylinder along X"
    );
    h.app.doc.job = Job::rotary(300.0, 50.0, Along::Y);
    assert_eq!(wraps(&h.app), Ok(Some("X".into())));
    h.app.doc.job = Job::default();
    assert_eq!(wraps(&h.app), Ok(None), "flat jobs keep the flat default");
}

#[test]
fn a_crash_is_not_offered_when_crash_logs_are_not_to_be_sent() {
    let mut h = Harness::new();
    h.app.options.bug_reports.crash_logs = crate::options::CrashLogs::DontSend;
    let ctx = h.ctx.clone();
    h.app.offer_crash(&ctx, CRASH.into());
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(!h.app.reporter.is_open(), "no report");
    assert!(
        h.app
            .status
            .message
            .starts_with("Cut Craft closed with an error last time."),
        "{}",
        h.app.status.message
    );
}

#[test]
fn a_post_that_cannot_change_tools_saves_a_file_for_each_tool_named_after_it() {
    let mut h = Harness::new();
    two_tools(&mut h);
    let plain = cut_craft_post::Post::parse(
        "POST_NAME = \"Plain (*.nc)\"\nFILE_EXTENSION = \"nc\"\nUNITS = \"MM\"\n\
         begin HEADER\n\"G21\"\nbegin RAPID_MOVE\n\"G0 [X] [Y] [Z]\"\n\
         begin FEED_MOVE\n\"G1 [X] [Y] [Z]\"\n",
    )
    .unwrap();
    let form = SaveForm {
        mode: crate::files::SaveMode::VisibleToOneFile,
        group: false,
        post: None,
        add_side: false,
        changes: crate::files::ToolChanges::ToolChanger,
        job_sheet: true,
    };
    let dir = std::env::temp_dir().join(format!("cut-craft-per-tool-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let done = h
        .app
        .save_toolpaths_to(&form, &plain, &dir.join("sign.nc"))
        .unwrap();
    assert!(done.starts_with("Saved 2 toolpath files"), "{done}");
    for name in ["sign_1_6_mm_End_Mill.nc", "sign_2_3_mm_End_Mill.nc"] {
        assert!(dir.join(name).exists(), "{name}");
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn short_names_never_send_two_files_under_one_name() {
    use cut_craft_machines::send::{Folder, Store};
    let mut h = Harness::new();
    two_tools(&mut h);
    let card = std::env::temp_dir().join(format!("cut-craft-short-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&card);
    std::fs::create_dir_all(&card).unwrap();
    h.app.machines.add();
    let machine = h.app.machines.active_mut().unwrap();
    machine.send_to.folder = card.display().to_string();
    machine.send_to.short_names = true;
    h.app.session.path = Some(std::path::PathBuf::from("/jobs/Oak sign front.ccjob"));
    // a post that can't change tools writes one file per tool, Oak sign front_1_6_mm... and
    // Oak sign front_2_3_mm...: cut to 8 letters both are Oaksignf.nc
    let plain = cut_craft_post::Post::parse(
        "POST_NAME = \"Plain (*.nc)\"\nFILE_EXTENSION = \"nc\"\nUNITS = \"MM\"\n\
         begin HEADER\n\"G21\"\nbegin RAPID_MOVE\n\"G0 [X] [Y] [Z]\"\n\
         begin FEED_MOVE\n\"G1 [X] [Y] [Z]\"\n",
    )
    .unwrap();
    let form = SaveForm {
        mode: crate::files::SaveMode::VisibleToOneFile,
        group: false,
        post: None,
        add_side: false,
        changes: crate::files::ToolChanges::ToolChanger,
        job_sheet: true,
    };
    let store = Folder::new(&card);
    let never = |_: &[String]| panic!("nothing there to replace yet");
    let done = h.app.send_to(&form, &plain, &store, &never).unwrap();
    assert!(done.starts_with("Sent 3 files"), "{done}");
    let folder = store.list(&[]).unwrap()[0].name.clone();
    let names: Vec<String> = store
        .list(&[folder])
        .unwrap()
        .into_iter()
        .map(|i| i.name)
        .collect();
    assert_eq!(
        names.len(),
        3,
        "two tool files and the sheet, none lost: {names:?}"
    );
    let _ = std::fs::remove_dir_all(card);
}

#[test]
fn nothing_is_sent_while_the_machine_runs_a_job() {
    use cut_craft_machines::send::{Folder, Item, Store};
    /// A card on a machine that is cutting.
    struct Cutting(Folder);
    impl Store for Cutting {
        fn place(&self) -> String {
            self.0.place()
        }
        fn busy(&self) -> Option<String> {
            Some("The machine is running a job".into())
        }
        fn list(&self, at: &[String]) -> Result<Vec<Item>, String> {
            self.0.list(at)
        }
        fn upload(&self, at: &[String], name: &str, bytes: &[u8]) -> Result<(), String> {
            self.0.upload(at, name, bytes)
        }
        fn exists(&self, at: &[String], name: &str) -> Result<bool, String> {
            self.0.exists(at, name)
        }
        fn rename(&self, at: &[String], from: &str, to: &str) -> Result<(), String> {
            self.0.rename(at, from, to)
        }
        fn delete(&self, at: &[String], name: &str) -> Result<(), String> {
            self.0.delete(at, name)
        }
        fn make_folder(&self, at: &[String], name: &str) -> Result<(), String> {
            self.0.make_folder(at, name)
        }
    }
    let mut h = Harness::new();
    two_tools(&mut h);
    let card = std::env::temp_dir().join(format!("cut-craft-cutting-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&card);
    std::fs::create_dir_all(card.join("Sign")).unwrap();
    h.app.machines.add();
    h.app.machines.active_mut().unwrap().send_to.folder = card.display().to_string();
    let form = SaveForm {
        mode: crate::files::SaveMode::VisibleToOneFile,
        group: false,
        post: None,
        add_side: false,
        changes: crate::files::ToolChanges::ToolChanger,
        job_sheet: false,
    };
    let post = h.app.form_post(&form).unwrap();
    let store = Cutting(Folder::new(&card));
    let never = |_: &[String]| panic!("not asked");
    let refused = h.app.send_to(&form, &post, &store, &never).unwrap_err();
    assert!(
        refused.starts_with("Not sent: The machine is running a job"),
        "{refused}"
    );
    let mut files =
        super::send::MachineFiles::new("Router", Box::new(Cutting(Folder::new(&card))), false);
    files.delete("Sign");
    assert!(card.join("Sign").is_dir(), "kept");
    assert_eq!(
        files.message,
        Err("The machine is running a job".to_owned())
    );
    let _ = std::fs::remove_dir_all(card);
}

#[test]
fn machine_files_uploads_renames_makes_folders_and_deletes_only_on_the_second_ask() {
    let card = std::env::temp_dir().join(format!("cut-craft-files-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&card);
    std::fs::create_dir_all(card.join("Sign")).unwrap();
    let local = card.with_extension("local.nc");
    std::fs::write(&local, "G0 X0").unwrap();
    let store = Box::new(cut_craft_machines::send::Folder::new(&card));
    let mut files = super::send::MachineFiles::new("Router", store, false);
    files.open_folder("Sign");
    files.upload(std::slice::from_ref(&local));
    assert_eq!(files.message, Ok("Uploaded 1 file".to_owned()));
    let uploaded = crate::app::file_name(&local);
    files.rename(&uploaded, "Front: v2.nc");
    assert!(
        card.join("Sign/Front_ v2.nc").exists(),
        "{:?}",
        files.message
    );
    files.make_folder("Old jobs");
    assert!(card.join("Sign/Old jobs").is_dir());
    files.delete("Old jobs");
    let names: Vec<String> = files
        .items
        .clone()
        .unwrap()
        .into_iter()
        .map(|i| i.name)
        .collect();
    assert_eq!(names, ["Front_ v2.nc"]);
    let _ = std::fs::remove_dir_all(card);
    let _ = std::fs::remove_file(local);
}

#[test]
fn a_machine_without_a_files_folder_says_where_to_set_it() {
    let mut h = Harness::new();
    h.app.machines.add();
    h.app.invoke(&h.ctx, "machine_files");
    assert!(h.app.machine_files.is_none());
    assert_eq!(h.app.status.message, super::send::NO_MACHINE_FOLDER);
}

#[test]
fn a_machines_file_that_cannot_be_read_is_said_kept_aside_and_never_saved_over() {
    let mut h = Harness::new();
    let dir = std::env::temp_dir().join(format!("cut-craft-bad-machines-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("machines.json");
    let broken = b"{\"machines\": [{\"width\": \"wide\"}]}";
    std::fs::write(&path, broken).unwrap();
    h.app.load_machines_at(&path);
    assert_eq!(h.app.machines.machines, vec![]);
    let said = h.app.status.message.clone();
    assert!(said.contains("could not be read"), "{said}");
    assert!(said.contains("machines.json.bad"), "{said}");
    assert_eq!(
        std::fs::read(dir.join("machines.json.bad")).unwrap(),
        broken
    );
    // a machine added now is not saved over the file
    h.app.machines.add();
    h.app.machines_changed("Machine added");
    assert_eq!(std::fs::read(&path).unwrap(), broken);
    assert!(
        h.app.status.message.contains("not saved"),
        "{}",
        h.app.status.message
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn machine_files_asks_before_uploading_over_a_file_already_on_the_machine() {
    let card = std::env::temp_dir().join(format!("cut-craft-replace-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&card);
    std::fs::create_dir_all(&card).unwrap();
    let local = card.with_extension("sign.nc");
    std::fs::write(&local, "G0 X1").unwrap();
    let name = crate::app::file_name(&local);
    std::fs::write(card.join(&name), "G0 X0").unwrap();
    let store = Box::new(cut_craft_machines::send::Folder::new(&card));
    let mut files = super::send::MachineFiles::new("Router", store, false);
    let asked = std::cell::RefCell::new(vec![]);
    files.upload_asking(std::slice::from_ref(&local), &|names: &[String]| {
        asked.borrow_mut().extend_from_slice(names);
        false
    });
    assert_eq!(*asked.borrow(), std::slice::from_ref(&name));
    assert_eq!(std::fs::read_to_string(card.join(&name)).unwrap(), "G0 X0");
    assert!(files.message.is_err(), "{:?}", files.message);
    files.upload_asking(std::slice::from_ref(&local), &|_: &[String]| true);
    assert_eq!(std::fs::read_to_string(card.join(&name)).unwrap(), "G0 X1");
    let _ = std::fs::remove_dir_all(card);
    let _ = std::fs::remove_file(local);
}

#[test]
fn selecting_a_post_makes_it_the_one_the_machine_saves_through() {
    let mut h = Harness::new();
    h.app.machines.add();
    let first = cut_craft_post::Post::parse(cut_craft_post::BUILT_IN[0].1)
        .unwrap()
        .name;
    // another flat post, so it suits this flat job
    let second = (cut_craft_post::BUILT_IN.iter())
        .map(|(_, text)| cut_craft_post::Post::parse(text).unwrap())
        .find(|p| p.wraps().is_none() && p.name != first)
        .unwrap()
        .name;
    h.app.machines.active_mut().unwrap().posts = vec![first.clone()];
    h.app
        .post_outcome(cut_craft_machines::PostOutcome::Select(second.clone()));
    let machine = h.app.machines.active().unwrap();
    assert_eq!(machine.post(), Some(second.as_str()));
    assert_eq!(h.app.output_post().unwrap().name, second, "what Save uses");
    let machine = h.app.machines.active().unwrap();
    assert!(machine.posts.contains(&first), "the others stay listed");
    assert!(
        h.app.status.message.contains("now saves through"),
        "{}",
        h.app.status.message
    );
}

#[test]
fn machine_configuration_s_install_offline_package_adds_to_the_machines_being_edited() {
    let mut h = Harness::new();
    let file = std::env::temp_dir().join(format!("cut-craft-package-{}.json", std::process::id()));
    let mut package = cut_craft_machines::Machines::default();
    package.add();
    package.machines[0].name = "Packaged Router".into();
    package.save(&file).unwrap();
    h.app.invoke(&h.ctx, "machine_configuration");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Machine Configuration Management").is_some());
    h.app.machine_outcome_install(&file);
    let draft = &h.app.machine_window.as_ref().unwrap().draft;
    assert!(draft.machines.iter().any(|m| m.name == "Packaged Router"));
    assert!(
        h.app.status.message.contains("Installed 1"),
        "{}",
        h.app.status.message
    );
    let _ = std::fs::remove_file(file);
}

#[test]
fn selecting_a_post_that_does_not_suit_the_open_job_says_which_post_it_saves_through() {
    use cut_craft_core::document::{Along, Job};
    let mut h = Harness::new();
    h.app.machines.add();
    let entries = h.app.posts.entries();
    let load = |name: &str| h.app.posts.load(name).unwrap();
    let flat = (entries.iter())
        .find(|e| load(&e.name).wraps().is_none())
        .unwrap()
        .name
        .clone();
    let round = (entries.iter())
        .find(|e| load(&e.name).wraps() == Some("Y"))
        .unwrap()
        .name
        .clone();
    let machine = h.app.machines.active_mut().unwrap();
    machine.rotary = true;
    machine.firmware = Some(cut_craft_machines::Firmware::FluidNc);
    machine.posts = vec![round.clone()];
    // a rotary job: a flat post picked is the default for flat jobs only
    h.app.doc.job = Job::rotary(300.0, 50.0, Along::X);
    h.app
        .post_outcome(cut_craft_machines::PostOutcome::Select(flat.clone()));
    let message = h.app.status.message.clone();
    assert_eq!(h.app.output_post().unwrap().name, round);
    assert!(
        message.contains(&format!("saves flat jobs through {flat}")),
        "{message}"
    );
    assert!(
        message.contains(&format!(
            "this job wraps round, so it saves through {round}"
        )),
        "{message}"
    );
    // a flat job: the picked one is used, and said plainly
    h.app.doc.job = Job::default();
    h.app
        .post_outcome(cut_craft_machines::PostOutcome::Select(flat.clone()));
    assert_eq!(h.app.output_post().unwrap().name, flat);
    assert!(
        h.app
            .status
            .message
            .contains(&format!("now saves through {flat}")),
        "{}",
        h.app.status.message
    );
}

#[test]
fn a_rotary_job_s_z_zero_reads_turning_axis_not_machine_bed() {
    use cut_craft_core::document::{Along, Job};
    for form in ["material_setup", "job_setup"] {
        let mut h = Harness::new();
        h.app.doc.job = Job::rotary(300.0, 50.0, Along::X);
        h.app.invoke(&h.ctx, form);
        for _ in 0..3 {
            h.frame(vec![]);
        }
        assert!(h.text_at("Turning Axis").is_some(), "{form}");
        assert!(h.text_at("Machine Bed").is_none(), "{form}");
        // a flat job keeps the bed
        h.app.form = None;
        h.app.doc.job = Job::default();
        h.app.invoke(&h.ctx, form);
        for _ in 0..3 {
            h.frame(vec![]);
        }
        assert!(h.text_at("Machine Bed").is_some(), "{form}");
        assert!(h.text_at("Turning Axis").is_none(), "{form}");
    }
}

/// #511: Machine has no "Update Post-Processor Database" (it only ran Cut Craft's own update
/// check), and File has one print item that says it opens a PDF, not Print and Print Preview
/// doing the same thing.
#[test]
fn print_is_one_honest_item_and_no_post_database_update() {
    fn actions(items: &[crate::gui::MenuItem], out: &mut Vec<(String, String)>) {
        for item in items {
            out.push((item.name.clone(), item.action.clone()));
            actions(&item.children, out);
        }
    }
    let h = Harness::new();
    let mut all = Vec::new();
    for menu in &h.app.current_menus().menus {
        actions(&menu.items, &mut all);
    }
    assert!(
        !all.iter()
            .any(|(_, a)| a == "update_post_processor_database")
    );
    assert!(!all.iter().any(|(_, a)| a == "print_preview"));
    let prints: Vec<_> = all.iter().filter(|(_, a)| a == "print").collect();
    assert_eq!(prints.len(), 1, "{prints:?}");
    assert_eq!(prints[0].0, "Print (opens a PDF)...");
    assert!(h.app.available("print"));
}

/// A rotary job's Machine Bed is the turning axis: half the blank under its top, and on the
/// outline's middle while a model is imported unwrapped.
#[test]
fn a_rotary_jobs_3d_origin_is_on_the_axis_for_machine_bed() {
    use cut_craft_core::document::{Along, Job, ZZero};
    let job = Job {
        z_zero: ZZero::MachineBed,
        ..Job::rotary(300.0, 40.0, Along::X)
    };
    assert!(
        (panels::origin_3d(&job, false) + 20.0).abs() < 1e-9,
        "20 mm under the top of a 40 mm blank"
    );
    assert!(
        panels::origin_3d(&job, true).abs() < 1e-9,
        "the outline's bottom, its top being the blank's radius up"
    );
}
