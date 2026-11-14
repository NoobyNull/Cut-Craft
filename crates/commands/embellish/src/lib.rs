//! Embellish: folk-swirl ornament designed for the selected shape rather than cut out of a
//! pattern. The shape is read as a distance field (how far each point is from its edge):
//! a border band and bead row follow the outline, and inside, a shape that repeats round
//! its middle (a regular polygon, a star) is designed one wedge at a time and mirrored into
//! the rest, while any other shape gets a vine down its middle with scrolls branching off
//! it, leaf fans in its points and flowers where it is roomiest. Every piece keeps the
//! smallest gap from the others, so the finishing bit can carve between them. The pieces
//! come out as vectors, as a 3D component, or both.

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
mod sym;

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

/// The one look the embellishment has: an ornament inside the shape, or a border round its
/// edge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Style {
    /// A band and bead row round the edge, and inside it a vine with scrolls, flowers, leaf
    /// fans and small curls together
    #[default]
    #[schemars(title = "Folk mix")]
    FolkMix,
    /// Inside only: a vine down the middle with scrolls curling off it, and small S-curls
    Scrolls,
    /// Inside only: rosettes where there is most room, and smaller flowers round them
    Flowers,
    /// Round the edge only: a rounded band with a row of beads inside it
    #[schemars(title = "Band and beads border")]
    BandBorder,
    /// Round the edge only: slanted strands laid side by side
    #[schemars(title = "Rope border")]
    Rope,
    /// Round the edge only: oval links seen face on, joined by links seen edge on
    #[schemars(title = "Chain border")]
    Chain,
    /// Round the edge only: two strands twisting round each other
    #[schemars(title = "Twist border")]
    Twist,
    /// Round the edge only: three strands plaited over and under
    #[schemars(title = "Braid border")]
    Braid,
    /// Round the edge only: oval beads with a pair of thin discs between each two
    #[schemars(title = "Bead and reel border")]
    BeadAndReel,
    /// Round the edge only: pairs of laurel leaves along a stem, as in a wreath
    #[schemars(title = "Laurel leaves border")]
    Laurel,
    /// Round the edge only: feathers one after another, each with a quill and barbs
    #[schemars(title = "Feathers border")]
    Feathers,
    /// Round the edge only: V shapes one after another, all pointing the same way
    #[schemars(title = "Chevrons border")]
    Chevrons,
}

impl Style {
    /// Where it goes, and what goes inside.
    fn layout(self) -> (Where, Motifs) {
        match self {
            Self::FolkMix => (Where::Both, Motifs::Mix),
            Self::Scrolls => (Where::Fill, Motifs::Scrolls),
            Self::Flowers => (Where::Fill, Motifs::Flowers),
            _ => (Where::Border, Motifs::Nothing),
        }
    }

