//! Job files, imports and exports: the app's only file I/O. Every operation takes a path, so
//! tests run them without dialogs.

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::file::{load_job_noting, save_job, write_atomic};
use cut_craft_core::toolpath::ToolDatabase;
use cut_craft_core::{Change, History, Registry};
use cut_craft_post::Post;
use std::path::{Path, PathBuf};

/// The job file being edited and whether it has unsaved changes.
#[derive(Default)]
pub struct Session {
    pub path: Option<PathBuf>,
    /// History revision at the last save or open.
    saved: u64,
    /// Settings in the opened file that this version does not know (a newer one added
    /// them): saving from here leaves them out.
    pub newer_settings: Vec<String>,
}

impl Session {
    pub fn is_dirty(&self, history: &History) -> bool {
        history.revision() != self.saved
    }

    /// The changes since the last save were thrown away on purpose: stop asking about them.
    pub fn discard_changes(&mut self, history: &History) {
        self.saved = history.revision();
    }

    /// Window title: file name (or "Untitled"), with `*` for unsaved changes.
    pub fn title(&self, app: &str, history: &History) -> String {
        let name = self.path.as_deref().and_then(Path::file_name).map_or_else(
            || "Untitled".to_owned(),
            |n| n.to_string_lossy().into_owned(),
        );
        let star = if self.is_dirty(history) { "*" } else { "" };
        format!("{name}{star} - {app}")
    }

    /// A job with no file, whose content is nowhere else yet (recovered after a crash): it
    /// counts as unsaved until it is saved.
    pub fn unsaved() -> Self {
        Self {
            path: None,
            saved: u64::MAX,
            newer_settings: vec![],
        }
    }

    /// [`Session::unsaved`], for work that belongs to the job file `path` (kept for
    /// recovery, or across an update): saving goes back to it.
    pub fn unsaved_at(path: Option<PathBuf>) -> Self {
        Self {
            path,
            ..Self::unsaved()
        }
    }

    pub fn save(&mut self, doc: &Document, history: &History, path: &Path) -> Result<(), String> {
        write_atomic(path, &save_job(doc))
            .map_err(|e| crate::messages::could_not_save(path, &e))?;
        self.path = Some(path.to_owned());
        self.saved = history.revision();
        Ok(())
    }

    /// The opened job with a fresh history; the session now points at it.
    pub fn open(&mut self, path: &Path) -> Result<(Document, History), String> {
        let bytes = read(path)?;
        let (doc, newer_settings) =
            load_job_noting(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        let history = History::default();
        self.newer_settings = newer_settings;
        self.path = Some(path.to_owned());
        self.saved = history.revision();
        Ok((doc, history))
    }
}

/// An import as an undoable change, chosen by the file's extension.
pub fn import(registry: &Registry, doc: &Document, path: &Path) -> Result<Change, String> {
    let extension = path
        .extension()
        .map_or_default(|e| e.to_string_lossy().into_owned());
    let importer = registry
        .importer_for(&extension)
        .ok_or_else(|| format!("no importer for .{extension} files"))?;
    (importer.import)(doc, &read(path)?).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn export(registry: &Registry, id: &str, doc: &Document, path: &Path) -> Result<(), String> {
    let exporter = registry
        .exporter(id)
        .ok_or_else(|| format!("no exporter called {id}"))?;
    let bytes = exporter.run(doc).map_err(|e| e.to_string())?;
    write_atomic(path, &bytes).map_err(|e| format!("could not write {}: {e}", path.display()))
}

/// Writes the toolpaths through `post` to `path`, tools numbered by their tool changer
/// `slots` (empty: in the order used); `job` is the saved job they come from.
#[cfg(test)]
pub fn save_through(
    post: &Post,
    slots: &[Option<String>],
    doc: &Document,
    path: &Path,
    job: Option<&Path>,
) -> Result<(), String> {
    let made = chrono::Local::now().naive_local();
    let text = render_through(post, slots, doc, path, job, made, &[])?;
    write_text(path, &text)
}

/// The G-code [`save_through`] writes to `path`, as made at `made`, without writing it
/// (View G-code); `notes` are the job's notes with their variables filled in, as G-code
/// comment lines (`cut_craft_core::notes::gcode_lines`).
pub fn render_through(
    post: &Post,
    slots: &[Option<String>],
    doc: &Document,
    path: &Path,
    job: Option<&Path>,
    made: chrono::NaiveDateTime,
    notes: &[String],
) -> Result<String, String> {
    let file = cut_craft_post::File {
        path,
        job,
        made,
        slots,
        notes,
    };
    post.render(doc, &file)
}

/// Writes a G-code file's `text` to `path`, saying where when it cannot.
pub fn write_text(path: &Path, text: &str) -> Result<(), String> {
    write_atomic(path, text.as_bytes())
        .map_err(|e| format!("could not write {}: {e}", path.display()))
}

/// How a file's tool changes are made, on Save Toolpaths' form.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToolChanges {
    /// A file for each tool in turn, the tool changed between files (no tool change in a
    /// file): the safest without a tool changer.
    #[default]
    SeparateFiles,
    /// One file; the machine's tool changer changes them (its post's `TOOLCHANGE`).
    ToolChanger,
    /// One file; the machine stops at each change for the tool to be changed by hand.
    Pause,
}

/// How Save Toolpaths splits the toolpaths into files (Save Toolpaths form).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SaveMode {
    /// The toolpath picked in the list, alone.
    Selected,
    /// Every shown toolpath in one file.
    #[default]
    VisibleToOneFile,
    /// Each shown toolpath in a file of its own.
    VisibleToMultipleFiles,
}

