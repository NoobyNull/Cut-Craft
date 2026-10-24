//! Toolpath Properties: a toolpath's name, and whether it is shown and previewed.

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::toolpath::Toolpath;
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_named, existing};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct ToolpathProperties;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub toolpath: EntityId,
    pub name: String,
    /// Hidden toolpaths are not drawn or previewed
    pub visible: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            toolpath: EntityId(1),
            name: "Toolpath".into(),
            visible: true,
        }
    }
}

impl Command for ToolpathProperties {
    const ID: &'static str = "toolpath_properties";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Toolpath Properties",
            group: "Toolpaths",
            icon: "",
            shortcut: "",
        }
    }

    fn initial_params(doc: &Document) -> Params {
        doc.toolpaths
            .iter()
            .next()
            .map_or_else(Params::default, |(&toolpath, t)| Params {
                toolpath,
                name: t.name.clone(),
                visible: t.visible,
            })
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let toolpath = existing(&doc.toolpaths, p.toolpath, "toolpath")?;
        let after = Toolpath {
            name: ensure_named(&p.name)?,
            visible: p.visible,
            ..toolpath.clone()
        };
        let mut c = Change::builder(doc);
        c.replace_toolpath(p.toolpath, after);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{rejection, run_on, with_toolpath};

    cut_craft_core::contract_tests!(ToolpathProperties, with_toolpath);

    #[test]
    fn renames_and_hides_a_toolpath() {
        let mut doc = with_toolpath();
        let p = Params {
            name: " Outline ".into(),
            visible: false,
            ..ToolpathProperties::initial_params(&doc)
        };
        run_on::<ToolpathProperties>(&mut doc, &p);
        let t = &doc.toolpaths[&EntityId(1)];
        assert_eq!((t.name.as_str(), t.visible), ("Outline", false));
        assert_eq!(t.strategy, "profile_toolpath", "the rest is kept");
    }

    #[test]
    fn needs_the_toolpath_and_a_name() {
        assert!(rejection::<ToolpathProperties>(&Params::default()).contains("does not exist"));
        let blank = Params {
            name: " ".into(),
            ..Params::default()
        };
        let why = ToolpathProperties::run(&with_toolpath(), &blank)
            .unwrap_err()
            .to_string();
        assert!(why.contains("name is empty"), "{why}");
    }
}
