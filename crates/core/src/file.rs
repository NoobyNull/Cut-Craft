//! The native job file: the whole document as versioned, self-describing JSON.

use crate::document::Document;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

pub const JOB_EXTENSION: &str = "ccjob";
/// Jobs saved before the app was renamed Cut Craft; they still open.
pub const OLD_JOB_EXTENSION: &str = "ajob";
/// Job templates: jobs that open as new, unsaved files.
pub const TEMPLATE_EXTENSION: &str = "ccjobt";
const FORMAT: &str = "cut-craft-job";
const OLD_FORMAT: &str = "aspire-clone-job";
/// Bump when the document's JSON shape changes, and teach `load_job` to read the old one.
const VERSION: u32 = 1;

/// Where Cut Craft keeps its settings: `%APPDATA%\cut-craft` on Windows,
/// `~/Library/Application Support/cut-craft` on macOS, elsewhere `$XDG_CONFIG_HOME/cut-craft`,
/// else `~/.config/cut-craft`.
pub fn config_dir() -> Option<PathBuf> {
    config_base().map(|dir| dir.join("cut-craft"))
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
            Self::NotAJob => f.write_str("this is not a Cut Craft job file"),
            Self::NewerVersion(v) => {
                write!(
                    f,
                    "this job was saved by a newer version (format {v}; this build reads up to {VERSION})"
                )
            }
            Self::Damaged(why) => write!(f, "the job file is damaged: {why}"),
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

pub fn load_job(bytes: &[u8]) -> Result<Document, LoadError> {
    let header: Header = serde_json::from_slice(bytes).map_err(|_| LoadError::NotAJob)?;
    if header.format != FORMAT && header.format != OLD_FORMAT {
        return Err(LoadError::NotAJob);
    }
    if header.version > VERSION {
        return Err(LoadError::NewerVersion(header.version));
    }
    let loaded: Loaded =
        serde_json::from_slice(bytes).map_err(|e| LoadError::Damaged(e.to_string()))?;
    Ok(loaded.document)
}
