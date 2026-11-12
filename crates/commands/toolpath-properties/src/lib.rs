//! Toolpath Properties: a toolpath's name, whether it is shown and previewed, and its
//! colour in a Toolpath Color preview.

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::toolpath::{PREVIEW_COLOUR, SUMMARY_NOTES, Toolpath};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_named, existing};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct ToolpathProperties;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// The toolpath these settings belong to
    pub toolpath: EntityId,
    /// What the toolpath is called in the toolpath list
    pub name: String,
    /// Hidden toolpaths are not drawn or previewed
    pub visible: bool,
    /// Its colour in a Toolpath Color preview (the toolpath list's swatch sets it)
    #[serde(default)]
    #[schemars(skip)]
    pub preview: PreviewFill,
    /// Its notes for the Toolpath Summary, when they are being set
    #[serde(default)]
    #[schemars(skip)]
    pub notes: Option<String>,
}

/// What Toolpath Properties does to the toolpath's preview colour.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewFill {
    /// Keeps the colour it has
    #[default]
    Unchanged,
    /// The material's colour: its areas are not filled
    NoFill,
    Colour([u8; 3]),
}

impl Default for Params {
    fn default() -> Self {
        Self {
            toolpath: EntityId(1),
            name: "Toolpath".into(),
            visible: true,
            preview: PreviewFill::Unchanged,
            notes: None,
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
                preview: PreviewFill::Unchanged,
                notes: None,
            })
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let toolpath = existing(&doc.toolpaths, p.toolpath, "toolpath")?;
        let mut after = Toolpath {
            name: ensure_named(&p.name)?,
            visible: p.visible,
            ..toolpath.clone()
        };
        match p.preview {
            PreviewFill::Unchanged => {}
            PreviewFill::NoFill => after.params[PREVIEW_COLOUR] = serde_json::Value::Null,
            PreviewFill::Colour(c) => after.params[PREVIEW_COLOUR] = serde_json::json!(c),
        }
        if let Some(notes) = &p.notes {
            after.params[SUMMARY_NOTES] = serde_json::json!(notes);
        }
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
    fn sets_the_preview_colour_or_no_fill() {
        let mut doc = with_toolpath();
        let t = &doc.toolpaths[&EntityId(1)];
        assert_eq!(
            t.preview_colour(1),
            Some([60, 140, 220]),
            "the palette's by place"
        );
        let start = ToolpathProperties::initial_params(&doc);
        let p = |preview| Params {
            preview,
            ..start.clone()
        };
        run_on::<ToolpathProperties>(&mut doc, &p(PreviewFill::Colour([1, 2, 3])));
        assert_eq!(
            doc.toolpaths[&EntityId(1)].preview_colour(1),
            Some([1, 2, 3])
        );
        run_on::<ToolpathProperties>(&mut doc, &p(PreviewFill::Unchanged));
        assert_eq!(
            doc.toolpaths[&EntityId(1)].preview_colour(1),
            Some([1, 2, 3])
        );
        run_on::<ToolpathProperties>(&mut doc, &p(PreviewFill::NoFill));
        assert_eq!(doc.toolpaths[&EntityId(1)].preview_colour(1), None);
        let noted = Params {
            notes: Some("check the clamps".into()),
            ..start.clone()
        };
        run_on::<ToolpathProperties>(&mut doc, &noted);
        assert_eq!(
            doc.toolpaths[&EntityId(1)].summary_notes(),
            "check the clamps"
        );
        run_on::<ToolpathProperties>(&mut doc, &start);
        assert_eq!(
            doc.toolpaths[&EntityId(1)].summary_notes(),
            "check the clamps",
            "kept when not being set"
        );
    }

    #[test]
    fn needs_the_toolpath_and_a_name() {
        assert!(
            rejection::<ToolpathProperties>(&Params::default())
                .contains("is not in the job any more")
        );
        let blank = Params {
            name: " ".into(),
            ..Params::default()
        };
        let why = ToolpathProperties::run(&with_toolpath(), &blank)
            .unwrap_err()
            .to_string();
        assert!(why.contains("Type a name"), "{why}");
    }
}
