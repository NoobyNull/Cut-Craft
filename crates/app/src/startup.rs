//! What happens before the window: the command line (`--version`, `--help`, `--screenshots`),
//! and a message box when the window can't open, so a start from a desktop launcher never just
//! does nothing. On Linux that is most often a system library the window needs that the
//! computer doesn't have; the message names the package to install for the common systems.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

/// What the command line asks for.
#[derive(Debug, PartialEq)]
pub enum Start {
    /// `--version`: print the version and stop (the `AppImage` check on GitHub runs this).
    Version,
    /// `--help`: print what the command line takes and stop.
    Help,
    /// `--screenshots [DIR]`: the README's pictures (see app/screenshots.rs).
    Screenshots(PathBuf),
    /// Anything else: the app.
    App,
}

/// What the arguments after the program's name ask for.
pub fn parse(mut args: impl Iterator<Item = String>) -> Start {
    match args.next().as_deref() {
        Some("--version" | "-V") => Start::Version,
        Some("--help" | "-h") => Start::Help,
        Some("--screenshots") => Start::Screenshots(
            args.next()
                .map_or_else(|| "docs/screenshots".into(), Into::into),
        ),
        _ => Start::App,
    }
}

/// What `--version` prints.
pub fn version() -> String {
    format!("Cut Craft {}", env!("CARGO_PKG_VERSION"))
}

/// What `--help` prints.
pub fn help() -> String {
    format!(
        "{}\n\nUsage: cut-craft [OPTION]\n\n  \
         (nothing)            open Cut Craft\n  \
         -V, --version        print the version and stop\n  \
         -h, --help           print this and stop\n  \
         --screenshots [DIR]  save the README's pictures in DIR (docs/screenshots)\n",
        version()
    )
}

/// `text` on standard output, ignoring a reader that has gone.
pub fn print(text: &str) {
    use std::io::Write;
    let _ = std::io::stdout().write_all(text.as_bytes());
}

/// Set once the app is set up in its window: from then on a crash is the crash log's to
/// report (offered at the next start). Before that, the next start would most likely crash
/// the same way before it could offer anything.
static WINDOW_OPEN: AtomicBool = AtomicBool::new(false);

/// The window is up and the app set up in it.
pub fn window_opened() {
    WINDOW_OPEN.store(true, Ordering::Relaxed);
}

/// Until the window opens, a panic also explains itself in a message box (and on the
/// terminal), after the hook already installed (the crash log) has run. Release builds abort
/// on a panic, so this is the last thing that happens.
pub fn explain_failures() {
    let earlier = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        earlier(info);
        if WINDOW_OPEN.load(Ordering::Relaxed) {
            return;
        }
        let text = failure_message(info.payload_as_str().unwrap_or_default());
        eprintln!("\n{text}");
        // on Linux rfd asks zenity or kdialog; with neither, the terminal line above is it
        let _ = rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title("Cut Craft can't start")
            .set_description(&text)
            .set_buttons(rfd::MessageButtons::Ok)
            .show();
    }));
}

/// A system library and the package that has it on the common Linux families.
#[derive(Debug, PartialEq)]
pub struct Package {
    /// The library's name without `.so` and its version, e.g. `libxkbcommon-x11`.
    pub library: &'static str,
    /// Ubuntu, Debian, Mint and the like (apt).
    pub debian: &'static str,
    /// Fedora and the like (dnf).
    pub fedora: &'static str,
    /// Arch, `CachyOS`, Manjaro and the like (pacman).
    pub arch: &'static str,
}

