//! Checks before cutting: what in a toolpath the tool may not manage on the machine. These
//! checks are Cut Craft's own.

use crate::component::placed_model;
use crate::document::Document;
use crate::geometry::Path;
use crate::reach::{self, Fix, Side, too_deep};
use crate::toolpath::{MoveKind, Toolpath};

/// The 3D Finishing strategy, whose toolpaths follow the model.
const FINISHING: &str = "3d_finishing_toolpath";

/// The Profile strategy, whose cut right through the material lets a part come free.
const PROFILE: &str = "profile_toolpath";

/// The Pocket strategy, which clears inside its shapes.
const POCKET: &str = "pocket_toolpath";

/// How close to the material's bottom a cut must reach to count as right through, mm.
const THROUGH: f64 = 1e-3;

/// Material left that is worth a mention, mm, until Options says otherwise.
pub const LEFT: f64 = 0.1;

/// What may go wrong cutting `toolpath`, one sentence each; empty when nothing was found.
pub fn problems(doc: &Document, toolpath: &Toolpath) -> Vec<String> {
    problems_over(doc, toolpath, LEFT)
}

/// [`problems`], mentioning material left behind only where it is more than `left` mm
/// (Options ▸ Confirmation Choices).
pub fn problems_over(doc: &Document, toolpath: &Toolpath, left: f64) -> Vec<String> {
    let (name, tool) = (&toolpath.name, &toolpath.tool);
    let mut found = vec![];
    // the material's surface is z 0
    let deepest = toolpath
        .moves
        .iter()
        .filter(|m| m.kind != MoveKind::Rapid)
        .map(|m| -m.to.z)
        .fold(0.0, f64::max);
    let units = doc.job.units;
    if let Some(deep) = too_deep(name, tool, deepest, units) {
        found.push(deep);
    } else if tool.cutting_length().is_none() && deepest > 0.0 {
        found.push(format!(
            "{name} cuts {} deep; {} has no flute length set, so whether it reaches is not \
             checked: set it in the Tool Database",
            units.length(deepest, 1),
            tool.name
        ));
    }
    if let Some(loose) = comes_loose(doc, toolpath, deepest) {
        found.push(loose);
    }
    if let Some(missed) = missed_spots(doc, toolpath, left) {
        found.push(missed);
    }
    found.extend(fell_back(toolpath, units));
    found
}

/// Where `toolpath` went down another way than its entry asked, there being no room: the
/// straight plunges first (hard on the bit, and a bit that cannot cut at its centre cannot
/// do them), then the rest, each with where.
fn fell_back(toolpath: &Toolpath, units: crate::document::Units) -> Vec<String> {
    use crate::entry::EntryMode;
    let name = &toolpath.name;
    let places = |which: &dyn Fn(&crate::entry::Fallback) -> bool| -> Vec<String> {
        toolpath
            .fallbacks
            .iter()
            .filter(|f| which(f))
            .map(|f| {
                format!(
                    "X {} Y {}",
                    units.length(f.at.x, 1),
                    units.length(f.at.y, 1)
                )
            })
            .collect()
    };
    let listed = |at: &[String]| {
        let shown: Vec<&str> = at.iter().take(3).map(String::as_str).collect();
        let more = at.len().saturating_sub(3);
        let more = if more > 0 {
            format!(" and {more} more")
        } else {
            String::new()
        };
        format!("{}{more}", shown.join("; "))
    };
    let wanted = toolpath
        .fallbacks
        .iter()
        .find(|f| f.wanted != EntryMode::Plunge)
        .map_or("entry", |f| f.wanted.said());
    let mut found = vec![];
    let plunged = places(&|f| f.used == EntryMode::Plunge);
    let vcarve = toolpath.strategy == "vcarve_engrave_toolpath";
    if !plunged.is_empty() && vcarve {
        found.push(format!(
            "{name} goes straight down in {} {} ({}) where the groove is too short for its \
             {wanted}. That is usual for a V-carve: nothing to fix",
            plunged.len(),
            if plunged.len() == 1 {
                "place"
            } else {
                "places"
            },
            listed(&plunged)
        ));
    } else if !plunged.is_empty() {
        found.push(format!(
            "{name} goes straight down into the material in {} {} ({}), there being no room \
             there for its {wanted} or a zig-zag. That is hard on the bit, and a bit that \
             does not cut at its centre cannot do it: use a smaller bit, or check the shape \
             is meant to be that small",
            plunged.len(),
            if plunged.len() == 1 {
                "place"
            } else {
                "places"
            },
            listed(&plunged)
        ));
    }
    let kept_off = places(&|f| f.wanted == EntryMode::Plunge && f.used != EntryMode::Plunge);
    if !kept_off.is_empty() {
        found.push(format!(
            "{name}: {} must not go straight down into the material (a surfacing bit, fly \
             cutter or form tool), so it ramps in instead in {} {} ({}). Nothing to fix; \
             set the ramp in the toolpath's Entry settings",
            toolpath.tool.name,
            kept_off.len(),
            if kept_off.len() == 1 {
                "place"
            } else {
                "places"
            },
            listed(&kept_off)
        ));
    }
    let other = places(&|f| f.wanted != EntryMode::Plunge && f.used != EntryMode::Plunge);
    if !other.is_empty() {
        found.push(format!(
            "{name} goes down another way than its {wanted} in {} {} ({}), there being too \
             little room there: a zig-zag along the cut instead, no steeper. Nothing to fix \
             unless you want the {wanted} there: use a smaller bit, or change the \
             toolpath's Entry settings",
            other.len(),
            if other.len() == 1 { "place" } else { "places" },
            listed(&other)
        ));
    }
    found
}

