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

/// The repository every language pack is published on: each release carries one
/// `language-<code>.zip` per language (and its signature), each installed on its own.
pub const TRANSLATIONS: &str = "NoobyNull/CutCraft-Translations";

/// The zip a language's pack is in, on [`TRANSLATIONS`]' releases.
pub fn language_zip(code: &str) -> String {
    format!("language-{code}.zip")
}

/// The languages `release` (of [`TRANSLATIONS`]) has a pack for, by code, sorted.
pub fn languages_in(release: &Release) -> Vec<String> {
    let mut codes: Vec<String> = release
        .assets
        .iter()
        .filter_map(|a| a.name.strip_prefix("language-")?.strip_suffix(".zip"))
        .filter(|code| !code.is_empty())
        .map(str::to_owned)
        .collect();
    codes.sort();
    codes
}

/// Asks GitHub which languages [`TRANSLATIONS`]' newest release has packs for.
pub fn languages_offered() -> Result<Vec<String>, String> {
    let repo = Repo::parse(TRANSLATIONS)?;
    let json = crate::get_text(&repo.latest_url()).map_err(|e| {
        if e.contains("no releases") {
            "No language packs are published yet.".to_owned()
        } else {
            e
        }
    })?;
    Ok(languages_in(&Release::parse(&json)?))
}

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

    /// The API address of its release tagged `tag`.
    pub fn tag_url(&self, tag: &str) -> String {
        format!(
            "https://api.github.com/repos/{}/{}/releases/tags/{tag}",
            self.owner, self.name
        )
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
    /// Files of the same release fetched only when the person asks for them (see
    /// [`fetch_optional`]), such as a larger model; an update keeps them while the release
    /// still carries the same file.
    #[serde(default)]
    pub optional: Vec<String>,
    /// Gadget add-ons only: the web hosts its gadgets may fetch from over https, such as
    /// `relay.example.com`. Empty: no web at all. (Showing the list before installing is
    /// planned, issue #587.)
    #[serde(default)]
    pub network: Vec<String>,
    /// For a language pack (kind `language`): the language its `.ftl` files are in, such as
    /// `ru` or `pt-BR`.
    #[serde(default)]
    pub language: Option<String>,
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
            .chain(&m.optional)
            .chain(&m.program)
            .find(|d| !plain_name(d))
        {
            return Err(format!("the add-on asks for an odd file \"{name}\""));
        }
        if let Some(host) = m.network.iter().find(|h| !host_name(h)) {
            return Err(format!(
                "the add-on lists \"{host}\" as a web host; list host names only, such as \
                 relay.example.com"
            ));
        }
        if !m.network.is_empty() && m.kind != "gadget" {
            return Err("only gadget add-ons may list web hosts".to_owned());
        }
        if m.kind == "language" {
            let code = m.language.as_deref().unwrap_or_default();
            let fine = !code.is_empty()
                && code.split('-').all(|part| {
                    !part.is_empty() && part.chars().all(|c| c.is_ascii_alphanumeric())
                });
            if !fine {
                return Err(format!(
                    "the language pack \"{}\" does not say which language it is (language = \
                     \"ru\", say)",
                    m.id
                ));
            }
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
/// A bare host name such as `relay.example.com`: no scheme, port, path or user name.
fn host_name(host: &str) -> bool {
    // a name, never a number like 10.0.0.1 or 256.0.0.1: no top-level name is all digits
    let numeric = host
        .rsplit('.')
        .next()
        .is_some_and(|last| last.chars().all(|c| c.is_ascii_digit()));
    host.len() <= 253
        && host.contains('.')
        && !numeric
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        })
}

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
    /// The zip it came in, when its release holds several add-ons to pick from (a
    /// language pack on [`TRANSLATIONS`]): updates take the same one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zip: Option<String>,
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
    /// The zip asked for by name, of a release holding several add-ons; `None` for a
    /// release of one add-on, whose zip is the one for this computer.
    pub named: Option<String>,
}

impl Offer {
    /// What of `release` installs on `platform` (see [`platform`]).
    pub fn from_release(repo: Repo, release: Release, platform: &str) -> Result<Self, String> {
        Self::pick(repo, release, platform, None)
    }

