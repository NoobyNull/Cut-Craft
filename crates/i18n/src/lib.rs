//! Cut Craft's words in every language, with English wherever a language has no word yet.
//!
//! Every word the program shows lives in a Fluent file (`.ftl`) under a key:
//!
//! ```text
//! commit-calculate = Calculate
//!     .tooltip = Work out the toolpath from these settings
//! start-whats-new = What's new in v{ $version }
//! ```
//!
//! The code asks for the key ([`tr!`]), never the words, so a language pack can swap them.
//!
//! - **English is built in** (`locales/en/*.ftl`, carried into the program when it is built).
//!   It is the source every pack translates, and it always works offline with no pack.
//! - **A language pack** is the same files in another language ([`Catalogue::parse`]). A key the
//!   pack does not have yet, or gets wrong, shows in English rather than as a blank or a key.
//! - **Numbers and lengths** come in already written, as text, by the code that knows the
//!   units (mm or in, how many decimals). Fluent only places them, so a length reads the same in
//!   every language as in the number box beside it. Counts that pick a plural (`{ $count ->`)
//!   come in as numbers.
//! - **A key with a dot** (`commit-calculate.tooltip`) is that message's attribute: a button's
//!   words and its tooltip travel together in one message.
//!
//! [`pseudo`] is a test language: every word stretched and accented, to see what longer
//! languages do to fixed label areas before any real translation exists.

use std::borrow::Cow;
use std::sync::{Arc, LazyLock, PoisonError, RwLock};

use fluent_bundle::FluentResource;
use fluent_bundle::concurrent::FluentBundle;
pub use fluent_bundle::{FluentArgs, FluentValue};
use fluent_syntax::ast;
use unic_langid::LanguageIdentifier;

include!(concat!(env!("OUT_DIR"), "/english.rs"));

/// English's language id.
pub const ENGLISH_ID: &str = "en";
/// The test language's id: English, stretched and accented ([`pseudo`]).
pub const PSEUDO_ID: &str = "en-XA";

/// One language's words, read from its `.ftl` files.
pub struct Catalogue {
    id: String,
    bundle: FluentBundle<FluentResource>,
    /// Every key it has, messages and `message.attribute`s, in file order.
    keys: Vec<String>,
    /// The test language: its words are wrapped in brackets, to show where text is cut.
    pseudo: bool,
}

impl std::fmt::Debug for Catalogue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Catalogue")
            .field("id", &self.id)
            .field("keys", &self.keys.len())
            .finish_non_exhaustive()
    }
}

impl Catalogue {
    /// Reads language `id` (`de`, `pt-BR`) from its files, `(file name, text)` each, with what
    /// was wrong in them, one line each. Lines that will not read, and keys given twice, are
    /// left out; the rest still load, so a pack with one broken line still shows the rest.
    pub fn parse(id: &str, files: &[(&str, &str)]) -> (Self, Vec<String>) {
        let lang: LanguageIdentifier = id.parse().unwrap_or_default();
        let mut bundle = FluentBundle::new_concurrent(vec![lang]);
        // no invisible direction marks around { $values }: they draw as boxes, and every
        // language Cut Craft ships reads left to right
        bundle.set_use_isolating(false);
        let mut keys = Vec::new();
        // every message's name, with or without words of its own: a name given twice
        let mut messages: Vec<String> = Vec::new();
        let mut problems = Vec::new();
        for (name, text) in files {
            let resource = match FluentResource::try_new((*text).to_owned()) {
                Ok(r) => r,
                Err((r, errors)) => {
                    for e in errors {
                        let line = 1 + text[..e.pos.start.min(text.len())].matches('\n').count();
                        problems.push(format!("{name}:{line}: {}", e.kind));
                    }
                    r
                }
            };
            let mut added: Vec<String> = Vec::new();
            for entry in resource.entries() {
                if let ast::Entry::Message(m) = entry {
                    let key = m.id.name.to_owned();
                    if messages.contains(&key) {
                        problems.push(format!("{name}: {key} is given twice"));
                        continue;
                    }
                    let attributes: Vec<String> = m
                        .attributes
                        .iter()
                        .map(|a| format!("{key}.{}", a.id.name))
                        .collect();
                    // a message with only attributes has no words of its own
                    if m.value.is_some() {
                        added.push(key.clone());
                    }
                    added.extend(attributes);
                    messages.push(key);
                }
            }
            // a key given twice is listed above, and the first one stays
            if let Err(errors) = bundle.add_resource(resource) {
                for e in errors {
                    if !matches!(e, fluent_bundle::FluentError::Overriding { .. }) {
                        problems.push(format!("{name}: {e}"));
                    }
                }
            }
            keys.extend(added);
        }
        let catalogue = Self {
            id: id.to_owned(),
            bundle,
            keys,
            pseudo: false,
        };
        (catalogue, problems)
    }

