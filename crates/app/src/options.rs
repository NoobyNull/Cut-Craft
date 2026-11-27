//! Edit ▸ Options: settings kept between runs, in the sections. Only the settings Cut
//! Craft acts on are here: what the views draw when it starts, and the recent files kept.

use crate::display::Display;
use eframe::egui;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
mod appearance;
pub use appearance::*;
mod services;
pub use services::*;
mod advanced;
pub use advanced::*;
mod toolpaths;
pub use toolpaths::*;
mod view;
pub use view::*;
mod performance;
pub use performance::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Options {
    #[schemars(title = "3D View Settings", extend("x-section" = true))]
    pub view_3d: View3d,
    #[schemars(title = "Toolpath Settings", extend("x-section" = true))]
    pub toolpaths: ToolpathSettings,
    #[schemars(title = "General Settings", extend("x-section" = true))]
    pub general: General,
    #[serde(default)]
    #[schemars(title = "Performance", extend("x-section" = true))]
    pub performance: Performance,
    /// The language Cut Craft's words are in: English, built in, or a language pack
    /// installed from Gadgets ▸ Add-ons.
    #[serde(default)]
    #[schemars(title = "Language", extend("x-section" = true))]
    pub language: Language,
    /// How Cut Craft starts, and what every new job starts with: the material, where X0 Y0
    /// and Z zero are, the rapid heights and home (the job settings under it).
    #[serde(default)]
    #[schemars(title = "Job Defaults", extend("x-section" = true))]
    pub job_defaults: JobDefaults,
    #[serde(default)]
    #[schemars(title = "Confirmation Choices", extend("x-section" = true))]
    pub confirmations: Confirmations,
    #[serde(default)]
    #[schemars(title = "Importing and Exporting", extend("x-section" = true))]
    pub import: Import,
    #[serde(default)]
    #[schemars(extend("x-section" = true))]
    pub appearance: Appearance,
    #[serde(default)]
    #[schemars(title = "Mouse and Shortcuts", extend("x-section" = true))]
    pub controls: Controls,
    #[serde(default)]
    #[schemars(title = "SpaceMouse", extend("x-section" = true))]
    pub space_mouse: SpaceMouse,
    #[serde(default)]
    #[schemars(title = "Updates", extend("x-section" = true))]
    pub updates: Updates,
    #[serde(default)]
    #[schemars(title = "Backups", extend("x-section" = true))]
    pub backups: Backups,
    #[serde(default)]
    #[schemars(title = "Bug Reports", extend("x-section" = true))]
    pub bug_reports: BugReports,
    #[serde(default)]
    #[schemars(title = "Web API", extend("x-section" = true))]
    pub web_api: WebApi,
    #[serde(default)]
    #[schemars(extend("x-section" = true))]
    pub advanced: Advanced,
    /// Snapping Options (F4): what the pointer snaps to, nudge distances and rotate snap
    /// angles, kept from one run to the next.
    #[serde(default)]
    #[schemars(skip)]
    pub snapping: cut_craft_core::snap::SnapOptions,
    /// Gadgets ▸ Job Setup Sheet Editor: how job sheets look.
    #[serde(default)]
    #[schemars(skip)]
    pub job_sheet: fmt_create_job_sheet::Style,
    /// Toolpaths ▸ Print Part Labels: the labels' size and what they show.
    #[serde(default)]
    #[schemars(skip)]
    pub part_labels: fmt_part_labels::Style,
    /// Export as STL: how the model is triangulated.
    #[serde(default)]
    #[schemars(skip)]
    pub stl: fmt_export_as_stl::Settings,
    /// What Job Setup starts a new job with, once its Set as Default is pressed; Cut Craft's
    /// own starting job until then. Set as Default changes only what is on its form, so the
    /// clearing kept by Clearing Settings' Use as My Default stays.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(skip)]
    pub default_job: Option<cut_craft_core::document::Job>,
    /// First-time setup was answered or skipped, so it is not shown at start again (Help ▸
    /// Run First-Time Setup Again still opens it). Kept here, in the options file, wherever
    /// the platform keeps that.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[schemars(skip)]
    pub first_run_done: bool,
    /// Whether the window layout question was answered (or closed), in first-time setup or
    /// once after updating to a version that asks it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[schemars(skip)]
    pub layout_asked: bool,
    /// No longer read (startup picks an unseen tip by weight); kept so settings saved by
    /// older versions load without a word about it.
    #[serde(default, skip_serializing)]
    #[schemars(skip)]
    pub next_tip: usize,
    /// The Did You Know? tips shown on this computer, by `Tip::key`, so each start shows one
    /// not seen yet; emptied once all have been.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(skip)]
    pub seen_tips: Vec<u64>,
    /// The announcements shown on this computer, and those turned off with Don't show this
    /// again.
    #[serde(default, skip_serializing_if = "crate::announcements::Seen::is_empty")]
    #[schemars(skip)]
    pub announcements: crate::announcements::Seen,
    /// The favourite tools of the Design and Toolpaths tabs, and whether each shows only
    /// them (right-click the tab).
    #[serde(default)]
    #[schemars(skip)]
    pub favourites: Favourites,
    /// The Design sub-tabs shown as sections of their own under the tab row rather than in
    /// it, by name (right-click a sub-tab). None to start with: every one in the row.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(skip)]
    pub design_sections: Vec<String>,
    /// The Toolpaths sub-tabs shown as sections of their own, as `design_sections`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(skip)]
    pub toolpath_sections: Vec<String>,
    /// The tool each toolbox flyout shows: the one picked from it last.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    #[schemars(skip)]
    pub flyout_faces: crate::flyout::Faces,
    /// The tools pinned to the Ribbon (`true`) or taken off it (`false`) by hand, by id;
    /// any other tool is on it when it is one of its group's first few. A group's list has
    /// every tool either way.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    #[schemars(skip)]
    pub ribbon_pins: std::collections::BTreeMap<String, bool>,
    /// The way each shape drawn more than one way was last drawn, picked under Draw from
    /// in its form: the shape's command, then the tool that way is
    /// ([`crate::flyout::WAYS`]). Its button opens that way again.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    #[schemars(skip)]
    pub shape_ways: std::collections::BTreeMap<String, String>,
    /// Toolbars of your own (right-click a tool or group ▸ Add to Toolbar), in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(skip)]
    pub toolbars: Vec<crate::custom_toolbars::Toolbar>,
    /// Where the Canvas layout's palette of tool groups sits (right-click its grip).
    #[serde(
        default = "crate::bars::Place::column",
        skip_serializing_if = "crate::bars::Place::is_column"
    )]
    #[schemars(skip)]
    pub canvas_palette: crate::bars::Place,
    /// Where the Canvas layout's toolpaths bar sits (right-click its grip).
    #[serde(
        default = "crate::bars::Place::column",
        skip_serializing_if = "crate::bars::Place::is_column"
    )]
    #[schemars(skip)]
    pub canvas_toolpaths: crate::bars::Place,
}

