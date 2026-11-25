//! Help ▸ Back Up Settings: what the user has set up in Cut Craft, in one `.ccbackup` file.
//!
//! A backup is an ordinary zip, so it can still be opened by hand. It holds `manifest.json`
//! (the Cut Craft version, the computer's system, when and why it was made, its parts, and a
//! checksum of every file) and the files themselves under `settings/`, by their place in the
//! settings folder. What goes in is chosen by [`Part`]: settings, machines, bits and the
//! user's own files. Never the recent files list, crash recovery copies, logs or caches, and
//! add-ons only as the record of where each came from (they are downloaded again), not their
//! programs. Secrets ([`SECRETS`]) are taken out unless asked for.
//!
//! [`Automatic`] makes backups into the settings folder's `backups` folder before big changes
//! (an update, a layout switch, a library rewrite) and once a week, keeping the newest few.
//!
//! Planned for stage 2 (designed, not built): restoring a backup with a preview, Merge or
//! Replace asked per part, and folders missing on this computer listed with Choose folder.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};

/// A backup file's extension.
pub const EXTENSION: &str = "ccbackup";
/// The settings folder's folder for automatic backups.
pub const FOLDER: &str = "backups";
/// How many automatic backups are kept unless Options says otherwise.
pub const KEEP: usize = 10;
/// The most automatic backups Options lets be kept.
pub const MOST_KEPT: usize = 100;
/// How long after a backup the weekly one is due, in seconds.
pub const WEEK: u64 = 7 * 24 * 60 * 60;
/// Where an add-on's folder says where it came from (the updater's `addon::RECORD`).
const ADD_ON_RECORD: &str = "installed.json";
const ADD_ONS: &str = "addons";
const FORMAT: &str = "cut-craft-backup";
/// Bump when the backup's shape changes, and teach [`read_manifest`] to read the old one.
const VERSION: u32 = 1;
const MANIFEST: &str = "manifest.json";
const FILES: &str = "settings/";

/// Values in the settings files that let someone else in: left out of a backup unless
/// [`Choice::secrets`] is set. Each is a file and the path of keys to the value.
pub const SECRETS: [(&str, &[&str]); 1] = [("options.json", &["web_api", "token"])];

/// A part of what Cut Craft keeps, ticked or not when backing up.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Part {
    /// Options, the window layout, favourites, Job Defaults and learned job times.
    Settings,
    /// Machines and the post processors added.
    Machines,
    /// The Bit Library: bits, groups and cutting data.
    BitLibrary,
    /// The Materials Library.
    Materials,
    /// Job, toolpath and form templates.
    Templates,
    /// Sculpting brushes, textures and fonts added.
    Brushes,
    /// My Gadgets.
    Gadgets,
    /// The Library's folders and the tags given to its files.
    Library,
    /// Words added to the spelling list.
    Words,
    /// The add-ons installed, as the list of where each came from.
    AddOns,
    /// The unsigned add-ons and gadgets allowed to run.
    UnsignedAllowed,
}

