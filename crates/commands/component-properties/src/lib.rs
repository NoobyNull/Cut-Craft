//! Component Properties: a component's name, how it combines with the model under it,
//! whether it is part of the model, and its dynamic shape and base height, fade and tilt
//! (kept apart from its heights until baked).

use cut_craft_core::component::{self, Combine, Component, Shaping};
use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::params::Xy;
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_named, existing};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct ComponentProperties;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub component: EntityId,
    pub name: String,
    #[schemars(title = "Combine Mode")]
    pub combine: Combine,
    /// Hidden components are left out of the model
    pub visible: bool,
    /// Height as a percentage of the component's own
    #[schemars(title = "Shape Height", extend("x-unit" = "%"))]
    pub shape_height: f64,
    /// Raises the component on a flat plane this thick
    #[schemars(title = "Base Height", extend("x-unit" = "mm"))]
    pub base_height: f64,
    #[schemars(extend("x-section" = true))]
    pub fade: Fade,
    #[schemars(extend("x-section" = true))]
    pub tilt: Tilt,
    /// Make the shape and base height, fade and tilt permanent, then reset them
    pub bake: bool,
}

/// Fades the heights down from the first point toward the second.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Fade {
    #[schemars(title = "Fade")]
    pub on: bool,
    /// Keeps its height
    pub from: Xy,
    /// Faded down by the percentage
    pub to: Xy,
    #[schemars(extend("x-unit" = "%"))]
    pub percent: f64,
}

/// Tilts the component up about the first point toward the second.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Tilt {
    #[schemars(title = "Tilt")]
    pub on: bool,
    /// Does not move
    pub pivot: Xy,
    /// Raised by the angle
    pub toward: Xy,
    #[schemars(extend("x-unit" = "deg"))]
    pub angle: f64,
}

impl Default for Params {
    fn default() -> Self {
        let shaping = Shaping::default();
        Self {
            component: EntityId(1),
            name: "Component".into(),
            combine: Combine::Add,
            visible: true,
            shape_height: shaping.shape_height,
            base_height: shaping.base_height,
            fade: Fade {
                on: false,
                from: Xy::default(),
                to: Xy::default(),
                // ponytail: a starting value; Aspire's default fade is not documented
                percent: 50.0,
            },
            tilt: Tilt {
                on: false,
                pivot: Xy::default(),
                toward: Xy::default(),
                // ponytail: a starting value; Aspire's default tilt is not documented
                angle: 5.0,
            },
            bake: false,
        }
    }
}

impl Params {
    /// The settings `c` has now, so a caller can change one and keep the rest.
    pub fn of(component: EntityId, c: &Component) -> Self {
        let mut p = Params {
            component,
            name: c.name.clone(),
            combine: c.combine,
            visible: c.visible,
            shape_height: c.shaping.shape_height,
            base_height: c.shaping.base_height,
            ..Params::default()
        };
        // the anchors start across the component, so ticking Fade or Tilt shows something
        let e = c.extent();
        let (left, right) = (
            Xy::new(e.min.x, e.center().y),
            Xy::new(e.max.x, e.center().y),
        );
        p.fade.from = left;
        p.fade.to = right;
        p.tilt.pivot = left;
        p.tilt.toward = right;
        if let Some(f) = c.shaping.fade {
            p.fade = Fade {
                on: true,
                from: f.from,
                to: f.to,
                percent: f.percent,
            };
        }
        if let Some(t) = c.shaping.tilt {
            p.tilt = Tilt {
                on: true,
                pivot: t.pivot,
                toward: t.toward,
                angle: t.angle,
            };
        }
        p
    }

    fn shaping(&self) -> Shaping {
        Shaping {
            shape_height: self.shape_height,
            base_height: self.base_height,
            fade: self.fade.on.then_some(component::Fade {
                from: self.fade.from,
                to: self.fade.to,
                percent: self.fade.percent,
            }),
            tilt: self.tilt.on.then_some(component::Tilt {
                pivot: self.tilt.pivot,
                toward: self.tilt.toward,
                angle: self.tilt.angle,
            }),
        }
    }
}

