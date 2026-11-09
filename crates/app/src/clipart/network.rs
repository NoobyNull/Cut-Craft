//! Library folders on network shares. A Linux desktop opens a share in a FUSE mount whose
//! path doesn't last: KDE's kio-fuse makes `$XDG_RUNTIME_DIR/kio-fuse-<random>/smb/host/...`
//! afresh each session, and GNOME's gvfs `$XDG_RUNTIME_DIR/gvfs/smb-share:server=host,...`
//! only while the share is open. So the Library keeps such a folder by its network
//! location (`smb://host/share/folder`) and finds where it is mounted now each time.
//!
//! Elsewhere a path is kept as it is: a CIFS share mounted from fstab, a Windows UNC path
//! (`\\host\share`) and a macOS `/Volumes/share` path are the same every time.
// ponytail: macOS mounts a share again at /Volumes/share-1 when /Volumes/share is taken;
// finding it then needs `mount`'s list, as /proc/mounts is read on Linux

use std::path::{Path, PathBuf};

/// How long mounting a share may take before the Library gives up on it for now.
#[cfg(all(unix, not(target_os = "macos")))]
const MOUNTING: std::time::Duration = std::time::Duration::from_secs(20);

/// Whether `path` is a network location (`smb://...`), not a path on this computer.
pub fn is_location(path: &Path) -> bool {
    path.to_str().is_some_and(|p| scheme(p).is_some())
}

/// `url`'s scheme (`smb`), if it is a URL.
fn scheme(url: &str) -> Option<&str> {
    let (scheme, _) = url.split_once("://")?;
    let ok = scheme.len() > 1
        && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && (scheme.chars()).all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c));
    ok.then_some(scheme)
}

/// The network location behind `path` when it is in a FUSE mount whose path doesn't last
/// (kio-fuse or gvfs), else `None`.
pub fn location(path: &Path) -> Option<String> {
    let parts: Vec<&str> = (path.components())
        .filter_map(|c| match c {
            std::path::Component::Normal(p) => p.to_str(),
            _ => None,
        })
        .collect();
    let at = parts
        .iter()
        .position(|p| p.starts_with("kio-fuse-") || *p == "gvfs")?;
    let rest = &parts[at + 1..];
    if parts[at] == "gvfs" {
        let (mount, rest) = rest.split_first()?;
        let share = gvfs_share(mount)?;
        return Some(joined(share, rest));
    }
    // kio-fuse: <protocol>/<host, with any user@>/<path>
    let [protocol, host, rest @ ..] = rest else {
        return None;
    };
    Some(joined(format!("{protocol}://{host}"), rest))
}

/// `smb://[user@]host/share` for a gvfs mount's name: `smb-share:server=host,share=s,user=u`.
fn gvfs_share(mount: &str) -> Option<String> {
    let fields = mount.strip_prefix("smb-share:")?;
    let field =
        |key: &str| (fields.split(',')).find_map(|f| f.strip_prefix(key)?.strip_prefix('='));
    let user = field("user").map_or_else(String::new, |u| format!("{u}@"));
    Some(format!(
        "smb://{user}{}/{}",
        field("server")?,
        field("share")?
    ))
}

fn joined(mut url: String, rest: &[&str]) -> String {
    for part in rest {
        url.push('/');
        url.push_str(part);
    }
    url
}

/// What the Library keeps for `path`: its network location if it's in a FUSE mount that
/// doesn't last, else the path itself.
pub fn kept(path: &Path) -> PathBuf {
    location(path).map_or_else(|| path.to_path_buf(), PathBuf::from)
}

/// The rest of `url` after `prefix` (`""` for `url` itself), if it is in it. The host and
/// share are matched in any letter case, as SMB does.
#[cfg_attr(any(windows, target_os = "macos"), allow(dead_code))]
fn under<'a>(url: &'a str, prefix: &str) -> Option<&'a str> {
    let prefix = prefix.trim_end_matches('/');
    let head = url.get(..prefix.len())?;
    let rest = &url[prefix.len()..];
    (head.eq_ignore_ascii_case(prefix) && (rest.is_empty() || rest.starts_with('/')))
        .then(|| rest.trim_start_matches('/'))
}

#[cfg_attr(any(windows, target_os = "macos"), allow(dead_code))]
fn below(root: &Path, rest: &str) -> PathBuf {
    (rest.split('/').filter(|p| !p.is_empty())).fold(root.to_path_buf(), |p, part| p.join(part))
}

