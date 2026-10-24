//! Settings forms generated from a command's parameter schema, so no command writes UI.

use cut_craft_core::document::Units;
use eframe::egui;
use serde_json::Value;

/// Choices with this many options or fewer are radio buttons; more make a drop-down.
// ponytail: fixed threshold; a per-field hint when a form needs otherwise
const MOST_RADIOS: usize = 5;

#[derive(Debug, PartialEq)]
pub struct Field {
    /// JSON key in the params object.
    pub key: String,
    pub label: String,
    /// Doc comment of the parameter, shown on hover.
    pub help: Option<String>,
    pub kind: Kind,
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
    Text,
    Choice(Vec<Choice>),
    /// A nested object, e.g. a point, edited inline.
    Group(Vec<Field>),
    /// A nested object marked `x-section`: a framed block with a heading, as Aspire's forms have.
    Section(Vec<Field>),
    /// An `[r, g, b]` array marked `x-kind: colour`.
    Colour,
    /// A list whose items all have this shape.
    List(Box<Kind>),
    /// Anything the form cannot edit yet; shown read-only.
    Other,
}

#[derive(Debug, PartialEq)]
pub struct Choice {
    pub value: String,
    pub label: String,
    pub help: Option<String>,
}

/// Edits `params` through its fields, lengths shown in `units`; true when anything changed.
pub fn show(ui: &mut egui::Ui, fields: &[Field], params: &mut Value, units: Units) -> bool {
    let mut changed = false;
    // runs of ordinary fields share a grid; each section is a framed block of its own
    let mut rest = fields;
    while !rest.is_empty() {
        let run = rest
            .iter()
            .position(|f| matches!(f.kind, Kind::Section(_)))
            .unwrap_or(rest.len());
        let (plain, after) = rest.split_at(run);
        if !plain.is_empty() {
            changed |= grid(ui, plain, params, units);
        }
        if let Some((section, after)) = after.split_first() {
            if let Kind::Section(inner) = &section.kind {
                changed |= ui
                    .group(|ui| {
                        ui.set_width(ui.available_width());
                        hover(ui.strong(&section.label), section.help.as_ref());
                        show(ui, inner, &mut params[&section.key], units)
                    })
                    .inner;
            }
            rest = after;
        } else {
            rest = after;
        }
    }
    changed
}

fn grid(ui: &mut egui::Ui, fields: &[Field], params: &mut Value, units: Units) -> bool {
    let mut changed = false;
    egui::Grid::new(ui.next_auto_id())
        .num_columns(2)
        .show(ui, |ui| {
            for f in fields {
                hover(ui.label(&f.label), f.help.as_ref());
                changed |= edit(ui, &f.kind, &mut params[&f.key], units);
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

fn edit(ui: &mut egui::Ui, kind: &Kind, value: &mut Value, units: Units) -> bool {
    match kind {
        Kind::Number {
            unit,
            min,
            max,
            integer,
        } => {
            let (factor, unit) = match unit.as_deref().map(|u| (u, shown_in(u, units))) {
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
        Kind::Text => {
            let mut s = value.as_str().unwrap_or_default().to_owned();
            let changed = ui.text_edit_singleline(&mut s).changed();
            if changed {
                *value = Value::String(s);
            }
            changed
        }
        Kind::Choice(options) => choose(ui, options, value),
        Kind::Group(inner) | Kind::Section(inner) => {
            ui.vertical(|ui| show(ui, inner, value, units)).inner
        }
        Kind::List(item) => ui.vertical(|ui| edit_list(ui, item, value, units)).inner,
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

/// Radio buttons for a few options, a drop-down for more.
fn choose(ui: &mut egui::Ui, options: &[Choice], value: &mut Value) -> bool {
    let current = value.as_str().unwrap_or_default().to_owned();
    let mut picked = None;
    if options.len() <= MOST_RADIOS {
        ui.horizontal_wrapped(|ui| {
            for o in options {
                let radio = ui.radio(o.value == current, &o.label);
                if radio.clicked() {
                    picked = Some(o.value.clone());
                }
                hover(radio, o.help.as_ref());
            }
        });
    } else {
        let shown = options
            .iter()
            .find(|o| o.value == current)
            .map_or(current.clone(), |o| o.label.clone());
        egui::ComboBox::from_id_source(ui.next_auto_id())
            .selected_text(shown)
            .show_ui(ui, |ui| {
                for o in options {
                    if ui.selectable_label(o.value == current, &o.label).clicked() {
                        picked = Some(o.value.clone());
                    }
                }
            });
    }
    match picked {
        Some(v) if v != current => {
            *value = Value::String(v);
            true
        }
        _ => false,
    }
}

fn edit_list(ui: &mut egui::Ui, item: &Kind, value: &mut Value, units: Units) -> bool {
    let Some(items) = value.as_array_mut() else {
        return false;
    };
    let mut changed = false;
    let mut remove = None;
    for (i, it) in items.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            changed |= edit(ui, item, it, units);
            if ui.small_button("✕").on_hover_text("Remove").clicked() {
                remove = Some(i);
            }
        });
    }
    if let Some(i) = remove {
        items.remove(i);
        changed = true;
    }
    if ui.small_button("+").on_hover_text("Add another").clicked() {
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
        return Kind::Choice(options);
    }
    if s["type"] == "object" && marked("x-section") == Some(&Value::Bool(true)) {
        return Kind::Section(object_fields(root, s));
    }
    match s["type"].as_str() {
        Some(t @ ("number" | "integer")) => Kind::Number {
            unit: s["x-unit"].as_str().map(String::from),
            min: s["minimum"].as_f64(),
            max: s["maximum"].as_f64(),
            integer: t == "integer",
        },
        Some("boolean") => Kind::Bool,
        Some("string") => Kind::Text,
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
        let Kind::Choice(options) = &field(&f, "z_zero").kind else {
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
        assert!(matches!(&field(&f, "size").kind, Kind::Section(inner) if inner.len() == 1));
        assert!(
            matches!(&field(&f, "at").kind, Kind::Group(_)),
            "unmarked groups stay inline"
        );
        assert_eq!(field(&f, "colour").kind, Kind::Colour);
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
        let Kind::Choice(options) = &field(&f, "corners").kind else {
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
