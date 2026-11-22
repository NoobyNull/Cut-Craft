//! Each kind of setting's input: numbers, choices, icon rows, corners, faces, lists and seeds.

use super::{
    CORNER_PICKER_ROWS, Choice, FACES_PICTURE_ROWS, HoverHint, Kind, Look, MOST_RADIOS, Number,
    Value, Widget, blank, decimals, egui, feel, hover, icon_path, named, next_seed, note_help,
    offered, pick_material, sequence, show, shown_in, slide, store, tr,
};

/// Edits `value`; `around` holds the fields beside it, for pictures named by them. `name` is
/// the setting's label, what a screen reader calls its box.
pub(super) fn edit(
    ui: &mut egui::Ui,
    name: &str,
    kind: &Kind,
    value: &mut Value,
    around: &Value,
    look: &Look,
) -> bool {
    match kind {
        Kind::Number {
            unit,
            min,
            max,
            integer,
            steps,
            slider,
            percent_when,
        } => {
            if !steps.is_empty() {
                return pick_step(ui, steps, value);
            }
            // a percentage while another choice says so, else the unit stored
            let unit = match percent_when {
                Some(when) if when.shown(around) => Some("%"),
                _ => unit.as_deref(),
            };
            let (factor, unit) = match unit.map(|u| (u, shown_in(u, look.units))) {
                Some((_, Some((factor, shown)))) => (factor, Some(shown.to_owned())),
                Some((stored, None)) => (1.0, Some(stored.to_owned())),
                None => (1.0, None),
            };
            if let (true, Some(lo), Some(hi)) = (*slider, *min, *max) {
                return slide(ui, name, lo..=hi, *integer, factor, unit.as_deref(), value);
            }
            let mut n = value.as_f64().unwrap_or_default() / factor;
            let range = min.map_or(f64::NEG_INFINITY, |m| m / factor)
                ..=max.map_or(f64::INFINITY, |m| m / factor);
            // steps and drags that fit the unit shown (an inch job drags in thousandths)
            let (step, speed) = feel(unit.as_deref(), *integer && factor == 1.0, range.clone());
            // a sum typed in (calculation edit boxes) is worked out
            let material = look.material.clone();
            let mut drag = Number::new(&mut n)
                .custom_parser(move |text| cut_craft_core::calc::evaluate(text, &material))
                .range(range)
                .step(step)
                .speed(speed)
                .trim_zeros()
                .name(name);
            if step < 0.001 {
                // a few micrometres in inches: enough places to see them
                drag = drag.max_decimals(decimals(step) + 1);
            }
            if let Some(unit) = unit {
                drag = drag.suffix(format!(" {unit}"));
            }
            // a whole number of micrometres shows as a fraction of an inch
            if *integer && factor == 1.0 {
                drag = drag.fixed_decimals(0);
            }
            store(ui.add(drag).changed(), value, n * factor, *integer)
        }
        Kind::Seed => seed(ui, name, value),
        Kind::Sequence => sequence(ui, value, around, look),
        Kind::Bool => {
            let mut b = value.as_bool().unwrap_or_default();
            let changed = ui.checkbox(&mut b, "").changed();
            if changed {
                *value = Value::Bool(b);
            }
            changed
        }
        Kind::Text { multiline } => {
            let mut s = value.as_str().unwrap_or_default().to_owned();
            let changed = if *multiline {
                ui.text_edit_multiline(&mut s)
            } else {
                ui.text_edit_singleline(&mut s)
            }
            .changed();
            if changed {
                *value = Value::String(s);
            }
            changed
        }
        Kind::Choice(options, widget) => choose(ui, options, widget, value, around, look),
        Kind::Material => pick_material(ui, value, look.materials),
        Kind::Group(inner) => ui.vertical(|ui| show(ui, inner, value, look)).inner,
        Kind::List(item) => {
            ui.vertical(|ui| edit_list(ui, name, item, value, look))
                .inner
        }
        Kind::Colour => {
            let mut rgb = [0, 1, 2].map(|i| {
                value[i]
                    .as_u64()
                    .and_then(|c| u8::try_from(c).ok())
                    .unwrap_or_default()
            });
            let changed = ui.color_edit_button_srgb(&mut rgb).changed();
            if changed {
                *value = Value::from(rgb.to_vec());
            }
            changed
        }
        Kind::Other => {
            ui.label(value.to_string());
            false
        }
    }
}

