//! Texture Toolpath: rows of shallow scallops of random depth and length, as a hand-tooled
//! finish, inside the selected shapes; or the selected vectors themselves cut at random depths.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Path, Point};
use cut_craft_core::glam::DVec3;
use cut_craft_core::polygon::{self, Corners};
use cut_craft_core::toolpath::{Move, Tool, contour_moves, cut_points, follow, made_by};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct TextureToolpath;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
    /// Cut along the selected vectors instead of a random pattern inside them
    pub use_selected_vectors_as_pattern: bool,
    /// For a pattern: dip in and out along each vector instead of cutting it at one depth
    pub ramp_at_start_and_end: bool,
    /// Deepest any scallop goes
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub max_cut_depth: f64,
    /// Shallowest a scallop goes, as a percentage of the deepest
    #[schemars(range(min = 0, max = 100))]
    pub min_depth_percent: f64,
    /// Longest any scallop is
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub max_cut_length: f64,
    /// Shortest a scallop is, as a percentage of the longest
    #[schemars(range(min = 0, max = 100))]
    pub min_length_percent: f64,
    /// How far a scallop may run back over the one before, as a percentage of the longest
    #[schemars(range(min = 0, max = 100))]
    pub max_overlap_percent: f64,
    /// How much the overlap varies, as a percentage of the most it may be
    #[schemars(range(min = 0, max = 100))]
    pub overlap_variation_percent: f64,
    /// Between rows of scallops
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub stepover: f64,
    /// Direction of the rows, anticlockwise from the X axis
    #[schemars(extend("x-unit" = "°"))]
    pub angle: f64,
    /// Keeps the texture this far in from the boundary (up to the tool radius in a recess)
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub boundary_offset: f64,
    /// Another number gives another random pattern
    pub seed: u32,
    pub tool: Tool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Texture".into(),
            use_selected_vectors_as_pattern: false,
            ramp_at_start_and_end: true,
            max_cut_depth: 1.5,
            min_depth_percent: 50.0,
            max_cut_length: 20.0,
            min_length_percent: 50.0,
            max_overlap_percent: 25.0,
            overlap_variation_percent: 100.0,
            stepover: 5.0,
            angle: 0.0,
            boundary_offset: 0.0,
            seed: 1,
            tool: Tool::default(),
        }
    }
}

/// A small seeded generator (splitmix64), so a toolpath recalculates to the same pattern.
struct Random(u64);

impl Random {
    /// Uniform in [0, 1).
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        #[expect(clippy::cast_precision_loss, reason = "53 random bits")]
        let unit = (z >> 11) as f64 / (1u64 << 53) as f64;
        unit
    }

    /// Uniform between `percent` % of `max` and `max`.
    fn between(&mut self, percent: f64, max: f64) -> f64 {
        let min = max * percent / 100.0;
        min + (max - min) * self.next()
    }
}

/// Samples along each dip, enough for a smooth curve.
const DIP_STEPS: u32 = 16;

/// Points down and back up from `a` to `b`: a half sine `depth` deep at the middle.
fn dip(a: Point, b: Point, depth: f64) -> impl Iterator<Item = DVec3> {
    (0..=DIP_STEPS).map(move |i| {
        let t = f64::from(i) / f64::from(DIP_STEPS);
        a.lerp(b, t)
            .extend(-depth * (t * std::f64::consts::PI).sin())
    })
}

/// Where the line `y` crosses the rings (each a closed polygon), as (enter, leave) x pairs.
fn spans(rings: &[Vec<Point>], y: f64) -> Vec<(f64, f64)> {
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
    xs.as_chunks::<2>().0.iter().map(|&[a, b]| (a, b)).collect()
}

impl Params {
    fn validate(&self) -> Result<(), CommandError> {
        ensure_positive("max cut depth", self.max_cut_depth)?;
        ensure_positive("tool diameter", self.tool.diameter)?;
        if !self.use_selected_vectors_as_pattern {
            ensure_positive("max cut length", self.max_cut_length)?;
            ensure_positive("stepover", self.stepover)?;
        }
        Ok(())
    }

