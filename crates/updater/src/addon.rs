//! Add-ons: installed from the newest release of a GitHub repository, and updated the same
//! way when that repository publishes a newer one. Shared by every kind of add-on (tools
//! such as Depth, gadgets, content packs).
//!
//! A release of an add-on carries:
//! - `<anything>-<os>-<arch>.zip` (such as `depth-linux-x86_64.zip`) for each computer it
//!   runs on, or one `<anything>.zip` for every computer, holding `addon.toml` and the
//!   add-on's files;
//! - any large files the manifest lists under `downloads` (model weights, say), once for
//!   every computer;
//! - optionally `<file>.minisig` beside each, signed with Cut Craft's release key. Those
//!   are "signed"; an add-on without signatures may still be installed, once the user has
//!   been warned. A signature that does not match refuses the install outright.
//!
//! An add-on is unpacked into `<add-ons folder>/<id>/`, beside `installed.json` saying where
//! it came from, so it can be updated later.

use crate::package;
use crate::release::{Asset, Release};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};

/// The file in an add-on's folder that says where it came from.
pub const RECORD: &str = "installed.json";
/// The manifest at the top of an add-on's download.
pub const MANIFEST: &str = "addon.toml";

/// An add-on Cut Craft offers by name, its repository filled in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Official {
    pub id: &'static str,
    pub name: &'static str,
    /// What it does, in a sentence.
    pub about: &'static str,
    pub repo: &'static str,
}

/// The add-ons listed in the Add-ons window.
pub const OFFICIAL: &[Official] = &[Official {
    id: "depth",
    name: "Depth (Relief from Photo)",
    about: "Estimates how far away each part of a photo is, so Relief from Photo can carve \
            it. About 100 MB.",
    repo: "NoobyNull/CutCraft-Depth",
}];

/// A GitHub repository, `owner/name`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Repo {
    pub owner: String,
    pub name: String,
}

impl Repo {
    /// `owner/name`, or a GitHub link to the repository or any page in it.
    pub fn parse(text: &str) -> Result<Self, String> {
        let text = text.trim().trim_end_matches('/');
        let (rest, link) = ["https://", "http://"]
            .iter()
            .find_map(|scheme| text.strip_prefix(scheme))
            .map_or((text, false), |rest| (rest, true));
        let rest = rest.strip_prefix("www.").unwrap_or(rest);
        let path = match rest.strip_prefix("github.com/") {
            Some(path) => Some(path),
            // a link must be to GitHub
            None if link => None,
            None => Some(rest),
        };
        let mut parts = path.into_iter().flat_map(|p| p.split('/'));
        let (owner, name) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
        let name = name.strip_suffix(".git").unwrap_or(name);
        let fine = |s: &str| {
            !s.is_empty()
                && !s.starts_with('.')
                && s.chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        };
        if fine(owner) && fine(name) {
            Ok(Self {
                owner: owner.to_owned(),
                name: name.to_owned(),
            })
        } else {
            Err(format!(
                "\"{text}\" is not a GitHub repository. Type it as owner/name, such as \
                 NoobyNull/CutCraft-Depth, or paste its GitHub link."
            ))
        }
    }

    /// The API address of its newest release.
    pub fn latest_url(&self) -> String {
        format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            self.owner, self.name
        )
    }
}

impl std::fmt::Display for Repo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.owner, self.name)
    }
}

/// An add-on's `addon.toml`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Manifest {
    /// Its folder's name: lower-case letters, digits, `-` and `_`.
    pub id: String,
    pub name: String,
    /// `tool`, `gadget` or `content`.
    pub kind: String,
    pub version: String,
    /// The Cut Craft versions it works with, such as `>=0.5.0`.
    #[serde(default)]
    pub needs_cutcraft: Option<String>,
    /// Its program, for a tool (`.exe` is added on Windows), made runnable on install.
    #[serde(default)]
    pub program: Option<String>,
    /// Files fetched from the same release beside the zip, once for every computer.
    #[serde(default)]
    pub downloads: Vec<String>,
}

