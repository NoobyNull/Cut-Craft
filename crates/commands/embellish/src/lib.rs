//! Embellish: ornament designed for the selected shape or line rather than cut out of a
//! pattern. A closed shape is filled: Folk mix, Scrolls and Flowers are designed for it
//! (a vine down its middle with scrolls, leaf fans in its points, flowers where it is
//! roomiest, mirrored when the shape repeats round its middle), and the patterns (Band and
//! beads, Rope, Chain, Twist, Braid, Bead and reel, Laurel leaves, Feathers, Chevrons) fill
//! it row after row inward from the edge. An open line has the style run along it, end to
//! end. Reverse runs a pattern the other way along its line; Reset puts every setting back
//! to its default and makes it again as new; Apply makes changes to one already made (select
//! it, or press E on it, to open it again). The Sequence says how each repeat is drawn in
//! turn (a space, flipped, back to front, turned, a random size or lean), typed or added
//! with the pattern maker's sample buttons under it. Every piece keeps the smallest gap from the
//! others, so the finishing bit can carve between them. The pieces come out as vectors, as a
//! 3D component, or both.

#![allow(
    clippy::many_single_char_names,
    clippy::similar_names,
    reason = "geometry written as its formulas: points p, q, a, b, gaps g, sizes s"
)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    reason = "counts and grid cells of shapes no bigger than a sheet of material"
)]
#![allow(
    clippy::too_many_lines,
    clippy::too_many_arguments,
    reason = "each design is one sequence of steps, read top to bottom, as its sketch was"
)]

mod border;
mod design;
mod field;
mod free;
mod motif;
mod occ;
mod relief;
mod rng;
mod seq;
mod sym;
mod vine;

use cut_craft_core::component::{Combine, Component};
use cut_craft_core::document::{Document, EntityId, LayerId};
use cut_craft_core::geometry::{Path, Point, Rect};
use cut_craft_core::trace::{Fit, fit};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use design::{Motifs, Settings, Where};
use field::Field;
use motif::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct Embellish;

/// The one look the embellishment has. Every style fills a closed shape and runs along an
/// open line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Style {
    /// A band and bead row round the edge, and inside it a vine with scrolls, flowers, leaf
    /// fans and small curls together (along a line, the vine and its scrolls)
    #[default]
    #[schemars(title = "Folk mix")]
    FolkMix,
    /// A vine down the middle with scrolls curling off it, and small S-curls
    Scrolls,
    /// Rosettes where there is most room, and smaller flowers round them
    Flowers,
    /// A rounded band with a row of beads beside it
    #[schemars(title = "Band and beads")]
    BandBorder,
    /// Slanted strands laid side by side
    #[schemars(title = "Rope")]
    Rope,
    /// Oval links seen face on, joined by links seen edge on
    #[schemars(title = "Chain")]
    Chain,
    /// Two strands twisting round each other
    #[schemars(title = "Twist")]
    Twist,
    /// Three strands plaited over and under
    #[schemars(title = "Braid")]
    Braid,
    /// Oval beads with a pair of thin discs between each two
    #[schemars(title = "Bead and reel")]
    BeadAndReel,
    /// Pairs of laurel leaves along a stem, as in a wreath
    #[schemars(title = "Laurel leaves")]
    Laurel,
    /// Feathers one after another, each with a quill and barbs
    #[schemars(title = "Feathers")]
    Feathers,
    /// V shapes one after another, all pointing the same way
    #[schemars(title = "Chevrons")]
    Chevrons,
}

impl Style {
    /// Where it goes, and what goes inside.
    fn layout(self) -> (Where, Motifs) {
        // every style is a pattern now, filling a shape row after row
        let _ = self;
        (Where::Border, Motifs::Nothing)
    }

    /// The patterned border's own pattern, for the patterned borders.
    #[expect(
        clippy::unnecessary_wraps,
        reason = "every style is a pattern for now; the shape-aware designs take none"
    )]
    fn pattern(self) -> Option<border::Style> {
        Some(match self {
            Self::Rope => border::Style::Rope,
            Self::Chain => border::Style::Chain,
            Self::Twist => border::Style::Twist,
            Self::Braid => border::Style::Braid,
            Self::BeadAndReel => border::Style::BeadAndReel,
            Self::Laurel => border::Style::Laurel,
            Self::Feathers => border::Style::Feathers,
            Self::Chevrons => border::Style::Chevrons,
            Self::BandBorder => border::Style::BandBeads,
            Self::FolkMix => border::Style::Vine(vine::Kind::FolkMix),
            Self::Scrolls => border::Style::Vine(vine::Kind::Scrolls),
            Self::Flowers => border::Style::Vine(vine::Kind::Flowers),
        })
    }
}

/// Which way the carving's pieces go.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// Every piece stands up from the material
    #[default]
    Raised,
    /// Every piece is hollowed into the material, as an incised carving
    #[schemars(title = "Sunk in")]
    Sunk,
    /// The border and the bigger pieces stand up; the small fillers are hollowed in
    Mixed,
}

/// What the ornament is made as.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Make {
    /// The pieces' outlines, to carve with 2D toolpaths
    Vectors,
    /// A 3D component, each piece rounded over, for 3D roughing and finishing
    #[schemars(title = "3D carving")]
    Carving,
    /// The outlines and the 3D component together
    #[default]
    Both,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
#[expect(clippy::struct_excessive_bools, reason = "a form's tick boxes")]
pub struct Params {
    /// The one look it has. A closed shape is filled with it; an open line has it run
    /// along the line
    #[schemars(extend("x-section" = true, "x-widget" = "named-icons", "x-icon" = "drawing_forms/embellish_{}.png"))]
    pub style: Style,
    /// How wide one row of the pattern is. A closed shape is filled row after row inward; on
    /// a shape narrower than this it is made narrower to fit, one row down the middle
    #[schemars(title = "Width", range(min = 0), extend("x-group" = "Pattern", "x-unit" = "mm"))]
    pub border_width: f64,
    /// How long each repeat of the pattern is, against its width; the count is rounded so
    /// the pattern meets itself
    #[schemars(title = "Repeat length", range(min = 50, max = 200), extend("x-group" = "Pattern", "x-unit" = "%", "x-widget" = "slider"))]
    pub repeat: u32,
    /// Run the pattern the other way along its line: chevrons, feathers, leaves and scrolls
    /// point back, and the rope and plaits twist the other way
    #[schemars(extend("x-group" = "Pattern"))]
    pub reverse: bool,
    /// How steeply the rope's strands lie
    #[schemars(title = "Twist angle", range(min = 20, max = 60), extend("x-group" = "Pattern", "x-unit" = "°", "x-widget" = "slider", "x-show-when" = {"style": "rope"}))]
    pub twist: u32,
    /// How far in from the shape's edge the pattern starts
    #[schemars(title = "Gap from the edge", range(min = 0), extend("x-group" = "Pattern", "x-unit" = "mm"))]
    pub border_inset: f64,
    /// How each repeat is drawn, in turn: 0 a space, 1 as designed, 2 flipped top to
    /// bottom, 3 back to front, 4 both. /n turns it by 180° ÷ n (1/4 is 45°), R turns it the
    /// other way, S gives it a random size and T a random lean; S or T before brackets
    /// reaches all inside, as ST(1,2). Split entries with commas. It starts again when it
    /// runs out, nothing added between. Empty is 1 over and over; 12 makes the vine wave.
    /// The buttons under it add each sample
    #[schemars(extend("x-group" = "Sequence", "x-widget" = "sequence", "x-hide-when" = {"style": ["twist", "braid"]}))]
    pub sequence: String,
    /// Thin raised lines along both sides of the pattern, framing it
    #[schemars(title = "Edge lines", extend("x-group" = "Pattern"))]
    pub rails: bool,
    /// The wavy stem down the middle that the other pieces grow from
    #[schemars(extend("x-group" = "Include", "x-show-when" = {"style": ["folk_mix", "scrolls", "flowers"]}))]
    pub vine: bool,
    /// Scrolls curling off the vine, one side then the other
    #[schemars(extend("x-group" = "Include", "x-show-when" = {"style": ["folk_mix", "scrolls"]}))]
    pub scrolls: bool,
    /// Flowers: the main pieces for Flowers, small ones beside the scrolls for Folk mix
    #[schemars(extend("x-group" = "Include", "x-show-when" = {"style": ["folk_mix", "flowers"]}))]
    pub flowers: bool,
    /// Pairs of leaves in the vine's bends (for Folk mix, where a flower does not fit)
    #[schemars(extend("x-group" = "Include", "x-show-when" = {"style": ["folk_mix", "scrolls", "flowers"]}))]
    pub leaves: bool,
    /// A dot where the vine crosses the middle, top then bottom
    #[schemars(extend("x-group" = "Include", "x-show-when" = {"style": ["folk_mix", "scrolls", "flowers"]}))]
    pub dots: bool,
    /// Low makes plain pieces; high adds eyes and bead rows to the scrolls and makes the
    /// flowers rosettes
    #[schemars(title = "Detail", range(min = 1, max = 10), extend("x-group" = "Ornament", "x-widget" = "slider", "x-show-when" = {"style": ["folk_mix", "scrolls", "flowers"]}))]
    pub refinement: u32,
    /// How much of its place each scroll, flower or leaf fills: low leaves more open ground
    #[schemars(title = "Piece size", range(min = 50, max = 160), extend("x-group" = "Ornament", "x-unit" = "%", "x-widget" = "slider", "x-show-when" = {"style": ["folk_mix", "scrolls", "flowers"]}))]
    pub scroll_size: u32,
    /// Each piece turns to follow the line it sits on; off, they all stay upright and only
    /// move along it
    #[schemars(title = "Turn with the line", extend("x-group" = "Ornament", "x-show-when" = {"style": ["folk_mix", "scrolls", "flowers"]}))]
    pub turn_with_line: bool,
    /// Each repeat a different size, from the Layout number (off: every repeat the same
    /// size)
    #[schemars(extend("x-group" = "Sequence", "x-hide-when" = {"style": ["twist", "braid"]}))]
    pub random_size: bool,
    /// Each repeat leaning a different way, up to 45° either side, from the Layout number
    /// (off: every repeat turned only as the Sequence says)
    #[schemars(extend("x-group" = "Sequence", "x-hide-when" = {"style": ["twist", "braid"]}))]
    pub random_rotation: bool,
    /// No longer used: the patterns repeat evenly
    #[schemars(skip)]
    pub objects: u32,
    /// How close the pieces sit: the smallest gap between them is 3 mm at 1 and 0.75 mm at
    /// 10. Finish with a ball-nose bit no wider than the gap
    #[schemars(range(min = 1, max = 10), extend("x-group" = "Fit to your bit", "x-widget" = "slider"))]
    pub tightness: u32,
    /// Outlines to cut with 2D toolpaths, a rounded 3D carving, or both
    #[schemars(extend("x-group" = "Make", "x-widget" = "row"))]
    pub make: Make,
    /// Picks the shape of the pieces: how tight the scrolls curl, the flowers' petals, how
    /// wide the leaves fan, the same in every repeat; with Random size or Random rotation,
    /// also how the repeats differ. 1 is the pattern as designed
    #[schemars(title = "Layout", extend("x-group" = "Sequence", "x-widget" = "seed", "x-hide-when" = {"style": ["twist", "braid"]}))]
    pub seed: u32,
    /// Names the new layer the ornament's vectors go on, and its 3D component
    #[schemars(extend("x-group" = "Make"))]
    pub name: String,
    /// Whether the 3D carving's pieces stand up, are hollowed in, or both
    #[schemars(extend("x-group" = "3D carving", "x-widget" = "row", "x-hide-when" = {"make": "vectors"}))]
    pub direction: Direction,
    /// How far the carving's raised pieces stand above the material (not used when every
    /// piece is sunk in)
    #[schemars(title = "Raise height", extend("x-group" = "3D carving", "x-unit" = "mm", "x-hide-when" = {"make": "vectors"}), range(min = 0))]
    pub height: f64,
    /// How deep the carving's hollowed pieces go into the material
    #[schemars(title = "Sink depth", extend("x-group" = "3D carving", "x-unit" = "mm", "x-show-when" = {"direction": ["sunk", "mixed"]}), range(min = 0))]
    pub depth: f64,
    /// Small dots punched into the open ground of the 3D carving
    #[schemars(title = "Stippled ground", extend("x-group" = "3D carving", "x-hide-when" = {"make": "vectors"}))]
    pub stipple: bool,
    /// How the 3D carving joins any 3D already in the job
    #[schemars(extend("x-group" = "3D carving", "x-hide-when" = {"make": "vectors"}))]
    pub combine: Combine,
    /// The embellishment being edited: it is made again, in place, from the outlines it keeps
    #[schemars(skip)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editing: Option<Editing>,
}

