//! How Cut Craft looks: themes, the canvas colours, its lines and the language.

use super::{Deserialize, JsonSchema, Serialize, egui, rgb};

/// How Cut Craft looks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Appearance {
    /// Light, dark, or the desktop's own
    #[serde(default)]
    pub theme: Theme,
    /// The desktop's own title bar above the menus, rather than Cut Craft's in the menu bar
    #[serde(default)]
    #[schemars(title = "Use the desktop's title bar")]
    pub system_title_bar: bool,
    /// The colors vectors, toolpaths and rapid moves are drawn in. Standard: as Cut Craft
    /// has always drawn them. Color-blind friendly: colors told apart by how light or dark
    /// they are, not only by their hue. My own colors: pick each one below, for each theme
    #[serde(default)]
    #[schemars(title = "Vector and toolpath colors")]
    pub line_colors: LinePalette,
    /// The 2D view's colors while the light theme is on
    #[serde(default = "CanvasColours::light")]
    #[schemars(title = "2D view in the light theme")]
    pub light_2d: CanvasColours,
    /// The 2D view's colors while the dark theme is on
    #[serde(default = "CanvasColours::dark")]
    #[schemars(title = "2D view in the dark theme")]
    pub dark_2d: CanvasColours,
    /// How big the snap grid's points are drawn, in screen points
    #[serde(default = "grid_point_size")]
    #[schemars(range(min = 0.5, max = 10))]
    pub grid_point_size: f32,
    /// Rotary jobs: how far past each seam the 2D view shows the other side of the
    /// material, so a line that runs off one edge can be seen coming back in at the other
    #[serde(default)]
    #[schemars(title = "Rotary wrap bands")]
    pub rotary_wrap: WrapBands,
    /// How big text, buttons and lines are, as a percentage: larger is easier to read and to
    /// click, smaller fits more on a small screen (from the next start)
    #[serde(default = "text_size")]
    #[schemars(title = "Text size", extend("x-unit" = "%"), range(min = 80, max = 200))]
    pub text_size: u32,
}

/// How far the 2D view's wrap bands reach past each seam of a rotary job.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WrapBands {
    /// No bands: the sheet alone, as on a flat job
    Off,
    /// A quarter of a turn (90°) past each seam
    #[default]
    #[schemars(title = "Quarter turn")]
    QuarterTurn,
    /// Half a turn (180°) past each seam: the whole of the other side at once
    #[schemars(title = "Half turn")]
    HalfTurn,
}

impl WrapBands {
    /// How far each band reaches, in turns of the material.
    pub fn turns(self) -> f64 {
        match self {
            Self::Off => 0.0,
            Self::QuarterTurn => 0.25,
            Self::HalfTurn => 0.5,
        }
    }
}

/// The size Cut Craft has always drawn its text at: egui's, plus 15%.
pub(super) fn text_size() -> u32 {
    115
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            system_title_bar: false,
            line_colors: LinePalette::default(),
            light_2d: CanvasColours::light(),
            dark_2d: CanvasColours::dark(),
            grid_point_size: grid_point_size(),
            rotary_wrap: WrapBands::default(),
            text_size: text_size(),
        }
    }
}

impl Appearance {
    /// The 2D view's colours for the dark theme, or for the light one.
    pub fn canvas(&self, dark: bool) -> &CanvasColours {
        if dark { &self.dark_2d } else { &self.light_2d }
    }

    /// The colours the 2D view draws vectors, toolpaths and rapid moves in (vectors in the
    /// 3D view too), for the dark theme or the light one, as [`Self::line_colors`] says.
    pub fn lines(&self, dark: bool) -> Lines {
        let (visuals, own) = if dark {
            (egui::Visuals::dark(), &self.dark_2d)
        } else {
            (egui::Visuals::light(), &self.light_2d)
        };
        match self.line_colors {
            LinePalette::Standard => Lines::standard(&visuals),
            LinePalette::ColorBlind => Lines::colour_blind(dark),
            LinePalette::MyOwn => Lines {
                vector: own.vector_color,
                toolpath: own.toolpath_color,
                rapid: own.rapid_color,
            },
        }
    }

    /// What egui's text and control sizes are multiplied by, kept between 80% and 200%.
    pub fn text_scale(&self) -> f32 {
        #[expect(clippy::cast_precision_loss, reason = "80 to 200")]
        let percent = self.text_size.clamp(80, 200) as f32;
        percent / 100.0
    }
}

