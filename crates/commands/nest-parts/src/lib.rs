//! Nest Parts: the selected vectors, as parts (vectors whose boxes overlap, or that are
//! grouped, stay together), laid out inside the job from a chosen corner, kept a tool's
//! width plus a clearance apart and a border in from the edge: by their boxes in rows, or
//! (True Shape) each where its own shape first fits, turned in steps, holes filled by
//! smaller parts, with as many copies of each as asked; mirrored where that fits better,
//! inside a boundary layer's vectors, onto new sheets when the job is full, with a double-
//! sided part's other side following it, and filler parts in what is left (Aspire's help:
//! Nest Parts).

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
#[expect(
    clippy::struct_excessive_bools,
    reason = "the tick boxes of Aspire's form"
)]
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
    /// True shape: also try each part flipped over, where that fits better; only when the
    /// direction the parts are cut in does not matter
    #[serde(default)]
    #[schemars(
        title = "Mirror parts to find best fit",
        extend("x-show-when" = {"true_shape": true})
    )]
    pub mirror_parts: bool,
    /// True shape: smaller parts go into the holes of bigger ones
    #[serde(default = "yes")]
    #[schemars(
        title = "Allow parts inside other parts",
        extend("x-show-when" = {"true_shape": true})
    )]
    pub parts_inside: bool,
    /// True shape: the layer whose closed vectors are the area to nest inside (holes in
    /// them are kept clear); empty nests inside the job
    #[serde(default)]
    #[schemars(
        title = "Nest boundary layer",
        extend("x-show-when" = {"true_shape": true})
    )]
    pub boundary_layer: String,
    /// True shape: the layer whose parts fill what space is left, as many as fit; empty for
    /// none
    #[serde(default)]
    #[schemars(title = "Filler part layer", extend("x-show-when" = {"true_shape": true}))]
    pub filler_layer: String,
    /// True shape: parts that do not fit go onto new sheets the size of this one
    #[serde(default)]
    #[schemars(
        title = "Add sheets as needed",
        extend("x-show-when" = {"true_shape": true})
    )]
    pub more_sheets: bool,
    /// True shape, double-sided jobs: what is on the other side under a part goes with it
    #[serde(default)]
    #[schemars(
        title = "Nest two-sided parts",
        extend("x-show-when" = {"true_shape": true})
    )]
    pub two_sided: bool,
}

fn yes() -> bool {
    true
}

/// The most sheets Add sheets as needed makes in one nesting.
const MOST_SHEETS: usize = 50;
/// The most filler parts one nesting places.
const MOST_FILLERS: usize = 2000;

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
            mirror_parts: false,
            parts_inside: true,
            boundary_layer: String::new(),
            filler_layer: String::new(),
            more_sheets: false,
            two_sided: false,
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

/// A part as it nests: its vectors, its box, and the other side's vectors under it.
struct Part {
    ids: Vec<EntityId>,
    bounds: Rect,
    riders: Vec<EntityId>,
    filler: bool,
}

/// Where a part (by its place in the list) goes: on which sheet (0 the active one, then
/// the new ones) and how it is moved there.
struct Placed {
    part: usize,
    sheet: usize,
    to: DAffine2,
}

/// True Shape nesting: each part (largest first), and each copy of it, put where its shape,
/// grown by half the gap all round, first fits on a grid of the room, at the turns (and
/// flips) allowed; onto new sheets when it is full and asked; then the filler parts until
/// nothing more fits.
fn true_shape(
    doc: &Document,
    p: &Params,
    parts: &[(Vec<EntityId>, Rect)],
) -> Result<Change, CommandError> {
    let gap = p.tool_diameter + p.clearance;
    let edge = p.border_gap + p.clearance;
    let (parts, boundary) = nestable(doc, p, parts)?;
    let room = if boundary.is_empty() {
        let area = doc.job.area();
        Rect::new(area.min + Point::splat(edge), area.max - Point::splat(edge))
    } else {
        let (mut lo, mut hi) = (Point::splat(f64::INFINITY), Point::splat(f64::NEG_INFINITY));
        for q in boundary.iter().flatten() {
            (lo, hi) = (lo.min(*q), hi.max(*q));
        }
        Rect::new(lo + Point::splat(edge), hi - Point::splat(edge))
    };
    let fresh = || {
        let mut grid = Grid::new(room, gap);
        grid.keep_inside(&boundary, edge);
        grid
    };
    let placed = place(doc, p, &parts, fresh);
    if placed.iter().all(|placed| parts[placed.part].filler) {
        return Err(CommandError::Failed("no part fits in the job".into()));
    }
    Ok(write(doc, p, &parts, &placed))
}

