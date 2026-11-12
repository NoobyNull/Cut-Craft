//! Two Rail Sweep: open cross-sections swept between two rails, stretched across the gap
//! between them all the way along. Select the two rails, then the cross-sections: one is
//! used all the way; several are each placed where they were drawn along the first rail and
//! the shape blends from one to the next. Add to All Rail Nodes ties the rails' nodes in
//! pairs (corners meet corners), and closed rails can have the inner one's centre filled.

use cut_craft_core::component::{Combine, Component};
use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Path, Point};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct TwoRailSweep;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[expect(clippy::struct_excessive_bools, reason = "the form's own check boxes")]
pub struct Params {
    /// Run along the first rail from its other end
    #[schemars(title = "Reverse rail 1 direction")]
    pub reverse_rail_1: bool,
    /// Run along the second rail from its other end
    #[schemars(title = "Reverse rail 2 direction")]
    pub reverse_rail_2: bool,
    /// The cross-section's height grows and shrinks with the gap between the rails
    #[schemars(title = "Scale cross section with width")]
    pub scale_with_width: bool,
    /// Scale the shape so its highest point is the height
    pub scale_to_exact_height: bool,
    /// The highest point of the shape, with Scale to exact height
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub height: f64,
    /// What the new component is called in the component tree
    pub name: String,
    #[schemars(title = "Combine with other components")]
    pub combine: Combine,
    /// The second rail taken first, so the cross-sections hang the other way
    #[serde(default)]
    pub swap_rail_order: bool,
    /// The rails' nodes joined in pairs, first to first and so on, so corners meet corners
    /// (the rails need as many nodes as each other)
    #[serde(default)]
    #[schemars(title = "Add to all rail nodes")]
    pub add_to_all_rail_nodes: bool,
    /// Closed rails: the inside of the inner one filled to the height the cross-sections
    /// reach at it
    #[serde(default)]
    #[schemars(title = "Fill center of inner closed vector rails")]
    pub fill_centre: bool,
    /// Between cross-sections with as many nodes as each other, each node flows to the same
    /// node of the next
    #[serde(default)]
    pub sweep_between_spans: bool,
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
            swap_rail_order: false,
            add_to_all_rail_nodes: false,
            fill_centre: false,
            sweep_between_spans: false,
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
            icon: "modeling/two_rail_sweep.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let picked: Vec<&Path> = doc.selection.iter().map(|id| &doc.vectors[id]).collect();
        let [one, two, sections @ ..] = picked.as_slice() else {
            return Err(select_them());
        };
        if sections.is_empty() {
            return Err(select_them());
        }
        if sections.iter().any(|s| s.closed) {
            return Err(CommandError::Failed(
                "the cross-sections must be open".into(),
            ));
        }
        let (one, two, reverse_1, reverse_2) = if p.swap_rail_order {
            (two, one, p.reverse_rail_2, p.reverse_rail_1)
        } else {
            (one, two, p.reverse_rail_1, p.reverse_rail_2)
        };
        let (r1, r2) = (Rail::new(one, reverse_1), Rail::new(two, reverse_2));
        if r1.walk.total <= 0.0 || r2.walk.total <= 0.0 {
            return Err(CommandError::Failed(
                "A rail is a single point. Draw each rail as a line or curve.".into(),
            ));
        }
        let pairs = if p.add_to_all_rail_nodes {
            let (a, b) = (r1.node_shares(), r2.node_shares());
            if a.len() != b.len() {
                return Err(CommandError::Failed(format!(
                    "to add to all rail nodes the rails need as many nodes as each other \
                     (they have {} and {})",
                    a.len(),
                    b.len()
                )));
            }
            Some((a, b))
        } else {
            None
        };
        let other = |station: f64| match &pairs {
            Some((a, b)) => remap(a, b, station),
            None => station,
        };
        // one cross-section is used all the way; several where each was drawn
        let mut placed = sections
            .iter()
            .map(|path| {
                let section = Section::new(path)?;
                Some((r1.walk.share_nearest(section.middle), section))
            })
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| {
                CommandError::Failed(
                    "the cross-sections need width: draw them upright, across from end to end"
                        .into(),
                )
            })?;
        placed.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut made = sweep(doc, p, (one, &r1), (two, &r2), &other, &placed);
        if p.fill_centre && one.closed && two.closed {
            fill_centre(&mut made, (one, two), &placed);
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

fn select_them() -> CommandError {
    CommandError::Failed("select two rails, then one or more open cross-sections".into())
}

/// The cross-sections splatted between the rails, station by station along the first rail
/// (the second's station from `other`), blending between the sections `placed` there.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "sample counts and cell indices are small"
)]
fn sweep(
    doc: &Document,
    p: &Params,
    (one, r1): (&Path, &Rail),
    (two, r2): (&Path, &Rail),
    other: &dyn Fn(f64) -> f64,
    placed: &[(f64, Section)],
) -> Component {
    let cell = doc.job.model_cell();
    let area = one.bounds().union(two.bounds());
    let mut made = Component::from_fn(&p.name, p.combine, area, cell, |_| None);
    // ponytail: dense samples splatted into the cells (max wins); a quad rasteriser if it is
    // ever slow
    let along = ((r1.walk.total.max(r2.walk.total) / (cell / 2.0)).ceil() as usize).max(2);
    let ends = |k: usize| {
        let station = k as f64 / along as f64;
        (station, r1.walk.at(station), r2.walk.at(other(station)))
    };
    let longest_gap = (0..=along)
        .map(|k| ends(k).1.distance(ends(k).2))
        .fold(0.0, f64::max);
    let across = ((longest_gap / (cell / 2.0)).ceil() as usize).max(2);
    for k in 0..=along {
        let (station, p1, p2) = ends(k);
        let gap = p1.distance(p2);
        for j in 0..=across {
            let part = j as f64 / across as f64;
            let local = (p1.lerp(p2, part) - made.origin) / cell;
            if local.x < 0.0 || local.y < 0.0 {
                continue;
            }
            let (col, row) = (local.x as usize, local.y as usize);
            if col >= made.columns || row >= made.rows {
                continue;
            }
            let how = (
                gap.max(f64::EPSILON),
                p.scale_with_width,
                p.sweep_between_spans,
            );
            let z = height(placed, station, part, how);
            let h = &mut made.heights[row * made.columns + col];
            if h.is_nan() || z > *h {
                *h = z;
            }
        }
    }
    made
}

