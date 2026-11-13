//! What a report must not give away. Reports become public GitHub issues, so before a log,
//! crash log or the app's state is shown or sent, the person's user name and home folder
//! are taken out of every path in it ([`scrub`]); and a crash's report is titled in plain
//! words rather than with the source path it panicked at ([`crash_title`]).

/// What a home folder becomes: `/home/sam/Jobs/sign.ccjob` reads `~/Jobs/sign.ccjob`.
const HOME: &str = "~";
/// The longest title a crash report gets.
const TITLE_CHARS: usize = 80;

use std::fmt::Write as _;

/// `text` with every home folder in it (`/home/<name>`, `/Users/<name>`,
/// `C:\Users\<name>`, any drive or slash) put as `~`, so the user name goes with it; and the
/// build machine's long library and compiler paths cut to `<crates>` and `<rust>`.
pub fn scrub(text: &str) -> String {
    let home = ["HOME", "USERPROFILE"]
        .iter()
        .filter_map(std::env::var_os)
        .map(|h| h.to_string_lossy().trim_end_matches(['/', '\\']).to_owned())
        .find(|h| h.len() > 1);
    scrub_with(text, home.as_deref())
}

/// [`scrub`], with `home` (this computer's home folder, wherever it is) put as `~` first.
fn scrub_with(text: &str, home: Option<&str>) -> String {
    let text = match home {
        Some(home) => text.replace(home, HOME),
        None => text.to_owned(),
    };
    let text = libraries(&text);
    let mut out = String::with_capacity(text.len());
    let mut rest = text.as_str();
    while let Some((start, end)) = next_home(rest) {
        out.push_str(&rest[..start]);
        out.push_str(HOME);
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

/// Where the first home folder in `text` starts (with its drive letter) and where its user
/// name ends.
fn next_home(text: &str) -> Option<(usize, usize)> {
    let lower = text.to_ascii_lowercase();
    let found = ["/home/", "\\home\\", "/users/", "\\users\\"]
        .iter()
        .filter_map(|marker| lower.find(marker).map(|at| (at, marker.len())))
        .min()?;
    let (at, marker) = found;
    let bytes = text.as_bytes();
    // `C:` before `\Users\`
    let start = if at >= 2 && bytes[at - 1] == b':' && bytes[at - 2].is_ascii_alphabetic() {
        at - 2
    } else {
        at
    };
    let name = at + marker;
    // a Windows profile folder's name may have spaces in it; a Linux or Mac one may not
    let windows = start < at || text[at..name].contains('\\');
    let end = text[name..]
        .find(|c: char| {
            matches!(c, '/' | '\\' | '"' | '\'' | '`' | ')' | ']' | '\n' | '\r')
                || (c.is_whitespace() && !windows)
        })
        .map_or(text.len(), |n| name + n);
    if end == name {
        // `/home/` with no name after it: nobody's folder, kept as it is
        let skip = name;
        return next_home(&text[skip..]).map(|(s, e)| (s + skip, e + skip));
    }
    Some((start, end))
}

/// The build machine's paths to the libraries' and the compiler's sources, shortened: they
/// say nothing about the person, and make a backtrace hard to read.
fn libraries(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        out.push_str(&shorten(line));
    }
    out
}

fn shorten(line: &str) -> String {
    let mut line = line.to_owned();
    for (marker, keep) in [
        ("/.cargo/registry/src/", "<crates>/"),
        ("\\.cargo\\registry\\src\\", "<crates>\\"),
        ("/rustc/", "<rust>/"),
        ("\\rustc\\", "<rust>\\"),
    ] {
        while let Some(at) = line.find(marker) {
            // the path starts at the space or quote before it
            let start = line[..at]
                .rfind(|c: char| c.is_whitespace() || matches!(c, '"' | '(' | '`'))
                .map_or(0, |s| s + 1);
            let after = at + marker.len();
            // the registry's index folder, or the compiler's commit, goes as well
            let sep = if marker.starts_with('/') { '/' } else { '\\' };
            let end = line[after..]
                .find(sep)
                .map_or(line.len(), |n| after + n + 1);
            line.replace_range(start..end, keep);
        }
    }
    line
}

/// What the person's own files end in (jobs, toolpaths, drawings, models, pictures, fonts):
/// a crash report sent by itself names none of them.
const THEIR_FILES: &[&str] = &[
    "ccjob", "ajob", "nc", "gcode", "ngc", "tap", "cnc", "txt", "svg", "dxf", "eps", "ai", "pdf",
    "stl", "obj", "3mf", "step", "stp", "png", "jpg", "jpeg", "gif", "bmp", "tif", "tiff", "webp",
    "csv", "json", "html", "pp", "vtdb", "ttf", "otf",
];
/// Where a path is in the person's own folders, once [`scrub`] has put home as `~`: home, a
/// drive, a network share, a mounted disk.
const THEIR_FOLDERS: &[&str] = &[
    "~/",
    "~\\",
    "\\\\",
    "/media/",
    "/mnt/",
    "/run/media/",
    "/volumes/",
    "/tmp/",
    "/srv/",
];

/// `text` (already [`scrub`]bed) with the person's files and folders taken out, for a crash
/// report that goes without them seeing it first: a path in their folders reads
/// `<file>.ccjob` (or `<folder>`), and a file name of theirs anywhere `<file>.ccjob`. Source
/// paths (`crates/app/src/view2d.rs:9:9`) stay: they are the build machine's.
pub fn hide_files(text: &str) -> String {
    text.split_inclusive('\n')
        .map(|line| bare_names(&folders(line)))
        .collect()
}

/// `line` with each path in the person's folders put as `<file>.<ext>` or `<folder>`.
fn folders(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(start) = folder_start(rest) {
        out.push_str(&rest[..start]);
        let path = &rest[start..];
        // a file of theirs ends the path, even past a space ("My Jobs/sign file.ccjob");
        // else the path runs to the next space
        let stop = path
            .find(['\n', '\r', '\t', '"', '\'', '`', '<', '>', '|'])
            .unwrap_or(path.len());
        let (end, put) = if let Some((end, ext)) = their_file(&path[..stop]) {
            (end, format!("<file>.{ext}"))
        } else {
            let end = path[..stop].find(char::is_whitespace).unwrap_or(stop);
            let token = &path[..end];
            let source = token
                .rsplit_once('.')
                .is_some_and(|(_, ext)| ext == "rs" || ext.starts_with("rs:"));
            (
                end,
                if source {
                    token.to_owned()
                } else {
                    "<folder>".to_owned()
                },
            )
        };
        out.push_str(&put);
        rest = &path[end..];
    }
    out.push_str(rest);
    out
}

/// Where the first path in the person's folders starts in `text`.
fn folder_start(text: &str) -> Option<usize> {
    let lower = text.to_ascii_lowercase();
    let bytes = text.as_bytes();
    text.char_indices().map(|(i, _)| i).find(|&i| {
        let free = i == 0
            || !matches!(
                bytes[i - 1],
                b'/' | b'\\' | b'.' | b'_' | b'-' | b'~' | b':'
            ) && !bytes[i - 1].is_ascii_alphanumeric();
        let here = &lower[i..];
        let drive = here.len() >= 3
            && bytes[i].is_ascii_alphabetic()
            && bytes[i + 1] == b':'
            && matches!(bytes[i + 2], b'/' | b'\\');
        free && (drive || THEIR_FOLDERS.iter().any(|f| here.starts_with(f)))
    })
}

/// The end of the first name of one of the person's files in `text`, and its extension.
fn their_file(text: &str) -> Option<(usize, String)> {
    text.match_indices('.').find_map(|(dot, _)| {
        let after = &text[dot + 1..];
        let len = after
            .find(|c: char| !c.is_ascii_alphanumeric())
            .unwrap_or(after.len());
        let ext = after[..len].to_ascii_lowercase();
        (dot > 0 && THEIR_FILES.contains(&ext.as_str())).then(|| (dot + 1 + len, ext))
    })
}

/// `line` with each file name of the person's put as `<file>.<ext>`.
fn bare_names(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some((end, ext)) = their_file(rest) {
        let dot = end - ext.len() - 1;
        let start = rest[..dot]
            .rfind(|c: char| {
                c.is_whitespace()
                    || matches!(
                        c,
                        '"' | '\'' | '`' | '(' | '[' | '/' | '\\' | '>' | ':' | '='
                    )
            })
            .map_or(0, |at| {
                at + rest[at..].chars().next().map_or(1, char::len_utf8)
            });
        out.push_str(&rest[..start]);
        if start < dot {
            let _ = write!(out, "<file>.{ext}");
        } else {
            out.push_str(&rest[start..end]);
        }
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

/// A crash report's title, in plain words: what went wrong when it is a known kind of
/// failure, else what Cut Craft was doing, else the panic's own message without the source
/// path it was raised at.
pub fn crash_title(crash: &str) -> String {
    let lower = crash.to_ascii_lowercase();
    let panic = panic_message(crash);
    let doing = doing_at_crash(crash);
    let known = if lower.contains("egui_vertex_buffer")
        || (lower.contains("buffer") && lower.contains("max_buffer_size"))
    {
        Some("the picture ran out of graphics memory")
    } else if lower.contains("memory allocation") || lower.contains("out of memory") {
        Some("ran out of memory")
    } else if lower.contains("device lost") || lower.contains("devicelost") {
        Some("the graphics card stopped responding")
    } else if lower.contains("wgpu") && lower.contains("validation error") {
        Some("a graphics card error")
    } else if lower.contains("no space left on device") {
        Some("the disk is full")
    } else {
        None
    };
    let title = match (known, doing) {
        (Some(what), Some(doing)) => format!("{what} while {}", lower_first(&doing)),
        (Some(what), None) => what.to_owned(),
        (None, Some(doing)) => format!("while {}: {panic}", lower_first(&doing)),
        (None, None) => panic,
    };
    let title: String = title.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut short: String = title.chars().take(TITLE_CHARS).collect();
    if short.len() < title.len() {
        short.push('…');
    }
    format!("Crash: {}", scrub(&short))
}

/// The panic's message from a crash log, without `panicked at <file>:<line>:<col>:`.
fn panic_message(crash: &str) -> String {
    let panic = crash
        .split_once("crashed: ")
        .map_or(crash, |(_, rest)| rest);
    let panic = panic.split("\n\n").next().unwrap_or_default();
    let message = match panic.strip_prefix("panicked at ") {
        // the location runs to the first line's end
        Some(rest) => rest.split_once('\n').map_or(rest, |(_, msg)| msg),
        None => panic,
    };
    let message = message.trim();
    if message.is_empty() {
        "Cut Craft closed with an error".to_owned()
    } else {
        message.to_owned()
    }
}

/// What the crash log says Cut Craft was doing when it crashed (the latest, when several).
pub fn doing_at_crash(crash: &str) -> Option<String> {
    let (_, rest) = crash.split_once(crate::logs::DOING_HEADING)?;
    rest.lines()
        .map(str::trim)
        .take_while(|l| !l.is_empty())
        .filter_map(|l| l.strip_prefix("- "))
        .last()
        // the step it was on is for the log, not the title
        .map(|l| {
            l.split(crate::logs::STAGE_MARK)
                .next()
                .unwrap_or(l)
                .to_owned()
        })
}

fn lower_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        // a name such as "V-Carve" keeps its capital; "Calculating" does not need one
        Some(first) if chars.clone().next().is_some_and(char::is_lowercase) => {
            first.to_lowercase().chain(chars).collect()
        }
        _ => text.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_folders_lose_the_user_name_on_every_platform() {
        assert_eq!(
            scrub("Opened /home/sam/Jobs/sign.ccjob"),
            "Opened ~/Jobs/sign.ccjob"
        );
        assert_eq!(
            scrub("Saved C:\\Users\\Sam Smith\\Desktop\\a.nc"),
            "Saved ~\\Desktop\\a.nc",
            "a Windows name may have a space"
        );
        assert_eq!(
            scrub_with(
                "Opened /var/lib/sam-home/a.ccjob",
                Some("/var/lib/sam-home")
            ),
            "Opened ~/a.ccjob",
            "this computer's home, wherever it is"
        );
        assert_eq!(
            scrub("Saved c:/users/sam/a.nc and /Users/jo/b.nc"),
            "Saved ~/a.nc and ~/b.nc"
        );
        assert_eq!(scrub("config in /home/sam"), "config in ~");
        assert_eq!(scrub("nothing private here"), "nothing private here");
    }

    #[test]
    fn the_build_machines_library_paths_are_shortened() {
        let line = "   at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/wgpu-27.0.1/src/backend/wgpu_core.rs:1:1";
        assert_eq!(
            scrub(line),
            "   at <crates>/wgpu-27.0.1/src/backend/wgpu_core.rs:1:1"
        );
        assert_eq!(
            scrub(
                "at /rustc/9b00956e56009bab2aa15d7bff10916599e3d6d6/library/core/src/panicking.rs:75"
            ),
            "at <rust>/library/core/src/panicking.rs:75"
        );
    }

    #[test]
    fn a_known_crash_is_titled_in_plain_words_with_what_was_running() {
        let crash = "Cut Craft 0.6.3 on linux crashed: panicked at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/wgpu-27.0.1/src/backend/wgpu_core.rs:1:1:\nwgpu error: Validation Error\n\nCaused by:\n  In Device::create_buffer, label = 'egui_vertex_buffer'\n\n\
            Doing when it crashed:\n- Previewing 3D Finishing 1 — now: Cutting\n\nBacktrace:\n";
        assert_eq!(
            crash_title(crash),
            "Crash: the picture ran out of graphics memory while previewing 3D Finishing 1"
        );
    }

    #[test]
    fn an_unknown_crash_keeps_its_message_but_not_its_source_path() {
        let crash = "Cut Craft 0.3.0 on linux crashed: panicked at crates/app/src/view3d.rs:9:9:\nindex out of bounds\n\nBacktrace:\n...";
        assert_eq!(crash_title(crash), "Crash: index out of bounds");
        let doing =
            format!("{crash}\n\nDoing when it crashed:\n- Calculating Pocket Toolpath\n\nLog:\n");
        assert_eq!(
            crash_title(&doing),
            "Crash: while calculating Pocket Toolpath: index out of bounds"
        );
        let long = format!(
            "Cut Craft crashed: panicked at a.rs:1:1:\n{}",
            "x ".repeat(200)
        );
        assert!(crash_title(&long).ends_with('…'));
        assert!(crash_title(&long).chars().count() <= "Crash: ".len() + TITLE_CHARS + 1);
    }

    #[test]
    fn a_report_sent_by_itself_names_none_of_the_persons_files() {
        assert_eq!(
            hide_files("opened ~/Jobs/Smith kitchen/sign file.ccjob at 12:00\n"),
            "opened <file>.ccjob at 12:00\n"
        );
        assert_eq!(
            hide_files("saved D:\\Work\\a.nc, then ~\\Desktop\\b.SVG"),
            "saved <file>.nc, then <file>.svg"
        );
        assert_eq!(
            hide_files("couldn't read /media/usb/Client Co/logo.png: denied"),
            "couldn't read <file>.png: denied"
        );
        assert_eq!(
            hide_files("config in ~/.config/cut-craft x"),
            "config in <folder> x"
        );
        assert_eq!(
            hide_files("Exported sign.nc (\"lid.stl\")"),
            "Exported <file>.nc (\"<file>.stl\")"
        );
        let source = "panicked at crates/app/src/view2d.rs:9:9:\n   at ~/src/a.rs:1:1\n";
        assert_eq!(
            hide_files(source),
            source,
            "source paths are the build machine's"
        );
        assert_eq!(
            hide_files("Cut Craft 0.6.3 on linux crashed: version 0.6.3"),
            "Cut Craft 0.6.3 on linux crashed: version 0.6.3"
        );
        assert_eq!(
            hide_files("see https://x.dev/a and C:"),
            "see https://x.dev/a and C:"
        );
    }
}
