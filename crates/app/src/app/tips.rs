//! Did You Know?: short tips for someone new to CAD/CAM, one shown when Cut Craft starts
//! (Edit ▸ Options ▸ General turns that off) and all of them from Help ▸ Did You Know? and
//! the Help window's own page of them. Cut Craft's own (see `re/SPEC.md`, Did You Know?).

use super::{CutCraftApp, Info, SPLASH_LOGO, egui};
use crate::options::WindowLayout;
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
    /// A scene drawn for this tip alone, in `assets/icons/tips`.
    Scene(&'static str),
    /// A tool's button icon (by the tool's id), or another of Cut Craft's icons (by its path),
    /// so the tool can be found by it.
    Icon(&'static str),
}

/// A tip: its picture, what it says (Markdown), what it is about and the help page that says
/// more (Tell Me More…); how often it comes up at start beside the others (`weight`), and the
/// layout it offers to try, if any (not shown to someone already in that layout).
#[derive(Clone, Copy, Debug)]
pub(super) struct Tip {
    pub area: Area,
    pub picture: Picture,
    pub text: &'static str,
    pub topic: &'static str,
    pub weight: u32,
    pub tries: Option<WindowLayout>,
}

impl Tip {
    /// This tip, coming up `weight` times as often as a plain one at start.
    const fn weighted(self, weight: u32) -> Self {
        Self { weight, ..self }
    }

    /// This tip, with a button to try `layout`; not shown in that layout.
    const fn tries(self, layout: WindowLayout) -> Self {
        Self {
            tries: Some(layout),
            ..self
        }
    }

    /// What names this tip in the list of tips shown: its words, hashed the same on every
    /// computer and version, so a tip that is reworded counts as new.
    pub fn key(&self) -> u64 {
        // FNV-1a
        self.text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
        })
    }
}

/// Which of `weights` (`(tip, weight)`) a `roll` lands on, each as likely as its weight;
/// `None` when there are none.
pub(super) fn pick(weights: &[(usize, u32)], roll: u64) -> Option<usize> {
    let total: u64 = weights.iter().map(|&(_, w)| u64::from(w)).sum();
    if total == 0 {
        return None;
    }
    let mut at = roll % total;
    weights.iter().find_map(|&(tip, w)| {
        if at < u64::from(w) {
            Some(tip)
        } else {
            at -= u64::from(w);
            None
        }
    })
}

/// A roll for `pick`, different each start.
fn roll() -> u64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    // spread the low bits, which change fastest, across all of them
    #[expect(clippy::cast_possible_truncation)]
    let nanos = nanos as u64;
    nanos.wrapping_mul(0x9e37_79b9_7f4a_7c15).rotate_left(17)
}

/// A tip with a scene of its own.
const fn scene(area: Area, picture: &'static str, text: &'static str, topic: &'static str) -> Tip {
    Tip {
        area,
        picture: Picture::Scene(picture),
        text,
        topic,
        weight: 1,
        tries: None,
    }
}

/// A tip about one tool, showing its icon and opening its help.
const fn tool(area: Area, id: &'static str, text: &'static str) -> Tip {
    Tip {
        area,
        picture: Picture::Icon(id),
        text,
        topic: id,
        weight: 1,
        tries: None,
    }
}

