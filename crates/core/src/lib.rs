//! Document model, invertible changes, and the [`Command`] contract every command implements.

mod change;
mod command;
pub mod document;
pub mod file;
mod formats;
pub mod geometry;
pub mod params;
pub mod polygon;
pub mod simulation;
#[cfg(feature = "testing")]
pub mod testing;
pub mod toolpath;

pub use glam;

pub use change::{ApplyError, Change, ChangeBuilder, Edit, History, Step};
pub use command::{
    Command, CommandError, Drag, DynCommand, Interactive, Meta, NoParams, Registry, ensure_named,
    ensure_positive, existing_layer,
};
pub use formats::{ExportEntry, Exporter, FileFormat, ImportEntry, Importer};