/// The toolpaths each file holds, in list order: the selected toolpath (or the toolpaths of
/// the selected group, as one file); every shown one; or each shown one alone, where `group`
/// puts toolpaths in a row that use the same tool in one file (Group Where Possible).
pub fn save_plan(
    doc: &Document,
    mode: SaveMode,
    selected: Option<EntityId>,
    selected_group: Option<&str>,
    group: bool,
) -> Result<Vec<Vec<EntityId>>, String> {
    let visible: Vec<EntityId> = doc
        .toolpaths
        .iter()
        .filter(|(_, t)| t.visible)
        .map(|(&id, _)| id)
        .collect();
    let plan = match mode {
        SaveMode::Selected => match (selected_group, selected) {
            (Some(name), _) => match doc.toolpaths_under_group(name) {
                ids if ids.is_empty() => return Err(format!("{name} has no toolpaths")),
                ids => vec![ids],
            },
            (None, Some(id)) if doc.toolpaths.contains_key(&id) => vec![vec![id]],
            _ => return Err("pick the toolpath to save in the toolpath list".into()),
        },
        SaveMode::VisibleToOneFile => vec![visible],
        SaveMode::VisibleToMultipleFiles => {
            let tool = |id: &EntityId| &doc.toolpaths[id].tool.name;
            let mut files: Vec<Vec<EntityId>> = vec![];
            for id in visible {
                match files.last_mut() {
                    Some(file) if group && tool(&file[0]) == tool(&id) => file.push(id),
                    _ => files.push(vec![id]),
                }
            }
            files
        }
    };
    if plan.iter().all(Vec::is_empty) {
        return Err("no toolpaths are shown: tick the ones to save".into());
    }
    Ok(plan)
}

/// `plan` with each file split where its tool changes, for a post that cannot change tools:
/// a file for each tool in turn, run in order.
pub fn by_tool(doc: &Document, plan: Vec<Vec<EntityId>>) -> Vec<Vec<EntityId>> {
    let tool = |id: &EntityId| &doc.toolpaths[id].tool.name;
    let mut files: Vec<Vec<EntityId>> = vec![];
    for file in plan {
        let start = files.len();
        for id in file {
            let same =
                files.len() > start && files.last().is_some_and(|l| tool(&l[0]) == tool(&id));
            match files.last_mut() {
                Some(last) if same => last.push(id),
                _ => files.push(vec![id]),
            }
        }
    }
    files
}

/// Each file of `plan` as the path it is saved at and the job with only its toolpaths.
/// One file keeps `path`; several get the number and first toolpath's name after the stem.
/// `side` (Top or Bottom) goes on the end of each name. A file whose toolpaths use more than
/// one tool needs a post that changes tools. `per_tool` names several files after their
/// tool instead (a file for each tool, for a post that cannot change tools).
pub fn save_files(
    post: &Post,
    doc: &Document,
    plan: &[Vec<EntityId>],
    path: &Path,
    side: Option<&str>,
    per_tool: bool,
) -> Result<Vec<(PathBuf, Document)>, String> {
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let extension = path.extension().unwrap_or_default().to_string_lossy();
    plan.iter()
        .enumerate()
        .map(|(n, ids)| {
            let tools: std::collections::BTreeSet<&str> = ids
                .iter()
                .map(|id| doc.toolpaths[id].tool.name.as_str())
                .collect();
            if tools.len() > 1 && !post.changes_tools() {
                return Err(format!(
                    "the toolpaths use different tools ({}) and {} does not change tools: \
                     save them to multiple files or pick a post with tool changes",
                    tools.into_iter().collect::<Vec<_>>().join(", "),
                    post.name
                ));
            }
            let mut name = stem.to_string();
            if plan.len() > 1 {
                let first = &doc.toolpaths[&ids[0]];
                let first = if per_tool {
                    &first.tool.name
                } else {
                    &first.name
                };
                let first: String = first
                    .chars()
                    .map(|c| {
                        if c.is_alphanumeric() || c == '-' {
                            c
                        } else {
                            '_'
                        }
                    })
                    .collect();
                name = format!("{name}_{}_{first}", n + 1);
            }
            if let Some(side) = side {
                name = format!("{name}_{side}");
            }
            let mut part = doc.clone();
            part.toolpaths.retain(|id, _| ids.contains(id));
            Ok((path.with_file_name(format!("{name}.{extension}")), part))
        })
        .collect()
}

/// `path` with `extension` on the end, unless its name already ends with it: a name typed
/// with a dot of its own (`sign v1.5`) keeps it (`sign v1.5.nc`).
pub fn with_extension_added(path: &Path, extension: &str) -> PathBuf {
    if path
        .extension()
        .is_some_and(|e| e.to_string_lossy().eq_ignore_ascii_case(extension))
    {
        return path.to_owned();
    }
    let mut name = path.as_os_str().to_owned();
    name.push(format!(".{extension}"));
    PathBuf::from(name)
}

