//! Spelling for Text: words checked against a Hunspell dictionary (the `.dic` word list
//! and the prefix and suffix rules of its `.aff` file) and the user's own words, with
//! suggestions one edit away.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

/// One letter of an affix rule's condition.
#[derive(Debug)]
enum Class {
    Any,
    In(Vec<char>),
    NotIn(Vec<char>),
}

impl Class {
    fn matches(&self, c: char) -> bool {
        match self {
            Self::Any => true,
            Self::In(set) => set.contains(&c),
            Self::NotIn(set) => !set.contains(&c),
        }
    }
}

/// A prefix or suffix rule: what comes off the dictionary word, what goes on, and what the
/// dictionary word must start (prefix) or end (suffix) with.
#[derive(Debug)]
struct Affix {
    flag: char,
    strip: String,
    add: String,
    condition: Vec<Class>,
    cross: bool,
}

fn condition(text: &str) -> Vec<Class> {
    let mut out = vec![];
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        match c {
            '.' => out.push(Class::Any),
            '[' => {
                let mut set: Vec<char> = chars.by_ref().take_while(|&c| c != ']').collect();
                if set.first() == Some(&'^') {
                    set.remove(0);
                    out.push(Class::NotIn(set));
                } else {
                    out.push(Class::In(set));
                }
            }
            c => out.push(Class::In(vec![c])),
        }
    }
    out
}

/// A dictionary: its words with their rule flags, its rules, the letters it tries for
/// suggestions, and the user's own words.
#[derive(Debug, Default)]
pub struct Speller {
    words: HashMap<String, Vec<char>>,
    prefixes: Vec<Affix>,
    suffixes: Vec<Affix>,
    tries: Vec<char>,
    user: BTreeSet<String>,
}

impl Speller {
    /// A dictionary from a Hunspell `.aff` and `.dic` (one-letter flags, as English ones).
    #[must_use]
    pub fn from_hunspell(aff: &str, dic: &str) -> Self {
        let mut s = Self::default();
        let mut cross: HashMap<(bool, char), bool> = HashMap::new();
        for line in aff.lines() {
            let fields: Vec<&str> = line.split_whitespace().collect();
            match fields.as_slice() {
                ["TRY", letters, ..] => s.tries = letters.chars().collect(),
                [kind @ ("PFX" | "SFX"), flag, yn @ ("Y" | "N"), count]
                    if count.parse::<usize>().is_ok() =>
                {
                    let flag = flag.chars().next().unwrap_or(' ');
                    cross.insert((*kind == "PFX", flag), *yn == "Y");
                }
                [kind @ ("PFX" | "SFX"), flag, strip, add, rest @ ..] => {
                    let prefix = *kind == "PFX";
                    let flag = flag.chars().next().unwrap_or(' ');
                    let bare = |t: &str| {
                        if t == "0" {
                            String::new()
                        } else {
                            t.to_owned()
                        }
                    };
                    let add = add.split('/').next().unwrap_or_default();
                    let rule = Affix {
                        flag,
                        strip: bare(strip),
                        add: bare(add),
                        condition: condition(rest.first().copied().unwrap_or(".")),
                        cross: cross.get(&(prefix, flag)).copied().unwrap_or(false),
                    };
                    if prefix {
                        &mut s.prefixes
                    } else {
                        &mut s.suffixes
                    }
                    .push(rule);
                }
                _ => {}
            }
        }
        if s.tries.is_empty() {
            s.tries = ('a'..='z').collect();
        }
        for line in dic.lines().skip(1) {
            let entry = line.split(['\t', ' ']).next().unwrap_or_default();
            let (word, flags) = entry.split_once('/').unwrap_or((entry, ""));
            if !word.is_empty() {
                s.words
                    .entry(word.to_owned())
                    .or_default()
                    .extend(flags.chars());
            }
        }
        s
    }

