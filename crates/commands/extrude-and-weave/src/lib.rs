//! Extrude and Weave: an open cross-section swept along a drive rail. The first vector
//! selected is the rail; the second, open, is the cross-section, laid across the rail with
//! its ends' line as the base and its middle on the rail.

use cut_craft_core::component::{Combine, Component};
use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Point, Rect, polyline_distance};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct ExtrudeAndWeave;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// Scale the shape so its highest point is the height
    pub scale_to_exact_height: bool,
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub height: f64,
    pub name: String,
    #[schemars(title = "Combine with other components")]
    pub combine: Combine,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            scale_to_exact_height: false,
            height: 10.0,
            name: "Extruded".into(),
            combine: Combine::Add,
        }
    }
}

impl Command for ExtrudeAndWeave {
    const ID: &'static str = "extrude_and_weave";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Extrude and Weave",
            group: "Modeling",
            icon: "modeling/extrude_and_weave.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let picked: Vec<_> = doc.selection.iter().map(|id| &doc.vectors[id]).collect();
        let [rail, section] = picked.as_slice() else {
            return Err(CommandError::Failed(
                "select the drive rail, then one open cross-section".into(),
            ));
        };
        if section.closed {
            return Err(CommandError::Failed(
                "the cross-section must be open".into(),
            ));
        }
        let tolerance = cut_craft_core::polygon::TOLERANCE;
        let rail_pts = rail.flatten(tolerance);
        let table = profile(&section.flatten(tolerance))?;
        let half = table.last().map_or(0.0, |e| e.0);
        let b = rail.bounds();
        let area = Rect::new(b.min - Point::splat(half), b.max + Point::splat(half));
        // ponytail: one cross-section, the same all along; Aspire blends between several
        // placed on the rail, and weaves components
        let mut made = Component::from_fn(&p.name, p.combine, area, doc.job.model_cell(), |q| {
            let off = polyline_distance(&rail_pts, q);
            if off > half {
                return None;
            }
            interpolate(&table, off)
        });
        if p.scale_to_exact_height
            && let Some((_, top)) = made.height_range().filter(|r| r.1 > 0.0)
        {
            for h in made.heights.iter_mut().flatten() {
                *h *= p.height / top;
            }
        }
        if made.height_range().is_none() {
            return Err(CommandError::Failed(
                "the cross-section makes no shape".into(),
            ));
        }
        let mut c = Change::builder(doc);
        c.add_component(made);
        Ok(c.finish())
    }
}

/// The cross-section's height over its base line against distance from its middle, folded
/// onto one side (the higher of the two sides at each distance).
fn profile(pts: &[Point]) -> Result<Vec<(f64, f64)>, CommandError> {
    let (a, b) = (pts[0], pts[pts.len() - 1]);
    let width = a.distance(b);
    if width <= 0.0 {
        return Err(CommandError::Failed(
            "the cross-section's ends must be apart".into(),
        ));
    }
    let dir = (b - a) / width;
    let mut table: Vec<(f64, f64)> = pts
        .iter()
        .map(|&q| {
            (
                ((q - a).dot(dir) - width / 2.0).abs(),
                (q - a).perp_dot(dir).abs(),
            )
        })
        .collect();
    table.sort_by(|x, y| x.0.total_cmp(&y.0));
    Ok(table)
}

fn interpolate(table: &[(f64, f64)], x: f64) -> Option<f64> {
    let i = table
        .partition_point(|e| e.0 < x)
        .clamp(1, table.len().checked_sub(1)?);
    let (lo, hi) = (table[i - 1], table[i]);
    let part = if hi.0 > lo.0 {
        ((x - lo.0) / (hi.0 - lo.0)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    Some(lo.1 + (hi.1 - lo.1) * part)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Path;
    use cut_craft_core::testing::{document_with, run_on};

    /// A rail along y = 50 and a triangular cross-section 20 wide, 10 high.
    fn rail_and_section() -> Document {
        document_with(vec![
            Path::polyline(&[Point::new(20.0, 50.0), Point::new(80.0, 50.0)], false),
            Path::polyline(
                &[
                    Point::new(0.0, 0.0),
                    Point::new(10.0, 10.0),
                    Point::new(20.0, 0.0),
                ],
                false,
            ),
        ])
    }

    cut_craft_core::contract_tests!(ExtrudeAndWeave, rail_and_section);

    #[test]
    fn a_triangle_swept_along_a_line_is_a_ridge() {
        let mut doc = rail_and_section();
        run_on::<ExtrudeAndWeave>(&mut doc, &Params::default());
        let k = doc.components.values().next().unwrap();
        let at = |x, y| k.height_at(Point::new(x, y));
        assert!((at(50.0, 50.0).unwrap() - 10.0).abs() < 0.2);
        assert!((at(50.0, 55.0).unwrap() - 5.0).abs() < 0.2);
        assert!(at(50.0, 61.0).is_none(), "past the half-width");
        // the ends round off: 5 mm past the rail's end is 5 down
        assert!((at(85.0, 50.0).unwrap() - 5.0).abs() < 0.2);
    }

    #[test]
    fn needs_a_rail_and_an_open_section() {
        let doc = document_with(vec![cut_craft_core::testing::square(0.0, 0.0, 5.0)]);
        assert!(ExtrudeAndWeave::run(&doc, &Params::default()).is_err());
    }
}