/// A name from a save dialog with one of `extensions` on its end: kept as it is when it
/// already ends with one of them, else the first is added, so a name typed with a dot of
/// its own (`plaque v1.2`) keeps it (`plaque v1.2.ccjob`) rather than losing what follows.
pub fn named_for_saving(path: &Path, extensions: &[&str]) -> PathBuf {
    let has = path.extension().is_some_and(|e| {
        let e = e.to_string_lossy();
        extensions.iter().any(|x| e.eq_ignore_ascii_case(x))
    });
    match extensions.first() {
        Some(first) if !has => with_extension_added(path, first),
        _ => path.to_owned(),
    }
}

/// Whether saving to `named` would replace a file the save dialog never asked about: the
/// extension was added to the name typed (`dialog`) and a file of that name is there already.
pub fn replaces_unasked(dialog: &Path, named: &Path) -> bool {
    dialog != named && named.exists()
}

/// This run's own recovery copy in `dir`, `recovery-<run>.ccjob`, and the lock that marks
/// it as in use for as long as the returned file is kept open: another window started
/// meanwhile leaves it alone. Lock files of runs that have ended, with no copy left, are
/// tidied away. Where the folder cannot lock files (some network or FUSE folders), the copy
/// is still kept, with no lock: other windows go by whether the run is still going. `None`
/// only when the folder cannot be made.
pub fn claim_recovery(dir: &Path, run: &str) -> Option<(PathBuf, Option<std::fs::File>)> {
    claim_recovery_locking(dir, run, |lock| lock.try_lock().is_ok())
}

/// [`claim_recovery`], locking with `lock` (whether it took the lock).
fn claim_recovery_locking(
    dir: &Path,
    run: &str,
    lock: impl Fn(&std::fs::File) -> bool,
) -> Option<(PathBuf, Option<std::fs::File>)> {
    std::fs::create_dir_all(dir).ok()?;
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        let stale = path.extension().is_some_and(|e| e == "lock" || e == "json")
            && is_recovery(&path)
            && !path.with_extension("ccjob").exists()
            && !recovery_in_use(&path.with_extension("lock"));
        if stale {
            let _ = std::fs::remove_file(&path);
        }
    }
    let lock = std::fs::File::create(dir.join(format!("recovery-{run}.lock")))
        .ok()
        .filter(|file| lock(file));
    let own = dir.join(format!("recovery-{run}.ccjob"));
    if own.exists() {
        // an ended run that had the same number left it: kept aside to be recovered
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis());
        let aside = dir.join(format!("recovery-{run}-{stamp}.ccjob"));
        let _ = std::fs::rename(own.with_extension("json"), aside.with_extension("json"));
        let _ = std::fs::rename(&own, aside);
    }
    Some((own, lock))
}

/// Whether `path` is named like a recovery copy or its lock (`recovery…`).
fn is_recovery(path: &Path) -> bool {
    path.file_stem()
        .is_some_and(|s| s.to_string_lossy().starts_with("recovery"))
}

/// Whether the run a recovery lock file belongs to is still running (it holds the lock).
fn recovery_in_use(lock: &Path) -> bool {
    let Ok(file) = std::fs::File::open(lock) else {
        return false;
    };
    match file.try_lock() {
        Err(std::fs::TryLockError::WouldBlock) => true,
        // a folder that cannot lock files: whether the run named on it is still going
        Err(std::fs::TryLockError::Error(_)) => lock
            .file_stem()
            .and_then(|s| s.to_str()?.strip_prefix("recovery-")?.parse::<u32>().ok())
            .is_some_and(process_running),
        Ok(()) => false,
    }
}

/// Whether process `pid` is running, where the system says (Linux's `/proc`); elsewhere
/// taken as ended.
fn process_running(pid: u32) -> bool {
    cfg!(target_os = "linux") && Path::new(&format!("/proc/{pid}")).exists()
}

/// What a recovery copy `recovery-<run>.ccjob` says about itself, in `recovery-<run>.json`
/// beside it: the job file its work belongs to, and whether an update left it for the new
/// version (rather than a crash). A copy with no note (an older version's) has neither.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RecoveryNote {
    /// The job's own file, if it has been saved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    /// Kept by Install Now for the new version to open, not left by a crash.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub after_update: bool,
}

/// Writes the recovery copy `copy`: the job's `bytes`, then `note` beside it.
pub fn write_recovery(copy: &Path, bytes: &[u8], note: &RecoveryNote) -> std::io::Result<()> {
    write_atomic(copy, bytes)?;
    write_recovery_note(copy, note)
}

/// Removes the recovery copy `copy` and everything kept beside it: its note and, with
/// `lock`, its lock file.
pub fn remove_recovery(copy: &Path, lock: bool) {
    let _ = std::fs::remove_file(copy);
    let _ = std::fs::remove_file(copy.with_extension("json"));
    if lock {
        let _ = std::fs::remove_file(copy.with_extension("lock"));
    }
}

