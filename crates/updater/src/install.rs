//! How this copy of Cut Craft was installed, which decides how it is updated.

use std::path::{Path, PathBuf};

/// What the running program can tell about where it came from.
pub struct Probe<'a> {
    /// The running program (`std::env::current_exe`).
    pub exe: PathBuf,
    /// `APPIMAGE`: the `AppImage` file, when running from one.
    pub appimage: Option<PathBuf>,
    /// Running in a Flatpak sandbox (`/.flatpak-info` exists).
    pub flatpak: bool,
    /// `std::env::consts::OS` and `ARCH`.
    pub os: &'a str,
    pub arch: &'a str,
    /// Whether a file or folder exists.
    pub exists: &'a dyn Fn(&Path) -> bool,
}

/// How this copy is updated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Install {
    /// A program unpacked from a release's `.tar.gz` (Linux) or `.zip` (Windows): the new
    /// program replaces it where it is.
    Archive { exe: PathBuf, windows: bool },
    /// A Linux `AppImage`: the new `AppImage` replaces this file.
    AppImage { file: PathBuf },
    /// Updated some other way (a Flatpak, a Mac, a build from the source code); the reason
    /// says how, for the user.
    Elsewhere { why: String },
}

/// Downloads are built for 64-bit Intel and AMD processors only.
const ARCH: &str = "x86_64";

impl Install {
    pub fn detect(p: &Probe) -> Self {
        if p.flatpak {
            return Self::Elsewhere {
                why: "This copy of Cut Craft is a Flatpak. Download the new .flatpak file from \
                      the release page and open it to install it over this one."
                    .to_owned(),
            };
        }
        if let Some(source) = source_checkout(&p.exe, p.exists) {
            return source;
        }
        let downloadable = p.arch == ARCH;
        match (p.os, &p.appimage) {
            ("linux", Some(file)) if downloadable => Self::AppImage { file: file.clone() },
            ("linux" | "windows", _) if downloadable => Self::Archive {
                exe: p.exe.clone(),
                windows: p.os == "windows",
            },
            ("linux" | "windows", _) => Self::Elsewhere {
                why: format!(
                    "Cut Craft's downloads are for 64-bit Intel and AMD processors, and this \
                     computer has a {} processor. Build the new version from its source code.",
                    p.arch
                ),
            },
            ("macos", _) => Self::Elsewhere {
                why: "On a Mac, download the new .dmg from the release page and drag Cut Craft \
                      to Applications again."
                    .to_owned(),
            },
            (os, _) => Self::Elsewhere {
                why: format!("Cut Craft cannot update itself on {os}."),
            },
        }
    }

    /// The end of the name of the release download this copy is updated from.
    pub fn asset_suffix(&self) -> Option<&'static str> {
        match self {
            Self::Archive { windows: true, .. } => Some("windows-x86_64.zip"),
            Self::Archive { windows: false, .. } => Some("linux-x86_64.tar.gz"),
            Self::AppImage { .. } => Some("x86_64.AppImage"),
            Self::Elsewhere { .. } => None,
        }
    }

    /// The program to start after updating.
    pub fn program(&self) -> Option<&Path> {
        match self {
            Self::Archive { exe, .. } => Some(exe),
            Self::AppImage { file } => Some(file),
            Self::Elsewhere { .. } => None,
        }
    }
}

/// `exe` is `<clone>/target/<release|debug>/cut-craft` of a clone of the code: replacing it
/// would be undone by the next build, so the clone is updated by hand.
fn source_checkout(exe: &Path, exists: &dyn Fn(&Path) -> bool) -> Option<Install> {
    let profile_dir = exe.parent()?;
    let profile = profile_dir.file_name()?.to_str()?;
    let target = profile_dir.parent()?;
    if target.file_name()? != "target" || !matches!(profile, "release" | "debug") {
        return None;
    }
    let root = target.parent()?;
    (exists(&root.join("Cargo.toml")) && exists(&root.join(".git"))).then(|| Install::Elsewhere {
        why: format!(
            "This copy of Cut Craft was built from the source code in {}. Update it there with \
             `git pull` and `cargo build --release`, or install a download from the release \
             page instead.",
            root.display()
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn probe<'a>(exe: &str, os: &'a str, exists: &'a dyn Fn(&Path) -> bool) -> Probe<'a> {
        Probe {
            exe: exe.into(),
            appimage: None,
            flatpak: false,
            os,
            arch: "x86_64",
            exists,
        }
    }

    const NOTHING: &dyn Fn(&Path) -> bool = &|_| false;
    const EVERYTHING: &dyn Fn(&Path) -> bool = &|_| true;

    #[test]
    fn an_unpacked_download_is_replaced_in_place() {
        let linux = Install::detect(&probe("/home/a/cut-craft/cut-craft", "linux", NOTHING));
        assert_eq!(
            linux,
            Install::Archive {
                exe: "/home/a/cut-craft/cut-craft".into(),
                windows: false
            }
        );
        assert_eq!(linux.asset_suffix(), Some("linux-x86_64.tar.gz"));
        let windows = Install::detect(&probe(r"C:\Tools\cut-craft.exe", "windows", NOTHING));
        assert_eq!(windows.asset_suffix(), Some("windows-x86_64.zip"));
    }

    #[test]
    fn an_appimage_replaces_the_appimage_not_the_program_inside() {
        let mut p = probe("/tmp/.mount_CutCr/usr/bin/cut-craft", "linux", NOTHING);
        p.appimage = Some("/home/a/Apps/CutCraft.AppImage".into());
        let found = Install::detect(&p);
        assert_eq!(
            found.program(),
            Some(Path::new("/home/a/Apps/CutCraft.AppImage"))
        );
        assert_eq!(found.asset_suffix(), Some("x86_64.AppImage"));
    }

    #[test]
    fn a_build_from_a_clone_is_updated_by_hand() {
        let exists = |p: &Path| {
            p == Path::new("/home/m/cut-craft/Cargo.toml")
                || p == Path::new("/home/m/cut-craft/.git")
        };
        let found = Install::detect(&probe(
            "/home/m/cut-craft/target/release/cut-craft",
            "linux",
            &exists,
        ));
        let Install::Elsewhere { why } = &found else {
            panic!("a build is not replaced: {found:?}")
        };
        assert!(
            why.contains("/home/m/cut-craft") && why.contains("git pull"),
            "{why}"
        );
        assert_eq!(found.asset_suffix(), None);
        // a target folder with no clone round it is just a folder of that name
        let lone = Install::detect(&probe("/opt/target/release/cut-craft", "linux", NOTHING));
        assert!(matches!(lone, Install::Archive { .. }));
        let debug = Install::detect(&probe("/c/target/debug/cut-craft", "linux", EVERYTHING));
        assert!(matches!(debug, Install::Elsewhere { .. }));
    }

    #[test]
    fn flatpak_mac_and_other_processors_say_how_to_update() {
        let mut p = probe("/app/bin/cut-craft", "linux", NOTHING);
        p.flatpak = true;
        let Install::Elsewhere { why } = Install::detect(&p) else {
            panic!("a Flatpak cannot replace itself")
        };
        assert!(why.contains(".flatpak"));
        let mac = Install::detect(&probe("/Applications/x", "macos", NOTHING));
        assert!(matches!(mac, Install::Elsewhere { .. }));
        let mut arm = probe("/home/a/cut-craft", "linux", NOTHING);
        arm.arch = "aarch64";
        let Install::Elsewhere { why } = Install::detect(&arm) else {
            panic!("no ARM downloads")
        };
        assert!(why.contains("aarch64"));
    }
}
