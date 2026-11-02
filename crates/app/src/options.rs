//! Edit ▸ Options: settings kept between runs, in Aspire's sections. Only the settings Cut
//! Craft acts on are here: what the views draw when it starts, and the recent files kept.

use crate::display::Display;
use eframe::egui;
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
    /// What fills the 3D view behind the material, when the shaded background is on
    #[serde(default)]
    #[schemars(title = "Shaded background style")]
    pub background_style: BackgroundStyle,
    /// Solid, and the top of a gradient
    #[serde(default = "background_top")]
    #[schemars(extend("x-kind" = "colour"))]
    pub background_color: [u8; 3],
    /// The bottom of a gradient
    #[serde(default = "background_bottom")]
    #[schemars(extend("x-kind" = "colour"))]
    pub gradient_background_color: [u8; 3],
    /// The picture behind an Image background
    #[serde(default)]
    pub image_file_path: String,
    /// Smooths the edges of everything drawn (from the next start)
    #[serde(default)]
    #[schemars(title = "Anti-aliasing mode")]
    pub anti_aliasing: AntiAliasing,
    /// Draw the model coarser while the view turns or zooms, and at full detail once it is
    /// still: smoother on a slow computer, blockier while moving (not one of Aspire's)
    #[serde(default)]
    #[schemars(title = "Simplify the model while the view moves")]
    pub simplify_while_moving: bool,
    /// How dragging turns the view: about the job's vertical with its up kept up, or any
    /// way without stops (not one of Aspire's)
    #[serde(default)]
    pub rotation: Rotation,
}

/// How dragging (and a SpaceMouse) turns the 3D view.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Rotation {
    /// Spin about the job's vertical, tilt between straight down and straight up: the job's
    /// up stays up, as on a machine bed
    Turntable,
    /// Any way, without stops: the job can roll and turn upside down
    #[default]
    Free,
}

impl Rotation {
    pub fn view(self) -> crate::view3d::Rotation {
        match self {
            Self::Turntable => crate::view3d::Rotation::Turntable,
            Self::Free => crate::view3d::Rotation::Free,
        }
    }
}

/// A 3Dconnexion SpaceMouse turning the 3D view (Linux, through spacenavd; not one of
/// Aspire's).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the dialog's own check boxes"
)]
pub struct SpaceMouse {
    /// Object: the cap moves the model as if holding it. Camera: it moves the viewpoint, so
    /// every direction is the other way
    #[serde(default)]
    pub mode: SpaceMouseMode,
    /// How fast the view follows the cap
    #[serde(default = "one")]
    #[schemars(range(min = 0.1, max = 10.0))]
    pub speed: f64,
    /// Turn each movement the other way
    #[serde(default)]
    #[schemars(title = "Invert left / right")]
    pub invert_x: bool,
    #[serde(default)]
    #[schemars(title = "Invert up / down")]
    pub invert_y: bool,
    #[serde(default)]
    #[schemars(title = "Invert zoom (push / pull)")]
    pub invert_zoom: bool,
    #[serde(default)]
    #[schemars(title = "Invert tilt")]
    pub invert_tilt: bool,
    #[serde(default)]
    #[schemars(title = "Invert spin")]
    pub invert_spin: bool,
    #[serde(default)]
    #[schemars(title = "Invert roll (Free rotation only)")]
    pub invert_roll: bool,
}

impl Default for SpaceMouse {
    fn default() -> Self {
        Self {
            mode: SpaceMouseMode::default(),
            speed: one(),
            invert_x: false,
            invert_y: false,
            invert_zoom: false,
            invert_tilt: false,
            invert_spin: false,
            invert_roll: false,
        }
    }
}