/// The note beside the recovery copy `copy`; an empty one if there is none to read.
pub fn recovery_note(copy: &Path) -> RecoveryNote {
    std::fs::read(copy.with_extension("json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

/// Writes `note` beside the recovery copy `copy`.
fn write_recovery_note(copy: &Path, note: &RecoveryNote) -> std::io::Result<()> {
    let json = serde_json::to_vec_pretty(note).map_err(std::io::Error::other)?;
    write_atomic(&copy.with_extension("json"), &json)
}

/// A recovery copy in `dir` left by a run that ended without saving or closing its job (a
/// crash), the newest first; not `own`, and not one a window still running keeps. A copy
/// with no lock (an older version's `recovery.ccjob`) counts as left behind.
pub fn orphaned_recovery(dir: &Path, own: &Path) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p != own && p.extension().is_some_and(|e| e == "ccjob") && is_recovery(p))
        // work handed to the new version by an update is left for it, even before the old
        // version has quite closed
        .filter(|p| !recovery_in_use(&p.with_extension("lock")) || recovery_note(p).after_update)
        .max_by_key(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok())
}

/// Whether the file `name` is one a save as `stem`.`extension` writes: that name itself, or
/// a numbered, tiled or sided file beside it (`sign_1_Profile.nc`, `sign_tile2.nc`,
/// `sign_Top.nc`).
pub fn of_earlier_save(name: &str, stem: &str, extension: &str) -> bool {
    let Some(rest) = name
        .strip_suffix(&format!(".{extension}"))
        .and_then(|n| n.strip_prefix(stem))
    else {
        return false;
    };
    if rest.is_empty() {
        return true;
    }
    let Some(rest) = rest.strip_prefix('_') else {
        return false;
    };
    let mut words = rest.split('_');
    let first = words.next().unwrap_or_default();
    let digits = |w: &str| !w.is_empty() && w.chars().all(|c| c.is_ascii_digit());
    (digits(first) && words.next().is_some())
        || first.strip_prefix("tile").is_some_and(digits)
        || first == "Top"
        || first == "Bottom"
}

/// The G-code files an earlier save to `path` left in its folder: `path` itself and the
/// numbered, tiled and sided files [`save_files`] writes beside it (`sign_1_Profile.nc`,
/// `sign_tile2.nc`, `sign_Top.nc`), with the same extension. Other files are left out.
pub fn earlier_save(path: &Path) -> Vec<PathBuf> {
    let (Some(dir), Some(stem), Some(extension)) = (
        path.parent(),
        path.file_stem().map(|s| s.to_string_lossy().into_owned()),
        path.extension().map(|e| e.to_string_lossy().into_owned()),
    ) else {
        return vec![];
    };
    let dir = if dir.as_os_str().is_empty() {
        Path::new(".")
    } else {
        dir
    };
    let ours = |name: &str| of_earlier_save(name, &stem, &extension);
    let Ok(read) = std::fs::read_dir(dir) else {
        return vec![];
    };
    let mut found: Vec<PathBuf> = read
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .filter(|e| ours(&e.file_name().to_string_lossy()))
        .map(|e| e.path())
        .collect();
    found.sort();
    found
}

/// Moves `files` (an earlier save to `path`) into a new folder beside `path`,
/// `Archive/<its stem> <when>` (with ` (2)`, ` (3)`… added when one of that name is
/// already there, so two archives in the same second never mix); that folder.
pub fn archive(
    files: &[PathBuf],
    path: &Path,
    when: chrono::NaiveDateTime,
) -> Result<PathBuf, String> {
    let dir = path.parent().unwrap_or(Path::new("."));
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let base = format!("{stem} {}", when.format("%Y-%m-%d %H%M%S"));
    let folder = new_folder(&dir.join("Archive"), &base)?;
    move_into(files, &folder)?;
    Ok(folder)
}

/// Moves `files` (an earlier save to `path`) into a new hidden folder beside `path`, kept
/// until the new save is written: then [`std::fs::remove_dir_all`] it, or
/// [`put_back`] them if the save fails. That folder.
pub fn set_aside(files: &[PathBuf], path: &Path) -> Result<PathBuf, String> {
    let dir = path.parent().unwrap_or(Path::new("."));
    let dir = if dir.as_os_str().is_empty() {
        Path::new(".")
    } else {
        dir
    };
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let folder = new_folder(dir, &format!(".{stem} replacing"))?;
    move_into(files, &folder)?;
    Ok(folder)
}

/// Puts `files` (moved into `folder` by [`archive`] or [`set_aside`]) back where they
/// were, over anything a failed save left in their place, after taking away the other
/// files that save wrote beside `path`; then removes `folder` when it is empty.
pub fn put_back(files: &[PathBuf], folder: &Path, path: &Path) -> Result<(), String> {
    for partial in earlier_save(path) {
        if !files.contains(&partial) {
            let _ = std::fs::remove_file(partial);
        }
    }
    for file in files {
        let from = folder.join(file.file_name().unwrap_or_default());
        if from.exists() {
            move_file(&from, file).map_err(|e| format!("{}: {e}", from.display()))?;
        }
    }
    let _ = std::fs::remove_dir(folder);
    Ok(())
}

/// A new, empty folder `<parent>/<name>`, or `<name> (2)`, `<name> (3)`… when taken.
fn new_folder(parent: &Path, name: &str) -> Result<PathBuf, String> {
    std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    for n in 1u32.. {
        let folder = if n == 1 {
            parent.join(name)
        } else {
            parent.join(format!("{name} ({n})"))
        };
        match std::fs::create_dir(&folder) {
            Ok(()) => return Ok(folder),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(format!("{}: {e}", folder.display())),
        }
    }
    Err(format!("{}: no free name", parent.display()))
}

/// Moves each of `files` into `folder`; if one cannot be moved, those already moved go
/// back and the folder is removed.
fn move_into(files: &[PathBuf], folder: &Path) -> Result<(), String> {
    for (n, file) in files.iter().enumerate() {
        let to = folder.join(file.file_name().unwrap_or_default());
        if let Err(e) = move_file(file, &to) {
            for back in &files[..n] {
                let _ = move_file(&folder.join(back.file_name().unwrap_or_default()), back);
            }
            let _ = std::fs::remove_dir(folder);
            return Err(format!("{}: {e}", file.display()));
        }
    }
    Ok(())
}

fn move_file(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::rename(from, to)
        .or_else(|_| std::fs::copy(from, to).and_then(|_| std::fs::remove_file(from)))
}

pub use cut_craft_core::file::config_dir;

/// Opens `path` (a folder, or a file such as a PDF to print) with the platform's own
/// program for it.
pub fn show_folder(path: &Path) -> std::io::Result<()> {
    cut_craft_bug_report::open_with_platform(path.as_os_str())
}

/// Shows `file` in the desktop's file manager, picked in its folder where the file manager
/// can (Windows Explorer, macOS Finder, and on Linux one that answers the freedesktop
/// `FileManager1` call, as Dolphin, Nautilus, Nemo and Thunar do); else opens its folder.
pub fn locate(file: &Path) -> std::io::Result<()> {
    if !file.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "the file isn't there any more",
        ));
    }
    if cfg!(windows) {
        return std::process::Command::new("explorer")
            .arg("/select,")
            .arg(file)
            .spawn()
            .map(drop);
    }
    if cfg!(target_os = "macos") {
        return std::process::Command::new("open")
            .arg("-R")
            .arg(file)
            .spawn()
            .map(drop);
    }
    let shown = std::process::Command::new("dbus-send")
        .args([
            "--session",
            "--print-reply",
            "--dest=org.freedesktop.FileManager1",
            "--type=method_call",
            "/org/freedesktop/FileManager1",
            "org.freedesktop.FileManager1.ShowItems",
        ])
        .arg(format!("array:string:{}", file_uri(file)))
        .arg("string:")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    if shown {
        return Ok(());
    }
    show_folder(file.parent().unwrap_or(file))
}

