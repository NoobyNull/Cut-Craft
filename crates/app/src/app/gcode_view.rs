//! View G-code: the file Save Toolpaths would write, shown in a window without saving it, to
//! read or copy (a 0.6.11 user: "Sometimes just need to quickly look at gcode and not need to
//! save it"). Also Save Toolpaths' question when the checks before cutting found something,
//! asked over the window: a system message box can open behind Cut Craft's window (on Linux
//! it is a window of another program, not kept above Cut Craft's).

use super::{CutCraftApp, SaveForm};
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_i18n::tr;
use eframe::egui;

/// The G-code window's contents, made when it opened (Refresh makes them again).
pub(super) struct GcodeView {
    /// The choices it was made with, which Save Toolpaths… opens the form with.
    pub(super) form: SaveForm,
    /// The post-processor it went through.
    pub(super) post: String,
    /// When it was made, as the file's header says.
    pub(super) made: chrono::NaiveDateTime,
    /// Each file's name and G-code, in the order they are run; why there is none.
    pub(super) files: Result<Vec<(String, String)>, String>,
    /// Where each line of each file starts, so a long file scrolls without splitting it
    /// every frame.
    starts: Vec<Vec<usize>>,
    /// The file shown.
    shown: usize,
}

/// Where each line of `text` starts.
fn line_starts(text: &str) -> Vec<usize> {
    let bytes = text.as_bytes();
    // a line ends at LF, CR LF or a lone CR (a post's LINE_ENDING can be any of them)
    let ends = bytes.iter().enumerate().filter_map(|(at, &b)| match b {
        b'\n' => Some(at + 1),
        b'\r' if bytes.get(at + 1) != Some(&b'\n') => Some(at + 1),
        _ => None,
    });
    std::iter::once(0)
        .chain(ends)
        .filter(|&at| at < text.len())
        .collect()
}

/// Line `n` of `text`, its line starts `starts`, without its line ending.
fn line<'a>(text: &'a str, starts: &[usize], n: usize) -> &'a str {
    let end = starts.get(n + 1).copied().unwrap_or(text.len());
    text[starts[n]..end].trim_end_matches(['\n', '\r'])
}

/// `text`'s lines, its line starts `starts`, numbered down the left: only those in sight are
/// laid out, so a 3D finish of millions of lines scrolls freely. `file` keeps each file's
/// place.
fn numbered_lines(ui: &mut egui::Ui, text: &str, starts: &[usize], file: usize) {
    let digits = starts.len().max(1).to_string().len();
    // a row is a horizontal layout: at least the interact height, whatever the text's
    let row = ui
        .text_style_height(&egui::TextStyle::Monospace)
        .max(ui.spacing().interact_size.y);
    // set before show_rows, which spaces the rows it skips by it
    ui.spacing_mut().item_spacing.y = 0.0;
    egui::ScrollArea::both()
        .id_salt(("gcode_lines", file))
        .auto_shrink(false)
        .show_rows(ui, row, starts.len(), |ui, rows| {
            for n in rows {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("{:>digits$}", n + 1))
                            .monospace()
                            .weak(),
                    );
                    ui.add(
                        egui::Label::new(egui::RichText::new(line(text, starts, n)).monospace())
                            .extend(),
                    );
                });
            }
        });
}

/// What the checks before cutting say of the G-code shown.
#[derive(Debug, PartialEq)]
enum Checks {
    /// No toolpaths to check (nothing to cut, or no post).
    None,
    Running,
    Cancelled,
    Found(Vec<cut_craft_core::preflight::Finding>),
}

