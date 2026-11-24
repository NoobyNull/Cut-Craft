//! Lowers the priority of the thread that calls it, so the computer's other programs (a video,
//! the browser) and Cut Craft's own window come before a long calculation, and reads how much
//! processor time Cut Craft has used.
//!
//! A lowered thread still runs flat out when nothing else wants the processor: it only gives
//! way while another program is waiting. Only ever lowered, never raised, so no system asks for
//! an administrator. Each system does it its own way:
//!
//! - Linux: the thread's nice value goes up to [`NICE`] (Linux keeps one per thread, so the
//!   window's thread keeps its own). A thread already nicer is left as it is.
//! - Windows: the thread's priority goes to "below normal". Not the "background mode", which
//!   also slows the thread's disk and memory work and can make a long job crawl.
//! - macOS: the thread's quality of service goes to "utility". On Apple silicon the system may
//!   run it on the slower efficiency cores, so a calculation can take a little longer there.
//! - Anything else: nothing changes.

use std::time::Duration;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as os;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as os;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as os;
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
mod other;
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
use other as os;

#[cfg(unix)]
mod rusage;

/// The nice value a calculation thread is given on Linux: clearly below the desktop's own
/// programs (0), so a video or a browser comes first, without starving behind every background
/// task as the lowest (19) would.
pub const NICE: i32 = 10;

/// Lowers the calling thread's priority for the rest of its life; whether it is lowered now.
pub fn lower_this_thread() -> bool {
    os::lower_this_thread()
}

/// Whether the calling thread runs at the lowered priority (or lower).
pub fn this_thread_is_lowered() -> bool {
    os::this_thread_is_lowered()
}

/// The processor time all of Cut Craft's threads have used since it started, both in its own
/// code and in the system's on its behalf; none where the system will not say.
pub fn process_time() -> Option<Duration> {
    os::process_time()
}
