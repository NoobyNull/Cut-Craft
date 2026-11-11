//! Create Vector Texture: rows of wavy lines at an angle over the selected closed vectors
//! (or the job), their spacing, waves and noise varied by a seeded pattern; or one of the
//! decorative fills (floral vines, baroque scrolls, frost, snowflakes, fractal branches,
//! stones, ripples, hammered dimples) grown inside them, every line kept a carvable gap from
//! the others and from the edge. On the current layer, or on a layer of their own.

mod bandanna;
mod fill;
mod motif;
mod patterns;
mod scroll;

use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Path, Point, Rect, inside};
use cut_craft_core::random::Random;
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct VectorTexture;

/// What fills the shapes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Style {
    /// Rows of wavy lines at an angle
    #[default]
    #[schemars(title = "Wavy lines")]
    WavyLines,
    /// Curling vines with leaves and flowers
    #[schemars(title = "Floral")]
    Floral,
    /// Broad C and S scrolls wrapped in lobed acanthus leaves, as Dutch and French carving,
    /// mirrored left to right
    #[schemars(title = "Baroque")]
    Baroque,
    /// Western (Sheridan-style) leaf: curling bands, broad wavy leaves with veins and layered
    /// flowers, as tooled leather
    #[schemars(title = "Western leaf")]
    Western,
    /// Ice ferns growing in from the edge, as frost on a window
    #[schemars(title = "Frost")]
    Frost,
    /// Six-armed snowflakes of different sizes
    #[schemars(title = "Snowflakes")]
    Snowflakes,
    /// Branching lines that fork again and again, as fractal (Lichtenberg) burning
    #[schemars(title = "Fractal")]
    Fractal,
    /// Rounded stones with a gap between, as cobbles
    #[schemars(title = "Stones")]
    Stones,
    /// Lines following the edge, each a step further in
    #[schemars(title = "Ripples")]
    Ripples,
    /// Round dimples of a few sizes, as hammered metal
    #[schemars(title = "Hammered")]
    Hammered,
    /// Paisley teardrops with flowers and dots between, as on a bandanna
    #[schemars(title = "Bandanna")]
    Bandanna,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    #[serde(default)]
    #[schemars(extend("x-widget" = "named-icons", "x-icon" = "drawing_forms/fill_{}.png"))]
    pub style: Style,
    /// The lines' angle from the X axis
    #[schemars(extend("x-unit" = "deg", "x-show-when" = {"style": "wavy_lines"}), range(min = -90, max = 90))]
    pub angle: f64,
    #[schemars(title = "Max. spacing", extend("x-unit" = "mm", "x-show-when" = {"style": "wavy_lines"}), range(min = 0))]
    pub max_spacing: f64,
    /// 0 keeps the lines evenly spaced; 100 lets the gaps run from nothing to the maximum
    #[schemars(title = "Spacing variation", extend("x-unit" = "%", "x-show-when" = {"style": "wavy_lines"}), range(min = 0, max = 100))]
    pub variation: f64,
    #[schemars(extend("x-unit" = "mm", "x-show-when" = {"style": "wavy_lines"}), range(min = 0))]
    pub wavelength: f64,
    #[schemars(extend("x-unit" = "mm", "x-show-when" = {"style": "wavy_lines"}), range(min = 0))]
    pub amplitude: f64,
    /// Randomness in the waves
    #[schemars(extend("x-unit" = "%", "x-show-when" = {"style": "wavy_lines"}), range(min = 0, max = 100))]
    pub noise: f64,
    /// How big the pattern's parts are: a scroll's curl, a snowflake, a stone
    #[serde(default = "pattern_size")]
    #[schemars(title = "Pattern size", extend("x-unit" = "mm", "x-hide-when" = {"style": "wavy_lines"}), range(min = 0))]
    pub size: f64,
    /// The least room between two lines, and between a line and the edge. Keep it wider than
    /// your bit cuts, or neighbouring cuts run together: a 60° V-bit 1 mm deep cuts about
    /// 1.2 mm wide
    #[serde(default = "smallest_gap")]
    #[schemars(title = "Smallest gap", extend("x-unit" = "mm", "x-hide-when" = {"style": "wavy_lines"}), range(min = 0))]
    pub gap: f64,
    /// Another seed gives another pattern with the same settings
    #[schemars(title = "Variation", extend("x-widget" = "seed"))]
    pub seed: u32,
    /// Put the lines on the layer named below (made if the job has none of that name)
    #[serde(default)]
    #[schemars(title = "Place vectors on layer")]
    pub place_on_layer: bool,
    #[serde(default = "texture_layer")]
    #[schemars(title = "Name", extend("x-show-when" = {"place_on_layer": true}))]
    pub layer_name: String,
}