/// Where a part's files are, in the settings folder.
enum Place {
    File(&'static str),
    Folder(&'static str),
    /// Each add-on's record of where it came from: `addons/<id>/installed.json`.
    AddOnRecords,
}

impl Part {
    /// Every part, in the order they are listed.
    pub const ALL: [Self; 11] = [
        Self::Settings,
        Self::Machines,
        Self::BitLibrary,
        Self::Materials,
        Self::Templates,
        Self::Brushes,
        Self::Gadgets,
        Self::Library,
        Self::Words,
        Self::AddOns,
        Self::UnsignedAllowed,
    ];

    /// Its name's key in the app's words (`backup-part-bit-library`); its description is the
    /// key's `.tooltip`.
    pub fn key(self) -> &'static str {
        match self {
            Self::Settings => "backup-part-settings",
            Self::Machines => "backup-part-machines",
            Self::BitLibrary => "backup-part-bit-library",
            Self::Materials => "backup-part-materials",
            Self::Templates => "backup-part-templates",
            Self::Brushes => "backup-part-brushes",
            Self::Gadgets => "backup-part-gadgets",
            Self::Library => "backup-part-library",
            Self::Words => "backup-part-words",
            Self::AddOns => "backup-part-add-ons",
            Self::UnsignedAllowed => "backup-part-unsigned-allowed",
        }
    }

    fn places(self) -> &'static [Place] {
        match self {
            Self::Settings => &[Place::File("options.json"), Place::File("job-times.json")],
            Self::Machines => &[Place::File("machines.json"), Place::Folder("posts")],
            Self::BitLibrary => &[Place::File("tools.json")],
            Self::Materials => &[Place::File("materials.json")],
            Self::Templates => &[Place::Folder("templates")],
            Self::Brushes => &[
                Place::Folder("brushes"),
                Place::Folder("textures"),
                Place::Folder("fonts"),
            ],
            Self::Gadgets => &[Place::Folder("gadgets")],
            Self::Library => &[
                Place::File("clipart.json"),
                Place::File("library-tags.json"),
            ],
            Self::Words => &[Place::File("words.txt")],
            Self::AddOns => &[Place::AddOnRecords],
            Self::UnsignedAllowed => &[Place::File("unsigned-code-allowed.json")],
        }
    }
}

/// What to put in a backup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    pub parts: BTreeSet<Part>,
    /// Keep [`SECRETS`] in, rather than taking them out.
    pub secrets: bool,
}

impl Default for Choice {
    /// Every part, secrets left out.
    fn default() -> Self {
        Self {
            parts: Part::ALL.into_iter().collect(),
            secrets: false,
        }
    }
}

/// Why a backup was made.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// Help ▸ Back Up Settings.
    ByHand,
    BeforeUpdate,
    BeforeLayoutChange,
    /// First-time setup run again, which rewrites the machine.
    BeforeSetup,
    /// Cut Craft rewriting a library itself, as joining one bit per material did.
    BeforeLibraryChange,
    Weekly,
}

impl Reason {
    /// Its words' key in the app (`backup-reason-weekly`).
    pub fn key(self) -> &'static str {
        match self {
            Self::ByHand => "backup-reason-by-hand",
            Self::BeforeUpdate => "backup-reason-before-update",
            Self::BeforeLayoutChange => "backup-reason-before-layout-change",
            Self::BeforeSetup => "backup-reason-before-setup",
            Self::BeforeLibraryChange => "backup-reason-before-library-change",
            Self::Weekly => "backup-reason-weekly",
        }
    }

    /// The end of an automatic backup's file name.
    fn slug(self) -> &'static str {
        match self {
            Self::ByHand => "by-hand",
            Self::BeforeUpdate => "before-update",
            Self::BeforeLayoutChange => "before-layout-change",
            Self::BeforeSetup => "before-setup",
            Self::BeforeLibraryChange => "before-library-change",
            Self::Weekly => "weekly",
        }
    }
}

/// What a backup holds, kept in it as `manifest.json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub format: String,
    pub version: u32,
    /// The Cut Craft that made it.
    pub app_version: String,
    /// The system it was made on (`linux`, `windows`, `macos`) and its processor: folders
    /// in it are that system's.
    pub os: String,
    pub arch: String,
    /// When, in seconds since 1970.
    pub created: u64,
    pub reason: Reason,
    pub parts: Vec<Part>,
    /// Whether [`SECRETS`] were kept in.
    pub secrets: bool,
    pub files: Vec<Entry>,
}

/// One file in a backup.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// Its place in the settings folder, with `/` between folders on every system.
    pub path: String,
    pub part: Part,
    pub size: u64,
    /// Its SHA-256, in hex.
    pub sha256: String,
}

