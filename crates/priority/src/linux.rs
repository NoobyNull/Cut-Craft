//! Linux: a nice value per thread.

use super::NICE;

/// The calling thread's own id, which Linux's priority calls take for one thread.
#[allow(unsafe_code)]
fn this_thread() -> libc::id_t {
    // SAFETY: gettid takes nothing and only reports the calling thread's id
    let tid = unsafe { libc::gettid() };
    libc::id_t::try_from(tid).unwrap_or(0)
}

/// The calling thread's nice value.
#[allow(unsafe_code)]
fn nice() -> Option<i32> {
    // getpriority can answer -1 and mean it, so errno is cleared first to tell a failure apart
    // SAFETY: errno is this thread's own; getpriority only reads this thread's nice value
    unsafe {
        *libc::__errno_location() = 0;
        let nice = libc::getpriority(libc::PRIO_PROCESS, this_thread());
        (*libc::__errno_location() == 0).then_some(nice)
    }
}

#[allow(unsafe_code)]
pub fn lower_this_thread() -> bool {
    if nice().is_some_and(|n| n >= NICE) {
        return true;
    }
    // SAFETY: setpriority changes only this thread's nice value, and only ever down, which
    // needs no special rights
    unsafe { libc::setpriority(libc::PRIO_PROCESS, this_thread(), NICE) == 0 }
}

pub fn this_thread_is_lowered() -> bool {
    nice().is_some_and(|n| n >= NICE)
}

pub use super::rusage::process_time;