    /// The patterned border's own pattern, for the patterned borders.
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
            _ => return None,
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
pub struct Params {
    /// The one look it has. Folk mix, Scrolls and Flowers fill the shape; the borders run
    /// round its edge (and round any hole in it)
    #[schemars(extend("x-section" = true, "x-widget" = "named-icons", "x-icon" = "drawing_forms/embellish_{}.png"))]
    pub style: Style,
    /// Fill the whole shape with the pattern: row after row following the outline inward,
    /// instead of one row round the edge
    #[schemars(title = "Fill the shape", extend("x-group" = "Pattern", "x-show-when" = {"style": ["rope", "chain", "twist", "braid", "bead_and_reel", "laurel", "feathers", "chevrons"]}))]
    pub fill_rows: bool,
    /// How wide the pattern is. On a shape narrower than this it is made narrower to fit,
    /// one row down the middle
    #[schemars(title = "Width", range(min = 0), extend("x-group" = "Pattern", "x-unit" = "mm", "x-show-when" = {"style": ["rope", "chain", "twist", "braid", "bead_and_reel", "laurel", "feathers", "chevrons"]}))]
    pub border_width: f64,
    /// How long each repeat of the pattern is, against its width; the count is rounded so
    /// the pattern meets itself
    #[schemars(title = "Repeat length", range(min = 50, max = 200), extend("x-group" = "Pattern", "x-unit" = "%", "x-widget" = "slider", "x-show-when" = {"style": ["rope", "chain", "twist", "braid", "bead_and_reel", "laurel", "feathers", "chevrons"]}))]
    pub repeat: u32,
    /// How steeply the rope's strands lie
    #[schemars(title = "Twist angle", range(min = 20, max = 60), extend("x-group" = "Pattern", "x-unit" = "°", "x-widget" = "slider", "x-show-when" = {"style": "rope"}))]
    pub twist: u32,
    /// How far in from the shape's edge the pattern starts
    #[schemars(title = "Gap from the edge", range(min = 0), extend("x-group" = "Pattern", "x-unit" = "mm", "x-show-when" = {"style": ["rope", "chain", "twist", "braid", "bead_and_reel", "laurel", "feathers", "chevrons"]}))]
    pub border_inset: f64,
    /// Thin raised lines along both sides of the pattern, framing it
    #[schemars(title = "Edge lines", extend("x-group" = "Pattern", "x-show-when" = {"style": ["rope", "chain", "twist", "braid", "bead_and_reel", "laurel", "feathers", "chevrons"]}))]
    pub rails: bool,
    /// Low makes a few bold, plain pieces; high adds bead rows, sunbursts, striped scrolls
    /// and small curls
    #[schemars(title = "Detail", range(min = 1, max = 10), extend("x-group" = "Ornament", "x-widget" = "slider", "x-show-when" = {"style": ["folk_mix", "scrolls", "flowers", "band_border"]}))]
    pub refinement: u32,
    /// How full the shape gets: low leaves open ground between the pieces
    #[schemars(title = "How full", range(min = 1, max = 10), extend("x-group" = "Ornament", "x-widget" = "slider", "x-show-when" = {"style": ["folk_mix", "scrolls", "flowers"]}))]
    pub objects: u32,
    /// How big the scrolls may grow
    #[schemars(title = "Scroll size", range(min = 50, max = 160), extend("x-group" = "Ornament", "x-unit" = "%", "x-widget" = "slider", "x-show-when" = {"style": ["folk_mix", "scrolls"]}))]
    pub scroll_size: u32,
    /// How close the pieces sit: the smallest gap between them is 3 mm at 1 and 0.75 mm at
    /// 10. Finish with a ball-nose bit no wider than the gap
    #[schemars(range(min = 1, max = 10), extend("x-group" = "Fit to your bit", "x-widget" = "slider"))]
    pub tightness: u32,
    /// Outlines to cut with 2D toolpaths, a rounded 3D carving, or both
    #[schemars(extend("x-group" = "Make", "x-widget" = "row"))]
    pub make: Make,
    /// Another number gives another layout with the same settings
    #[schemars(title = "Layout", extend("x-group" = "Make", "x-widget" = "seed"))]
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
            border_width: 14.0,
            repeat: 100,
            twist: 45,
            border_inset: 3.0,
            rails: false,
            fill_rows: false,
            refinement: 6,
            objects: 6,
            scroll_size: 100,
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
        })
    }

    fn settings(&self, seed: u32) -> Settings {
        let unit = |v: u32| f64::from(v.clamp(1, 10) - 1) / 9.0;
        Settings {
            gap: self.gap(),
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

/// One shape to embellish: an outline and the holes in it, and where its field sits.
struct Region {
    field: Field,
    offset: Point,
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
        });
    }
    out
}

