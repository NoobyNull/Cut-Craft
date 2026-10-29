//! Nest Parts: the selected vectors, as parts (vectors whose boxes overlap, or that are
//! grouped, stay together), laid out in rows inside the job from a chosen corner, kept a
//! tool's width plus a clearance apart and a border in from the edge.

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