/// The height `part` of the way across at `station` along the first rail, the rails `gap`
/// apart: the placed cross-sections' blended, each scaled to the gap when `scaled`, and
/// (`spans`, sections with as many nodes) node to node rather than at the same `part`.
pub fn height(
    placed: &[(f64, Section)],
    station: f64,
    part: f64,
    (gap, scaled, spans): (f64, bool, bool),
) -> f64 {
    let at = |s: &Section, part: f64| {
        let scale = if scaled { gap / s.width } else { 1.0 };
        lookup(&s.shape, part) * scale
    };
    let next = placed.partition_point(|(s, _)| *s < station);
    match (next.checked_sub(1).map(|i| &placed[i]), placed.get(next)) {
        (Some((a, before)), Some((b, after))) if b > a => {
            let t = (station - a) / (b - a);
            let (from, to) = if spans && before.nodes.len() == after.nodes.len() {
                // the same share of the way along the same span of each
                let blended: Vec<f64> = (before.nodes.iter().zip(&after.nodes))
                    .map(|(x, y)| x + (y - x) * t)
                    .collect();
                (
                    remap(&blended, &before.nodes, part),
                    remap(&blended, &after.nodes, part),
                )
            } else {
                (part, part)
            };
            at(before, from) + (at(after, to) - at(before, from)) * t
        }
        (Some((_, only)), _) | (None, Some((_, only))) => at(only, part),
        (None, None) => 0.0,
    }
}

/// The inner closed rail's inside filled to the height the cross-sections reach at it.
#[expect(clippy::cast_precision_loss, reason = "cell indices are small")]
fn fill_centre(made: &mut Component, (one, two): (&Path, &Path), placed: &[(f64, Section)]) {
    let tolerance = cut_craft_core::polygon::TOLERANCE;
    let first_inner =
        cut_craft_core::polygon::area(one).abs() < cut_craft_core::polygon::area(two).abs();
    let (inner, part) = if first_inner { (one, 0.0) } else { (two, 1.0) };
    // ponytail: the cross-sections' mean height at the inner rail; per station if they vary
    let level = placed
        .iter()
        .map(|(_, s)| lookup(&s.shape, part))
        .sum::<f64>()
        / placed.len() as f64;
    let ring = [inner.flatten(tolerance)];
    for row in 0..made.rows {
        for col in 0..made.columns {
            let centre = made.origin + Point::new(col as f64 + 0.5, row as f64 + 0.5) * made.cell;
            let h = &mut made.heights[row * made.columns + col];
            if h.is_nan() && cut_craft_core::geometry::inside(&ring, centre) {
                *h = level;
            }
        }
    }
}

