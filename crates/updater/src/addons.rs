//! Help ▸ Add-ons: programs that work with Cut Craft but ship on their own, downloaded from
//! their GitHub releases and checked exactly as Cut Craft's own updates are (GitHub's
//! checksum and Cut Craft's release key), then unpacked per user into
//! `<add-ons folder>/<id>/`.
//!
//! A release of an add-on carries one zip per computer, `<anything>-<platform>.zip` (see
//! [`platform`]), with `<zip>.minisig` beside it, signed with Cut Craft's release key. The
//! zip holds `addon.toml` (its [`Manifest`]) and the files it lists, side by side.

use crate::release::{Asset, Release};
use crate::{RELEASE_KEY, Step, download, get_text, package};
use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};

/// Where an add-on comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Source {
    /// Its folder under the add-ons folder, and the id in its `addon.toml`.
    pub id: &'static str,
    pub name: &'static str,
    /// One line on what it does, for the Add-ons window.
    pub about: &'static str,
    /// The GitHub API's newest release.
    pub latest: &'static str,
    /// Its releases page, for downloading by hand.
    pub page: &'static str,
}

/// The add-ons Cut Craft can install.
pub const SOURCES: &[Source] = &[DEPTH2STL];

/// Depth to STL: depth maps made into detailed reliefs sized for a carving bit.
pub const DEPTH2STL: Source = Source {
    id: "depth2stl",
    name: "Depth to STL",
    about: "Makes a depth map into a detailed relief sized for your carving bit \
            (Model ▸ Depth to STL).",
    latest: "https://api.github.com/repos/NoobyNull/depth2stl/releases/latest",
    page: "https://github.com/NoobyNull/depth2stl/releases",
};

/// The manifest file in every add-on's zip and folder.
pub const MANIFEST: &str = "addon.toml";

/// This computer, as add-on downloads name it; `None` where no add-on is built for it.
pub fn platform() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Some("linux-x64"),
        ("windows", "x86_64") => Some("windows-x64"),
        _ => None,
    }
}

/// An add-on's `addon.toml`: the few keys Cut Craft reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: String,
    /// The plugin API it speaks, for add-ons Cut Craft drives (`api = 1`).
    pub api: Option<u32>,
    /// Its files, beside `addon.toml`; the first is its program.
    pub files: Vec<String>,
}

impl Manifest {
    /// Reads the flat TOML an add-on's manifest is written in: `key = "text"`,
    /// `key = 1` and `key = ["text", ...]` lines, `#` comments.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut found = std::collections::HashMap::new();
        for line in text.lines().map(str::trim) {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| format!("addon.toml has a line that is not key = value: {line}"))?;
            found.insert(key.trim().to_owned(), value.trim().to_owned());
        }
        let text_of = |key: &str| -> Result<String, String> {
            let v = found
                .get(key)
                .ok_or_else(|| format!("addon.toml has no {key}"))?;
            unquote(v).ok_or_else(|| format!("addon.toml's {key} is not text"))
        };
        let files = found
            .get("files")
            .and_then(|v| v.strip_prefix('[')?.strip_suffix(']'))
            .ok_or("addon.toml lists no files")?
            .split(',')
            .map(str::trim)
            .filter(|f| !f.is_empty())
            .map(|f| unquote(f).ok_or_else(|| format!("addon.toml's file {f} is not text")))
            .collect::<Result<Vec<_>, _>>()?;
        if files.is_empty() {
            return Err("addon.toml lists no files".into());
        }
        if let Some(bad) = files.iter().find(|f| !plain_name(f)) {
            return Err(format!(
                "addon.toml lists {bad}, which is not a plain file name"
            ));
        }
        let api = match found.get("api") {
            Some(v) => Some(
                v.parse()
                    .map_err(|_| format!("addon.toml's api is not a number: {v}"))?,
            ),
            None => None,
        };
        Ok(Self {
            id: text_of("id")?,
            name: text_of("name")?,
            version: text_of("version")?,
            api,
            files,
        })
    }

    /// Its program: the first file.
    pub fn program(&self) -> &str {
        &self.files[0]
    }
}

