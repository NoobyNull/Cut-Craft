//! The Machine menu: machines (name, maker, model, controller, bed size) each with the
//! post-processors it saves toolpaths through, and the post library they come from: Cut
//! Craft's own posts and the user's custom ones, kept in a folder of their own.

pub mod catalogue;
pub mod library;
pub mod machine;
pub mod manager;
pub mod send;
pub mod webdav;

pub use catalogue::{Controller, catalogue, makers};
pub use library::{Entry, Library};
pub use machine::{Machine, Machines};
pub use manager::{
    FolderFor, Look, MachineManager, MachineOutcome, PostManager, PostOutcome, tool_changer,
};
