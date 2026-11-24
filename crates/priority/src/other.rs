//! Systems with no per-thread priority Cut Craft knows how to set: nothing changes.

use std::time::Duration;

pub fn lower_this_thread() -> bool {
    false
}

pub fn this_thread_is_lowered() -> bool {
    false
}

#[cfg(unix)]
pub use super::rusage::process_time;

#[cfg(not(unix))]
pub fn process_time() -> Option<Duration> {
    None
}