/// The checks' findings above the G-code: each error in red, each warning in amber, or a
/// green line when they found nothing. Nothing is stopped: the operator decides.
fn found_list(ui: &mut egui::Ui, checks: &Checks) {
    use cut_craft_core::preflight::Severity;
    let visuals = ui.visuals().clone();
    match checks {
        Checks::None => {}
        Checks::Running => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.weak(tr!("gcode-view-checks-running"));
            });
        }
        Checks::Cancelled => {
            ui.colored_label(visuals.warn_fg_color, tr!("gcode-view-checks-cancelled"));
        }
        Checks::Found(found) if found.is_empty() => {
            ui.colored_label(
                egui::Color32::from_rgb(0x3c, 0xa0, 0x4a),
                tr!("gcode-view-checks-passed"),
            );
        }
        Checks::Found(found) => {
            ui.label(tr!("gcode-view-checks-found"));
            // errors first, as they need the operator most
            let mut sorted: Vec<_> = found.iter().collect();
            sorted.sort_by_key(|f| f.severity != Severity::Error);
            let height = ui.spacing().interact_size.y * 5.0;
            egui::ScrollArea::vertical()
                .id_salt("gcode_findings")
                .max_height(height)
                .show(ui, |ui| {
                    for f in sorted {
                        let message = f.message.clone();
                        if f.severity == Severity::Error {
                            ui.colored_label(
                                visuals.error_fg_color,
                                tr!("gcode-view-error", message = message),
                            );
                        } else {
                            ui.colored_label(
                                visuals.warn_fg_color,
                                tr!("gcode-view-warning", message = message),
                            );
                        }
                    }
                });
        }
    }
    ui.separator();
}

/// Frames the Save Toolpaths window takes to close: not drawn the next frame, it is gone the
/// one after.
const FRAMES_TO_CLOSE: u8 = 2;

/// Save or Send to Machine pressed in Save Toolpaths, run once its window has gone.
pub(super) struct SaveAfter {
    form: SaveForm,
    sending: bool,
    /// Frames since it was pressed.
    waited: u8,
}

impl SaveAfter {
    pub(super) fn new(form: SaveForm, sending: bool) -> Self {
        Self {
            form,
            sending,
            waited: 0,
        }
    }
}

/// Save Toolpaths held for the question about what the checks before cutting found.
pub(super) struct SaveAnyway {
    form: SaveForm,
    problems: Vec<String>,
    /// Asked for Send to Machine rather than Save.
    sending: bool,
}

impl CutCraftApp {
    /// Opens the G-code window on what saving `form` would write, through its post (the
    /// machine's default unless the form picked another).
    pub(super) fn view_gcode(&mut self, form: SaveForm) {
        let made = chrono::Local::now().naive_local();
        let post = self.form_post(&form);
        let post_name = post
            .as_ref()
            .map_or_else(|_| String::new(), |p| p.name.clone());
        let files = post.and_then(|post| {
            let name = self.output().file_name(&form, &post);
            let folder = self
                .session
                .path
                .as_deref()
                .and_then(std::path::Path::parent);
            let path = folder.map_or_else(|| name.clone().into(), |f| f.join(&name));
            let texts = self.toolpath_texts(&form, &post, &path, made)?;
            Ok(texts
                .into_iter()
                .map(|(path, text)| (super::file_name(&path), text))
                .collect::<Vec<_>>())
        });
        let starts = files.as_ref().map_or_else(
            |_| vec![],
            |f| f.iter().map(|(_, t)| line_starts(t)).collect(),
        );
        self.gcode_view = Some(GcodeView {
            form,
            post: post_name,
            made,
            files,
            starts,
            shown: 0,
        });
    }

