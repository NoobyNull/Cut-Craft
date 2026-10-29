//! Checks before cutting: what in a toolpath the tool may not manage on the machine. Aspire
//! makes none of these; they are Cut Craft's own.

use crate::component::placed_model;
use crate::document::Document;
use crate::toolpath::{MoveKind, Tool, Toolpath};

/// The 3D Finishing strategy, whose toolpaths follow the model.
const FINISHING: &str = "3d_finishing_toolpath";

/// Material left that is worth a mention, mm.
// ponytail: one figure for every job; a setting if it proves too fussy or too lax
const LEFT: f64 = 0.1;

/// What may go wrong cutting `toolpath`, one sentence each; empty when nothing was found.
pub fn problems(doc: &Document, toolpath: &Toolpath) -> Vec<String> {
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
            .filter(|&d| d > LEFT)
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
    }
}
