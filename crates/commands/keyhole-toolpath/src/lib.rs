//! Keyhole Toolpath: hanging slots on the back of a sign. A keyhole cutter plunges at the
//! centre of each selected vector, runs along the slot and back, and lifts out where it went in.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Path, Point};
use cut_craft_core::polygon;
use cut_craft_core::toolpath::{Tool, contour_moves, made_by};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct KeyholeToolpath;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// From the entry hole up the job
    #[default]
    BottomToTop,
    TopToBottom,
    LeftToRight,
    RightToLeft,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
    pub direction: Direction,
    /// Deep enough that the cutter's wide part is below the surface
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub slot_depth: f64,
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub slot_length: f64,
    /// Draw how the slots will look on the surface
    pub create_preview_vectors: bool,
    /// Of the hole the cutter makes going in, for the preview
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub entry_hole_diameter: f64,
    /// Of the slot on the surface, for the preview
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub slot_diameter: f64,
    /// An end mill with the keyhole cutter's feed rates
    pub tool: Tool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Keyhole".into(),
            direction: Direction::BottomToTop,
            slot_depth: 6.35,
            slot_length: 25.4,
            create_preview_vectors: true,
            entry_hole_diameter: 9.5,
            slot_diameter: 4.8,
            tool: Tool::default(),
        }
    }
}

impl Params {
    fn slot_end(&self, start: Point) -> Point {
        let l = self.slot_length;
        start
            + match self.direction {
                Direction::BottomToTop => Point::new(0.0, l),
                Direction::TopToBottom => Point::new(0.0, -l),
                Direction::LeftToRight => Point::new(l, 0.0),
                Direction::RightToLeft => Point::new(-l, 0.0),
            }
    }

    /// The outline of a slot on the surface: the entry hole and a round-ended slot.
    fn preview(&self, start: Point, end: Point) -> Vec<Path> {
        let r = self.slot_diameter / 2.0;
        let side = (end - start).normalize().perp() * r;
        let slot = Path::polyline(&[start + side, end + side, end - side, start - side], true);
        polygon::union(&[
            Path::circle(start, self.entry_hole_diameter / 2.0),
            slot,
            Path::circle(end, r),
        ])
    }
}

impl Command for KeyholeToolpath {
    const ID: &'static str = "keyhole_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Keyhole Toolpath",
            group: "Toolpath Operations",
            icon: "toolpaths/keyhole_toolpath.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("slot length", p.slot_length)?;
        ensure_positive("slot depth", p.slot_depth)?;
        if p.create_preview_vectors {
            ensure_positive("entry hole diameter", p.entry_hole_diameter)?;
            ensure_positive("slot diameter", p.slot_diameter)?;
        }
        let (ids, paths) = doc.selected_paths(|_| true);
        if ids.is_empty() {
            return Err(CommandError::Failed(
                "select circles where the slots go in".into(),
            ));
        }
        let safe = doc.job.clearance;
        let mut moves = vec![];
        let mut previews = vec![];
        for path in &paths {
            let start = path.bounds().center();
            let end = p.slot_end(start);
            moves.extend(contour_moves(
                &[start, end, start],
                false,
                &[-p.slot_depth],
                safe,
            ));
            if p.create_preview_vectors {
                previews.extend(p.preview(start, end));
            }
        }
        let mut c = Change::builder(doc);
        // ponytail: previews go on the current layer; a named preview layer when asked for
        for preview in previews {
            c.add(preview);
        }
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
    use cut_craft_core::testing::{document_with, made_toolpath};
    use cut_craft_core::toolpath::MoveKind;

    cut_craft_core::contract_tests!(KeyholeToolpath);

    #[test]
    fn plunges_at_the_centre_runs_the_slot_and_comes_back_out() {
        let p = Params {
            direction: Direction::LeftToRight,
            create_preview_vectors: false,
            ..Params::default()
        };
        let t =
            made_toolpath::<KeyholeToolpath>(vec![Path::circle(Point::new(50.0, 40.0), 5.0)], &p)
                .unwrap();
        let kinds: Vec<_> = t.moves.iter().map(|m| (m.kind, m.to)).collect();
        let at = |x: f64, z: f64| cut_craft_core::glam::DVec3::new(x, 40.0, z);
        let d = -p.slot_depth;
        assert!(kinds[1].0 == MoveKind::Plunge && kinds[1].1.distance(at(50.0, d)) < 1e-9);
        assert!(kinds[2].1.distance(at(50.0 + p.slot_length, d)) < 1e-9);
        assert!(kinds[3].1.distance(at(50.0, d)) < 1e-9, "back to the entry");
        assert_eq!(kinds[4].0, MoveKind::Rapid, "lifts out where it went in");
        assert!((kinds[4].1.x - 50.0).abs() < 1e-9);
    }

    #[test]
    fn preview_vectors_outline_the_hole_and_slot() {
        let mut doc = document_with(vec![Path::circle(Point::new(50.0, 40.0), 5.0)]);
        let change = KeyholeToolpath::run(&doc, &Params::default()).unwrap();
        doc.apply(&change).unwrap();
        assert_eq!(doc.vectors.len(), 2, "the circle and one keyhole outline");
        let p = Params::default();
        let b = doc
            .vectors
            .values()
            .map(Path::bounds)
            .max_by(|a, b| a.max.y.total_cmp(&b.max.y))
            .unwrap();
        let top = 40.0 + p.slot_length + p.slot_diameter / 2.0;
        assert!((b.max.y - top).abs() < 0.01, "{b:?}");
        assert!(
            (b.min.y - (40.0 - p.entry_hole_diameter / 2.0)).abs() < 0.01,
            "{b:?}"
        );
    }
}
