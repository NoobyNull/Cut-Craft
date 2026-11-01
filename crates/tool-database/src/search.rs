//! Searching the Tool Database by tool name: each name's words are indexed, and a search
//! finds the tools with a word starting with each word typed, in any order and case.

use cut_craft_core::toolpath::ToolDatabase;
use std::collections::BTreeMap;

/// The tools' names, word by word (lower case), each word to the tools it is in.
#[derive(Default)]
pub struct SearchIndex {
    /// The names indexed, by tool index, to see when the database changed.
    names: Vec<String>,
    words: BTreeMap<String, Vec<usize>>,
}

/// A name's words: split on spaces and punctuation, a decimal point kept (`6.35`).
fn words(name: &str) -> impl Iterator<Item = String> + '_ {
    name.split(|c: char| !(c.is_alphanumeric() || c == '.'))
        .map(|w| w.trim_matches('.').to_lowercase())
        .filter(|w| !w.is_empty())
}

impl SearchIndex {
    pub fn new(db: &ToolDatabase) -> Self {
        let mut index = Self {
            names: db.tools.iter().map(|t| t.name.clone()).collect(),
            ..Self::default()
        };
        for (i, name) in index.names.iter().enumerate() {
            for word in words(name) {
                let tools = index.words.entry(word).or_default();
                if tools.last() != Some(&i) {
                    tools.push(i);
                }
            }
        }
        index
    }

    /// Whether `db`'s tool names are no longer the ones indexed.
    pub fn is_stale(&self, db: &ToolDatabase) -> bool {
        self.names.len() != db.tools.len()
            || self.names.iter().zip(&db.tools).any(|(n, t)| *n != t.name)
    }

    /// The tools whose names have a word starting with every word of `query`, by index;
    /// none for an empty query.
    pub fn find(&self, query: &str) -> Vec<usize> {
        let mut found: Option<Vec<usize>> = None;
        for word in words(query) {
            let mut tools: Vec<usize> = self
                .words
                .range(word.clone()..)
                .take_while(|(w, _)| w.starts_with(&word))
                .flat_map(|(_, tools)| tools.iter().copied())
                .collect();
            tools.sort_unstable();
            tools.dedup();
            found = Some(match found {
                None => tools,
                Some(before) => before.into_iter().filter(|t| tools.contains(t)).collect(),
            });
        }
        found.unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::toolpath::Tool;

    fn db(names: &[&str]) -> ToolDatabase {
        ToolDatabase {
            tools: names
                .iter()
                .map(|n| Tool {
                    name: (*n).into(),
                    ..Tool::default()
                })
                .collect(),
            ..ToolDatabase::default()
        }
    }

    #[test]
    fn words_typed_find_the_tools_whose_names_start_them_in_any_order() {
        let tools = db(&[
            "6 mm End Mill",
            "1/4\" Down-cut End Mill",
            "90° V-Bit 12.7 mm",
            "6.35 mm Ball Nose",
        ]);
        let index = SearchIndex::new(&tools);
        assert_eq!(index.find("end"), [0, 1]);
        assert_eq!(index.find("MILL 6"), [0], "any case and order, word starts");
        assert_eq!(index.find("down"), [1]);
        assert_eq!(index.find("v-bit"), [2]);
        assert_eq!(index.find("6.3"), [3], "a decimal stays one word");
        assert_eq!(index.find("ball end"), Vec::<usize>::new());
        assert_eq!(index.find("  "), Vec::<usize>::new());
    }

    #[test]
    fn a_renamed_or_added_tool_makes_the_index_stale() {
        let mut tools = db(&["A"]);
        let index = SearchIndex::new(&tools);
        assert!(!index.is_stale(&tools));
        tools.tools[0].name = "B".into();
        assert!(index.is_stale(&tools));
    }
}
