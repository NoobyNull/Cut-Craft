//! Help ▸ Credits: the libraries, pictures and icons PowerWhittle is built with, and their
//! licences. The texts are the committed `credits` folder at the top of the repository:
//! `libraries.md` is made by cargo-about from Cargo.lock (CI fails when it is stale) and
//! `other.md`, for what Cargo.lock does not list, is kept by hand.

use cut_craft_i18n::tr;

/// Every library's licence, made from Cargo.lock by `cargo about generate`.
pub const LIBRARIES: &str = include_str!("../../../credits/libraries.md");
/// What Cargo.lock does not list (bundled pictures, icons and the Rust standard library),
/// credited by hand.
pub const OTHER: &str = include_str!("../../../credits/other.md");

/// One credit: a `## ` heading of a credits file and what follows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Credit<'a> {
    pub title: &'a str,
    /// Markdown: who and what it covers, then the licence text in a fenced block.
    pub body: &'a str,
}

impl Credit<'_> {
    /// The part a search looks in: the title and everything but the licence text itself, so
    /// "MIT" finds the MIT licences and not every licence that mentions it.
    fn searchable(&self) -> String {
        let outside_fences = self
            .body
            .split("```")
            .step_by(2)
            .collect::<Vec<_>>()
            .join(" ");
        format!("{} {outside_fences}", self.title).to_lowercase()
    }

    /// What the credit covers: the libraries' names from its "Used by:" line (without
    /// versions), else that line's words as they are.
    pub fn used_by(&self) -> Vec<&str> {
        let Some(line) = self
            .body
            .lines()
            .find_map(|line| line.strip_prefix("Used by:"))
        else {
            return vec![];
        };
        if !line.contains('[') {
            return vec![line.trim().trim_end_matches('.')];
        }
        line.split('[')
            .skip(1)
            .filter_map(|link| link.split_once(']'))
            .map(|(text, _)| text.split_once(' ').map_or(text, |(name, _)| name))
            .collect()
    }

    /// The line a collapsed credit shows: its licence and the first few names it covers.
    pub fn heading(&self) -> String {
        const NAMED: usize = 3;
        let names = self.used_by();
        let shown = names
            .iter()
            .take(NAMED)
            .copied()
            .collect::<Vec<_>>()
            .join(", ");
        let who = if names.len() > NAMED {
            tr!("credits-more", names = shown, count = names.len() - NAMED)
        } else {
            shown
        };
        if who.is_empty() {
            self.title.to_owned()
        } else {
            format!("{}: {who}", self.title)
        }
    }

    /// Does this credit match what was typed in the search box? Empty matches everything.
    pub fn matches(&self, query: &str) -> bool {
        let words = query.to_lowercase();
        let text = self.searchable();
        words.split_whitespace().all(|word| text.contains(word))
    }
}

/// The credits in a credits file, in order. The text before the first `## ` is its
/// introduction and is not a credit. A `## ` inside a fenced licence text (some licences are
/// Markdown) stays part of that text.
pub fn credits(markdown: &str) -> Vec<Credit<'_>> {
    // (where the heading line starts, where it ends, the title)
    let mut headings = Vec::new();
    let (mut in_fence, mut offset) = (false, 0);
    for line in markdown.split_inclusive('\n') {
        let bare = line.trim_end();
        if bare.starts_with("```") {
            in_fence = !in_fence;
        } else if !in_fence && let Some(title) = bare.strip_prefix("## ") {
            headings.push((offset, offset + line.len(), title.trim()));
        }
        offset += line.len();
    }
    headings
        .iter()
        .enumerate()
        .map(|(n, &(_, body_start, title))| {
            let body_end = headings.get(n + 1).map_or(markdown.len(), |next| next.0);
            Credit {
                title,
                body: markdown[body_start..body_end].trim(),
            }
        })
        .collect()
}

/// Every credit PowerWhittle ships: the hand-kept ones first, then the libraries.
pub fn all() -> Vec<Credit<'static>> {
    let mut all = credits(OTHER);
    all.extend(credits(LIBRARIES));
    all
}

/// The Credits window's search and rendered texts, kept while it is open.
#[derive(Default)]
pub struct Window {
    pub search: String,
    cache: egui_commonmark::CommonMarkCache,
}