fn one() -> f64 {
    1.0
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SpaceMouseMode {
    /// The cap moves the model, as if holding it (the usual CAD way)
    #[default]
    Object,
    /// The cap moves the viewpoint round the model
    Camera,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundStyle {
    Solid,
    #[default]
    Gradient,
    Image,
}

/// Multisampling: Aspire offers 2x, 4x and 6x too.
// ponytail: 4x is the only count every graphics card takes; others can stop the window opening
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AntiAliasing {
    #[default]
    None,
    #[schemars(title = "4X")]
    X4,
}

impl AntiAliasing {
    /// Samples a pixel for the window's renderer.
    pub fn samples(self) -> u16 {
        match self {
            Self::None => 0,
            Self::X4 => 4,
        }
    }
}

/// The light theme's own background, so the view looks as it did before these settings.
fn background_top() -> [u8; 3] {
    rgb(egui::Visuals::light().faint_bg_color)
}

fn background_bottom() -> [u8; 3] {
    rgb(egui::Visuals::light().extreme_bg_color)
}

fn rgb(c: egui::Color32) -> [u8; 3] {
    [c.r(), c.g(), c.b()]
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
    #[serde(default = "solid_colour")]
    #[schemars(title = "2D solid preview color", extend("x-kind" = "colour"))]
    pub solid_2d_preview_color: [u8; 3],
    /// Saved toolpaths leave out cutting points this close to the line past them, for
    /// shorter files; 0 keeps every point. Profile, pocket and other 2D toolpaths
    #[serde(default = "tolerance")]
    #[schemars(title = "2D toolpath tolerance", extend("x-unit" = "mm"), range(min = 0))]
    pub tolerance_2d: f64,
    /// The same for 3D roughing, finishing, texture, moulding and the like
    #[serde(default = "tolerance")]
    #[schemars(title = "3D toolpath tolerance", extend("x-unit" = "mm"), range(min = 0))]
    pub tolerance_3d: f64,
    /// The same for V-carving, prism, inlay and chamfer toolpaths
    #[serde(default = "tolerance")]
    #[schemars(title = "VCarve toolpath tolerance", extend("x-unit" = "mm"), range(min = 0))]
    pub tolerance_vcarve: f64,
}

fn solid_colour() -> [u8; 3] {
    Display::default().solid_colour
}

/// A ten-thousandth of an inch.
// ponytail: Aspire's defaults are not known
fn tolerance() -> f64 {
    0.002_54
}

impl ToolpathSettings {
    /// The tolerance for a toolpath of this family.
    pub fn tolerance(&self, family: cut_craft_core::simplify::Family) -> f64 {
        use cut_craft_core::simplify::Family;
        match family {
            Family::TwoD => self.tolerance_2d,
            Family::ThreeD => self.tolerance_3d,
            Family::VCarve => self.tolerance_vcarve,
        }
    }

    /// `doc` with each toolpath's cuts thinned to its tolerance, ready to save.
    pub fn thinned(
        &self,
        doc: &cut_craft_core::document::Document,
    ) -> cut_craft_core::document::Document {
        use cut_craft_core::simplify::{family, simplify};
        let mut doc = doc.clone();
        for toolpath in doc.toolpaths.values_mut() {
            // a laser's power goes move by move, so its moves stay as they are
            if toolpath.power.is_empty() {
                toolpath.moves = simplify(&toolpath.moves, self.tolerance(family(toolpath)));
            }
        }
        doc
    }
}

/// Aspire's Confirmation Choices: what happens when something looks wrong.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Confirmations {
    /// Crossing vectors chosen for a V-carve or pocket toolpath
    #[serde(default)]
    #[schemars(title = "Toolpath vector intersections check")]
    pub toolpath_intersections: IntersectionCheck,
    /// Crossing vectors chosen for a radiused (round-cornered) offset
    #[serde(default)]
    #[schemars(title = "Radiused offset intersections check")]
    pub radiused_offset_intersections: IntersectionCheck,
    /// Crossing vectors chosen for a sharp-cornered offset
    #[serde(default)]
    #[schemars(title = "Sharp offset intersections check")]
    pub sharp_offset_intersections: IntersectionCheck,
    /// Problems the checks before cutting find when saving toolpaths (not one of Aspire's)
    #[serde(default)]
    #[schemars(title = "Checks before cutting")]
    pub cutting_checks: CuttingChecks,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum IntersectionCheck {
    /// Ask whether to go on, open the Vector Validator, or stop
    #[default]
    Ask,
    /// Select the crossing vectors in the Vector Validator instead
    OpenVectorValidator,
    /// Go on without checking
    #[schemars(title = "Don't check")]
    DontCheck,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CuttingChecks {
    /// List the problems and ask before saving
    #[default]
    Ask,
    /// Save, and say what was found in the status bar
    #[schemars(title = "Save and tell")]
    SaveAndTell,
}

/// Aspire's General Settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct General {
    /// How many files File ▸ Recent Files and the start page list
    #[schemars(range(min = 1, max = 20))]
    pub recent_file_list_size: u32,
    /// Read every icon at startup, so none shows a loading spinner the first time it is
    /// drawn; off reads each when first drawn, for low-end systems (from the next start).
    /// Dialog pictures are always read at startup
    #[serde(default = "on")]
    pub preload_icons: bool,
    /// The folder file dialogs open in
    #[serde(default)]
    #[schemars(title = "File dialog default")]
    pub file_dialog_default: FileDialogDefault,
    /// How the window is arranged: Aspire's pages and side tabs, a rail of tool groups with
    /// an inspector, or a ribbon of tabs over the view
    #[serde(default)]
    #[schemars(title = "Window layout")]
    pub layout: WindowLayout,
}

/// Importing 3D models: what Orientate 3D Model offers (not one of Aspire's).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Import {
    /// What an imported model is kept as: whole (its underside on the other side of a
    /// double-sided job, wrapped in a rotary one), its top seen from above, or the choice
    /// of both on the form
    #[serde(default)]
    #[schemars(title = "Imported model type")]
    pub model_types: ModelTypes,
    /// A model with more faces than this is shown as the surface it makes while it is
    /// imported, with only the faces out of reach drawn over it; drawing millions of faces
    /// a frame makes the view drag
    #[serde(default = "preview_faces")]
    #[schemars(title = "Model preview faces", range(min = 1000))]
    pub preview_faces: u32,
}

fn preview_faces() -> u32 {
    250_000
}

impl Default for Import {
    fn default() -> Self {
        Self {
            model_types: ModelTypes::default(),
            preview_faces: preview_faces(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ModelTypes {
    #[default]
    #[schemars(title = "Full 3D model")]
    Full,
    #[schemars(title = "Flat model")]
    Flat,
    #[schemars(title = "Both, chosen on the form")]
    Both,
}

/// Settings that change how hard Cut Craft works the computer, or how exactly it cuts: listed
/// only once Show advanced settings is ticked, and put back to their defaults when the file
/// was changed outside Cut Craft (not one of Aspire's).
// ponytail: wayland_native moved here from General; one saved there before starts off again
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Advanced {
    /// On a Wayland desktop, open the window there directly rather than through `XWayland`;
    /// files cannot be dragged into it then (from the next start). Listed only on Wayland
    #[serde(default)]
    #[schemars(title = "Use Wayland directly (no file drag and drop)")]
    pub wayland_native: bool,
    /// How many processor cores previews, simulations and imports may use at once; 0 uses
    /// them all. Fewer keeps the computer responsive for other programs, but makes previews
    /// slower
    #[serde(default)]
    #[schemars(title = "Processor cores to use (0 = all)", range(min = 0, max = 256))]
    pub cores: u32,
}

impl Advanced {
    /// Whether this desktop is a Wayland one on Linux or a BSD, where `wayland_native` applies.
    pub fn wayland() -> bool {
        cfg!(all(unix, not(target_os = "macos"))) && std::env::var_os("WAYLAND_DISPLAY").is_some()
    }

    /// Hands these settings to the code they steer.
    pub fn apply(&self) {
        cut_craft_core::simulation::set_core_limit(usize::try_from(self.cores).unwrap_or(0));
    }

    /// The check saved beside these settings: a keyed hash of them, so an edit made outside
    /// Cut Craft shows. The key ships in the app, so it stops a hand edit, not a determined
    /// one; that is all it is for.
    fn seal(&self) -> String {
        use hmac::Mac;
        let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(SEAL_KEY).expect("any key fits");
        mac.update(&serde_json::to_vec(self).expect("settings serialise"));
        mac.finalize()
            .into_bytes()
            .iter()
            .fold(String::new(), |mut hex, b| {
                use std::fmt::Write as _;
                let _ = write!(hex, "{b:02x}");
                hex
            })
    }

    /// Whether `seal` is these settings' own. Options saved before the check have none: they
    /// pass when only `wayland_native`, the one setting there was then, is set.
    fn sealed_by(&self, seal: Option<&str>) -> bool {
        match seal {
            Some(seal) => seal == self.seal(),
            None => {
                *self
                    == Self {
                        wayland_native: self.wayland_native,
                        ..Self::default()
                    }
            }
        }
    }
}

/// The key of [`Advanced::seal`], and the options file's name for it.
const SEAL_KEY: &[u8] = b"Cut Craft advanced settings, v1";
const SEAL: &str = "advanced_check";

/// The warning above the advanced settings, and what it goes on to say.
pub const WARNING: &str = "⚠ Changing these can freeze Cut Craft or spoil a cut.";
pub const WARNING_MORE: &str = "They set how hard Cut Craft works your computer and how exactly \
    it cuts. Leave them alone unless you know why you are changing one; Reset to defaults puts \
    them all back.";

/// What the status bar says when the advanced settings were changed outside Cut Craft.
pub const TAMPERED: &str = "Advanced settings were changed outside Cut Craft, so they were put \
    back to their safe defaults. Change them in Edit ▸ Options ▸ Advanced.";

/// How Cut Craft looks (not one of Aspire's).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Appearance {
    /// Light, dark, or the desktop's own
    #[serde(default)]
    pub theme: Theme,
    /// The desktop's own title bar above the menus, rather than Cut Craft's in the menu bar
    #[serde(default)]
    #[schemars(title = "Use the desktop's title bar")]
    pub system_title_bar: bool,
    /// The snap grid's points in the 2D view
    #[serde(default = "grid_color")]
    #[schemars(title = "Grid color", extend("x-kind" = "colour"))]
    pub grid_color: [u8; 3],
    /// How big the snap grid's points are drawn, in screen points
    #[serde(default = "grid_point_size")]
    #[schemars(range(min = 0.5, max = 10))]
    pub grid_point_size: f32,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            system_title_bar: false,
            grid_color: grid_color(),
            grid_point_size: grid_point_size(),
        }
    }
}

/// The light theme's faint text colour, as other faint marks are.
fn grid_color() -> [u8; 3] {
    let c = egui::Visuals::light().weak_text_color();
    [c.r(), c.g(), c.b()]
}

/// As thick as two of the theme's lines.
fn grid_point_size() -> f32 {
    2.0 * egui::Visuals::light()
        .widgets
        .noninteractive
        .fg_stroke
        .width
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    /// Light, as Aspire is, whatever the desktop's theme
    #[default]
    Light,
    Dark,
    /// The desktop's own light or dark theme
    System,
}

impl Theme {
    pub fn preference(self) -> egui::ThemePreference {
        match self {
            Self::Light => egui::ThemePreference::Light,
            Self::Dark => egui::ThemePreference::Dark,
            Self::System => egui::ThemePreference::System,
        }
    }
}

/// The window's arrangement (`docs/superpowers/specs/2026-10-27-window-layout-setting-design.md`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WindowLayout {
    /// Aspire's: Design page left, Toolpaths page right, side tabs
    #[default]
    Classic,
    /// A rail of tool groups on the left opening a list of tools; an inspector on the right
    Rail,
    /// Tabs of tool groups across the top; the browser left, the form right
    Ribbon,
}

// ponytail: Aspire's third choice, a folder remembered for each kind of operation, is left out
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FileDialogDefault {
    /// The desktop's own choice, usually the last folder used
    #[default]
    Global,
    /// The open job's folder
    Job,
}

/// What a setting added after options were first saved starts as.
fn on() -> bool {
    true
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Options {
    #[schemars(title = "3D View Settings", extend("x-section" = true))]
    pub view_3d: View3d,
    #[schemars(title = "Toolpath Settings", extend("x-section" = true))]
    pub toolpaths: ToolpathSettings,
    #[schemars(title = "General Settings", extend("x-section" = true))]
    pub general: General,
    #[serde(default)]
    #[schemars(title = "Confirmation Choices", extend("x-section" = true))]
    pub confirmations: Confirmations,
    #[serde(default)]
    #[schemars(title = "Importing Models", extend("x-section" = true))]
    pub import: Import,
    #[serde(default)]
    #[schemars(extend("x-section" = true))]
    pub appearance: Appearance,
    #[serde(default)]
    #[schemars(title = "SpaceMouse", extend("x-section" = true))]
    pub space_mouse: SpaceMouse,
    #[serde(default)]
    #[schemars(extend("x-section" = true))]
    pub advanced: Advanced,
    /// Gadgets ▸ Job Setup Sheet Editor: how job sheets look.
    #[serde(default)]
    #[schemars(skip)]
    pub job_sheet: fmt_create_job_sheet::Style,
    /// Export as STL: how the model is triangulated.
    #[serde(default)]
    #[schemars(skip)]
    pub stl: fmt_export_as_stl::Settings,
    /// What Job Setup starts a new job with, once its Set as Default is pressed; Cut Craft's
    /// own starting job until then.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(skip)]
    pub default_job: Option<cut_craft_core::document::Job>,
    /// The favourite tools of the Design and Toolpaths tabs, and whether each shows only
    /// them (right-click the tab).
    #[serde(default)]
    #[schemars(skip)]
    pub favourites: Favourites,
    /// The tool each toolbox flyout shows: the one picked from it last.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    #[schemars(skip)]
    pub flyout_faces: crate::flyout::Faces,
}

/// Each tool page's favourite tools, in the order shown, and whether it shows only those.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Favourites {
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
            },
            toolpaths: ToolpathSettings {
                auto_open_3d_view: d.auto_open_3d,
                create_2d_previews: d.previews_2d,
                solid_2d_previews: d.solid_previews,
                solid_2d_preview_color: d.solid_colour,
                tolerance_2d: tolerance(),
                tolerance_3d: tolerance(),
                tolerance_vcarve: tolerance(),
            },
            general: General {
                recent_file_list_size: u32::try_from(crate::files::RECENT_COUNT)
                    .expect("a small count"),
                preload_icons: on(),
                file_dialog_default: FileDialogDefault::default(),
                layout: WindowLayout::default(),
            },
            confirmations: Confirmations::default(),
            job_sheet: fmt_create_job_sheet::Style::default(),
            stl: fmt_export_as_stl::Settings::default(),
            import: Import::default(),
            appearance: Appearance::default(),
            advanced: Advanced::default(),
            space_mouse: SpaceMouse::default(),
            default_job: None,
            favourites: Favourites::default(),
            flyout_faces: crate::flyout::Faces::new(),
        }
    }
}