impl Manifest {
    pub fn parse(text: &str) -> Result<Self, String> {
        let m: Self =
            toml::from_str(text).map_err(|e| format!("the add-on's {MANIFEST} is broken: {e}"))?;
        let id_fine = !m.id.is_empty()
            && m.id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_'));
        if !id_fine {
            return Err(format!("the add-on's id \"{}\" is not a folder name", m.id));
        }
        if let Some(name) = m
            .downloads
            .iter()
            .chain(&m.program)
            .find(|d| !plain_name(d))
        {
            return Err(format!("the add-on asks for an odd file \"{name}\""));
        }
        Ok(m)
    }

    /// Whether this Cut Craft (`current`) is one it works with.
    pub fn suits(&self, current: &Version) -> Result<(), String> {
        let Some(needs) = &self.needs_cutcraft else {
            return Ok(());
        };
        let req = semver::VersionReq::parse(needs)
            .map_err(|_| format!("the add-on needs an odd Cut Craft version \"{needs}\""))?;
        if req.matches(current) {
            Ok(())
        } else {
            Err(format!(
                "{} needs Cut Craft {needs}; this is {current}. Update Cut Craft first \
                 (Help ▸ Check for Updates).",
                self.name
            ))
        }
    }
}

/// A file name with no folders in it.
fn plain_name(name: &str) -> bool {
    !name.is_empty() && !name.contains(['/', '\\']) && name != "." && name != ".."
}

/// Where an installed add-on came from, kept in its folder as [`RECORD`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub repo: Repo,
    pub tag: String,
    pub version: String,
    pub signed: bool,
}

/// The installed add-on `id` in `addons`, if any: where it came from.
pub fn installed(addons: &Path, id: &str) -> Option<Record> {
    let text = fs::read_to_string(addons.join(id).join(RECORD)).ok()?;
    serde_json::from_str(&text).ok()
}

/// This computer's part of a download's name: `linux-x86_64` or `windows-x86_64`.
pub fn platform() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

/// A release of an add-on, ready to install here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offer {
    pub repo: Repo,
    pub release: Release,
    /// The zip for this computer.
    pub zip: Asset,
    /// Every file in the release has a signature beside it. Each is checked as it arrives.
    pub signed: bool,
}

impl Offer {
    /// What of `release` installs on `platform` (see [`platform`]).
    pub fn from_release(repo: Repo, release: Release, platform: &str) -> Result<Self, String> {
        const OTHERS: [&str; 3] = ["-linux-", "-windows-", "-macos-"];
        let zips = || {
            release
                .assets
                .iter()
                .filter(|a| a.name.to_ascii_lowercase().ends_with(".zip"))
        };
        let zip = zips()
            .find(|a| a.name.ends_with(&format!("-{platform}.zip")))
            .or_else(|| zips().find(|a| !OTHERS.iter().any(|o| a.name.contains(o))))
            .cloned()
            .ok_or_else(|| {
                format!(
                    "{repo}'s newest release ({}) has no add-on for this computer \
                     ({platform}). Ask its author, or look at {}.",
                    release.tag, release.page
                )
            })?;
        let signed = release
            .assets
            .iter()
            .filter(|a| !a.name.ends_with(".minisig"))
            .all(|a| release.signature_of(a).is_some());
        Ok(Self {
            repo,
            release,
            zip,
            signed,
        })
    }

    /// Whether this is newer than what `record` says is installed.
    pub fn is_newer_than(&self, record: &Record) -> bool {
        crate::release::version_of(&record.version).is_none_or(|v| self.release.version > v)
    }
}

/// Asks GitHub for `repo`'s newest release, as an add-on for this computer.
pub fn find(repo: &Repo) -> Result<Offer, String> {
    let json = crate::get_text(&repo.latest_url()).map_err(|e| {
        if e.contains("no releases") {
            format!("{repo} has no releases on GitHub (or there is no such repository)")
        } else {
            e
        }
    })?;
    Offer::from_release(repo.clone(), Release::parse(&json)?, &platform())
}

/// The newest release of the installed add-on `id`, if it is newer.
pub fn update_for(addons: &Path, id: &str) -> Result<Option<Offer>, String> {
    let record = installed(addons, id).ok_or("that add-on is not installed")?;
    let offer = find(&record.repo)?;
    Ok(offer.is_newer_than(&record).then_some(offer))
}