/// Where an embellishment Embellish made lives: its layer of vectors, its 3D component, or
/// both.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Editing {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<LayerId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub component: Option<EntityId>,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            style: Style::FolkMix,
            border_width: 30.0,
            repeat: 100,
            twist: 45,
            border_inset: 3.0,
            rails: false,
            sequence: String::new(),
            reverse: false,
            refinement: 6,
            objects: 6,
            scroll_size: 100,
            vine: true,
            scrolls: true,
            flowers: true,
            leaves: true,
            dots: true,
            turn_with_line: true,
            random_size: false,
            random_rotation: false,
            tightness: 7,
            direction: Direction::Raised,
            height: 3.0,
            depth: 2.0,
            make: Make::Both,
            stipple: false,
            seed: 4,
            name: "Embellishment".into(),
            combine: Combine::Add,
            editing: None,
        }
    }
}

impl Params {
    /// The smallest gap between pieces, in mm.
    pub fn gap(&self) -> f64 {
        3.25 - 0.25 * f64::from(self.tightness.clamp(1, 10))
    }

    /// The patterned border, when one is chosen.
    fn pattern(&self) -> Option<border::Spec> {
        let style = self.style.pattern()?;
        Some(border::Spec {
            style,
            width: self.border_width,
            repeat: f64::from(self.repeat.clamp(50, 200)) / 100.0,
            lay: f64::from(self.twist.clamp(20, 60)).to_radians(),
            inset: self.border_inset.max(0.0),
            rails: self.rails,
            single: false,
            reverse: self.reverse,
            vine: border::VineSpec {
                parts: vine::Parts {
                    vine: self.vine,
                    scrolls: self.scrolls,
                    flowers: self.flowers,
                    leaves: self.leaves,
                    dots: self.dots,
                },
                detail: f64::from(self.refinement.clamp(1, 10) - 1) / 9.0,
                fill: f64::from(self.scroll_size.clamp(50, 160) - 50) / 110.0,
            },
            upright: !self.turn_with_line,
            sequence: seq::Seq::parse(&self.sequence),
            vary: border::Vary {
                size: self.random_size,
                turn: self.random_rotation,
                seed: self.seed,
            },
        })
    }

    fn settings(&self, seed: u32, gap: f64) -> Settings {
        let unit = |v: u32| f64::from(v.clamp(1, 10) - 1) / 9.0;
        Settings {
            gap,
            detail: unit(self.refinement),
            density: unit(self.objects),
            scale: f64::from(self.scroll_size.clamp(50, 160)) / 100.0,
            place: self.style.layout().0,
            motifs: self.style.layout().1,
            stipple: self.stipple,
            seed,
            inner_from: 0.0,
        }
    }
}

/// Room left round a shape in its field, mm.
const MARGIN: f64 = 6.0;

/// The most cells the carving may have; a bigger area gets coarser cells.
const MOST_CELLS: f64 = 4_000_000.0;

/// One shape to embellish: an outline and the holes in it, and where its field sits. For an
/// open line, its field is the band round it; with a patterned border, `line` is the line
/// itself (in the field's frame), the pattern running along it, and with an ornament it is
/// empty.
struct Region {
    field: Field,
    offset: Point,
    line: Option<Vec<Point>>,
}

/// The selected outlines grouped into shapes: each outer outline with the holes directly
/// inside it.
fn regions(rings: &[Vec<Point>]) -> Vec<Region> {
    let depth: Vec<usize> = rings
        .iter()
        .enumerate()
        .map(|(i, r)| {
            rings
                .iter()
                .enumerate()
                .filter(|&(j, o)| j != i && free::inside_ring(o, r[0]))
                .count()
        })
        .collect();
    let mut out = vec![];
    for (i, outer) in rings.iter().enumerate() {
        if depth[i] % 2 == 1 {
            continue;
        }
        let mut members = vec![outer.clone()];
        for (j, hole) in rings.iter().enumerate() {
            if depth[j] == depth[i] + 1 && free::inside_ring(outer, hole[0]) {
                members.push(hole.clone());
            }
        }
        let bounds = members
            .iter()
            .flatten()
            .fold(Rect::new(outer[0], outer[0]), |b, &p| {
                b.union(Rect::new(p, p))
            });
        let offset = bounds.min - Point::splat(MARGIN);
        let size = bounds.size() + Point::splat(2.0 * MARGIN);
        let local = members
            .into_iter()
            .map(|r| r.into_iter().map(|p| p - offset).collect())
            .collect();
        out.push(Region {
            field: Field::new(local, size.x, size.y),
            offset,
            line: None,
        });
    }
    out
}

/// The rings of the band within `half` of the open line `line`, with round ends.
fn band_rings(line: &Path, half: f64) -> Vec<Vec<Point>> {
    cut_craft_core::polygon::stroke(std::slice::from_ref(line), half)
        .iter()
        .map(|b| b.flatten(cut_craft_core::polygon::TOLERANCE))
        .filter(|r| r.len() >= 3)
        .collect()
}

/// The shapes to embellish for the open lines `lines`: with a patterned border, the pattern
/// along each line; otherwise a band the pattern's width round each, filled as a shape.
fn line_regions(lines: &[Path], p: &Params) -> Vec<Region> {
    let mut out = vec![];
    for line in lines {
        match p.pattern() {
            Some(spec) => {
                let points = line.flatten(cut_craft_core::polygon::TOLERANCE);
                if points.len() < 2 {
                    continue;
                }
                let rings = band_rings(line, spec.half_band(p.gap()) + 1.0);
                // the band's outline and any holes in it (where the line crosses itself)
                // as one region
                let Some(mut region) = regions(&rings)
                    .into_iter()
                    .max_by(|a, b| (a.field.w * a.field.h).total_cmp(&(b.field.w * b.field.h)))
                else {
                    continue;
                };
                region.line = Some(points.into_iter().map(|q| q - region.offset).collect());
                out.push(region);
            }
            None => out.extend(
                regions(&band_rings(line, p.border_width.max(0.0) / 2.0))
                    .into_iter()
                    .map(|r| Region {
                        line: Some(vec![]),
                        ..r
                    }),
            ),
        }
    }
    out
}

