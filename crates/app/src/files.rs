//! Job files, imports and exports: the app's only file I/O. Every operation takes a path, so
//! tests run them without dialogs.

use cut_craft_core::document::Document;
use cut_craft_core::file::{load_job, save_job};
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
}

impl Session {
    pub fn is_dirty(&self, history: &History) -> bool {
        history.revision() != self.saved
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

    pub fn save(&mut self, doc: &Document, history: &History, path: &Path) -> Result<(), String> {
        std::fs::write(path, save_job(doc))
            .map_err(|e| format!("could not save {}: {e}", path.display()))?;
        self.path = Some(path.to_owned());
        self.saved = history.revision();
        Ok(())
    }

    /// The opened job with a fresh history; the session now points at it.
    pub fn open(&mut self, path: &Path) -> Result<(Document, History), String> {
        let bytes = read(path)?;
        let doc = load_job(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        let history = History::default();
        self.path = Some(path.to_owned());
        self.saved = history.revision();
        Ok((doc, history))
    }
}

/// An import as an undoable change, chosen by the file's extension.
pub fn import(registry: &Registry, doc: &Document, path: &Path) -> Result<Change, String> {
    let extension = path
        .extension()
        .map(|e| e.to_string_lossy().into_owned())
        .unwrap_or_default();
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
    std::fs::write(path, bytes).map_err(|e| format!("could not write {}: {e}", path.display()))
}

/// A `.pp` post-processor, checked as it is read.
pub fn load_post(path: &Path) -> Result<Post, String> {
    let text = String::from_utf8(read(path)?)
        .map_err(|_| format!("{} is not a text file", path.display()))?;
    Post::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Writes the toolpaths as the post's output.
/// Writes the toolpaths through `post` to `path`; `job` is the saved job they come from.
pub fn save_through(
    post: &Post,
    doc: &Document,
    path: &Path,
    job: Option<&Path>,
) -> Result<(), String> {
    let made = chrono::Local::now().naive_local();
    let text = post.render(doc, &cut_craft_post::File { path, job, made })?;
    std::fs::write(path, text).map_err(|e| format!("could not write {}: {e}", path.display()))
}

/// Where Cut Craft keeps its settings: `$XDG_CONFIG_HOME/cut-craft`, else `~/.config/cut-craft`.
// ponytail: XDG only; add the Windows/macOS config folders when those builds exist
pub fn config_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .map(|dir| dir.join("cut-craft"))
}

/// The next file name for Increment and Save: a name ending `_N` or `-N` counts up, any
/// other gets `_1`, and it keeps counting past names that `exists` says are taken.
pub fn incremented(path: &Path, exists: impl Fn(&Path) -> bool) -> PathBuf {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled");
    let (base, mut n) = match stem.rfind(['_', '-']) {
        Some(at) if stem[at + 1..].parse::<u64>().is_ok() => {
            (&stem[..=at], stem[at + 1..].parse::<u64>().unwrap_or(0))
        }
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

pub fn tool_database_path() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("tools.json"))
}

pub fn recent_files_path() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("recent.json"))
}

/// The tool database at `path`; the starter tools when there is none yet.
pub fn load_tools(path: &Path) -> Result<ToolDatabase, String> {
    if !path.exists() {
        return Ok(ToolDatabase::starter());
    }
    ToolDatabase::from_json(&read(path)?).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn save_tools(db: &ToolDatabase, path: &Path) -> Result<(), String> {
    write_creating(path, &db.to_json())
}

/// How many jobs the start page lists.
pub const RECENT_COUNT: usize = 4;

/// Puts `path` first in `recent`, once, keeping the latest [`RECENT_COUNT`].
pub fn remember(recent: &mut Vec<PathBuf>, path: &Path) {
    recent.retain(|p| p != path);
    recent.insert(0, path.to_path_buf());
    recent.truncate(RECENT_COUNT);
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
fn write_creating(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let written = match path.parent() {
        Some(dir) => std::fs::create_dir_all(dir).and_then(|()| std::fs::write(path, bytes)),
        None => std::fs::write(path, bytes),
    };
    written.map_err(|e| format!("could not write {}: {e}", path.display()))
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("could not read {}: {e}", path.display()))
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
}

#[cfg(test)]
mod tests {
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
            remember(&mut recent, Path::new(name));
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
        let config = recent_files_path().expect("HOME is set");
        assert!(
            config.ends_with("cut-craft/recent.json"),
            "{}",
            config.display()
        );
    }

    #[test]
    fn the_tool_database_lives_in_the_cut_craft_config_folder() {
        let path = tool_database_path().expect("HOME is set");
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
    fn a_broken_tool_database_names_the_file() {
        let path = temp("broken-tools.json");
        fs::write(&path, "{").unwrap();
        let err = load_tools(&path).unwrap_err();
        assert!(err.contains("broken-tools.json"), "{err}");
    }

    #[test]
    fn installing_a_post_reads_it() {
        let path = temp("good.pp");
        fs::write(&path, POST).unwrap();
        assert_eq!(load_post(&path).unwrap().name, "Test post");
    }

    #[test]
    fn a_broken_post_names_the_file_and_the_problem() {
        let path = temp("broken.pp");
        fs::write(&path, POST.replace("begin FEED_MOVE", "begin NOPE")).unwrap();
        let err = load_post(&path).unwrap_err();
        assert!(
            err.contains("broken.pp") && err.contains("no FEED_MOVE block"),
            "{err}"
        );
    }

    #[test]
    fn toolpaths_save_through_the_post() {
        let post_path = temp("save.pp");
        fs::write(&post_path, POST).unwrap();
        let post = load_post(&post_path).unwrap();
        let mut doc = sample_document();
        let registry = crate::commands::registry();
        let made = registry.get("drilling_toolpath").unwrap();
        let params = made.default_params();
        let change = made.run_json(&doc, &params).unwrap();
        doc.apply(&change).unwrap();
        let out = temp("out.tap");
        save_through(&post, &doc, &out, None).unwrap();
        assert!(
            fs::read_to_string(&out)
                .unwrap()
                .lines()
                .any(|l| l.starts_with("G0 X"))
        );
    }
}
