//! Duplicate Toolpath: a copy added to the toolpath list, named with the next free number
//! after it ("Cut out (1)"), as Aspire's.

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::{Change, Command, CommandError, Meta, existing};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct DuplicateToolpath;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub toolpath: EntityId,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            toolpath: EntityId(1),
        }
    }
}

impl Command for DuplicateToolpath {
    const ID: &'static str = "duplicate_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Duplicate Toolpath",
            group: "Toolpaths",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let original = existing(&doc.toolpaths, p.toolpath, "toolpath")?;
        let taken = |name: &str| doc.toolpaths.values().any(|t| t.name == name);
        // n toolpaths take at most n of these n + 1 names
        let name = (1..=doc.toolpaths.len() + 1)
            .map(|n| format!("{} ({n})", original.name))
            .find(|name| !taken(name))
            .expect("some number is free");
        let mut c = Change::builder(doc);
        c.add_toolpath(cut_craft_core::toolpath::Toolpath {
            name,
            ..original.clone()
        });
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{rejection, run_on, with_toolpath};

    cut_craft_core::contract_tests!(DuplicateToolpath, with_toolpath);

    #[test]
    fn copies_the_toolpath_under_the_next_free_number() {
        let mut doc = with_toolpath();
        run_on::<DuplicateToolpath>(&mut doc, &Params::default());
        run_on::<DuplicateToolpath>(&mut doc, &Params::default());
        let names: Vec<&str> = doc.toolpaths.values().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["Profile", "Profile (1)", "Profile (2)"]);
        let (first, copy) = (&doc.toolpaths[&EntityId(1)], &doc.toolpaths[&EntityId(2)]);
        assert_eq!(
            (&copy.strategy, &copy.moves),
            (&first.strategy, &first.moves)
        );
    }

    #[test]
    fn needs_a_toolpath() {
        assert!(
            rejection::<DuplicateToolpath>(&Params::default())
                .contains("toolpath 1 does not exist")
        );
    }
}
