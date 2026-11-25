//! Old jobs open into the rotary toolpaths that took over from 4-Axis Rotary Machining's
//! strategies: Radial and Off centre become a Rotary Finishing, Indexed a Rotary Locked
//! Angles. Sides then details stays as it is: it still works, and no one new toolpath does
//! all of it. The moves saved with the toolpath are kept as they are, so the job cuts the
//! same until it is recalculated.

use crate::document::Document;
use serde_json::{Map, Value, json};

/// The toolpath that 4-Axis Rotary Machining's strategies came from.
const OLD: &str = "rotary_machining_toolpath";

/// The settings both kinds of toolpath have, kept as they were.
const KEPT: [&str; 11] = [
    "name",
    "tool",
    "start",
    "end",
    "allowance",
    "machining_boundary",
    "boundary_level",
    "machining_boundary_offset",
    "surroundings",
    "registration_mark",
    "cut_support",
];

/// `doc`'s old 4-Axis Rotary Machining toolpaths moved into the toolpaths that took over
/// their strategies; how many were.
pub fn upgrade(doc: &mut Document) -> usize {
    let mut moved = 0;
    for toolpath in doc.toolpaths.values_mut() {
        if toolpath.strategy != OLD {
            continue;
        }
        let Some((strategy, params)) = taken_over(&toolpath.params) else {
            continue;
        };
        toolpath.strategy = strategy.to_owned();
        toolpath.params = params;
        moved += 1;
    }
    moved
}

/// The toolpath that took over `old`'s strategy, with its settings: none for Sides then
/// details.
fn taken_over(old: &Value) -> Option<(&'static str, Value)> {
    let way = old
        .get("strategy")
        .and_then(Value::as_str)
        .unwrap_or("radial");
    let mut params: Map<String, Value> = KEPT
        .iter()
        .filter_map(|&key| Some((key.to_owned(), old.get(key)?.clone())))
        .collect();
    // what the old toolpath kept for its notes and its preview stays with it
    for key in crate::toolpath::OWN_PARAMS {
        if let Some(kept) = old.get(key) {
            params.insert(key.to_owned(), kept.clone());
        }
    }
    let strategy = match way {
        "radial" | "off_centre" => {
            params.insert("way".into(), json!("round_the_part"));
            params.insert("cut_direction".into(), json!("both"));
            if way == "off_centre" {
                params.insert("held".into(), json!("off_centre"));
                params.insert(
                    "offset".into(),
                    old.get("offset").cloned().unwrap_or(json!(3.0)),
                );
                params.insert("sides".into(), json!("one_side"));
            } else {
                params.insert("held".into(), json!("over_the_axis"));
            }
            "rotary_finishing_toolpath"
        }
        "indexed" => {
            // as many angles as it had sides: the new one picks the best of them, and stops
            // sooner when nothing worth another is left
            let faces = old.get("faces").and_then(Value::as_u64).unwrap_or(4);
            params.insert("most_angles".into(), json!(faces.clamp(1, 24)));
            // the new one has no registration mark or centre support of its own
            params.remove("registration_mark");
            params.remove("cut_support");
            "rotary_locked_angles_toolpath"
        }
        _ => return None,
    };
    Some((strategy, Value::Object(params)))
}

#[cfg(test)]
mod tests;
