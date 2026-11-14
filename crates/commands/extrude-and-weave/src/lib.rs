//! Extrude and Weave: a shape swept along drive rails. The first vectors selected are the
//! rails (as many as the form says); the open vectors after them are cross-sections, each
//! hung across the rail nearest it where it was drawn (its first end on the rail's left),
//! and the shape blends from one to the next along the rail. Or a component (the selected
//! one) is stretched along each rail, or repeated along it with overlap and alternate copies
//! flipped. Corners are rounded, or sharp (mitred); where rails cross, Weave raises one and
//! lowers the other, alternately along each rail.

use cmd_two_rail_sweep::{Section, height as section_height};
use cut_craft_core::component::{Combine, Component};
use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Path, Point, Rect};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct ExtrudeAndWeave;

/// What is swept along the rails.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Sweep {
    /// The open vectors selected after the rails
    #[default]
    #[schemars(title = "Use vector cross sections")]
    CrossSections,
    /// The selected component, its X along the rail
    #[schemars(title = "Use a component")]
    Component,
}

/// How a component follows a rail.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Copies {
    /// One copy stretched along the whole rail
    #[default]
    #[schemars(title = "Stretch to drive rail length")]
    Stretch,
    /// Copies at its own size, one after another
    #[schemars(title = "Fill with multiple copies")]
    Fill,
}

/// How a weave raises and lowers the shape where rails cross.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WeaveBy {
    /// The shape's height scaled by the percentages
    #[default]
    ScaleShape,
    /// The shape kept, lifted on a base of the percentages of its height
    AddBase,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[expect(clippy::struct_excessive_bools, reason = "the form's own check boxes")]
pub struct Params {
    /// How many of the vectors selected first are drive rails
    #[serde(default = "one")]
    #[schemars(title = "Drive rails", range(min = 1))]
    pub rails: u32,
    #[serde(default)]
    pub sweep: Sweep,
    #[serde(default)]
    #[schemars(extend("x-show-when" = {"sweep": "component"}))]
    pub copies: Copies,
    /// Each copy overlaps the one before by this share of its length
    #[serde(default)]
    #[schemars(
        title = "Overlap",
        extend("x-unit" = "%"),
        range(min = 0, max = 90),
        extend("x-show-when" = {"copies": "fill"})
    )]
    pub overlap: f64,
    /// Every other copy mirrored across the rail
    #[serde(default)]
    #[schemars(extend("x-show-when" = {"copies": "fill"}))]
    pub flip_alternate_copies: bool,
    /// Corners in the rails make sharp corners in the shape, not rounded ones
    #[serde(default)]
    #[schemars(title = "Create sharp corners")]
    pub sharp_corners: bool,
    /// Between cross-sections with as many nodes as each other, each node flows to the same
    /// node of the next
    #[serde(default)]
    pub sweep_between_spans: bool,
    /// Where rails cross, one goes over and the other under, alternately along each rail
    #[serde(default)]
    #[schemars(title = "Weave under/over at crossings")]
    pub weave: bool,
    #[serde(default)]
    #[schemars(extend("x-show-when" = {"weave": true}))]
    pub weave_by: WeaveBy,
    /// Where rails cross, the height of the one going under, as a percentage of its own
    #[serde(default = "under")]
    #[schemars(
        title = "Z Under",
        extend("x-unit" = "%"),
        range(min = 0),
        extend("x-show-when" = {"weave": true})
    )]
    pub z_under: f64,
    /// Where rails cross, the height of the one going over, as a percentage of its own
    #[serde(default = "over")]
    #[schemars(
        title = "Z Over",
        extend("x-unit" = "%"),
        range(min = 0),
        extend("x-show-when" = {"weave": true})
    )]
    pub z_over: f64,
    /// Scale the shape so its highest point is the height
    pub scale_to_exact_height: bool,
    /// The highest point of the shape, with Scale to exact height
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    #[schemars(extend("x-dim" = "H", "x-diagram" = "diagrams/height_only.png"))]
    pub height: f64,
    /// What the new component is called in the component tree
    pub name: String,
    #[schemars(title = "Combine with other components")]
    pub combine: Combine,
}

