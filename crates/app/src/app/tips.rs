//! Did You Know?: short tips for someone new to CAD/CAM, one shown when Cut Craft starts
//! (Edit ▸ Options ▸ General turns that off) and all of them from Help ▸ Did You Know? and
//! the Help window's own page of them. Cut Craft's own (see `re/SPEC.md`, Did You Know?).

use super::{CutCraftApp, Info, SPLASH_LOGO, egui};
use cut_craft_i18n::tr;
use std::fmt::Write as _;
mod list;
pub(super) use list::*;

/// What a tip is about: the Help window's page of tips lists them under these, in this order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Area {
    Start,
    Drawing,
    Modelling,
    Toolpaths,
    Cutting,
    Program,
    Safety,
}

impl Area {
    /// Every subject, in the order the page lists them.
    pub const ALL: [Self; 7] = [
        Self::Start,
        Self::Drawing,
        Self::Modelling,
        Self::Toolpaths,
        Self::Cutting,
        Self::Program,
        Self::Safety,
    ];

    /// The page's heading for this subject.
    pub fn heading(self) -> &'static str {
        match self {
            Self::Start => "Getting started",
            Self::Drawing => "Drawing and editing",
            Self::Modelling => "3D models",
            Self::Toolpaths => "Toolpaths",
            Self::Cutting => "Preview, save and send",
            Self::Program => "Settings, help and updates",
            Self::Safety => "Cutting safely",
        }
    }
}