/// Installs (or updates) `offer` into `addons`, telling `progress` the bytes downloaded of
/// the whole. The caller asks the user first when it is not signed. `usable` says whether
/// this Cut Craft can use the add-on its manifest describes (its kind, say), or why not;
/// nothing is installed unless it can.
pub fn install(
    addons: &Path,
    offer: &Offer,
    usable: &dyn Fn(&Manifest) -> Result<(), String>,
    progress: &dyn Fn(u64, u64),
) -> Result<Manifest, String> {
    install_with(
        addons,
        offer,
        &crate::current_version(),
        crate::RELEASE_KEY,
        usable,
        &|asset, done| crate::download(asset, done),
        progress,
    )
}

type Fetch<'a> = dyn Fn(&Asset, &dyn Fn(u64)) -> Result<Vec<u8>, String> + 'a;

/// [`install`], with the Cut Craft version, release key and downloader given.
fn install_with(
    addons: &Path,
    offer: &Offer,
    current: &Version,
    key: &str,
    usable: &dyn Fn(&Manifest) -> Result<(), String>,
    fetch: &Fetch<'_>,
    progress: &dyn Fn(u64, u64),
) -> Result<Manifest, String> {
    let release = &offer.release;
    let done_before = std::cell::Cell::new(0);
    let total = std::cell::Cell::new(offer.zip.size);
    // a file of the release, checked against GitHub's checksum and its signature if any
    let get = |asset: &Asset| -> Result<Vec<u8>, String> {
        let bytes = fetch(asset, &|done| {
            progress(done_before.get() + done, total.get());
        })?;
        done_before.set(done_before.get() + bytes.len() as u64);
        package::verify(&bytes, asset.digest.as_deref())?;
        if let Some(sig) = release.signature_of(asset) {
            let sig = String::from_utf8(fetch(sig, &|_| {})?)
                .map_err(|_| format!("{}'s signature is not text", asset.name))?;
            package::verify_signature(&bytes, &sig, key, &asset.name)?;
        }
        Ok(bytes)
    };

    let zip = get(&offer.zip)?;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&zip))
        .map_err(|e| format!("{} is not a readable .zip ({e})", offer.zip.name))?;
    let manifest = {
        let mut text = String::new();
        archive
            .by_name(MANIFEST)
            .map_err(|_| {
                format!(
                    "{} has no {MANIFEST}: it is not a Cut Craft add-on",
                    offer.zip.name
                )
            })?
            .read_to_string(&mut text)
            .map_err(|e| format!("could not read the add-on's {MANIFEST} ({e})"))?;
        Manifest::parse(&text)?
    };
    manifest.suits(current)?;
    usable(&manifest)?;
    let downloads = manifest
        .downloads
        .iter()
        .map(|name| {
            release
                .assets
                .iter()
                .find(|a| a.name == *name)
                .ok_or_else(|| format!("the release has no {name}, which the add-on needs"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    total.set(offer.zip.size + downloads.iter().map(|a| a.size).sum::<u64>());

    // unpacked beside the old one, which stays in use until everything has arrived
    fs::create_dir_all(addons).map_err(|e| cannot(addons, &e))?;
    let staging = addons.join(format!(".{}.new", manifest.id));
    let _ = fs::remove_dir_all(&staging);
    let unpacked = (|| {
        unpack(&mut archive, &staging)?;
        for asset in downloads {
            let bytes = get(asset)?;
            fs::write(staging.join(&asset.name), bytes).map_err(|e| cannot(&staging, &e))?;
        }
        if let Some(program) = &manifest.program {
            runnable(&staging.join(format!("{program}{}", std::env::consts::EXE_SUFFIX)))?;
        }
        let record = Record {
            repo: offer.repo.clone(),
            tag: release.tag.clone(),
            version: release.version.to_string(),
            signed: offer.signed,
        };
        let json = serde_json::to_string_pretty(&record).map_err(|e| e.to_string())?;
        fs::write(staging.join(RECORD), json).map_err(|e| cannot(&staging, &e))
    })();
    if let Err(e) = unpacked {
        let _ = fs::remove_dir_all(&staging);
        return Err(e);
    }
    swap_in(&staging, &addons.join(&manifest.id))?;
    log::info!(
        "installed add-on {} {} from {}",
        manifest.id,
        release.tag,
        offer.repo
    );
    Ok(manifest)
}

fn cannot(dir: &Path, e: &std::io::Error) -> String {
    format!("could not write the add-on to {} ({e})", dir.display())
}

/// Every file of `archive` into `dir`, keeping its folders; nothing outside `dir`.
fn unpack(
    archive: &mut zip::ZipArchive<std::io::Cursor<&Vec<u8>>>,
    dir: &Path,
) -> Result<(), String> {
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("the add-on's .zip is damaged ({e})"))?;
        let Some(path) = file.enclosed_name() else {
            return Err(format!(
                "the add-on's .zip holds an odd path \"{}\"",
                file.name()
            ));
        };
        let to = dir.join(path);
        if file.is_dir() {
            fs::create_dir_all(&to).map_err(|e| cannot(dir, &e))?;
            continue;
        }
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).map_err(|e| cannot(dir, &e))?;
        }
        let mut bytes = vec![];
        file.read_to_end(&mut bytes)
            .map_err(|e| format!("the add-on's .zip is damaged ({e})"))?;
        fs::write(&to, bytes).map_err(|e| cannot(dir, &e))?;
    }
    Ok(())
}

