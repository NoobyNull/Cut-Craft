//! A finished preview kept on the graphics card in tiles, at its own detail and at a half
//! and a quarter of it. Each frame the card draws one of those, the one whose points are
//! about a pixel apart on screen, and only its tiles in sight: zoomed in, the full detail
//! of the part looked at; zoomed out, a quarter of the points over the whole preview.
//! Nothing is thrown away on zooming out: the tiles stay on the card within a memory budget
//! the Preview detail slider sets, the ones not looked at for longest let go first. All of
//! it goes with the preview (Reset Preview, a new preview) and when PowerWhittle closes.
//!
//! The tiles of a level share its heights on the card, so the shading and the shadows run
//! on across their edges; and a frame draws one level throughout, so no cracks open where
//! tiles of different detail would meet.

use super::gpu::{self, Geometry};
use cut_craft_core::geometry::Point;
use cut_craft_core::glam::DVec3;
use cut_craft_core::simulation::Material;
use eframe::egui::{Pos2, Rect};
use std::sync::Arc;

/// Quads along a tile's side, at every level.
pub const TILE: usize = 128;
/// The full detail, then a half and a quarter of it.
pub const LEVELS: usize = 3;
/// Points on screen a level's points may be apart before a finer one is drawn, in pixels.
const PIXELS_APART: f64 = 1.0;

/// `m` at half its detail: every other point kept, each the deepest of the points round
/// it (so a fine groove still shows zoomed out), and its paint and mark with it. Its edges
/// stay where `m`'s are, near enough (half a cell of `m` out at most).
#[must_use]
pub fn halved(m: &Material) -> Material {
    let half = |n: usize| n / 2 + 1;
    let (columns, rows) = (half(m.columns), half(m.rows));
    let mut chosen = Vec::with_capacity(columns * rows);
    for r in 0..rows {
        for c in 0..columns {
            let around = |i: usize, n: usize| (2 * i).saturating_sub(1)..(2 * i + 2).min(n);
            let mut deepest: Option<usize> = None;
            let mut any = None;
            for fr in around(r, m.rows) {
                for fc in around(c, m.columns) {
                    let i = fr * m.columns + fc;
                    any.get_or_insert(i);
                    if m.heights[i].is_nan() {
                        continue;
                    }
                    if deepest.is_none_or(|d| m.heights[i] < m.heights[d]) {
                        deepest = Some(i);
                    }
                }
            }
            chosen.push(deepest.or(any).unwrap_or(0));
        }
    }
    let of = |values: &[u32]| -> Vec<u32> {
        if values.is_empty() {
            vec![]
        } else {
            chosen.iter().map(|&i| values[i]).collect()
        }
    };
    let cell = m.cell * 2.0;
    Material {
        // a point's centre lands on the centre of the point it was taken from
        origin: m.origin - Point::splat(m.cell / 2.0),
        columns,
        rows,
        cell,
        bottom: m.bottom,
        heights: chosen.iter().map(|&i| m.heights[i]).collect(),
        cut_by: of(&m.cut_by),
        cutting: m.cutting,
        paint: of(&m.paint),
        other_side: None,
        under: vec![],
    }
}

/// The level drawn when a full-detail point is `pixels` apart on screen: the coarsest whose
/// points are still no more than [`PIXELS_APART`] apart, so it is never coarser than the
/// screen shows.
#[must_use]
pub fn level_for(pixels: f64, levels: usize) -> usize {
    let mut level = 0;
    while level + 1 < levels && pixels * f64::from(1u32 << (level + 1)) <= PIXELS_APART {
        level += 1;
    }
    level
}

/// A level's tiles: `[first column, first row, quads across, quads up]` each, covering
/// its quads once.
#[must_use]
pub fn spans(columns: usize, rows: usize) -> Vec<[usize; 4]> {
    let (across, up) = (columns.saturating_sub(1), rows.saturating_sub(1));
    let mut spans = vec![];
    for r0 in (0..up).step_by(TILE) {
        for c0 in (0..across).step_by(TILE) {
            spans.push([c0, r0, TILE.min(across - c0), TILE.min(up - r0)]);
        }
    }
    spans
}

