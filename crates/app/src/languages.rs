//! Language packs: add-ons of kind `language`, each a folder of `.ftl` files with its
//! `addon.toml` (which names the language), installed from Gadgets ▸ Add-ons like any other
//! add-on and never with PowerWhittle itself. Edit ▸ Options ▸ Language picks one; English is
//! built in, and any words a pack lacks show in English.

use cut_craft_i18n::{Catalogue, tr};
use cut_craft_updater::addon::{self, Manifest, Repo};
use std::path::Path;

/// What Options keeps for the test language: English stretched and accented, to show where
/// a longer language would be cut or pushed out of place.
pub const TEST_LANGUAGE: &str = "test";

/// An installed language pack.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pack {
    /// Its add-on id, the folder it is in, kept in Options.
    pub id: String,
    /// Its name, in its own language ("Русский").
    pub name: String,
    /// The language it is in, such as `ru`.
    pub code: String,
    pub version: String,
    /// Signed with PowerWhittle's release key: official.
    pub signed: bool,
    /// The volunteers who translated it, from its `addon.toml`.
    pub translators: Vec<String>,
    /// The repository it was installed from, when that is recorded.
    pub from: Option<Repo>,
}

/// The language packs installed in `dir` (the add-ons folder), by id.
pub fn installed(dir: &Path) -> Vec<Pack> {
    let mut packs: Vec<Pack> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().join(addon::RECORD).is_file())
        .filter_map(|e| {
            let id = e.file_name().into_string().ok()?;
            let text = std::fs::read_to_string(e.path().join(addon::MANIFEST)).ok()?;
            let manifest = Manifest::parse(&text).ok()?;
            let code = manifest.language.filter(|_| manifest.kind == "language")?;
            let record = addon::installed(dir, &id);
            let signed = record.as_ref().is_some_and(|r| r.signed);
            Some(Pack {
                id,
                name: manifest.name,
                code,
                version: manifest.version,
                signed,
                translators: manifest.translators,
                from: record.map(|r| r.repo),
            })
        })
        .collect();
    packs.sort_by(|a, b| a.id.cmp(&b.id));
    packs
}

/// Reads `pack`'s words from its folder in `dir`: every `.ftl` file in it, by name, with what
/// was wrong in them (lines that will not read are left out; the rest still load).
pub fn load(dir: &Path, pack: &Pack) -> Result<(Catalogue, Vec<String>), String> {
    let folder = dir.join(&pack.id);
    let mut files: Vec<(String, String)> = std::fs::read_dir(&folder)
        .map_err(|e| format!("{}: {e}", folder.display()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "ftl"))
        .filter_map(|p| {
            let name = p.file_name()?.to_string_lossy().into_owned();
            let text = std::fs::read_to_string(&p).ok()?;
            Some((name, text))
        })
        .collect();
    if files.is_empty() {
        return Err(format!("{} has no .ftl files", pack.name));
    }
    files.sort();
    let borrowed: Vec<(&str, &str)> = files
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    Ok(Catalogue::parse(&pack.code, &borrowed))
}

/// Shows every word in `chosen` (a pack's id, [`TEST_LANGUAGE`], or empty for English) from
/// now on. A pack that is no longer installed or will not read leaves English, and says why.
pub fn use_chosen(dir: Option<&Path>, chosen: &str) -> Result<(), String> {
    if chosen.is_empty() {
        cut_craft_i18n::use_language(None);
        return Ok(());
    }
    if chosen == TEST_LANGUAGE {
        cut_craft_i18n::use_language(Some(cut_craft_i18n::pseudo()));
        return Ok(());
    }
    let found = dir.and_then(|dir| {
        installed(dir)
            .into_iter()
            .find(|p| p.id == chosen)
            .map(|p| (dir, p))
    });
    let Some((dir, pack)) = found else {
        cut_craft_i18n::use_language(None);
        return Err(format!(
            "The language pack \"{chosen}\" is not installed any more, so PowerWhittle is in \
             English. Install it again in Gadgets ▸ Add-ons, or pick another language in Edit \
             ▸ Options ▸ Language."
        ));
    };
    match load(dir, &pack) {
        Ok((catalogue, problems)) => {
            for p in &problems {
                log::warn!("language pack {}: {p}", pack.id);
            }
            cut_craft_i18n::use_language(Some(catalogue));
            Ok(())
        }
        Err(e) => {
            cut_craft_i18n::use_language(None);
            Err(format!(
                "The language pack {} could not be read ({e}), so PowerWhittle is in English.",
                pack.name
            ))
        }
    }
}

/// How much of PowerWhittle `pack` covers, as a whole percentage of English's words (0 when it
/// will not read).
pub fn coverage(dir: &Path, pack: &Pack) -> u32 {
    load(dir, pack).map_or(0, |(catalogue, _)| {
        let (done, total) = catalogue.coverage();
        percent(done, total)
    })
}

/// What is said about `pack` beside its name: an official pack is "Official, community
/// supported" (checked and signed, but its words are written and kept up by volunteers),
/// then who translated it, then how much of PowerWhittle it covers once `percent` is known. One
/// that is not signed gets no "Official" line; it is marked not official where it shows.
pub fn support_lines(pack: &Pack, percent: Option<u32>) -> Vec<String> {
    let mut lines = vec![];
    if pack.signed {
        lines.push(tr!("language-community-supported"));
    }
    if !pack.translators.is_empty() {
        lines.push(tr!(
            "language-translated-by",
            names = pack.translators.join(", ")
        ));
    }
    if let Some(percent) = percent {
        lines.push(tr!("language-coverage", percent = percent));
    }
    lines
}