/// Lets `program` run (on Windows every file may).
fn runnable(program: &Path) -> Result<(), String> {
    if !program.is_file() {
        return Err(format!(
            "the add-on has no {} for this computer",
            program.file_name().unwrap_or_default().to_string_lossy()
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        fs::set_permissions(program, fs::Permissions::from_mode(0o755))
            .map_err(|e| cannot(program, &e))?;
    }
    Ok(())
}

/// Puts `staging` at `target`, the old add-on (if any) moved aside first and removed after.
fn swap_in(staging: &Path, target: &Path) -> Result<(), String> {
    let old: PathBuf = {
        let mut name = target.file_name().unwrap_or_default().to_os_string();
        name.push(".old");
        target.with_file_name(format!(".{}", name.to_string_lossy()))
    };
    let _ = fs::remove_dir_all(&old);
    let had = target.exists();
    if had {
        fs::rename(target, &old).map_err(|e| {
            format!(
                "could not replace the add-on in {} ({e}). Close anything using it and try \
                 again.",
                target.display()
            )
        })?;
    }
    if let Err(e) = fs::rename(staging, target) {
        if had {
            let _ = fs::rename(&old, target);
        }
        return Err(cannot(target, &e));
    }
    let _ = fs::remove_dir_all(&old);
    Ok(())
}

/// Removes the installed add-on `id`.
pub fn remove(addons: &Path, id: &str) -> Result<(), String> {
    if !plain_name(id) {
        return Err(format!("\"{id}\" is not an add-on"));
    }
    fs::remove_dir_all(addons.join(id))
        .map_err(|e| format!("could not remove the add-on ({e}). Close anything using it."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::io::Write as _;

    fn repo() -> Repo {
        Repo::parse("NoobyNull/CutCraft-Depth").unwrap()
    }

    #[test]
    fn a_repository_reads_as_owner_and_name_or_a_link() {
        let want = repo();
        for text in [
            "NoobyNull/CutCraft-Depth",
            "  NoobyNull/CutCraft-Depth/ ",
            "https://github.com/NoobyNull/CutCraft-Depth",
            "https://www.github.com/NoobyNull/CutCraft-Depth.git",
            "github.com/NoobyNull/CutCraft-Depth/releases/tag/v0.1.0",
        ] {
            assert_eq!(Repo::parse(text), Ok(want.clone()), "{text}");
        }
        for bad in ["", "NoobyNull", "https://gitlab.com/a/b", "a b/c", "../x"] {
            let e = Repo::parse(bad).unwrap_err();
            assert!(e.contains("owner/name"), "{bad}: {e}");
        }
        assert_eq!(want.to_string(), "NoobyNull/CutCraft-Depth");
        assert!(
            want.latest_url()
                .ends_with("/repos/NoobyNull/CutCraft-Depth/releases/latest")
        );
    }

    fn sha(bytes: &[u8]) -> String {
        use sha2::Digest as _;
        use std::fmt::Write as _;
        sha2::Sha256::digest(bytes)
            .iter()
            .fold("sha256:".to_owned(), |mut s, b| {
                let _ = write!(s, "{b:02x}");
                s
            })
    }

    fn zip(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut z = zip::ZipWriter::new(std::io::Cursor::new(vec![]));
        for (path, data) in files {
            z.start_file(*path, zip::write::SimpleFileOptions::default())
                .unwrap();
            z.write_all(data).unwrap();
        }
        z.finish().unwrap().into_inner()
    }

    /// A release of `files`, each signed with `key` when there is one; the files by url.
    struct Fake {
        release: Release,
        files: HashMap<String, Vec<u8>>,
        key: String,
    }

    fn fake(files: &[(&str, Vec<u8>)], signed: bool) -> Fake {
        let pair = minisign::KeyPair::generate_unencrypted_keypair().unwrap();
        let mut assets = vec![];
        let mut by_url = HashMap::new();
        let mut add = |name: String, bytes: Vec<u8>| {
            let url = format!("https://x/{name}");
            assets.push(Asset {
                name,
                url: url.clone(),
                size: bytes.len() as u64,
                digest: Some(sha(&bytes)),
            });
            by_url.insert(url, bytes);
        };
        for (name, bytes) in files {
            add((*name).to_owned(), bytes.clone());
            if signed {
                let sig = minisign::sign(
                    Some(&pair.pk),
                    &pair.sk,
                    std::io::Cursor::new(bytes),
                    Some(&format!("file:{name}")),
                    None,
                )
                .unwrap();
                add(format!("{name}.minisig"), sig.to_string().into_bytes());
            }
        }
        Fake {
            release: Release {
                tag: "v0.2.0".into(),
                version: Version::new(0, 2, 0),
                notes: String::new(),
                page: "https://github.com/NoobyNull/CutCraft-Depth/releases/tag/v0.2.0".into(),
                assets,
            },
            files: by_url,
            key: pair.pk.to_box().unwrap().to_string(),
        }
    }

    impl Fake {
        fn install(&self, addons: &Path, platform: &str) -> Result<Manifest, String> {
            let offer = Offer::from_release(repo(), self.release.clone(), platform)?;
            install_with(
                addons,
                &offer,
                &Version::new(0, 6, 0),
                &self.key,
                &|m| {
                    if m.kind == "theme" {
                        Err("Cut Craft cannot use a theme".into())
                    } else {
                        Ok(())
                    }
                },
                &|asset, done| {
                    let bytes = self.files[&asset.url].clone();
                    done(bytes.len() as u64);
                    Ok(bytes)
                },
                &|_, _| {},
            )
        }
    }

    const DEPTH: &str = r#"
        id = "depth"
        name = "Depth (Relief from Photo)"
        kind = "tool"
        version = "0.2.0"
        needs_cutcraft = ">=0.5.0"
        program = "cutcraft-depth"
        downloads = ["weights.safetensors"]
    "#;

    fn program() -> String {
        format!("cutcraft-depth{}", std::env::consts::EXE_SUFFIX)
    }

    fn depth_release(signed: bool) -> Fake {
        let here = zip(&[
            ("addon.toml", DEPTH.as_bytes()),
            (&program(), b"the program"),
            ("README.md", b"readme"),
        ]);
        fake(
            &[
                (&format!("depth-{}.zip", platform()), here),
                ("depth-macos-aarch64.zip", b"mac".to_vec()),
                ("weights.safetensors", b"weights".to_vec()),
            ],
            signed,
        )
    }

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cut-craft-addon-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn installs_the_zip_for_this_computer_and_its_downloads() {
        let addons = scratch("install");
        let fake = depth_release(true);
        let manifest = fake.install(&addons, &platform()).unwrap();
        assert_eq!(manifest.id, "depth");
        let dir = addons.join("depth");
        assert_eq!(fs::read(dir.join(program())).unwrap(), b"the program");
        assert_eq!(
            fs::read(dir.join("weights.safetensors")).unwrap(),
            b"weights"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = fs::metadata(dir.join(program()))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o111, 0o111, "runnable");
        }
        let record = installed(&addons, "depth").unwrap();
        assert_eq!(
            record,
            Record {
                repo: repo(),
                tag: "v0.2.0".into(),
                version: "0.2.0".into(),
                signed: true
            }
        );
        // nothing left over beside it
        let names: Vec<_> = fs::read_dir(&addons)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, ["depth"]);
        fs::remove_dir_all(addons).unwrap();
    }

    #[test]
    fn an_update_replaces_the_old_add_on_whole() {
        let addons = scratch("update");
        fs::create_dir_all(addons.join("depth")).unwrap();
        fs::write(addons.join("depth").join("stale.txt"), b"old").unwrap();
        depth_release(true).install(&addons, &platform()).unwrap();
        assert!(!addons.join("depth").join("stale.txt").exists());
        assert!(addons.join("depth").join(program()).is_file());
        remove(&addons, "depth").unwrap();
        assert!(!addons.join("depth").exists());
        fs::remove_dir_all(addons).unwrap();
    }

    #[test]
    fn an_unsigned_add_on_says_so_and_still_installs() {
        let addons = scratch("unsigned");
        let fake = depth_release(false);
        let offer = Offer::from_release(repo(), fake.release.clone(), &platform()).unwrap();
        assert!(!offer.signed);
        fake.install(&addons, &platform()).unwrap();
        assert!(!installed(&addons, "depth").unwrap().signed);
        fs::remove_dir_all(addons).unwrap();
    }

    #[test]
    fn a_signature_that_does_not_match_refuses_it_and_keeps_the_old_one() {
        let addons = scratch("tampered");
        fs::create_dir_all(addons.join("depth")).unwrap();
        fs::write(addons.join("depth").join("keep.txt"), b"old").unwrap();
        let mut fake = depth_release(true);
        // the weights changed after signing (GitHub's checksum follows the new file)
        let weights = fake
            .release
            .assets
            .iter_mut()
            .find(|a| a.name == "weights.safetensors")
            .unwrap();
        weights.digest = Some(sha(b"other weights"));
        fake.files
            .insert(weights.url.clone(), b"other weights".to_vec());
        let e = fake.install(&addons, &platform()).unwrap_err();
        assert!(e.contains("does not match"), "{e}");
        assert!(
            addons.join("depth").join("keep.txt").exists(),
            "old one kept"
        );
        let names: Vec<_> = fs::read_dir(&addons)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, ["depth"], "no half-made add-on left");
        fs::remove_dir_all(addons).unwrap();
    }

    #[test]
    fn a_damaged_download_is_refused() {
        let addons = scratch("damaged");
        let mut fake = depth_release(false);
        let zip = fake
            .release
            .assets
            .iter_mut()
            .find(|a| a.name.to_ascii_lowercase().ends_with(".zip"))
            .unwrap();
        zip.digest = Some(sha(b"something else"));
        let e = fake.install(&addons, &platform()).unwrap_err();
        assert!(e.contains("checksum"), "{e}");
        assert!(!addons.join("depth").exists());
    }

    #[test]
    fn a_release_without_this_computer_s_zip_says_so() {
        let fake = depth_release(true);
        let e = Offer::from_release(repo(), fake.release, "freebsd-riscv64").unwrap_err();
        assert!(e.contains("no add-on for this computer"), "{e}");
        // one zip for every computer serves all of them
        let any = fake_one_zip();
        let offer = Offer::from_release(repo(), any.release, "freebsd-riscv64").unwrap();
        assert_eq!(offer.zip.name, "pack.zip");
    }

    fn fake_one_zip() -> Fake {
        let manifest = "id = \"pack\"\nname = \"Pack\"\nkind = \"content\"\nversion = \"1.0.0\"";
        fake(
            &[(
                "pack.zip",
                zip(&[("addon.toml", manifest.as_bytes()), ("a/b.svg", b"<svg/>")]),
            )],
            false,
        )
    }

    #[test]
    fn a_content_pack_keeps_its_folders() {
        let addons = scratch("pack");
        fake_one_zip().install(&addons, &platform()).unwrap();
        assert_eq!(
            fs::read(addons.join("pack").join("a").join("b.svg")).unwrap(),
            b"<svg/>"
        );
        fs::remove_dir_all(addons).unwrap();
    }

    #[test]
    fn what_is_not_an_add_on_is_refused() {
        let addons = scratch("not");
        let no_manifest = fake(&[("x.zip", zip(&[("readme", b"hi")]))], false);
        let e = no_manifest.install(&addons, &platform()).unwrap_err();
        assert!(e.contains("not a Cut Craft add-on"), "{e}");
        let escaping = "id = \"../x\"\nname = \"x\"\nkind = \"tool\"\nversion = \"1.0.0\"";
        assert!(
            Manifest::parse(escaping)
                .unwrap_err()
                .contains("not a folder name")
        );
        let sneaky = format!("{escaping}\ndownloads = [\"../../.bashrc\"]").replace("../x", "x");
        assert!(Manifest::parse(&sneaky).unwrap_err().contains("odd file"));
        assert!(!addons.exists() || fs::read_dir(&addons).unwrap().next().is_none());
        let _ = fs::remove_dir_all(addons);
    }

    #[test]
    fn a_program_outside_the_add_on_is_refused() {
        let base = "id = \"x\"\nname = \"x\"\nkind = \"tool\"\nversion = \"1.0.0\"";
        for program in ["../../../../.ssh/id_rsa", "/etc/passwd", "a/b", "..", ""] {
            let m = format!("{base}\nprogram = \"{program}\"");
            let e = Manifest::parse(&m).unwrap_err();
            assert!(e.contains("odd file"), "{program}: {e}");
        }
    }

    #[test]
    fn an_add_on_this_cut_craft_cannot_use_is_not_installed() {
        let addons = scratch("unusable");
        let manifest = "id = \"dark\"\nname = \"Dark\"\nkind = \"theme\"\nversion = \"1.0.0\"";
        let fake = fake(
            &[("dark.zip", zip(&[("addon.toml", manifest.as_bytes())]))],
            true,
        );
        let e = fake.install(&addons, &platform()).unwrap_err();
        assert!(e.contains("cannot use a theme"), "{e}");
        assert!(!addons.join("dark").exists());
        let _ = fs::remove_dir_all(addons);
    }

    #[test]
    fn an_add_on_for_a_newer_cut_craft_waits_for_it() {
        let m = Manifest::parse(DEPTH).unwrap();
        assert_eq!(m.suits(&Version::new(0, 5, 3)), Ok(()));
        let e = m.suits(&Version::new(0, 4, 9)).unwrap_err();
        assert!(e.contains("needs Cut Craft >=0.5.0"), "{e}");
    }

    #[test]
    fn only_a_newer_release_is_an_update() {
        let offer = Offer::from_release(repo(), depth_release(true).release, &platform()).unwrap();
        let had = |version: &str| Record {
            repo: repo(),
            tag: format!("v{version}"),
            version: version.into(),
            signed: true,
        };
        assert!(offer.is_newer_than(&had("0.1.0")));
        assert!(!offer.is_newer_than(&had("0.2.0")));
        assert!(!offer.is_newer_than(&had("0.3.0")));
    }

    #[test]
    fn the_depth_add_on_s_manifest_reads() {
        let m = Manifest::parse(include_str!("../../depth-addon/addon.toml")).unwrap();
        assert_eq!((m.id.as_str(), m.kind.as_str()), ("depth", "tool"));
        assert_eq!(m.program.as_deref(), Some("cutcraft-depth"));
        assert_eq!(m.downloads, ["depth_anything_v2_vits.safetensors"]);
        assert_eq!(m.suits(&crate::current_version()), Ok(()));
    }

    #[test]
    fn the_official_add_ons_name_their_repositories() {
        for a in OFFICIAL {
            assert!(Repo::parse(a.repo).is_ok(), "{}", a.repo);
        }
        assert!(OFFICIAL.iter().any(|a| a.id == "depth"));
    }
}
