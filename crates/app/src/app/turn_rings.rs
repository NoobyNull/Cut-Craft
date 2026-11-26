//! Orientate 3D Model's rotation handles in the 3D view: the red, green and blue rings
//! that turn the model about X, Y and Z, and the grey sphere in their middle that turns it
//! about all three. Kept as last drawn, on screen, so a press can find the handle under it
//! and a drag can turn the model the way it runs round the ring held.

use egui::{Pos2, Vec2};

/// The rings on screen (each ring's axis and its points), and the middle sphere's centre
/// and radius.
#[cfg(test)]
pub(super) type Drawn = (Vec<(&'static str, Vec<Pos2>)>, Option<(Pos2, f32)>);

#[derive(Default)]
pub(super) struct TurnRings {
    /// Each ring's axis (`x`, `y`, `z`) and its points on screen, as drawn last frame.
    rings: Vec<(&'static str, Vec<Pos2>)>,
    /// The middle sphere's centre and radius on screen, as drawn last frame.
    sphere: Option<(Pos2, f32)>,
    /// The handle a drag started on (`xyz`: the sphere, all three axes).
    held: Option<&'static str>,
    /// While a ring is held: the way + runs round it under the pointer, on screen, and how
    /// far it reaches from its middle, points.
    grip: Option<(Vec2, f32)>,
}

impl TurnRings {
    /// Nothing drawn yet this frame: no handles to find.
    pub(super) fn clear(&mut self) {
        self.rings.clear();
        self.sphere = None;
    }

    /// The ring about `axis`, drawn through `points`.
    pub(super) fn add_ring(&mut self, axis: &'static str, points: Vec<Pos2>) {
        self.rings.push((axis, points));
    }

    /// The sphere, drawn at `centre` with `radius`.
    pub(super) fn set_sphere(&mut self, centre: Pos2, radius: f32) {
        self.sphere = Some((centre, radius));
    }

    /// Each ring's points, in the order drawn.
    pub(super) fn ring_points(&self) -> Vec<&[Pos2]> {
        self.rings.iter().map(|(_, p)| p.as_slice()).collect()
    }

    /// The rings and the sphere as last drawn.
    #[cfg(test)]
    pub(super) fn drawn(&self) -> Drawn {
        (self.rings.clone(), self.sphere)
    }

    /// The handle within `reach` of `at` as last drawn: the sphere (`xyz`) first, else the
    /// nearest ring.
    pub(super) fn at(&self, at: Pos2, reach: f32) -> Option<&'static str> {
        if let Some((centre, radius)) = self.sphere
            && centre.distance(at) <= radius
        {
            return Some("xyz");
        }
        let off = |points: &[Pos2]| {
            points
                .windows(2)
                .map(|w| {
                    let (a, b) = (w[0], w[1]);
                    let along = (at - a).dot(b - a) / (b - a).length_sq().max(f32::EPSILON);
                    at.distance(a + (b - a) * along.clamp(0.0, 1.0))
                })
                .fold(f32::INFINITY, f32::min)
        };
        self.rings
            .iter()
            .map(|(axis, points)| (*axis, off(points)))
            .filter(|(_, d)| *d <= reach)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(axis, _)| axis)
    }

    /// A drag that started at `pressed` takes the handle within `reach` of it.
    pub(super) fn press(&mut self, pressed: Option<Pos2>, reach: f32) {
        self.held = pressed.and_then(|at| self.at(at, reach));
    }

    /// The drag is over: no handle is held.
    pub(super) fn let_go(&mut self) {
        self.held = None;
    }

    /// The handle held, if any.
    pub(super) fn held(&self) -> Option<&'static str> {
        self.held
    }

    /// Follows the pointer at `pointer` round the ring held: see [`Self::grip`].
    pub(super) fn follow(&mut self, pointer: Option<Pos2>) {
        self.grip = self.held.and_then(|axis| {
            let (_, points) = self.rings.iter().find(|(a, _)| *a == axis)?;
            let (centre, _) = self.sphere?;
            let along = crate::axes::ring_tangent(points, pointer?)?;
            let radius = points
                .iter()
                .map(|p| p.distance(centre))
                .fold(0.0, f32::max);
            Some((along, radius))
        });
    }

    /// While a ring is held: the way + runs round it under the pointer, on screen, and how
    /// far it reaches from its middle.
    pub(super) fn grip(&self) -> Option<(Vec2, f32)> {
        self.grip
    }
}

#[cfg(test)]
mod tests;
