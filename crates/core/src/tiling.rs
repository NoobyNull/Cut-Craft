//! Toolpath tiling: a job bigger than the machine cut in tiles, each moved to the same place
//! on the machine (help: Toolpath Tiling Manager). Individual tiles split the job
//! both ways; feed-through splits it one way, the material fed through between tiles.

use crate::document::Document;
use crate::geometry::{Point, Rect};
use crate::toolpath::{Move, MoveKind};
use glam::DVec3;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Layout {
    /// Separate pieces, split in X and Y
    #[default]
    #[schemars(title = "Individual Tiles")]
    Individual,
    /// One strip fed through the machine along X
    #[schemars(title = "Feed-through in X")]
    FeedX,
    /// One strip fed through the machine along Y
    #[schemars(title = "Feed-through in Y")]
    FeedY,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Tiling {
    /// Separate tiles, or one strip fed through the machine
    #[schemars(extend("x-widget" = "row"))]
    pub layout: Layout,
    /// Tile width (along X)
    #[schemars(title = "Tile width", extend("x-unit" = "mm"), range(min = 0))]
    pub width: f64,
    /// Tile height (along Y)
    #[schemars(title = "Tile height", extend("x-unit" = "mm"), range(min = 0))]
    pub height: f64,
    /// How far each tile's cuts run past its edges (at least the tool's radius for V-carving).
    /// Each tile file's zero is at the start of its overlap, so a file covers the tile and
    /// the overlap on both sides, and nothing in it goes below zero. Laser toolpaths have
    /// none: their tiles meet edge to edge, so no seam burns twice
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub overlap: f64,
    /// Put the part-size tile first rather than last
    #[schemars(title = "Machine smallest tile first")]
    pub smallest_first: bool,
    /// The tile previewed alone, counting from the bottom left along X (double-click one in
    /// the 2D view while this form is open); 0 previews the whole job
    #[serde(default)]
    #[schemars(title = "Active tile", range(min = 0))]
    pub active: u32,
    /// Preview the active tile where it sits in the finished piece, not moved to the
    /// machine's corner as it is cut
    #[serde(default)]
    #[schemars(title = "Draw toolpaths in original position")]
    pub original_position: bool,
}

impl Default for Tiling {
    fn default() -> Self {
        Self {
            layout: Layout::Individual,
            width: 600.0,
            height: 600.0,
            overlap: 3.0,
            smallest_first: false,
            active: 0,
            original_position: false,
        }
    }
}

/// `lo..hi` cut into lengths of `size`, the part-size one last (or first).
fn spans(lo: f64, hi: f64, size: f64, smallest_first: bool) -> Vec<(f64, f64)> {
    if size <= 0.0 || hi - lo <= size {
        return vec![(lo, hi)];
    }
    let mut lengths = vec![];
    let mut left = hi - lo;
    while left > 1e-9 {
        lengths.push(size.min(left));
        left -= size;
    }
    if smallest_first {
        lengths.rotate_right(1);
    }
    let mut at = lo;
    lengths
        .into_iter()
        .map(|l| {
            let span = (at, at + l);
            at += l;
            span
        })
        .collect()
}

impl Tiling {
    /// The tiles over `area`, from the bottom left, along X first.
    pub fn tiles(&self, area: Rect) -> Vec<Rect> {
        let full = |lo: f64, hi: f64| vec![(lo, hi)];
        let xs = match self.layout {
            Layout::FeedY => full(area.min.x, area.max.x),
            _ => spans(area.min.x, area.max.x, self.width, self.smallest_first),
        };
        let ys = match self.layout {
            Layout::FeedX => full(area.min.y, area.max.y),
            _ => spans(area.min.y, area.max.y, self.height, self.smallest_first),
        };
        ys.iter()
            .flat_map(|&(y0, y1)| {
                xs.iter()
                    .map(move |&(x0, x1)| Rect::new(Point::new(x0, y0), Point::new(x1, y1)))
            })
            .collect()
    }

    /// `tile` with its overlap: both ways for individual tiles, along the feed otherwise.
    pub fn reach(&self, tile: Rect) -> Rect {
        let o = self.overlap;
        let grow = match self.layout {
            Layout::Individual => Point::new(o, o),
            Layout::FeedX => Point::new(o, 0.0),
            Layout::FeedY => Point::new(0.0, o),
        };
        Rect::new(tile.min - grow, tile.max + grow)
    }

    /// What a file of `tile` covers on the machine: the tile with its overlap, whose start
    /// is the file's zero.
    pub fn file_size(&self, tile: Rect) -> Point {
        self.reach(tile).size()
    }

    /// The biggest any tile's file covers on the machine (see [`Self::file_size`]).
    pub fn largest_file(&self, area: Rect) -> Point {
        self.tiles(area)
            .into_iter()
            .map(|t| self.file_size(t))
            .fold(Point::ZERO, |a, b| Point::new(a.x.max(b.x), a.y.max(b.y)))
    }

    /// The active tile, if one is picked and there is one of that number.
    pub fn active_tile(&self, area: Rect) -> Option<Rect> {
        let i = usize::try_from(self.active.checked_sub(1)?).ok()?;
        self.tiles(area).get(i).copied()
    }

    /// What a preview cuts while a tile is active: `doc` with its toolpaths cut to the tile,
    /// moved to the machine's corner unless they are drawn in their original position.
    pub fn preview(&self, doc: &Document) -> Option<Document> {
        let tile = self.active_tile(doc.job.area())?;
        Some(self.cut_to(doc, tile, !self.original_position))
    }

    /// `doc` with each toolpath's moves cut to `tile` (with the overlap) and moved so the
    /// start of the tile's overlap sits where the job's lower-left corner is: nothing in a
    /// tile file goes below zero for its overlap.
    pub fn tiled(&self, doc: &Document, tile: Rect) -> Document {
        self.cut_to(doc, tile, true)
    }

    /// `doc` with each toolpath's moves cut to `tile` (with the overlap), and when `moved`
    /// moved so the start of the tile's overlap sits where the job's lower-left corner is.
    fn cut_to(&self, doc: &Document, tile: Rect, moved: bool) -> Document {
        let area = doc.job.area();
        let reach = self.reach(tile);
        let shift = if moved {
            reach.min - area.min
        } else {
            Point::ZERO
        };
        let by = DVec3::new(shift.x, shift.y, 0.0);
        let mut out = doc.clone();
        for toolpath in out.toolpaths.values_mut() {
            if let Some(bank) = toolpath.drill_bank.take() {
                // a drill bank plunges the whole head at once: a hit is in the tile whose
                // area holds its reference drill (not the overlap, or it drills twice)
                let mut drilled = vec![];
                let mut hits = vec![];
                for hit in &bank.hits {
                    if !past_edges(tile, tile, area).contains(hit.head) {
                        continue;
                    }
                    let from = drilled.len();
                    let own = toolpath
                        .moves
                        .get(hit.start..hit.end.min(toolpath.moves.len()))
                        .unwrap_or_default();
                    drilled.extend(own.iter().map(|m| Move {
                        to: m.to - by,
                        ..*m
                    }));
                    hits.push(crate::drill_bank::BankHit {
                        start: from,
                        end: drilled.len(),
                        head: hit.head - shift,
                        drills: hit.drills.clone(),
                    });
                }
                toolpath.moves = drilled;
                toolpath.power.clear();
                toolpath.drill_bank = Some(crate::drill_bank::BankToolpath { hits, ..bank });
                continue;
            }
            let safe = toolpath
                .moves
                .iter()
                .map(|m| m.to.z)
                .fold(f64::MIN, f64::max);
            // a laser's tiles meet edge to edge: run on into the overlap, it would burn the
            // seam twice, once from each side
            let over = if toolpath.power.is_empty() {
                reach
            } else {
                tile
            };
            let clipped = clip(&toolpath.moves, past_edges(over, tile, area), safe);
            // a laser's power goes with each move it was set for; none on the moves added
            if !toolpath.power.is_empty() {
                toolpath.power = clipped
                    .iter()
                    .map(|&(_, from)| {
                        from.and_then(|i| toolpath.power.get(i))
                            .copied()
                            .unwrap_or(0.0)
                    })
                    .collect();
            }
            toolpath.moves = clipped
                .into_iter()
                .map(|(m, _)| Move { to: m.to - by, ..m })
                .collect();
        }
        // a toolpath with nothing in the tile is left out, so its tool is not changed to
        // and its spindle not started for nothing
        out.toolpaths
            .retain(|_, t| t.moves.iter().any(|m| m.kind != MoveKind::Rapid));
        out
    }
}

/// `over` (`tile`, or it with its overlap) run on out without end wherever `tile` is at an
/// edge of the material `area`: a cut past the material's edge (an outside profile's
/// centreline, a lead-in) is in the tile at that edge, never left out of every tile.
fn past_edges(over: Rect, tile: Rect, area: Rect) -> Rect {
    const FAR: f64 = 1e9;
    let at = |a: f64, b: f64| (a - b).abs() < 1e-9;
    let out = |edge: bool, far: f64, own: f64| if edge { far } else { own };
    Rect::new(
        Point::new(
            out(at(tile.min.x, area.min.x), -FAR, over.min.x),
            out(at(tile.min.y, area.min.y), -FAR, over.min.y),
        ),
        Point::new(
            out(at(tile.max.x, area.max.x), FAR, over.max.x),
            out(at(tile.max.y, area.max.y), FAR, over.max.y),
        ),
    )
}

/// The parts of `moves` over `area`, joined by rapids at `safe` height and plunges down:
/// straight up where a part leaves, across, and straight down where the next comes in.
/// Each with the index in `moves` of the move it is (part of), `None` for those added.
fn clip(moves: &[Move], area: Rect, safe: f64) -> Vec<(Move, Option<usize>)> {
    let mut out: Vec<(Move, Option<usize>)> = vec![];
    let mut at: Option<DVec3> = None;
    let rise = |out: &mut Vec<(Move, Option<usize>)>| {
        if let Some(last) = out.last().map(|(m, _)| m.to).filter(|p| p.z < safe) {
            out.push((
                Move {
                    kind: MoveKind::Rapid,
                    to: DVec3::new(last.x, last.y, safe),
                },
                None,
            ));
        }
    };
    for (i, pair) in moves.windows(2).enumerate() {
        let (a, b) = (pair[0].to, pair[1].to);
        let Some((t0, t1)) = clip_segment(a, b, area) else {
            at = None;
            continue;
        };
        let (enter, leave) = (a.lerp(b, t0), a.lerp(b, t1));
        if at.is_none_or(|p| p.distance(enter) > 1e-9) {
            // never across at the depth the last part left at
            rise(&mut out);
            out.push((
                Move {
                    kind: MoveKind::Rapid,
                    to: DVec3::new(enter.x, enter.y, safe),
                },
                None,
            ));
            out.push((
                Move {
                    kind: MoveKind::Plunge,
                    to: enter,
                },
                None,
            ));
        }
        out.push((
            Move {
                kind: pair[1].kind,
                to: leave,
            },
            Some(i + 1),
        ));
        at = (t1 >= 1.0).then_some(leave);
    }
    rise(&mut out);
    out
}

/// The part of `a`–`b` over `area` in plan, as fractions along it (Liang–Barsky).
fn clip_segment(a: DVec3, b: DVec3, area: Rect) -> Option<(f64, f64)> {
    let d = b - a;
    let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
    for (p, q) in [
        (-d.x, a.x - area.min.x),
        (d.x, area.max.x - a.x),
        (-d.y, a.y - area.min.y),
        (d.y, area.max.y - a.y),
    ] {
        if p.abs() < 1e-12 {
            if q < 0.0 {
                return None;
            }
        } else {
            let r = q / p;
            if p < 0.0 {
                t0 = t0.max(r);
            } else {
                t1 = t1.min(r);
            }
        }
    }
    (t0 <= t1).then_some((t0, t1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area() -> Rect {
        Rect::new(Point::ZERO, Point::new(1000.0, 500.0))
    }

    #[test]
    fn an_active_tile_previews_alone_at_the_corner_or_where_it_sits() {
        use crate::testing::with_toolpath;
        let mut doc = with_toolpath();
        let area = doc.job.area();
        // one cut right across the job, 1 deep
        let y = area.center().y;
        for t in doc.toolpaths.values_mut() {
            t.moves = [
                (area.min.x + 1.0, 5.0),
                (area.min.x + 1.0, -1.0),
                (area.max.x - 1.0, -1.0),
            ]
            .iter()
            .map(|&(x, z)| Move {
                kind: MoveKind::Cut,
                to: DVec3::new(x, y, z),
            })
            .collect();
        }
        let tiling = Tiling {
            width: area.size().x / 2.0,
            height: area.size().y,
            overlap: 0.0,
            ..Tiling::default()
        };
        assert!(
            tiling.preview(&doc).is_none(),
            "no tile active: the whole job"
        );
        let second = Tiling {
            active: 2,
            ..tiling
        };
        doc.tiling = Some(second);
        let tile = second.active_tile(area).unwrap();
        let cut_moves = |d: &Document| -> Vec<DVec3> {
            d.toolpaths
                .values()
                .flat_map(|t| t.moves.iter().filter(|m| m.to.z < 0.0).map(|m| m.to))
                .collect()
        };
        let at_corner = second.preview(&doc).unwrap();
        assert!(
            cut_moves(&at_corner)
                .iter()
                .all(|p| p.x <= area.min.x + tile.size().x + 1e-9)
        );
        let original = Tiling {
            original_position: true,
            ..second
        };
        let in_place = original.preview(&doc).unwrap();
        assert!(
            !cut_moves(&in_place).is_empty(),
            "the toolpath reaches the second tile"
        );
        assert!(
            cut_moves(&in_place)
                .iter()
                .all(|p| p.x >= tile.min.x - 1e-9)
        );
        assert_ne!(cut_moves(&in_place), cut_moves(&at_corner), "moved, or not");
        assert!(
            Tiling {
                active: 9,
                ..tiling
            }
            .preview(&doc)
            .is_none(),
            "no ninth tile"
        );
    }

    #[test]
    fn a_cut_that_leaves_the_tile_lifts_straight_up_before_moving_across() {
        // a rectangle's pass crossing the tile edge: out at the bottom, back in at the top
        let at = |kind, x, y, z| Move {
            kind,
            to: DVec3::new(x, y, z),
        };
        let moves = vec![
            at(MoveKind::Rapid, 300.0, 100.0, 5.0),
            at(MoveKind::Plunge, 300.0, 100.0, -6.0),
            at(MoveKind::Cut, 500.0, 100.0, -6.0),
            at(MoveKind::Cut, 500.0, 300.0, -6.0),
            at(MoveKind::Cut, 300.0, 300.0, -6.0),
            at(MoveKind::Cut, 300.0, 100.0, -6.0),
        ];
        let tile = Rect::new(Point::ZERO, Point::new(403.0, 500.0));
        let clipped: Vec<Move> = clip(&moves, tile, 5.0)
            .into_iter()
            .map(|(m, _)| m)
            .collect();
        for pair in clipped.windows(2) {
            if pair[1].kind == MoveKind::Rapid && pair[0].to.z < 5.0 {
                let (a, b) = (pair[0].to, pair[1].to);
                assert!(
                    (a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9,
                    "a rapid out of a cut goes straight up first: {a} to {b}"
                );
            }
        }
        assert_eq!(
            clipped.last().unwrap().to.z,
            5.0,
            "ends up at the safe height"
        );
    }

    #[test]
    fn a_tile_keeps_each_moves_laser_power_and_only_its_own_drill_hits() {
        use crate::testing::with_toolpath;
        let mut doc = with_toolpath();
        let area = doc.job.area();
        let y = area.center().y;
        let (left, right) = (area.min.x + 10.0, area.max.x - 10.0);
        let t = doc.toolpaths.values_mut().next().unwrap();
        let at = |kind, x: f64, z| Move {
            kind,
            to: DVec3::new(x, y, z),
        };
        t.moves = vec![
            at(MoveKind::Rapid, left, 5.0),
            at(MoveKind::Plunge, left, -1.0),
            at(MoveKind::Cut, right, -1.0),
        ];
        t.power = vec![0.0, 0.2, 0.9];
        let tiling = Tiling {
            width: area.size().x / 2.0,
            height: area.size().y,
            overlap: 0.0,
            ..Tiling::default()
        };
        let tiles = tiling.tiles(area);
        let second = tiling.tiled(&doc, tiles[1]);
        let t2 = second.toolpaths.values().next().unwrap();
        assert_eq!(t2.power.len(), t2.moves.len());
        for (m, &p) in t2.moves.iter().zip(&t2.power) {
            let want = if m.kind == MoveKind::Cut { 0.9 } else { 0.0 };
            assert!((p - want).abs() < 1e-9, "{m:?} at {p}");
        }
        // a drill bank: one hit in each half
        let t = doc.toolpaths.values_mut().next().unwrap();
        t.power.clear();
        let hole = |x: f64| {
            [
                at(MoveKind::Rapid, x, 5.0),
                at(MoveKind::Plunge, x, -3.0),
                at(MoveKind::Rapid, x, 5.0),
            ]
        };
        t.moves = hole(left).into_iter().chain(hole(right)).collect();
        t.drill_bank = Some(crate::drill_bank::BankToolpath {
            hits: vec![
                crate::drill_bank::BankHit {
                    start: 0,
                    end: 3,
                    head: Point::new(left, y),
                    drills: vec![],
                },
                crate::drill_bank::BankHit {
                    start: 3,
                    end: 6,
                    head: Point::new(right, y),
                    drills: vec![],
                },
            ],
            ..Default::default()
        });
        let second = tiling.tiled(&doc, tiles[1]);
        let t2 = second.toolpaths.values().next().unwrap();
        let bank = t2.drill_bank.as_ref().unwrap();
        assert_eq!(bank.hits.len(), 1, "only the hit in this tile");
        let shift = tiles[1].min - area.min;
        assert_eq!(bank.hits[0].head, Point::new(right, y) - shift);
        assert_eq!((bank.hits[0].start, bank.hits[0].end), (0, 3));
        assert!((t2.moves[1].to.x - (right - shift.x)).abs() < 1e-9);
    }

    #[test]
    fn a_laser_toolpath_is_burned_once_along_a_seam_with_no_overlap() {
        use crate::testing::with_toolpath;
        let mut doc = with_toolpath();
        let area = doc.job.area();
        let y = area.center().y;
        let t = doc.toolpaths.values_mut().next().unwrap();
        let at = |kind, x: f64, z| Move {
            kind,
            to: DVec3::new(x, y, z),
        };
        t.moves = vec![
            at(MoveKind::Rapid, area.min.x + 10.0, 5.0),
            at(MoveKind::Plunge, area.min.x + 10.0, 0.0),
            at(MoveKind::Cut, area.max.x - 10.0, 0.0),
        ];
        t.power = vec![0.0, 0.0, 0.8];
        let tiling = Tiling {
            width: area.size().x / 2.0,
            height: area.size().y,
            overlap: 3.0,
            ..Tiling::default()
        };
        let tiles = tiling.tiles(area);
        let burned = |n: usize| -> f64 {
            let cut = tiling.cut_to(&doc, tiles[n], false);
            let t = cut.toolpaths.values().next().unwrap();
            t.moves
                .windows(2)
                .zip(&t.power[1..])
                .filter(|(_, p)| **p > 0.0)
                .map(|(pair, _)| pair[0].to.distance(pair[1].to))
                .sum()
        };
        let whole = area.size().x - 20.0;
        assert!(
            (burned(0) + burned(1) - whole).abs() < 1e-6,
            "the seam burned once: {} + {} of {whole}",
            burned(0),
            burned(1)
        );
        // a router's cut still runs on into the overlap
        doc.toolpaths.values_mut().next().unwrap().power.clear();
        let routed = tiling.cut_to(&doc, tiles[0], false);
        let far = routed
            .toolpaths
            .values()
            .next()
            .unwrap()
            .moves
            .iter()
            .map(|m| m.to.x);
        assert!((far.fold(f64::MIN, f64::max) - (tiles[0].max.x + 3.0)).abs() < 1e-9);
    }

    #[test]
    fn cuts_past_the_material_s_edge_go_in_the_tile_at_that_edge() {
        use crate::testing::with_toolpath;
        let mut doc = with_toolpath();
        let area = doc.job.area();
        // an outside profile's centreline, 5 mm out past the left and the top edges
        let cut = |x: f64, y: f64| Move {
            kind: MoveKind::Cut,
            to: DVec3::new(x, y, -1.0),
        };
        let (left, top) = (area.min.x - 5.0, area.max.y + 5.0);
        doc.toolpaths.values_mut().next().unwrap().moves = vec![
            cut(left, area.min.y + 10.0),
            cut(left, top),
            cut(area.max.x - 10.0, top),
        ];
        let cut_length = |tiling: &Tiling| -> f64 {
            tiling
                .tiles(area)
                .iter()
                .map(|&tile| {
                    let tiled = tiling.cut_to(&doc, tile, false);
                    // a tile with nothing to cut has no toolpath
                    tiled
                        .toolpaths
                        .values()
                        .flat_map(|t| t.moves.windows(2))
                        .filter(|pair| pair[1].kind == MoveKind::Cut)
                        .map(|pair| pair[0].to.distance(pair[1].to))
                        .sum::<f64>()
                })
                .sum()
        };
        let whole = (top - area.min.y - 10.0) + (area.max.x - 10.0 - left);
        for layout in [Layout::Individual, Layout::FeedX, Layout::FeedY] {
            let tiling = Tiling {
                layout,
                width: area.size().x / 2.0,
                height: area.size().y / 2.0,
                overlap: 0.0,
                ..Tiling::default()
            };
            assert!(
                (cut_length(&tiling) - whole).abs() < 1e-6,
                "{layout:?}: {} of {whole}",
                cut_length(&tiling)
            );
        }
    }

    #[test]
    fn each_tile_file_starts_at_its_overlap_so_nothing_goes_below_zero() {
        let mut doc = crate::testing::with_toolpath();
        let a = doc.job.area();
        let cut = |x: f64| Move {
            kind: MoveKind::Cut,
            to: DVec3::new(a.min.x + x, a.min.y + 10.0, -1.0),
        };
        // a cut along X from just before the second tile's start
        doc.toolpaths.values_mut().next().unwrap().moves = vec![cut(98.0), cut(140.0)];
        let t = Tiling {
            layout: Layout::FeedX,
            width: 100.0,
            overlap: 3.0,
            ..Tiling::default()
        };
        let second = t.tiles(a)[1];
        let tiled = t.tiled(&doc, second);
        let xs: Vec<f64> = tiled
            .toolpaths
            .values()
            .next()
            .unwrap()
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Cut)
            .map(|m| m.to.x - a.min.x)
            .collect();
        assert!(xs.iter().all(|&x| x >= -1e-9), "none below zero: {xs:?}");
        assert!(
            (xs[0] - 43.0).abs() < 1e-9,
            "the tile's start is at the overlap: {xs:?}"
        );
        assert_eq!(t.file_size(second), Point::new(106.0, a.size().y));
    }

    #[test]
    fn a_toolpath_with_nothing_in_a_tile_is_left_out_of_it() {
        let mut doc = crate::testing::with_toolpath();
        let a = doc.job.area();
        let cut = |x: f64| Move {
            kind: MoveKind::Cut,
            to: DVec3::new(a.min.x + x, a.min.y + 10.0, -1.0),
        };
        doc.toolpaths.values_mut().next().unwrap().moves = vec![cut(10.0), cut(40.0)];
        let t = Tiling {
            layout: Layout::FeedX,
            width: 100.0,
            overlap: 3.0,
            ..Tiling::default()
        };
        let tiles = t.tiles(a);
        assert_eq!(t.tiled(&doc, tiles[0]).toolpaths.len(), 1);
        assert!(
            t.tiled(&doc, tiles[2]).toolpaths.is_empty(),
            "nothing to cut there"
        );
    }

    #[test]
    fn individual_tiles_split_both_ways_the_part_tile_last() {
        let t = Tiling {
            width: 400.0,
            height: 300.0,
            ..Tiling::default()
        };
        let tiles = t.tiles(area());
        assert_eq!(tiles.len(), 3 * 2);
        assert_eq!(tiles[0], Rect::new(Point::ZERO, Point::new(400.0, 300.0)));
        assert_eq!(tiles[2].size().x, 200.0, "the part tile last");
        let first = Tiling {
            smallest_first: true,
            ..t
        };
        assert_eq!(first.tiles(area())[0].size(), Point::new(200.0, 200.0));
    }

    #[test]
    fn feed_through_splits_one_way_and_overlaps_along_it() {
        let t = Tiling {
            layout: Layout::FeedX,
            width: 400.0,
            ..Tiling::default()
        };
        let tiles = t.tiles(area());
        assert_eq!(tiles.len(), 3);
        assert!(tiles.iter().all(|r| (r.size().y - 500.0).abs() < 1e-9));
        let reach = t.reach(tiles[1]);
        assert_eq!((reach.min.x, reach.min.y), (397.0, 0.0));
    }

    #[test]
    fn a_tile_of_the_document_sits_at_the_jobs_corner() {
        let mut doc = crate::testing::with_toolpath();
        let a = doc.job.area();
        let cut = |x: f64| Move {
            kind: MoveKind::Cut,
            to: DVec3::new(a.min.x + x, a.min.y + 10.0, -1.0),
        };
        let toolpath = doc.toolpaths.values_mut().next().unwrap();
        toolpath.moves = vec![cut(110.0), cut(140.0)];
        let t = Tiling {
            layout: Layout::FeedX,
            width: 100.0,
            overlap: 0.0,
            ..Tiling::default()
        };
        let second = t.tiles(a)[1];
        let tiled = t.tiled(&doc, second);
        let moves = &tiled.toolpaths.values().next().unwrap().moves;
        let xs: Vec<f64> = moves
            .iter()
            .filter(|m| m.kind == MoveKind::Cut)
            .map(|m| m.to.x - a.min.x)
            .collect();
        assert_eq!(xs, [40.0], "110..140 is 10..40 into the second tile");
    }

    #[test]
    fn a_cut_across_tiles_is_split_and_each_piece_moved_to_the_origin() {
        let cut = |x: f64, y: f64| Move {
            kind: MoveKind::Cut,
            to: DVec3::new(x, y, -2.0),
        };
        let moves = [cut(100.0, 100.0), cut(700.0, 100.0)];
        let t = Tiling {
            layout: Layout::FeedX,
            width: 400.0,
            overlap: 0.0,
            ..Tiling::default()
        };
        let second = t.tiles(area())[1];
        let clipped: Vec<Move> = clip(&moves, t.reach(second), 5.0)
            .into_iter()
            .map(|(m, _)| m)
            .collect();
        let cuts: Vec<f64> = clipped
            .iter()
            .filter(|m| m.kind == MoveKind::Cut)
            .map(|m| m.to.x)
            .collect();
        assert_eq!(cuts, [700.0], "from 400 (entered) to 700");
        let plunge = clipped.iter().find(|m| m.kind == MoveKind::Plunge).unwrap();
        assert_eq!(plunge.to, DVec3::new(400.0, 100.0, -2.0));
        assert_eq!(clipped[0].to.z, 5.0, "a rapid at the safe height first");
        let outside = clip(
            &moves,
            Rect::new(Point::new(800.0, 0.0), Point::new(900.0, 50.0)),
            5.0,
        );
        assert_eq!(outside, vec![]);
    }
}
