//! Content packs: optional sets of models, pictures and drawings installed into the Library.
//!
//! A pack is a folder with a `pack.json` at its top listing every file in it: a title for
//! the Library, tags, where it came from, its licence and a SHA-256 checksum. The installer
//! unpacks a pack's release into a folder of its own and adds that folder to the Library,
//! so its files appear there like any others; the licences feed Help ▸ Credits.
//!
//! Only licences that let people sell what they make are allowed (CC0, public domain,
//! CC BY), so nothing in a pack can stop a user selling a carving. Non-commercial and
//! no-derivatives licences are refused when the pack is read.

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::fmt::{self, Write as _};
use std::path::{Path, PathBuf};

/// The file at a pack's top that lists what is in it.
pub const MANIFEST: &str = "pack.json";
/// The add-on installer's own manifest beside it: a pack installs like any add-on, as
/// kind `content`, into its add-on folder.
pub const ADDON_MANIFEST: &str = "addon.toml";
const FORMAT: &str = "cut-craft-pack";
/// Bump when the manifest's shape changes, and teach [`Pack::parse`] to read the old one.
const VERSION: u32 = 1;

/// A content pack's `pack.json`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pack {
    pub format: String,
    pub version: u32,
    /// Short, lower-case and hyphenated (`staunton-chess`): the folder it installs into.
    pub id: String,
    /// What the Library and the Add-ons list call it.
    pub name: String,
    pub description: String,
    /// The pack's own version (`1.0.0`), apart from the app's.
    pub pack_version: String,
    /// The licence of every item that doesn't name its own.
    pub licence: Licence,
    /// Who made the pack; items can name their own authors.
    pub author: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    pub items: Vec<Item>,
}

/// A file in a pack.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Item {
    /// Where it is in the pack, with `/` between folders on every system.
    pub file: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// Library tags, so a search finds it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Its own licence, when not the pack's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub licence: Option<Licence>,
    /// Who made it, when not the pack's author.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Where it came from (a web page), for credits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Its SHA-256 checksum in lower-case hex.
    pub sha256: String,
}

/// The licences a pack's items may have: each lets people sell what they make with them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Licence {
    #[serde(rename = "CC0-1.0")]
    Cc0,
    #[serde(rename = "public-domain")]
    PublicDomain,
    #[serde(rename = "CC-BY-4.0")]
    CcBy4,
    #[serde(rename = "CC-BY-3.0")]
    CcBy3,
}

impl Licence {
    /// Whether the author must be credited.
    pub fn needs_credit(self) -> bool {
        matches!(self, Self::CcBy4 | Self::CcBy3)
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Cc0 => "CC0 1.0 (public domain dedication)",
            Self::PublicDomain => "Public domain",
            Self::CcBy4 => "Creative Commons Attribution 4.0",
            Self::CcBy3 => "Creative Commons Attribution 3.0",
        }
    }
}

/// Why a `pack.json` can't be used, each saying what to fix.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PackError {
    NotAPack(String),
    NewerVersion(u32),
    /// A licence that isn't one of [`Licence`]'s, such as a non-commercial one.
    Licence(String),
    BadId(String),
    /// A file path that is empty, absolute, goes up out of the pack or uses `\`.
    BadPath(String),
    Duplicate(String),
    BadChecksum(String),
    /// A CC BY item without the source its credit must name.
    NoSource(String),
    Empty,
}

impl fmt::Display for PackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAPack(why) => write!(f, "this is not a Cut Craft content pack ({why})"),
            Self::NewerVersion(v) => write!(
                f,
                "this pack needs a newer Cut Craft (pack format {v}; this version reads up to \
                 {VERSION}). Update Cut Craft from Help ▸ Check for Updates, then install it again"
            ),
            Self::Licence(l) => write!(
                f,
                "the licence \"{l}\" isn't allowed in a pack: packs may only hold CC0, public \
                 domain or CC BY items, so everyone can sell what they make with them"
            ),
            Self::BadId(id) => write!(
                f,
                "the pack id \"{id}\" must be lower-case letters, digits and hyphens"
            ),
            Self::BadPath(p) => write!(
                f,
                "the file \"{p}\" must be a path inside the pack, with / between folders"
            ),
            Self::Duplicate(p) => write!(f, "the file \"{p}\" is listed twice"),
            Self::BadChecksum(p) => write!(
                f,
                "the checksum of \"{p}\" must be 64 lower-case hex digits (SHA-256)"
            ),
            Self::NoSource(p) => write!(
                f,
                "\"{p}\" is under a CC BY licence, so its source (where it came from) must be \
                 listed for its credit"
            ),
            Self::Empty => f.write_str("the pack lists no files"),
        }
    }
}