fn texture_layer() -> String {
    "Vector Texture".into()
}

fn pattern_size() -> f64 {
    30.0
}

fn smallest_gap() -> f64 {
    3.0
}

impl Default for Params {
    fn default() -> Self {
        Self {
            style: Style::default(),
            angle: 0.0,
            max_spacing: 5.0,
            variation: 0.0,
            wavelength: 40.0,
            amplitude: 2.0,
            noise: 0.0,
            size: pattern_size(),
            gap: smallest_gap(),
            seed: 1,
            place_on_layer: false,
            layer_name: texture_layer(),
        }
    }
}

impl Command for VectorTexture {
    const ID: &'static str = "vector_texture";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Create Vector Texture",
            group: "Create Vectors",
            icon: "drawing/create_vectors/vector_texture.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let (_, boundary) = doc.selected_paths(|path| path.closed);
        let mut rings: Vec<_> = boundary
            .iter()
            .map(|b| b.flatten(cut_craft_core::polygon::TOLERANCE))
            .collect();
        let area = boundary
            .iter()
            .map(Path::bounds)
            .reduce(Rect::union)
            .unwrap_or_else(|| doc.job.area());
        let lines = if p.style == Style::WavyLines {
            ensure_positive("max. spacing", p.max_spacing)?;
            wavy(p, &rings, area)
        } else {
            if rings.is_empty() {
                rings.push(vec![
                    area.min,
                    Point::new(area.max.x, area.min.y),
                    area.max,
                    Point::new(area.min.x, area.max.y),
                ]);
            }
            decorate(p, rings)?
        };
        if lines.is_empty() {
            return Err(CommandError::Failed(
                "the area is too small for the texture. Make the texture smaller, or draw a bigger area".into(),
            ));
        }
        if !p.place_on_layer {
            return Ok(Change::swap(doc, vec![], lines));
        }
        let name = cut_craft_core::ensure_named(&p.layer_name)?;
        let mut c = Change::builder(doc);
        let layer = doc
            .layers
            .iter()
            .find(|l| l.name == name)
            .map_or_else(|| c.add_layer(&name), |l| l.id);
        let ids: Vec<_> = lines.into_iter().map(|l| c.add(l)).collect();
        for &id in &ids {
            c.move_to_layer(id, layer);
        }
        c.select(ids);
        Ok(c.finish())
    }
}

/// Aspire's Vector Texture: rows of wavy lines across `area`, kept inside `rings` (or the
/// area's rectangle when there are none).
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "sample counts are small"
)]
fn wavy(p: &Params, rings: &[Vec<Point>], area: Rect) -> Vec<Path> {
    let inside_area = |q: Point| {
        if rings.is_empty() {
            area.contains(q)
        } else {
            inside(rings, q)
        }
    };
    let (along, across) = {
        let a = p.angle.to_radians();
        (Point::new(a.cos(), a.sin()), Point::new(-a.sin(), a.cos()))
    };
    // cover the area's circle so every angle fills it
    let (centre, radius) = (area.center(), area.size().length() / 2.0);
    let step = (p.max_spacing / 4.0).min(p.wavelength.max(p.max_spacing) / 16.0);
    let samples = ((2.0 * radius / step).ceil() as usize).max(2);
    let mut random = Random(u64::from(p.seed));
    let mut lines = Vec::new();
    let mut offset = -radius;
    while offset <= radius {
        let phase = random.unit() * std::f64::consts::TAU * p.noise / 100.0;
        let wobble = 1.0 + (random.unit() - 0.5) * p.noise / 100.0;
        let mut run: Vec<Point> = Vec::new();
        for k in 0..=samples {
            let s = -radius + 2.0 * radius * k as f64 / samples as f64;
            let wave = if p.wavelength > 0.0 {
                p.amplitude * wobble * (s / p.wavelength * std::f64::consts::TAU + phase).sin()
            } else {
                0.0
            };
            let q = centre + along * s + across * (offset + wave);
            if inside_area(q) {
                run.push(q);
            } else if run.len() > 1 {
                lines.push(Path::polyline(&std::mem::take(&mut run), false));
            } else {
                run.clear();
            }
        }
        if run.len() > 1 {
            lines.push(Path::polyline(&run, false));
        }
        let gap = p.max_spacing * (1.0 - p.variation / 100.0 * random.unit());
        offset += gap.max(p.max_spacing / 100.0);
    }
    lines
}