fn one() -> u32 {
    1
}

fn under() -> f64 {
    70.0
}

fn over() -> f64 {
    130.0
}

impl Default for Params {
    fn default() -> Self {
        Self {
            rails: 1,
            sweep: Sweep::CrossSections,
            copies: Copies::Stretch,
            overlap: 0.0,
            flip_alternate_copies: false,
            sharp_corners: false,
            sweep_between_spans: false,
            weave: false,
            weave_by: WeaveBy::ScaleShape,
            z_under: under(),
            z_over: over(),
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
        let picked: Vec<&Path> = doc.selection.iter().map(|id| &doc.vectors[id]).collect();
        let count = usize::try_from(p.rails.max(1)).unwrap_or(usize::MAX);
        if picked.len() < count {
            return Err(CommandError::Failed(format!(
                "select the {count} drive rail(s), then the cross-sections"
            )));
        }
        let (rails, sections) = picked.split_at(count);
        let rails: Vec<Rail> = rails.iter().map(|path| Rail::new(path)).collect();
        if rails.iter().any(|r| r.total <= 0.0) {
            return Err(CommandError::Failed(
                "A drive rail is a single point. Draw each drive rail as a line or curve.".into(),
            ));
        }
        let shape = match p.sweep {
            Sweep::CrossSections => Shape::Sections(placed(&rails, sections)?),
            Sweep::Component => Shape::Component(Box::new(component(doc)?)),
        };
        let weaves = if p.weave {
            weaves(&rails, p)
        } else {
            vec![vec![]; rails.len()]
        };
        let half = shape.half();
        let b = rails
            .iter()
            .map(|r| r.bounds)
            .reduce(Rect::union)
            .expect("at least one rail");
        let area = Rect::new(b.min - Point::splat(half), b.max + Point::splat(half));
        let tall = shape.tallest();
        let mut made = Component::from_fn(&p.name, p.combine, area, doc.job.model_cell(), |q| {
            rails
                .iter()
                .zip(&weaves)
                .enumerate()
                .filter_map(|(i, (rail, weave))| {
                    let (along, off) = rail.nearest(q, p.sharp_corners)?;
                    if off.abs() > half {
                        return None;
                    }
                    let h = shape.height(i, rail, along, off, p)?;
                    let lift = weave_at(weave, along / rail.total);
                    Some(match (p.weave, p.weave_by) {
                        (false, _) => h,
                        (true, WeaveBy::ScaleShape) => h * lift,
                        (true, WeaveBy::AddBase) => h + lift * tall,
                    })
                })
                .reduce(f64::max)
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
                "The cross-section makes no shape. Draw it as an open curve that rises away from \
                 the line between its ends."
                    .into(),
            ));
        }
        let mut c = Change::builder(doc);
        c.add_component(made);
        Ok(c.finish())
    }
}

/// What is swept: cross-sections placed on each rail (by rail, at their share along it),
/// or a component.
enum Shape {
    Sections(Vec<Vec<(f64, Section)>>),
    Component(Box<Component>),
}

impl Shape {
    /// How far either side of a rail it reaches.
    fn half(&self) -> f64 {
        match self {
            Self::Sections(by_rail) => by_rail
                .iter()
                .flatten()
                .map(|(_, s)| s.width / 2.0)
                .fold(0.0, f64::max),
            Self::Component(k) => k.extent().size().y / 2.0,
        }
    }

    /// Its highest point, for a weave's base.
    #[expect(clippy::cast_precision_loss, reason = "a small sample count")]
    fn tallest(&self) -> f64 {
        match self {
            Self::Sections(by_rail) => by_rail
                .iter()
                .flatten()
                .map(|(_, s)| {
                    (0..=SAMPLES)
                        .map(|i| {
                            section_height(
                                &[(0.0, s.clone())],
                                0.0,
                                i as f64 / SAMPLES as f64,
                                (1.0, false, false),
                            )
                        })
                        .fold(0.0, f64::max)
                })
                .fold(0.0, f64::max),
            Self::Component(k) => k.height_range().map_or(0.0, |r| r.1),
        }
    }