/// The parts to nest (the boundary layer's left out, the filler layer's marked, a two-sided
/// part's riders found) and the boundary layer's closed vectors.
fn nestable(
    doc: &Document,
    p: &Params,
    parts: &[(Vec<EntityId>, Rect)],
) -> Result<(Vec<Part>, Vec<Vec<Point>>), CommandError> {
    let layer = |name: &str| {
        let name = name.trim();
        (!name.is_empty())
            .then(|| doc.layers.iter().find(|l| l.name == name).map(|l| l.id))
            .flatten()
    };
    let boundary_layer = layer(&p.boundary_layer);
    if !p.boundary_layer.trim().is_empty() && boundary_layer.is_none() {
        return Err(CommandError::InvalidParams(format!(
            "there is no layer called {}",
            p.boundary_layer.trim()
        )));
    }
    let boundary: Vec<Vec<Point>> = boundary_layer.map_or_else(Vec::new, |id| {
        doc.vectors
            .iter()
            .filter(|(v, path)| path.closed && doc.layer_of(**v) == id)
            .map(|(_, path)| path.flatten(0.05))
            .collect()
    });
    let filler_layer = layer(&p.filler_layer);
    let parts: Vec<Part> = parts
        .iter()
        .filter(|(ids, _)| {
            ids.iter()
                .all(|id| Some(doc.layer_of(*id)) != boundary_layer)
        })
        .map(|(ids, bounds)| Part {
            ids: ids.clone(),
            bounds: *bounds,
            riders: if p.two_sided {
                riders(doc, *bounds)
            } else {
                vec![]
            },
            filler: filler_layer.is_some()
                && ids.iter().all(|id| Some(doc.layer_of(*id)) == filler_layer),
        })
        .collect();
    Ok((parts, boundary))
}

/// Where each part and copy goes: largest first on the first grid with room, a new grid
/// (`fresh`) when asked, then the fillers until nothing more fits.
fn place(doc: &Document, p: &Params, parts: &[Part], fresh: impl Fn() -> Grid) -> Vec<Placed> {
    let gap = p.tool_diameter + p.clearance;
    let mut grids = vec![fresh()];
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
    let flips: &[bool] = if p.mirror_parts {
        &[false, true]
    } else {
        &[false]
    };
    // the largest parts first, so small ones find the gaps left; fillers last
    let mut order: Vec<usize> = (0..parts.len()).collect();
    order.sort_by(|&a, &b| {
        let size = |i: usize| parts[i].bounds.size().x * parts[i].bounds.size().y;
        parts[a]
            .filler
            .cmp(&parts[b].filler)
            .then(size(b).total_cmp(&size(a)))
    });
    let mut placed: Vec<Placed> = vec![];
    for i in order {
        let part = &parts[i];
        let mut rings: Vec<Vec<Point>> = part
            .ids
            .iter()
            .map(|id| doc.vectors[id].flatten(grids[0].cell / 4.0))
            .collect();
        // a two-sided part's other side takes room too
        let mirror = doc.job.double_sided.map(|d| d.flip.mirror(doc.job.area()));
        if let Some(m) = mirror {
            for id in &part.riders {
                let path = &doc.other_side.vectors[id].path;
                rings.push(path.transformed(&m.inverse()).flatten(grids[0].cell / 4.0));
            }
        }
        let tries: Vec<DAffine2> = flips
            .iter()
            .flat_map(|&flip| {
                turns.iter().map(move |&turn| {
                    let flip = DAffine2::from_scale(Point::new(if flip { -1.0 } else { 1.0 }, 1.0));
                    DAffine2::from_angle(turn.to_radians()) * flip
                })
            })
            .map(|spin| spin * DAffine2::from_translation(-part.bounds.center()))
            .collect();
        let wanted = if part.filler {
            MOST_FILLERS
        } else {
            p.copies.max(1) as usize
        };
        let mut sheet = 0;
        let mut done = 0;
        while done < wanted {
            let found = tries.iter().find_map(|&spin| {
                let cells = grids[sheet].shape(&rings, spin, gap / 2.0, p.parts_inside);
                let at = grids[sheet].first_fit(
                    &cells,
                    p.nest_from,
                    p.nest_direction,
                    p.parts_inside,
                )?;
                Some((spin, cells, at))
            });
            if let Some((spin, cells, at)) = found {
                grids[sheet].fill(&cells, at);
                let to = DAffine2::from_translation(grids[sheet].centre(at)) * spin;
                placed.push(Placed { part: i, sheet, to });
                done += 1;
                continue;
            }
            // full: a filler stops; another part tries the next sheet, made when asked
            if part.filler {
                break;
            }
            sheet += 1;
            if sheet == grids.len() {
                if !p.more_sheets || grids.len() > MOST_SHEETS {
                    break;
                }
                grids.push(fresh());
            }
        }
    }
    placed
}