/// The ornament for one shape, in its field's frame: a patterned border when there is one,
/// then the ornament inside it, kept clear of it. Also how many of the pieces, from the
/// start, are the patterned border's.
fn design(
    field: &Field,
    s: Settings,
    pattern: Option<border::Spec>,
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
fn fitted(field: &Field, b: border::Spec) -> border::Spec {
    let room = half_width(field);
    if b.inset + b.width <= room * 0.95 {
        return b;
    }
    let inset = b.inset.min(room * 0.1);
    // the row's middle just off the band's middle, where the two edge lines nearly meet
    let middle = room * 0.92;
    border::Spec {
        inset,
        width: 2.0 * (middle - inset).max(0.0),
        rails: false,
        single: true,
        ..b
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
        let (_, closed) = doc.selected_paths(|path| path.closed);
        if closed.is_empty() {
            return None;
        }
        kept.iter().position(|k| {
            k.paths.len() == closed.len() && closed.iter().all(|q| k.paths.contains(q))
        })
    };
    let i = if doc.selection.is_empty() {
        by_component()?
    } else {
        by_piece().or_else(by_outline)?
    };
    kept.into_iter().nth(i)
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
            ensure_positive("border width", p.border_width)?;
        }
        let editing = p.editing.and_then(|at| {
            all_kept(doc).into_iter().find(|k| {
                k.at == at
                    || (at.component.is_some() && k.at.component == at.component)
                    || (at.component.is_none() && at.layer.is_some() && k.at.layer == at.layer)
            })
        });
        let shapes = match &editing {
            Some(k) => k.paths.clone(),
            None => doc.selected_paths(|path| path.closed).1,
        };
        if shapes.is_empty() {
            return Err(CommandError::Failed(
                "select a closed vector to embellish: the ornament is designed to fit it".into(),
            ));
        }
        let rings: Vec<Vec<Point>> = shapes
            .iter()
            .map(|s| s.flatten(cut_craft_core::polygon::TOLERANCE))
            .filter(|r| r.len() >= 3)
            .collect();
        let regions = regions(&rings);
        let mut items: Vec<Item> = vec![];
        let mut sunk: Vec<bool> = vec![];
        for (k, region) in regions.iter().enumerate() {
            let seed = p.seed.wrapping_add(k as u32 * 7919);
            let offset = region.offset;
            let (made, n) = design(&region.field, p.settings(seed), p.pattern(), p.fill_rows);
            sunk.extend(match p.direction {
                Direction::Raised => vec![false; made.len()],
                Direction::Sunk => vec![true; made.len()],
                Direction::Mixed => small_ones(&made, n),
            });
            items.extend(made.iter().map(|it| it.map(&|q| q + offset)));
        }
        if !items
            .iter()
            .any(|it| matches!(it, Item::Shape { sink: false, .. }))
        {
            let wide = 2.0
                * regions
                    .iter()
                    .map(|r| half_width(&r.field))
                    .fold(0.0, f64::max);
            return Err(CommandError::Failed(format!(
                "nothing fits: the shape is only about {wide:.1} mm wide, too \
                 narrow for this style with a {gap:.2} mm smallest gap between pieces. Pick a \
                 wider shape, turn Tightness up for a smaller gap, or try Chevrons or Laurel \
                 leaves, which fit the narrowest bands",
                gap = p.gap(),
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
            let mut part = carving(p, &regions, &items, &sunk, doc.job.model_cell());
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
        Ok(c.finish())
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
    let cell = (p.gap() / 4.0)
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
                ..with(Direction::Mixed)
            };
            let (pieces, n) = {
                let rings: Vec<Vec<Point>> = vec![polygon(6, 110.0).flatten(0.05)];
                let r = regions(&rings);
                design(&r[0].field, p.settings(4), p.pattern(), false)
            };
            let small = small_ones(&pieces, n);
            assert!(small[..n].iter().all(|s| !s), "the border stays raised");
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
            assert!(new.len() > 40, "{} pieces", new.len());
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
        let other = Params {
            seed: 5,
            ..p.clone()
        };
        assert_ne!(shapes(&made(&doc, &p)), shapes(&made(&doc, &other)));
    }

    #[test]
    fn more_objects_and_refinement_make_more_pieces() {
        let doc = document_with(vec![lopsided()]);
        let count = |objects, refinement| {
            let p = Params {
                objects,
                refinement,
                make: Make::Vectors,
                ..Params::default()
            };
            made(&doc, &p).vectors.len()
        };
        assert!(count(10, 10) > count(1, 1));
    }

    #[test]
    fn a_border_keeps_to_the_edge_and_a_fill_has_no_band() {
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
        assert_eq!(near_middle(Style::BandBorder), 0);
        assert!(near_middle(Style::FolkMix) > 0);
    }

    #[test]
    fn a_patterned_border_runs_round_the_edge_alone() {
        let shape = polygon(6, 110.0);
        let doc = document_with(vec![shape.clone()]);
        let centre = Point::new(150.0, 150.0);
        for border in [
            Style::Rope,
            Style::Chain,
            Style::Twist,
            Style::Braid,
            Style::BeadAndReel,
            Style::Laurel,
            Style::Feathers,
            Style::Chevrons,
        ] {
            let only = Params {
                style: border,
                make: Make::Vectors,
                ..Params::default()
            };
            let after = made(&doc, &only);
            let pieces: Vec<&Path> = after.vectors.values().filter(|v| **v != shape).collect();
            assert!(pieces.len() > 30, "{border:?}: {}", pieces.len());
            // the hexagon's apothem is 95 mm: the border keeps to its outer part
            for v in &pieces {
                let d = v.bounds().center().distance(centre);
                assert!(d > 65.0 || v.bounds().size().x > 100.0, "{border:?} at {d}");
            }
        }
    }

    /// The pieces' sizes, longest side, biggest first.
    fn piece_sizes(doc: &Document, outline: &Path) -> Vec<f64> {
        let mut sizes: Vec<f64> = doc
            .vectors
            .values()
            .filter(|v| *v != outline)
            .map(|v| {
                let s = v.bounds().size();
                s.x.max(s.y)
            })
            .collect();
        sizes.sort_by(|a, b| b.total_cmp(a));
        sizes
    }

    #[test]
    fn each_style_is_one_kind_of_piece() {
        for shape in [lopsided(), polygon(6, 110.0)] {
            let doc = document_with(vec![shape.clone()]);
            let of = |style| {
                made(
                    &doc,
                    &Params {
                        style,
                        make: Make::Vectors,
                        ..Params::default()
                    },
                )
            };
            let (mix, scrolls, flowers) =
                (of(Style::FolkMix), of(Style::Scrolls), of(Style::Flowers));
            let (m, sc, fl) = (
                piece_sizes(&mix, &shape),
                piece_sizes(&scrolls, &shape),
                piece_sizes(&flowers, &shape),
            );
            eprintln!(
                "mix {:?}\nscrolls {:?}\nflowers {:?}",
                &m[..8],
                &sc[..8.min(sc.len())],
                &fl[..8.min(fl.len())]
            );
            assert!(sc.len() > 10 && fl.len() > 10, "{} {}", sc.len(), fl.len());
            assert!(
                sc.len() < m.len() && fl.len() < m.len(),
                "fewer kinds, fewer pieces"
            );
            // no border band (the biggest piece of the mix) in either
            let outline = shape.bounds().size().x;
            assert!(m[0] > outline * 0.9);
            assert!(sc[0] < outline * 0.9 && fl[0] < outline * 0.9);
        }
    }

    #[test]
    fn a_patterned_border_can_fill_the_shape_row_after_row() {
        let shape = polygon(6, 110.0);
        let doc = document_with(vec![shape.clone()]);
        let centre = Point::new(150.0, 150.0);
        for style in [
            Style::Rope,
            Style::Chain,
            Style::BeadAndReel,
            Style::Laurel,
            Style::Feathers,
            Style::Chevrons,
        ] {
            let p = |fill_rows| Params {
                style,
                fill_rows,
                make: Make::Vectors,
                ..Params::default()
            };
            let one = made(&doc, &p(false));
            let rows = made(&doc, &p(true));
            assert!(
                rows.vectors.len() > 2 * one.vectors.len(),
                "{style:?}: {} rows vs {}",
                rows.vectors.len(),
                one.vectors.len()
            );
            let near_middle = rows
                .vectors
                .values()
                .filter(|v| v.bounds().center().distance(centre) < 40.0)
                .count();
            assert!(near_middle > 0, "{style:?}: rows reach the middle");
        }
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
                border_width: 14.0,
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
    fn too_narrow_a_shape_says_how_wide_it_is_and_what_to_change() {
        let doc = band(1.5);
        let p = Params {
            style: Style::Rope,
            ..Params::default()
        };
        let err = Embellish::run(&doc, &p).unwrap_err().to_string();
        assert!(err.contains("mm wide"), "{err}");
        assert!(err.contains("Tightness"), "{err}");
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
    fn needs_a_closed_vector() {
        let err = Embellish::run(&Document::default(), &Params::default()).unwrap_err();
        assert!(err.to_string().contains("closed vector"), "{err}");
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
                    fill_rows: true,
                    ..Params::default()
                },
            ),
            (
                "-rope-fill",
                Params {
                    style: Style::Rope,
                    fill_rows: true,
                    ..Params::default()
                },
            ),
            (
                "-braid-fill",
                Params {
                    style: Style::Braid,
                    fill_rows: true,
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
                    fill_rows: true,
                    ..Params::default()
                },
            ),
            (
                "-feathers-fill",
                Params {
                    style: Style::Feathers,
                    fill_rows: true,
                    ..Params::default()
                },
            ),
            (
                "-chevrons-fill",
                Params {
                    style: Style::Chevrons,
                    fill_rows: true,
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
