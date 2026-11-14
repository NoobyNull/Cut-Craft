//! Help Contents (F1 help): Markdown pages, the app's own and ones made from the
//! tools' doc comments, linked to each other with `help:` links.

use super::{CutCraftApp, Info, Page, dialog_size, egui, form, gui};
use egui_commonmark::CommonMarkViewer;
use std::fmt::Write as _;

/// The words of help page `id` from the help crate, without its header: empty when it has none.
fn written(id: &str) -> &'static str {
    cut_craft_help::page(id).map_or("", |p| p.body)
}

impl CutCraftApp {
    /// The Help window on `topic`: a `help:` link clicked goes to its page. The search box
    /// along its top lists every page whose name or words match, in place of the page.
    pub(super) fn help_window(&mut self, ctx: &egui::Context, topic: &'static str) {
        let topics = self.help_topics();
        for t in &topics {
            self.help_cache.add_link_hook(format!("help:{t}"));
        }
        let query = self.help_search.trim().to_owned();
        let found = if query.is_empty() {
            vec![]
        } else {
            self.help_search_results(&query)
        };
        let markdown = if query.is_empty() {
            self.help_markdown(topic)
        } else {
            String::new()
        };
        let mut picked = None;
        let (mut open, cache, search) = (true, &mut self.help_cache, &mut self.help_search);
        let (id, size) = (egui::Id::unique("help"), dialog_size(ctx));
        cut_craft_bug_report::dialog(ctx, id, "Cut Craft Help", size, &mut open, |ui| {
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(search)
                        .hint_text("🔍 Search help: a tool, a setting or a word")
                        .desired_width(ui.available_width() * 0.6),
                );
                if !search.is_empty() && ui.button("Clear").clicked() {
                    search.clear();
                }
                if ui.button("Contents").clicked() {
                    picked = Some("contents");
                }
                if ui.button("Did You Know?").clicked() {
                    picked = Some("tips");
                }
            });
            ui.separator();
            egui::ScrollArea::vertical().id_salt(topic).show(ui, |ui| {
                if query.is_empty() {
                    CommonMarkViewer::new().show(ui, cache, &markdown);
                    return;
                }
                if found.is_empty() {
                    ui.label(format!(
                        "Nothing in the help matches \"{query}\". Try fewer or shorter words, \
                         such as \"pocket\" or \"tabs\"."
                    ));
                    return;
                }
                ui.weak(match found.len() {
                    1 => "1 page found".to_owned(),
                    n => format!("{n} pages found"),
                });
                for (t, title, line) in &found {
                    if ui.link(egui::RichText::new(title).strong()).clicked() {
                        picked = Some(*t);
                    }
                    if !line.is_empty() {
                        ui.weak(line);
                    }
                    ui.add_space(ui.spacing().item_spacing.y);
                }
            });
        });
        let clicked = picked.or_else(|| {
            topics
                .into_iter()
                .find(|t| self.help_cache.get_link_hook(&format!("help:{t}")) == Some(true))
        });
        if picked.is_some() {
            self.help_search.clear();
        }
        self.info = match (open, clicked) {
            (false, _) => None,
            (true, Some(t)) => Some(Info::Help(t)),
            (true, None) => Some(Info::Help(topic)),
        };
    }

    /// The pages matching every word of `query` (any case), as (topic, title, the first
    /// line that matches): those with a word in their title first, then in Help's own order.
    pub(super) fn help_search_results(
        &mut self,
        query: &str,
    ) -> Vec<(&'static str, String, String)> {
        if self.help_index.is_empty() {
            self.help_index = self
                .help_topics()
                .into_iter()
                .map(|t| (t, self.help_title(t), self.help_markdown(t)))
                .collect();
        }
        search(&self.help_index, query)
    }

    /// A help page's name, as its search result shows it.
    fn help_title(&self, topic: &str) -> String {
        match topic {
            "contents" => "Help Contents".into(),
            "design" => "Design page".into(),
            "toolpaths" => "Toolpaths page".into(),
            "tips" => "Did You Know?".into(),
            id => self.tool_label(id),
        }
    }

    /// Every help page: Contents, the two tool pages, the tips, and each of the tools.
    pub(super) fn help_topics(&self) -> Vec<&'static str> {
        let mut topics = vec!["contents", "design", "toolpaths", "tips"];
        for page in [Page::Design, Page::Toolpaths] {
            topics.extend(self.help_groups(page).iter().flat_map(|g| g.tools));
        }
        topics
    }

    fn help_groups(&self, page: Page) -> Vec<gui::ToolGroup> {
        match page {
            Page::Design => self.design_groups("Design"),
            Page::Toolpaths => self.toolbar.toolpath_groups.clone(),
        }
    }

    /// A help page's Markdown: Contents as written, a tool page's intro then its tools by
    /// group as links, or a tool's help from its params' doc comments.
    pub(super) fn help_markdown(&self, topic: &str) -> String {
        let tools_page = |page: Page, intro: &str| {
            let mut md = format!("[Contents](help:contents)\n\n{intro}");
            for g in self.help_groups(page) {
                let _ = write!(md, "\n## {}\n\n", g.name);
                for id in g.tools {
                    let _ = writeln!(md, "- [{}](help:{id})", self.tool_label(id));
                }
            }
            md
        };
        match topic {
            "contents" => written("contents").to_owned(),
            "design" => tools_page(Page::Design, written("design")),
            "toolpaths" => tools_page(Page::Toolpaths, written("toolpaths")),
            "tips" => super::tips::markdown(),
            id => {
                let (page, name) = match self.page_of(id) {
                    Page::Design => ("design", "Design"),
                    Page::Toolpaths => ("toolpaths", "Toolpaths"),
                };
                let (about, fields) = self.tool_help(id);
                let about = about.unwrap_or_else(|| "No help written for this tool yet.".into());
                let mut md = format!(
                    "[Contents](help:contents) ▸ [{name}](help:{page})\n\n# {}\n\n{about}\n\n",
                    self.tool_label(id)
                );
                let steps = written(id);
                if !steps.trim().is_empty() {
                    let _ = write!(md, "{}\n\n", steps.trim_end());
                }
                if !fields.is_empty() {
                    md.push_str("## Settings\n\n");
                }
                for (label, help) in fields {
                    let _ = writeln!(md, "- **{label}**: {help}");
                }
                let related = cut_craft_help::related_markdown(id, |t| self.help_title(t));
                if !related.is_empty() {
                    let _ = write!(md, "\n{related}");
                }
                md
            }
        }
    }

    /// A command's help, from its params' doc comments (else its crate's), what it does, then
    /// each setting.
    pub(super) fn tool_help(&self, id: &str) -> (Option<String>, Vec<(String, String)>) {
        let own = || gui::intro(id, &self.tool_label(id));
        let Some(cmd) = self.registry.get(form_of(id)) else {
            return (own(), vec![]);
        };
        let schema = cmd.schema();
        let about = schema["description"]
            .as_str()
            .map(String::from)
            .or_else(own);
        let fields = form::fields(&schema)
            .into_iter()
            .filter_map(|f| Some((f.label, f.help?)))
            .collect();
        (about, fields)
    }
}

