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
/// `$XDG_CONFIG_HOME/powerwhittle`, else `~/.config/powerwhittle`.
pub fn config_dir() -> Option<PathBuf> {
    config_base().map(|dir| dir.join(SETTINGS_FOLDER))
}

/// On the first start after the rename, copies the old settings folder in `base` to the new
/// one, so everything set before carries on. The old folder is left as it was, so going back
/// to an older copy still finds its settings. Whether anything was copied: nothing is when
/// the new folder is there already or there was never an old one.
pub fn bring_over_settings(base: &Path) -> std::io::Result<bool> {
    let (old, new) = (base.join(OLD_SETTINGS_FOLDER), base.join(SETTINGS_FOLDER));
    if new.exists() || !old.is_dir() {
        return Ok(false);
    }
    // copied beside, then renamed into place: a copy cut short is never taken as done
    let part = base.join(format!("{SETTINGS_FOLDER}.copying"));
    let _ = std::fs::remove_dir_all(&part);
    copy_folder(&old, &part)?;
    std::fs::rename(&part, &new)?;
    Ok(true)
}

/// [`bring_over_settings`] in this system's settings place.
pub fn bring_over_old_settings() -> std::io::Result<bool> {
    config_base().map_or(Ok(false), |base| bring_over_settings(&base))
}

fn copy_folder(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_folder(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
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