#[derive(Debug)]
pub enum Error {
    /// A file could not be read or written.
    File(PathBuf, std::io::Error),
    /// Not a Cut Craft backup, or damaged.
    NotABackup(PathBuf),
    /// Made by a newer Cut Craft, in a shape this one cannot read.
    NewerVersion(u32),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::File(path, e) => write!(f, "{}: {e}", path.display()),
            Self::NotABackup(path) => write!(f, "{}: not a Cut Craft backup", path.display()),
            Self::NewerVersion(v) => write!(f, "made by a newer Cut Craft (backup format {v})"),
        }
    }
}

impl std::error::Error for Error {}

/// Seconds since 1970 by this computer's clock: when a backup is made.
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// One file gathered for a backup: its entry and what goes in.
struct Gathered {
    entry: Entry,
    bytes: Vec<u8>,
}

/// Writes a backup of `choice` from the settings folder `config` to `to`, replacing it only
/// once the whole backup is written. What it holds.
pub fn write(
    config: &Path,
    choice: &Choice,
    reason: Reason,
    app_version: &str,
    now: u64,
    to: &Path,
) -> Result<Manifest, Error> {
    let gathered = gather(config, choice)?;
    write_gathered(&gathered, choice, reason, app_version, now, to)
}

fn write_gathered(
    gathered: &[Gathered],
    choice: &Choice,
    reason: Reason,
    app_version: &str,
    now: u64,
    to: &Path,
) -> Result<Manifest, Error> {
    let manifest = Manifest {
        format: FORMAT.to_owned(),
        version: VERSION,
        app_version: app_version.to_owned(),
        os: std::env::consts::OS.to_owned(),
        arch: std::env::consts::ARCH.to_owned(),
        created: now,
        reason,
        parts: choice.parts.iter().copied().collect(),
        secrets: choice.secrets,
        files: gathered.iter().map(|g| g.entry.clone()).collect(),
    };
    let mut part = to.as_os_str().to_owned();
    part.push(".part");
    let part = PathBuf::from(part);
    let written = zip_up(&manifest, gathered, &part)
        .and_then(|()| std::fs::rename(&part, to).map_err(|e| Error::File(to.to_owned(), e)));
    if written.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    written.map(|()| manifest)
}

fn zip_up(manifest: &Manifest, gathered: &[Gathered], to: &Path) -> Result<(), Error> {
    let failed = |e: std::io::Error| Error::File(to.to_owned(), e);
    let zipped = |e: zip::result::ZipError| failed(std::io::Error::other(e));
    if let Some(folder) = to.parent() {
        std::fs::create_dir_all(folder).map_err(|e| Error::File(folder.to_owned(), e))?;
    }
    let file = std::fs::File::create(to).map_err(failed)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let json = serde_json::to_vec_pretty(manifest).expect("a manifest serialises");
    zip.start_file(MANIFEST, options).map_err(zipped)?;
    zip.write_all(&json).map_err(failed)?;
    for g in gathered {
        zip.start_file(format!("{FILES}{}", g.entry.path), options)
            .map_err(zipped)?;
        zip.write_all(&g.bytes).map_err(failed)?;
    }
    zip.finish().map_err(zipped)?.sync_all().map_err(failed)
}

/// The start of a manifest, read first: a backup of a newer shape still says which it is.
#[derive(Deserialize)]
struct Header {
    format: String,
    version: u32,
}

/// What a backup file holds, read from its manifest.
pub fn read_manifest(file: &Path) -> Result<Manifest, Error> {
    let open = std::fs::File::open(file).map_err(|e| Error::File(file.to_owned(), e))?;
    let not = || Error::NotABackup(file.to_owned());
    let mut zip = zip::ZipArchive::new(open).map_err(|_| not())?;
    let mut text = String::new();
    zip.by_name(MANIFEST)
        .map_err(|_| not())?
        .read_to_string(&mut text)
        .map_err(|_| not())?;
    let header: Header = serde_json::from_str(&text).map_err(|_| not())?;
    if header.format != FORMAT {
        return Err(not());
    }
    if header.version > VERSION {
        return Err(Error::NewerVersion(header.version));
    }
    serde_json::from_str(&text).map_err(|_| not())
}

