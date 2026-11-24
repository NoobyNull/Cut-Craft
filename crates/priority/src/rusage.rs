//! The processor time this process has used, as Linux, macOS and the BSDs report it.

use std::time::Duration;

/// The process's user and system time.
#[allow(unsafe_code)]
pub fn process_time() -> Option<Duration> {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::zeroed();
    // SAFETY: getrusage fills the rusage it is handed and nothing else; it is zeroed first
    let ok = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) } == 0;
    if !ok {
        return None;
    }
    // SAFETY: zeroed is a valid rusage, and getrusage succeeded in filling it
    let usage = unsafe { usage.assume_init() };
    Some(time(usage.ru_utime) + time(usage.ru_stime))
}

fn time(t: libc::timeval) -> Duration {
    let secs = u64::try_from(t.tv_sec).unwrap_or(0);
    let micros = u32::try_from(t.tv_usec).unwrap_or(0);
    Duration::from_secs(secs) + Duration::from_micros(u64::from(micros))
}