impl Options {
    /// The options kept in the config folder; the defaults when there are none.
    pub fn load() -> Self {
        Self::load_checked().0
    }

    /// [`Self::load`], and whether the advanced settings had been changed outside Cut Craft
    /// (and so were put back to their defaults).
    pub fn load_checked() -> (Self, bool) {
        path().and_then(|p| std::fs::read(p).ok()).map_or_else(
            || (Self::default(), false),
            |bytes| Self::from_bytes(&bytes),
        )
    }

    /// Options read from a file's bytes, the advanced ones only if their check matches; the
    /// defaults for a file that is not options at all.
    pub fn from_bytes(bytes: &[u8]) -> (Self, bool) {
        let Ok(mut file) = serde_json::from_slice::<serde_json::Value>(bytes) else {
            return (Self::default(), false);
        };
        let seal = file
            .as_object_mut()
            .and_then(|o| o.remove(SEAL))
            .and_then(|s| s.as_str().map(String::from));
        let Ok(mut options) = serde_json::from_value::<Self>(file) else {
            return (Self::default(), false);
        };
        let tampered = !options.advanced.sealed_by(seal.as_deref());
        if tampered {
            options.advanced = Advanced::default();
        }
        (options, tampered)
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
        d.colour_shaded = v.color_shaded_view;
        d.shaded_background = v.shaded_background;
        d.light_follows = v.light_follows_viewer;
        d.auto_open_3d = t.auto_open_3d_view;
        d.previews_2d = t.create_2d_previews;
        d.solid_previews = t.solid_2d_previews;
        d.solid_colour = t.solid_2d_preview_color;
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
    fn the_window_layout_is_classic_unless_the_options_say_otherwise() {
        assert_eq!(Options::default().general.layout, WindowLayout::Classic);
        let mut o = Options::default();
        o.general.layout = WindowLayout::Rail;
        let text = serde_json::to_string(&o).unwrap();
        assert!(text.contains("\"layout\":\"rail\""));
        let back: Options = serde_json::from_str(&text).unwrap();
        assert_eq!(back.general.layout, WindowLayout::Rail);
        // options saved before the setting existed
        let mut old: serde_json::Value = serde_json::to_value(&o).unwrap();
        old["general"].as_object_mut().unwrap().remove("layout");
        let old: Options = serde_json::from_value(old).unwrap();
        assert_eq!(old.general.layout, WindowLayout::Classic);
    }

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
        o.toolpaths.solid_2d_preview_color = [1, 2, 3];
        let mut d = Display::default();
        o.apply_to(&mut d);
        assert!(!d.origin && d.solid_previews);
        assert_eq!(d.solid_colour, [1, 2, 3]);
    }

