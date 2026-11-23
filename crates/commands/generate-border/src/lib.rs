//! Generate Border: one repeating length of picture-frame moulding in a style you pick,
//! made by the Borders add-on with your own Google key and saved to your Library under
//! Generated ▸ Borders as a depth map with its photo, ready to place like any 3D model.
//!
//! The form's Generate (its Apply) is the app's: it runs the add-on in the background and
//! saves what comes back. This command only checks the settings, and changes nothing in
//! the drawing.

use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Form, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct GenerateBorder;

/// The look of the moulding. Each is a description the add-on sends with your details.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Style {
    /// Ornate acanthus leaves, scrolls and shell motifs, deeply carved
    #[default]
    #[schemars(title = "Classical Baroque")]
    ClassicalBaroque,
    /// Clean stepped profiles and flat bands, with no ornament
    #[schemars(title = "Modern Minimalist")]
    ModernMinimalist,
    /// Geometric fans, chevrons and stepped zig-zags
    #[schemars(title = "Art Deco")]
    ArtDeco,
    /// Weathered wood grain, knots and simple beading
    #[schemars(title = "Rustic Vintage")]
    RusticVintage,
    /// Rich floral garlands, ribbons and beaded edges
    #[schemars(title = "Victorian Carved")]
    VictorianCarved,
    /// Sleek brushed ridges and fine parallel grooves
    #[schemars(title = "Contemporary Metallic")]
    ContemporaryMetallic,
    /// Flowing asymmetric C- and S-scrolls with shells and flowers
    #[schemars(title = "Rococo")]
    Rococo,
    /// Simple repeating ovals, dots and tapering lines
    #[schemars(title = "Mid-Century Modern")]
    MidCenturyModern,
    /// Pointed arches, tracery and quatrefoils
    #[schemars(title = "Gothic Pewter")]
    GothicPewter,
    /// Nothing but what you type under Details
    #[schemars(title = "Your Own Description")]
    Custom,
}

impl Style {
    /// Its id, as the add-on and the recipe know it.
    pub fn id(self) -> &'static str {
        match self {
            Self::ClassicalBaroque => "classical_baroque",
            Self::ModernMinimalist => "modern_minimalist",
            Self::ArtDeco => "art_deco",
            Self::RusticVintage => "rustic_vintage",
            Self::VictorianCarved => "victorian_carved",
            Self::ContemporaryMetallic => "contemporary_metallic",
            Self::Rococo => "rococo",
            Self::MidCenturyModern => "mid_century_modern",
            Self::GothicPewter => "gothic_pewter",
            Self::Custom => "custom",
        }
    }
}

/// The tile's shape, its length to its height.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Shape {
    /// 21 long to 9 high: a narrow moulding
    #[schemars(title = "Narrow (21:9)")]
    Narrow,
    /// 16 long to 9 high
    #[default]
    #[schemars(title = "Medium (16:9)")]
    Medium,
    /// 4 long to 3 high: a wide moulding
    #[schemars(title = "Wide (4:3)")]
    Wide,
}

impl Shape {
    /// As the add-on and the recipe write it, "16:9".
    pub fn ratio(self) -> &'static str {
        match self {
            Self::Narrow => "21:9",
            Self::Medium => "16:9",
            Self::Wide => "4:3",
        }
    }

    /// Height for each unit of length.
    pub fn height_per_length(self) -> f64 {
        match self {
            Self::Narrow => 9.0 / 21.0,
            Self::Medium => 9.0 / 16.0,
            Self::Wide => 3.0 / 4.0,
        }
    }
}

/// Where the tile's depth comes from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DepthFrom {
    /// Google turns the picture it made into a depth map: it knows what it drew
    #[default]
    #[schemars(title = "Google")]
    Google,
    /// The Depth add-on works it out on your own computer: free, and nothing more is sent
    #[schemars(title = "Depth add-on")]
    DepthAddOn,
}

