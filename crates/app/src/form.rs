//! Settings forms generated from a command's parameter schema, so no command writes UI.

use cut_craft_core::document::Units;
use eframe::egui;
use serde_json::Value;

/// Choices with this many options or fewer are radio buttons; more make a drop-down.
const MOST_RADIOS: usize = 5;
/// The corner picker's size, in rows of the theme's interactive height.
// ponytail: fixed ratio until a preferences dialog exists
const CORNER_PICKER_ROWS: f32 = 3.0;
/// The picture the face radio buttons sit round, in rows of the theme's interactive height.
// ponytail: fixed ratio until a preferences dialog exists
const FACES_PICTURE_ROWS: f32 = 4.0;
/// A field's own picture beside its label (Aspire's small width / radius pictures), in rows
/// of the theme's interactive height.
// ponytail: fixed ratio until a preferences dialog exists
const FIELD_PICTURE_ROWS: f32 = 1.5;

#[derive(Debug, PartialEq)]
pub struct Field {
    /// JSON key in the params object.
    pub key: String,
    pub label: String,
    /// Doc comment of the parameter, shown on hover.
    pub help: Option<String>,
    pub kind: Kind,
    /// Marked `x-section`: drawn as a framed block with a heading, as Aspire's forms are.
    pub section: bool,
    /// `x-icon`: the section's picture, which may follow its value (see [`icon_path`]).
    pub icon: Option<String>,
    /// Only shown while another choice says so.
    pub when: Option<When>,
}

/// When a field shows (`x-show-when` / `x-hide-when`: `{ "key": "value" or ["values"] }`),
/// judged on the form's top-level parameters: Aspire's forms change with what is chosen.
#[derive(Debug, PartialEq)]
pub struct When {
    pub key: String,
    pub values: Vec<String>,
    /// Shown when the value is one of `values`; else hidden then.
    pub show: bool,
}

impl When {
    fn from_schema(s: &Value) -> Option<Self> {
        let (rule, show) = match (s.get("x-show-when"), s.get("x-hide-when")) {
            (Some(rule), _) => (rule, true),
            (None, Some(rule)) => (rule, false),
            (None, None) => return None,
        };
        let (key, values) = rule.as_object()?.iter().next()?;
        let values = match values {
            Value::Array(list) => list
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect(),
            one => vec![one.as_str()?.to_owned()],
        };
        Some(Self {
            key: key.clone(),
            values,
            show,
        })
    }

    /// Whether the field shows, given the form's top-level `params`.
    pub fn shown(&self, params: &Value) -> bool {
        let is = params[&self.key]
            .as_str()
            .is_some_and(|v| self.values.iter().any(|w| w == v));
        is == self.show
    }
}

/// How a form looks: the units lengths are shown in, and the pictures sections show.
pub struct Look<'a> {
    pub units: Units,
    /// A picture by icon path, sized for a section; `None` when the icon is missing.
    pub picture: &'a dyn Fn(&str) -> Option<egui::Image<'static>>,
    /// The material's size in the units shown, for sums typed into number fields (W, H, T).
    pub material: cut_craft_core::calc::Values,
}

/// The job's material, in `units`, and its document variables, for [`Look::material`].
pub fn material(
    doc: &cut_craft_core::document::Document,
    units: Units,
) -> cut_craft_core::calc::Values {
    let factor = match units {
        Units::Inch => 25.4,
        Units::Mm => 1.0,
    };
    let job = &doc.job;
    cut_craft_core::calc::Values {
        width: job.width / factor,
        height: job.height / factor,
        thickness: job.thickness / factor,
        variables: doc.variables.clone(),
    }
}

#[derive(Debug, PartialEq)]
pub enum Kind {
    Number {
        unit: Option<String>,
        min: Option<f64>,
        max: Option<f64>,
        integer: bool,
        /// `x-steps`: only these values, as radio buttons side by side.
        steps: Vec<f64>,
    },
    Bool,
    /// A string; `multiline` (`x-widget: multiline`) is a box of several lines.
    Text {
        multiline: bool,
    },
    Choice(Vec<Choice>, Widget),
    /// A nested object, e.g. a point.
    Group(Vec<Field>),
    /// An `[r, g, b]` array marked `x-kind: colour`.
    Colour,
    /// A list whose items all have this shape.
    List(Box<Kind>),
    /// Anything the form cannot edit yet; shown read-only.
    Other,
}