/// The picture above a tip's words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Picture {
    /// A scene drawn for this tip alone, by `scripts/make_icons.py` into `assets/icons/tips`.
    Scene(&'static str),
    /// A tool's button icon (by the tool's id), or another of Cut Craft's icons (by its path),
    /// so the tool can be found by it.
    Icon(&'static str),
}

/// A tip: its picture, what it says (Markdown), what it is about and the help page that says
/// more (Tell Me More…).
#[derive(Clone, Copy, Debug)]
pub(super) struct Tip {
    pub area: Area,
    pub picture: Picture,
    pub text: &'static str,
    pub topic: &'static str,
}

/// A tip with a scene of its own.
const fn scene(area: Area, picture: &'static str, text: &'static str, topic: &'static str) -> Tip {
    Tip {
        area,
        picture: Picture::Scene(picture),
        text,
        topic,
    }
}

/// A tip about one tool, showing its icon and opening its help.
const fn tool(area: Area, id: &'static str, text: &'static str) -> Tip {
    Tip {
        area,
        picture: Picture::Icon(id),
        text,
        topic: id,
    }
}

/// A tip showing `icon` (a tool's id or an icon's path), opening help page `topic`.
const fn icon(area: Area, icon: &'static str, text: &'static str, topic: &'static str) -> Tip {
    Tip {
        area,
        picture: Picture::Icon(icon),
        text,
        topic,
    }
}

/// The tip after `n`, starting over after the last.
pub(super) fn next(n: usize) -> usize {
    (n + 1) % TIPS.len()
}

/// The tip before `n`, the last before the first.
pub(super) fn previous(n: usize) -> usize {
    (n + TIPS.len() - 1) % TIPS.len()
}

/// The Help window's page of every tip, by subject, each linked to its help page.
pub(super) fn markdown() -> String {
    let mut md = String::from(
        "[Contents](help:contents)\n\n# Did You Know?\n\nShort tips for getting more out of \
         Cut Craft. One is shown each time Cut Craft starts; turn that off in the tip's window \
         or in **Edit ▸ Options ▸ General**. The tip's window stays in front of Cut Craft \
         until you close it (see [Windows that open beside Cut Craft](help:contents)). Each \
         tip's **Tell me more** opens the help page about it.\n",
    );
    for area in Area::ALL {
        let _ = write!(md, "\n## {}\n\n", area.heading());
        for t in TIPS.iter().filter(|t| t.area == area) {
            let _ = writeln!(md, "- {} [Tell me more](help:{})", t.text, t.topic);
        }
    }
    md
}

/// What the tips window was closed or moved on with.
enum Press {
    Previous,
    Next,
    More(&'static str),
    Close,
}

impl CutCraftApp {
    /// The icon `id` stands for: a tool's button icon, else the icon at that path.
    pub(super) fn tip_icon(&self, id: &str) -> Option<&str> {
        self.icons
            .find(self.explicit_icon(id), id)
            .or_else(|| self.icons.find(id, ""))
    }

    /// The picture `tip` shows, if it is drawn.
    pub(super) fn tip_picture(&self, tip: &Tip) -> Option<&str> {
        match tip.picture {
            Picture::Scene(path) => self.icons.find(path, ""),
            Picture::Icon(id) => self.tip_icon(id),
        }
    }

    /// Help ▸ Did You Know?, or startup: the tips window on `n`.
    pub(super) fn open_tips(&mut self, n: usize) {
        self.tip = Some(n % TIPS.len());
    }

    /// At startup, unless turned off: the next tip, and the one after it next time.
    pub(super) fn tip_at_startup(&mut self) {
        if !self.options.general.show_tips_at_startup || self.first_run.is_some() {
            return;
        }
        let n = self.options.next_tip % TIPS.len();
        self.open_tips(n);
        self.options.next_tip = next(n);
        self.save_options();
    }

    /// First-time setup, then the tip, while open.
    pub(super) fn welcome_windows(&mut self, ctx: &egui::Context) {
        self.first_run_window(ctx);
        self.tips_window(ctx);
    }

    /// The tips window, while it is open.
    pub(super) fn tips_window(&mut self, ctx: &egui::Context) {
        let Some(n) = self.tip else { return };
        let mut open = true;
        let mut show = self.options.general.show_tips_at_startup;
        let mut press = None;
        let size = egui::vec2(28.0, 20.0) * ctx.global_style().spacing.interact_size.y;
        let tip = TIPS[n];
        let key = self.tip_picture(&tip).map(str::to_owned);
        let scene = matches!(tip.picture, Picture::Scene(_));
        let (cache, icons) = (&mut self.tips_cache, &self.icons);
        let id = egui::Id::unique("did_you_know");
        cut_craft_bug_report::dialog(ctx, id, "Did You Know?", size, &mut open, |ui| {
            ui.weak(tr!("tips-tip-of", number = n + 1, count = TIPS.len()));
            // the picture across the top, in a 2:1 space, then the words under it: a scene
            // fills the space; an icon is shown no bigger than it is drawn, in its middle
            let wide = ui
                .available_width()
                .min(22.0 * ui.spacing().interact_size.y);
            let picture = match &key {
                Some(key) => icons.image(key),
                None => egui::Image::from_bytes("bytes://splash-logo.jpg", SPLASH_LOGO),
            };
            let high = if scene || key.is_none() {
                wide / 2.0
            } else {
                (wide / 2.0).min(7.0 * ui.spacing().interact_size.y)
            };
            let gap = (wide / 2.0 - high) / 2.0;
            ui.add_space(gap);
            ui.vertical_centered(|ui| {
                ui.add(picture.fit_to_exact_size(egui::vec2(wide, high)));
            });
            ui.add_space(gap);
            ui.add_space(ui.spacing().item_spacing.y);
            egui_commonmark::CommonMarkViewer::new().show(ui, cache, tip.text);
            ui.add_space(ui.spacing().item_spacing.y);
            ui.horizontal(|ui| {
                if ui.button(tr!("tips-previous")).clicked() {
                    press = Some(Press::Previous);
                }
                if ui.button(tr!("tips-next")).clicked() {
                    press = Some(Press::Next);
                }
                if ui.button(tr!("tips-tell-me-more")).clicked() {
                    press = Some(Press::More(tip.topic));
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.checkbox(&mut show, tr!("tips-show-a-tip-when-cut"));
                if ui.button(tr!("tips-close")).clicked() {
                    press = Some(Press::Close);
                }
            });
        });
        if show != self.options.general.show_tips_at_startup {
            self.options.general.show_tips_at_startup = show;
            self.save_options();
        }
        self.tip = match press {
            _ if !open => None,
            Some(Press::Previous) => Some(previous(n)),
            Some(Press::Next) => Some(next(n)),
            Some(Press::More(topic)) => {
                self.info = Some(Info::Help(topic));
                None
            }
            Some(Press::Close) => None,
            None => Some(n),
        };
    }
}