    /// The first dictionary found: the user's (a `.dic` beside its `.aff` in Cut Craft's
    /// `dictionaries` folder), else the system's English one.
    #[must_use]
    pub fn find() -> Option<Self> {
        let mut dics: Vec<PathBuf> = crate::file::config_dir()
            .map(|d| d.join("dictionaries"))
            .and_then(|d| std::fs::read_dir(d).ok())
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "dic"))
            .collect();
        // ponytail: English first, as Cut Craft is in English
        for folder in [
            "/usr/share/hunspell",
            "/usr/share/myspell/dicts",
            "/usr/share/myspell",
            "/Library/Spelling",
        ] {
            for name in ["en_US", "en_GB", "en_CA", "en_AU"] {
                dics.push(Path::new(folder).join(format!("{name}.dic")));
            }
        }
        dics.iter().find_map(|dic| {
            let read = |p: &Path| {
                std::fs::read(p)
                    .ok()
                    .map(|b| String::from_utf8_lossy(&b).into_owned())
            };
            Some(Self::from_hunspell(
                &read(&dic.with_extension("aff"))?,
                &read(dic)?,
            ))
        })
    }

    /// With `words` as the user's own.
    #[must_use]
    pub fn with_user_words<'a>(mut self, words: impl IntoIterator<Item = &'a str>) -> Self {
        for w in words {
            self.add_word(w);
        }
        self
    }

    pub fn add_word(&mut self, word: &str) {
        let w = word.trim();
        if !w.is_empty() {
            self.user.insert(w.to_lowercase());
        }
    }

    /// Takes back a word the user added; the dictionary's own stay.
    pub fn remove_word(&mut self, word: &str) {
        self.user.remove(&word.trim().to_lowercase());
    }

    #[must_use]
    pub fn user_words(&self) -> &BTreeSet<String> {
        &self.user
    }

    fn has(&self, root: &str, flag: char, also: Option<char>) -> bool {
        self.words
            .get(root)
            .is_some_and(|f| f.contains(&flag) && also.is_none_or(|a| f.contains(&a)))
    }

    /// `word` as a dictionary word with one of its suffixes (also carrying `also`).
    fn suffixed(&self, word: &str, also: Option<char>) -> bool {
        self.suffixes.iter().any(|r| {
            (also.is_none() || r.cross)
                && word.strip_suffix(r.add.as_str()).is_some_and(|base| {
                    let root = format!("{base}{}", r.strip);
                    let chars: Vec<char> = root.chars().collect();
                    !base.is_empty()
                        && chars.len() >= r.condition.len()
                        && r.condition
                            .iter()
                            .zip(&chars[chars.len() - r.condition.len()..])
                            .all(|(class, &c)| class.matches(c))
                        && self.has(&root, r.flag, also)
                })
        })
    }

    /// `word` as a dictionary word with one of its prefixes, and perhaps a suffix too.
    fn prefixed(&self, word: &str) -> bool {
        self.prefixes.iter().any(|r| {
            word.strip_prefix(r.add.as_str()).is_some_and(|rest| {
                let root = format!("{}{rest}", r.strip);
                let chars: Vec<char> = root.chars().collect();
                !rest.is_empty()
                    && chars.len() >= r.condition.len()
                    && r.condition
                        .iter()
                        .zip(&chars)
                        .all(|(class, &c)| class.matches(c))
                    && (self.has(&root, r.flag, None)
                        || (r.cross && self.suffixed(&root, Some(r.flag))))
            })
        })
    }

    fn known(&self, word: &str) -> bool {
        self.words.contains_key(word) || self.suffixed(word, None) || self.prefixed(word)
    }

    /// Whether `word` is spelt right: a dictionary word (as written, in lower case, or
    /// capitalised), or one of the user's; words with digits are let be.
    #[must_use]
    pub fn correct(&self, word: &str) -> bool {
        let lower = word.to_lowercase();
        let mut cap = lower.chars();
        let capital: String = cap
            .next()
            .map_or_default(|c| c.to_uppercase().chain(cap).collect());
        word.chars().any(|c| c.is_ascii_digit())
            || self.user.contains(&lower)
            || self.known(word)
            || self.known(&lower)
            || self.known(&capital)
    }

    /// Words one edit (a letter left out, added, changed, or two swapped) from `word` that
    /// are spelt right, at most six, capitalised as `word` is.
    #[must_use]
    pub fn suggest(&self, word: &str) -> Vec<String> {
        let lower: Vec<char> = word.to_lowercase().chars().collect();
        let n = lower.len();
        let mut tries: Vec<String> = vec![];
        let make = |v: Vec<char>| v.into_iter().collect::<String>();
        for i in 0..n {
            let mut v = lower.clone();
            v.remove(i);
            tries.push(make(v));
            if i + 1 < n {
                let mut v = lower.clone();
                v.swap(i, i + 1);
                tries.push(make(v));
            }
        }
        for &c in self.tries.iter().filter(|c| !c.is_uppercase()) {
            for i in 0..=n {
                let mut v = lower.clone();
                v.insert(i, c);
                tries.push(make(v));
                if i < n && lower[i] != c {
                    let mut v = lower.clone();
                    v[i] = c;
                    tries.push(make(v));
                }
            }
        }
        let capitalised = word.chars().next().is_some_and(char::is_uppercase);
        let mut out: Vec<String> = vec![];
        for t in tries {
            if t.is_empty() || !self.correct(&t) {
                continue;
            }
            let t = if capitalised {
                let mut c = t.chars();
                c.next()
                    .map_or_default(|f| f.to_uppercase().chain(c).collect())
            } else {
                t
            };
            if !out.contains(&t) {
                out.push(t);
            }
            if out.len() == 6 {
                break;
            }
        }
        out
    }

    /// The words of `text` spelt wrong, each once, in order; `!!variables!!` are let be.
    #[must_use]
    pub fn misspelled(&self, text: &str) -> Vec<String> {
        let mut out: Vec<String> = vec![];
        for w in words(&without_variables(text)) {
            if !self.correct(&w) && !out.contains(&w) {
                out.push(w);
            }
        }
        out
    }
}

