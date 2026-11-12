//! Cut Craft's log: what the app, wgpu, winit and eframe say, kept in memory for bug reports
//! and written to `cut-craft.log`; and on a panic, `crash.log`, which the next start offers
//! to report.
//!
//! What the app does is traced here too: [`step`] logs a tool being used, and [`doing`]
//! logs a longer job's start and how it ended, and holds it as running meanwhile, so a crash
//! log says what was running when it crashed (and the step it was on), not only the panic.

use std::collections::VecDeque;
use std::io::Write as _;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// The log lines kept for a report.
const KEPT_LINES: usize = 200;
/// The log file, started afresh each run.
const LOG_FILE: &str = "cut-craft.log";
/// What a panic leaves for the next start.
const CRASH_FILE: &str = "crash.log";

/// The crash log's list of what was running.
pub const DOING_HEADING: &str = "Doing when it crashed:\n";
/// Between a job and the step it was on, in that list.
pub const STAGE_MARK: &str = " — now: ";

static RECENT: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());
static FILE: Mutex<Option<std::fs::File>> = Mutex::new(None);

/// What the job says it is on now.
type Stage = Box<dyn Fn() -> String + Send>;

/// The jobs running now, by number, oldest first.
static DOING: Mutex<Vec<(u64, String, Option<Stage>)>> = Mutex::new(Vec::new());
static NEXT: AtomicU64 = AtomicU64::new(0);

/// Logs a tool being used: `what` it is and how it is used ("Tool: Pocket Toolpath").
pub fn step(what: &str) {
    log::info!(target: "cut_craft::trace", "{what}");
}

/// A job running: logged as started now, and as finished, failed or stopped when it ends,
/// with how long it took; while it runs a crash log names it.
#[must_use = "the job counts as running until this is dropped"]
pub struct Doing {
    number: u64,
    what: String,
    started: Instant,
    ended: bool,
}

/// Starts job `what` ("Calculating Pocket Toolpath").
pub fn doing(what: &str) -> Doing {
    doing_in(what, None)
}

/// Starts job `what`, `stage` saying the step it is on whenever a crash log asks.
pub fn doing_with(what: &str, stage: impl Fn() -> String + Send + 'static) -> Doing {
    doing_in(what, Some(Box::new(stage)))
}

fn doing_in(what: &str, stage: Option<Stage>) -> Doing {
    let number = NEXT.fetch_add(1, Ordering::Relaxed);
    log::info!(target: "cut_craft::trace", "Started: {what}");
    if let Ok(mut doing) = DOING.lock() {
        doing.push((number, what.to_owned(), stage));
    }
    Doing {
        number,
        what: what.to_owned(),
        started: Instant::now(),
        ended: false,
    }
}

impl Doing {
    /// It worked: `result` says what came of it ("12 toolpath moves"), if anything.
    pub fn finished(mut self, result: &str) {
        self.end("Finished", result);
    }

    /// It went wrong: `why`.
    pub fn failed(mut self, why: &str) {
        self.end("Failed", why);
    }

    fn end(&mut self, how: &str, note: &str) {
        self.ended = true;
        let took = self.started.elapsed().as_secs_f64();
        let note = if note.is_empty() {
            String::new()
        } else {
            format!(": {note}")
        };
        let line = format!("{how}: {} in {took:.2} s{note}", self.what);
        if how == "Failed" {
            log::warn!(target: "cut_craft::trace", "{line}");
        } else {
            log::info!(target: "cut_craft::trace", "{line}");
        }
    }
}

impl Drop for Doing {
    fn drop(&mut self) {
        if !self.ended {
            self.end("Stopped", "");
        }
        if let Ok(mut doing) = DOING.lock() {
            doing.retain(|(n, ..)| *n != self.number);
        }
    }
}

/// What is running now, one line each, oldest first, with the step each is on.
pub fn running() -> Vec<String> {
    // try_lock: a panic while the list was changing must still write its crash log
    DOING
        .try_lock()
        .map(|doing| {
            doing
                .iter()
                .map(|(_, what, stage)| {
                    let now = stage.as_ref().map(|s| s()).unwrap_or_default();
                    if now.is_empty() {
                        what.clone()
                    } else {
                        format!("{what}{STAGE_MARK}{now}")
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

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
        let said = format!(
            "{:5} {}: {}",
            record.level(),
            record.target(),
            record.args()
        );
        // the same line again and again (a drag, a frame) is counted, not kept each time
        let Some(repeats) = repeated(&said) else {
            return;
        };
        let stamp = chrono::Local::now().format("%H:%M:%S%.3f");
        if repeats > 0 {
            keep(format!(
                "{stamp} (the line before came {repeats} more times)"
            ));
        }
        let line = format!("{stamp} {said}");
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

/// The last line logged, and how many times since it came again.
static LAST: Mutex<(String, usize)> = Mutex::new((String::new(), 0));

/// `None` when `said` is the line before over again (it is counted); else how many times
/// the line before came again, unsaid.
fn repeated(said: &str) -> Option<usize> {
    let Ok(mut last) = LAST.lock() else {
        return Some(0);
    };
    if last.0 == said {
        last.1 += 1;
        return None;
    }
    let repeats = last.1;
    *last = (said.to_owned(), 0);
    Some(repeats)
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

/// What a crash log says: the version and system, the panic and where, what was running
/// then, how it got there, and the log up to it.
fn crash_text(panic: &str) -> String {
    let running = running();
    let doing = if running.is_empty() {
        String::new()
    } else {
        let list: String = running.iter().flat_map(|r| ["- ", r, "\n"]).collect();
        format!("{DOING_HEADING}{list}\n")
    };
    format!(
        "Cut Craft {} on {} crashed: {panic}\n\n{doing}Backtrace:\n{}\n\nLog:\n{}\n",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::backtrace::Backtrace::force_capture(),
        recent()
    )
}

/// Puts crash log `text` back in `dir`, to be taken at the next start (a report that could
/// not be sent yet).
pub fn put_back_crash(dir: &Path, text: &str) -> std::io::Result<()> {
    std::fs::write(dir.join(CRASH_FILE), text)
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

    #[test]
    fn a_crash_log_names_the_job_running_and_the_step_it_was_on() {
        let job = doing_with("Calculating 3D Finishing for the test", || {
            "Dropping the tool".into()
        });
        let text = crash_text("panicked at a.rs:1:1:\nboom");
        assert!(
            text.contains(
                "Doing when it crashed:\n- Calculating 3D Finishing for the test — now: Dropping the tool\n"
            ),
            "{text}"
        );
        job.finished("done");
        assert!(
            !running()
                .iter()
                .any(|r| r.contains("3D Finishing for the test")),
            "over once finished"
        );
        {
            let _dropped = doing("Previewing for the test");
            assert!(running().iter().any(|r| r == "Previewing for the test"));
        }
        assert!(
            !running().iter().any(|r| r == "Previewing for the test"),
            "over once dropped, however it ends"
        );
    }
}
