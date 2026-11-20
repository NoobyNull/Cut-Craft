//! How finely a toolpath preview is worked out: the Performance ◀ ▶ Detail slider (its
//! default in Options ▸ 3D View, the "now" one on the Preview tab), the quick picks in
//! Toolpaths ▸ Preview Simulation Quality, and the part of the sheet a preview covers.
//!
//! A preview spends its points where the toolpaths cut (plus a margin), not on the whole
//! sheet, so a small part on a big sheet is drawn as finely as it would be on its own. The
//! slider sets the finest spacing and the most points; the graphics card's own limits cap
//! them too. The rest of the sheet is uncut and drawn flat around it.

use cut_craft_core::document::Job;
use cut_craft_core::geometry::{Point, Rect};
use cut_craft_core::toolpath::{MoveKind, Toolpath};

/// Where the slider starts, until Options ▸ 3D View says otherwise.
pub const DEFAULT_PERCENT: u8 = 50;

/// Toolpaths ▸ Preview Simulation Quality's choices: each sets the slider here.
pub const QUICK_PICKS: [(&str, u8); 5] = [
    ("preview_quality_1", 20),
    ("preview_quality_2", 40),
    ("preview_quality_4", 60),
    ("preview_quality_8", 80),
    ("preview_quality_25", 100),
];

/// Millimetres between points at 0%, 50% and 100%.
const SPACING: [f64; 3] = [0.5, 0.1, 0.025];

/// The most points a preview takes at 0%, 50% and 100%, before the card's own limit.
const POINTS: [f64; 3] = [1.0e6, 4.0e6, 16.0e6];

/// How far past what a bit reaches the preview reaches too, mm: the cut's edges are seen
/// against uncut material.
pub const MARGIN: f64 = 5.0;

/// Bytes a preview point takes, near enough: its height, mark and paint as worked out, and
/// its height, paint and triangles on the graphics card.
pub const BYTES_PER_POINT: usize = 48;

/// `percent` along `[at 0, at 50, at 100]`, evenly on a log scale within each half.
fn along(percent: u8, [low, mid, high]: [f64; 3]) -> f64 {
    let t = f64::from(percent.min(100)) / 50.0;
    let (from, to, t) = if t <= 1.0 {
        (low, mid, t)
    } else {
        (mid, high, t - 1.0)
    };
    from * (to / from).powf(t)
}

/// The finest spacing between points at `percent`, mm.
pub fn finest(percent: u8) -> f64 {
    along(percent, SPACING)
}

/// The most points a preview takes at `percent`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a point count"
)]
pub fn most_points(percent: u8) -> usize {
    along(percent, POINTS).round() as usize
}

/// The least percent whose finest spacing is `spacing` or finer; `None` when even 100%
/// is coarser.
pub fn percent_for(spacing: f64) -> Option<u8> {
    (0..=100u8).find(|&p| finest(p) <= spacing + 1e-12)
}

/// What the toolpaths cut, seen from above, with what their bits reach and [`MARGIN`]
/// round it, inside the job's block; `None` when they cut nothing (or there are none).
pub fn cut_area<'a>(job: &Job, toolpaths: impl IntoIterator<Item = &'a Toolpath>) -> Option<Rect> {
    let mut found: Option<(Point, Point)> = None;
    let mut take = |p: Point, reach: f64| {
        let (lo, hi) = (p - Point::splat(reach), p + Point::splat(reach));
        found = Some(match found {
            Some((a, b)) => (a.min(lo), b.max(hi)),
            None => (lo, hi),
        });
    };
    for toolpath in toolpaths {
        let reach = toolpath.tool.reach().max(toolpath.tool.diameter / 2.0) + MARGIN;
        let mut from: Option<Point> = None;
        for m in &toolpath.moves {
            let at = m.to.truncate();
            if m.kind != MoveKind::Rapid {
                take(at, reach);
                if let Some(from) = from {
                    take(from, reach);
                }
            }
            from = Some(at);
        }
    }
    let (lo, hi) = found?;
    let area = job.area();
    let (lo, hi) = (lo.max(area.min), hi.min(area.max));
    (lo.x < hi.x && lo.y < hi.y).then_some(Rect { min: lo, max: hi })
}

/// Why a preview is coarser than the slider's finest spacing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Limit {
    /// The slider's most points: further right gives more.
    Slider,
    /// The graphics card takes no more.
    Card,
}

/// The grid a preview is worked out on: the part of the block, its spacing, and what (if
/// anything) kept it coarser than the slider asked.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grid {
    pub area: Rect,
    pub cell: f64,
    pub limit: Option<Limit>,
}