/// An open cross-section, drawn upright as Aspire's are (or on its side): its height
/// above its lowest point against 0..1 across from its first end to its last, its width
/// end to end across, the middle of its ends (where it was placed), and its nodes' shares
/// across.
#[derive(Clone, Debug)]
pub struct Section {
    shape: Vec<(f64, f64)>,
    pub width: f64,
    pub middle: Point,
    /// Each node's share of the way across, in order.
    nodes: Vec<f64>,
}

impl Section {
    pub fn new(path: &Path) -> Option<Self> {
        let pts = path.flatten(cut_craft_core::polygon::TOLERANCE);
        let (first, last) = (*pts.first()?, *pts.last()?);
        let chord = last - first;
        // across the way its ends are apart; up is Y for an upright one, else the side of X
        // it stands out to (an unwrapped profile lies along Y)
        let (across, up) = if chord.x.abs() >= chord.y.abs() {
            (Point::new(chord.x.signum(), 0.0), Point::Y)
        } else {
            let mean_out = pts.iter().map(|q| q.x - first.x).sum::<f64>();
            let side = if mean_out < 0.0 { -1.0 } else { 1.0 };
            (Point::new(0.0, chord.y.signum()), Point::new(side, 0.0))
        };
        let width = chord.dot(across);
        if width <= f64::EPSILON {
            return None;
        }
        let low = pts.iter().map(|q| q.dot(up)).fold(f64::INFINITY, f64::min);
        let part_of = |q: Point| (q - first).dot(across) / width;
        let mut shape: Vec<_> = pts.iter().map(|&q| (part_of(q), q.dot(up) - low)).collect();
        shape.sort_by(|x, y| x.0.total_cmp(&y.0));
        Some(Self {
            shape,
            width,
            middle: first.lerp(last, 0.5),
            nodes: cut_craft_core::nodes::nodes(path)
                .into_iter()
                .map(part_of)
                .collect(),
        })
    }
}

/// A rail: its path walked from its start (or end, reversed), and its nodes in that order.
struct Rail {
    walk: Walk,
    nodes: Vec<Point>,
    closed: bool,
}

impl Rail {
    fn new(path: &Path, reverse: bool) -> Self {
        let mut pts = path.flatten(cut_craft_core::polygon::TOLERANCE);
        let mut nodes = cut_craft_core::nodes::nodes(path);
        if reverse {
            pts.reverse();
            nodes.reverse();
            if path.closed {
                // the start node stays first going the other way round
                nodes.rotate_right(1);
            }
        }
        Self {
            walk: Walk::new(pts),
            nodes,
            closed: path.closed,
        }
    }

    /// Each node's share of the way along, in order (a closed rail's start again at 1).
    fn node_shares(&self) -> Vec<f64> {
        let mut shares = vec![0.0];
        shares.extend(
            self.nodes
                .iter()
                .skip(1)
                .map(|&n| self.walk.share_nearest(n)),
        );
        if self.closed {
            shares.push(1.0);
        }
        shares
    }
}