/// The ornament for one shape, in its field's frame: a patterned border when there is one,
/// then the ornament inside it, kept clear of it. Also how many of the pieces, from the
/// start, are the patterned border's.
fn design(
    field: &Field,
    s: Settings,
    pattern: Option<&border::Spec>,
    rows: bool,
) -> (Vec<Item>, usize) {
    let Some(b) = pattern.filter(|_| s.place != Where::Fill) else {
        return (fill(field, s, &[]), 0);
    };
    let b = fitted(field, b);
    let (mut items, mut reach) = border::design(field, &b, s.gap);
    // more rows inward, each a gap in from the last, while there is room for one
    if rows && !b.single {
        // where rows coming in from different edges meet (round a hole), a row that would
        // run into the rows already laid is left out, and a piece or two that would touch
        // them is dropped (the rows keep a gap apart, so this only catches real overlaps)
        let mut occ = occ::Occ::new(field.w, field.h, s.gap * 0.4, |q, _| field.sample(q) > 0.0);
        occ.mark(&items);
        for _ in 0..200 {
            let next = border::Spec {
                inset: reach + s.gap,
                ..b
            };
            let (loops, r) = border::design_loops(field, &next, s.gap);
            if loops.iter().all(Vec::is_empty) {
                break;
            }
            for row in loops {
                let fits: Vec<bool> = row
                    .iter()
                    .map(|piece| occ.fits(std::slice::from_ref(piece)))
                    .collect();
                let clashes = fits.iter().filter(|f| !**f).count();
                if clashes * 10 > row.len() {
                    continue;
                }
                let kept: Vec<Item> = row
                    .into_iter()
                    .zip(fits)
                    .filter_map(|(piece, fit)| fit.then_some(piece))
                    .collect();
                occ.mark(&kept);
                items.extend(kept);
            }
            reach = r;
        }
    }
    let n = items.len();
    if s.place == Where::Border {
        return (items, n);
    }
    let inside = Settings {
        place: Where::Fill,
        inner_from: reach + s.gap,
        ..s
    };
    let more = fill(field, inside, &items);
    items.extend(more);
    (items, n)
}

/// `b` fitted to a shape too narrow for it as set: one row down the middle of the band,
/// as wide as the band allows, its inset smaller. The pattern's repeat shrinks with its
/// width.
fn fitted(field: &Field, b: &border::Spec) -> border::Spec {
    let room = half_width(field);
    if b.inset + b.width <= room * 0.95 {
        return *b;
    }
    let inset = b.inset.min(room * 0.1);
    // the row's middle just off the band's middle, where the two edge lines nearly meet
    let middle = room * 0.92;
    border::Spec {
        inset,
        width: 2.0 * (middle - inset).max(0.0),
        rails: false,
        single: true,
        ..*b
    }
}

/// How far in from the edge a line still runs most of the way round the shape: half the
/// width of a band, the radius of a disc. (Its widest point alone, a band's corner, says
/// too much.)
fn half_width(field: &Field) -> f64 {
    let longest = field
        .outline
        .iter()
        .map(|r| field::loop_len(r))
        .fold(0.0, f64::max);
    let steps = 60;
    (0..steps)
        .map(|k| field.dmax * f64::from(steps - k) / f64::from(steps))
        .find(|&d| {
            let along: f64 = field
                .rings_longer(d, 0.0)
                .iter()
                .map(|r| field::loop_len(r))
                .sum();
            along >= longest * 0.5
        })
        .unwrap_or(0.0)
}

/// The pattern along one line across the middle of a shape too small for a row round it:
/// the longer way across its box, as wide as its widest point allows.
fn across_the_middle(field: &Field, spec: &border::Spec, g: f64) -> Option<Vec<Item>> {
    let half = field.dmax * 0.5;
    if half <= 0.0 {
        return None;
    }
    let (w, h) = (field.w, field.h);
    let line: Vec<Point> = (0..=200)
        .map(|i| {
            let t = f64::from(i) / 200.0;
            if w >= h {
                Point::new(w * t, h / 2.0)
            } else {
                Point::new(w / 2.0, h * t)
            }
        })
        .filter(|&q| field.sample(q) >= half)
        .collect();
    if line.len() < 2 {
        return None;
    }
    let spec = border::Spec {
        width: 2.0 * half,
        inset: 0.0,
        rails: false,
        ..*spec
    };
    let made = border::along(&line, &spec, g);
    (!made.is_empty()).then_some(made)
}

/// How wide the shapes are, as told in a message: the widest circle in any.
fn wide(regions: &[Region]) -> f64 {
    regions
        .iter()
        .map(|r| 2.0 * r.field.dmax)
        .fold(0.0, f64::max)
}

/// Which of one shape's pieces a Mixed carving hollows in: the small fillers (dots, beads,
/// buds), less than half the size of its big pieces; never the patterned border's first `n`.
fn small_ones(items: &[Item], n: usize) -> Vec<bool> {
    let size = |it: &Item| it.area().abs().sqrt();
    let mut sizes: Vec<f64> = items[n..]
        .iter()
        .filter(|it| matches!(it, Item::Shape { sink: false, .. }))
        .map(size)
        .collect();
    sizes.sort_by(f64::total_cmp);
    // the big pieces' size, past the one outsized band round the edge
    let big = sizes
        .get((sizes.len() as f64 * 0.95) as usize)
        .or(sizes.last())
        .copied()
        .unwrap_or(0.0);
    items
        .iter()
        .enumerate()
        .map(|(k, it)| {
            k >= n && matches!(it, Item::Shape { sink: false, .. }) && size(it) < big * 0.5
        })
        .collect()
}

fn fill(field: &Field, s: Settings, pre: &[Item]) -> Vec<Item> {
    // scrolls or flowers alone spread over the whole shape: in one mirrored wedge, few fit
    if matches!(s.motifs, Motifs::Flowers | Motifs::Scrolls) {
        return free::design(field, s, pre);
    }
    match sym::detect(field) {
        Some(found) => sym::design(field, found, s, pre),
        None => free::design(field, s, pre),
    }
}

/// What Embellish keeps with an embellishment: on its component when it has one, else on its
/// layer.
fn recipe(p: &Params, paths: &[Path], layer: Option<LayerId>) -> serde_json::Value {
    let kept = Params {
        editing: None,
        ..p.clone()
    };
    serde_json::json!({
        "command": Embellish::ID,
        "params": kept,
        "paths": paths,
        "layer": layer,
    })
}

/// One embellishment in the document: its settings, the outlines it was made for and where it
/// lives.
struct Kept {
    params: Params,
    paths: Vec<Path>,
    at: Editing,
}

fn read_recipe(v: &serde_json::Value) -> Option<(Params, Vec<Path>)> {
    if v.get("command")?.as_str()? != Embellish::ID {
        return None;
    }
    Some((
        Params::deserialize(&v["params"]).ok()?,
        Vec::<Path>::deserialize(&v["paths"]).ok()?,
    ))
}

/// Every embellishment Embellish made in `doc`.
fn all_kept(doc: &Document) -> Vec<Kept> {
    let on_components = doc.components.iter().filter_map(|(&id, part)| {
        let shape = part.shape.as_ref()?;
        let (params, paths) = read_recipe(shape)?;
        let layer = Option::<LayerId>::deserialize(&shape["layer"])
            .ok()
            .flatten()
            .filter(|l| doc.layers.iter().any(|x| x.id == *l));
        Some(Kept {
            params,
            paths,
            at: Editing {
                layer,
                component: Some(id),
            },
        })
    });
    let on_layers = doc.layers.iter().filter_map(|layer| {
        let (params, paths) = read_recipe(layer.made.as_ref()?)?;
        Some(Kept {
            params,
            paths,
            at: Editing {
                layer: Some(layer.id),
                component: None,
            },
        })
    });
    on_components.chain(on_layers).collect()
}

/// The embellishment the selection picks: its component, any of its pieces, or (with
/// `by_outlines`) the very outlines it was made for.
fn picked(doc: &Document, by_outlines: bool) -> Option<Kept> {
    let kept = all_kept(doc);
    let by_component = || {
        kept.iter().position(|k| {
            k.at.component
                .is_some_and(|c| doc.selected_components.contains(&c))
        })
    };
    let by_piece = || {
        let mut layers = doc.selection.iter().map(|&id| doc.layer_of(id));
        let first = layers.next()?;
        if !layers.all(|l| l == first) {
            return None;
        }
        kept.iter().position(|k| k.at.layer == Some(first))
    };
    let by_outline = || {
        if !by_outlines {
            return None;
        }
        let (_, chosen) = doc.selected_paths(|_| true);
        if chosen.is_empty() {
            return None;
        }
        kept.iter().position(|k| {
            k.paths.len() == chosen.len() && chosen.iter().all(|q| k.paths.contains(q))
        })
    };
    let i = if doc.selection.is_empty() {
        by_component()?
    } else {
        by_piece().or_else(by_outline)?
    };
    kept.into_iter().nth(i)
}