impl DepthFrom {
    /// As the recipe writes it.
    pub fn id(self) -> &'static str {
        match self {
            Self::Google => "google",
            Self::DepthAddOn => "depth_add_on",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Params {
    /// What the border is called in the Library (a number is added if the name is taken)
    pub name: String,
    /// The look of the moulding
    pub style: Style,
    /// Anything to add to the style in your own words, such as "laurel leaves" or "a
    /// twisted rope". With Your Own Description, the whole description. Sent to Google
    #[schemars(extend("x-widget" = "multiline"))]
    pub details: String,
    /// The tile's shape: how tall it is for its length
    pub shape: Shape,
    /// One repeat, end to end. The border's height follows from its shape
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub length: f64,
    /// How far its highest point stands above its lowest
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub depth: f64,
    /// Where the depth comes from
    #[schemars(extend("x-group" = "Depth"))]
    pub depth_from: DepthFrom,
    /// Below 1 the middle heights rise for a fuller carving; above 1 they fall for crisper
    /// tops. 1 leaves them as they came
    #[schemars(extend("x-group" = "Depth"), range(min = 0.2, max = 5))]
    pub depth_curve: f64,
    /// Smooths the steps between greys so the carving isn't terraced, as a share of the
    /// border's height. 0 for none
    #[schemars(extend("x-group" = "Depth", "x-unit" = "%"), range(min = 0, max = 5))]
    pub smoothing: f64,
    /// How far in from each end the two ends are blended so repeats meet without a step, as
    /// a share of the length. 0 for none
    #[schemars(extend("x-group" = "Depth", "x-unit" = "%"), range(min = 0, max = 25))]
    pub seam_blend: f64,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Border".into(),
            style: Style::ClassicalBaroque,
            details: String::new(),
            shape: Shape::Medium,
            length: 150.0,
            depth: 6.0,
            depth_from: DepthFrom::Google,
            depth_curve: 1.0,
            smoothing: 0.4,
            seam_blend: 6.0,
        }
    }
}

impl Params {
    /// The border's height, top to bottom, in mm.
    pub fn height(&self) -> f64 {
        self.length * self.shape.height_per_length()
    }

    /// The settings checked, with what to change said when one won't do.
    pub fn check(&self) -> Result<(), CommandError> {
        ensure_positive("length", self.length)?;
        ensure_positive("depth", self.depth)?;
        if self.style == Style::Custom && self.details.trim().is_empty() {
            return Err(CommandError::InvalidParams(
                "describe the border under Details, or pick a style".into(),
            ));
        }
        if !(0.2..=5.0).contains(&self.depth_curve) {
            return Err(CommandError::InvalidParams(
                "set the depth curve between 0.2 and 5".into(),
            ));
        }
        Ok(())
    }
}

impl Command for GenerateBorder {
    const ID: &'static str = "generate_border";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Generate Border",
            group: "Modeling",
            icon: "",
            shortcut: "",
            form: Form::Generate,
        }
    }

    fn run(_doc: &Document, p: &Params) -> Result<Change, CommandError> {
        p.check()?;
        Ok(Change::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_height_follows_the_shape() {
        let p = Params {
            length: 160.0,
            shape: Shape::Medium,
            ..Params::default()
        };
        assert!((p.height() - 90.0).abs() < 1e-9);
        let wide = Params {
            shape: Shape::Wide,
            ..p
        };
        assert!((wide.height() - 120.0).abs() < 1e-9);
    }

    #[test]
    fn your_own_description_needs_one() {
        let mut p = Params {
            style: Style::Custom,
            ..Params::default()
        };
        assert!(p.check().is_err());
        p.details = "a twisted rope".into();
        assert!(p.check().is_ok());
    }

    #[test]
    fn ids_match_what_the_add_on_knows() {
        for s in [
            Style::ClassicalBaroque,
            Style::MidCenturyModern,
            Style::Custom,
        ] {
            assert_eq!(serde_json::to_value(s).unwrap(), s.id());
        }
        assert_eq!(Shape::Narrow.ratio(), "21:9");
    }

    #[test]
    fn it_changes_nothing_in_the_drawing() {
        let doc = Document::default();
        let change = GenerateBorder::run(&doc, &Params::default()).unwrap();
        assert!(change.is_empty());
    }
}