/// One tile of a tiled preview: the triangles over the quads `columns` × `rows` from
/// (`c0`, `r0`) of a grid `across` cells wide, the quads not `shown` (each named by its
/// first corner) left out; with `under` (where the grid's flat bottom's vertices start),
/// the bottom under the quads shown, so the preview is closed underneath. They index the
/// grid's own heights ([`gpu::heights`]), so neighbouring tiles share their edges and the
/// shading and shadows run on across them.
pub struct Tile {
    bytes: Vec<u8>,
}

impl Tile {
    pub fn new(
        across: usize,
        [c0, r0, columns, rows]: [usize; 4],
        shown: impl Fn(usize, usize) -> bool,
        under: Option<u32>,
    ) -> Self {
        let mut bytes = Vec::with_capacity(columns * rows * 24);
        let index = |c: usize, r: usize| u32::try_from(r * across + c).expect("fits u32");
        let mut quad = |corners: [(usize, usize); 4], from: u32| {
            let [a, b, cc, d] = corners.map(|(c, r)| from + index(c, r));
            for i in [a, b, cc, a, cc, d] {
                bytes.extend_from_slice(&i.to_ne_bytes());
            }
        };
        let mut all = true;
        for r in r0..r0 + rows {
            for c in c0..c0 + columns {
                if shown(c, r) {
                    quad([(c, r), (c + 1, r), (c + 1, r + 1), (c, r + 1)], 0);
                } else {
                    all = false;
                }
            }
        }
        // the bottom, wound the other way to face down: one quad under a whole tile
        if let Some(under) = under {
            let (c1, r1) = (c0 + columns, r0 + rows);
            if all {
                quad([(c0, r0), (c0, r1), (c1, r1), (c1, r0)], under);
            } else {
                for r in r0..r1 {
                    for c in (c0..c1).filter(|&c| shown(c, r)) {
                        quad([(c, r), (c, r + 1), (c + 1, r + 1), (c + 1, r)], under);
                    }
                }
            }
        }
        Self { bytes }
    }

    /// Bytes on the card.
    pub fn bytes(&self) -> usize {
        self.bytes.len()
    }

    /// Its indices, as the card takes them.
    pub(super) fn indices(&self) -> &[u8] {
        &self.bytes
    }

    /// Indices drawn.
    pub(super) fn count(&self) -> u32 {
        u32::try_from(self.bytes.len() / 4).expect("fits u32")
    }
}

/// A tile: where it is, its lowest and highest points (to see whether it is in sight),
/// and its triangles once made, with the frame it was last drawn in.
struct Slot {
    span: [usize; 4],
    low: f64,
    high: f64,
    made: Option<(Arc<Tile>, u64)>,
}

/// One level: its heights on the card and its tiles.
struct Level {
    geometry: Arc<Geometry>,
    origin: Point,
    cell: f64,
    columns: usize,
    /// Whether the quad from each cell is drawn ([`Material::quad_shown`]); none when
    /// all are.
    shown: Vec<bool>,
    /// Where the grid's flat bottom's vertices start, when the preview is closed
    /// underneath (the block's sides shown).
    under: Option<u32>,
    slots: Vec<Slot>,
}

