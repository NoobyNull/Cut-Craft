//! Parameter types shared by many commands, so their forms and validation match.

use crate::geometry::{Point, Rect};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A position in millimetres.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Xy {
    #[schemars(extend("x-unit" = "mm"))]
    pub x: f64,
    #[schemars(extend("x-unit" = "mm"))]
    pub y: f64,
}

impl Xy {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

impl From<Xy> for Point {
    fn from(p: Xy) -> Self {
        Point::new(p.x, p.y)
    }
}

/// Which point of a bounding box a move or resize is measured from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Anchor {
    TopLeft,
    Top,
    TopRight,
    Left,
    #[default]
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl Anchor {
    pub fn point(self, r: Rect) -> Point {
        let (fx, fy) = match self {
            Self::TopLeft => (0.0, 1.0),
            Self::Top => (0.5, 1.0),
            Self::TopRight => (1.0, 1.0),
            Self::Left => (0.0, 0.5),
            Self::Center => (0.5, 0.5),
            Self::Right => (1.0, 0.5),
            Self::BottomLeft => (0.0, 0.0),
            Self::Bottom => (0.5, 0.0),
            Self::BottomRight => (1.0, 0.0),
        };
        r.min + r.size() * Point::new(fx, fy)
    }
}

/// The kinds of item the right-click families (Show / Hide, Delete) act on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Items {
    #[default]
    Layers,
    Toolpaths,
    Components,
    Levels,
}

impl Items {
    /// One of them, as messages name it: "layer 9 does not exist".
    pub fn one(self) -> &'static str {
        match self {
            Self::Layers => "layer",
            Self::Toolpaths => "toolpath",
            Self::Components => "component",
            Self::Levels => "level",
        }
    }
}
