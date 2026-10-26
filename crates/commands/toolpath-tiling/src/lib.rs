//! Toolpath Tiling: save the toolpaths in tiles for a machine smaller than the job (Aspire's
//! help: Toolpath Tiling Manager). The tiles are drawn in the 2D view; Save Toolpaths writes
//! one file per tile, each moved to the job's lower-left corner.

use cut_craft_core::document::Document;
use cut_craft_core::tiling::Tiling;
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct ToolpathTiling;

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    #[schemars(title = "Tile toolpaths")]
    pub enabled: bool,
    #[schemars(title = "Tiling Options", extend("x-section" = true))]
    pub tiling: Tiling,
}

impl Command for ToolpathTiling {
    const ID: &'static str = "toolpath_tiling";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Toolpath Tiling",
            group: "Toolpaths",
            icon: "",
            shortcut: "",
        }
    }

    fn initial_params(doc: &Document) -> Params {
        Params {
            enabled: true,
            tiling: doc.tiling.unwrap_or_default(),
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if p.enabled {
            ensure_positive("tile width", p.tiling.width)?;
            ensure_positive("tile height", p.tiling.height)?;
        }
        let mut c = Change::builder(doc);
        c.set_tiling(p.enabled.then_some(p.tiling));
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{after, rejection};

    cut_craft_core::contract_tests!(ToolpathTiling);

    #[test]
    fn turns_tiling_on_with_its_settings_and_off() {
        let on = after::<ToolpathTiling>(&Params {
            enabled: true,
            tiling: Tiling::default(),
        });
        assert_eq!(on.tiling, Some(Tiling::default()));
        assert_eq!(
            ToolpathTiling::initial_params(&on).tiling,
            Tiling::default()
        );
        let off = after::<ToolpathTiling>(&Params::default());
        assert!(off.tiling.is_none());
    }

    #[test]
    fn tiles_have_a_size() {
        let p = Params {
            enabled: true,
            tiling: Tiling {
                width: 0.0,
                ..Tiling::default()
            },
        };
        assert!(rejection::<ToolpathTiling>(&p).contains("tile width"));
    }
}
