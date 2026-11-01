//! Nest Parts: the selected vectors, as parts (vectors whose boxes overlap, or that are
//! grouped, stay together), laid out inside the job from a chosen corner, kept a tool's
//! width plus a clearance apart and a border in from the edge: by their boxes in rows, or
//! (True Shape) each where its own shape first fits, turned in steps, holes filled by
//! smaller parts, with as many copies of each as asked.

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::geometry::{Point, Rect};
use cut_craft_core::glam::DAffine2;
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct NestParts;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Corner {
    #[default]
    BottomLeft,
    BottomRight,
    TopLeft,
    TopRight,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    #[default]
    AlongX,
    AlongY,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// The cut-out tool: the least gap left between parts
    #[schemars(title = "Tool dia. (D)", extend("x-unit" = "mm"), range(min = 0))]
    pub tool_diameter: f64,
    /// Added to the tool diameter between parts, and to the border
    #[schemars(title = "Clearance (C)", extend("x-unit" = "mm"), range(min = 0))]
    pub clearance: f64,
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub border_gap: f64,
    /// Turn parts a quarter turn where that fits better
    #[schemars(title = "Rotate parts to find best fit")]
    pub rotate_parts: bool,
    /// Move the parts; otherwise copies are nested and the originals stay
    pub remove_original_parts: bool,
    #[schemars(extend("x-widget" = "corners"))]
    pub nest_from: Corner,
    #[schemars(extend("x-widget" = "row"))]
    pub nest_direction: Direction,
    /// Nest by the parts' own shapes, not their boxes: parts fit round each other and into
    /// each other's holes
    #[serde(default)]
    pub true_shape: bool,
    /// True shape: the steps parts are turned in to fit (0: quarter turns when rotating)
    #[serde(default)]
    #[schemars(
        title = "Rotation step",
        extend("x-unit" = "deg", "x-show-when" = {"true_shape": true}),
        range(min = 0, max = 180)
    )]
    pub rotation_step: f64,
    /// True shape: how many of each part to nest
    #[serde(default = "one")]
    #[schemars(
        title = "Copies of each part",
        extend("x-show-when" = {"true_shape": true}),
        range(min = 1, max = 500)
    )]
    pub copies: u32,
}

fn one() -> u32 {
    1
}

impl Default for Params {
    fn default() -> Self {
        Self {
            tool_diameter: 6.35,
            clearance: 1.0,
            border_gap: 5.0,
            rotate_parts: true,
            remove_original_parts: true,
            nest_from: Corner::BottomLeft,
            nest_direction: Direction::AlongX,
            true_shape: false,
            rotation_step: 0.0,
            copies: 1,
        }
    }
}

