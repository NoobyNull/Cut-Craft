//! Part of the rotary carving engine.

use cut_craft_core::component::Combine;
use cut_craft_core::document::{Document, Support};
use cut_craft_core::glam::DVec3;
use cut_craft_core::toolpath::Tool;
use cut_craft_rotary::{Cutter, Mesh};

/// Full 3D models as they really are round the axis: undercuts, holes through and all.
pub struct Solid {
    pub mesh: Mesh,
    /// The bit grown by the allowance all round.
    pub cutter: Cutter,
    pub allowance: f64,
    /// How far out the models are moved from where they really are: none.
    pub out: f64,
    /// Where the material starts along the axis in the job's X or Y: the models are kept
    /// along the job as it is, the toolpath works from the material's start.
    pub along_from: f64,
}

impl Solid {
    /// Every shown full 3D model of `doc` kept round the axis, raised or merged onto the
    /// rest (one sunk into the rest, or multiplying it, is left to its heights), moved `out`
    /// from the axis.
    pub fn of(
        doc: &Document,
        tool: &Tool,
        allowance: f64,
        out: f64,
        along_from: f64,
    ) -> Option<Self> {
        let triangles: Vec<_> = doc
            .components
            .values()
            .filter(|k| k.visible && takes_solid(k))
            .flat_map(cut_craft_core::component::Component::solid_triangles)
            .collect();
        if triangles.is_empty() {
            return None;
        }
        let grown = Tool {
            diameter: tool.diameter + 2.0 * allowance.max(0.0),
            ..tool.clone()
        };
        Some(Self {
            mesh: Mesh::new(triangles),
            cutter: Cutter::new(&grown),
            allowance: allowance.max(0.0),
            out,
            along_from,
        })
    }

    /// [`Turned::drop`] on the models themselves, A turned `turn` radians.
    pub fn drop(&self, along: f64, turn: f64, across: f64) -> f64 {
        self.mesh
            .drop(&self.cutter, along + self.along_from, across, turn)
            .map_or(0.0, |tip| tip + self.allowance + self.out)
    }
}

/// Whether a component is cut as its real 3D shape: a full model on a rotary job, raised
/// onto or merged with what is under it.
pub fn takes_solid(k: &cut_craft_core::component::Component) -> bool {
    !k.solid.is_empty() && matches!(k.combine, Combine::Add | Combine::Merge)
}

/// Segments round each round part of the support: fine enough that the bit stays within
/// a few thousandths of a millimetre of it, and never inside it.
pub const SUPPORT_SEGMENTS: usize = 96;

/// A round bar on the turning axis, `radius` out, from `from` to `to` along it (the job's X
/// or Y), ends closed: its sides just outside the circle, so nothing inside it is cut.
pub fn bar(radius: f64, from: f64, to: f64) -> Vec<[DVec3; 3]> {
    use std::f64::consts::TAU;
    #[expect(clippy::cast_precision_loss, reason = "a few segments")]
    let step = TAU / SUPPORT_SEGMENTS as f64;
    let out = radius / (step / 2.0).cos();
    let at = |along: f64, k: usize| {
        #[expect(clippy::cast_precision_loss, reason = "a few segments")]
        let (sin, cos) = (k as f64 * step).sin_cos();
        DVec3::new(along, out * sin, out * cos)
    };
    let (a, b) = (DVec3::new(from, 0.0, 0.0), DVec3::new(to, 0.0, 0.0));
    (0..SUPPORT_SEGMENTS)
        .flat_map(|k| {
            let n = k + 1;
            [
                [at(from, k), at(to, k), at(to, n)],
                [at(from, k), at(to, n), at(from, n)],
                [a, at(from, n), at(from, k)],
                [b, at(to, k), at(to, n)],
            ]
        })
        .collect()
}

/// What Job Setup's Holding the Part leaves of the material, as a model the bit is kept
/// off: the centre stock along the whole axis (unless it is being cut away), and the
/// material whole past the model (from `model`, along the material) at an end it stays
/// joined at. `None` when it leaves nothing.
pub fn supporting(
    support: Support,
    tool: &Tool,
    swing: f64,
    length: f64,
    along_from: f64,
    model: Option<(f64, f64)>,
    cut_beam: bool,
) -> Option<Solid> {
    let mut triangles = vec![];
    if support.beam > 0.0 && !cut_beam {
        triangles.extend(bar(support.beam / 2.0, along_from, along_from + length));
    }
    if let Some((first, last)) = model {
        // a little past the blank's corners, so all of it is left
        let whole = swing + 1.0;
        let (chuck, tailstock) = support.joined.ends();
        if chuck && first > 0.0 {
            triangles.extend(bar(whole, along_from, along_from + first));
        }
        if tailstock && last < length {
            triangles.extend(bar(whole, along_from + last, along_from + length));
        }
    }
    (!triangles.is_empty()).then(|| Solid {
        mesh: Mesh::new(triangles),
        cutter: Cutter::new(tool),
        allowance: 0.0,
        out: 0.0,
        along_from,
    })
}

/// One pass of the bit: points (along, A as mm round the surface, the bit's height from the
/// material's top) each with how far across the axis the bit is held there.
pub type Pass = Vec<(DVec3, f64)>;
