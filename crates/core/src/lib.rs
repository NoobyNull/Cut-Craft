//! Document model, invertible changes, and the [`Command`] contract every command implements.

mod change;
mod command;
pub mod document;
pub mod geometry;
#[cfg(feature = "testing")]
pub mod testing;

pub use change::{ApplyError, Change, ChangeBuilder, Edit, History, Step};
pub use command::{Command, CommandError, DynCommand, Meta, NoParams, Registry};
