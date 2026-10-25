//! Copy Along Vectors: copies of the first selected object (or circles) spaced along the
//! other selected vectors, by a distance or a count, upright or turned with the curve.

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::geometry::{Path, Point};
use cut_craft_core::glam::DAffine2;
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct CopyAlongVectors;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum What {
    /// The first object selected, copied along the rest
    #[default]
    CopyObject,
    /// Circles of the diameter, along every selected vector
    CopyCircles,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Specify {
    #[default]
    DistanceBetweenCopies,
    NumberOfCopies,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[expect(clippy::struct_excessive_bools, reason = "the form's own check boxes")]
pub struct Params {
    pub what: What,
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub diameter: f64,
    pub specify: Specify,
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub distance: f64,
    /// Stretch the distance a little so copies land on both ends
    pub force_even_spacing: bool,
    #[schemars(title = "Number of copies", range(min = 1, max = 10000))]
    pub copies: u32,
    /// Turn each copy with the curve under it
    pub align_objects_to_curve: bool,
    pub create_copies_on_new_layer: bool,
    /// Go along the curves the other way
    pub reverse_direction: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            what: What::CopyObject,
            diameter: 5.0,
            specify: Specify::DistanceBetweenCopies,
            distance: 20.0,
            force_even_spacing: true,
            copies: 10,
            align_objects_to_curve: false,
            create_copies_on_new_layer: false,
            reverse_direction: false,
        }
    }
}

impl Command for CopyAlongVectors {
    const ID: &'static str = "copy_along_vectors";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Copy Along Vectors",
            group: "Offset and Layout",
            icon: "drawing/create_vectors/copy_along_vectors.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let picked: Vec<EntityId> = doc.selection.iter().copied().collect();
        let (object, curves): (Vec<Path>, Vec<EntityId>) = match p.what {
            What::CopyCircles => {
                if p.diameter <= 0.0 {
                    return Err(CommandError::InvalidParams(
                        "diameter must be above 0".into(),
                    ));
                }
                (vec![Path::circle(Point::ZERO, p.diameter / 2.0)], picked)
            }
            What::CopyObject => {
                let Some(&first) = picked.first() else {
                    return Err(CommandError::NothingSelected);
                };
                let members = doc.group_members(first);
                let object = members.iter().map(|id| doc.vectors[id].clone()).collect();
                (
                    object,
                    picked
                        .into_iter()
                        .filter(|id| !members.contains(id))
                        .collect(),
                )
            }
        };
        if curves.is_empty() {
            return Err(CommandError::Failed(
                "select the object, then the vectors to copy it along".into(),
            ));
        }
        let middle = object
            .iter()
            .map(Path::bounds)
            .reduce(cut_craft_core::geometry::Rect::union)
            .expect("an object")
            .center();
        let mut c = Change::builder(doc);
        let layer = p.create_copies_on_new_layer.then(|| c.add_layer("Copies"));
        for id in curves {
            let mut curve = doc.vectors[&id].clone();
            if p.reverse_direction {
                curve = curve.reversed();
            }
            for (at, along) in stations(&curve, p)? {
                let turn = if p.align_objects_to_curve {
                    along.to_angle()
                } else {
                    0.0
                };
                let t = DAffine2::from_translation(at)
                    * DAffine2::from_angle(turn)
                    * DAffine2::from_translation(-middle);
                for part in &object {
                    let copy = c.add(part.transformed(&t));
                    if let Some(layer) = layer {
                        c.move_to_layer(copy, layer);
                    }
                }
            }
        }
        Ok(c.finish())
    }
}

