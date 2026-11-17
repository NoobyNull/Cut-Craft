//! Help Contents (F1 help): Markdown pages, the app's own and ones made from the
//! tools' doc comments, linked to each other with `help:` links.

use super::{CutCraftApp, Info, Page, dialog_size, egui, form, gui};
use egui_commonmark::CommonMarkViewer;
use std::fmt::Write as _;

const CONTENTS: &str = include_str!("../../help/contents.md");
const DESIGN: &str = include_str!("../../help/design.md");
const TOOLPATHS: &str = include_str!("../../help/toolpaths.md");
const WEB_API: &str = include_str!("../../help/web-api.md");
const VIEW_3D: &str = include_str!("../../help/view-3d.md");
const GROUPS: &str = include_str!("../../help/groups.md");
const PENCIL: &str = include_str!("../../help/3d-pencil.md");
const UPDATES: &str = include_str!("../../help/updates.md");
const MODEL_POSITION: &str = include_str!("../../help/model-position.md");
const NOTES: &str = include_str!("../../help/notes.md");

/// The help's own pages: every other topic is a tool's, headed by its button's icon.
pub(super) const OWN_PAGES: [&str; 8] = [
    "contents",
    "design",
    "toolpaths",
    "tips",
    "web_api",
    "view_3d",
    "groups",
    "updates",
];

/// How many points across a tool's icon is beside its name at the top of its help page.
pub(super) const PAGE_ICON: u16 = 48;
/// And beside its name in a list of tools or of search results.
pub(super) const LIST_ICON: u16 = 24;
/// Where the help's small copies of the tool icons are kept in egui: then the size, then
/// the icon's own path (`bytes://help-icons/48/toolpaths/pocket_toolpath.png`).
const ICON_URI: &str = "bytes://help-icons/";

/// More help for a tool than its settings' doc comments give: pictures and steps, added
/// under its settings.
fn tool_extra(id: &str) -> Option<&'static str> {
    match id {
        "3d_pencil_toolpath" => Some(PENCIL),
        "material_setup" | "orientate_3d_model" => Some(MODEL_POSITION),
        "notes" => Some(NOTES),
        _ => None,
    }
}

/// The tools whose help pages link the Groups page.
const GROUP_TOOLS: [&str; 4] = [
    "group",
    "ungroup",
    "ungroup_onto_current_layer",
    "edit_text",
];

/// The tools whose help pages link the 3D View's model height buttons.
const HEIGHT_TOOLS: [&str; 2] = ["transform", "scale_model_z_height"];

/// The help's own pictures, by the `bytes://help/` name its pages show them by.
pub(super) const PICTURES: &[(&str, &[u8])] = &[
    (
        "notes-steps.gif",
        include_bytes!("../../help/pictures/notes-steps.gif"),
    ),
    (
        "notes-explorer.png",
        include_bytes!("../../help/pictures/notes-explorer.png"),
    ),
    (
        "notes-gcode.png",
        include_bytes!("../../help/pictures/notes-gcode.png"),
    ),
    (
        "notes-preview.png",
        include_bytes!("../../help/pictures/notes-preview.png"),
    ),
    (
        "web-api-options.png",
        include_bytes!("../../help/pictures/web-api-options.png"),
    ),
    (
        "web-api-network.png",
        include_bytes!("../../help/pictures/web-api-network.png"),
    ),
    (
        "web-api-running.png",
        include_bytes!("../../help/pictures/web-api-running.png"),
    ),
    (
        "web-api-not-running.png",
        include_bytes!("../../help/pictures/web-api-not-running.png"),
    ),
    (
        "web-api-docs.png",
        include_bytes!("../../help/pictures/web-api-docs.png"),
    ),
    (
        "view-gcode.png",
        include_bytes!("../../help/pictures/view-gcode.png"),
    ),
    (
        "auto-rotate.gif",
        include_bytes!("../../help/pictures/auto-rotate.gif"),
    ),
    (
        "model-height-buttons.gif",
        include_bytes!("../../help/pictures/model-height-buttons.gif"),
    ),
    (
        "model-height-buttons.png",
        include_bytes!("../../help/pictures/model-height-buttons.png"),
    ),
    (
        "line-colors-dark.png",
        include_bytes!("../../help/pictures/line-colors-dark.png"),
    ),
    (
        "line-colors-light.png",
        include_bytes!("../../help/pictures/line-colors-light.png"),
    ),
    (
        "group-step-inside.png",
        include_bytes!("../../help/pictures/group-step-inside.png"),
    ),
    (
        "pencil-before-after.png",
        include_bytes!("../../help/pictures/pencil-before-after.png"),
    ),
    (
        "pencil-pass.gif",
        include_bytes!("../../help/pictures/pencil-pass.gif"),
    ),
    (
        "update-offer.png",
        include_bytes!("../../help/pictures/update-offer.png"),
    ),
    (
        "update-steps.gif",
        include_bytes!("../../help/pictures/update-steps.gif"),
    ),
    (
        "update-by-hand.png",
        include_bytes!("../../help/pictures/update-by-hand.png"),
    ),
    (
        "model-top-at-top.png",
        include_bytes!("../../help/pictures/model-top-at-top.png"),
    ),
    (
        "model-bottom-on-bottom.png",
        include_bytes!("../../help/pictures/model-bottom-on-bottom.png"),
    ),
    (
        "model-sticks-out.png",
        include_bytes!("../../help/pictures/model-sticks-out.png"),
    ),
];