    /// The G-code window: which post and file, the file's lines numbered, Copy All and Save
    /// Toolpaths….
    pub(super) fn gcode_window(&mut self, ctx: &egui::Context) {
        let checks = self.gcode_checks();
        let Some(view) = self.gcode_view.as_mut() else {
            return;
        };
        let mut open = true;
        let (mut refresh, mut save) = (false, false);
        let id = egui::Id::unique("view_gcode");
        let size = super::dialog_size(ctx);
        cut_craft_bug_report::dialog_with_panels(ctx, id, "View G-code", size, &mut open, |ui| {
            if !view.post.is_empty() {
                ui.horizontal(|ui| {
                    ui.label(tr!("gcode-view-post-processor"));
                    ui.strong(&view.post);
                });
            }
            let files = match &view.files {
                Ok(files) if !files.is_empty() => files,
                Ok(_) => {
                    ui.label(tr!("gcode-view-there-is-nothing-to-cut"));
                    return;
                }
                Err(why) => {
                    ui.colored_label(ui.visuals().warn_fg_color, why);
                    refresh = ui.button(tr!("gcode-view-refresh")).clicked();
                    return;
                }
            };
            found_list(ui, &checks);
            view.shown = view.shown.min(files.len() - 1);
            ui.horizontal(|ui| {
                ui.label(tr!("gcode-view-file"));
                if files.len() > 1 {
                    egui::ComboBox::from_id_salt("gcode_file")
                        .selected_text(&files[view.shown].0)
                        .show_ui(ui, |ui| {
                            for (n, (name, _)) in files.iter().enumerate() {
                                ui.selectable_value(&mut view.shown, n, name);
                            }
                        })
                        .response
                        .on_hover_hint(tr!("gcode-view-saving-makes-these-files-run"));
                } else {
                    ui.strong(&files[0].0);
                }
                ui.weak(tr!(
                    "gcode-view-lines-made-at",
                    count = view.starts[view.shown].len(),
                    time = view.made.format("%H:%M:%S").to_string()
                ));
            });
            ui.weak(tr!("gcode-view-what-saving-would-write-through"));
            let (_, text) = &files[view.shown];
            ui.horizontal(|ui| {
                if ui
                    .button(tr!("gcode-view-copy-all"))
                    .on_hover_hint(tr!("gcode-view-copy-this-file-s-g"))
                    .clicked()
                {
                    ui.ctx().copy_text(text.clone());
                }
                save = ui
                    .button(tr!("gcode-view-save-toolpaths"))
                    .on_hover_hint(tr!("gcode-view-check-the-toolpaths-and-save"))
                    .clicked();
                refresh = ui
                    .button(tr!("gcode-view-refresh"))
                    .on_hover_hint(tr!("gcode-view-make-the-g-code-again"))
                    .clicked();
            });
            ui.separator();
            numbered_lines(ui, text, &view.starts[view.shown], view.shown);
        });
        let form = view.form.clone();
        if !open {
            self.gcode_view = None;
        } else if refresh {
            // cancelled checks run again with it
            self.check_again();
            self.view_gcode(form);
        } else if save {
            // its window would sit over the file dialog the save opens
            self.gcode_view = None;
            self.save_form = Some(form);
        }
    }

    /// What the checks before cutting found for the G-code window's toolpaths: the same
    /// checks Save Toolpaths runs (kept, and run again only when the job or machine
    /// changed), started in the background when they have not run.
    fn gcode_checks(&mut self) -> Checks {
        let Some(form) = self.gcode_view.as_ref().map(|v| v.form.clone()) else {
            return Checks::None;
        };
        let Some(plan) = self
            .form_post(&form)
            .and_then(|post| self.plan(&form, &post))
            .ok()
        else {
            return Checks::None;
        };
        if let Some(found) = self.save_checks(&plan, form.changes) {
            return Checks::Found(found.to_vec());
        }
        if self.checking() || self.busy.is_some() {
            Checks::Running
        } else {
            Checks::Cancelled
        }
    }

    /// Runs a Save or Send pressed in Save Toolpaths once its window has closed, then opens
    /// the form again once the file dialog and every question the save asks are answered.
    pub(super) fn save_when_form_closed(&mut self, ctx: &egui::Context) {
        if let Some(after) = self.save_after.as_mut() {
            ctx.request_repaint();
            if after.waited < FRAMES_TO_CLOSE {
                after.waited += 1;
                return;
            }
            let Some(after) = self.save_after.take() else {
                return;
            };
            self.form_back = Some(after.form.clone());
            // a window of its own too: nothing may cover the save's dialog and questions
            self.gcode_view = None;
            if after.sending {
                self.send_toolpaths(&after.form);
            } else {
                self.save_toolpaths(&after.form);
            }
        }
        if self.save_anyway.is_none()
            && self.earlier_ask.is_none()
            && let Some(form) = self.form_back.take()
        {
            self.save_form = Some(form);
        }
    }