/// The spots `toolpath`'s bit is too wide to get into, as a sentence; `None` when it gets
/// in everywhere, when a later toolpath cuts after it with a smaller bit (rest machining,
/// a pocket's clearing bits), or for a toolpath this is not checked for.
fn missed_spots(doc: &Document, toolpath: &Toolpath, left: f64) -> Option<String> {
    let (params, tool) = (&toolpath.params, &toolpath.tool);
    if finished_after(doc, toolpath) {
        return None;
    }
    let shapes = || -> Vec<Path> {
        toolpath
            .vectors
            .iter()
            .filter_map(|id| doc.vectors.get(id))
            .filter(|v| v.closed)
            .cloned()
            .collect()
    };
    let (missed, fix) = match toolpath.strategy.as_str() {
        POCKET => (reach::flat(&shapes(), tool, Side::Inside), Fix::Pocket),
        PROFILE => {
            let side = match params["side"].as_str() {
                Some("inside") => Side::Inside,
                Some("outside") | None => Side::Outside,
                _ => return None,
            };
            (reach::flat(&shapes(), tool, side), Fix::Profile)
        }
        FINISHING => {
            let model = placed_model(doc, tool).ok()?;
            (reach::surface(&model, tool, left), Fix::Finish)
        }
        _ => return None,
    };
    missed
        .any()
        .then(|| reach::missed_sentence(&toolpath.name, tool, &missed, fix, doc.job.units))
}

/// Whether another toolpath of `doc` cuts after `toolpath`'s tool, to get what it left:
/// rest machining after it, or a pocket finished with a smaller bit after it cleared.
fn finished_after(doc: &Document, toolpath: &Toolpath) -> bool {
    let same = |t: &serde_json::Value| {
        t["name"].as_str() == Some(toolpath.tool.name.as_str())
            && t["diameter"]
                .as_f64()
                .is_some_and(|d| (d - toolpath.tool.diameter).abs() < 1e-9)
    };
    doc.toolpaths.values().any(|other| {
        let after = |v: &serde_json::Value| v.as_array().is_some_and(|a| a.iter().any(same));
        other.vectors == toolpath.vectors
            && (after(&other.params["after"]) || after(&other.params["rest"]["after"]))
    })
}

/// A profile that cuts right through the material round a closed shape, with no tabs: what
/// it cuts out comes free and can be thrown by the cutter, or move into it. `deepest` is
/// the deepest the toolpath cuts below the surface.
fn comes_loose(doc: &Document, toolpath: &Toolpath, deepest: f64) -> Option<String> {
    let params = &toolpath.params;
    let through = doc.job.thickness > 0.0 && deepest + THROUGH >= doc.job.thickness;
    if toolpath.strategy != PROFILE || held_by_tabs(params) || !through {
        return None;
    }
    // along the line itself, only a closed shape is cut out
    let closed = toolpath
        .vectors
        .iter()
        .any(|id| doc.vectors.get(id).is_some_and(|v| v.closed));
    if params["side"] == "on" && !closed {
        return None;
    }
    Some(format!(
        "{} cuts right through the material with no tabs, so the piece it cuts out comes \
         loose and can move into the cutter. Add tabs (Tabs in the toolpath's form), or hold \
         the piece down another way: tape, screws or a vacuum table",
        toolpath.name
    ))
}

