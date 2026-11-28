//! Help Contents (F1 help): Markdown pages, the app's own and ones made from the
//! tools' doc comments, linked to each other with `help:` links.

use super::help_browser::Step;
use super::{CutCraftApp, Info, Page, dialog_size, egui, form, gui};
use cut_craft_help::wiki::{Name, Sidebar, autolink, sidebar};
use cut_craft_i18n::tr;
use egui_commonmark::CommonMarkViewer;
use std::fmt::Write as _;

/// The help's own pages, which the app writes: every other topic in [`CutCraftApp::help_topics`]
/// is a tool's, headed by its button's icon.
pub(super) const OWN_PAGES: [&str; 4] = ["contents", "design", "toolpaths", "tips"];

/// Tools whose help pages are tool pages though no toolbar group holds them.
const TOOLS_WITHOUT_BUTTONS: [&str; 1] = ["job_setup"];

/// How many points across a tool's icon is beside its name at the top of its help page.
pub(super) const PAGE_ICON: u16 = 48;
/// And beside its name in a list of tools or of search results.
pub(super) const LIST_ICON: u16 = 24;
/// Where the help's small copies of the tool icons are kept in egui: then the size, then
/// the icon's own path (`bytes://help-icons/48/toolpaths/pocket_toolpath.png`).
const ICON_URI: &str = "bytes://help-icons/";

/// A side bar list's pages: (topic, title, its tool's icon).
type SidebarRows = Vec<(&'static str, String, Option<egui::Image<'static>>)>;

/// The side bar's lists, each under its heading.
type SidebarLists = Vec<(&'static str, SidebarRows)>;

/// The side bar's width to start with, points: room for a tool's name and its icon.
const SIDEBAR_WIDTH: f32 = 220.0;

/// The side bar's icons' size, points.
const SIDEBAR_ICON: f32 = 20.0;

/// The widest a help picture is shown, points: the step pictures' own width.
const HELP_PICTURE_WIDTH: usize = 720;

/// The words of help page `id` from the help crate, without its header: empty when it has none.
fn written(id: &str) -> &'static str {
    cut_craft_help::page(id).map_or("", |p| p.body)
}

impl CutCraftApp {
    /// The Help window on `topic`, laid out like a wiki: Back and Forward and the search box
    /// along its top, the page, and beside it the side bar of other tools like this one,
    /// related pages and what links here. A `help:` link clicked goes to its page; the search
    /// box lists the pages answering what is typed, best first, in place of the page.
    pub(super) fn help_window(&mut self, ctx: &egui::Context, topic: &'static str) {
        load_pictures(ctx);
        let topics = self.all_help_topics();
        for t in &topics {
            self.help.cache.add_link_hook(format!("help:{t}"));
        }
        let query = self.help.search.trim().to_owned();
        let found = if query.is_empty() {
            vec![]
        } else {
            self.help_search_results(&query)
        };
        let (markdown, bar) = if query.is_empty() {
            let md = self.help_page(topic);
            let bar = self.help_sidebar_rows(topic, &md, &topics);
            (cut_craft_help::with_picture_uris(&md), bar)
        } else {
            (String::new(), vec![])
        };
        let found_icons: Vec<Option<String>> = found
            .iter()
            .map(|(t, ..)| self.help_icon_uri(t, LIST_ICON))
            .collect();
        self.load_help_icons(ctx, &markdown);
        for uri in found_icons.iter().flatten() {
            self.load_help_icons(ctx, uri);
        }
        let (can_back, can_forward) = self.help.can_step();
        let mut picked = None;
        let mut step = None;
        let (mut open, cache, search) = (true, &mut self.help.cache, &mut self.help.search);
        let (id, size) = (egui::Id::unique("help"), dialog_size(ctx));
        cut_craft_bug_report::dialog(ctx, id, "PowerWhittle Help", size, &mut open, |ui| {
            let (stepped, top) = top_bar(ui, search, can_back, can_forward);
            step = stepped.or(step);
            picked = top.or(picked);
            ui.separator();
            if query.is_empty() {
                if let Some(t) = page_and_side_bar(ui, cache, topic, &markdown, &bar) {
                    picked = Some(t);
                }
                return;
            }
            egui::ScrollArea::vertical().id_salt(topic).show(ui, |ui| {
                if found.is_empty() {
                    ui.label(tr!("help-nothing-found", query = query.as_str()));
                    return;
                }
                if let Some(t) = results_ui(ui, &found, &found_icons) {
                    picked = Some(t);
                }
            });
        });
        let clicked = picked.or_else(|| {
            topics
                .into_iter()
                .find(|t| self.help.cache.get_link_hook(&format!("help:{t}")) == Some(true))
        });
        if picked.is_some() {
            self.help.search.clear();
        }
        let to = self.help.go(topic, step, clicked);
        self.info = match (open, to) {
            (false, _) => None,
            (true, Some(t)) => Some(Info::Help(t)),
            (true, None) => Some(Info::Help(topic)),
        };
    }

