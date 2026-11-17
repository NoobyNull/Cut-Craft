//! The Notes panel: rich notes (headings, lists, check boxes, links, bold, italic; never
//! HTML) written in a box that fills the panel, with a toolbar, a Preview, and the Variable
//! Explorer, which lists every live value notes can show and puts one in with a
//! double-click, a drag or Insert.

use cut_craft_bug_report::hint::HoverHint;
use cut_craft_core::notes::{self, Format, Kind, Variable};
use eframe::egui;
use serde_json::Value;
use std::rc::Rc;

/// The share of the panel's height the Variable Explorer takes while it is shown.
const EXPLORER_SHARE: f32 = 0.45;
/// The fewest rows of text the notes box keeps, however small the panel.
const LEAST_TEXT_ROWS: f32 = 6.0;

/// The toolbar: its label, what it does, and its hint.
const TOOLBAR: [(&str, Format, &str); 7] = [
    ("B", Format::Bold, "Bold: **words**"),
    ("I", Format::Italic, "Italic: *words*"),
    ("H", Format::Heading, "Heading: a line starting ## "),
    ("•", Format::Bullet, "Bulleted list: lines starting - "),
    ("1.", Format::Numbered, "Numbered list: lines starting 1. "),
    (
        "☐",
        Format::Check,
        "Check list: lines starting - [ ] , ticked in Preview",
    ),
    (
        "Link",
        Format::Link,
        "Link to a web page: [words](https://…)",
    ),
];

/// What the Notes panel keeps between frames.
#[derive(Default)]
pub struct NotesUi {
    /// Showing the notes as they will read, variables filled in, instead of writing them.
    pub preview: bool,
    /// The Variable Explorer is put away.
    pub explorer_hidden: bool,
    /// What is typed in the Variable Explorer's search box.
    pub search: String,
    /// The variable picked in the Variable Explorer, for Insert.
    pub picked: Option<String>,
    /// The chars picked in the notes box (a cursor when empty), for the toolbar and Insert.
    pub selection: Option<std::ops::Range<usize>>,
}

/// A variable being dragged from the Variable Explorer: its key.
#[derive(Clone, Debug)]
struct Dragged(String);

/// Whether `v` matches every word of `search` (in its name, key, value or description).
fn matches(v: &Variable, search: &str) -> bool {
    let all = format!(
        "{} {} {} {} {} {}",
        v.group, v.under, v.label, v.key, v.value, v.description
    )
    .to_lowercase();
    search
        .to_lowercase()
        .split_whitespace()
        .all(|word| all.contains(word))
}

/// The first line of a value, short enough for a row.
fn short(value: &str) -> String {
    let first = value.lines().next().unwrap_or_default();
    let more = value.lines().nth(1).is_some();
    let mut out: String = first.chars().take(28).collect();
    if more || first.chars().count() > 28 {
        out.push('…');
    }
    out
}

impl super::CutCraftApp {
    /// The notes' variables now, as the Variable Explorer, the job sheet, the G-code and the
    /// Web API show them; worked out again only when something they come from changes.
    pub(super) fn note_variables(&self) -> Rc<Vec<Variable>> {
        let now = chrono::Local::now();
        let ctx = self.notes_context(&now);
        let key = format!(
            "{}|{}|{:?}|{}|{}|{:?}|{}|{}|{:?}|{}|{:?}|{}|{}|{:?}",
            self.job_epoch,
            self.history.revision(),
            ctx.model_height,
            ctx.machine,
            ctx.controller,
            ctx.bed,
            ctx.post,
            ctx.time,
            ctx.acceleration,
            ctx.job_file,
            ctx.slots,
            ctx.rapid,
            ctx.scale,
            ctx.holding,
        );
        if let Some((at, vars)) = self.note_vars.borrow().as_ref()
            && *at == key
        {
            return vars.clone();
        }
        let vars = Rc::new(notes::variables(&self.doc, &ctx));
        *self.note_vars.borrow_mut() = Some((key, vars.clone()));
        vars
    }