/// A tip showing `icon` (a tool's id or an icon's path), opening help page `topic`.
const fn icon(area: Area, icon: &'static str, text: &'static str, topic: &'static str) -> Tip {
    Tip {
        area,
        picture: Picture::Icon(icon),
        text,
        topic,
        weight: 1,
        tries: None,
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

/// Whether `tip` is for someone in `layout`: one offering to try a layout is not for those
/// already in it.
pub(super) fn offered(tip: &Tip, layout: WindowLayout) -> bool {
    tip.tries != Some(layout)
}

/// The tip to show at start, for someone in `layout` who has seen the tips keyed in `seen`:
/// one not seen yet, picked by weight with `roll`. Once every tip has been seen, `seen`
/// is emptied and they all come round again.
pub(super) fn startup_tip(layout: WindowLayout, seen: &mut Vec<u64>, roll: u64) -> Option<usize> {
    let unseen = |seen: &[u64]| -> Vec<(usize, u32)> {
        TIPS.iter()
            .enumerate()
            .filter(|(_, t)| offered(t, layout) && !seen.contains(&t.key()))
            .map(|(n, t)| (n, t.weight))
            .collect()
    };
    let mut weights = unseen(seen);
    if weights.is_empty() {
        seen.clear();
        weights = unseen(seen);
    }
    pick(&weights, roll)
}

/// The Help window's page of every tip, by subject, each linked to its help page.
pub(super) fn markdown() -> String {
    let mut md = String::from(
        "[Contents](help:contents)\n\n# Did You Know?\n\nShort tips for getting more out of \
         Cut Craft. One is shown each time Cut Craft starts, one you have not seen yet (a few \
         come up more often than others), until you have seen them all and they come round \
         again; turn that off in the tip's window or in **Edit ▸ Options ▸ General**. A tip \
         counts as seen once it is on screen, from **Next** or **Previous** too. Which you \
         have seen is kept with your settings on this computer and goes nowhere else. The tip's window stays in front of Cut Craft \
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
    Try(WindowLayout),
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

    /// At startup, unless turned off: a tip not seen yet, picked by weight.
    pub(super) fn tip_at_startup(&mut self) {
        if !self.options.general.show_tips_at_startup || self.first_run.is_some() {
            return;
        }
        let layout = self.options.general.layout;
        if let Some(n) = startup_tip(layout, &mut self.options.seen_tips, roll()) {
            self.open_tips(n);
        }
    }

    /// Next or Previous from tip `n`: the next tip `step` gives that is for this layout.
    fn step_tips(&self, n: usize, step: fn(usize) -> usize) -> usize {
        let layout = self.options.general.layout;
        let mut at = step(n);
        while at != n && !offered(&TIPS[at], layout) {
            at = step(at);
        }
        at
    }

    /// Tip `n` was shown: remembered, so startup shows another until all have been seen.
    fn tip_seen(&mut self, n: usize) {
        let key = TIPS[n].key();
        if !self.options.seen_tips.contains(&key) {
            self.options.seen_tips.push(key);
            self.save_options();
        }
    }

    /// First-time setup, the layout question, then the tip, while open.
    pub(super) fn welcome_windows(&mut self, ctx: &egui::Context) {
        self.first_run_window(ctx);
        self.layout_window(ctx);
        self.tips_window(ctx);
    }

    /// The tips window, while it is open.
    pub(super) fn tips_window(&mut self, ctx: &egui::Context) {
        let Some(n) = self.tip else { return };
        self.tip_seen(n);
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
                if let Some(layout) = tip.tries {
                    let name = super::layout_choice::name(layout);
                    if ui.button(tr!("tips-try-layout", layout = name)).clicked() {
                        press = Some(Press::Try(layout));
                    }
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
            Some(Press::Previous) => Some(self.step_tips(n, previous)),
            Some(Press::Next) => Some(self.step_tips(n, next)),
            Some(Press::Try(layout)) => {
                self.options.general.layout = layout;
                self.save_options();
                None
            }
            Some(Press::More(topic)) => {
                self.info = Some(Info::Help(topic));
                None
            }
            Some(Press::Close) => None,
            None => Some(n),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_roll_lands_on_each_tip_as_often_as_its_weight() {
        let weights = [(7, 1), (8, 5), (9, 2)];
        let mut landed = [0; 3];
        for roll in 0..800 {
            let tip = pick(&weights, roll).unwrap();
            landed[tip - 7] += 1;
        }
        assert_eq!(landed, [100, 500, 200]);
        assert_eq!(pick(&[], 3), None);
    }

    #[test]
    fn a_tips_key_is_its_words() {
        let tip = scene(Area::Start, "tips/x.png", "Some words", "x");
        assert_eq!(tip.key(), tip.weighted(3).key());
        let reworded = scene(Area::Start, "tips/x.png", "Other words", "x");
        assert_ne!(tip.key(), reworded.key());
        // the same on every computer and version: FNV-1a of "a"
        assert_eq!(scene(Area::Start, "", "a", "").key(), 0xaf63_dc4c_8601_ec8c);
    }

    #[test]
    fn startup_skips_seen_tips_and_starts_over_once_all_are() {
        let mut seen: Vec<u64> = TIPS.iter().skip(1).map(Tip::key).collect();
        for roll in 0..20 {
            assert_eq!(startup_tip(WindowLayout::Ribbon, &mut seen, roll), Some(0));
        }
        seen.push(TIPS[0].key());
        assert!(startup_tip(WindowLayout::Ribbon, &mut seen, 1).is_some());
        assert!(seen.is_empty(), "all seen: round again");
    }
}
