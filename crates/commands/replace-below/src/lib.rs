//! Replace Below: the parts of the selected component lower than a height are removed or
//! flattened to it, and the component can be lowered by that height. Several selected
//! components are baked into one first.

use cut_craft_core::component::chosen_or_baked;
use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct ReplaceBelow;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReplaceWith {
    /// Remove those parts
    #[default]
    Transparent,
    /// Flatten them to the height
    Plane,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// The component's own heights lower than this are replaced
    #[schemars(title = "Replace value below", extend("x-unit" = "mm"))]
    pub below: f64,
    #[schemars(title = "Replace value with", extend("x-widget" = "row"))]
    pub replace_with: ReplaceWith,
    /// Drop what is left by the height, so it starts at the zero plane
    #[schemars(title = "Lower component by value")]
    pub lower: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            below: 0.0,
            replace_with: ReplaceWith::Transparent,
            lower: false,
        }
    }
}

impl Command for ReplaceBelow {
    const ID: &'static str = "replace_below";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Replace Below",
            group: "Modeling",
            icon: "modeling/replace_below.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let Some((ids, mut part)) = chosen_or_baked(doc) else {
            return Err(CommandError::Failed("select a component".into()));
        };
        let drop = if p.lower { p.below } else { 0.0 };
        for h in &mut part.heights {
            let z = *h;
            *h = if z.is_nan() {
                z
            } else if z < p.below {
                match p.replace_with {
                    ReplaceWith::Transparent => f64::NAN,
                    ReplaceWith::Plane => p.below - drop,
                }
            } else {
                z - drop
            };
        }
        if part.heights.iter().all(|h| h.is_nan()) {
            return Err(CommandError::Failed(
                "that would remove the whole component".into(),
            ));
        }
        let mut c = Change::builder(doc);
        for id in ids {
            c.remove_component(id);
        }
        c.add_component(part);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::component::Component;
    use cut_craft_core::geometry::Point;
    use cut_craft_core::testing::{domed, run_on, two_components};

    cut_craft_core::contract_tests!(ReplaceBelow, domed);

    fn only(doc: &Document) -> &Component {
        assert_eq!(doc.components.len(), 1);
        doc.components.values().next().unwrap()
    }

    #[test]
    fn transparent_cuts_the_dome_down_to_a_smaller_cap() {
        let mut doc = domed();
        let p = Params {
            below: 4.0,
            ..Params::default()
        };
        run_on::<ReplaceBelow>(&mut doc, &p);
        let dome = only(&doc);
        let (lo, hi) = dome.height_range().unwrap();
        assert!(lo >= 4.0 && (hi - 8.0).abs() < 0.1);
        // the dome is 4 high where r² = 48: beyond that it is gone
        assert!(dome.height_at(Point::new(27.5, 20.0)).is_none());
        assert!(dome.height_at(Point::new(20.0, 20.0)).is_some());
    }

    #[test]
    fn plane_flattens_and_lowering_drops_it_to_zero() {
        let mut doc = domed();
        let p = Params {
            below: 4.0,
            replace_with: ReplaceWith::Plane,
            lower: true,
        };
        run_on::<ReplaceBelow>(&mut doc, &p);
        let dome = only(&doc);
        assert!(dome.height_at(Point::new(27.5, 20.0)).unwrap().abs() < 1e-9);
        let top = dome.height_at(Point::new(20.1, 20.1)).unwrap();
        assert!((top - 4.0).abs() < 0.1, "8 high dropped by 4: {top}");
    }

    #[test]
    fn several_selected_components_are_baked_first() {
        let mut doc = two_components();
        for id in doc.components.keys().copied().collect::<Vec<_>>() {
            doc.selected_components.insert(id);
        }
        run_on::<ReplaceBelow>(
            &mut doc,
            &Params {
                below: 1.0,
                ..Params::default()
            },
        );
        assert_eq!(doc.components.len(), 1, "two became one");
    }

    #[test]
    fn removing_everything_is_refused() {
        let doc = domed();
        let p = Params {
            below: 100.0,
            ..Params::default()
        };
        let err = ReplaceBelow::run(&doc, &p).unwrap_err().to_string();
        assert!(err.contains("whole component"), "{err}");
        assert!(
            cut_craft_core::testing::rejection::<ReplaceBelow>(&Params::default())
                .contains("select")
        );
    }
}