/// `path` as a `file://` URI: each byte outside the unreserved set and `/` percent-encoded.
fn file_uri(path: &Path) -> String {
    use std::fmt::Write as _;
    let path = path.to_string_lossy().replace('\\', "/");
    let mut uri = String::from("file://");
    if !path.starts_with('/') {
        uri.push('/'); // a Windows drive
    }
    for b in path.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~/:".contains(&b) {
            uri.push(char::from(b));
        } else {
            let _ = write!(uri, "%{b:02X}");
        }
    }
    uri
}

/// The next file name for Increment and Save: a name ending `_N` or `-N` counts up, any
/// other gets `_1`, and it keeps counting past names that `exists` says are taken. Only a
/// short number counts up: a year (`Smith-2024`) or a date (`order_20241115`,
/// `order_2024-11-15`, `15-11-2024`) on the end of the name is part of it, so it gets `_1`
/// after it.
pub fn incremented(path: &Path, exists: impl Fn(&Path) -> bool) -> PathBuf {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled");
    let counter = |at: usize| {
        let digits = &stem[at + 1..];
        let n = digits
            .parse::<u64>()
            .ok()
            .filter(|n| digits.len() <= 4 && !(1900..=2099).contains(n))?;
        let sep = stem[at..].chars().next()?;
        (!ends_with_year_and_month(&stem[..at], sep) || digits.len() > 2 || !(1..=31).contains(&n))
            .then_some(n)
    };
    let (base, mut n) = match stem.rfind(['_', '-']) {
        Some(at) if counter(at).is_some() => (&stem[..=at], counter(at).unwrap_or(0)),
        _ => (stem, 0),
    };
    let base = if n == 0 && !base.ends_with(['_', '-']) {
        format!("{base}_")
    } else {
        base.to_owned()
    };
    loop {
        n += 1;
        let mut next = path.with_file_name(format!("{base}{n}"));
        if let Some(ext) = path.extension() {
            next.set_extension(ext);
        }
        if !exists(&next) {
            return next;
        }
    }
}

/// Whether `text` ends with a year, `sep` and a month (`order_2024-11` with `-`), so that a
/// day after it is the end of a date, not a counter.
fn ends_with_year_and_month(text: &str, sep: char) -> bool {
    let Some((before, month)) = text.rsplit_once(sep) else {
        return false;
    };
    let (front, year) = before
        .len()
        .checked_sub(4)
        .and_then(|at| before.split_at_checked(at))
        .unwrap_or(("", ""));
    (1..=2).contains(&month.len())
        && month.parse::<u8>().is_ok_and(|m| (1..=12).contains(&m))
        && year.len() == 4
        && year
            .parse::<u16>()
            .is_ok_and(|y| (1900..=2099).contains(&y))
        && !front.ends_with(|c: char| c.is_ascii_digit())
}

