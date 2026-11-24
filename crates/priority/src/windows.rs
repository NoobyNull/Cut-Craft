//! Windows: a priority per thread, and the process's times.

use std::time::Duration;
use windows_sys::Win32::Foundation::FILETIME;
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentThread, GetProcessTimes, GetThreadPriority, SetThreadPriority,
    THREAD_PRIORITY_BELOW_NORMAL,
};

#[allow(unsafe_code)]
fn priority() -> i32 {
    // SAFETY: GetCurrentThread is a stand-in handle that needs no closing; only read here
    unsafe { GetThreadPriority(GetCurrentThread()) }
}

#[allow(unsafe_code)]
pub fn lower_this_thread() -> bool {
    if this_thread_is_lowered() {
        return true;
    }
    // SAFETY: changes only the calling thread's priority, lowering it
    unsafe { SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL) != 0 }
}

pub fn this_thread_is_lowered() -> bool {
    // THREAD_PRIORITY_ERROR_RETURN is i32::MAX, so a failure reads as not lowered
    priority() <= THREAD_PRIORITY_BELOW_NORMAL
}

#[allow(unsafe_code)]
pub fn process_time() -> Option<Duration> {
    let zero = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let (mut created, mut ended, mut kernel, mut user) = (zero, zero, zero, zero);
    // SAFETY: GetCurrentProcess is a stand-in handle that needs no closing; the call only
    // fills the four times it is handed
    let ok = unsafe {
        GetProcessTimes(
            GetCurrentProcess(),
            &raw mut created,
            &raw mut ended,
            &raw mut kernel,
            &raw mut user,
        )
    } != 0;
    ok.then(|| ticks(kernel) + ticks(user))
}

/// A FILETIME span (100 ns ticks) as a duration.
fn ticks(t: FILETIME) -> Duration {
    let ticks = (u64::from(t.dwHighDateTime) << 32) | u64::from(t.dwLowDateTime);
    Duration::from_nanos(ticks.saturating_mul(100))
}
