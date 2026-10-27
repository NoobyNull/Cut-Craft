//! Sending a report: pictures go to a `bug-reports` branch of the repository (issues cannot
//! hold uploads through the API), then an issue links them, through the GitHub CLI.

use base64::Engine;
use eframe::egui;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The branch reports' pictures are kept on, apart from the code.
const PICTURE_BRANCH: &str = "bug-reports";
/// The label issues get, when the repository has it.
const LABEL: &str = "bug";

/// Runs `gh` with these arguments and this standard input; its trimmed output, or why it
/// failed.
pub type Run<'a> = &'a dyn Fn(&[String], &[u8]) -> Result<String, String>;

/// What is sent.
#[derive(Clone, Debug, Default)]
pub struct Report {
    pub title: String,
    pub text: String,
    /// Where the right-click was, in window points.
    pub at: (f32, f32),
    /// The named parts of the window there, outermost first, each with its source line.
    pub regions: Vec<(String, String)>,
    pub state: String,
    /// The window, as a PNG.
    pub screenshot: Option<Vec<u8>>,
    pub pictures: Vec<PathBuf>,
}

/// The window as a PNG.
pub fn png(image: &egui::ColorImage) -> Result<Vec<u8>, String> {
    let [w, h] = image.size;
    let rgba: Vec<u8> = image
        .pixels
        .iter()
        .flat_map(egui::Color32::to_array)
        .collect();
    let mut out = std::io::Cursor::new(vec![]);
    let (w, h) = (
        u32::try_from(w).map_err(|e| e.to_string())?,
        u32::try_from(h).map_err(|e| e.to_string())?,
    );
    image::RgbaImage::from_raw(w, h, rgba)
        .ok_or_else(|| format!("a {w} x {h} picture needs more pixels than there are"))?
        .write_to(&mut out, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(out.into_inner())
}

/// GitHub's new-issue page for `repo` with the report filled in, for when `gh` is not there
/// to send it; the pictures cannot go this way.
pub fn issue_url(repo: &str, report: &Report) -> String {
    format!(
        "https://github.com/{repo}/issues/new?labels={LABEL}&title={}&body={}",
        percent(&report.title),
        percent(&compose(report, &[]))
    )
}

/// `text` as a URL query value: everything but unreserved characters as `%XX`.
fn percent(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(byte));
            }
            _ => {
                let _ = write!(out, "%{byte:02X}");
            }
        }
    }
    out
}

/// Whether the GitHub CLI is installed.
pub fn gh_installed() -> bool {
    Command::new("gh")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

/// Opens `target` (a folder, a file such as a PDF to print, or a web address) with the
/// platform's own program for it.
pub fn open_with_platform(target: &std::ffi::OsStr) -> std::io::Result<()> {
    let opener = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    Command::new(opener).arg(target).spawn().map(drop)
}

/// The issue's text: the reporter's words, where it was, the state, then the pictures.
pub fn compose(report: &Report, pictures: &[(String, String)]) -> String {
    let mut body = String::new();
    body.push_str(if report.text.trim().is_empty() {
        "_No description._"
    } else {
        report.text.trim()
    });
    body.push_str("\n\n### Where\n\n");
    let _ = write!(
        body,
        "Right-clicked at ({:.0}, {:.0}), in (outermost first):\n\n",
        report.at.0, report.at.1
    );
    if report.regions.is_empty() {
        body.push_str("- no named part of the window\n");
    }
    for (name, at) in &report.regions {
        let _ = writeln!(body, "- {name} — `{at}`");
    }
    body.push_str("\n### State\n\n```\n");
    body.push_str(report.state.trim());
    body.push_str("\n```\n");
    if !pictures.is_empty() {
        body.push_str("\n### Pictures\n\n");
        for (name, url) in pictures {
            let _ = writeln!(body, "![{name}]({url}?raw=true)");
        }
    }
    body.push_str("\n_Sent from Cut Craft's Report a Bug._\n");
    body
}

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

/// The pictures to send, named: the screenshot, then each added file.
fn pictures(report: &Report) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut out = vec![];
    if let Some(shot) = &report.screenshot {
        out.push(("screenshot.png".to_owned(), shot.clone()));
    }
    for path in &report.pictures {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let name = path.file_name().map_or_else(
            || "picture".to_owned(),
            |n| {
                n.to_string_lossy()
                    .chars()
                    .map(|c| {
                        if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                            c
                        } else {
                            '_'
                        }
                    })
                    .collect()
            },
        );
        out.push((name, bytes));
    }
    Ok(out)
}