    /// What the notes' variables need from outside the job: its file, the machine, the time,
    /// the Toolpath Summary's run time settings, the model's height and the collet nut.
    fn notes_context(&self, now: &chrono::DateTime<chrono::Local>) -> notes::Context {
        let machine = self.machines.active();
        let s = &self.summary;
        notes::Context {
            job_file: self
                .session
                .path
                .as_deref()
                .map_or_default(|p| p.display().to_string()),
            machine: machine.map_or_default(|m| m.name.clone()),
            controller: machine.map_or_default(|m| m.controller.clone()),
            post: machine
                .and_then(|m| m.post())
                .unwrap_or_default()
                .to_owned(),
            bed: machine.map_or([0.0; 2], |m| [m.width, m.height]),
            slots: self.slots(),
            date: now.format("%Y-%m-%d").to_string(),
            time: now.format("%H:%M").to_string(),
            rapid: s.rapid,
            acceleration: machine
                .filter(|_| s.acceleration)
                .map_or([0.0; 3], |m| m.acceleration),
            scale: s.scale,
            model_height: self.model_height_now(),
            holding: self.holding(),
        }
    }

    /// The 3D model's height, lowest point to highest; `None` with no model. Measured once
    /// a change.
    pub(super) fn model_height_now(&self) -> Option<f64> {
        if self.doc.components.is_empty() {
            return None;
        }
        let revision = self.history.revision();
        let tall = match self.model_tall.get() {
            Some((at, tall)) if at == revision => tall,
            _ => {
                let tall = cmd_scale_model_z_height::model_height(&self.doc);
                let tall = if tall.is_finite() { tall } else { 0.0 };
                self.model_tall.set(Some((revision, tall)));
                tall
            }
        };
        Some(tall)
    }

    /// The Notes form: its toolbar, its two ticks, the notes box filling the panel (or the
    /// Preview) and the Variable Explorer under it.
    #[expect(clippy::too_many_lines, reason = "the panel's parts, top to bottom")]
    pub(super) fn notes_editor(&mut self, ui: &mut egui::Ui) {
        let vars = self.note_variables();
        let state = &mut self.notes_ui;
        let Some(open) = self.form.as_mut() else {
            return;
        };
        let raw = open.params["notes"].as_str().unwrap_or_default().to_owned();
        let mut opens = notes::opens_with_job(&raw);
        let mut body = notes::strip_open_mark(&raw).to_owned();
        let mut in_gcode = open.params["in_gcode"].as_bool().unwrap_or(false);
        let mut changed = false;
        let text_id = egui::Id::unique("notes_text");

        // Write / Preview and the Variable Explorer, then the toolbar
        let mut format = None;
        ui.horizontal(|ui| {
            ui.selectable_value(&mut state.preview, false, "Write")
                .on_hover_hint("Write the notes; {{variables}} show as coloured chips");
            ui.selectable_value(&mut state.preview, true, "Preview")
                .on_hover_hint("See the notes as they will read, with every variable filled in");
            ui.separator();
            let mut shown = !state.explorer_hidden;
            if ui
                .toggle_value(&mut shown, "{x} Variables")
                .on_hover_hint(
                    "Show or hide the Variable Explorer: live values to put in the notes",
                )
                .changed()
            {
                state.explorer_hidden = !shown;
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.add_enabled_ui(!state.preview, |ui| {
                for (label, f, hint) in TOOLBAR {
                    if ui.button(label).on_hover_hint(hint).clicked() {
                        format = Some(f);
                    }
                }
            });
        });
        changed |= ui
            .checkbox(&mut opens, "Show these notes when the job opens")
            .on_hover_hint("The notes open by themselves each time the job is opened")
            .changed();
        changed |= ui
            .checkbox(&mut in_gcode, "Put these notes at the top of the G-code")
            .on_hover_hint(
                "Each saved G-code file starts with the notes, variables filled in, as \
                 comments the machine skips",
            )
            .changed();

        // the notes box gets what the Variable Explorer leaves
        let row = ui.text_style_height(&egui::TextStyle::Body);
        let explorer = if state.explorer_hidden {
            0.0
        } else {
            (ui.available_height() * EXPLORER_SHARE).max(row * 8.0)
        };
        let text_height = (ui.available_height() - explorer).max(row * LEAST_TEXT_ROWS);
        let mut insert: Option<(String, Option<usize>)> = None;
        let size = egui::vec2(ui.available_width(), text_height);
        ui.allocate_ui_with_layout(size, egui::Layout::top_down(egui::Align::Min), |ui| {
            ui.set_min_size(size);
            if state.preview {
                if let Some(line) = preview(ui, &raw, &vars, size) {
                    notes::strip_open_mark(&notes::toggle_check(&raw, line)).clone_into(&mut body);
                    changed = true;
                }
                return;
            }
            let known: std::collections::HashSet<&str> =
                vars.iter().map(|v| v.key.as_str()).collect();
            let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap: f32| {
                let galley_job = chips(ui, text.as_str(), &known, wrap);
                ui.fonts_mut(|f| f.layout_job(galley_job))
            };
            // long notes scroll inside the box, so the Variable Explorer stays in sight
            let scrolled = egui::ScrollArea::vertical()
                .id_salt("notes-text")
                .max_height(text_height)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    egui::TextEdit::multiline(&mut body)
                        .id(text_id)
                        .hint_text(
                            "Notes for this job: what to clamp, which bit goes first…\n\
                     The buttons above make headings and lists; drag a variable in from \
                     the Variable Explorer to show a live value.",
                        )
                        .desired_width(f32::INFINITY)
                        .min_size(size)
                        .layouter(&mut layouter)
                        .show(ui)
                });
            let output = scrolled.inner;
            if output.response.changed() {
                changed = true;
            }
            // what was picked while typing stays picked for the toolbar and Insert, which
            // take the focus away from the box
            if let Some(range) = output.cursor_range.filter(|_| output.response.has_focus()) {
                let r = range.as_sorted_char_range();
                state.selection = Some(r.start.0..r.end.0);
            }
            let at_pointer = |pos: egui::Pos2| {
                output
                    .galley
                    .cursor_from_pos(pos - output.galley_pos)
                    .index
                    .0
            };
            // a variable dropped on the notes goes in where it was let go
            if let Some(dragged) = output.response.dnd_release_payload::<Dragged>() {
                let at = ui.ctx().pointer_interact_pos().map(at_pointer);
                insert = Some((dragged.0.clone(), at));
            } else if output.response.dnd_hover_payload::<Dragged>().is_some() {
                ui.painter().rect_stroke(
                    output.response.rect,
                    2.0,
                    ui.visuals().selection.stroke,
                    egui::StrokeKind::Inside,
                );
            }
            // pointing at a variable chip says what it is now
            if let Some(pos) = output.response.hover_pos() {
                let at = at_pointer(pos);
                let text = body.as_str();
                let char_of = |b: usize| text[..b].chars().count();
                if let Some((_, key)) = notes::variable_spans(text)
                    .into_iter()
                    .find(|(r, _)| char_of(r.start) <= at && at < char_of(r.end))
                {
                    let said = vars.iter().find(|v| v.key == key).map_or_else(
                        || format!("{{{{{key}}}}} is not a variable: check its spelling"),
                        |v| format!("{}: {}", v.label, v.value),
                    );
                    output
                        .response
                        .response
                        .clone()
                        .on_hover_text_at_pointer(said);
                }
            }
        });