    /// The pages answering `query`, as (topic, title, the first line saying one of its words):
    /// ranked by the help crate's search, so a question ("how do I cut out a sign with
    /// tabs?") finds its answers, best first.
    pub(super) fn help_search_results(
        &mut self,
        query: &str,
    ) -> Vec<(&'static str, String, String)> {
        if self.help.needs_pages() {
            let pages = self
                .all_help_topics()
                .into_iter()
                .map(|t| {
                    (
                        t,
                        self.help_title(t),
                        without_pictures(&self.help_markdown(t)),
                    )
                })
                .collect();
            self.help.index_pages(pages);
        }
        self.help.results(query)
    }

    /// Page `topic`'s side bar as the window lists it: each list under its heading, each
    /// page with its title and its tool's icon. Lists with nothing in them are left out.
    fn help_sidebar_rows(
        &self,
        topic: &str,
        markdown: &str,
        topics: &[&'static str],
    ) -> SidebarLists {
        let bar = self.help_sidebar(topic, markdown);
        [
            ("Other tools like this", &bar.like_this),
            ("Related", &bar.related),
            ("What links here", &bar.links_here),
        ]
        .into_iter()
        .filter(|(_, ids)| !ids.is_empty())
        .map(|(heading, ids)| {
            let rows = ids
                .iter()
                .filter_map(|id| topics.iter().find(|t| **t == id.as_str()))
                .map(|t| (*t, self.help_title(t), self.help_icon(t)))
                .collect();
            (heading, rows)
        })
        .collect()
    }

    /// The icon of tool `id`, as its button shows it: for the side bar and its page's title.
    pub(super) fn help_icon(&self, id: &str) -> Option<egui::Image<'static>> {
        self.help_icon_key(id).map(|path| self.icons.image(path))
    }

    /// The icon page `id` shows, by its path: the one its header names (`icon:`, for a page
    /// that is no tool's), else the button icon of the tool it is.
    pub(super) fn help_icon_key(&self, id: &str) -> Option<&str> {
        match cut_craft_help::page(id).and_then(|p| p.icon) {
            Some(named) => self.tip_icon(named),
            None => self.icons.find(self.explicit_icon(id), id),
        }
    }

