//! Language packs: add-ons of kind `language`, each a folder of `.ftl` files with its
//! `addon.toml` (which names the language), installed from Gadgets ▸ Add-ons like any other
//! add-on and never with Cut Craft itself. Edit ▸ Options ▸ Language picks one; English is
//! built in, and any words a pack lacks show in English.

use cut_craft_i18n::Catalogue;
use cut_craft_updater::addon::{self, Manifest};
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
    /// Signed with Cut Craft's release key: official.
    pub signed: bool,
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
            let signed = addon::installed(dir, &id).is_some_and(|r| r.signed);
            Some(Pack {
                id,
                name: manifest.name,
                code,
                version: manifest.version,
                signed,
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
            "The language pack \"{chosen}\" is not installed any more, so Cut Craft is in \
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
                "The language pack {} could not be read ({e}), so Cut Craft is in English.",
                pack.name
            ))
        }
    }
}

/// How much of Cut Craft `pack` covers, as a whole percentage of English's words (0 when it
/// will not read).
pub fn coverage(dir: &Path, pack: &Pack) -> u32 {
    load(dir, pack).map_or(0, |(catalogue, _)| {
        let (done, total) = catalogue.coverage();
        percent(done, total)
    })
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
    fn coverage_is_a_whole_percentage() {
        assert_eq!(percent(0, 0), 0);
        assert_eq!(percent(1, 3), 33);
        assert_eq!(percent(41, 41), 100);
    }
}
