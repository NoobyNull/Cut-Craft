//! Turn and Spin: a component from an open profile vector, turned round the line from its
//! start to its end (a lathe shape) or spun round its left end point (a dish or dome).

use cut_craft_core::component::{Combine, Component};
use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Point, Rect};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct TurnAndSpin;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Shape {
    /// Turn (rotate) round the line from the start point to the end point
    #[default]
    Turn,
    /// Spin round the left end point
    Spin,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    #[schemars(title = "Type of shape")]
    pub shape: Shape,
    /// Scale the shape so its highest point is the height
    pub scale_to_exact_height: bool,
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub height: f64,
    pub name: String,
    #[schemars(title = "Combine with other components")]
    pub combine: Combine,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            shape: Shape::Turn,
            scale_to_exact_height: false,
            height: 10.0,
            name: "Turned".into(),
            combine: Combine::Add,
        }
    }
}

impl Command for TurnAndSpin {
    const ID: &'static str = "turn_and_spin";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Turn and Spin",
            group: "Modeling",
            icon: "modeling/turn_and_spin.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let (_, paths) = doc.selected_paths(|path| !path.closed);
        let [profile] = paths.as_slice() else {
            return Err(CommandError::Failed(
                "select one open vector as the profile".into(),
            ));
        };
        let pts = profile.flatten(cut_craft_core::polygon::TOLERANCE);
        let cell = doc.job.model_cell();
        let mut made = match p.shape {
            Shape::Turn => turned(&pts, cell, &p.name, p.combine)?,
            Shape::Spin => spun(&pts, cell, &p.name, p.combine)?,
        };
        if p.scale_to_exact_height
            && let Some((_, top)) = made.height_range().filter(|r| r.1 > 0.0)
        {
            for h in made.heights.iter_mut().filter(|h| !h.is_nan()) {
                *h *= p.height / top;
            }
        }
        if made.height_range().is_none() {
            return Err(CommandError::Failed("the profile makes no shape".into()));
        }
        let mut c = Change::builder(doc);
        c.add_component(made);
        Ok(c.finish())
    }
}

/// The profile's distance from the axis at each distance along it, as (along, off) pairs.
fn radii(pts: &[Point], origin: Point, dir: Point) -> Vec<(f64, f64)> {
    let mut r: Vec<(f64, f64)> = pts
        .iter()
        .map(|&q| ((q - origin).dot(dir), (q - origin).perp_dot(dir).abs()))
        .collect();
    r.sort_by(|a, b| a.0.total_cmp(&b.0));
    r
}

/// Linear interpolation in sorted (x, y) pairs; `None` outside them.
fn at(table: &[(f64, f64)], x: f64) -> Option<f64> {
    let i = table.partition_point(|e| e.0 < x);
    if i == 0 {
        return table
            .first()
            .filter(|e| (e.0 - x).abs() < 1e-9)
            .map(|e| e.1);
    }
    let (lo, hi) = (*table.get(i - 1)?, *table.get(i)?);
    let part = if hi.0 > lo.0 {
        (x - lo.0) / (hi.0 - lo.0)
    } else {
        0.0
    };
    Some(lo.1 + (hi.1 - lo.1) * part)
}

fn turned(
    pts: &[Point],
    cell: f64,
    name: &str,
    combine: Combine,
) -> Result<Component, CommandError> {
    let (start, end) = (pts[0], pts[pts.len() - 1]);
    let length = start.distance(end);
    if length <= 0.0 {
        return Err(CommandError::Failed(
            "the profile's ends must be apart to turn round".into(),
        ));
    }
    let dir = (end - start) / length;
    let table = radii(pts, start, dir);
    let reach = table.iter().map(|e| e.1).fold(0.0, f64::max);
    let across = dir.perp() * reach;
    let area = [start + across, start - across, end + across, end - across]
        .into_iter()
        .fold(Rect::point(start), Rect::include);
    Ok(Component::from_fn(name, combine, area, cell, |q| {
        let along = (q - start).dot(dir);
        let off = (q - start).perp_dot(dir).abs();
        let r = at(&table, along)?;
        (off <= r).then(|| (r * r - off * off).sqrt())
    }))
}

fn spun(pts: &[Point], cell: f64, name: &str, combine: Combine) -> Result<Component, CommandError> {
    let (first, last) = (pts[0], pts[pts.len() - 1]);
    let centre = if first.x <= last.x { first } else { last };
    let table: Vec<(f64, f64)> = {
        let mut t: Vec<_> = pts
            .iter()
            .map(|q| (q.x - centre.x, q.y - centre.y))
            .collect();
        t.sort_by(|a, b| a.0.total_cmp(&b.0));
        t
    };
    let reach = table.last().map_or(0.0, |e| e.0);
    if reach <= 0.0 {
        return Err(CommandError::Failed(
            "the profile must run to the right of its left end".into(),
        ));
    }
    let area = Rect::new(centre - Point::splat(reach), centre + Point::splat(reach));
    Ok(Component::from_fn(name, combine, area, cell, |q| {
        at(&table, q.distance(centre)).map(|z| z.max(0.0))
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Path;
    use cut_craft_core::testing::{document_with, run_on};

    /// A flat-topped profile 10 above the line from (10, 50) to (90, 50).
    fn profile() -> Document {
        let mut doc = document_with(vec![Path::polyline(
            &[
                Point::new(10.0, 50.0),
                Point::new(10.0, 60.0),
                Point::new(90.0, 60.0),
                Point::new(90.0, 50.0),
            ],
            false,
        )]);
        doc.job.width = 100.0;
        doc.job.height = 100.0;
        doc
    }

    cut_craft_core::contract_tests!(TurnAndSpin, profile);

    fn made(p: &Params) -> Component {
        let mut doc = profile();
        run_on::<TurnAndSpin>(&mut doc, p);
        doc.components.values().next().unwrap().clone()
    }

    #[test]
    fn turning_a_flat_profile_makes_a_half_cylinder() {
        let k = made(&Params::default());
        let top = k.height_at(Point::new(50.0, 50.0)).unwrap();
        assert!((top - 10.0).abs() < 0.1, "{top}");
        let side = k.height_at(Point::new(50.0, 56.0)).unwrap();
        assert!((side - 8.0).abs() < 0.15, "√(100 − 36) = 8: {side}");
        assert!(k.height_at(Point::new(50.0, 61.0)).is_none());
    }

    #[test]
    fn spinning_and_scaling() {
        let p = Params {
            shape: Shape::Spin,
            scale_to_exact_height: true,
            height: 5.0,
            ..Params::default()
        };
        let k = made(&p);
        // round (10, 50): the profile is 10 high from radius 0 to 80; scaled to 5
        let h = k.height_at(Point::new(10.0 + 40.0, 50.0)).unwrap();
        assert!((h - 5.0).abs() < 1e-6, "{h}");
        assert!(TurnAndSpin::run(&Document::default(), &Params::default()).is_err());
    }
}
