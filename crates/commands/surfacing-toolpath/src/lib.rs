//! Surfacing Toolpath: flattens the whole material, the spoilboard, or a shape with a wide
//! surfacing bit, in rows or a spiral that run past every edge. The bit always goes down
//! beside the work and feeds in from the side: a surfacing bit or fly cutter must never
//! plunge into the material.

use cut_craft_core::document::{Document, ZZero};
use cut_craft_core::geometry::{Path, Point, Rect, polyline_distance, winding};
use cut_craft_core::glam::DVec3;
use cut_craft_core::polygon::{self, Corners};
use cut_craft_core::toolpath::{Move, MoveKind, Tool, cut_points, made_by};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct SurfacingToolpath;

/// The form's group the area choice is in: first on the form.
pub const AREA_GROUP: &str = "What to Flatten";

/// How far clear of the work's edge (mm) the bit's side is when it goes down beside it.
const GAP: f64 = 1.0;

/// How steeply (degrees) the bit ramps in where there is no room to go down beside the work.
const RAMP_ANGLE: f64 = 2.0;

/// What is flattened.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Area {
    /// The material set in Job Setup, all of it
    #[default]
    #[schemars(title = "Whole material")]
    WholeMaterial,
    /// The spoilboard on your machine's bed, from its front-left corner (X0 Y0): this cuts
    /// the spoilboard itself, with nothing on it
    Spoilboard,
    /// The inside of the closed shapes selected, such as a slab's outline
    #[schemars(title = "Selected vectors")]
    SelectedVectors,
}

/// The way the bit covers the area.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Pattern {
    /// Rows running left and right (along X), stepping towards the back
    #[default]
    #[schemars(title = "Rows along X")]
    AlongX,
    /// Rows running front to back (along Y), stepping to the right
    #[schemars(title = "Rows along Y")]
    AlongY,
    /// Rings from the outside in, each a stepover inside the last
    #[schemars(title = "Spiral in")]
    Spiral,
}

/// Which way the bit runs along each row or ring, for a spindle turning clockwise seen from
/// above.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// The cutter's edge meets the material head on: usually the smoother finish
    #[default]
    Climb,
    /// The other way round
    Conventional,
}

/// Whether the rows go back and forth or all one way.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Rows {
    /// Each row back the way the last came, staying down: the quickest
    #[default]
    BackAndForth,
    /// Every row the same way, lifting and going back between rows: every row cuts the
    /// direction chosen, for the most even finish
    OneWay,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// What the toolpath is called in the toolpath list
    pub name: String,
    /// The whole material, your spoilboard, or the inside of shapes you selected
    #[schemars(title = "Flatten", extend("x-group" = AREA_GROUP))]
    pub area: Area,
    /// Across the spoilboard, left to right; filled in from your machine's bed size
    #[serde(default)]
    #[schemars(range(min = 0))]
    #[schemars(extend(
        "x-unit" = "mm",
        "x-group" = AREA_GROUP,
        "x-show-when" = {"area": "spoilboard"}
    ))]
    pub spoilboard_width: f64,
    /// Of the spoilboard, front to back; filled in from your machine's bed size
    #[serde(default)]
    #[schemars(range(min = 0))]
    #[schemars(extend(
        "x-unit" = "mm",
        "x-group" = AREA_GROUP,
        "x-show-when" = {"area": "spoilboard"}
    ))]
    pub spoilboard_height: f64,
    /// The machine's bed size, mm (0 when not known): flattening the spoilboard stays inside
    /// it, ramping in gently where there is no room to go down beside the board
    #[serde(default)]
    #[schemars(skip)]
    pub bed: [f64; 2],
    /// How much comes off the top: a light skim is enough to flatten. Edit Passes… takes it
    /// off in more than one pass
    #[schemars(title = "Depth to Take Off", extend("x-unit" = "mm"), range(min = 0))]
    #[schemars(extend("x-group" = "Cutting Depths"))]
    pub depth: f64,
    /// A surfacing bit (spoilboard bit or fly cutter) from your tools; its stepover and
    /// feeds are used
    pub tool: Tool,
    /// Rows along X or Y, or a spiral from the outside in
    #[serde(default)]
    #[schemars(title = "Cut in", extend("x-group" = "Pattern"))]
    pub pattern: Pattern,
    /// How far the middle of the bit runs past every edge, as a share of the bit's width,
    /// so the edges are cut cleanly
    #[serde(default = "overhang")]
    #[schemars(extend("x-unit" = "%", "x-group" = "Pattern"), range(min = 0, max = 200))]
    pub overhang: f64,
    /// Set in the Cutting window
    #[serde(default)]
    #[schemars(skip)]
    pub direction: Direction,
    /// Set in the Cutting window
    #[serde(default)]
    #[schemars(skip)]
    pub rows: Rows,
}