    /// The tools in the same group as tool `id` on its panel, itself among them; empty for a
    /// page that is no tool's.
    pub(super) fn help_group_of(&self, id: &str) -> Vec<&'static str> {
        [Page::Design, Page::Toolpaths]
            .into_iter()
            .flat_map(|p| self.help_groups(p))
            .find(|g| g.tools.contains(&id))
            .map_or_default(|g| g.tools.to_vec())
    }

    /// Page `topic`'s side bar: its tool group, the pages its words link to, and back.
    pub(super) fn help_sidebar(&self, topic: &str, markdown: &str) -> Sidebar {
        let group = self.help_group_of(topic);
        let more = cut_craft_help::help_links(markdown);
        sidebar(topic, &group, &more)
    }

    /// Page `topic` as the Help window shows it: [`Self::help_markdown`] with every other
    /// page's name in its words made a link the first time it is said, as a wiki does.
    pub(super) fn help_page(&self, topic: &str) -> String {
        let md = self.help_markdown(topic);
        if matches!(topic, "contents" | "design" | "toolpaths" | "tips") {
            return md;
        }
        autolink(&md, topic, &self.help_names())
    }

    /// The names a page's words are linked by ([`autolink`]): every tool's of two words or
    /// more ("Draw Circle", not "Group", a word said for other things too) with its capitals,
    /// and every idea's ("stepover") however it is written.
    fn help_names(&self) -> Vec<Name> {
        self.all_help_topics()
            .into_iter()
            .filter(|t| !matches!(*t, "contents" | "design" | "toolpaths" | "tips"))
            .filter_map(|t| {
                let words = self.help_title(t);
                let idea = cut_craft_help::IDEAS.contains(&t);
                (idea || words.split_whitespace().count() >= 2).then(|| Name {
                    words,
                    id: t.to_owned(),
                    exact_case: !idea,
                })
            })
            .collect()
    }

    /// A help page's name, as its search result shows it.
    pub(super) fn help_title(&self, topic: &str) -> String {
        match topic {
            "contents" => "Help Contents".into(),
            "design" => "Design page".into(),
            "toolpaths" => "Toolpaths page".into(),
            "tips" => tr!("help-did-you-know"),
            id if self.is_concept(id) => concept_title(id),
            id => self.tool_label(id),
        }
    }

    /// Every help page: Contents, the two tool pages, the tips, and each of the tools.
    pub(super) fn help_topics(&self) -> Vec<&'static str> {
        let mut topics = OWN_PAGES.to_vec();
        for page in [Page::Design, Page::Toolpaths] {
            topics.extend(self.help_groups(page).iter().flat_map(|g| g.tools));
        }
        // a tool with no button of its own: Setup Material's More settings for this job…
        topics.extend(TOOLS_WITHOUT_BUTTONS);
        topics
    }

    /// [`Self::help_topics`] and the help crate's other pages: the ideas behind the tools
    /// (stepover, tabs…), each linked from the tools and settings that use it.
    pub(super) fn all_help_topics(&self) -> Vec<&'static str> {
        let mut topics = self.help_topics();
        let concepts: Vec<&'static str> = cut_craft_help::pages()
            .map(|p| p.id)
            .filter(|id| !topics.contains(id))
            .collect();
        topics.extend(concepts);
        topics
    }

    /// Whether `id` is a help page of its own, not a tool's, nor one the app writes.
    fn is_concept(&self, id: &str) -> bool {
        !self.help_topics().contains(&id) && cut_craft_help::page(id).is_some()
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
                let _ = write!(md, "\n## {}\n\n", crate::tool_names::group_name(g.name));
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
            "contents" => written("contents").to_owned(),
            "design" => tools_page(Page::Design, written("design")),
            "toolpaths" => tools_page(Page::Toolpaths, written("toolpaths")),
            "tips" => super::tips::markdown(),
            id if self.is_concept(id) => {
                let words = written(id).trim_end();
                // the icon its header names, beside its title as a tool's page shows its own
                let words = match self.help_icon_uri(id, PAGE_ICON) {
                    Some(uri) if words.starts_with("# ") => {
                        let title = concept_title(id);
                        format!("# ![{title} icon]({uri}) {}", &words["# ".len()..])
                    }
                    _ => words.to_owned(),
                };
                format!("[Contents](help:contents)\n\n{words}\n")
            }
            id => {
                let (page, name) = match self.tool_model().kind_of(id) {
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
                md
            }
        }
    }

    /// Where the help shows tool `id`'s button icon, `edge` points across: the same picture
    /// as its button, made small. `None` for a tool with no icon.
    pub(super) fn help_icon_uri(&self, id: &str, edge: u16) -> Option<String> {
        let key = self.help_icon_key(id)?;
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
            .map(form::setting_text)
            .or_else(own);
        let fields = form::fields(&schema)
            .into_iter()
            .filter_map(|f| Some((f.label, f.help?)))
            .collect();
        (about, fields)
    }
}