pub(super) fn choose(
    ui: &mut egui::Ui,
    options: &[Choice],
    widget: &Widget,
    value: &mut Value,
    around: &Value,
    look: &Look,
) -> bool {
    let mut current = value.as_str().unwrap_or_default().to_owned();
    let all = options;
    let options = &offered(all, around)[..];
    // a pick a rule now hides (Profile Pass: First, then Adaptive chosen) becomes the first
    // one offered, so a choice is always shown picked
    let hidden = !options.is_empty()
        && all.iter().any(|o| o.value == current)
        && !options.iter().any(|o| o.value == current);
    if hidden {
        current.clone_from(&options[0].value);
        *value = Value::String(current.clone());
    }
    let picked = match widget {
        Widget::Auto if options.len() <= MOST_RADIOS => {
            // stacked, so their size does not depend on the width the layout gives them
            ui.vertical(|ui| radios(ui, options, &current)).inner
        }
        Widget::Row => ui.horizontal(|ui| radios(ui, options, &current)).inner,
        Widget::Auto => drop_down(ui, options, &current),
        Widget::DropDown => {
            ui.vertical(|ui| {
                let picked = drop_down(ui, options, &current);
                let shown = picked.as_deref().unwrap_or(&current);
                if let Some(help) = options
                    .iter()
                    .find(|o| o.value == shown)
                    .and_then(|o| o.help.as_ref())
                {
                    ui.weak(help);
                }
                picked
            })
            .inner
        }
        Widget::Corners => corners(ui, options, &current),
        Widget::Faces { picture } => {
            let picture = picture.as_deref().and_then(look.picture);
            faces(ui, options, &current, picture)
        }
        Widget::Icons {
            picture,
            named: with_names,
        } => {
            let template = picture.as_deref().map(|t| named(t, around));
            icon_row(
                ui,
                options,
                &current,
                look,
                template.as_deref(),
                *with_names,
            )
        }
    };
    match picked {
        Some(v) if v != current => {
            *value = Value::String(v);
            true
        }
        _ => hidden,
    }
}

/// A picture button an option, side by side, the chosen one pressed; an option with no
/// picture shows its name.
pub(super) fn icon_row(
    ui: &mut egui::Ui,
    options: &[Choice],
    current: &str,
    look: &Look,
    template: Option<&str>,
    named: bool,
) -> Option<String> {
    // named pictures are the size of two rows, so what they show can be made out
    let rows = if named { NAMED_ICON_ROWS } else { 1.0 };
    let edge = egui::Vec2::splat(ui.spacing().interact_size.y * rows);
    let mut picked = None;
    ui.horizontal_wrapped(|ui| {
        for o in options {
            let path = template.map(|t| icon_path(t, &Value::String(o.value.clone())));
            let chosen = o.value == current;
            let button = match path.as_deref().and_then(look.picture) {
                Some(picture) => egui::Button::image(picture.fit_to_exact_size(edge))
                    .selected(chosen)
                    .min_size(edge),
                None => egui::Button::selectable(chosen, &o.label),
            };
            let hover = o
                .help
                .as_deref()
                .map_or_else(|| o.label.clone(), |h| format!("{}: {h}", o.label));
            let response = if named {
                // a picture with its name under it is a block egui cannot measure before
                // laying it out, so it never wraps by itself: start a new row when it would
                // not fit
                // (measured to what can be seen: the panel may be laid out wider than it shows)
                let room = ui
                    .available_width()
                    .min(ui.clip_rect().max.x - ui.cursor().min.x);
                if room < edge.x + ui.spacing().button_padding.x * 2.0 {
                    ui.end_row();
                }
                ui.vertical(|ui| {
                    let width = edge.x + ui.spacing().button_padding.x * 2.0;
                    ui.set_max_width(width);
                    let r = ui.add(button).on_hover_hint(&hover);
                    // the name has a fixed place under its picture: a longer one never
                    // makes its row taller and moves what is below
                    fixed_text(ui, width, NAMED_LABEL_ROWS, |ui| {
                        // a choice of one axis is named in that axis's colour
                        match crate::axes::Axis::named(&o.value) {
                            Some(axis) => {
                                ui.label(crate::axes::label(&o.label, axis).small());
                            }
                            None => {
                                ui.small(&o.label);
                            }
                        }
                    });
                    r
                })
                .inner
            } else {
                ui.add(button).on_hover_hint(&hover)
            };
            if response.contains_pointer() {
                note_help(ui.ctx(), &hover);
            }
            if response.clicked() {
                picked = Some(o.value.clone());
            }
        }
    });
    picked
}

