//! Cut Craft's log: what the app, wgpu, winit and eframe say, kept in memory for bug reports
//! and written to `cut-craft.log`; and on a panic, `crash.log`, which the next start offers
//! to report.

use std::collections::VecDeque;
use std::io::Write as _;
use std::path::Path;
use std::sync::Mutex;

/// The log lines kept for a report.
const KEPT_LINES: usize = 200;
/// The log file, started afresh each run.
const LOG_FILE: &str = "cut-craft.log";
/// What a panic leaves for the next start.
const CRASH_FILE: &str = "crash.log";

static RECENT: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());
static FILE: Mutex<Option<std::fs::File>> = Mutex::new(None);

struct Logger;

impl log::Log for Logger {
    /// Cut Craft's own news and up; others' warnings and up (wgpu's info is a flood).
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        let floor = if metadata.target().starts_with("cut_craft") {
            log::Level::Info
        } else {
            log::Level::Warn
        };
        metadata.level() <= floor
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = format!(
            "{} {:5} {}: {}",
            chrono::Local::now().format("%H:%M:%S%.3f"),
            record.level(),
            record.target(),
            record.args()
        );
        eprintln!("{line}");
        keep(line);
    }

    fn flush(&self) {
        if let Ok(mut file) = FILE.lock()
            && let Some(file) = file.as_mut()
        {
            let _ = file.flush();
        }
    }
}

fn keep(line: String) {
    if let Ok(mut file) = FILE.lock()
        && let Some(file) = file.as_mut()
    {
        let _ = writeln!(file, "{line}");
    }
    if let Ok(mut recent) = RECENT.lock() {
        if recent.len() == KEPT_LINES {
            recent.pop_front();
        }
        recent.push_back(line);
    }
}

/// Starts the log (in `dir`, when there is one) and the crash log for a panic. Once, at
/// startup; tests do not call it.
pub fn install(dir: Option<&Path>) {
    if log::set_logger(&Logger).is_ok() {
        log::set_max_level(log::LevelFilter::Info);
    }
    let dir = dir.filter(|d| std::fs::create_dir_all(d).is_ok());
    if let (Some(dir), Ok(mut file)) = (dir, FILE.lock()) {
        *file = std::fs::File::create(dir.join(LOG_FILE)).ok();
    }
    let crash = dir.map(|d| d.join(CRASH_FILE));
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Some(path) = &crash {
            let _ = std::fs::write(path, crash_text(&info.to_string()));
        }
        default(info);
    }));
}

/// The log so far, oldest line first.
pub fn recent() -> String {
    // try_lock: a panic while a line was being kept must still write its crash log
    RECENT
        .try_lock()
        .map(|lines| lines.iter().cloned().collect::<Vec<_>>().join("\n"))
        .unwrap_or_default()
}

/// What a crash log says: the version and system, the panic and where, how it got there,
/// and the log up to it.
fn crash_text(panic: &str) -> String {
    format!(
        "Cut Craft {} on {} crashed: {panic}\n\nBacktrace:\n{}\n\nLog:\n{}\n",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::backtrace::Backtrace::force_capture(),
        recent()
    )
}

/// The crash log the last run left in `dir`, taken so it is offered once.
pub fn take_crash(dir: &Path) -> Option<String> {
    let path = dir.join(CRASH_FILE);
    let text = std::fs::read_to_string(&path).ok()?;
    let _ = std::fs::remove_file(path);
    Some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_crash_log_says_what_panicked_and_is_offered_once() {
        keep("12:00:00.000 WARN  wgpu: something odd".into());
        let text = crash_text("panicked at src/app.rs:1:1:\nboom");
        assert!(text.contains("crashed: panicked at src/app.rs:1:1:\nboom"));
        assert!(text.contains("Backtrace:"));
        assert!(text.contains("wgpu: something odd"), "the log up to it");
        let dir = std::env::temp_dir().join(format!("cut-craft-crash-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(CRASH_FILE), &text).unwrap();
        assert_eq!(take_crash(&dir).as_deref(), Some(text.as_str()));
        assert_eq!(take_crash(&dir), None, "offered once");
        let _ = std::fs::remove_dir_all(dir);
    }
}
