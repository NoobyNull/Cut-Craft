//! Checks that every tool a beginner can reach has help worth reading, and lists the ones
//! that do not.
//!
//! This is its own test suite, run apart from the others (`cargo nextest run --profile help`,
//! and the Help coverage workflow): a gap does not stop a pull request unless it is new. The
//! gaps already known are listed in `crates/app/help/coverage-baseline.txt`; the workflow
//! opens or updates one GitHub issue listing every gap still open.
//!
//! What a tool must have (the minimum standard):
//! - **A tooltip** (its crate's first `//!` sentence, shown on hover and in the status bar)
//!   that says what the tool does in at least [`TOOLTIP_WORDS`] words, beyond its own name.
//! - **A help page intro** (the doc comment on its parameters, shown by F1 and Help) of at
//!   least [`ABOUT_WORDS`] words.
//! - **Help on every setting** of its form (each parameter's doc comment, shown on hover and
//!   on the help page), at least [`SETTING_WORDS`] words.
//! - **Help on every option** of a choice (each variant's doc comment), so a beginner can tell
//!   Outside from Inside without trying both.
//! - No placeholder text ("TODO", "No help written…").
//!
//! Menu items that run no tool need only a tooltip.

use super::CutCraftApp;
use crate::form::{self, Field, Kind};
use std::collections::BTreeSet;
use std::fmt::Write as _;

/// Fewest words a tooltip needs after the tool's own name.
const TOOLTIP_WORDS: usize = 4;
/// Fewest words a help page's intro needs.
const ABOUT_WORDS: usize = 8;
/// Fewest words a setting's or an option's help needs.
const SETTING_WORDS: usize = 3;
/// Text that is not help yet.
const PLACEHOLDERS: &[&str] = &[
    "todo",
    "tbd",
    "fixme",
    "xxx",
    "lorem",
    "placeholder",
    "no help written",
    "not documented",
];

/// The gaps already known when the suite started, one [`Gap::key`] a line.
const BASELINE: &str = include_str!("../../help/coverage-baseline.txt");

/// One thing a tool's help is missing.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Gap {
    /// The tool's (or menu action's) id.
    pub id: String,
    /// Its name as the person sees it.
    pub label: String,
    pub what: What,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum What {
    NoTooltip,
    ShortTooltip,
    NoIntro,
    ShortIntro,
    /// A setting, by its label path (`Tabs ▸ Length`).
    NoSettingHelp(String),
    /// A choice's option: the setting, then the option's label.
    NoOptionHelp(String, String),
    /// Where the placeholder is: `tooltip`, `intro`, or a setting's label.
    Placeholder(String),
    /// The same tooltip as another tool (a crate holding several commands says one thing
    /// for all of them): by the other tool's id.
    SharedTooltip(String),
}

impl Gap {
    /// The line naming this gap in the baseline: stable while wording elsewhere changes.
    pub fn key(&self) -> String {
        let what = match &self.what {
            What::NoTooltip => "no tooltip".to_owned(),
            What::ShortTooltip => "short tooltip".to_owned(),
            What::NoIntro => "no help page intro".to_owned(),
            What::ShortIntro => "short help page intro".to_owned(),
            What::NoSettingHelp(s) => format!("setting without help: {s}"),
            What::NoOptionHelp(s, o) => format!("option without help: {s} = {o}"),
            What::Placeholder(at) => format!("placeholder text in {at}"),
            What::SharedTooltip(other) => format!("same tooltip as {other}"),
        };
        format!("{} | {what}", self.id)
    }
}

fn words(text: &str) -> usize {
    text.split_whitespace()
        .filter(|w| w.chars().any(char::is_alphanumeric))
        .count()
}

fn is_placeholder(text: &str) -> bool {
    let lower = text.to_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let spaced = words.join(" ");
    PLACEHOLDERS.iter().any(|p| {
        if p.contains(' ') {
            format!(" {spaced} ").contains(&format!(" {p} "))
        } else {
            words.contains(p)
        }
    })
}

/// A tooltip's words after its `Name:` lead, which only repeats the button's label.
fn tooltip_body<'a>(tooltip: &'a str, label: &str) -> &'a str {
    tooltip.strip_prefix(label).map_or(tooltip, |rest| {
        rest.trim_start_matches([':', ' ', '-', '–'])
    })
}