pub fn tool_database_path() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("tools.json"))
}

/// Where the Materials Library is kept.
pub fn materials_path() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("materials.json"))
}

/// The Materials Library at `path`; the starter list when there is none yet.
pub fn load_materials(
    path: &Path,
) -> Result<cut_craft_core::material_library::MaterialLibrary, String> {
    use cut_craft_core::material_library::MaterialLibrary;
    if !path.exists() {
        return Ok(MaterialLibrary::starter());
    }
    MaterialLibrary::from_json(&read(path)?).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn recent_files_path() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("recent.json"))
}

/// Where the Clipart tab's library folders are kept.
pub fn clipart_folders_path() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("clipart.json"))
}

/// Where the Library keeps the tags given to its files.
pub fn library_tags_path() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("library-tags.json"))
}

/// Where the Library tab keeps the previews it has made of models.
pub fn clipart_previews_path() -> Option<PathBuf> {
    cache_dir().map(|dir| dir.join("clipart-previews"))
}

/// Where Cut Craft keeps files it can make again: `%LOCALAPPDATA%\cut-craft` on Windows,
/// `~/Library/Caches/cut-craft` on macOS, elsewhere `$XDG_CACHE_HOME/cut-craft`, else
/// `~/.cache/cut-craft`.
fn cache_dir() -> Option<PathBuf> {
    cache_base().map(|dir| dir.join("cut-craft"))
}

#[cfg(windows)]
fn cache_base() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
}

#[cfg(target_os = "macos")]
fn cache_base() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Library/Caches"))
}

#[cfg(not(any(windows, target_os = "macos")))]
fn cache_base() -> Option<PathBuf> {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
}

/// The tool database at `path`; the starter tools when there is none yet.
pub fn load_tools(path: &Path) -> Result<ToolDatabase, String> {
    if !path.exists() {
        return Ok(ToolDatabase::starter());
    }
    ToolDatabase::from_json(&read(path)?).map_err(|e| format!("{}: {e}", path.display()))
}

/// Where the tool database is copied before it is first changed by one shape per bit.
pub fn tools_backup_path(path: &Path) -> std::path::PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_owned();
    name.push(".before-one-shape-per-bit");
    path.with_file_name(name)
}

/// [`load_tools`], each bit listed once per material joined into one
/// ([`ToolDatabase::join_material_copies`]): when any was, the file as it was is copied to
/// [`tools_backup_path`] (once) and the joined database saved. What was not kept, if any.
pub fn load_tools_joining(path: &Path) -> Result<(ToolDatabase, Vec<String>), String> {
    let mut db = load_tools(path)?;
    let before = db.tools.len();
    let report = db.join_material_copies();
    if db.tools.len() != before && path.exists() {
        let backup = tools_backup_path(path);
        if !backup.exists() {
            std::fs::copy(path, &backup).map_err(|e| format!("{}: {e}", backup.display()))?;
        }
        save_tools(&db, path)?;
    }
    Ok((db, report))
}

/// Tool database files Tool Database ▸ Import Tools reads.
pub const TOOL_FILE_EXTENSIONS: [&str; 3] = ["vtdb", "tools", "json"];

/// The tools in a Vectric `.vtdb`, a Fusion 360 `.tools` or a Cut Craft `.json` database.
pub fn read_tool_file(path: &Path) -> Result<ToolDatabase, String> {
    let ext = path
        .extension()
        .map_or_default(|e| e.to_string_lossy().to_lowercase());
    match ext.as_str() {
        "vtdb" => cut_craft_vtdb::read(path),
        "tools" => cut_craft_tool_database::fusion::read(&read(path)?)
            .map_err(|e| format!("{}: {e}", path.display())),
        _ => ToolDatabase::from_json(&read(path)?).map_err(|e| format!("{}: {e}", path.display())),
    }
}

pub fn save_tools(db: &ToolDatabase, path: &Path) -> Result<(), String> {
    write_creating(path, &db.to_json())
}

/// How many jobs the start page lists.
pub const RECENT_COUNT: usize = 4;
/// How many recent files are kept, whatever Options lists: its largest list size, so
/// making the list smaller and larger again loses none.
pub const RECENT_KEPT: usize = 20;

/// Toolpath templates: the toolpaths' settings, without their moves or vectors.
pub const TOOLPATH_TEMPLATE_EXTENSION: &str = "cctpt";

/// The toolpaths `ids` as a template file.
pub fn toolpath_template(doc: &Document, ids: &[cut_craft_core::document::EntityId]) -> Vec<u8> {
    let toolpaths: Vec<_> = ids
        .iter()
        .filter_map(|id| Some((id, doc.toolpaths.get(id)?)))
        .map(|(id, t)| {
            let mut t = cut_craft_core::toolpath::Toolpath {
                moves: vec![],
                vectors: vec![],
                power: vec![],
                drill_bank: None,
                fallbacks: vec![],
                across: vec![],
                ..t.clone()
            };
            // like its vectors, the picture it carved is this job's: another job's own
            // picture is carved instead
            drop_job_ids(&mut t.params);
            // its id here, so a Merged or Array Copy toolpath saved with it still names it
            // once both are loaded into another job (at other ids)
            if let Some(params) = t.params.as_object_mut() {
                params.insert(TEMPLATE_ID.into(), serde_json::json!(id.0));
            }
            t
        })
        .collect();
    serde_json::to_vec_pretty(&toolpaths).expect("toolpaths serialise")
}

