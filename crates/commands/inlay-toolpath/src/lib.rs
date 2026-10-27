//! Inlay Toolpath: a part and the cavity it fits, cut with one tool. Every corner of the
//! shapes is rounded to the tool's radius first, so the part's outside corners match the
//! cavity's, which the tool cannot cut sharp.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::Path;
use cut_craft_core::polygon::{self, Corners};
use cut_craft_core::toolpath::{Move, Tool, contour_moves, cut_points, made_by, pass_depths};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct InlayToolpath;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InlayType {
    /// The insert, cut out with straight sides
    #[default]
    MaleStraight,
    /// The insert with a shelf round its base (push-through letters)
    MaleStepped,
    /// A pocket for the insert
    FemalePocket,
    /// A hole for the insert to go through
    FemaleHole,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
    pub inlay_type: InlayType,
    /// Below the material surface where cutting starts
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub start_depth: f64,
    /// Below the start depth
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub cut_depth: f64,
    /// Stepped insert: how far down from the start depth the step is
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub step_depth: f64,
    /// Stepped insert: how far out from the shape the shelf reaches
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub step_width: f64,
    /// Makes an insert this much smaller, or a cavity this much bigger, so they fit
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub allowance: f64,
    /// The same tool for both halves
    pub tool: Tool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Inlay".into(),
            inlay_type: InlayType::MaleStraight,
            start_depth: 0.0,
            cut_depth: 6.0,
            step_depth: 3.0,
            step_width: 3.0,
            allowance: 0.0,
            tool: Tool::default(),
        }
    }
}

/// `regions` grown (or shrunk) by `by`.
fn grow(regions: &[Path], by: f64) -> Vec<Path> {
    if by.abs() < 1e-12 {
        return regions.to_vec();
    }
    polygon::offset_regions(regions, by, Corners::Round)
}

/// The shapes with every corner, inside and out, rounded to `radius`: opened (convex corners
/// rounded, as a pocket leaves them) then closed (concave ones, as a cut-out leaves them).
pub fn inlay_shape(regions: &[Path], radius: f64) -> Vec<Path> {
    grow(&grow(&grow(regions, -radius), 2.0 * radius), -radius)
}

impl Params {
    /// Contours round `rings` at each pass down to `depth` below the start.
    fn around(&self, rings: &[Path], from: f64, depth: f64, safe: f64) -> Vec<Move> {
        let depths: Vec<f64> = pass_depths(depth, self.tool.pass_depth)
            .into_iter()
            .map(|z| z - from)
            .collect();
        rings
            .iter()
            .flat_map(|r| contour_moves(&cut_points(r), true, &depths, safe))
            .collect()
    }

    /// Rings stepping in from `outline` (already a tool radius inside the edge) until
    /// nothing is left, middle first, at each pass.
    fn clear(&self, outline: &[Path], depth: f64, safe: f64) -> Vec<Move> {
        let step = self.tool.stepover * self.tool.diameter;
        let mut rings = vec![];
        let mut ring = outline.to_vec();
        while !ring.is_empty() {
            let next = grow(&ring, -step);
            rings.extend(ring);
            ring = next;
        }
        rings.reverse();
        pass_depths(depth, self.tool.pass_depth)
            .into_iter()
            .flat_map(|z| {
                let z = z - self.start_depth;
                rings
                    .iter()
                    .flat_map(move |r| contour_moves(&cut_points(r), true, &[z], safe))
                    .collect::<Vec<_>>()
            })
            .collect()
    }
}

