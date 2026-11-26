//! Rotary Locked Angles' passes: A locked at one angle after another, each the one that
//! reaches the most of what is left facing it squarely, and the side facing the bit finished back and forth along
//! the part with A held still; no more than the angles asked for, and fewer when the angles
//! already cut leave nothing worth another.

use cut_craft_core::CommandError;
use cut_craft_core::progress;
use cut_craft_rotary::{Head, Pose, Voxels};

use crate::{Check, DETAIL, Outcome, Pass, Work};

/// What locking A at one angle after another came to.
#[derive(Debug, Default)]
pub struct Locked {
    /// The passes, and whether the bit can come down to each at the rapid rate.
    pub passes: Vec<(Pass, bool)>,
    /// The angles locked at, and where the holder kept the bit from the model.
    pub outcome: Outcome,
    /// Whether another angle would still reach enough to be worth it: the most asked for
    /// were too few.
    pub more: bool,
}

impl Work<'_> {
    /// Up to `most` angles, each picked where it reaches the most of what is left of the
    /// `material` (which it cuts as it goes), the holder kept clear; and what came of it: the
    /// angles locked at, in order, where the holder kept the bit from material still left for
    /// it, and whether more is left than `most` angles reach.
    ///
    /// # Errors
    /// When it is stopped.
    pub fn locked_angles(
        &self,
        material: &mut Voxels,
        most: u32,
        head: &Head,
    ) -> Result<Locked, CommandError> {
        let cell = material.cell();
        // a side is cut wherever the bit there meets any of what is left (looking any lower
        // finds the cells the model's surface passes through, and cuts it all again); an
        // angle is picked where it finds a few cells' worth more than half a cell above the
        // model: the cells the finish's cusps leave are not worth another angle
        let fine = Check::new(self.tool, self.tool.diameter, 0.0, 1, cell);
        let detail = Check::new(self.tool, self.tool.diameter, -cell / 2.0, DETAIL, cell);
        // the holder's room, worked out a cell or a millimetre at a time, whichever is coarser
        let room = cell.max(1.0);
        let radius = self.turned.radius;
        let mut passes = vec![];
        let mut refused = vec![];
        let mut tilts: Vec<f64> = vec![];
        let mut more = true;
        progress::stage("Locking A where it reaches the most");
        for k in 0..most as usize {
            progress::stop_if_cancelled()?;
            let Some(angle) = self.best_facing(material, &detail, head, &tilts) else {
                more = false;
                break;
            };
            let round = angle.to_radians() * radius;
            passes.extend(self.locked(round, material, &fine, head, room, &mut refused));
            tilts.push(angle);
            progress::report_steps(k + 1, most as usize);
        }
        if more {
            more = self.best_facing(material, &detail, head, &tilts).is_some();
        } else if let Some(angle) = self.most_held_back(material, &detail, head, &tilts) {
            // no angle worth cutting, but the holder keeps the bit from something left: where
            // it does is said, found by trying that angle on a copy (what it cuts is slivers,
            // not worth an angle)
            let mut probe = material.clone();
            let round = angle.to_radians() * radius;
            let _ = self.locked(round, &mut probe, &fine, head, room, &mut refused);
        }
        // where the holder kept the bit off, and something is still left for it there now
        // (not in the chuck or tailstock: that is meant to be left)
        let left: Vec<Pose> = refused
            .into_iter()
            .filter(|p: &Pose| !head.held(p.along))
            .filter(|&p| {
                let looking = Pose {
                    height: p.height - detail.margin,
                    ..p
                };
                material.held(&detail.looks, looking, DETAIL) >= DETAIL
            })
            .collect();
        Ok(Locked {
            passes,
            outcome: Outcome { left, tilts },
            more,
        })
    }
}