/// A short run of the chosen pattern with every repeat drawn as the Sequence `entry` says
/// (`2`, `R1/4`…), for the pattern maker's sample buttons: the pieces' outlines, along x
/// and across y, mm. Nothing for a space or a style with no repeats to turn.
pub fn sample(params: &serde_json::Value, entry: &str) -> Vec<Vec<Point>> {
    let Ok(p) = serde_json::from_value::<Params>(params.clone()) else {
        return vec![];
    };
    let Some(mut spec) = p.pattern() else {
        return vec![];
    };
    let width = 20.0;
    spec.width = width;
    spec.rails = false;
    spec.inset = 0.0;
    spec.sequence = seq::Seq::parse(entry);
    // two of the vine's half-waves, or about three of a border's repeats
    let len = match spec.style {
        border::Style::Vine(_) => 2.0 * spec.repeat * width,
        _ => 3.0 * width,
    };
    let line: Vec<Point> = (0..=20)
        .map(|i| Point::new(len * f64::from(i) / 20.0, 0.0))
        .collect();
    border::along(&line, &spec, 0.6)
        .iter()
        .flat_map(|it| match it {
            Item::Shape { rings, .. } => rings.clone(),
            Item::Line { points, .. } => vec![points.clone()],
        })
        .collect()
}

/// A Sequence made up from `seed`: a short run of entries repeated, so it still reads as a
/// pattern.
pub fn random_sequence(seed: u32) -> String {
    seq::random(seed)
}

/// Whether the selection is an embellishment Embellish made, its component or any of its
/// pieces: E opens Embellish on it. (Its outline stays its drawing tool's to edit.)
pub fn edits_selection(doc: &Document) -> bool {
    picked(doc, false).is_some()
}

impl Command for Embellish {
    const ID: &'static str = "embellish";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Embellish",
            group: "Create Vectors",
            icon: "drawing/create_vectors/embellish.png",
            shortcut: "",
        }
    }

    /// On a selected embellishment (its component, a piece of it, or its outline), its own
    /// settings, so the form edits it rather than adding another.
    fn initial_params(doc: &Document) -> Params {
        picked(doc, true).map_or_else(Params::default, |k| Params {
            editing: Some(k.at),
            ..k.params
        })
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if p.make != Make::Vectors {
            if p.direction != Direction::Sunk {
                ensure_positive("raise height", p.height)?;
            }
            if p.direction != Direction::Raised {
                ensure_positive("sink depth", p.depth)?;
            }
        }
        if p.pattern().is_some() {
            ensure_positive("pattern width", p.border_width)?;
        }
        let selected = doc.selected_paths(|_| true).1;
        let editing = p.editing.and_then(|at| {
            all_kept(doc).into_iter().find(|k| {
                k.at == at
                    || (at.component.is_some() && k.at.component == at.component)
                    || (at.component.is_none() && at.layer.is_some() && k.at.layer == at.layer)
            })
        });
        let shapes = match &editing {
            Some(k) => k.paths.clone(),
            None => selected,
        };
        if shapes.is_empty() {
            return Err(CommandError::Failed(
                "select a vector to embellish: a closed shape gets the ornament designed to \
                 fit it, an open line gets it along the line"
                    .into(),
            ));
        }
        let rings: Vec<Vec<Point>> = shapes
            .iter()
            .filter(|s| s.closed)
            .map(|s| s.flatten(cut_craft_core::polygon::TOLERANCE))
            .filter(|r| r.len() >= 3)
            .collect();
        let lines: Vec<Path> = shapes.iter().filter(|s| !s.closed).cloned().collect();
        let mut regions = regions(&rings);
        regions.extend(line_regions(&lines, p));
        let mut items: Vec<Item> = vec![];
        let mut sunk: Vec<bool> = vec![];
        // the smallest gap any shape needed
        let mut gap = p.gap();
        for (k, region) in regions.iter().enumerate() {
            let seed = p.seed.wrapping_add(k as u32 * 7919);
            let offset = region.offset;
            // a shape too narrow for the style at the gap asked is still drawn, the gap
            // halved until it fits: what was asked for is made, too fine to carve in full
            // (the note says so)
            let mut g = p.gap();
            let (made, _n) = loop {
                let (made, n) = match (&region.line, p.pattern()) {
                    (Some(line), Some(spec)) => {
                        let made = border::along(line, &spec, g);
                        let n = made.len();
                        (made, n)
                    }
                    // a band round an open line: the folk mix runs along it, with no
                    // border round the band's own edge eating its room
                    (Some(_), None) if p.style == Style::FolkMix => {
                        let s = Settings {
                            place: Where::Fill,
                            ..p.settings(seed, g)
                        };
                        design(&region.field, s, None, false)
                    }
                    _ => design(
                        &region.field,
                        p.settings(seed, g),
                        p.pattern().as_ref(),
                        true,
                    ),
                };
                // a border alone, its pieces or ornament missing, is not what was asked;
                // nor are pieces too big for the shape, out over its edge
                let raised = made
                    .iter()
                    .filter(|it| matches!(it, Item::Shape { sink: false, .. }))
                    .count();
                let enough = if p.style.layout().0 == Where::Border {
                    raised > 0
                } else {
                    raised > 2
                };
                // (the patterned border's own rows, the first `n`, are laid inside it)
                let inside = made[n..]
                    .iter()
                    .flat_map(Item::points)
                    .all(|q| region.field.sample(q) > region.field.res * 0.5);
                // too small for even one row round it: one row across its middle
                if raised == 0
                    && g < 0.05
                    && let Some(spec) = p.pattern()
                    && let Some(made) = across_the_middle(&region.field, &spec, g)
                {
                    let n = made.len();
                    break (made, n);
                }
                if (enough && inside) || g < 0.05 {
                    break (made, n);
                }
                g /= 2.0;
            };
            gap = gap.min(g);
            sunk.extend(match p.direction {
                Direction::Raised => vec![false; made.len()],
                Direction::Sunk => vec![true; made.len()],
                Direction::Mixed => small_ones(&made, 0),
            });
            items.extend(made.iter().map(|it| it.map(&|q| q + offset)));
        }
        if !items
            .iter()
            .any(|it| matches!(it, Item::Shape { sink: false, .. }))
        {
            let wide = wide(&regions);
            return Err(CommandError::Failed(format!(
                "the shape is only about {wide:.2} mm wide: too small to draw this style in. \
                 Pick a bigger shape"
            )));
        }
        let at = editing.as_ref().map_or(
            Editing {
                layer: None,
                component: None,
            },
            |k| k.at,
        );
        let mut c = Change::builder(doc);
        // an embellishment edited: its old pieces go, and its layer if it no longer has any
        if let Some(old) = at.layer {
            let pieces: Vec<EntityId> = doc
                .vectors
                .keys()
                .copied()
                .filter(|&id| doc.layer_of(id) == old)
                .collect();
            for id in pieces {
                c.remove(id);
            }
            if p.make == Make::Carving {
                c.remove_layer(old);
            }
        }
        let layer = if p.make == Make::Carving {
            None
        } else {
            let id = match at.layer {
                Some(old) => old,
                None => c.add_layer(&p.name),
            };
            let mut layer = c
                .layers()
                .iter()
                .find(|l| l.id == id)
                .cloned()
                .expect("the layer was just found or added");
            layer.name.clone_from(&p.name);
            layer.made = (p.make == Make::Vectors).then(|| recipe(p, &shapes, None));
            c.replace_layer(layer);
            let group = doc.next_group();
            for it in &items {
                for path in outlines(it) {
                    let piece = c.add(path);
                    c.move_to_layer(piece, id);
                    c.set_group(piece, Some(group));
                }
            }
            Some(id)
        };
        let old_part = at.component.filter(|id| doc.components.contains_key(id));
        if p.make == Make::Vectors {
            if let Some(id) = old_part {
                c.remove_component(id);
            }
        } else {
            let mut part = carving(p, &regions, &items, &sunk, doc.job.model_cell(), gap);
            part.shape = Some(recipe(p, &shapes, layer));
            match old_part {
                Some(id) => {
                    let old = &doc.components[&id];
                    part.level = old.level;
                    part.group.clone_from(&old.group);
                    part.visible = old.visible;
                    part.colour = old.colour;
                    part.shaping = old.shaping.clone();
                    c.replace_component(id, part);
                }
                None => {
                    c.add_component(part);
                }
            }
        }
        let mut change = c.finish();
        if gap < p.gap() - 1e-9 {
            let wide = wide(&regions);
            change.notes.push(format!(
                "Drawn smaller than it will carve: the shape is about {wide:.1} mm wide, so \
                 the pieces sit {gap:.2} mm apart, not {asked:.2} mm. A bit wider than \
                 {gap:.2} mm will lose the fine detail",
                asked = p.gap(),
            ));
        }
        Ok(change)
    }
}

/// A piece's vectors: its outline rings, or its line; the ground's stipple has none.
fn outlines(it: &Item) -> Vec<Path> {
    match it {
        Item::Shape { sink: true, .. } => vec![],
        Item::Shape { rings, .. } => rings
            .iter()
            .filter(|r| r.len() >= 3)
            .map(|r| fit(r, true, Fit::Beziers, 0.05, 50_f64.to_radians()))
            .collect(),
        Item::Line { points, .. } => {
            vec![fit(points, false, Fit::Beziers, 0.05, 50_f64.to_radians())]
        }
    }
}