impl Command for ComponentProperties {
    const ID: &'static str = "component_properties";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Component Properties",
            group: "Modeling",
            icon: "",
            shortcut: "",
        }
    }

    fn initial_params(doc: &Document) -> Params {
        doc.chosen_component()
            .map_or_else(Params::default, |id| Params::of(id, &doc.components[&id]))
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let component = existing(&doc.components, p.component, "component")?;
        if p.shape_height < 0.0 {
            return Err(CommandError::InvalidParams(
                "shape height cannot be negative".into(),
            ));
        }
        let mut after = Component {
            name: ensure_named(&p.name)?,
            combine: p.combine,
            visible: p.visible,
            shaping: p.shaping(),
            ..component.clone()
        };
        if p.bake {
            after = after.baked();
        }
        let mut c = Change::builder(doc);
        c.replace_component(p.component, after);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Point;
    use cut_craft_core::testing::{domed, rejection, run_on};

    cut_craft_core::contract_tests!(ComponentProperties, domed);

    #[test]
    fn renames_recombines_and_hides_a_component() {
        let mut doc = domed();
        let p = Params {
            name: "Bowl".into(),
            combine: Combine::Subtract,
            visible: false,
            ..ComponentProperties::initial_params(&doc)
        };
        run_on::<ComponentProperties>(&mut doc, &p);
        let c = &doc.components[&EntityId(1)];
        assert_eq!(
            (c.name.as_str(), c.combine, c.visible),
            ("Bowl", Combine::Subtract, false)
        );
    }

    fn top(doc: &Document) -> f64 {
        doc.components[&EntityId(1)].height_range().unwrap().1
    }

    #[test]
    fn shape_and_base_height_change_the_model_but_not_the_heights() {
        let mut doc = domed();
        let (raw, high) = (doc.components[&EntityId(1)].heights.clone(), top(&doc));
        let p = Params {
            shape_height: 50.0,
            base_height: 2.0,
            ..ComponentProperties::initial_params(&doc)
        };
        run_on::<ComponentProperties>(&mut doc, &p);
        assert!((top(&doc) - (high / 2.0 + 2.0)).abs() < 1e-9);
        assert_eq!(
            doc.components[&EntityId(1)].heights,
            raw,
            "kept until baked"
        );
        let again = ComponentProperties::initial_params(&doc);
        assert_eq!((again.shape_height, again.base_height), (50.0, 2.0));
    }

    #[test]
    fn bake_makes_the_shaping_permanent() {
        let mut doc = domed();
        let high = top(&doc);
        let p = Params {
            shape_height: 50.0,
            bake: true,
            ..ComponentProperties::initial_params(&doc)
        };
        run_on::<ComponentProperties>(&mut doc, &p);
        let k = &doc.components[&EntityId(1)];
        assert!(k.shaping.is_identity());
        assert!((top(&doc) - high / 2.0).abs() < 1e-9);
    }

    #[test]
    fn ticking_fade_lowers_the_far_side() {
        let mut doc = domed();
        let mut p = ComponentProperties::initial_params(&doc);
        p.fade.on = true;
        p.fade.percent = 100.0;
        run_on::<ComponentProperties>(&mut doc, &p);
        let k = &doc.components[&EntityId(1)];
        let (near, far) = (Point::from(p.fade.from), Point::from(p.fade.to));
        let probe = |at: Point| k.height_at(at + (k.extent().center() - at) * 0.4);
        assert!(probe(far).unwrap() < probe(near).unwrap());
        assert!(ComponentProperties::initial_params(&doc).fade.on);
    }

    #[test]
    fn negative_shape_height_is_refused() {
        let doc = domed();
        let p = Params {
            shape_height: -1.0,
            ..ComponentProperties::initial_params(&doc)
        };
        assert!(ComponentProperties::run(&doc, &p).is_err());
    }

    #[test]
    fn needs_the_component() {
        assert!(rejection::<ComponentProperties>(&Params::default()).contains("does not exist"));
    }
}
