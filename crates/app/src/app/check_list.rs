//! The checks before cutting as Save Toolpaths and the Analyse tab show them: every check
//! by name, waiting, running or checked while they run, then passed, to read and tick,
//! must fix or not checked (with why), each with a ? that opens its help. A warning is
//! ticked as read before the toolpaths are saved or sent: the ⚠ and the asking are never
//! switched off.

use super::{CutCraftApp, Info, egui};
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_core::preflight::{Check, Finding, Phase, Severity};

/// The help page about all the checks.
pub(super) const OVERVIEW: &str = "checks_before_cutting";

/// What the list says while the checks run: each check's place in the run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Now {
    Waiting,
    Running,
    Checked,
}

/// How a check came out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Outcome {
    Passed,
    /// Found something to read and tick.
    Warning,
    /// Found something that must be fixed before saving.
    MustFix,
    /// Could not be made (the reason is beside it).
    NotChecked,
}

/// Where `check` is while the checks are on `phase` (`None` before any has started).
pub(super) fn running(check: Check, phase: Option<Phase>) -> Now {
    match phase {
        Some(now) if check.phase() < now => Now::Checked,
        Some(now) if check.phase() == now => Now::Running,
        _ => Now::Waiting,
    }
}

/// How `check` came out, from what was `found` and the checks `skipped`.
pub(super) fn outcome(check: Check, found: &[Finding], skipped: &[(Check, String)]) -> Outcome {
    let mine = || found.iter().filter(|f| f.check == check);
    if mine().any(|f| f.severity == Severity::Error) {
        Outcome::MustFix
    } else if mine().next().is_some() {
        Outcome::Warning
    } else if skipped.iter().any(|(c, _)| *c == check) {
        Outcome::NotChecked
    } else {
        Outcome::Passed
    }
}

/// The line over the finished list: how many checks passed, are to read and tick, must be
/// fixed and could not be made.
pub(super) fn summary(found: &[Finding], skipped: &[(Check, String)]) -> String {
    let count = |o: Outcome| {
        Check::ALL
            .iter()
            .filter(|&&c| outcome(c, found, skipped) == o)
            .count()
    };
    let mut parts = vec![format!("✔ {} passed", count(Outcome::Passed))];
    let warnings = count(Outcome::Warning);
    if warnings > 0 {
        parts.push(format!("⚠ {warnings} to read"));
    }
    let errors = count(Outcome::MustFix);
    if errors > 0 {
        parts.push(format!("✖ {errors} must fix"));
    }
    let skipped = count(Outcome::NotChecked);
    if skipped > 0 {
        parts.push(format!("{skipped} not checked"));
    }
    parts.join(" · ")
}

/// How wide the status column is, so the names line up whatever each status says.
const STATUS_WIDTH: f32 = 22.0;

impl CutCraftApp {
    /// The checks before cutting: the heading with its ?, then every check while they run
    /// or once they have, with what each found; the warnings are ticked as read here.
    /// `found` is `None` while they run or once they were cancelled. Whether Check Again
    /// was pressed.
    pub(super) fn check_list(
        &mut self,
        ui: &mut egui::Ui,
        found: Option<&[Finding]>,
        skipped: &[(Check, String)],
    ) -> bool {
        let mut help = None;
        ui.horizontal(|ui| {
            ui.strong("Checks before cutting");
            if help_button(
                ui,
                "What each check looks for, and how to fix what it finds",
            ) {
                help = Some(OVERVIEW);
            }
        });
        let mut again = false;
        match found {
            None if self.checking() => {
                let phase = self.busy.as_ref().and_then(|b| Phase::of_stage(&b.stage()));
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Checking the toolpaths: a big 3D toolpath takes a little while");
                });
                rows_area(ui, |ui| {
                    for check in Check::ALL {
                        if let Some(topic) = running_row(ui, check, running(check, phase)) {
                            help = Some(topic);
                        }
                    }
                });
            }
            None => {
                again = self.checks_waiting(ui);
            }
            Some(found) => {
                ui.label(summary(found, skipped));
                if found.iter().any(|f| f.severity != Severity::Error)
                    && !self.all_acknowledged(found)
                {
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        "Read each ⚠ and tick it to say you have: Save Toolpaths and Send to \
                         Machine wait until every one is ticked",
                    );
                }
                rows_area(ui, |ui| {
                    // what needs a look first, then what passed, then what was not checked
                    let mut order = Check::ALL;
                    order.sort_by_key(|&c| match outcome(c, found, skipped) {
                        Outcome::MustFix => 0,
                        Outcome::Warning => 1,
                        Outcome::Passed => 2,
                        Outcome::NotChecked => 3,
                    });
                    for check in order {
                        if let Some(topic) = self.outcome_row(ui, check, found, skipped) {
                            help = Some(topic);
                        }
                    }
                });
            }
        }
        if let Some(topic) = help {
            self.info = Some(Info::Help(topic));
        }
        again
    }

    /// One check once the checks have run: how it came out, then each thing it found (a
    /// warning with its tick box); the help topic when its ? was pressed.
    fn outcome_row(
        &mut self,
        ui: &mut egui::Ui,
        check: Check,
        found: &[Finding],
        skipped: &[(Check, String)],
    ) -> Option<&'static str> {
        let visuals = ui.visuals().clone();
        let result = outcome(check, found, skipped);
        let (mark, colour) = match result {
            Outcome::Passed => ("✔", egui::Color32::from_rgb(0x3c, 0xa0, 0x4a)),
            Outcome::Warning => ("⚠", visuals.warn_fg_color),
            Outcome::MustFix => ("✖", visuals.error_fg_color),
            Outcome::NotChecked => ("–", visuals.weak_text_color()),
        };
        let why = skipped
            .iter()
            .find(|(c, _)| *c == check)
            .map(|(_, why)| why.as_str());
        let pressed = row(
            ui,
            check,
            |ui| {
                status(ui, egui::RichText::new(mark).color(colour));
            },
            why.filter(|_| result == Outcome::NotChecked),
        );
        for f in found.iter().filter(|f| f.check == check) {
            ui.horizontal(|ui| {
                ui.add_space(STATUS_WIDTH + ui.spacing().item_spacing.x);
                ui.vertical(|ui| {
                    if f.severity == Severity::Error {
                        ui.colored_label(
                            visuals.error_fg_color,
                            format!("Must fix: {}", f.message),
                        );
                        return;
                    }
                    let mut ticked = self.acknowledged.contains(&f.message);
                    let text = egui::RichText::new(format!("⚠ {}", f.message))
                        .color(visuals.warn_fg_color);
                    if ui
                        .checkbox(&mut ticked, text)
                        .on_hover_hint("Tick to say you have read this and it is what you want")
                        .changed()
                    {
                        if ticked {
                            self.acknowledged.insert(f.message.clone());
                        } else {
                            self.acknowledged.remove(&f.message);
                        }
                    }
                });
            });
        }
        pressed
    }
}

