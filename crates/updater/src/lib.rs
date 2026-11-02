//! Help ▸ Check for Updates: finds a newer Cut Craft on GitHub, and with the user's go-ahead
//! downloads it, checks it against GitHub's checksum and swaps it in for the running
//! program, ready for a restart. A copy built from a clone of the code fetches the newest
//! code and builds it instead. Only GitHub is asked, never a mirror.

mod install;
mod package;
mod release;

pub use install::{Install, Probe};
pub use package::clean_up;
pub use release::{Asset, Release, version_of};
pub use semver::Version;

use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The newest release that is not a test build.
pub const LATEST: &str = "https://api.github.com/repos/NoobyNull/Cut-Craft/releases/latest";
/// Where releases are listed, for downloading by hand.
pub const RELEASES_PAGE: &str = "https://github.com/NoobyNull/Cut-Craft/releases";
/// How long after one check the next start checks again.
pub const CHECK_EVERY_SECONDS: u64 = 20 * 60 * 60;
/// A download larger than this is not Cut Craft.
const MAX_DOWNLOAD: u64 = 500 * 1024 * 1024;

/// This copy of Cut Craft, as the running program sees itself.
pub fn this_install() -> Install {
    let exe = std::env::current_exe().unwrap_or_default();
    // the real file, not a link to it in ~/.local/bin or the like
    let exe = std::fs::canonicalize(&exe).unwrap_or(exe);
    Install::detect(&Probe {
        exe,
        appimage: std::env::var_os("APPIMAGE").map(PathBuf::from),
        flatpak: Path::new("/.flatpak-info").exists(),
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        exists: &|p| p.exists(),
    })
}

/// The running version.
pub fn current_version() -> Version {
    Version::parse(env!("CARGO_PKG_VERSION")).expect("the package version is a version")
}

/// Whether the startup check is due: never checked, or checked over
/// [`CHECK_EVERY_SECONDS`] ago (or, by a clock put back, in the future).
pub fn due(last_check: Option<u64>, now: u64) -> bool {
    last_check.is_none_or(|last| now < last || now - last >= CHECK_EVERY_SECONDS)
}

/// What a check found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Found {
    UpToDate,
    /// A newer release; `ready` says whether it can be installed from here, or why not.
    Release {
        release: Release,
        ready: Result<(), String>,
    },
}

impl Found {
    /// What a newer release can do here: install itself, or why the user must update by hand.
    pub fn for_release(install: &Install, release: Release) -> Self {
        let ready = match install {
            Install::Elsewhere { why } => Err(why.clone()),
            _ => match install.asset_suffix().and_then(|s| release.asset(s)) {
                None => Err(format!(
                    "Version {} has no download for this kind of install. Download it from \
                     the release page instead.",
                    release.version
                )),
                Some(a) if a.digest.is_none() => Err(format!(
                    "GitHub has no checksum for version {}'s download, so Cut Craft cannot \
                     check it arrives whole. Download it from the release page instead.",
                    release.version
                )),
                Some(_) => Ok(()),
            },
        };
        Self::Release { release, ready }
    }
}

/// Asks GitHub whether there is something newer than `current` for `install`.
pub fn check(install: &Install, current: &Version) -> Result<Found, String> {
    let release = Release::parse(&get_text(LATEST)?)?;
    Ok(if release.is_newer_than(current) {
        Found::for_release(install, release)
    } else {
        Found::UpToDate
    })
}

/// How far an update has got.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Downloading { done: u64, total: u64 },
    Checking,
    Installing,
}

/// Installs what `found` found; the program to start for the new version.
pub fn install(install: &Install, found: &Found, step: &dyn Fn(Step)) -> Result<PathBuf, String> {
    match (install, found) {
        (
            _,
            Found::Release {
                release,
                ready: Ok(()),
            },
        ) => {
            let target = install
                .program()
                .ok_or("this copy of Cut Craft cannot update itself")?;
            let asset = install
                .asset_suffix()
                .and_then(|s| release.asset(s))
                .ok_or("the release has no download for this computer")?;
            let bytes = download(asset, &|done| {
                step(Step::Downloading {
                    done,
                    total: asset.size,
                });
            })?;
            step(Step::Checking);
            package::verify(&bytes, asset.digest.as_deref())?;
            let program = match install {
                Install::AppImage { .. } => bytes,
                Install::Archive { windows, .. } => {
                    let name = if *windows {
                        "cut-craft.exe"
                    } else {
                        "cut-craft"
                    };
                    package::program_from(&bytes, *windows, name)?
                }
                Install::Elsewhere { .. } => unreachable!("only downloads have an asset"),
            };
            step(Step::Installing);
            package::replace(target, &program)?;
            log::info!("updated {} to {}", target.display(), release.version);
            Ok(target.to_path_buf())
        }
        (
            _,
            Found::Release {
                ready: Err(why), ..
            },
        ) => Err(why.clone()),
        (_, Found::UpToDate) => Err("Cut Craft is already up to date".to_owned()),
    }
}

/// Starts `program`, the new version, apart from this one (which is closing).
pub fn restart(program: &Path) -> std::io::Result<()> {
    let mut command = std::process::Command::new(program);
    // not the old program's folder, which an AppImage unmounts as it closes
    if let Some(dir) = home() {
        command.current_dir(dir);
    }
    command.spawn().map(drop)
}

fn home() -> Option<PathBuf> {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from)
}

fn agent(timeout: Option<Duration>) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_global(timeout)
        .user_agent(concat!("Cut-Craft/", env!("CARGO_PKG_VERSION")))
        // the computer's own certificates, so antivirus and office proxies that check
        // HTTPS are trusted as they are by the browser
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                .build(),
        )
        .build()
        .into()
}

