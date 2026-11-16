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

    pub fn save(&mut self, doc: &Document, history: &History, path: &Path) -> Result<(), String> {
        write_atomic(path, &save_job(doc))
            .map_err(|e| format!("could not save {}: {e}", path.display()))?;
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
    let bytes = (exporter.export)(doc).map_err(|e| e.to_string())?;
    write_atomic(path, &bytes).map_err(|e| format!("could not write {}: {e}", path.display()))
}

/// Writes the toolpaths through `post` to `path`, tools numbered by their tool changer
/// `slots` (empty: in the order used); `job` is the saved job they come from.
pub fn save_through(
    post: &Post,
    slots: &[Option<String>],
    doc: &Document,
    path: &Path,
    job: Option<&Path>,
) -> Result<(), String> {
    let made = chrono::Local::now().naive_local();
    let file = cut_craft_post::File {
        path,
        job,
        made,
        slots,
    };
    let text = post.render(doc, &file)?;
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
            (Some(name), _) => match doc.toolpaths_in_group(name) {
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
        let stale = path.extension().is_some_and(|e| e == "lock")
            && is_recovery(&path)
            && !path.with_extension("ccjob").exists()
            && !recovery_in_use(&path);
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
        let _ = std::fs::rename(&own, dir.join(format!("recovery-{run}-{stamp}.ccjob")));
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

/// A recovery copy in `dir` left by a run that ended without saving or closing its job (a
/// crash), the newest first; not `own`, and not one a window still running keeps. A copy
/// with no lock (an older version's `recovery.ccjob`) counts as left behind.
pub fn orphaned_recovery(dir: &Path, own: &Path) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p != own && p.extension().is_some_and(|e| e == "ccjob") && is_recovery(p))
        .filter(|p| !recovery_in_use(&p.with_extension("lock")))
        .max_by_key(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok())
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
    let ours = |name: &str| {
        let Some(rest) = name
            .strip_suffix(&format!(".{extension}"))
            .and_then(|n| n.strip_prefix(stem.as_str()))
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
    };
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
        .filter_map(|id| doc.toolpaths.get(id))
        .map(|t| cut_craft_core::toolpath::Toolpath {
            moves: vec![],
            vectors: vec![],
            power: vec![],
            drill_bank: None,
            fallbacks: vec![],
            ..t.clone()
        })
        .collect();
    serde_json::to_vec_pretty(&toolpaths).expect("toolpaths serialise")
}

/// A template's toolpaths added, empty, ready to be edited and calculated on vectors.
pub fn load_toolpath_template(doc: &Document, bytes: &[u8]) -> Result<Change, String> {
    let toolpaths: Vec<cut_craft_core::toolpath::Toolpath> =
        serde_json::from_slice(bytes).map_err(|e| format!("not a toolpath template: {e}"))?;
    if toolpaths.is_empty() {
        return Err("the template holds no toolpaths".into());
    }
    let mut c = Change::builder(doc);
    for t in toolpaths {
        c.add_toolpath(t);
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
    merge_settings(params, saved);
    Ok(())
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
mod tests {
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
        super::load_form_settings("texture_toolpath", "Texture Toolpath", &mut params, &bytes)
            .unwrap();
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
        super::load_form_settings("profile_toolpath", "Profile Toolpath", &mut params, &bytes)
            .unwrap();
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
        let why =
            save_files(&no_changes, &doc, &[ids[..3].to_vec()], path, None, false).unwrap_err();
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
}
