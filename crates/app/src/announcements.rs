//! Announcements: short notes from the team, shown in a window at the first start of each
//! day they are meant for, until "Don't show this again" (Help ▸ Announcements shows the
//! current ones whenever you like).
//!
//! They come two ways, read the same:
//!
//! - **Bundled** with a release ([`BUNDLED`]), their words in Fluent so language packs
//!   translate them.
//! - **Fetched** from `announcements.md` on GitHub, which the team writes by hand: nothing is
//!   made up by the program. Each note in the file is a header, then its words in Markdown:
//!
//! ```text
//! ---
//! id: open-day-2026
//! title: Workshop open day
//! from: 2026-12-05
//! until: 2026-12-06
//! ---
//! Come and **see the machines**.
//! ```
//!
//! `date:` sets `from` and `until` to one day. A note with `lang: ru` and another note's id
//! is that note in Russian, shown instead of it when PowerWhittle is in Russian. `support: yes`
//! adds the "Support us on GitHub" row. Dates are the computer's own calendar day. Text
//! before the first header (the file's own explanation) is skipped.

use chrono::NaiveDate;
use std::collections::BTreeMap;

/// The longest title shown; a longer note is left out.
pub const MOST_TITLE: usize = 80;
/// The longest body shown; a longer note is left out.
pub const MOST_BODY: usize = 4000;
/// The most notes read from the file.
pub const MOST_NOTES: usize = 20;

/// One announcement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    /// Its short name: what "shown today" and "don't show again" are kept against.
    pub id: String,
    pub title: String,
    /// Markdown.
    pub body: String,
    /// The first day it shows; any day before `until` when not given.
    pub from: Option<NaiveDate>,
    /// The last day it shows; any day from `from` on when not given.
    pub until: Option<NaiveDate>,
    /// The language this copy is in (a language pack's id); none for English.
    pub lang: Option<String>,
    /// Shows the "Support us on GitHub" row.
    pub support: bool,
}

impl Note {
    /// Whether it is meant for `today`.
    pub fn is_on(&self, today: NaiveDate) -> bool {
        self.from.is_none_or(|from| from <= today) && self.until.is_none_or(|until| today <= until)
    }
}

/// An announcement that ships with the program, its words under Fluent keys.
#[derive(Clone, Copy, Debug)]
pub struct Bundled {
    pub id: &'static str,
    pub title: &'static str,
    pub body: &'static str,
    /// The last day it shows, `YYYY-MM-DD`.
    pub until: &'static str,
    pub support: bool,
}

/// The announcements this version ships with.
pub const BUNDLED: &[Bundled] = &[Bundled {
    id: "powerwhittle-heads-up",
    title: "announcement-powerwhittle-title",
    body: "announcement-powerwhittle-body",
    until: "2026-12-31",
    support: true,
}];

/// The bundled announcements, in the language in use.
pub fn bundled() -> Vec<Note> {
    BUNDLED
        .iter()
        .map(|b| Note {
            id: b.id.to_owned(),
            title: cut_craft_i18n::tr(b.title),
            body: cut_craft_i18n::tr(b.body),
            from: None,
            until: date(b.until),
            lang: None,
            support: b.support,
        })
        .collect()
}

fn date(text: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(text.trim(), "%Y-%m-%d").ok()
}

/// The notes in an `announcements.md`, and what was wrong with any left out (by line).
pub fn parse(text: &str) -> (Vec<Note>, Vec<String>) {
    let lines: Vec<&str> = text.lines().collect();
    let starts_header = |i: usize| {
        lines[i].trim_end() == "---"
            && lines[i + 1..]
                .iter()
                .find(|l| !l.trim().is_empty())
                .is_some_and(|l| l.trim_start().starts_with("id:"))
    };
    let heads: Vec<usize> = (0..lines.len()).filter(|&i| starts_header(i)).collect();
    let (mut notes, mut problems) = (Vec::new(), Vec::new());
    for (n, &head) in heads.iter().enumerate() {
        let next = heads.get(n + 1).copied().unwrap_or(lines.len());
        let Some(close) = (head + 1..next).find(|&i| lines[i].trim_end() == "---") else {
            problems.push(format!("line {}: the header has no closing ---", head + 1));
            continue;
        };
        let body = lines[close + 1..next].join("\n").trim().to_owned();
        match note(&lines[head + 1..close], body) {
            Ok(note) if notes.len() < MOST_NOTES => notes.push(note),
            Ok(_) => problems.push(format!("line {}: more than {MOST_NOTES} notes", head + 1)),
            Err(e) => problems.push(format!("line {}: {e}", head + 1)),
        }
    }
    (notes, problems)
}

