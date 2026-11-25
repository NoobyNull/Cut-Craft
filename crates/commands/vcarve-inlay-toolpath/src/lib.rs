//! V-Carve Inlay Toolpath (V-bit carved inlay): two V-carvings that fit together. The pocket carves
//! the selected shapes; the plug carves round mirrored copies of them, so that turned over it drops
//! into the pocket leaving the glue gap under it. The plug is cut from a second board (usually a
//! contrasting wood), so it goes on a sheet of its own in the Sheets list, called Inlay plug: open
//! that sheet to see or save the Plug toolpath. Only the Pocket shows in this sheet's toolpath
//! list, and Calculate says where the Plug went.

use cmd_vcarve_engrave_toolpath::{VcarveEngraveToolpath, carve};
use cut_craft_core::document::{Document, Sheet, StoredVector};
use cut_craft_core::geometry::{Path, Point};
use cut_craft_core::glam::{DAffine2, DVec2};
use cut_craft_core::polygon::{self, Corners};
use cut_craft_core::toolpath::{Tool, ToolDatabase, ToolShape, made_by};
use cut_craft_core::{Change, Command, CommandError, Form, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct VcarveInlayToolpath;

/// The sheet the plug is put on.
pub const PLUG_SHEET: &str = "Inlay plug";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PlugBoundary {
    /// Only round the shapes, the surface clearance plus the boundary offset out from them
    #[default]
    VectorOffset,
    /// From the shapes out to the edges of the plug's board (the Inlay plug sheet)
    SheetLimits,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// What the toolpath is called in the toolpath list
    pub name: String,
    /// Deepest the pocket goes (narrow parts stop short of it)
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    #[schemars(extend("x-group" = "Cutting Depths"))]
    #[schemars(extend("x-dim" = "D", "x-diagram" = "diagrams/depth_vcarve_inlay.png"))]
    pub pocket_depth: f64,
    /// Between the top of the plug and the bottom of the pocket once fitted
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    #[schemars(extend("x-group" = "Inlay"))]
    #[schemars(extend("x-dim" = "G", "x-diagram" = "diagrams/vcarve_inlay_fit.png"))]
    pub glue_gap: f64,
    /// Between the plug's material and the pocket's surface once fitted
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    #[schemars(extend("x-group" = "Inlay"))]
    #[schemars(extend("x-dim" = "G2"))]
    pub surface_clearance: f64,
    /// How far round the shapes the plug's surface is cleared, on the Inlay plug sheet
    #[schemars(extend("x-group" = "Boundary"))]
    pub plug_boundary: PlugBoundary,
    /// Extra material cut away round the plug
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    #[schemars(extend("x-group" = "Boundary"))]
    pub boundary_offset: f64,
    /// A V-bit, the same for both halves
    pub tool: Tool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "V-Carve Inlay".into(),
            pocket_depth: 5.0,
            glue_gap: 0.2,
            surface_clearance: 1.5,
            plug_boundary: PlugBoundary::VectorOffset,
            boundary_offset: 3.0,
            tool: ToolDatabase::starter()
                .tools
                .into_iter()
                .find(|t| t.shape == ToolShape::VBit)
                .expect("the starter tools include a V-bit"),
        }
    }
}

