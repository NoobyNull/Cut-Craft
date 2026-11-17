//! What makes the help read like a wiki: the side bar beside every page (other tools like
//! this one, related pages, what links here), links put into the words wherever a page names
//! another, and Back and Forward through the pages read.

use crate::{linked_from, links, page};

/// The side bar beside a help page.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sidebar {
    /// "Other tools like this": the tools beside it in its group on the page's panel, so a
    /// beginner who opened the wrong one finds the right one a click away.
    pub like_this: Vec<String>,
    /// "Related": the pages its header's `see-also` names, then the pages its words link to.
    pub related: Vec<String>,
    /// "What links here": the pages that point at it, not already listed above.
    pub links_here: Vec<String>,
}

impl Sidebar {
    /// Whether there is nothing to list.
    pub fn is_empty(&self) -> bool {
        self.like_this.is_empty() && self.related.is_empty() && self.links_here.is_empty()
    }
}

/// The pages that are signposts, not topics: every page links to them, so listing them
/// beside a page tells the reader nothing.
const SIGNPOSTS: [&str; 3] = ["contents", "design", "toolpaths"];

/// The side bar for page `id`: `group` is its tool group's tools (empty when it is no tool),
/// `more` the pages its words link to that the program wrote itself (its settings' help).
/// Each page is listed once, in the first list that holds it, and never the page itself.
pub fn sidebar(id: &str, group: &[&str], more: &[&str]) -> Sidebar {
    let mut seen: Vec<String> = vec![id.to_owned()];
    let mut take = |ids: &mut dyn Iterator<Item = &str>| -> Vec<String> {
        let mut out = vec![];
        for i in ids {
            if !seen.iter().any(|s| s == i) && !SIGNPOSTS.contains(&i) {
                seen.push(i.to_owned());
                out.push(i.to_owned());
            }
        }
        out
    };
    let like_this = take(&mut group.iter().copied());
    let own: Vec<&str> = page(id).map_or_default(|p| links(&p));
    let related = take(&mut own.into_iter().chain(more.iter().copied()));
    let links_here = take(&mut linked_from(id).into_iter());
    Sidebar {
        like_this,
        related,
        links_here,
    }
}

/// The pages read before and after the one shown, for the Back and Forward buttons.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct History<T> {
    back: Vec<T>,
    forward: Vec<T>,
}

impl<T> Default for History<T> {
    fn default() -> Self {
        Self {
            back: vec![],
            forward: vec![],
        }
    }
}

/// The most pages Back remembers.
const REMEMBERED: usize = 100;

impl<T: Clone + PartialEq> History<T> {
    /// Going from page `from` to a new page by a link: Back returns to `from`, and what
    /// Forward held is dropped, as in a web browser.
    pub fn visit(&mut self, from: T) {
        if self.back.last() != Some(&from) {
            self.back.push(from);
        }
        if self.back.len() > REMEMBERED {
            self.back.remove(0);
        }
        self.forward.clear();
    }

    /// The page Back goes to from `current`, if any; Forward then returns to `current`.
    pub fn back(&mut self, current: T) -> Option<T> {
        let to = self.back.pop()?;
        self.forward.push(current);
        Some(to)
    }

    /// The page Forward goes to from `current`, if any; Back then returns to `current`.
    pub fn forward(&mut self, current: T) -> Option<T> {
        let to = self.forward.pop()?;
        self.back.push(current);
        Some(to)
    }

    /// Whether Back has somewhere to go.
    pub fn can_go_back(&self) -> bool {
        !self.back.is_empty()
    }

    /// Whether Forward has somewhere to go.
    pub fn can_go_forward(&self) -> bool {
        !self.forward.is_empty()
    }
}

/// A name that, where a page's words say it, becomes a link to its page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name {
    /// The words, as the page's title says them.
    pub words: String,
    /// The page they link to.
    pub id: String,
    /// Whether the words must match their capitals too (a tool's name, "Draw Circle"), or
    /// match however they are written (an idea's name: "stepover", "Stepover").
    pub exact_case: bool,
}

