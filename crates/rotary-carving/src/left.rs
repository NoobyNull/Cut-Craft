//! What is left of the material once the toolpaths above one have cut: each of their
//! moves cut away from the blank as the machine would cut it, A, the across axis and Z
//! moving together.

use cut_craft_core::glam::DVec3;
use cut_craft_core::rotary_axis;
use cut_craft_core::toolpath::{MoveKind, Toolpath};
use cut_craft_rotary::{Cutter, Pose, Voxels};

/// Where a move of a rotary toolpath puts the bit: its moves are where the tip cuts on the
/// unrolled material, from the material's corner `corner` in the job's X and Y, with how
/// far across the axis the bit is held.
#[must_use]
pub fn pose_of(to: DVec3, across: f64, corner: DVec3, radius: f64, along_x: bool) -> Pose {
    let (along, round) = if along_x {
        (to.x - corner.x, to.y - corner.y)
    } else {
        (to.y - corner.y, to.x - corner.x)
    };
    let (round, z) = if across == 0.0 {
        (round, to.z)
    } else {
        rotary_axis::held_at(radius, round, to.z, across)
    };
    Pose::new(along, across, radius + z, round / radius)
}

/// Cuts from `material` what each of `earlier` (in the order they are cut) takes away: every
/// cutting and plunging move, straight from where the move before it left the bit. A laser's
/// toolpath cuts nothing away here. How many cells were cut.
pub fn cut_by(
    material: &mut Voxels,
    earlier: &[&Toolpath],
    corner: DVec3,
    radius: f64,
    along_x: bool,
) -> usize {
    let mut cut = 0;
    for toolpath in earlier.iter().filter(|t| t.power.is_empty()) {
        let cutter = Cutter::new(&toolpath.tool);
        let mut from: Option<Pose> = None;
        for (i, m) in toolpath.moves.iter().enumerate() {
            let across = toolpath.across.get(i).copied().unwrap_or(0.0);
            let to = pose_of(m.to, across, corner, radius, along_x);
            if m.kind != MoveKind::Rapid {
                cut += match from {
                    Some(from) => material.cut(&cutter, from, to),
                    None => material.cut_at(&cutter, to),
                };
            }
            from = Some(to);
        }
    }
    cut
}

/// How far out from the axis `material` reaches in each of `count` stretches `bin` long along
/// it, from its start: 0 where none of it is left.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "bin indices are small and positive"
)]
pub fn reach_along(material: &Voxels, bin: f64, count: usize) -> Vec<f64> {
    let mut reach = vec![0.0f64; count];
    let half = material.cell() * 0.87;
    material.each_solid(|p| {
        if p.x >= 0.0 {
            let i = (p.x / bin) as usize;
            if let Some(r) = reach.get_mut(i) {
                *r = r.max(p.y.hypot(p.z) + half);
            }
        }
    });
    reach
}