impl Level {
    fn new(m: &Material, sides: bool) -> Self {
        let (columns, rows) = (m.columns, m.rows);
        let quad = |c: usize, r: usize| {
            let at = |c: usize, r: usize| r * columns + c;
            c + 1 < columns
                && r + 1 < rows
                && m.quad_shown([at(c, r), at(c + 1, r), at(c + 1, r + 1), at(c, r + 1)])
        };
        let shown: Vec<bool> = if m.heights.iter().any(|&h| h.is_nan() || m.through(h)) {
            (0..rows)
                .flat_map(|r| (0..columns).map(move |c| (c, r)))
                .map(|(c, r)| quad(c, r))
                .collect()
        } else {
            vec![]
        };
        // after the heights, the sides' two vertices an edge cell and the bottom's corners
        let under = sides
            .then(|| u32::try_from(columns * rows + 4 * (columns + rows) + 4).expect("fits u32"));
        let slots = spans(m.columns, m.rows)
            .into_iter()
            .map(|span @ [c0, r0, across, up]| {
                let (mut low, mut high) = (f64::INFINITY, f64::NEG_INFINITY);
                for r in r0..=r0 + up {
                    for &h in &m.heights[r * m.columns + c0..=r * m.columns + c0 + across] {
                        if !h.is_nan() {
                            low = low.min(h);
                            high = high.max(h);
                        }
                    }
                }
                if sides {
                    low = m.bottom;
                }
                Slot {
                    span,
                    low: low.min(high),
                    high,
                    made: None,
                }
            })
            .collect();
        Self {
            geometry: Arc::new(gpu::heights(m, None, false)),
            origin: m.origin,
            cell: m.cell,
            columns: m.columns,
            shown,
            under,
            slots,
        }
    }

    /// A tile's eight corners, in the job's coordinates (its top and bottom heights).
    fn corners(&self, slot: &Slot) -> [DVec3; 8] {
        let [c0, r0, across, up] = slot.span;
        let at = |c: usize, r: usize| {
            #[expect(clippy::cast_precision_loss, reason = "cell indices")]
            let (x, y) = ((c as f64 + 0.5) * self.cell, (r as f64 + 0.5) * self.cell);
            self.origin + Point::new(x, y)
        };
        let (lo, hi) = (at(c0, r0), at(c0 + across, r0 + up));
        let (low, high) = if slot.low.is_finite() {
            (slot.low, slot.high)
        } else {
            (0.0, 0.0)
        };
        let mut out = [DVec3::ZERO; 8];
        for (k, p) in out.iter_mut().enumerate() {
            let x = if k & 1 == 0 { lo.x } else { hi.x };
            let y = if k & 2 == 0 { lo.y } else { hi.y };
            *p = DVec3::new(x, y, if k & 4 == 0 { low } else { high });
        }
        out
    }

    /// The tiles in sight: each one's slot and how far apart its points are on screen, in
    /// points: along its rows and along its columns, at its top and its bottom, the widest
    /// of those, so a tile seen nearly edge-on one way is never drawn short the other way.
    fn in_sight(&self, screen: &impl Fn(DVec3) -> Pos2, view: Rect) -> Vec<(usize, f64)> {
        (self.slots.iter().enumerate())
            .filter_map(|(i, slot)| {
                let at = self.corners(slot).map(screen);
                if !Rect::from_points(&at).intersects(view) || slot.high.is_infinite() {
                    return None;
                }
                // corners: bit 0 the far column, bit 1 the far row, bit 2 the top
                let along = |step: usize, quads: usize| {
                    let length = [0, 4]
                        .map(|z| at[z].distance(at[z + step]))
                        .into_iter()
                        .fold(0.0, f32::max);
                    #[expect(clippy::cast_precision_loss, reason = "a quad count")]
                    let quads = quads.max(1) as f64;
                    f64::from(length) / quads
                };
                Some((i, along(1, slot.span[2]).max(along(2, slot.span[3]))))
            })
            .collect()
    }
}

/// A preview in tiles at [`LEVELS`] details, kept within `budget` bytes of triangles.
pub struct Tiled {
    levels: Vec<Level>,
    budget: usize,
    frame: u64,
}

/// What a frame draws: a level's heights, where its corner goes (the job's coordinates),
/// and its tiles in sight.
pub struct Frame {
    /// Which of [`LEVELS`] it is.
    #[cfg(test)]
    pub level: usize,
    pub geometry: Arc<Geometry>,
    pub origin: Point,
    pub tiles: Vec<Arc<Tile>>,
}