/// The help's pictures handed to egui, once, so its pages can show them.
fn load_pictures(ctx: &egui::Context) {
    let id = egui::Id::unique("help_pictures_loaded");
    if ctx.data(|d| d.get_temp::<bool>(id)).is_some() {
        return;
    }
    for (name, bytes) in PICTURES {
        ctx.include_bytes(format!("bytes://help/{name}"), *bytes);
    }
    ctx.data_mut(|d| d.insert_temp(id, true));
}

impl CutCraftApp {
    /// The Help window on `topic`: a `help:` link clicked goes to its page. The search box
    /// along its top lists every page whose name or words match, in place of the page.
    pub(super) fn help_window(&mut self, ctx: &egui::Context, topic: &'static str) {
        load_pictures(ctx);
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
        let found_icons: Vec<Option<String>> = found
            .iter()
            .map(|(t, ..)| self.help_icon_uri(t, LIST_ICON))
            .collect();
        self.load_help_icons(ctx, &markdown);
        for uri in found_icons.iter().flatten() {
            self.load_help_icons(ctx, uri);
        }
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
                let edge = egui::Vec2::splat(f32::from(LIST_ICON));
                for ((t, title, line), icon) in found.iter().zip(&found_icons) {
                    ui.horizontal(|ui| {
                        if let Some(uri) = icon {
                            ui.add(egui::Image::new(uri.clone()).fit_to_exact_size(edge));
                        }
                        if ui.link(egui::RichText::new(title).strong()).clicked() {
                            picked = Some(*t);
                        }
                    });
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
                .map(|t| {
                    (
                        t,
                        self.help_title(t),
                        without_pictures(&self.help_markdown(t)),
                    )
                })
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
            "web_api" => "Web API".into(),
            "view_3d" => "3D View".into(),
            "groups" => "Groups".into(),
            "updates" => "Updates".into(),
            id => self.tool_label(id),
        }
    }

