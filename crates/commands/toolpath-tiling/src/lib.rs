//! Toolpath Tiling: save the toolpaths in tiles for a machine smaller than the job. The tiles are
//! drawn in the 2D view; Save Toolpaths writes
//! one file per tile, each moved so the start of its overlap is at the job's lower-left
//! corner (nothing goes below zero); the form says in red when a tile with its overlap is
//! bigger than the machine's bed. Where a cut runs off a
//! tile the tool lifts straight up before moving across to where it comes back in; a
//! laser keeps each line's own power, and a drill bank drills only the holes of its tile.
//! A cut past the material's edge (an outside profile's centreline, a lead-in) goes in the
//! tile at that edge, so nothing is left out of every tile. A toolpath with nothing to cut
//! in a tile is left out of that tile's file (no tool change or spindle start for it), and a
//! tile with nothing to cut at all gets no file. Each tile is numbered in the 2D view as
//! its file is named. Saving again where an earlier save left tile files (from another
//! layout too, or from before tiling was turned off) asks to Replace or Archive them, as
//! Edit ▸ Options ▸ Confirmations sets.

use cut_craft_core::document::Document;
use cut_craft_core::tiling::Tiling;
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct ToolpathTiling;

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// Save the toolpaths as tiles, one file each (ticked while the job is tiled). While it
    /// is on, Save Toolpaths writes one file for each tile, numbered after the name given
    /// (tile1, tile2…), in place of that name
    #[schemars(title = "Tile toolpaths")]
    pub enabled: bool,
    /// How the job is split into tiles
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
        // as the job has it: opened just to look, OK leaves tiling as it was
        Params {
            enabled: doc.tiling.is_some(),
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
    fn the_form_opens_showing_whether_tiling_is_on() {
        let off = Document::default();
        assert!(
            !ToolpathTiling::initial_params(&off).enabled,
            "OK on a look does not turn it on"
        );
        let on = after::<ToolpathTiling>(&Params {
            enabled: true,
            tiling: Tiling::default(),
        });
        assert!(ToolpathTiling::initial_params(&on).enabled);
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