impl Tiled {
    /// `m` in tiles, keeping up to `budget` bytes of triangles on the card; with `sides`
    /// (the block's sides shown), closed underneath.
    #[must_use]
    pub fn new(m: &Material, budget: usize, sides: bool) -> Self {
        let mut levels = vec![Level::new(m, sides)];
        let mut at = m.clone();
        while levels.len() < LEVELS && at.columns > 2 && at.rows > 2 {
            at = halved(&at);
            levels.push(Level::new(&at, sides));
        }
        Self {
            levels,
            budget,
            frame: 0,
        }
    }

    /// This frame's drawing: `screen` places a point of the job on screen (in points),
    /// `pixels` of which there are in a point, over `view`. The level whose points show
    /// about a pixel apart (the finest any tile in sight needs, the nearest in perspective)
    /// and its tiles in sight,
    /// their triangles made the first time; then those drawn longest ago are let go while
    /// the budget is passed. `None` when no tile is in sight.
    pub fn frame(
        &mut self,
        screen: impl Fn(DVec3) -> Pos2,
        view: Rect,
        pixels: f32,
    ) -> Option<Frame> {
        self.frame += 1;
        let finest = self.levels[0].in_sight(&screen, view);
        // the widest apart any tile in sight shows them: no tile drawn coarser than a pixel
        let apart = (finest.iter().map(|&(_, a)| a)).fold(f64::NEG_INFINITY, f64::max);
        if !apart.is_finite() {
            return None;
        }
        let level = level_for(apart * f64::from(pixels), self.levels.len());
        let frame = self.frame;
        let l = &mut self.levels[level];
        let sight: Vec<usize> = l
            .in_sight(&screen, view)
            .into_iter()
            .map(|(i, _)| i)
            .collect();
        let mut tiles = Vec::with_capacity(sight.len());
        for i in sight {
            let (columns, shown, under) = (l.columns, &l.shown, l.under);
            let slot = &mut l.slots[i];
            let tile = if let Some((tile, seen)) = &mut slot.made {
                *seen = frame;
                Arc::clone(tile)
            } else {
                let shown =
                    |c: usize, r: usize| shown.get(r * columns + c).copied().unwrap_or(true);
                let tile = Arc::new(Tile::new(columns, slot.span, shown, under));
                slot.made = Some((Arc::clone(&tile), frame));
                tile
            };
            tiles.push(tile);
        }
        self.keep_to_budget();
        let l = &self.levels[level];
        Some(Frame {
            #[cfg(test)]
            level,
            geometry: Arc::clone(&l.geometry),
            origin: l.origin,
            tiles,
        })
    }

    /// Bytes of triangles kept.
    #[must_use]
    pub fn kept(&self) -> usize {
        self.made().map(|(_, _, tile, _)| tile.bytes()).sum()
    }

    /// Every tile made: its level, slot, triangles and the frame it was last drawn in.
    fn made(&self) -> impl Iterator<Item = (usize, usize, &Arc<Tile>, u64)> {
        self.levels.iter().enumerate().flat_map(|(l, level)| {
            (level.slots.iter().enumerate())
                .filter_map(move |(i, s)| s.made.as_ref().map(|(t, seen)| (l, i, t, *seen)))
        })
    }