        if !state.explorer_hidden {
            ui.separator();
            if let Some(key) = variable_explorer(ui, state, &vars) {
                insert = Some((key, None));
            }
        }

        // the toolbar and Insert work on what is picked in the notes box
        let picked = state.selection.clone().unwrap_or_else(|| {
            let end = body.chars().count();
            end..end
        });
        let mut select = None;
        if let Some(f) = format {
            let (text, after) = f.apply(&body, picked.clone());
            body = text;
            select = Some(after);
            changed = true;
        }
        if let Some((key, at)) = insert {
            let (text, after) =
                notes::insert_at(&body, at.unwrap_or(picked.end), &format!("{{{{{key}}}}}"));
            body = text;
            select = Some(after..after);
            changed = true;
        }
        if let Some(range) = select {
            let mut text_state =
                egui::text_edit::TextEditState::load(ui.ctx(), text_id).unwrap_or_default();
            text_state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::two(
                    egui::text::CCursor::new(range.start),
                    egui::text::CCursor::new(range.end),
                )));
            text_state.store(ui.ctx(), text_id);
            ui.memory_mut(|m| m.request_focus(text_id));
            state.selection = Some(range);
        }
        if changed {
            let mark = if opens {
                notes::OPEN_MARK.to_string()
            } else {
                String::new()
            };
            open.params["notes"] = Value::String(format!("{mark}{body}"));
            open.params["in_gcode"] = Value::Bool(in_gcode);
            open.dirty = true;
        }
    }
}

