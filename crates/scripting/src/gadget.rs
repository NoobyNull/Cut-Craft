//! A gadget file as a command: named, filed under Gadgets, with a settings form built from the
//! `settings` list it declares.

use crate::{Limits, ScriptError, describe, run_gadget};
use cut_craft_core::document::Document;
use cut_craft_core::geometry::Path;
use cut_craft_core::{Change, CommandError, Drag, DynCommand, Meta, Registry};
use serde_json::{Map, Value, json};
use std::sync::Arc;

/// A loaded gadget, ready to join the [`Registry`] with [`Registry::add_dyn`].
pub struct GadgetCommand {
    id: &'static str,
    meta: Meta,
    schema: Value,
    defaults: Value,
    file_name: String,
    source: String,
    /// The commands the gadget may run (gadgets cannot run other gadgets).
    commands: Arc<Registry>,
}

/// Reads the gadget in `source` (from the file `file_name`), checking its `settings`, so a
/// mistake is reported when the gadget is loaded rather than when someone runs it.
pub fn load_gadget(
    file_name: &str,
    source: &str,
    commands: Arc<Registry>,
) -> Result<GadgetCommand, ScriptError> {
    let (about, settings) = describe(file_name, source)?;
    let stem = file_name.strip_suffix(".lua").unwrap_or(file_name);
    let label = about["name"]
        .as_str()
        .map_or_else(|| title_case(stem), str::to_owned);
    let (mut schema, defaults) = settings_schema(&settings).map_err(ScriptError::Syntax)?;
    if let Some(description) = about["description"].as_str() {
        schema["description"] = description.into();
    }
    // a gadget is loaded once, when the app starts, and lives as long as the app
    let id: &'static str = Box::leak(format!("gadget_{}", slug(stem)).into_boxed_str());
    let label: &'static str = Box::leak(label.into_boxed_str());
    Ok(GadgetCommand {
        id,
        meta: Meta {
            label,
            group: "Gadgets",
            icon: "",
            shortcut: "",
        },
        schema,
        defaults,
        file_name: file_name.to_owned(),
        source: source.to_owned(),
        commands,
    })
}

/// The JSON schema of the form, and the settings it starts with, from a gadget's `settings`
/// list: `{ key = "count", label = "...", type = "integer", default = 6, min = 1, max = 99,
/// help = "..." }`, with types `integer`, `number`, `length` (in mm), `boolean`, `text` and
/// `choice` (with `choices = { ... }`).
fn settings_schema(settings: &Value) -> Result<(Value, Value), String> {
    let list = match settings {
        Value::Null => &Vec::new(),
        Value::Array(list) => list,
        Value::Object(o) if o.is_empty() => &Vec::new(),
        _ => return Err("`settings` must be a list of { key = ..., type = ... } entries".into()),
    };
    let (mut properties, mut defaults) = (Map::new(), Map::new());
    for (n, s) in list.iter().enumerate() {
        let key = s["key"]
            .as_str()
            .ok_or_else(|| format!("setting {} has no `key`", n + 1))?;
        let kind = s["type"].as_str().unwrap_or("number");
        let mut property = match kind {
            "integer" => json!({ "type": "integer" }),
            "number" => json!({ "type": "number" }),
            "length" => json!({ "type": "number", "x-unit": "mm" }),
            "boolean" => json!({ "type": "boolean" }),
            "text" => json!({ "type": "string" }),
            "choice" => {
                let choices = s["choices"]
                    .as_array()
                    .filter(|c| !c.is_empty() && c.iter().all(Value::is_string))
                    .ok_or_else(|| {
                        format!("setting \"{key}\" is a choice but has no `choices` list")
                    })?;
                json!({ "type": "string", "enum": choices })
            }
            other => {
                return Err(format!(
                    "setting \"{key}\" has type \"{other}\"; use integer, number, length, \
                     boolean, text or choice"
                ));
            }
        };
        property["title"] = s["label"]
            .as_str()
            .map_or_else(|| title_case(key), str::to_owned)
            .into();
        if let Some(help) = s["help"].as_str() {
            property["description"] = help.into();
        }
        if let Some(min) = s["min"].as_f64() {
            property["minimum"] = min.into();
        }
        if let Some(max) = s["max"].as_f64() {
            property["maximum"] = max.into();
        }
        let default = match &s["default"] {
            Value::Null => match kind {
                // zero, unless the setting's range leaves zero out
                "integer" => json!(whole(in_range(s).ceil())),
                "number" | "length" => json!(in_range(s)),
                "boolean" => json!(false),
                "choice" => property["enum"][0].clone(),
                _ => json!(""),
            },
            given => given.clone(),
        };
        property["default"] = default.clone();
        properties.insert(key.to_owned(), property);
        defaults.insert(key.to_owned(), default);
    }
    Ok((
        json!({ "type": "object", "properties": properties }),
        Value::Object(defaults),
    ))
}

/// Zero, or the end of the setting's `min`..`max` range nearest it.
fn in_range(setting: &Value) -> f64 {
    let min = setting["min"].as_f64().unwrap_or(f64::NEG_INFINITY);
    let max = setting["max"].as_f64().unwrap_or(f64::INFINITY);
    0.0_f64.max(min).min(max)
}

/// A whole number setting's starting value, which a gadget's range keeps small.
#[expect(
    clippy::cast_possible_truncation,
    reason = "saturates; a form setting's range is nowhere near i64's"
)]
fn whole(value: f64) -> i64 {
    value as i64
}

/// `hole-pattern` → `Hole Pattern`.
fn title_case(name: &str) -> String {
    name.split(['-', '_', ' '])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map(|f| f.to_uppercase().chain(c).collect::<String>())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// `Hole-Pattern 2` → `hole_pattern_2`: an id made only of what command ids are made of.
fn slug(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

impl DynCommand for GadgetCommand {
    fn id(&self) -> &'static str {
        self.id
    }

    fn meta(&self) -> Meta {
        self.meta
    }

    fn schema(&self) -> Value {
        self.schema.clone()
    }

    fn default_params(&self) -> Value {
        self.defaults.clone()
    }

    fn initial_params(&self, _doc: &Document) -> Value {
        self.defaults.clone()
    }

    fn params_from(&self, _path: &Path, _current: &Value) -> Option<Value> {
        None
    }

    fn run_json(&self, doc: &Document, params: &Value) -> Result<Change, CommandError> {
        run_gadget(
            &self.file_name,
            &self.source,
            doc,
            &self.commands,
            params,
            Limits::default(),
        )
        .map_err(|e| CommandError::Failed(e.to_string()))
    }

    fn interactive(&self) -> bool {
        false
    }

    fn drag_params(&self, _drag: Drag, _current: &Value) -> Result<Value, CommandError> {
        Err(CommandError::Failed(format!(
            "{} cannot be drawn with the mouse",
            self.meta.label
        )))
    }
}
