//! Draw Text's spelling: its dictionary, looked for once when spelling is first wanted,
//! and the user's own words, kept one a line in their words file.

use cut_craft_core::spell::Speller;
use std::path::PathBuf;

#[derive(Default)]
pub(super) struct Spelling {
    /// The user's own spelling words, one a line; none in tests.
    words_path: Option<PathBuf>,
    /// The dictionary, once looked for.
    speller: Option<Speller>,
    /// Whether the dictionary was looked for (found or not).
    looked: bool,
}

impl Spelling {
    /// The user's own words are kept at `path` from now on (`None`: in memory only).
    pub(super) fn words_at(&mut self, path: Option<PathBuf>) {
        self.words_path = path;
    }

    /// Spelling is checked with `speller` (tests: no dictionary is looked for).
    #[cfg(test)]
    pub(super) fn given(&mut self, speller: Speller) {
        self.speller = Some(speller);
        self.looked = true;
    }

    /// The dictionary as found, without looking for it.
    #[cfg(test)]
    pub(super) fn found(&self) -> Option<&Speller> {
        self.speller.as_ref()
    }

    /// The dictionary, Hunspell's from the dictionaries folder or the system, with the
    /// user's own words: looked for the first time it is wanted.
    pub(super) fn speller(&mut self) -> Option<&mut Speller> {
        self.speller_from(Speller::find)
    }

    /// As [`Self::speller`], the dictionary looked for by `find`.
    fn speller_from(&mut self, find: impl FnOnce() -> Option<Speller>) -> Option<&mut Speller> {
        if !self.looked {
            self.looked = true;
            let words = self
                .words_path
                .as_ref()
                .and_then(|p| std::fs::read_to_string(p).ok())
                .unwrap_or_default();
            self.speller = find().map(|s| s.with_user_words(words.lines()));
        }
        self.speller.as_mut()
    }

    /// Adds `word` to the user's own words (or takes it off) and keeps them in the words
    /// file: an error saying why when the file could not be written. Nothing to do with no
    /// dictionary.
    pub(super) fn user_word(&mut self, word: &str, add: bool) -> Result<(), String> {
        let path = self.words_path.clone();
        let Some(speller) = self.speller() else {
            return Ok(());
        };
        if add {
            speller.add_word(word);
        } else {
            speller.remove_word(word);
        }
        let words: Vec<&str> = speller.user_words().iter().map(String::as_str).collect();
        match path {
            Some(path) => std::fs::write(&path, words.join("\n"))
                .map_err(|e| crate::messages::cannot_keep_words(&path, &e)),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests;
