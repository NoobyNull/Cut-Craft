//! What `GET job`, `GET commands` and `POST select` read from the job and the tools, kept
//! here (not in the app) so they are tested without a window.

use crate::{ApiError, Pick};
use cut_craft_core::Registry;
use cut_craft_core::document::{Document, EntityId, Units};
use cut_craft_core::geometry::{Point, Rect};
use serde_json::{Value, json};

const MM_PER_INCH: f64 = 25.4;

/// The job's units as calls name them.
pub fn units_name(units: Units) -> &'static str {
    match units {
        Units::Mm => "mm",
        Units::Inch => "in",
    }
}

/// Millimetres per unit for a call's `units` ("mm" or "in"); the job's when `None`.
fn scale(units: Option<&str>, job: Units) -> Result<f64, ApiError> {
    match units.map(str::to_ascii_lowercase).as_deref() {
        Some("mm") => Ok(1.0),
        Some("in" | "inch" | "inches") => Ok(MM_PER_INCH),
        None => Ok(if job == Units::Inch { MM_PER_INCH } else { 1.0 }),
        Some(other) => Err(ApiError::bad(format!(
            "\"units\" is \"{other}\": use \"mm\" or \"in\"."
        ))),
    }
}

/// `value` in millimetres, read in the job's units, rounded to a thousandth of them.
fn shown(mm: f64, job: Units) -> f64 {
    let unit = if job == Units::Inch { MM_PER_INCH } else { 1.0 };
    (mm / unit * 1000.0).round() / 1000.0
}

/// `GET job`: the layers, every vector (its layer, part name, size and whether it is closed),
/// the toolpaths and the selection. Lengths are in the job's units.
pub fn summary(doc: &Document) -> Value {
    let u = doc.job.units;
    let layers: Vec<Value> = doc
        .layers
        .iter()
        .map(|l| {
            json!({
                "id": l.id.0,
                "name": l.name,
                "visible": l.visible,
                "locked": l.locked,
                "current": l.id == doc.current_layer,
            })
        })
        .collect();
    let vectors: Vec<Value> = doc
        .vectors
        .iter()
        .map(|(&id, path)| {
            let b = path.bounds();
            let layer = doc.layer(doc.layer_of(id)).map(|l| l.name.as_str());
            json!({
                "id": id.0,
                "layer": layer,
                "name": part_name(doc, id),
                "closed": path.closed,
                "selected": doc.selection.contains(&id),
                "min": [shown(b.min.x, u), shown(b.min.y, u)],
                "max": [shown(b.max.x, u), shown(b.max.y, u)],
            })
        })
        .collect();
    let toolpaths: Vec<Value> = doc
        .toolpaths
        .iter()
        .map(|(&id, t)| {
            json!({
                "id": id.0,
                "name": t.name,
                "command": t.strategy,
                "tool": t.tool.name,
                "visible": t.visible,
                "out_of_date": t.stale,
                "vectors": t.vectors.iter().map(|v| v.0).collect::<Vec<_>>(),
            })
        })
        .collect();
    json!({
        "units": units_name(u),
        "material": {
            "width": shown(doc.job.width, u),
            "height": shown(doc.job.height, u),
            "thickness": shown(doc.job.thickness, u),
        },
        "layers": layers,
        "vectors": vectors,
        "selection": doc.selection.iter().map(|v| v.0).collect::<Vec<_>>(),
        "toolpaths": toolpaths,
    })
}

/// The name of the part vector `id` belongs to, if it was named.
fn part_name(doc: &Document, id: EntityId) -> Option<&str> {
    doc.part_name(id)
}