fn unquote(v: &str) -> Option<String> {
    let v = v.strip_prefix('"')?.strip_suffix('"')?;
    (!v.contains('"')).then(|| v.to_owned())
}

/// A file name alone: no folders, nothing hidden, nothing that climbs out.
fn plain_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && !name.contains(['/', '\\', ':'])
        && Path::new(name).file_name().is_some_and(|f| f == name)
}

/// The add-on `id` installed in `addons` (the add-ons folder): its manifest, when that and
/// every file it lists are there.
pub fn installed(addons: &Path, id: &str) -> Option<Manifest> {
    let dir = addons.join(id);
    let manifest = Manifest::parse(&fs::read_to_string(dir.join(MANIFEST)).ok()?).ok()?;
    (manifest.id == id && manifest.files.iter().all(|f| dir.join(f).is_file())).then_some(manifest)
}

/// The newest release of `source` on GitHub.
pub fn latest(source: &Source) -> Result<Release, String> {
    let json = get_text(source.latest).map_err(|e| {
        if e.starts_with("GitHub has no releases") {
            format!("{} has no releases yet", source.name)
        } else {
            e
        }
    })?;
    Release::parse(&json)
}

/// The download in `release` for `platform`, with its signature, or why it cannot be
/// installed from Cut Craft.
pub fn download_for<'a>(
    release: &'a Release,
    platform: Option<&str>,
) -> Result<(&'a Asset, &'a Asset), String> {
    let platform = platform.ok_or("No add-on is built for this kind of computer yet.")?;
    let asset = release.asset(&format!("-{platform}.zip")).ok_or_else(|| {
        format!(
            "Version {} has no download for {platform}.",
            release.version
        )
    })?;
    let signature = release.signature_of(asset).ok_or_else(|| {
        format!(
            "Version {}'s download is not signed, so Cut Craft cannot check it is genuine.",
            release.version
        )
    })?;
    if asset.digest.is_none() {
        return Err(format!(
            "GitHub has no checksum for version {}'s download.",
            release.version
        ));
    }
    Ok((asset, signature))
}

/// Downloads `source`'s `release` for this computer, checks it, and installs it in
/// `addons` (over any older copy). Its manifest.
pub fn install(
    source: &Source,
    release: &Release,
    addons: &Path,
    step: &dyn Fn(Step),
) -> Result<Manifest, String> {
    let (asset, signature) = download_for(release, platform())?;
    let signature = String::from_utf8(download(signature, &|_| {})?)
        .map_err(|_| "the download's signature is not text".to_owned())?;
    let bytes = download(asset, &|done| {
        step(Step::Downloading {
            done,
            total: asset.size,
        });
    })?;
    step(Step::Checking);
    package::verify(&bytes, asset.digest.as_deref())?;
    package::verify_signature(&bytes, &signature, RELEASE_KEY, &asset.name)?;
    step(Step::Installing);
    let manifest = unpack(&bytes, addons, source.id)?;
    log::info!("installed add-on {} {}", manifest.id, manifest.version);
    Ok(manifest)
}

