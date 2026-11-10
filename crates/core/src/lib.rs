//! Document model, invertible changes, and the [`Command`] contract every command implements.

pub mod bitmap;
pub mod calc;
mod change;
pub mod checks;
mod command;
pub mod component;
pub mod compute;
pub mod constraint;
pub mod csv;
pub mod document;
pub mod drill_bank;
pub mod entry;
pub mod file;
pub mod font;
mod formats;
pub mod geometry;
pub mod handles;
pub mod keep_out;
pub mod nodes;
pub mod packed;
pub mod params;
pub mod playback;
pub mod polygon;
pub mod preflight;
pub mod progress;
pub mod random;
pub mod rest;
pub mod selection;
pub mod selector;
pub mod simplify;
pub mod simulation;
pub mod snap;
pub mod spell;
#[cfg(feature = "testing")]
pub mod testing;
pub mod texture_area;
pub mod tiling;
pub mod tool_names;
pub mod toolpath;
pub mod trace;
pub mod vfont;

pub use glam;

pub use change::{ApplyError, Change, ChangeBuilder, Edit, History, Sequence, Step};
pub use command::{
    Command, CommandError, Drag, DynCommand, EMPTY_MODEL, Interactive, Meta, NoParams, Registry,
    SELECT_FIRST, ensure_named, ensure_positive, existing, existing_layer, gone, sentence,
    tool_does_not_fit,
};
pub use formats::{ExportEntry, Exporter, FileFormat, ImportEntry, Importer};