    /// What of `release` installs on `platform`: the zip called `named` when given (one of
    /// the add-ons a release holds several of), else the one for this computer.
    pub fn pick(
        repo: Repo,
        release: Release,
        platform: &str,
        named: Option<&str>,
    ) -> Result<Self, String> {
        const OTHERS: [&str; 3] = ["-linux-", "-windows-", "-macos-"];
        if let Some(name) = named {
            let zip = release
                .assets
                .iter()
                .find(|a| a.name == name)
                .cloned()
                .ok_or_else(|| {
                    format!(
                        "{repo}'s newest release ({}) has no {name}. Look at {}.",
                        release.tag, release.page
                    )
                })?;
            let signed = release.signature_of(&zip).is_some();
            return Ok(Self {
                repo,
                release,
                zip,
                signed,
                named: Some(name.to_owned()),
            });
        }
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
            named: None,
        })
    }

    /// Whether this is newer than what `record` says is installed.
    pub fn is_newer_than(&self, record: &Record) -> bool {
        crate::release::version_of(&record.version).is_none_or(|v| self.release.version > v)
    }
}

/// What to say when GitHub has no release of `repo` (or no such repository). For an add-on
/// Cut Craft offers by name, that only means it has not been published yet, so it says so
/// without blaming the person; for any other, it points at the likely typo.
pub fn no_release(repo: &Repo) -> String {
    let full = repo.to_string();
    match OFFICIAL.iter().find(|a| a.repo.eq_ignore_ascii_case(&full)) {
        Some(official) => format!(
            "The {} add-on is not available to download yet. Nothing is wrong on your \
             computer: try again after the next Cut Craft update.",
            official.name
        ),
        None => format!(
            "GitHub has no release of {full} to install. Check its spelling, or ask its \
             author to publish a release."
        ),
    }
}

/// Asks GitHub for `repo`'s newest release, as an add-on for this computer, or as the zip
/// called `named` in it.
pub fn find(repo: &Repo, named: Option<&str>) -> Result<Offer, String> {
    let json = crate::get_text(&repo.latest_url()).map_err(|e| {
        if e.contains("no releases") {
            no_release(repo)
        } else {
            e
        }
    })?;
    Offer::pick(repo.clone(), Release::parse(&json)?, &platform(), named)
}