    /// Its height `off` to the left of rail `index` (`rail`), `along` it.
    fn height(&self, index: usize, rail: &Rail, along: f64, off: f64, p: &Params) -> Option<f64> {
        let half = self.half();
        match self {
            Self::Sections(by_rail) => {
                // the first end on the rail's left, the last on its right
                let part = (half - off) / (2.0 * half);
                let how = (1.0, false, p.sweep_between_spans);
                Some(section_height(
                    &by_rail[index],
                    along / rail.total,
                    part,
                    how,
                ))
            }
            Self::Component(k) => {
                let e = k.extent();
                let length = e.size().x;
                let at = |u: f64, flip: bool| {
                    let across = if flip { -off } else { off };
                    k.height_at(Point::new(e.min.x + u, e.center().y + across))
                };
                match p.copies {
                    Copies::Stretch => at(along / rail.total * length, false),
                    Copies::Fill => {
                        let step = length * (1.0 - p.overlap.clamp(0.0, 90.0) / 100.0);
                        copies(along, rail.total, length, step)
                            .filter_map(|(k, start)| {
                                at(along - start, p.flip_alternate_copies && k % 2 == 1)
                            })
                            .reduce(f64::max)
                    }
                }
            }
        }
    }
}

/// How finely a cross-section is sampled for its highest point.
const SAMPLES: usize = 64;

/// The copies of a `length`-long component every `step` along a rail `total` long that
/// cover `along`: each one's number and where it starts.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "copy counts are small"
)]
fn copies(along: f64, total: f64, length: f64, step: f64) -> impl Iterator<Item = (usize, f64)> {
    let first = ((along - length) / step).ceil().max(0.0) as usize;
    let last = (along / step).floor().max(0.0) as usize;
    (first..=last)
        .map(move |k| (k, k as f64 * step))
        .filter(move |&(_, start)| start <= total)
}

/// The selected component, to sweep along the rails.
fn component(doc: &Document) -> Result<Component, CommandError> {
    doc.selected_components
        .iter()
        .find_map(|id| doc.components.get(id))
        .cloned()
        .ok_or_else(|| {
            CommandError::Failed("select the component to extrude along the rails".into())
        })
}

/// The cross-sections, each on the rail nearest it, where it was drawn along it; one
/// section serves every rail, and a rail with none takes the first.
fn placed(rails: &[Rail], sections: &[&Path]) -> Result<Vec<Vec<(f64, Section)>>, CommandError> {
    if sections.is_empty() {
        return Err(CommandError::Failed(
            "select the drive rail(s), then one or more open cross-sections".into(),
        ));
    }
    if sections.iter().any(|s| s.closed) {
        return Err(CommandError::Failed(
            "the cross-sections must be open".into(),
        ));
    }
    let sections: Vec<Section> = sections
        .iter()
        .map(|s| Section::new(s))
        .collect::<Option<_>>()
        .ok_or_else(|| CommandError::Failed("the cross-section starts and ends at the same point. Draw it so its ends are apart".into()))?;
    let mut by_rail: Vec<Vec<(f64, Section)>> = vec![vec![]; rails.len()];
    if let [only] = sections.as_slice() {
        for list in &mut by_rail {
            list.push((0.0, only.clone()));
        }
        return Ok(by_rail);
    }
    for s in &sections {
        let (i, along) = rails
            .iter()
            .enumerate()
            .filter_map(|(i, r)| {
                let (along, off) = r.nearest(s.middle, false)?;
                Some((off.abs(), i, along / r.total))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, i, along)| (i, along))
            .expect("there are rails");
        by_rail[i].push((along, s.clone()));
    }
    for list in &mut by_rail {
        if list.is_empty() {
            list.push((0.0, sections[0].clone()));
        }
        list.sort_by(|a, b| a.0.total_cmp(&b.0));
    }
    Ok(by_rail)
}