/// The libraries the window loads when it opens (winit's X11 and Wayland, keyboard included).
pub const PACKAGES: &[Package] = &[
    Package {
        library: "libxkbcommon-x11",
        debian: "libxkbcommon-x11-0",
        fedora: "libxkbcommon-x11",
        arch: "libxkbcommon-x11",
    },
    Package {
        library: "libxkbcommon",
        debian: "libxkbcommon0",
        fedora: "libxkbcommon",
        arch: "libxkbcommon",
    },
    Package {
        library: "libxcb-xkb",
        debian: "libxcb-xkb1",
        fedora: "libxcb",
        arch: "libxcb",
    },
    Package {
        library: "libX11-xcb",
        debian: "libx11-xcb1",
        fedora: "libX11-xcb",
        arch: "libx11",
    },
    Package {
        library: "libX11",
        debian: "libx11-6",
        fedora: "libX11",
        arch: "libx11",
    },
    Package {
        library: "libXcursor",
        debian: "libxcursor1",
        fedora: "libXcursor",
        arch: "libxcursor",
    },
    Package {
        library: "libXi",
        debian: "libxi6",
        fedora: "libXi",
        arch: "libxi",
    },
    Package {
        library: "libXrandr",
        debian: "libxrandr2",
        fedora: "libXrandr",
        arch: "libxrandr",
    },
    Package {
        library: "libwayland-client",
        debian: "libwayland-client0",
        fedora: "libwayland-client",
        arch: "wayland",
    },
    Package {
        library: "libvulkan",
        debian: "libvulkan1",
        fedora: "vulkan-loader",
        arch: "vulkan-icd-loader",
    },
];

/// The shared library a failure names, such as `libxkbcommon-x11.so` in xkbcommon-dl's
/// "Library libxkbcommon-x11.so could not be loaded." or `libXcursor.so.1` in
/// "libXcursor.so.1: cannot open shared object file".
pub fn library_in(failure: &str) -> Option<&str> {
    failure
        .split(|c: char| c.is_whitespace() || matches!(c, '(' | ')' | '"' | '\'' | '`' | ','))
        .map(|word| word.trim_end_matches(['.', ':', ';']))
        .find(|word| {
            word.starts_with("lib")
                && word.find(".so").is_some_and(|at| {
                    at > 3
                        && word[at + 3..]
                            .chars()
                            .all(|c| c == '.' || c.is_ascii_digit())
                })
        })
}

/// The package that has `library` (`libxkbcommon-x11.so.0`, or without its version).
pub fn package_for(library: &str) -> Option<&'static Package> {
    let name = library.split(".so").next().unwrap_or(library);
    PACKAGES.iter().find(|p| p.library == name)
}