impl Command for NestParts {
    const ID: &'static str = "nest_parts";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Nest Parts",
            group: "Offset and Layout",
            icon: "drawing/weld_and_offset/vector_texture_fill.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if doc.selection.is_empty() {
            return Err(CommandError::NothingSelected);
        }
        let parts = parts(doc);
        if p.true_shape {
            return true_shape(doc, p, &parts);
        }
        let gap = p.tool_diameter + p.clearance;
        let edge = p.border_gap + p.clearance;
        let area = doc.job.area();
        let room = area.size() - Point::splat(2.0 * edge);
        // ponytail: boxes in rows (a shelf packer); Aspire nests true shapes at any step angle,
        // mirrors and fills holes
        // lay out in a frame where rows run along +u from the start corner and stack along +v
        let along_x = p.nest_direction == Direction::AlongX;
        let (room_u, room_v) = if along_x {
            (room.x, room.y)
        } else {
            (room.y, room.x)
        };
        let mut order: Vec<(usize, Point, bool)> = parts
            .iter()
            .enumerate()
            .map(|(i, (_, bounds))| {
                let mut size = bounds.size();
                let mut turned = false;
                // a quarter turn when it makes the part lower in its row and still fits
                if p.rotate_parts && size.x < size.y && size.y <= room_u {
                    size = Point::new(size.y, size.x);
                    turned = true;
                }
                let size_uv = if along_x {
                    size
                } else {
                    Point::new(size.y, size.x)
                };
                (i, size_uv, turned)
            })
            .collect();
        order.sort_by(|first, second| second.1.y.total_cmp(&first.1.y));
        let (mut across, mut up, mut row) = (0.0_f64, 0.0_f64, 0.0_f64);
        let mut placed = Vec::new();
        for (i, size, turned) in order {
            if across > 0.0 && across + size.x > room_u {
                (across, up, row) = (0.0, up + row + gap, 0.0);
            }
            if size.x > room_u || up + size.y > room_v {
                continue;
            }
            placed.push((i, Point::new(across, up), size, turned));
            across += size.x + gap;
            row = row.max(size.y);
        }
        if placed.is_empty() {
            return Err(CommandError::Failed("no part fits in the job".into()));
        }
        let (right, top) = (
            p.nest_from == Corner::BottomRight || p.nest_from == Corner::TopRight,
            matches!(p.nest_from, Corner::TopLeft | Corner::TopRight),
        );
        let mut c = Change::builder(doc);
        let mut made = Vec::new();
        for (i, at_uv, size_uv, turned) in placed {
            let (ids, bounds) = &parts[i];
            let (at, size) = if along_x {
                (at_uv, size_uv)
            } else {
                (
                    Point::new(at_uv.y, at_uv.x),
                    Point::new(size_uv.y, size_uv.x),
                )
            };
            // corner of the job the layout starts from, mirrored so rows grow inwards
            let left_x = if right {
                area.max.x - edge - at.x - size.x
            } else {
                area.min.x + edge + at.x
            };
            let low_y = if top {
                area.max.y - edge - at.y - size.y
            } else {
                area.min.y + edge + at.y
            };
            let target = Point::new(left_x, low_y) + size / 2.0;
            let place = DAffine2::from_translation(target)
                * DAffine2::from_angle(if turned {
                    std::f64::consts::FRAC_PI_2
                } else {
                    0.0
                })
                * DAffine2::from_translation(-bounds.center());
            for id in ids {
                let moved = doc.vectors[id].transformed(&place);
                if p.remove_original_parts {
                    c.replace(*id, moved);
                    made.push(*id);
                } else {
                    made.push(c.add(moved));
                }
            }
        }
        c.select(made);
        Ok(c.finish())
    }
}

/// True Shape nesting: each part (largest first), and each copy of it, put where its shape,
/// grown by half the gap all round, first fits on a grid of the job, at the turns allowed.
fn true_shape(
    doc: &Document,
    p: &Params,
    parts: &[(Vec<EntityId>, Rect)],
) -> Result<Change, CommandError> {
    let gap = p.tool_diameter + p.clearance;
    let edge = p.border_gap + p.clearance;
    let area = doc.job.area();
    let room = Rect::new(area.min + Point::splat(edge), area.max - Point::splat(edge));
    let mut grid = Grid::new(room, gap);
    let turns: Vec<f64> = if p.rotation_step > 0.0 {
        // the turns a whole step apart below a full turn
        std::iter::successors(Some(0.0), |a| Some(a + p.rotation_step))
            .take_while(|&a| a < 360.0 - 1e-9)
            .collect()
    } else if p.rotate_parts {
        vec![0.0, 90.0]
    } else {
        vec![0.0]
    };
    // the largest parts first, so small ones find the gaps left
    let mut order: Vec<usize> = (0..parts.len()).collect();
    order.sort_by(|&a, &b| {
        let size = |i: usize| parts[i].1.size().x * parts[i].1.size().y;
        size(b).total_cmp(&size(a))
    });
    let mut c = Change::builder(doc);
    let mut made = Vec::new();
    let mut nested = 0;
    for i in order {
        let (ids, bounds) = &parts[i];
        let rings: Vec<Vec<Point>> = ids
            .iter()
            .map(|id| doc.vectors[id].flatten(grid.cell / 4.0))
            .collect();
        for copy in 0..p.copies.max(1) {
            let Some(place) = turns.iter().find_map(|&turn| {
                let spin = DAffine2::from_angle(turn.to_radians())
                    * DAffine2::from_translation(-bounds.center());
                let cells = grid.shape(&rings, spin, gap / 2.0);
                let at = grid.first_fit(&cells, p.nest_from, p.nest_direction)?;
                Some((spin, cells, at))
            }) else {
                continue;
            };
            let (spin, cells, at) = place;
            grid.fill(&cells, at);
            let to = DAffine2::from_translation(grid.centre(at)) * spin;
            for id in ids {
                let moved = doc.vectors[id].transformed(&to);
                if p.remove_original_parts && copy == 0 {
                    c.replace(*id, moved);
                    made.push(*id);
                } else {
                    made.push(c.add(moved));
                }
            }
            nested += 1;
        }
    }
    if nested == 0 {
        return Err(CommandError::Failed("no part fits in the job".into()));
    }
    c.select(made);
    Ok(c.finish())
}