    /// Every help page: Contents, the two tool pages, the tips, and each of the tools.
    pub(super) fn help_topics(&self) -> Vec<&'static str> {
        let mut topics = OWN_PAGES.to_vec();
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
                    let icon = self
                        .help_icon_uri(id, LIST_ICON)
                        .map_or_else(String::new, |uri| format!("![]({uri}) "));
                    let _ = writeln!(md, "- {icon}[{}](help:{id})", self.tool_label(id));
                }
            }
            md
        };
        match topic {
            "contents" => CONTENTS.to_owned(),
            "design" => tools_page(Page::Design, DESIGN),
            "toolpaths" => tools_page(Page::Toolpaths, TOOLPATHS),
            "tips" => super::tips::markdown(),
            "web_api" => WEB_API.to_owned(),
            "view_3d" => VIEW_3D.to_owned(),
            "groups" => GROUPS.to_owned(),
            "updates" => UPDATES.to_owned(),
            id => {
                let (page, name) = match self.page_of(id) {
                    Page::Design => ("design", "Design"),
                    Page::Toolpaths => ("toolpaths", "Toolpaths"),
                };
                let (about, fields) = self.tool_help(id);
                let about = about.unwrap_or_else(|| "No help written for this tool yet.".into());
                let label = self.tool_label(id);
                let icon = self
                    .help_icon_uri(id, PAGE_ICON)
                    .map_or_else(String::new, |uri| format!("![{label} icon]({uri}) "));
                let mut md = format!(
                    "[Contents](help:contents) ▸ [{name}](help:{page})\n\n# {icon}{label}\n\n\
                     {about}\n\n"
                );
                for (label, help) in fields {
                    let _ = writeln!(md, "- **{label}**: {help}");
                }
                if let Some(extra) = tool_extra(id) {
                    md.push_str(extra);
                }
                if GROUP_TOOLS.contains(&id) {
                    md.push_str(
                        "\nDouble-click a group to step inside it and change one thing in it. \
                         See [Groups](help:groups).\n",
                    );
                }
                if HEIGHT_TOOLS.contains(&id) {
                    md.push_str(
                        "\nTo make one model taller or lower, double-click it in the 3D view, \
                         then double-click it again for its height buttons. See \
                         [3D View](help:view_3d), Changing a model's height.\n",
                    );
                }
                md
            }
        }
    }

    /// Where the help shows tool `id`'s button icon, `edge` points across: the same picture
    /// as its button, made small. `None` for a tool with no icon.
    pub(super) fn help_icon_uri(&self, id: &str, edge: u16) -> Option<String> {
        let key = self.icons.find(self.explicit_icon(id), id)?;
        Some(format!("{ICON_URI}{edge}/{key}"))
    }

    /// Hands egui each small tool icon `text` names that it does not have yet.
    fn load_help_icons(&self, ctx: &egui::Context, text: &str) {
        for uri in icon_uris(text) {
            let id = egui::Id::unique(("help_icon", uri));
            if ctx.data(|d| d.get_temp::<bool>(id)).is_some() {
                continue;
            }
            let small = uri
                .strip_prefix(ICON_URI)
                .and_then(|rest| rest.split_once('/'))
                .and_then(|(edge, key)| self.icons.small_png(key, edge.parse().ok()?));
            if let Some(png) = small {
                ctx.include_bytes(uri.to_owned(), png);
            }
            ctx.data_mut(|d| d.insert_temp(id, true));
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

/// The small tool icons `text` names, by their `bytes://help-icons/` URI.
fn icon_uris(text: &str) -> impl Iterator<Item = &str> {
    text.match_indices(ICON_URI).map(|(at, _)| {
        let rest = &text[at..];
        &rest[..rest.find([')', ' ', '\n']).unwrap_or(rest.len())]
    })
}

/// Markdown with each picture as its words alone: search finds a page by what it says, not
/// by its pictures' file names.
fn without_pictures(md: &str) -> String {
    let mut out = String::with_capacity(md.len());
    let mut rest = md;
    while let Some(open) = rest.find("![") {
        let after = &rest[open + 2..];
        let Some((alt, tail)) = after
            .split_once("](")
            .and_then(|(alt, tail)| Some((alt, tail.split_once(')')?.1)))
        else {
            break;
        };
        out.push_str(&rest[..open]);
        out.push_str(alt);
        // a picture with no words leaves no gap where it was
        rest = if alt.is_empty() {
            tail.strip_prefix(' ').unwrap_or(tail)
        } else {
            tail
        };
    }
    out.push_str(rest);
    out
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
    use super::{icon_uris, plain_line, search, without_pictures};

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
    fn search_reads_a_pictures_words_not_its_file_name() {
        assert_eq!(
            without_pictures(
                "# ![Pocket icon](bytes://help-icons/48/toolpaths/pocket.png) Pocket\n\
                 - ![](bytes://help-icons/24/a.png) [A](help:a)\nno [link](help:b) lost ![x"
            ),
            "# Pocket icon Pocket\n- [A](help:a)\nno [link](help:b) lost ![x"
        );
    }

    #[test]
    fn a_pages_small_icons_are_found_by_their_uri() {
        let md = "# ![P](bytes://help-icons/48/t/p.png) P\n- ![](bytes://help-icons/24/d/c.png) C";
        let found: Vec<&str> = icon_uris(md).collect();
        assert_eq!(
            found,
            [
                "bytes://help-icons/48/t/p.png",
                "bytes://help-icons/24/d/c.png"
            ]
        );
        assert_eq!(icon_uris("bytes://help-icons/24/x.png").count(), 1);
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