/// Where `url` could be on this computer now, from mounts already made: in each kio-fuse
/// mount in `runtime` (the `XDG_RUNTIME_DIR`), in gvfs's, and in the CIFS mounts listed in
/// `mounts` (as `/proc/mounts` lists them). Only places that are folders now.
#[cfg_attr(any(windows, target_os = "macos"), allow(dead_code))]
pub fn mounted(url: &str, runtime: Option<&Path>, mounts: &str) -> Vec<PathBuf> {
    let mut places = Vec::new();
    if let Some(runtime) = runtime
        && let Ok(entries) = std::fs::read_dir(runtime)
    {
        let (Some(scheme), Some((_, rest))) = (scheme(url), url.split_once("://")) else {
            return places;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            if name.to_str().is_some_and(|n| n.starts_with("kio-fuse-")) {
                places.push(below(&entry.path().join(scheme), rest));
            }
        }
        if let Ok(entries) = std::fs::read_dir(runtime.join("gvfs")) {
            for entry in entries.flatten() {
                let share = (entry.file_name().to_str()).and_then(gvfs_share);
                if let Some(rest) = share.as_deref().and_then(|s| under(url, s)) {
                    places.push(below(&entry.path(), rest));
                }
            }
        }
    }
    for line in mounts.lines() {
        let mut fields = line.split_whitespace();
        let (Some(source), Some(point), Some(kind)) = (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        if !matches!(kind, "cifs" | "smb3" | "smbfs") {
            continue;
        }
        let share = format!("smb:{}", unescape(source));
        if let Some(rest) = under(url, &share) {
            places.push(below(Path::new(&unescape(point)), rest));
        }
    }
    places.retain(|p| p.is_dir());
    places
}

/// A `/proc/mounts` field with its spaces, tabs and backslashes written back.
#[cfg_attr(any(windows, target_os = "macos"), allow(dead_code))]
fn unescape(field: &str) -> String {
    field
        .replace("\\040", " ")
        .replace("\\011", "\t")
        .replace("\\134", "\\")
}

/// `url` as a Windows UNC path: `smb://host/share/a` is `\\host\share\a`.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn unc(url: &str) -> Option<PathBuf> {
    let rest = url.strip_prefix("smb://")?;
    let rest = rest.rsplit_once('@').map_or(rest, |(_, r)| r);
    Some(PathBuf::from(format!(r"\\{}", rest.replace('/', r"\"))))
}

/// Where the folder kept as `kept` is on this computer now: itself if it's a path that is
/// there; else, for a network location (or a FUSE path gone stale), where the share is
/// mounted now, mounting it if the desktop can. `None` while it can't be reached. This may
/// wait on the network, so it's only called in the background.
pub fn find(kept: &Path) -> Option<PathBuf> {
    if !is_location(kept) && kept.is_dir() {
        return Some(kept.to_path_buf());
    }
    let url = match kept.to_str().filter(|_| is_location(kept)) {
        Some(url) => url.to_owned(),
        None => location(kept)?,
    };
    find_url(&url)
}

#[cfg(windows)]
fn find_url(url: &str) -> Option<PathBuf> {
    unc(url).filter(|p| p.is_dir())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn find_url(url: &str) -> Option<PathBuf> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from);
    let mounts = std::fs::read_to_string("/proc/mounts").unwrap_or_default();
    let found = |places: Vec<PathBuf>| places.into_iter().next();
    found(mounted(url, runtime.as_deref(), &mounts))
        .or_else(|| {
            // KDE: kio-fuse mounts it (starting itself if need be) and says where
            let out = run(
                "dbus-send",
                &[
                    "--session",
                    "--print-reply=literal",
                    &format!("--reply-timeout={}", MOUNTING.as_millis()),
                    "--dest=org.kde.KIOFuse",
                    "/org/kde/KIOFuse",
                    "org.kde.KIOFuse.VFS.mountUrl",
                    &format!("string:{url}"),
                ],
            )?;
            Some(PathBuf::from(out.trim())).filter(|p| p.is_dir())
        })
        .or_else(|| {
            // GNOME and the rest: gvfs mounts it (without asking for a password)
            run("gio", &["mount", url])?;
            found(mounted(url, runtime.as_deref(), &mounts))
        })
}

#[cfg(target_os = "macos")]
fn find_url(_url: &str) -> Option<PathBuf> {
    None
}

/// What `program` printed, if it ran and succeeded within [`MOUNTING`]; nothing is typed
/// into it, so one that would ask for a password fails instead.
#[cfg(all(unix, not(target_os = "macos")))]
fn run(program: &str, args: &[&str]) -> Option<String> {
    use std::io::Read as _;
    use std::process::{Command, Stdio};
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let start = std::time::Instant::now();
    let status = loop {
        if let Ok(Some(status)) = child.try_wait() {
            break status;
        }
        if start.elapsed() > MOUNTING {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    };
    let mut out = String::new();
    child.stdout.take()?.read_to_string(&mut out).ok()?;
    status.success().then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const KIO: &str = "/run/user/1000/kio-fuse-UrUqhD/smb/synology.local/STL/A-Team/02_CLUB A";

    #[test]
    fn kio_fuse_paths_are_kept_as_their_share() {
        assert_eq!(
            location(Path::new(KIO)).as_deref(),
            Some("smb://synology.local/STL/A-Team/02_CLUB A")
        );
        assert_eq!(
            location(Path::new(
                "/run/user/1000/kio-fuse-ab12/sftp/me@box/home/me"
            ))
            .as_deref(),
            Some("sftp://me@box/home/me")
        );
        // the mount itself, with no share in it, is nothing to keep
        assert_eq!(
            location(Path::new("/run/user/1000/kio-fuse-ab12/smb")),
            None
        );
    }

    #[test]
    fn gvfs_paths_are_kept_as_their_share() {
        let gvfs = "/run/user/1000/gvfs/smb-share:server=nas.local,share=stl,user=matt/A-Team";
        assert_eq!(
            location(Path::new(gvfs)).as_deref(),
            Some("smb://matt@nas.local/stl/A-Team")
        );
        let plain = "/run/user/1000/gvfs/smb-share:server=nas.local,share=stl";
        assert_eq!(
            location(Path::new(plain)).as_deref(),
            Some("smb://nas.local/stl")
        );
        // other gvfs mounts (a phone, sftp) aren't shares
        assert_eq!(
            location(Path::new("/run/user/1000/gvfs/mtp:host=Phone/DCIM")),
            None
        );
    }

    /// Paths that last are kept as they are: local folders, CIFS mounts, Windows UNC
    /// paths and macOS volumes.
    #[test]
    fn lasting_paths_are_kept_as_they_are() {
        for path in [
            "/home/matthew/STL",
            "/mnt/nas/STL",
            r"\\synology\STL\A-Team",
            "/Volumes/STL/A-Team",
            r"C:\Users\me\STL",
        ] {
            assert_eq!(location(Path::new(path)), None, "{path}");
            assert_eq!(kept(Path::new(path)), PathBuf::from(path));
            assert!(!is_location(Path::new(path)), "{path}");
        }
        assert_eq!(
            kept(Path::new(KIO)),
            PathBuf::from("smb://synology.local/STL/A-Team/02_CLUB A")
        );
        assert!(is_location(&kept(Path::new(KIO))));
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cut-craft-net-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A share is found again in this session's kio-fuse mount, whatever its random name.
    #[test]
    fn found_in_todays_kio_fuse_mount() {
        let runtime = scratch("kio");
        let today = runtime.join("kio-fuse-NeWtOk/smb/synology.local/STL/A-Team/02_CLUB A");
        std::fs::create_dir_all(&today).unwrap();
        // yesterday's mount, empty now
        std::fs::create_dir_all(runtime.join("kio-fuse-UrUqhD")).unwrap();
        let url = "smb://synology.local/STL/A-Team/02_CLUB A";
        assert_eq!(mounted(url, Some(&runtime), ""), [today]);
        assert_eq!(
            mounted("smb://elsewhere/STL", Some(&runtime), ""),
            Vec::<PathBuf>::new()
        );
    }

    // gvfs's mount names have a ':' in them, which Windows doesn't allow
    #[cfg(unix)]
    #[test]
    fn found_in_gvfs_and_cifs_mounts() {
        let runtime = scratch("gvfs");
        let share = runtime.join("gvfs/smb-share:server=nas.local,share=stl");
        std::fs::create_dir_all(share.join("A-Team")).unwrap();
        assert_eq!(
            mounted("smb://NAS.local/STL/A-Team", Some(&runtime), ""),
            [share.join("A-Team")]
        );

        let point = scratch("cifs mount");
        std::fs::create_dir_all(point.join("A-Team")).unwrap();
        let mounts = format!(
            "proc /proc proc rw 0 0\n//nas.local/stl {} cifs rw,vers=3.0 0 0\n",
            point.display().to_string().replace(' ', "\\040")
        );
        assert_eq!(
            mounted("smb://nas.local/stl/A-Team", None, &mounts),
            [point.join("A-Team")]
        );
        // a share whose name only starts the same isn't it
        assert_eq!(
            mounted("smb://nas.local/stl2/A-Team", None, &mounts),
            Vec::<PathBuf>::new()
        );
    }

    #[test]
    fn unc_paths_for_windows() {
        assert_eq!(
            unc("smb://synology.local/STL/A-Team"),
            Some(PathBuf::from(r"\\synology.local\STL\A-Team"))
        );
        assert_eq!(unc("smb://matt@nas/stl"), Some(PathBuf::from(r"\\nas\stl")));
        assert_eq!(unc("sftp://box/home"), None);
    }

    /// A local folder that is there is found as itself, without the network.
    #[test]
    fn local_folders_are_themselves() {
        let dir = scratch("local");
        assert_eq!(find(&dir), Some(dir.clone()));
        assert_eq!(find(&dir.join("gone")), None);
    }
}