/// How a choice is drawn (`x-widget`); Aspire's forms use each of these.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Widget {
    /// Radio buttons, one under another, for a few options; a drop-down for more.
    #[default]
    Auto,
    /// Radio buttons side by side (`row`).
    Row,
    /// A drop-down with the chosen option's description under it (`drop-down`).
    DropDown,
    /// A square with a spot at each corner and the centre, as the XY datum picker (`corners`).
    Corners,
    /// A radio button for each face of a model round its picture (`faces`, with `x-icon`),
    /// as Aspire's Initial Orientation.
    Faces { picture: Option<String> },
}

#[derive(Debug, PartialEq)]
pub struct Choice {
    pub value: String,
    pub label: String,
    pub help: Option<String>,
}

/// Edits `params` through its fields; true when anything changed.
pub fn show(ui: &mut egui::Ui, fields: &[Field], params: &mut Value, look: &Look) -> bool {
    let top = params.clone();
    show_within(ui, fields, params, look, &top)
}

/// [`show`] for `params` somewhere inside the form, whose top-level parameters are `top`.
fn show_within(
    ui: &mut egui::Ui,
    fields: &[Field],
    params: &mut Value,
    look: &Look,
    top: &Value,
) -> bool {
    let shown: Vec<&Field> = fields
        .iter()
        .filter(|f| f.when.as_ref().is_none_or(|w| w.shown(top)))
        .collect();
    let mut changed = false;
    // runs of ordinary fields share a grid; each section is a framed block of its own
    let mut rest = shown.as_slice();
    while !rest.is_empty() {
        let run = rest.iter().position(|f| f.section).unwrap_or(rest.len());
        let (plain, after) = rest.split_at(run);
        if !plain.is_empty() {
            changed |= grid(ui, plain, params, look);
        }
        let Some((section, after)) = after.split_first() else {
            break;
        };
        changed |= ui
            .group(|ui| {
                ui.set_width(ui.available_width());
                hover(ui.strong(&section.label), section.help.as_ref());
                let value = &mut params[&section.key];
                let picture = section
                    .icon
                    .as_ref()
                    .and_then(|template| (look.picture)(&icon_path(template, value)));
                ui.horizontal(|ui| {
                    if let Some(picture) = picture {
                        ui.add(picture);
                    }
                    ui.vertical(|ui| match &section.kind {
                        Kind::Group(inner) => show_within(ui, inner, value, look, top),
                        kind => edit(ui, kind, value, look),
                    })
                    .inner
                })
                .inner
            })
            .inner;
        rest = after;
    }
    changed
}

fn grid(ui: &mut egui::Ui, fields: &[&Field], params: &mut Value, look: &Look) -> bool {
    let mut changed = false;
    egui::Grid::new(ui.next_auto_id())
        .num_columns(2)
        .show(ui, |ui| {
            for f in fields {
                // the faces widget draws its picture itself, with the radio buttons round it
                let own = !matches!(f.kind, Kind::Choice(_, Widget::Faces { .. }));
                let picture = f
                    .icon
                    .as_ref()
                    .filter(|_| own)
                    .and_then(|template| (look.picture)(&icon_path(template, &params[&f.key])));
                ui.horizontal(|ui| {
                    if let Some(picture) = picture {
                        let edge = ui.spacing().interact_size.y * FIELD_PICTURE_ROWS;
                        ui.add(picture.fit_to_exact_size(egui::vec2(edge, edge)));
                    }
                    hover(ui.label(&f.label), f.help.as_ref());
                });
                changed |= edit(ui, &f.kind, &mut params[&f.key], look);
                ui.end_row();
            }
        });
    changed
}

fn hover(label: egui::Response, help: Option<&String>) {
    if let Some(help) = help {
        label.on_hover_text(help);
    }
}

/// How a stored unit shows in `units`: the factor to divide by and the unit to show.
pub fn shown_in(unit: &str, units: Units) -> Option<(f64, &'static str)> {
    match (unit, units) {
        ("mm", Units::Inch) => Some((25.4, "in")),
        ("mm/min", Units::Inch) => Some((25.4, "in/min")),
        _ => None,
    }
}