/// A drive rail, flattened, with the length walked to each point.
struct Rail {
    pts: Vec<Point>,
    lengths: Vec<f64>,
    total: f64,
    closed: bool,
    bounds: Rect,
}

impl Rail {
    fn new(path: &Path) -> Self {
        let pts = path.flatten(cut_craft_core::polygon::TOLERANCE);
        let mut lengths = vec![0.0];
        for pair in pts.windows(2) {
            lengths.push(lengths[lengths.len() - 1] + pair[0].distance(pair[1]));
        }
        Self {
            total: lengths[lengths.len() - 1],
            lengths,
            pts,
            closed: path.closed,
            bounds: path.bounds(),
        }
    }

    /// The length along the rail of the point nearest `q`, and how far `q` is from it, to
    /// the left (positive) or right. With `sharp` corners a point off a corner's outside is
    /// as far as from the farther of the two spans' lines (a mitre), and nothing reaches
    /// past an open rail's ends.
    fn nearest(&self, q: Point, sharp: bool) -> Option<(f64, f64)> {
        let mut best: Option<(f64, f64, f64)> = None;
        let spans = self.pts.len() - 1;
        for i in 0..spans {
            let (a, d) = (self.pts[i], self.pts[i + 1] - self.pts[i]);
            let length = d.length();
            if length <= 0.0 {
                continue;
            }
            let dir = d / length;
            let t = (q - a).dot(dir);
            let side = dir.perp_dot(q - a);
            let (along, gap, off) = if t < 0.0 || t > length {
                let (end, at) = if t < 0.0 { (a, 0.0) } else { (a + d, length) };
                let open_end =
                    !self.closed && ((t < 0.0 && i == 0) || (t > length && i + 1 == spans));
                let off = if sharp {
                    if open_end {
                        continue;
                    }
                    // the mitre: the corner's other span decides, being the farther line
                    side.abs()
                } else {
                    q.distance(end)
                };
                (self.lengths[i] + at, q.distance(end), off.copysign(side))
            } else {
                (self.lengths[i] + t, side.abs(), side)
            };
            if best.is_none_or(|b| gap < b.1) {
                best = Some((along, gap, off));
            }
        }
        let (along, _, off) = best?;
        if sharp {
            // at a corner both spans' lines count: the shape reaches the farther one
            let mitre = self.mitre(q, along);
            return Some((along, mitre.map_or(off, |m| m.copysign(off))));
        }
        Some((along, off))
    }

    /// Off a corner at `along` (a point between spans), the larger of `q`'s distances to the
    /// two spans' lines.
    fn mitre(&self, q: Point, along: f64) -> Option<f64> {
        let i = self
            .lengths
            .iter()
            .position(|&l| (l - along).abs() < 1e-9)?;
        let (before, after) = match (i.checked_sub(1), self.pts.get(i + 1)) {
            (Some(b), Some(_)) => (b, i + 1),
            (None, Some(_)) if self.closed => (self.pts.len() - 2, 1),
            (Some(b), None) if self.closed => (b, 1),
            _ => return None,
        };
        let line = |from: Point, to: Point| {
            let d = (to - from).normalize_or_zero();
            d.perp_dot(q - from).abs()
        };
        let corner = self.pts[i];
        Some(line(self.pts[before], corner).max(line(corner, self.pts[after])))
    }
}