/// How many rows tall a named icon's picture is.
pub(super) const NAMED_ICON_ROWS: f32 = 2.0;

/// How many lines of small text a named icon's name has room for.
pub(super) const NAMED_LABEL_ROWS: u8 = 2;

/// A fixed place, `width` wide and `rows` lines of small text tall, for text that changes or
/// differs in length: it wraps inside it and anything past is cut off, so it never grows
/// or shrinks its panel (which flickers and moves everything below).
pub fn fixed_text(ui: &mut egui::Ui, width: f32, rows: u8, add: impl FnOnce(&mut egui::Ui)) {
    let line = ui.text_style_height(&egui::TextStyle::Small);
    fixed_area(ui, egui::vec2(width, line * f32::from(rows)), add);
}

/// A fixed place of `size` for what `add` shows, clipped to it.
pub fn fixed_area(ui: &mut egui::Ui, size: egui::Vec2, add: impl FnOnce(&mut egui::Ui)) {
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let clip = rect.intersect(ui.clip_rect());
    // a child that does not move the cursor on: only `rect` is taken, whatever it holds
    let mut inside = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    inside.set_clip_rect(clip);
    add(&mut inside);
}

pub(super) fn radios(ui: &mut egui::Ui, options: &[Choice], current: &str) -> Option<String> {
    let mut picked = None;
    // a choice's name never breaks mid-word in a narrow column ("Convent / ional")
    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
    for o in options {
        let radio = ui.radio(o.value == current, &o.label);
        if radio.clicked() {
            picked = Some(o.value.clone());
        }
        hover(radio, o.help.as_ref());
    }
    picked
}

pub(super) fn drop_down(ui: &mut egui::Ui, options: &[Choice], current: &str) -> Option<String> {
    let mut picked = None;
    let shown = options
        .iter()
        .find(|o| o.value == current)
        .map_or(current.to_owned(), |o| o.label.clone());
    egui::ComboBox::from_id_salt(ui.next_auto_id())
        .selected_text(shown)
        .show_ui(ui, |ui| {
            for o in options {
                if ui.selectable_label(o.value == current, &o.label).clicked() {
                    picked = Some(o.value.clone());
                }
            }
        });
    picked
}

/// The XY datum picker: the material as a square, a spot at each corner and the centre.
pub(super) fn corners(ui: &mut egui::Ui, options: &[Choice], current: &str) -> Option<String> {
    let row = ui.spacing().interact_size.y;
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(row, row) * CORNER_PICKER_ROWS,
        egui::Sense::click(),
    );
    let spot = row / 4.0;
    let square = rect.shrink(spot);
    let visuals = ui.visuals();
    let painter = ui.painter_at(rect);
    painter.rect_stroke(
        square,
        0,
        visuals.widgets.inactive.fg_stroke,
        egui::StrokeKind::Middle,
    );
    for o in options {
        if let Some(at) = corner_spot(&o.value) {
            let fill = if o.value == current {
                visuals.selection.bg_fill
            } else {
                visuals.extreme_bg_color
            };
            painter.circle(
                square.min + at * square.size(),
                spot,
                fill,
                visuals.widgets.inactive.fg_stroke,
            );
        }
    }
    let label = options
        .iter()
        .find(|o| o.value == current)
        .map(|o| o.label.as_str());
    let response = response.on_hover_hint(label.unwrap_or_default());
    let at = response
        .interact_pointer_pos()
        .filter(|_| response.clicked())?;
    let across = (at - square.min) / square.size();
    nearest_corner(options, across).map(|o| o.value.clone())
}