/// Unpacks a checked add-on zip into `addons/<id>/`, in place of what was there. The zip
/// must hold a manifest for `id` and every file it lists, and nothing in folders.
pub fn unpack(zip: &[u8], addons: &Path, id: &str) -> Result<Manifest, String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip))
        .map_err(|e| format!("the download is not a zip ({e})"))?;
    let mut files = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_owned();
        if !plain_name(&name) {
            return Err(format!(
                "the download holds {name}, which is not a plain file name"
            ));
        }
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .map_err(|e| format!("the download's {name} does not read: {e}"))?;
        files.push((name, bytes));
    }
    let manifest = files
        .iter()
        .find(|(n, _)| n == MANIFEST)
        .ok_or("the download has no addon.toml")
        .and_then(|(_, b)| std::str::from_utf8(b).map_err(|_| "addon.toml is not text"))
        .map_err(str::to_owned)
        .and_then(Manifest::parse)?;
    if manifest.id != id {
        return Err(format!(
            "the download is the add-on {}, not {id}",
            manifest.id
        ));
    }
    if let Some(missing) = manifest
        .files
        .iter()
        .find(|f| !files.iter().any(|(n, _)| n == *f))
    {
        return Err(format!("the download has no {missing}"));
    }
    let fail = |what: &str, path: &Path, e: std::io::Error| {
        format!("cannot {what} {}: {e}", path.display())
    };
    fs::create_dir_all(addons).map_err(|e| fail("make", addons, e))?;
    // written beside the old copy and swapped in, so a failure leaves the old one working
    let fresh = addons.join(format!(".{id}.new"));
    let _ = fs::remove_dir_all(&fresh);
    fs::create_dir_all(&fresh).map_err(|e| fail("make", &fresh, e))?;
    for (name, bytes) in &files {
        let path = fresh.join(name);
        fs::write(&path, bytes).map_err(|e| fail("write", &path, e))?;
        if manifest.files.first() == Some(name) {
            executable(&path).map_err(|e| fail("make runnable", &path, e))?;
        }
    }
    let dir = addons.join(id);
    let old = addons.join(format!(".{id}.old"));
    let _ = fs::remove_dir_all(&old);
    if dir.exists() {
        fs::rename(&dir, &old).map_err(|e| {
            let _ = fs::remove_dir_all(&fresh);
            format!(
                "cannot replace {} ({e}). Close anything using the add-on and try again.",
                dir.display()
            )
        })?;
    }
    if let Err(e) = fs::rename(&fresh, &dir) {
        let _ = fs::rename(&old, &dir);
        return Err(fail("install into", &dir, e));
    }
    let _ = fs::remove_dir_all(&old);
    Ok(manifest)
}

#[cfg(unix)]
fn executable(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt as _;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
}

#[cfg(not(unix))]
fn executable(_: &Path) -> std::io::Result<()> {
    Ok(())
}