/// The gaps in one tooltip.
fn tooltip_gaps(id: &str, label: &str, tooltip: Option<&str>) -> Vec<Gap> {
    let gap = |what| Gap {
        id: id.to_owned(),
        label: label.to_owned(),
        what,
    };
    match tooltip.map(str::trim).filter(|t| !t.is_empty()) {
        None => vec![gap(What::NoTooltip)],
        Some(t) if is_placeholder(t) => vec![gap(What::Placeholder("tooltip".into()))],
        Some(t) if words(tooltip_body(t, label)) < TOOLTIP_WORDS => vec![gap(What::ShortTooltip)],
        Some(_) => vec![],
    }
}

/// The gaps in a tool's help page: its intro, then each setting and option of its form
/// (`schema`, when it has one), nested ones too.
fn page_gaps(
    id: &str,
    label: &str,
    intro: Option<&str>,
    schema: Option<&serde_json::Value>,
) -> Vec<Gap> {
    let gap = |what| Gap {
        id: id.to_owned(),
        label: label.to_owned(),
        what,
    };
    let mut gaps = vec![];
    match intro.map(str::trim) {
        None | Some("") => gaps.push(gap(What::NoIntro)),
        Some(t) if is_placeholder(t) => gaps.push(gap(What::Placeholder("intro".into()))),
        Some(t) if words(t) < ABOUT_WORDS => gaps.push(gap(What::ShortIntro)),
        Some(_) => {}
    }
    let mut whats = vec![];
    if let Some(schema) = schema {
        setting_gaps(&form::fields(schema), "", &mut whats);
    }
    gaps.extend(whats.into_iter().map(gap));
    gaps
}

/// The settings (and their options) among `fields`, under `path`, that have no help.
fn setting_gaps(fields: &[Field], path: &str, out: &mut Vec<What>) {
    for f in fields {
        let at = if path.is_empty() {
            f.label.clone()
        } else {
            format!("{path} ▸ {}", f.label)
        };
        match f.help.as_deref().map(str::trim) {
            None | Some("") => out.push(What::NoSettingHelp(at.clone())),
            Some(t) if is_placeholder(t) => out.push(What::Placeholder(at.clone())),
            Some(t) if words(t) < SETTING_WORDS => out.push(What::NoSettingHelp(at.clone())),
            Some(_) => {}
        }
        let mut kind = &f.kind;
        while let Kind::List(inner) = kind {
            kind = inner;
        }
        // a tool's own settings are the Tool Database's, drawn by the tool picker
        let picked = f.key == "tool" || form::TOOL_LISTS.contains(&f.key.as_str());
        match kind {
            Kind::Group(_) if picked => {}
            Kind::Group(inner) => setting_gaps(inner, &at, out),
            // the fonts installed on this computer: names, different on each machine
            Kind::Choice(..) if f.key == "font" => {}
            Kind::Choice(options, _) if options.len() > 1 => {
                for o in options {
                    let help = o.help.as_deref().map_or_default(str::trim);
                    if words(help) < SETTING_WORDS || is_placeholder(help) {
                        out.push(What::NoOptionHelp(at.clone(), o.label.clone()));
                    }
                }
            }
            _ => {}
        }
    }
}

/// Every action on the app's menus, with its name: `File ▸ Save`.
fn menu_actions(app: &CutCraftApp) -> Vec<(String, String)> {
    fn walk(items: &[crate::gui::MenuItem], out: &mut Vec<(String, String)>) {
        for item in items {
            if item.children.is_empty() {
                if !item.name.is_empty() && !item.action.is_empty() {
                    out.push((item.action.clone(), item.name.clone()));
                }
            } else {
                walk(&item.children, out);
            }
        }
    }
    let mut out = vec![];
    for menu in &app.menus.menus {
        walk(&menu.items, &mut out);
    }
    out
}

/// Every gap in the app's help: each tool on the Design and Toolpaths pages, each registered
/// command, then each menu item.
pub(super) fn gaps(app: &CutCraftApp) -> Vec<Gap> {
    let mut seen = BTreeSet::new();
    let mut gaps = vec![];
    let tools = app
        .help_topics()
        .into_iter()
        .filter(|t| !matches!(*t, "contents" | "design" | "toolpaths" | "tips"))
        .map(String::from)
        .chain(app.registry.iter().map(|c| c.id().to_owned()));
    for id in tools {
        if !seen.insert(id.clone()) {
            continue;
        }
        let label = app.tool_label(&id);
        gaps.extend(tooltip_gaps(&id, &label, crate::gui::about(&id)));
        // a built-in tool with no form still needs a page saying what it does
        let (intro, _) = app.tool_help(&id);
        let schema = app
            .registry
            .get(&id)
            .map(cut_craft_core::DynCommand::schema);
        gaps.extend(page_gaps(&id, &label, intro.as_deref(), schema.as_ref()));
    }
    for (action, name) in menu_actions(app) {
        if seen.insert(action.clone()) {
            gaps.extend(tooltip_gaps(&action, &name, crate::gui::about(&action)));
        }
    }
    let tooltips = seen
        .iter()
        .filter_map(|id| Some((id.as_str(), crate::gui::about(id)?)));
    for (id, other) in shared_tooltips(tooltips) {
        gaps.push(Gap {
            id: id.to_owned(),
            label: app.tool_label(id),
            what: What::SharedTooltip(other.to_owned()),
        });
    }
    gaps.sort();
    gaps
}