/// The key a template keeps each toolpath's id under, in its settings.
const TEMPLATE_ID: &str = "template_id";

/// Each id in `params` naming another toolpath: a Merged toolpath's `sources`, and an
/// Array Copy's `sources` and `only` (the one it copies).
fn named_toolpaths(params: &mut serde_json::Value) -> Vec<&mut serde_json::Value> {
    let Some(params) = params.as_object_mut() else {
        return vec![];
    };
    params
        .iter_mut()
        .flat_map(|(key, value)| match key.as_str() {
            "sources" => value
                .as_array_mut()
                .map_or_else(Vec::new, |ids| ids.iter_mut().collect()),
            "only" if value.is_u64() => vec![value],
            _ => vec![],
        })
        .collect()
}

/// A template's toolpaths added, empty, ready to be edited and calculated on vectors.
pub fn load_toolpath_template(doc: &Document, bytes: &[u8]) -> Result<Change, String> {
    let toolpaths: Vec<cut_craft_core::toolpath::Toolpath> =
        serde_json::from_slice(bytes).map_err(|e| format!("not a toolpath template: {e}"))?;
    if toolpaths.is_empty() {
        return Err("the template holds no toolpaths".into());
    }
    let mut c = Change::builder(doc);
    // the template's ids, and where those toolpaths are now
    let mut moved = std::collections::BTreeMap::new();
    let mut added = vec![];
    for mut t in toolpaths {
        let was = t
            .params
            .as_object_mut()
            .and_then(|p| p.remove(TEMPLATE_ID))
            .and_then(|id| id.as_u64());
        added.push((c.add_toolpath(t.clone()), t));
        if let Some(was) = was {
            moved.insert(was, added.last().map(|(id, _)| *id).expect("just added"));
        }
    }
    // a Merged or Array Copy toolpath names the template's own toolpaths at their new ids;
    // one it named that the template left out (or an older template's) is of the job it
    // was saved from, so it names none of this job's
    for (id, mut t) in added {
        let names = named_toolpaths(&mut t.params);
        if names.is_empty() {
            continue;
        }
        let mut lost = false;
        for name in names {
            if let Some(now) = name.as_u64().and_then(|was| moved.get(&was)) {
                *name = serde_json::json!(now.0);
            } else {
                *name = serde_json::Value::Null;
                lost = true;
            }
        }
        if let Some(sources) = t.params.get_mut("sources").and_then(|s| s.as_array_mut()) {
            sources.retain(|s| !s.is_null());
        }
        if lost {
            if t.params.get("only").is_some_and(serde_json::Value::is_null)
                && let Some(params) = t.params.as_object_mut()
            {
                params.remove("only");
            }
            c.note(format!(
                "{} came without the toolpaths it was made from: tick the ones to use in the \
                 toolpath list, then calculate it",
                t.name
            ));
        }
        c.replace_toolpath(id, t);
    }
    Ok(c.finish())
}

/// A toolpath form's settings, saved to load back into the same form later.
pub const FORM_SETTINGS_EXTENSION: &str = "ccsettings";

/// The settings `params` of the form `id` (shown as `label`) as a settings file.
pub fn form_settings(id: &str, label: &str, params: &serde_json::Value) -> Vec<u8> {
    serde_json::to_vec_pretty(
        &serde_json::json!({ "form": id, "label": label, "settings": params }),
    )
    .expect("settings serialise")
}

/// A settings file's values put into the form `id`'s `params`: only the settings the form
/// has, so a file from an older version still loads.
pub fn load_form_settings(
    id: &str,
    label: &str,
    params: &mut serde_json::Value,
    bytes: &[u8],
) -> Result<(), String> {
    let file: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|e| format!("this is not a toolpath settings file ({e})"))?;
    let form = file["form"].as_str().unwrap_or_default();
    if form != id {
        let theirs = file["label"].as_str().unwrap_or(form);
        return Err(format!(
            "these settings are for {theirs}, not {label}: open {theirs} to load them"
        ));
    }
    let (Some(saved), Some(params)) = (file["settings"].as_object(), params.as_object_mut()) else {
        return Err("the file holds no settings".into());
    };
    let mut saved = saved.clone();
    // the picture a toolpath was made from belongs to its job: the form keeps its own
    for key in JOB_IDS {
        saved.remove(key);
    }
    merge_settings(params, &saved);
    Ok(())
}

/// Settings naming something of the job they were saved in (the picture a Photo V-Carve or
/// Sketch Carving carved), left out of templates and settings files.
const JOB_IDS: [&str; 1] = ["bitmap"];

/// `params` without the settings that name something of their job ([`JOB_IDS`]).
fn drop_job_ids(params: &mut serde_json::Value) {
    if let Some(params) = params.as_object_mut() {
        for key in JOB_IDS {
            params.remove(key);
        }
    }
}