/// The notes as they are typed, each `{{variable}}` drawn as a chip: in the selection's
/// colour when it is a variable, in red when it is not (a typo).
fn chips(
    ui: &egui::Ui,
    text: &str,
    known: &std::collections::HashSet<&str>,
    wrap: f32,
) -> egui::text::LayoutJob {
    let font = egui::TextStyle::Body.resolve(ui.style());
    let plain = egui::TextFormat::simple(font.clone(), ui.visuals().text_color());
    let chip = |ok: bool| egui::TextFormat {
        background: if ok {
            ui.visuals().selection.bg_fill.gamma_multiply(0.6)
        } else {
            ui.visuals().error_fg_color.gamma_multiply(0.35)
        },
        color: ui.visuals().strong_text_color(),
        ..egui::TextFormat::simple(font.clone(), ui.visuals().text_color())
    };
    let mut job = egui::text::LayoutJob::default();
    let mut at = 0;
    for (range, key) in notes::variable_spans(text) {
        job.append(&text[at..range.start], 0.0, plain.clone());
        job.append(&text[range.clone()], 0.0, chip(known.contains(key)));
        at = range.end;
    }
    job.append(&text[at..], 0.0, plain);
    job.wrap.max_width = wrap;
    job
}

/// The notes as they will read: headings, lists, links and ticks, variables filled in.
/// Gives the line whose check box was clicked.
fn preview(ui: &mut egui::Ui, raw: &str, vars: &[Variable], size: egui::Vec2) -> Option<usize> {
    let mut ticked = None;
    egui::Frame::group(ui.style()).show(ui, |ui| {
        let inner = size - egui::vec2(12.0, 12.0);
        egui::ScrollArea::vertical()
            .id_salt("notes_preview")
            .max_height(inner.y)
            .show(ui, |ui| {
                ui.set_min_size(inner);
                let lines = notes::parse_resolved(raw, vars);
                if lines.iter().all(|l| l.text().trim().is_empty()) {
                    ui.weak("No notes yet: write some under Write.");
                }
                for (i, line) in lines.iter().enumerate() {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        let size = match line.kind {
                            Kind::Heading(1) => Some(1.5),
                            Kind::Heading(2) => Some(1.3),
                            Kind::Heading(_) => Some(1.15),
                            _ => None,
                        };
                        match &line.kind {
                            Kind::Bullet => {
                                ui.label("  •  ");
                            }
                            Kind::Numbered(n) => {
                                ui.label(format!("  {n}.  "));
                            }
                            Kind::Check(done) => {
                                let mut done = *done;
                                if ui
                                    .checkbox(&mut done, "")
                                    .on_hover_hint("Tick or untick it")
                                    .changed()
                                {
                                    ticked = Some(i);
                                }
                            }
                            _ => {}
                        }
                        for span in &line.spans {
                            let mut text = egui::RichText::new(&span.text);
                            if let Some(scale) = size {
                                text = text
                                    .size(ui.text_style_height(&egui::TextStyle::Body) * scale)
                                    .strong();
                            }
                            if span.bold {
                                text = text.strong();
                            }
                            if span.italic {
                                text = text.italics();
                            }
                            if span.variable.is_some() {
                                text = text.background_color(
                                    ui.visuals().selection.bg_fill.gamma_multiply(0.35),
                                );
                            }
                            match &span.link {
                                Some(url) => {
                                    ui.hyperlink_to(text, url).on_hover_text(url);
                                }
                                None => {
                                    ui.label(text);
                                }
                            }
                        }
                        if line.spans.is_empty() {
                            ui.label(" ");
                        }
                    });
                }
            });
    });
    ticked
}