/// The pieces raised, or hollowed where `sunk`, as one component over the shapes.
fn carving(
    p: &Params,
    regions: &[Region],
    items: &[Item],
    sunk: &[bool],
    model_cell: f64,
    gap: f64,
) -> Component {
    let area = regions
        .iter()
        .map(|r| {
            Rect::new(
                r.offset + Point::splat(MARGIN),
                r.offset + Point::new(r.field.w, r.field.h) - Point::splat(MARGIN),
            )
        })
        .reduce(Rect::union)
        .expect("at least one shape");
    let size = area.size();
    let cell = (gap / 4.0)
        .min(model_cell)
        .max((size.x * size.y / MOST_CELLS).sqrt());
    let mut part = Component::from_fn(&p.name, p.combine, area, cell, |_| None);
    let mut grid = relief::grid(area.min, cell, part.columns, part.rows, |q| {
        regions.iter().any(|r| r.field.sample(q - r.offset) > 0.0)
    });
    relief::raise(&mut grid, items, sunk, p.height, p.depth);
    part.heights = grid.heights;
    part
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_samples_show_each_way_round_and_turned_ones_stay_in_the_band() {
        let params = |style: Style| {
            serde_json::to_value(Params {
                style,
                ..Params::default()
            })
            .unwrap()
        };
        let spread = |rings: &[Vec<Point>]| {
            rings
                .iter()
                .flatten()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), q| {
                    (lo.min(q.y), hi.max(q.y))
                })
        };
        for style in [Style::Chevrons, Style::FolkMix, Style::Feathers] {
            let p = params(style);
            // a space is empty (the vine runs straight through it)
            let (lo, hi) = spread(&sample(&p, "0"));
            assert!(hi - lo < 4.0 || lo > hi, "{style:?}: a space is empty");
            let one = sample(&p, "1");
            let two = sample(&p, "2");
            assert!(!one.is_empty() && !two.is_empty(), "{style:?}");
            assert_ne!(one, two, "{style:?}: 2 differs from 1");
            let (lo, hi) = spread(&sample(&p, "1/4"));
            assert!(
                lo >= -10.0 - 1e-6 && hi <= 10.0 + 1e-6,
                "{style:?}: {lo} {hi}"
            );
        }
        assert!(
            !sample(&params(Style::Twist), "1").is_empty(),
            "a plait draws whole"
        );
        assert!(matches!(
            sample(&serde_json::json!("nonsense"), "1").as_slice(),
            []
        ));
    }

    use super::*;
    use cut_craft_core::Change;
    use cut_craft_core::testing::{document_with, square};
    use std::f64::consts::TAU;

    fn framed() -> Document {
        document_with(vec![square(0.0, 0.0, 200.0)])
    }

    cut_craft_core::contract_tests!(Embellish, framed);

    fn polygon(n: u32, r: f64) -> Path {
        let pts: Vec<Point> = (0..n)
            .map(|k| {
                Point::new(150.0, 150.0) + Point::from_angle(TAU * f64::from(k) / f64::from(n)) * r
            })
            .collect();
        Path::polyline(&pts, true)
    }

    fn lopsided() -> Path {
        let p = Point::new;
        Path::polyline(
            &[
                p(0.0, 0.0),
                p(240.0, 20.0),
                p(200.0, 120.0),
                p(120.0, 200.0),
                p(60.0, 90.0),
            ],
            true,
        )
    }

    fn made(doc: &Document, p: &Params) -> Document {
        let mut after = doc.clone();
        after.apply(&Embellish::run(doc, p).unwrap()).unwrap();
        after
    }

    fn selecting_components(doc: &Document) -> Document {
        let mut d = doc.clone();
        d.selection = cut_craft_core::selection::Selection::default();
        d.selected_components = d.components.keys().copied().collect();
        d
    }

    fn layer_vectors(doc: &Document, layer: cut_craft_core::document::LayerId) -> usize {
        doc.vectors
            .keys()
            .filter(|id| doc.layer_of(**id) == layer)
            .count()
    }

    #[test]
    fn a_selected_embellishment_opens_with_its_own_settings_and_is_remade_in_place() {
        let doc = document_with(vec![polygon(6, 110.0)]);
        let p = Params {
            name: "Lid".into(),
            style: Style::Scrolls,
            objects: 3,
            ..Params::default()
        };
        let once = selecting_components(&made(&doc, &p));
        assert_eq!(
            Embellish::initial_params(&doc).editing,
            None,
            "nothing to edit yet"
        );
        let opened = Embellish::initial_params(&once);
        assert!(
            opened.editing.is_some(),
            "the component opens Embellish on itself"
        );
        assert_eq!(opened.style, Style::Scrolls);
        assert_eq!(opened.objects, 3);
        assert_eq!(opened.name, "Lid");
        let changed = Params {
            name: "Top".into(),
            objects: 9,
            ..opened
        };
        let twice = made(&once, &changed);
        assert_eq!(twice.layers.len(), once.layers.len(), "the same layer");
        assert_eq!(twice.components.len(), 1, "remade, not added");
        let layer = twice.layers.last().unwrap();
        assert_eq!(layer.name, "Top");
        let fresh = made(
            &doc,
            &Params {
                editing: None,
                ..changed.clone()
            },
        );
        assert_eq!(
            layer_vectors(&twice, layer.id),
            layer_vectors(&fresh, fresh.layers.last().unwrap().id),
            "new pieces replace the old"
        );
        assert_eq!(twice.components.values().next().unwrap().name, "Top");
        assert_eq!(
            Embellish::initial_params(&selecting_components(&twice)).objects,
            9,
            "and it remembers the new settings"
        );
        // undo brings back the first one exactly
        let mut undone = twice.clone();
        undone
            .apply(&Embellish::run(&once, &changed).unwrap().inverse())
            .unwrap();
        assert_eq!(undone.vectors.len(), once.vectors.len());
        assert_eq!(undone.layers, once.layers);
    }

    #[test]
    fn its_pieces_or_the_outline_it_was_made_for_open_it_too() {
        let shape = polygon(6, 110.0);
        let doc = document_with(vec![shape.clone()]);
        let p = Params {
            objects: 2,
            ..Params::default()
        };
        let after = made(&doc, &p);
        assert!(
            Embellish::initial_params(&after).editing.is_some(),
            "the outline is still selected: Embellish edits its embellishment"
        );
        let layer = after.layers.last().unwrap().id;
        let piece = *after
            .vectors
            .keys()
            .find(|id| after.layer_of(**id) == layer)
            .unwrap();
        let mut picked = after.clone();
        picked.selection = [piece].into();
        let opened = Embellish::initial_params(&picked);
        assert!(opened.editing.is_some(), "a piece of it");
        assert_eq!(opened.objects, 2);
        // another outline is a new embellishment
        let mut other = after.clone();
        other
            .apply(&Change::swap(&other, [], vec![square(400.0, 0.0, 100.0)]))
            .unwrap();
        assert_eq!(Embellish::initial_params(&other).editing, None);
    }

    #[test]
    fn vectors_only_keeps_its_settings_on_its_layer_and_can_become_a_carving() {
        let doc = document_with(vec![polygon(6, 110.0)]);
        let p = Params {
            make: Make::Vectors,
            ..Params::default()
        };
        let once = made(&doc, &p);
        assert!(once.components.is_empty());
        let opened = Embellish::initial_params(&once);
        assert!(opened.editing.is_some());
        let both = made(
            &once,
            &Params {
                make: Make::Both,
                ..opened
            },
        );
        assert_eq!(both.components.len(), 1, "a carving added to it");
        assert_eq!(both.layers.len(), once.layers.len());
        assert!(
            both.layers.last().unwrap().made.is_none(),
            "the component keeps the settings now"
        );
        let opened = Embellish::initial_params(&selecting_components(&both));
        let carving = made(
            &both,
            &Params {
                make: Make::Carving,
                ..opened
            },
        );
        assert_eq!(
            carving.layers.len(),
            doc.layers.len(),
            "its vectors and layer go"
        );
        assert_eq!(carving.vectors.len(), doc.vectors.len());
        assert_eq!(carving.components.len(), 1);
        let opened = Embellish::initial_params(&selecting_components(&carving));
        assert_eq!(opened.make, Make::Carving);
        let back = made(
            &carving,
            &Params {
                make: Make::Vectors,
                ..opened
            },
        );
        assert!(back.components.is_empty(), "the carving goes");
        assert_eq!(back.layers.len(), doc.layers.len() + 1);
        assert!(Embellish::initial_params(&back).editing.is_some());
    }

    #[test]
    fn each_embellishment_goes_on_a_layer_of_its_own_named_for_it() {
        let shape = polygon(6, 110.0);
        let doc = document_with(vec![shape.clone()]);
        let p = Params {
            name: "Lid".into(),
            ..Params::default()
        };
        let once = made(&doc, &p);
        let twice = made(&once, &p);
        assert_eq!(
            twice.layers.len(),
            doc.layers.len() + 2,
            "a new layer each time"
        );
        assert_eq!(
            twice.current_layer, doc.current_layer,
            "drawing stays where it was"
        );
        for after in [&once, &twice] {
            let layer = after.layers.last().unwrap();
            assert_eq!(layer.name, "Lid");
            let on: Vec<_> = after
                .vectors
                .iter()
                .filter(|(id, _)| after.layer_of(**id) == layer.id)
                .collect();
            assert!(on.len() > 40, "{} pieces on it", on.len());
            assert!(
                on.iter().all(|(_, v)| **v != shape),
                "the outline stays put"
            );
        }
    }

    fn range_of(doc: &Document) -> (f64, f64) {
        let part = doc.components.values().next().unwrap();
        part.heights
            .iter()
            .filter(|h| !h.is_nan())
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &h| {
                (lo.min(h), hi.max(h))
            })
    }

    #[test]
    fn the_pieces_stand_up_sink_in_or_both_by_their_own_distances() {
        let doc = document_with(vec![polygon(6, 110.0)]);
        let with = |direction| Params {
            direction,
            height: 3.0,
            depth: 1.5,
            ..Params::default()
        };
        let (lo, hi) = range_of(&made(&doc, &with(Direction::Raised)));
        assert!((hi - 3.0).abs() < 0.15 && lo > -0.5, "raised {lo} {hi}");
        let (lo, hi) = range_of(&made(&doc, &with(Direction::Sunk)));
        assert!(hi <= 0.0 && (lo + 1.5).abs() < 0.3, "sunk {lo} {hi}");
        let (lo, hi) = range_of(&made(&doc, &with(Direction::Mixed)));
        assert!((hi - 3.0).abs() < 0.15 && lo < -1.0, "mixed {lo} {hi}");
        for style in [Style::FolkMix, Style::Scrolls] {
            let p = Params {
                style,
                sequence: "12".into(),
                ..with(Direction::Mixed)
            };
            let (pieces, n) = {
                let rings: Vec<Vec<Point>> = vec![polygon(6, 110.0).flatten(0.05)];
                let r = regions(&rings);
                design(
                    &r[0].field,
                    p.settings(4, p.gap()),
                    p.pattern().as_ref(),
                    false,
                )
            };
            let _ = n;
            let small = small_ones(&pieces, 0);
            let raised = small
                .iter()
                .zip(&pieces)
                .filter(|(s, it)| !**s && matches!(it, Item::Shape { sink: false, .. }));
            assert!(raised.count() >= 12, "the main pieces stay raised");
            assert!(small.iter().filter(|s| **s).count() >= 12, "fillers sink");
        }
    }

    #[test]
    fn a_direction_needs_its_own_distance() {
        let doc = document_with(vec![polygon(6, 110.0)]);
        let sunk = Params {
            direction: Direction::Sunk,
            height: 0.0,
            ..Params::default()
        };
        assert!(Embellish::run(&doc, &sunk).is_ok(), "no height wanted");
        let flat = Params { depth: 0.0, ..sunk };
        assert!(Embellish::run(&doc, &flat).is_err());
    }

    #[test]
    fn makes_grouped_vectors_and_one_carving_inside_the_shape() {
        for shape in [polygon(6, 110.0), lopsided()] {
            let doc = document_with(vec![shape.clone()]);
            let after = made(&doc, &Params::default());
            let new: Vec<&Path> = after.vectors.values().filter(|v| **v != shape).collect();
            assert!(new.len() > 20, "{} pieces", new.len());
            let bounds = shape.bounds();
            for v in &new {
                let b = v.bounds();
                assert!(
                    b.min.x >= bounds.min.x - 0.5 && b.max.x <= bounds.max.x + 0.5,
                    "{b:?}"
                );
            }
            assert_eq!(after.components.len(), 1);
            let part = after.components.values().next().unwrap();
            let top = part
                .heights
                .iter()
                .copied()
                .filter(|h| !h.is_nan())
                .fold(0.0, f64::max);
            assert!((top - 3.0).abs() < 0.3, "highest {top}");
        }
    }

    #[test]
    fn a_seed_makes_the_same_design_and_another_seed_another() {
        let doc = document_with(vec![lopsided()]);
        let p = Params {
            make: Make::Vectors,
            ..Params::default()
        };
        let shapes = |d: &Document| {
            let mut v: Vec<String> = d
                .vectors
                .values()
                .map(|v| format!("{:?}", v.bounds()))
                .collect();
            v.sort();
            v
        };
        assert_eq!(shapes(&made(&doc, &p)), shapes(&made(&doc, &p)));
        // another number, another shape of piece (each repeat the same, see the vine's tests)
        let other = Params {
            seed: 5,
            ..p.clone()
        };
        assert_ne!(shapes(&made(&doc, &p)), shapes(&made(&doc, &other)));
        let random = |seed| Params {
            seed,
            random_size: true,
            ..p.clone()
        };
        assert_eq!(
            shapes(&made(&doc, &random(4))),
            shapes(&made(&doc, &random(4)))
        );
        assert_ne!(
            shapes(&made(&doc, &random(4))),
            shapes(&made(&doc, &random(5)))
        );
    }

    #[test]
    fn unticked_pieces_are_left_out() {
        let doc = document_with(vec![lopsided()]);
        let count = |dots, leaves| {
            let p = Params {
                dots,
                leaves,
                make: Make::Vectors,
                ..Params::default()
            };
            made(&doc, &p).vectors.len()
        };
        assert!(count(true, true) > count(false, true));
        assert!(count(false, true) > count(false, false));
    }

    #[test]
    fn the_ornament_reaches_the_middle() {
        let shape = polygon(4, 110.0);
        let doc = document_with(vec![shape]);
        let centre = Point::new(150.0, 150.0);
        let near_middle = |style| {
            let p = Params {
                style,
                make: Make::Vectors,
                ..Params::default()
            };
            made(&doc, &p)
                .vectors
                .values()
                .filter(|v| {
                    let b = v.bounds();
                    b.size().x < 60.0 && b.center().distance(centre) < 25.0
                })
                .count()
        };
        assert!(near_middle(Style::FolkMix) > 0);
    }

    #[test]
    fn every_patterned_style_fills_a_closed_shape_row_after_row() {
        let shape = polygon(6, 110.0);
        let doc = document_with(vec![shape.clone()]);
        let centre = Point::new(150.0, 150.0);
        for style in [
            Style::BandBorder,
            Style::Rope,
            Style::Chain,
            Style::BeadAndReel,
            Style::Laurel,
            Style::Feathers,
            Style::Chevrons,
        ] {
            let p = Params {
                style,
                make: Make::Vectors,
                ..Params::default()
            };
            let rows = made(&doc, &p);
            let near_middle = rows
                .vectors
                .values()
                .filter(|v| v.bounds().center().distance(centre) < 40.0)
                .count();
            assert!(near_middle > 0, "{style:?}: rows reach the middle");
        }
    }

    #[test]
    fn no_style_is_named_a_border() {
        let schema = serde_json::to_string(&schemars::schema_for!(Params)).unwrap();
        assert!(
            !schema.contains(" border\""),
            "a style title ends in border"
        );
        assert!(!schema.contains("Fill the shape"));
    }

    /// A band `w` mm wide round a 200 mm square.
    fn band(w: f64) -> Document {
        document_with(vec![square(0.0, 0.0, 200.0), square(w, w, 200.0 - 2.0 * w)])
    }

    #[test]
    fn a_border_too_wide_for_a_narrow_band_is_made_narrower_to_fit() {
        let doc = band(12.0);
        for style in [Style::Rope, Style::Chain, Style::Chevrons, Style::Laurel] {
            let p = Params {
                style,
                border_width: 30.0,
                make: Make::Vectors,
                ..Params::default()
            };
            let after = match Embellish::run(&doc, &p) {
                Ok(c) => {
                    let mut a = doc.clone();
                    a.apply(&c).unwrap();
                    a
                }
                Err(e) => panic!("{style:?}: {e}"),
            };
            let pieces = after.vectors.len() - doc.vectors.len();
            assert!(pieces > 40, "{style:?}: {pieces}");
            // every piece stays inside the band
            for v in after.vectors.values() {
                let b = v.bounds();
                let inner = Rect::new(Point::splat(12.0), Point::splat(188.0));
                let across = b.min.x.max(b.min.y) >= -0.01 && b.max.x.max(b.max.y) <= 200.01;
                let clear = !(b.min.x > inner.min.x + 0.01
                    && b.min.y > inner.min.y + 0.01
                    && b.max.x < inner.max.x - 0.01
                    && b.max.y < inner.max.y - 0.01);
                assert!(across && clear, "{style:?}: {b:?}");
            }
        }
    }

    #[test]
    fn too_narrow_a_shape_is_still_drawn_with_a_note_that_it_is_too_fine_to_carve() {
        let doc = band(4.0);
        for style in [Style::Rope, Style::Chain, Style::FolkMix, Style::Laurel] {
            let p = Params {
                style,
                ..Params::default()
            };
            let change = Embellish::run(&doc, &p).unwrap_or_else(|e| panic!("{style:?}: {e}"));
            let mut after = doc.clone();
            after.apply(&change).unwrap();
            assert!(
                after.vectors.len() > doc.vectors.len() + 10,
                "{style:?}: {}",
                after.vectors.len() - doc.vectors.len()
            );
            let note = change.notes.join(" ");
            assert!(
                note.contains("smaller than it will carve"),
                "{style:?}: {note}"
            );
        }
        // a shape it fits makes no such note
        let fits = Embellish::run(&band(40.0), &Params::default()).unwrap();
        assert!(fits.notes.is_empty(), "{:?}", fits.notes);
    }

    #[test]
    fn a_tiny_shapes_pieces_are_made_small_enough_to_stay_inside_it() {
        let doc = document_with(vec![square(10.0, 10.0, 3.0)]);
        let change = Embellish::run(&doc, &Params::default()).unwrap();
        let mut after = doc.clone();
        after.apply(&change).unwrap();
        // (to the field's half cell: 0.25 mm)
        let inside = Rect::new(Point::splat(9.75), Point::splat(13.25));
        for v in after.vectors.values() {
            for q in v.flatten(0.01) {
                assert!(inside.contains(q), "{q:?}");
            }
        }
        assert!(
            change
                .notes
                .join(" ")
                .contains("smaller than it will carve")
        );
    }

    #[test]
    fn tightness_sets_the_gap() {
        let p = |tightness| Params {
            tightness,
            ..Params::default()
        };
        assert_eq!(p(1).gap(), 3.0);
        assert_eq!(p(10).gap(), 0.75);
    }

    #[test]
    fn needs_a_vector() {
        let err = Embellish::run(&Document::default(), &Params::default()).unwrap_err();
        assert!(
            err.to_string().to_lowercase().contains("select a vector"),
            "{err}"
        );
    }

    /// An open S-curve from (20, 100) to (320, 100).
    fn wave() -> Path {
        let pts: Vec<Point> = (0..=150)
            .map(|i| {
                let x = f64::from(i) * 2.0;
                Point::new(20.0 + x, 100.0 + 30.0 * (x / 300.0 * TAU).sin())
            })
            .collect();
        Path::polyline(&pts, false)
    }

    fn pieces_of(after: &Document, line: &Path) -> Vec<Path> {
        after
            .vectors
            .values()
            .filter(|v| *v != line)
            .cloned()
            .collect()
    }

    #[test]
    fn every_patterned_style_runs_along_an_open_line_from_end_to_end() {
        let line = wave();
        let flat = line.flatten(0.05);
        let doc = document_with(vec![line.clone()]);
        for style in [
            Style::FolkMix,
            Style::Scrolls,
            Style::Flowers,
            Style::BandBorder,
            Style::Rope,
            Style::Chain,
            Style::Twist,
            Style::Braid,
            Style::BeadAndReel,
            Style::Laurel,
            Style::Feathers,
            Style::Chevrons,
        ] {
            let p = Params {
                style,
                make: Make::Vectors,
                ..Params::default()
            };
            let after = made(&doc, &p);
            let pieces = pieces_of(&after, &line);
            assert!(pieces.len() > 8, "{style:?}: {}", pieces.len());
            let (mut first, mut last) = (f64::INFINITY, f64::INFINITY);
            for v in &pieces {
                for q in v.flatten(0.1) {
                    let d = cut_craft_core::geometry::polyline_distance(&flat, q);
                    assert!(
                        d < p.border_width / 2.0 + 0.6,
                        "{style:?}: {d} off the line"
                    );
                    first = first.min(q.distance(flat[0]));
                    last = last.min(q.distance(*flat.last().unwrap()));
                }
            }
            assert!(
                first < 20.0 && last < 20.0,
                "{style:?}: ends {first} {last}"
            );
        }
    }

    #[test]
    fn an_open_lines_carving_lies_along_it_and_selecting_the_line_reopens_it() {
        let line = wave();
        let doc = document_with(vec![line.clone()]);
        let p = Params {
            style: Style::Rope,
            objects: 3,
            ..Params::default()
        };
        let after = made(&doc, &p);
        assert_eq!(after.components.len(), 1);
        let part = after.components.values().next().unwrap();
        let top = part
            .heights
            .iter()
            .copied()
            .filter(|h| !h.is_nan())
            .fold(0.0, f64::max);
        assert!((top - p.height).abs() < 0.4, "highest {top}");
        // the line is still selected: Embellish opens on its embellishment
        let opened = Embellish::initial_params(&after);
        assert!(opened.editing.is_some());
        assert_eq!(opened.style, Style::Rope);
        let again = made(
            &after,
            &Params {
                style: Style::Chain,
                ..opened
            },
        );
        assert_eq!(again.components.len(), 1, "remade, not added");
        assert_eq!(again.layers.len(), after.layers.len());
    }

    #[test]
    fn a_closed_shape_and_an_open_line_are_embellished_together() {
        let line = wave();
        let shape = square(400.0, 0.0, 200.0);
        let doc = document_with(vec![line.clone(), shape.clone()]);
        let p = Params {
            style: Style::Laurel,
            make: Make::Vectors,
            ..Params::default()
        };
        let after = made(&doc, &p);
        let pieces = pieces_of(&after, &line);
        let on_line = pieces.iter().filter(|v| v.bounds().max.x < 360.0).count();
        let in_square = pieces.iter().filter(|v| v.bounds().min.x > 395.0).count();
        assert!(on_line > 8 && in_square > 8, "{on_line} {in_square}");
    }
}

