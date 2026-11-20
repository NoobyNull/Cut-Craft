//! Rotary work in true 3D: where the material and the model really are round the turning
//! axis, where the bit can go without touching the model at any turn of A, and what a file's
//! moves really cut.
//!
//! Everything here is measured in the part's own frame, which turns with A: along the axis
//! (from the job's start, the chuck end), and `y`, `z` across it, with `z` straight up and
//! `y` towards the machine's positive across side when A is 0. The machine's frame stays
//! still: along, across (Y beside a rotary along X) and height above the axis. [`Pose`]
//! says where the bit is in the machine's frame, and turns points between the two.
//!
//! - [`Stock`]: the material's shape (round, square, a flat board) and where it sits on the
//!   axis, and how far out it swings.
//! - [`Mesh`]: the model as triangles, and [`Mesh::drop`]: the highest the bit's tip can sit,
//!   held anywhere, with A turned anywhere, without touching it.
//! - [`Voxels`]: the material as a fine grid of solid cells, cut by the bit moving exactly
//!   as a file moves it (A, along, across and height all together), and its surface for
//!   drawing.
//! - [`Head`] and [`Clearance`]: the holder above the bit, and the chuck and tailstock, kept
//!   clear of what is left of the material.
//! - [`gcode::read`]: a rotary file read back as the bit's poses, for the simulator to cut
//!   exactly what the machine will.
//! - [`Replay`]: a file played back into the material over the machine's own time, to be
//!   shown turning as it is cut, and moved back and forth.

mod clearance;
mod cutter;
mod frame;
pub mod gcode;
mod mesh;
mod replay;
mod stock;
mod voxels;

pub use clearance::{Clearance, Head};
pub use cutter::Cutter;
pub use frame::{Pose, angle, to_machine, to_part};
pub use mesh::Mesh;
pub use replay::{Replay, Speeds};
pub use stock::{Section, Stock};
pub use voxels::Voxels;
