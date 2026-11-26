//! The popup the 3D view's Settings handle opens: Component Properties' combine mode, fade,
//! tilt and appearance for one component, applied through the command as they change (a
//! dragged value as it goes, the whole drag one undo), and the fade's or tilt's anchors
//! placed with two clicks in the 3D view.

use cmd_component_properties::Params;
use cut_craft_core::document::EntityId;
use cut_craft_core::geometry::Point;

/// Whose anchors the 3D view's clicks place (help: in the 3D view, the fade's or
/// tilt's direction is set there).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Anchor {
    Fade,
    Tilt,
}

impl Anchor {
    /// What to click next, once `first` is placed or not.
    pub(super) fn prompt(self, first: bool) -> &'static str {
        match (self, first) {
            (Self::Fade, false) => "Click the point that keeps its height",
            (Self::Fade, true) => "Click the point the component fades down toward",
            (Self::Tilt, false) => "Click the point that does not move",
            (Self::Tilt, true) => "Click the point tilted up",
        }
    }
}

pub(super) struct Settings3d {
    /// The component whose settings these are.
    pub(super) component: EntityId,
    /// Its settings as being edited.
    pub(super) params: Params,
    /// Where it opened on screen.
    pub(super) at: egui::Pos2,
    /// Its combine-mode radios, where they were drawn last frame.
    pub(super) radios: Vec<(cut_craft_core::component::Combine, egui::Pos2)>,
    /// Set pressed: the fade's or tilt's anchors being clicked in the 3D view, and the
    /// first point once clicked.
    pub(super) anchors: Option<(Anchor, Option<Point>)>,
    /// Edited since last applied.
    dirty: bool,
    /// While a value is dragged, the history's revision after its last step: the next step
    /// takes it back first, if nothing has changed the history since.
    dragged: Option<u64>,
}

impl Settings3d {
    /// The popup for `component`, with its settings `params`, opened at `at`.
    pub(super) fn open(component: EntityId, params: Params, at: egui::Pos2) -> Self {
        Self {
            component,
            params,
            at,
            radios: vec![],
            anchors: None,
            dirty: false,
            dragged: None,
        }
    }

    /// Something was edited (`true`), to be applied.
    pub(super) fn changed(&mut self, edited: bool) {
        self.dirty |= edited;
    }

    /// A click in the 3D view at `p` while `which`'s anchors are set (`first` the point
    /// clicked before, if one was): the first is kept until the second, which turns the
    /// fade or tilt on between them.
    pub(super) fn place_anchor(&mut self, which: Anchor, first: Option<Point>, p: Point) {
        use cut_craft_core::params::Xy;
        let Some(first) = first else {
            self.anchors = Some((which, Some(p)));
            return;
        };
        let (from, to) = (Xy::new(first.x, first.y), Xy::new(p.x, p.y));
        match which {
            Anchor::Fade => {
                self.params.fade.on = true;
                self.params.fade.from = from;
                self.params.fade.to = to;
            }
            Anchor::Tilt => {
                self.params.tilt.on = true;
                self.params.tilt.pivot = from;
                self.params.tilt.toward = to;
            }
        }
        self.anchors = None;
        self.dirty = true;
    }

    /// The settings to apply this frame, if they were edited, and the history's revision
    /// after the last step of the same drag (to take back first, so the drag is one undo).
    /// `pressed`: the pointer is down, so a drag may still be going on.
    pub(super) fn take_edits(&mut self, pressed: bool) -> Option<(Params, Option<u64>)> {
        if !pressed {
            self.dragged = None;
        }
        if !self.dirty {
            return None;
        }
        self.dirty = false;
        Some((self.params.clone(), self.dragged.take()))
    }

    /// The settings were applied, `made` a step in the history now at `revision`: while the
    /// pointer is `pressed`, the next change takes that step back first.
    pub(super) fn applied(&mut self, pressed: bool, made: bool, revision: u64) {
        if pressed && made {
            self.dragged = Some(revision);
        }
    }
}

#[cfg(test)]
mod tests;