/// Pictures of the carving for a look by eye: `EMBELLISH_OUT=dir cargo nextest run -p
/// cmd-embellish pictures --run-ignored only` (and `EMBELLISH_SHAPE`, a JSON list of
/// `[x, y]`, for a drawn shape).
#[cfg(test)]
mod pictures {
    use super::*;
    use std::f64::consts::{PI, TAU};

    fn shade(part: &Component, path: &std::path::Path) {
        let (w, h) = (part.columns, part.rows);
        let at = |i: usize, j: usize| {
            let v = part.heights[j.min(h - 1) * w + i.min(w - 1)];
            if v.is_nan() { 1.0 } else { v }
        };
        let (el, az) = (24_f64.to_radians(), (-135_f64).to_radians() + PI / 2.0);
        let l = [el.cos() * az.cos(), el.cos() * az.sin(), el.sin()];
        let mut img = image::RgbImage::new(w as u32, h as u32);
        for j in 0..h {
            for i in 0..w {
                let dx = (at(i + 1, j) - at(i.saturating_sub(1), j)) / (2.0 * part.cell);
                let dy = (at(i, j + 1) - at(i, j.saturating_sub(1))) / (2.0 * part.cell);
                let n = [-dx, -dy, 1.0];
                let len = (n[0] * n[0] + n[1] * n[1] + 1.0).sqrt();
                let lam = ((n[0] * l[0] + n[1] * l[1] + n[2] * l[2]) / len).max(0.0);
                let k = 0.34 + 1.1 * lam;
                let px = image::Rgb([
                    (232.0 * k).min(255.0) as u8,
                    (190.0 * k).min(255.0) as u8,
                    (138.0 * k).min(255.0) as u8,
                ]);
                img.put_pixel(i as u32, (h - 1 - j) as u32, px);
            }
        }
        img.save(path).unwrap();
    }

