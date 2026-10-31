//! Toolpath tiling: a job bigger than the machine cut in tiles, each moved to the same place
//! on the machine (Aspire's help: Toolpath Tiling Manager). Individual tiles split the job
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
    #[schemars(extend("x-widget" = "row"))]
    pub layout: Layout,
    /// Tile width (along X)
    #[schemars(title = "Tile width", extend("x-unit" = "mm"), range(min = 0))]
    pub width: f64,
    /// Tile height (along Y)
    #[schemars(title = "Tile height", extend("x-unit" = "mm"), range(min = 0))]
    pub height: f64,
    /// How far each tile's cuts run past its edges (at least the tool's radius for V-carving)
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub overlap: f64,
    /// Put the part-size tile first rather than last
    #[schemars(title = "Machine smallest tile first")]
    pub smallest_first: bool,
    /// Preview only this tile's toolpaths (tiles count from the bottom left, along X first;
    /// 0 shows every toolpath whole)
    #[schemars(title = "Active tile")]
    #[serde(default)]
    pub active_tile: u32,
    /// Draw the active tile's toolpaths where they sit in the job, not moved to the job's
    /// corner as they are saved
    #[schemars(title = "Draw in original position")]
    #[serde(default = "on")]
    pub original_position: bool,
}

fn on() -> bool {
    true
}

impl Default for Tiling {
    fn default() -> Self {
        Self {
            layout: Layout::Individual,
            width: 600.0,
            height: 600.0,
            overlap: 3.0,
            smallest_first: false,
            active_tile: 0,
            original_position: true,
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

    /// `doc` with each toolpath's moves cut to `tile` (with the overlap) and moved so the
    /// tile sits where the job's lower-left corner is.
    pub fn tiled(&self, doc: &Document, tile: Rect) -> Document {
        let area = doc.job.area();
        let mut out = doc.clone();
        for toolpath in out.toolpaths.values_mut() {
            toolpath.moves = self.tile_moves(&toolpath.moves, tile, tile.min - area.min);
        }
        out
    }

    /// The active tile, if one is picked and the job has it.
    pub fn active(&self, area: Rect) -> Option<Rect> {
        let n = usize::try_from(self.active_tile).ok()?.checked_sub(1)?;
        self.tiles(area).get(n).copied()
    }

    /// `moves` as the active tile previews them: cut to it, and moved to the job's corner
    /// unless drawn in their original position; `None` when no tile is active.
    pub fn preview(&self, moves: &[Move], area: Rect) -> Option<Vec<Move>> {
        let tile = self.active(area)?;
        let shift = if self.original_position {
            Point::ZERO
        } else {
            tile.min - area.min
        };
        Some(self.tile_moves(moves, tile, shift))
    }

    /// `moves` cut to `tile` (with the overlap) and moved back by `shift`.
    fn tile_moves(&self, moves: &[Move], tile: Rect, shift: Point) -> Vec<Move> {
        let safe = moves.iter().map(|m| m.to.z).fold(f64::MIN, f64::max);
        clip(moves, self.reach(tile), safe)
            .into_iter()
            .map(|m| Move {
                to: m.to - DVec3::new(shift.x, shift.y, 0.0),
                ..m
            })
            .collect()
    }
}

/// The parts of `moves` over `area`, joined by rapids at `safe` height and plunges down.
fn clip(moves: &[Move], area: Rect, safe: f64) -> Vec<Move> {
    let mut out: Vec<Move> = vec![];
    let mut at: Option<DVec3> = None;
    for pair in moves.windows(2) {
        let (a, b) = (pair[0].to, pair[1].to);
        let Some((t0, t1)) = clip_segment(a, b, area) else {
            at = None;
            continue;
        };
        let (enter, leave) = (a.lerp(b, t0), a.lerp(b, t1));
        if at.is_none_or(|p| p.distance(enter) > 1e-9) {
            out.push(Move {
                kind: MoveKind::Rapid,
                to: DVec3::new(enter.x, enter.y, safe),
            });
            out.push(Move {
                kind: MoveKind::Plunge,
                to: enter,
            });
        }
        out.push(Move {
            kind: pair[1].kind,
            to: leave,
        });
        at = (t1 >= 1.0).then_some(leave);
    }
    if let Some(last) = out.last().map(|m| m.to) {
        out.push(Move {
            kind: MoveKind::Rapid,
            to: DVec3::new(last.x, last.y, safe),
        });
    }
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
    fn the_active_tile_previews_its_own_moves_in_place_or_at_the_corner() {
        let cut = |x: f64| Move {
            kind: MoveKind::Cut,
            to: DVec3::new(x, 100.0, -2.0),
        };
        let moves = [cut(100.0), cut(700.0)];
        let mut t = Tiling {
            layout: Layout::FeedX,
            width: 400.0,
            overlap: 0.0,
            ..Tiling::default()
        };
        assert_eq!(t.preview(&moves, area()), None, "no tile active");
        t.active_tile = 2;
        let xs = |t: &Tiling| -> Vec<f64> {
            t.preview(&moves, area())
                .unwrap()
                .iter()
                .filter(|m| m.kind == MoveKind::Cut)
                .map(|m| m.to.x)
                .collect()
        };
        assert_eq!(xs(&t), [700.0], "where it sits in the job");
        t.original_position = false;
        assert_eq!(xs(&t), [300.0], "moved to the corner, as saved");
        t.active_tile = 9;
        assert_eq!(t.preview(&moves, area()), None, "no such tile");
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
        let clipped = clip(&moves, t.reach(second), 5.0);
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
