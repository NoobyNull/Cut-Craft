//! The help search: type a question the way you'd ask it ("how do I cut out a sign with
//! tabs?") and get the pages that answer it, best first.
//!
//! The pages are put in a small [tantivy] index held in memory the first time the search is
//! used: nothing is written to disk and nothing goes on the network, so it works the same on
//! every platform. A question is read like this:
//!
//! 1. Question and filler words ("how", "do", "I", "a", "with"...) are dropped ([`STOP_WORDS`]).
//! 2. Each word left, and each phrase in the synonym list, also finds the words that mean the
//!    same in CNC work: "bridges" finds Tabs, "cut out" finds Profile ([`SYNONYMS`], one group
//!    of words per line in `synonyms.txt`, so the list can grow without code).
//! 3. Words are matched by their stem, so "cutting" finds "cut" and "pockets" finds "pocket".
//! 4. Pages are ranked by BM25: a page matching more of the words, rarer words, or words in
//!    its title (worth [`TITLE_BOOST`] times as much) comes first.
//!
//! Ranking by meaning (a small embedding model) can be added later as a second score mixed
//! into [`Index::search`]'s; nothing else needs to change for it.

use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{STORED, STRING, Schema, TextFieldIndexing, TextOptions, Value};
use tantivy::tokenizer::{
    Language, LowerCaser, RemoveLongFilter, SimpleTokenizer, Stemmer, TextAnalyzer,
};
use tantivy::{TantivyDocument, doc};

/// Words a question is made of that say nothing about what is asked.
pub const STOP_WORDS: &[&str] = &[
    "a", "about", "an", "and", "any", "are", "as", "at", "be", "can", "could", "do", "does", "for",
    "from", "get", "how", "i", "if", "in", "into", "is", "it", "its", "me", "my", "of", "on", "or",
    "please", "should", "so", "some", "that", "the", "their", "then", "there", "this", "to", "use",
    "using", "want", "way", "what", "when", "where", "which", "who", "why", "will", "with",
    "would", "you", "your",
];

/// Groups of words that mean the same in the help: one group per line, words or phrases
/// separated by commas. Lines starting `#` are notes.
pub const SYNONYMS: &str = include_str!("../synonyms.txt");

/// How much more a word in a page's title counts than one in its words.
pub const TITLE_BOOST: f32 = 3.0;

/// The most pages ranked for a search.
const MOST: usize = 50;

/// The most results a search gives: past these the pages only brush the question.
const MOST_SHOWN: usize = 15;

/// The results always shown, however far below the best: a word that is one page's title
/// still lists the other pages using it.
const LEAST_SHOWN: usize = 5;

/// How good a result must be to be shown past [`LEAST_SHOWN`], as a share of the best one's score: below it a
/// page only says one common word of the question.
const WORTH_SHOWING: f32 = 0.25;

/// One page as the search sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Doc {
    /// The page's id.
    pub id: String,
    /// Its title.
    pub title: String,
    /// Its words (Markdown is fine).
    pub text: String,
}

/// The pages, indexed for searching.
pub struct Index {
    reader: tantivy::IndexReader,
    parser: QueryParser,
    id: tantivy::schema::Field,
    synonyms: Vec<Vec<String>>,
}

/// The analyser every word goes through, both pages' and questions': split on anything not a
/// letter or digit, lower-case, then cut to its English stem.
const ANALYSER: &str = "help_en";

fn analyser() -> TextAnalyzer {
    TextAnalyzer::builder(SimpleTokenizer::default())
        .filter(RemoveLongFilter::limit(40))
        .filter(LowerCaser)
        .filter(Stemmer::new(Language::English))
        .build()
}

