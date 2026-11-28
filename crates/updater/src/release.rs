//! A release on GitHub: its version, notes and downloads, read from the releases API.

use semver::Version;
use serde_json::Value;

/// One download attached to a release.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: u64,
    /// GitHub's checksum of the file, as `sha256:<hex>`; old releases have none.
    pub digest: Option<String>,
}

/// A published release.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    /// The tag, such as `v0.4.4`.
    pub tag: String,
    pub version: Version,
    /// What changed, as written on the release page (Markdown).
    pub notes: String,
    /// The release's page, for downloading by hand.
    pub page: String,
    pub assets: Vec<Asset>,
}

impl Release {
    /// Reads one release from the GitHub API's JSON. Drafts, pre-releases (the nightly) and
    /// tags that are not a version are refused.
    pub fn parse(json: &str) -> Result<Self, String> {
        let v: Value = serde_json::from_str(json)
            .map_err(|e| format!("GitHub sent something that is not a release ({e})"))?;
        Self::from_value(&v)
    }

    /// Reads the GitHub API's list of releases, leaving out drafts, pre-releases and tags
    /// that are not a version.
    pub fn parse_list(json: &str) -> Result<Vec<Self>, String> {
        let v: Value = serde_json::from_str(json)
            .map_err(|e| format!("GitHub sent something that is not a list of releases ({e})"))?;
        let list = v
            .as_array()
            .ok_or("GitHub sent something that is not a list of releases")?;
        Ok(list
            .iter()
            .filter_map(|r| Self::from_value(r).ok())
            .collect())
    }

    fn from_value(v: &Value) -> Result<Self, String> {
        let text = |key: &str| v.get(key).and_then(Value::as_str).unwrap_or_default();
        let flag = |key: &str| v.get(key).and_then(Value::as_bool).unwrap_or(false);
        if flag("draft") || flag("prerelease") {
            return Err("the newest release on GitHub is a test build".to_owned());
        }
        let tag = text("tag_name").to_owned();
        let version = version_of(&tag)
            .ok_or_else(|| format!("the release tag \"{tag}\" is not a version number"))?;
        let assets = v
            .get("assets")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|a| {
                Some(Asset {
                    name: a.get("name")?.as_str()?.to_owned(),
                    url: a.get("browser_download_url")?.as_str()?.to_owned(),
                    size: a.get("size").and_then(Value::as_u64).unwrap_or(0),
                    digest: a.get("digest").and_then(Value::as_str).map(str::to_owned),
                })
            })
            .collect();
        Ok(Self {
            tag,
            version,
            notes: text("body").trim().to_owned(),
            page: text("html_url").to_owned(),
            assets,
        })
    }

    /// Whether this release is newer than the running `current` version.
    pub fn is_newer_than(&self, current: &Version) -> bool {
        self.version > *current
    }

    /// The minisign signature published beside `asset`: `<name>.minisig`.
    pub fn signature_of(&self, asset: &Asset) -> Option<&Asset> {
        let name = format!("{}.minisig", asset.name);
        self.assets.iter().find(|a| a.name == name)
    }

    /// The download whose name ends with `suffix` (see [`crate::Install::asset_suffix`]).
    pub fn asset(&self, suffix: &str) -> Option<&Asset> {
        self.assets
            .iter()
            .find(|a| a.name.ends_with(suffix) && !a.name.ends_with(".minisig"))
    }

    /// Whether the download ending `suffix` and its signature are both attached yet.
    pub fn has_signed(&self, suffix: &str) -> bool {
        self.asset(suffix)
            .is_some_and(|a| self.signature_of(a).is_some())
    }
}

/// The newest of `releases` with a signed download ending `suffix`: the newest release for
/// that kind of computer. Each computer's builds are attached to a release as they finish
/// (Linux, Windows and macOS are built apart), so a release only counts for a computer once
/// its own file and signature are there. Of two the same version (one release on two
/// repositories), the first listed.
pub fn newest_for<'a>(releases: &'a [Release], suffix: &str) -> Option<&'a Release> {
    releases
        .iter()
        .filter(|r| r.has_signed(suffix))
        .fold(None, |best: Option<&Release>, r| match best {
            Some(b) if b.version >= r.version => Some(b),
            _ => Some(r),
        })
}

