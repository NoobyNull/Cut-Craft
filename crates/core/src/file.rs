//! The native job file: the whole document as versioned, self-describing JSON.

use crate::document::Document;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

pub const JOB_EXTENSION: &str = "pwjob";
/// Jobs saved before the app was renamed `PowerWhittle`: the same file, so they still open.
pub const CUT_CRAFT_JOB_EXTENSION: &str = "ccjob";
/// Jobs saved before the app was renamed Cut Craft; they still open.
pub const OLD_JOB_EXTENSION: &str = "ajob";
/// Job templates: jobs that open as new, unsaved files.
pub const TEMPLATE_EXTENSION: &str = "pwjobt";
/// Job templates saved before the app was renamed `PowerWhittle`.
pub const CUT_CRAFT_TEMPLATE_EXTENSION: &str = "ccjobt";
const FORMAT: &str = "cut-craft-job";
const OLD_FORMAT: &str = "aspire-clone-job";
/// Bump when the document's JSON shape changes, and teach `load_job` to read the old one.
const VERSION: u32 = 1;
/// Every extension a job file opens under, the one jobs are saved as first.
pub const JOB_EXTENSIONS: [&str; 3] = [JOB_EXTENSION, CUT_CRAFT_JOB_EXTENSION, OLD_JOB_EXTENSION];
/// Every extension a job template opens under, the one templates are saved as first.
pub const TEMPLATE_EXTENSIONS: [&str; 2] = [TEMPLATE_EXTENSION, CUT_CRAFT_TEMPLATE_EXTENSION];

/// Whether `path` is a job template, which opens as a new, unsaved job.
pub fn is_template(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
        TEMPLATE_EXTENSIONS
            .iter()
            .any(|t| e.eq_ignore_ascii_case(t))
    })
}

/// The settings folder's name, in the place each system keeps settings.
pub const SETTINGS_FOLDER: &str = "powerwhittle";
/// Its name before the app was renamed `PowerWhittle`.
pub const OLD_SETTINGS_FOLDER: &str = "cut-craft";

/// Where the app keeps its settings: `%APPDATA%\powerwhittle` on Windows,
/// `~/Library/Application Support/powerwhittle` on macOS, elsewhere
/// `$XDG_CONFIG_HOME/powerwhittle`, else `~/.config/powerwhittle`. Until
/// [`bring_over_settings`] has finished, the old folder instead ([`settings_folder`]).
pub fn config_dir() -> Option<PathBuf> {
    config_base().map(|base| settings_folder(&base))
}

/// The settings folder in `base`: the new one, unless the old one is there and has not all
/// come over yet. Then the old one, so nothing the app saves can shadow a setting still to come
/// over, and the next start brings everything across, changes and all.
pub fn settings_folder(base: &Path) -> PathBuf {
    let (old, new) = (base.join(OLD_SETTINGS_FOLDER), base.join(SETTINGS_FOLDER));
    if old.is_dir() && !new.join(BROUGHT_OVER).exists() {
        old
    } else {
        new
    }
}

/// Left in the new settings folder once everything from the old one is in it.
pub const BROUGHT_OVER: &str = ".brought-over-from-cut-craft";
/// Locked in the new settings folder while a bring-over runs.
const BRINGING_OVER: &str = ".bringing-over.lock";

/// On the first start after the rename, copies the old settings folder in `base` into the new
/// one, so everything set before carries on. The old folder is left as it was, so going back
/// to an older copy still finds its settings. A file already in the new folder is kept unless
/// the old one changed after it, so a start that stopped part way is finished the next time
/// with what was saved meanwhile in the old folder ([`settings_folder`]), and a file put there
/// by hand first stays. [`BROUGHT_OVER`] marks it done only once every file is in. Two starts
/// at once take turns, and the second finds it done. Whether anything was copied: nothing is
/// once it is done, or when there was never an old folder.
pub fn bring_over_settings(base: &Path) -> std::io::Result<bool> {
    let (old, new) = (base.join(OLD_SETTINGS_FOLDER), base.join(SETTINGS_FOLDER));
    if !old.is_dir() || new.join(BROUGHT_OVER).exists() {
        return Ok(false);
    }
    // one bring-over at a time, so none replaces a file after another has finished and the
    // app has saved over it; the lock goes with the file when this returns
    std::fs::create_dir_all(&new)?;
    let lock = std::fs::File::create(new.join(BRINGING_OVER))?;
    lock.lock()?;
    if new.join(BROUGHT_OVER).exists() {
        return Ok(false);
    }
    copy_missing(&old, &new)?;
    std::fs::write(new.join(BROUGHT_OVER), "")?;
    Ok(true)
}

/// [`bring_over_settings`] in this system's settings place.
pub fn bring_over_old_settings() -> std::io::Result<bool> {
    config_base().map_or(Ok(false), |base| bring_over_settings(&base))
}

