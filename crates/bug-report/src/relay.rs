//! Sending a crash's log to Cut Craft's crash-report service, for people who said yes to
//! sending crash logs: no GitHub account needed and nothing to press. What goes is only
//! what [`payload`] makes: the version and system, a plain title, a signature that groups
//! crashes with the same cause, a random number for this install (so one install can't
//! flood the service; it says nothing about the person) and the crash log, scrubbed of home
//! folders and cut to [`MOST_LOG_BYTES`].

use crate::privacy;
use std::fmt::Write as _;

/// The most of a crash log sent: the service refuses more.
pub const MOST_LOG_BYTES: usize = 60 * 1024;
/// Of a log cut short, how much of its start is kept (the panic and what was running);
/// the rest is its end (what led up to it).
const HEAD_BYTES: usize = 16 * 1024;

/// Posts `body` (JSON) to `url`; why it failed, if it did.
pub type Post<'a> = &'a dyn Fn(&str, &str) -> Result<(), String>;

/// What is sent for crash log `crash` from install `install`.
pub fn payload(crash: &str, install: &str) -> serde_json::Value {
    let log = privacy::scrub(crash);
    serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "os": std::env::consts::OS,
        "title": privacy::crash_title(crash),
        "signature": signature(&log),
        "install_id": install,
        "log": cut(&log),
    })
}

/// Sends crash log `crash` to the service at `url`.
pub fn send(url: &str, crash: &str, install: &str, post: Post) -> Result<(), String> {
    post(url, &payload(crash, install).to_string())
}

/// Posts JSON over HTTPS.
pub fn post_https(url: &str, body: &str) -> Result<(), String> {
    ureq::post(url)
        .header("Content-Type", "application/json")
        .header(
            "User-Agent",
            &format!("CutCraft/{}", env!("CARGO_PKG_VERSION")),
        )
        .send(body)
        .map(drop)
        .map_err(|e| format!("the crash report could not be sent: {e}"))
}

/// What groups crashes with the same cause: the panic's first line (where it was raised,
/// and the message's start), as a short hex number that stays the same on every computer.
pub fn signature(crash: &str) -> String {
    let panic = crash
        .split_once("crashed: ")
        .map_or(crash, |(_, rest)| rest);
    let mut lines = panic.lines();
    let first = lines.next().unwrap_or_default();
    // "panicked at <file>:<line>:<col>:" then the message on the next line
    let message = lines.next().unwrap_or_default();
    // numbers in a message (sizes, counts) differ from one crash of a kind to the next
    fnv(&format!("{first}\n{}", strip_digits(message)))
}

/// FNV-1a, as hex: the same on every computer and Rust version, unlike the standard hasher.
fn fnv(text: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn strip_digits(text: &str) -> String {
    text.chars().filter(|c| !c.is_ascii_digit()).collect()
}

/// `log` cut to [`MOST_LOG_BYTES`]: its start and its end, with a line saying what was left
/// out between.
fn cut(log: &str) -> String {
    if log.len() <= MOST_LOG_BYTES {
        return log.to_owned();
    }
    let head = floor(log, HEAD_BYTES);
    let tail_from = ceil(log, log.len() - (MOST_LOG_BYTES - HEAD_BYTES));
    let mut out = log[..head].to_owned();
    let _ = write!(
        out,
        "\n[… {} bytes left out …]\n",
        tail_from.saturating_sub(head)
    );
    out.push_str(&log[tail_from..]);
    out
}

/// The nearest character boundary at or before `at`.
fn floor(text: &str, at: usize) -> usize {
    (0..=at.min(text.len()))
        .rev()
        .find(|&i| text.is_char_boundary(i))
        .unwrap_or(0)
}

/// The nearest character boundary at or after `at`.
fn ceil(text: &str, at: usize) -> usize {
    (at..=text.len())
        .find(|&i| text.is_char_boundary(i))
        .unwrap_or(text.len())
}

/// A new install's number: random enough that two installs don't share one, and nothing
/// about the computer or the person.
pub fn new_install_id() -> String {
    static MADE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let made = MADE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    fnv(&format!(
        "{}-{}-{made}-{:p}",
        now.as_nanos(),
        std::process::id(),
        &now
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    const CRASH: &str = "Cut Craft 0.6.3 on linux crashed: panicked at crates/app/src/view2d.rs:9:9:\nindex 12 out of bounds\n\nDoing when it crashed:\n- Previewing Pocket 1\n\nBacktrace:\n  at /home/sam/src/a.rs\n\nLog:\nopened /home/sam/Jobs/sign.ccjob\n";

    #[test]
    fn what_is_sent_is_scrubbed_titled_and_grouped() {
        let sent = RefCell::new(String::new());
        let post = |url: &str, body: &str| {
            assert_eq!(url, "https://crash.example/v1/crash");
            *sent.borrow_mut() = body.to_owned();
            Ok(())
        };
        send("https://crash.example/v1/crash", CRASH, "abc", &post).unwrap();
        let body: serde_json::Value = serde_json::from_str(&sent.borrow()).unwrap();
        assert_eq!(body["install_id"], "abc");
        assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(
            body["title"],
            "Crash: while previewing Pocket 1: index 12 out of bounds"
        );
        let log = body["log"].as_str().unwrap();
        assert!(!log.contains("/home/sam"), "{log}");
        assert!(log.contains("~/Jobs/sign.ccjob"));
    }

    #[test]
    fn the_same_cause_has_the_same_signature_whatever_the_numbers() {
        let other = CRASH.replace("index 12", "index 400");
        assert_eq!(signature(CRASH), signature(&other));
        let elsewhere = CRASH.replace("view2d.rs:9:9", "view3d.rs:9:9");
        assert_ne!(signature(CRASH), signature(&elsewhere));
        assert_eq!(signature(CRASH).len(), 16);
    }

    #[test]
    fn a_long_log_keeps_its_start_and_its_end() {
        let long = format!("START{}END", "é".repeat(MOST_LOG_BYTES));
        let short = cut(&long);
        assert!(short.len() <= MOST_LOG_BYTES + 64);
        assert!(short.starts_with("START") && short.ends_with("END"));
        assert!(short.contains("bytes left out"));
    }

    #[test]
    fn installs_get_different_numbers() {
        assert_ne!(new_install_id(), new_install_id());
    }
}