/// The pages of `index` (topic, title, Markdown) that `ranked` finds for `query`, best
/// first, each with the first line saying one of the question's words (or what means the
/// same) as plain words.
pub(super) fn search(
    index: &[(&'static str, String, String)],
    ranked: &cut_craft_help::search::Index,
    query: &str,
) -> Vec<(&'static str, String, String)> {
    use cut_craft_help::search::{SYNONYMS, expand, synonym_groups, words};
    let terms = expand(&words(query), &synonym_groups(SYNONYMS));
    // a word's start finds its other endings in a line: "cutting" shows a line saying "cut"
    let starts: Vec<String> = terms
        .iter()
        .map(|t| t.chars().take(t.chars().count().clamp(3, 5)).collect())
        .collect();
    ranked
        .search(query)
        .into_iter()
        .filter_map(|id| index.iter().find(|(t, ..)| *t == id))
        .map(|(topic, title, md)| {
            let line = md
                .lines()
                .filter(|l| !l.starts_with('[') && !l.starts_with('#'))
                .filter(|l| !l.trim_start().starts_with("!["))
                .find(|l| {
                    let l = l.to_lowercase();
                    starts.iter().any(|w| l.contains(w.as_str()))
                })
                .map_or_else(String::new, plain_line);
            (*topic, title.clone(), line)
        })
        .collect()
}

/// The page, and its side bar down its right: two columns, each scrolling on its own. Gives
/// the side bar's page clicked.
fn page_and_side_bar(
    ui: &mut egui::Ui,
    cache: &mut egui_commonmark::CommonMarkCache,
    topic: &str,
    markdown: &str,
    bar: &SidebarLists,
) -> Option<&'static str> {
    let gap = ui.spacing().item_spacing.x * 2.0;
    let side = if bar.is_empty() {
        0.0
    } else {
        SIDEBAR_WIDTH.min(ui.available_width() * 0.35)
    };
    let main = ui.available_width() - side - if side > 0.0 { gap } else { 0.0 };
    let height = ui.available_height();
    let column = egui::Layout::top_down(egui::Align::Min);
    let mut picked = None;
    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(egui::vec2(main, height), column, |ui| {
            ui.set_width(main);
            egui::ScrollArea::vertical().id_salt(topic).show(ui, |ui| {
                // pictures no wider than a step picture is taken, so they stay sharp, nor
                // than the column, so the side bar stays clear
                let fits = (main - ui.spacing().scroll.bar_width * 2.0).max(1.0);
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "pixels"
                )]
                let width = HELP_PICTURE_WIDTH.min(fits.round() as usize);
                CommonMarkViewer::new()
                    .max_image_width(Some(width))
                    .show(ui, cache, markdown);
            });
        });
        if side > 0.0 {
            ui.separator();
            ui.allocate_ui_with_layout(egui::vec2(side, height), column, |ui| {
                ui.set_width(side);
                egui::ScrollArea::vertical()
                    .id_salt(("help-side-bar", topic))
                    .show(ui, |ui| picked = sidebar_ui(ui, bar));
            });
        }
    });
    picked
}

/// The row along the Help window's top: Back, Forward, the search box, Contents and Did You
/// Know?. Gives the way Back or Forward was asked for (by the buttons, a mouse's own back and
/// forward buttons, or Alt+Left and Alt+Right, as in a web browser) and the page asked for.
fn top_bar(
    ui: &mut egui::Ui,
    search: &mut String,
    can_back: bool,
    can_forward: bool,
) -> (Option<Step>, Option<&'static str>) {
    let (mut step, mut picked) = (None, None);
    let (mouse_back, mouse_forward) = ui.input(|i| {
        (
            i.pointer.button_pressed(egui::PointerButton::Extra1)
                || i.modifiers.alt && i.key_pressed(egui::Key::ArrowLeft),
            i.pointer.button_pressed(egui::PointerButton::Extra2)
                || i.modifiers.alt && i.key_pressed(egui::Key::ArrowRight),
        )
    });
    ui.horizontal(|ui| {
        let back = ui
            .add_enabled(can_back, egui::Button::new(tr!("help-back")))
            .on_hover_text(tr!("help-the-page-you-were-on"));
        if (back.clicked() || mouse_back) && can_back {
            step = Some(Step::Back);
        }
        let forward = ui
            .add_enabled(can_forward, egui::Button::new(tr!("help-forward")))
            .on_hover_text(tr!("help-the-page-you-came-back"));
        if (forward.clicked() || mouse_forward) && can_forward {
            step = Some(Step::Forward);
        }
        ui.add(
            egui::TextEdit::singleline(search)
                .hint_text(tr!("help-ask-a-question-or-type"))
                .desired_width(ui.available_width() * 0.55),
        );
        if !search.is_empty() && ui.button(tr!("help-clear")).clicked() {
            search.clear();
        }
        if ui.button(tr!("help-contents")).clicked() {
            picked = Some("contents");
        }
        if ui.button(tr!("help-did-you-know")).clicked() {
            picked = Some("tips");
        }
    });
    (step, picked)
}