    /// The language's id, as given to [`Catalogue::parse`].
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Every key it has, messages and `message.attribute`s, in file order.
    pub fn keys(&self) -> &[String] {
        &self.keys
    }

    /// Whether it has words for `key`.
    pub fn has(&self, key: &str) -> bool {
        self.keys.iter().any(|k| k == key)
    }

    /// Its words for `key`, or `None` when it has none or they will not write (a `{ $value }`
    /// not given, or misspelt in a pack), so the caller can use English instead.
    pub fn format(&self, key: &str, args: Option<&FluentArgs>) -> Option<String> {
        let (text, wrote) = self.write(key, args)?;
        wrote.then_some(text)
    }

    /// Its words for `key` however they write: a `{ $value }` not given shows its name. For
    /// checking the words themselves, and English's last resort.
    pub fn words(&self, key: &str, args: Option<&FluentArgs>) -> Option<String> {
        self.write(key, args).map(|(text, _)| text)
    }

    /// Its words for `key`, and whether they wrote with no errors.
    fn write(&self, key: &str, args: Option<&FluentArgs>) -> Option<(String, bool)> {
        let (message, attribute) = match key.split_once('.') {
            Some((m, a)) => (m, Some(a)),
            None => (key, None),
        };
        let message = self.bundle.get_message(message)?;
        let pattern = match attribute {
            Some(a) => message.get_attribute(a)?.value(),
            None => message.value()?,
        };
        // a missing { $value } still writes the rest, with the value's name in its place
        let mut errors = Vec::new();
        let text = self.bundle.format_pattern(pattern, args, &mut errors);
        let text = if self.pseudo {
            format!("[{text}]")
        } else {
            text.into_owned()
        };
        Some((text, errors.is_empty()))
    }

    /// How many of English's keys it has words for, of how many.
    pub fn coverage(&self) -> (usize, usize) {
        let english = english().keys();
        let done = english.iter().filter(|k| self.has(k)).count();
        (done, english.len())
    }
}

/// English, built in.
pub fn english() -> &'static Catalogue {
    static ENGLISH_WORDS: LazyLock<Catalogue> =
        LazyLock::new(|| Catalogue::parse(ENGLISH_ID, ENGLISH).0);
    &ENGLISH_WORDS
}

/// The test language: English with every word accented and about 40% longer, in brackets, so
/// text that would be cut or pushed out of place by a longer language shows before any
/// translation is made.
pub fn pseudo() -> Catalogue {
    let mut catalogue = Catalogue::parse(PSEUDO_ID, ENGLISH).0;
    catalogue.bundle.set_transform(Some(stretch));
    catalogue.pseudo = true;
    catalogue
}

/// The test language's words: vowels accented and doubled, a few consonants accented (letters
/// every bundled font has), padded with `~` to at least 40% longer.
fn stretch(text: &str) -> Cow<'_, str> {
    let mut out = String::with_capacity(text.len() * 2);
    for c in text.chars() {
        let (accented, twice) = match c {
            'a' => ('à', true),
            'e' => ('é', true),
            'i' => ('ï', true),
            'o' => ('ö', true),
            'u' => ('ü', true),
            'A' => ('À', true),
            'E' => ('É', true),
            'I' => ('Ï', true),
            'O' => ('Ö', true),
            'U' => ('Ü', true),
            'c' => ('ç', false),
            'C' => ('Ç', false),
            'n' => ('ñ', false),
            'N' => ('Ñ', false),
            'y' => ('ý', false),
            'Y' => ('Ý', false),
            other => (other, false),
        };
        out.push(accented);
        if twice {
            out.push(accented.to_lowercase().next().unwrap_or(accented));
        }
    }
    // German and French run about 40% longer than English: pad to that
    let long = text.chars().count() * 14 / 10;
    while out.chars().count() < long {
        out.push('~');
    }
    Cow::Owned(out)
}

