//! What one-click updating needs around the swap itself: a version-named `AppImage` gets the
//! new version's name (and the links and menu entries that started the old one follow it),
//! and the new version starts clean of the old `AppImage`'s surroundings, with the job that
//! was open.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

/// Where the new `AppImage` goes: under the release's own name when the old file's name
/// carries the running version (`CutCraft-0.6.13-x86_64.AppImage` becomes
/// `CutCraft-0.6.14-x86_64.AppImage`, so the name never lies), else over the old file, a
/// name the user chose (`CutCraft.AppImage`).
pub fn appimage_destination(file: &Path, current: &semver::Version, asset_name: &str) -> PathBuf {
    let named_for_version = file
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.contains(&current.to_string()));
    if named_for_version && file.file_name() != Some(asset_name.as_ref()) {
        file.with_file_name(asset_name)
    } else {
        file.to_path_buf()
    }
}

/// The folders whose links and menu entries can start an `AppImage` in `dir`: the program
/// folders on the user's path, the desktop's application menu, and the `AppImage`'s own folder.
pub fn launcher_folders(dir: &Path, home: Option<&Path>, data_home: Option<&Path>) -> Vec<PathBuf> {
    let mut folders = vec![dir.to_path_buf()];
    if let Some(home) = home {
        folders.push(home.join(".local/bin"));
        folders.push(home.join("bin"));
        if data_home.is_none() {
            folders.push(home.join(".local/share/applications"));
        }
    }
    if let Some(data) = data_home {
        folders.push(data.join("applications"));
    }
    folders.dedup();
    folders
}

/// Points what started `old` at `new` instead, in `folders` (not their sub-folders): a link
/// to `old` (`~/.local/bin/cut-craft`) is made again to `new`, and a desktop menu entry
/// naming `old` names `new`. What was changed, for the log. Anything that cannot be changed
/// is left as it was: the new version still runs from its own file.
pub fn repoint_launchers(old: &Path, new: &Path, folders: &[PathBuf]) -> Vec<PathBuf> {
    let Ok(old_real) = fs::canonicalize(old) else {
        return vec![];
    };
    let (old_text, new_text) = (old.to_string_lossy(), new.to_string_lossy());
    let mut changed = vec![];
    for folder in folders {
        let Ok(entries) = fs::read_dir(folder) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_symlink() {
                if fs::canonicalize(&path).ok().as_deref() == Some(old_real.as_path())
                    && relink(&path, new).is_ok()
                {
                    changed.push(path);
                }
            } else if kind.is_file()
                && path.extension().is_some_and(|e| e == "desktop")
                && let Ok(text) = fs::read_to_string(&path)
                && text.contains(old_text.as_ref())
                && fs::write(&path, text.replace(old_text.as_ref(), new_text.as_ref())).is_ok()
            {
                changed.push(path);
            }
        }
    }
    changed
}

/// `link` made again, pointing at `target`; a new link is put beside it and renamed over it,
/// so the old link stays if anything fails.
#[cfg(unix)]
fn relink(link: &Path, target: &Path) -> std::io::Result<()> {
    let mut name = link.file_name().unwrap_or_default().to_os_string();
    name.push(".new");
    let fresh = link.with_file_name(name);
    let _ = fs::remove_file(&fresh);
    std::os::unix::fs::symlink(target, &fresh)?;
    fs::rename(&fresh, link).inspect_err(|_| {
        let _ = fs::remove_file(&fresh);
    })
}

#[cfg(not(unix))]
fn relink(_: &Path, _: &Path) -> std::io::Result<()> {
    Err(std::io::ErrorKind::Unsupported.into())
}

