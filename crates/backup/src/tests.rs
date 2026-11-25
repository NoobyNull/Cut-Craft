use super::*;
use std::fs;

/// A settings folder of its own for one test, emptied first.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("cut-craft-backup-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn put(dir: &Path, path: &str, text: &str) {
    let path = dir.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// A settings folder like a real one: what a backup takes, and what it must leave.
fn settings_folder(name: &str) -> PathBuf {
    let dir = scratch(name);
    put(
        &dir,
        "options.json",
        r#"{"general":{"layout":"canvas"},"web_api":{"enabled":true,"token":"secret"}}"#,
    );
    put(&dir, "machines.json", r#"{"machines":[]}"#);
    put(&dir, "posts/my router.pp", "post");
    put(&dir, "tools.json", r#"{"tools":[]}"#);
    put(&dir, "templates/signs/oval.ccjobt", "template");
    put(&dir, "words.txt", "Whittle\n");
    put(
        &dir,
        "addons/depth/installed.json",
        r#"{"version":"0.2.0"}"#,
    );
    put(&dir, "addons/depth/weights.safetensors", "very large");
    put(&dir, "unsigned-code-allowed.json", "[]");
    // never backed up
    put(&dir, "recent.json", r#"["/home/me/job.ccjob"]"#);
    put(&dir, "recovery.ccjob", "unsaved work");
    put(&dir, "cut-craft.log", "log");
    put(&dir, "last-crash.log", "crash");
    dir
}

fn paths(manifest: &Manifest) -> Vec<&str> {
    manifest.files.iter().map(|f| f.path.as_str()).collect()
}

/// What `file` holds at `path` in the settings folder.
fn inside(file: &Path, path: &str) -> String {
    let mut zip = zip::ZipArchive::new(fs::File::open(file).unwrap()).unwrap();
    let mut text = String::new();
    zip.by_name(&format!("{FILES}{path}"))
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    text
}

#[test]
fn a_backup_holds_settings_and_own_files_but_never_the_recent_list_logs_or_recovery() {
    let dir = settings_folder("parts");
    let to = dir.join("out.ccbackup");
    let manifest = write(&dir, &Choice::default(), Reason::ByHand, "0.7.7", 100, &to).unwrap();
    assert_eq!(
        paths(&manifest),
        [
            "options.json",
            "machines.json",
            "posts/my router.pp",
            "tools.json",
            "templates/signs/oval.ccjobt",
            "words.txt",
            "addons/depth/installed.json",
            "unsigned-code-allowed.json",
        ]
    );
    assert_eq!(read_manifest(&to).unwrap(), manifest);
    assert_eq!(inside(&to, "posts/my router.pp"), "post");
}

#[test]
fn add_ons_go_in_as_where_they_came_from_not_their_programs() {
    let dir = settings_folder("add-ons");
    let to = dir.join("out.ccbackup");
    let choice = Choice {
        parts: [Part::AddOns].into(),
        secrets: false,
    };
    let manifest = write(&dir, &choice, Reason::ByHand, "0.7.7", 100, &to).unwrap();
    assert_eq!(paths(&manifest), ["addons/depth/installed.json"]);
    assert_eq!(manifest.files[0].part, Part::AddOns);
}

#[test]
fn only_the_parts_ticked_go_in() {
    let dir = settings_folder("ticked");
    let to = dir.join("out.ccbackup");
    let choice = Choice {
        parts: [Part::BitLibrary, Part::Machines].into(),
        secrets: false,
    };
    let manifest = write(&dir, &choice, Reason::ByHand, "0.7.7", 100, &to).unwrap();
    assert_eq!(
        paths(&manifest),
        ["machines.json", "posts/my router.pp", "tools.json"]
    );
    assert_eq!(manifest.parts, [Part::Machines, Part::BitLibrary]);
}

#[test]
fn the_web_api_token_is_left_out_unless_asked_for() {
    let dir = settings_folder("secrets");
    let to = dir.join("out.ccbackup");
    write(&dir, &Choice::default(), Reason::ByHand, "0.7.7", 100, &to).unwrap();
    let options: serde_json::Value = serde_json::from_str(&inside(&to, "options.json")).unwrap();
    assert_eq!(options["web_api"]["enabled"], true);
    assert!(options["web_api"].get("token").is_none());
    assert_eq!(options["general"]["layout"], "canvas");

    let choice = Choice {
        secrets: true,
        ..Choice::default()
    };
    let manifest = write(&dir, &choice, Reason::ByHand, "0.7.7", 100, &to).unwrap();
    assert!(manifest.secrets);
    assert!(inside(&to, "options.json").contains("secret"));
}

#[test]
fn every_file_has_its_checksum_and_the_manifest_says_where_it_was_made() {
    let dir = settings_folder("manifest");
    let to = dir.join("out.ccbackup");
    let manifest = write(&dir, &Choice::default(), Reason::Weekly, "0.7.7", 1234, &to).unwrap();
    let words = manifest
        .files
        .iter()
        .find(|f| f.path == "words.txt")
        .unwrap();
    assert_eq!(words.size, 8);
    assert_eq!(words.sha256, sha256(b"Whittle\n"));
    assert_eq!(
        (
            manifest.app_version.as_str(),
            manifest.created,
            manifest.reason
        ),
        ("0.7.7", 1234, Reason::Weekly)
    );
    assert_eq!(manifest.os, std::env::consts::OS);
    // no half-written file is left beside it
    assert!(!dir.join("out.ccbackup.part").exists());
}

#[test]
fn a_file_that_is_not_a_backup_says_so() {
    let dir = scratch("not-a-backup");
    put(&dir, "job.ccbackup", "not a zip");
    assert!(matches!(
        read_manifest(&dir.join("job.ccbackup")),
        Err(Error::NotABackup(_))
    ));
}

#[test]
fn a_backup_from_a_newer_cut_craft_says_so() {
    let dir = settings_folder("newer");
    let to = dir.join("out.ccbackup");
    let mut manifest = write(&dir, &Choice::default(), Reason::ByHand, "9.0.0", 1, &to).unwrap();
    manifest.version = VERSION + 1;
    zip_up(&manifest, &[], &to).unwrap();
    assert!(matches!(
        read_manifest(&to),
        Err(Error::NewerVersion(v)) if v == VERSION + 1
    ));
}

#[test]
fn automatic_backups_go_in_the_backups_folder_and_the_oldest_beyond_the_count_go() {
    let dir = settings_folder("keep");
    let auto = Automatic::new(dir.clone(), 3, "0.7.7");
    for n in 0..5 {
        put(&dir, "words.txt", &format!("word {n}\n"));
        let made = auto.make(Reason::BeforeLayoutChange, 1000 + n).unwrap();
        assert!(made.unwrap().starts_with(dir.join(FOLDER)));
    }
    let kept = auto.kept();
    let created: Vec<u64> = kept.iter().map(|k| k.manifest.created).collect();
    assert_eq!(created, [1004, 1003, 1002]);
    assert!(
        kept[0]
            .path
            .to_string_lossy()
            .ends_with("before-layout-change.ccbackup")
    );
}

#[test]
fn a_backup_saved_into_the_folder_by_hand_is_never_removed() {
    let dir = settings_folder("by-hand-kept");
    let auto = Automatic::new(dir.clone(), 1, "0.7.7");
    let mine = auto.folder().join("mine.ccbackup");
    write(&dir, &Choice::default(), Reason::ByHand, "0.7.7", 1, &mine).unwrap();
    for n in 0..3 {
        put(&dir, "words.txt", &format!("word {n}\n"));
        auto.make(Reason::Weekly, 10 + n).unwrap();
    }
    let kept: Vec<Reason> = auto.kept().iter().map(|k| k.manifest.reason).collect();
    assert_eq!(kept, [Reason::Weekly, Reason::ByHand]);
    assert!(mine.exists());
}

#[test]
fn no_automatic_backup_when_nothing_changed_since_one_kept() {
    let dir = settings_folder("same");
    let auto = Automatic::new(dir.clone(), KEEP, "0.7.7");
    assert!(auto.make(Reason::BeforeLayoutChange, 1).unwrap().is_some());
    assert!(auto.make(Reason::BeforeLayoutChange, 2).unwrap().is_none());
    // switching layout and back again: the first is still kept, so no third
    put(&dir, "options.json", r#"{"general":{"layout":"ribbon"}}"#);
    assert!(auto.make(Reason::BeforeLayoutChange, 3).unwrap().is_some());
    put(
        &dir,
        "options.json",
        r#"{"general":{"layout":"canvas"},"web_api":{"enabled":true,"token":"secret"}}"#,
    );
    assert!(auto.make(Reason::BeforeLayoutChange, 4).unwrap().is_none());
    assert_eq!(auto.kept().len(), 2);
}

#[test]
fn a_new_install_has_nothing_to_back_up() {
    let dir = scratch("empty");
    let auto = Automatic::new(dir.clone(), KEEP, "0.7.7");
    assert!(auto.make(Reason::Weekly, 1).unwrap().is_none());
    assert!(!auto.folder().exists());
}

#[test]
fn the_weekly_backup_is_due_a_week_after_the_newest() {
    let dir = settings_folder("weekly");
    let auto = Automatic::new(dir.clone(), KEEP, "0.7.7");
    assert!(auto.weekly_due(0));
    auto.make(Reason::BeforeUpdate, 1000).unwrap();
    assert!(!auto.weekly_due(1000 + WEEK - 1));
    assert!(auto.weekly_due(1000 + WEEK));
}

#[test]
fn two_backups_in_the_same_second_get_names_of_their_own() {
    let dir = settings_folder("same-second");
    let auto = Automatic::new(dir.clone(), KEEP, "0.7.7");
    let first = auto.make(Reason::Weekly, 50).unwrap().unwrap();
    put(&dir, "words.txt", "changed\n");
    let second = auto.make(Reason::Weekly, 50).unwrap().unwrap();
    assert_ne!(first, second);
    assert_eq!(auto.kept().len(), 2);
}

#[cfg(unix)]
#[test]
fn a_link_back_up_the_folders_is_not_followed() {
    let dir = settings_folder("link-loop");
    std::os::unix::fs::symlink(dir.join("templates"), dir.join("templates/signs/again")).unwrap();
    let to = dir.join("out.ccbackup");
    let choice = Choice {
        parts: [Part::Templates].into(),
        secrets: false,
    };
    let manifest = write(&dir, &choice, Reason::ByHand, "0.7.7", 1, &to).unwrap();
    assert_eq!(paths(&manifest), ["templates/signs/oval.ccjobt"]);
}

#[cfg(unix)]
#[test]
fn a_folder_that_cannot_be_read_fails_the_backup_rather_than_leave_it_short() {
    use std::os::unix::fs::PermissionsExt as _;
    let dir = settings_folder("unreadable");
    let locked = dir.join("templates/signs");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    // a user who can read anything (root, as in some containers) reads it anyway
    let readable = fs::read_dir(&locked).is_ok();
    let made = write(
        &dir,
        &Choice::default(),
        Reason::ByHand,
        "0.7.7",
        1,
        &dir.join("out.ccbackup"),
    );
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
    if !readable {
        assert!(matches!(made, Err(Error::File(path, _)) if path == locked));
        assert!(!dir.join("out.ccbackup").exists());
    }
}
