//! Dimensions: a length, horizontal or vertical dimension between two points, as vectors:
//! extension lines, a dimension line with arrow heads, and the measurement in a font, grouped.

use cut_craft_core::document::{Document, Units};
use cut_craft_core::font::{self, Align};
use cut_craft_core::geometry::{Path, Point};
use cut_craft_core::glam::DAffine2;
use cut_craft_core::params::Xy;
use cut_craft_core::{Change, Command, CommandError, Drag, Interactive, Meta, ensure_positive};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};

pub struct Dimensions;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// The straight distance, in any direction
    #[default]
    #[schemars(title = "Length Dimension")]
    Length,
    #[schemars(title = "Vertical Dimension")]
    Vertical,
    #[schemars(title = "Horizontal Dimension")]
    Horizontal,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub kind: Kind,
    /// Drag from one point to the other to set both
    pub from: Xy,
    pub to: Xy,
    #[schemars(schema_with = "font_choices")]
    pub font: String,
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub text_height: f64,
    #[schemars(range(min = 0, max = 6))]
    pub decimal_places: u32,
    /// How far the dimension line sits from the measured points, to their left
    #[schemars(extend("x-unit" = "mm"))]
    pub offset: f64,
    /// Shown instead of the measurement when not empty
    #[schemars(title = "Use custom text")]
    pub custom_text: String,
}

fn font_choices(_: &mut SchemaGenerator) -> Schema {
    json_schema!({ "type": "string", "enum": font::installed(), "x-widget": "drop-down" })
}

impl Default for Params {
    fn default() -> Self {
        Self {
            kind: Kind::Length,
            from: Xy::new(10.0, 10.0),
            to: Xy::new(60.0, 10.0),
            font: font::installed().into_iter().next().unwrap_or_default(),
            text_height: 5.0,
            decimal_places: 2,
            offset: 10.0,
            custom_text: String::new(),
        }
    }
}

impl Command for Dimensions {
    const ID: &'static str = "dimensions";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Dimensions",
            group: "Create Vectors",
            icon: "drawing/create_vectors/dimensions.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("text height", p.text_height)?;
        let (a, b): (Point, Point) = (p.from.into(), p.to.into());
        // the measured direction
        let along = match p.kind {
            Kind::Length => b - a,
            Kind::Horizontal => Point::new(b.x - a.x, 0.0),
            Kind::Vertical => Point::new(0.0, b.y - a.y),
        };
        let length = along.length();
        if length <= 0.0 {
            return Err(CommandError::InvalidParams(
                "drag between two different points".into(),
            ));
        }
        let dir = along / length;
        let side = dir.perp();
        // the dimension line runs from a's foot to b's foot, `offset` to the left of a
        let start = a + side * p.offset;
        let end = start + dir * length;
        let head = p.text_height * 0.6;
        let arrow = |tip: Point, back: Point| {
            let wing = side * head * 0.3;
            Path::polyline(
                &[tip, tip + back * head + wing, tip + back * head - wing],
                true,
            )
        };
        // extension lines leave a small gap at the measured point and run a little past
        let gap = p.text_height * 0.3;
        let extension = |from: Point, to: Point| {
            let out = (to - from).normalize_or_zero() * gap;
            Path::polyline(&[from + out, to + out], false)
        };
        let mut paths = vec![
            extension(a, start),
            extension(b, end),
            Path::polyline(&[start, end], false),
            arrow(start, dir),
            arrow(end, -dir),
        ];
        let shown = match doc.job.units {
            Units::Mm => length,
            Units::Inch => length / 25.4,
        };
        let text = if p.custom_text.is_empty() {
            format!("{shown:.*}", p.decimal_places as usize)
        } else {
            p.custom_text.clone()
        };
        let face = font::load(&p.font, false, false, &text).map_err(CommandError::Failed)?;
        // text centred above the middle of the line, turned with it
        let middle = start.lerp(end, 0.5) + side * gap;
        let place = DAffine2::from_translation(middle) * DAffine2::from_angle(dir.to_angle());
        paths.extend(
            face.layout(&text, p.text_height, Align::Center, Point::ZERO)
                .iter()
                .map(|q| q.transformed(&place)),
        );
        let group = doc.next_group();
        let mut c = Change::builder(doc);
        let ids: Vec<_> = paths.into_iter().map(|q| c.add(q)).collect();
        for &id in &ids {
            c.set_group(id, Some(group));
        }
        c.select(ids);
        Ok(c.finish())
    }
}

impl Interactive for Dimensions {
    fn from_drag(drag: Drag, current: &Params) -> Params {
        Params {
            from: Xy::new(drag.start.x, drag.start.y),
            to: Xy::new(drag.end.x, drag.end.y),
            ..current.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Rect;
    use cut_craft_core::testing::{after, rejection};

    cut_craft_core::contract_tests!(Dimensions);

    fn bounds(doc: &Document) -> Rect {
        doc.selection
            .iter()
            .map(|id| doc.vectors[id].bounds())
            .reduce(Rect::union)
            .unwrap()
    }

    #[test]
    fn a_length_dimension_sits_offset_above_its_points_with_its_text() {
        let doc = after::<Dimensions>(&Params::default());
        let b = bounds(&doc);
        // from (10, 10) to (60, 10), the line 10 above; text 5 tall above that
        assert!(
            (b.min.x - 10.0).abs() < 1e-6 && (b.max.x - 60.0).abs() < 1e-6,
            "{b:?}"
        );
        assert!(b.max.y > 25.0 && b.max.y < 27.0, "{b:?}");
        let first = *doc.selection.iter().next().unwrap();
        assert_eq!(
            doc.group_members(first).len(),
            doc.selection.len(),
            "one group"
        );
    }

    #[test]
    fn vertical_dimensions_measure_only_height() {
        let p = Params {
            kind: Kind::Vertical,
            from: Xy::new(0.0, 0.0),
            to: Xy::new(30.0, 20.0),
            custom_text: "H".into(),
            ..Params::default()
        };
        let doc = after::<Dimensions>(&p);
        // the dimension line runs up 20 mm, 10 mm to the left of x = 0
        let line = doc
            .vectors
            .values()
            .find(|v| v.segments.len() == 1 && v.bounds().size().y > 19.0)
            .unwrap();
        let lb = line.bounds();
        assert!(
            (lb.min.x + 10.0).abs() < 1e-6 && (lb.size().y - 20.0).abs() < 1e-6,
            "{lb:?}"
        );
        let same = Params {
            to: Xy::new(30.0, 0.0),
            ..p
        };
        assert!(rejection::<Dimensions>(&same).contains("two different points"));
    }
}
