//! Send to Machine: where a machine's files are, so saved toolpaths go straight to it, and
//! what is there (list, upload, rename, delete, new folder). The place is a folder this
//! computer already sees: a network share the machine serves (a wireless controller's SD card),
//! or its SD card or USB stick plugged in here. Each project keeps its files in a folder of
//! its own there, named from the project's name in a way the machine's card can hold.
//! Or the machine's web address, for a controller that serves its card over WebDAV
//! (`crate::webdav`).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

/// The longest name kept for a project's folder or a file, in characters: well inside what
/// FAT's long names (255) and the controllers' web pages show.
const LONGEST: usize = 64;

/// Names Windows (and so a FAT card read on Windows) never lets a file or folder have.
const RESERVED: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Where a machine's files are sent, part of its Machine Configuration.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SendTo {
    /// Where Save Toolpaths' file dialog opens for this machine's G-code programs (in the
    /// project's own folder there); empty for wherever a file was last saved.
    pub programs: String,
    /// The folder the machine's files are in, as this computer sees it; empty when the
    /// machine has none (toolpaths are only saved where the user picks).
    pub folder: String,
    /// Names cut to 8 letters (and spaces dropped), for controllers that read only short
    /// names off their card.
    pub short_names: bool,
    /// The folder each project's files went to, by the project (its file, or "" for one
    /// not saved yet), so a project keeps its folder and two projects never share one.
    pub projects: BTreeMap<String, String>,
}

impl SendTo {
    /// Whether the machine has a place to send to.
    pub fn is_set(&self) -> bool {
        !self.folder.trim().is_empty()
    }

    /// The folder `project` (by its file) keeps its files in, named from `name`: the one it
    /// had, else `name` made safe for the card, numbered (`Sign 2`) when another project has
    /// that folder already. FAT ignores letter case, so `sign` is taken by `Sign`.
    pub fn project_folder(&mut self, project: &str, name: &str) -> String {
        if let Some(folder) = self.projects.get(project) {
            return folder.clone();
        }
        let base = safe_name(name, self.short_names);
        let taken = |folder: &str| {
            self.projects
                .iter()
                .any(|(p, f)| p != project && f.eq_ignore_ascii_case(folder))
        };
        // each project takes one number at most, so one past their count is always free
        let folder = (1..=self.projects.len() + 1)
            .map(|n| numbered(&base, n, self.short_names))
            .find(|f| !taken(f))
            .expect("some number is free");
        self.projects.insert(project.to_owned(), folder.clone());
        folder
    }

    /// Where the file dialog saving `project`'s G-code opens: its folder in the machine's
    /// G-code programs folder, made if need be; `None` without one (or when it can't be
    /// made, as when the drive is gone).
    pub fn programs_folder(&mut self, project: &str, name: &str) -> Option<PathBuf> {
        let root = PathBuf::from(self.programs.trim());
        if root.as_os_str().is_empty() || !root.is_dir() {
            return None;
        }
        let dir = root.join(self.project_folder(project, name));
        std::fs::create_dir_all(&dir).ok()?;
        Some(dir)
    }

    /// The place to send to, while there is one: a web address (WebDAV), else a folder.
    pub fn store(&self) -> Option<Box<dyn Store>> {
        let place = self.folder.trim();
        if !self.is_set() {
            None
        } else if crate::webdav::is_address(place) {
            Some(Box::new(crate::webdav::WebDav::new(place)))
        } else {
            Some(Box::new(Folder::new(place)))
        }
    }
}

/// `base`, or `base` with ` n` on its end (cut to fit 8 letters for short names).
fn numbered(base: &str, n: usize, short: bool) -> String {
    if n == 1 {
        return base.to_owned();
    }
    let tail = if short {
        format!("{n}")
    } else {
        format!(" {n}")
    };
    let longest = if short { 8 } else { LONGEST };
    let keep = longest.saturating_sub(tail.chars().count());
    format!("{}{tail}", base.chars().take(keep).collect::<String>())
}