/// For each rail, where it crosses rails (itself included) and whether it goes over there,
/// as (share along it, the weave's factor there): over and under alternate along every rail,
/// the first rail starting over.
fn weaves(rails: &[Rail], p: &Params) -> Vec<Vec<(f64, f64)>> {
    // every crossing: (rail, share along) for each strand
    let mut crossings: Vec<[(usize, f64); 2]> = vec![];
    for (a, ra) in rails.iter().enumerate() {
        for (b, rb) in rails.iter().enumerate().skip(a) {
            for (i, sa) in ra.pts.windows(2).enumerate() {
                for (j, sb) in rb.pts.windows(2).enumerate() {
                    // a rail's neighbouring spans meet but do not cross
                    if a == b && (j <= i + 1 || (ra.closed && i == 0 && j + 2 == ra.pts.len())) {
                        continue;
                    }
                    if let Some((t, u)) = crossing(sa[0], sa[1], sb[0], sb[1]) {
                        let along = |r: &Rail, k: usize, t: f64, s: &[Point]| {
                            (r.lengths[k] + s[0].distance(s[1]) * t) / r.total
                        };
                        crossings.push([(a, along(ra, i, t, sa)), (b, along(rb, j, u, sb))]);
                    }
                }
            }
        }
    }
    // over (true) or under for each strand of each crossing, alternating along each rail
    let mut over: Vec<Option<bool>> = vec![None; crossings.len()];
    for rail in 0..rails.len() {
        let mut events: Vec<(f64, usize, usize)> = crossings
            .iter()
            .enumerate()
            .flat_map(|(c, strands)| {
                strands
                    .iter()
                    .enumerate()
                    .filter(move |(_, (r, _))| *r == rail)
                    .map(move |(s, (_, along))| (*along, c, s))
            })
            .collect();
        events.sort_by(|a, b| a.0.total_cmp(&b.0));
        // the parity a crossing already decided sets the rest; else this rail starts over
        let start = events
            .iter()
            .enumerate()
            .find_map(|(k, &(_, c, s))| over[c].map(|o| (o == (s == 0)) ^ (k % 2 == 1)))
            .unwrap_or(true);
        for (k, &(_, c, s)) in events.iter().enumerate() {
            if over[c].is_none() {
                let this_over = start ^ (k % 2 == 1);
                over[c] = Some(this_over == (s == 0));
            }
        }
    }
    // a scale for Scale Shape, a share of the height for Add Base
    let factor = |is_over: bool| if is_over { p.z_over } else { p.z_under } / 100.0;
    let mut by_rail: Vec<Vec<(f64, f64)>> = vec![vec![]; rails.len()];
    for (strands, o) in crossings.iter().zip(&over) {
        let first_over = o.unwrap_or(true);
        for (s, &(rail, along)) in strands.iter().enumerate() {
            by_rail[rail].push((along, factor(first_over == (s == 0))));
        }
    }
    for list in &mut by_rail {
        list.sort_by(|a, b| a.0.total_cmp(&b.0));
    }
    by_rail
}

/// The weave's factor at `share` along a rail: linear between the crossings either side,
/// the nearest one's past the first and last; 1 on a rail that crosses nothing.
fn weave_at(weave: &[(f64, f64)], share: f64) -> f64 {
    match weave {
        [] => 1.0,
        [(_, only)] => *only,
        _ => {
            let i = weave.partition_point(|e| e.0 < share);
            match (i.checked_sub(1).map(|k| weave[k]), weave.get(i)) {
                (Some((a, fa)), Some(&(b, fb))) if b > a => fa + (fb - fa) * (share - a) / (b - a),
                (Some((_, f)), _) | (None, Some(&(_, f))) => f,
                (None, None) => 1.0,
            }
        }
    }
}

