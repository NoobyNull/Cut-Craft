//! Sending a crash's log to Cut Craft's crash-report service, for people who said yes to
//! sending crash logs: no GitHub account needed and nothing to press. What goes is only
//! what [`payload`] makes: the version and system, a plain title, a signature that groups
//! crashes with the same cause, a random number for this install (so one install can't
//! flood the service; it says nothing about the person), the crash log, scrubbed of home
//! folders and the person's file names and cut to [`MOST_LOG_BYTES`], and a proof of work
//! ([`prove`]) the service asks of every report so junk costs whoever sends it.

use crate::privacy;
use sha2::{Digest as _, Sha256};
use std::fmt::Write as _;

/// The most of a crash log sent: the service refuses more than 64 KB.
pub const MOST_LOG_BYTES: usize = 60 * 1024;
/// How many leading zero bits the service wants of a report's proof of work (its
/// `POW_BITS`): about four million tries, a fraction of a second in a release build.
pub const WORK_BITS: u32 = 22;
/// Where the crash log's own log starts: the first of it to go when the log is too long.
const LOG_HEADING: &str = "\nLog:\n";
/// Of a log cut short, how much of its start is kept (the panic and what was running);
/// the rest is its end (what led up to it).
const HEAD_BYTES: usize = 16 * 1024;

/// Posts `body` (JSON) to `url`; why it failed, if it did.
pub type Post<'a> = &'a dyn Fn(&str, &str) -> Result<(), Failure>;

/// A crash report the service did not take: why, and whether to try again at a later start
/// (no connection, the service busy or down, or too soon after the last one) rather than
/// give up on it (the service refused it as it is).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failure {
    pub retry: bool,
    pub why: String,
}

impl Failure {
    /// The service answered `status`: worth trying again later only when it was busy (429)
    /// or failing (5xx).
    pub fn of_status(status: u16) -> Self {
        Self {
            retry: status == 429 || status >= 500,
            why: format!("the crash-report service answered {status}"),
        }
    }
}

/// What is sent for crash log `crash` from install `install`, its proof of work made for
/// `day` (`YYYY-MM-DD`, UTC) with `bits` zero bits.
pub fn payload(crash: &str, install: &str, day: &str, bits: u32) -> serde_json::Value {
    let log = cut(&privacy::hide_files(&privacy::scrub(crash)));
    serde_json::json!({
        "version": version_of(crash),
        "os": std::env::consts::OS,
        "title": privacy::hide_files(&privacy::crash_title(crash)),
        "signature": signature(&log),
        "install_id": install,
        "work": prove(&log, install, day, bits),
        "log": log,
    })
}

/// Sends crash log `crash` to the service at `url`, with a proof of work of `bits` zero bits
/// made for today. Slow enough to want a thread of its own.
pub fn send(url: &str, crash: &str, install: &str, bits: u32, post: Post) -> Result<(), Failure> {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    post(url, &payload(crash, install, &today, bits).to_string())
}

/// The version the crash log says crashed (its first line, `Cut Craft <version> on …`): the
/// service wants the two the same, and a log the last version left is sent by this one.
fn version_of(crash: &str) -> String {
    crash
        .strip_prefix("Cut Craft ")
        .and_then(|rest| rest.split_once(" on "))
        .map(|(version, _)| version)
        .filter(|v| !v.is_empty() && !v.contains(char::is_whitespace))
        .unwrap_or(env!("CARGO_PKG_VERSION"))
        .to_owned()
}

/// The text a proof of work hashes, `nonce` last: what the service hashes to check it.
fn work_text(log: &str, install: &str, day: &str, nonce: &str) -> String {
    format!(
        "cutcraft-crash-v1\n{day}\n{install}\n{:x}\n{nonce}",
        Sha256::digest(log.as_bytes())
    )
}

/// A proof of work for sending `log` from `install` on `day`: the first counter (in base 36,
/// as the service wants it) whose hash starts with `bits` zero bits.
pub fn prove(log: &str, install: &str, day: &str, bits: u32) -> String {
    let start = work_text(log, install, day, "");
    let mut sha = Sha256::new();
    sha.update(start.as_bytes());
    (0..u64::MAX)
        .map(base36)
        .find(|nonce| {
            let mut hash = sha.clone();
            hash.update(nonce.as_bytes());
            leading_zeros(&hash.finalize()) >= bits
        })
        .unwrap_or_default()
}