/// Each tool page's favourite tools (and whole groups, as `group:<name>`), in the order
/// shown, and whether it shows only those; and the Favourites bar: its name, if renamed, and
/// where it sits.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Favourites {
    /// What it was renamed to; Favourites (in the language in use) until then.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Where the Favourites bar sits: floating over the drawing at first.
    #[serde(default)]
    pub bar: crate::bars::Place,
    #[serde(default)]
    pub design: Vec<String>,
    #[serde(default)]
    pub toolpaths: Vec<String>,
    #[serde(default)]
    pub design_only: bool,
    #[serde(default)]
    pub toolpaths_only: bool,
}

impl Favourites {
    /// `page`'s favourites and whether it shows only them.
    pub fn of(&self, page: crate::pages::Page) -> (&[String], bool) {
        match page {
            crate::pages::Page::Design => (&self.design, self.design_only),
            crate::pages::Page::Toolpaths => (&self.toolpaths, self.toolpaths_only),
        }
    }

    pub fn of_mut(&mut self, page: crate::pages::Page) -> (&mut Vec<String>, &mut bool) {
        match page {
            crate::pages::Page::Design => (&mut self.design, &mut self.design_only),
            crate::pages::Page::Toolpaths => (&mut self.toolpaths, &mut self.toolpaths_only),
        }
    }
}

