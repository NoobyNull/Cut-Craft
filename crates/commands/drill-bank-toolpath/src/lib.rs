//! Drill Bank Toolpath: the selected circles drilled by the machine's drill bank, as many at
//! once as line up with its drills. Each plunge of the head drops the drills over holes and
//! keeps the rest up; its moves drill the holes one after another, so the preview and the
//! simulation show every hole, and the post writes one plunge of the head for them
//! (`re/SPEC.md`, Drill Bank Toolpath).

use cmd_drilling_toolpath::Retract;
use cut_craft_core::document::Document;
use cut_craft_core::drill_bank::{BankHit, BankToolpath, DrillBank, Hole, Tolerances};
use cut_craft_core::glam::DVec3;
use cut_craft_core::toolpath::{Move, MoveKind, Tool, made_by};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct DrillBankToolpath;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// What the toolpath is called in the toolpath list
    pub name: String,
    /// Below the material surface
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    #[schemars(title = "Cut Depth", extend("x-group" = "Cutting Depths"))]
    pub depth: f64,
    /// The feed, plunge rate, spindle speed and pass depth the bank's drills run at; the
    /// holes' sizes come from the drills themselves
    pub tool: Tool,
    /// Drill in pecks of the tool's pass depth, rising between them to clear the chips; off
    /// drills straight down
    #[serde(default = "yes")]
    #[schemars(title = "Use Peck Drilling", extend("x-group" = "Peck Drilling"))]
    pub peck: bool,
    #[serde(default)]
    #[schemars(extend("x-group" = "Peck Drilling", "x-show-when" = {"peck": "true"}))]
    pub retract: Retract,
    /// How far above the start depth, or the last peck, the drills rise between pecks
    #[serde(default = "retract_gap")]
    #[schemars(extend("x-unit" = "mm", "x-group" = "Peck Drilling"), range(min = 0))]
    pub retract_gap: f64,
    /// Seconds to wait at the bottom of each peck (0: none); the post must be able to dwell
    #[serde(default)]
    #[schemars(title = "Dwell", extend("x-unit" = "s"), range(min = 0))]
    pub dwell: f64,
    /// How far a hole's centre may be from where a drill comes down and still be drilled by
    /// it: a little for holes drawn slightly off the pitch
    #[serde(default = "tolerance")]
    #[schemars(
        title = "Pitch Tolerance",
        extend("x-unit" = "mm", "x-group" = "Matching Holes to Drills"),
        range(min = 0)
    )]
    pub pitch_tolerance: f64,
    /// How much a hole's size may differ from a drill's and still be drilled by it
    #[serde(default = "tolerance")]
    #[schemars(
        title = "Diameter Tolerance",
        extend("x-unit" = "mm", "x-group" = "Matching Holes to Drills"),
        range(min = 0)
    )]
    pub diameter_tolerance: f64,
    /// Drill the holes in the order the vectors were picked; else in the order they were drawn
    #[serde(default)]
    pub use_vector_selection_order: bool,
    /// The machine's drill bank, from Machine Configuration when the form opens
    #[serde(default)]
    #[schemars(skip)]
    pub bank: DrillBank,
}

fn yes() -> bool {
    true
}

fn retract_gap() -> f64 {
    1.0
}

fn tolerance() -> f64 {
    0.1
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Drill Bank".into(),
            depth: 12.0,
            tool: Tool::default(),
            peck: true,
            retract: Retract::AboveStartDepth,
            retract_gap: retract_gap(),
            dwell: 0.0,
            pitch_tolerance: tolerance(),
            diameter_tolerance: tolerance(),
            use_vector_selection_order: false,
            bank: DrillBank::default(),
        }
    }
}

