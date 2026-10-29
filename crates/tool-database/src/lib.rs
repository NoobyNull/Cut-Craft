//! The Tool Database manager, as Aspire's: a tree of tool groups on the left, the picked
//! group's or tool's settings on the right, and changes kept only on Apply or OK.

pub mod fusion;
pub mod manager;
pub mod tree;

pub use manager::{Changer, Look, Manager, Outcome, Pick};