/// Takes the add-on `id` out of `addons`.
pub fn remove(addons: &Path, id: &str) -> Result<(), String> {
    let dir: PathBuf = addons.join(id);
    fs::remove_dir_all(&dir).map_err(|e| format!("cannot remove {}: {e}", dir.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    const MANIFEST_TEXT: &str = r#"# a comment
id = "depth2stl"
name = "Depth to STL"
kind = "tool"
version = "1.1.0"
api = 1
needs_cutcraft = ">=0.6.0"
files = ["depth2stl"]
"#;

    fn zip_of(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for (name, bytes) in files {
            out.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            out.write_all(bytes).unwrap();
        }
        out.finish().unwrap().into_inner()
    }

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cutcraft-addons-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn reads_a_manifest() {
        let m = Manifest::parse(MANIFEST_TEXT).unwrap();
        assert_eq!(
            (m.id.as_str(), m.version.as_str(), m.api, m.program()),
            ("depth2stl", "1.1.0", Some(1), "depth2stl")
        );
        let two = Manifest::parse(
            &MANIFEST_TEXT.replace(r#"["depth2stl"]"#, r#"["depth2stl.exe", "weights.bin"]"#),
        )
        .unwrap();
        assert_eq!(two.files, ["depth2stl.exe", "weights.bin"]);
    }

    #[test]
    fn a_manifest_names_only_plain_files() {
        for bad in [
            r#"["../evil"]"#,
            r#"["dir/x"]"#,
            r#"[".hidden"]"#,
            "[]",
            r#"["a\\b"]"#,
        ] {
            let text = MANIFEST_TEXT.replace(r#"["depth2stl"]"#, bad);
            assert!(Manifest::parse(&text).is_err(), "{bad}");
        }
        assert!(Manifest::parse(&MANIFEST_TEXT.replace("api = 1", "api = one")).is_err());
    }

    #[test]
    fn unpacks_and_replaces_an_add_on() {
        let addons = scratch("unpack");
        let zip = zip_of(&[
            ("addon.toml", MANIFEST_TEXT.as_bytes()),
            ("depth2stl", b"v1"),
        ]);
        let m = unpack(&zip, &addons, "depth2stl").unwrap();
        assert_eq!(installed(&addons, "depth2stl"), Some(m));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = fs::metadata(addons.join("depth2stl/depth2stl"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o111, 0o111, "the program runs");
        }
        let newer = MANIFEST_TEXT.replace("1.1.0", "1.2.0");
        let zip = zip_of(&[("addon.toml", newer.as_bytes()), ("depth2stl", b"v2")]);
        unpack(&zip, &addons, "depth2stl").unwrap();
        assert_eq!(installed(&addons, "depth2stl").unwrap().version, "1.2.0");
        assert_eq!(fs::read(addons.join("depth2stl/depth2stl")).unwrap(), b"v2");
        let left: Vec<_> = fs::read_dir(&addons)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(left, ["depth2stl"], "nothing left over beside it");
        remove(&addons, "depth2stl").unwrap();
        assert_eq!(installed(&addons, "depth2stl"), None);
        fs::remove_dir_all(addons).unwrap();
    }

    #[test]
    fn refuses_a_zip_that_is_not_the_add_on() {
        let addons = scratch("refuse");
        let other = MANIFEST_TEXT.replace(r#"id = "depth2stl""#, r#"id = "other""#);
        for (zip, why) in [
            (zip_of(&[("depth2stl", b"x")]), "no addon.toml"),
            (
                zip_of(&[("addon.toml", MANIFEST_TEXT.as_bytes())]),
                "no depth2stl",
            ),
            (
                zip_of(&[("addon.toml", other.as_bytes()), ("depth2stl", b"x")]),
                "not depth2stl",
            ),
            (
                zip_of(&[
                    ("addon.toml", MANIFEST_TEXT.as_bytes()),
                    ("depth2stl", b"x"),
                    ("../up", b"x"),
                ]),
                "not a plain file name",
            ),
            (b"not a zip".to_vec(), "not a zip"),
        ] {
            let e = unpack(&zip, &addons, "depth2stl").unwrap_err();
            assert!(e.contains(why), "{e}");
        }
        assert_eq!(installed(&addons, "depth2stl"), None);
        let _ = fs::remove_dir_all(addons);
    }

    #[test]
    fn a_release_offers_the_signed_zip_for_this_computer() {
        let asset = |name: &str, digest: bool| Asset {
            name: name.into(),
            url: String::new(),
            size: 1,
            digest: digest.then(|| "sha256:00".into()),
        };
        let release = |assets| Release {
            tag: "v1.1.0".into(),
            version: crate::Version::new(1, 1, 0),
            notes: String::new(),
            page: String::new(),
            assets,
        };
        let good = release(vec![
            asset("depth2stl-1.1.0-linux-x64.zip", true),
            asset("depth2stl-1.1.0-linux-x64.zip.minisig", true),
            asset("depth2stl-1.1.0-windows-x64.zip", true),
        ]);
        let (zip, sig) = download_for(&good, Some("linux-x64")).unwrap();
        assert_eq!(
            (zip.name.as_str(), sig.name.as_str()),
            (
                "depth2stl-1.1.0-linux-x64.zip",
                "depth2stl-1.1.0-linux-x64.zip.minisig"
            )
        );
        assert!(
            download_for(&good, Some("windows-x64"))
                .unwrap_err()
                .contains("not signed")
        );
        assert!(
            download_for(&good, Some("macos-arm64"))
                .unwrap_err()
                .contains("no download")
        );
        assert!(download_for(&good, None).is_err());
        let unchecked = release(vec![
            asset("d-linux-x64.zip", false),
            asset("d-linux-x64.zip.minisig", true),
        ]);
        assert!(
            download_for(&unchecked, Some("linux-x64"))
                .unwrap_err()
                .contains("checksum")
        );
    }
}