impl Index {
    /// Indexes `docs`, in memory.
    ///
    /// # Panics
    /// Never in practice: an in-memory index with a fixed schema cannot fail to build.
    pub fn new(docs: &[Doc]) -> Self {
        let mut schema = Schema::builder();
        let text = TextOptions::default().set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer(ANALYSER)
                .set_index_option(tantivy::schema::IndexRecordOption::WithFreqsAndPositions),
        );
        let id = schema.add_text_field("id", STRING | STORED);
        let title = schema.add_text_field("title", text.clone());
        let body = schema.add_text_field("body", text);
        let index = tantivy::Index::create_in_ram(schema.build());
        index.tokenizers().register(ANALYSER, analyser());
        let mut writer: tantivy::IndexWriter = index
            .writer_with_num_threads(1, 15_000_000)
            .expect("an in-memory index takes a writer");
        for d in docs {
            writer
                .add_document(
                    doc!(id => d.id.as_str(), title => d.title.as_str(), body => d.text.as_str()),
                )
                .expect("a page is indexed");
        }
        writer.commit().expect("the pages are saved in memory");
        let reader = index.reader().expect("an in-memory index reads");
        let mut parser = QueryParser::for_index(&index, vec![title, body]);
        parser.set_field_boost(title, TITLE_BOOST);
        Self {
            reader,
            parser,
            id,
            synonyms: synonym_groups(SYNONYMS),
        }
    }

    /// The pages answering `question`, best first, by id.
    pub fn search(&self, question: &str) -> Vec<String> {
        let Some(query) = self.query_text(question) else {
            return vec![];
        };
        let Ok(query) = self.parser.parse_query(&query) else {
            return vec![];
        };
        let searcher = self.reader.searcher();
        let Ok(top) = searcher.search(&query, &TopDocs::with_limit(MOST).order_by_score()) else {
            return vec![];
        };
        let best = top.first().map_or(0.0, |(score, _)| *score);
        top.into_iter()
            .enumerate()
            .take_while(|(i, (score, _))| *i < LEAST_SHOWN || *score >= best * WORTH_SHOWING)
            .map(|(_, hit)| hit)
            .take(MOST_SHOWN)
            .filter_map(|(_, at)| {
                let d: TantivyDocument = searcher.doc(at).ok()?;
                d.get_first(self.id)?.as_str().map(str::to_owned)
            })
            .collect()
    }

    /// `question` as the query parser's text: each word or synonym phrase kept, with what
    /// means the same beside it, all joined by OR. `None` when nothing is left to look for.
    fn query_text(&self, question: &str) -> Option<String> {
        let terms = expand(&words(question), &self.synonyms);
        if terms.is_empty() {
            return None;
        }
        let quoted: Vec<String> = terms
            .iter()
            .map(|t| {
                if t.contains(' ') {
                    format!("\"{t}\"")
                } else {
                    t.clone()
                }
            })
            .collect();
        Some(quoted.join(" OR "))
    }
}

/// The words of `question` worth looking for: lower-case, only letters and digits, no
/// question or filler words, each once.
pub fn words(question: &str) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for w in question
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
    {
        if !STOP_WORDS.contains(&w.as_str()) && !out.contains(&w) {
            out.push(w);
        }
    }
    out
}

/// The synonym list read: each line's words and phrases, lower-case.
pub fn synonym_groups(text: &str) -> Vec<Vec<String>> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            l.split(',')
                .map(|w| {
                    w.split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ")
                        .to_lowercase()
                })
                .filter(|w| !w.is_empty())
                .collect()
        })
        .filter(|g: &Vec<String>| g.len() > 1)
        .collect()
}

/// `words` with every synonym group they touch added: a group touches them when one of its
/// entries is one of the words, or a phrase of them in order ("cut out"). Each term once.
pub fn expand(words: &[String], groups: &[Vec<String>]) -> Vec<String> {
    let said = format!(" {} ", words.join(" "));
    let mut out: Vec<String> = words.to_vec();
    for group in groups {
        let touched = group.iter().any(|entry| {
            // an entry matches the same words, or the same stems: "bridges" for "bridge"
            let entry_words: Vec<String> = self::words(entry);
            !entry_words.is_empty()
                && (said.contains(&format!(" {} ", entry_words.join(" ")))
                    || entry_words.len() == 1
                        && words.iter().any(|w| same_stem(w, &entry_words[0])))
        });
        if touched {
            for entry in group {
                let entry = self::words(entry).join(" ");
                if !entry.is_empty() && !out.contains(&entry) {
                    out.push(entry);
                }
            }
        }
    }
    out
}

