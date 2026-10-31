//! Set Size.

use cut_craft_core::document::Document;
use cut_craft_core::ensure_positive;
use cut_craft_core::geometry::Point;
use cut_craft_core::glam::DAffine2;
use cut_craft_core::params::Anchor;
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct SetSize;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub width: f64,
    /// Ignored when keeping proportions
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub height: f64,
    pub keep_proportions: bool,
    /// Point of the selection that stays put
    pub anchor: Anchor,
    /// Scales by this much, keeping proportions, instead of to the width and height (100: off)
    #[schemars(extend("x-unit" = "%"), range(min = 0))]
    pub percent: f64,
    /// Each selected item is scaled about its own anchor point rather than the selection's
    pub scale_individually: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            width: 100.0,
            height: 100.0,
            keep_proportions: true,
            anchor: Anchor::Center,
            percent: 100.0,
            scale_individually: false,
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
        ensure_positive("percent", p.percent)?;
        let by_percent = (p.percent - 100.0).abs() > f64::EPSILON;
        if !p.keep_proportions && !by_percent {
            ensure_positive("height", p.height)?;
        }
        let bounds = doc
            .selection_bounds()
            .ok_or(CommandError::NothingSelected)?;
        let size = bounds.size();
        if size.x <= 0.0 || size.y <= 0.0 {
            return Err(CommandError::Failed(
                "the selection has no area to resize".into(),
            ));
        }
        let sx = p.width / size.x;
        let scale = if by_percent {
            Point::splat(p.percent / 100.0)
        } else if p.keep_proportions {
            Point::splat(sx)
        } else {
            Point::new(sx, p.height / size.y)
        };
        let resize = |a: Point| {
            DAffine2::from_translation(a)
                * DAffine2::from_scale(scale)
                * DAffine2::from_translation(-a)
        };
        let whole = resize(p.anchor.point(bounds));
        Change::transform_selected(doc, |item| {
            if p.scale_individually {
                resize(p.anchor.point(item))
            } else {
                whole
            }
        })
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
            "invalid parameters: width must be greater than 0"
        );
    }

    #[test]
    fn a_percentage_scales_both_sides() {
        let b = bounds(&Params {
            percent: 50.0,
            ..Params::default()
        });
        assert!(
            (b.size() - Point::new(107.5, 57.5)).length() < 1e-6,
            "{b:?}"
        );
    }

    #[test]
    fn items_scaled_individually_keep_their_own_centres() {
        let doc = sample_document();
        assert!(doc.selection.len() > 1, "the sample selects several items");
        let after = after::<SetSize>(&Params {
            percent: 50.0,
            scale_individually: true,
            ..Params::default()
        });
        for id in &doc.selection {
            let (was, is) = (doc.vectors[id].bounds(), after.vectors[id].bounds());
            assert!(
                (was.center() - is.center()).length() < 1e-6,
                "{was:?} -> {is:?}"
            );
            assert!(
                (was.size() * 0.5 - is.size()).length() < 1e-6,
                "{was:?} -> {is:?}"
            );
        }
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