/// Makes the picture branch from the default branch, unless it is there.
fn picture_branch(repo: &str, run: Run) -> Result<(), String> {
    let branch = format!("repos/{repo}/git/ref/heads/{PICTURE_BRANCH}");
    if run(&args(&["api", &branch]), &[]).is_ok() {
        return Ok(());
    }
    let default = run(
        &args(&["api", &format!("repos/{repo}"), "--jq", ".default_branch"]),
        &[],
    )?;
    let head = format!("repos/{repo}/git/ref/heads/{default}");
    let sha = run(&args(&["api", &head, "--jq", ".object.sha"]), &[])?;
    let made = [
        "api",
        "-X",
        "POST",
        &format!("repos/{repo}/git/refs"),
        "-f",
        &format!("ref=refs/heads/{PICTURE_BRANCH}"),
        "-f",
        &format!("sha={sha}"),
    ];
    run(&args(&made), &[]).map(|_| ())
}

/// Sends `report` as an issue in `repo`; the issue's address.
pub fn submit(report: &Report, repo: &str, run: Run) -> Result<String, String> {
    let pictures = pictures(report)?;
    let mut links = vec![];
    if !pictures.is_empty() {
        picture_branch(repo, run)?;
        let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        for (i, (name, bytes)) in pictures.iter().enumerate() {
            let path = format!("repos/{repo}/contents/{stamp}/{}-{name}", i + 1);
            let upload = serde_json::json!({
                "message": format!("Picture for bug report: {}", report.title),
                "content": base64::engine::general_purpose::STANDARD.encode(bytes),
                "branch": PICTURE_BRANCH,
            });
            let put = [
                "api",
                "-X",
                "PUT",
                &path,
                "--input",
                "-",
                "--jq",
                ".content.html_url",
            ];
            let url = run(&args(&put), upload.to_string().as_bytes())?;
            links.push((name.clone(), url));
        }
    }
    let body = compose(report, &links);
    let create = [
        "issue",
        "create",
        "-R",
        repo,
        "--title",
        &report.title,
        "--body-file",
        "-",
    ];
    let mut labelled = args(&create);
    labelled.extend(args(&["--label", LABEL]));
    // a repository without the label still gets the issue
    run(&labelled, body.as_bytes()).or_else(|_| run(&args(&create), body.as_bytes()))
}

/// Runs the GitHub CLI.
pub fn run_gh(arguments: &[String], input: &[u8]) -> Result<String, String> {
    let mut child = Command::new("gh")
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("the GitHub CLI (gh) could not run: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input).map_err(|e| e.to_string())?;
    }
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    let text = |b: &[u8]| String::from_utf8_lossy(b).trim().to_owned();
    if out.status.success() {
        Ok(text(&out.stdout))
    } else {
        Err(format!("gh: {}", text(&out.stderr)))
    }
}