impl Command for VcarveInlayToolpath {
    const ID: &'static str = "vcarve_inlay_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "V-Carve Inlay Toolpath",
            group: "Toolpath Operations",
            icon: "toolpaths/vcarve_inlay.png",
            shortcut: "",
            form: Form::TOOLPATH,
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let invalid = |why: &str| Err(CommandError::InvalidParams(why.into()));
        if p.tool.shape != ToolShape::VBit {
            return invalid("an inlay needs a V-bit");
        }
        ensure_positive("pocket depth", p.pocket_depth)?;
        if !(p.glue_gap > 0.0 && p.glue_gap < p.pocket_depth) {
            return invalid("the glue gap must be more than 0 and less than the pocket depth");
        }
        if !(p.surface_clearance > 0.0 && p.surface_clearance < doc.job.thickness) {
            return invalid(
                "the surface clearance must be more than 0 and less than the material thickness",
            );
        }
        let (ids, shapes) = doc.selected_paths(|path| path.closed);
        if ids.is_empty() {
            return Err(CommandError::Failed(
                "select the closed vectors to inlay".into(),
            ));
        }
        let carving =
            |name: &str, start_depth: f64, flat_depth: f64| cmd_vcarve_engrave_toolpath::Params {
                name: format!("{} - {name}", p.name),
                start_depth,
                flat_depth,
                tool: p.tool.clone(),
                project_onto_model: false,
                ..cmd_vcarve_engrave_toolpath::Params::default()
            };
        let pocket = carving("Pocket", 0.0, p.pocket_depth);
        // turned over, the plug's top sits the glue gap above the pocket floor and its
        // material the surface clearance above the pocket's surface
        let plug = carving("Plug", p.pocket_depth - p.glue_gap, p.surface_clearance);
        // mirrored across the job's centre line, so that turned over it lies over the pocket
        let mirror = DAffine2::from_translation(DVec2::new(doc.job.width, 0.0))
            * DAffine2::from_scale(DVec2::new(-1.0, 1.0));
        let mirrored: Vec<Path> = shapes.iter().map(|s| s.transformed(&mirror)).collect();
        let boundary = match p.plug_boundary {
            PlugBoundary::SheetLimits => vec![Path::polyline(
                &[
                    Point::new(0.0, 0.0),
                    Point::new(doc.job.width, 0.0),
                    Point::new(doc.job.width, doc.job.height),
                    Point::new(0.0, doc.job.height),
                ],
                true,
            )],
            PlugBoundary::VectorOffset => {
                let spread = (p.tool.angle.to_radians() / 2.0).tan();
                let wall = p.surface_clearance * spread;
                polygon::offset(
                    &polygon::union(&mirrored),
                    wall + p.boundary_offset.max(0.0),
                    Corners::Round,
                )
            }
        };
        let plug_paths: Vec<Path> = mirrored.iter().chain(&boundary).cloned().collect();
        let safe = doc.job.clearance;
        let mut c = Change::builder(doc);
        c.add_toolpath(made_by::<VcarveEngraveToolpath>(
            &pocket,
            pocket.name.clone(),
            p.tool.clone(),
            ids,
            carve(&shapes, &pocket, safe),
        ));
        // the plug is cut from another board, so it goes on a sheet of its own (the one an
        // earlier inlay made, else a new one sized as this)
        let mut sheets = c.sheets().to_vec();
        let at = plug_sheet(&mut sheets, doc);
        let layer = Some(doc.current_layer);
        let plug_ids = plug_paths
            .iter()
            .map(|path| {
                let id = c.fresh_id();
                sheets[at].objects.vectors.insert(
                    id,
                    StoredVector {
                        path: path.clone(),
                        layer,
                        group: None,
                    },
                );
                id
            })
            .collect();
        let toolpath = made_by::<VcarveEngraveToolpath>(
            &plug,
            plug.name.clone(),
            p.tool.clone(),
            plug_ids,
            carve(&plug_paths, &plug, safe),
        );
        let id = c.fresh_id();
        sheets[at].objects.toolpaths.insert(id, toolpath);
        c.set_sheets(sheets, doc.active_sheet);
        c.note(format!(
            "The Plug toolpath is on the {PLUG_SHEET} sheet, for the second board: open it in \
             the Sheets tab to see or save it"
        ));
        Ok(c.finish())
    }
}

