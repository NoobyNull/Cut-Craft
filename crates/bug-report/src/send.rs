//! Sending a report the way the person chose, and what happens when it cannot go: the
//! report is kept on disk, and the browser opens it as a new issue beside the folder of its
//! pictures, to finish by hand. Every failure ends the same way, whichever route was taken.

use crate::github::{self, Report};
use crate::relay;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// How a report goes to GitHub.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Route {
    /// Through PowerWhittle's bug-report service (its `/v1/bug` address): no GitHub account
    /// or GitHub CLI needed.
    Service(String),
    /// Through the person's own GitHub CLI, as them; pictures need write access to the
    /// repository.
    Cli,
}

/// Everything a send needs besides the report; the outside world is passed in, so tests
/// never reach it.
pub struct Sending<'a> {
    pub route: Route,
    /// The repository the issue goes to (`owner/name`), for the CLI and the browser.
    pub repo: &'a str,
    /// Where a report that could not be sent is kept.
    pub keep: Option<&'a Path>,
    /// Whether the browser and the kept folder may be opened when the report cannot go.
    pub browser: bool,
    pub install: &'a str,
    /// The person's GitHub name, for the issue to name them; empty for none.
    pub reporter: &'a str,
    pub work_bits: u32,
    pub post_bug: relay::PostBug<'a>,
    pub run: github::Run<'a>,
    /// Opens a web address or a folder with the platform's own program.
    pub open: &'a dyn Fn(&std::ffi::OsStr) -> std::io::Result<()>,
}

/// Sends `report`; the issue's address, or what went wrong and what was done about it.
pub fn send(report: &Report, s: &Sending) -> Result<String, String> {
    let sent = match &s.route {
        Route::Service(url) => through_service(report, url, s),
        Route::Cli => github::submit(report, s.repo, s.run),
    };
    sent.map_err(|why| fall_back(report, s, &why))
}

fn through_service(report: &Report, url: &str, s: &Sending) -> Result<String, String> {
    let pictures = github::pictures(report)?;
    let text = github::details(report);
    let bug = relay::Bug {
        title: &report.title,
        text: &text,
        install: s.install,
        reporter: s.reporter,
        pictures: &pictures,
    };
    relay::send_bug(url, &bug, s.work_bits, s.post_bug)
}