/// Each tool whose tooltip an earlier one (by id) already has, with that one's id.
fn shared_tooltips<'a>(
    tooltips: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Vec<(&'a str, &'a str)> {
    let mut first: std::collections::BTreeMap<&str, &str> = std::collections::BTreeMap::new();
    let mut shared = vec![];
    for (id, tooltip) in tooltips {
        match first.get(tooltip.trim()) {
            Some(other) => shared.push((id, *other)),
            None => {
                first.insert(tooltip.trim(), id);
            }
        }
    }
    shared
}

/// The gaps as Markdown, grouped by tool: the body of the GitHub issue.
pub(super) fn report(gaps: &[Gap]) -> String {
    let tools: BTreeSet<&str> = gaps.iter().map(|g| g.id.as_str()).collect();
    let mut md = format!(
        "{} help gaps in {} tools and menu items.\n\n\
         Each tool needs: a tooltip saying what it does ({TOOLTIP_WORDS}+ words after its \
         name), a help page intro ({ABOUT_WORDS}+ words), and help on every setting and every \
         option of a choice ({SETTING_WORDS}+ words). Menu items that run no tool need a \
         tooltip only.\n",
        gaps.len(),
        tools.len()
    );
    for id in tools {
        let of: Vec<&Gap> = gaps.iter().filter(|g| g.id == id).collect();
        let _ = write!(md, "\n### {} (`{id}`)\n\n", of[0].label);
        for g in of {
            let line = match &g.what {
                What::NoTooltip => "no tooltip".to_owned(),
                What::ShortTooltip => "tooltip too short to say what it does".to_owned(),
                What::NoIntro => "no help page intro".to_owned(),
                What::ShortIntro => "help page intro too short".to_owned(),
                What::NoSettingHelp(s) => format!("**{s}** has no help"),
                What::NoOptionHelp(s, o) => format!("**{s}**: option **{o}** has no help"),
                What::Placeholder(at) => format!("placeholder text in {at}"),
                What::SharedTooltip(other) => {
                    format!("tooltip is the same as `{other}`'s: say what this one does")
                }
            };
            let _ = writeln!(md, "- [ ] {line}");
        }
    }
    md
}

/// The baseline's gap keys, comments and blank lines left out.
fn baseline() -> BTreeSet<&'static str> {
    BASELINE
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect()
}

mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_missing_or_short_tooltip_is_a_gap() {
        assert_eq!(
            tooltip_gaps("x", "X", None)[0].what,
            What::NoTooltip,
            "none at all"
        );
        assert_eq!(tooltip_gaps("x", "X", Some("  "))[0].what, What::NoTooltip);
        assert_eq!(
            tooltip_gaps("draw_circle", "Draw Circle", Some("Draw Circle: a circle"))[0].what,
            What::ShortTooltip,
            "the name, then two words"
        );
        assert_eq!(
            tooltip_gaps(
                "draw_circle",
                "Draw Circle",
                Some("Draw Circle: drag from the centre out to its edge")
            ),
            vec![]
        );
        assert_eq!(
            tooltip_gaps("x", "X", Some("TODO: write what this does"))[0].what,
            What::Placeholder("tooltip".into())
        );
    }

    #[test]
    fn two_tools_with_one_tooltip_is_a_gap() {
        let tooltips = [
            ("align_left", "Align: lines the selection up"),
            ("align_right", "Align: lines the selection up"),
            ("rotate", "Rotate: turns the selection about a point"),
        ];
        assert_eq!(
            shared_tooltips(tooltips),
            vec![("align_right", "align_left")]
        );
    }

    #[test]
    fn placeholders_are_whole_words() {
        assert!(is_placeholder("TODO"));
        assert!(is_placeholder("No help written for this tool yet."));
        assert!(is_placeholder("Width (tbd)"));
        assert!(!is_placeholder(
            "Cuts round the outside, like a jigsaw would do"
        ));
        assert!(
            !is_placeholder("The toddler's step stool"),
            "todo in a word"
        );
    }

    #[test]
    fn a_form_needs_an_intro_and_help_on_every_setting_and_option() {
        let schema = json!({
            "type": "object",
            "properties": {
                "depth": { "type": "number", "description": "How deep the bit goes in" },
                "width": { "type": "number" },
                "side": {
                    "description": "Which side of the line the bit cuts",
                    "oneOf": [
                        { "const": "outside", "description": "Cut round the outside of the shape" },
                        { "const": "inside" }
                    ]
                },
                "font": {
                    "type": "string",
                    "description": "The lettering the text is drawn in",
                    "enum": ["Lato", "DejaVu Sans"]
                },
                "tool": {
                    "type": "object",
                    "description": "The bit that cuts it",
                    "properties": { "diameter": { "type": "number" } }
                },
                "tabs": {
                    "type": "object",
                    "description": "Small bridges that hold the part",
                    "properties": { "length": { "type": "number" } }
                }
            }
        });
        let intro = schema["description"].as_str();
        let whats: Vec<What> = page_gaps("profile", "Profile", intro, Some(&schema))
            .into_iter()
            .map(|g| g.what)
            .collect();
        assert_eq!(
            whats,
            vec![
                What::NoIntro,
                What::NoSettingHelp("Width".into()),
                What::NoOptionHelp("Side".into(), "Inside".into()),
                What::NoSettingHelp("Tabs ▸ Length".into()),
            ]
        );
        let described = json!({
            "type": "object",
            "description": "Cuts along a vector, round its outside, inside or on the line",
            "properties": {}
        });
        let intro = described["description"].as_str();
        assert_eq!(
            page_gaps("profile", "Profile", intro, Some(&described)),
            vec![]
        );
        assert_eq!(
            page_gaps("p", "P", Some("Cuts things"), None)[0].what,
            What::ShortIntro
        );
    }

    #[test]
    fn the_report_lists_each_tools_gaps_under_its_name() {
        let gap = |what| Gap {
            id: "profile_toolpath".into(),
            label: "Profile Toolpath".into(),
            what,
        };
        let md = report(&[gap(What::NoIntro), gap(What::NoSettingHelp("Width".into()))]);
        assert!(md.starts_with("2 help gaps in 1 tools"), "{md}");
        assert!(
            md.contains("### Profile Toolpath (`profile_toolpath`)"),
            "{md}"
        );
        assert!(md.contains("- [ ] **Width** has no help"), "{md}");
    }

    #[test]
    fn a_gaps_key_names_the_tool_and_what_is_missing() {
        let g = Gap {
            id: "pocket_toolpath".into(),
            label: "Pocket Toolpath".into(),
            what: What::NoOptionHelp("Direction".into(), "Climb".into()),
        };
        assert_eq!(
            g.key(),
            "pocket_toolpath | option without help: Direction = Climb"
        );
    }
}

/// The coverage check itself, kept out of the ordinary test run (see `.config/nextest.toml`):
/// it fails only on gaps not in the baseline, and writes the full list for the issue.
mod coverage {
    use super::*;

    #[test]
    fn every_tool_has_help() {
        let app = CutCraftApp::new();
        let gaps = gaps(&app);
        let md = report(&gaps);
        if let Some(path) = std::env::var_os("HELP_COVERAGE_REPORT") {
            std::fs::write(&path, &md).expect("the report is written");
        }
        if let Some(path) = std::env::var_os("HELP_COVERAGE_BASELINE_OUT") {
            let mut keys: Vec<String> = gaps.iter().map(Gap::key).collect();
            keys.dedup();
            let text = format!(
                "# Help gaps known when this list was last written: one `id | gap` a line.\n\
                 # The help coverage check fails on any gap not listed here. Fix a gap, then \
                 delete its line.\n{}\n",
                keys.join("\n")
            );
            std::fs::write(&path, text).expect("the baseline is written");
        }
        let known = baseline();
        let new: Vec<String> = gaps
            .iter()
            .map(Gap::key)
            .filter(|k| !known.contains(k.as_str()))
            .collect();
        let fixed: Vec<&str> = {
            let now: BTreeSet<String> = gaps.iter().map(Gap::key).collect();
            known
                .iter()
                .copied()
                .filter(|k| !now.contains(*k))
                .collect()
        };
        if !fixed.is_empty() {
            println!(
                "{} gaps in crates/app/help/coverage-baseline.txt are fixed; delete their lines:",
                fixed.len()
            );
            for k in &fixed {
                println!("  {k}");
            }
        }
        assert!(
            new.is_empty(),
            "{} new help gaps. Write the missing help (the crate's first `//!` sentence for \
             the tooltip, `///` doc comments on the params, each setting and each option), \
             then run the check again:\n  {}",
            new.len(),
            new.join("\n  ")
        );
    }
}