/// The version a tag names: `v0.4.3` or `0.4.3`.
pub fn version_of(tag: &str) -> Option<Version> {
    Version::parse(tag.strip_prefix('v').unwrap_or(tag)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const V043: &str = r#"{
        "tag_name": "v0.4.3", "draft": false, "prerelease": false,
        "html_url": "https://github.com/Power-Whittle/PowerWhittle/releases/tag/v0.4.3",
        "body": "  **Full Changelog**: v0.4.2...v0.4.3\n",
        "assets": [
            {"name": "cut-craft-v0.4.3-linux-x86_64.tar.gz", "size": 14704949,
             "digest": "sha256:14033911f1eea6a2c5ea736ae80cf3139aa8028b96c47e620d050306d29f9a06",
             "browser_download_url": "https://github.com/x/linux.tar.gz"},
            {"name": "cut-craft-v0.4.3-windows-x86_64.zip", "size": 14036928,
             "browser_download_url": "https://github.com/x/windows.zip"}
        ]
    }"#;

    #[test]
    fn reads_a_release_from_github() {
        let r = Release::parse(V043).unwrap();
        assert_eq!(r.tag, "v0.4.3");
        assert_eq!(r.version, Version::new(0, 4, 3));
        assert_eq!(r.notes, "**Full Changelog**: v0.4.2...v0.4.3");
        assert!(r.page.ends_with("/v0.4.3"));
        assert_eq!(r.assets.len(), 2);
        assert_eq!(r.assets[0].size, 14_704_949);
        assert!(
            r.assets[0]
                .digest
                .as_deref()
                .unwrap()
                .starts_with("sha256:")
        );
        // older releases were published before GitHub kept checksums
        assert_eq!(r.assets[1].digest, None);
    }

    #[test]
    fn test_builds_and_odd_tags_are_not_updates() {
        let nightly = V043.replace(r#""prerelease": false"#, r#""prerelease": true"#);
        assert!(Release::parse(&nightly).is_err());
        let draft = V043.replace(r#""draft": false"#, r#""draft": true"#);
        assert!(Release::parse(&draft).is_err());
        let odd = V043.replace("v0.4.3\", \"draft", "latest\", \"draft");
        assert!(Release::parse(&odd).unwrap_err().contains("latest"));
        assert!(Release::parse("<html>rate limited</html>").is_err());
    }

    #[test]
    fn only_a_higher_version_is_newer() {
        let r = Release::parse(V043).unwrap();
        assert!(r.is_newer_than(&Version::new(0, 4, 2)));
        assert!(r.is_newer_than(&Version::new(0, 3, 9)));
        assert!(!r.is_newer_than(&Version::new(0, 4, 3)));
        assert!(!r.is_newer_than(&Version::new(0, 10, 0)));
        // 0.4.3 is newer than a 0.4.3 test version
        assert!(r.is_newer_than(&Version::parse("0.4.3-beta.1").unwrap()));
    }

    #[test]
    fn picks_the_download_for_this_computer() {
        let r = Release::parse(V043).unwrap();
        assert!(
            r.asset("windows-x86_64.zip")
                .unwrap()
                .url
                .ends_with("windows.zip")
        );
        assert!(r.asset("linux-x86_64.tar.gz").is_some());
        assert!(r.asset("x86_64.AppImage").is_none());
    }

    /// Each computer's builds are added to a release as they finish, so for a while a
    /// release can have the Linux files and not yet the Windows or Mac ones (or a file
    /// before its signature). A computer is offered only a release with its own file and
    /// that file's signature; until then it keeps being offered the one before.
    #[test]
    fn a_release_counts_for_a_computer_only_once_its_signed_file_is_there() {
        let list = format!(
            "[{}, {}, {}, {}]",
            // the newest: Linux done, Windows's file there but not yet its signature
            r#"{"tag_name": "v0.4.5", "draft": false, "prerelease": false, "assets": [
                {"name": "cut-craft-0.4.5-linux-x86_64.tar.gz", "browser_download_url": "l"},
                {"name": "cut-craft-0.4.5-linux-x86_64.tar.gz.minisig", "browser_download_url": "l"},
                {"name": "cut-craft-0.4.5-windows-x86_64.zip", "browser_download_url": "w"}]}"#,
            // the nightly is never an update
            r#"{"tag_name": "nightly", "draft": false, "prerelease": true, "assets": [
                {"name": "cut-craft-nightly-windows-x86_64.zip", "browser_download_url": "w"},
                {"name": "cut-craft-nightly-windows-x86_64.zip.minisig", "browser_download_url": "w"}]}"#,
            V043,
            // everything there; listed out of order
            r#"{"tag_name": "v0.4.4", "draft": false, "prerelease": false, "assets": [
                {"name": "cut-craft-0.4.4-linux-x86_64.tar.gz", "browser_download_url": "l"},
                {"name": "cut-craft-0.4.4-linux-x86_64.tar.gz.minisig", "browser_download_url": "l"},
                {"name": "cut-craft-0.4.4-windows-x86_64.zip", "browser_download_url": "w"},
                {"name": "cut-craft-0.4.4-windows-x86_64.zip.minisig", "browser_download_url": "w"}]}"#,
        );
        let releases = Release::parse_list(&list).unwrap();
        assert_eq!(releases.len(), 3, "the nightly is left out");
        let newest = |suffix| newest_for(&releases, suffix).map(|r| r.tag.clone());
        assert_eq!(newest("linux-x86_64.tar.gz").as_deref(), Some("v0.4.5"));
        assert_eq!(newest("windows-x86_64.zip").as_deref(), Some("v0.4.4"));
        // no Mac build yet in any of them
        assert_eq!(newest(".dmg"), None);
        assert!(
            Release::parse_list(V043).is_err(),
            "one release is not a list"
        );
        assert!(Release::parse_list("<html>rate limited</html>").is_err());
    }

    #[test]
    fn tags_with_and_without_v() {
        assert_eq!(version_of("v1.2.3"), Some(Version::new(1, 2, 3)));
        assert_eq!(version_of("1.2.3"), Some(Version::new(1, 2, 3)));
        assert_eq!(version_of("nightly"), None);
    }
}