/// Copies what `to` lacks, or has only an older copy of, from `from`. A copy's time is when it
/// was made, so a file that came over before is copied again only when the old one has changed
/// since. Each file is written beside its place first and then linked into it, so a copy cut
/// short is never taken for the whole file, and two windows started together never write over
/// each other's files.
fn copy_missing(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_missing(&entry.path(), &target)?;
            continue;
        }
        let made = entry.metadata()?.modified()?;
        if is_older(&target, made) {
            let mut part = target.clone().into_os_string();
            part.push(format!(".copying-{}", std::process::id()));
            std::fs::copy(entry.path(), &part)?;
            // a link is made only where there is no file yet, never over one. An older copy is
            // replaced only while it is still older: until the bring-over is done nothing else
            // writes here ([`settings_folder`]), and bring-overs take turns. A folder that cannot
            // link files fails the bring-over, so the app keeps to the old folder.
            let linked = match std::fs::hard_link(&part, &target) {
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    if is_older(&target, made) {
                        std::fs::rename(&part, &target)
                    } else {
                        Ok(())
                    }
                }
                done => done,
            };
            let _ = std::fs::remove_file(&part);
            linked?;
        }
    }
    Ok(())
}

/// Whether `file` is missing, or was last changed before `than`.
fn is_older(file: &Path, than: std::time::SystemTime) -> bool {
    std::fs::metadata(file)
        .and_then(|m| m.modified())
        .map_or(true, |changed| changed < than)
}

#[cfg(windows)]
fn config_base() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(PathBuf::from)
}

/// macOS keeps applications' settings in the user's Library.
#[cfg(target_os = "macos")]
fn config_base() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Library/Application Support"))
}

#[cfg(not(any(windows, target_os = "macos")))]
fn config_base() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
}

#[derive(Serialize)]
struct Saved<'a> {
    format: &'static str,
    version: u32,
    document: &'a Document,
}

#[derive(Deserialize)]
struct Header {
    format: String,
    version: u32,
}

#[derive(Deserialize)]
struct Loaded {
    document: Document,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoadError {
    NotAJob,
    NewerVersion(u32),
    Damaged(String),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAJob => f.write_str("this is not a PowerWhittle job file"),
            Self::NewerVersion(v) => {
                write!(
                    f,
                    "this job was saved by a newer version of PowerWhittle (format {v}; this build \
                     reads up to {VERSION}). Update PowerWhittle (Help ▸ Check for Updates) to open it"
                )
            }
            Self::Damaged(why) => write!(
                f,
                "the job file is damaged ({why}). It may have been cut short while being copied \
                 or synced: open a backup or an earlier copy of it if you have one"
            ),
        }
    }
}

impl std::error::Error for LoadError {}

/// Writes `bytes` to `path` all at once: into a sibling `.tmp` file first, synced, then
/// renamed over `path`, so a crash, a full disk or a power cut mid-write leaves the old file
/// as it was. Makes the folder first.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write as _;
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    let written = std::fs::File::create(&tmp)
        .and_then(|mut f| f.write_all(bytes).and_then(|()| f.sync_all()))
        .and_then(|()| std::fs::rename(&tmp, path));
    if written.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    written
}

pub fn save_job(doc: &Document) -> Vec<u8> {
    serde_json::to_vec_pretty(&Saved {
        format: FORMAT,
        version: VERSION,
        document: doc,
    })
    .expect("documents always serialize")
}

/// Whether `bytes` start like a job file: JSON naming the job format near its start, even
/// if what follows is cut short or broken.
fn looks_like_a_job(bytes: &[u8]) -> bool {
    let text = String::from_utf8_lossy(&bytes[..bytes.len().min(256)]);
    text.trim_start().starts_with('{')
        && [FORMAT, OLD_FORMAT]
            .iter()
            .any(|f| text.contains(&format!("\"{f}\"")))
}

pub fn load_job(bytes: &[u8]) -> Result<Document, LoadError> {
    let header: Header = serde_json::from_slice(bytes).map_err(|e| {
        if looks_like_a_job(bytes) {
            LoadError::Damaged(e.to_string())
        } else {
            LoadError::NotAJob
        }
    })?;
    if header.format != FORMAT && header.format != OLD_FORMAT {
        return Err(LoadError::NotAJob);
    }
    if header.version > VERSION {
        return Err(LoadError::NewerVersion(header.version));
    }
    let loaded: Loaded =
        serde_json::from_slice(bytes).map_err(|e| LoadError::Damaged(e.to_string()))?;
    let mut doc = loaded.document;
    // 4-Axis Rotary Machining's strategies have toolpaths of their own now
    crate::rotary_upgrade::upgrade(&mut doc);
    Ok(doc)
}

/// [`load_job`], with the settings in the file this build does not know (a newer version
/// added them), by their place in the document (`job.glitter`). They are not kept: saving
/// the job from this build leaves them out.
pub fn load_job_noting(bytes: &[u8]) -> Result<(Document, Vec<String>), LoadError> {
    let doc = load_job(bytes)?;
    let mut unknown = Vec::new();
    let mut reader = serde_json::Deserializer::from_slice(bytes);
    let _ = serde_ignored::deserialize(&mut reader, |path| {
        let path = path.to_string();
        // the document's own settings, not the file's header
        if let Some(inside) = path.strip_prefix("document.") {
            unknown.push(inside.to_owned());
        }
    })
    .map(|loaded: Loaded| loaded.document);
    Ok((doc, unknown))
}