    fn polygon(n: u32, r: f64, catenary: bool) -> Path {
        let c = Point::new(150.0, 150.0);
        let rot = if n.is_multiple_of(2) {
            PI / f64::from(n)
        } else {
            0.0
        } + PI / 2.0;
        let mut pts = vec![];
        for k in 0..n {
            let a0 = rot + TAU * f64::from(k) / f64::from(n);
            let (p0, p1) = (
                c + Point::from_angle(a0) * r,
                c + Point::from_angle(a0 + TAU / f64::from(n)) * r,
            );
            for s in 0..40 {
                let t = f64::from(s) / 40.0;
                let q = p0.lerp(p1, t);
                let sag = if catenary {
                    let half = p0.distance(p1) / 2.0;
                    let x = (t - 0.5) * 2.0 * half;
                    let cc = half * 0.55;
                    let full = (half * 0.3).min(r * (PI / f64::from(n)).cos() * 0.26);
                    full * (1.0
                        - (((x / cc).cosh() - 1.0) / ((half / cc).cosh() - 1.0)).clamp(0.0, 1.0))
                } else {
                    0.0
                };
                pts.push(q + (c - q).normalize() * sag);
            }
        }
        Path::polyline(&pts, true)
    }

    /// The Style list's pictures: each style's vectors on a rounded square, as JSON lists of
    /// points for `scripts/make_embellish_icons.py`: `EMBELLISH_ICONS=dir cargo nextest run
    /// -p cmd-embellish style_pictures --run-ignored only`.
    #[test]
    #[ignore = "writes the style pictures' outlines"]
    fn style_pictures() {
        let out = std::path::PathBuf::from(std::env::var("EMBELLISH_ICONS").unwrap());
        // a 100 mm square with 16 mm round corners
        let mut pts = vec![];
        for (cx, cy, a0) in [
            (84.0, 16.0, -90.0),
            (84.0, 84.0, 0.0),
            (16.0, 84.0, 90.0),
            (16.0, 16.0, 180.0),
        ] {
            for k in 0..=12 {
                let a = f64::to_radians(a0 + 90.0 * f64::from(k) / 12.0);
                pts.push(Point::new(cx, cy) + Point::from_angle(a) * 16.0);
            }
        }
        let shape = Path::polyline(&pts, true);
        let doc = cut_craft_core::testing::document_with(vec![shape.clone()]);
        for (key, style) in [
            ("folk_mix", Style::FolkMix),
            ("scrolls", Style::Scrolls),
            ("flowers", Style::Flowers),
            ("band_border", Style::BandBorder),
            ("rope", Style::Rope),
            ("chain", Style::Chain),
            ("twist", Style::Twist),
            ("braid", Style::Braid),
            ("bead_and_reel", Style::BeadAndReel),
            ("laurel", Style::Laurel),
            ("feathers", Style::Feathers),
            ("chevrons", Style::Chevrons),
        ] {
            let p = Params {
                style,
                make: Make::Vectors,
                border_width: 16.0,
                tightness: 5,
                objects: 9,
                ..Params::default()
            };
            let mut after = doc.clone();
            after.apply(&Embellish::run(&doc, &p).unwrap()).unwrap();
            let rings: Vec<Vec<[f64; 2]>> = after
                .vectors
                .values()
                .map(|v| v.flatten(0.1).iter().map(|q| [q.x, q.y]).collect())
                .collect();
            std::fs::write(
                out.join(format!("{key}.json")),
                serde_json::to_string(&rings).unwrap(),
            )
            .unwrap();
        }
    }