/// Where segments `a`→`b` and `c`→`d` cross: the share along each.
fn crossing(start: Point, end: Point, other: Point, other_end: Point) -> Option<(f64, f64)> {
    let (along, across) = (end - start, other_end - other);
    let denom = along.perp_dot(across);
    if denom.abs() < 1e-12 {
        return None;
    }
    let first = (other - start).perp_dot(across) / denom;
    let second = (other - start).perp_dot(along) / denom;
    ((0.0..1.0).contains(&first) && (0.0..1.0).contains(&second)).then_some((first, second))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{document_with, run_on};

    fn polyline(points: &[(f64, f64)], closed: bool) -> Path {
        let points: Vec<Point> = points.iter().map(|&(x, y)| Point::new(x, y)).collect();
        Path::polyline(&points, closed)
    }

    /// A rail along y = 50 and a triangular cross-section 20 wide, 10 high.
    fn rail_and_section() -> Document {
        document_with(vec![
            polyline(&[(20.0, 50.0), (80.0, 50.0)], false),
            polyline(&[(0.0, 0.0), (10.0, 10.0), (20.0, 0.0)], false),
        ])
    }

    fn made(doc: &Document, p: &Params) -> Component {
        let mut doc = doc.clone();
        run_on::<ExtrudeAndWeave>(&mut doc, p);
        doc.components.values().last().unwrap().clone()
    }

    cut_craft_core::contract_tests!(ExtrudeAndWeave, rail_and_section);

    #[test]
    fn a_triangle_swept_along_a_line_is_a_ridge() {
        let k = made(&rail_and_section(), &Params::default());
        let at = |x, y| k.height_at(Point::new(x, y));
        assert!((at(50.0, 50.0).unwrap() - 10.0).abs() < 0.2);
        assert!((at(50.0, 55.0).unwrap() - 5.0).abs() < 0.2);
        assert!(at(50.0, 61.0).is_none(), "past the half-width");
        // the ends round off: 5 mm past the rail's end is 5 down
        assert!((at(85.0, 50.0).unwrap() - 5.0).abs() < 0.2);
        // sharp, the ends are cut square
        let sharp = made(
            &rail_and_section(),
            &Params {
                sharp_corners: true,
                ..Params::default()
            },
        );
        assert!(
            sharp
                .height_at(Point::new(85.0, 50.0))
                .is_none_or(f64::is_nan)
        );
    }

    #[test]
    fn sharp_corners_mitre_round_a_square_rail() {
        let doc = document_with(vec![
            polyline(
                &[(20.0, 20.0), (80.0, 20.0), (80.0, 80.0), (20.0, 80.0)],
                true,
            ),
            polyline(&[(0.0, 0.0), (10.0, 10.0), (20.0, 0.0)], false),
        ]);
        // 4 mm out from the corner along both sides: round, it is 5.66 off; mitred, 4
        let round = made(&doc, &Params::default());
        let sharp = made(
            &doc,
            &Params {
                sharp_corners: true,
                ..Params::default()
            },
        );
        let corner = Point::new(84.0, 84.0);
        let (r, s) = (
            round.height_at(corner).unwrap(),
            sharp.height_at(corner).unwrap(),
        );
        assert!((r - (10.0 - 32f64.sqrt())).abs() < 0.5, "round: {r}");
        assert!((s - 6.0).abs() < 0.5, "mitred: {s}");
    }

    #[test]
    fn several_sections_blend_along_the_rail() {
        let doc = document_with(vec![
            polyline(&[(0.0, 50.0), (100.0, 50.0)], false),
            polyline(&[(0.0, 45.0), (5.0, 47.0), (10.0, 45.0)], false),
            polyline(&[(90.0, 45.0), (95.0, 53.0), (100.0, 45.0)], false),
        ]);
        let k = made(&doc, &Params::default());
        let peak = |x| k.height_at(Point::new(x, 50.0)).unwrap();
        assert!((peak(3.0) - 2.0).abs() < 0.3, "{}", peak(3.0));
        assert!((peak(97.0) - 8.0).abs() < 0.3, "{}", peak(97.0));
        assert!((peak(50.0) - 5.0).abs() < 0.3, "{}", peak(50.0));
    }

    #[test]
    fn a_component_is_stretched_or_repeated_along_the_rail() {
        // a ramp component 10 long (rising along X), 10 across
        let mut doc = document_with(vec![polyline(&[(0.0, 50.0), (40.0, 50.0)], false)]);
        let ramp = Component::from_fn(
            "ramp",
            Combine::Add,
            Rect::new(Point::new(0.0, 0.0), Point::new(10.0, 10.0)),
            0.5,
            |q| Some(q.x),
        );
        let mut c = Change::builder(&doc);
        let id = c.add_component(ramp);
        doc.apply(&c.finish()).unwrap();
        doc.selected_components = [id].into();
        let p = Params {
            sweep: Sweep::Component,
            ..Params::default()
        };
        let stretched = made(&doc, &p);
        let at = |k: &Component, x| k.height_at(Point::new(x, 50.0)).unwrap();
        assert!(
            (at(&stretched, 20.0) - 5.0).abs() < 0.6,
            "{}",
            at(&stretched, 20.0)
        );
        let filled = made(
            &doc,
            &Params {
                copies: Copies::Fill,
                ..p
            },
        );
        // four copies end to end: a saw tooth, 5 high halfway through each
        assert!(
            (at(&filled, 15.0) - 5.0).abs() < 0.6,
            "{}",
            at(&filled, 15.0)
        );
        assert!(
            (at(&filled, 35.0) - 5.0).abs() < 0.6,
            "{}",
            at(&filled, 35.0)
        );
    }

    #[test]
    fn where_rails_cross_one_goes_over_and_the_other_under() {
        // a plus sign of two rails, a flat 10 wide, 2 high section
        let doc = document_with(vec![
            polyline(&[(10.0, 50.0), (90.0, 50.0)], false),
            polyline(&[(50.0, 10.0), (50.0, 90.0)], false),
            polyline(&[(0.0, 0.0), (0.0, 2.0), (10.0, 2.0), (10.0, 0.0)], false),
        ]);
        let p = Params {
            rails: 2,
            weave: true,
            ..Params::default()
        };
        let k = made(&doc, &p);
        let at = |x, y| k.height_at(Point::new(x, y)).unwrap();
        // the first rail goes over (130%), so the crossing is its height
        assert!((at(50.0, 50.0) - 2.6).abs() < 0.1, "{}", at(50.0, 50.0));
        // along the second rail just past the first's edge it is under (70%)
        assert!((at(50.0, 56.0) - 1.4).abs() < 0.1, "{}", at(50.0, 56.0));
        let base = made(
            &doc,
            &Params {
                weave_by: WeaveBy::AddBase,
                z_under: 0.0,
                z_over: 50.0,
                ..p
            },
        );
        // the over strand keeps its 2 and stands on a base of half its height
        let crossing = base.height_at(Point::new(50.0, 50.0)).unwrap();
        assert!((crossing - 3.0).abs() < 0.1, "{crossing}");
    }

    #[test]
    fn over_and_under_alternate_along_each_rail() {
        // a straight rail crossed twice by a staple-shaped one
        let doc = document_with(vec![
            polyline(&[(10.0, 50.0), (90.0, 50.0)], false),
            polyline(
                &[(30.0, 10.0), (30.0, 90.0), (70.0, 90.0), (70.0, 10.0)],
                false,
            ),
            polyline(&[(0.0, 0.0), (0.0, 2.0), (10.0, 2.0), (10.0, 0.0)], false),
        ]);
        let k = made(
            &doc,
            &Params {
                rails: 2,
                weave: true,
                ..Params::default()
            },
        );
        let at = |x, y| k.height_at(Point::new(x, y)).unwrap();
        // the straight one over at the first crossing, under at the second, the staple the
        // other way round: each crossing shows the strand on top, and between them the
        // straight one runs from high to low
        assert!((at(30.0, 50.0) - 2.6).abs() < 0.1, "{}", at(30.0, 50.0));
        assert!((at(70.0, 50.0) - 2.6).abs() < 0.1, "{}", at(70.0, 50.0));
        assert!(
            (at(50.0, 50.0) - 2.0).abs() < 0.1,
            "between: {}",
            at(50.0, 50.0)
        );
        assert!(
            at(70.0, 30.0) > at(30.0, 30.0),
            "the staple goes under, then over"
        );
    }

    #[test]
    fn needs_rails_and_an_open_section() {
        let doc = document_with(vec![cut_craft_core::testing::square(0.0, 0.0, 5.0)]);
        assert!(ExtrudeAndWeave::run(&doc, &Params::default()).is_err());
    }
}