    /// Rows of scallops across `regions`, in rows `stepover` apart at the angle.
    fn scallops(&self, regions: &[Path], random: &mut Random, safe: f64) -> Vec<Move> {
        let turn = Point::from_angle(self.angle.to_radians());
        // work with the rows along x, turning the shapes the other way first
        let level = |p: Point| Point::new(turn.x * p.x + turn.y * p.y, turn.x * p.y - turn.y * p.x);
        let back = |p: Point| turn.rotate(p);
        let rings: Vec<Vec<Point>> = regions
            .iter()
            .map(|r| cut_points(r).into_iter().map(level).collect())
            .collect();
        let (low, high) = rings
            .iter()
            .flatten()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), p| {
                (l.min(p.y), h.max(p.y))
            });
        let mut moves = vec![];
        let mut y = low + self.stepover / 2.0;
        while y < high {
            for (start, end) in spans(&rings, y) {
                let mut row = vec![];
                let mut x = start;
                while end - x > 1e-6 {
                    let length = random.between(self.min_length_percent, self.max_cut_length);
                    let to = (x + length).min(end);
                    let depth = random.between(self.min_depth_percent, self.max_cut_depth);
                    let a = back(Point::new(x, y));
                    let b = back(Point::new(to, y));
                    row.extend(dip(a, b, depth));
                    let overlap = self.max_cut_length * self.max_overlap_percent / 100.0
                        * self.overlap_variation_percent
                        / 100.0
                        * random.next();
                    // always move on, by at least a tenth of this scallop
                    x = (to - overlap).max(x + (to - x) / 10.0).max(x + 1e-3);
                    if x < to {
                        row.push(back(Point::new(x, y)).extend(0.0));
                    }
                }
                moves.extend(follow(&row, safe));
            }
            y += self.stepover;
        }
        moves
    }

    /// Each vector cut at a random depth: dipping in and out along it, or at that depth.
    fn pattern(&self, paths: &[Path], random: &mut Random, safe: f64) -> Vec<Move> {
        paths
            .iter()
            .flat_map(|path| {
                let mut points = cut_points(path);
                if path.closed {
                    points.push(points[0]);
                }
                let depth = random.between(self.min_depth_percent, self.max_cut_depth);
                if !self.ramp_at_start_and_end {
                    return contour_moves(&points, false, &[-depth], safe);
                }
                let length: f64 = points.windows(2).map(|w| w[0].distance(w[1])).sum();
                let mut along = 0.0;
                let mut cut = vec![];
                for (i, &p) in points.iter().enumerate() {
                    if i > 0 {
                        along += points[i - 1].distance(p);
                    }
                    let t = if length > 0.0 { along / length } else { 0.0 };
                    cut.push(p.extend(-depth * (t * std::f64::consts::PI).sin()));
                }
                follow(&cut, safe)
            })
            .collect()
    }
}