/// What the new version's start leaves out of this one's surroundings: the old `AppImage`'s
/// own variables (the new one sets its own), and its library folder in `LD_LIBRARY_PATH`,
/// which is gone once the old `AppImage` closes. `(name, None)` removes a variable;
/// `(name, Some(value))` sets it.
pub fn relaunch_env(vars: &[(OsString, OsString)]) -> Vec<(OsString, Option<OsString>)> {
    const APPIMAGE_OWN: [&str; 4] = ["APPIMAGE", "APPDIR", "ARGV0", "OWD"];
    let get = |name: &str| vars.iter().find(|(k, _)| k == name).map(|(_, v)| v);
    let mut out: Vec<(OsString, Option<OsString>)> = APPIMAGE_OWN
        .iter()
        .filter(|name| get(name).is_some())
        .map(|name| (OsString::from(name), None))
        .collect();
    if let (Some(appdir), Some(paths)) = (get("APPDIR"), get("LD_LIBRARY_PATH")) {
        let appdir = Path::new(appdir);
        let kept: Vec<PathBuf> = std::env::split_paths(paths)
            .filter(|p| !p.starts_with(appdir))
            .collect();
        let name = OsString::from("LD_LIBRARY_PATH");
        if kept.is_empty() {
            out.push((name, None));
        } else if let Ok(joined) = std::env::join_paths(kept)
            && &joined != paths
        {
            out.push((name, Some(joined)));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> semver::Version {
        semver::Version::parse(s).unwrap()
    }

    #[test]
    fn a_version_named_appimage_takes_the_new_versions_name() {
        let old = Path::new("/home/m/Applications/CutCraft-0.6.13-x86_64.AppImage");
        assert_eq!(
            appimage_destination(old, &v("0.6.13"), "CutCraft-0.6.14-x86_64.AppImage"),
            Path::new("/home/m/Applications/CutCraft-0.6.14-x86_64.AppImage")
        );
        // a name the user chose is kept
        let own = Path::new("/home/m/Apps/CutCraft.AppImage");
        assert_eq!(
            appimage_destination(own, &v("0.6.13"), "CutCraft-0.6.14-x86_64.AppImage"),
            own
        );
        // an older name than the running version is the user's too
        let older = Path::new("/a/CutCraft-0.6.10-x86_64.AppImage");
        assert_eq!(
            appimage_destination(older, &v("0.6.13"), "CutCraft-0.6.14-x86_64.AppImage"),
            older
        );
    }

    #[test]
    fn launchers_are_looked_for_on_the_path_and_in_the_menu() {
        let found = launcher_folders(
            Path::new("/home/m/Applications"),
            Some(Path::new("/home/m")),
            None,
        );
        assert_eq!(
            found,
            [
                "/home/m/Applications",
                "/home/m/.local/bin",
                "/home/m/bin",
                "/home/m/.local/share/applications"
            ]
            .map(PathBuf::from)
        );
        let data = launcher_folders(
            Path::new("/x"),
            Some(Path::new("/home/m")),
            Some(Path::new("/data")),
        );
        assert_eq!(data.last(), Some(&PathBuf::from("/data/applications")));
    }

    #[cfg(unix)]
    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cut-craft-relaunch-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::canonicalize(dir).unwrap()
    }

    #[cfg(unix)]
    #[test]
    fn links_and_menu_entries_follow_the_renamed_appimage() {
        let dir = scratch("links");
        let (apps, bin, menu) = (dir.join("Applications"), dir.join("bin"), dir.join("menu"));
        for d in [&apps, &bin, &menu] {
            fs::create_dir_all(d).unwrap();
        }
        let old = apps.join("CutCraft-0.6.13-x86_64.AppImage");
        let new = apps.join("CutCraft-0.6.14-x86_64.AppImage");
        fs::write(&old, b"old").unwrap();
        fs::write(&new, b"new").unwrap();
        let other = apps.join("Something.AppImage");
        fs::write(&other, b"x").unwrap();
        std::os::unix::fs::symlink(&old, bin.join("cut-craft")).unwrap();
        std::os::unix::fs::symlink(&other, bin.join("something")).unwrap();
        let entry = menu.join("cut-craft.desktop");
        fs::write(
            &entry,
            format!("[Desktop Entry]\nExec=\"{}\" %f\n", old.display()),
        )
        .unwrap();
        fs::write(menu.join("other.desktop"), "Exec=other\n").unwrap();

        let changed = repoint_launchers(&old, &new, &[apps.clone(), bin.clone(), menu.clone()]);

        assert_eq!(changed.len(), 2, "{changed:?}");
        assert_eq!(fs::read_link(bin.join("cut-craft")).unwrap(), new);
        assert_eq!(fs::read_link(bin.join("something")).unwrap(), other);
        assert!(
            fs::read_to_string(&entry)
                .unwrap()
                .contains("CutCraft-0.6.14-x86_64.AppImage")
        );
        assert_eq!(
            fs::read_to_string(menu.join("other.desktop")).unwrap(),
            "Exec=other\n"
        );
        // a folder that is not there is passed over
        assert_eq!(
            repoint_launchers(&new, &old, &[dir.join("missing")]),
            Vec::<PathBuf>::new()
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(unix)]
    fn vars(list: &[(&str, &str)]) -> Vec<(OsString, OsString)> {
        list.iter()
            .map(|(k, v)| ((*k).into(), (*v).into()))
            .collect()
    }

    // Linux paths, `:`-separated as Linux separates them
    #[cfg(unix)]
    #[test]
    fn the_new_version_starts_clean_of_the_old_appimage() {
        let env = relaunch_env(&vars(&[
            ("APPIMAGE", "/a/CutCraft-0.6.13-x86_64.AppImage"),
            ("APPDIR", "/tmp/.mount_CutCrA1"),
            ("ARGV0", "cut-craft"),
            ("LD_LIBRARY_PATH", "/tmp/.mount_CutCrA1/usr/lib:/opt/lib"),
            ("HOME", "/home/m"),
        ]));
        assert!(env.contains(&("APPIMAGE".into(), None)));
        assert!(env.contains(&("APPDIR".into(), None)));
        assert!(env.contains(&("ARGV0".into(), None)));
        assert!(env.contains(&("LD_LIBRARY_PATH".into(), Some("/opt/lib".into()))));
        assert!(!env.iter().any(|(k, _)| k == "HOME" || k == "OWD"));
        // only the AppImage's libraries: nothing is left
        let only = relaunch_env(&vars(&[
            ("APPDIR", "/tmp/.mount_x"),
            ("LD_LIBRARY_PATH", "/tmp/.mount_x/usr/lib"),
        ]));
        assert!(only.contains(&("LD_LIBRARY_PATH".into(), None)));
        // not from an AppImage: nothing changes
        assert_eq!(
            relaunch_env(&vars(&[("LD_LIBRARY_PATH", "/opt/lib")])),
            vec![]
        );
    }
}