/// Where copies go along `curve`, with the curve's direction there.
fn stations(curve: &Path, p: &Params) -> Result<Vec<(Point, Point)>, CommandError> {
    let pts = curve.flatten(cut_craft_core::polygon::TOLERANCE);
    let mut lengths = vec![0.0];
    for pair in pts.windows(2) {
        lengths.push(lengths.last().copied().unwrap_or_default() + pair[0].distance(pair[1]));
    }
    let total = lengths.last().copied().unwrap_or_default();
    if total <= 0.0 {
        return Ok(vec![]);
    }
    // a closed curve's end is its start, so it takes one copy fewer
    let ends = u32::from(!curve.closed);
    let (count, step) = match p.specify {
        Specify::NumberOfCopies => {
            let wanted = p.copies.max(1);
            let gaps = (wanted - ends).max(1);
            (wanted, total / f64::from(gaps))
        }
        Specify::DistanceBetweenCopies => {
            if p.distance <= 0.0 {
                return Err(CommandError::InvalidParams(
                    "distance must be above 0".into(),
                ));
            }
            let gaps = total / p.distance;
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a positive count"
            )]
            let gaps = if p.force_even_spacing {
                gaps.round().max(1.0)
            } else {
                gaps.floor()
            } as u32;
            let step = if p.force_even_spacing {
                total / f64::from(gaps)
            } else {
                p.distance
            };
            (gaps + ends, step)
        }
    };
    Ok((0..count)
        .map(|copy| {
            let along = (step * f64::from(copy)).min(total);
            let i = lengths
                .partition_point(|&l| l < along)
                .clamp(1, pts.len() - 1);
            let (from, to) = (pts[i - 1], pts[i]);
            let span = lengths[i] - lengths[i - 1];
            let part = if span > 0.0 {
                (along - lengths[i - 1]) / span
            } else {
                0.0
            };
            (from.lerp(to, part), (to - from).normalize_or_zero())
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{document_with, run_on, square};

    /// A 2 mm square picked first, then a 100 mm line along x.
    fn star_and_line() -> Document {
        let line = Path::polyline(&[Point::new(0.0, 50.0), Point::new(100.0, 50.0)], false);
        document_with(vec![square(-1.0, -1.0, 2.0), line])
    }

    cut_craft_core::contract_tests!(CopyAlongVectors, star_and_line);

    fn centres(doc: &Document) -> Vec<Point> {
        doc.vectors
            .values()
            .skip(2)
            .map(|v| v.bounds().center())
            .collect()
    }

    #[test]
    fn distance_with_even_spacing_lands_on_both_ends() {
        let mut doc = star_and_line();
        let p = Params {
            distance: 30.0,
            ..Params::default()
        };
        run_on::<CopyAlongVectors>(&mut doc, &p);
        let c = centres(&doc);
        assert_eq!(c.len(), 4, "100 / 30 rounds to 3 gaps");
        assert!((c[0] - Point::new(0.0, 50.0)).length() < 1e-9);
        assert!((c[3] - Point::new(100.0, 50.0)).length() < 1e-9);
    }

    #[test]
    fn counts_circles_layers_and_reversing() {
        let mut doc = star_and_line();
        let p = Params {
            what: What::CopyCircles,
            specify: Specify::NumberOfCopies,
            copies: 5,
            create_copies_on_new_layer: true,
            reverse_direction: true,
            ..Params::default()
        };
        run_on::<CopyAlongVectors>(&mut doc, &p);
        // both selected vectors get circles: 5 along the line, 5 round the square
        assert_eq!(doc.vectors.len(), 12);
        assert_eq!(doc.layers.len(), 2, "a new layer for the copies");
        let line_copies: Vec<Point> = centres(&doc).into_iter().filter(|q| q.y > 40.0).collect();
        assert!(
            (line_copies[0] - Point::new(100.0, 50.0)).length() < 1e-9,
            "reversed"
        );
        assert!((line_copies[1] - Point::new(75.0, 50.0)).length() < 1e-9);
    }

    #[test]
    fn aligned_copies_turn_with_the_curve() {
        let up = Path::polyline(&[Point::new(0.0, 0.0), Point::new(0.0, 100.0)], false);
        let mut doc = document_with(vec![
            Path::polyline(&[Point::new(-4.0, 0.0), Point::new(4.0, 0.0)], false),
            up,
        ]);
        let p = Params {
            specify: Specify::NumberOfCopies,
            copies: 2,
            align_objects_to_curve: true,
            ..Params::default()
        };
        run_on::<CopyAlongVectors>(&mut doc, &p);
        let copy = doc.vectors.values().nth(2).unwrap().bounds();
        assert!(
            copy.size().x < 1e-9 && (copy.size().y - 8.0).abs() < 1e-9,
            "turned upright"
        );
    }
}