/// A note from its header's lines and its words.
fn note(header: &[&str], body: String) -> Result<Note, String> {
    let mut fields = BTreeMap::new();
    for line in header.iter().filter(|l| !l.trim().is_empty()) {
        let (key, value) = line
            .split_once(':')
            .ok_or_else(|| format!("\"{}\" is not \"name: value\"", line.trim()))?;
        fields.insert(key.trim().to_lowercase(), value.trim().to_owned());
    }
    let field = |k: &str| fields.get(k).map(String::as_str).filter(|v| !v.is_empty());
    let day = |k: &str| match field(k) {
        None => Ok(None),
        Some(v) => date(v)
            .map(Some)
            .ok_or_else(|| format!("{k}: \"{v}\" is not a date like 2026-11-26")),
    };
    let id = field("id").ok_or("it has no id")?.to_owned();
    let title = field("title").ok_or("it has no title")?.to_owned();
    let (from, until) = match day("date")? {
        Some(d) => (Some(d), Some(d)),
        None => (day("from")?, day("until")?),
    };
    if let (Some(f), Some(u)) = (from, until)
        && u < f
    {
        return Err("until is before from".into());
    }
    if title.chars().count() > MOST_TITLE {
        return Err(format!("the title is longer than {MOST_TITLE} letters"));
    }
    if body.is_empty() {
        return Err("it has no words under the header".into());
    }
    if body.chars().count() > MOST_BODY {
        return Err(format!("its words are longer than {MOST_BODY} letters"));
    }
    Ok(Note {
        id,
        title,
        body,
        from,
        until,
        lang: field("lang").map(str::to_owned),
        support: field("support").is_some_and(|v| matches!(v, "yes" | "true")),
    })
}

/// The notes meant for `today`, one copy of each: the one in `language` when there is one,
/// else the English.
pub fn current(notes: &[Note], today: NaiveDate, language: &str) -> Vec<Note> {
    let mut out: Vec<Note> = Vec::new();
    for note in notes.iter().filter(|n| n.is_on(today)) {
        let english = note.lang.is_none();
        let ours = note.lang.as_deref() == Some(language);
        if !english && !ours {
            continue;
        }
        match out.iter_mut().find(|n| n.id == note.id) {
            Some(kept) if ours => *kept = note.clone(),
            Some(_) => {}
            None => out.push(note.clone()),
        }
    }
    out
}

/// What this computer has done with announcements, kept with the settings.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Seen {
    /// Don't show this again was ticked for these.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dismissed: Vec<String>,
    /// The day each was last shown at start, `YYYY-MM-DD`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub shown: BTreeMap<String, String>,
}

impl Seen {
    /// Nothing kept yet.
    pub fn is_empty(&self) -> bool {
        self.dismissed.is_empty() && self.shown.is_empty()
    }

    /// Whether `note` opens at this start: meant for today, not turned off, and not shown
    /// yet today.
    pub fn is_due(&self, note: &Note, today: NaiveDate) -> bool {
        note.is_on(today)
            && !self.dismissed.contains(&note.id)
            && self.shown.get(&note.id).and_then(|d| date(d)) != Some(today)
    }

    /// `notes` were shown at start `today`; days kept for notes no longer current are let go.
    pub fn shown_on(&mut self, notes: &[Note], today: NaiveDate, current: &[Note]) {
        self.shown
            .retain(|id, _| current.iter().any(|n| &n.id == id));
        for note in notes {
            self.shown
                .insert(note.id.clone(), today.format("%Y-%m-%d").to_string());
        }
    }

    /// Ticks or clears Don't show this again for `id`.
    pub fn set_dismissed(&mut self, id: &str, dismissed: bool) {
        self.dismissed.retain(|d| d != id);
        if dismissed {
            self.dismissed.push(id.to_owned());
        }
    }
}