/// The most vectors a decorative fill may make before it is judged too fine for the area.
const MOST_PIECES: usize = 40_000;

/// One of the decorative fills grown inside `rings`.
fn decorate(p: &Params, rings: Vec<Vec<Point>>) -> Result<Vec<Path>, CommandError> {
    check_sizes(p.size, p.gap)?;
    let area = fill::Area::new(rings);
    let mut g = fill::Grower::new(&area, p.gap, p.seed);
    let paths = match p.style {
        Style::WavyLines => unreachable!("wavy lines are drawn on their own"),
        Style::Floral => fill::paths(
            &scroll::scrolls(&mut g, p.size, scroll::Look::Floral).details,
            p.gap,
        ),
        Style::Western => {
            let made = motif::ornament(&mut g, p.size, scroll::Look::Western, None);
            merged(&made, p.gap)
        }
        Style::Baroque => baroque(&mut g, p.size),
        Style::Frost => fill::paths(&patterns::frost(&mut g, p.size), p.gap),
        Style::Snowflakes => fill::paths(&patterns::snowflakes(&mut g, p.size), p.gap),
        Style::Fractal => fill::paths(&patterns::fractal(&mut g, p.size), p.gap),
        Style::Stones => patterns::stones(&mut g, p.size).ok_or_else(too_fine)?,
        Style::Ripples => patterns::ripples(&area, p.size, p.gap),
        Style::Hammered => patterns::hammered(&mut g, p.size),
        Style::Bandanna => merged(&bandanna::bandanna(&mut g, p.size), p.gap),
    };
    if g.full() || paths.len() > MOST_PIECES {
        return Err(too_fine());
    }
    Ok(paths)
}

/// Positive sizes, the pattern at least three gaps across.
fn check_sizes(size: f64, gap: f64) -> Result<(), CommandError> {
    ensure_positive("pattern size", size)?;
    ensure_positive("smallest gap", gap)?;
    if size < gap * 3.0 {
        return Err(CommandError::Failed(format!(
            "the pattern size must be at least three times the smallest gap ({} mm). Make the \
             pattern size bigger or the gap smaller",
            gap * 3.0
        )));
    }
    Ok(())
}

fn too_fine() -> CommandError {
    CommandError::Failed(
        "the pattern is too fine for an area this big. Make the pattern size or the smallest \
         gap bigger"
            .into(),
    )
}

/// Baroque scrolls, mirrored and merged into outlines.
fn baroque(g: &mut fill::Grower, size: f64) -> Vec<Path> {
    let made = mirrored_scrolls(g, size);
    merged(&made, g.gap())
}

/// Baroque scrolls: grown on the left of the area's middle, inside the area and its mirror
/// image, then mirrored, so the two halves meet in the middle.
fn mirrored_scrolls(g: &mut fill::Grower, size: f64) -> scroll::Made {
    let axis = g.area.bounds.center().x;
    let mirrored: Vec<Vec<Point>> = g
        .area
        .rings
        .iter()
        .map(|r| {
            r.iter()
                .map(|q| Point::new(2.0 * axis - q.x, q.y))
                .collect()
        })
        .collect();
    for ring in &mirrored {
        g.space.wall(ring, true);
    }
    let line = g.space.stroke();
    let b = g.area.bounds;
    for q in fill::along(
        Point::new(axis, b.min.y),
        Point::new(axis, b.max.y),
        g.gap() / 3.0,
    ) {
        g.space.add(q, 0.0, line, 0);
    }
    let made = motif::ornament(g, size, scroll::Look::Baroque, Some((line, axis)));
    let both = |strokes: &[fill::Stroke]| -> Vec<fill::Stroke> {
        strokes
            .iter()
            .flat_map(|s| [s.clone(), s.mirrored(axis)])
            .collect()
    };
    let mirror = |p: Point| Point::new(2.0 * axis - p.x, p.y);
    scroll::Made {
        shapes: both(&made.shapes),
        details: both(&made.details),
        pieces: made
            .pieces
            .iter()
            .flat_map(|piece| [piece.clone(), piece.map(mirror)])
            .collect(),
    }
}

