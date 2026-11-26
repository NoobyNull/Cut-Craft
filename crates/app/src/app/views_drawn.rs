//! Where the 2D and 3D views were drawn on screen last frame, and how the 2D view mapped
//! the job: kept so the next frame, a key or the web API can find what is under the pointer,
//! which view it is over, and how far the 2D view moved on screen.

use crate::view2d::ViewTransform;
use egui::{Pos2, Rect, Vec2};

#[derive(Default)]
pub(super) struct ViewsDrawn {
    /// How the 2D view mapped the job last frame.
    transform: Option<ViewTransform>,
    /// Where the 2D view was on screen last frame.
    area_2d: Option<Rect>,
    /// Where the 3D view was last drawn.
    area_3d: Option<Rect>,
    /// Where the 2D view's top left corner was on screen last frame.
    at_2d: Option<Pos2>,
}

impl ViewsDrawn {
    /// The 2D view was drawn in `area`, mapping the job with `transform`.
    pub(super) fn drawn_2d(&mut self, transform: ViewTransform, area: Rect) {
        self.transform = Some(transform);
        self.area_2d = Some(area);
    }

    /// The 2D view is not shown (the start page): no job point is under the pointer.
    pub(super) fn forget_2d(&mut self) {
        self.transform = None;
    }

    /// How the 2D view mapped the job last frame, if it was drawn.
    pub(super) fn transform(&self) -> Option<ViewTransform> {
        self.transform
    }

    /// Where the 2D view was last drawn.
    pub(super) fn area_2d(&self) -> Option<Rect> {
        self.area_2d
    }

    /// The 3D view was drawn in `area`.
    pub(super) fn drawn_3d(&mut self, area: Rect) {
        self.area_3d = Some(area);
    }

    /// Where the 3D view was last drawn.
    pub(super) fn area_3d(&self) -> Option<Rect> {
        self.area_3d
    }

    /// Whether `at` is over either view, as last drawn.
    pub(super) fn over_a_view(&self, at: Pos2) -> bool {
        [self.area_2d, self.area_3d]
            .into_iter()
            .flatten()
            .any(|a| a.contains(at))
    }

    /// The 2D view's top left corner is at `at` this frame: how far it moved on screen
    /// since last frame (none the first time).
    pub(super) fn moved_2d(&mut self, at: Pos2) -> Option<Vec2> {
        self.at_2d.replace(at).map(|was| at - was)
    }

    /// Where the 2D view's top left corner was last frame.
    #[cfg(test)]
    pub(super) fn at_2d(&self) -> Option<Pos2> {
        self.at_2d
    }
}

#[cfg(test)]
mod tests;