impl Default for Options {
    /// What the views draw without settings: the display's own starting state.
    fn default() -> Self {
        let d = Display::default();
        Self {
            view_3d: View3d {
                draw_origin: d.origin,
                draw_material_block: d.material_block,
                draw_vectors: d.vectors_3d,
                color_shaded_view: d.colour_shaded,
                shaded_background: d.shaded_background,
                light_follows_viewer: d.light_follows,
                background_style: BackgroundStyle::default(),
                background_color: background_top(),
                gradient_background_color: background_bottom(),
                image_file_path: String::new(),
                anti_aliasing: AntiAliasing::default(),
                simplify_while_moving: false,
                rotation: Rotation::default(),
                click_reach: click_reach(),
                perspective_angle: perspective_angle(),
                settle_seconds: settle_seconds(),
                spin_speed: SPIN_SPEED,
                cube_style: CubeStyle::default(),
                cube_size: CubeSize::default(),
                smooth_turns: true,
                home_view: None,
            },
            toolpaths: ToolpathSettings {
                auto_open_3d_view: d.auto_open_3d,
                create_2d_previews: d.previews_2d,
                solid_2d_previews: d.solid_previews,
                solid_2d_preview_color: d.solid_colour,
                tolerance_2d: tolerance(),
                tolerance_3d: tolerance(),
                tolerance_vcarve: tolerance(),
                preview_multiplier: preview_speed(),
                keep_out_labels: d.keep_out_labels,
                clip_at_keep_out_zones: false,
                change_by_hand: change_by_hand(),
                change_by_changer: change_by_changer(),
                file_name_underscores: false,
                bit_in_file_names: true,
                advanced_rotary: false,
            },
            language: Language::default(),
            performance: Performance::default(),
            general: General {
                recent_file_list_size: u32::try_from(crate::files::RECENT_COUNT)
                    .expect("a small count"),
                preload_icons: on(),
                file_dialog_default: FileDialogDefault::default(),
                layout: WindowLayout::default(),
                autosave_seconds: autosave_seconds(),
                flyout_hold_seconds: flyout_hold_seconds(),
                fold_tool_groups: false,
                ribbon_icons: RibbonIcons::Compact,
                other_ways: OtherWays::InTheForm,
                canvas_flyouts: CanvasFlyouts::OnHover,
                canvas_modeling_bar: true,
                canvas_toolpaths_bar: true,
                snapped_bars: SnappedBars::PushTheDrawingOver,
                new_bars: NewBars::Floating,
                add_to_menus: AddToMenus::EachToolbar,
                show_tips_at_startup: true,
            },
            job_defaults: JobDefaults::default(),
            confirmations: Confirmations::default(),
            snapping: cut_craft_core::snap::SnapOptions::default(),
            job_sheet: fmt_create_job_sheet::Style::default(),
            part_labels: fmt_part_labels::Style::default(),
            stl: fmt_export_as_stl::Settings::default(),
            import: Import::default(),
            appearance: Appearance::default(),
            advanced: Advanced::default(),
            controls: Controls::default(),
            space_mouse: SpaceMouse::default(),
            updates: Updates::default(),
            backups: Backups::default(),
            bug_reports: BugReports::default(),
            web_api: WebApi::default(),
            default_job: None,
            first_run_done: false,
            layout_asked: false,
            next_tip: 0,
            seen_tips: Vec::new(),
            announcements: crate::announcements::Seen::default(),
            favourites: Favourites::default(),
            design_sections: vec![],
            toolpath_sections: vec![],
            flyout_faces: crate::flyout::Faces::new(),
            ribbon_pins: std::collections::BTreeMap::new(),
            shape_ways: std::collections::BTreeMap::new(),
            toolbars: Vec::new(),
            canvas_palette: crate::bars::Place::column(),
            canvas_toolpaths: crate::bars::Place::column(),
        }
    }
}