impl std::error::Error for PackError {}

/// Something wrong with an unpacked pack's files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Problem {
    Missing(String),
    /// The file isn't the one the pack lists: damaged, or changed since.
    Changed(String),
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(p) => write!(f, "\"{p}\" is missing; install the pack again"),
            Self::Changed(p) => write!(
                f,
                "\"{p}\" is damaged or was changed; install the pack again"
            ),
        }
    }
}

/// `bytes`' SHA-256 checksum in lower-case hex.
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut hex, b| {
            let _ = write!(hex, "{b:02x}");
            hex
        })
}

impl Item {
    /// An item for `bytes` at `file`, its checksum worked out, the rest left plain.
    pub fn new(file: &str, title: &str, bytes: &[u8]) -> Self {
        Self {
            file: file.to_owned(),
            title: title.to_owned(),
            description: String::new(),
            tags: Vec::new(),
            licence: None,
            author: None,
            source: None,
            sha256: sha256_hex(bytes),
        }
    }

    /// Where it is once the pack is unpacked at `root`, in this system's own separators.
    pub fn path(&self, root: &Path) -> PathBuf {
        self.file
            .split('/')
            .fold(root.to_path_buf(), |p, part| p.join(part))
    }
}

impl Pack {
    /// An empty pack in the current format; add its items, then [`Pack::to_json`].
    pub fn new(id: &str, name: &str, licence: Licence, author: &str) -> Self {
        Self {
            format: FORMAT.to_owned(),
            version: VERSION,
            id: id.to_owned(),
            name: name.to_owned(),
            description: String::new(),
            pack_version: "1.0.0".to_owned(),
            licence,
            author: author.to_owned(),
            homepage: None,
            items: Vec::new(),
        }
    }