/// The other side's vectors under the box `bounds` of a part on this side: they go where it
/// goes.
fn riders(doc: &Document, bounds: Rect) -> Vec<EntityId> {
    let Some(two) = doc.job.double_sided else {
        return vec![];
    };
    let back = two.flip.mirror(doc.job.area()).inverse();
    doc.other_side
        .vectors
        .iter()
        .filter(|(_, v)| {
            let b = v.path.transformed(&back).bounds();
            b.min.x <= bounds.max.x
                && bounds.min.x <= b.max.x
                && b.min.y <= bounds.max.y
                && bounds.min.y <= b.max.y
        })
        .map(|(&id, _)| id)
        .collect()
}

/// The nested parts written: on the active sheet as moves or copies, and on the new sheets
/// (named after the active one) as stored vectors; a two-sided part's riders moved to match
/// on the other side.
fn write(doc: &Document, p: &Params, parts: &[Part], placed: &[Placed]) -> Change {
    use cut_craft_core::document::{Sheet, SheetObjects, StoredVector};
    let mut c = Change::builder(doc);
    let mut made = Vec::new();
    let mirror = doc.job.double_sided.map(|d| d.flip.mirror(doc.job.area()));
    let (bottom, mut other) = c.side().clone();
    let mut sheets: Vec<SheetObjects> = vec![];
    let mut moved_once = std::collections::BTreeSet::new();
    for placed in placed {
        let part = &parts[placed.part];
        // the first placing of a part moves it when asked; the rest are copies
        let relocate = p.remove_original_parts && moved_once.insert(placed.part);
        let rider_to = mirror.map(|m| m * placed.to * m.inverse());
        if placed.sheet == 0 {
            for id in &part.ids {
                let moved = doc.vectors[id].transformed(&placed.to);
                if relocate {
                    c.replace(*id, moved);
                    made.push(*id);
                } else {
                    made.push(c.add(moved));
                }
            }
            for id in &part.riders {
                let Some(to) = rider_to else { continue };
                let stored = &doc.other_side.vectors[id];
                let moved = StoredVector {
                    path: stored.path.transformed(&to),
                    ..stored.clone()
                };
                let key = if relocate { *id } else { c.fresh_id() };
                other.vectors.insert(key, moved);
            }
            continue;
        }
        while sheets.len() < placed.sheet {
            sheets.push(SheetObjects {
                other_side: (doc.job.double_sided.is_some())
                    .then(|| Box::new(SheetObjects::default())),
                ..SheetObjects::default()
            });
        }
        let objects = &mut sheets[placed.sheet - 1];
        for id in &part.ids {
            if relocate {
                c.remove(*id);
            }
            let key = c.fresh_id();
            objects.vectors.insert(
                key,
                StoredVector {
                    path: doc.vectors[id].transformed(&placed.to),
                    layer: Some(doc.layer_of(*id)),
                    group: None,
                },
            );
        }
        for id in &part.riders {
            let (Some(to), Some(back)) = (rider_to, objects.other_side.as_mut()) else {
                continue;
            };
            if relocate {
                other.vectors.remove(id);
            }
            let stored = &doc.other_side.vectors[id];
            let key = c.fresh_id();
            back.vectors.insert(
                key,
                StoredVector {
                    path: stored.path.transformed(&to),
                    ..stored.clone()
                },
            );
        }
    }
    c.set_side(bottom, other);
    if !sheets.is_empty() {
        let name = doc
            .sheets
            .get(doc.active_sheet)
            .map_or_else(|| "Sheet".to_owned(), |s| s.name.clone());
        let mut all = doc.sheets.clone();
        for (k, objects) in sheets.into_iter().enumerate() {
            let mut sheet = Sheet::new(&format!("{name} ({})", k + 2), doc.job);
            sheet.objects = objects;
            all.push(sheet);
        }
        c.set_sheets(all, doc.active_sheet);
    }
    c.select(made);
    c.finish()
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
    /// With `holes` off, a part's holes count as part of it, so nothing nests in them.
    fn shape(
        &self,
        rings: &[Vec<Point>],
        spin: DAffine2,
        grow: f64,
        holes: bool,
    ) -> Vec<(i64, i64)> {
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
                let within = if holes {
                    cut_craft_core::geometry::inside(&turned, q)
                } else {
                    turned
                        .iter()
                        .any(|r| cut_craft_core::geometry::inside(std::slice::from_ref(r), q))
                };
                if near || within {
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
        holes: bool,
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
        let in_holes = self
            .boxes
            .iter()
            .filter(|_| holes)
            .find_map(|&((x0, y0), (x1, y1))| {
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

    /// Only the inside of the boundary's `rings` (its holes kept out), `edge` in from them,
    /// is room; no rings: all of it.
    fn keep_inside(&mut self, rings: &[Vec<Point>], edge: f64) {
        if rings.is_empty() {
            return;
        }
        for row in 0..self.rows {
            for column in 0..self.columns {
                let q = self.centre((column as i64, row as i64));
                let clear = cut_craft_core::geometry::inside(rings, q)
                    && rings.iter().all(|r| {
                        cut_craft_core::geometry::polyline_distance(r, q) >= edge + self.cell / 2.0
                    });
                if !clear {
                    self.full[row * self.columns + column] = true;
                }
            }
        }
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

    use cut_craft_core::document::{Layer, LayerId};
    use cut_craft_core::geometry::Path;

    /// `doc` with a layer `name` holding the vectors `ids`.
    fn on_layer(doc: &mut Document, name: &str, ids: &[EntityId]) {
        let id = LayerId(u32::try_from(doc.layers.len()).unwrap() + 1);
        doc.layers.push(Layer {
            id,
            name: name.into(),
            visible: true,
            locked: false,
        });
        for v in ids {
            doc.vector_layers.insert(*v, id);
        }
    }

    fn triangle(points: [(f64, f64); 3]) -> Path {
        Path::polyline(&points.map(|(x, y)| Point::new(x, y)), true)
    }

    fn true_shape() -> Params {
        Params {
            true_shape: true,
            tool_diameter: 1.0,
            clearance: 0.0,
            border_gap: 0.0,
            rotate_parts: false,
            ..Params::default()
        }
    }

    #[test]
    fn a_part_mirrored_fits_a_boundary_its_own_way_round_does_not() {
        // the part's square corner bottom left; the boundary's bottom right: only mirrored
        // does the part go in
        let mut doc = document_with(vec![
            triangle([(400.0, 300.0), (440.0, 300.0), (400.0, 340.0)]),
            triangle([(100.0, 100.0), (170.0, 100.0), (170.0, 170.0)]),
        ]);
        let ids: Vec<EntityId> = doc.vectors.keys().copied().collect();
        on_layer(&mut doc, "Board", &ids[1..]);
        doc.selection = [ids[0]].into();
        let p = Params {
            boundary_layer: "Board".into(),
            ..true_shape()
        };
        assert!(
            NestParts::run(&doc, &p).is_err(),
            "its own way round it cannot go in"
        );
        run_on::<NestParts>(
            &mut doc,
            &Params {
                mirror_parts: true,
                ..p
            },
        );
        let part = doc.vectors[&ids[0]].bounds();
        let board = doc.vectors[&ids[1]].bounds();
        assert!(board.contains_rect(part), "{part:?} inside {board:?}");
        let flat = doc.vectors[&ids[0]].flatten(0.1);
        let lowest = flat.iter().fold(f64::INFINITY, |m, q| m.min(q.y));
        let corner = flat
            .iter()
            .find(|q| (q.y - lowest).abs() < 1e-6 && (q.x - part.max.x).abs() < 1e-6);
        assert!(
            corner.is_some(),
            "the square corner is now bottom right: {flat:?}"
        );
        // and a missing boundary layer is said so
        let missing = Params {
            boundary_layer: "Nope".into(),
            ..true_shape()
        };
        assert!(NestParts::run(&doc, &missing).is_err());
    }

    #[test]
    fn with_parts_inside_off_nothing_goes_in_a_hole() {
        let mut doc = document_with(vec![
            square(300.0, 300.0, 60.0),
            square(310.0, 310.0, 40.0),
            square(500.0, 300.0, 10.0),
        ]);
        doc.selection = doc.vectors.keys().copied().collect();
        run_on::<NestParts>(
            &mut doc,
            &Params {
                parts_inside: false,
                ..true_shape()
            },
        );
        let small = doc
            .vectors
            .values()
            .map(Path::bounds)
            .find(|b| b.size().x < 11.0)
            .unwrap();
        let frame = doc
            .vectors
            .values()
            .map(Path::bounds)
            .find(|b| b.size().x > 59.0)
            .unwrap();
        assert!(!frame.contains_rect(small), "{small:?} outside {frame:?}");
    }

    #[test]
    fn parts_that_do_not_fit_go_onto_new_sheets_and_fillers_fill_what_is_left() {
        let mut doc = document_with(vec![square(500.0, 300.0, 60.0), square(20.0, 20.0, 5.0)]);
        doc.job.width = 100.0;
        doc.job.height = 100.0;
        let ids: Vec<EntityId> = doc.vectors.keys().copied().collect();
        on_layer(&mut doc, "Fill", &ids[1..]);
        doc.selection = ids.iter().copied().collect();
        let p = Params {
            copies: 3,
            more_sheets: true,
            filler_layer: "Fill".into(),
            ..true_shape()
        };
        run_on::<NestParts>(&mut doc, &p);
        assert_eq!(doc.sheets.len(), 3, "one big square a sheet");
        for sheet in &doc.sheets[1..] {
            assert_eq!(sheet.objects.vectors.len(), 1);
        }
        let fillers = doc
            .vectors
            .values()
            .filter(|v| v.bounds().size().x < 6.0)
            .count();
        assert!(
            fillers > 20,
            "the space round the big square filled: {fillers}"
        );
        // without new sheets, the copies that do not fit are left out
        let mut one = document_with(vec![square(500.0, 300.0, 60.0)]);
        one.job.width = 100.0;
        one.job.height = 100.0;
        one.selection = one.vectors.keys().copied().collect();
        run_on::<NestParts>(
            &mut one,
            &Params {
                copies: 3,
                ..true_shape()
            },
        );
        assert_eq!((one.sheets.len(), one.vectors.len()), (1, 1));
    }

    #[test]
    fn a_two_sided_part_takes_its_other_side_with_it() {
        use cut_craft_core::document::{DoubleSided, Flip, StoredVector};
        let mut doc = document_with(vec![square(400.0, 200.0, 50.0)]);
        doc.job.double_sided = Some(DoubleSided {
            flip: Flip::Horizontal,
            zero_same_side: false,
        });
        let mirror = Flip::Horizontal.mirror(doc.job.area());
        // a pocket outline under the part, seen from the other side
        let under = square(410.0, 210.0, 20.0).transformed(&mirror);
        let rider = EntityId(99);
        doc.other_side.vectors.insert(
            rider,
            StoredVector {
                path: under,
                layer: None,
                group: None,
            },
        );
        let part = *doc.vectors.keys().next().unwrap();
        doc.selection = [part].into();
        run_on::<NestParts>(
            &mut doc,
            &Params {
                two_sided: true,
                ..true_shape()
            },
        );
        let moved = doc.vectors[&part].bounds();
        assert!(moved.min.x < 100.0, "nested from the corner: {moved:?}");
        let back = doc.other_side.vectors[&rider]
            .path
            .transformed(&mirror.inverse())
            .bounds();
        assert!(
            moved.contains_rect(back)
                && (back.min - moved.min - Point::splat(10.0)).length() < 1e-6,
            "the other side moved with it: {back:?} in {moved:?}"
        );
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
