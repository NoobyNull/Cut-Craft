//! The engine the rotary carving toolpaths share: the model as radii round the turning axis
//! and as full 3D solids, passes worked out over it with the bit as low as it goes, what is
//! left of the material as the passes cut, and the holder, chuck and tailstock kept clear.
//! Each rotary toolpath is a short command over it, so they all cut the same way.

mod carving;
mod finish;
mod left;
mod locked;
mod rough;
mod shape;
mod solid;
mod turned;
mod work;

pub use carving::*;
pub use finish::*;
pub use left::*;
pub use locked::*;
pub use rough::*;
pub use shape::*;
pub use solid::*;
pub use turned::*;
pub use work::*;
