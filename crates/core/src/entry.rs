//! Getting the tool into and out of a cut along a path: ramps in place of straight plunges,
//! lead-in and lead-out moves, and tabs that hold a part in place (profile options).

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
    #[serde(alias = "both")]
    Climb,
    /// The other way round
    Conventional,
}

/// Whether every pass runs the same way, or each runs back the way the last came: the only
/// way a toolpath cuts both climb and conventional.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Ways {
    /// Every pass the cut direction's way
    #[default]
    #[schemars(title = "Single direction")]
    One,
    /// The first pass the cut direction's way, the next back the other way (climb then
    /// conventional, or the other way about), and so on
    #[schemars(title = "Both directions")]
    Both,
}

/// How the tool moves along its path.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Motion {
    /// Straight along it, its whole side cutting
    #[default]
    Straight,
    /// In small loops as it moves along, so only a sliver of its edge cuts at a time: a
    /// light cut at full depth, for hard material or a small machine
    Trochoidal,
}

/// Which way a trochoidal toolpath's loops turn, seen from above. With the spindle turning
/// clockwise, anticlockwise loops cut climb and clockwise ones conventional, whichever way
/// the tool travels along its path.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Turn {
    /// Each loop cuts climb
    #[default]
    Anticlockwise,
    /// Each loop cuts conventional
    Clockwise,
}

/// The loops a trochoidal toolpath makes, as shares of the tool's diameter, and which way
/// they turn.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Loops {
    /// How wide each loop is
    #[schemars(title = "Loop Width", extend("x-unit" = "%"), range(min = 10, max = 400))]
    pub width: f64,
    /// How far each loop moves on from the last: less is a lighter cut and a longer toolpath
    #[schemars(title = "Loop Step", extend("x-unit" = "%"), range(min = 1, max = 100))]
    pub step: f64,
    /// Apart from the cut direction, which sets the way along the path (added with loop turn)
    #[serde(default)]
    #[schemars(
        title = "Loops Turn",
        extend("x-widget" = "named-icons", "x-icon" = "toolpaths/loops_{}.png")
    )]
    pub turn: Turn,
}

impl Default for Loops {
    fn default() -> Self {
        Self {
            width: 100.0,
            step: 10.0,
            turn: Turn::Anticlockwise,
        }
    }
}

impl Loops {
    /// Whether the loops turn anticlockwise.
    pub fn anticlockwise(self) -> bool {
        self.turn == Turn::Anticlockwise
    }

    /// Each loop's radius and how far it moves on, in mm, for a tool `diameter` across.
    pub fn sized(self, diameter: f64) -> (f64, f64) {
        (
            diameter * self.width / 200.0,
            (diameter * self.step / 100.0).max(0.01),
        )
    }
}

/// Loops `radius` round the path through `points`, each a whole turn at most `advance` on
/// from the last, anticlockwise (climb: the loop's outside meets the material head on) or
/// clockwise. On a closed path they end where they began. Run backwards, they turn the other
/// way.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn trochoid(
    points: &[Point],
    closed: bool,
    radius: f64,
    advance: f64,
    anticlockwise: bool,
) -> Vec<Point> {
    const PER_LOOP: u32 = 24;
    let mut points = points.to_vec();
    if closed {
        points.extend(points.first().copied());
    }
    let mut along = vec![0.0];
    for w in points.windows(2) {
        along.push(along[along.len() - 1] + w[0].distance(w[1]));
    }
    let length = along[along.len() - 1];
    if points.len() < 2 || length <= 0.0 {
        return vec![];
    }
    let at = |s: f64| {
        let s = if closed {
            s.rem_euclid(length)
        } else {
            s.clamp(0.0, length)
        };
        let i = along
            .partition_point(|&a| a <= s)
            .clamp(1, points.len() - 1);
        let t = (s - along[i - 1]) / (along[i] - along[i - 1]).max(1e-12);
        points[i - 1].lerp(points[i], t)
    };
    let side = if anticlockwise { 1.0 } else { -1.0 };
    let samples = (length / advance).ceil().max(1.0) as u32 * PER_LOOP;
    (0..=samples)
        .map(|k| {
            let s = length * f64::from(k) / f64::from(samples);
            // the way on, over a loop's width, so a corner in the path turns the loops smoothly
            let ahead = (at(s + radius) - at(s - radius))
                .try_normalize()
                .unwrap_or(Point::X);
            let a = std::f64::consts::TAU * f64::from(k % PER_LOOP) / f64::from(PER_LOOP);
            at(s) + radius * (a.sin() * ahead - side * a.cos() * ahead.perp())
        })
        .collect()
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
    /// Straight down into the cut
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

/// How long or steep each ramp is.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Ramping {
    /// Size the ramp by its length or by its angle
    pub by: RampBy,
    /// How far along the path the ramp runs
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub distance: f64,
    /// Down from level, when ramping by angle
    #[schemars(extend("x-unit" = "°"), range(min = 0, max = 90))]
    pub angle: f64,
}

