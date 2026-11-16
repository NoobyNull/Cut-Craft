//! The app's own status-bar errors about files the person picked or saved: each a sentence
//! in plain words that says what to do, not the raw OS or parser text (that goes to the log).

use std::fmt::Display;
use std::io::{Error, ErrorKind};
use std::path::Path;

/// Why `e` stopped a file being read or written, in plain words.
fn reason(e: &Error) -> &'static str {
    match e.kind() {
        ErrorKind::PermissionDenied | ErrorKind::ReadOnlyFilesystem => {
            "Cut Craft is not allowed to use that folder or file"
        }
        ErrorKind::NotFound => "that file or its folder is not there",
        ErrorKind::StorageFull | ErrorKind::QuotaExceeded => "the disk is full",
        ErrorKind::ResourceBusy | ErrorKind::ExecutableFileBusy => {
            "another program is using that file"
        }
        _ => "the computer would not let it",
    }
}

/// Saving `path` failed with `e`.
pub fn could_not_save(path: &Path, e: &Error) -> String {
    log::warn!("could not save {}: {e}", path.display());
    format!(
        "Could not save {}: {}. Save it in a folder you can write to (Documents, say), or \
         close any program that has it open, then try again.",
        path.display(),
        reason(e)
    )
}

/// Saving the picture `path` failed with `e`.
pub fn could_not_save_picture(path: &Path, e: &impl Display) -> String {
    log::warn!("could not save {}: {e}", path.display());
    format!(
        "Could not save the picture {}. Save it as .png or .jpg, in a folder you can write \
         to.",
        path.display()
    )
}

/// Reading the picked file `path` failed with `e`.
pub fn could_not_read(path: &Path, e: &Error) -> String {
    log::warn!("could not read {}: {e}", path.display());
    format!(
        "Could not open {}: {}. Check the file is still there, or pick another.",
        path.display(),
        reason(e)
    )
}

/// The picked file `path` is not a machine package (the parser's own words are logged).
pub fn not_a_machine_package(path: &Path) -> String {
    format!(
        "The file {} is not a machine package Cut Craft can read. Pick a machine package: a .json file \
         from the machines folder, or one shared from another Cut Craft.",
        path.display()
    )
}

/// The spelling checker's own words could not be kept in `path`.
pub fn cannot_keep_words(path: &Path, e: &Error) -> String {
    log::warn!("cannot keep the words in {}: {e}", path.display());
    format!(
        "The word is checked as spelt right until Cut Craft closes, but could not be kept in \
         {}: {}. Check that folder can be written to.",
        path.display(),
        reason(e)
    )
}
