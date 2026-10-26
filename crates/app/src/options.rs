//! Edit ▸ Options: settings kept between runs, in Aspire's sections. Only the settings Cut
//! Craft acts on are here: what the views draw when it starts, and the recent files kept.

use crate::display::Display;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Aspire's 3D View Settings: what the 3D view draws when Cut Craft starts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the dialog's own check boxes"
)]
pub struct View3d {
    pub draw_origin: bool,
    pub draw_material_block: bool,
    #[schemars(title = "Use color shaded view")]
    pub color_shaded_view: bool,
    #[schemars(title = "Use shaded background")]
    pub shaded_background: bool,
    pub light_follows_viewer: bool,
}

/// Aspire's Toolpath Settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ToolpathSettings {
    /// Swap to the 3D view after calculating a toolpath
    #[schemars(title = "Auto open 3D view")]
    pub auto_open_3d_view: bool,
    /// Draw toolpaths in the 2D view
    #[schemars(title = "Create 2D previews")]
    pub create_2d_previews: bool,
    /// Draw them as the area the tool covers
    #[schemars(title = "Solid 2D previews")]
    pub solid_2d_previews: bool,
}

/// Aspire's General Settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct General {
    /// How many files File ▸ Recent Files and the start page list
    #[schemars(range(min = 1, max = 20))]
    pub recent_file_list_size: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Options {
    #[schemars(title = "3D View Settings", extend("x-section" = true))]
    pub view_3d: View3d,
    #[schemars(title = "Toolpath Settings", extend("x-section" = true))]
    pub toolpaths: ToolpathSettings,
    #[schemars(title = "General Settings", extend("x-section" = true))]
    pub general: General,
}

impl Default for Options {
    /// What the views draw without settings: the display's own starting state.
    fn default() -> Self {
        let d = Display::default();
        Self {
            view_3d: View3d {
                draw_origin: d.origin,
                draw_material_block: d.material_block,
                color_shaded_view: d.colour_shaded,
                shaded_background: d.shaded_background,
                light_follows_viewer: d.light_follows,
            },
            toolpaths: ToolpathSettings {
                auto_open_3d_view: d.auto_open_3d,
                create_2d_previews: d.previews_2d,
                solid_2d_previews: d.solid_previews,
            },
            general: General {
                recent_file_list_size: u32::try_from(crate::files::RECENT_COUNT)
                    .expect("a small count"),
            },
        }
    }
}

impl Options {
    /// The options kept in the config folder; the defaults when there are none.
    pub fn load() -> Self {
        path()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let Some(path) = path() else {
            return Ok(());
        };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        let json = serde_json::to_vec_pretty(self).expect("options serialise");
        std::fs::write(&path, json).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Sets what the views draw to these options.
    pub fn apply_to(&self, d: &mut Display) {
        let (v, t) = (&self.view_3d, &self.toolpaths);
        d.origin = v.draw_origin;
        d.material_block = v.draw_material_block;
        d.colour_shaded = v.color_shaded_view;
        d.shaded_background = v.shaded_background;
        d.light_follows = v.light_follows_viewer;
        d.auto_open_3d = t.auto_open_3d_view;
        d.previews_2d = t.create_2d_previews;
        d.solid_previews = t.solid_2d_previews;
    }

    /// How many recent files are kept.
    pub fn recent_count(&self) -> usize {
        usize::try_from(self.general.recent_file_list_size.max(1)).unwrap_or(1)
    }
}

fn path() -> Option<PathBuf> {
    crate::files::config_dir().map(|dir| dir.join("options.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_leave_the_display_as_it_starts() {
        let mut d = Display::default();
        Options::default().apply_to(&mut d);
        assert_eq!(d, Display::default());
    }

    #[test]
    fn options_set_what_the_views_draw() {
        let mut o = Options::default();
        o.view_3d.draw_origin = false;
        o.toolpaths.solid_2d_previews = true;
        let mut d = Display::default();
        o.apply_to(&mut d);
        assert!(!d.origin && d.solid_previews);
    }

    #[test]
    fn they_load_back_as_saved_and_fill_gaps_with_defaults() {
        let o: Options =
            serde_json::from_str(&serde_json::to_string(&Options::default()).unwrap()).unwrap();
        assert_eq!(o, Options::default());
        let fields =
            crate::form::fields(&serde_json::to_value(schemars::schema_for!(Options)).unwrap());
        let sections: Vec<&str> = fields
            .iter()
            .filter(|f| f.section)
            .map(|f| f.label.as_str())
            .collect();
        assert_eq!(
            sections,
            ["3D View Settings", "Toolpath Settings", "General Settings"]
        );
    }
}