impl Default for Ramping {
    fn default() -> Self {
        // ponytail: our own defaults
        Self {
            by: RampBy::Distance,
            distance: 10.0,
            angle: 10.0,
        }
    }
}

/// How the tool goes down into the material where a cut starts: set for each toolpath, and
/// fitted to the room at every place it goes down.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EntryMode {
    /// Straight down, as a drill does: quickest, but hardest on the bit, and only for bits
    /// that cut at their centre
    Plunge,
    /// Down in circles, like a corkscrew: the gentlest way into a pocket or hole
    #[default]
    Helix,
    /// Back and forth along the start of the cut, a little lower each time
    ZigZag,
    /// Down along the cut in one straight slope
    Ramp,
}

impl EntryMode {
    /// Its name in a sentence.
    pub fn said(self) -> &'static str {
        match self {
            Self::Plunge => "straight plunge",
            Self::Helix => "helix",
            Self::ZigZag => "zig-zag",
            Self::Ramp => "ramp",
        }
    }
}

/// A toolpath's entry: the way it goes down and how big and steep. Each place it goes down
/// takes the mode asked for where it fits, and where it does not, the next that does, never
/// steeper than asked: a helix shrinks (more turns), then gives way to a zig-zag along the
/// cut, a zig-zag or ramp takes shorter legs (more of them), and a straight plunge is the
/// last resort.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct EntrySettings {
    pub mode: EntryMode,
    /// Across the circle the bit's centre goes round, as a share of the bit's diameter: 75%
    /// on a 1/4" bit is 4.8 mm. Smaller where the room is smaller
    #[schemars(extend("x-unit" = "%"), range(min = 10, max = 400))]
    pub helix_diameter: f64,
    /// How steeply the helix goes down, from level: 2° to 5° suits wood
    #[schemars(extend("x-unit" = "°"), range(min = 0.1, max = 45))]
    pub helix_angle: f64,
    /// How steeply each zig-zag leg goes down, from level
    #[schemars(extend("x-unit" = "°"), range(min = 0.1, max = 45))]
    pub zigzag_angle: f64,
    /// How many legs (there and back is two) to go down a pass in; more where the cut is too
    /// short for legs this long. An odd number is made even, so the tool ends where it began
    #[schemars(range(min = 1, max = 100))]
    pub zigzag_passes: u32,
    /// How far along the cut the ramp takes to go down a pass
    #[schemars(extend("x-unit" = "mm"), range(min = 0.1))]
    pub ramp_distance: f64,
}

impl Default for EntrySettings {
    fn default() -> Self {
        // ponytail: our own defaults, gentle for wood with a hobby machine
        Self {
            mode: EntryMode::Helix,
            helix_diameter: 75.0,
            helix_angle: 3.0,
            zigzag_angle: 10.0,
            zigzag_passes: 4,
            ramp_distance: 25.0,
        }
    }
}

impl EntrySettings {
    /// The nearest entry to a toolpath's older ramp settings, for one taking passes
    /// `pass_depth` deep (a zig-zag's slope was its pass depth over its distance there and
    /// back).
    pub fn from_old(ramp: Ramp, ramping: Ramping, pass_depth: f64) -> Self {
        let by_angle = ramping.by == RampBy::Angle && ramping.angle > 0.0;
        let distance = if by_angle && pass_depth > 0.0 {
            pass_depth / ramping.angle.to_radians().tan()
        } else {
            ramping.distance
        };
        let mode = match ramp {
            Ramp::None => EntryMode::Plunge,
            Ramp::Smooth | Ramp::Spiral => EntryMode::Ramp,
            Ramp::ZigZag => EntryMode::ZigZag,
        };
        let zigzag_angle = if by_angle {
            ramping.angle
        } else {
            (pass_depth / 2.0).atan2(distance.max(1e-9)).to_degrees()
        };
        Self {
            mode,
            zigzag_angle,
            zigzag_passes: 2,
            ramp_distance: distance.max(0.1),
            ..Self::default()
        }
    }
}