/// The colors of the 2D view's material sheet and of the snap grid's points over it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[expect(
    clippy::struct_field_names,
    reason = "the names options.json has kept them under"
)]
pub struct CanvasColours {
    /// The material sheet the drawing sits on
    #[schemars(title = "Sheet color", extend("x-kind" = "colour"))]
    pub sheet_color: [u8; 3],
    /// The snap grid's points: pick one that stands out from the sheet color
    #[schemars(title = "Grid color", extend("x-kind" = "colour"))]
    pub grid_color: [u8; 3],
    /// Your drawing's lines, the shapes toolpaths are made from
    #[schemars(
        title = "Vector color",
        extend("x-kind" = "colour", "x-show-when" = {"appearance.line_colors": "my_own"})
    )]
    pub vector_color: [u8; 3],
    /// Where the bit cuts: pick one that looks different from the vector color, lighter or
    /// darker as well as another hue
    #[schemars(
        title = "Toolpath color",
        extend("x-kind" = "colour", "x-show-when" = {"appearance.line_colors": "my_own"})
    )]
    pub toolpath_color: [u8; 3],
    /// The bit's quick moves above the material, and its plunges and lifts, when the
    /// Toolpath Drawing menu shows them
    #[schemars(
        title = "Rapid move color",
        extend("x-kind" = "colour", "x-show-when" = {"appearance.line_colors": "my_own"})
    )]
    pub rapid_color: [u8; 3],
}

impl CanvasColours {
    /// The light theme's: its own field colour for the sheet, a grey that stands out on it.
    pub fn light() -> Self {
        Self::of(&egui::Visuals::light())
    }

    /// The dark theme's, as [`Self::light`].
    pub fn dark() -> Self {
        Self::of(&egui::Visuals::dark())
    }

    /// The sheet in the theme's field colour, the grid as near it as stays easy to see.
    pub(super) fn of(visuals: &egui::Visuals) -> Self {
        let sheet = rgb(visuals.widgets.inactive.bg_fill);
        let lines = Lines::standard(visuals);
        Self {
            sheet_color: sheet,
            grid_color: contrasting(sheet, rgb(visuals.text_color())),
            vector_color: lines.vector,
            toolpath_color: lines.toolpath,
            rapid_color: lines.rapid,
        }
    }
}

/// Which colours vectors, toolpaths and rapid moves are drawn in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LinePalette {
    /// Vectors in the text color, toolpaths orange, rapid moves red: as Cut Craft has
    /// always drawn them
    #[default]
    Standard,
    /// Vectors black (white in the dark theme), toolpaths deep orange, rapid moves pink:
    /// told apart by how light or dark they are as well as by their hue
    #[schemars(title = "Color-blind friendly")]
    ColorBlind,
    /// The vector, toolpath and rapid move colors you pick for each theme below
    #[schemars(title = "My own colors")]
    MyOwn,
}

/// The colours the views draw vectors, toolpaths and rapid moves in, `[r, g, b]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lines {
    pub vector: [u8; 3],
    pub toolpath: [u8; 3],
    pub rapid: [u8; 3],
}

impl Lines {
    /// The theme's own: text, warning and error colours, as the views always drew them.
    pub(super) fn standard(visuals: &egui::Visuals) -> Self {
        Self {
            vector: rgb(visuals.text_color()),
            toolpath: rgb(visuals.warn_fg_color),
            rapid: rgb(visuals.error_fg_color),
        }
    }

    /// From Okabe and Ito's colour-blind safe colours: vermillion toolpaths and reddish
    /// purple rapids (one warm, one blue-ish to a red-green colour-blind eye) against black
    /// or white vectors, far apart in lightness so hue is not needed to tell them apart.
    /// Blue is left to the selection.
    pub(super) fn colour_blind(dark: bool) -> Self {
        Self {
            vector: if dark { [255, 255, 255] } else { [0, 0, 0] },
            toolpath: [213, 94, 0],
            rapid: [204, 121, 167],
        }
    }
}

/// The colour the grid was before each theme had its own: the light theme's faint text,
/// all but hidden on the dark theme's sheet. Options saved with it drop it.
pub(super) fn old_grid_color() -> [u8; 3] {
    rgb(egui::Visuals::light().weak_text_color())
}