/// The job's room as square cells, filled where nested parts are.
struct Grid {
    room: Rect,
    cell: f64,
    columns: usize,
    rows: usize,
    full: Vec<bool>,
    /// The cells each nested part spans (its box), lowest first: where holes may be.
    boxes: Vec<((i64, i64), (i64, i64))>,
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    reason = "cell counts and indices are small"
)]
impl Grid {
    /// About 200 cells along the room's longer side, and no coarser than the gap.
    fn new(room: Rect, gap: f64) -> Self {
        let size = room.size();
        let cell = (size.x.max(size.y) / 200.0).min(gap.max(0.25)).max(0.25);
        let columns = (size.x / cell).floor().max(0.0) as usize;
        let rows = (size.y / cell).floor().max(0.0) as usize;
        Self {
            room,
            cell,
            columns,
            rows,
            full: vec![false; columns * rows],
            boxes: vec![],
        }
    }

    /// The cells a part covers (turned and centred by `spin`), grown by `grow`, as offsets
    /// from its centre cell.
    fn shape(&self, rings: &[Vec<Point>], spin: DAffine2, grow: f64) -> Vec<(i64, i64)> {
        let turned: Vec<Vec<Point>> = rings
            .iter()
            .map(|r| r.iter().map(|&q| spin.transform_point2(q)).collect())
            .collect();
        let (mut lo, mut hi) = (Point::splat(f64::INFINITY), Point::splat(f64::NEG_INFINITY));
        for q in turned.iter().flatten() {
            (lo, hi) = (lo.min(*q), hi.max(*q));
        }
        let (lo, hi) = (lo - Point::splat(grow), hi + Point::splat(grow));
        let n = |v: f64| (v / self.cell).round() as i64;
        let mut cells = vec![];
        for row in n(lo.y)..=n(hi.y) {
            for column in n(lo.x)..=n(hi.x) {
                let q = Point::new(column as f64, row as f64) * self.cell;
                let near = turned.iter().any(|r| {
                    cut_craft_core::geometry::polyline_distance(r, q) <= grow + self.cell / 2.0
                });
                if near || cut_craft_core::geometry::inside(&turned, q) {
                    cells.push((column, row));
                }
            }
        }
        cells
    }

