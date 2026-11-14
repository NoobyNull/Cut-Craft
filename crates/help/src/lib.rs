//! Cut Craft's help pages, kept apart from the program that shows them.
//!
//! Every page is a Markdown file under `pages/` (a tool's under `pages/tools/<tool id>.md`),
//! carried into the program when it is built, so help works offline on every platform. Each
//! starts with a short header:
//!
//! ```text
//! ---
//! reviewed: 2026-11-14
//! see-also: profile_toolpath, tabs
//! ---
//! ```
//!
//! - `reviewed` is the day someone last checked the page against the tool. A pull request
//!   that changes a tool must change its page, if only this date (`scripts/help-touched`).
//! - `see-also` names related pages by id. With every `help:` link in the page's words they
//!   make the help's link graph: [`links`] goes out from a page, [`linked_from`] comes back
//!   in, so each page can list the pages around it without anyone keeping two lists.

use std::fmt::Write as _;

include!(concat!(env!("OUT_DIR"), "/pages.rs"));

/// One help page, its header read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page<'a> {
    /// The page's id: its file's name without `.md` (a tool's id for a tool's page).
    pub id: &'a str,
    /// The day it was last checked, `YYYY-MM-DD`.
    pub reviewed: &'a str,
    /// Related pages, by id, in the header's order.
    pub see_also: Vec<&'a str>,
    /// The Markdown after the header.
    pub body: &'a str,
}

/// Reads page `id` from its file's `text`, or says what is wrong with its header.
pub fn parse<'a>(id: &'a str, text: &'a str) -> Result<Page<'a>, String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let Some(rest) = text
        .strip_prefix("---\n")
        .or_else(|| text.strip_prefix("---\r\n"))
    else {
        return Err(format!(
            "{id}: the page must start with a header (---, reviewed: YYYY-MM-DD, see-also:, ---)"
        ));
    };
    let Some((header, body)) = rest
        .split_once("\n---\n")
        .or_else(|| rest.split_once("\r\n---\r\n"))
        .or_else(|| rest.strip_suffix("\n---").map(|h| (h, "")))
        .or_else(|| rest.strip_suffix("\n---\n").map(|h| (h, "")))
    else {
        return Err(format!("{id}: the header has no closing ---"));
    };
    let (mut reviewed, mut see_also) = (None, vec![]);
    for line in header.lines().map(str::trim).filter(|l| !l.is_empty()) {
        match line.split_once(':').map(|(k, v)| (k.trim(), v.trim())) {
            Some(("reviewed", day)) => reviewed = Some(day),
            Some(("see-also", ids)) => {
                see_also = ids
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            _ => return Err(format!("{id}: the header line \"{line}\" is not known")),
        }
    }
    let reviewed = reviewed.ok_or_else(|| format!("{id}: the header has no reviewed: date"))?;
    if !is_day(reviewed) {
        return Err(format!(
            "{id}: reviewed: \"{reviewed}\" is not a day written YYYY-MM-DD"
        ));
    }
    Ok(Page {
        id,
        reviewed,
        see_also,
        body: body.trim_start_matches(['\r', '\n']),
    })
}

/// `2026-11-14`, a real month and day.
fn is_day(text: &str) -> bool {
    let parts: Vec<&str> = text.split('-').collect();
    let number = |s: &str, len: usize| {
        (s.len() == len && s.chars().all(|c| c.is_ascii_digit())).then(|| s.parse::<u32>().ok())?
    };
    match parts.as_slice() {
        [y, m, d] => {
            number(y, 4).is_some()
                && number(m, 2).is_some_and(|m| (1..=12).contains(&m))
                && number(d, 2).is_some_and(|d| (1..=31).contains(&d))
        }
        _ => false,
    }
}

/// Every page carried into the program, read. A page whose header is wrong is left out here;
/// [`problems`] names it.
pub fn pages() -> impl Iterator<Item = Page<'static>> {
    PAGES.iter().filter_map(|(id, text)| parse(id, text).ok())
}

/// Page `id`, if there is one.
pub fn page(id: &str) -> Option<Page<'static>> {
    PAGES
        .iter()
        .find(|(p, _)| *p == id)
        .and_then(|(p, text)| parse(p, text).ok())
}

/// The ids a `help:` link in `markdown` goes to, in order, each once.
pub fn help_links(markdown: &str) -> Vec<&str> {
    let mut out: Vec<&str> = vec![];
    let mut rest = markdown;
    while let Some(at) = rest.find("](help:") {
        rest = &rest[at + "](help:".len()..];
        let end = rest.find(')').unwrap_or(rest.len());
        let id = rest[..end].trim();
        if !id.is_empty() && !out.contains(&id) {
            out.push(id);
        }
    }
    out
}

/// Where `page` points: its see-also pages, then the pages its words link to, each once.
pub fn links<'a>(page: &Page<'a>) -> Vec<&'a str> {
    let mut out = page.see_also.clone();
    for id in help_links(page.body) {
        if !out.contains(&id) && id != page.id {
            out.push(id);
        }
    }
    out
}

