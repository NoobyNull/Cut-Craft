//! Set Size: the selection, or each selected item on its own, sized to a width and height
//! (in millimetres, or as a percentage of the size it has), about an anchor point.

use cut_craft_core::document::Document;
use cut_craft_core::ensure_positive;
use cut_craft_core::geometry::Point;
use cut_craft_core::glam::DAffine2;
use cut_craft_core::params::Anchor;
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct SetSize;

/// What is sized.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// The whole selection, as one group
    #[default]
    ScaleSelection,
    /// Each selected item on its own, as if it alone were selected
    ScaleItemsIndividually,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    #[serde(default)]
    #[schemars(extend("x-widget" = "row"))]
    pub mode: Mode,
    /// The width, or with Percent, the share of the present width
    #[schemars(
        range(min = 0),
        extend("x-unit" = "mm", "x-percent-when" = {"percent": "true"})
    )]
    pub width: f64,
    /// Ignored when keeping proportions
    #[schemars(
        range(min = 0),
        extend("x-unit" = "mm", "x-percent-when" = {"percent": "true"})
    )]
    pub height: f64,
    /// Width and height are percentages of the present size, not millimetres
    #[serde(default)]
    pub percent: bool,
    /// Change the width and the height is worked out, so the shape is not stretched
    #[schemars(title = "Link XY")]
    pub keep_proportions: bool,
    /// Point of the selection that stays put
    pub anchor: Anchor,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            mode: Mode::ScaleSelection,
            width: 100.0,
            height: 100.0,
            percent: false,
            keep_proportions: true,
            anchor: Anchor::Center,
        }
    }
}

impl Command for SetSize {
    const ID: &'static str = "set_size";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Set Size",
            group: "Transform Objects",
            icon: "drawing/edit_objects/set_size.png",
            shortcut: "T",
        }
    }

    /// Starts from the selection's current size, so only the changed value needs typing.
    fn initial_params(doc: &Document) -> Params {
        doc.selection_bounds()
            .map_or_else(Params::default, |b| Params {
                width: b.size().x,
                height: b.size().y,
                ..Params::default()
            })
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("width", p.width)?;
        if !p.keep_proportions {
            ensure_positive("height", p.height)?;
        }
        let bounds = doc
            .selection_bounds()
            .ok_or(CommandError::NothingSelected)?;
        let scale = |size: Point| -> Option<Point> {
            if size.x <= 0.0 || size.y <= 0.0 {
                return None;
            }
            let (w, h) = if p.percent {
                (size.x * p.width / 100.0, size.y * p.height / 100.0)
            } else {
                (p.width, p.height)
            };
            let sx = w / size.x;
            Some(if p.keep_proportions {
                Point::splat(sx)
            } else {
                Point::new(sx, h / size.y)
            })
        };
        let about = |b: cut_craft_core::geometry::Rect, by: Point| {
            let a = p.anchor.point(b);
            DAffine2::from_translation(a)
                * DAffine2::from_scale(by)
                * DAffine2::from_translation(-a)
        };
        if p.mode == Mode::ScaleItemsIndividually {
            // each item about its own anchor; one without area stays as it is
            return Change::transform_selected(doc, |b| {
                scale(b.size()).map_or(DAffine2::IDENTITY, |by| about(b, by))
            });
        }
        let Some(by) = scale(bounds.size()) else {
            return Err(CommandError::Failed(
                "the selection has no area to resize. Select objects that are not flat lines"
                    .into(),
            ));
        };
        let t = about(bounds, by);
        Change::transform_selected(doc, |_| t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    cut_craft_core::contract_tests!(SetSize);

    use cut_craft_core::testing::{after, sample_document};

    fn bounds(p: &Params) -> cut_craft_core::geometry::Rect {
        after::<SetSize>(p).selection_bounds().unwrap()
    }

    #[test]
    fn percentages_and_items_one_by_one() {
        use cut_craft_core::testing::{document_with, square};
        let mut doc = document_with(vec![square(0.0, 0.0, 10.0), square(50.0, 0.0, 20.0)]);
        doc.selection = doc.vectors.keys().copied().collect();
        let p = Params {
            mode: Mode::ScaleItemsIndividually,
            width: 50.0,
            height: 50.0,
            percent: true,
            anchor: Anchor::BottomLeft,
            ..Params::default()
        };
        let change = SetSize::run(&doc, &p).unwrap();
        doc.apply(&change).unwrap();
        let sizes: Vec<f64> = doc.vectors.values().map(|v| v.bounds().size().x).collect();
        assert_eq!(sizes, [5.0, 10.0], "each halved, about its own corner");
        let corners: Vec<f64> = doc.vectors.values().map(|v| v.bounds().min.x).collect();
        assert_eq!(corners, [0.0, 50.0]);
        // as a selection, 200% doubles the whole group about its centre
        let p = Params {
            width: 200.0,
            height: 200.0,
            percent: true,
            ..Params::default()
        };
        let before = doc.selection_bounds().unwrap();
        doc.apply(&SetSize::run(&doc, &p).unwrap()).unwrap();
        let after = doc.selection_bounds().unwrap();
        assert!((after.size().x - 2.0 * before.size().x).abs() < 1e-9);
        assert!(after.center().distance(before.center()) < 1e-9);
    }

    #[test]
    fn keeping_proportions_scales_both_sides_by_the_width_ratio() {
        let b = bounds(&Params {
            width: 107.5,
            ..Params::default()
        });
        assert!(
            (b.size() - Point::new(107.5, 57.5)).length() < 1e-6,
            "{b:?}"
        );
    }

    #[test]
    fn free_resize_sets_each_side() {
        let b = bounds(&Params {
            width: 100.0,
            height: 50.0,
            keep_proportions: false,
            ..Params::default()
        });
        assert!(
            (b.size() - Point::new(100.0, 50.0)).length() < 1e-6,
            "{b:?}"
        );
    }

    #[test]
    fn the_anchor_point_stays_put() {
        let before = sample_document().selection_bounds().unwrap();
        let p = Params {
            width: 50.0,
            anchor: Anchor::BottomLeft,
            ..Params::default()
        };
        assert!((bounds(&p).min - before.min).length() < 1e-6);
    }

    #[test]
    fn rejects_a_zero_width() {
        assert_eq!(
            cut_craft_core::testing::after_err::<SetSize>(&Params {
                width: 0.0,
                ..Params::default()
            }),
            "Set width to more than 0."
        );
    }

    #[test]
    fn the_form_starts_from_the_selection_size() {
        let p = SetSize::initial_params(&sample_document());
        assert!(
            (p.width - 215.0).abs() < 1e-9 && (p.height - 115.0).abs() < 1e-9,
            "{p:?}"
        );
    }
}

#[cfg(test)]
mod component_tests {
    use super::*;
    use cut_craft_core::document::EntityId;
    use cut_craft_core::testing::{domed, run_on};

    #[test]
    fn sizes_a_selected_component_keeping_its_height() {
        let mut doc = domed();
        doc.selected_components = [EntityId(1)].into();
        let top = doc.components[&EntityId(1)].height_range().unwrap().1;
        let p = Params {
            width: 32.0,
            ..Params::default()
        };
        run_on::<SetSize>(&mut doc, &p);
        let k = &doc.components[&EntityId(1)];
        assert!((k.extent().size().x - 32.0).abs() < 0.3, "{:?}", k.extent());
        assert!((k.height_range().unwrap().1 - top).abs() < 0.2);
    }
}