fn overhang() -> f64 {
    25.0
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Surfacing".into(),
            area: Area::WholeMaterial,
            spoilboard_width: 0.0,
            spoilboard_height: 0.0,
            bed: [0.0; 2],
            depth: 0.25,
            tool: Tool::surfacing_bit(),
            pattern: Pattern::AlongX,
            overhang: overhang(),
            direction: Direction::Climb,
            rows: Rows::BackAndForth,
        }
    }
}

impl Params {
    /// The spoilboard's size, or `None` with it not set.
    fn spoilboard(&self) -> Option<Point> {
        (self.spoilboard_width > 0.0 && self.spoilboard_height > 0.0)
            .then(|| Point::new(self.spoilboard_width, self.spoilboard_height))
    }
}

/// Whether a toolpath made by this command flattens the spoilboard, from its settings.
pub fn cuts_spoilboard(params: &serde_json::Value) -> bool {
    params["area"] == "spoilboard"
}

impl Command for SurfacingToolpath {
    const ID: &'static str = "surfacing_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Surfacing Toolpath",
            group: "Toolpath Operations",
            icon: "toolpaths/surfacing_toolpath.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("depth to take off", p.depth)?;
        ensure_positive("bit diameter", p.tool.diameter)?;
        let stepover = p.tool.stepover;
        if !(stepover > 0.0 && stepover <= 1.0) {
            return Err(CommandError::Failed(
                "set the bit's stepover between 1% and 100% of its width (Edit… under Tool)".into(),
            ));
        }
        let (ids, area) = area(doc, p)?;
        let top = if p.area == Area::Spoilboard && doc.job.z_zero == ZZero::MachineBed {
            // zeroed on the bed, the spoilboard's top is the bed: under the material
            -doc.job.thickness
        } else {
            0.0
        };
        let bed = (p.area == Area::Spoilboard && p.bed[0] > 0.0 && p.bed[1] > 0.0)
            .then(|| Rect::new(Point::ZERO, Point::new(p.bed[0], p.bed[1])));
        let plan = Plan {
            area: &area,
            radius: p.tool.diameter / 2.0,
            step: stepover * p.tool.diameter,
            overhang: p.overhang.max(0.0) / 100.0 * p.tool.diameter,
            bed,
        };
        let runs = plan.runs(p);
        if runs.is_empty() {
            return Err(CommandError::Failed("there is nothing to flatten".into()));
        }
        let safe = doc.job.clearance.max(top + GAP);
        let mut moves = vec![];
        for dz in p.tool.pass_depths(p.depth) {
            plan.cut(&runs, top, top + dz, safe, &mut moves);
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

/// The vectors the toolpath is made from (none for the material or spoilboard) and the
/// area to flatten, as closed outlines.
fn area(
    doc: &Document,
    p: &Params,
) -> Result<(Vec<cut_craft_core::document::EntityId>, Vec<Path>), CommandError> {
    let rect = |r: Rect| {
        Path::polyline(
            &[
                r.min,
                Point::new(r.max.x, r.min.y),
                r.max,
                Point::new(r.min.x, r.max.y),
            ],
            true,
        )
    };
    match p.area {
        Area::WholeMaterial => {
            ensure_positive("material width", doc.job.width)?;
            ensure_positive("material height", doc.job.height)?;
            Ok((vec![], vec![rect(doc.job.area())]))
        }
        Area::Spoilboard => {
            let size = p.spoilboard().ok_or_else(|| {
                CommandError::Failed(
                    "enter the spoilboard's width and height (set your machine's bed size in \
                     Machine ▸ Machines and they fill in)"
                        .into(),
                )
            })?;
            Ok((vec![], vec![rect(Rect::new(Point::ZERO, size))]))
        }
        Area::SelectedVectors => {
            let (ids, paths) = doc.selected_paths(|path| path.closed);
            if ids.is_empty() {
                return Err(CommandError::Failed(
                    "select the closed outline of what to flatten, or flatten the whole \
                     material"
                        .into(),
                ));
            }
            Ok((ids, polygon::regions(&paths)))
        }
    }
}

/// Where the bit goes, before it is given heights.
struct Plan<'a> {
    /// What is flattened
    area: &'a [Path],
    radius: f64,
    step: f64,
    /// How far the bit's middle runs past the edges
    overhang: f64,
    /// The machine's bed when flattening the spoilboard: nothing goes outside it
    bed: Option<Rect>,
}

/// One stretch the bit cuts along without lifting, and whether it carries on from the last
/// without lifting (a row joined to the row before, or a ring to the ring outside it).
struct Run {
    points: Vec<Point>,
    joined: bool,
}

impl Plan<'_> {
    /// The area grown by the overhang: where the bit's middle goes.
    fn grown(&self) -> Vec<Path> {
        polygon::offset_regions(&polygon::regions(self.area), self.overhang, Corners::Sharp)
    }