/// The two fills composed of whole scrolls, which can also be carved as a relief.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Ornament {
    /// Broad C and S scrolls wrapped in lobed acanthus leaves, as Dutch and French carving,
    /// mirrored left to right
    #[default]
    #[schemars(title = "Baroque")]
    Baroque,
    /// Western (Sheridan-style) leaf: curling bands, broad wavy leaves and layered flowers
    #[schemars(title = "Western leaf")]
    Western,
}

/// The scrolls of `style` composed inside `rings` as raised pieces, lowest first: what
/// the fill of that style would outline, to carve as a relief.
///
/// # Errors
/// When the sizes are not positive, the pattern is under three gaps across, or it is too
/// fine for the area.
pub fn ornament_pieces(
    style: Ornament,
    rings: Vec<Vec<Point>>,
    size: f64,
    gap: f64,
    seed: u32,
) -> Result<Vec<scroll::Piece>, CommandError> {
    check_sizes(size, gap)?;
    let area = fill::Area::new(rings);
    let mut g = fill::Grower::new(&area, gap, seed);
    g.lush = true;
    let made = match style {
        Ornament::Baroque => mirrored_scrolls(&mut g, size),
        Ornament::Western => motif::ornament(&mut g, size, scroll::Look::Western, None),
    };
    if g.full() {
        return Err(too_fine());
    }
    Ok(made.pieces)
}

pub use scroll::{Part, Piece};

/// Scrolls' stems and leaves merged into whole shapes, and their details kept apart.
fn merged(made: &scroll::Made, gap: f64) -> Vec<Path> {
    use cut_craft_core::polygon::{Corners, offset_regions, union};
    // pieces nearer each other than the gap are joined (grown by half the gap and shrunk
    // back), then slivers too small to carve are dropped
    let joined = union(&fill::paths(&made.shapes, gap));
    let grown = offset_regions(&joined, gap / 2.0, Corners::Round);
    let mut paths: Vec<Path> = offset_regions(&grown, -gap / 2.0, Corners::Round)
        .into_iter()
        .filter(|p| {
            let size = p.bounds().size();
            size.x.max(size.y) >= gap && cut_craft_core::polygon::area(p) >= gap * gap * 0.5
        })
        .collect();
    let details = clear_of(&made.details, &paths, gap);
    paths.extend(fill::paths(&details, gap));
    paths
}

