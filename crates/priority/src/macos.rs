//! macOS: a quality of service per thread.

/// Whether `class` is utility or below (background).
fn low(class: libc::qos_class_t) -> bool {
    matches!(
        class,
        libc::qos_class_t::QOS_CLASS_UTILITY | libc::qos_class_t::QOS_CLASS_BACKGROUND
    )
}

#[allow(unsafe_code)]
fn class() -> libc::qos_class_t {
    let mut class = libc::qos_class_t::QOS_CLASS_UNSPECIFIED;
    let mut relative = 0;
    // SAFETY: only reads the calling thread's class into the two values handed to it
    unsafe {
        libc::pthread_get_qos_class_np(libc::pthread_self(), &raw mut class, &raw mut relative);
    }
    class
}

#[allow(unsafe_code)]
pub fn lower_this_thread() -> bool {
    if low(class()) {
        return true;
    }
    // SAFETY: sets only the calling thread's quality of service, lowering it
    unsafe { libc::pthread_set_qos_class_self_np(libc::qos_class_t::QOS_CLASS_UTILITY, 0) == 0 }
}

pub fn this_thread_is_lowered() -> bool {
    low(class())
}

pub use super::rusage::process_time;