impl Command for InlayToolpath {
    const ID: &'static str = "inlay_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Inlay Toolpath",
            group: "Toolpath Operations",
            icon: "toolpaths/inlay_toolpath.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("cut depth", p.cut_depth)?;
        ensure_positive("tool diameter", p.tool.diameter)?;
        ensure_positive("pass depth", p.tool.pass_depth)?;
        if !(p.tool.stepover > 0.0 && p.tool.stepover <= 1.0) {
            return Err(CommandError::InvalidParams(
                "stepover must be more than 0 and at most 1".into(),
            ));
        }
        if p.inlay_type == InlayType::MaleStepped
            && !(p.step_depth > 0.0 && p.step_depth < p.cut_depth)
        {
            return Err(CommandError::InvalidParams(
                "the step depth must be within the cut depth".into(),
            ));
        }
        let (ids, paths) = doc.selected_paths(|path| path.closed);
        if ids.is_empty() {
            return Err(CommandError::Failed(
                "select the closed vectors to inlay".into(),
            ));
        }
        let r = p.tool.diameter / 2.0;
        let shape = inlay_shape(&polygon::regions(&paths), r);
        if shape.is_empty() {
            return Err(CommandError::Failed(
                "the shapes are too narrow for the tool".into(),
            ));
        }
        let safe = doc.job.clearance;
        let (start, depth) = (p.start_depth, p.cut_depth);
        let moves = match p.inlay_type {
            InlayType::MaleStraight => p.around(&grow(&shape, r - p.allowance), start, depth, safe),
            InlayType::MaleStepped => {
                // the face down to the step, clearing out to the shelf's edge, then the
                // shelf cut out to the full depth
                let face = grow(&shape, -p.allowance);
                let shelf = grow(&face, p.step_width.max(0.0));
                let step = p.tool.stepover * p.tool.diameter;
                let mut rings = vec![];
                let mut out = r;
                while out < p.step_width.max(0.0) + r {
                    rings.extend(grow(&face, out));
                    out += step;
                }
                let mut moves = p.around(&rings, start, p.step_depth, safe);
                moves.extend(p.around(&grow(&shelf, r), start, depth, safe));
                moves
            }
            InlayType::FemalePocket => p.clear(&grow(&shape, p.allowance - r), depth, safe),
            InlayType::FemaleHole => p.around(&grow(&shape, p.allowance - r), start, depth, safe),
        };
        if moves.is_empty() {
            return Err(CommandError::Failed(
                "the tool is too big for the shapes".into(),
            ));
        }
        let mut c = Change::builder(doc);
        c.add_toolpath(made_by::<Self>(
            p,
            p.name.clone(),
            p.tool.clone(),
            ids,
            moves,
        ));
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::document::Job;
    use cut_craft_core::geometry::Point;
    use cut_craft_core::simulation::Material;
    use cut_craft_core::testing::{made_toolpath, square};
    use cut_craft_core::toolpath::Toolpath;

    cut_craft_core::contract_tests!(InlayToolpath);

    fn inlay(kind: InlayType, allowance: f64) -> Toolpath {
        let p = Params {
            inlay_type: kind,
            allowance,
            ..Params::default()
        };
        made_toolpath::<InlayToolpath>(vec![square(20.0, 20.0, 40.0)], &p).unwrap()
    }

    fn carved(t: &Toolpath) -> Material {
        let job = Job {
            width: 80.0,
            height: 80.0,
            ..Job::default()
        };
        let mut m = Material::new(&job, 800);
        m.cut(t);
        m
    }

    #[test]
    fn the_inlay_shape_rounds_every_corner_to_the_tool() {
        let shape = inlay_shape(&polygon::regions(&[square(0.0, 0.0, 20.0)]), 3.0);
        let points = shape[0].flatten(0.01);
        // the square's corner is cut back along the diagonal by r (sqrt 2 - 1)
        let nearest = points
            .iter()
            .map(|q| q.distance(Point::ZERO))
            .fold(f64::INFINITY, f64::min);
        assert!(
            (nearest - 3.0 * (2f64.sqrt() - 1.0)).abs() < 0.05,
            "{nearest}"
        );
    }

    #[test]
    fn a_male_insert_is_cut_out_round_the_shape() {
        let m = carved(&inlay(InlayType::MaleStraight, 0.0));
        assert!(
            m.at(Point::new(40.0, 40.0)).unwrap().abs() < 1e-9,
            "the part is left"
        );
        assert!(
            (m.at(Point::new(18.0, 40.0)).unwrap() + 6.0).abs() < 1e-9,
            "cut round it"
        );
        assert!(
            m.at(Point::new(20.3, 40.0)).unwrap().abs() < 1e-9,
            "the edge is where drawn"
        );
    }

    #[test]
    fn a_female_pocket_clears_the_shape_grown_by_the_allowance() {
        let m = carved(&inlay(InlayType::FemalePocket, 0.5));
        assert!(
            (m.at(Point::new(40.0, 40.0)).unwrap() + 6.0).abs() < 1e-9,
            "cleared"
        );
        assert!(
            (m.at(Point::new(19.7, 40.0)).unwrap() + 6.0).abs() < 1e-9,
            "half a mm over"
        );
        assert!(
            m.at(Point::new(19.3, 40.0)).unwrap().abs() < 1e-9,
            "not more"
        );
    }

    #[test]
    fn a_stepped_insert_has_a_shelf_round_its_base() {
        let m = carved(&inlay(InlayType::MaleStepped, 0.0));
        assert!(
            m.at(Point::new(40.0, 40.0)).unwrap().abs() < 1e-9,
            "the face"
        );
        assert!(
            (m.at(Point::new(18.5, 40.0)).unwrap() + 3.0).abs() < 1e-9,
            "the shelf at the step depth"
        );
        assert!(
            (m.at(Point::new(16.0, 40.0)).unwrap() + 6.0).abs() < 1e-9,
            "cut out beyond it"
        );
    }

    #[test]
    fn a_hole_is_cut_inside() {
        let m = carved(&inlay(InlayType::FemaleHole, 0.0));
        assert!((m.at(Point::new(20.5, 40.0)).unwrap() + 6.0).abs() < 1e-9);
        assert!(
            m.at(Point::new(40.0, 40.0)).unwrap().abs() < 1e-9,
            "the middle drops out"
        );
    }
}