/// `name` as a folder or file name any card and computer can hold: without the characters
/// FAT and Windows refuse (`<>:"/\|?*` and control characters), no dots or spaces at the
/// end, not a name Windows keeps for itself (`CON`), and at most 64 characters; with
/// `short`, 8 letters and digits (and `-`, `_`). Never empty: "Untitled" stands in.
pub fn safe_name(name: &str, short: bool) -> String {
    let kept: String = name
        .chars()
        .map(|c| {
            if c.is_control() || "<>:\"/\\|?*".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    let mut safe = if short {
        kept.chars()
            .filter(|c| c.is_ascii_alphanumeric() || "-_".contains(*c))
            .take(8)
            .collect::<String>()
    } else {
        let collapsed = kept.split_whitespace().collect::<Vec<_>>().join(" ");
        collapsed.chars().take(LONGEST).collect()
    };
    safe = safe.trim_end_matches(['.', ' ']).trim_start().to_owned();
    if safe.trim_matches('_').is_empty() {
        if short { "UNTITLED" } else { "Untitled" }.clone_into(&mut safe);
    }
    let stem = safe.split('.').next().unwrap_or_default();
    if RESERVED.iter().any(|r| r.eq_ignore_ascii_case(stem)) {
        safe.insert(0, '_');
    }
    safe
}

/// A file's name made safe as [`safe_name`] makes it, its extension (`.nc`) kept as it is.
pub fn safe_file_name(name: &str, short: bool) -> String {
    match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() && !ext.is_empty() => {
            format!("{}.{}", safe_name(stem, short), safe_name(ext, false))
        }
        _ => safe_name(name, short),
    }
}

/// One thing in a folder of the machine's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub name: String,
    pub folder: bool,
    /// Its size in bytes (0 for a folder).
    pub size: u64,
}

/// What a place to send to does: the folder parts in `at` are inside it, from its top.
pub trait Store {
    /// Where it is, as the user gave it (a folder, or a web address).
    fn place(&self) -> String;
    /// Why nothing may be changed there now (the machine is running a job), when it can tell.
    fn busy(&self) -> Option<String> {
        None
    }
    /// What is in folder `at`, folders first, each lot by name.
    fn list(&self, at: &[String]) -> Result<Vec<Item>, String>;
    /// Writes `bytes` as `name` in folder `at`, making the folder when it is not there.
    fn upload(&self, at: &[String], name: &str, bytes: &[u8]) -> Result<(), String>;
    /// Whether `name` is in folder `at` already.
    fn exists(&self, at: &[String], name: &str) -> Result<bool, String>;
    fn rename(&self, at: &[String], from: &str, to: &str) -> Result<(), String>;
    /// Deletes file or folder `name` (with all in it) from folder `at`.
    fn delete(&self, at: &[String], name: &str) -> Result<(), String>;
    fn make_folder(&self, at: &[String], name: &str) -> Result<(), String>;
}

/// A folder this computer sees (a share, card or stick) as the machine's place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Folder {
    root: PathBuf,
}

impl Folder {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Folder `at` on this computer, once the machine's folder is there; a part that would
    /// step out of it (`..`, a path) is refused.
    fn dir(&self, at: &[String]) -> Result<PathBuf, String> {
        if !self.root.is_dir() {
            return Err(unreachable(&self.root));
        }
        at.iter().try_fold(self.root.clone(), |dir, part| {
            plain(part)?;
            Ok(dir.join(part))
        })
    }

    fn file(&self, at: &[String], name: &str) -> Result<PathBuf, String> {
        plain(name)?;
        Ok(self.dir(at)?.join(name))
    }
}

/// A name of one thing in a folder, nothing that reaches elsewhere.
fn plain(name: &str) -> Result<(), String> {
    let bad = name.is_empty()
        || name == "."
        || name == ".."
        || name.contains(['/', '\\'])
        || Path::new(name).is_absolute();
    if bad {
        Err(format!(
            "\"{name}\" is not a name a file or folder can have"
        ))
    } else {
        Ok(())
    }
}

/// What to say when the machine's folder is not there.
fn unreachable(root: &Path) -> String {
    format!(
        "Can't reach the machine's files at {}. Check the machine is switched on and on the \
         network, or that its card or stick is plugged in, then try again",
        root.display()
    )
}

/// `error` doing `what` (to `path`), in words that say what to do about it.
pub fn explain(what: &str, path: &Path, error: &std::io::Error) -> String {
    let at = path.display();
    match error.kind() {
        ErrorKind::StorageFull => format!(
            "{what}: the machine's card is full. Delete old jobs in Machine ▸ Machine Files, \
             then try again"
        ),
        ErrorKind::PermissionDenied | ErrorKind::ReadOnlyFilesystem => format!(
            "{what}: {at} can't be changed. Check the card's lock switch is off and the share \
             lets you write to it"
        ),
        ErrorKind::NotFound => format!("{what}: {at} is not there any more. Refresh the list"),
        ErrorKind::AlreadyExists => {
            format!("{what}: there is already something called that. Pick another name")
        }
        ErrorKind::DirectoryNotEmpty => {
            format!("{what}: {at} still has files in it")
        }
        _ => format!("{what}: {at}: {error}"),
    }
}