/// The vectors `pick` names, in the order picked: only those on visible, unlocked layers, as
/// a click would. Names that match nothing are an error that says what there is.
pub fn pick(doc: &Document, pick: &Pick) -> Result<Vec<EntityId>, ApiError> {
    let pickable = || doc.vectors.keys().copied().filter(|&id| doc.editable(id));
    let picked: Vec<EntityId> = match pick {
        Pick::Ids { ids } => {
            let ids: Vec<EntityId> = ids.iter().copied().map(EntityId).collect();
            if let Some(missing) = ids.iter().find(|id| !doc.vectors.contains_key(id)) {
                return Err(ApiError::not_found(format!(
                    "There is no vector {}. GET {}job lists the vectors.",
                    missing.0,
                    crate::PREFIX
                )));
            }
            if let Some(locked) = ids.iter().find(|&&id| !doc.editable(id)) {
                return Err(ApiError::refused(format!(
                    "Vector {} is on a hidden or locked layer. Show or unlock its layer first.",
                    locked.0
                )));
            }
            ids
        }
        Pick::Layer { layer } => {
            let Some(found) = doc
                .layers
                .iter()
                .find(|l| l.name.eq_ignore_ascii_case(layer.trim()))
            else {
                let names: Vec<&str> = doc.layers.iter().map(|l| l.name.as_str()).collect();
                return Err(ApiError::not_found(format!(
                    "There is no layer called \"{layer}\". The layers are: {}.",
                    names.join(", ")
                )));
            };
            pickable()
                .filter(|&id| doc.layer_of(id) == found.id)
                .collect()
        }
        Pick::Name { name } => {
            let named: Vec<EntityId> = pickable()
                .filter(|&id| {
                    part_name(doc, id).is_some_and(|n| n.eq_ignore_ascii_case(name.trim()))
                })
                .collect();
            if named.is_empty() {
                return Err(ApiError::not_found(format!(
                    "No part is called \"{name}\". Name parts with Name Part first."
                )));
            }
            named
        }
        Pick::Inside {
            x1,
            y1,
            x2,
            y2,
            units,
        } => {
            let k = scale(units.as_deref(), doc.job.units)?;
            let area = Rect::new(Point::new(x1 * k, y1 * k), Point::new(x2 * k, y2 * k));
            pickable()
                .filter(|&id| area.contains_rect(doc.vectors[&id].bounds()))
                .collect()
        }
        Pick::All => pickable().collect(),
        Pick::Open => pickable().filter(|id| !doc.vectors[id].closed).collect(),
        Pick::Closed => pickable().filter(|id| doc.vectors[id].closed).collect(),
        Pick::None => vec![],
    };
    Ok(picked)
}

/// `GET commands`: every tool, by id, with its name and where it is in the app.
pub fn commands(registry: &Registry) -> Value {
    let list: Vec<Value> = registry
        .commands()
        .map(|c| {
            let m = c.meta();
            json!({ "id": c.id(), "name": m.label, "group": m.group })
        })
        .collect();
    Value::Array(list)
}

/// `GET commands/<id>`: a tool's settings: the schema of what `POST run` takes, and what its
/// form would start with on this job.
pub fn command(registry: &Registry, doc: &Document, id: &str) -> Result<Value, ApiError> {
    let c = registry.get(id).ok_or_else(|| unknown_command(id))?;
    let m = c.meta();
    Ok(json!({
        "id": c.id(),
        "name": m.label,
        "group": m.group,
        "settings": c.schema(),
        "starts_with": c.initial_params(doc),
    }))
}

/// The error for a tool id that is not one.
pub fn unknown_command(id: &str) -> ApiError {
    ApiError::not_found(format!(
        "There is no tool \"{id}\". GET {}commands lists every tool's id.",
        crate::PREFIX
    ))
}

/// What `POST run` runs: the tool's starting settings with the caller's laid over them, so a
/// call need only give the settings it changes.
pub fn run_params(
    registry: &Registry,
    doc: &Document,
    id: &str,
    given: &Value,
) -> Result<Value, ApiError> {
    let c = registry.get(id).ok_or_else(|| unknown_command(id))?;
    let mut params = c.initial_params(doc);
    // a misspelt setting would be dropped without a word, and the tool run as it starts
    let schema = c.schema();
    let named = |key: &str| schema.get("properties").and_then(|p| p.get(key)).is_some();
    unknown_settings(&params, given, "", &named)?;
    overlay(&mut params, given);
    Ok(params)
}

/// An error naming the first setting in `given` that `base` (or, at the top, the schema)
/// does not have, with the names it does have.
fn unknown_settings(
    base: &Value,
    given: &Value,
    path: &str,
    named: &dyn Fn(&str) -> bool,
) -> Result<(), ApiError> {
    let (Value::Object(base), Value::Object(given)) = (base, given) else {
        return Ok(());
    };
    for (key, value) in given {
        let here = format!("{path}{key}");
        match base.get(key) {
            Some(inner) => unknown_settings(inner, value, &format!("{here}."), &|_| false)?,
            None if path.is_empty() && named(key) => {}
            None => {
                let known: Vec<&str> = base.keys().map(String::as_str).collect();
                return Err(ApiError::bad(format!(
                    "There is no setting \"{here}\". The settings{} are: {}.",
                    if path.is_empty() {
                        String::new()
                    } else {
                        format!(" in \"{}\"", path.trim_end_matches('.'))
                    },
                    known.join(", ")
                )));
            }
        }
    }
    Ok(())
}