/// Help ▸ Credits: a thank-you, a search box and every credit, folded to one line each.
pub fn show(ui: &mut egui::Ui, window: &mut Window) {
    ui.label(tr!("credits-intro"));
    ui.add(
        egui::TextEdit::singleline(&mut window.search)
            .hint_text(tr!("credits-search"))
            .desired_width(f32::INFINITY),
    );
    let all = all();
    let shown: Vec<_> = all
        .iter()
        .enumerate()
        .filter(|(_, credit)| credit.matches(&window.search))
        .collect();
    if shown.is_empty() {
        ui.label(tr!("credits-none", search = window.search.trim()));
        return;
    }
    ui.weak(tr!(
        "credits-showing",
        shown = shown.len(),
        total = all.len()
    ));
    egui::ScrollArea::vertical()
        .id_salt("credits")
        .max_height(400.0)
        .show(ui, |ui| {
            for (n, credit) in shown {
                egui::CollapsingHeader::new(credit.heading())
                    .id_salt(("credit", n))
                    .show(ui, |ui| {
                        egui_commonmark::CommonMarkViewer::new().show(
                            ui,
                            &mut window.cache,
                            credit.body,
                        );
                    });
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# Libraries\n\nIntro text.\n\n\
## MIT License\n\nUsed by: [serde 1.0](https://crates.io/crates/serde)\n\n```\nMIT text\n```\n\n\
## Apache License 2.0\n\nUsed by: [glam 0.30](https://crates.io/crates/glam)\n\n```\nApache text naming MIT\n```\n";

    #[test]
    fn a_credits_file_splits_into_its_headings_without_the_introduction() {
        let found = credits(SAMPLE);
        let titles: Vec<_> = found.iter().map(|c| c.title).collect();
        assert_eq!(titles, ["MIT License", "Apache License 2.0"]);
        assert!(found[0].body.starts_with("Used by: [serde"));
        assert!(found[0].body.contains("MIT text"));
    }

    #[test]
    fn a_heading_inside_a_licence_text_stays_part_of_that_text() {
        let file = "# Libraries\n\n## CDLA\n\nUsed by: x\n\n```\n# CDLA\n## 1. Provision\n```\n\n## MIT License\n\nUsed by: y\n";
        let found = credits(file);
        let titles: Vec<_> = found.iter().map(|c| c.title).collect();
        assert_eq!(titles, ["CDLA", "MIT License"]);
        assert!(found[0].body.contains("## 1. Provision"));
    }

    #[test]
    fn a_folded_credit_names_its_licence_and_the_first_few_libraries() {
        let found = credits(SAMPLE);
        assert_eq!(found[0].used_by(), ["serde"]);
        assert_eq!(found[0].heading(), "MIT License: serde");
        let many = Credit {
            title: "MIT License",
            body: "Used by: [a 1](u), [b 2](u), [c 3](u), [d 4](u), [e 5](u)\n\n```\nt\n```",
        };
        assert_eq!(many.heading(), "MIT License: a, b, c and 2 more");
        let by_hand = Credit {
            title: "Link icons",
            body: "Used by: the Help menu.\n\nIcons by Icons8.",
        };
        assert_eq!(by_hand.heading(), "Link icons: the Help menu");
    }

    #[test]
    fn search_finds_a_library_by_name_and_skips_words_inside_licence_texts() {
        let found = credits(SAMPLE);
        let hits = |q: &str| -> Vec<&str> {
            found
                .iter()
                .filter(|c| c.matches(q))
                .map(|c| c.title)
                .collect()
        };
        assert_eq!(hits("SERDE"), ["MIT License"]);
        assert_eq!(hits("mit"), ["MIT License"], "not Apache's text naming MIT");
        assert_eq!(hits("glam apache"), ["Apache License 2.0"]);
        assert_eq!(hits("").len(), 2, "an empty search shows everything");
        assert_eq!(hits("no-such-library"), Vec::<&str>::new());
    }

    #[test]
    fn the_shipped_credits_name_the_main_libraries_with_their_licence_texts() {
        let libraries = credits(LIBRARIES);
        assert!(libraries.len() > 5, "{} licences", libraries.len());
        for name in ["egui", "serde", "wgpu", "usvg"] {
            assert!(
                libraries.iter().any(|c| c.matches(name)),
                "{name} is credited"
            );
        }
        for credit in &libraries {
            assert!(credit.body.contains("Used by:"), "{}", credit.title);
            assert!(credit.body.contains("```"), "{} has its text", credit.title);
        }
    }

    #[test]
    fn the_shipped_credits_keep_the_hand_kept_picture_icon_and_rust_credits() {
        let other = credits(OTHER);
        assert!(other.iter().any(|c| c.matches("icons8")));
        assert!(other.iter().any(|c| c.matches("printables")));
        assert!(other.iter().any(|c| c.matches("rust standard library")));
        assert_eq!(all().len(), other.len() + credits(LIBRARIES).len());
    }
}
