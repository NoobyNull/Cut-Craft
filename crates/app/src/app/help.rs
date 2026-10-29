//! Help Contents (Aspire's F1 help): Markdown pages, the app's own and ones made from the
//! tools' doc comments, linked to each other with `help:` links.

use super::{CutCraftApp, Info, Page, dialog_size, egui, form, gui};
use egui_commonmark::CommonMarkViewer;
use std::fmt::Write as _;

const CONTENTS: &str = include_str!("../../help/contents.md");
const DESIGN: &str = include_str!("../../help/design.md");
const TOOLPATHS: &str = include_str!("../../help/toolpaths.md");

impl CutCraftApp {
    /// The Help window on `topic`: a `help:` link clicked goes to its page.
    pub(super) fn help_window(&mut self, ctx: &egui::Context, topic: &'static str) {
        let topics = self.help_topics();
        let markdown = self.help_markdown(topic);
        for t in &topics {
            self.help_cache.add_link_hook(format!("help:{t}"));
        }
        let (mut open, cache) = (true, &mut self.help_cache);
        let (id, size) = (egui::Id::unique("help"), dialog_size(ctx));
        cut_craft_bug_report::dialog(ctx, id, "Cut Craft Help", size, &mut open, |ui| {
            egui::ScrollArea::vertical().id_salt(topic).show(ui, |ui| {
                CommonMarkViewer::new().show(ui, cache, &markdown);
            });
        });
        let clicked = topics
            .into_iter()
            .find(|t| self.help_cache.get_link_hook(&format!("help:{t}")) == Some(true));
        self.info = match (open, clicked) {
            (false, _) => None,
            (true, Some(t)) => Some(Info::Help(t)),
            (true, None) => Some(Info::Help(topic)),
        };
    }

    /// Every help page: Contents, the two tool pages, and each of their tools.
    fn help_topics(&self) -> Vec<&'static str> {
        let mut topics = vec!["contents", "design", "toolpaths"];
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
            "contents" => CONTENTS.to_owned(),
            "design" => tools_page(Page::Design, DESIGN),
            "toolpaths" => tools_page(Page::Toolpaths, TOOLPATHS),
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
                for (label, help) in fields {
                    let _ = writeln!(md, "- **{label}**: {help}");
                }
                md
            }
        }
    }

    /// A command's help, from its params' doc comments: what it does, then each setting.
    pub(super) fn tool_help(&self, id: &str) -> (Option<String>, Vec<(String, String)>) {
        let Some(cmd) = self.registry.get(id) else {
            return (None, vec![]);
        };
        let schema = cmd.schema();
        let about = schema["description"].as_str().map(String::from);
        let fields = form::fields(&schema)
            .into_iter()
            .filter_map(|f| Some((f.label, f.help?)))
            .collect();
        (about, fields)
    }
}