    /// Lets go of the tiles drawn longest ago while more than the budget is kept; never
    /// those drawn this frame.
    fn keep_to_budget(&mut self) {
        let mut kept = self.kept();
        if kept <= self.budget {
            return;
        }
        let mut old: Vec<(u64, usize, usize, usize)> = self
            .made()
            .filter(|&(_, _, _, seen)| seen < self.frame)
            .map(|(l, i, tile, seen)| (seen, l, i, tile.bytes()))
            .collect();
        old.sort_unstable();
        for (_, l, i, bytes) in old {
            if kept <= self.budget {
                break;
            }
            self.levels[l].slots[i].made = None;
            kept -= bytes;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::document::Job;

    /// A flat grid `columns` × `rows`, 1 mm cells from (10, 20).
    fn grid(columns: usize, rows: usize) -> Material {
        let mut m = Material::new(&Job::default(), 10);
        m.origin = Point::new(10.0, 20.0);
        m.cell = 1.0;
        (m.columns, m.rows) = (columns, rows);
        m.heights = vec![0.0; columns * rows];
        m.cut_by = vec![0; columns * rows];
        m.paint = vec![];
        m
    }

    #[test]
    #[expect(clippy::cast_precision_loss, reason = "cell indices")]
    fn halving_keeps_the_deepest_point_round_each_and_the_edges() {
        let mut m = grid(9, 5);
        // a groove one point wide at an odd column: kept at half detail
        for r in 0..5 {
            m.heights[r * 9 + 3] = -2.0;
        }
        m.paint = (0..45).collect();
        let h = halved(&m);
        assert_eq!((h.columns, h.rows), (5, 3));
        assert!((h.cell - 2.0).abs() < 1e-12);
        for r in 0..3 {
            assert_eq!(
                h.heights[r * 5 + 1],
                -2.0,
                "the groove at column 3 shows at 1"
            );
            assert_eq!(
                h.heights[r * 5 + 2],
                -2.0,
                "and at 2, whose round takes it in"
            );
            assert_eq!(h.heights[r * 5], 0.0);
        }
        // a point's centre is on the centre of the one it came from
        let centre = |m: &Material, c: usize| m.origin.x + (c as f64 + 0.5) * m.cell;
        assert!((centre(&h, 2) - centre(&m, 4)).abs() < 1e-12);
        assert!(
            (centre(&h, 4) - centre(&m, 8)).abs() < 1e-12,
            "the far edge stays"
        );
        assert_eq!(h.paint[1], 3, "the paint of the point it took");
    }

    #[test]
    fn halving_steps_over_gone_points() {
        let mut m = grid(4, 4);
        m.heights[0] = f64::NAN;
        m.heights[1] = 1.0;
        let h = halved(&m);
        assert!(
            !h.heights[0].is_nan(),
            "a point left beside a gone one is taken"
        );
        let mut all = grid(3, 3);
        all.heights = vec![f64::NAN; 9];
        assert!(halved(&all).heights.iter().all(|h| h.is_nan()));
    }

    #[test]
    fn the_level_drawn_is_never_coarser_than_a_pixel() {
        assert_eq!(level_for(2.0, 3), 0, "points far apart: full detail");
        assert_eq!(level_for(1.0, 3), 0);
        assert_eq!(level_for(0.5, 3), 1, "two to a pixel: half");
        assert_eq!(level_for(0.4, 3), 1);
        assert_eq!(level_for(0.25, 3), 2, "four to a pixel: a quarter");
        assert_eq!(level_for(0.01, 3), 2, "no coarser than there are");
        assert_eq!(level_for(0.01, 1), 0);
    }

    #[test]
    fn tiles_cover_every_quad_once() {
        for (columns, rows) in [(2, 2), (129, 129), (300, 140), (1000, 7)] {
            let mut seen = vec![0u8; (columns - 1) * (rows - 1)];
            for [c0, r0, across, up] in spans(columns, rows) {
                assert!(across <= TILE && up <= TILE);
                for r in r0..r0 + up {
                    for c in c0..c0 + across {
                        seen[r * (columns - 1) + c] += 1;
                    }
                }
            }
            assert!(seen.iter().all(|&n| n == 1), "{columns} x {rows}");
        }
        assert!(spans(1, 5).is_empty(), "no quads, no tiles");
    }

    /// Seen from above, 1 point a mm (and `zoom` times that), the job's (0, 0) at the view's
    /// corner.
    fn above(zoom: f32) -> impl Fn(DVec3) -> Pos2 {
        #[expect(clippy::cast_possible_truncation, reason = "a test's screen")]
        move |p: DVec3| Pos2::new(p.x as f32 * zoom, p.y as f32 * zoom)
    }

    #[test]
    fn zoomed_out_draws_a_quarter_and_zoomed_in_only_the_tiles_in_sight() {
        let m = grid(1001, 1001);
        let mut t = Tiled::new(&m, usize::MAX, false);
        let view = Rect::from_min_size(Pos2::ZERO, eframe::egui::vec2(600.0, 600.0));
        // the whole 1000 mm in 600 pixels: a quarter is still under a pixel apart
        let far = t.frame(above(0.6), view, 1.0).expect("in sight");
        assert_eq!(far.level, 0, "0.6 px apart: half would be 1.2, too coarse");
        let far = t.frame(above(0.5), view, 1.0).expect("in sight");
        assert_eq!(far.level, 1, "0.5 px apart: half is a pixel");
        let far = t.frame(above(0.25), view, 1.0).expect("in sight");
        assert_eq!(far.level, 2);
        let quarter = spans(251, 251).len();
        assert_eq!(far.tiles.len(), quarter, "the whole of it");
        // zoomed in 4 times: full detail, and only the corner in sight
        let near = t.frame(above(4.0), view, 1.0).expect("in sight");
        assert_eq!(near.level, 0);
        let all = spans(1001, 1001).len();
        assert!(near.tiles.len() < all / 4, "{} of {all}", near.tiles.len());
        assert!((near.origin.x - 10.0).abs() < 1e-12);
        // looking away: nothing
        let away = |p: DVec3| above(1.0)(p) + eframe::egui::vec2(5000.0, 0.0);
        assert!(t.frame(away, view, 1.0).is_none());
    }

    #[test]
    fn the_nearest_tile_and_the_widest_way_round_decide_the_detail() {
        let m = grid(1001, 1001);
        let view = Rect::from_min_size(Pos2::ZERO, eframe::egui::vec2(2000.0, 2000.0));
        let mut t = Tiled::new(&m, usize::MAX, false);
        // nearer at the bottom: 0.2 px apart far off, 1.0 near (as in perspective)
        #[expect(clippy::cast_possible_truncation, reason = "a test's screen")]
        let leaning = |p: DVec3| {
            let zoom = 0.2 + 0.8 * (1.0 - p.y / 1010.0);
            Pos2::new((p.x * zoom) as f32, (p.y * 0.2) as f32)
        };
        assert_eq!(
            t.frame(leaning, view, 1.0).unwrap().level,
            0,
            "the near part"
        );
        // nearly edge-on one way: 0.6 px apart across, a hair up
        #[expect(clippy::cast_possible_truncation, reason = "a test's screen")]
        let edge_on = |p: DVec3| Pos2::new((p.x * 0.6) as f32, (p.y * 0.0006) as f32);
        assert_eq!(
            t.frame(edge_on, view, 1.0).unwrap().level,
            0,
            "0.6 px across"
        );
    }

    #[test]
    fn tiles_stay_made_and_the_oldest_go_past_the_budget() {
        let m = grid(1001, 1001);
        let view = Rect::from_min_size(Pos2::ZERO, eframe::egui::vec2(600.0, 600.0));
        let mut t = Tiled::new(&m, usize::MAX, false);
        let first = t.frame(above(4.0), view, 1.0).unwrap();
        let again = t.frame(above(4.0), view, 1.0).unwrap();
        assert!(
            first
                .tiles
                .iter()
                .zip(&again.tiles)
                .all(|(a, b)| Arc::ptr_eq(a, b)),
            "made once, kept"
        );
        let one_view = t.kept();
        // a budget of one view: panning on lets the first view's tiles go
        let mut t = Tiled::new(&m, one_view, false);
        let _ = t.frame(above(4.0), view, 1.0).unwrap();
        let moved = |p: DVec3| above(4.0)(p) - eframe::egui::vec2(2400.0, 0.0);
        let later = t.frame(moved, view, 1.0).unwrap();
        assert!(t.kept() <= one_view.max(later.tiles.iter().map(|t| t.bytes()).sum()));
        assert!(
            t.levels[0].slots[0].made.is_none(),
            "the first corner, let go"
        );
        // zooming out keeps the full detail made, within the budget
        let mut t = Tiled::new(&m, usize::MAX, false);
        let _ = t.frame(above(4.0), view, 1.0).unwrap();
        let _ = t.frame(above(0.25), view, 1.0).unwrap();
        assert!(
            t.levels[0].slots[0].made.is_some(),
            "never thrown away zooming out"
        );
    }

    fn indices(tile: &Tile) -> Vec<u32> {
        tile.indices()
            .chunks(4)
            .map(|b| u32::from_ne_bytes(b.try_into().unwrap()))
            .collect()
    }

    #[test]
    fn a_tile_indexes_the_whole_grid_s_heights_and_leaves_out_quads_not_shown() {
        // a grid 5 across; the tile is the 2 x 1 quads from (1, 2)
        let tile = Tile::new(5, [1, 2, 2, 1], |_, _| true, None);
        assert_eq!(tile.count(), 12);
        // the quad (1,2)-(2,3): corners 11, 12, 17, 16 of the whole grid
        assert_eq!(indices(&tile)[..6], [11, 12, 17, 11, 17, 16]);
        let holed = Tile::new(5, [1, 2, 2, 1], |c, r| (c, r) != (2, 2), None);
        assert_eq!(holed.count(), 6, "the quad not shown is left out");
        assert_eq!(holed.bytes(), 24);
    }

    #[test]
    fn a_tile_is_closed_underneath_under_the_quads_it_shows() {
        let under = 1_000;
        // whole: one quad under the tile, wound to face down
        let whole = Tile::new(5, [1, 2, 2, 1], |_, _| true, Some(under));
        assert_eq!(whole.count(), 12 + 6);
        assert_eq!(
            indices(&whole)[12..],
            [1_011, 1_016, 1_018, 1_011, 1_018, 1_013]
        );
        // a quad not shown (cut through, or gone): no bottom under it either
        let holed = Tile::new(5, [1, 2, 2, 1], |c, r| (c, r) != (2, 2), Some(under));
        assert_eq!(
            indices(&holed)[6..],
            [1_011, 1_016, 1_017, 1_011, 1_017, 1_012]
        );
    }

    #[test]
    fn a_preview_cut_right_through_draws_no_floor_and_is_closed_underneath() {
        // a slot cut right through columns 4 to 6, the left of it taken away
        let mut m = grid(11, 5);
        m.bottom = -10.0;
        for (i, h) in m.heights.iter_mut().enumerate() {
            match i % 11 {
                0..4 => *h = f64::NAN,
                4..7 => *h = -10.0,
                _ => {}
            }
        }
        let view = Rect::from_min_size(Pos2::ZERO, eframe::egui::vec2(1000.0, 1000.0));
        let mut t = Tiled::new(&m, usize::MAX, true);
        let frame = t.frame(above(40.0), view, 1.0).unwrap();
        let all: Vec<u32> = frame.tiles.iter().flat_map(|t| indices(t)).collect();
        let cells = 11 * 5;
        let under = cells + 4 * (11 + 5) + 4;
        let (top, bottom): (Vec<u32>, Vec<u32>) = all.iter().partition(|&&i| i < under);
        let columns = |is: &[u32], from: u32| {
            let mut c: Vec<u32> = is.iter().map(|&i| (i - from) % 11).collect();
            c.sort_unstable();
            c.dedup();
            c
        };
        // the wall from 6 up to 7 and the part right of it; nothing over the slot
        assert_eq!(columns(&top, 0), [6, 7, 8, 9, 10]);
        assert_eq!(columns(&bottom, under), [6, 7, 8, 9, 10], "closed under it");
        assert_eq!(top.len(), bottom.len());
        // without the block's sides, the surface alone
        let mut t = Tiled::new(&m, usize::MAX, false);
        let frame = t.frame(above(40.0), view, 1.0).unwrap();
        assert!(
            frame
                .tiles
                .iter()
                .flat_map(|t| indices(t))
                .all(|i| i < under)
        );
    }
}