/// A section's picture for its value: `{}` stands for the value, `{name}` for its field `name`.
pub fn icon_path(template: &str, value: &Value) -> String {
    let mut path = template.replace("{}", value.as_str().unwrap_or_default());
    if let Some(fields) = value.as_object() {
        for (name, v) in fields {
            if let Some(v) = v.as_str() {
                path = path.replace(&format!("{{{name}}}"), v);
            }
        }
    }
    path
}

/// The units a form itself sets (Job Setup's), found anywhere in its params.
pub fn units_chosen(params: &Value) -> Option<Units> {
    match params {
        Value::Object(map) => map
            .get("units")
            .and_then(|u| serde_json::from_value(u.clone()).ok())
            .or_else(|| map.values().find_map(units_chosen)),
        _ => None,
    }
}

fn edit(ui: &mut egui::Ui, kind: &Kind, value: &mut Value, look: &Look) -> bool {
    match kind {
        Kind::Number {
            unit,
            min,
            max,
            integer,
            steps,
        } => {
            if !steps.is_empty() {
                return pick_step(ui, steps, value);
            }
            let (factor, unit) = match unit.as_deref().map(|u| (u, shown_in(u, look.units))) {
                Some((_, Some((factor, shown)))) => (factor, Some(shown.to_owned())),
                Some((stored, None)) => (1.0, Some(stored.to_owned())),
                None => (1.0, None),
            };
            let mut n = value.as_f64().unwrap_or_default() / factor;
            // a sum typed in (Aspire's calculation edit boxes) is worked out
            let material = look.material.clone();
            let mut drag = egui::DragValue::new(&mut n)
                .custom_parser(move |text| cut_craft_core::calc::evaluate(text, &material))
                .range(
                    min.map_or(f64::NEG_INFINITY, |m| m / factor)
                        ..=max.map_or(f64::INFINITY, |m| m / factor),
                );
            if let Some(unit) = unit {
                drag = drag.suffix(format!(" {unit}"));
            }
            if *integer {
                drag = drag.fixed_decimals(0);
            }
            let changed = ui.add(drag).changed();
            if changed {
                *value = if *integer {
                    Value::from(round(n))
                } else {
                    Value::from(n * factor)
                };
            }
            changed
        }
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
        Kind::Choice(options, widget) => choose(ui, options, widget, value, look),
        Kind::Group(inner) => ui.vertical(|ui| show(ui, inner, value, look)).inner,
        Kind::List(item) => ui.vertical(|ui| edit_list(ui, item, value, look)).inner,
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

fn choose(
    ui: &mut egui::Ui,
    options: &[Choice],
    widget: &Widget,
    value: &mut Value,
    look: &Look,
) -> bool {
    let current = value.as_str().unwrap_or_default().to_owned();
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
    };
    match picked {
        Some(v) if v != current => {
            *value = Value::String(v);
            true
        }
        _ => false,
    }
}

fn radios(ui: &mut egui::Ui, options: &[Choice], current: &str) -> Option<String> {
    let mut picked = None;
    for o in options {
        let radio = ui.radio(o.value == current, &o.label);
        if radio.clicked() {
            picked = Some(o.value.clone());
        }
        hover(radio, o.help.as_ref());
    }
    picked
}

fn drop_down(ui: &mut egui::Ui, options: &[Choice], current: &str) -> Option<String> {
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

/// Aspire's XY datum picker: the material as a square, a spot at each corner and the centre.
fn corners(ui: &mut egui::Ui, options: &[Choice], current: &str) -> Option<String> {
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
    let response = response.on_hover_text(label.unwrap_or_default());
    let at = response
        .interact_pointer_pos()
        .filter(|_| response.clicked())?;
    let across = (at - square.min) / square.size();
    nearest_corner(options, across).map(|o| o.value.clone())
}

/// One of a few numbers, as radio buttons side by side; true when another was picked.
fn pick_step(ui: &mut egui::Ui, steps: &[f64], value: &mut Value) -> bool {
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

/// A step as Aspire labels it: positive ones carry a plus.
pub fn step_label(step: f64) -> String {
    if step > 0.0 {
        format!("+{step}")
    } else {
        format!("{step}")
    }
}

/// Aspire's Initial Orientation: the model's picture with a radio button for each face round it.
fn faces(
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
/// (above, upper, lower, below), as in Aspire's dialog.
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

fn edit_list(ui: &mut egui::Ui, item: &Kind, value: &mut Value, look: &Look) -> bool {
    let Some(items) = value.as_array_mut() else {
        return false;
    };
    let mut changed = false;
    let mut remove = None;
    for (i, it) in items.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            changed |= edit(ui, item, it, look);
            if ui.small_button("✕").on_hover_text("Remove").clicked() {
                remove = Some(i);
            }
        });
    }
    if let Some(i) = remove {
        items.remove(i);
        changed = true;
    }
    // "+" copies the last item; an empty list of objects has nothing to copy
    let copyable = !items.is_empty() || !matches!(item, Kind::Group(_));
    if copyable && ui.small_button("+").on_hover_text("Add another").clicked() {
        items.push(items.last().cloned().unwrap_or(Value::Null));
        changed = true;
    }
    changed
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "integer params are small counts"
)]
fn round(n: f64) -> i64 {
    n.round() as i64
}