/// The Variable Explorer: a search box and every variable in its group (a toolpath's or a
/// tool's under it), each with its value now; gives the key to put in, from a double-click
/// or Insert. Rows can be dragged into the notes too.
fn variable_explorer(ui: &mut egui::Ui, state: &mut NotesUi, vars: &[Variable]) -> Option<String> {
    let mut chosen = None;
    ui.strong("Variable Explorer");
    ui.weak("Double-click a variable, or drag it into the notes. Click one, then Insert.");
    let row = egui::vec2(ui.available_width(), ui.spacing().interact_size.y);
    ui.allocate_ui_with_layout(
        row,
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            let picked = state.picked.clone();
            if ui
                .add_enabled(picked.is_some(), egui::Button::new("Insert"))
                .on_hover_hint("Put the picked variable in the notes, where the text cursor is")
                .clicked()
            {
                chosen = picked;
            }
            ui.add(
                egui::TextEdit::singleline(&mut state.search)
                    .hint_text("Search: thickness, feed, tool…")
                    .desired_width(ui.available_width()),
            )
            .on_hover_hint("Find a variable by its name, its value or what it is");
        },
    );
    let searching = !state.search.trim().is_empty();
    egui::ScrollArea::vertical()
        .id_salt("variable_explorer")
        .auto_shrink([false, false])
        .max_height(ui.available_height())
        .show(ui, |ui| {
            let mut any = false;
            for group in notes::GROUPS {
                let rows: Vec<&Variable> = vars
                    .iter()
                    .filter(|v| v.group == group && (!searching || matches(v, &state.search)))
                    .collect();
                if rows.is_empty() {
                    continue;
                }
                any = true;
                egui::CollapsingHeader::new(format!("{group} ({})", rows.len()))
                    .id_salt(("variables", group))
                    .default_open(matches!(group, "Job" | "Material"))
                    .open(searching.then_some(true))
                    .show(ui, |ui| {
                        for v in rows.iter().filter(|v| v.under.is_empty()) {
                            chosen = chosen.take().or_else(|| variable_row(ui, state, v));
                        }
                        let mut unders: Vec<&str> = vec![];
                        for v in &rows {
                            if !v.under.is_empty() && !unders.contains(&v.under.as_str()) {
                                unders.push(&v.under);
                            }
                        }
                        for under in unders {
                            egui::CollapsingHeader::new(under)
                                .id_salt(("variables", group, under))
                                .open(searching.then_some(true))
                                .show(ui, |ui| {
                                    for v in rows.iter().filter(|v| v.under == under) {
                                        chosen =
                                            chosen.take().or_else(|| variable_row(ui, state, v));
                                    }
                                });
                        }
                    });
            }
            if !any {
                ui.weak("No variable matches that: try fewer or other words.");
            }
        });
    chosen
}

/// One variable's row: its name and value now, picked with a click, put in with a
/// double-click or dragged into the notes. Gives its key when double-clicked.
fn variable_row(ui: &mut egui::Ui, state: &mut NotesUi, v: &Variable) -> Option<String> {
    let picked = state.picked.as_deref() == Some(v.key.as_str());
    let id = egui::Id::unique(("variable", &v.key));
    let inner = ui.horizontal(|ui| {
        ui.set_width(ui.available_width());
        if picked {
            ui.strong(&v.label);
        } else {
            ui.label(&v.label);
        }
        ui.weak(short(&v.value));
    });
    let rect = inner.response.rect;
    if picked {
        ui.painter().rect_filled(
            rect,
            2.0,
            ui.visuals().selection.bg_fill.gamma_multiply(0.25),
        );
    }
    let response = ui
        .interact(rect, id, egui::Sense::click_and_drag())
        .on_hover_text(format!(
            "{}\n{}\nNow: {}\nIn the notes: {{{{{}}}}}",
            v.label, v.description, v.value, v.key
        ));
    if response.clicked() {
        state.picked = Some(v.key.clone());
    }
    if response.drag_started() {
        egui::DragAndDrop::set_payload(ui.ctx(), Dragged(v.key.clone()));
    }
    if response.dragged()
        && let Some(at) = ui.ctx().pointer_interact_pos()
    {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        egui::Area::new(id.with("dragged"))
            .order(egui::Order::Tooltip)
            .fixed_pos(at + egui::vec2(12.0, 4.0))
            .interactable(false)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.label(format!("{{{{{}}}}}", v.key));
                });
            });
    }
    response.double_clicked().then(|| v.key.clone())
}

/// A variable's value now, by key (for the tests).
#[cfg(test)]
pub(super) fn value_now(app: &super::CutCraftApp, key: &str) -> String {
    notes::value(&app.note_variables(), key)
        .unwrap_or_default()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn var(key: &str, label: &str, value: &str) -> Variable {
        Variable {
            key: key.into(),
            group: "Material",
            under: String::new(),
            label: label.into(),
            description: "How thick the material is (Z)".into(),
            value: value.into(),
        }
    }

    #[test]
    fn the_explorer_finds_a_variable_by_any_of_its_words() {
        let v = var("material.thickness", "Thickness", "18 mm");
        for search in ["thick", "THICKNESS", "18 mm", "material z", ""] {
            assert!(matches(&v, search), "{search}");
        }
        assert!(!matches(&v, "feed"));
    }

    #[test]
    fn a_long_or_many_line_value_is_cut_short_in_its_row() {
        assert_eq!(short("18 mm"), "18 mm");
        assert_eq!(short("T1: 6 mm\nT2: 3 mm"), "T1: 6 mm…");
        assert_eq!(short(&"x".repeat(40)).chars().count(), 29);
    }
}
