//! Trim Objects (Trim selected objects to last selected objects): every selected vector is
//! cut where it crosses the closed vector picked last (and the rest of its group), and the
//! pieces inside it, or outside it, are cleared away. The boundary itself stays.

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::geometry::{Path, Point, inside};
use cut_craft_core::trace::{Fit, fit};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct TrimObjects;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Clear {
    /// What lies outside the boundary goes
    #[default]
    OutsideBoundary,
    /// What lies inside the boundary goes
    InsideBoundary,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub clear: Clear,
    /// Turn the trimmed pieces back into curves rather than many short lines
    pub refit_curves_after_clipping: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            clear: Clear::OutsideBoundary,
            refit_curves_after_clipping: true,
        }
    }
}

/// Where segments ab and cd cross, as a share of ab.
fn crossing(from: Point, to: Point, other: (Point, Point)) -> Option<f64> {
    let (r, s) = (to - from, other.1 - other.0);
    let den = r.perp_dot(s);
    if den.abs() < 1e-12 {
        return None;
    }
    let t = (other.0 - from).perp_dot(s) / den;
    let u = (other.0 - from).perp_dot(r) / den;
    ((0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)).then_some(t)
}

/// `points` (a run; a closed ring repeats its first point last) cut where it crosses `rings`.
fn pieces(points: &[Point], rings: &[Vec<Point>]) -> Vec<Vec<Point>> {
    let mut out = vec![vec![points[0]]];
    for pair in points.windows(2) {
        let mut cuts: Vec<f64> = rings
            .iter()
            .flat_map(|ring| {
                ring.iter()
                    .zip(ring.iter().cycle().skip(1))
                    .filter_map(|(&a, &b)| crossing(pair[0], pair[1], (a, b)))
                    .collect::<Vec<_>>()
            })
            .filter(|t| *t > 1e-9 && *t < 1.0 - 1e-9)
            .collect();
        cuts.sort_by(f64::total_cmp);
        for t in cuts {
            let at = pair[0].lerp(pair[1], t);
            out.last_mut().expect("never empty").push(at);
            out.push(vec![at]);
        }
        out.last_mut().expect("never empty").push(pair[1]);
    }
    out.retain(|p| p.len() > 1);
    out
}

impl Command for TrimObjects {
    const ID: &'static str = "trim_objects";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Trim Objects",
            group: "Edit Objects",
            icon: "drawing/edit_objects/trim_objects.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let last = doc.selection.last().ok_or(CommandError::NothingSelected)?;
        let boundary: Vec<EntityId> = doc.group_members(last);
        if boundary.iter().any(|id| !doc.vectors[id].closed) {
            return Err(CommandError::Failed(
                "the object picked last, the boundary, must be closed. Pick a closed vector last"
                    .into(),
            ));
        }
        let tolerance = cut_craft_core::polygon::TOLERANCE;
        let rings: Vec<Vec<Point>> = boundary
            .iter()
            .map(|id| doc.vectors[id].flatten(tolerance))
            .collect();
        let trimmed: Vec<EntityId> = doc
            .selection
            .iter()
            .copied()
            .filter(|id| !boundary.contains(id) && doc.editable(*id))
            .collect();
        if trimmed.is_empty() {
            return Err(CommandError::Failed(
                "pick the objects to trim, then last the boundary".into(),
            ));
        }
        let keep_inside = p.clear == Clear::OutsideBoundary;
        let mut removed = vec![];
        let mut added = vec![];
        for id in trimmed {
            let path = &doc.vectors[&id];
            let mut points = path.flatten(tolerance);
            if path.closed {
                points.push(points[0]);
            }
            let mut parts = pieces(&points, &rings);
            // a closed vector's first and last pieces are one, joined at its start
            if path.closed && parts.len() > 1 {
                let first = parts.remove(0);
                parts
                    .last_mut()
                    .expect("several")
                    .extend(first.into_iter().skip(1));
            }
            // the middle of a piece's middle span says which side it lies on
            let middle = |part: &Vec<Point>| {
                let k = (part.len() - 2) / 2;
                part[k].lerp(part[k + 1], 0.5)
            };
            let uncut = parts.len() == 1;
            let kept: Vec<Vec<Point>> = parts
                .into_iter()
                .filter(|part| inside(&rings, middle(part)) == keep_inside)
                .collect();
            if uncut && kept.len() == 1 {
                continue; // nothing crossed and it stays whole
            }
            removed.push(id);
            let whole = kept.len() == 1 && path.closed && kept[0].first() == kept[0].last();
            for part in kept {
                added.push(if p.refit_curves_after_clipping {
                    fit(&part, whole, Fit::Beziers, tolerance, 30f64.to_radians())
                } else {
                    Path::polyline(&part, false)
                });
            }
        }
        if removed.is_empty() {
            return Err(CommandError::Failed(
                "nothing lies on the side to clear. Click on the other side".into(),
            ));
        }
        Ok(Change::swap(doc, removed, added))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{document_with, square};

    /// A long line across a 20 mm square, the square picked last.
    fn crossed() -> Document {
        let line = Path::polyline(&[Point::new(-10.0, 10.0), Point::new(30.0, 10.0)], false);
        let mut doc = document_with(vec![line, square(0.0, 0.0, 20.0)]);
        let ids: Vec<_> = doc.vectors.keys().copied().collect();
        doc.selection = cut_craft_core::selection::Selection::default();
        doc.selection.insert(ids[0]);
        doc.selection.insert(ids[1]);
        doc
    }

    cut_craft_core::contract_tests!(TrimObjects, crossed);

    fn ends(doc: &Document) -> Vec<(f64, f64)> {
        doc.vectors
            .values()
            .filter(|v| !v.closed)
            .map(|v| (v.start.x, v.segments.last().unwrap().end().x))
            .collect()
    }

    #[test]
    fn clearing_outside_keeps_the_piece_in_the_boundary() {
        let mut doc = crossed();
        doc.apply(&TrimObjects::run(&doc, &Params::default()).unwrap())
            .unwrap();
        assert_eq!(ends(&doc), [(0.0, 20.0)]);
        assert_eq!(
            doc.vectors.values().filter(|v| v.closed).count(),
            1,
            "the boundary stays"
        );
    }

    #[test]
    fn clearing_inside_keeps_the_two_ends() {
        let mut doc = crossed();
        let p = Params {
            clear: Clear::InsideBoundary,
            refit_curves_after_clipping: false,
        };
        doc.apply(&TrimObjects::run(&doc, &p).unwrap()).unwrap();
        let mut got = ends(&doc);
        got.sort_by(|a, b| a.0.total_cmp(&b.0));
        assert_eq!(got, [(-10.0, 0.0), (20.0, 30.0)]);
    }

    #[test]
    fn the_boundary_must_be_closed_and_picked_last() {
        let mut doc = crossed();
        let ids: Vec<_> = doc.vectors.keys().copied().collect();
        doc.selection = cut_craft_core::selection::Selection::default();
        doc.selection.insert(ids[1]);
        doc.selection.insert(ids[0]);
        assert!(TrimObjects::run(&doc, &Params::default()).is_err());
    }
}