impl Command for DrillBankToolpath {
    const ID: &'static str = "drill_bank_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Drill Bank Toolpath",
            group: "Toolpath Operations",
            icon: "toolpaths/drilling_toolpath.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("depth", p.depth)?;
        ensure_positive("pass depth", p.tool.pass_depth)?;
        if let Some(problem) = p.bank.problem() {
            return Err(CommandError::Failed(problem));
        }
        if p.dwell < 0.0 || p.retract_gap < 0.0 {
            return Err(CommandError::InvalidParams(
                "set the dwell and retract gap to 0 or more".into(),
            ));
        }
        let (ids, paths) = doc.selected_paths(|path| path.closed);
        if ids.is_empty() {
            return Err(CommandError::Failed(
                "select the circles to drill: each is drilled at its centre by a drill its size"
                    .into(),
            ));
        }
        let mut order: Vec<usize> = (0..paths.len()).collect();
        if !p.use_vector_selection_order {
            order.sort_by_key(|&i| ids[i]);
        }
        let holes: Vec<Hole> = order
            .iter()
            .map(|&i| {
                let b = paths[i].bounds();
                let size = b.size();
                Hole {
                    at: b.center(),
                    diameter: f64::midpoint(size.x, size.y),
                }
            })
            .collect();
        let tolerances = Tolerances {
            pitch: p.pitch_tolerance,
            diameter: p.diameter_tolerance,
        };
        let plan = p
            .bank
            .plan(&holes, tolerances)
            .map_err(CommandError::Failed)?;
        let safe = doc.job.clearance;
        let depths = if p.peck {
            p.tool.pass_depths(p.depth)
        } else {
            vec![-p.depth]
        };
        let mut moves = vec![];
        let mut hits = vec![];
        for hit in &plan {
            let start = moves.len();
            for &k in &hit.holes {
                let c = holes[k].at;
                let at = |z: f64| DVec3::new(c.x, c.y, z);
                moves.push(Move {
                    kind: MoveKind::Rapid,
                    to: at(safe),
                });
                for (n, &z) in depths.iter().enumerate() {
                    moves.push(Move {
                        kind: MoveKind::Plunge,
                        to: at(z),
                    });
                    let rise = match p.retract {
                        _ if n + 1 == depths.len() => safe,
                        Retract::AboveStartDepth => p.retract_gap,
                        Retract::AbovePreviousPass => z + p.retract_gap,
                    };
                    moves.push(Move {
                        kind: MoveKind::Rapid,
                        to: at(rise),
                    });
                }
            }
            hits.push(BankHit {
                start,
                end: moves.len(),
                head: hit.head,
                drills: hit.drills.clone(),
            });
        }
        let mut toolpath = made_by::<Self>(p, p.name.clone(), p.tool.clone(), ids, moves);
        toolpath.drill_bank = Some(BankToolpath {
            bank: p.bank.clone(),
            hits,
        });
        let mut c = Change::builder(doc);
        c.add_toolpath(toolpath);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::{Path, Point};
    use cut_craft_core::testing::made_toolpath;

    use cut_craft_core::testing;

    // the shared contract, but run with a bank: without one there is nothing to drill with
    #[test]
    fn keeps_the_command_contract() {
        testing::id_is_a_slug_and_meta_is_labelled::<DrillBankToolpath>();
        testing::default_params_round_trip_through_json::<DrillBankToolpath>();
        testing::schema_describes_an_object::<DrillBankToolpath>();
    }

    #[test]
    fn undo_restores_the_document() {
        let original = testing::document_with(row());
        let change = DrillBankToolpath::run(&original, &with_bank()).unwrap();
        let mut doc = original.clone();
        doc.apply(&change).unwrap();
        doc.apply(&change.inverse()).unwrap();
        assert_eq!(doc, original);
    }

    fn with_bank() -> Params {
        Params {
            bank: DrillBank::new(5, 0, 32.0, 5.0),
            ..Params::default()
        }
    }

    /// Seven 5 mm shelf-pin holes 32 mm apart.
    fn row() -> Vec<Path> {
        (0..7)
            .map(|i| Path::circle(Point::new(50.0 + 32.0 * f64::from(i), 40.0), 2.5))
            .collect()
    }

    #[test]
    fn a_row_of_holes_takes_two_plunges_of_a_five_drill_bank() {
        let tp = made_toolpath::<DrillBankToolpath>(row(), &with_bank()).unwrap();
        let bank = tp.drill_bank.as_ref().expect("a drill bank toolpath");
        assert_eq!(bank.hits.len(), 2);
        assert_eq!(bank.hits[0].drills.len(), 5);
        assert_eq!(bank.hits[1].drills.len(), 2);
        // the reference (middle) drill over the third hole, then two pitches further on
        assert!(bank.hits[0].head.distance(Point::new(114.0, 40.0)) < 1e-6);
        assert_eq!(bank.hits[0].start, 0);
        assert_eq!(bank.hits[0].end, bank.hits[1].start);
        assert_eq!(bank.hits[1].end, tp.moves.len());
        // every hole is drilled in the moves, for the preview: 12 mm in 3 mm pecks
        let plunges = tp
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Plunge)
            .count();
        assert_eq!(plunges, 7 * 4);
        assert!(tp.moves.last().unwrap().to.z > 0.0, "finishes clear");
    }

    #[test]
    fn without_a_bank_it_says_where_to_set_one_up() {
        let err = made_toolpath::<DrillBankToolpath>(row(), &Params::default()).unwrap_err();
        assert!(err.contains("Machine Configuration"), "{err}");
    }

    #[test]
    fn holes_no_drill_fits_are_refused_with_the_sizes() {
        let holes = vec![Path::circle(Point::new(10.0, 10.0), 4.0)];
        let err = made_toolpath::<DrillBankToolpath>(holes, &with_bank()).unwrap_err();
        assert!(err.contains("8 mm") && err.contains("5 mm"), "{err}");
    }

    #[test]
    fn needs_something_to_drill() {
        let err = made_toolpath::<DrillBankToolpath>(
            vec![Path::polyline(&[Point::ZERO, Point::X], false)],
            &with_bank(),
        )
        .unwrap_err();
        assert!(err.starts_with("Select the circles"), "{err}");
    }
}
