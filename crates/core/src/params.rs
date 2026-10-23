//! Parameter types shared by many commands, so their forms and validation match.

use crate::geometry::Point;
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