    fn runs(&self, p: &Params) -> Vec<Run> {
        let grown = self.grown();
        let climb = p.direction == Direction::Climb;
        let mut runs = match p.pattern {
            Pattern::AlongX => self.rows(&grown, false, climb, p.rows),
            Pattern::AlongY => self.rows(&grown, true, climb, p.rows),
            Pattern::Spiral => self.rings(&grown, climb),
        };
        if let Some(bed) = self.bed {
            let inside = |q: Point| {
                Point::new(
                    q.x.clamp(bed.min.x, bed.max.x),
                    q.y.clamp(bed.min.y, bed.max.y),
                )
            };
            for run in &mut runs {
                for q in &mut run.points {
                    *q = inside(*q);
                }
                run.points.dedup_by(|a, b| a.distance(*b) < 1e-9);
            }
            runs.retain(|r| r.points.len() > 1);
            // rows pushed onto the bed's edge on top of one another: one is enough
            runs.dedup_by(|a, b| same_line(&a.points, &b.points));
        }
        runs
    }

    /// Rows across `grown`, evenly spaced no more than a stepover apart from one edge to
    /// the other; along Y when `along_y`.
    fn rows(&self, grown: &[Path], along_y: bool, climb: bool, rows: Rows) -> Vec<Run> {
        // work along X: turn rows along Y over to X, and back at the end
        let flip = |q: Point| if along_y { Point::new(q.y, q.x) } else { q };
        let rings: Vec<Vec<Point>> = grown
            .iter()
            .map(|r| cut_points(r).into_iter().map(flip).collect())
            .collect();
        let (low, high) = rings
            .iter()
            .flatten()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), q| {
                (l.min(q.y), h.max(q.y))
            });
        if !low.is_finite() {
            return vec![];
        }
        let span = high - low;
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a small positive count"
        )]
        let gaps = (span / self.step).ceil().max(1.0) as u32;
        let gap = span / f64::from(gaps);
        // just inside the first and last edges, so a row along them meets the outline
        let nudge = (span * 1e-9).max(1e-7);
        // the rows run the direction that climbs: with the uncut material on the right as
        // the rows step across, along X that is right to left; along Y the turn over to X
        // mirrors it
        let forward = climb == along_y;
        let mut runs = vec![];
        for i in 0..=gaps {
            let y = (low + gap * f64::from(i)).clamp(low + nudge, high - nudge);
            let mut spans = crossings(&rings, y);
            let backward = match rows {
                Rows::OneWay => !forward,
                Rows::BackAndForth => (i % 2 == 0) != forward,
            };
            if backward {
                spans.reverse();
                for s in &mut spans {
                    *s = (s.1, s.0);
                }
            }
            let row_start = runs.len();
            for (a, b) in spans {
                // a row's first stretch follows on from the last row's end, when the rows
                // go back and forth
                let joined = rows == Rows::BackAndForth && runs.len() == row_start && i > 0;
                runs.push(Run {
                    points: vec![flip(a), flip(b)],
                    joined,
                });
            }
        }
        // a joined stretch must start where the last ended, beside it
        for i in 1..runs.len() {
            let (before, run) = runs.split_at_mut(i);
            let end = *before[i - 1].points.last().expect("a run has points");
            let run = &mut run[0];
            if run.joined && end.distance(run.points[0]) > 2.0 * self.step {
                run.joined = false;
            }
        }
        runs
    }

    /// Rings from the outside of `grown` in, evenly spaced no more than a stepover apart,
    /// the last along the middle; clockwise to climb.
    fn rings(&self, grown: &[Path], climb: bool) -> Vec<Run> {
        let inset = |d: f64| polygon::offset_regions(grown, -d, Corners::Round);
        // how far in the area goes before nothing is left of it
        let (mut gone, mut left) = (self.step, 0.0);
        while !inset(gone).is_empty() {
            left = gone;
            gone *= 2.0;
        }
        for _ in 0..30 {
            let mid = f64::midpoint(left, gone);
            if inset(mid).is_empty() {
                gone = mid;
            } else {
                left = mid;
            }
        }
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a small positive count"
        )]
        let gaps = (left / self.step).ceil().max(1.0) as u32;
        let gap = left / f64::from(gaps);
        let mut runs: Vec<Run> = vec![];
        for i in 0..=gaps {
            let mut rings: Vec<Vec<Point>> = inset(gap * f64::from(i))
                .iter()
                .map(cut_points)
                .filter(|r| r.len() > 1)
                .collect();
            while !rings.is_empty() {
                // the ring nearest where the bit is, started at its nearest point
                let at = runs.last().and_then(|r| r.points.last().copied());
                let (pick, start) = match at {
                    Some(at) => nearest(&rings, at),
                    None => (0, corner(&rings[0])),
                };
                let mut ring = rings.swap_remove(pick);
                // clockwise (negative winding) to climb, as round the outside of a part
                if (winding(&ring) < 0.0) != climb {
                    ring.reverse();
                }
                let first = nearest_index(&ring, start);
                ring.rotate_left(first);
                ring.push(ring[0]);
                let joined = at.is_some_and(|at| at.distance(ring[0]) <= 2.0 * self.step);
                runs.push(Run {
                    points: ring,
                    joined,
                });
            }
        }
        runs
    }

    /// Whether the bit at `q` is clear of the work, with a gap to spare.
    fn clear_at(&self, q: Point) -> bool {
        let rings: Vec<Vec<Point>> = self.area.iter().map(cut_points).collect();
        !cut_craft_core::geometry::inside(&rings, q)
            && rings
                .iter()
                .all(|r| polyline_distance(&closed(r), q) >= self.radius + GAP - 1e-9)
    }

    /// The moves for one pass at height `z` (the work's top at `top`): each stretch entered
    /// from beside the work, or ramped into where there is no room for that.
    fn cut(&self, runs: &[Run], top: f64, z: f64, safe: f64, moves: &mut Vec<Move>) {
        let at = |q: Point, z: f64| DVec3::new(q.x, q.y, z);
        let mut down = false;
        for run in runs {
            let first = run.points[0];
            if down && run.joined {
                moves.push(Move {
                    kind: MoveKind::Cut,
                    to: at(first, z),
                });
            } else {
                if down {
                    let last = moves.last().expect("down after a move").to;
                    moves.push(Move {
                        kind: MoveKind::Rapid,
                        to: last.truncate().extend(safe),
                    });
                }
                self.enter(&run.points, top, z, safe, moves);
            }
            moves.extend(run.points[1..].iter().map(|&q| Move {
                kind: MoveKind::Cut,
                to: at(q, z),
            }));
            down = true;
        }
        if let Some(last) = moves.last().map(|m| m.to) {
            moves.push(Move {
                kind: MoveKind::Rapid,
                to: last.truncate().extend(safe),
            });
        }
    }

    /// Into the start of `points` at height `z`: down beside the work, backed off along
    /// the way in until clear of it; where that is outside the bed, or it can't get clear,
    /// down to the top and a gentle ramp along the way in.
    fn enter(&self, points: &[Point], top: f64, z: f64, safe: f64, moves: &mut Vec<Move>) {
        let at = |q: Point, z: f64| DVec3::new(q.x, q.y, z);
        let first = points[0];
        let along = (points[1] - first).normalize_or_zero();
        let back = self.radius + GAP + self.overhang;
        let beside = first - along * back;
        let on_bed = self.bed.is_none_or(|b| b.contains(beside));
        // backed off only as far as needed
        let mut start = first;
        if on_bed {
            let mut lo = 0.0;
            let mut hi = back;
            if self.clear_at(beside) {
                for _ in 0..30 {
                    let mid = f64::midpoint(lo, hi);
                    if self.clear_at(first - along * mid) {
                        hi = mid;
                    } else {
                        lo = mid;
                    }
                }
                start = first - along * hi;
            }
        }
        moves.push(Move {
            kind: MoveKind::Rapid,
            to: at(start, safe),
        });
        if self.clear_at(start) {
            moves.push(Move {
                kind: MoveKind::Plunge,
                to: at(start, z),
            });
            if start != first {
                moves.push(Move {
                    kind: MoveKind::Cut,
                    to: at(first, z),
                });
            }
            return;
        }
        // no room beside the work: down to its top, then ramp in along the way and back
        moves.push(Move {
            kind: MoveKind::Plunge,
            to: at(start, top),
        });
        let length = length_of(points);
        let ramp = ((top - z) / RAMP_ANGLE.to_radians().tan()).min(length / 2.0);
        if ramp <= 0.0 {
            return;
        }
        let ahead = point_along(points, ramp);
        moves.push(Move {
            kind: MoveKind::Cut,
            to: at(ahead, z),
        });
        moves.push(Move {
            kind: MoveKind::Cut,
            to: at(first, z),
        });
    }
}