/// `markdown` with the first time it says each of `names` made a link to that page, as a
/// wiki does: never in a heading, a picture, code, or words already a link, never to page
/// `own` itself, and never to a page the page links to already. Longer names win, so
/// "Pocket Toolpath" links whole, not as "Pocket".
pub fn autolink(markdown: &str, own: &str, names: &[Name]) -> String {
    let mut names: Vec<&Name> = names
        .iter()
        .filter(|n| n.id != own && !n.words.trim().is_empty())
        .collect();
    names.sort_by_key(|n| std::cmp::Reverse(n.words.len()));
    let mut done: Vec<&str> = crate::help_links(markdown);
    let mut out = String::with_capacity(markdown.len() + 64);
    let mut fenced = false;
    for line in markdown.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            fenced = !fenced;
        }
        if fenced || trimmed.starts_with('#') || trimmed.starts_with("```") {
            out.push_str(line);
            continue;
        }
        let mut line = line.to_owned();
        for name in &names {
            if done.contains(&name.id.as_str()) {
                continue;
            }
            if let Some(at) = find_free(&line, &name.words, name.exact_case) {
                let end = at + name.words.len();
                let linked = format!("[{}](help:{})", &line[at..end], name.id);
                line.replace_range(at..end, &linked);
                done.push(&name.id);
            }
        }
        out.push_str(&line);
    }
    out
}

