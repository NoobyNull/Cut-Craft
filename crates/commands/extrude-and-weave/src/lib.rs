//! Extrude and Weave: open cross-sections swept along a drive rail. The first vector
//! selected is the rail; the rest, open, are cross-sections, each laid across the rail with
//! its ends' line as the base and its middle on the rail. One section runs the whole way;
//! several are spaced evenly from the rail's start to its end, in the order picked, and the
//! shape blends from each to the next.

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
            icon: "modeling/extrude_and_weave.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let picked: Vec<_> = doc.selection.iter().map(|id| &doc.vectors[id]).collect();
        let [rail, _, ..] = picked.as_slice() else {
            return Err(CommandError::Failed(
                "select the drive rail, then the open cross-sections".into(),
            ));
        };
        let sections = &picked[1..];
        if sections.iter().any(|s| s.closed) {
            return Err(CommandError::Failed(
                "the cross-sections must be open".into(),
            ));
        }
        let tolerance = cut_craft_core::polygon::TOLERANCE;
        let rail_pts = rail.flatten(tolerance);
        let tables = sections
            .iter()
            .map(|s| profile(&s.flatten(tolerance)))
            .collect::<Result<Vec<_>, _>>()?;
        let half = |t: &[(f64, f64)]| t.last().map_or(0.0, |e| e.0);
        let widest = tables.iter().map(|t| half(t)).fold(0.0, f64::max);
        let b = rail.bounds();
        let area = Rect::new(b.min - Point::splat(widest), b.max + Point::splat(widest));
        // ponytail: sections evenly spaced, blended straight between neighbours; Aspire also
        // lets them be placed at chosen points on the rail, and weaves components
        let mut made = Component::from_fn(&p.name, p.combine, area, doc.job.model_cell(), |q| {
            let (off, along) = nearest(&rail_pts, q);
            #[expect(
                clippy::cast_precision_loss,
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a handful of sections"
            )]
            let (a, b, t) = {
                let at = along * (tables.len() - 1) as f64;
                let i = (at.floor() as usize).min(tables.len().saturating_sub(2));
                (
                    &tables[i],
                    &tables[(i + 1).min(tables.len() - 1)],
                    at - i as f64,
                )
            };
            let width = half(a) + (half(b) - half(a)) * t;
            if off > width || width <= 0.0 {
                return None;
            }
            // the same share of the way out on both sections
            let across = off / width;
            let (ha, hb) = (
                interpolate(a, across * half(a))?,
                interpolate(b, across * half(b))?,
            );
            Some(ha + (hb - ha) * t)
        });
        if p.scale_to_exact_height
            && let Some((_, top)) = made.height_range().filter(|r| r.1 > 0.0)
        {
            for h in made.heights.iter_mut().filter(|h| !h.is_nan()) {
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

/// Distance from `at` to the polyline through `points`, and how far along it, 0 to 1, the
/// nearest point is.
fn nearest(points: &[Point], at: Point) -> (f64, f64) {
    let total: f64 = points.windows(2).map(|w| w[0].distance(w[1])).sum();
    if total <= 0.0 {
        return (polyline_distance(points, at), 0.0);
    }
    let mut walked = 0.0;
    let mut best = (f64::INFINITY, 0.0);
    for w in points.windows(2) {
        let (a, d) = (w[0], w[1] - w[0]);
        let len = d.length();
        let t = if len > 0.0 {
            ((at - a).dot(d) / (len * len)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let off = (a + d * t).distance(at);
        if off < best.0 {
            best = (off, (walked + t * len) / total);
        }
        walked += len;
    }
    best
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
    fn two_sections_blend_from_the_start_to_the_end() {
        // the rail from x 20 to 80; a 10 high triangle at its start, a 20 high one at its end
        let mut doc = document_with(vec![
            Path::polyline(&[Point::new(20.0, 50.0), Point::new(80.0, 50.0)], false),
            Path::polyline(
                &[
                    Point::new(0.0, 0.0),
                    Point::new(10.0, 10.0),
                    Point::new(20.0, 0.0),
                ],
                false,
            ),
            Path::polyline(
                &[
                    Point::new(0.0, 0.0),
                    Point::new(10.0, 20.0),
                    Point::new(20.0, 0.0),
                ],
                false,
            ),
        ]);
        run_on::<ExtrudeAndWeave>(&mut doc, &Params::default());
        let k = doc.components.values().next().unwrap();
        let at = |x, y| k.height_at(Point::new(x, y)).unwrap();
        assert!((at(20.5, 50.0) - 10.0).abs() < 0.5, "{}", at(20.5, 50.0));
        assert!((at(50.0, 50.0) - 15.0).abs() < 0.5, "{}", at(50.0, 50.0));
        assert!((at(79.5, 50.0) - 20.0).abs() < 0.5, "{}", at(79.5, 50.0));
        // halfway out, half as high
        assert!((at(50.0, 55.0) - 7.5).abs() < 0.5, "{}", at(50.0, 55.0));
    }

    #[test]
    fn needs_a_rail_and_an_open_section() {
        let doc = document_with(vec![cut_craft_core::testing::square(0.0, 0.0, 5.0)]);
        assert!(ExtrudeAndWeave::run(&doc, &Params::default()).is_err());
    }
}