/// `top`'s fields laid over `base`'s, object into object.
fn overlay(base: &mut Value, top: &Value) {
    match (base, top) {
        (Value::Object(base), Value::Object(top)) => {
            for (k, v) in top {
                match base.get_mut(k) {
                    Some(b) if b.is_object() && v.is_object() => overlay(b, v),
                    _ => {
                        base.insert(k.clone(), v.clone());
                    }
                }
            }
        }
        (base, top) => *base = top.clone(),
    }
}

/// A name as calls write it: the setting's own serialised name (`material_surface`).
fn named(value: impl serde::Serialize) -> Value {
    let v = serde_json::to_value(value).unwrap_or(Value::Null);
    match v {
        Value::String(s) => Value::String(snake(&s)),
        other => other,
    }
}

/// `MaterialSurface` as `material_surface`; already snake case is left as it is.
fn snake(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// `GET notes`: the notes as typed (`text`, the open mark left off), whether they open with
/// the job and go at the top of the G-code, the notes with `variables` filled in as plain
/// text and as HTML, and every variable with its value now.
pub fn notes(doc: &Document, variables: &[cut_craft_core::notes::Variable]) -> Value {
    use cut_craft_core::notes;
    json!({
        "text": notes::strip_open_mark(&doc.notes),
        "opens_with_job": notes::opens_with_job(&doc.notes),
        "in_gcode": doc.notes_in_gcode,
        "plain": notes::plain(&doc.notes, variables),
        "html": notes::html(&doc.notes, variables),
        "variables": variables,
    })
}

/// The `notes` command's settings for `POST notes`: what `body` gives, the rest as the job
/// has it now. Notes kept with a "." first open with the job, so text starting with one
/// cannot be kept without opening: that is refused, saying so.
pub fn notes_params(doc: &Document, body: &crate::call::NotesBody) -> Result<Value, ApiError> {
    use cut_craft_core::notes;
    let text = body
        .text
        .clone()
        .unwrap_or_else(|| notes::strip_open_mark(&doc.notes).to_owned());
    let opens = body
        .opens_with_job
        .unwrap_or_else(|| notes::opens_with_job(&doc.notes));
    if !opens && text.starts_with(notes::OPEN_MARK) {
        return Err(ApiError::bad(
            "Notes starting with \".\" open with the job. Send \"opens_with_job\": true, or \
             start the text with something else.",
        ));
    }
    let mark = if opens { "." } else { "" };
    Ok(json!({
        "notes": format!("{mark}{text}"),
        "in_gcode": body.in_gcode.unwrap_or(doc.notes_in_gcode),
    }))
}

/// `GET state`: the material as Job Setup has it, where the 3D model sits in it (Material
/// Setup) and each part of the model, and what is picked: vectors, model parts and the
/// toolpath picked in the toolpath list (`toolpath`). Lengths are in the job's units.
pub fn state(doc: &Document, toolpath: Option<EntityId>) -> Value {
    let u = doc.job.units;
    let j = &doc.job;
    // the model's placement as it is kept, its gaps shown in the job's units: whatever gaps
    // Material Setup keeps are read the same way
    let mut position = serde_json::to_value(j.model_position).unwrap_or(Value::Null);
    if let Value::Object(fields) = &mut position {
        for (key, value) in fields.iter_mut() {
            if key.contains("gap")
                && let Some(mm) = value.as_f64()
            {
                *value = json!(shown(mm, u));
            }
        }
    }
    let components: Vec<Value> = doc
        .components
        .iter()
        .map(|(&id, c)| {
            #[expect(clippy::cast_precision_loss, reason = "a grid's size")]
            let (w, h) = (c.columns as f64 * c.cell, c.rows as f64 * c.cell);
            let range = c.height_range();
            json!({
                "id": id.0,
                "name": c.name,
                "visible": c.visible,
                "selected": doc.selected_components.contains(&id),
                "min": [shown(c.origin.x, u), shown(c.origin.y, u)],
                "max": [shown(c.origin.x + w, u), shown(c.origin.y + h, u)],
                "lowest": range.map(|(lo, _)| shown(lo, u)),
                "highest": range.map(|(_, hi)| shown(hi, u)),
                "height": range.map(|(lo, hi)| shown(hi - lo, u)),
            })
        })
        .collect();
    json!({
        "units": units_name(u),
        "material": {
            "width": shown(j.width, u),
            "height": shown(j.height, u),
            "thickness": shown(j.thickness, u),
            "z_zero": named(j.z_zero),
            "datum": named(j.datum),
            "offset": [shown(j.offset.x, u), shown(j.offset.y, u)],
            "clearance": shown(j.clearance, u),
            "plunge": j.plunge.map(|p| shown(p, u)),
            "colour": j.colour,
            "double_sided": j.double_sided.is_some(),
            "rotary": j.rotary.is_some(),
        },
        "model": {
            "position": position,
            "components": components,
        },
        "selection": {
            "vectors": doc.selection.iter().map(|v| v.0).collect::<Vec<_>>(),
            "components": doc.selected_components.iter().map(|v| v.0).collect::<Vec<_>>(),
            "toolpath": toolpath.map(|t| t.0),
        },
    })
}

/// A run time as the Toolpath Summary says it: `1h 05m 30s`, `4m 10s`, `12s`.
pub fn time_said(minutes: f64) -> String {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a run time, rounded to seconds"
    )]
    let seconds = (minutes.max(0.0) * 60.0).round() as u64;
    let (h, m, s) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    match (h, m) {
        (0, 0) => format!("{s}s"),
        (0, _) => format!("{m}m {s:02}s"),
        _ => format!("{h}h {m:02}m {s:02}s"),
    }
}