    /// Asks over the window (not by a message box of the system's) whether to save
    /// toolpaths the checks before cutting found `problems` in; the answer saves or sends
    /// `form` again.
    pub(super) fn ask_save_anyway(
        &mut self,
        form: &SaveForm,
        problems: Vec<String>,
        sending: bool,
    ) {
        self.save_anyway = Some(SaveAnyway {
            form: form.clone(),
            problems,
            sending,
        });
    }

    /// The question [`Self::ask_save_anyway`] asks: Save Anyway, or Don't Save.
    pub(super) fn save_anyway_question(&mut self, ctx: &egui::Context) {
        let Some(ask) = self.save_anyway.as_ref() else {
            return;
        };
        let mut answer = None;
        egui::Modal::new(egui::Id::unique("save_anyway")).show(ctx, |ui| {
            ui.set_width(ui.spacing().interact_size.x * 11.0);
            ui.heading(tr!("gcode-view-check-before-cutting"));
            // a long list scrolls, so the buttons under it stay in reach
            egui::ScrollArea::vertical()
                .max_height(ui.ctx().content_rect().height() * 0.5)
                .show(ui, |ui| {
                    for problem in &ask.problems {
                        ui.label(format!("• {problem}"));
                    }
                });
            ui.add_space(ui.spacing().item_spacing.y);
            ui.strong(tr!("gcode-view-save-the-toolpaths-anyway"));
            ui.separator();
            ui.horizontal(|ui| {
                if ui
                    .button(if ask.sending {
                        tr!("gcode-view-send-anyway")
                    } else {
                        tr!("gcode-view-save-anyway")
                    })
                    .on_hover_hint(tr!("gcode-view-go-ahead-you-have-read"))
                    .clicked()
                {
                    answer = Some(true);
                }
                if ui
                    .button(if ask.sending {
                        tr!("gcode-view-don-t-send")
                    } else {
                        tr!("gcode-view-don-t-save")
                    })
                    .on_hover_hint(tr!("gcode-view-save-nothing-to-put-the"))
                    .clicked()
                {
                    answer = Some(false);
                }
            });
        });
        let Some(go) = answer else {
            return;
        };
        let Some(ask) = self.save_anyway.take() else {
            return;
        };
        if !go {
            self.status
                .set_message(&cut_craft_i18n::tr!(if ask.sending {
                    "status-toolpaths-not-sent"
                } else {
                    "status-toolpaths-not-saved"
                }));
            return;
        }
        // agreed to the problems shown, and no others
        self.checks_accepted = ask.problems;
        if ask.sending {
            self.send_toolpaths(&ask.form);
        } else {
            self.save_toolpaths(&ask.form);
        }
        // only for that save, whatever stopped it short
        self.checks_accepted.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::{line, line_starts};

    #[test]
    fn lines_are_found_by_where_they_start() {
        let text = "G21\r\nG90\n\nM30\n";
        let starts = line_starts(text);
        assert_eq!(starts.len(), 4);
        let lines: Vec<_> = (0..starts.len()).map(|n| line(text, &starts, n)).collect();
        assert_eq!(lines, ["G21", "G90", "", "M30"]);
        assert_eq!(line_starts("G0 X1"), [0]);
        // a post ending its lines with a lone CR
        let text = "G21\rG90\r";
        let starts = line_starts(text);
        assert_eq!(starts, [0, 4]);
        assert_eq!(line(text, &starts, 1), "G90");
        assert_eq!(line_starts(""), Vec::<usize>::new());
    }
}