/// The newest release of the installed add-on `id`, if it is newer.
pub fn update_for(addons: &Path, id: &str) -> Result<Option<Offer>, String> {
    let record = installed(addons, id).ok_or("that add-on is not installed")?;
    let offer = find(&record.repo, record.zip.as_deref())?;
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
            // a manifest is a few lines; never read more than this of one
            .take(64 * 1024)
            .read_to_string(&mut text)
            .map_err(|e| format!("could not read the add-on's {MANIFEST} ({e})"))?;
        Manifest::parse(&text)?
    };
    manifest.suits(current)?;
    usable(&manifest)?;
    owns_its_id(addons, &manifest, &offer.repo)?;
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
    // official only when every file it brings is signed, not just its zip
    let signed = offer.signed && downloads.iter().all(|a| release.signature_of(a).is_some());

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
        keep_optional(&addons.join(&manifest.id), &staging, &manifest, release);
        if let Some(program) = &manifest.program {
            runnable(&staging.join(format!("{program}{}", std::env::consts::EXE_SUFFIX)))?;
        }
        let record = Record {
            repo: offer.repo.clone(),
            tag: release.tag.clone(),
            version: release.version.to_string(),
            signed,
            zip: offer.named.clone(),
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

/// Moves the optional files the person fetched from the add-on's folder (`old`) into its
/// new one (`staging`) when `release` carries the very same file; one it changed is left to
/// be fetched again.
fn keep_optional(old: &Path, staging: &Path, manifest: &Manifest, release: &Release) {
    for name in &manifest.optional {
        let (from, to) = (old.join(name), staging.join(name));
        let Some(asset) = release.assets.iter().find(|a| a.name == *name) else {
            continue;
        };
        let same = fs::metadata(&from).is_ok_and(|m| m.len() == asset.size)
            && fs::read(&from).is_ok_and(|b| package::verify(&b, asset.digest.as_deref()).is_ok());
        if same && fs::rename(&from, &to).is_err() {
            let _ = fs::copy(&from, &to);
        }
    }
}

/// Fetches `name`, one of the installed add-on `id`'s optional files, from the release it
/// was installed from, into its folder. Checked like every download; nothing is left
/// behind if it fails. `progress` hears (bytes so far, bytes in all).
pub fn fetch_optional(
    addons: &Path,
    id: &str,
    name: &str,
    progress: &dyn Fn(u64, u64),
) -> Result<(), String> {
    let record = installed(addons, id).ok_or("that add-on is not installed")?;
    let json = crate::get_text(&record.repo.tag_url(&record.tag))?;
    fetch_optional_with(
        addons,
        id,
        name,
        &Release::parse(&json)?,
        crate::RELEASE_KEY,
        &|asset, done| crate::download(asset, done),
        progress,
    )
}

/// [`fetch_optional`], with the release, release key and downloader given.
fn fetch_optional_with(
    addons: &Path,
    id: &str,
    name: &str,
    release: &Release,
    key: &str,
    fetch: &Fetch<'_>,
    progress: &dyn Fn(u64, u64),
) -> Result<(), String> {
    let dir = addons.join(id);
    let manifest = fs::read_to_string(dir.join(MANIFEST))
        .map_err(|_| "that add-on is not installed".to_string())
        .and_then(|text| Manifest::parse(&text))?;
    if !manifest.optional.iter().any(|o| o == name) {
        return Err(format!("{} has no optional {name}", manifest.name));
    }
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == name)
        .ok_or_else(|| format!("the add-on's release has no {name}"))?;
    let bytes = fetch(asset, &|done| progress(done, asset.size))?;
    package::verify(&bytes, asset.digest.as_deref())?;
    if let Some(sig) = release.signature_of(asset) {
        let sig = String::from_utf8(fetch(sig, &|_| {})?)
            .map_err(|_| format!("{name}'s signature is not text"))?;
        package::verify_signature(&bytes, &sig, key, name)?;
    }
    // written beside, then put in place whole: never a half file under its real name
    let part = dir.join(format!(".{name}.part"));
    let written = fs::write(&part, bytes).and_then(|()| fs::rename(&part, dir.join(name)));
    if let Err(e) = written {
        let _ = fs::remove_file(&part);
        return Err(cannot(&dir, &e));
    }
    log::info!("fetched {name} for add-on {id} ({})", release.tag);
    Ok(())
}

/// Whether `repo`'s add-on may go in the folder its manifest names: never in place of one
/// of Cut Craft's own add-ons (which come only from their own repositories), nor of an
/// add-on installed from another repository. An update from the same repository may.
fn owns_its_id(addons: &Path, manifest: &Manifest, repo: &Repo) -> Result<(), String> {
    let same = |a: &Repo, b: &Repo| {
        a.owner.eq_ignore_ascii_case(&b.owner) && a.name.eq_ignore_ascii_case(&b.name)
    };
    if let Some(own) = OFFICIAL.iter().find(|o| o.id == manifest.id)
        && Repo::parse(own.repo).is_ok_and(|r| !same(&r, repo))
    {
        return Err(format!(
            "Not installed: {repo}'s add-on uses the name \"{}\", which belongs to Cut \
             Craft's own {} add-on from {}. Installing it would replace that one. Ask \
             {repo}'s maker to give it a name of its own",
            manifest.id, own.name, own.repo
        ));
    }
    if let Some(record) = installed(addons, &manifest.id)
        && !same(&record.repo, repo)
    {
        return Err(format!(
            "Not installed: {repo}'s add-on uses the name \"{}\", the same as the add-on \
             installed from {}. Installing it would replace that one. Remove that add-on \
             first if you want this one instead",
            manifest.id, record.repo
        ));
    }
    Ok(())
}

fn cannot(dir: &Path, e: &std::io::Error) -> String {
    format!("could not write the add-on to {} ({e})", dir.display())
}