    /// Every style along an open S-curve, as 3D carvings: `EMBELLISH_OUT=dir cargo nextest
    /// run -p cmd-embellish line_pictures --run-ignored only`.
    #[test]
    #[ignore = "writes pictures to look at"]
    fn line_pictures() {
        let out = std::path::PathBuf::from(std::env::var("EMBELLISH_OUT").unwrap());
        let pts: Vec<Point> = (0..=150)
            .map(|i| {
                let x = f64::from(i) * 2.0;
                Point::new(20.0 + x, 100.0 + 40.0 * (x / 300.0 * TAU).sin())
            })
            .collect();
        let line = Path::polyline(&pts, false);
        let doc = cut_craft_core::testing::document_with(vec![line]);
        for (key, style) in [
            ("folk_mix", Style::FolkMix),
            ("scrolls", Style::Scrolls),
            ("flowers", Style::Flowers),
            ("band_border", Style::BandBorder),
            ("rope", Style::Rope),
            ("chain", Style::Chain),
            ("twist", Style::Twist),
            ("braid", Style::Braid),
            ("bead_and_reel", Style::BeadAndReel),
            ("laurel", Style::Laurel),
            ("feathers", Style::Feathers),
            ("chevrons", Style::Chevrons),
        ] {
            let p = Params {
                style,
                make: Make::Carving,
                border_width: 30.0,
                ..Params::default()
            };
            let mut after = doc.clone();
            after.apply(&Embellish::run(&doc, &p).unwrap()).unwrap();
            let part = after.components.values().next().unwrap();
            shade(part, &out.join(format!("line-{key}.png")));
            let shape = cut_craft_core::testing::square(0.0, 0.0, 160.0);
            let boxed = cut_craft_core::testing::document_with(vec![shape]);
            let mut after = boxed.clone();
            after.apply(&Embellish::run(&boxed, &p).unwrap()).unwrap();
            let part = after.components.values().next().unwrap();
            shade(part, &out.join(format!("square-{key}.png")));
        }
    }

    #[test]
    #[ignore = "writes pictures to look at"]
    fn sequence_pictures() {
        let out = std::path::PathBuf::from(std::env::var("EMBELLISH_OUT").unwrap());
        let pts: Vec<Point> = (0..=100)
            .map(|i| Point::new(10.0 + f64::from(i) * 3.0, 50.0))
            .collect();
        let doc = cut_craft_core::testing::document_with(vec![Path::polyline(&pts, false)]);
        for (key, style, sequence) in [
            ("chevrons-111000222", Style::Chevrons, "111000222"),
            ("chevrons-turns", Style::Chevrons, "1,1/4,R1/4,2,2/4,R2/4"),
            ("feathers-1234", Style::Feathers, "1234"),
            ("folk-12", Style::FolkMix, "12"),
            ("folk-1", Style::FolkMix, ""),
            ("folk-1120", Style::FolkMix, "1120"),
            ("laurel-st", Style::Laurel, "ST(1,2),1,0"),
        ] {
            let p = Params {
                style,
                make: Make::Carving,
                border_width: 30.0,
                sequence: sequence.into(),
                ..Params::default()
            };
            let mut after = doc.clone();
            after.apply(&Embellish::run(&doc, &p).unwrap()).unwrap();
            let part = after.components.values().next().unwrap();
            shade(part, &out.join(format!("sequence-{key}.png")));
        }
    }

    #[test]
    #[ignore = "writes pictures to look at"]
    fn pictures() {
        let out = std::path::PathBuf::from(std::env::var("EMBELLISH_OUT").unwrap());
        let mut shapes: Vec<(String, Path)> = vec![];
        for (n, name) in [
            (3, "triangle"),
            (4, "square"),
            (6, "hexagon"),
            (8, "octagon"),
        ] {
            shapes.push((name.into(), polygon(n, 120.0, false)));
            shapes.push((format!("{name}-catenary"), polygon(n, 120.0, true)));
        }
        if let Ok(f) = std::env::var("EMBELLISH_SHAPE") {
            let pts: Vec<[f64; 2]> =
                serde_json::from_str(&std::fs::read_to_string(f).unwrap()).unwrap();
            let pts: Vec<Point> = pts.iter().map(|p| Point::new(p[0], p[1])).collect();
            shapes.push(("your-shape".into(), Path::polyline(&pts, true)));
        }
        let variants: Vec<(&str, Params)> = vec![
            ("", Params::default()),
            (
                "-low",
                Params {
                    refinement: 1,
                    objects: 1,
                    scroll_size: 60,
                    ..Params::default()
                },
            ),
            (
                "-high",
                Params {
                    refinement: 10,
                    objects: 10,
                    scroll_size: 150,
                    tightness: 10,
                    ..Params::default()
                },
            ),
            (
                "-border",
                Params {
                    style: Style::BandBorder,
                    ..Params::default()
                },
            ),
            (
                "-rope",
                Params {
                    style: Style::Rope,
                    ..Params::default()
                },
            ),
            (
                "-chain",
                Params {
                    style: Style::Chain,
                    ..Params::default()
                },
            ),
            (
                "-twist",
                Params {
                    style: Style::Twist,
                    ..Params::default()
                },
            ),
            (
                "-braid",
                Params {
                    style: Style::Braid,
                    rails: true,
                    ..Params::default()
                },
            ),
            (
                "-bead",
                Params {
                    style: Style::BeadAndReel,
                    ..Params::default()
                },
            ),
            (
                "-chain-fill",
                Params {
                    style: Style::Chain,
                    ..Params::default()
                },
            ),
            (
                "-rope-fill",
                Params {
                    style: Style::Rope,
                    ..Params::default()
                },
            ),
            (
                "-braid-fill",
                Params {
                    style: Style::Braid,
                    ..Params::default()
                },
            ),
            (
                "-laurel",
                Params {
                    style: Style::Laurel,
                    ..Params::default()
                },
            ),
            (
                "-feathers",
                Params {
                    style: Style::Feathers,
                    ..Params::default()
                },
            ),
            (
                "-chevrons",
                Params {
                    style: Style::Chevrons,
                    ..Params::default()
                },
            ),
            (
                "-laurel-fill",
                Params {
                    style: Style::Laurel,
                    ..Params::default()
                },
            ),
            (
                "-feathers-fill",
                Params {
                    style: Style::Feathers,
                    ..Params::default()
                },
            ),
            (
                "-chevrons-fill",
                Params {
                    style: Style::Chevrons,
                    ..Params::default()
                },
            ),
            (
                "-scrolls",
                Params {
                    style: Style::Scrolls,
                    ..Params::default()
                },
            ),
            (
                "-flowers",
                Params {
                    style: Style::Flowers,
                    ..Params::default()
                },
            ),
            (
                "-sunk",
                Params {
                    direction: Direction::Sunk,
                    ..Params::default()
                },
            ),
            (
                "-mixed",
                Params {
                    direction: Direction::Mixed,
                    ..Params::default()
                },
            ),
            (
                "-mixed-rope",
                Params {
                    direction: Direction::Mixed,
                    style: Style::Rope,
                    ..Params::default()
                },
            ),
        ];
        let only = std::env::var("EMBELLISH_ONLY").unwrap_or_default();
        let only_variant = std::env::var("EMBELLISH_VARIANT").ok();
        let mut shapes: Vec<(String, Vec<Path>)> =
            shapes.into_iter().map(|(n, p)| (n, vec![p])).collect();
        // a ring: a round shape with a round hole
        let circle = |r: f64| {
            let pts: Vec<Point> = (0..240)
                .map(|k| {
                    Point::new(150.0, 150.0) + Point::from_angle(TAU * f64::from(k) / 240.0) * r
                })
                .collect();
            Path::polyline(&pts, true)
        };
        shapes.push(("ring".into(), vec![circle(120.0), circle(35.0)]));
        for (name, shape) in &shapes {
            if !only.is_empty() && !name.contains(&only) {
                continue;
            }
            for (suffix, p) in &variants {
                if only_variant
                    .as_ref()
                    .is_some_and(|v| !v.split(',').any(|v| v == *suffix))
                {
                    continue;
                }
                let doc = cut_craft_core::testing::document_with(shape.clone());
                let t = std::time::Instant::now();
                let change = Embellish::run(&doc, p).unwrap();
                let ms = t.elapsed().as_millis();
                let mut after = doc.clone();
                after.apply(&change).unwrap();
                eprintln!("{name}{suffix}: {} vectors, {ms} ms", after.vectors.len());
                shade(
                    after.components.values().next().unwrap(),
                    &out.join(format!("{name}{suffix}.png")),
                );
            }
        }
    }
}
