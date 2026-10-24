//! Delete Toolpath.

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::{Change, Command, CommandError, Meta, existing};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct DeleteToolpath;

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

impl Command for DeleteToolpath {
    const ID: &'static str = "delete_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Delete Toolpath",
            group: "Toolpaths",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        existing(&doc.toolpaths, p.toolpath, "toolpath")?;
        let mut c = Change::builder(doc);
        c.remove_toolpath(p.toolpath);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{rejection, run_on, with_toolpath};

    cut_craft_core::contract_tests!(DeleteToolpath, with_toolpath);

    #[test]
    fn removes_the_toolpath() {
        let mut doc = with_toolpath();
        run_on::<DeleteToolpath>(&mut doc, &Params::default());
        assert!(doc.toolpaths.is_empty());
        assert!(rejection::<DeleteToolpath>(&Params::default()).contains("does not exist"));
    }
}
