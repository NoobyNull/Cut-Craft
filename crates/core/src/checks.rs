//! Checks before cutting: what in a toolpath the tool may not manage on the machine. Aspire
//! makes none of these; they are Cut Craft's own.

use crate::component::placed_model;
use crate::document::Document;
use crate::toolpath::{MoveKind, Tool, Toolpath};

/// The 3D Finishing strategy, whose toolpaths follow the model.
const FINISHING: &str = "3d_finishing_toolpath";

/// The Profile strategy, whose cut right through the material lets a part come free.
const PROFILE: &str = "profile_toolpath";

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
    if tool.flute_length > 0.0 && deepest > tool.flute_length + 1e-6 {
        found.push(format!(
            "{name} cuts {deepest:.1} mm deep, but the flutes of {} are only {:.1} mm long",
            tool.name, tool.flute_length
        ));
    } else if tool.flute_length <= 0.0 && deepest > 0.0 {
        found.push(format!(
            "{name} cuts {deepest:.1} mm deep; {} has no flute length set, so whether it \
             reaches is not checked",
            tool.name
        ));
    }
    if let Some(loose) = comes_loose(doc, toolpath, deepest) {
        found.push(loose);
    }
    if toolpath.strategy == FINISHING
        && tool.reach() > tool.diameter / 2.0
        && let Ok(model) = placed_model(doc, tool)
    {
        // what the taper holds the tip off, over what the same tip alone would leave
        let tip_alone = Tool {
            taper: 0.0,
            ..tool.clone()
        };
        let (tapered, alone) = (model.left_by(tool), model.left_by(&tip_alone));
        let (cells, most) = tapered
            .heights
            .iter()
            .zip(&alone.heights)
            .map(|(t, a)| t - a)
            .filter(|&d| d > left)
            .fold((0_u32, 0.0_f64), |(n, most), d| (n + 1, most.max(d)));
        if cells > 0 {
            let area = f64::from(cells) * model.cell * model.cell;
            found.push(format!(
                "{name}: the taper of {} keeps it off {:.1} cm² of the model, where it is too \
                 steep or narrow, and leaves up to {most:.1} mm there",
                tool.name,
                area / 100.0
            ));
        }
    }
    found
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
    use crate::toolpath::{Move, ToolShape};
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
        let found = problems(&doc, &toolpath(tool, "pocket", 25.0));
        assert_eq!(found.len(), 1);
        assert!(
            found[0].contains("25.0 mm deep") && found[0].contains("22.0 mm"),
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
        assert!(found[0].contains("too steep"), "{found:?}");
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
}
