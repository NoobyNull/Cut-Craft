//! Where a library the user keeps (the Bit Library, the Materials Library) is saved, and
//! the one rule both follow: a file that could not be read at start is never written
//! over, so a broken file is not lost under a fresh one, and every save says why.

use std::path::{Path, PathBuf};

#[derive(Default)]
pub(super) struct LibraryFile {
    /// Where it is saved; `None` keeps it in memory (tests, or a file that failed to load).
    path: Option<PathBuf>,
    /// The file that could not be read at start: nothing is saved over it.
    unreadable: Option<PathBuf>,
}

impl LibraryFile {
    /// Saves to `path` from now on (`None` keeps it in memory).
    pub(super) fn at(&mut self, path: Option<PathBuf>) {
        self.path = path;
    }

    /// Where it is saved, if anywhere.
    pub(super) fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Its file could not be read: it is kept on disk as it is, and not written over.
    pub(super) fn unreadable(&mut self) {
        self.unreadable = self.path.take();
    }

    /// Saves it with `write`, if it has a file. An error when `write` fails, or when its
    /// file could not be read at start and so is not written over (`what` names it).
    pub(super) fn save(
        &self,
        what: &str,
        write: impl FnOnce(&Path) -> Result<(), String>,
    ) -> Result<(), String> {
        if let Some(path) = &self.unreadable {
            return Err(not_saved(what, path));
        }
        self.path.as_deref().map_or(Ok(()), write)
    }
}

/// What a save says when `what`'s file, at `path`, could not be read when Cut Craft started:
/// the changes are kept until it closes, but not written over that file.
fn not_saved(what: &str, path: &Path) -> String {
    format!(
        "{what} not saved: {} could not be read when Cut Craft started, so it is not written \
         over. Fix or rename that file, then restart Cut Craft; your changes are kept only \
         until it closes",
        path.display()
    )
}

#[cfg(test)]
mod tests;