    /// The first cell (from `corner`, along `direction`) where `cells` fit clear.
    fn first_fit(
        &self,
        cells: &[(i64, i64)],
        corner: Corner,
        direction: Direction,
    ) -> Option<(i64, i64)> {
        let (columns, rows) = (self.columns as i64, self.rows as i64);
        let from_right = matches!(corner, Corner::BottomRight | Corner::TopRight);
        let from_top = matches!(corner, Corner::TopLeft | Corner::TopRight);
        let column = |k: i64| if from_right { columns - 1 - k } else { k };
        let row = |k: i64| if from_top { rows - 1 - k } else { k };
        let fits = |at: (i64, i64)| {
            cells.iter().all(|&(dx, dy)| {
                let (x, y) = (at.0 + dx, at.1 + dy);
                x >= 0
                    && y >= 0
                    && x < columns
                    && y < rows
                    && !self.full[(y * columns + x) as usize]
            })
        };
        // inside the boxes of parts already nested first: their holes
        let in_holes = self.boxes.iter().find_map(|&((x0, y0), (x1, y1))| {
            (y0..=y1).find_map(|y| (x0..=x1).find_map(|x| fits((x, y)).then_some((x, y))))
        });
        if in_holes.is_some() {
            return in_holes;
        }
        let (outer, inner) = match direction {
            Direction::AlongX => (rows, columns),
            Direction::AlongY => (columns, rows),
        };
        (0..outer).find_map(|o| {
            (0..inner).find_map(|i| {
                let at = match direction {
                    Direction::AlongX => (column(i), row(o)),
                    Direction::AlongY => (column(o), row(i)),
                };
                fits(at).then_some(at)
            })
        })
    }

    fn fill(&mut self, cells: &[(i64, i64)], at: (i64, i64)) {
        let xs = cells.iter().map(|c| at.0 + c.0);
        let ys = cells.iter().map(|c| at.1 + c.1);
        if let (Some(x0), Some(x1), Some(y0), Some(y1)) =
            (xs.clone().min(), xs.max(), ys.clone().min(), ys.max())
        {
            self.boxes.push(((x0, y0), (x1, y1)));
        }
        for &(dx, dy) in cells {
            let (x, y) = (at.0 + dx, at.1 + dy);
            self.full[(y * self.columns as i64 + x) as usize] = true;
        }
    }

    /// The middle of cell `at`, in job coordinates.
    fn centre(&self, at: (i64, i64)) -> Point {
        self.room.min + Point::new(at.0 as f64, at.1 as f64) * self.cell
    }
}