impl Store for Folder {
    fn place(&self) -> String {
        self.root.display().to_string()
    }

    fn list(&self, at: &[String]) -> Result<Vec<Item>, String> {
        let dir = self.dir(at)?;
        let entries = std::fs::read_dir(&dir).map_err(|e| explain("Not listed", &dir, &e))?;
        let mut items: Vec<Item> = entries
            .filter_map(Result::ok)
            .filter_map(|e| {
                let name = e.file_name().to_str()?.to_owned();
                // the controller's and the computer's own (.Trash, .DS_Store, System Volume
                // Information) are no business of the user's
                let hidden = name.starts_with('.') || name == "System Volume Information";
                let meta = e.metadata().ok()?;
                (!hidden).then(|| Item {
                    name,
                    folder: meta.is_dir(),
                    size: if meta.is_dir() { 0 } else { meta.len() },
                })
            })
            .collect();
        items.sort_by(|a, b| {
            (b.folder, a.name.to_lowercase()).cmp(&(a.folder, b.name.to_lowercase()))
        });
        Ok(items)
    }

    fn upload(&self, at: &[String], name: &str, bytes: &[u8]) -> Result<(), String> {
        let path = self.file(at, name)?;
        cut_craft_core::file::write_atomic(&path, bytes)
            .map_err(|e| explain(&format!("{name} not sent"), &path, &e))
    }

    fn exists(&self, at: &[String], name: &str) -> Result<bool, String> {
        Ok(self.file(at, name)?.exists())
    }

    fn rename(&self, at: &[String], from: &str, to: &str) -> Result<(), String> {
        let (old, new) = (self.file(at, from)?, self.file(at, to)?);
        // a new name differing only in letter case is the same file on a FAT card
        if new.exists() && !from.eq_ignore_ascii_case(to) {
            return Err(format!(
                "Not renamed: there is already something called {to}. Pick another name"
            ));
        }
        std::fs::rename(&old, &new).map_err(|e| explain("Not renamed", &old, &e))
    }

    fn delete(&self, at: &[String], name: &str) -> Result<(), String> {
        let path = self.file(at, name)?;
        let gone = if path.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
        gone.map_err(|e| explain("Not deleted", &path, &e))
    }

    fn make_folder(&self, at: &[String], name: &str) -> Result<(), String> {
        let path = self.file(at, name)?;
        std::fs::create_dir(&path).map_err(|e| explain("Folder not made", &path, &e))
    }
}