/// One of a few numbers, as radio buttons side by side; true when another was picked.
pub(super) fn pick_step(ui: &mut egui::Ui, steps: &[f64], value: &mut Value) -> bool {
    let n = value.as_f64().unwrap_or_default();
    let is = |step: f64| (step - n).abs() < f64::EPSILON;
    let mut picked = None;
    ui.horizontal(|ui| {
        for &step in steps {
            if ui.radio(is(step), step_label(step)).clicked() {
                picked = Some(step);
            }
        }
    });
    match picked {
        Some(step) if !is(step) => {
            *value = Value::from(step);
            true
        }
        _ => false,
    }
}

/// A step: positive ones carry a plus.
pub fn step_label(step: f64) -> String {
    if step > 0.0 {
        format!("+{step}")
    } else {
        format!("{step}")
    }
}

/// The Initial Orientation: the model's picture with a radio button for each face round it.
pub(super) fn faces(
    ui: &mut egui::Ui,
    options: &[Choice],
    current: &str,
    picture: Option<egui::Image<'static>>,
) -> Option<String> {
    let row = ui.spacing().interact_size.y;
    let edge = row * FACES_PICTURE_ROWS;
    let font = egui::TextStyle::Button.resolve(ui.style());
    let widths: Vec<f32> = options
        .iter()
        .map(|o| {
            let text = ui.painter().layout_no_wrap(
                o.label.clone(),
                font.clone(),
                egui::Color32::PLACEHOLDER,
            );
            // a radio button wraps its text in what is left after its padding either side
            let padding = 2.0 * ui.spacing().button_padding.x;
            text.size().x + ui.spacing().icon_width + ui.spacing().icon_spacing + padding
        })
        .collect();
    let side = options
        .iter()
        .zip(&widths)
        .filter(|(o, _)| face_spot(&o.value).is_some_and(|(column, _)| column != 1))
        .map(|(_, w)| *w)
        .fold(0.0, f32::max);
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(2.0 * side + edge, 2.0 * row + edge),
        egui::Sense::hover(),
    );
    let middle =
        egui::Rect::from_min_size(rect.min + egui::vec2(side, row), egui::vec2(edge, edge));
    if let Some(picture) = picture {
        ui.put(middle, picture.fit_to_exact_size(middle.size()));
    }
    let mut picked = None;
    for (o, &width) in options.iter().zip(&widths) {
        let Some((column, line)) = face_spot(&o.value) else {
            continue;
        };
        let x = match column {
            0 => rect.min.x,
            1 => middle.center().x - width / 2.0,
            _ => middle.max.x,
        };
        let y = match line {
            0 => rect.min.y + row / 2.0,
            1 => middle.min.y + edge / 4.0,
            2 => middle.max.y - edge / 4.0,
            _ => middle.max.y + row / 2.0,
        };
        let at =
            egui::Rect::from_center_size(egui::pos2(x + width / 2.0, y), egui::vec2(width, row));
        let radio = ui.put(at, egui::RadioButton::new(o.value == current, &o.label));
        if radio.clicked() {
            picked = Some(o.value.clone());
        }
        hover(radio, o.help.as_ref());
    }
    picked
}

/// Where a face's radio button sits round the picture: column (left, middle, right) and line
/// (above, upper, lower, below).
pub fn face_spot(value: &str) -> Option<(u8, u8)> {
    Some(match value {
        "top" => (1, 0),
        "back" => (0, 1),
        "left" => (0, 2),
        "right" => (2, 1),
        "front" => (2, 2),
        "bottom" => (1, 3),
        _ => return None,
    })
}

/// Where a corner choice sits in the picker, as fractions across and down the square.
pub fn corner_spot(value: &str) -> Option<egui::Vec2> {
    let (x, y) = match value {
        "top_left" => (0.0, 0.0),
        "top_right" => (1.0, 0.0),
        "bottom_left" => (0.0, 1.0),
        "bottom_right" => (1.0, 1.0),
        "centre" | "center" => (0.5, 0.5),
        _ => return None,
    };
    Some(egui::vec2(x, y))
}