/// How many lines of checks show at once: the list scrolls within them, so it is as tall
/// while the checks run as once they have, and the buttons under it never move.
const ROWS_SHOWN: f32 = 9.0;

/// The checks' lines, in a box of their own that scrolls.
fn rows_area(ui: &mut egui::Ui, rows: impl FnOnce(&mut egui::Ui)) {
    let height = ui.spacing().interact_size.y * ROWS_SHOWN;
    egui::ScrollArea::vertical()
        .id_salt("check_list_rows")
        .auto_shrink([false, false])
        .max_height(height)
        .min_scrolled_height(height)
        .show(ui, rows);
}

/// A small ? that opens help; whether it was pressed.
fn help_button(ui: &mut egui::Ui, hint: &str) -> bool {
    ui.small_button("?").on_hover_hint(hint).clicked()
}

/// The status column, always as wide, so every name starts in the same place.
fn status(ui: &mut egui::Ui, text: egui::RichText) {
    ui.add_sized(
        [STATUS_WIDTH, ui.spacing().interact_size.y],
        egui::Label::new(text),
    );
}

/// One check's line: its status (drawn by `mark`), its name (pointing at it says what it
/// protects against), why it was not checked, and its ?; the help topic when pressed.
fn row(
    ui: &mut egui::Ui,
    check: Check,
    mark: impl FnOnce(&mut egui::Ui),
    not_checked: Option<&str>,
) -> Option<&'static str> {
    let mut pressed = false;
    ui.horizontal(|ui| {
        mark(ui);
        ui.label(check.name()).on_hover_hint(check.guards());
        if let Some(why) = not_checked {
            ui.weak(format!("not checked: {why}"));
        }
        pressed = help_button(ui, check.guards());
    });
    pressed.then_some(check.help())
}

/// One check while the checks run: waiting, running or checked.
fn running_row(ui: &mut egui::Ui, check: Check, now: Now) -> Option<&'static str> {
    row(
        ui,
        check,
        |ui| match now {
            Now::Running => {
                ui.add_sized(
                    [STATUS_WIDTH, ui.spacing().interact_size.y],
                    egui::Spinner::new(),
                );
            }
            Now::Waiting => status(ui, egui::RichText::new("○").weak()),
            Now::Checked => status(ui, egui::RichText::new("✔").weak()),
        },
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn warning(check: Check) -> Finding {
        Finding::warning(check, format!("{check:?} found something"))
    }

    #[test]
    fn checks_wait_run_and_finish_in_phase_order() {
        assert_eq!(running(Check::Rapids, None), Now::Waiting);
        let settings = Some(Phase::Settings);
        assert_eq!(running(Check::Reach, settings), Now::Running);
        assert_eq!(running(Check::Bed, settings), Now::Waiting);
        let cutting = Some(Phase::Cutting);
        assert_eq!(running(Check::Reach, cutting), Now::Checked);
        assert_eq!(running(Check::KeepOut, cutting), Now::Checked);
        assert_eq!(running(Check::AirCutting, cutting), Now::Running);
    }

    #[test]
    fn each_check_comes_out_by_the_worst_it_found() {
        let found = vec![
            warning(Check::Reach),
            Finding::error(Check::Bed, "too big".into()),
            warning(Check::Bed),
        ];
        let skipped = vec![(Check::ZTravel, "no Z travel".to_owned())];
        assert_eq!(outcome(Check::Reach, &found, &skipped), Outcome::Warning);
        assert_eq!(outcome(Check::Bed, &found, &skipped), Outcome::MustFix);
        assert_eq!(
            outcome(Check::ZTravel, &found, &skipped),
            Outcome::NotChecked
        );
        assert_eq!(outcome(Check::Rapids, &found, &skipped), Outcome::Passed);
        let all = Check::ALL.len();
        assert_eq!(
            summary(&found, &skipped),
            format!(
                "✔ {} passed · ⚠ 1 to read · ✖ 1 must fix · 1 not checked",
                all - 3
            )
        );
        assert_eq!(summary(&[], &[]), format!("✔ {all} passed"));
    }
}