/// A ring's points with the first again at the end.
fn closed(ring: &[Point]) -> Vec<Point> {
    let mut points = ring.to_vec();
    if let Some(&first) = ring.first() {
        points.push(first);
    }
    points
}

/// Where a scan line at `y` crosses the `rings`, as (left, right) spans.
fn crossings(rings: &[Vec<Point>], y: f64) -> Vec<(Point, Point)> {
    let mut xs: Vec<f64> = rings
        .iter()
        .flat_map(|ring| {
            let n = ring.len();
            (0..n).filter_map(move |i| {
                let (a, b) = (ring[i], ring[(i + 1) % n]);
                ((a.y <= y) != (b.y <= y)).then(|| a.x + (y - a.y) / (b.y - a.y) * (b.x - a.x))
            })
        })
        .collect();
    xs.sort_by(f64::total_cmp);
    xs.as_chunks::<2>()
        .0
        .iter()
        .map(|&[a, b]| (Point::new(a, y), Point::new(b, y)))
        .collect()
}

/// Two runs along the same straight line.
fn same_line(a: &[Point], b: &[Point]) -> bool {
    a.len() == 2
        && b.len() == 2
        && ((a[0].distance(b[0]) < 1e-9 && a[1].distance(b[1]) < 1e-9)
            || (a[0].distance(b[1]) < 1e-9 && a[1].distance(b[0]) < 1e-9))
}

