//! Other file formats as pure converters: exporters turn the document into bytes, importers
//! turn bytes into an undoable change. The app does the actual file I/O.

use crate::document::Document;
use crate::{Change, CommandError};

#[derive(Clone, Copy, Debug)]
pub struct FileFormat {
    pub label: &'static str,
    /// Without the dot; the first is the default when saving.
    pub extensions: &'static [&'static str],
}

impl FileFormat {
    pub fn matches(&self, extension: &str) -> bool {
        self.extensions
            .iter()
            .any(|e| e.eq_ignore_ascii_case(extension))
    }
}

pub trait Exporter: 'static {
    const ID: &'static str;
    const FORMAT: FileFormat;
    fn export(doc: &Document) -> Result<Vec<u8>, CommandError>;
}

pub trait Importer: 'static {
    const ID: &'static str;
    const FORMAT: FileFormat;
    fn import(doc: &Document, bytes: &[u8]) -> Result<Change, CommandError>;
}

pub struct ExportEntry {
    pub id: &'static str,
    pub format: FileFormat,
    pub export: fn(&Document) -> Result<Vec<u8>, CommandError>,
}

pub struct ImportEntry {
    pub id: &'static str,
    pub format: FileFormat,
    pub import: fn(&Document, &[u8]) -> Result<Change, CommandError>,
}
