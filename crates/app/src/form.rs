//! Settings forms generated from a command's parameter schema, so no command writes UI.

use cut_craft_core::document::Units;
use eframe::egui;
use serde_json::Value;

/// Choices with this many options or fewer are radio buttons; more make a drop-down.
const MOST_RADIOS: usize = 5;
/// The corner picker's size, in rows of the theme's interactive height.
// ponytail: fixed ratio until a preferences dialog exists
const CORNER_PICKER_ROWS: f32 = 3.0;

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
}

/// How a form looks: the units lengths are shown in, and the pictures sections show.
pub struct Look<'a> {
    pub units: Units,
    /// A picture by icon path, sized for a section; `None` when the icon is missing.
    pub picture: &'a dyn Fn(&str) -> Option<egui::Image<'static>>,
}

#[derive(Debug, PartialEq)]
pub enum Kind {
    Number {
        unit: Option<String>,
        min: Option<f64>,
        max: Option<f64>,
        integer: bool,
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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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
}

#[derive(Debug, PartialEq)]
pub struct Choice {
    pub value: String,
    pub label: String,
    pub help: Option<String>,
}

/// Edits `params` through its fields; true when anything changed.
pub fn show(ui: &mut egui::Ui, fields: &[Field], params: &mut Value, look: &Look) -> bool {
    let mut changed = false;
    // runs of ordinary fields share a grid; each section is a framed block of its own
    let mut rest = fields;
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
                        Kind::Group(inner) => show(ui, inner, value, look),
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

fn grid(ui: &mut egui::Ui, fields: &[Field], params: &mut Value, look: &Look) -> bool {
    let mut changed = false;
    egui::Grid::new(ui.next_auto_id())
        .num_columns(2)
        .show(ui, |ui| {
            for f in fields {
                hover(ui.label(&f.label), f.help.as_ref());
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
        } => {
            let (factor, unit) = match unit.as_deref().map(|u| (u, shown_in(u, look.units))) {
                Some((_, Some((factor, shown)))) => (factor, Some(shown.to_owned())),
                Some((stored, None)) => (1.0, Some(stored.to_owned())),
                None => (1.0, None),
            };
            let mut n = value.as_f64().unwrap_or_default() / factor;
            let mut drag = egui::DragValue::new(&mut n).clamp_range(
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
        Kind::Choice(options, widget) => choose(ui, options, *widget, value),
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

fn choose(ui: &mut egui::Ui, options: &[Choice], widget: Widget, value: &mut Value) -> bool {
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
    egui::ComboBox::from_id_source(ui.next_auto_id())
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
    painter.rect_stroke(square, 0.0, visuals.widgets.inactive.fg_stroke);
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
            Kind::Choice(_, widget) => *widget,
            other => panic!("{key} is {other:?}"),
        };
        assert_eq!(widget("units"), Widget::Row);
        assert_eq!(widget("resolution"), Widget::DropDown);
        assert_eq!(widget("datum"), Widget::Corners);
        assert_eq!(widget("side"), Widget::Auto);
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
    fn section_pictures_follow_the_choice_made() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "z_zero": { "enum": ["material_surface", "machine_bed"], "x-icon": "job_setup/z_zero_{}.svg" }
            }
        });
        assert_eq!(
            field(&fields(&schema), "z_zero").icon.as_deref(),
            Some("job_setup/z_zero_{}.svg")
        );
        let pick = |template, value| icon_path(template, &value);
        assert_eq!(
            pick("job_setup/z_zero_{}.svg", serde_json::json!("machine_bed")),
            "job_setup/z_zero_machine_bed.svg"
        );
        assert_eq!(
            pick(
                "job_setup/xy_datum_{position}.svg",
                serde_json::json!({ "position": "centre" })
            ),
            "job_setup/xy_datum_centre.svg"
        );
        assert_eq!(
            pick(
                "job_setup/material_size_xyz.svg",
                serde_json::json!({ "width": 1 })
            ),
            "job_setup/material_size_xyz.svg"
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
                integer: false
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
                integer: true
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
