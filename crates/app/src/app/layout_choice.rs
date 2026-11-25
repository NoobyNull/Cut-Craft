//! Choosing the window layout: the same choices, each with a picture, wherever it is
//! asked. First-time setup asks it, installs that had Cut Craft before ask it once after
//! updating, and the Interface Layout button (under 2D View Control) opens it any time
//! (`docs/superpowers/specs/2026-11-24-shared-tool-model-design.md`).

use super::CutCraftApp;
use crate::options::{Options, WindowLayout};
use cut_craft_i18n::tr;
use eframe::egui;

/// The layout question while it is open: the layout picked so far, and whether it is the
/// once-after-updating question (closing it keeps the layout in use).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct LayoutQuestion {
    pub chosen: WindowLayout,
    pub after_update: bool,
}

/// Whether to ask once after updating: an install that had Cut Craft before (first-time
/// setup, which asks it, is not wanted) and has not answered or closed it yet.
pub(super) fn asks_after_update(options: &Options, first_run: bool) -> bool {
    !first_run && !options.layout_asked
}

/// The layout question answered: `Some` layout to use, or `None` to keep the one in use.
/// Either way it is not asked again.
pub(super) fn answer(options: &mut Options, chosen: Option<WindowLayout>) {
    if let Some(layout) = chosen {
        options.general.layout = layout;
    }
    options.layout_asked = true;
}

/// A layout picture's height for its width (every picture is 1400 by 433).
const PICTURE_SHAPE: f32 = 433.0 / 1400.0;

/// A layout's picture in the help, light theme beside dark.
pub(super) fn picture(layout: WindowLayout) -> &'static str {
    match layout {
        WindowLayout::Ribbon => "layouts/ribbon-light-and-dark.png",
        WindowLayout::Rail => "layouts/rail-light-and-dark.png",
        WindowLayout::Canvas => "layouts/canvas-light-and-dark.png",
        WindowLayout::Classic => "layouts/classic-light-and-dark.png",
    }
}

/// A layout's name.
pub(super) fn name(layout: WindowLayout) -> String {
    match layout {
        WindowLayout::Ribbon => tr!("layout-choice-ribbon"),
        WindowLayout::Rail => tr!("layout-choice-rail"),
        WindowLayout::Canvas => tr!("layout-choice-canvas"),
        WindowLayout::Classic => tr!("layout-choice-classic"),
    }
}

/// What a layout is like, in a line.
fn about(layout: WindowLayout) -> String {
    match layout {
        WindowLayout::Ribbon => tr!("layout-choice-ribbon-about"),
        WindowLayout::Rail => tr!("layout-choice-rail-about"),
        WindowLayout::Canvas => tr!("layout-choice-canvas-about"),
        WindowLayout::Classic => tr!("layout-choice-classic-about"),
    }
}

/// Each layout card's width: all across, as wide as fits in most of a `screen` that wide,
/// up to fourteen `row`s. Sized from the screen, not from the window it is in, which
/// sizes itself to its cards: measured from that, it would grow a little every frame.
fn card_width(screen: f32, edges: f32, gap: f32, row: f32) -> f32 {
    let across = f32::from(u8::try_from(WindowLayout::ALL.len()).unwrap_or(u8::MAX));
    let room = 0.85 * screen - (across - 1.0) * gap - across * edges;
    (room / across).clamp(row, 14.0 * row)
}