/// `saved`'s values put into the `params` the form has, group by group, so a setting a
/// newer version added inside a group keeps its default.
fn merge_settings(
    params: &mut serde_json::Map<String, serde_json::Value>,
    saved: &serde_json::Map<String, serde_json::Value>,
) {
    for (key, value) in saved {
        match (params.get_mut(key), value) {
            (Some(serde_json::Value::Object(group)), serde_json::Value::Object(saved)) => {
                merge_settings(group, saved);
            }
            (Some(v), _) => v.clone_from(value),
            (None, _) => {}
        }
    }
}

/// Puts `path` first in `recent`, once, keeping the latest `keep` (Options; [`RECENT_COUNT`]
/// by default).
pub fn remember(recent: &mut Vec<PathBuf>, path: &Path, keep: usize) {
    recent.retain(|p| p != path);
    recent.insert(0, path.to_path_buf());
    recent.truncate(keep);
}

/// The recent files list; a missing or unreadable one is empty, as there is nothing to lose.
pub fn load_recent(path: &Path) -> Vec<PathBuf> {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

pub fn save_recent(recent: &[PathBuf], path: &Path) -> Result<(), String> {
    write_creating(
        path,
        &serde_json::to_vec_pretty(recent).expect("paths serialize"),
    )
}

/// Writes `bytes` to `path`, making its folder first.
pub fn write_creating(path: &Path, bytes: &[u8]) -> Result<(), String> {
    write_atomic(path, bytes).map_err(|e| format!("could not write {}: {e}", path.display()))
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("could not read {}: {e}", path.display()))
}

#[cfg(test)]
mod recovery_tests {
    use super::*;

    #[test]
    fn a_folder_that_cannot_lock_files_still_keeps_a_recovery_copy() {
        let dir = std::env::temp_dir().join(format!("cut-craft-no-locks-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (own, lock) = claim_recovery_locking(&dir, "12", |_| false).expect("a copy is kept");
        assert_eq!(own, dir.join("recovery-12.ccjob"));
        assert!(lock.is_none());
        // with no lock held, another window goes by whether the run is still going
        std::fs::write(&own, b"work").unwrap();
        let ours = dir.join("recovery-ours.ccjob");
        if cfg!(target_os = "linux") {
            assert!(process_running(std::process::id()));
            assert!(!process_running(u32::MAX));
        }
        assert_eq!(orphaned_recovery(&dir, &ours), Some(own));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

#[cfg(test)]
mod increment_tests {
    use super::incremented;
    use std::path::{Path, PathBuf};

    #[test]
    fn increment_counts_up_a_trailing_number_and_skips_taken_names() {
        let none = |_: &Path| false;
        assert_eq!(
            incremented(Path::new("/j/sign_1.ccjob"), none),
            PathBuf::from("/j/sign_2.ccjob")
        );
        assert_eq!(
            incremented(Path::new("/j/sign-9.ccjob"), none),
            PathBuf::from("/j/sign-10.ccjob")
        );
        assert_eq!(
            incremented(Path::new("/j/sign.ccjob"), none),
            PathBuf::from("/j/sign_1.ccjob")
        );
        let taken = |p: &Path| p.ends_with("sign_2.ccjob");
        assert_eq!(
            incremented(Path::new("/j/sign_1.ccjob"), taken),
            PathBuf::from("/j/sign_3.ccjob")
        );
    }

    #[test]
    fn increment_leaves_a_year_or_date_on_the_end_of_the_name_alone() {
        let none = |_: &Path| false;
        assert_eq!(
            incremented(Path::new("/j/Smith-2024.ccjob"), none),
            PathBuf::from("/j/Smith-2024_1.ccjob")
        );
        assert_eq!(
            incremented(Path::new("/j/order_20241115.ccjob"), none),
            PathBuf::from("/j/order_20241115_1.ccjob")
        );
        assert_eq!(
            incremented(Path::new("/j/order_2024-11-15.ccjob"), none),
            PathBuf::from("/j/order_2024-11-15_1.ccjob"),
            "a date written with dashes"
        );
        assert_eq!(
            incremented(Path::new("/j/Smith 2024_11_05.ccjob"), none),
            PathBuf::from("/j/Smith 2024_11_05_1.ccjob")
        );
        assert_eq!(
            incremented(Path::new("/j/order_15-11-2024.ccjob"), none),
            PathBuf::from("/j/order_15-11-2024_1.ccjob")
        );
        assert_eq!(
            incremented(Path::new("/j/order_2024-11-15_1.ccjob"), none),
            PathBuf::from("/j/order_2024-11-15_2.ccjob"),
            "the counter after a date keeps counting"
        );
        assert_eq!(
            incremented(Path::new("/j/Smith_2024_1.ccjob"), none),
            PathBuf::from("/j/Smith_2024_2.ccjob"),
            "a counter after a year keeps counting"
        );
        assert_eq!(
            incremented(Path::new("/j/sign_999.ccjob"), none),
            PathBuf::from("/j/sign_1000.ccjob"),
            "a counter it made itself keeps counting"
        );
    }
}

#[cfg(test)]
mod tests;