/// Keeps the report, then opens it in the browser as a new issue with the kept folder
/// beside it, so its pictures can be dragged in; what the person is told.
fn fall_back(report: &Report, s: &Sending, why: &str) -> String {
    let kept: Option<PathBuf> = s.keep.and_then(|dir| github::keep(report, dir).ok());
    let mut told = why.to_owned();
    if let Some(folder) = &kept {
        let _ = write!(told, "; the report is kept in {}", folder.display());
    }
    if !s.browser {
        return told;
    }
    let url = github::issue_url(s.repo, report);
    if (s.open)(url.as_ref()).is_err() {
        return told;
    }
    let has_pictures = report.screenshot.is_some() || !report.pictures.is_empty();
    match kept.filter(|_| has_pictures) {
        Some(folder) if (s.open)(folder.as_os_str()).is_ok() => format!(
            "{told}. Your browser has it as a new issue, and its pictures' folder is open: \
             drag the pictures into the issue, then submit it"
        ),
        _ => format!("{told}. Your browser has it as a new issue: check it and submit it"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    fn report() -> Report {
        Report {
            title: "Material is hollow".into(),
            text: "The material is supposed to be solid.".into(),
            state: "Screen: Job".into(),
            screenshot: Some(vec![1, 2, 3]),
            ..Report::default()
        }
    }

    struct World {
        posted: RefCell<Vec<String>>,
        gh: RefCell<Vec<String>>,
        opened: RefCell<Vec<String>>,
    }

    impl World {
        fn new() -> Self {
            Self {
                posted: RefCell::new(vec![]),
                gh: RefCell::new(vec![]),
                opened: RefCell::new(vec![]),
            }
        }
    }

    fn sending<'a>(
        route: Route,
        keep: Option<&'a Path>,
        post_bug: relay::PostBug<'a>,
        run: github::Run<'a>,
        open: &'a dyn Fn(&std::ffi::OsStr) -> std::io::Result<()>,
    ) -> Sending<'a> {
        Sending {
            route,
            repo: "pw/app",
            keep,
            browser: true,
            install: "install-1",
            reporter: "Ray-1961",
            work_bits: 1,
            post_bug,
            run,
            open,
        }
    }

    #[test]
    fn the_service_files_it_with_its_pictures_and_no_github_cli() {
        let w = World::new();
        let post = |url: &str, body: &str| {
            w.posted.borrow_mut().push(format!("{url} {body}"));
            Ok(r#"{"url":"https://github.com/pw/app/issues/669"}"#.into())
        };
        let run = |a: &[String], _: &[u8]| {
            w.gh.borrow_mut().push(a.join(" "));
            Err("gh: Not Found (HTTP 404)".into())
        };
        let open = |t: &std::ffi::OsStr| {
            w.opened.borrow_mut().push(t.to_string_lossy().into());
            Ok(())
        };
        let route = Route::Service("https://relay/v1/bug".into());
        let url = send(&report(), &sending(route, None, &post, &run, &open)).unwrap();
        assert_eq!(url, "https://github.com/pw/app/issues/669");
        let posted = w.posted.borrow();
        assert!(posted[0].starts_with("https://relay/v1/bug "));
        assert!(
            posted[0].contains(r#""name":"screenshot.png""#),
            "{}",
            posted[0]
        );
        assert!(posted[0].contains(r#""reporter":"Ray-1961""#));
        assert!(posted[0].contains("supposed to be solid"));
        assert!(w.gh.borrow().is_empty(), "the GitHub CLI is not used");
        assert!(w.opened.borrow().is_empty());
    }

    #[test]
    fn a_failed_send_opens_the_issue_and_the_pictures_folder() {
        let dir = std::env::temp_dir().join(format!("pw-bug-send-{}", std::process::id()));
        let w = World::new();
        let post = |_: &str, _: &str| Err("the bug-report service could not be reached".into());
        let run = |_: &[String], _: &[u8]| Err("gh: unused".into());
        let open = |t: &std::ffi::OsStr| {
            w.opened.borrow_mut().push(t.to_string_lossy().into());
            Ok(())
        };
        let route = Route::Service("https://relay/v1/bug".into());
        let told = send(&report(), &sending(route, Some(&dir), &post, &run, &open)).unwrap_err();
        assert!(
            told.contains("could not be reached; the report is kept in"),
            "{told}"
        );
        assert!(told.contains("drag the pictures into the issue"), "{told}");
        let opened = w.opened.borrow();
        assert!(opened[0].starts_with("https://github.com/pw/app/issues/new?"));
        assert!(PathBuf::from(&opened[1]).join("screenshot.png").exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Issue #669: the GitHub CLI was there but could not push the pictures, and the report
    /// was only kept on disk.
    #[test]
    fn the_github_cli_failing_still_opens_the_browser() {
        let w = World::new();
        let post = |_: &str, _: &str| Err("unused".into());
        let run = |a: &[String], _: &[u8]| {
            w.gh.borrow_mut().push(a.join(" "));
            Err("gh: Not Found (HTTP 404)".into())
        };
        let open = |t: &std::ffi::OsStr| {
            w.opened.borrow_mut().push(t.to_string_lossy().into());
            Ok(())
        };
        let mut no_pictures = report();
        no_pictures.screenshot = None;
        let told = send(&no_pictures, &sending(Route::Cli, None, &post, &run, &open)).unwrap_err();
        assert!(!w.gh.borrow().is_empty(), "the GitHub CLI was tried");
        assert!(told.starts_with("gh: Not Found (HTTP 404)"), "{told}");
        assert!(told.ends_with("Your browser has it as a new issue: check it and submit it"));
        assert_eq!(w.opened.borrow().len(), 1, "no pictures, no folder");
    }

    #[test]
    fn with_the_browser_off_a_failure_is_only_kept() {
        let post = |_: &str, _: &str| Err("refused".into());
        let run = |_: &[String], _: &[u8]| Err("unused".into());
        let open = |_: &std::ffi::OsStr| -> std::io::Result<()> { panic!("not opened") };
        let mut s = sending(
            Route::Service("https://relay/v1/bug".into()),
            None,
            &post,
            &run,
            &open,
        );
        s.browser = false;
        assert_eq!(send(&report(), &s).unwrap_err(), "refused");
    }
}
