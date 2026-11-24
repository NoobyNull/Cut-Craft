//! The thread's priority as the system sees it, and the process's processor time.

use std::time::Duration;

#[test]
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
fn a_lowered_thread_stays_lowered_and_others_are_not() {
    // the tests themselves may run lowered (`nice cargo test`): a fresh thread starts as they do
    let fresh = || {
        std::thread::spawn(cut_craft_priority::this_thread_is_lowered)
            .join()
            .unwrap()
    };
    let started = fresh();
    let lowered = std::thread::spawn(|| {
        let did = cut_craft_priority::lower_this_thread();
        (did, cut_craft_priority::this_thread_is_lowered())
    })
    .join()
    .unwrap();
    assert_eq!(lowered, (true, true), "the call, and after it");
    assert_eq!(
        fresh(),
        started,
        "a thread nobody lowered keeps the priority it started with"
    );
}

#[test]
fn lowering_twice_is_harmless() {
    std::thread::spawn(|| {
        let first = cut_craft_priority::lower_this_thread();
        assert_eq!(cut_craft_priority::lower_this_thread(), first);
    })
    .join()
    .unwrap();
}

#[test]
#[cfg(any(unix, windows))]
fn processor_time_grows_while_working() {
    let before = cut_craft_priority::process_time().expect("the system says");
    let start = std::time::Instant::now();
    let mut x = 0u64;
    while start.elapsed() < Duration::from_millis(200) {
        x = std::hint::black_box(x.wrapping_mul(31).wrapping_add(7));
    }
    let after = cut_craft_priority::process_time().expect("the system says");
    assert!(
        after.saturating_sub(before) >= Duration::from_millis(100),
        "{before:?} → {after:?}"
    );
}
