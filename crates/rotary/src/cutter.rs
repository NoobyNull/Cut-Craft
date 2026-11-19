//! The bit's shape, as the drop and the simulator see it.

use cut_craft_core::toolpath::{Tool, ToolShape};

/// A bit: how far its cutting edge is above its tip at each distance from its axis, how far
/// out it reaches, and how much of it cuts.
#[derive(Clone, Debug)]
pub struct Cutter {
    tool: Tool,
    reach: f64,
    /// Its edge rises ever more steeply outwards (every round, flat, V and tapered bit):
    /// the drop can then find the highest touch exactly.
    convex: bool,
    /// How far up from the tip it cuts: its flutes, or three diameters when they are not
    /// known.
    length: f64,
}

impl Cutter {
    #[must_use]
    pub fn new(tool: &Tool) -> Self {
        let reach = tool.reach().max(1e-6);
        let convex = !matches!(tool.shape, ToolShape::FormTool | ToolShape::ThreadMill);
        let length = if tool.flute_length > 0.0 {
            tool.flute_length
        } else {
            3.0 * tool.diameter
        };
        Self {
            tool: tool.clone(),
            reach,
            convex,
            length: length.max(reach),
        }
    }

    #[must_use]
    pub fn tool(&self) -> &Tool {
        &self.tool
    }

    /// How far out from its axis it reaches.
    #[must_use]
    pub fn reach(&self) -> f64 {
        self.reach
    }

    /// How far up from its tip it cuts.
    #[must_use]
    pub fn length(&self) -> f64 {
        self.length
    }

    #[must_use]
    pub fn is_convex(&self) -> bool {
        self.convex
    }

    /// How far above its tip its edge is at `r` from its axis; `None` beyond it.
    #[must_use]
    pub fn edge(&self, r: f64) -> Option<f64> {
        if r > self.reach + 1e-12 {
            return None;
        }
        self.tool.profile(r.min(self.reach))
    }

    /// Whether a point `r` from its axis and `up` above its tip is inside the part of it
    /// that cuts.
    #[must_use]
    pub fn holds(&self, r: f64, up: f64) -> bool {
        up <= self.length && self.edge(r).is_some_and(|edge| up >= edge)
    }
}
