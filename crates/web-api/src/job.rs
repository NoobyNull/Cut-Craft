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