    /// Reads and checks a `pack.json`.
    pub fn parse(bytes: &[u8]) -> Result<Self, PackError> {
        #[derive(Deserialize)]
        struct Header {
            format: String,
            version: u32,
        }
        let header: Header =
            serde_json::from_slice(bytes).map_err(|e| PackError::NotAPack(e.to_string()))?;
        if header.format != FORMAT {
            return Err(PackError::NotAPack(format!("format \"{}\"", header.format)));
        }
        if header.version > VERSION {
            return Err(PackError::NewerVersion(header.version));
        }
        let value: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|e| PackError::NotAPack(e.to_string()))?;
        // a licence we don't allow is named in the error, not lost in serde's
        let licences = std::iter::once(value.get("licence"))
            .chain(
                (value
                    .get("items")
                    .and_then(|i| i.as_array())
                    .into_iter()
                    .flatten())
                .map(|item| item.get("licence")),
            )
            .flatten();
        for licence in licences {
            if serde_json::from_value::<Licence>(licence.clone()).is_err() {
                return Err(PackError::Licence(
                    licence
                        .as_str()
                        .map_or_else(|| licence.to_string(), str::to_owned),
                ));
            }
        }
        let pack: Self =
            serde_json::from_value(value).map_err(|e| PackError::NotAPack(e.to_string()))?;
        pack.check()?;
        Ok(pack)
    }

    fn check(&self) -> Result<(), PackError> {
        let id_ok = !self.id.is_empty()
            && !self.id.starts_with('-')
            && (self.id.bytes()).all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
        if !id_ok {
            return Err(PackError::BadId(self.id.clone()));
        }
        if self.items.is_empty() {
            return Err(PackError::Empty);
        }
        let mut seen = std::collections::HashSet::new();
        for item in &self.items {
            let parts_ok = !item.file.is_empty()
                && !item.file.contains('\\')
                && !item.file.contains(':')
                && (item.file.split('/')).all(|part| !matches!(part, "" | "." | ".."));
            if !parts_ok || item.file == MANIFEST || item.file == ADDON_MANIFEST {
                return Err(PackError::BadPath(item.file.clone()));
            }
            if !seen.insert(item.file.to_lowercase()) {
                return Err(PackError::Duplicate(item.file.clone()));
            }
            let hex = item.sha256.len() == 64
                && (item.sha256.bytes()).all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
            if !hex {
                return Err(PackError::BadChecksum(item.file.clone()));
            }
            let sourced = item.source.as_deref().is_some_and(|s| !s.trim().is_empty());
            if self.licence_of(item).needs_credit() && !sourced {
                return Err(PackError::NoSource(item.file.clone()));
            }
        }
        Ok(())
    }

    /// The `addon.toml` the add-on installer reads: the same id, as kind `content`.
    pub fn addon_toml(&self) -> String {
        let quoted = |s: &str| serde_json::to_string(s).expect("strings always serialize");
        format!(
            "id = {}\nname = {}\nkind = \"content\"\nversion = {}\n",
            quoted(&self.id),
            quoted(&self.name),
            quoted(&self.pack_version)
        )
    }

    pub fn to_json(&self) -> Vec<u8> {
        let mut out = serde_json::to_vec_pretty(self).expect("packs always serialize");
        out.push(b'\n');
        out
    }

    /// The licence `item` is under.
    pub fn licence_of(&self, item: &Item) -> Licence {
        item.licence.unwrap_or(self.licence)
    }

    /// Whether the pack unpacked at `root` has every file it lists, unchanged.
    pub fn verify(&self, root: &Path) -> Vec<Problem> {
        (self.items.iter())
            .filter_map(|item| match std::fs::read(item.path(root)) {
                Err(_) => Some(Problem::Missing(item.file.clone())),
                Ok(bytes) if sha256_hex(&bytes) != item.sha256 => {
                    Some(Problem::Changed(item.file.clone()))
                }
                Ok(_) => None,
            })
            .collect()
    }

    /// A line for each item whose licence asks for credit: "Title by Author (licence), source".
    pub fn credits(&self) -> Vec<String> {
        (self.items.iter())
            .filter(|item| self.licence_of(item).needs_credit())
            .map(|item| {
                let author = item.author.as_deref().unwrap_or(&self.author);
                let mut line = format!(
                    "{} by {author} ({})",
                    item.title,
                    self.licence_of(item).name()
                );
                if let Some(source) = &item.source {
                    let _ = write!(line, ", {source}");
                }
                line
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Pack {
        let mut pack = Pack::new(
            "staunton-chess",
            "Staunton Chess Set",
            Licence::Cc0,
            "Cut Craft",
        );
        pack.items
            .push(Item::new("Pieces/Pawn half.stl", "Pawn", b"pawn"));
        pack
    }

    #[test]
    fn a_pack_reads_back_as_written() {
        let pack = sample();
        assert_eq!(Pack::parse(&pack.to_json()).unwrap(), pack);
    }

    #[test]
    fn checksums_are_sha256_in_lower_case_hex() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn other_files_and_newer_formats_are_refused() {
        assert!(matches!(Pack::parse(b"{}"), Err(PackError::NotAPack(_))));
        assert!(matches!(
            Pack::parse(br#"{"format": "something-else", "version": 1}"#),
            Err(PackError::NotAPack(_))
        ));
        let mut newer = sample();
        newer.version = VERSION + 1;
        assert_eq!(
            Pack::parse(&newer.to_json()),
            Err(PackError::NewerVersion(VERSION + 1))
        );
    }

    #[test]
    fn non_commercial_and_no_derivatives_licences_are_refused_by_name() {
        let json = String::from_utf8(sample().to_json()).unwrap();
        let pack_nc = json.replacen("\"CC0-1.0\"", "\"CC-BY-NC-4.0\"", 1);
        assert_eq!(
            Pack::parse(pack_nc.as_bytes()),
            Err(PackError::Licence("CC-BY-NC-4.0".into()))
        );
        let item_nd = json.replace(
            "\"sha256\"",
            "\"licence\": \"CC-BY-NC-ND-4.0\",\n      \"sha256\"",
        );
        let err = Pack::parse(item_nd.as_bytes()).unwrap_err();
        assert_eq!(err, PackError::Licence("CC-BY-NC-ND-4.0".into()));
        assert!(err.to_string().contains("sell"), "{err}");
    }

    #[test]
    fn files_must_stay_inside_the_pack() {
        for bad in [
            "",
            "../evil.stl",
            "a/../../evil.stl",
            "/etc/passwd",
            "C:/x.stl",
            "a\\b.stl",
            "a//b.stl",
            "./a.stl",
            "pack.json",
            "addon.toml",
        ] {
            let mut pack = sample();
            pack.items[0].file = bad.into();
            assert_eq!(
                Pack::parse(&pack.to_json()),
                Err(PackError::BadPath(bad.into())),
                "{bad}"
            );
        }
    }

    #[test]
    fn ids_names_and_checksums_are_checked() {
        let mut pack = sample();
        pack.id = "Staunton Chess".into();
        assert!(matches!(
            Pack::parse(&pack.to_json()),
            Err(PackError::BadId(_))
        ));
        let mut pack = sample();
        pack.items
            .push(Item::new("pieces/pawn HALF.stl", "Again", b"x"));
        assert!(matches!(
            Pack::parse(&pack.to_json()),
            Err(PackError::Duplicate(_))
        ));
        let mut pack = sample();
        pack.items[0].sha256 = "ABC".into();
        assert!(matches!(
            Pack::parse(&pack.to_json()),
            Err(PackError::BadChecksum(_))
        ));
        let mut pack = sample();
        pack.items.clear();
        assert_eq!(Pack::parse(&pack.to_json()), Err(PackError::Empty));
    }

    #[test]
    fn an_unpacked_pack_is_checked_file_by_file() {
        let root = std::env::temp_dir().join(format!("cc-pack-verify-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut pack = sample();
        pack.items.push(Item::new("Board.svg", "Board", b"board"));
        pack.items.push(Item::new("Lost.stl", "Lost", b"lost"));
        std::fs::create_dir_all(root.join("Pieces")).unwrap();
        std::fs::write(root.join("Pieces").join("Pawn half.stl"), b"pawn").unwrap();
        std::fs::write(root.join("Board.svg"), b"changed").unwrap();
        assert_eq!(
            pack.verify(&root),
            [
                Problem::Changed("Board.svg".into()),
                Problem::Missing("Lost.stl".into())
            ]
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_pack_installs_as_a_content_add_on_of_the_same_id() {
        let mut pack = sample();
        pack.name = "Chess \"Staunton\"".into();
        assert_eq!(
            pack.addon_toml(),
            "id = \"staunton-chess\"\nname = \"Chess \\\"Staunton\\\"\"\nkind = \"content\"\nversion = \"1.0.0\"\n"
        );
    }

    #[test]
    fn a_cc_by_item_must_say_where_it_came_from() {
        let mut pack = sample();
        pack.items[0].licence = Some(Licence::CcBy4);
        assert_eq!(
            Pack::parse(&pack.to_json()),
            Err(PackError::NoSource("Pieces/Pawn half.stl".into()))
        );
        pack.items[0].source = Some("https://example.org/pawn".into());
        assert!(Pack::parse(&pack.to_json()).is_ok());
    }

    #[test]
    fn only_attribution_licences_are_credited() {
        let mut pack = sample();
        let mut icon = Item::new("Icons/knight.svg", "Knight icon", b"k");
        icon.licence = Some(Licence::CcBy3);
        icon.author = Some("Lorc".into());
        icon.source = Some("https://game-icons.net".into());
        pack.items.push(icon);
        assert_eq!(
            pack.credits(),
            ["Knight icon by Lorc (Creative Commons Attribution 3.0), https://game-icons.net"]
        );
    }
}