impl Command for TextureToolpath {
    const ID: &'static str = "texture_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Texture Toolpath",
            group: "Toolpath Operations",
            icon: "toolpaths/texture_toolpath.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        p.validate()?;
        let mut random = Random(u64::from(p.seed));
        let safe = doc.job.clearance;
        let (ids, moves) = if p.use_selected_vectors_as_pattern {
            let (ids, paths) = doc.selected_paths(|_| true);
            if ids.is_empty() {
                return Err(CommandError::Failed(
                    "select the vectors to use as the pattern".into(),
                ));
            }
            let moves = p.pattern(&paths, &mut random, safe);
            (ids, moves)
        } else {
            let (ids, paths) = doc.selected_paths(|path| path.closed);
            // nothing selected: the whole material
            let paths = if ids.is_empty() {
                vec![Path::polyline(
                    &[
                        Point::new(0.0, 0.0),
                        Point::new(doc.job.width, 0.0),
                        Point::new(doc.job.width, doc.job.height),
                        Point::new(0.0, doc.job.height),
                    ],
                    true,
                )]
            } else {
                paths
            };
            let regions = polygon::offset_regions(
                &polygon::regions(&paths),
                -p.boundary_offset.max(0.0),
                Corners::Round,
            );
            let moves = p.scallops(&regions, &mut random, safe);
            (ids, moves)
        };
        if moves.is_empty() {
            return Err(CommandError::Failed(
                "the boundary offset leaves nothing to texture".into(),
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
    use cut_craft_core::testing::{made_toolpath, square};
    use cut_craft_core::toolpath::{MoveKind, Toolpath};

    cut_craft_core::contract_tests!(TextureToolpath);

    fn texture(p: &Params) -> Toolpath {
        made_toolpath::<TextureToolpath>(vec![square(0.0, 0.0, 100.0)], p).unwrap()
    }

    fn cuts(t: &Toolpath) -> impl Iterator<Item = DVec3> + '_ {
        t.moves
            .iter()
            .filter(|m| m.kind != MoveKind::Rapid)
            .map(|m| m.to)
    }

    #[test]
    fn stays_inside_the_shape_between_the_depths() {
        let p = Params::default();
        let t = texture(&p);
        assert!(cuts(&t).all(|q| q.z <= 1e-9 && q.z >= -p.max_cut_depth - 1e-9));
        let within = |v: f64| (-1e-6..=100.0 + 1e-6).contains(&v);
        assert!(cuts(&t).all(|q| within(q.x) && within(q.y)));
        let deepest = cuts(&t).map(|q| -q.z).fold(0.0, f64::max);
        assert!(deepest > p.max_cut_depth * p.min_depth_percent / 100.0);
    }

    #[test]
    fn rows_are_the_stepover_apart_along_the_angle() {
        let t = texture(&Params::default());
        let mut rows: Vec<f64> = cuts(&t).map(|q| q.y).collect();
        rows.sort_by(f64::total_cmp);
        rows.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        assert_eq!(rows.len(), 20, "{rows:?}");
        assert!((rows[1] - rows[0] - 5.0).abs() < 1e-6);
        let across = texture(&Params {
            angle: 90.0,
            ..Params::default()
        });
        let mut columns: Vec<f64> = cuts(&across).map(|q| q.x).collect();
        columns.sort_by(f64::total_cmp);
        columns.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        assert_eq!(columns.len(), 20, "rows run along y at 90 degrees");
    }

    #[test]
    fn the_same_seed_gives_the_same_pattern_and_another_differs() {
        let a = texture(&Params::default());
        assert_eq!(a.moves, texture(&Params::default()).moves);
        let b = texture(&Params {
            seed: 2,
            ..Params::default()
        });
        assert_ne!(a.moves, b.moves);
    }

    #[test]
    fn scallops_are_no_longer_than_the_max_cut_length() {
        let p = Params {
            max_overlap_percent: 0.0,
            ..Params::default()
        };
        let t = texture(&p);
        // between two surface points along a row there is at most one scallop
        let surface: Vec<DVec3> = cuts(&t).filter(|q| q.z.abs() < 1e-9).collect();
        for w in surface.windows(2) {
            if (w[0].y - w[1].y).abs() < 1e-9 {
                assert!(w[1].x - w[0].x <= p.max_cut_length + 1e-6, "{w:?}");
            }
        }
    }

    #[test]
    fn selected_vectors_as_a_pattern_are_cut_along_at_one_random_depth() {
        let line = Path::polyline(&[Point::new(0.0, 0.0), Point::new(50.0, 0.0)], false);
        let p = Params {
            use_selected_vectors_as_pattern: true,
            ramp_at_start_and_end: false,
            ..Params::default()
        };
        let t = made_toolpath::<TextureToolpath>(vec![line.clone()], &p).unwrap();
        let depths: Vec<f64> = t
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Cut)
            .map(|m| m.to.z)
            .collect();
        assert!(depths.windows(2).all(|w| (w[0] - w[1]).abs() < 1e-12) && depths[0] < 0.0);
        let ramped = made_toolpath::<TextureToolpath>(
            vec![line],
            &Params {
                ramp_at_start_and_end: true,
                ..p
            },
        )
        .unwrap();
        let z: Vec<f64> = cuts(&ramped).map(|q| q.z).collect();
        assert!(
            z[0].abs() < 1e-9 && z[z.len() - 1].abs() < 1e-9,
            "dips in and out"
        );
    }
}