/// The selection as parts: groups, and vectors whose boxes overlap, belong together.
fn parts(doc: &Document) -> Vec<(Vec<EntityId>, Rect)> {
    let mut parts: Vec<(Vec<EntityId>, Rect)> = Vec::new();
    for &id in &doc.selection {
        if parts.iter().any(|(ids, _)| ids.contains(&id)) {
            continue;
        }
        let members = doc.group_members(id);
        let b = members
            .iter()
            .map(|m| doc.vectors[m].bounds())
            .reduce(Rect::union)
            .expect("a member");
        parts.push((members, b));
    }
    // merge parts whose boxes overlap until none do
    let overlap = |a: Rect, b: Rect| {
        a.min.x <= b.max.x && b.min.x <= a.max.x && a.min.y <= b.max.y && b.min.y <= a.max.y
    };
    let mut merged = true;
    while merged {
        merged = false;
        'outer: for i in 0..parts.len() {
            for j in i + 1..parts.len() {
                if overlap(parts[i].1, parts[j].1) {
                    let (ids, b) = parts.remove(j);
                    parts[i].0.extend(ids);
                    parts[i].1 = parts[i].1.union(b);
                    merged = true;
                    break 'outer;
                }
            }
        }
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{document_with, run_on, square};

    #[test]
    fn true_shapes_fill_a_ring_s_hole_and_nest_copies() {
        use cut_craft_core::geometry::Path;
        // a 60 mm frame with a 40 mm hole (one part: its two squares), and a 10 mm square
        let mut doc = document_with(vec![
            square(300.0, 300.0, 60.0),
            square(310.0, 310.0, 40.0),
            square(500.0, 300.0, 10.0),
        ]);
        doc.selection = doc.vectors.keys().copied().collect();
        let p = Params {
            true_shape: true,
            tool_diameter: 2.0,
            clearance: 0.0,
            border_gap: 0.0,
            rotate_parts: false,
            ..Params::default()
        };
        run_on::<NestParts>(&mut doc, &p);
        let small = doc
            .vectors
            .values()
            .find(|v| v.bounds().size().x < 11.0)
            .unwrap()
            .bounds();
        let hole = doc
            .vectors
            .values()
            .find(|v| (v.bounds().size().x - 40.0).abs() < 0.5)
            .unwrap()
            .bounds();
        assert!(
            hole.min.x < small.min.x
                && small.max.x < hole.max.x
                && hole.min.y < small.min.y
                && small.max.y < hole.max.y,
            "the small square sits in the frame's hole: {small:?} in {hole:?}"
        );
        // three copies of the small square, none overlapping
        let mut doc = document_with(vec![square(500.0, 300.0, 10.0)]);
        doc.selection = doc.vectors.keys().copied().collect();
        run_on::<NestParts>(&mut doc, &Params { copies: 3, ..p });
        let boxes: Vec<Rect> = doc.vectors.values().map(Path::bounds).collect();
        assert_eq!(boxes.len(), 3);
        for (a, b) in [(0, 1), (0, 2), (1, 2)] {
            let apart = boxes[a].max.x + 1.9 <= boxes[b].min.x
                || boxes[b].max.x + 1.9 <= boxes[a].min.x
                || boxes[a].max.y + 1.9 <= boxes[b].min.y
                || boxes[b].max.y + 1.9 <= boxes[a].min.y;
            assert!(apart, "a gap between copies: {:?} {:?}", boxes[a], boxes[b]);
        }
    }

    /// Three 100 mm squares scattered, one with a 20 mm square inside it, on a 600 x 400 job.
    fn scattered() -> Document {
        document_with(vec![
            square(300.0, 250.0, 100.0),
            square(340.0, 290.0, 20.0),
            square(50.0, 280.0, 100.0),
            square(450.0, 10.0, 100.0),
        ])
    }

    cut_craft_core::contract_tests!(NestParts, scattered);

    fn boxes(doc: &Document) -> Vec<Rect> {
        let mut v: Vec<Rect> = doc
            .vectors
            .values()
            .map(cut_craft_core::geometry::Path::bounds)
            .filter(|r| r.size().x > 50.0)
            .collect();
        v.sort_by(|a, b| a.min.x.total_cmp(&b.min.x));
        v
    }

    #[test]
    fn parts_line_up_from_the_corner_a_gap_apart() {
        let mut doc = scattered();
        run_on::<NestParts>(&mut doc, &Params::default());
        let b = boxes(&doc);
        let gap = 6.35 + 1.0;
        assert!(
            (b[0].min - Point::new(6.0, 6.0)).length() < 1e-9,
            "{:?}",
            b[0]
        );
        assert!((b[1].min.x - (b[0].max.x + gap)).abs() < 1e-9);
        assert!((b[2].min.x - (b[1].max.x + gap)).abs() < 1e-9);
        // the inner square came along with its outer one
        let inner = doc
            .vectors
            .values()
            .find(|p| p.bounds().size().x < 50.0)
            .unwrap()
            .bounds();
        assert!(b.iter().any(|o| o.contains_rect(inner)));
    }

    #[test]
    fn copies_from_the_top_right_and_rows_that_wrap() {
        let mut doc = scattered();
        doc.job.width = 250.0;
        let p = Params {
            remove_original_parts: false,
            nest_from: Corner::TopRight,
            ..Params::default()
        };
        run_on::<NestParts>(&mut doc, &p);
        assert_eq!(doc.vectors.len(), 8, "copies beside the originals");
        let copies: Vec<Rect> = doc
            .selection
            .iter()
            .map(|id| doc.vectors[id].bounds())
            .filter(|r| r.size().x > 50.0)
            .collect();
        // two fit across 250 mm; the third wraps to a second row below
        let top = copies.iter().map(|b| b.max.y).fold(f64::MIN, f64::max);
        assert!((top - (400.0 - 6.0)).abs() < 1e-9);
        assert!(copies.iter().any(|b| b.max.y < 300.0), "a second row");
    }
}