/// Every file `choice` takes from `config`, in a steady order, secrets taken out.
fn gather(config: &Path, choice: &Choice) -> Result<Vec<Gathered>, Error> {
    let mut gathered = vec![];
    for &part in &choice.parts {
        for place in part.places() {
            for path in place_files(config, place)? {
                let bytes = std::fs::read(config.join(&path))
                    .map_err(|e| Error::File(config.join(&path), e))?;
                let bytes = if choice.secrets {
                    bytes
                } else {
                    without_secrets(&path, bytes)
                };
                gathered.push(Gathered {
                    entry: Entry {
                        sha256: sha256(&bytes),
                        size: bytes.len() as u64,
                        path,
                        part,
                    },
                    bytes,
                });
            }
        }
    }
    Ok(gathered)
}

/// The files at `place` that are there, by their `/` path in `config`. A folder that is not
/// there holds nothing; one that cannot be read fails the backup rather than leave it short.
fn place_files(config: &Path, place: &Place) -> Result<Vec<String>, Error> {
    let mut found = vec![];
    match place {
        Place::File(name) => {
            if config.join(name).is_file() {
                found.push((*name).to_owned());
            }
        }
        Place::Folder(name) => walk(&config.join(name), name, &mut found)?,
        Place::AddOnRecords => {
            for entry in entries(&config.join(ADD_ONS))? {
                if entry.path().join(ADD_ON_RECORD).is_file() {
                    found.push(format!(
                        "{ADD_ONS}/{}/{ADD_ON_RECORD}",
                        entry.file_name().to_string_lossy()
                    ));
                }
            }
        }
    }
    found.sort();
    Ok(found)
}

/// What is in `dir`: nothing when it is not there, an error when it cannot be read.
fn entries(dir: &Path) -> Result<Vec<std::fs::DirEntry>, Error> {
    let failed = |e| Error::File(dir.to_owned(), e);
    match std::fs::read_dir(dir) {
        Ok(read) => read.collect::<Result<_, _>>().map_err(failed),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(vec![]),
        Err(e) => Err(failed(e)),
    }
}

/// Every file under `dir`, named `prefix/...`. A link to a folder is not followed, so a link
/// back up the tree cannot make the walk go round for ever; a link to a file is backed up as
/// the file.
fn walk(dir: &Path, prefix: &str, found: &mut Vec<String>) -> Result<(), Error> {
    for entry in entries(dir)? {
        let name = format!("{prefix}/{}", entry.file_name().to_string_lossy());
        let kind = entry
            .file_type()
            .map_err(|e| Error::File(entry.path(), e))?;
        if kind.is_dir() {
            walk(&entry.path(), &name, found)?;
        } else if entry.path().is_file() {
            found.push(name);
        }
    }
    Ok(())
}

/// `bytes` of the file at `path` with [`SECRETS`] taken out; as they are when there are none,
/// or the file does not read as JSON.
fn without_secrets(path: &str, bytes: Vec<u8>) -> Vec<u8> {
    let keys: Vec<&[&str]> = SECRETS
        .iter()
        .filter(|(file, _)| *file == path)
        .map(|(_, keys)| *keys)
        .collect();
    if keys.is_empty() {
        return bytes;
    }
    let Ok(mut json) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return bytes;
    };
    let mut changed = false;
    for keys in keys {
        if let Some((last, path)) = keys.split_last() {
            let mut at = &mut json;
            for key in path {
                at = &mut at[*key];
            }
            changed |= at
                .as_object_mut()
                .and_then(|object| object.remove(*last))
                .is_some();
        }
    }
    if changed {
        serde_json::to_vec_pretty(&json).expect("JSON serialises")
    } else {
        bytes
    }
}

fn sha256(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    sha2::Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut hex, b| {
            use fmt::Write as _;
            let _ = write!(hex, "{b:02x}");
            hex
        })
}