/// The option whose spot is closest to `at` (fractions across and down).
pub fn nearest_corner(options: &[Choice], at: egui::Vec2) -> Option<&Choice> {
    options
        .iter()
        .filter_map(|o| corner_spot(&o.value).map(|spot| (o, (spot - at).length_sq())))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(o, _)| o)
}

pub(super) fn edit_list(
    ui: &mut egui::Ui,
    name: &str,
    item: &Kind,
    value: &mut Value,
    look: &Look,
) -> bool {
    let Some(items) = value.as_array_mut() else {
        return false;
    };
    let mut changed = false;
    let mut remove = None;
    for (i, it) in items.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            let named = format!("{name} {}", i + 1);
            changed |= edit(ui, &named, item, it, &Value::Null, look);
            if ui
                .small_button("×")
                .on_hover_hint(tr!("form-remove"))
                .clicked()
            {
                remove = Some(i);
            }
        });
    }
    if let Some(i) = remove {
        items.remove(i);
        changed = true;
    }
    // "+" copies the last item; the first is a blank one
    if ui
        .small_button("+")
        .on_hover_hint(tr!("form-add-another"))
        .clicked()
    {
        items.push(items.last().cloned().unwrap_or_else(|| blank(item)));
        changed = true;
    }
    changed
}

/// A seed's number, and a New variation button that steps it on.
pub(super) fn seed(ui: &mut egui::Ui, name: &str, value: &mut Value) -> bool {
    let mut n = value.as_u64().unwrap_or_default();
    let mut changed = ui
        .add(Number::new(&mut n).range(0..=u32::MAX).name(name))
        .changed();
    if ui
        .button(tr!("form-new-variation"))
        .on_hover_hint(tr!("form-another-design-with-the-same"))
        .clicked()
    {
        n = u64::from(next_seed(u32::try_from(n).unwrap_or_default()));
        changed = true;
    }
    if changed {
        *value = Value::from(n);
    }
    changed
}

/// What a turn of 180° ÷ `div` comes to (0 for none).
pub(super) fn turn_name(div: u16) -> String {
    if div == 0 {
        tr!("form-no-turn")
    } else {
        tr!(
            "form-turn-degrees",
            angle = ((180.0 / f64::from(div) * 100.0).round() / 100.0).to_string()
        )
    }
}

/// What each sample digit does, for its button's hint: the keys of its words.
pub(super) const SAMPLES: [&str; 5] = [
    "form-sample-space",
    "form-sample-as-designed",
    "form-sample-flipped-ud",
    "form-sample-flipped-lr",
    "form-sample-flipped-both",
];

/// What the pattern maker adds with each sample: the other way (R), a turn (/n), a
/// random size (S) and a random lean (T).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AddWith {
    pub rev: bool,
    pub div: u16,
    pub size: bool,
    pub lean: bool,
}

/// The entry a sample button adds: S and T for a random size and lean, R for the other
/// way, the digit, /n for its turn.
pub fn entry_text(code: u8, with: AddWith) -> String {
    if code == 0 {
        return "0".to_owned();
    }
    let AddWith {
        rev,
        div,
        size,
        lean,
    } = with;
    let mut e = String::new();
    if size {
        e.push('S');
    }
    if lean {
        e.push('T');
    }
    if rev {
        e.push('R');
    }
    e.push(char::from(b'0' + code));
    if div > 0 {
        e = format!("{e}/{div}");
    }
    e
}

/// A Sequence's entries: split at commas and spaces once any has an R or a turn, else one
/// digit each.
pub fn entries(text: &str) -> Vec<String> {
    if text.contains([
        ',', ' ', 'R', 'r', '/', 'S', 's', 'O', 'o', 'T', 't', '(', ')',
    ]) {
        text.split([',', ' '])
            .filter(|e| !e.is_empty())
            .map(String::from)
            .collect()
    } else {
        text.chars().map(String::from).collect()
    }
}

/// `list` written back: plain digits run together, else split by commas.
pub fn join_entries(list: &[String]) -> String {
    let plain = list.iter().all(|e| e.len() == 1);
    list.join(if plain { "" } else { "," })
}