/// The ring with the point nearest `at`, and that point.
fn nearest(rings: &[Vec<Point>], at: Point) -> (usize, Point) {
    rings
        .iter()
        .enumerate()
        .flat_map(|(i, r)| r.iter().map(move |&q| (i, q)))
        .min_by(|a, b| a.1.distance(at).total_cmp(&b.1.distance(at)))
        .expect("rings with points")
}

/// The place in `ring` of its point nearest `at`.
fn nearest_index(ring: &[Point], at: Point) -> usize {
    ring.iter()
        .enumerate()
        .min_by(|a, b| a.1.distance(at).total_cmp(&b.1.distance(at)))
        .map_or(0, |(i, _)| i)
}

/// The ring's front-left corner: where the first ring starts.
fn corner(ring: &[Point]) -> Point {
    *ring
        .iter()
        .min_by(|a, b| (a.x + a.y).total_cmp(&(b.x + b.y)))
        .expect("a ring with points")
}

fn length_of(points: &[Point]) -> f64 {
    points.windows(2).map(|w| w[0].distance(w[1])).sum()
}

/// The point `distance` along `points` from the first.
fn point_along(points: &[Point], distance: f64) -> Point {
    let mut left = distance;
    for w in points.windows(2) {
        let length = w[0].distance(w[1]);
        if left <= length {
            return w[0].lerp(w[1], left / length.max(1e-12));
        }
        left -= length;
    }
    *points.last().expect("points")
}

#[cfg(test)]
mod tests;
