//! Cut Craft's own 3D clipart (`.ccclip`): Model ▸ Export as 3D Clipart saves the selected
//! components, baked into one with their shaping, as a component whose grid starts at the
//! origin; Import Component / 3D Model puts one back, centred on the job, on the first
//! level. (Aspire's `.3dclip` files are encrypted, so they are not read; see `re/SPEC.md`.)

use cut_craft_core::component::{Component, chosen_or_baked};
use cut_craft_core::document::Document;
use cut_craft_core::geometry::Point;
use cut_craft_core::{Change, CommandError, Exporter, FileFormat, Importer};
use serde::{Deserialize, Serialize};

/// What a clipart file holds.
#[derive(Serialize, Deserialize)]
struct Clip {
    /// Always [`FORMAT_NAME`]: says what the file is.
    format: String,
    version: u32,
    component: Component,
}

const FORMAT_NAME: &str = "cut-craft-clipart";
const VERSION: u32 = 1;
const CLIPART: FileFormat = FileFormat {
    label: "Cut Craft 3D clipart",
    extensions: &["ccclip"],
};

pub struct ExportClipart;

impl Exporter for ExportClipart {
    const ID: &'static str = "export_as_3d_clipart";
    const FORMAT: FileFormat = CLIPART;

    fn export(doc: &Document) -> Result<Vec<u8>, CommandError> {
        let (_, k) = chosen_or_baked(doc).ok_or_else(|| {
            CommandError::Failed("there is no component to export: create one first".into())
        })?;
        let mut component = k.baked();
        component.origin = Point::ZERO;
        component.level = 0;
        component.group = None;
        component.unsculpted = None;
        component.visible = true;
        let clip = Clip {
            format: FORMAT_NAME.into(),
            version: VERSION,
            component,
        };
        serde_json::to_vec(&clip).map_err(|e| CommandError::Failed(e.to_string()))
    }
}

pub struct ImportClipart;

impl Importer for ImportClipart {
    const ID: &'static str = "import_3d_clipart";
    const FORMAT: FileFormat = CLIPART;

    fn import(doc: &Document, bytes: &[u8]) -> Result<Change, CommandError> {
        let clip: Clip = serde_json::from_slice(bytes)
            .map_err(|e| CommandError::Failed(format!("not a Cut Craft clipart file: {e}")))?;
        if clip.format != FORMAT_NAME || clip.version > VERSION {
            return Err(CommandError::Failed(
                "not a clipart file this version of Cut Craft reads".into(),
            ));
        }
        let mut k = clip.component;
        if k.heights.len() != k.columns * k.rows {
            return Err(CommandError::Failed("the clipart file is damaged".into()));
        }
        k.origin = doc.job.area().center() - k.extent().size() / 2.0;
        k.level = doc.levels.keys().next().copied().unwrap_or(0);
        let mut c = Change::builder(doc);
        c.add_component(k);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::document::EntityId;
    use cut_craft_core::testing::domed;

    #[test]
    fn a_component_comes_back_with_its_shaped_heights_in_the_middle_of_the_job() {
        let mut doc = domed();
        let mut k = doc.components[&EntityId(1)].clone();
        k.shaping.shape_height = 50.0;
        let mut c = Change::builder(&doc);
        c.replace_component(EntityId(1), k.clone());
        doc.apply(&c.finish()).unwrap();
        let bytes = ExportClipart::export(&doc).unwrap();
        let mut other = Document::default();
        let change = ImportClipart::import(&other, &bytes).unwrap();
        other.apply(&change).unwrap();
        let back = other.components.values().next().unwrap();
        assert_eq!(
            back.height_range(),
            k.height_range(),
            "baked at half height"
        );
        assert!(back.shaping.is_identity());
        assert!((back.extent().center() - other.job.area().center()).length() < 1e-9);
    }

    #[test]
    fn other_files_and_no_component_are_refused() {
        assert!(ImportClipart::import(&Document::default(), b"{}").is_err());
        assert!(ImportClipart::import(&Document::default(), b"solid x").is_err());
        assert!(ExportClipart::export(&Document::default()).is_err());
    }
}