/// The fields of a params schema, in declaration order.
pub fn fields(schema: &Value) -> Vec<Field> {
    object_fields(schema, schema)
}

fn object_fields(root: &Value, schema: &Value) -> Vec<Field> {
    let Some(props) = resolve(root, schema)["properties"].as_object() else {
        return vec![];
    };
    props
        .iter()
        .map(|(key, s)| Field {
            key: key.clone(),
            label: s["title"]
                .as_str()
                .map_or_else(|| sentence_case(key), String::from),
            help: s["description"]
                .as_str()
                .or_else(|| resolve(root, s)["description"].as_str())
                .map(String::from),
            kind: kind(root, s),
            section: s.get("x-section") == Some(&Value::Bool(true))
                || resolve(root, s).get("x-section") == Some(&Value::Bool(true)),
            icon: s["x-icon"]
                .as_str()
                .or_else(|| resolve(root, s)["x-icon"].as_str())
                .map(String::from),
            when: When::from_schema(s),
        })
        .collect()
}

/// Follows a local `$ref` into `$defs`, where schemars puts shared types.
fn resolve<'a>(root: &'a Value, s: &'a Value) -> &'a Value {
    match s["$ref"].as_str().and_then(|r| r.strip_prefix("#/$defs/")) {
        Some(name) => &root["$defs"][name],
        None => s,
    }
}

fn kind(root: &Value, property: &Value) -> Kind {
    let s = resolve(root, property);
    // marks may sit on the property or on the shared type it refers to
    let marked = |key: &str| property.get(key).or_else(|| s.get(key));
    if marked("x-kind").and_then(Value::as_str) == Some("colour") {
        return Kind::Colour;
    }
    if let Some(options) = choices(s) {
        let widget = match marked("x-widget").and_then(Value::as_str) {
            Some("row") => Widget::Row,
            Some("drop-down") => Widget::DropDown,
            Some("corners") => Widget::Corners,
            Some("faces") => Widget::Faces {
                picture: marked("x-icon").and_then(Value::as_str).map(String::from),
            },
            _ => Widget::Auto,
        };
        return Kind::Choice(options, widget);
    }
    match s["type"].as_str() {
        Some(t @ ("number" | "integer")) => Kind::Number {
            unit: s["x-unit"].as_str().map(String::from),
            min: s["minimum"].as_f64(),
            max: s["maximum"].as_f64(),
            integer: t == "integer",
            steps: marked("x-steps")
                .and_then(Value::as_array)
                .map(|steps| steps.iter().filter_map(Value::as_f64).collect())
                .unwrap_or_default(),
        },
        Some("boolean") => Kind::Bool,
        Some("string") => Kind::Text {
            multiline: marked("x-widget").and_then(Value::as_str) == Some("multiline"),
        },
        Some("object") => Kind::Group(object_fields(root, s)),
        Some("array") => Kind::List(Box::new(kind(root, &s["items"]))),
        _ => Kind::Other,
    }
}