fn leading_zeros(hash: &[u8]) -> u32 {
    let mut bits = 0;
    for &byte in hash {
        if byte != 0 {
            return bits + byte.leading_zeros();
        }
        bits += 8;
    }
    bits
}

fn base36(mut n: u64) -> String {
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut out = Vec::new();
    loop {
        out.push(DIGITS[usize::try_from(n % 36).unwrap_or(0)]);
        n /= 36;
        if n == 0 {
            break;
        }
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

/// Posts JSON over HTTPS.
pub fn post_https(url: &str, body: &str) -> Result<(), Failure> {
    ureq::post(url)
        .header("Content-Type", "application/json")
        .header(
            "User-Agent",
            &format!("CutCraft/{}", env!("CARGO_PKG_VERSION")),
        )
        .send(body)
        .map(drop)
        .map_err(|e| match e {
            ureq::Error::StatusCode(status) => Failure::of_status(status),
            // no connection, no name lookup, a time-out: another start may do better
            other => Failure {
                retry: true,
                why: format!("the crash report could not be sent: {other}"),
            },
        })
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

/// `log` cut to [`MOST_LOG_BYTES`]: the oldest lines of its own log go first, so the panic,
/// what was running and the backtrace (which the service checks for) stay; only should that
/// still be too long, its start and its end, with a line saying what was left out between.
fn cut(log: &str) -> String {
    if log.len() <= MOST_LOG_BYTES {
        return log.to_owned();
    }
    if let Some(at) = log.find(LOG_HEADING) {
        let (head, lines) = log.split_at(at + LOG_HEADING.len());
        let lines: Vec<&str> = lines.split_inclusive('\n').collect();
        let mut kept = lines.len();
        let mut size: usize = lines.iter().map(|l| l.len()).sum();
        loop {
            let note = format!("[… {} earlier lines left out …]\n", lines.len() - kept);
            if head.len() + note.len() + size <= MOST_LOG_BYTES {
                let mut out = head.to_owned();
                out.push_str(&note);
                out.extend(lines[lines.len() - kept..].iter().copied());
                return out;
            }
            if kept == 0 {
                return halves(head);
            }
            size -= lines[lines.len() - kept].len();
            kept -= 1;
        }
    }
    halves(log)
}

/// `log`'s start and its end, to [`MOST_LOG_BYTES`].
fn halves(log: &str) -> String {
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

/// A new install's number: a random UUID (version 4), so two installs don't share one, and
/// nothing about the computer or the person. The randomness is the operating system's, by
/// way of the standard library's hash keys.
pub fn new_install_id() -> String {
    use std::hash::{BuildHasher as _, Hasher as _};
    let random = || {
        let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
        hasher.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
        );
        hasher.finish()
    };
    let random = (u128::from(random()) << 64) | u128::from(random());
    // the version (4) and the variant (RFC 4122)
    let bits = (random & !((0xf000 << 64) | (0xc000 << 48))) | (0x4000 << 64) | (0x8000 << 48);
    let hex = format!("{bits:032x}");
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    const CRASH: &str = "Cut Craft 0.6.1 on linux crashed: panicked at crates/app/src/view2d.rs:9:9:\nindex 12 out of bounds\n\nDoing when it crashed:\n- Previewing Pocket 1\n\nBacktrace:\n  at /home/sam/src/a.rs\n\nLog:\nopened /home/sam/Jobs/sign.ccjob\n";

    #[test]
    fn what_is_sent_is_scrubbed_titled_and_grouped() {
        let sent = RefCell::new(String::new());
        let post = |url: &str, body: &str| {
            assert_eq!(url, "https://crash.example/v1/crash");
            *sent.borrow_mut() = body.to_owned();
            Ok(())
        };
        send(
            "https://crash.example/v1/crash",
            CRASH,
            "abc-123",
            WORK_BITS,
            &post,
        )
        .unwrap();
        let body: serde_json::Value = serde_json::from_str(&sent.borrow()).unwrap();
        assert_eq!(body["install_id"], "abc-123");
        assert_eq!(
            body["version"], "0.6.1",
            "the version that crashed, as its log says"
        );
        assert_eq!(
            body["title"],
            "Crash: while previewing Pocket 1: index 12 out of bounds"
        );
        let log = body["log"].as_str().unwrap();
        assert!(!log.contains("/home/sam") && !log.contains("sign"), "{log}");
        assert!(log.contains("opened <file>.ccjob"), "{log}");
        assert!(
            log.starts_with(
                "Cut Craft 0.6.1 on linux crashed: panicked at crates/app/src/view2d.rs:9:9:\n"
            ),
            "the layout the service checks for: {log}"
        );
        // the proof of work holds for the log as sent, today or (just after midnight) yesterday
        let work = body["work"].as_str().unwrap();
        assert!(!work.is_empty() && work.len() <= 24);
        assert!(
            work.chars()
                .all(|c| c.is_ascii_digit() || c.is_ascii_lowercase())
        );
        let now = chrono::Utc::now();
        let proven = [now, now - chrono::Duration::days(1)].iter().any(|day| {
            let text = work_text(log, "abc-123", &day.format("%Y-%m-%d").to_string(), work);
            leading_zeros(&Sha256::digest(text.as_bytes())) >= WORK_BITS
        });
        assert!(proven, "{work}");
    }

    #[test]
    fn the_proof_of_work_hashes_what_the_service_hashes() {
        // the service's text for this report: SHA-256 of "log", as `sha256sum` gives it
        assert_eq!(
            work_text("log", "id-1", "2026-11-12", "z"),
            "cutcraft-crash-v1\n2026-11-12\nid-1\n836ff184e7b41b1e13cb5fd89fa1de98dbbab99e9d2918913ff43b86a5c7c213\nz"
        );
        let nonce = prove("log", "id-1", "2026-11-12", 12);
        let hash = Sha256::digest(work_text("log", "id-1", "2026-11-12", &nonce).as_bytes());
        assert!(leading_zeros(&hash) >= 12);
        assert_eq!(base36(0), "0");
        assert_eq!(base36(36 * 36 + 35), "10z");
        assert_eq!(leading_zeros(&[0, 0b0001_0000]), 11);
    }

    #[test]
    fn a_log_from_an_older_version_is_sent_as_that_version() {
        assert_eq!(version_of(CRASH), "0.6.1");
        assert_eq!(version_of("garbled"), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn only_a_busy_or_failing_service_is_tried_again() {
        assert!(Failure::of_status(503).retry);
        assert!(Failure::of_status(429).retry);
        assert!(!Failure::of_status(400).retry);
        assert!(!Failure::of_status(413).retry);
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
    fn a_long_log_loses_its_oldest_lines_first_and_keeps_the_backtrace() {
        let lines = (0..5000).fold(String::new(), |mut out, n| {
            let _ = writeln!(out, "12:00:00.000 INFO  cut_craft: line {n}");
            out
        });
        let long = format!(
            "{}{lines}",
            CRASH.replace("Log:\nopened /home/sam/Jobs/sign.ccjob\n", "Log:\n")
        );
        let short = cut(&long);
        assert!(short.len() <= MOST_LOG_BYTES, "{}", short.len());
        assert!(short.starts_with("Cut Craft 0.6.1 on linux crashed: "));
        assert!(short.contains("\n\nBacktrace:\n"));
        assert!(short.contains("earlier lines left out"));
        assert!(
            short.ends_with("line 4999\n"),
            "the lines just before the crash stay"
        );
        assert!(!short.contains("line 0\n"));
    }

    #[test]
    fn installs_get_different_random_uuids() {
        let id = new_install_id();
        assert_ne!(id, new_install_id());
        assert_eq!(id.len(), 36);
        assert_eq!(&id[14..15], "4", "{id}");
        assert!(matches!(&id[19..20], "8" | "9" | "a" | "b"), "{id}");
        assert!(id.chars().all(|c| c == '-' || c.is_ascii_hexdigit()));
    }
}