/// `GET toolpaths`: each toolpath in list order with its tool, its run time as the
/// Toolpath Summary works it out (`minutes`), and what to look at before cutting it: its
/// own warnings (out of date, entries made another way) and what the checks before cutting
/// found (`checks`), errors apart. The shown ones' times are added up.
pub fn toolpaths(
    doc: &Document,
    minutes: &dyn Fn(&cut_craft_core::toolpath::Toolpath) -> f64,
    checks: &std::collections::BTreeMap<EntityId, Vec<cut_craft_core::preflight::Finding>>,
) -> Value {
    use cut_craft_core::preflight::Severity;
    let mut total = 0.0;
    let rows: Vec<Value> = doc
        .toolpaths
        .iter()
        .map(|(id, t)| {
            let time = minutes(t);
            if t.visible {
                total += time;
            }
            let mut warnings = vec![];
            if t.stale {
                warnings.push(
                    "Out of date: what it was made from has changed since. Recalculate it \
                     before cutting."
                        .to_owned(),
                );
            }
            warnings.extend(fallbacks_said(&t.fallbacks));
            let found = checks.get(id).map_or(&[][..], Vec::as_slice);
            let said = |severity| {
                found
                    .iter()
                    .filter(|f| f.severity == severity)
                    .map(|f| f.message.clone())
                    .collect::<Vec<_>>()
            };
            warnings.extend(said(Severity::Warning));
            json!({
                "id": id.0,
                "name": t.name,
                "command": t.strategy,
                "tool": t.tool.name,
                "group": t.group,
                "visible": t.visible,
                "out_of_date": t.stale,
                "moves": t.moves.len(),
                "minutes": (time * 100.0).round() / 100.0,
                "time": time_said(time),
                "warnings": warnings,
                "errors": said(Severity::Error),
            })
        })
        .collect();
    json!({
        "units": units_name(doc.job.units),
        "toolpaths": rows,
        "total_minutes": (total * 100.0).round() / 100.0,
        "total_time": time_said(total),
    })
}

/// Where a toolpath's entries went down another way than asked, there being no room: one
/// line for each change ("3 entries went down by zig-zag, not helix: no room for it").
fn fallbacks_said(fallbacks: &[cut_craft_core::entry::Fallback]) -> Vec<String> {
    let mut counted: Vec<(&str, &str, usize)> = vec![];
    for f in fallbacks {
        let (wanted, used) = (f.wanted.said(), f.used.said());
        match counted
            .iter_mut()
            .find(|(w, u, _)| *w == wanted && *u == used)
        {
            Some((_, _, n)) => *n += 1,
            None => counted.push((wanted, used, 1)),
        }
    }
    counted
        .into_iter()
        .map(|(wanted, used, n)| {
            let entries = if n == 1 {
                "1 entry".to_owned()
            } else {
                format!("{n} entries")
            };
            format!("{entries} went down by {used}, not {wanted}: there was no room for it")
        })
        .collect()
}