    #[test]
    fn they_load_back_as_saved_and_fill_gaps_with_defaults() {
        let o: Options =
            serde_json::from_str(&serde_json::to_string(&Options::default()).unwrap()).unwrap();
        assert_eq!(o, Options::default());
        let older = r#"{ "view_3d": { "draw_origin": false, "draw_material_block": true,
            "color_shaded_view": true, "shaded_background": true, "light_follows_viewer": true },
            "toolpaths": { "auto_open_3d_view": true, "create_2d_previews": true,
            "solid_2d_previews": false }, "general": { "recent_file_list_size": 4 } }"#;
        let older: Options = serde_json::from_str(older).expect("options saved before preloading");
        assert!(!older.view_3d.draw_origin && older.general.preload_icons);
        assert_eq!(
            older.appearance.theme,
            Theme::Light,
            "light, as before themes"
        );
        let fields =
            crate::form::fields(&serde_json::to_value(schemars::schema_for!(Options)).unwrap());
        let sections: Vec<&str> = fields
            .iter()
            .filter(|f| f.section)
            .map(|f| f.label.as_str())
            .collect();
        assert_eq!(
            sections,
            [
                "3D View Settings",
                "Toolpath Settings",
                "General Settings",
                "Confirmation Choices",
                "Importing Models",
                "Appearance",
                "SpaceMouse",
                "Advanced"
            ]
        );
    }

    #[test]
    fn saving_thins_each_toolpath_to_its_familys_tolerance() {
        use cut_craft_core::glam::DVec3;
        use cut_craft_core::toolpath::{Move, MoveKind, Toolpath};
        let line = |strategy: &str| Toolpath {
            name: strategy.into(),
            strategy: strategy.into(),
            params: serde_json::Value::Null,
            tool: cut_craft_core::toolpath::Tool::default(),
            vectors: vec![],
            moves: (0..=10)
                .map(|i| Move {
                    kind: MoveKind::Cut,
                    to: DVec3::new(f64::from(i), 0.0, -1.0),
                })
                .collect(),
            visible: true,
            power: vec![],
            stale: false,
            group: None,
            drill_bank: None,
        };
        let mut doc = cut_craft_core::document::Document::default();
        doc.toolpaths.insert(
            cut_craft_core::document::EntityId(1),
            line("profile_toolpath"),
        );
        doc.toolpaths.insert(
            cut_craft_core::document::EntityId(2),
            line("3d_finishing_toolpath"),
        );
        let mut t = Options::default().toolpaths;
        t.tolerance_3d = 0.0;
        let thinned = t.thinned(&doc);
        let count = |id| {
            thinned.toolpaths[&cut_craft_core::document::EntityId(id)]
                .moves
                .len()
        };
        assert_eq!(
            (count(1), count(2)),
            (2, 11),
            "2D thinned, 3D kept whole at 0"
        );
    }

    /// `o` as saved to a file, read back.
    fn saved(o: &Options) -> Vec<u8> {
        static FILES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = FILES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("cc-options-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{n}.json"));
        o.save_to(&path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let _ = std::fs::remove_file(path);
        bytes
    }

    #[test]
    fn advanced_settings_saved_by_cut_craft_load_back_as_saved() {
        let mut o = Options::default();
        o.advanced.cores = 2;
        o.general.recent_file_list_size = 7;
        let (back, tampered) = Options::from_bytes(&saved(&o));
        assert!(!tampered);
        assert_eq!(back, o);
    }

    #[test]
    fn advanced_settings_edited_outside_cut_craft_go_back_to_their_defaults() {
        let mut o = Options::default();
        o.advanced.cores = 2;
        let mut file: serde_json::Value = serde_json::from_slice(&saved(&o)).unwrap();
        file["advanced"]["cores"] = 64.into();
        // a normal setting edited by hand is kept
        file["general"]["recent_file_list_size"] = 9.into();
        let (back, tampered) = Options::from_bytes(&serde_json::to_vec(&file).unwrap());
        assert!(tampered);
        assert_eq!(back.advanced, Advanced::default());
        assert_eq!(back.general.recent_file_list_size, 9);
        // taking the check away does not get round it
        file.as_object_mut().unwrap().remove(SEAL);
        let (back, tampered) = Options::from_bytes(&serde_json::to_vec(&file).unwrap());
        assert!(tampered);
        assert_eq!(back.advanced, Advanced::default());
    }

    #[test]
    fn options_saved_before_the_check_keep_their_wayland_choice() {
        let older = r#"{ "view_3d": { "draw_origin": true, "draw_material_block": true,
            "color_shaded_view": true, "shaded_background": true, "light_follows_viewer": true },
            "toolpaths": { "auto_open_3d_view": true, "create_2d_previews": true,
            "solid_2d_previews": false }, "general": { "recent_file_list_size": 4 },
            "advanced": { "wayland_native": true } }"#;
        let (back, tampered) = Options::from_bytes(older.as_bytes());
        assert!(!tampered, "nothing to warn about");
        assert!(back.advanced.wayland_native);
    }

    #[test]
    fn a_page_resets_to_its_defaults_alone() {
        let mut o = Options::default();
        o.general.recent_file_list_size = 9;
        o.advanced.cores = 3;
        o.favourites.design = vec!["draw_circle".into()];
        let general = o.reset("general");
        assert_eq!(general.general, Options::default().general);
        assert_eq!(general.advanced.cores, 3, "other pages kept");
        assert_eq!(general.favourites.design, ["draw_circle"]);
        assert_eq!(o.reset("advanced").advanced, Advanced::default());
        assert_eq!(o.reset("no_such_page"), o);
    }
}