/// `station` along the first rail, mapped onto the second by the paired shares `a` and `b`.
pub fn remap(a: &[f64], b: &[f64], station: f64) -> f64 {
    let i = a.partition_point(|&s| s <= station).clamp(1, a.len() - 1);
    let span = a[i] - a[i - 1];
    let t = if span > 0.0 {
        (station - a[i - 1]) / span
    } else {
        0.0
    };
    b[i - 1] + (b[i] - b[i - 1]) * t
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

    /// The share of the way along of the point nearest `p`.
    fn share_nearest(&self, p: Point) -> f64 {
        let mut best = (f64::INFINITY, 0.0);
        for (i, pair) in self.pts.windows(2).enumerate() {
            let d = pair[1] - pair[0];
            let t = if d.length_squared() > 0.0 {
                ((p - pair[0]).dot(d) / d.length_squared()).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let gap = p.distance(pair[0] + d * t);
            if gap < best.0 {
                best = (gap, self.lengths[i] + d.length() * t);
            }
        }
        if self.total > 0.0 {
            best.1 / self.total
        } else {
            0.0
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

    fn polyline(points: &[(f64, f64)], closed: bool) -> Path {
        let points: Vec<Point> = points.iter().map(|&(x, y)| Point::new(x, y)).collect();
        Path::polyline(&points, closed)
    }

    fn swept(doc: &Document, p: &Params) -> Component {
        let mut doc = doc.clone();
        run_on::<TwoRailSweep>(&mut doc, p);
        doc.components.values().next().unwrap().clone()
    }

    #[test]
    fn several_sections_are_placed_where_drawn_and_blended_between() {
        // straight rails 20 apart; a low arch drawn near the left, a tall one near the right
        let mut doc = document_with(vec![
            polyline(&[(0.0, 40.0), (100.0, 40.0)], false),
            polyline(&[(0.0, 60.0), (100.0, 60.0)], false),
            polyline(&[(0.0, 45.0), (5.0, 47.0), (10.0, 45.0)], false),
            polyline(&[(90.0, 45.0), (95.0, 53.0), (100.0, 45.0)], false),
        ]);
        (doc.job.width, doc.job.height) = (100.0, 100.0);
        let k = swept(&doc, &Params::default());
        let peak = |x| k.height_at(Point::new(x, 50.0)).unwrap();
        assert!((peak(3.0) - 2.0).abs() < 0.3, "the low one: {}", peak(3.0));
        assert!(
            (peak(97.0) - 8.0).abs() < 0.3,
            "the tall one: {}",
            peak(97.0)
        );
        assert!(
            (peak(50.0) - 5.0).abs() < 0.3,
            "halfway between: {}",
            peak(50.0)
        );
    }

    #[test]
    fn sweeping_between_spans_carries_each_node_to_the_same_node_of_the_next() {
        // a peak a quarter across at the left, three quarters across at the right
        let mut doc = document_with(vec![
            polyline(&[(0.0, 40.0), (100.0, 40.0)], false),
            polyline(&[(0.0, 60.0), (100.0, 60.0)], false),
            polyline(&[(0.0, 45.0), (5.0, 50.0), (20.0, 45.0)], false),
            polyline(&[(80.0, 45.0), (95.0, 50.0), (100.0, 45.0)], false),
        ]);
        (doc.job.width, doc.job.height) = (100.0, 100.0);
        let p = Params {
            sweep_between_spans: true,
            ..Params::default()
        };
        let k = swept(&doc, &p);
        // halfway along, the peak has moved halfway across: the middle, still 5 high
        let mid = k.height_at(Point::new(50.0, 50.0)).unwrap();
        assert!((mid - 5.0).abs() < 0.3, "the peak travels: {mid}");
        let plain = swept(&doc, &Params::default());
        let blended = plain.height_at(Point::new(50.0, 50.0)).unwrap();
        assert!(blended < 4.5, "blended in place it sags: {blended}");
    }

    #[test]
    fn rail_nodes_pair_up_only_when_the_rails_have_as_many() {
        assert!((remap(&[0.0, 0.2, 1.0], &[0.0, 0.8, 1.0], 0.2) - 0.8).abs() < 1e-12);
        assert!((remap(&[0.0, 0.2, 1.0], &[0.0, 0.8, 1.0], 0.6) - 0.9).abs() < 1e-12);
        let mut doc = rails();
        let p = Params {
            add_to_all_rail_nodes: true,
            ..Params::default()
        };
        run_on::<TwoRailSweep>(&mut doc, &p);
        let mut uneven = rails();
        let rail = *uneven.vectors.keys().next().unwrap();
        uneven.vectors.insert(
            rail,
            polyline(&[(0.0, 40.0), (50.0, 35.0), (100.0, 30.0)], false),
        );
        let why = TwoRailSweep::run(&uneven, &p).unwrap_err().to_string();
        assert!(why.contains("as many nodes"), "{why}");
    }

    #[test]
    fn closed_rails_fill_the_inner_centre_and_swapping_hangs_the_section_the_other_way() {
        // a frame: outer square, inner square, a ramp rising to 5 at its right-hand end
        let mut doc = document_with(vec![
            polyline(
                &[(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)],
                true,
            ),
            polyline(
                &[(20.0, 20.0), (80.0, 20.0), (80.0, 80.0), (20.0, 80.0)],
                true,
            ),
            polyline(&[(0.0, 0.0), (10.0, 1.0), (20.0, 5.0)], false),
        ]);
        (doc.job.width, doc.job.height) = (100.0, 100.0);
        let open = swept(&doc, &Params::default());
        assert!(
            open.height_at(Point::new(50.0, 50.0)).is_none(),
            "nothing inside"
        );
        let filled = swept(
            &doc,
            &Params {
                fill_centre: true,
                ..Params::default()
            },
        );
        let centre = filled.height_at(Point::new(50.0, 50.0)).unwrap();
        assert!(
            (centre - 5.0).abs() < 0.2,
            "filled to the inner end: {centre}"
        );
        // by the outer edge the ramp is low, unless the rails are swapped
        let edge = |k: &Component| k.height_at(Point::new(50.0, 1.5)).unwrap();
        let swapped = swept(
            &doc,
            &Params {
                swap_rail_order: true,
                ..Params::default()
            },
        );
        assert!(
            edge(&open) < 1.0 && edge(&swapped) > 4.0,
            "{} {}",
            edge(&open),
            edge(&swapped)
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
