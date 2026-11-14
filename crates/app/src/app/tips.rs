//! Did You Know?: short tips for someone new to CAD/CAM, one shown when Cut Craft starts
//! (Edit ▸ Options ▸ General turns that off) and all of them from Help ▸ Did You Know? and
//! the Help window's own page of them. Cut Craft's own (see `re/SPEC.md`, Did You Know?).

use super::{CutCraftApp, Info, SPLASH_LOGO, egui};

/// A tip: its illustration (drawn by `scripts/make_icons.py` into `assets/icons/tips`), what
/// it says (Markdown), and the help page that says more, if one does.
#[derive(Clone, Copy, Debug)]
pub(super) struct Tip {
    pub picture: &'static str,
    pub text: &'static str,
    pub topic: Option<&'static str>,
}

const fn tip(picture: &'static str, text: &'static str, topic: &'static str) -> Tip {
    Tip {
        picture,
        text,
        topic: Some(topic),
    }
}

const fn plain(picture: &'static str, text: &'static str) -> Tip {
    Tip {
        picture,
        text,
        topic: None,
    }
}

/// Every tip, in the order startup shows them: the first few are what a beginner needs on day
/// one, the rest make the work quicker.
pub(super) const TIPS: &[Tip] = &[
    plain(
        "tips/sample_project.png",
        "New to Cut Craft? On the start page, **Try a sample project** opens a finished sign \
         with its toolpaths already made, so you can see the whole journey from drawing to \
         cutting file before you start your own.",
    ),
    tip(
        "tips/f1_help.png",
        "Press **F1** with a tool's settings open to read the help for that tool. With nothing \
         open, F1 opens Help Contents, where you can search every tool.",
        "contents",
    ),
    plain(
        "tips/hover_note.png",
        "Not sure what a setting means? Hover the pointer over its name for a short note about \
         it.",
    ),
    tip(
        "tips/tool_list.png",
        "The **?** at the top of the Design and Toolpaths pages lists every tool on that page, \
         each with a page of help.",
        "design",
    ),
    tip(
        "tips/design_then_toolpaths.png",
        "Cut Craft works in two steps. **Design** (down the left) is where you draw what you \
         want. **Toolpaths** (down the right) is where you tell the machine how to cut it.",
        "contents",
    ),
    tip(
        "tips/preview_cut.png",
        "Always **Preview** a toolpath before you cut. It shows the bit carving the material on \
         screen, so a mistake costs nothing but a click.",
        "toolpaths",
    ),
    plain(
        "tips/closed_shape.png",
        "Most toolpaths need a **closed** shape: one whose line meets itself all the way round. \
         **View ▸ Show Open Vector Ends** marks the gaps, and **Edit ▸ Join Open Ends** closes \
         them.",
    ),
    plain(
        "tips/tool_database.png",
        "Picking a bit from the **Tool Database** fills in its speeds, feeds and depth per pass \
         for you. Set them once for each bit and every toolpath uses them.",
    ),
    tip(
        "tips/out_of_date.png",
        "Changed a shape after making its toolpath? The toolpath is marked out of date, and \
         Cut Craft offers to recalculate it. Saving or sending an out-of-date toolpath always \
         stops to ask first.",
        "toolpaths",
    ),
    tip(
        "tips/number_sums.png",
        "Number fields take sums. Type **W/2** for half the material's width, or **T** for its \
         thickness, and the answer is worked out when you press Enter.",
        "contents",
    ),
    tip(
        "tips/number_arrows.png",
        "Every number field has small up and down arrows. Hold **Shift** while clicking them \
         for steps ten times as big.",
        "contents",
    ),
    plain(
        "tips/units.png",
        "Switching between millimetres and inches converts every measurement in the job, so \
         nothing changes size. Work in whichever your rulers and bits use.",
    ),
    plain(
        "tips/mouse_zoom.png",
        "Roll the mouse wheel to zoom in and out about the pointer. Drag with the middle mouse \
         button to move around the drawing.",
    ),
    tip(
        "tips/views_2d_3d.png",
        "**F2** shows the 2D view and **F3** the 3D view. **F11** and **F12** switch between \
         the layout for designing and the layout for machining.",
        "contents",
    ),
    tip(
        "tips/esc_close.png",
        "Press **Esc** to close the open tool's settings without making anything.",
        "contents",
    ),
    tip(
        "tips/edit_again.png",
        "Select a shape and press **E** to change it in the tool that made it, with the same \
         settings you used.",
        "contents",
    ),
    plain(
        "tips/snapping.png",
        "Drawing snaps to the grid, guides and the ends and middles of lines. **F4** chooses \
         what it snaps to, and holding **Shift** while you draw turns snapping off.",
    ),
    plain(
        "tips/undo.png",
        "Made a mistake? **Ctrl+Z** undoes it, as many times as you need. **Ctrl+Y** puts it \
         back.",
    ),
    plain(
        "tips/favourites.png",
        "Use a tool all the time? Right-click its button ▸ **Add to Favourites**, then \
         right-click the page's tab to show just your favourites.",
    ),
    plain(
        "tips/bigger_bits_first.png",
        "Pockets with fine detail cut much faster with **Clear with bigger bits first**: a \
         large bit clears the open space and the small bit only cuts the corners it can't \
         reach.",
    ),
    tip(
        "tips/toolpath_time.png",
        "**Toolpath Summary**, on the Toolpaths page, says how long each toolpath takes to cut, \
         so you know how long the machine will be busy before you start.",
        "toolpath_summary",
    ),
    plain(
        "tips/increment_save.png",
        "**File ▸ Increment and Save** saves a new copy with the next number on its name (Sign \
         2, Sign 3), so you can always go back to an earlier version.",
    ),
    plain(
        "tips/recovery_copy.png",
        "Cut Craft keeps a recovery copy of unsaved work every 30 seconds. If the power goes \
         off, your work can be opened again. **Edit ▸ Options ▸ General** sets how often.",
    ),
    plain(
        "tips/trace_bitmap.png",
        "Have a picture or logo? **Trace Bitmap** turns it into vectors you can carve.",
    ),
    plain(
        "tips/relief_from_photo.png",
        "**Relief from Photo** (Modeling ▸ Shape Creation) turns a picture into a 3D shape to \
         carve.",
    ),
    plain(
        "tips/job_sheet.png",
        "**Toolpaths ▸ Print Job Sheet** prints what the job needs at the machine: the file \
         names, each bit and where it goes, and how long it all takes.",
    ),
    plain(
        "tips/report_bug.png",
        "Something not working as it should? Right-click ▸ **Report a Bug Here…** sends a \
         report about the part of the window you clicked on.",
    ),
    plain(
        "tips/first_setup.png",
        "Got a new machine, or set it up wrong the first time? **Help ▸ Run First-Time Setup \
         Again** asks the starting questions again.",
    ),
    tip(
        "tips/keyboard.png",
        "**Help ▸ Keyboard Shortcuts** lists the key for every tool, so you can work without \
         the menus.",
        "contents",
    ),
];

