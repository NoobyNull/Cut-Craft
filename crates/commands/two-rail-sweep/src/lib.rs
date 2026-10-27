//! Two Rail Sweep: an open cross-section swept between two rails, stretched across the gap
//! between them all the way along. Select the two rails, then the cross-section.

use cut_craft_core::component::{Combine, Component};
use cut_craft_core::document::Document;
use cut_craft_core::geometry::Point;
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct TwoRailSweep;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[expect(clippy::struct_excessive_bools, reason = "the form's own check boxes")]
pub struct Params {
    #[schemars(title = "Reverse rail 1 direction")]
    pub reverse_rail_1: bool,
    #[schemars(title = "Reverse rail 2 direction")]
    pub reverse_rail_2: bool,
    /// The cross-section's height grows and shrinks with the gap between the rails
    #[schemars(title = "Scale cross section with width")]
    pub scale_with_width: bool,
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
            reverse_rail_1: false,
            reverse_rail_2: false,
            scale_with_width: false,
            scale_to_exact_height: false,
            height: 10.0,
            name: "Two Rail Sweep".into(),
            combine: Combine::Add,
        }
    }
}

impl Command for TwoRailSweep {
    const ID: &'static str = "two_rail_sweep";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Two Rail Sweep",
            group: "Modeling",
            icon: "modeling/two_rail_sweep.svg",
            shortcut: "",
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "sample counts and cell indices are small"
    )]
    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let picked: Vec<_> = doc.selection.iter().map(|id| &doc.vectors[id]).collect();
        let [one, two, section] = picked.as_slice() else {
            return Err(CommandError::Failed(
                "select two rails, then one open cross-section".into(),
            ));
        };
        if section.closed {
            return Err(CommandError::Failed(
                "the cross-section must be open".into(),
            ));
        }
        let tolerance = cut_craft_core::polygon::TOLERANCE;
        let rail = |path: &cut_craft_core::geometry::Path, reverse: bool| {
            let mut pts = path.flatten(tolerance);
            if reverse {
                pts.reverse();
            }
            Walk::new(pts)
        };
        let (r1, r2) = (rail(one, p.reverse_rail_1), rail(two, p.reverse_rail_2));
        let pts = section.flatten(tolerance);
        let (first, last) = (pts[0], pts[pts.len() - 1]);
        let width = first.distance(last);
        if width <= 0.0 || r1.total <= 0.0 || r2.total <= 0.0 {
            return Err(CommandError::Failed(
                "the rails and cross-section need length".into(),
            ));
        }
        let dir = (last - first) / width;
        // the cross-section as height against 0..1 across
        let shape: Vec<(f64, f64)> = {
            let mut t: Vec<_> = pts
                .iter()
                .map(|&q| {
                    (
                        (q - first).dot(dir) / width,
                        (q - first).perp_dot(dir).abs(),
                    )
                })
                .collect();
            t.sort_by(|x, y| x.0.total_cmp(&y.0));
            t
        };
        let cell = doc.job.model_cell();
        let area = one.bounds().union(two.bounds());
        let mut made = Component::from_fn(&p.name, p.combine, area, cell, |_| None);
        // ponytail: dense samples splatted into the cells (max wins); a quad rasteriser
        // if it is ever slow
        let along = ((r1.total.max(r2.total) / (cell / 2.0)).ceil() as usize).max(2);
        let longest_gap = (0..=along)
            .map(|k| {
                r1.at(k as f64 / along as f64)
                    .distance(r2.at(k as f64 / along as f64))
            })
            .fold(0.0, f64::max);
        let across = ((longest_gap / (cell / 2.0)).ceil() as usize).max(2);
        for k in 0..=along {
            let station = k as f64 / along as f64;
            let (p1, p2) = (r1.at(station), r2.at(station));
            let scale = if p.scale_with_width {
                p1.distance(p2) / width
            } else {
                1.0
            };
            for j in 0..=across {
                let part = j as f64 / across as f64;
                let q = p1.lerp(p2, part);
                let local = (q - made.origin) / cell;
                if local.x < 0.0 || local.y < 0.0 {
                    continue;
                }
                let (col, row) = (local.x as usize, local.y as usize);
                if col >= made.columns || row >= made.rows {
                    continue;
                }
                let z = lookup(&shape, part) * scale;
                let h = &mut made.heights[row * made.columns + col];
                if h.is_nan() || z > *h {
                    *h = z;
                }
            }
        }
        if p.scale_to_exact_height
            && let Some((_, top)) = made.height_range().filter(|r| r.1 > 0.0)
        {
            for h in made.heights.iter_mut().filter(|h| !h.is_nan()) {
                *h *= p.height / top;
            }
        }
        let mut c = Change::builder(doc);
        c.add_component(made);
        Ok(c.finish())
    }
}