/// Whether two words share their English stem ("pockets", "pocket").
fn same_stem(a: &str, b: &str) -> bool {
    let stem = |w: &str| {
        let mut a = analyser();
        let mut s = a.token_stream(w);
        let mut out = String::new();
        s.process(&mut |t| out.push_str(&t.text));
        out
    };
    a == b || stem(a) == stem(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn docs() -> Vec<Doc> {
        let d = |id: &str, title: &str, text: &str| Doc {
            id: id.into(),
            title: title.into(),
            text: text.into(),
        };
        vec![
            d(
                "profile_toolpath",
                "Profile Toolpath",
                "Cuts along the selected vectors, outside, inside or on the line, to cut a part \
                 free of the sheet.",
            ),
            d(
                "tabs",
                "Tabs",
                "Little bits of material left uncut so the part stays held while the machine \
                 finishes cutting round it.",
            ),
            d(
                "pocket_toolpath",
                "Pocket Toolpath",
                "Clears the inside of a closed shape down to a depth.",
            ),
            d(
                "draw_text",
                "Draw Text",
                "Writes words on the sheet in a font, for a sign or a name plate.",
            ),
            d(
                "stepover",
                "Stepover",
                "How far the bit moves over between passes.",
            ),
        ]
    }

    fn top(index: &Index, q: &str, n: usize) -> Vec<String> {
        index.search(q).into_iter().take(n).collect()
    }

    #[test]
    fn a_question_finds_its_answers_with_the_question_words_dropped() {
        let index = Index::new(&docs());
        let found = top(&index, "How do I cut out a sign with tabs?", 3);
        assert!(found.contains(&"tabs".into()), "{found:?}");
        assert!(found.contains(&"profile_toolpath".into()), "{found:?}");
    }

    #[test]
    fn words_that_mean_the_same_find_the_page() {
        let index = Index::new(&docs());
        assert_eq!(top(&index, "bridges", 1), ["tabs"]);
        assert_eq!(top(&index, "what are hold-downs", 1), ["tabs"]);
        assert_eq!(top(&index, "clear out the middle", 1), ["pocket_toolpath"]);
    }

    #[test]
    fn a_word_finds_its_other_endings() {
        let index = Index::new(&docs());
        assert_eq!(top(&index, "pockets", 1), ["pocket_toolpath"]);
        assert_eq!(top(&index, "clearing", 1), ["pocket_toolpath"]);
    }

    #[test]
    fn a_title_match_comes_first() {
        let index = Index::new(&docs());
        // "stepover" is Stepover's title; no other page says it
        assert_eq!(top(&index, "stepover", 1), ["stepover"]);
        // "text" is Draw Text's title, and no other page says it
        assert_eq!(top(&index, "text", 1), ["draw_text"]);
    }

    #[test]
    fn only_question_words_find_nothing() {
        let index = Index::new(&docs());
        assert_eq!(index.search("how do I"), [] as [String; 0]);
        assert_eq!(index.search("   "), [] as [String; 0]);
        assert!(
            index.search("\"(*:").is_empty(),
            "odd characters are no error"
        );
    }

    #[test]
    fn the_question_keeps_its_words_and_drops_fillers() {
        assert_eq!(words("How do I cut OUT a sign?"), ["cut", "out", "sign"]);
    }

    #[test]
    fn synonyms_add_their_group_once() {
        let groups = synonym_groups("# note\ntabs, bridges, hold-downs\nprofile, cut out\n");
        assert_eq!(groups.len(), 2);
        let got = expand(&words("cut out bridges"), &groups);
        for want in [
            "cut",
            "out",
            "bridges",
            "tabs",
            "hold downs",
            "profile",
            "cut out",
        ] {
            assert!(got.contains(&want.into()), "{want}: {got:?}");
        }
        assert_eq!(
            expand(&words("bridge"), &groups),
            ["bridge", "tabs", "bridges", "hold downs"],
            "matched by stem"
        );
    }

    #[test]
    fn the_synonym_list_reads() {
        let groups = synonym_groups(SYNONYMS);
        assert!(groups.len() >= 20, "{}", groups.len());
        assert!(
            groups
                .iter()
                .any(|g| g.contains(&"tabs".into()) && g.contains(&"bridges".into()))
        );
    }
}