/// The least blend of `under` toward `toward` that stands out from `under` as well as
/// WCAG asks of shapes (3 to 1); black or white, whichever stands out more, if none does.
pub(super) fn contrasting(under: [u8; 3], toward: [u8; 3]) -> [u8; 3] {
    const ENOUGH: f64 = 3.0;
    let mixed = |t: f64| {
        let mix = |a: u8, b: u8| {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "between two bytes"
            )]
            let v = (f64::from(a) + (f64::from(b) - f64::from(a)) * t).round() as u8;
            v
        };
        [
            mix(under[0], toward[0]),
            mix(under[1], toward[1]),
            mix(under[2], toward[2]),
        ]
    };
    (1..=20)
        .map(|step| mixed(f64::from(step) / 20.0))
        .find(|c| contrast(*c, under) >= ENOUGH)
        .unwrap_or_else(|| {
            let (black, white) = ([0, 0, 0], [255, 255, 255]);
            if contrast(black, under) > contrast(white, under) {
                black
            } else {
                white
            }
        })
}

/// WCAG's contrast ratio between two colours, from 1 (the same) to 21 (black on white).
pub(super) fn contrast(a: [u8; 3], b: [u8; 3]) -> f64 {
    let luminance = |c: [u8; 3]| {
        let channel = |v: u8| {
            let v = f64::from(v) / 255.0;
            if v <= 0.040_45 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(c[0]) + 0.7152 * channel(c[1]) + 0.0722 * channel(c[2])
    };
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// As thick as two of the theme's lines.
pub(super) fn grid_point_size() -> f32 {
    2.0 * egui::Visuals::light()
        .widgets
        .noninteractive
        .fg_stroke
        .width
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    /// Light, whatever the desktop's theme
    Light,
    Dark,
    /// The desktop's own light or dark theme
    System,
}

impl Default for Theme {
    fn default() -> Self {
        Self::for_system(egui::os::OperatingSystem::from_target_os())
    }
}

impl Theme {
    /// Light; on a Mac the desktop's own, as Mac users expect apps to follow
    /// their light or dark appearance.
    pub fn for_system(os: egui::os::OperatingSystem) -> Self {
        if os == egui::os::OperatingSystem::Mac {
            Self::System
        } else {
            Self::Light
        }
    }

    pub fn preference(self) -> egui::ThemePreference {
        match self {
            Self::Light => egui::ThemePreference::Light,
            Self::Dark => egui::ThemePreference::Dark,
            Self::System => egui::ThemePreference::System,
        }
    }
}

/// How many tools each Ribbon group shows as icons.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RibbonIcons {
    /// The tools that fit under the group's name; the name lists the rest
    #[default]
    Compact,
    /// Every tool in the group
    EveryTool,
}

/// Where the other ways to draw a shape are picked.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OtherWays {
    /// One button for each shape; Draw from, at the top of its form, picks the way
    #[default]
    InTheForm,
    /// A button of its own for each way too, beside the shape's
    AsButtons,
}

/// When a Canvas palette group lists its tools beside it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CanvasFlyouts {
    /// As soon as the pointer is over the group
    #[default]
    OnHover,
    /// When the group is clicked
    OnClick,
}

/// The window's arrangement (`docs/superpowers/specs/2026-10-27-window-layout-setting-design.md`,
/// `docs/superpowers/specs/2026-11-24-shared-tool-model-design.md`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WindowLayout {
    /// Tabs of tool groups across the top; your sheets, layers and toolpaths on the left,
    /// the open tool's settings on the right
    #[default]
    Ribbon,
    /// A column of tool groups on the left, each opening its list of tools by name; the
    /// open tool's settings on the right
    Rail,
    /// The whole window is the drawing: the tools, the open tool's settings and your
    /// sheets, layers and toolpaths float over it
    Canvas,
    /// The Design page of tool icons on the left, the Toolpaths page on the right, and
    /// tabs down the sides
    Classic,
}

impl WindowLayout {
    /// Every layout, in the order offered.
    pub const ALL: [Self; 4] = [Self::Ribbon, Self::Rail, Self::Canvas, Self::Classic];

    /// The layout options saved before the setting existed had: Classic.
    pub fn before_the_setting() -> Self {
        Self::Classic
    }
}

// ponytail: the third choice, a folder remembered for each kind of operation, is left out
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
pub(super) fn on() -> bool {
    true
}

/// Which language Cut Craft's words are in. Its page lists the choices itself (the packs
/// installed), so it has no settings of the ordinary kind.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Language {
    /// The language pack in use, by its add-on id (`language-ru`), or
    /// [`crate::languages::TEST_LANGUAGE`]; empty for English.
    #[serde(default)]
    #[schemars(skip)]
    pub pack: String,
}

/// The Language page's key.
pub const LANGUAGE: &str = "language";