/// A backup in the backups folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Kept {
    pub path: PathBuf,
    pub manifest: Manifest,
    /// The file's size, in bytes.
    pub size: u64,
}

/// The backups Cut Craft makes by itself, into the settings folder's [`FOLDER`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Automatic {
    /// The settings folder.
    pub config: PathBuf,
    /// How many automatic backups to keep: older ones are removed. Backups saved into the
    /// folder by hand are never removed.
    pub keep: usize,
    pub app_version: String,
}

impl Automatic {
    pub fn new(config: PathBuf, keep: usize, app_version: &str) -> Self {
        Self {
            config,
            keep,
            app_version: app_version.to_owned(),
        }
    }

    /// Where they are kept.
    pub fn folder(&self) -> PathBuf {
        self.config.join(FOLDER)
    }

    /// Backs up every part, secrets left out, for `reason` at `now`, then removes the oldest
    /// beyond [`Self::keep`]. `None` when there is nothing to back up yet, or the files are
    /// just as a kept backup holds them.
    pub fn make(&self, reason: Reason, now: u64) -> Result<Option<PathBuf>, Error> {
        let choice = Choice::default();
        let gathered = gather(&self.config, &choice)?;
        if gathered.is_empty() {
            return Ok(None);
        }
        let same = |kept: &Kept| {
            kept.manifest.files.len() == gathered.len()
                && kept
                    .manifest
                    .files
                    .iter()
                    .zip(&gathered)
                    .all(|(a, b)| a.path == b.entry.path && a.sha256 == b.entry.sha256)
        };
        if self.kept().iter().any(same) {
            return Ok(None);
        }
        let to = self.free_name(reason, now);
        write_gathered(&gathered, &choice, reason, &self.app_version, now, &to)?;
        self.prune();
        Ok(Some(to))
    }

    /// Whether the weekly backup is due at `now`: no backup in the folder is under a week old.
    pub fn weekly_due(&self, now: u64) -> bool {
        self.kept()
            .first()
            .is_none_or(|newest| now.saturating_sub(newest.manifest.created) >= WEEK)
    }

    /// The backups in the folder, newest first. Files that are not backups are left out.
    pub fn kept(&self) -> Vec<Kept> {
        let mut kept: Vec<Kept> = std::fs::read_dir(self.folder())
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == EXTENSION))
            .filter_map(|path| {
                let manifest = read_manifest(&path).ok()?;
                let size = std::fs::metadata(&path).map_or(0, |m| m.len());
                Some(Kept {
                    path,
                    manifest,
                    size,
                })
            })
            .collect();
        kept.sort_by(|a, b| {
            b.manifest
                .created
                .cmp(&a.manifest.created)
                .then_with(|| b.path.cmp(&a.path))
        });
        kept
    }

    /// Removes the oldest automatic backups beyond [`Self::keep`].
    fn prune(&self) {
        let automatic = self
            .kept()
            .into_iter()
            .filter(|k| k.manifest.reason != Reason::ByHand);
        for old in automatic.skip(self.keep.max(1)) {
            let _ = std::fs::remove_file(old.path);
        }
    }

    /// A name for a backup made at `now` for `reason` that no file has yet: the local date
    /// and time, then why (`2026-11-25 20-38-12 before-update.ccbackup`).
    fn free_name(&self, reason: Reason, now: u64) -> PathBuf {
        let when = i64::try_from(now)
            .ok()
            .and_then(|s| chrono::DateTime::from_timestamp(s, 0))
            .unwrap_or_default()
            .with_timezone(&chrono::Local)
            .format("%Y-%m-%d %H-%M-%S");
        let base = format!("{when} {}", reason.slug());
        let mut path = self.folder().join(format!("{base}.{EXTENSION}"));
        let mut n = 2;
        while path.exists() {
            path = self.folder().join(format!("{base} {n}.{EXTENSION}"));
            n += 1;
        }
        path
    }
}

#[cfg(test)]
mod tests;
