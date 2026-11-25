//! Previewing toolpaths: playing them on the material, and the material as it is cut.

use super::super::*;

impl Harness {
    /// Options ▸ Animate toolpath preview, on or off.
    pub(in super::super) fn given_animated_preview(&mut self, on: bool) {
        self.app.display.animate_preview = on;
    }

    /// The Preview Control's speed.
    pub(in super::super) fn set_playback_speed(&mut self, speed: f64) {
        self.app.player.speed = speed;
    }

    /// The preview playing, if one is.
    pub(in super::super) fn playback(&self) -> Option<&cut_craft_core::playback::Playback> {
        self.app.player.playback.as_ref()
    }

    /// A rotary job's file played turning the material, if one is shown.
    pub(in super::super) fn turning(&self) -> Option<&cut_craft_rotary::Replay> {
        self.app.turning.as_ref().map(|t| &t.replay)
    }

    /// Runs the playing preview to its end at once.
    pub(in super::super) fn finish_playback(&mut self) {
        self.app.finish_playback();
    }

    /// Preview on toolpath `id` alone.
    pub(in super::super) fn preview_toolpath(&mut self, id: EntityId) {
        self.app.preview_toolpath(id);
    }

    /// Preview on the toolpaths `ids`, as a group's Preview does.
    pub(in super::super) fn preview_toolpaths(&mut self, ids: &[EntityId]) {
        self.app.preview_ids(ids);
    }

    /// The previewed material's height at its first cell.
    pub(in super::super) fn previewed_height(&self) -> f64 {
        self.app
            .preview
            .material
            .as_ref()
            .expect("a preview was played")
            .heights[0]
    }

    /// An earlier preview's cut left in the material: its first cell at `z`.
    pub(in super::super) fn given_previewed_height(&mut self, z: f64) {
        self.app
            .preview
            .material
            .as_mut()
            .expect("a preview was played")
            .heights[0] = z;
    }

    // ---- the material in the 3D view ----

    /// The material's surface the 3D view shows: uncut, the model on it, or cut by a preview.
    pub(in super::super) fn surface_3d(&mut self) -> Option<Material> {
        self.app.surface_3d().map(std::borrow::Cow::into_owned)
    }

    /// Whether the 3D view shows the uncut block (no model yet).
    pub(in super::super) fn showing_uncut_material(&self) -> bool {
        self.app.showing_uncut_material()
    }

    /// Whether the model is shown seated in the material block.
    pub(in super::super) fn model_on_material(&self) -> bool {
        self.app.model_on_material()
    }

    /// Whether the model being imported is shown whole (under Options ▸ Model preview faces).
    pub(in super::super) fn importing_whole(&self) -> bool {
        self.app.importing_whole()
    }

    /// The height the 3D view's origin arrows are drawn at.
    pub(in super::super) fn origin_height_3d(&self) -> f64 {
        self.app.origin_height_3d()
    }

    /// The part of the model being imported that is past the material's edge, if measured.
    pub(in super::super) fn importing_beyond(&self) -> Option<&[cmd_orientate_3d_model::Seen]> {
        self.app
            .form
            .as_ref()
            .and_then(|f| f.beyond.as_deref())
            .map(Vec::as_slice)
    }

    /// Whether the open import form draws its model whole, not as its component.
    pub(in super::super) fn import_drawn_whole(&self) -> bool {
        self.app.form.as_ref().expect("a form is open").model_whole
    }

    /// Material Setup's measure of the model's height, mm.
    pub(in super::super) fn model_height(&self) -> f64 {
        self.app.placement_room().tall
    }

    /// Asks for Material Setup on the first click on the Toolpaths page, as a new job does.
    pub(in super::super) fn given_material_not_asked(&mut self) {
        self.app.ask_material = true;
    }
}