/// Whether a profile's settings leave tabs that hold the piece: tabs on, and either sized
/// from the tool (never nothing) or given a thickness.
pub fn held_by_tabs(params: &serde_json::Value) -> bool {
    let tab = &params["tab"];
    let sized_by_tool = tab["size_from_tool"].as_bool().unwrap_or(true);
    let thickness = tab["thickness"].as_f64().unwrap_or(1.0);
    params["tabs"] != "none" && (sized_by_tool || thickness > 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::domed;
    use crate::toolpath::{Move, Tool, ToolShape};
    use glam::DVec3;

    fn toolpath(tool: Tool, strategy: &str, depth: f64) -> Toolpath {
        let at = |kind, z| Move {
            kind,
            to: DVec3::new(20.0, 20.0, z),
        };
        Toolpath {
            name: "Finish".into(),
            strategy: strategy.into(),
            params: serde_json::Value::Null,
            tool,
            vectors: vec![],
            moves: vec![
                at(MoveKind::Rapid, 5.0),
                at(MoveKind::Plunge, -depth),
                at(MoveKind::Rapid, 5.0),
            ],
            visible: true,
            power: vec![],
            stale: false,
            group: None,
            drill_bank: None,
            fallbacks: vec![],
        }
    }

    #[test]
    fn a_cut_deeper_than_the_flutes_is_flagged() {
        let tool = Tool {
            name: "1/4 end mill".into(),
            flute_length: 22.0,
            ..Tool::default()
        };
        let doc = domed();
        assert_eq!(
            problems(&doc, &toolpath(tool.clone(), "pocket", 20.0)),
            Vec::<String>::new()
        );
        let found = problems(&doc, &toolpath(tool.clone(), "pocket", 25.0));
        assert_eq!(found.len(), 1);
        assert!(
            found[0].contains("25.0 mm deep") && found[0].contains("22.0 mm"),
            "{found:?}"
        );
        // an inch job says it in inches
        let mut inch = domed();
        inch.job.units = crate::document::Units::Inch;
        let found = problems(&inch, &toolpath(tool, "pocket", 25.4));
        assert!(
            found[0].contains("1.000 in deep") && found[0].contains("0.866 in long"),
            "{found:?}"
        );
        let unknown = problems(&doc, &toolpath(Tool::default(), "pocket", 5.0));
        assert!(unknown[0].contains("no flute length"), "{unknown:?}");
    }

    #[test]
    fn a_taper_too_wide_for_the_model_is_flagged() {
        let doc = domed(); // an 8 mm dome; its foot meets the flat at a right angle
        let ball = Tool {
            name: "ball".into(),
            shape: ToolShape::BallNose,
            diameter: 2.0,
            flute_length: 30.0,
            ..Tool::default()
        };
        let straight = problems(&doc, &toolpath(ball.clone(), FINISHING, 8.0));
        assert_eq!(
            straight,
            Vec::<String>::new(),
            "a 2 mm ball fits round the foot"
        );
        let tapered = Tool {
            taper: 20.0,
            shank_diameter: 8.0,
            ..ball
        };
        let found = problems(&doc, &toolpath(tapered, FINISHING, 8.0));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("can't reach"), "{found:?}");
        assert!(found[0].contains("tip and taper"), "{found:?}");
        // a taper narrow enough for the dome's 90° foot still leaves nothing extra on its top
        assert!(!found[0].contains("0.0 cm²"), "{found:?}");
        // told to mention only more material than the taper leaves, nothing is found
        let lax = problems_over(&doc, &toolpath(tapered_again(), FINISHING, 8.0), 50.0);
        assert_eq!(lax, Vec::<String>::new());
    }

    fn tapered_again() -> Tool {
        Tool {
            name: "ball".into(),
            shape: ToolShape::BallNose,
            diameter: 2.0,
            flute_length: 30.0,
            taper: 20.0,
            shank_diameter: 8.0,
            ..Tool::default()
        }
    }

    /// A profile round `vectors` of the sample document, `depth` deep, cut `side` with
    /// `tabs`.
    fn profile(vectors: &[u64], side: &str, tabs: &str, depth: f64) -> Toolpath {
        Toolpath {
            name: "Profile 1".into(),
            params: serde_json::json!({ "side": side, "tabs": tabs }),
            vectors: vectors
                .iter()
                .map(|&n| crate::document::EntityId(n))
                .collect(),
            ..toolpath(Tool::default(), PROFILE, depth)
        }
    }

    fn loose(found: &[String]) -> bool {
        found.iter().any(|f| f.contains("comes loose"))
    }

    #[test]
    fn a_profile_right_through_with_no_tabs_lets_the_part_come_loose() {
        let mut doc = crate::testing::sample_document();
        doc.job.thickness = 20.0;
        // the first square is closed, the third vector an open polyline
        let found = problems(&doc, &profile(&[1], "outside", "none", 20.0));
        assert!(loose(&found), "{found:?}");
        assert!(
            found.iter().any(|f| f.contains("Add tabs")),
            "says how to fix it"
        );
        assert!(loose(&problems(
            &doc,
            &profile(&[1], "inside", "none", 20.5)
        )));
        assert!(loose(&problems(&doc, &profile(&[1], "on", "none", 20.0))));
        // tabs hold it; a cut short of the bottom leaves it joined; an open line frees nothing
        assert!(!loose(&problems(
            &doc,
            &profile(&[1], "outside", "flat", 20.0)
        )));
        assert!(!loose(&problems(
            &doc,
            &profile(&[1], "outside", "none", 19.0)
        )));
        assert!(!loose(&problems(&doc, &profile(&[3], "on", "none", 20.0))));
        // tabs given no thickness hold nothing
        let mut thin = profile(&[1], "outside", "flat", 20.0);
        thin.params["tab"] = serde_json::json!({ "size_from_tool": false, "thickness": 0.0 });
        assert!(loose(&problems(&doc, &thin)));
        // only a profile's cut is judged so
        let pocket = Toolpath {
            strategy: "pocket_toolpath".into(),
            ..profile(&[1], "outside", "none", 20.0)
        };
        assert!(!loose(&problems(&doc, &pocket)));
    }

    /// A pocket (or profile) of the sample document's first square, cut with `tool`.
    fn pocket(tool: Tool, strategy: &str, params: serde_json::Value) -> Toolpath {
        Toolpath {
            name: "Pocket 1".into(),
            params,
            vectors: vec![crate::document::EntityId(1)],
            ..toolpath(tool, strategy, 5.0)
        }
    }

    /// [`pocket`] on vector `id`.
    fn pocket_of(
        id: crate::document::EntityId,
        tool: Tool,
        strategy: &str,
        params: serde_json::Value,
    ) -> Toolpath {
        Toolpath {
            vectors: vec![id],
            ..pocket(tool, strategy, params)
        }
    }

    #[test]
    fn a_bit_too_wide_for_a_spot_is_flagged_on_pockets_and_profiles() {
        // a 40 mm square with a 4 mm wide channel going up out of it
        let spur = crate::geometry::Path::polyline(
            &[
                crate::geometry::Point::new(18.0, 39.0),
                crate::geometry::Point::new(22.0, 39.0),
                crate::geometry::Point::new(22.0, 60.0),
                crate::geometry::Point::new(18.0, 60.0),
            ],
            true,
        );
        let joined =
            crate::polygon::union(&[crate::testing::square(0.0, 0.0, 40.0), spur]).remove(0);
        let mut doc = crate::testing::document_with(vec![joined]);
        let first = *doc.vectors.keys().next().unwrap();
        let bit = |d: f64| Tool {
            name: format!("{d} mm End Mill"),
            diameter: d,
            flute_length: 20.0,
            ..Tool::default()
        };
        let found = problems(
            &doc,
            &pocket_of(first, bit(6.0), POCKET, serde_json::Value::Null),
        );
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].starts_with("Pocket 1: 6 mm End Mill can't reach 1 spot"),
            "{found:?}"
        );
        assert!(
            found[0].contains("Clear with bigger bits first"),
            "{found:?}"
        );
        assert_eq!(
            problems(
                &doc,
                &pocket_of(first, bit(3.0), POCKET, serde_json::Value::Null)
            ),
            Vec::<String>::new()
        );
        // a profile inside the same shape misses it too; outside, nothing is missed
        let inside = pocket_of(
            first,
            bit(6.0),
            PROFILE,
            serde_json::json!({ "side": "inside" }),
        );
        assert_eq!(problems(&doc, &inside).len(), 1);
        let outside = pocket_of(
            first,
            bit(6.0),
            PROFILE,
            serde_json::json!({ "side": "outside" }),
        );
        assert_eq!(problems(&doc, &outside), Vec::<String>::new());
        // a smaller bit cutting after it (a pocket's clearing bits) gets what it left
        let finish = pocket_of(
            first,
            bit(3.0),
            POCKET,
            serde_json::json!({ "rest": { "after": [bit(6.0)] } }),
        );
        let mut c = crate::Change::builder(&doc);
        c.add_toolpath(finish);
        doc.apply(&c.finish()).unwrap();
        assert_eq!(
            problems(
                &doc,
                &pocket_of(first, bit(6.0), POCKET, serde_json::Value::Null)
            ),
            Vec::<String>::new()
        );
    }
}
