//! Level Properties: a level's name, how it combines with the levels below, whether it is
//! part of the model, its base height, mirror mode and (rotary jobs) wrapping.

use cut_craft_core::component::{Combine, Level, Mirror};
use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_named};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct LevelProperties;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    #[schemars(skip)]
    #[serde(default)]
    pub level: u64,
    pub name: String,
    #[schemars(title = "Combine Mode")]
    pub combine: Combine,
    /// Hidden levels are left out of the model
    pub visible: bool,
    /// Raises the whole level on a flat plane this thick
    #[schemars(title = "Base Height", extend("x-unit" = "mm"))]
    pub base_height: f64,
    #[schemars(title = "Mirror Mode")]
    pub mirror: Mirror,
    /// Rotary jobs: what runs off one side comes back on the other
    pub wrapping: bool,
}

impl Params {
    /// The settings level `id` has now, so a caller can change one and keep the rest.
    pub fn of(id: u64, l: &Level) -> Self {
        Self {
            level: id,
            name: l.name.clone(),
            combine: l.combine,
            visible: l.visible,
            base_height: l.base_height,
            mirror: l.mirror,
            wrapping: l.wrapping,
        }
    }
}

impl Default for Params {
    fn default() -> Self {
        Self::of(0, &Level::default())
    }
}

impl Command for LevelProperties {
    const ID: &'static str = "level_properties";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Level Properties",
            group: "Modeling",
            icon: "",
            shortcut: "",
        }
    }

    /// The level of the chosen component, else the first.
    fn initial_params(doc: &Document) -> Params {
        let id = doc
            .chosen_component()
            .map(|k| doc.components[&k].level)
            .filter(|l| doc.levels.contains_key(l))
            .or_else(|| doc.levels.keys().next().copied());
        id.map_or_else(Params::default, |id| Params::of(id, &doc.levels[&id]))
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let before = doc
            .levels
            .get(&p.level)
            .ok_or_else(|| CommandError::Failed(format!("level {} does not exist", p.level)))?;
        if p.wrapping && doc.job.rotary.is_none() {
            return Err(CommandError::InvalidParams(
                "wrapping is for rotary jobs only".into(),
            ));
        }
        let after = Level {
            name: ensure_named(&p.name)?,
            combine: p.combine,
            visible: p.visible,
            base_height: p.base_height,
            mirror: p.mirror,
            wrapping: p.wrapping,
            ..before.clone()
        };
        let mut c = Change::builder(doc);
        c.set_level(p.level, Some(after));
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::document::{Along, Job};
    use cut_craft_core::testing::{domed, run_on};

    cut_craft_core::contract_tests!(LevelProperties, domed);

    #[test]
    fn renames_recombines_and_mirrors_a_level() {
        let mut doc = domed();
        let p = Params {
            name: "Back".into(),
            combine: Combine::Merge,
            mirror: Mirror::TopToBottom,
            ..LevelProperties::initial_params(&doc)
        };
        run_on::<LevelProperties>(&mut doc, &p);
        let l = &doc.levels[&0];
        assert_eq!(
            (l.name.as_str(), l.combine, l.mirror),
            ("Back", Combine::Merge, Mirror::TopToBottom)
        );
    }

    #[test]
    fn wrapping_needs_a_rotary_job() {
        let mut doc = domed();
        let p = Params {
            wrapping: true,
            ..LevelProperties::initial_params(&doc)
        };
        assert!(LevelProperties::run(&doc, &p).is_err());
        doc.job = Job::rotary(40.0, 40.0 / std::f64::consts::PI, Along::X);
        run_on::<LevelProperties>(&mut doc, &p);
        assert!(doc.levels[&0].wrapping);
    }

    #[test]
    fn a_missing_level_is_refused() {
        let p = Params {
            level: 5,
            ..Params::default()
        };
        assert!(LevelProperties::run(&domed(), &p).is_err());
    }
}