/// Where the bit's centre may go when entering: inside the rings (a pocket's, or a hole's
/// own path), or outside them (round parts cut out). A helix must keep inside it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Room {
    /// Closed rings; one inside another is a hole (even-odd).
    pub rings: Vec<Vec<Point>>,
    /// Whether the room is inside the rings, or outside them.
    pub inside: bool,
}

impl Room {
    pub fn inside(rings: Vec<Vec<Point>>) -> Self {
        Self {
            rings,
            inside: true,
        }
    }

    pub fn outside(rings: Vec<Vec<Point>>) -> Self {
        Self {
            rings,
            inside: false,
        }
    }

    /// How far `p` is from the room's edge, or None when it is not in the room.
    fn clearance(&self, p: Point) -> Option<f64> {
        if crate::geometry::inside(&self.rings, p) != self.inside {
            return None;
        }
        let mut near = f64::INFINITY;
        for ring in &self.rings {
            for (i, &a) in ring.iter().enumerate() {
                let b = ring[(i + 1) % ring.len()];
                near = near.min(segment_gap(p, a, b));
            }
        }
        Some(near)
    }

    /// Whether a circle `radius` round `centre` is in the room.
    pub fn fits(&self, centre: Point, radius: f64) -> bool {
        self.clearance(centre).is_some_and(|c| c >= radius - 1e-9)
    }

    /// Whether the straight line from `a` to `b` stays in the room.
    pub fn holds(&self, a: Point, b: Point) -> bool {
        (0..=8).all(|i| {
            let p = a.lerp(b, f64::from(i) / 8.0);
            self.clearance(p).is_some_and(|c| c >= -1e-6) || p.distance(b) < 1e-6
        })
    }
}

/// How far `p` is from the segment `a` to `b`.
fn segment_gap(p: Point, a: Point, b: Point) -> f64 {
    let span = b - a;
    let t = if span.length_squared() > 0.0 {
        ((p - a).dot(span) / span.length_squared()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    p.distance(a + span * t)
}

/// How far a zig-zag or ramp may run along `track` from `from`: round a closed one, to the
/// end of an open one.
fn room_along(track: &Line, from: f64) -> f64 {
    if track.closed {
        track.length
    } else {
        track.length - from
    }
}

/// A place a toolpath went down another way than its entry asked, there being no room for
/// it: shown in the preview and in Checks before cutting.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fallback {
    /// Where the tool went down.
    pub at: Point,
    pub wanted: EntryMode,
    pub used: EntryMode,
}

/// The fallbacks of a toolpath being made, gathered from every path it cuts.
pub type Fallbacks = std::sync::Arc<std::sync::Mutex<Vec<Fallback>>>;

/// The least a helix's centre goes round, as a share of the bit's diameter: any smaller is
/// as good as a plunge.
const SMALLEST_HELIX: f64 = 0.2;

/// The shortest zig-zag leg, as a share of the bit's diameter, and in mm.
const SHORTEST_LEG: f64 = 0.25;
const SHORTEST_LEG_MM: f64 = 0.1;

/// The points round a helix's turn.
const PER_TURN: u32 = 32;

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

/// The size and shape of the lead in and out.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Leads {
    /// A straight lead's angle to the path
    #[schemars(extend("x-unit" = "°"), range(min = 0, max = 90))]
    pub angle: f64,
    /// A circular lead's radius; its sweep is the length over the radius, at most 90°
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub radius: f64,
    /// How long the lead in is
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
        // ponytail: our own defaults
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
    /// No tabs: the part comes loose when cut through
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
    /// About one every distance, at least three on a path, and no more than a third of the
    /// path in tabs: big parts get more, small ones are not all tab
    #[default]
    Automatic,
    /// This many tabs spread evenly along each path
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
    /// As thick as the tool's pass depth (at most half the cut) and twice as long as thick:
    /// a 1/4" bit taking 1/4" passes leaves 1/2" x 1/4" tabs. Off: the length and thickness
    /// below (added with tab sizes)
    #[serde(default = "yes")]
    pub size_from_tool: bool,
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
    /// Constant distance: at least this many tabs on a path
    #[serde(default = "one")]
    #[schemars(title = "Minimum Number", range(min = 1))]
    pub minimum: u32,
    /// Constant distance: at most this many tabs on a path
    #[serde(default = "many")]
    #[schemars(title = "Maximum Number", range(min = 1))]
    pub maximum: u32,
    /// Where tabs spread by number or distance go (added with tab placement)
    #[serde(default)]
    pub placement: TabPlacement,
}