/// `text` with its `!!variable!!` names blanked out.
fn without_variables(text: &str) -> String {
    text.split("!!")
        .enumerate()
        .map(|(i, part)| if i % 2 == 1 { " " } else { part })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The words of `text`: runs of letters, with apostrophes inside them.
fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '\'' || c == '\u{2019}'))
        .map(|w| w.trim_matches(['\'', '\u{2019}']))
        .filter(|w| w.chars().any(char::is_alphabetic))
        .map(str::to_owned)
        .collect()
}

/// `text` with every whole word `from` made `to`.
#[must_use]
pub fn replace_word(text: &str, from: &str, to: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut word = String::new();
    let flush = |word: &mut String, out: &mut String| {
        // a possessive or contraction keeps its ending: woood's becomes wood's
        match word.strip_prefix(from) {
            Some(rest) if rest.is_empty() || rest.starts_with('\'') => {
                out.push_str(to);
                out.push_str(rest);
            }
            _ => out.push_str(word),
        }
        word.clear();
    };
    for c in text.chars() {
        if c.is_alphanumeric() || c == '\'' {
            word.push(c);
        } else {
            flush(&mut word, &mut out);
            out.push(c);
        }
    }
    flush(&mut word, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const AFF: &str = "SET UTF-8\nTRY esianrtolcdugmphbyfvkwz\n\
        PFX U Y 1\nPFX U 0 un .\n\
        SFX S Y 1\nSFX S 0 s [^sxzhy]\n\
        SFX D Y 3\nSFX D 0 d e\nSFX D y ied [^aeiou]y\nSFX D 0 ed [^ey]\n";
    const DIC: &str = "5\ncarve/SD\nwood/S\nlock/UD\nboard\ncopy/D\n";

    fn speller() -> Speller {
        Speller::from_hunspell(AFF, DIC)
    }

    #[test]
    fn dictionary_words_with_their_prefixes_and_suffixes_are_spelt_right() {
        let s = speller();
        for good in [
            "carve", "carves", "carved", "woods", "Board", "copied", "unlock", "unlocked", "BOARD",
            "3mm",
        ] {
            assert!(s.correct(good), "{good}");
        }
        for bad in ["carveing", "boards", "unwood", "copyed", "unboard"] {
            assert!(!s.correct(bad), "{bad}");
        }
    }

    #[test]
    fn suggestions_are_one_edit_away_and_user_words_count() {
        let mut s = speller();
        assert!(s.suggest("woodd").contains(&"wood".to_owned()));
        assert!(
            s.suggest("Carvs").contains(&"Carves".to_owned()),
            "{:?}",
            s.suggest("Carvs")
        );
        assert_eq!(
            s.misspelled("Carve woood, !!name!! Woodcrafty wood"),
            ["woood", "Woodcrafty"]
        );
        s.add_word("Woodcrafty");
        assert_eq!(s.misspelled("Woodcrafty woood"), ["woood"]);
        s.remove_word("woodcrafty");
        assert!(!s.correct("Woodcrafty"));
        assert_eq!(
            replace_word("woood and woood's woood", "woood", "wood"),
            "wood and wood's wood"
        );
    }
}