/// Where to report a wrong or missing word in `pack`: the Issues of the repository it came
/// from, or of the translations' own when that is not recorded.
pub fn wording_issues(pack: &Pack) -> String {
    pack.from.as_ref().map_or_else(
        || format!("https://github.com/{}/issues", addon::TRANSLATIONS),
        addon::wording_issues,
    )
}

fn percent(done: usize, total: usize) -> u32 {
    if total == 0 {
        return 0;
    }
    u32::try_from(done * 100 / total).unwrap_or(100)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An add-ons folder holding a Russian pack (signed or not) and a gadget.
    fn folder(name: &str, signed: bool) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cutcraft-languages-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let ru = dir.join("language-ru");
        std::fs::create_dir_all(&ru).unwrap();
        std::fs::write(
            ru.join(addon::MANIFEST),
            "id = \"language-ru\"\nname = \"Русский\"\nkind = \"language\"\nversion = \
             \"0.1.0\"\nlanguage = \"ru\"",
        )
        .unwrap();
        std::fs::write(
            ru.join(addon::RECORD),
            format!(
                "{{\"repo\":{{\"owner\":\"NoobyNull\",\"name\":\"CutCraft-Russian\"}},\"tag\":\
                 \"v0.1.0\",\"version\":\"0.1.0\",\"signed\":{signed}}}"
            ),
        )
        .unwrap();
        std::fs::write(
            ru.join("menus.ftl"),
            "menu-file = Файл\nmenu-edit = Правка\n",
        )
        .unwrap();
        let gadget = dir.join("snowflake");
        std::fs::create_dir_all(&gadget).unwrap();
        std::fs::write(
            gadget.join(addon::MANIFEST),
            "id = \"snowflake\"\nname = \"Snowflake\"\nkind = \"gadget\"\nversion = \"1.0.0\"",
        )
        .unwrap();
        std::fs::write(gadget.join(addon::RECORD), "{}").unwrap();
        dir
    }

    #[test]
    fn only_language_packs_are_listed_with_whether_they_are_official() {
        let dir = folder("list", false);
        let packs = installed(&dir);
        assert_eq!(packs.len(), 1);
        assert_eq!(
            (
                packs[0].id.as_str(),
                packs[0].name.as_str(),
                packs[0].code.as_str()
            ),
            ("language-ru", "Русский", "ru")
        );
        assert!(!packs[0].signed);
        assert!(installed(&folder("signed", true))[0].signed);
        assert_eq!(installed(&dir.join("none")), []);
    }

    #[test]
    fn a_pack_brings_its_words_and_english_fills_the_rest() {
        let dir = folder("load", true);
        let pack = &installed(&dir)[0];
        let (catalogue, problems) = load(&dir, pack).unwrap();
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(catalogue.words("menu-file", None).as_deref(), Some("Файл"));
        // what it lacks is not there, so English shows in its place
        assert!(!catalogue.has("menu-help"));
        let shown = coverage(&dir, pack);
        assert!(shown < 100, "{shown}");
    }

    #[test]
    fn a_pack_no_longer_installed_leaves_english_and_says_so() {
        let dir = folder("gone", true);
        let e = use_chosen(Some(&dir), "language-xx").unwrap_err();
        assert!(e.contains("not installed"), "{e}");
        assert!(use_chosen(Some(&dir), "").is_ok());
        assert!(use_chosen(None, TEST_LANGUAGE).is_ok());
        cut_craft_i18n::use_language(None);
    }

    #[test]
    fn an_official_pack_says_it_is_community_supported_and_who_made_it() {
        let dir = folder("support", true);
        let ru = dir.join("language-ru").join(addon::MANIFEST);
        let text = std::fs::read_to_string(&ru).unwrap();
        std::fs::write(&ru, format!("{text}\ntranslators = [\"Ivan\", \"Olga\"]")).unwrap();
        let pack = &installed(&dir)[0];
        assert_eq!(pack.translators, ["Ivan", "Olga"]);
        assert_eq!(
            support_lines(pack, Some(92)),
            [
                "Official, community supported",
                "Translated by Ivan, Olga",
                "92% translated"
            ]
        );
        // wording problems go to the repository it came from
        assert_eq!(
            wording_issues(pack),
            "https://github.com/NoobyNull/CutCraft-Russian/issues"
        );
        // no names, no "Translated by"; no percentage until it is known
        let mut quiet = pack.clone();
        quiet.translators.clear();
        assert_eq!(
            support_lines(&quiet, None),
            ["Official, community supported"]
        );
        // one that is not signed is not official, which is said elsewhere
        let unsigned = &installed(&folder("support-unsigned", false))[0];
        assert!(
            !support_lines(unsigned, None)
                .iter()
                .any(|l| l.contains("Official"))
        );
    }

    #[test]
    fn a_pack_with_no_record_reports_wording_to_the_translations() {
        let dir = folder("support-norecord", true);
        let mut pack = installed(&dir)[0].clone();
        pack.from = None;
        assert_eq!(
            wording_issues(&pack),
            format!("https://github.com/{}/issues", addon::TRANSLATIONS)
        );
    }

    #[test]
    fn coverage_is_a_whole_percentage() {
        assert_eq!(percent(0, 0), 0);
        assert_eq!(percent(1, 3), 33);
        assert_eq!(percent(41, 41), 100);
    }
}