/// Where `words` first stand on their own in `line` outside a link, picture or code.
fn find_free(line: &str, words: &str, exact_case: bool) -> Option<usize> {
    let hay = if exact_case {
        line.to_owned()
    } else {
        line.to_lowercase()
    };
    let needle = if exact_case {
        words.to_owned()
    } else {
        words.to_lowercase()
    };
    // lower-casing can change a string's length (rare letters): then match only exactly
    if hay.len() != line.len() {
        return None;
    }
    let taken = protected(line);
    let mut from = 0;
    while let Some(i) = hay[from..].find(&needle) {
        let at = from + i;
        let end = at + needle.len();
        let before = hay[..at].chars().next_back();
        let after = hay[end..].chars().next();
        let alone = !before.is_some_and(is_word) && !after.is_some_and(is_word);
        if alone && !taken.iter().any(|(s, e)| at < *e && end > *s) {
            return Some(at);
        }
        from = at + needle.len().max(1);
        while !hay.is_char_boundary(from) {
            from += 1;
        }
    }
    None
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The byte ranges of `line` a link must not go into: links, pictures and inline code.
fn protected(line: &str) -> Vec<(usize, usize)> {
    let mut out = vec![];
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'`' => {
                let end = line[i + 1..]
                    .find('`')
                    .map_or(line.len(), |e| i + 1 + e + 1);
                out.push((i, end));
                i = end;
            }
            b'[' | b'!' => {
                let start = i;
                let open = if bytes[i] == b'!' { i + 1 } else { i };
                if bytes.get(open) != Some(&b'[') {
                    i += 1;
                    continue;
                }
                let Some(close) = line[open..].find("](").map(|c| open + c) else {
                    i += 1;
                    continue;
                };
                let end = line[close..]
                    .find(')')
                    .map_or(line.len(), |e| close + e + 1);
                out.push((start, end));
                i = end;
            }
            b'<' => {
                // an autolink or HTML tag
                let end = line[i..].find('>').map_or(i + 1, |e| i + e + 1);
                out.push((i, end));
                i = end;
            }
            _ => i += 1,
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(words: &str, id: &str, exact_case: bool) -> Name {
        Name {
            words: words.into(),
            id: id.into(),
            exact_case,
        }
    }

    #[test]
    fn the_side_bar_lists_each_page_once_in_its_first_list() {
        // Pocket's header names Profile; Profile is also in its group
        let bar = sidebar(
            "pocket_toolpath",
            &["profile_toolpath", "pocket_toolpath", "drilling_toolpath"],
            &["stepover"],
        );
        assert_eq!(bar.like_this, ["profile_toolpath", "drilling_toolpath"]);
        assert!(!bar.related.contains(&"profile_toolpath".to_owned()));
        assert!(
            !bar.related.contains(&"pocket_toolpath".to_owned()),
            "never itself"
        );
        assert!(bar.related.contains(&"stepover".to_owned()), "{bar:?}");
        for list in [&bar.like_this, &bar.related, &bar.links_here] {
            assert!(
                !list.contains(&"contents".to_owned()),
                "no signposts: {bar:?}"
            );
        }
    }

    #[test]
    fn what_links_here_comes_back_from_pages_that_point_in() {
        // Draw Circle's page links to Pocket, so Pocket lists it unless already above
        let bar = sidebar("pocket_toolpath", &[], &[]);
        let all: Vec<&String> = bar
            .like_this
            .iter()
            .chain(&bar.related)
            .chain(&bar.links_here)
            .collect();
        assert!(all.iter().any(|i| *i == "draw_circle"), "{bar:?}");
    }

    #[test]
    fn back_and_forward_walk_the_pages_read() {
        let mut h = History::default();
        assert!(!h.can_go_back() && !h.can_go_forward());
        h.visit("contents");
        h.visit("design");
        // now on draw_circle
        assert_eq!(h.back("draw_circle"), Some("design"));
        assert_eq!(h.back("design"), Some("contents"));
        assert_eq!(h.back("contents"), None);
        assert_eq!(h.forward("contents"), Some("design"));
        assert!(h.can_go_forward());
        // a new link from here drops what was ahead
        h.visit("design");
        assert!(!h.can_go_forward());
        assert_eq!(h.back("pocket"), Some("design"));
    }

    #[test]
    fn back_remembers_a_page_once_when_it_is_left_twice_in_a_row() {
        let mut h = History::default();
        h.visit("a");
        h.visit("a");
        assert_eq!(h.back("b"), Some("a"));
        assert!(!h.can_go_back());
    }

    #[test]
    fn a_name_in_the_words_links_to_its_page_once() {
        let names = [
            name("Draw Circle", "draw_circle", true),
            name("stepover", "stepover", false),
        ];
        let md = "Use Draw Circle, then Draw Circle again.\n\nA bigger Stepover is faster.\n";
        assert_eq!(
            autolink(md, "pocket_toolpath", &names),
            "Use [Draw Circle](help:draw_circle), then Draw Circle again.\n\n\
             A bigger [Stepover](help:stepover) is faster.\n"
        );
    }

    #[test]
    fn links_stay_out_of_headings_pictures_code_and_links() {
        let names = [name("Draw Circle", "draw_circle", true)];
        for md in [
            "# Draw Circle\n",
            "![Draw Circle](pictures/x.png)\n",
            "Type `Draw Circle`.\n",
            "See [the Draw Circle tool](help:other).\n",
            "```\nDraw Circle\n```\n",
            "Draw Circles are round.\n",
        ] {
            assert_eq!(autolink(md, "x", &names), md, "{md}");
        }
    }

    #[test]
    fn a_page_never_links_itself_or_a_page_it_links_already() {
        let names = [name("Draw Circle", "draw_circle", true)];
        let md = "Draw Circle draws.\n";
        assert_eq!(autolink(md, "draw_circle", &names), md);
        let linked = "[Circles](help:draw_circle) and Draw Circle.\n";
        assert_eq!(autolink(linked, "x", &names), linked);
    }

    #[test]
    fn the_longer_name_wins() {
        let names = [
            name("Pocket", "pocket", true),
            name("Pocket Toolpath", "pocket_toolpath", true),
        ];
        assert_eq!(
            autolink("Make a Pocket Toolpath.\n", "x", &names),
            "Make a [Pocket Toolpath](help:pocket_toolpath).\n"
        );
    }

    #[test]
    fn capitals_matter_only_for_exact_names() {
        let names = [name("Node Editing", "node_editing", true)];
        let md = "node editing by hand\n";
        assert_eq!(autolink(md, "x", &names), md);
    }
}