/// A size for people: `512 bytes`, `12.3 KB`, `4.5 MB`.
pub fn size_text(bytes: u64) -> String {
    #[allow(clippy::cast_precision_loss)]
    let b = bytes as f64;
    if bytes < 1024 {
        format!("{bytes} bytes")
    } else if b < 1024.0 * 1024.0 {
        format!("{:.1} KB", b / 1024.0)
    } else {
        format!("{:.1} MB", b / (1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cut-craft-send-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn names_are_made_safe_for_a_cards_file_system() {
        assert_eq!(
            safe_name("Oak sign: \"Smith\" 2/3", false),
            "Oak sign_ _Smith_ 2_3"
        );
        assert_eq!(safe_name("  Welcome   home.  ", false), "Welcome home");
        assert_eq!(safe_name("con", false), "_con");
        assert_eq!(safe_name("???", false), "Untitled");
        assert_eq!(safe_name("", false), "Untitled");
        assert_eq!(safe_name(&"x".repeat(100), false).len(), LONGEST);
        assert_eq!(safe_name("Oak sign, front", true), "Oaksignf");
        assert_eq!(safe_name("Ünïcödé", true), "ncd");
        assert_eq!(safe_name("!!", true), "UNTITLED");
        assert_eq!(
            safe_file_name("Pocket 1: deep.nc", false),
            "Pocket 1_ deep.nc"
        );
        assert_eq!(
            safe_file_name("Profile outside.gcode", true),
            "Profileo.gcode"
        );
    }

    #[test]
    fn each_project_keeps_its_own_folder_and_a_clash_is_numbered() {
        let mut to = SendTo::default();
        assert_eq!(to.project_folder("/jobs/sign.ccp", "Sign"), "Sign");
        assert_eq!(to.project_folder("/jobs/sign.ccp", "Sign"), "Sign", "kept");
        // another project whose name makes the same folder, whatever its letter case
        assert_eq!(to.project_folder("/old/SIGN.ccp", "SIGN"), "SIGN 2");
        assert_eq!(to.project_folder("/x/sign?.ccp", "Sign?"), "Sign_");
        let mut short = SendTo {
            short_names: true,
            ..SendTo::default()
        };
        assert_eq!(short.project_folder("a", "Kitchen sign"), "Kitchens");
        assert_eq!(short.project_folder("b", "Kitchen signs"), "Kitchen2");
    }

    #[test]
    fn a_machine_with_no_folder_has_nowhere_to_send() {
        assert!(SendTo::default().store().is_none());
        let to = SendTo {
            folder: " /media/sd ".into(),
            ..SendTo::default()
        };
        assert_eq!(to.store().unwrap().place(), "/media/sd");
        let fluidnc = SendTo {
            folder: "http://fluidnc.local/sd".into(),
            ..SendTo::default()
        };
        assert_eq!(fluidnc.store().unwrap().place(), "http://fluidnc.local/sd/");
    }

    #[test]
    fn the_programs_folder_opens_in_the_projects_own_folder() {
        let root = temp("programs");
        let mut to = SendTo::default();
        assert_eq!(to.programs_folder("/a/Sign.ccp", "Sign"), None, "none set");
        to.programs = root.display().to_string();
        let dir = to.programs_folder("/a/Sign.ccp", "Sign").unwrap();
        assert_eq!(dir, root.join("Sign"));
        assert!(dir.is_dir(), "made");
        to.programs = root.join("gone").display().to_string();
        assert_eq!(
            to.programs_folder("/a/Sign.ccp", "Sign"),
            None,
            "drive gone"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn files_are_uploaded_listed_renamed_and_deleted_in_the_machines_folder() {
        let root = temp("manage");
        let store = Folder::new(&root);
        let sign = vec!["Sign".to_owned()];
        store.upload(&sign, "b.nc", b"G0 X0").unwrap();
        store.upload(&sign, "A.nc", b"G0").unwrap();
        store.make_folder(&sign, "old").unwrap();
        assert!(store.exists(&sign, "b.nc").unwrap());
        let names = |s: &Folder| -> Vec<String> {
            s.list(&sign).unwrap().into_iter().map(|i| i.name).collect()
        };
        assert_eq!(
            names(&store),
            ["old", "A.nc", "b.nc"],
            "folders first, by name"
        );
        assert_eq!(store.list(&sign).unwrap()[2].size, 5);
        let err = store.rename(&sign, "A.nc", "b.nc").unwrap_err();
        assert!(err.contains("already something called b.nc"), "{err}");
        store.rename(&sign, "A.nc", "c.nc").unwrap();
        store
            .upload(&[sign[0].clone(), "old".into()], "x.nc", b"")
            .unwrap();
        store.delete(&sign, "old").unwrap();
        store.delete(&sign, "b.nc").unwrap();
        assert_eq!(names(&store), ["c.nc"]);
        assert_eq!(store.list(&[]).unwrap()[0].name, "Sign");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn nothing_reaches_outside_the_machines_folder() {
        let root = temp("outside");
        let store = Folder::new(&root);
        for bad in ["..", "../x.nc", "a/b", "", "."] {
            assert!(store.upload(&[], bad, b"").is_err(), "{bad}");
            assert!(store.list(&[bad.to_owned()]).is_err(), "{bad}");
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_machine_that_cant_be_reached_says_what_to_check() {
        let store = Folder::new("/no/such/machine/card");
        let err = store.list(&[]).unwrap_err();
        assert!(err.starts_with("Can't reach the machine's files"), "{err}");
        assert!(err.contains("switched on"), "{err}");
        let full = std::io::Error::from(ErrorKind::StorageFull);
        assert!(explain("x not sent", Path::new("/m"), &full).contains("card is full"));
    }

    #[test]
    fn sizes_read_as_people_say_them() {
        assert_eq!(size_text(512), "512 bytes");
        assert_eq!(size_text(12_595), "12.3 KB");
        assert_eq!(size_text(4_718_592), "4.5 MB");
    }
}
