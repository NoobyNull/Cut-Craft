//! Settings forms generated from a command's parameter schema, so no command writes UI.

use eframe::egui;
use serde_json::Value;

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
    /// A nested object, e.g. a point.
    Group(Vec<Field>),
    /// A list whose items all have this shape.
    List(Box<Kind>),
    /// Anything the form cannot edit yet; shown read-only.
    Other,
}

#[derive(Debug, PartialEq)]
pub struct Choice {
    pub value: String,
    pub label: String,
}

/// Edits `params` through its fields; true when anything changed.
pub fn show(ui: &mut egui::Ui, fields: &[Field], params: &mut Value) -> bool {
    let mut changed = false;
    egui::Grid::new(ui.next_auto_id())
        .num_columns(2)
        .show(ui, |ui| {
            for f in fields {
                let label = ui.label(&f.label);
                if let Some(help) = &f.help {
                    label.on_hover_text(help);
                }
                changed |= edit(ui, &f.kind, &mut params[&f.key]);
                ui.end_row();
            }
        });
    changed
}

fn edit(ui: &mut egui::Ui, kind: &Kind, value: &mut Value) -> bool {
    match kind {
        Kind::Number {
            unit,
            min,
            max,
            integer,
        } => {
            let mut n = value.as_f64().unwrap_or_default();
            let mut drag = egui::DragValue::new(&mut n)
                .clamp_range(min.unwrap_or(f64::NEG_INFINITY)..=max.unwrap_or(f64::INFINITY));
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
                    Value::from(n)
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
        Kind::Choice(options) => {
            let current = value.as_str().unwrap_or_default().to_owned();
            let shown = options
                .iter()
                .find(|o| o.value == current)
                .map_or(current.clone(), |o| o.label.clone());
            let mut changed = false;
            egui::ComboBox::from_id_source(ui.next_auto_id())
                .selected_text(shown)
                .show_ui(ui, |ui| {
                    for o in options {
                        if ui.selectable_label(o.value == current, &o.label).clicked() {
                            *value = Value::String(o.value.clone());
                            changed = true;
                        }
                    }
                });
            changed
        }
        Kind::Group(inner) => ui.vertical(|ui| show(ui, inner, value)).inner,
        Kind::List(item) => ui.vertical(|ui| edit_list(ui, item, value)).inner,
        Kind::Other => {
            ui.label(value.to_string());
            false
        }
    }
}

fn edit_list(ui: &mut egui::Ui, item: &Kind, value: &mut Value) -> bool {
    let Some(items) = value.as_array_mut() else {
        return false;
    };
    let mut changed = false;
    let mut remove = None;
    for (i, it) in items.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            changed |= edit(ui, item, it);
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
            label: sentence_case(key),
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

fn kind(root: &Value, s: &Value) -> Kind {
    let s = resolve(root, s);
    if let Some(options) = choices(s) {
        return Kind::Choice(options);
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

/// String enums: schemars writes `enum: [...]`, or `oneOf` of single values when variants are documented.
fn choices(s: &Value) -> Option<Vec<Choice>> {
    let values: Vec<&str> = match s["enum"].as_array() {
        Some(values) => values.iter().map(Value::as_str).collect::<Option<_>>()?,
        None => s["oneOf"]
            .as_array()?
            .iter()
            .map(|o| o["const"].as_str().or_else(|| o["enum"][0].as_str()))
            .collect::<Option<_>>()?,
    };
    Some(
        values
            .into_iter()
            .map(|v| Choice {
                value: v.into(),
                label: sentence_case(v),
            })
            .collect(),
    )
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
