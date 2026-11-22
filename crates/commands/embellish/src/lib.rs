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
    /// How each repeat is drawn, in turn: 0 a space, 1 as designed, 2 flipped UD, 3
    /// flipped LR, 4 flipped LR and UD. /n turns it by 180° ÷ n (1/4 is 45°), R turns it the
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
                    c.add_new_component(part);
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
mod tests;

/// Pictures of the carving for a look by eye: `EMBELLISH_OUT=dir cargo nextest run -p
/// cmd-embellish pictures --run-ignored only` (and `EMBELLISH_SHAPE`, a JSON list of
/// `[x, y]`, for a drawn shape).
#[cfg(test)]
mod pictures;