impl Grid {
    /// Points across and up.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a point count"
    )]
    pub fn points(&self) -> (usize, usize) {
        let count = |length: f64| ((length / self.cell).round() as usize).max(1);
        (
            count(self.area.max.x - self.area.min.x),
            count(self.area.max.y - self.area.min.y),
        )
    }

    /// About how much memory it takes, megabytes.
    pub fn megabytes(&self) -> usize {
        let (columns, rows) = self.points();
        (columns * rows * BYTES_PER_POINT).div_ceil(1_000_000)
    }
}

/// The grid a preview at `percent` is worked out on: over `cut` (else the whole block), as
/// finely as the slider says, unless that takes more points than it or the card (`card`,
/// when known) allow. Its edges fall on steps of its spacing from the block's corner, so a
/// grid made later at the same spacing lines up with it.
pub fn grid(job: &Job, cut: Option<Rect>, percent: u8, card: Option<usize>) -> Grid {
    let whole = job.area();
    let area = cut.unwrap_or(whole);
    let (width, height) = (area.max.x - area.min.x, area.max.y - area.min.y);
    let slider = most_points(percent);
    let most = card.map_or(slider, |c| c.min(slider)).max(1);
    #[expect(clippy::cast_precision_loss, reason = "a point count")]
    let needs = (width * height / most as f64).sqrt();
    let finest = finest(percent);
    let (cell, limit) = if needs > finest {
        let by = if card.is_some_and(|c| c < slider) {
            Limit::Card
        } else {
            Limit::Slider
        };
        (needs, Some(by))
    } else {
        (finest, None)
    };
    // whole steps out from the block's corner, never past its far edges
    let corner = whole.min;
    let snap_down = |v: Point| corner + ((v - corner) / cell).floor() * cell;
    let snap_up = |v: Point| corner + ((v - corner) / cell).ceil() * cell;
    let far = corner + ((whole.max - corner) / cell).round() * cell;
    let lo = snap_down(area.min).max(corner);
    let hi = snap_up(area.max).min(far);
    Grid {
        area: Rect { min: lo, max: hi },
        cell,
        limit,
    }
}

/// The slider's hover text: what `grid` means for this job.
pub fn about(grid: &Grid, units: cut_craft_core::document::Units) -> String {
    let (w, h) = (
        grid.area.max.x - grid.area.min.x,
        grid.area.max.y - grid.area.min.y,
    );
    format!(
        "Faster on the left, finer on the right. Now: about {} between points, over the \
         {} × {} where the toolpaths cut, using up to {} MB. Takes effect when you let go; \
         Options ▸ 3D View keeps the default",
        units.length(grid.cell, 3),
        units.length(w, 0),
        units.length(h, 0),
        grid.megabytes(),
    )
}

