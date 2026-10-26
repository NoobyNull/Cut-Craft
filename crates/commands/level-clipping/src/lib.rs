//! Level Clipping: a level kept only inside the selected closed vectors, without changing
//! its components. Update reads the vectors again (the selected ones, else the ones it was
//! made from); Remove takes the clipping off.

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct LevelClipping;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Clip to the selected closed vectors
    #[default]
    Apply,
    /// Clip again to the selected vectors, else the ones used last
    Update,
    /// No clipping
    Remove,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    #[schemars(skip)]
    #[serde(default)]
    pub level: u64,
    pub action: Action,
}

impl Command for LevelClipping {
    const ID: &'static str = "level_clipping";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Level Clipping",
            group: "Modeling",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let mut level = doc
            .levels
            .get(&p.level)
            .ok_or_else(|| CommandError::Failed(format!("level {} does not exist", p.level)))?
            .clone();
        let closed = |ids: &[EntityId]| -> Vec<EntityId> {
            ids.iter()
                .copied()
                .filter(|id| doc.vectors.get(id).is_some_and(|v| v.closed))
                .collect()
        };
        let selected = closed(&doc.selection.iter().copied().collect::<Vec<_>>());
        let ids = match p.action {
            Action::Remove => vec![],
            Action::Update if selected.is_empty() => closed(&level.clip_vectors),
            Action::Apply | Action::Update => selected,
        };
        if ids.is_empty() && p.action != Action::Remove {
            return Err(CommandError::Failed(
                "select the closed vectors to clip the level to".into(),
            ));
        }
        level.clip = ids.iter().map(|id| doc.vectors[id].clone()).collect();
        level.clip_vectors = ids;
        let mut c = Change::builder(doc);
        c.set_level(p.level, Some(level));
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::component::model;
    use cut_craft_core::geometry::Point;
    use cut_craft_core::testing::{domed, run_on, square};

    /// The dome with a square over its left half, selected.
    fn dome_and_square() -> Document {
        let mut doc = domed();
        let mut c = Change::builder(&doc);
        let id = c.add(square(10.0, 10.0, 10.0));
        c.select([id]);
        doc.apply(&c.finish()).unwrap();
        doc
    }

    cut_craft_core::contract_tests!(LevelClipping, dome_and_square);

    fn height(doc: &Document, x: f64) -> f64 {
        model(doc, 200).at(Point::new(x, 15.0)).unwrap()
    }

    #[test]
    fn apply_update_and_remove() {
        let mut doc = dome_and_square();
        assert!(height(&doc, 22.0) > 1.0);
        run_on::<LevelClipping>(&mut doc, &Params::default());
        assert!(height(&doc, 18.0) > 1.0, "inside the square");
        assert!(height(&doc, 22.0).abs() < 1e-9, "clipped off");
        // the square moves right; Update with nothing selected follows it
        let mut c = Change::builder(&doc);
        c.replace(EntityId(2), square(20.0, 10.0, 10.0));
        c.select([]);
        doc.apply(&c.finish()).unwrap();
        run_on::<LevelClipping>(
            &mut doc,
            &Params {
                action: Action::Update,
                ..Params::default()
            },
        );
        assert!(height(&doc, 22.0) > 1.0);
        assert!(height(&doc, 18.0).abs() < 1e-9);
        run_on::<LevelClipping>(
            &mut doc,
            &Params {
                action: Action::Remove,
                ..Params::default()
            },
        );
        assert!(height(&doc, 18.0) > 1.0 && height(&doc, 22.0) > 1.0);
    }

    #[test]
    fn apply_needs_closed_vectors() {
        assert!(LevelClipping::run(&domed(), &Params::default()).is_err());
    }
}