/// The language in use; `None` is English.
static ACTIVE: RwLock<Option<Arc<Catalogue>>> = RwLock::new(None);

/// Switches every word the program shows to `language`, or back to English with `None`.
/// Windows drawn after this use it.
pub fn use_language(language: Option<Catalogue>) {
    let language = language.filter(|l| l.id != ENGLISH_ID).map(Arc::new);
    *ACTIVE.write().unwrap_or_else(PoisonError::into_inner) = language;
}

/// The id of the language in use.
pub fn language() -> String {
    ACTIVE
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        .map_or_else(|| ENGLISH_ID.to_owned(), |l| l.id.clone())
}

/// The words for `key` in the language in use, in English when it has none, or the key
/// itself when English has none either (a mistake the tests catch).
pub fn tr(key: &str) -> String {
    tr_args(key, None)
}

/// [`tr`], with `{ $values }` filled from `args`.
pub fn tr_args(key: &str, args: Option<&FluentArgs>) -> String {
    let active = ACTIVE
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    active
        .and_then(|l| l.format(key, args))
        .or_else(|| english().words(key, args))
        .unwrap_or_else(|| key.to_owned())
}

/// The words for a key, in the language in use: `tr!("start-recent")`, or with values
/// `tr!("start-whats-new", version = env!("CARGO_PKG_VERSION"))`. A length or other measured
/// number goes in already written in the job's units; a count that picks a plural goes in as
/// a number.
#[macro_export]
macro_rules! tr {
    ($key:expr) => {
        $crate::tr($key)
    };
    ($key:expr, $($name:ident = $value:expr),+ $(,)?) => {{
        let mut args = $crate::FluentArgs::new();
        $( args.set(stringify!($name), $value); )+
        $crate::tr_args($key, Some(&args))
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    const GERMAN: &str = "\
commit-calculate = Berechnen
    .tooltip = Werkzeugweg berechnen
start-whats-new = Neu in v{ $version }
";

    fn german() -> Catalogue {
        Catalogue::parse("de", &[("app.ftl", GERMAN)]).0
    }

    #[test]
    fn english_reads_with_no_problems() {
        let (_, problems) = Catalogue::parse(ENGLISH_ID, ENGLISH);
        assert!(problems.is_empty(), "{}", problems.join("\n"));
        assert_ne!(english().keys().len(), 0);
    }

    #[test]
    fn every_english_key_has_words() {
        for key in english().keys() {
            let text = english().words(key, None).unwrap();
            assert!(!text.trim().is_empty(), "{key} is blank");
        }
    }

    #[test]
    fn english_keys_are_lower_case_and_hyphenated() {
        for key in english().keys() {
            let message = key.split('.').next().unwrap();
            assert!(
                message
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "{key}: keys are lower-case words joined by hyphens"
            );
        }
    }

    #[test]
    fn a_language_shows_its_own_words_and_values() {
        let de = german();
        assert_eq!(de.format("commit-calculate", None).unwrap(), "Berechnen");
        assert_eq!(
            de.format("commit-calculate.tooltip", None).unwrap(),
            "Werkzeugweg berechnen"
        );
        let mut args = FluentArgs::new();
        args.set("version", "0.6.18");
        // no invisible direction marks around the value
        assert_eq!(
            de.format("start-whats-new", Some(&args)).unwrap(),
            "Neu in v0.6.18"
        );
        assert_eq!(de.format("no-such-key", None), None);
        assert_eq!(de.format("commit-calculate.nothing", None), None);
    }

    #[test]
    fn keys_list_messages_and_their_attributes() {
        assert_eq!(
            german().keys(),
            [
                "commit-calculate",
                "commit-calculate.tooltip",
                "start-whats-new"
            ]
        );
    }

    #[test]
    fn a_broken_line_leaves_the_rest_of_the_pack_working() {
        let text = "good = Gut\nbad = { $\nalso-good = Auch gut\n";
        let (de, problems) = Catalogue::parse("de", &[("app.ftl", text)]);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].starts_with("app.ftl:2:"), "{problems:?}");
        assert_eq!(de.format("good", None).unwrap(), "Gut");
        assert_eq!(de.format("also-good", None).unwrap(), "Auch gut");
    }

    #[test]
    fn a_message_with_only_attributes_has_no_words_of_its_own() {
        let (de, problems) =
            Catalogue::parse("de", &[("a.ftl", "commit-create =\n    .tooltip = Neu\n")]);
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(de.keys(), ["commit-create.tooltip"]);
        assert!(!de.has("commit-create"));
        let (_, problems) = Catalogue::parse(
            "de",
            &[(
                "a.ftl",
                "commit-create =\n    .tooltip = Neu\ncommit-create = Neu\n",
            )],
        );
        assert_eq!(problems, ["a.ftl: commit-create is given twice"]);
    }

    #[test]
    fn words_that_will_not_write_are_left_to_english() {
        let (de, _) = Catalogue::parse(
            "de",
            &[("a.ftl", "start-whats-new = Neu in v{ $versoin }\n")],
        );
        let mut args = FluentArgs::new();
        args.set("version", "1.0");
        assert_eq!(de.format("start-whats-new", Some(&args)), None);
        assert_eq!(
            de.words("start-whats-new", Some(&args)).unwrap(),
            "Neu in v{$versoin}"
        );
    }

    #[test]
    fn a_key_given_twice_keeps_the_first() {
        let a = ("a.ftl", "same = Erste\n");
        let b = ("b.ftl", "same = Zweite\n");
        let (de, problems) = Catalogue::parse("de", &[a, b]);
        assert_eq!(problems, ["b.ftl: same is given twice"]);
        assert_eq!(de.format("same", None).unwrap(), "Erste");
        assert_eq!(de.keys(), ["same"]);
    }

    #[test]
    fn plurals_follow_the_language() {
        let en = Catalogue::parse(
            "en",
            &[(
                "a.ftl",
                "toolpaths = { $count ->\n    [one] { $count } toolpath\n   *[other] { $count } toolpaths\n}\n",
            )],
        )
        .0;
        let count = |n: u32| {
            let mut args = FluentArgs::new();
            args.set("count", n);
            en.format("toolpaths", Some(&args)).unwrap()
        };
        assert_eq!(count(1), "1 toolpath");
        assert_eq!(count(3), "3 toolpaths");
    }

    #[test]
    fn coverage_counts_english_keys_a_pack_has() {
        let (done, total) = german().coverage();
        assert_eq!(total, english().keys().len());
        assert!(done <= total);
        assert_eq!(english().coverage(), (total, total));
    }

    #[test]
    fn the_test_language_is_longer_and_keeps_its_values() {
        let p = pseudo();
        for key in english().keys() {
            let plain = english().words(key, None).unwrap();
            let long = p.words(key, None).unwrap();
            assert!(long.starts_with('[') && long.ends_with(']'), "{long}");
            if plain.chars().filter(|c| c.is_alphabetic()).count() > 4 {
                assert!(
                    long.chars().count() * 10 >= plain.chars().count() * 13,
                    "{plain} -> {long}"
                );
            }
        }
        assert_eq!(stretch("Cut 12 mm"), "Çüüt 12 mm~~");
    }

    // One test switches languages, so the others never see another language mid-test.
    #[test]
    fn missing_words_fall_back_to_english_then_the_key() {
        use_language(None);
        assert_eq!(language(), ENGLISH_ID);
        let english_word = tr("commit-calculate");
        assert_eq!(english_word, "Calculate");
        use_language(Some(
            Catalogue::parse("de", &[("a.ftl", "commit-create = Erstellen\n")]).0,
        ));
        assert_eq!(language(), "de");
        assert_eq!(tr("commit-create"), "Erstellen");
        // German has no word for it yet: English
        assert_eq!(tr("commit-calculate"), "Calculate");
        // nobody has: the key, which the tests catch
        assert_eq!(tr("no-such-key"), "no-such-key");
        assert_eq!(
            tr!("start-whats-new", version = "1.2.3"),
            "What's new in v1.2.3"
        );
        // German's words will not write (a misspelt value): English
        use_language(Some(
            Catalogue::parse(
                "de",
                &[("a.ftl", "start-whats-new = Neu in v{ $versoin }\n")],
            )
            .0,
        ));
        assert_eq!(
            tr!("start-whats-new", version = "1.2.3"),
            "What's new in v1.2.3"
        );
        use_language(None);
        assert_eq!(tr("commit-create"), "Create");
    }
}