/// The red note under the slider when the preview is coarser than it asks, saying why and
/// what to do; `None` when it is as fine as asked.
pub fn limit_note(grid: &Grid, units: cut_craft_core::document::Units) -> Option<String> {
    let spacing = units.length(grid.cell, 3);
    Some(match grid.limit? {
        Limit::Slider => format!(
            "⚠ The toolpaths cover a big area, so the preview is drawn about {spacing} \
             apart to stay quick. Slide toward More detail for finer, if this computer can \
             take it"
        ),
        Limit::Card => format!(
            "⚠ This computer's graphics card takes no more points, so the preview is drawn \
             about {spacing} apart. The toolpaths and their G-code are not affected"
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::glam::DVec3;
    use cut_craft_core::toolpath::{Move, Tool};

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9 * b.abs().max(1.0)
    }

    #[test]
    fn the_slider_goes_from_half_a_millimetre_to_a_fortieth() {
        assert!(close(finest(0), 0.5));
        assert!(close(finest(50), 0.1));
        assert!(close(finest(100), 0.025));
        assert!(finest(25) < 0.5 && finest(25) > 0.1);
        for p in 0..100 {
            assert!(finest(p + 1) < finest(p), "finer at {}", p + 1);
            assert!(most_points(p + 1) >= most_points(p), "more at {}", p + 1);
        }
        assert_eq!(most_points(0), 1_000_000);
        assert_eq!(most_points(50), 4_000_000);
        assert_eq!(most_points(100), 16_000_000);
    }

    #[test]
    fn the_quick_picks_climb_and_the_default_is_the_middle() {
        assert_eq!(DEFAULT_PERCENT, 50);
        assert!(QUICK_PICKS.windows(2).all(|w| w[0].1 < w[1].1));
        assert_eq!(QUICK_PICKS[4], ("preview_quality_25", 100));
    }

    #[test]
    fn the_percent_for_a_spacing_is_the_least_that_reaches_it() {
        assert_eq!(percent_for(0.5), Some(0));
        assert_eq!(percent_for(0.1), Some(50));
        assert_eq!(percent_for(0.025), Some(100));
        assert_eq!(percent_for(0.01), None);
        let p = percent_for(0.2).unwrap();
        assert!(finest(p) <= 0.2 && finest(p - 1) > 0.2);
    }

    fn cut(points: &[(f64, f64, f64, MoveKind)], diameter: f64) -> Toolpath {
        Toolpath {
            name: "t".into(),
            strategy: "t".into(),
            params: serde_json::Value::Null,
            tool: Tool {
                diameter,
                ..Tool::default()
            },
            vectors: vec![],
            moves: points
                .iter()
                .map(|&(x, y, z, kind)| Move {
                    kind,
                    to: DVec3::new(x, y, z),
                })
                .collect(),
            visible: true,
            power: vec![],
            stale: false,
            group: None,
            drill_bank: None,
            fallbacks: vec![],
        }
    }

    #[test]
    fn the_cut_area_is_where_bits_cut_with_their_reach_and_a_margin() {
        let job = Job::default(); // 600 x 400
        let o = job.area().min;
        let path = cut(
            &[
                (o.x + 300.0, o.y + 300.0, 5.0, MoveKind::Rapid),
                (o.x + 100.0, o.y + 100.0, 5.0, MoveKind::Rapid),
                (o.x + 100.0, o.y + 100.0, -3.0, MoveKind::Plunge),
                (o.x + 200.0, o.y + 150.0, -3.0, MoveKind::Cut),
                (o.x + 500.0, o.y + 350.0, 5.0, MoveKind::Rapid),
            ],
            6.0,
        );
        let area = cut_area(&job, [&path]).unwrap();
        let reach = 3.0 + MARGIN;
        assert!(close(area.min.x, o.x + 100.0 - reach));
        assert!(close(area.min.y, o.y + 100.0 - reach));
        assert!(close(area.max.x, o.x + 200.0 + reach));
        assert!(close(area.max.y, o.y + 150.0 + reach));
        // only rapids, or nothing: no area
        let rapids = cut(&[(o.x, o.y, 5.0, MoveKind::Rapid)], 6.0);
        assert_eq!(cut_area(&job, [&rapids]), None);
        assert_eq!(cut_area(&job, []), None);
        // never past the block's edges
        let edge = cut(&[(o.x, o.y, -1.0, MoveKind::Cut)], 6.0);
        let area = cut_area(&job, [&edge]).unwrap();
        assert!(close(area.min.x, o.x) && close(area.min.y, o.y));
    }

    #[test]
    fn a_small_part_on_a_big_sheet_gets_the_finest_spacing_on_steps_from_the_corner() {
        // the issue's case: a 206 x 142 part on a 600 x 400 sheet
        let job = Job::default();
        let o = job.area().min;
        let part = Rect {
            min: o + Point::new(20.3, 30.7),
            max: o + Point::new(236.3, 182.7),
        };
        let g = grid(&job, Some(part), 50, None);
        assert!(close(g.cell, 0.1), "{g:?}");
        assert_eq!(g.limit, None);
        assert!(g.area.min.x <= part.min.x && g.area.max.y >= part.max.y);
        let steps = (g.area.min - o) / g.cell;
        assert!(close(steps.x, steps.x.round()) && close(steps.y, steps.y.round()));
        let (c, r) = g.points();
        assert!(c * r <= most_points(50));
        // five times finer than the old 1,200 points across the whole sheet
        assert!(g.cell * 5.0 <= 600.0 / 1200.0 + 1e-9);
    }

    #[test]
    fn a_big_area_is_kept_to_the_slider_s_points_and_then_the_card_s() {
        let job = Job::default(); // 600 x 400
        let g = grid(&job, None, 50, None);
        assert_eq!(g.limit, Some(Limit::Slider));
        let (c, r) = g.points();
        assert!(c * r <= most_points(50) * 101 / 100, "{c} x {r}");
        assert!(g.cell > 0.1);
        // a card that takes fewer points than the slider: coarser, and said so
        let small = grid(&job, None, 50, Some(1_000_000));
        assert_eq!(small.limit, Some(Limit::Card));
        assert!(small.cell > g.cell);
        // never past the block
        assert!(g.area.max.x <= job.area().max.x + g.cell);
        assert!(
            limit_note(&small, cut_craft_core::document::Units::Mm)
                .unwrap()
                .contains("graphics card")
        );
        let fine = grid(&job, Some(job.area()), 0, Some(10_000_000));
        assert!(close(fine.cell, 0.5), "600 x 400 at half a millimetre fits");
        assert_eq!(limit_note(&fine, cut_craft_core::document::Units::Mm), None);
    }

    #[test]
    fn the_hover_text_says_the_spacing_area_and_memory() {
        let job = Job::default();
        let g = grid(&job, None, 0, None);
        let text = about(&g, cut_craft_core::document::Units::Mm);
        assert!(text.contains("0.500 mm"), "{text}");
        assert!(text.contains("600 mm × 400 mm"), "{text}");
        assert!(text.contains("MB"), "{text}");
    }
}