/// String enums: schemars writes `enum: [...]`, or `oneOf` of single values when variants are
/// documented; a variant's `title` is its label and its doc comment its hover help.
fn choices(s: &Value) -> Option<Vec<Choice>> {
    let choice = |value: &str, about: Option<&Value>| Choice {
        value: value.into(),
        label: about
            .and_then(|o| o["title"].as_str())
            .map_or_else(|| sentence_case(value), String::from),
        help: about
            .and_then(|o| o["description"].as_str())
            .map(String::from),
    };
    match s["enum"].as_array() {
        Some(values) => values
            .iter()
            .map(|v| Some(choice(v.as_str()?, None)))
            .collect(),
        None => s["oneOf"]
            .as_array()?
            .iter()
            .map(|o| {
                let value = o["const"].as_str().or_else(|| o["enum"][0].as_str())?;
                Some(choice(value, Some(o)))
            })
            .collect(),
    }
}

fn sentence_case(key: &str) -> String {
    let words = key.replace('_', " ");
    let mut chars = words.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::registry;
    use cut_craft_core::document::Units;

    fn schema(id: &str) -> Value {
        registry().get(id).unwrap().schema()
    }

    fn keys(fields: &[Field]) -> Vec<&str> {
        fields.iter().map(|f| f.key.as_str()).collect()
    }

    fn field<'a>(fields: &'a [Field], key: &str) -> &'a Field {
        fields.iter().find(|f| f.key == key).unwrap()
    }

    #[test]
    fn titles_name_fields_and_choices() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "width": { "title": "Width (X)", "type": "number" },
                "z_zero": { "oneOf": [
                    { "const": "material_surface", "title": "Material Surface", "description": "Tool tip set on the top" },
                    { "const": "machine_bed" }
                ]}
            }
        });
        let f = fields(&schema);
        assert_eq!(field(&f, "width").label, "Width (X)");
        let Kind::Choice(options, _) = &field(&f, "z_zero").kind else {
            panic!("a choice")
        };
        assert_eq!(options[0].label, "Material Surface");
        assert_eq!(options[0].help.as_deref(), Some("Tool tip set on the top"));
        assert_eq!(
            options[1].label, "Machine bed",
            "untitled choices fall back to the value"
        );
    }

    #[test]
    fn marked_groups_are_sections_and_marked_arrays_are_colours() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "size": { "type": "object", "x-section": true, "properties": { "w": { "type": "number" } } },
                "at": { "type": "object", "properties": { "x": { "type": "number" } } },
                "colour": { "type": "array", "items": { "type": "integer" }, "x-kind": "colour" }
            }
        });
        let f = fields(&schema);
        assert!(field(&f, "size").section);
        assert!(matches!(&field(&f, "size").kind, Kind::Group(inner) if inner.len() == 1));
        assert!(!field(&f, "at").section, "unmarked groups stay inline");
        assert_eq!(field(&f, "colour").kind, Kind::Colour);
    }

    #[test]
    fn choices_take_a_widget_hint() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "units": { "enum": ["mm", "inch"], "x-widget": "row" },
                "resolution": { "enum": ["standard", "high"], "x-widget": "drop-down" },
                "datum": { "enum": ["bottom_left", "centre"], "x-widget": "corners" },
                "side": { "enum": ["inside", "outside"] }
            }
        });
        let f = fields(&schema);
        let widget = |key| match &field(&f, key).kind {
            Kind::Choice(_, widget) => widget.clone(),
            other => panic!("{key} is {other:?}"),
        };
        assert_eq!(widget("units"), Widget::Row);
        assert_eq!(widget("resolution"), Widget::DropDown);
        assert_eq!(widget("datum"), Widget::Corners);
        assert_eq!(widget("side"), Widget::Auto);
    }

    #[test]
    fn fields_show_or_hide_with_another_choice() {
        let s = serde_json::json!({
            "type": "object",
            "properties": {
                "kind": { "enum": ["flat", "rotary"] },
                "diameter": { "type": "number", "x-show-when": { "kind": "rotary" } },
                "thickness": { "type": "number", "x-hide-when": { "kind": ["rotary"] } },
            }
        });
        let fields = fields(&s);
        let when = |key| field(&fields, key).when.as_ref().unwrap();
        let (flat, rotary) = (
            serde_json::json!({ "kind": "flat" }),
            serde_json::json!({ "kind": "rotary" }),
        );
        assert!(!when("diameter").shown(&flat) && when("diameter").shown(&rotary));
        assert!(when("thickness").shown(&flat) && !when("thickness").shown(&rotary));
        assert!(field(&fields, "kind").when.is_none());
    }

    #[test]
    fn text_marked_multiline_is_a_box_of_lines() {
        let f = fields(&schema("draw_text"));
        assert_eq!(field(&f, "text").kind, Kind::Text { multiline: true });
        let job = fields(&schema("layer_properties"));
        assert_eq!(field(&job, "name").kind, Kind::Text { multiline: false });
        assert!(
            matches!(&field(&f, "font").kind, Kind::Choice(fonts, Widget::DropDown) if fonts.iter().any(|c| c.value == "Test Boxes")),
            "the font list is the installed fonts"
        );
    }

    #[test]
    fn the_corner_picker_puts_each_datum_where_it_is() {
        use eframe::egui::vec2;
        assert_eq!(corner_spot("top_right"), Some(vec2(1.0, 0.0)));
        assert_eq!(corner_spot("bottom_left"), Some(vec2(0.0, 1.0)));
        assert_eq!(corner_spot("centre"), Some(vec2(0.5, 0.5)));
        assert_eq!(corner_spot("middle_left"), None);
        let options: Vec<Choice> = ["bottom_left", "top_right", "centre"]
            .map(|v| Choice {
                value: v.into(),
                label: v.into(),
                help: None,
            })
            .into();
        let nearest = |x, y| nearest_corner(&options, vec2(x, y)).map(|c| c.value.as_str());
        assert_eq!(nearest(0.9, 0.2), Some("top_right"));
        assert_eq!(nearest(0.4, 0.6), Some("centre"));
        assert_eq!(nearest(0.1, 0.8), Some("bottom_left"));
    }

    #[test]
    fn orientate_3d_model_is_in_aspires_sections() {
        let f = fields(&schema("orientate_3d_model"));
        let sections: Vec<&str> = f
            .iter()
            .filter(|f| f.section)
            .map(|f| f.label.as_str())
            .collect();
        assert_eq!(
            sections,
            [
                "Initial Orientation",
                "Model Size",
                "Position relative to Modeling Plane"
            ]
        );
        let Kind::Group(orientation) = &field(&f, "orientation").kind else {
            panic!("a group")
        };
        assert!(matches!(
            &field(orientation, "face").kind,
            Kind::Choice(faces, Widget::Faces { picture: Some(p) })
                if faces.len() == 6 && p == "import/initial_orientation.png"
        ));
        assert!(matches!(
            &field(orientation, "rotation").kind,
            Kind::Number { steps, .. } if *steps == [-180.0, -90.0, 0.0, 90.0, 180.0]
        ));
    }

    #[test]
    fn face_labels_stay_on_one_line() {
        let options: Vec<Choice> = ["top", "bottom", "front", "back", "left", "right"]
            .iter()
            .map(|v| Choice {
                value: (*v).into(),
                label: crate::gui::title_case(v),
                help: None,
            })
            .collect();
        let ctx = egui::Context::default();
        // bigger text, as the app's readable style has
        ctx.all_styles_mut(|s| {
            for font in s.text_styles.values_mut() {
                font.size *= 1.15;
            }
        });
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| faces(ui, &options, "top", None));
        });
        // no renderer takes a headless frame's texture changes
        out.textures_delta.clear();
        let mut labels = 0;
        for clipped in &out.shapes {
            if let egui::Shape::Text(t) = &clipped.shape {
                assert_eq!(t.galley.rows.len(), 1, "{:?} wraps", t.galley.text());
                labels += 1;
            }
        }
        assert_eq!(labels, 6);
    }

    #[test]
    fn faces_sit_round_the_picture_as_in_aspire() {
        // columns left, middle, right; rows above, upper, lower, below the picture
        assert_eq!(face_spot("top"), Some((1, 0)));
        assert_eq!(face_spot("back"), Some((0, 1)));
        assert_eq!(face_spot("left"), Some((0, 2)));
        assert_eq!(face_spot("right"), Some((2, 1)));
        assert_eq!(face_spot("front"), Some((2, 2)));
        assert_eq!(face_spot("bottom"), Some((1, 3)));
        assert_eq!(face_spot("centre"), None);
    }

    #[test]
    fn steps_are_labelled_with_their_sign() {
        assert_eq!(step_label(-90.0), "-90");
        assert_eq!(step_label(0.0), "0");
        assert_eq!(step_label(180.0), "+180");
        assert_eq!(step_label(22.5), "+22.5");
    }

    #[test]
    fn section_pictures_follow_the_choice_made() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "z_zero": { "enum": ["material_surface", "machine_bed"], "x-icon": "job_setup/z_zero_{}.png" }
            }
        });
        assert_eq!(
            field(&fields(&schema), "z_zero").icon.as_deref(),
            Some("job_setup/z_zero_{}.png")
        );
        let pick = |template, value| icon_path(template, &value);
        assert_eq!(
            pick("job_setup/z_zero_{}.png", serde_json::json!("machine_bed")),
            "job_setup/z_zero_machine_bed.png"
        );
        assert_eq!(
            pick(
                "job_setup/xy_datum_{position}.png",
                serde_json::json!({ "position": "centre" })
            ),
            "job_setup/xy_datum_centre.png"
        );
        assert_eq!(
            pick(
                "job_setup/material_size_xyz.png",
                serde_json::json!({ "width": 1 })
            ),
            "job_setup/material_size_xyz.png"
        );
    }

    #[test]
    fn lengths_and_feeds_show_in_the_units_chosen() {
        assert_eq!(shown_in("mm", Units::Inch), Some((25.4, "in")));
        assert_eq!(shown_in("mm/min", Units::Inch), Some((25.4, "in/min")));
        assert_eq!(shown_in("mm", Units::Mm), None);
        assert_eq!(shown_in("rpm", Units::Inch), None);
        let job_setup = serde_json::json!({ "size": { "width": 10.0, "units": "Inch" } });
        assert_eq!(units_chosen(&job_setup), Some(Units::Inch));
        assert_eq!(units_chosen(&serde_json::json!({ "width": 1.0 })), None);
    }

    #[test]
    fn fields_keep_declaration_order() {
        let f = fields(&schema("draw_rectangle"));
        assert_eq!(keys(&f), ["x", "y", "width", "height", "corners", "radius"]);
    }

    #[test]
    fn lengths_are_numbers_in_mm_with_their_limits() {
        let f = fields(&schema("draw_rectangle"));
        assert_eq!(
            field(&f, "width").kind,
            Kind::Number {
                unit: Some("mm".into()),
                min: Some(0.0),
                max: None,
                integer: false,
                steps: vec![]
            }
        );
        assert_eq!(field(&f, "x").help.as_deref(), Some("Left edge"));
    }

    #[test]
    fn enums_become_choices_with_readable_labels() {
        let f = fields(&schema("draw_rectangle"));
        let Kind::Choice(options, _) = &field(&f, "corners").kind else {
            panic!("corners should be a choice")
        };
        let labels: Vec<&str> = options.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(labels, ["Square", "Rounded", "Notched"]);
        assert_eq!(options[2].value, "notched");
    }

    #[test]
    fn shared_point_params_become_a_group_of_x_and_y() {
        let f = fields(&schema("draw_circle"));
        let Kind::Group(inner) = &field(&f, "center").kind else {
            panic!("center should be a group")
        };
        assert_eq!(keys(inner), ["x", "y"]);
    }

    #[test]
    fn counts_are_integers_with_their_range() {
        let f = fields(&schema("draw_polygon"));
        assert_eq!(
            field(&f, "sides").kind,
            Kind::Number {
                unit: None,
                min: Some(3.0),
                max: Some(100.0),
                integer: true,
                steps: vec![]
            }
        );
    }

    #[test]
    fn lists_of_points_and_flags() {
        let f = fields(&schema("draw_polyline"));
        assert!(
            matches!(&field(&f, "points").kind, Kind::List(item) if matches!(**item, Kind::Group(_)))
        );
        assert_eq!(field(&f, "closed").kind, Kind::Bool);
    }

    #[test]
    fn labels_are_sentence_case() {
        let f = fields(&schema("draw_star"));
        assert_eq!(field(&f, "outer_radius").label, "Outer radius");
    }
}
