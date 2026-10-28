//! Getting the tool into and out of a cut along a path: ramps in place of straight plunges,
//! lead-in and lead-out moves, and tabs that hold a part in place (Aspire's profile options).

use crate::geometry::Point;
use crate::toolpath::{Move, MoveKind};
use glam::DVec3;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Which way round a closed path is cut, for a spindle turning clockwise seen from above.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// The cutter's edge meets the material head on: anticlockwise round a hole or pocket,
    /// clockwise round the outside of a part
    #[default]
    Climb,
    /// The other way round
    Conventional,
}

impl Direction {
    /// `points` turned to run the way this direction cuts, with the material kept `inside`
    /// the closed path (a hole or pocket) or outside it (a part's outline).
    pub fn orient(self, mut points: Vec<Point>, inside: bool) -> Vec<Point> {
        let anticlockwise = crate::geometry::winding(&points) > 0.0;
        if anticlockwise != (inside == (self == Self::Climb)) {
            points.reverse();
        }
        points
    }
}

/// How the tool goes down into each pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Ramp {
    /// Straight down
    #[default]
    None,
    /// Down along the path at a steady slope
    Smooth,
    /// Back and forth along the start of the path on the way down
    ZigZag,
    /// Down over the whole of each loop of a closed path, a pass depth a turn
    Spiral,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RampBy {
    /// Over the distance along the path
    #[default]
    Distance,
    /// Down at the angle, for cutters with an entry angle; a zig zag goes back and forth no
    /// further than the distance
    Angle,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Ramping {
    pub by: RampBy,
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub distance: f64,
    /// Down from level, when ramping by angle
    #[schemars(extend("x-unit" = "°"), range(min = 0, max = 90))]
    pub angle: f64,
}

impl Default for Ramping {
    fn default() -> Self {
        // ponytail: Aspire's defaults are not known
        Self {
            by: RampBy::Distance,
            distance: 10.0,
            angle: 10.0,
        }
    }
}

/// How the tool comes onto the path.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Lead {
    /// Straight down onto the path
    #[default]
    None,
    /// Along a straight line meeting the path at the angle
    Straight,
    /// Along an arc that meets the path along it
    Circular,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Leads {
    /// A straight lead's angle to the path
    #[schemars(extend("x-unit" = "°"), range(min = 0, max = 90))]
    pub angle: f64,
    /// A circular lead's radius; its sweep is the length over the radius, at most 90°
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub radius: f64,
    #[schemars(title = "Lead Length", extend("x-unit" = "mm"), range(min = 0))]
    pub length: f64,
    /// Leave the path the same way at the end
    pub lead_out: bool,
    /// Carry on past the start of a closed path before leaving it
    #[schemars(title = "Overcut Distance", extend("x-unit" = "mm"), range(min = 0))]
    pub overcut: f64,
}

impl Default for Leads {
    fn default() -> Self {
        // ponytail: Aspire's defaults are not known
        Self {
            angle: 45.0,
            radius: 5.0,
            length: 5.0,
            lead_out: false,
            overcut: 0.0,
        }
    }
}

/// Bridges of material left along the path to hold the part.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Tabs {
    #[default]
    None,
    /// Up at the start of each tab, across, and down at its end
    #[schemars(title = "2D Tabs")]
    Flat,
    /// Up to the tab's thickness at its middle and down the other side, without stopping
    #[schemars(title = "3D Tabs")]
    Triangular,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TabSpacing {
    /// This many tabs spread evenly along each path
    #[default]
    ConstantNumber,
    /// A tab about every this far
    ConstantDistance,
    /// Where clicked: with the form open, click a vector in the 2D view to add a tab there,
    /// click a tab to take it off
    Placed,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TabPlacement {
    /// Spread evenly from half a spacing in
    #[default]
    Evenly,
    /// The first at the machining start point, the rest spread evenly after it
    AtStart,
    /// Moved along a little where that keeps them off corners and curves
    AvoidCorners,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TabSettings {
    /// Of the material left in each tab, along the path
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub length: f64,
    /// Up from the bottom of the cut
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub thickness: f64,
    pub spacing: TabSpacing,
    #[schemars(range(min = 1))]
    pub number: u32,
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub distance: f64,
    /// Where tabs spread by number or distance go (added with tab placement)
    #[serde(default)]
    pub placement: TabPlacement,
}

impl Default for TabSettings {
    fn default() -> Self {
        // ponytail: Aspire's defaults are not known
        Self {
            length: 10.0,
            thickness: 3.0,
            spacing: TabSpacing::ConstantNumber,
            number: 4,
            distance: 100.0,
            placement: TabPlacement::Evenly,
        }
    }
}

/// Everything about how one toolpath enters, leaves and bridges its paths.
#[derive(Clone, Debug, Default)]
pub struct Entry {
    pub ramp: Ramp,
    pub ramping: Ramping,
    pub lead: Lead,
    pub leads: Leads,
    /// The side leads come from: 1 left of the direction of travel, -1 right.
    pub side: f64,
    pub tabs: Tabs,
    pub tab: TabSettings,
    /// The tab's top, as a depth (negative below the surface).
    pub tab_top: f64,
    /// Added to the tab length along the centre line, so the tab left is the length given.
    pub tool_diameter: f64,
    /// Where this path's tabs go when they are placed by clicking: each moves onto the path.
    pub placed: Vec<Point>,
}

/// Points along one path by distance.
struct Line {
    points: Vec<Point>,
    /// Distance along the line to each point.
    at: Vec<f64>,
    /// Of the path once round.
    length: f64,
    closed: bool,
}

impl Line {
    fn new(points: &[Point], closed: bool) -> Self {
        // a closed path goes round three times, so a pass can start anywhere on it and run
        // on past the end of a ramp or an overcut
        let turns = if closed { 3 } else { 1 };
        let mut all: Vec<Point> = (0..turns).flat_map(|_| points.iter().copied()).collect();
        if closed {
            all.push(points[0]);
        }
        let mut at = vec![0.0];
        for w in all.windows(2) {
            at.push(at[at.len() - 1] + w[0].distance(w[1]));
        }
        let total = at[at.len() - 1];
        Self {
            points: all,
            at,
            length: total / f64::from(turns),
            closed,
        }
    }

    fn segment(&self, s: f64) -> usize {
        self.at
            .partition_point(|&a| a <= s)
            .clamp(1, self.points.len() - 1)
            - 1
    }

    fn point(&self, s: f64) -> Point {
        let i = self.segment(s);
        let span = self.at[i + 1] - self.at[i];
        let t = if span > 0.0 {
            ((s - self.at[i]) / span).clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.points[i].lerp(self.points[i + 1], t)
    }

    fn tangent(&self, s: f64) -> Point {
        let i = self.segment(s);
        (self.points[i + 1] - self.points[i]).normalize_or_zero()
    }

    /// How far along the path (once round) the point nearest `p` is.
    fn nearest(&self, p: Point) -> f64 {
        let end = self.at.partition_point(|&a| a < self.length - 1e-12).max(1);
        let mut best = (f64::INFINITY, 0.0);
        for i in 0..end.min(self.points.len() - 1) {
            let (a, b) = (self.points[i], self.points[i + 1]);
            let span = b - a;
            let t = if span.length_squared() > 0.0 {
                ((p - a).dot(span) / span.length_squared()).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let gap = p.distance(a + span * t);
            if gap < best.0 {
                best = (gap, self.at[i] + t * span.length());
            }
        }
        best.1
    }

    /// How much the path turns (radians, either way) between `a` and `b` along it.
    fn turning(&self, a: f64, b: f64) -> f64 {
        (1..self.points.len() - 1)
            .filter(|&i| self.at[i] > a && self.at[i] < b)
            .map(|i| {
                let into = self.points[i] - self.points[i - 1];
                let out = self.points[i + 1] - self.points[i];
                into.angle_to(out).abs()
            })
            .sum()
    }

    /// The distances from `a` to `b` (either way round) where the path turns, ends included.
    fn stations(&self, a: f64, b: f64) -> Vec<f64> {
        let (lo, hi) = (a.min(b), a.max(b));
        let mut s: Vec<f64> = std::iter::once(lo)
            .chain(self.at.iter().copied().filter(|&x| x > lo && x < hi))
            .chain(std::iter::once(hi))
            .collect();
        if b < a {
            s.reverse();
        }
        s
    }
}

/// The moves that cut along `points` at each depth in turn, entering from `top` (the depth
/// already cut down to), with the ramps, leads and tabs `entry` asks for, rapids at `safe_z`.
pub fn path_moves(
    points: &[Point],
    closed: bool,
    depths: &[f64],
    top: f64,
    safe_z: f64,
    entry: &Entry,
) -> Vec<Move> {
    if points.len() < 2 || depths.is_empty() {
        return crate::toolpath::contour_moves(points, closed, depths, safe_z);
    }
    let line = Line::new(points, closed);
    let mut out = Out {
        moves: vec![],
        tabs: tab_centres(&line, entry),
        line: &line,
        entry,
    };
    let leads = entry.lead != Lead::None && entry.leads.length > 0.0;
    let spiral = entry.ramp == Ramp::Spiral && closed && !leads;
    let mut from = 0.0; // where on a closed path the next pass starts
    let mut above = top;
    for (pass, &z) in depths.iter().enumerate() {
        if leads {
            out.with_leads(above, z, safe_z);
        } else if spiral {
            if pass == 0 {
                out.rapid_down(line.point(0.0), safe_z, above, MoveKind::Plunge);
            }
            out.along(0.0, line.length, z, Some(above), MoveKind::Cut);
        } else {
            if pass == 0 || !closed {
                let to = if entry.ramp == Ramp::None { z } else { above };
                out.rapid_down(line.point(from), safe_z, to, MoveKind::Plunge);
            }
            let start = out.ramp_down(from, above, z);
            out.along(start, start + line.length, z, None, MoveKind::Cut);
            if closed {
                from = start % line.length;
            } else {
                out.rapid_up(safe_z);
            }
        }
        above = z;
    }
    if spiral {
        out.along(0.0, line.length, above, None, MoveKind::Cut);
    }
    if !leads && (closed || spiral) {
        out.rapid_up(safe_z);
    }
    out.moves
}

struct Out<'a> {
    moves: Vec<Move>,
    /// Where along the path each tab's middle is.
    tabs: Vec<f64>,
    line: &'a Line,
    entry: &'a Entry,
}

impl Out<'_> {
    fn push(&mut self, kind: MoveKind, p: Point, z: f64) {
        self.moves.push(Move {
            kind,
            to: DVec3::new(p.x, p.y, z),
        });
    }

    fn here(&self) -> Point {
        self.moves.last().map_or(Point::ZERO, |m| m.to.truncate())
    }

    fn rapid_down(&mut self, p: Point, safe_z: f64, to: f64, kind: MoveKind) {
        self.push(MoveKind::Rapid, p, safe_z);
        self.push(kind, p, to);
    }

    fn rapid_up(&mut self, safe_z: f64) {
        let p = self.here();
        self.push(MoveKind::Rapid, p, safe_z);
    }

    /// Down from `above` to `z` along the path from `from`, as the ramp says; returns where
    /// along the path the tool is at the bottom.
    fn ramp_down(&mut self, from: f64, above: f64, z: f64) -> f64 {
        let (line, ramping) = (self.line, self.entry.ramping);
        let drop = above - z;
        let slope = ramping.angle.to_radians().tan();
        let by_angle = ramping.by == RampBy::Angle && slope > 0.0;
        match self.entry.ramp {
            _ if drop <= 0.0 => {}
            Ramp::None => {
                if self.moves.last().is_none_or(|m| m.to.z > z) {
                    self.push(MoveKind::Plunge, line.point(from), z);
                }
            }
            Ramp::Smooth | Ramp::Spiral => {
                let wanted = if by_angle {
                    drop / slope
                } else {
                    ramping.distance
                };
                let room = if line.closed {
                    line.length
                } else {
                    line.length - from
                };
                let length = wanted.min(room);
                if length <= 0.0 {
                    self.push(MoveKind::Plunge, line.point(from), z);
                    return from;
                }
                self.descend(&line.stations(from, from + length), above, z);
                if line.closed {
                    return from + length;
                }
                // an open path: back to its start along the cut just made
                for s in line.stations(from + length, from).into_iter().skip(1) {
                    self.push(MoveKind::Cut, line.point(s), z);
                }
            }
            Ramp::ZigZag => {
                let room = if line.closed {
                    line.length
                } else {
                    line.length - from
                };
                let leg = ramping.distance.min(room);
                if leg <= 0.0 {
                    self.push(MoveKind::Plunge, line.point(from), z);
                    return from;
                }
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "a small positive count"
                )]
                let legs = if by_angle {
                    let n = (drop / (slope * leg)).ceil().max(2.0) as usize;
                    n + n % 2
                } else {
                    2
                };
                let mut stations = vec![];
                for i in 0..legs {
                    let (out, back) = if i % 2 == 0 {
                        (from, from + leg)
                    } else {
                        (from + leg, from)
                    };
                    let leg = line.stations(out, back);
                    stations.extend(if i == 0 { &leg[..] } else { &leg[1..] });
                }
                self.descend(&stations, above, z);
            }
        }
        from
    }

    /// Down from `above` to `z` steadily over the path through `stations`.
    fn descend(&mut self, stations: &[f64], above: f64, z: f64) {
        let total: f64 = stations.windows(2).map(|w| (w[1] - w[0]).abs()).sum();
        let mut gone = 0.0;
        for w in stations.windows(2) {
            gone += (w[1] - w[0]).abs();
            let depth = above + (z - above) * gone / total;
            self.push(MoveKind::Plunge, self.line.point(w[1]), depth);
        }
    }

    /// Cuts from `a` to `b` along the path at `z` (or from `start` down to `z`, a spiral),
    /// lifting over the tabs.
    fn along(&mut self, from: f64, to: f64, z: f64, start: Option<f64>, kind: MoveKind) {
        let lift = self.tab_lift(z);
        let mut stations = self.line.stations(from, to);
        if let Some((half, _, _)) = lift {
            let period = self.line.length;
            for &c in &self.tabs {
                for turn in [0.0, period, 2.0 * period] {
                    for edge in [c - half, c, c + half] {
                        let at = edge + turn;
                        if at > from && at < to {
                            stations.push(at);
                        }
                    }
                }
            }
            stations.sort_by(f64::total_cmp);
        }
        let depth = |at: f64| start.map_or(z, |top| top + (z - top) * (at - from) / (to - from));
        for at in stations.into_iter().skip(1) {
            let point = self.line.point(at);
            let base = depth(at);
            let Some((half, top, flat)) = lift else {
                self.push(kind, point, base);
                continue;
            };
            // how far into the nearest tab, from its middle
            let off = self
                .tabs
                .iter()
                .map(|&c| {
                    let gap = (at - c).rem_euclid(self.line.length);
                    gap.min(self.line.length - gap)
                })
                .fold(f64::INFINITY, f64::min);
            if flat {
                let edge = (off - half).abs() < 1e-9;
                let entering = edge && self.moves.last().is_some_and(|m| m.to.z < top - 1e-9);
                if edge {
                    // straight up or down at the tab's ends
                    let (first, then) = if entering { (base, top) } else { (top, base) };
                    self.push(kind, point, first.max(base));
                    self.push(kind, point, then.max(base));
                } else {
                    self.push(kind, point, if off < half { top.max(base) } else { base });
                }
            } else {
                let rise = (top - base) * (1.0 - off / half);
                self.push(kind, point, base + rise.max(0.0));
            }
        }
    }

    /// Half a tab's lifted length, its top, and whether it is flat, when `z` is below it.
    fn tab_lift(&self, z: f64) -> Option<(f64, f64, bool)> {
        let e = self.entry;
        (e.tabs != Tabs::None && !self.tabs.is_empty() && z < e.tab_top - 1e-9).then(|| {
            (
                f64::midpoint(e.tab.length, e.tool_diameter),
                e.tab_top,
                e.tabs == Tabs::Flat,
            )
        })
    }

    /// One pass entered and left along leads: down at the lead's start (ramping along it if
    /// asked), round the path, past its start by the overcut, out, and up.
    fn with_leads(&mut self, above: f64, z: f64, safe_z: f64) {
        let (line, leads, side) = (self.line, self.entry.leads, self.entry.side);
        let lead_in = lead(self.entry, line.point(0.0), line.tangent(0.0), side, false);
        let ramping = self.entry.ramp != Ramp::None && above > z;
        self.rapid_down(
            lead_in[0],
            safe_z,
            if ramping { above } else { z },
            MoveKind::Plunge,
        );
        let total: f64 = lead_in.windows(2).map(|w| w[0].distance(w[1])).sum();
        let mut gone = 0.0;
        for w in lead_in.windows(2) {
            gone += w[0].distance(w[1]);
            if ramping {
                let depth = above + (z - above) * gone / total;
                self.push(MoveKind::Plunge, w[1], depth);
            } else {
                self.push(MoveKind::Cut, w[1], z);
            }
        }
        let end = if line.closed {
            line.length + leads.overcut.min(line.length)
        } else {
            line.length
        };
        self.along(0.0, end, z, None, MoveKind::Cut);
        if leads.lead_out {
            let out = lead(self.entry, line.point(end), line.tangent(end), side, true);
            for &p in &out[1..] {
                self.push(MoveKind::Cut, p, z);
            }
        }
        self.rapid_up(safe_z);
    }
}

/// A lead onto the path at `at`, travelling along `tangent` (or off it, `out`), from the
/// `side` of it, as points in the order cut.
fn lead(entry: &Entry, at: Point, tangent: Point, side: f64, out: bool) -> Vec<Point> {
    let leads = entry.leads;
    let normal = tangent.perp() * side;
    let away = if out { 1.0 } else { -1.0 };
    let points = match entry.lead {
        Lead::Circular if leads.radius > 0.0 => {
            let sweep = (leads.length / leads.radius).min(std::f64::consts::FRAC_PI_2);
            let centre = at + normal * leads.radius;
            // the arc runs anticlockwise round its centre when it is on the left
            let turn = side * away;
            let steps = 16;
            (0..=steps)
                .map(|i| {
                    let a = turn * sweep * f64::from(i) / f64::from(steps);
                    centre + Point::from_angle(a).rotate(at - centre)
                })
                .collect::<Vec<_>>()
        }
        _ => {
            let a = leads.angle.to_radians();
            let off = at + (tangent * a.cos() * away + normal * a.sin()) * leads.length;
            vec![at, off]
        }
    };
    if out {
        points
    } else {
        points.into_iter().rev().collect()
    }
}

/// Where along the path each tab's middle goes.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a small positive count"
)]
fn tab_centres(line: &Line, entry: &Entry) -> Vec<f64> {
    if entry.tabs == Tabs::None {
        return vec![];
    }
    let t = entry.tab;
    let count = match t.spacing {
        TabSpacing::Placed => {
            let mut at: Vec<f64> = entry.placed.iter().map(|&p| line.nearest(p)).collect();
            at.sort_by(f64::total_cmp);
            return at;
        }
        TabSpacing::ConstantNumber => t.number,
        TabSpacing::ConstantDistance if t.distance > 0.0 => {
            (line.length / t.distance).round().max(1.0) as u32
        }
        TabSpacing::ConstantDistance => 1,
    };
    let gap = line.length / f64::from(count.max(1));
    let first = if t.placement == TabPlacement::AtStart {
        0.0
    } else {
        0.5
    };
    let centres = (0..count).map(|i| (f64::from(i) + first) * gap);
    if t.placement != TabPlacement::AvoidCorners {
        return centres.collect();
    }
    // the nearest place, within half a spacing either way, where the tab sits on a stretch
    // that turns less than a few degrees
    let half = f64::midpoint(t.length, entry.tool_diameter);
    let straight = |c: f64| {
        let turns = |a: f64, b: f64| line.turning(a, b);
        let (a, b) = (c - half, c + half);
        let turned = if line.closed {
            // wrapped by a turn either way, so a tab across the start is judged whole
            turns(a + line.length, b + line.length)
        } else if a < 0.0 || b > line.length {
            f64::INFINITY
        } else {
            turns(a, b)
        };
        turned < CORNER
    };
    let steps = 50;
    centres
        .map(|c| {
            (0..=steps)
                .flat_map(|i| {
                    let d = gap / 2.0 * f64::from(i) / f64::from(steps);
                    [c + d, c - d]
                })
                .find(|&at| straight(at))
                .map_or(c, |at| at.rem_euclid(line.length))
        })
        .collect()
}

/// How much a tab's stretch of path may turn and still count as straight (radians, 5°).
const CORNER: f64 = 5.0 * std::f64::consts::PI / 180.0;