/// The parts of the details at least the gap from every outline, each long enough to carve
/// (three gaps; shorter scraps read as scratches).
#[expect(clippy::cast_possible_truncation, reason = "grid cells")]
fn clear_of(details: &[fill::Stroke], outlines: &[Path], gap: f64) -> Vec<fill::Stroke> {
    let key = |q: Point| ((q.x / gap).floor() as i64, (q.y / gap).floor() as i64);
    let mut grid: std::collections::HashMap<(i64, i64), Vec<Point>> =
        std::collections::HashMap::default();
    for path in outlines {
        let points = path.flatten(0.01);
        for w in points.windows(2) {
            for q in fill::along(w[0], w[1], gap / 4.0) {
                grid.entry(key(q)).or_default().push(q);
            }
        }
    }
    let clear = |q: Point| {
        let (cx, cy) = key(q);
        (cx - 1..=cx + 1).all(|x| {
            (cy - 1..=cy + 1).all(|y| {
                grid.get(&(x, y))
                    .is_none_or(|near| near.iter().all(|r| r.distance(q) >= gap))
            })
        })
    };
    let mut out = Vec::new();
    for d in details {
        let mut points = d.points.clone();
        if d.closed {
            points.push(points[0]);
        }
        if points.iter().all(|&q| clear(q)) {
            out.push(d.clone());
            continue;
        }
        let mut run: Vec<Point> = Vec::new();
        for q in points.into_iter().chain([Point::splat(f64::NAN)]) {
            if !q.x.is_nan() && clear(q) {
                run.push(q);
            } else if !run.is_empty() {
                let length: f64 = run.windows(2).map(|w| w[0].distance(w[1])).sum();
                if length >= gap * 3.0 {
                    out.push(fill::Stroke::open(std::mem::take(&mut run)));
                }
                run.clear();
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{document_with, square};

    cut_craft_core::contract_tests!(VectorTexture);

    fn textured(p: &Params) -> Vec<Path> {
        let mut doc = document_with(vec![square(0.0, 0.0, 50.0)]);
        let change = VectorTexture::run(&doc, p).unwrap();
        doc.apply(&change).unwrap();
        doc.vectors.values().skip(1).cloned().collect()
    }

    #[test]
    fn straight_lines_fill_the_square_at_the_spacing() {
        let lines = textured(&Params {
            amplitude: 0.0,
            ..Params::default()
        });
        assert_eq!(lines.len(), 10, "50 mm at 5 mm spacing");
        for l in &lines {
            let b = l.bounds();
            assert!(b.size().y < 1e-9 && b.min.x >= -1e-9 && b.max.x <= 50.0 + 1e-9);
        }
    }

    #[test]
    fn the_lines_can_go_on_a_layer_of_their_own() {
        let mut doc = document_with(vec![square(0.0, 0.0, 50.0)]);
        let p = Params {
            place_on_layer: true,
            layer_name: "Texture".into(),
            ..Params::default()
        };
        doc.apply(&VectorTexture::run(&doc, &p).unwrap()).unwrap();
        let layer = doc
            .layers
            .iter()
            .find(|l| l.name == "Texture")
            .expect("made");
        let on = doc
            .vectors
            .keys()
            .filter(|&&id| doc.layer_of(id) == layer.id)
            .count();
        assert_eq!(on, doc.vectors.len() - 1, "every line, not the square");
        // a second texture goes on the same layer, not a new one
        let layers = doc.layers.len();
        doc.apply(&VectorTexture::run(&doc, &p).unwrap()).unwrap();
        assert_eq!(doc.layers.len(), layers);
    }

    #[test]
    fn waves_stay_inside_and_seeds_vary_noisy_patterns() {
        let wavy = textured(&Params::default());
        assert!(
            wavy.iter().any(|l| l.bounds().size().y > 3.0),
            "the waves rise and fall"
        );
        for l in &wavy {
            for q in l.flatten(0.01) {
                assert!(Rect::new(Point::ZERO, Point::splat(50.0)).contains(q));
            }
        }
        let noisy = |seed| {
            textured(&Params {
                noise: 80.0,
                variation: 50.0,
                seed,
                ..Params::default()
            })
        };
        assert_eq!(noisy(3), noisy(3));
        assert_ne!(noisy(3), noisy(4));
    }

    const FILLS: [Style; 10] = [
        Style::Floral,
        Style::Baroque,
        Style::Western,
        Style::Frost,
        Style::Snowflakes,
        Style::Fractal,
        Style::Stones,
        Style::Ripples,
        Style::Hammered,
        Style::Bandanna,
    ];

    fn fill(style: Style, seed: u32, boundary: Vec<Path>) -> Vec<Path> {
        let mut doc = document_with(boundary);
        let before = doc.vectors.len();
        let p = Params {
            style,
            seed,
            ..Params::default()
        };
        let change = VectorTexture::run(&doc, &p).unwrap();
        doc.apply(&change).unwrap();
        let made: Vec<Path> = doc.vectors.values().skip(before).cloned().collect();
        assert!(!made.is_empty(), "{style:?} made nothing");
        made
    }

    /// Points along `path` at most `step` apart, its corners among them.
    fn samples(path: &Path, step: f64) -> Vec<Point> {
        let pts = path.flatten(0.01);
        let mut out = vec![pts[0]];
        for w in pts.windows(2) {
            out.extend(fill::along(w[0], w[1], step));
        }
        out
    }

    fn distance_to_square_edge(q: Point, side: f64) -> f64 {
        q.x.min(q.y).min(side - q.x).min(side - q.y)
    }

    #[test]
    fn every_fill_stays_inside_and_clear_of_the_edge_by_the_gap() {
        for style in FILLS {
            for path in fill(style, 1, vec![square(0.0, 0.0, 100.0)]) {
                for q in samples(&path, 0.5) {
                    let d = distance_to_square_edge(q, 100.0);
                    assert!(d >= 3.0 * 0.95, "{style:?}: {q} is {d} mm from the edge");
                }
            }
        }
    }

    /// The pieces (paths touching one another counted as one piece), and the least distance
    /// between two of them.
    #[expect(clippy::cast_possible_truncation, reason = "grid cells")]
    fn closest_pieces(paths: &[Path], gap: f64) -> f64 {
        fn root(piece: &mut [usize], mut i: usize) -> usize {
            while piece[i] != i {
                piece[i] = piece[piece[i]];
                i = piece[i];
            }
            i
        }
        let pts: Vec<(Point, usize)> = paths
            .iter()
            .enumerate()
            .flat_map(|(i, p)| samples(p, gap / 4.0).into_iter().map(move |q| (q, i)))
            .collect();
        let cell = gap;
        let key = |q: Point| ((q.x / cell).floor() as i64, (q.y / cell).floor() as i64);
        let mut grid: std::collections::HashMap<(i64, i64), Vec<usize>> =
            std::collections::HashMap::default();
        for (k, &(q, _)) in pts.iter().enumerate() {
            grid.entry(key(q)).or_default().push(k);
        }
        let near = |q: Point, each: &mut dyn FnMut(usize)| {
            let (cx, cy) = key(q);
            for x in cx - 1..=cx + 1 {
                for y in cy - 1..=cy + 1 {
                    for &k in grid.get(&(x, y)).into_iter().flatten() {
                        each(k);
                    }
                }
            }
        };
        // paths that touch are one piece
        let mut piece: Vec<usize> = (0..paths.len()).collect();
        for &(q, i) in &pts {
            near(q, &mut |k| {
                let (r, j) = pts[k];
                if j != i && r.distance(q) < gap * 0.3 {
                    let (a, b) = (root(&mut piece, i), root(&mut piece, j));
                    piece[a] = b;
                }
            });
        }
        let mut closest = f64::INFINITY;
        for &(q, i) in &pts {
            near(q, &mut |k| {
                let (r, j) = pts[k];
                if root(&mut piece.clone(), i) != root(&mut piece.clone(), j) {
                    closest = closest.min(r.distance(q));
                }
            });
        }
        closest
    }

    #[test]
    fn separate_pieces_keep_the_smallest_gap_between_them() {
        for style in FILLS {
            let made = fill(style, 2, vec![square(0.0, 0.0, 90.0)]);
            let closest = closest_pieces(&made, 3.0);
            assert!(closest >= 3.0 * 0.9, "{style:?}: pieces {closest} mm apart");
        }
    }

    #[test]
    fn no_piece_is_too_small_to_carve() {
        for style in FILLS {
            for path in fill(style, 3, vec![square(0.0, 0.0, 100.0)]) {
                let b = path.bounds().size();
                assert!(
                    b.x.max(b.y) >= 3.0,
                    "{style:?}: a piece only {b} across, closed {}",
                    path.closed
                );
            }
        }
    }

    #[test]
    fn a_fill_is_the_same_for_the_same_variation_and_new_for_another() {
        for style in FILLS {
            let one = fill(style, 4, vec![square(0.0, 0.0, 80.0)]);
            assert_eq!(
                one,
                fill(style, 4, vec![square(0.0, 0.0, 80.0)]),
                "{style:?}"
            );
            if style != Style::Ripples {
                assert_ne!(
                    one,
                    fill(style, 5, vec![square(0.0, 0.0, 80.0)]),
                    "{style:?}"
                );
            }
        }
    }

    #[test]
    fn fills_keep_out_of_holes() {
        let hole = square(30.0, 30.0, 40.0).reversed();
        for style in FILLS {
            let made = fill(style, 6, vec![square(0.0, 0.0, 100.0), hole.clone()]);
            for path in made {
                for q in samples(&path, 0.5) {
                    let dx = (30.0 - q.x).max(q.x - 70.0).max(0.0);
                    let dy = (30.0 - q.y).max(q.y - 70.0).max(0.0);
                    let from_hole = dx.hypot(dy);
                    assert!(from_hole >= 3.0 * 0.95, "{style:?}: {q} in or by the hole");
                }
            }
        }
    }

    #[test]
    fn baroque_scrolls_mirror_left_to_right() {
        let made = fill(Style::Baroque, 7, vec![square(0.0, 0.0, 120.0)]);
        let pts: Vec<Point> = made.iter().flat_map(|p| samples(p, 0.5)).collect();
        for q in &pts {
            let m = Point::new(120.0 - q.x, q.y);
            let nearest = pts
                .iter()
                .map(|r| r.distance(m))
                .fold(f64::INFINITY, f64::min);
            assert!(nearest < 0.5, "{q} has no mirror image ({nearest} mm)");
        }
    }

    #[test]
    fn a_pattern_smaller_than_three_gaps_says_how_to_fix_it() {
        let doc = document_with(vec![square(0.0, 0.0, 50.0)]);
        let p = Params {
            style: Style::Floral,
            size: 6.0,
            gap: 3.0,
            ..Params::default()
        };
        let Err(CommandError::Failed(why)) = VectorTexture::run(&doc, &p) else {
            panic!("too small a pattern ran");
        };
        assert!(why.contains("pattern size bigger"), "{why}");
    }

    #[test]
    fn a_fill_with_nothing_selected_fills_the_job() {
        let mut doc = document_with(vec![]);
        let p = Params {
            style: Style::Snowflakes,
            ..Params::default()
        };
        doc.apply(&VectorTexture::run(&doc, &p).unwrap()).unwrap();
        assert!(doc.vectors.len() > 10);
    }

    #[test]
    fn older_settings_without_a_style_still_make_wavy_lines() {
        let old = serde_json::json!({
            "angle": 0.0, "max_spacing": 5.0, "variation": 0.0, "wavelength": 40.0,
            "amplitude": 0.0, "noise": 0.0, "seed": 1
        });
        let p: Params = serde_json::from_value(old).unwrap();
        assert_eq!(p.style, Style::WavyLines);
        assert_eq!(textured(&p).len(), 10);
    }

    /// Writes each fill as an SVG to the folder named by `FILL_PREVIEWS`, to look at.
    #[test]
    #[ignore = "writes pictures to look at"]
    fn draw_previews() {
        use std::fmt::Write;
        let dir = std::env::var("FILL_PREVIEWS").unwrap();
        let triangle = std::env::var("FILL_TRIANGLE").is_ok();
        let shape = |style| {
            if triangle {
                Path::polyline(
                    &[
                        Point::new(2.0, 2.0),
                        Point::new(198.0, 2.0),
                        Point::new(100.0, 148.0),
                    ],
                    true,
                )
            } else if style == Style::Baroque {
                // an oval cartouche
                Path::polyline(
                    &(0..96)
                        .map(|i| {
                            let a = std::f64::consts::TAU * f64::from(i) / 96.0;
                            Point::new(100.0 + 95.0 * a.cos(), 70.0 + 65.0 * a.sin())
                        })
                        .collect::<Vec<_>>(),
                    true,
                )
            } else {
                square(0.0, 0.0, 150.0)
            }
        };
        for style in FILLS {
            let made = fill(style, 1, vec![shape(style)]);
            let mut svg = String::from(
                "<svg xmlns='http://www.w3.org/2000/svg' viewBox='-5 -5 210 160' width='840' height='640'><rect x='-5' y='-5' width='210' height='160' fill='white'/>",
            );
            for path in std::iter::once(shape(style)).chain(made) {
                let pts = path.flatten(0.05);
                let d: Vec<String> = pts
                    .iter()
                    .map(|q| format!("{:.2},{:.2}", q.x, 150.0 - q.y))
                    .collect();
                let tag = if path.closed { "polygon" } else { "polyline" };
                let _ = write!(
                    svg,
                    "<{tag} points='{}' fill='none' stroke='black' stroke-width='0.6'/>",
                    d.join(" ")
                );
            }
            svg += "</svg>";
            std::fs::write(format!("{dir}/{style:?}.svg"), svg).unwrap();
        }
    }
}