/// Every file of `archive` into `dir`, keeping its folders; nothing outside `dir`.
fn unpack(
    archive: &mut zip::ZipArchive<std::io::Cursor<&Vec<u8>>>,
    dir: &Path,
) -> Result<(), String> {
    unpack_within(archive, dir, MAX_UNPACKED)
}

/// What an add-on's zip may unpack to in all; large files belong in `downloads`.
const MAX_UNPACKED: u64 = 1024 * 1024 * 1024;

/// [`unpack`], refusing a zip that unpacks to more than `limit` bytes (however small it is
/// packed), streamed to disk so nothing large is held in memory.
fn unpack_within(
    archive: &mut zip::ZipArchive<std::io::Cursor<&Vec<u8>>>,
    dir: &Path,
    limit: u64,
) -> Result<(), String> {
    let mut left = limit;
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
        let mut out = fs::File::create(&to).map_err(|e| cannot(dir, &e))?;
        // one byte past what is left shows it would go over
        let written = std::io::copy(&mut (&mut file).take(left + 1), &mut out)
            .map_err(|e| format!("the add-on's .zip is damaged ({e})"))?;
        if written > left {
            return Err(format!(
                "the add-on's .zip unpacks to more than {} MB, so it is not installed",
                limit / (1024 * 1024)
            ));
        }
        left -= written;
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
        assert!(
            want.tag_url("v0.2.0")
                .ends_with("/repos/NoobyNull/CutCraft-Depth/releases/tags/v0.2.0")
        );
    }

    #[test]
    fn a_gadget_add_on_lists_its_web_hosts_by_name_only() {
        let base = "id = \"daily\"\nname = \"Daily\"\nkind = \"gadget\"\nversion = \"0.1.0\"\n";
        let m = Manifest::parse(&format!(
            "{base}network = [\"relay.example.com\", \"drive.usercontent.google.com\"]"
        ))
        .unwrap();
        assert_eq!(
            m.network,
            ["relay.example.com", "drive.usercontent.google.com"]
        );
        assert_eq!(Manifest::parse(base).unwrap().network, Vec::<String>::new());
        for bad in [
            "https://relay.example.com",
            "relay.example.com:443",
            "relay.example.com/api",
            "me@relay.example.com",
            "localhost",
            "Relay.Example.com",
            "-bad.example.com",
            "256.0.0.1",
            "10.0.0.1",
            "",
        ] {
            let e = Manifest::parse(&format!("{base}network = [\"{bad}\"]")).unwrap_err();
            assert!(e.contains("host names only"), "{bad}: {e}");
        }
        let tool = base.replace("gadget", "tool");
        let e = Manifest::parse(&format!("{tool}network = [\"relay.example.com\"]")).unwrap_err();
        assert!(e.contains("only gadget add-ons"), "{e}");
    }

    #[test]
    fn a_missing_release_explains_itself_in_plain_words() {
        // issue #567: the official Depth add-on had no release yet
        let official = repo();
        let e = no_release(&official);
        assert!(e.contains("Depth (Relief from Photo)"), "{e}");
        assert!(e.contains("not available to download yet"), "{e}");
        assert!(!e.contains("no such repository"), "{e}");
        // anyone else's repository: name it, and say what might be wrong
        let other = Repo::parse("someone/thing").unwrap();
        let e = no_release(&other);
        assert!(e.contains("someone/thing"), "{e}");
        assert!(e.contains("spelling"), "{e}");
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
            self.install_from(repo(), addons, platform)
        }

        fn install_from(
            &self,
            from: Repo,
            addons: &Path,
            platform: &str,
        ) -> Result<Manifest, String> {
            let offer = Offer::from_release(from, self.release.clone(), platform)?;
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

    /// A computer that is not this one, whose zip a release also carries.
    fn other_platform() -> &'static str {
        if platform() == "macos-aarch64" {
            "windows-x86_64"
        } else {
            "macos-aarch64"
        }
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
                (
                    &format!("depth-{}.zip", other_platform()),
                    b"another computer's".to_vec(),
                ),
                ("weights.safetensors", b"weights".to_vec()),
            ],
            signed,
        )
    }

    const DEPTH_WITH_LARGE: &str = r#"
        id = "depth"
        name = "Depth (Relief from Photo)"
        kind = "tool"
        version = "0.2.0"
        program = "cutcraft-depth"
        downloads = ["weights.safetensors"]
        optional = ["large.safetensors"]
    "#;

    fn release_with_large(large: &[u8]) -> Fake {
        let here = zip(&[
            ("addon.toml", DEPTH_WITH_LARGE.as_bytes()),
            (&program(), b"the program"),
        ]);
        fake(
            &[
                (&format!("depth-{}.zip", platform()), here),
                ("weights.safetensors", b"weights".to_vec()),
                ("large.safetensors", large.to_vec()),
            ],
            false,
        )
    }

    impl Fake {
        fn fetch_optional(&self, addons: &Path, name: &str) -> Result<(), String> {
            fetch_optional_with(
                addons,
                "depth",
                name,
                &self.release,
                &self.key,
                &|asset, done| {
                    let bytes = self.files[&asset.url].clone();
                    done(bytes.len() as u64);
                    Ok(bytes)
                },
                &|_, _| {},
            )
        }
    }

    #[test]
    fn an_optional_file_comes_only_when_asked_for() {
        let addons = scratch("optional");
        let fake = release_with_large(b"large weights");
        fake.install(&addons, &platform()).unwrap();
        let large = addons.join("depth").join("large.safetensors");
        assert!(!large.exists(), "not part of the install");
        fake.fetch_optional(&addons, "large.safetensors").unwrap();
        assert_eq!(fs::read(&large).unwrap(), b"large weights");
        // only what the add-on lists as optional
        let e = fake.fetch_optional(&addons, "README.md").unwrap_err();
        assert!(e.contains("README.md"), "{e}");
        fs::remove_dir_all(addons).unwrap();
    }

    #[test]
    fn an_optional_file_needs_the_add_on_installed() {
        let addons = scratch("optional-none");
        let fake = release_with_large(b"large weights");
        let e = fake
            .fetch_optional(&addons, "large.safetensors")
            .unwrap_err();
        assert!(e.contains("not installed"), "{e}");
    }

    #[test]
    fn an_optional_file_that_does_not_check_out_is_not_kept() {
        let addons = scratch("optional-bad");
        let mut fake = release_with_large(b"large weights");
        fake.install(&addons, &platform()).unwrap();
        let asset = fake
            .release
            .assets
            .iter_mut()
            .find(|a| a.name == "large.safetensors")
            .unwrap();
        asset.digest = Some(sha(b"something else"));
        assert!(fake.fetch_optional(&addons, "large.safetensors").is_err());
        let dir = addons.join("depth");
        assert!(!dir.join("large.safetensors").exists());
        assert!(
            fs::read_dir(&dir).unwrap().all(|e| !e
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains("large")),
            "no part-file left behind"
        );
        fs::remove_dir_all(addons).unwrap();
    }

    #[test]
    fn an_update_keeps_an_optional_file_unless_the_release_changed_it() {
        let addons = scratch("optional-update");
        let fake = release_with_large(b"large weights");
        fake.install(&addons, &platform()).unwrap();
        fake.fetch_optional(&addons, "large.safetensors").unwrap();
        let large = addons.join("depth").join("large.safetensors");
        // the same file in the new release: kept, not downloaded again
        fake.install(&addons, &platform()).unwrap();
        assert_eq!(fs::read(&large).unwrap(), b"large weights");
        // a new file: the old one is dropped, to be fetched again when wanted
        release_with_large(b"newer large weights")
            .install(&addons, &platform())
            .unwrap();
        assert!(!large.exists());
        fs::remove_dir_all(addons).unwrap();
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
                signed: true,
                zip: None,
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
    fn a_download_without_its_signature_is_not_official() {
        let addons = scratch("unsigned-download");
        let mut fake = depth_release(true);
        fake.release
            .assets
            .retain(|a| a.name != "weights.safetensors.minisig");
        fake.install(&addons, &platform()).unwrap();
        assert!(!installed(&addons, "depth").unwrap().signed);
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
    fn another_repository_s_add_on_never_takes_an_installed_or_official_add_on_s_place() {
        let fork = Repo::parse("someone/CutCraft-Depth-fork").unwrap();
        // Cut Craft's own id comes only from its own repository
        let addons = scratch("fork");
        let e = depth_release(false)
            .install_from(fork.clone(), &addons, &platform())
            .unwrap_err();
        assert!(e.contains("NoobyNull/CutCraft-Depth"), "{e}");
        assert!(installed(&addons, "depth").is_none());
        // installed from its repository, it is kept: the fork does not replace it
        depth_release(true).install(&addons, &platform()).unwrap();
        let e = depth_release(true)
            .install_from(fork, &addons, &platform())
            .unwrap_err();
        assert!(e.contains("someone/CutCraft-Depth-fork"), "{e}");
        assert_eq!(installed(&addons, "depth").unwrap().repo, repo());
        // the same repository again is an update, as before
        depth_release(true).install(&addons, &platform()).unwrap();
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
    fn a_zip_asked_for_by_name_is_the_one_offered_and_kept_for_updates() {
        let manifest = |code: &str| {
            format!(
                "id = \"language-{code}\"\nname = \"{code}\"\nkind = \"language\"\nversion = \
                 \"1.0.0\"\nlanguage = \"{code}\""
            )
        };
        let (ru, de) = (manifest("ru"), manifest("de"));
        let fake = fake(
            &[
                ("language-ru.zip", zip(&[("addon.toml", ru.as_bytes())])),
                ("language-de.zip", zip(&[("addon.toml", de.as_bytes())])),
            ],
            true,
        );
        assert_eq!(languages_in(&fake.release), ["de", "ru"]);
        let offer = Offer::pick(
            repo(),
            fake.release.clone(),
            "linux-x86_64",
            Some("language-ru.zip"),
        )
        .unwrap();
        assert_eq!(offer.zip.name, "language-ru.zip");
        assert_eq!(offer.named.as_deref(), Some("language-ru.zip"));
        assert!(offer.signed);
        let e = Offer::pick(
            repo(),
            fake.release,
            "linux-x86_64",
            Some("language-fr.zip"),
        )
        .unwrap_err();
        assert!(e.contains("has no language-fr.zip"), "{e}");
        assert_eq!(language_zip("pt-BR"), "language-pt-BR.zip");
    }

    #[test]
    fn a_language_pack_says_which_language_it_is() {
        let base =
            "id = \"language-ru\"\nname = \"Русский\"\nkind = \"language\"\nversion = \"0.1.0\"";
        let m = Manifest::parse(&format!("{base}\nlanguage = \"ru\"")).unwrap();
        assert_eq!(m.language.as_deref(), Some("ru"));
        // a regional code keeps its capitals; the id, a folder name, is in lower case
        let pt = "id = \"language-pt-br\"\nname = \"Português (Brasil)\"\nkind = \"language\"\nversion \
                  = \"0.1.0\"\nlanguage = \"pt-BR\"";
        assert_eq!(
            Manifest::parse(pt).unwrap().language.as_deref(),
            Some("pt-BR")
        );
        for wrong in [
            "",
            "\nlanguage = \"\"",
            "\nlanguage = \"../ru\"",
            "\nlanguage = \"ru-\"",
        ] {
            let e = Manifest::parse(&format!("{base}{wrong}")).unwrap_err();
            assert!(e.contains("which language"), "{e}");
        }
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
    fn a_zip_that_unpacks_too_big_is_refused() {
        let big = zip(&[("a.bin", &[0_u8; 600][..]), ("b.bin", &[0_u8; 600][..])]);
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&big)).unwrap();
        let dir = scratch("bomb");
        let e = unpack_within(&mut archive, &dir, 1000).unwrap_err();
        assert!(e.contains("unpacks to more than"), "{e}");
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&big)).unwrap();
        assert_eq!(unpack_within(&mut archive, &dir, 1200), Ok(()));
        let _ = fs::remove_dir_all(dir);
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
            zip: None,
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
