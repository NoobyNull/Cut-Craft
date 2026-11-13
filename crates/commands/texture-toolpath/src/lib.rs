//! Texture Toolpath: rows of shallow scallops of random depth and length, as a hand-tooled
//! finish, inside the selected shapes; or the selected vectors themselves cut at random depths.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Path, Point};
use cut_craft_core::glam::DVec3;
use cut_craft_core::polygon::{self, Corners};
use cut_craft_core::random::Random;
use cut_craft_core::toolpath::{
    Move, Tool, contour_moves, cut_points, follow, hatch_lines, made_by, project_onto_model,
};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub struct TextureToolpath;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// What the toolpath is called in the toolpath list
    pub name: String,
    /// Cut along the selected vectors instead of a random pattern inside them
    #[schemars(extend("x-group" = "Texture"))]
    pub use_selected_vectors_as_pattern: bool,
    /// For a pattern: dip in and out along each vector instead of cutting it at one depth
    #[schemars(extend("x-group" = "Texture"))]
    pub ramp_at_start_and_end: bool,
    /// Deepest any scallop goes
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    #[schemars(extend("x-group" = "Cutting Depths"))]
    #[schemars(extend("x-dim" = "D", "x-diagram" = "diagrams/depth_texture.png"))]
    pub max_cut_depth: f64,
    /// Shallowest a scallop goes, as a percentage of the deepest
    #[schemars(range(min = 0, max = 100))]
    #[schemars(extend("x-group" = "Cutting Depths"))]
    pub min_depth_percent: f64,
    /// Longest any scallop is
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    #[schemars(extend("x-group" = "Texture"))]
    pub max_cut_length: f64,
    /// Shortest a scallop is, as a percentage of the longest
    #[schemars(range(min = 0, max = 100))]
    #[schemars(extend("x-group" = "Texture"))]
    pub min_length_percent: f64,
    /// How far a scallop may run back over the one before, as a percentage of the longest
    #[schemars(range(min = 0, max = 100))]
    #[schemars(extend("x-group" = "Texture"))]
    pub max_overlap_percent: f64,
    /// How much the overlap varies, as a percentage of the most it may be
    #[schemars(range(min = 0, max = 100))]
    #[schemars(extend("x-group" = "Texture"))]
    pub overlap_variation_percent: f64,
    /// Between rows of scallops
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    #[schemars(extend("x-group" = "Texture"))]
    pub stepover: f64,
    /// Direction of the rows, anticlockwise from the X axis
    #[schemars(extend("x-unit" = "°"))]
    #[schemars(extend("x-group" = "Texture"))]
    pub angle: f64,
    /// Keeps the texture this far in from the boundary (up to the tool radius in a recess)
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    #[schemars(extend("x-group" = "Boundary"))]
    pub boundary_offset: f64,
    /// Another number gives another random pattern
    #[schemars(extend("x-group" = "Texture"))]
    pub seed: u32,
    /// The bit that cuts the texture, usually a ball nose
    pub tool: Tool,
    /// After calculating, drop the toolpath onto the 3D model: its depths are then below the
    /// model's surface, going no lower than the bottom of the material
    #[serde(default)]
    #[schemars(title = "Project toolpath onto 3D Model")]
    pub project_onto_model: bool,
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
            project_onto_model: false,
        }
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
        let mut moves = vec![];
        for (start, end) in hatch_lines(regions, self.stepover, self.angle)
            .into_iter()
            .flatten()
        {
            let along = |s: f64| start.lerp(end, s / start.distance(end));
            let span = start.distance(end);
            let mut row = vec![];
            let mut x = 0.0;
            while span - x > 1e-6 {
                let length = random.between(self.min_length_percent, self.max_cut_length);
                let to = (x + length).min(span);
                let depth = random.between(self.min_depth_percent, self.max_cut_depth);
                row.extend(dip(along(x), along(to), depth));
                let overlap = self.max_cut_length * self.max_overlap_percent / 100.0
                    * self.overlap_variation_percent
                    / 100.0
                    * random.unit();
                // always move on, by at least a tenth of this scallop
                x = (to - overlap).max(x + (to - x) / 10.0).max(x + 1e-3);
                if x < to {
                    row.push(along(x).extend(0.0));
                }
            }
            moves.extend(follow(&row, safe));
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

/// Texture settings files: the `.vtexture` text files the vendor keeps in its
/// `VectorTextures` folder, so a texture saved by either program loads in the other.
pub const TEXTURE_EXTENSION: &str = "vtexture";

/// The first line of every `.vtexture` file.
const TEXTURE_HEADER: &str = "Vectric Vector Texture Definition";

impl Params {
    /// The texture as a `.vtexture` file: its lengths, depths, overlap and angle, each with
    /// its percentage; not the name, tool, seed or boundary offset.
    #[must_use]
    pub fn to_vtexture(&self) -> String {
        // ponytail: Type 1, Units 1 and the second numbers of Stepover and Angle as in every
        // file seen (all in millimetres); what else they can hold is unknown (re/SPEC.md)
        format!(
            "{TEXTURE_HEADER}\r\nVersion: 1\r\nType: 1\r\nUnits: 1\r\nToolRadius: {:.5}\r\n\
             Length: {:.5} {:.0}\r\nDepth: {:.5} {:.0}\r\nStepover: {:.5} 0\r\n\
             Overlap: {:.5} {:.0}\r\nAngle: {:.5} 0\r\n",
            self.tool.diameter / 2.0,
            self.max_cut_length,
            self.min_length_percent,
            self.max_cut_depth,
            self.min_depth_percent,
            self.stepover,
            self.max_overlap_percent / 100.0,
            self.overlap_variation_percent,
            self.angle,
        )
    }

    /// These settings with the texture in a `.vtexture` file over them.
    pub fn with_vtexture(&self, text: &str) -> Result<Self, String> {
        let mut lines = text.lines().map(str::trim);
        if lines.next() != Some(TEXTURE_HEADER) {
            return Err("not a texture settings file".into());
        }
        let values: HashMap<&str, Vec<f64>> = lines
            .filter_map(|line| {
                let (key, numbers) = line.split_once(':')?;
                let numbers = numbers.split_whitespace().filter_map(|n| n.parse().ok());
                Some((key.trim(), numbers.collect()))
            })
            .collect();
        // a setting's value and the percentage after it
        let pair = |key: &str| -> Result<(f64, f64), String> {
            let v = values.get(key).filter(|v| !v.is_empty());
            let v = v.ok_or_else(|| format!("no {key} in the texture file"))?;
            Ok((v[0], v.get(1).copied().unwrap_or(0.0)))
        };
        let (max_cut_length, min_length_percent) = pair("Length")?;
        let (max_cut_depth, min_depth_percent) = pair("Depth")?;
        let (stepover, _) = pair("Stepover")?;
        let (overlap, overlap_variation_percent) = pair("Overlap")?;
        let (angle, _) = pair("Angle")?;
        Ok(Self {
            max_cut_length,
            min_length_percent,
            max_cut_depth,
            min_depth_percent,
            stepover,
            max_overlap_percent: overlap * 100.0,
            overlap_variation_percent,
            angle,
            ..self.clone()
        })
    }
}

impl Command for TextureToolpath {
    const ID: &'static str = "texture_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Texture Toolpath",
            group: "Toolpath Operations",
            icon: "toolpaths/texture_toolpath.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        p.validate()?;
        let mut random = Random(u64::from(p.seed));
        let safe = doc.job.clearance;
        let (ids, mut moves) = if p.use_selected_vectors_as_pattern {
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
                "the boundary offset leaves nothing to texture. Make the boundary offset smaller"
                    .into(),
            ));
        }
        if p.project_onto_model {
            moves = project_onto_model(doc, &p.tool, &moves)?;
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
    fn projected_onto_the_model_it_textures_its_surface() {
        // nothing selected: the whole 40 mm material, with the 8 mm dome in the middle
        let mut doc = cut_craft_core::testing::domed();
        let p = Params {
            stepover: 1.0,
            project_onto_model: true,
            ..Params::default()
        };
        let change = TextureToolpath::run(&doc, &p).unwrap();
        doc.apply(&change).unwrap();
        let t = doc.toolpaths.into_values().next().unwrap();
        let r = |q: DVec3| q.truncate().distance(Point::new(20.0, 20.0));
        let surface = |q: DVec3| {
            let r = r(q);
            if r < 8.0 {
                (64.0 - r * r).sqrt() - 8.0
            } else {
                -8.0
            }
        };
        // within the scallop depth of the model's surface (and a cell's slope off it), away
        // from the dome's near-vertical rim
        let off = cuts(&t)
            .filter(|&q| !(7.0..8.5).contains(&r(q)))
            .find(|&q| {
                let below = surface(q) - q.z;
                !(-0.5..=p.max_cut_depth + 0.5).contains(&below)
            });
        assert!(off.is_none(), "{off:?} against {:?}", off.map(surface));
        assert!(cuts(&t).any(|q| q.z < -8.0), "the flat round the dome");
        assert!(
            cuts(&t).all(|q| q.z >= -10.0 - 1e-9),
            "not through the bottom"
        );
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

    #[test]
    fn a_texture_saves_to_a_vtexture_file_and_loads_from_one() {
        let p = Params {
            max_cut_length: 20.0,
            min_length_percent: 66.0,
            max_cut_depth: 1.2,
            min_depth_percent: 57.0,
            stepover: 3.0,
            max_overlap_percent: 30.0,
            overlap_variation_percent: 62.0,
            angle: 15.0,
            ..Params::default()
        };
        let text = p.to_vtexture();
        assert!(
            text.starts_with("Vectric Vector Texture Definition\r\n"),
            "{text}"
        );
        assert!(text.contains("\r\nOverlap: 0.30000 62\r\n"), "{text}");
        let q = Params {
            name: "Lid".into(),
            seed: 7,
            ..Params::default()
        }
        .with_vtexture(&text)
        .unwrap();
        assert_eq!(q.to_vtexture(), text, "the same texture again");
        assert!((q.max_overlap_percent - 30.0).abs() < 1e-9);
        assert_eq!((q.name.as_str(), q.seed), ("Lid", 7), "the form's own stay");
    }

    #[test]
    fn a_file_that_is_no_texture_or_lacks_a_setting_is_refused() {
        let p = Params::default();
        assert_eq!(
            p.with_vtexture("Hello\r\nLength: 1 1").unwrap_err(),
            "not a texture settings file"
        );
        let short = "Vectric Vector Texture Definition\nVersion: 1\nLength: 10 50\nDepth: 2 50\n\
                     Stepover: 4 0\nAngle: 0 0\n";
        assert_eq!(
            p.with_vtexture(short).unwrap_err(),
            "no Overlap in the texture file"
        );
    }
}