/// Where in `sheets` the plug goes: the Inlay plug sheet an earlier inlay made, else a new one
/// sized as the active sheet.
fn plug_sheet(sheets: &mut Vec<Sheet>, doc: &Document) -> usize {
    let found = (0..sheets.len()).find(|&i| i != doc.active_sheet && sheets[i].name == PLUG_SHEET);
    found.unwrap_or_else(|| {
        sheets.push(Sheet::new(PLUG_SHEET, doc.job));
        sheets.len() - 1
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::simulation::Material;
    use cut_craft_core::testing::{document_with, square};
    use cut_craft_core::toolpath::Toolpath;

    cut_craft_core::contract_tests!(VcarveInlayToolpath);

    /// The document after an inlay of a 20 mm square near the left of the default job.
    fn inlaid(p: &Params) -> Document {
        let mut doc = document_with(vec![square(10.0, 10.0, 20.0)]);
        let change = VcarveInlayToolpath::run(&doc, p).unwrap();
        doc.apply(&change).unwrap();
        doc
    }

    /// The toolpath named so, on the design's sheet or the plug's.
    fn toolpath<'a>(doc: &'a Document, ends_with: &str) -> &'a Toolpath {
        doc.toolpaths
            .values()
            .chain(doc.sheets.iter().flat_map(|s| s.objects.toolpaths.values()))
            .find(|t| t.name.ends_with(ends_with))
            .unwrap()
    }

    /// A vector on the plug's sheet.
    fn plug_vector(doc: &Document, id: cut_craft_core::document::EntityId) -> &Path {
        &doc.sheets[1].objects.vectors[&id].path
    }

    fn carved(doc: &Document, t: &Toolpath) -> Material {
        let mut m = Material::new(&doc.job, 400);
        m.cut(t);
        m
    }

    #[test]
    fn makes_a_pocket_and_a_plug_as_v_carvings_that_recalculate() {
        let doc = inlaid(&Params::default());
        for t in [toolpath(&doc, "Pocket"), toolpath(&doc, "Plug")] {
            assert_eq!(t.strategy, "vcarve_engrave_toolpath");
        }
        assert_eq!(toolpath(&doc, "Pocket").params["flat_depth"], 5.0);
        let plug = toolpath(&doc, "Plug");
        assert!((plug.params["start_depth"].as_f64().unwrap() - 4.8).abs() < 1e-9);
        assert_eq!(plug.params["flat_depth"], 1.5);
    }

    #[test]
    fn the_plug_is_mirrored_across_the_job_with_a_boundary_round_it() {
        let doc = inlaid(&Params::default());
        let plug = toolpath(&doc, "Plug");
        assert_eq!(
            plug.vectors.len(),
            2,
            "the mirrored square and its boundary"
        );
        let w = doc.job.width;
        let mirrored = plug_vector(&doc, plug.vectors[0]).bounds();
        assert!(
            mirrored.min.distance(Point::new(w - 30.0, 10.0)) < 1e-9,
            "{mirrored:?}"
        );
        assert!(
            plug_vector(&doc, plug.vectors[1])
                .bounds()
                .contains_rect(mirrored)
        );
    }

    #[test]
    fn the_plug_fits_the_pocket_with_the_glue_gap_under_it() {
        let p = Params::default();
        let doc = inlaid(&p);
        let pocket = carved(&doc, toolpath(&doc, "Pocket"));
        let plug = carved(&doc, toolpath(&doc, "Plug"));
        let spread = (p.tool.angle.to_radians() / 2.0).tan();
        let w = doc.job.width;
        // the pocket's surface edge is the square's edge; the plug's top shrinks in from
        // the mirrored edge by the depth it sits in the pocket
        let sits = p.pocket_depth - p.glue_gap;
        let pocket_edge = Point::new(10.0 + 0.5, 20.0);
        assert!(
            pocket.at(pocket_edge).unwrap() < -0.1,
            "pocket starts at the edge"
        );
        let inside_plug_top = Point::new(w - 10.0 - sits * spread - 0.5, 20.0);
        assert!(
            plug.at(inside_plug_top).unwrap().abs() < 1e-9,
            "plug top is uncut"
        );
        let outside_plug_top = Point::new(w - 10.0 - sits * spread + 0.5, 20.0);
        assert!(
            plug.at(outside_plug_top).unwrap() < -0.1,
            "plug wall begins there"
        );
        // the floor round the plug is the plug's whole height
        let floor = Point::new(w - 10.0 + p.surface_clearance * spread + 1.0, 20.0);
        let height = sits + p.surface_clearance;
        assert!(
            (plug.at(floor).unwrap() + height).abs() < 0.3,
            "{:?}",
            plug.at(floor)
        );
    }

    #[test]
    fn the_plug_goes_on_its_own_sheet_called_inlay_plug() {
        let mut doc = document_with(vec![square(10.0, 10.0, 20.0)]);
        doc.apply(&VcarveInlayToolpath::run(&doc, &Params::default()).unwrap())
            .unwrap();
        assert_eq!(
            doc.toolpaths.len(),
            1,
            "only the pocket on the design's sheet"
        );
        assert!(toolpath(&doc, "Pocket").name.ends_with("Pocket"));
        assert_eq!(doc.sheets.len(), 2);
        let sheet = &doc.sheets[1];
        assert_eq!(sheet.name, "Inlay plug");
        assert_eq!(sheet.objects.toolpaths.len(), 1);
        let plug = sheet.objects.toolpaths.values().next().unwrap();
        assert!(plug.name.ends_with("Plug"));
        assert_eq!(
            sheet.objects.vectors.len(),
            2,
            "the mirrored square and its boundary"
        );
        for id in &plug.vectors {
            assert!(sheet.objects.vectors.contains_key(id));
        }
        // a second inlay's plug joins the same plug sheet
        doc.apply(&VcarveInlayToolpath::run(&doc, &Params::default()).unwrap())
            .unwrap();
        assert_eq!(doc.sheets.len(), 2);
        assert_eq!(doc.sheets[1].objects.toolpaths.len(), 2);
    }

    #[test]
    fn calculate_says_where_the_plug_went() {
        // only the Pocket shows in the toolpath list, so the person is told where the Plug is
        let doc = document_with(vec![square(10.0, 10.0, 20.0)]);
        let change = VcarveInlayToolpath::run(&doc, &Params::default()).unwrap();
        assert_eq!(change.notes.len(), 1, "{:?}", change.notes);
        let note = &change.notes[0];
        assert!(note.contains("Plug") && note.contains(PLUG_SHEET), "{note}");
        assert!(note.contains("Sheets tab"), "{note}");
    }

    #[test]
    fn the_glue_gap_must_be_inside_the_pocket() {
        let p = Params {
            glue_gap: 6.0,
            ..Params::default()
        };
        let doc = document_with(vec![square(0.0, 0.0, 10.0)]);
        let why = VcarveInlayToolpath::run(&doc, &p).unwrap_err().to_string();
        assert!(why.contains("glue gap"), "{why}");
    }

    #[test]
    fn it_needs_a_v_bit() {
        let p = Params {
            tool: Tool::default(),
            ..Params::default()
        };
        let doc = document_with(vec![square(0.0, 0.0, 10.0)]);
        let why = VcarveInlayToolpath::run(&doc, &p).unwrap_err().to_string();
        assert!(why.contains("V-bit"), "{why}");
    }
}