/// What the message box says when the window couldn't open: how to fix a missing library when
/// the failure names one, otherwise what the system said.
pub fn failure_message(failure: &str) -> String {
    match library_in(failure) {
        Some(library) => {
            let fix = match package_for(library) {
                Some(p) => format!(
                    "Install it, then start Cut Craft again:\n\n  \
                     Ubuntu, Debian, Mint:     sudo apt install {}\n  \
                     Fedora:                   sudo dnf install {}\n  \
                     Arch, CachyOS, Manjaro:   sudo pacman -S {}",
                    p.debian, p.fedora, p.arch
                ),
                None => format!(
                    "Install the package that has {library} with your system's software \
                     manager or package manager, then start Cut Craft again."
                ),
            };
            format!(
                "Cut Craft can't open its window: this computer is missing {library}, a \
                 system library the window needs.\n\n{fix}"
            )
        }
        None if failure.contains("Wgpu(") => format!(
            "Cut Craft couldn't open its window: it draws with the graphics card, through \
             Vulkan or OpenGL, and couldn't use it.\n\nInstall or update the graphics driver \
             (on Linux, Mesa for AMD and Intel, or NVIDIA's driver), then start Cut Craft \
             again.\n\nWhat the system said:\n{}",
            failure.trim()
        ),
        None => format!(
            "Cut Craft couldn't open its window.\n\nWhat the system said:\n{}\n\nIf this \
             keeps happening, please report it at github.com/NoobyNull/Cut-Craft/issues \
             with the text above.",
            failure.trim()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> impl Iterator<Item = String> {
        list.iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .into_iter()
    }

    #[test]
    fn version_and_help_stop_before_the_window() {
        assert_eq!(parse(args(&["--version"])), Start::Version);
        assert_eq!(parse(args(&["-V"])), Start::Version);
        assert_eq!(parse(args(&["--help"])), Start::Help);
        assert_eq!(parse(args(&["-h"])), Start::Help);
    }

    #[test]
    fn screenshots_keep_their_folder_and_default() {
        assert_eq!(
            parse(args(&["--screenshots", "out"])),
            Start::Screenshots("out".into())
        );
        assert_eq!(
            parse(args(&["--screenshots"])),
            Start::Screenshots("docs/screenshots".into())
        );
    }

    #[test]
    fn nothing_or_anything_else_opens_the_app() {
        assert_eq!(parse(args(&[])), Start::App);
        assert_eq!(parse(args(&["drawing.ccjob"])), Start::App);
    }

    #[test]
    fn version_prints_the_name_and_number() {
        assert_eq!(
            version(),
            format!("Cut Craft {}", env!("CARGO_PKG_VERSION"))
        );
        assert!(help().contains("--version"));
    }

    #[test]
    fn finds_the_library_in_the_failures_seen() {
        assert_eq!(
            library_in("Library libxkbcommon-x11.so could not be loaded."),
            Some("libxkbcommon-x11.so")
        );
        assert_eq!(
            library_in(
                "event loop error: Os(OsError { error: XNotSupported(LibraryOpenError(OpenError \
                 { kind: Library, detail: \"opening library failed (libXcursor.so.1: cannot \
                 open shared object file: No such file or directory); \" })) })"
            ),
            Some("libXcursor.so.1")
        );
        assert_eq!(library_in("event loop error: no adapter"), None);
        // words that only look like a library
        assert_eq!(library_in("libso, lib.so, libfoo.sol"), None);
    }

    #[test]
    fn names_the_package_with_or_without_the_version() {
        let p = package_for("libxkbcommon-x11.so.0").unwrap();
        assert_eq!(
            (p.debian, p.fedora, p.arch),
            ("libxkbcommon-x11-0", "libxkbcommon-x11", "libxkbcommon-x11")
        );
        assert_eq!(
            package_for("libxkbcommon.so").unwrap().debian,
            "libxkbcommon0"
        );
        assert!(package_for("libfoo.so.3").is_none());
    }

    #[test]
    fn a_missing_library_message_says_how_to_install_it() {
        let text = failure_message("Library libxkbcommon-x11.so could not be loaded.");
        assert!(text.contains("missing libxkbcommon-x11.so"), "{text}");
        assert!(
            text.contains("sudo apt install libxkbcommon-x11-0"),
            "{text}"
        );
        assert!(text.contains("sudo dnf install libxkbcommon-x11"), "{text}");
        assert!(text.contains("sudo pacman -S libxkbcommon-x11"), "{text}");
    }

    #[test]
    fn an_unknown_library_is_still_named() {
        let text = failure_message("libfoo.so.3: cannot open shared object file");
        assert!(text.contains("missing libfoo.so.3"), "{text}");
        assert!(text.contains("package that has libfoo.so.3"), "{text}");
    }

    #[test]
    fn any_other_failure_shows_what_the_system_said() {
        let text = failure_message("event loop error: neither WAYLAND_DISPLAY nor DISPLAY is set");
        assert!(
            text.starts_with("Cut Craft couldn't open its window."),
            "{text}"
        );
        assert!(
            text.contains("neither WAYLAND_DISPLAY nor DISPLAY is set"),
            "{text}"
        );
    }

    #[test]
    fn no_graphics_driver_says_to_install_one() {
        let text = failure_message(
            "event loop error: Wgpu(CreateSurfaceError(CreateSurfaceError { inner: \
             Hal(FailedToCreateSurfaceForAnyBackend({})) }))",
        );
        assert!(text.contains("graphics driver"), "{text}");
        assert!(
            text.contains("FailedToCreateSurfaceForAnyBackend"),
            "{text}"
        );
    }

    #[test]
    fn every_library_has_all_three_packages() {
        for p in PACKAGES {
            assert!(p.library.starts_with("lib"), "{p:?}");
            assert!(
                !p.debian.is_empty() && !p.fedora.is_empty() && !p.arch.is_empty(),
                "{p:?}"
            );
        }
    }
}