/// The tip after `n`, starting over after the last.
pub(super) fn next(n: usize) -> usize {
    (n + 1) % TIPS.len()
}

/// The tip before `n`, the last before the first.
pub(super) fn previous(n: usize) -> usize {
    (n + TIPS.len() - 1) % TIPS.len()
}

/// The Help window's page of every tip.
pub(super) fn markdown() -> String {
    let mut md = String::from(
        "[Contents](help:contents)\n\n# Did You Know?\n\nShort tips for getting more out of \
         Cut Craft. One is shown each time Cut Craft starts; turn that off in the tip's window \
         or in **Edit ▸ Options ▸ General**.\n\n",
    );
    for t in TIPS {
        md.push_str("- ");
        md.push_str(t.text);
        md.push('\n');
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
        let (cache, icons) = (&mut self.tips_cache, &self.icons);
        let id = egui::Id::unique("did_you_know");
        cut_craft_bug_report::dialog(ctx, id, "Did You Know?", size, &mut open, |ui| {
            let tip = TIPS[n];
            ui.weak(format!("Tip {} of {}", n + 1, TIPS.len()));
            // the illustration across the top, 2:1, then the words under it
            let wide = ui
                .available_width()
                .min(22.0 * ui.spacing().interact_size.y);
            let picture = match icons.find(tip.picture, "") {
                Some(key) => icons.image(key),
                None => egui::Image::from_bytes("bytes://splash-logo.jpg", SPLASH_LOGO),
            };
            ui.vertical_centered(|ui| {
                ui.add(picture.fit_to_exact_size(egui::vec2(wide, wide / 2.0)));
            });
            ui.add_space(ui.spacing().item_spacing.y);
            egui_commonmark::CommonMarkViewer::new().show(ui, cache, tip.text);
            ui.add_space(ui.spacing().item_spacing.y);
            ui.horizontal(|ui| {
                if ui.button("◀ Previous").clicked() {
                    press = Some(Press::Previous);
                }
                if ui.button("Next ▶").clicked() {
                    press = Some(Press::Next);
                }
                if let Some(topic) = tip.topic
                    && ui.button("Tell Me More…").clicked()
                {
                    press = Some(Press::More(topic));
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.checkbox(&mut show, "Show a tip when Cut Craft starts");
                if ui.button("Close").clicked() {
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