/// The layouts side by side, each its picture, name and what it is like; a click
/// on any part picks it.
pub(super) fn layout_choices(ui: &mut egui::Ui, chosen: &mut WindowLayout) {
    let frame = egui::Frame::group(ui.style());
    let edges = frame.total_margin().sum().x;
    let width = card_width(
        ui.ctx().content_rect().width(),
        edges,
        ui.spacing().item_spacing.x,
        ui.spacing().interact_size.y,
    );
    ui.horizontal_top(|ui| {
        for layout in WindowLayout::ALL {
            let picked = *chosen == layout;
            let frame = egui::Frame::group(ui.style()).fill(if picked {
                ui.visuals().selection.bg_fill.gamma_multiply(0.35)
            } else {
                egui::Color32::TRANSPARENT
            });
            let mut radio = None;
            let response = frame
                .show(ui, |ui| {
                    ui.set_width(width);
                    ui.vertical(|ui| {
                        // sized before it loads, so nothing moves when it does
                        ui.add(
                            egui::Image::new(cut_craft_help::picture_uri(picture(layout)))
                                .fit_to_exact_size(egui::vec2(width, width * PICTURE_SHAPE))
                                .alt_text(name(layout)),
                        );
                        radio = Some(ui.radio(picked, egui::RichText::new(name(layout)).strong()));
                        ui.label(about(layout));
                    });
                })
                .response
                .interact(egui::Sense::click());
            if response.clicked() || radio.is_some_and(|r| r.clicked()) {
                *chosen = layout;
            }
        }
    });
}

impl CutCraftApp {
    /// Interface Layout: the layout question, starting from the layout in use.
    pub(super) fn open_layout_picker(&mut self) {
        self.layout_question = Some(LayoutQuestion {
            chosen: self.options.general.layout,
            after_update: false,
        });
    }

    /// The once-after-updating question, starting from Ribbon.
    pub(super) fn ask_layout_after_update(&mut self) {
        self.layout_question = Some(LayoutQuestion {
            chosen: WindowLayout::default(),
            after_update: true,
        });
    }

    /// The layout question answered (`None` keeps the layout in use); the window rearranges
    /// next frame and the answer is saved.
    pub(super) fn answer_layout(&mut self, chosen: Option<WindowLayout>) {
        self.layout_question = None;
        answer(&mut self.options, chosen);
        self.save_options();
    }

    /// The layout question's window, while it is open.
    pub(super) fn layout_window(&mut self, ctx: &egui::Context) {
        let Some(mut question) = self.layout_question else {
            return;
        };
        super::help::load_pictures(ctx);
        let mut answered = None;
        let modal = egui::Modal::new(egui::Id::unique("layout_question")).show(ctx, |ui| {
            cut_craft_bug_report::mark(ui, "Window layout");
            if question.after_update {
                ui.heading(tr!("layout-choice-pick-your-layout"));
                ui.label(tr!("layout-choice-now-starts-in-ribbon"));
            } else {
                ui.heading(tr!("layout-choice-window-layout"));
                ui.label(tr!("layout-choice-every-tool-in-each"));
            }
            ui.add_space(ui.spacing().item_spacing.y);
            layout_choices(ui, &mut question.chosen);
            ui.add_space(ui.spacing().item_spacing.y);
            ui.weak(tr!("layout-choice-change-any-time"));
            ui.separator();
            ui.horizontal(|ui| {
                let use_it = egui::Button::new(
                    egui::RichText::new(tr!("layout-choice-use-this-layout")).strong(),
                )
                .fill(ui.visuals().selection.bg_fill);
                if ui.add(use_it).clicked() {
                    answered = Some(Some(question.chosen));
                }
                let keep = if question.after_update {
                    tr!(
                        "layout-choice-keep-my-layout",
                        layout = name(self.options.general.layout)
                    )
                } else {
                    tr!("layout-choice-cancel")
                };
                if ui.button(keep).clicked() {
                    answered = Some(None);
                }
            });
        });
        // Escape or a click outside closes it as Keep or Cancel would
        if answered.is_none() && modal.should_close() {
            answered = Some(None);
        }
        match answered {
            // Cancel on the button's picker changes nothing and asks nothing
            Some(None) if !question.after_update => self.layout_question = None,
            Some(chosen) => self.answer_layout(chosen),
            None => self.layout_question = Some(question),
        }
    }
}

#[cfg(test)]
mod tests;