fn offline(e: &ureq::Error) -> String {
    match e {
        ureq::Error::StatusCode(403 | 429) => {
            "GitHub is not answering this computer for a while (too many checks). Try again \
             in an hour."
                .to_owned()
        }
        ureq::Error::StatusCode(404) => "GitHub has no releases of Cut Craft yet".to_owned(),
        ureq::Error::StatusCode(code) => format!("GitHub answered with error {code}"),
        _ => format!("Could not reach GitHub. Check this computer is online. ({e})"),
    }
}

fn get_text(url: &str) -> Result<String, String> {
    agent(Some(Duration::from_secs(30)))
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| offline(&e))?
        .body_mut()
        .read_to_string()
        .map_err(|e| offline(&e))
}

/// Downloads `asset`, telling `progress` the bytes so far.
fn download(asset: &Asset, progress: &dyn Fn(u64)) -> Result<Vec<u8>, String> {
    let mut response = agent(None)
        .get(&asset.url)
        .call()
        .map_err(|e| offline(&e))?;
    let mut reader = response
        .body_mut()
        .with_config()
        .limit(MAX_DOWNLOAD)
        .reader();
    let mut bytes = Vec::with_capacity(usize::try_from(asset.size).unwrap_or(0));
    let mut chunk = vec![0; 256 * 1024];
    loop {
        let n = reader
            .read(&mut chunk)
            .map_err(|e| format!("the download stopped part way ({e}). Try again."))?;
        if n == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..n]);
        progress(bytes.len() as u64);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(assets: &[(&str, Option<&str>)]) -> Release {
        Release {
            tag: "v0.5.0".into(),
            version: Version::new(0, 5, 0),
            notes: String::new(),
            page: String::new(),
            assets: assets
                .iter()
                .map(|(name, digest)| Asset {
                    name: (*name).into(),
                    url: String::new(),
                    size: 1,
                    digest: digest.map(Into::into),
                })
                .collect(),
        }
    }

    const LINUX: Install = Install::Archive {
        exe: PathBuf::new(),
        windows: false,
    };

    #[test]
    fn a_release_installs_only_with_a_checked_download() {
        let ok = release(&[("cut-craft-v0.5.0-linux-x86_64.tar.gz", Some("sha256:00"))]);
        let Found::Release { ready, .. } = Found::for_release(&LINUX, ok) else {
            panic!()
        };
        assert_eq!(ready, Ok(()));
        let unchecked = release(&[("cut-craft-v0.5.0-linux-x86_64.tar.gz", None)]);
        let Found::Release { ready, .. } = Found::for_release(&LINUX, unchecked) else {
            panic!()
        };
        assert!(ready.unwrap_err().contains("no checksum"));
        let windows_only = release(&[("cut-craft-v0.5.0-windows-x86_64.zip", Some("sha256:00"))]);
        let Found::Release { ready, .. } = Found::for_release(&LINUX, windows_only) else {
            panic!()
        };
        assert!(ready.unwrap_err().contains("no download"));
        let flatpak = Install::Elsewhere {
            why: "use the .flatpak".into(),
        };
        let Found::Release { ready, .. } = Found::for_release(&flatpak, release(&[])) else {
            panic!()
        };
        assert_eq!(ready, Err("use the .flatpak".into()));
    }

    #[test]
    fn checks_at_most_about_once_a_day() {
        assert!(due(None, 1_000));
        assert!(!due(Some(1_000), 1_000 + 3_600));
        assert!(due(Some(1_000), 1_000 + CHECK_EVERY_SECONDS));
        // the clock was put back
        assert!(due(Some(1_000), 10));
    }

    #[test]
    fn the_running_version_is_the_package_version() {
        assert_eq!(current_version().to_string(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn nothing_to_install_says_why() {
        assert!(install(&LINUX, &Found::UpToDate, &|_| {}).is_err());
        let blocked = Found::Release {
            release: release(&[]),
            ready: Err("by hand".into()),
        };
        assert_eq!(install(&LINUX, &blocked, &|_| {}), Err("by hand".into()));
    }

    /// Asks the real GitHub: `cargo test -p cut-craft-updater -- --ignored`.
    #[test]
    #[ignore = "needs the network"]
    fn asks_github() {
        let old = Version::new(0, 0, 1);
        let Ok(Found::Release { release, ready }) = check(&LINUX, &old) else {
            panic!("GitHub has a release newer than 0.0.1")
        };
        assert!(release.version > old);
        assert_eq!(ready, Ok(()), "{release:?}");
        assert_eq!(check(&LINUX, &Version::new(999, 0, 0)), Ok(Found::UpToDate));
    }

    /// Downloads the real newest Linux release and installs it over a stand-in program.
    #[test]
    #[ignore = "needs the network; downloads about 15 MB"]
    fn installs_from_github() {
        let dir = std::env::temp_dir().join(format!("cut-craft-update-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("cut-craft");
        std::fs::write(&exe, b"old").unwrap();
        let here = Install::Archive {
            exe: exe.clone(),
            windows: false,
        };
        let found = check(&here, &Version::new(0, 0, 1)).unwrap();
        let steps = std::cell::RefCell::new(vec![]);
        let started = install(&here, &found, &|s| steps.borrow_mut().push(s)).unwrap();
        assert_eq!(started, exe);
        assert!(std::fs::read(&exe).unwrap().starts_with(b"\x7fELF"));
        let steps = steps.into_inner();
        assert!(matches!(steps[0], Step::Downloading { .. }));
        assert_eq!(steps[steps.len() - 2..], [Step::Checking, Step::Installing]);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