impl Options {
    /// Hands the settings that code outside the app reads (the calculation limit,
    /// importing) to it.
    pub fn apply(&self) {
        self.performance.apply();
        self.advanced.apply();
        self.import.apply();
    }

    /// The options kept in the config folder; the defaults when there are none.
    pub fn load() -> Self {
        Self::load_checked().options
    }

    /// [`Self::load`], and what could not be read of it: whether the advanced settings had
    /// been changed outside Cut Craft (and so were put back to their defaults), and the
    /// settings left at their defaults. A file with settings left out is first copied to
    /// `options.json.bak`, so saving the defaults over it loses nothing.
    pub fn load_checked() -> Read {
        let Some(path) = path() else {
            return Read::default();
        };
        let Ok(bytes) = std::fs::read(&path) else {
            return Read::default();
        };
        let read = Self::read_bytes(&bytes);
        if !read.unread.is_empty() {
            let _ = std::fs::write(path.with_extension("json.bak"), &bytes);
        }
        read
    }

    /// Options read from a file's bytes, the advanced ones only if their check matches; the
    /// defaults for a file that is not options at all.
    #[cfg(test)]
    pub fn from_bytes(bytes: &[u8]) -> (Self, bool) {
        let read = Self::read_bytes(bytes);
        (read.options, read.tampered)
    }

    /// [`Self::from_bytes`], naming what could not be read. A setting this version cannot
    /// read (one from a newer version, or a hand edit gone wrong) goes back to its default
    /// on its own: the page it is on, and the rest of the file, are kept.
    pub fn read_bytes(bytes: &[u8]) -> Read {
        let Ok(mut file) = serde_json::from_slice::<serde_json::Value>(bytes) else {
            return Read {
                unread: vec!["everything".to_owned()],
                ..Read::default()
            };
        };
        let seal = file
            .as_object_mut()
            .and_then(|o| o.remove(SEAL))
            .and_then(|s| s.as_str().map(String::from));
        // the one grid colour of older versions: kept for the light theme if it was chosen
        let old_grid = file
            .get_mut("appearance")
            .and_then(serde_json::Value::as_object_mut)
            .and_then(|a| a.remove("grid_color"))
            .and_then(|g| serde_json::from_value::<[u8; 3]>(g).ok());
        // each theme's colours from before line colours: the theme's own filled in
        if let Some(appearance) = file
            .get_mut("appearance")
            .and_then(serde_json::Value::as_object_mut)
        {
            for (key, theme) in [
                ("light_2d", CanvasColours::light()),
                ("dark_2d", CanvasColours::dark()),
            ] {
                let Some(saved) = appearance
                    .get_mut(key)
                    .and_then(serde_json::Value::as_object_mut)
                else {
                    continue;
                };
                if let serde_json::Value::Object(defaults) =
                    serde_json::to_value(theme).expect("colours serialise")
                {
                    for (name, value) in defaults {
                        saved.entry(name).or_insert(value);
                    }
                }
            }
        }
        carry_over(&mut file);
        // options saved before the layout setting had the Classic window: said here, so a
        // file read setting by setting keeps it too
        if let Some(general) = file
            .get_mut("general")
            .and_then(serde_json::Value::as_object_mut)
        {
            general.entry("layout").or_insert_with(|| {
                serde_json::to_value(WindowLayout::before_the_setting()).expect("layout serialises")
            });
        }
        let (mut options, unread) = match serde_json::from_value::<Self>(file.clone()) {
            Ok(options) => (options, vec![]),
            Err(_) => Self::read_each(file),
        };
        if let Some(grid) = old_grid.filter(|g| *g != old_grid_color()) {
            options.appearance.light_2d.grid_color = grid;
        }
        let tampered = !options.advanced.sealed_by(seal.as_deref());
        if tampered {
            options.advanced = Advanced::default();
        }
        Read {
            options,
            tampered,
            unread,
        }
    }