/// The pages pointing at `id` (by see-also or a link in their words), in id order.
pub fn linked_from(id: &str) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = pages()
        .filter(|p| p.id != id && links(p).contains(&id))
        .map(|p| p.id)
        .collect();
    out.sort_unstable();
    out
}

/// The "See also" end of page `id`: the pages it points to by see-also, then those pointing
/// back at it, each once and never itself, as Markdown `help:` links named by `title`.
/// Empty when there are none.
pub fn related_markdown(id: &str, title: impl Fn(&str) -> String) -> String {
    let mut ids: Vec<&str> = page(id).map_or_default(|p| p.see_also);
    for back in linked_from(id) {
        if !ids.contains(&back) {
            ids.push(back);
        }
    }
    ids.retain(|i| *i != id);
    if ids.is_empty() {
        return String::new();
    }
    let mut md = String::from("## See also\n\n");
    for i in ids {
        let _ = writeln!(md, "- [{}](help:{i})", title(i));
    }
    md
}

/// What is wrong with the pages: a header that will not read, or a link (see-also or `help:`)
/// to a page that is neither among them nor one `known` says the program makes itself.
pub fn problems(known: impl Fn(&str) -> bool) -> Vec<String> {
    let ids: Vec<&str> = PAGES.iter().map(|(id, _)| *id).collect();
    let mut out = vec![];
    for (id, text) in PAGES {
        match parse(id, text) {
            Err(e) => out.push(e),
            Ok(page) => {
                for to in links(&page) {
                    if !ids.contains(&to) && !known(to) {
                        out.push(format!("{id}: links to \"{to}\", which is no help page"));
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = "---\nreviewed: 2026-11-14\nsee-also: tabs, profile_toolpath\n---\n\n\
                        # Pocket\n\nClears inside. See [Tabs](help:tabs) and [Drill](help:drill).\n";

    #[test]
    fn a_page_reads_its_header_and_keeps_its_words() {
        let page = parse("pocket_toolpath", GOOD).expect("a good page");
        assert_eq!(page.reviewed, "2026-11-14");
        assert_eq!(page.see_also, ["tabs", "profile_toolpath"]);
        assert!(page.body.starts_with("# Pocket"), "{}", page.body);
    }

    #[test]
    fn a_header_only_page_reads_with_no_words() {
        let page = parse("x", "---\nreviewed: 2026-11-14\nsee-also:\n---\n").expect("a stub");
        assert_eq!(page.body, "");
        assert_eq!(page.see_also, [] as [&str; 0]);
        let windows = "---\r\nreviewed: 2026-11-14\r\nsee-also: a\r\n---\r\n\r\nHi\r\n";
        assert_eq!(parse("x", windows).expect("CRLF").body, "Hi\r\n");
    }

    #[test]
    fn a_bad_header_says_what_is_wrong() {
        let bad = |text| parse("x", text).expect_err("bad");
        assert!(bad("# No header").contains("must start with a header"));
        assert!(bad("---\nreviewed: 2026-11-14\n").contains("no closing"));
        assert!(bad("---\nsee-also:\n---\n").contains("no reviewed"));
        assert!(bad("---\nreviewed: yesterday\n---\n").contains("YYYY-MM-DD"));
        assert!(bad("---\nreviewed: 2026-13-01\n---\n").contains("YYYY-MM-DD"));
        assert!(bad("---\nreviewed: 2026-11-14\nauthor: me\n---\n").contains("not known"));
    }

    #[test]
    fn links_are_see_also_then_the_words_each_once() {
        let page = parse("pocket_toolpath", GOOD).expect("good");
        assert_eq!(links(&page), ["tabs", "profile_toolpath", "drill"]);
        assert_eq!(
            help_links("[a](help:x) [b](help:y) [c](help:x) [w](https://e.com)"),
            ["x", "y"]
        );
    }

    #[test]
    fn every_page_carried_in_has_a_good_header() {
        let bad: Vec<String> = PAGES
            .iter()
            .filter_map(|(id, text)| parse(id, text).err())
            .collect();
        assert!(bad.is_empty(), "{bad:#?}");
        assert!(page("contents").is_some(), "Help Contents is a page");
        assert!(page("pocket_toolpath").is_some(), "a tool's page");
    }

    #[test]
    fn page_ids_are_unique_whatever_their_folder() {
        let mut ids: Vec<&str> = PAGES.iter().map(|(id, _)| *id).collect();
        let all = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), all, "two pages share a file name");
    }

    #[test]
    fn contents_links_back_from_the_pages_it_lists() {
        assert!(linked_from("design").contains(&"contents"));
        let related = related_markdown("design", |id| format!("<{id}>"));
        assert!(
            related.contains("- [<contents>](help:contents)"),
            "{related}"
        );
        assert!(!related.contains("help:design)"), "never itself: {related}");
    }
}