/// A polyline walked by the share of its length.
struct Walk {
    pts: Vec<Point>,
    lengths: Vec<f64>,
    total: f64,
}

impl Walk {
    fn new(pts: Vec<Point>) -> Self {
        let mut lengths = vec![0.0];
        for pair in pts.windows(2) {
            lengths.push(lengths[lengths.len() - 1] + pair[0].distance(pair[1]));
        }
        let total = lengths[lengths.len() - 1];
        Self {
            pts,
            lengths,
            total,
        }
    }

    fn at(&self, share: f64) -> Point {
        let d = share * self.total;
        let i = self
            .lengths
            .partition_point(|&l| l < d)
            .clamp(1, self.pts.len() - 1);
        let span = self.lengths[i] - self.lengths[i - 1];
        let part = if span > 0.0 {
            (d - self.lengths[i - 1]) / span
        } else {
            0.0
        };
        self.pts[i - 1].lerp(self.pts[i], part)
    }
}

fn lookup(table: &[(f64, f64)], x: f64) -> f64 {
    let i = table.partition_point(|e| e.0 < x).clamp(1, table.len() - 1);
    let (lo, hi) = (table[i - 1], table[i]);
    let part = if hi.0 > lo.0 {
        ((x - lo.0) / (hi.0 - lo.0)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    lo.1 + (hi.1 - lo.1) * part
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Path;
    use cut_craft_core::testing::{document_with, run_on};

    /// Rails 20 apart at the left, 40 apart at the right; a half-circle-ish cross-section.
    fn rails() -> Document {
        let mut doc = document_with(vec![
            Path::polyline(&[Point::new(0.0, 40.0), Point::new(100.0, 30.0)], false),
            Path::polyline(&[Point::new(0.0, 60.0), Point::new(100.0, 70.0)], false),
            Path::polyline(
                &[
                    Point::new(0.0, 0.0),
                    Point::new(5.0, 5.0),
                    Point::new(10.0, 0.0),
                ],
                false,
            ),
        ]);
        doc.job.width = 100.0;
        doc.job.height = 100.0;
        doc
    }

    cut_craft_core::contract_tests!(TwoRailSweep, rails);

    #[test]
    fn the_section_stretches_between_the_rails() {
        let mut doc = rails();
        run_on::<TwoRailSweep>(&mut doc, &Params::default());
        let k = doc.components.values().next().unwrap();
        // midway between the rails it is the section's peak, 5 high
        assert!((k.height_at(Point::new(10.0, 50.0)).unwrap() - 5.0).abs() < 0.2);
        assert!((k.height_at(Point::new(90.0, 50.0)).unwrap() - 5.0).abs() < 0.2);
        assert!(
            k.height_at(Point::new(90.0, 75.0)).is_none(),
            "outside the rails"
        );
    }

    #[test]
    fn scaling_with_width_makes_the_wide_end_taller() {
        let mut doc = rails();
        let p = Params {
            scale_with_width: true,
            ..Params::default()
        };
        run_on::<TwoRailSweep>(&mut doc, &p);
        let k = doc.components.values().next().unwrap();
        let narrow = k.height_at(Point::new(1.0, 50.0)).unwrap();
        let wide = k.height_at(Point::new(99.0, 50.0)).unwrap();
        assert!(
            (narrow - 10.0).abs() < 0.5 && (wide - 20.0).abs() < 0.5,
            "{narrow} {wide}"
        );
    }
}