/// The computer's calendar day.
pub fn today() -> NaiveDate {
    chrono::Local::now().date_naive()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(text: &str) -> NaiveDate {
        date(text).unwrap()
    }

    const TWO: &str = "This file holds the notes.\n\
        ---\n\
        id: holiday\n\
        title: Happy holiday\n\
        date: 2026-11-26\n\
        ---\n\
        Thanks for **carving** with us.\n\
        \n\
        ---\n\
        \n\
        A line, not a header.\n\
        ---\n\
        id: open-day\n\
        title: Open day\n\
        from: 2026-12-05\n\
        until: 2026-12-06\n\
        support: yes\n\
        ---\n\
        Come along.\n";

    #[test]
    fn a_file_reads_as_its_notes_with_their_dates() {
        let (notes, problems) = parse(TWO);
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(notes.len(), 2);
        let holiday = &notes[0];
        assert_eq!(holiday.id, "holiday");
        assert_eq!(holiday.title, "Happy holiday");
        assert_eq!(
            (holiday.from, holiday.until),
            (Some(day("2026-11-26")), Some(day("2026-11-26")))
        );
        // a --- that does not start a header is part of the words
        assert!(
            holiday.body.starts_with("Thanks for **carving**"),
            "{}",
            holiday.body
        );
        assert!(
            holiday.body.ends_with("A line, not a header."),
            "{}",
            holiday.body
        );
        assert!(!holiday.support);
        assert!(notes[1].support);
        assert_eq!(notes[1].until, Some(day("2026-12-06")));
    }

    #[test]
    fn a_note_shows_only_on_its_days() {
        let (notes, _) = parse(TWO);
        let open_day = &notes[1];
        assert!(!open_day.is_on(day("2026-12-04")));
        assert!(open_day.is_on(day("2026-12-05")));
        assert!(open_day.is_on(day("2026-12-06")));
        assert!(!open_day.is_on(day("2026-12-07")));
        let any_day = Note {
            from: None,
            until: None,
            ..open_day.clone()
        };
        assert!(any_day.is_on(day("2030-01-01")));
    }

    #[test]
    fn a_note_that_will_not_read_is_left_out_and_says_why() {
        let text = "---\nid: a\ntitle: A\ndate: 26 Nov\n---\nWords\n\
                    ---\nid: b\n---\nWords\n\
                    ---\nid: c\ntitle: C\nfrom: 2026-12-02\nuntil: 2026-12-01\n---\nWords\n\
                    ---\nid: d\ntitle: D\n---\n\
                    ---\nid: e\ntitle: E\n---\nFine\n";
        let (notes, problems) = parse(text);
        assert_eq!(
            notes.iter().map(|n| n.id.as_str()).collect::<Vec<_>>(),
            ["e"]
        );
        assert_eq!(problems.len(), 4, "{problems:?}");
        assert!(problems[0].contains("line 1") && problems[0].contains("2026-11-26"));
        assert!(problems[1].contains("no title"));
        assert!(problems[2].contains("until is before from"));
        assert!(problems[3].contains("no words"));
        let long = format!(
            "---\nid: x\ntitle: {}\n---\nWords\n",
            "t".repeat(MOST_TITLE + 1)
        );
        assert_eq!(parse(&long).0, vec![]);
    }

    #[test]
    fn an_empty_file_or_one_with_only_its_explanation_has_no_notes() {
        assert_eq!(parse(""), (vec![], vec![]));
        assert_eq!(
            parse("# Announcements\n\nHow to write one.\n---\n"),
            (vec![], vec![])
        );
    }

    #[test]
    fn a_translated_copy_shows_in_its_language_and_english_elsewhere() {
        let text = "---\nid: n\ntitle: Hello\n---\nEnglish words\n\
                    ---\nid: n\nlang: ru\ntitle: Привет\n---\nРусские слова\n\
                    ---\nid: only-ru\nlang: ru\ntitle: Только\n---\nСлова\n";
        let (notes, _) = parse(text);
        let today = day("2026-11-26");
        let ru = current(&notes, today, "ru");
        assert_eq!(
            ru.iter().map(|n| n.title.as_str()).collect::<Vec<_>>(),
            ["Привет", "Только"]
        );
        let en = current(&notes, today, "en");
        assert_eq!(
            en.iter().map(|n| n.title.as_str()).collect::<Vec<_>>(),
            ["Hello"]
        );
    }

    #[test]
    fn a_note_opens_once_a_day_until_turned_off() {
        let (notes, _) = parse(TWO);
        let open_day = &notes[1];
        let mut seen = Seen::default();
        let (fri, sat) = (day("2026-12-05"), day("2026-12-06"));
        assert!(seen.is_due(open_day, fri));
        seen.shown_on(std::slice::from_ref(open_day), fri, &notes);
        assert!(!seen.is_due(open_day, fri), "not twice the same day");
        assert!(seen.is_due(open_day, sat), "again the next day");
        seen.set_dismissed("open-day", true);
        assert!(!seen.is_due(open_day, sat), "turned off");
        seen.set_dismissed("open-day", false);
        assert!(seen.is_due(open_day, sat), "turned back on");
        assert!(!seen.is_due(open_day, day("2026-12-07")), "past its days");
        // a day kept for a note no longer in the file is let go
        seen.shown_on(&[], sat, &notes[..1]);
        assert!(seen.shown.is_empty(), "{:?}", seen.shown);
    }

    #[test]
    fn the_bundled_announcements_have_words_and_dates() {
        for b in BUNDLED {
            let english = cut_craft_i18n::english();
            assert!(english.has(b.title) && english.has(b.body), "{}", b.id);
            assert!(date(b.until).is_some(), "{}: {}", b.id, b.until);
        }
        assert_eq!(bundled().len(), BUNDLED.len());
    }

    #[test]
    fn the_announcements_file_in_the_repository_reads_cleanly() {
        let file = include_str!("../../../announcements.md");
        let (_, problems) = parse(file);
        assert!(problems.is_empty(), "announcements.md: {problems:?}");
    }
}