/// The pages of `index` (topic, title, Markdown) matching every word of `query`, any case,
/// with the first line holding one: title matches first, otherwise in the index's order.
fn search(
    index: &[(&'static str, String, String)],
    query: &str,
) -> Vec<(&'static str, String, String)> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    if words.is_empty() {
        return vec![];
    }
    let mut found: Vec<(bool, &'static str, String, String)> = index
        .iter()
        .filter_map(|(topic, title, md)| {
            let (title_lc, md_lc) = (title.to_lowercase(), md.to_lowercase());
            let all = words
                .iter()
                .all(|w| title_lc.contains(w.as_str()) || md_lc.contains(w.as_str()));
            if !all {
                return None;
            }
            let in_title = words.iter().any(|w| title_lc.contains(w.as_str()));
            let line = md
                .lines()
                .filter(|l| !l.starts_with('[') && !l.starts_with('#'))
                .find(|l| {
                    let l = l.to_lowercase();
                    words.iter().any(|w| l.contains(w.as_str()))
                })
                .map_or_else(String::new, plain_line);
            Some((in_title, *topic, title.clone(), line))
        })
        .collect();
    // stable: within each kind, Help's own order
    found.sort_by_key(|(in_title, ..)| !in_title);
    found
        .into_iter()
        .map(|(_, t, title, line)| (t, title, line))
        .collect()
}

/// A Markdown line as plain words: no list dash, bold stars or link targets.
fn plain_line(line: &str) -> String {
    let mut out = String::new();
    let mut rest = line.trim().trim_start_matches("- ");
    while let Some(open) = rest.find("](") {
        out.push_str(&rest[..open]);
        rest = rest[open + 2..]
            .split_once(')')
            .map_or("", |(_, after)| after);
    }
    out.push_str(rest);
    out.replace(['*', '[', '`'], "")
}

/// The command whose form tool `id` opens: itself, or the one a flyout variant or a Create
/// Shape icon or Edit Text opens with its own settings.
fn form_of(id: &str) -> &str {
    match crate::flyout::variant(id) {
        Some(v) => v.command,
        None if id.starts_with("create_shape_") => "create_shape",
        None if id == "edit_text" => "draw_text",
        None => id,
    }
}

#[cfg(test)]
mod tests {
    use super::{plain_line, search};

    fn index() -> Vec<(&'static str, String, String)> {
        vec![
            (
                "profile",
                "Profile Toolpath".into(),
                "# Profile\n\nCuts along a line.\n- **Tabs**: hold the part in place".into(),
            ),
            (
                "pocket",
                "Pocket Toolpath".into(),
                "Clears the inside of a shape, leaving tabs out".into(),
            ),
            ("tabs", "Add Tabs".into(), "Little bridges".into()),
        ]
    }

    #[test]
    fn every_word_must_match_in_the_name_or_the_words() {
        let found: Vec<_> = search(&index(), "TOOLPATH tabs")
            .into_iter()
            .map(|(t, ..)| t)
            .collect();
        assert_eq!(found, ["profile", "pocket"]);
        assert_eq!(search(&index(), "spoilboard"), vec![]);
        assert_eq!(
            search(&index(), "   "),
            vec![],
            "nothing typed, nothing found"
        );
    }

    #[test]
    fn names_that_match_come_first() {
        let found: Vec<_> = search(&index(), "tabs")
            .into_iter()
            .map(|(t, ..)| t)
            .collect();
        assert_eq!(found, ["tabs", "profile", "pocket"]);
    }

    #[test]
    fn a_result_shows_its_matching_line_as_plain_words() {
        let found = search(&index(), "hold");
        assert_eq!(found[0].2, "Tabs: hold the part in place");
        assert_eq!(
            plain_line("- [Design](help:design): the page down the left"),
            "Design: the page down the left"
        );
    }
}