    /// A file that does not read whole, read setting by setting onto the defaults: each
    /// section kept if it reads, else each of its settings that does; and what was left out.
    fn read_each(file: serde_json::Value) -> (Self, Vec<String>) {
        let reads = |v: &serde_json::Value| serde_json::from_value::<Self>(v.clone()).is_ok();
        let mut kept = serde_json::to_value(Self::default()).expect("options serialise");
        let mut unread = vec![];
        let serde_json::Value::Object(sections) = file else {
            return (Self::default(), vec!["everything".to_owned()]);
        };
        for (section, value) in sections {
            let mut trial = kept.clone();
            trial[&section] = value.clone();
            if reads(&trial) {
                kept = trial;
                continue;
            }
            let (Some(settings), true) = (value.as_object(), kept[&section].is_object()) else {
                unread.push(section);
                continue;
            };
            for (name, setting) in settings {
                let mut trial = kept.clone();
                trial[&section][name] = setting.clone();
                if reads(&trial) {
                    kept = trial;
                } else {
                    unread.push(format!("{section} ▸ {name}"));
                }
            }
        }
        let options = serde_json::from_value(kept).unwrap_or_default();
        (options, unread)
    }

    /// These options with one page (`general`, `advanced`…) back to its defaults, as its
    /// Reset to defaults button does; anything else kept.
    #[must_use]
    pub fn reset(&self, page: &str) -> Self {
        let mut values = serde_json::to_value(self).expect("options serialise");
        let defaults = serde_json::to_value(Self::default()).expect("options serialise");
        if let Some(default) = defaults.get(page) {
            values[page] = default.clone();
        }
        // the Job Defaults page shows the job new jobs start with: back to Cut Craft's own
        if page == JOB_DEFAULTS
            && let Some(values) = values.as_object_mut()
        {
            values.remove("default_job");
        }
        serde_json::from_value(values).unwrap_or_else(|_| self.clone())
    }

    /// Where the options are kept, when there is a config folder.
    pub fn path() -> Option<PathBuf> {
        path()
    }

    /// Saves the options, with the check that keeps the advanced ones from hand edits.
    pub fn save_to(&self, path: &std::path::Path) -> Result<(), String> {
        let mut file = serde_json::to_value(self).expect("options serialise");
        file[SEAL] = self.advanced.seal().into();
        let json = serde_json::to_vec_pretty(&file).expect("options serialise");
        cut_craft_core::file::write_atomic(path, &json)
            .map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Sets what the views draw to these options.
    pub fn apply_to(&self, d: &mut Display) {
        let (v, t) = (&self.view_3d, &self.toolpaths);
        d.origin = v.draw_origin;
        d.material_block = v.draw_material_block;
        d.vectors_3d = v.draw_vectors;
        d.colour_shaded = v.color_shaded_view;
        d.shaded_background = v.shaded_background;
        d.light_follows = v.light_follows_viewer;
        d.auto_open_3d = t.auto_open_3d_view;
        d.previews_2d = t.create_2d_previews;
        d.solid_previews = t.solid_2d_previews;
        d.solid_colour = t.solid_2d_preview_color;
        d.keep_out_labels = t.keep_out_labels;
        d.detail = self.performance.preview_detail.min(100);
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
mod tests;