fn yes() -> bool {
    true
}

fn one() -> u32 {
    1
}

fn many() -> u32 {
    100
}

impl Default for TabSettings {
    fn default() -> Self {
        // ponytail: our own defaults; these are a woodworker's rule of thumb
        Self {
            size_from_tool: true,
            length: 12.7,
            thickness: 6.35,
            spacing: TabSpacing::Automatic,
            number: 4,
            distance: 150.0,
            minimum: one(),
            maximum: many(),
            placement: TabPlacement::Evenly,
        }
    }
}

impl TabSettings {
    /// The settings with the size worked out from the tool, if asked, for a cut `depth`
    /// deep taken in passes `pass_depth` deep.
    #[must_use]
    pub fn sized(self, pass_depth: f64, depth: f64) -> Self {
        if !self.size_from_tool {
            return self;
        }
        let half = depth / 2.0;
        let thickness = if pass_depth > 0.0 {
            pass_depth.min(half)
        } else {
            half
        };
        Self {
            thickness,
            length: 2.0 * thickness,
            ..self
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
    /// The toolpath's entry, in place of `ramp` and `ramping` (toolpaths made before entries
    /// have none, and ramp as they did).
    pub enter: Option<EntrySettings>,
    /// Where a helix may go; without it, no helix fits.
    pub room: Option<Room>,
    /// Where an entry went down another way than asked, added to as the moves are made.
    pub fallbacks: Fallbacks,
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

/// A plain contour's moves (no ramps, leads or tabs): `contour_moves` for a single direction,
/// each pass back the way the last came for both.
pub fn contour_ways(
    ways: Ways,
    points: &[Point],
    closed: bool,
    depths: &[f64],
    safe_z: f64,
) -> Vec<Move> {
    match ways {
        Ways::One => crate::toolpath::contour_moves(points, closed, depths, safe_z),
        Ways::Both => directed_moves(ways, points, closed, depths, 0.0, safe_z, &Entry::default()),
    }
}

/// Moves cutting `points` at each of `depths`: as `path_moves` does, except that Both
/// directions runs every other pass (from the second) back along the path, from where the
/// pass before ended. Tabs stay where the first pass puts them, and leads stay on the waste
/// side.
pub fn directed_moves(
    ways: Ways,
    points: &[Point],
    closed: bool,
    depths: &[f64],
    top: f64,
    safe_z: f64,
    entry: &Entry,
) -> Vec<Move> {
    if ways == Ways::One || depths.len() < 2 {
        return path_moves(points, closed, depths, top, safe_z, entry);
    }
    let mut ahead = entry.clone();
    if ahead.tabs != Tabs::None && ahead.tab.spacing != TabSpacing::Placed {
        ahead.placed = spread_tabs(points, closed, entry);
        ahead.tab.spacing = TabSpacing::Placed;
    }
    let back = Entry {
        side: -ahead.side,
        ..ahead.clone()
    };
    let reversed: Vec<Point> = points.iter().rev().copied().collect();
    let mut above = top;
    let mut moves = vec![];
    for (pass, &z) in depths.iter().enumerate() {
        let (way, entry) = if pass % 2 == 0 {
            (points, &ahead)
        } else {
            (reversed.as_slice(), &back)
        };
        moves.extend(path_moves(way, closed, &[z], above, safe_z, entry));
        above = z;
    }
    moves
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
    let spiral = entry.ramp == Ramp::Spiral && entry.enter.is_none() && closed && !leads;
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
                let straight = entry
                    .enter
                    .map_or(entry.ramp == Ramp::None, |e| e.mode == EntryMode::Plunge);
                let to = if straight { z } else { above };
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
        if let Some(settings) = self.entry.enter {
            return self.enter(line, from, above, z, settings);
        }
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

    /// Down from `above` to `z` where `track` is at `from`, by `settings`, fitted to the room
    /// there; a fallback is noted where the mode asked for has no room. Returns where along
    /// `track` the tool is at the bottom.
    fn enter(
        &mut self,
        track: &Line,
        from: f64,
        above: f64,
        z: f64,
        settings: EntrySettings,
    ) -> f64 {
        let drop = above - z;
        if drop <= 0.0 {
            return from;
        }
        let wanted = settings.mode;
        let at = track.point(from);
        let (used, end) = match wanted {
            EntryMode::Plunge => {
                if self.moves.last().is_none_or(|m| m.to.z > z) {
                    self.push(MoveKind::Plunge, at, z);
                }
                (EntryMode::Plunge, from)
            }
            EntryMode::Helix => {
                if self.helix(track, from, above, z, settings) {
                    (EntryMode::Helix, from)
                } else {
                    self.zig_zag(track, from, above, z, settings.helix_angle, 2)
                }
            }
            EntryMode::ZigZag => self.zig_zag(
                track,
                from,
                above,
                z,
                settings.zigzag_angle,
                settings.zigzag_passes,
            ),
            EntryMode::Ramp => {
                let distance = settings.ramp_distance.max(1e-6);
                if room_along(track, from) + 1e-9 >= distance {
                    (EntryMode::Ramp, self.slope(track, from, distance, above, z))
                } else {
                    let angle = drop.atan2(distance).to_degrees();
                    self.zig_zag(track, from, above, z, angle, 2)
                }
            }
        };
        if used != wanted {
            self.entry
                .fallbacks
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(Fallback { at, wanted, used });
        }
        end
    }

    /// Down along `track` from `from` over `distance` in one slope; on a closed track the
    /// cut goes on from the bottom, on an open one the tool comes back to `from` at depth.
    fn slope(&mut self, track: &Line, from: f64, distance: f64, above: f64, z: f64) -> f64 {
        self.descend_on(track, &track.stations(from, from + distance), above, z);
        if track.closed {
            return from + distance;
        }
        for s in track.stations(from + distance, from).into_iter().skip(1) {
            self.push(MoveKind::Cut, track.point(s), z);
        }
        from
    }

    /// Down back and forth along `track` from `from`, in `passes` legs (made even, so it ends
    /// where it began) no steeper than `angle`°: shorter legs, and more, where the track is
    /// short. Straight down where there is no room for a leg.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a small positive count"
    )]
    fn zig_zag(
        &mut self,
        track: &Line,
        from: f64,
        above: f64,
        z: f64,
        angle: f64,
        passes: u32,
    ) -> (EntryMode, f64) {
        let drop = above - z;
        let slope = angle.clamp(0.1, 89.0).to_radians().tan();
        let room = room_along(track, from);
        let shortest = (SHORTEST_LEG * self.entry.tool_diameter).max(SHORTEST_LEG_MM);
        if room < shortest {
            self.push(MoveKind::Plunge, track.point(from), z);
            return (EntryMode::Plunge, from);
        }
        let even = |n: usize| n.max(2) + n % 2;
        let mut legs = even(passes.max(1) as usize);
        let mut leg = drop / (legs as f64 * slope);
        if leg > room {
            leg = room;
            legs = even((drop / (leg * slope) - 1e-9).ceil() as usize);
        }
        let mut stations = vec![];
        for i in 0..legs {
            let (out, back) = if i % 2 == 0 {
                (from, from + leg)
            } else {
                (from + leg, from)
            };
            let leg = track.stations(out, back);
            stations.extend(if i == 0 { &leg[..] } else { &leg[1..] });
        }
        self.descend_on(track, &stations, above, z);
        (EntryMode::ZigZag, from)
    }

    /// Down in a helix beside where `track` is at `from`, in the room there: as big as asked,
    /// or as big as fits, no steeper than asked; then once round flat at the bottom, and
    /// onto the track. False, with nothing done, where even a small one does not fit.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a small positive count"
    )]
    fn helix(
        &mut self,
        track: &Line,
        from: f64,
        above: f64,
        z: f64,
        settings: EntrySettings,
    ) -> bool {
        let Some(room) = self.entry.room.as_ref() else {
            return false;
        };
        let bit = self.entry.tool_diameter;
        if bit <= 0.0 {
            return false;
        }
        let wanted = bit * settings.helix_diameter / 200.0;
        let least = (bit * SMALLEST_HELIX / 2.0).min(wanted);
        if wanted <= 0.0 {
            return false;
        }
        let p = track.point(from);
        let ahead = track.tangent(from);
        let before = if from > 1e-6 {
            from - 1e-6
        } else if track.closed {
            track.length - 1e-6
        } else {
            from
        };
        let behind = track.tangent(before);
        let square = (behind.perp() + ahead.perp()).normalize_or(ahead.perp());
        let sides: &[f64] = match self.entry.side {
            s if s > 0.0 => &[1.0],
            s if s < 0.0 => &[-1.0],
            _ => &[1.0, -1.0],
        };
        // the centre of a helix `r` round that fits, out from the track on a side
        let place = |r: f64| {
            sides.iter().find_map(|&side| {
                [1.0, 1.1, 1.25, 1.5, 2.0, 3.0].iter().find_map(|&k| {
                    let centre = p + square * side * r * k;
                    let start = centre + (p - centre).normalize_or_zero() * r;
                    (room.fits(centre, r) && room.holds(start, p)).then_some(centre)
                })
            })
        };
        let (radius, centre) = if let Some(c) = place(wanted) {
            (wanted, c)
        } else {
            let Some(mut fit) = place(least).map(|c| (least, c)) else {
                return false;
            };
            let (mut lo, mut hi) = (least, wanted);
            for _ in 0..24 {
                let mid = f64::midpoint(lo, hi);
                if let Some(c) = place(mid) {
                    fit = (mid, c);
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            fit
        };
        let start = centre + (p - centre).normalize_or(square) * radius;
        let a0 = (start - centre).to_angle();
        // round the way the cut goes on
        let turn = if (start - centre).perp().dot(ahead) >= 0.0 {
            1.0
        } else {
            -1.0
        };
        let drop = above - z;
        let slope = settings.helix_angle.clamp(0.1, 89.0).to_radians().tan();
        let round = std::f64::consts::TAU * radius;
        let turns = (drop / (round * slope) - 1e-9).ceil().max(1.0) as u32;
        let per_turn = PER_TURN.max((round / 1.0).ceil() as u32);
        let on = |k: u32| {
            let a = a0 + turn * std::f64::consts::TAU * f64::from(k) / f64::from(per_turn);
            centre + Point::from_angle(a) * radius
        };
        if start.distance(p) > 1e-9 {
            self.push(MoveKind::Cut, start, above);
        }
        let steps = turns * per_turn;
        for k in 1..=steps {
            let depth = above - drop * f64::from(k) / f64::from(steps);
            self.push(MoveKind::Plunge, on(k), depth);
        }
        for k in 1..=per_turn {
            self.push(MoveKind::Cut, on(k), z);
        }
        if start.distance(p) > 1e-9 {
            self.push(MoveKind::Cut, p, z);
        }
        true
    }

    /// Down from `above` to `z` steadily over `track` through `stations`.
    fn descend_on(&mut self, track: &Line, stations: &[f64], above: f64, z: f64) {
        let total: f64 = stations.windows(2).map(|w| (w[1] - w[0]).abs()).sum();
        let mut gone = 0.0;
        for w in stations.windows(2) {
            gone += (w[1] - w[0]).abs();
            let depth = above + (z - above) * gone / total.max(1e-12);
            self.push(MoveKind::Plunge, track.point(w[1]), depth);
        }
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
        if let Some(settings) = self.entry.enter {
            // down where the lead starts, then along it at depth
            let track = Line::new(&lead_in, false);
            self.rapid_down(lead_in[0], safe_z, above.max(z), MoveKind::Plunge);
            self.enter(&track, 0.0, above, z, settings);
            for &p in &lead_in[1..] {
                self.push(MoveKind::Cut, p, z);
            }
        } else {
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

/// Where `entry`'s tab settings spread tabs on the path `points` (by number or distance, as
/// placed by hand when set so), for Edit Tabs' Add Tabs to put them there to move by hand.
pub fn spread_tabs(points: &[Point], closed: bool, entry: &Entry) -> Vec<Point> {
    if points.len() < 2 {
        return vec![];
    }
    let line = Line::new(points, closed);
    let mut entry = entry.clone();
    if entry.tab.spacing == TabSpacing::Placed {
        entry.tab.spacing = TabSpacing::ConstantNumber;
    }
    tab_centres(&line, &entry)
        .into_iter()
        .map(|s| line.point(s))
        .collect()
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
        TabSpacing::Automatic => {
            let wanted = if t.distance > 0.0 {
                (line.length / t.distance).round().max(3.0)
            } else {
                3.0
            };
            // what is lifted over for each tab, so a third of the path at most
            let lifted = t.length + entry.tool_diameter;
            let room = if lifted > 0.0 {
                (line.length / (3.0 * lifted)).floor()
            } else {
                wanted
            };
            wanted.min(room).max(1.0) as u32
        }
        TabSpacing::ConstantNumber => t.number,
        TabSpacing::ConstantDistance => {
            let wanted = if t.distance > 0.0 {
                (line.length / t.distance).round().max(1.0) as u32
            } else {
                1
            };
            wanted.clamp(t.minimum.max(1), t.maximum.max(t.minimum.max(1)))
        }
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