/// The side bar's lists, each under its heading: a tool's icon beside its name. Gives the
/// page clicked.
fn sidebar_ui(ui: &mut egui::Ui, lists: &SidebarLists) -> Option<&'static str> {
    // a title wider than the bar wraps: one that ran on would widen the window, and with it
    // the page beside, a little more every frame
    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
    let mut picked = None;
    for (heading, rows) in lists {
        ui.label(egui::RichText::new(*heading).strong());
        for (t, title, icon) in rows {
            let edge = egui::vec2(SIDEBAR_ICON, SIDEBAR_ICON);
            let link = match icon {
                Some(icon) => ui.add(
                    egui::Button::image_and_text(icon.clone().fit_to_exact_size(edge), title)
                        .frame(false),
                ),
                None => {
                    ui.horizontal(|ui| {
                        ui.add_space(SIDEBAR_ICON + ui.spacing().button_padding.x);
                        ui.link(title)
                    })
                    .inner
                }
            };
            if link
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                picked = Some(*t);
            }
        }
        ui.add_space(ui.spacing().item_spacing.y * 3.0);
    }
    picked
}

/// The search results, best first: each page's icon, its title as a link, and the line that
/// matched. Gives the page clicked.
fn results_ui(
    ui: &mut egui::Ui,
    found: &[(&'static str, String, String)],
    icons: &[Option<String>],
) -> Option<&'static str> {
    let mut picked = None;
    ui.weak(tr!("help-pages-found", count = found.len()));
    let edge = egui::Vec2::splat(f32::from(LIST_ICON));
    for ((t, title, line), icon) in found.iter().zip(icons) {
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
    picked
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

/// A help page's name from its first `# ` heading, else its id.
fn concept_title(id: &str) -> String {
    written(id)
        .lines()
        .find_map(|l| l.strip_prefix("# "))
        .map_or_else(|| id.to_owned(), |t| t.trim().to_owned())
}

/// Hands the help's pictures to the window's picture loader, once.
pub(super) fn load_pictures(ctx: &egui::Context) {
    let done = egui::Id::unique("help-pictures-loaded");
    if ctx.data(|d| d.get_temp::<bool>(done).unwrap_or(false)) {
        return;
    }
    for (path, bytes) in cut_craft_help::PICTURES {
        ctx.include_bytes(cut_craft_help::picture_uri(path), *bytes);
    }
    ctx.data_mut(|d| d.insert_temp(done, true));
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
    use cut_craft_help::search::{Doc, Index};

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

    fn ranked(index: &[(&'static str, String, String)]) -> Index {
        let docs: Vec<Doc> = index
            .iter()
            .map(|(t, title, md)| Doc {
                id: (*t).into(),
                title: title.clone(),
                text: md.clone(),
            })
            .collect();
        Index::new(&docs)
    }

    #[test]
    fn a_question_finds_its_pages_best_first() {
        let index = index();
        let found: Vec<_> = search(&index, &ranked(&index), "how do I add bridges?")
            .into_iter()
            .map(|(t, ..)| t)
            .collect();
        assert_eq!(found.first(), Some(&"tabs"), "{found:?}");
        assert_eq!(search(&index, &ranked(&index), "spoilboard").len(), 0);
        assert_eq!(search(&index, &ranked(&index), "   ").len(), 0);
    }

    #[test]
    fn a_result_shows_its_matching_line_as_plain_words() {
        let index = index();
        let found = search(&index, &ranked(&index), "hold");
        assert_eq!(found[0].2, "Tabs: hold the part in place");
        assert_eq!(
            plain_line("- [Design](help:design): the page down the left"),
            "Design: the page down the left"
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
}
