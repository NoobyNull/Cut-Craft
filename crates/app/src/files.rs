//! Job files, imports and exports: the app's only file I/O. Every operation takes a path, so
//! tests run them without dialogs.

use aspire_core::document::Document;
use aspire_core::file::{load_job, save_job};
use aspire_core::{Change, History, Registry};
use aspire_post::Post;
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
pub fn save_through(post: &Post, doc: &Document, path: &Path) -> Result<(), String> {
    let text = post.render(doc)?;
    std::fs::write(path, text).map_err(|e| format!("could not write {}: {e}", path.display()))
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("could not read {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use aspire_core::testing::sample_document;
    use std::fs;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("aspire-files-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    fn registry() -> Registry {
        crate::commands::registry()
    }

    #[test]
    fn saving_then_opening_restores_the_job_with_a_clean_history() {
        let path = temp("round-trip.ajob");
        let doc = sample_document();
        let mut session = Session::default();
        session.save(&doc, &History::default(), &path).unwrap();
        let (opened, history) = Session::default().open(&path).unwrap();
        assert_eq!(opened, doc);
        assert!(!history.can_undo());
    }

    #[test]
    fn changes_make_the_session_dirty_until_saved() {
        let path = temp("dirty.ajob");
        let mut doc = Document::default();
        let mut history = History::default();
        let mut session = Session::default();
        assert!(!session.is_dirty(&history));
        let change = Change::add_and_select(
            &doc,
            aspire_core::geometry::Path::circle(aspire_core::geometry::Point::ZERO, 1.0),
        );
        history.apply(&mut doc, change).unwrap();
        assert!(session.is_dirty(&history));
        assert_eq!(
            session.title("Aspire Clone", &history),
            "Untitled* - Aspire Clone"
        );
        session.save(&doc, &history, &path).unwrap();
        assert!(!session.is_dirty(&history));
        assert_eq!(
            session.title("Aspire Clone", &history),
            "dirty.ajob - Aspire Clone"
        );
    }

    #[test]
    fn opening_something_that_is_not_a_job_explains_why() {
        let path = temp("not-a-job.ajob");
        fs::write(&path, b"{}").unwrap();
        let err = Session::default().open(&path).unwrap_err();
        assert!(err.contains("not an Aspire Clone job"), "{err}");
        assert!(
            Session::default()
                .open(&temp("missing.ajob"))
                .unwrap_err()
                .contains("missing.ajob")
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
        save_through(&post, &doc, &out).unwrap();
        assert!(
            fs::read_to_string(&out)
                .unwrap()
                .lines()
                .any(|l| l.starts_with("G0 X"))
        );
    }
}