/// Keeps a report that could not be sent in a folder of its own under `dir`; that folder.
pub fn keep(report: &Report, dir: &Path) -> Result<PathBuf, String> {
    let folder = dir.join(chrono::Local::now().format("%Y%m%d-%H%M%S").to_string());
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let pictures = pictures(report).unwrap_or_default();
    for (name, bytes) in &pictures {
        std::fs::write(folder.join(name), bytes).map_err(|e| e.to_string())?;
    }
    let local: Vec<(String, String)> = pictures
        .iter()
        .map(|(n, _)| (n.clone(), n.clone()))
        .collect();
    let text = format!("# {}\n\n{}", report.title, compose(report, &local));
    std::fs::write(folder.join("report.md"), text).map_err(|e| e.to_string())?;
    Ok(folder)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn the_browser_fallback_fills_in_the_issue() {
        let url = issue_url("me/app", &report());
        assert!(url.starts_with(
            "https://github.com/me/app/issues/new?labels=bug&title=Apply%20does%20nothing&body="
        ));
        assert!(
            url.contains("Clicked%20Apply.") && url.contains("%23%23%23%20Where"),
            "{url}"
        );
        assert!(!url.contains('\n') && !url.contains(' '), "{url}");
        assert_eq!(percent("a-b_c.d~e Z/9"), "a-b_c.d~e%20Z%2F9");
    }

    fn report() -> Report {
        Report {
            title: "Apply does nothing".into(),
            text: "Clicked Apply.".into(),
            at: (12.0, 34.0),
            regions: vec![(
                "Tool Database".into(),
                "crates/tool-database/src/manager.rs:120".into(),
            )],
            state: "Screen: Job".into(),
            screenshot: Some(vec![1, 2, 3]),
            pictures: vec![],
        }
    }

    #[test]
    fn the_issue_says_where_and_links_the_pictures() {
        let body = compose(
            &report(),
            &[("screenshot.png".into(), "https://x/s.png".into())],
        );
        assert!(body.starts_with("Clicked Apply."));
        assert!(body.contains("- Tool Database — `crates/tool-database/src/manager.rs:120`"));
        assert!(body.contains("Screen: Job"));
        assert!(body.contains("![screenshot.png](https://x/s.png?raw=true)"));
    }

    #[test]
    fn pictures_go_to_their_branch_then_the_issue_is_made() {
        let calls = RefCell::new(vec![]);
        let run = |a: &[String], input: &[u8]| {
            calls.borrow_mut().push((a.join(" "), input.len()));
            let line = a.join(" ");
            if line == "api repos/me/app/git/ref/heads/bug-reports" {
                Err("gh: Not Found".into())
            } else if line.contains("--jq .default_branch") {
                Ok("main".into())
            } else if line.contains("--jq .object.sha") {
                Ok("abc123".into())
            } else if line.contains("contents/") {
                Ok("https://github.com/me/app/blob/bug-reports/1-screenshot.png".into())
            } else if line.contains("--label bug") {
                Err("gh: could not add label".into())
            } else {
                Ok("https://github.com/me/app/issues/7".into())
            }
        };
        let url = submit(&report(), "me/app", &run).unwrap();
        assert_eq!(url, "https://github.com/me/app/issues/7");
        let calls = calls.into_inner();
        assert!(
            calls
                .iter()
                .any(|(c, _)| c.contains("refs -f ref=refs/heads/bug-reports -f sha=abc123"))
        );
        let issue = calls.last().unwrap();
        assert!(
            issue
                .0
                .starts_with("issue create -R me/app --title Apply does nothing")
        );
        assert!(issue.1 > 0, "the body goes on standard input");
        assert_eq!(
            calls
                .iter()
                .filter(|(c, _)| c.starts_with("issue create"))
                .count(),
            2,
            "retried without the label"
        );
    }

    #[test]
    fn an_unsent_report_is_kept_with_its_pictures() {
        let dir = std::env::temp_dir().join(format!("cut-craft-bug-{}", std::process::id()));
        let folder = keep(&report(), &dir).unwrap();
        assert!(folder.join("screenshot.png").exists());
        let text = std::fs::read_to_string(folder.join("report.md")).unwrap();
        assert!(text.starts_with("# Apply does nothing"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn screenshots_become_pngs() {
        let image = egui::ColorImage::new([2, 1], egui::Color32::RED);
        assert!(png(&image).unwrap().starts_with(&[0x89, b'P', b'N', b'G']));
    }
}
